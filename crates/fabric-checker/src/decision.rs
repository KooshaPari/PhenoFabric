//! Decision taxonomy for `fabric-checker`.
//!
//! See ADR-0027 for the rationale. A `Decision` is the answer to one
//! question: "is this host + this manifest combination admissible for
//! placement?"  Three answers, no more:
//!   * `Admit`         — every requirement satisfied with at least Margin=0.
//!   * `AdmitWithNotes` — admissible but with at least one advisory (e.g.,
//!                        `AcceleratorMissing` for an optional GPU).
//!   * `Reject`        — at least one hard requirement is not met.
//!
//! Reasons are the evidence: every `Decision` carries one or more
//! `CheckOutcome`s. The `Severity` is `Hard` (block) or `Soft` (advisory).
//!
//! The `ReasonCode` enum is the exhaustive set of machine-readable reason
//! codes a checker may emit. Tests in `tests/reason_codes.rs` lock the
//! wire format — changing a name is a breaking change.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Decision {
    Admit,
    AdmitWithNotes,
    Reject,
}

impl Decision {
    pub fn is_admissible(&self) -> bool {
        matches!(self, Self::Admit | Self::AdmitWithNotes)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    Hard,
    Soft,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CheckOutcome {
    pub severity: Severity,
    pub code: ReasonCode,
    pub message: String,
}

impl CheckOutcome {
    pub fn hard(code: ReasonCode, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Hard,
            code,
            message: message.into(),
        }
    }

    pub fn soft(code: ReasonCode, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Soft,
            code,
            message: message.into(),
        }
    }
}

/// One decision, with all the evidence that produced it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionReport {
    pub decision: Decision,
    pub outcomes: Vec<CheckOutcome>,
    pub descriptor_node_id: String,
    pub manifest_digest: String,
    pub checked_at_unix: u64,
    pub checker_version: &'static str,
}

impl DecisionReport {
    pub fn from_outcomes(
        decision: Decision,
        outcomes: Vec<CheckOutcome>,
        descriptor_node_id: String,
        manifest_digest: String,
        checked_at_unix: u64,
    ) -> Self {
        Self {
            decision,
            outcomes,
            descriptor_node_id,
            manifest_digest,
            checked_at_unix,
            checker_version: env!("CARGO_PKG_VERSION"),
        }
    }
}

/// Exhaustive list of machine-readable reason codes emitted by the checker.
///
/// Wire format note: the variant name (e.g. `"MemoryInsufficient"`) is the
/// stable identifier that downstream tooling and tests key off. Renaming is
/// a breaking change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ReasonCode {
    // --- hardware resources (Hard) ---
    /// Manifest requires more memory than the host exposes.
    MemoryInsufficient,
    /// Manifest requires more cores than the host exposes.
    CoresInsufficient,
    /// Manifest requires an accelerator class (e.g. CUDA) the host
    /// does not advertise.
    AcceleratorClassMissing,
    /// Manifest requires a specific accelerator generation or compute
    /// capability that the host does not match.
    AcceleratorGenerationMismatch,
    /// Manifest requires a display, host has none.
    DisplayRequiredButMissing,
    /// Manifest requires a capture device (mic, camera), host has none.
    CaptureRequiredButMissing,

    // --- display capability (Hard/Soft) ---
    /// Host display resolution below the manifest's minimum.
    DisplayResolutionInsufficient,
    /// Host display refresh-rate below the manifest's minimum.
    DisplayRefreshRateInsufficient,
    /// Host display does not support the requested color depth.
    DisplayColorDepthInsufficient,

    // --- network (Hard) ---
    /// Manifest requires a minimum link bandwidth the host cannot reach
    /// on any interface.
    BandwidthInsufficient,
    /// Manifest requires a latency budget the host's worst link
    /// exceeds.
    LatencyBudgetExceeded,
    /// Manifest requires a specific link class (RDMA, PCIe P2P) the
    /// host does not provide.
    LinkClassMissing,

    // --- real-time guarantees (Soft unless explicit) ---
    /// Manifest declares `real_time: true` but the host does not
    /// advertise any RT island.  Soft because some manifests tolerate
    /// degraded scheduling.
    RealTimeIslandMissing,
    /// Manifest's required RT priority is higher than the host offers.
    RealTimePriorityUnavailable,

    // --- trust (Hard) ---
    /// Manifest requires a higher trust level than the descriptor
    /// carries (e.g. requires Audited, descriptor is Provided).
    TrustLevelInsufficient,
    /// Manifest requires a specific trust root, host descriptor has
    /// no matching root in its chain.
    TrustRootMissing,
    /// Descriptor signature is present but not by a trusted key.
    SignatureUntrusted,

    // --- data locality (Soft) ---
    /// Manifest declares data-residency requirements the host cannot
    /// meet (e.g. requires EU-only data, host is in US).
    DataResidencyViolation,
    /// Manifest's preferred locality tier is stricter than the host's
    /// best achievable (e.g. prefers L0SameProcess, host can only
    /// reach L3SameHostPcie).
    LocalityPreferenceUnsatisfied,

    // --- presence / schema (Hard) ---
    /// Manifest references a capability the descriptor has no record of.
    /// (Different from "missing" — the manifest asks for X, the host
    /// doesn't even know what X is.)
    CapabilityUnknown,
    /// Manifest's schema version is newer than the checker knows.
    ManifestSchemaTooNew,
    /// Descriptor's schema version is older than the checker knows.
    DescriptorSchemaTooOld,
    /// Manifest or descriptor failed to parse as JSON.
    MalformedInput,

    // --- advisories (Soft) ---
    /// Manifest is admissible but a newer driver / firmware would
    /// improve some metric.
    FirmwareUpdateAvailable,
    /// Host exposes a capability the manifest did not declare; flag
    /// as advisory so operators can re-check the manifest.
    UnexpectedCapability,
    /// Manifest's declared requirements are self-inconsistent (e.g.
    /// requires more memory than declared host total).
    ManifestSelfInconsistent,
}

