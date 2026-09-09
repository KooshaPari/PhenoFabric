//! Integration tests for `fabric_graph::leases_fairness` (spec 022).
//!
//! Spec 022 §5 mandates end-to-end coverage for multi-tenant lease fairness.
//! Each test exercises `FairnessQueue::try_acquire` / `release` / `pardon`
//! against real `SurfaceSpec` instances — proving the fairness layer
//! composes with the surface plane (PF-WP-015) cleanly.
//!
//! Reader's guide (per ADR-0028): tests use only the public API. Internal
//! helpers are tested in the in-module `tests` mod.

use fabric_capability::LocalityTier;
use fabric_graph::{
    leases_fairness::{
        pardon, DenyReason, FairnessDecision, FairnessPolicy, FairnessQueue, PardonError, TenantId,
    },
    surface::{SurfaceProtocol, SurfaceSpec, SurfaceSpecError},
    TrustLevel,
};

// ---------------------------------------------------------------------------
// Helpers (mirror of in-module helpers; intentional duplication so this
// file is independently readable).
// ---------------------------------------------------------------------------

fn valid_spec(name: &str) -> SurfaceSpec {
    SurfaceSpec {
        name: name.to_string(),
        protocol: SurfaceProtocol::WebRtc,
        capture: None,
        locality_floor: LocalityTier::L2CrossNumaShm,
        refresh_hz: Some(60),
        audio_sample_rate_hz: None,
        requires_rt_island: false,
        strict_epoch_binding: false,
        min_host_trust: TrustLevel::Attested,
        expires_at: None,
    }
}

fn spec_invalid() -> SurfaceSpec {
    // Empty name -> SpecError::EmptyName
    SurfaceSpec {
        name: String::new(),
        protocol: SurfaceProtocol::WebRtc,
        capture: None,
        locality_floor: LocalityTier::L2CrossNumaShm,
        refresh_hz: Some(60),
        audio_sample_rate_hz: None,
        requires_rt_island: false,
        strict_epoch_binding: false,
        min_host_trust: TrustLevel::Attested,
        expires_at: None,
    }
}

// ---------------------------------------------------------------------------
// T-FI01 — Fifo rotation round-trips a snapshot for audit serialization
// ---------------------------------------------------------------------------

#[test]
fn fifo_snapshot_serializes_and_round_trips() {
    let mut q = FairnessQueue::new(FairnessPolicy::Fifo);
    let a = TenantId::new("acme-corp");
    let b = TenantId::new("initech");
    q.try_acquire(a.clone(), 7);
    q.try_acquire(b.clone(), 3);

    let snap = q.snapshot();
    let json = serde_json::to_string(&snap).expect("snapshot must serialize");
    let back: fabric_graph::leases_fairness::FairnessSnapshot =
        serde_json::from_str(&json).expect("snapshot must deserialize");
    assert_eq!(snap, back);
    assert_eq!(snap.total_granted, 10);
    assert_eq!(snap.accounting.get(&a).unwrap().granted, 7);
    assert_eq!(snap.accounting.get(&b).unwrap().granted, 3);
}

// ---------------------------------------------------------------------------
// T-FI02 — FairShare gives deficit-ranked tenants equal share under churn
// ---------------------------------------------------------------------------

#[test]
fn fair_share_deficit_rebalances_under_churn() {
    let mut q = FairnessQueue::new(FairnessPolicy::FairShare { weight: 5 });
    let a = TenantId::new("a");
    let b = TenantId::new("b");
    // A keeps grabbing.
    q.try_acquire(a.clone(), 5);
    q.try_acquire(a.clone(), 5);
    q.try_acquire(a.clone(), 5);
    // B asks next — deficit ranking serves B even though A has more history.
    let r = q.try_acquire(b.clone(), 5);
    assert!(matches!(r, FairnessDecision::Granted { .. }));
    let snap = q.snapshot();
    assert_eq!(snap.total_granted, 20);
}

// ---------------------------------------------------------------------------
// T-FI03 — WRR serves tenants in rotation order with policy.weight slots
// ---------------------------------------------------------------------------

