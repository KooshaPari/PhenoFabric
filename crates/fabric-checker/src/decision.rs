//! Decision taxonomy for `fabric-checker`.
//!
//! See ADR-0027 for the rationale. A `Decision` is the answer to one
//! question: "is this host + this manifest combination admissible for
//! placement?"  Three answers, no more:
//!   * `Admit`         — every requirement satisfied with at least Margin=0.
//!   * `AdmitWithNotes` — admissible but with at least one advisory (e.g.,
//!     `AcceleratorMissing` for an optional GPU).
//!   * `Reject`        — at least one hard requirement is not met.
//!
//! Reasons are the evidence: every `Decision` carries one or more
//! `CheckOutcome`s. The `Severity` is `Hard` (block) or `Soft` (advisory).
//!
//! The `ReasonCode` enum is the exhaustive set of machine-readable reason
//! codes a checker may emit. Tests in `tests/reason_codes.rs` lock the
//! wire format — changing a name is a breaking change.

use serde::{Deserialize, Serialize};

/// The outcome of a single check: Admit, AdmitWithNotes, or Reject.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision {
    /// All checks passed.
    Admit,
    /// Admissible but with advisory notes.
    AdmitWithNotes { notes: Vec<CheckOutcome> },
    /// At least one hard requirement failed.
    Reject {
        reason_code: ReasonCode,
        reason_message: String,
    },
}

impl PartialEq for Decision {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Admit, Self::Admit) => true,
            (Self::AdmitWithNotes { notes: a }, Self::AdmitWithNotes { notes: b }) => a == b,
            (
                Self::Reject {
                    reason_code: rc1,
                    reason_message: rm1,
                },
                Self::Reject {
                    reason_code: rc2,
                    reason_message: rm2,
                },
            ) => rc1 == rc2 && rm1 == rm2,
            _ => false,
        }
    }
}

impl Eq for Decision {}

