//! Surface plane — surface leasing and route binding (PF-WP-015, spec 019).
//!
// A `Surface` is a presentation binding: an opaque handle the user keeps open
//! while the route plan may be silently re-planned underneath. The lease FSM
//! guarantees that a held surface either continues to work or is invalidated
//! with a stable failure reason — never silently re-bound to a different
//! capability surface (this is the spec 019 "no-steal" invariant).
//!
//! ## Module map
//! - `SurfaceSpec` — declarative description of a surface (protocol, route binding,
//!   locality floor, display/audio capture rules, RT constraints).
//! - `SurfaceLease` — FSM-held lease bound to a `RouteStep`; transitions through
//!   `Pending → Active → (Completed | Failed | Revoked | Expired)`.
//! - `SurfaceHandle` — opaque id issued to a user-facing session; never reveals
//!   underlying topology changes.
//! - `bind`/`rebind`/`invalidate` — surface operations.
//!
//! ## Stability classification (per ADR-0027 stability model)
//! `SurfaceSpec` is **Stable** for the R1 release. `SurfaceLease` and
//! `SurfaceHandle` are **Provisional** until R1 lands.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::model::{NodeId, RoutePlanId, RouteStep, TrustLevel};
use crate::LocalityTier;

// -----------------------------------------------------------------------------
// SurfaceSpec
// -----------------------------------------------------------------------------

/// Which presentation protocol a surface uses.
///
/// Stable. New variants may be added in minor releases; existing variants
/// are never repurposed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum SurfaceProtocol {
    /// POSIX process I/O (stdin/stdout/pipes/files).
    Posix,
    /// VNC-compatible framebuffer + keyboard/mouse.
    Vnc,
    /// RDP-compatible remote desktop.
    Rdp,
    /// WebRTC (browser-based, audio/video/data channels).
    WebRtc,
    /// Application Streaming Protocol (Apple's iOS-style remote UI).
    Asp,
    /// Custom user-supplied protocol (must be registered via the fabric-capability
    /// extension system).
    Custom(String),
}

/// Per-surface directionality hint for capturing surfaces (webcam, microphone).
///
/// Stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum CaptureDirection {
    /// Read-only capture (the surface sends data toward the user).
    Sink,
    /// Write-only capture (the surface sends commands toward the host).
    Source,
    /// Bidirectional capture (audio + video + control).
    Bidirectional,
}

/// Declarative specification of a user-facing surface.
///
/// Stable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SurfaceSpec {
    /// Stable, human-readable name for this surface (e.g. "primary-display",
    /// "headphones", "webcam-0"). Unique within a workspace.
    pub name: String,
    /// Which protocol the surface uses.
    pub protocol: SurfaceProtocol,
    /// Which direction the surface captures (None for non-capturing surfaces
    /// like displays).
    pub capture: Option<CaptureDirection>,
    /// Hard floor on locality: the surface must be served from a node at
    /// least this close (e.g. `L2CrossNumaShm` means "same host or die").
    pub locality_floor: LocalityTier,
    /// Display refresh rate in Hz (None for non-display surfaces).
    pub refresh_hz: Option<u32>,
    /// Audio sample rate in Hz (None for non-audio surfaces).
    pub audio_sample_rate_hz: Option<u32>,
    /// Whether the surface requires a real-time thread (affects routing
    /// — RT islands are reserved for the highest-priority surfaces).
    pub requires_rt_island: bool,
    /// Whether the surface must be bound to the host whose capability
    /// descriptor was probed *for the topology epoch that produced the
    /// underlying RoutePlan*. When true, an epoch drift invalidates the
    /// surface (no silent re-bind).
    pub strict_epoch_binding: bool,
    /// Minimum trust level for the host that serves this surface.
    pub min_host_trust: TrustLevel,
    /// Expiry for the surface lease (None = no time-based expiry, lifetime
    /// governed only by workspace/agent shutdown).
    pub expires_at: Option<DateTime<Utc>>,
}

impl SurfaceSpec {
    /// Validate this surface spec. Returns the first violation, if any.
    pub fn validate(&self) -> Result<(), SurfaceSpecError> {
        if self.name.trim().is_empty() {
            return Err(SurfaceSpecError::EmptyName);
        }
        if let Some(c) = self.capture {
            if matches!(self.protocol, SurfaceProtocol::Posix) && !matches!(c, CaptureDirection::Source | CaptureDirection::Bidirectional) {
                return Err(SurfaceSpecError::IncompatibleCapture {
                    protocol: self.protocol.clone(),
                    capture: c,
                });
            }
        }
        if self.requires_rt_island && matches!(self.locality_floor, LocalityTier::L8Oob) {
            return Err(SurfaceSpecError::RtRequiresLocality);
        }
        Ok(())
    }
}

/// Error produced by `SurfaceSpec::validate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceSpecError {
    EmptyName,
    IncompatibleCapture { protocol: SurfaceProtocol, capture: CaptureDirection },
    RtRequiresLocality,
}

impl std::fmt::Display for SurfaceSpecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(f, "SurfaceSpec.name must be non-empty"),
            Self::IncompatibleCapture { protocol, capture } => write!(
                f,
                "SurfaceSpec: capture {:?} not compatible with protocol {:?}",
                capture, protocol
            ),
            Self::RtRequiresLocality => write!(
                f,
                "SurfaceSpec: RT-island requirement cannot be satisfied at L8Oob"
            ),
        }
    }
}