#[test]
fn wrr_serves_in_rotation_order() {
    let mut q = FairnessQueue::new(FairnessPolicy::WeightedRoundRobin { weight: 2 });
    let a = TenantId::new("a");
    let b = TenantId::new("b");
    // Register A (2 slots) and exhaust them. After exhaustion, B comes next.
    q.try_acquire(a.clone(), 1);
    q.try_acquire(a.clone(), 1);
    let r_b1 = q.try_acquire(b.clone(), 1);
    assert!(matches!(r_b1, FairnessDecision::Granted { .. }));
    let r_b2 = q.try_acquire(b.clone(), 1);
    assert!(matches!(r_b2, FairnessDecision::Granted { .. }));
    let r_a3 = q.try_acquire(a.clone(), 1);
    assert!(matches!(r_a3, FairnessDecision::Granted { .. }));
    let snap = q.snapshot();
    assert_eq!(snap.accounting.get(&a).unwrap().granted, 3);
    assert_eq!(snap.accounting.get(&b).unwrap().granted, 2);
    assert_eq!(snap.total_granted, 5);
}

// ---------------------------------------------------------------------------
// T-FI04 — PriorityWeighted denies lower-priority tenants under contention
// ---------------------------------------------------------------------------

#[test]
fn priority_weighted_denies_lower_priority_under_contention() {
    let mut q = FairnessQueue::new(FairnessPolicy::PriorityWeighted { priority: 1 });
    let a = TenantId::new("a");
    let b = TenantId::new("b");
    q.try_acquire(a.clone(), 1);
    q.try_acquire(b.clone(), 1);
    // Demote B; A is the higher-priority tenant.
    q.set_priority(&b, 2);
    let r = q.try_acquire(b.clone(), 5);
    match r {
        FairnessDecision::Denied {
            reason: DenyReason::LowerPriority { blocking, .. },
            ..
        } => {
            assert_eq!(blocking, a);
        }
        other => panic!("expected LowerPriority denial, got {other:?}"),
    }
    // A still can acquire.
    let r_a = q.try_acquire(a.clone(), 5);
    assert!(matches!(r_a, FairnessDecision::Granted { .. }));
}

// ---------------------------------------------------------------------------
// T-FI05 — release records lifecycle back to the queue
// ---------------------------------------------------------------------------

#[test]
fn release_records_lifecycle_audit_trail() {
    let mut q = FairnessQueue::new(FairnessPolicy::Fifo);
    let a = TenantId::new("acme-corp");
    q.try_acquire(a.clone(), 10);
    q.release(a.clone(), 6);
    q.release(a.clone(), 4);
    let snap = q.snapshot();
    assert_eq!(snap.accounting.get(&a).unwrap().granted, 10);
    assert_eq!(snap.accounting.get(&a).unwrap().released, 10);
    assert_eq!(snap.total_released, 10);
}

// ---------------------------------------------------------------------------
// T-FI06 — pardon with valid token returns a Pending lease (Q4-C contract)
// ---------------------------------------------------------------------------

#[test]
fn pardon_returns_pending_lease_with_valid_token() {
    let spec = valid_spec("rescued-workstation");
    let lease = pardon(spec, "ops:phenotype:default").expect("pardon must succeed");
    assert_eq!(lease.spec.name, "rescued-workstation");
    assert_eq!(lease.state, fabric_graph::surface::LeaseState::Pending);
    assert!(lease.history.is_empty(), "new lease has no binding history");
}

// ---------------------------------------------------------------------------
// T-FI07 — pardon rejects bad tokens and bad specs (Q4-C audit guarantees)
// ---------------------------------------------------------------------------

#[test]
fn pardon_rejects_bad_token_and_bad_spec() {
    // Bad token
    let spec = valid_spec("workstation-a");
    let err = pardon(spec, "forged-token").unwrap_err();
    assert_eq!(err, PardonError::TokenRejected);

    // Bad spec (empty name)
    let bad_spec = spec_invalid();
    let err = pardon(bad_spec, "ops:phenotype:default").unwrap_err();
    match err {
        PardonError::SpecInvalid(SurfaceSpecError::EmptyName) => {}
        other => panic!("expected SpecInvalid(EmptyName), got {other:?}"),
    }
}