impl Decision {
    pub fn is_admissible(&self) -> bool {
        matches!(self, Self::Admit | Self::AdmitWithNotes { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Hard failure — rejects the decision immediately.
    Reject,
    /// Soft advisory — produces an AdmitWithNotes.
    AdmitWithNotes,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CheckOutcome {
    pub severity: Severity,
    pub code: ReasonCode,
    pub message: String,
}

impl CheckOutcome {
    /// Create a hard (reject-level) check outcome.
    pub fn hard(code: ReasonCode, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Reject,
            code,
            message: message.into(),
        }
    }

    /// Create a soft (advisory) check outcome.
    pub fn soft(code: ReasonCode, message: impl Into<String>) -> Self {
        Self {
            severity: Severity::AdmitWithNotes,
            code,
            message: message.into(),
        }
    }

    /// Create a check outcome with an explicit severity.
    pub fn fail(code: ReasonCode, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            severity,
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
    // --- hardware resources (Reject) ---
    /// Manifest requires more memory than the host exposes.
    MemoryInsufficient,
    /// Host memory could not be determined.
    MemoryUnknown,
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

    // --- storage (Reject) ---
    /// Host storage could not be determined.
    StorageUnknown,
    /// Manifest requires more storage than the host provides.
    StorageInsufficient,

    // --- OS / arch (Reject) ---
    /// Host OS is not in the manifest's allowed families.
    OsIncompatible,
    /// Host architecture is not in the manifest's allowed arches.
    ArchIncompatible,

    // --- display capability (Reject/AdmitWithNotes) ---
    /// Host display resolution below the manifest's minimum.
    DisplayResolutionInsufficient,
    /// Host display refresh-rate below the manifest's minimum.
    DisplayRefreshRateInsufficient,
    /// Host display does not support the requested color depth.
    DisplayColorDepthInsufficient,

    // --- network (Reject) ---
    /// Manifest requires a minimum link bandwidth the host cannot reach
    /// on any interface.
    BandwidthInsufficient,
    /// Manifest requires a latency budget the host's worst link
    /// exceeds.
    LatencyBudgetExceeded,
    /// Manifest requires a specific link class (RDMA, PCIe P2P) the
    /// host does not provide.
    LinkClassMissing,

    // --- real-time guarantees (AdmitWithNotes unless explicit) ---
    /// Manifest declares `real_time: true` but the host does not
    /// advertise any RT island.  Soft because some manifests tolerate
    /// degraded scheduling.
    RealTimeIslandMissing,
    /// Manifest's required RT priority is higher than the host offers.
    RealTimePriorityUnavailable,

    // --- trust (Reject) ---
    /// Manifest requires a higher trust level than the descriptor
    /// carries (e.g. requires Audited, descriptor is Provided).
    TrustLevelInsufficient,
    /// Manifest requires a specific trust root, host descriptor has
    /// no matching root in its chain.
    TrustRootMissing,
    /// Descriptor signature is present but not by a trusted key.
    SignatureUntrusted,

    // --- data locality (AdmitWithNotes) ---
    /// Manifest declares data-residency requirements the host cannot
    /// meet (e.g. requires EU-only data, host is in US).
    DataResidencyViolation,
    /// Manifest's preferred locality tier is stricter than the host's
    /// best achievable (e.g. prefers L0SameProcess, host can only
    /// reach L3SameHostPcie).
    LocalityPreferenceUnsatisfied,

    // --- presence / schema (Reject) ---
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

    // --- advisories (AdmitWithNotes) ---
    /// Manifest is admissible but a newer driver / firmware would
    /// improve some metric.
    FirmwareUpdateAvailable,
    /// Host exposes a capability the manifest did not declare; flag
    /// as advisory so operators can re-check the manifest.
    UnexpectedCapability,
    /// Manifest's declared requirements are self-inconsistent (e.g.
    /// requires more memory than declared host total).
    ManifestSelfInconsistent,

    // --- descriptor shape (Reject / AdmitWithNotes) ---
    /// Descriptor has no signatures.
    SignatureMissing,
    /// Descriptor epoch is 0 (initial probe, never validated).
    EpochZero,
    /// Descriptor schema version is not in the supported list.
    SchemaUnsupported,
    /// Descriptor node_id is the nil UUID.
    NodeIdNil,
    /// Descriptor topology hash is empty.
    TopologyHashMissing,
    /// Descriptor probe timestamp is stale (>24 h).
    ProbeStale,
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
            | StorageInsufficient
            | OsIncompatible
            | ArchIncompatible
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
            | MalformedInput
            | MemoryUnknown
            | StorageUnknown => Severity::Reject,

            RealTimeIslandMissing
            | DataResidencyViolation
            | LocalityPreferenceUnsatisfied
            | FirmwareUpdateAvailable
            | UnexpectedCapability
            | ManifestSelfInconsistent
            | SignatureMissing
            | EpochZero
            | SchemaUnsupported
            | NodeIdNil
            | TopologyHashMissing
            | ProbeStale => Severity::AdmitWithNotes,
        }
    }

    /// Stable wire-format identifier. NEVER rename — Go reference
    /// checker keys off this string.
    pub fn as_str(&self) -> &'static str {
        use ReasonCode::*;
        match self {
            MemoryInsufficient => "MemoryInsufficient",
            MemoryUnknown => "MemoryUnknown",
            CoresInsufficient => "CoresInsufficient",
            AcceleratorClassMissing => "AcceleratorClassMissing",
            AcceleratorGenerationMismatch => "AcceleratorGenerationMismatch",
            DisplayRequiredButMissing => "DisplayRequiredButMissing",
            CaptureRequiredButMissing => "CaptureRequiredButMissing",
            StorageUnknown => "StorageUnknown",
            StorageInsufficient => "StorageInsufficient",
            OsIncompatible => "OsIncompatible",
            ArchIncompatible => "ArchIncompatible",
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
            SignatureMissing => "SignatureMissing",
            EpochZero => "EpochZero",
            SchemaUnsupported => "SchemaUnsupported",
            NodeIdNil => "NodeIdNil",
            TopologyHashMissing => "TopologyHashMissing",
            ProbeStale => "ProbeStale",
        }
    }
}