impl ReasonCode {
    pub fn default_severity(&self) -> Severity {
        use ReasonCode::*;
        match self {
            MemoryInsufficient
            | CoresInsufficient
            | AcceleratorClassMissing
            | AcceleratorGenerationMismatch
            | DisplayRequiredButMissing
            | CaptureRequiredButMissing
            | DisplayResolutionInsufficient
            | DisplayRefreshRateInsufficient
            | DisplayColorDepthInsufficient
            | BandwidthInsufficient
            | LatencyBudgetExceeded
            | LinkClassMissing
            | RealTimePriorityUnavailable
            | TrustLevelInsufficient
            | TrustRootMissing
            | SignatureUntrusted
            | CapabilityUnknown
            | ManifestSchemaTooNew
            | DescriptorSchemaTooOld
            | MalformedInput => Severity::Hard,

            RealTimeIslandMissing
            | DataResidencyViolation
            | LocalityPreferenceUnsatisfied
            | FirmwareUpdateAvailable
            | UnexpectedCapability
            | ManifestSelfInconsistent => Severity::Soft,
        }
    }

    /// Stable wire-format identifier. NEVER rename — Go reference
    /// checker keys off this string.
    pub fn as_str(&self) -> &'static str {
        use ReasonCode::*;
        match self {
            MemoryInsufficient => "MemoryInsufficient",
            CoresInsufficient => "CoresInsufficient",
            AcceleratorClassMissing => "AcceleratorClassMissing",
            AcceleratorGenerationMismatch => "AcceleratorGenerationMismatch",
            DisplayRequiredButMissing => "DisplayRequiredButMissing",
            CaptureRequiredButMissing => "CaptureRequiredButMissing",
            DisplayResolutionInsufficient => "DisplayResolutionInsufficient",
            DisplayRefreshRateInsufficient => "DisplayRefreshRateInsufficient",
            DisplayColorDepthInsufficient => "DisplayColorDepthInsufficient",
            BandwidthInsufficient => "BandwidthInsufficient",
            LatencyBudgetExceeded => "LatencyBudgetExceeded",
            LinkClassMissing => "LinkClassMissing",
            RealTimeIslandMissing => "RealTimeIslandMissing",
            RealTimePriorityUnavailable => "RealTimePriorityUnavailable",
            TrustLevelInsufficient => "TrustLevelInsufficient",
            TrustRootMissing => "TrustRootMissing",
            SignatureUntrusted => "SignatureUntrusted",
            DataResidencyViolation => "DataResidencyViolation",
            LocalityPreferenceUnsatisfied => "LocalityPreferenceUnsatisfied",
            CapabilityUnknown => "CapabilityUnknown",
            ManifestSchemaTooNew => "ManifestSchemaTooNew",
            DescriptorSchemaTooOld => "DescriptorSchemaTooOld",
            MalformedInput => "MalformedInput",
            FirmwareUpdateAvailable => "FirmwareUpdateAvailable",
            UnexpectedCapability => "UnexpectedCapability",
            ManifestSelfInconsistent => "ManifestSelfInconsistent",
        }
    }
}
