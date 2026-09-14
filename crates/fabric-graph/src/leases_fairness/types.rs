use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A tenant's identifier. Opaque string, e.g. "ui-window:abc123" or
/// "agent:workspace-1". Stable for the lifetime of the tenant's
/// participation in the queue.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TenantId(pub String);

impl TenantId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// FairnessPolicy
// ---------------------------------------------------------------------------

/// How capacity is divided across tenants.
///
/// New variants may be added in minor releases; existing variants are
/// never repurposed (Stable per ADR-0027).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FairnessPolicy {
    /// Round-robin, no weights. First-come-first-served.
    Fifo,
    /// Per-tenant weight; pick the tenant with the largest deficit first.
    /// Ties broken by FIFO insertion order.
    FairShare { weight: u32 },
    /// Lower `priority` number = higher priority. Within the same priority,
    /// FIFO order applies.
    PriorityWeighted { priority: u8 },
    /// Round-robin scaled by per-tenant `weight`. Each tenant gets
    /// `weight` slots per full rotation.
    WeightedRoundRobin { weight: u32 },
}

impl FairnessPolicy {
    /// The weight this policy assigns a tenant that hasn't explicitly
    /// configured one. 1 is a sensible default for share-based policies.
    pub fn default_weight(&self) -> u32 {
        1
    }
}

// ---------------------------------------------------------------------------
// FairnessDecision
// ---------------------------------------------------------------------------

/// Outcome of `FairnessQueue::try_acquire`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FairnessDecision {
    Granted {
        tenant: TenantId,
        granted_weight: u32,
        /// The tenant's deficit after this grant. Should decrease or stay
        /// at zero for healthy operation; large positive values indicate
        /// the tenant is consistently starved.
        deficit_after: i64,
    },
    Denied {
        tenant: TenantId,
        reason: DenyReason,
        current_deficit: i64,
    },
}

/// Why a `try_acquire` request was denied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DenyReason {
    /// Tenant has been granted more than they requested across their
    /// lifetime. Deficit has gone negative.
    OverAllotment,
    /// Priority-based denial: a higher-priority tenant is currently waiting.
    LowerPriority {
        blocking: TenantId,
        blocking_priority: u8,
    },
    /// The queue is at capacity and this tenant already has a slice.
    QueueFull,
}

// ---------------------------------------------------------------------------
// TenantAccounting
// ---------------------------------------------------------------------------

/// Per-tenant accounting state. Public for snapshotting and audit logs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantAccounting {
    /// Total weight granted to this tenant over its lifetime.
    pub granted: u64,
    /// Total weight released back to the queue.
    pub released: u64,
    /// `max(0, requested - granted)` for share-based policies. May go
    /// negative if a tenant was over-allotted.
    pub deficit: i64,
    /// Tenant's priority (0 = highest; u8::MAX = lowest). Populated by
    /// `try_acquire` when the tenant first appears.
    pub priority: u8,
}

// ---------------------------------------------------------------------------
// FairnessSnapshot
// ---------------------------------------------------------------------------

/// A point-in-time snapshot of the fairness queue. Designed for the audit
/// log that spec 020 §3 reserves ("emits events but does not own persistence").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FairnessSnapshot {
    pub policy: FairnessPolicy,
    pub accounting: BTreeMap<TenantId, TenantAccounting>,
    pub total_granted: u64,
    pub total_released: u64,
    /// Tenants in current rotation order (Fifo + WeightedRoundRobin only).
    pub rotation: Vec<TenantId>,
}
