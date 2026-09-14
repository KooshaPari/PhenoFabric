//! Shared types: trust levels, capability references, and link metrics.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Trust level for a capability descriptor attached to a node.
///
/// Fabric does not *trust* descriptors by default; it requires an attestation
/// chain. The trust level reflects how much verification has been performed.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Default,
)]
pub enum TrustLevel {
    /// Node reported its own capabilities. No external verification.
    /// PF-FR-012: No default trust.
    #[default]
    Untrusted = 0,
    /// Bootstrap trust assigned by local policy (e.g. same admin domain).
    Bootstrap = 1,
    /// Verified against a signed attestation (e.g. TPM quote, SEV-SNP).
    Attested = 2,
    /// Audit complete, cross-verified by a trusted third party.
    Audited = 3,
}

impl TrustLevel {
    /// Whether this trust level is sufficient to meet a minimum requirement.
    #[must_use]
    pub fn satisfies(&self, minimum: TrustLevel) -> bool {
        *self >= minimum
    }
}

// ---------------------------------------------------------------------------
// Capability reference
// ---------------------------------------------------------------------------

/// A reference to a capability descriptor, optionally with trust metadata.
///
/// When attached to a node, this is a pointer to the descriptor plus trust info.
/// When used in an intent, this is a *requirement* against which capabilities
/// are matched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRef {
    /// SHA-256 of the canonical JSON bytes of the descriptor.
    /// Used as the stable, content-addressable identity key.
    pub descriptor_id: String,
    /// Trust level assigned to this descriptor.
    pub trust: TrustLevel,
    /// When this descriptor was last refreshed.
    pub refreshed_at: Option<DateTime<Utc>>,
}

impl CapabilityRef {
    pub fn new(descriptor_id: String) -> Self {
        Self {
            descriptor_id,
            trust: TrustLevel::default(),
            refreshed_at: None,
        }
    }

    pub fn with_trust(mut self, trust: TrustLevel) -> Self {
        self.trust = trust;
        self
    }
}

// ---------------------------------------------------------------------------
// Link metrics
// ---------------------------------------------------------------------------

/// Metrics for a single link (edge) in the topology.
///
/// All fields are Option<T> because not every link has every metric measured.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LinkMetrics {
    /// Round-trip latency in microseconds. None if not measured.
    pub latency_us: Option<f64>,
    /// Estimated one-way bandwidth in bytes per second. None if not measured.
    pub bandwidth_bps: Option<u64>,
    /// Packet loss rate 0.0..1.0. None if not measured.
    pub packet_loss: Option<f64>,
    /// Jitter in microseconds (stddev of latency samples). None if not measured.
    pub jitter_us: Option<f64>,
}

impl LinkMetrics {
    /// Whether all metrics are known (i.e. the link has been actively measured).
    pub fn is_complete(&self) -> bool {
        self.latency_us.is_some()
            && self.bandwidth_bps.is_some()
            && self.packet_loss.is_some()
            && self.jitter_us.is_some()
    }

    /// Estimate effective bandwidth given a required latency.
    pub fn effective_bandwidth(&self, max_latency_us: f64) -> Option<u64> {
        let lat = self.latency_us?;
        if lat > max_latency_us {
            return None;
        }
        let bw = self.bandwidth_bps?;
        // Simple model: effective = bandwidth * (1 - loss_rate) * latency_factor
        let loss_factor = 1.0 - self.packet_loss.unwrap_or(0.0);
        let latency_factor = (max_latency_us / lat).min(1.0);
        Some(((bw as f64) * loss_factor * latency_factor) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_level_satisfies() {
        assert!(TrustLevel::Attested.satisfies(TrustLevel::Untrusted));
        assert!(TrustLevel::Audited.satisfies(TrustLevel::Bootstrap));
        assert!(!TrustLevel::Bootstrap.satisfies(TrustLevel::Attested));
        assert!(TrustLevel::Untrusted.satisfies(TrustLevel::Untrusted));
    }

    #[test]
    fn test_link_metrics_effective_bandwidth() {
        let metrics = LinkMetrics {
            latency_us: Some(100.0),
            bandwidth_bps: Some(1_000_000_000),
            packet_loss: Some(0.001),
            jitter_us: Some(5.0),
        };
        // Under 200us: good
        assert_eq!(metrics.effective_bandwidth(200.0), Some(999_000_000));
        // Under 50us: lat=100 > 50, so fails
        assert_eq!(metrics.effective_bandwidth(50.0), None);
        // Under 100us: exactly the bandwidth
        assert_eq!(metrics.effective_bandwidth(100.0), Some(999_000_000));
        // Over limit: fails
        assert_eq!(metrics.effective_bandwidth(50.0), None);
    }

    #[test]
    fn test_link_metrics_incomplete() {
        let partial = LinkMetrics {
            latency_us: Some(100.0),
            bandwidth_bps: None,
            packet_loss: None,
            jitter_us: None,
        };
        assert!(!partial.is_complete());
    }

    #[test]
    fn test_capability_ref_new() {
        let r = CapabilityRef::new("sha256:test".to_string());
        assert_eq!(r.trust, TrustLevel::Untrusted);
        assert!(r.refreshed_at.is_none());
        let r2 = r.with_trust(TrustLevel::Attested);
        assert_eq!(r2.trust, TrustLevel::Attested);
    }
}
