use crate::surface::{SurfaceLease, SurfaceSpec, SurfaceSpecError};
use crate::surface_ops::new_lease;

// ---------------------------------------------------------------------------
// PardonError
// ---------------------------------------------------------------------------

/// Errors from `pardon`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PardonError {
    /// The provided spec failed validation.
    SpecInvalid(SurfaceSpecError),
    /// The operator token was rejected (signature/identity verification
    /// failed). Production would verify an Ed25519 signature; this MVP
    /// checks for a known-prefix token.
    TokenRejected,
}

impl std::fmt::Display for PardonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SpecInvalid(e) => write!(f, "pardon: invalid SurfaceSpec: {e}"),
            Self::TokenRejected => write!(
                f,
                "pardon: operator token rejected (must be 'ops:phenotype:default' or signed)"
            ),
        }
    }
}

impl std::error::Error for PardonError {}

// ---------------------------------------------------------------------------
// pardon
// ---------------------------------------------------------------------------

/// Operator-initiated rescue of a `Revoked` lease (Q4-C from
/// `releases/2026-09-08-R1.md:184`).
///
/// Creates a NEW `SurfaceLease` from the same spec — the revoked lease is
/// left in place (audit trail intact). The new lease is issued in
/// `LeaseState::Pending`; the caller is expected to bind it via the normal
/// `surface_ops::bind` path.
///
/// This is intentionally NOT a re-bind of the existing lease: spec 019's
/// "no-steal" invariant says once a lease is `Revoked`, it stays
/// terminal. Operator override is an out-of-band operation that creates
/// a new lease, which is logged separately.
///
/// # Errors
///
/// - `PardonError::TokenRejected` — `operator_token` doesn't match the
///   known prefix. Production would verify an Ed25519 signature.
///
/// - `PardonError::SpecInvalid` — `spec.validate()` returned an error.
///
/// The function never touches the revoked lease; that audit trail stays.
pub fn pardon(spec: SurfaceSpec, operator_token: &str) -> Result<SurfaceLease, PardonError> {
    // Token check (MVP): known prefix. Production would verify Ed25519
    // signature per ADR-0028.
    const VALID_TOKEN_PREFIX: &str = "ops:phenotype:";
    if !operator_token.starts_with(VALID_TOKEN_PREFIX) {
        return Err(PardonError::TokenRejected);
    }
    // Re-validate the spec (defense in depth — even if the caller already
    // validated, we re-run validation to catch a tampered spec).
    spec.validate().map_err(PardonError::SpecInvalid)?;
    // Issue a new lease via the canonical path. `new_lease` returns
    // `SurfaceSpecError` (no topology involved), which we map 1:1.
    new_lease(spec).map_err(PardonError::SpecInvalid)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::surface;

    // ---------------- T-F07 ----------------

    #[test]
    fn pardon_valid_token_returns_pending_lease() {
        let spec = SurfaceSpec {
            name: "operator-rescued".to_string(),
            protocol: surface::SurfaceProtocol::WebRtc,
            capture: None,
            locality_floor: crate::LocalityTier::L2CrossNumaShm,
            refresh_hz: None,
            audio_sample_rate_hz: None,
            requires_rt_island: false,
            strict_epoch_binding: false,
            min_host_trust: crate::TrustLevel::Attested,
            expires_at: None,
        };
        let lease = pardon(spec, "ops:phenotype:default").expect("valid token");
        assert_eq!(
            lease.state,
            surface::LeaseState::Pending,
            "pardoned lease must be Pending"
        );
        assert_eq!(lease.spec.name, "operator-rescued");
        assert!(lease.history.is_empty());
    }

    // ---------------- T-F08 ----------------

    #[test]
    fn pardon_bad_token_rejected() {
        let spec = SurfaceSpec {
            name: "x".to_string(),
            protocol: surface::SurfaceProtocol::WebRtc,
            capture: None,
            locality_floor: crate::LocalityTier::L2CrossNumaShm,
            refresh_hz: None,
            audio_sample_rate_hz: None,
            requires_rt_island: false,
            strict_epoch_binding: false,
            min_host_trust: crate::TrustLevel::Attested,
            expires_at: None,
        };
        let result = pardon(spec, "evil-token");
        match result {
            Err(PardonError::TokenRejected) => {}
            other => panic!("expected TokenRejected, got {other:?}"),
        }
    }
}