impl std::error::Error for SurfaceSpecError {}

// -----------------------------------------------------------------------------
// RouteBinding — concrete wire-up from a RouteStep to a surface
// -----------------------------------------------------------------------------

/// Which concrete capability surface is bound (display index, audio device
/// index, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum CapabilityEndpoint {
    Display { index: u32 },
    Audio { index: u32 },
    Input { index: u32 },
    Network { port: u16 },
    Storage { path: String },
    Compute { pid: u32 },
}

/// The concrete binding of a `SurfaceSpec` to a `RouteStep` + capability endpoint.
///
/// Provisional until R1 lands.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RouteBinding {
    /// Stable id for this binding; rotated when the route is re-planned.
    pub binding_id: Uuid,
    /// Which `RoutePlan` this binding lives within.
    pub plan_id: RoutePlanId,
    /// Which step in the plan serves this surface.
    pub step_node: NodeId,
    /// Which capability endpoint on the host.
    pub endpoint: CapabilityEndpoint,
    /// The topology epoch at bind time; if `SurfaceSpec::strict_epoch_binding`
    /// is true, drift on this value invalidates the surface.
    pub bound_at_epoch: u64,
    /// Wall-clock bind timestamp.
    pub bound_at: DateTime<Utc>,
}

// -----------------------------------------------------------------------------
// SurfaceLease FSM
// -----------------------------------------------------------------------------

/// Surface lease lifecycle.
///
/// Provisional. Transitions are documented in spec 019 §3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum LeaseState {
    Pending,
    Active,
    Completed,
    Failed,
    Revoked,
    Expired,
}

/// Reason a surface lease left the active state.
///
/// Provisional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum LeaseExitReason {
    /// The underlying route completed normally (workload finished).
    NormalCompletion,
    /// The host running the route failed (link down, host OOM).
    /// Captured from ADR-0030 failover triggers.
    HostFailure { host_node: NodeId },
    /// Plan epoch drift invalidated the binding (strict_epoch_binding only).
    EpochDrift { previous_epoch: u64, new_epoch: u64 },
    /// An admin/workspace operator explicitly revoked the lease.
    OperatorRevoked,
    /// The lease expired (time-based, from `expires_at`).
    Expired,
    /// The workload itself reported an unrecoverable error.
    WorkloadReported { code: String, message: String },
}

/// A held surface lease. The lease is the user-facing handle.
///
/// Provisional.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SurfaceLease {
    /// Opaque, stable handle id (different from `binding_id` — the handle
    /// survives re-binding, the binding id rotates).
    pub handle: SurfaceHandle,
    /// The spec that was requested.
    pub spec: SurfaceSpec,
    /// Current binding (None while Pending).
    pub current: Option<RouteBinding>,
    /// Prior binding(s) — populated if the surface has been re-bound.
    /// Used for "what stayed the same / what changed" telemetry.
    pub history: Vec<RouteBinding>,
    /// Current state.
    pub state: LeaseState,
    /// Why the surface left the Active state (only meaningful for terminal
    /// states — Completed/Failed/Revoked/Expired).
    pub exit_reason: Option<LeaseExitReason>,
    /// When this lease was created.
    pub created_at: DateTime<Utc>,
    /// When this lease transitioned out of `Active` (None while active).
    pub terminated_at: Option<DateTime<Utc>>,
}

/// Opaque user-facing surface handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SurfaceHandle(pub Uuid);

impl SurfaceHandle {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for SurfaceHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// A pending or active binding (used for the `Pending → Active` transition
/// bookkeeping inside `SurfaceLease`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PendingBinding {
    pub requested_at: DateTime<Utc>,
    pub step: RouteStep,
}

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

/// Errors that can arise from surface plane operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceError {
    /// The provided spec failed validation.
    InvalidSpec(SurfaceSpecError),
    /// No `RouteStep` in the plan matches the spec's locality floor + protocol.
    NoMatchingRoute,
    /// The host has the required capability but at a trust level below the spec's
    /// `min_host_trust`.
    InsufficientTrust { required: TrustLevel, offered: TrustLevel },
    /// The lease FSM rejected the transition (e.g. trying to invalidate a
    /// Completed lease).
    IllegalTransition { from: LeaseState, attempted: &'static str },
    /// Plan epoch drift invalidates the binding (`strict_epoch_binding`).
    EpochDrift { previous: u64, current: u64 },
}

impl std::fmt::Display for SurfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSpec(e) => write!(f, "invalid SurfaceSpec: {e}"),
            Self::NoMatchingRoute => write!(f, "no RouteStep in the plan satisfies the surface's locality+protocol requirements"),
            Self::InsufficientTrust { required, offered } => write!(
                f,
                "host trust {offered:?} is below surface requirement {required:?}"
            ),
            Self::IllegalTransition { from, attempted } => write!(
                f,
                "cannot {attempted} from state {from:?}"
            ),
            Self::EpochDrift { previous, current } => write!(
                f,
                "plan epoch drifted from {previous} to {current}; strict-binding surface invalidated"
            ),
        }
    }
}

impl std::error::Error for SurfaceError {}
