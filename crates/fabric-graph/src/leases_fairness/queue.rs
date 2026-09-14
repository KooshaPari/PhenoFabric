use std::collections::{BTreeMap, VecDeque};

use super::types::*;

// ---------------------------------------------------------------------------
// FairnessQueue
// ---------------------------------------------------------------------------

/// The fairness queue itself. Tracks per-tenant accounting across many
/// `try_acquire` / `release` cycles. Single-instance only — distributed
/// fairness is out of scope (spec 022 §2).
pub struct FairnessQueue {
    policy: FairnessPolicy,
    accounting: BTreeMap<TenantId, TenantAccounting>,
    /// FIFO insertion order for tie-breaking (FairShare) and priority-group
    /// FIFO (PriorityWeighted).
    insertion_order: Vec<TenantId>,
    /// Active rotation for Fifo and WeightedRoundRobin policies.
    rotation: VecDeque<TenantId>,
    /// Slots remaining for the current front of rotation (WRR only).
    wrr_remaining: BTreeMap<TenantId, u32>,
    total_granted: u64,
    total_released: u64,
}

impl FairnessQueue {
    /// Construct a new queue with the given policy. Empty until the first
    /// `try_acquire` call.
    pub fn new(policy: FairnessPolicy) -> Self {
        Self {
            policy,
            accounting: BTreeMap::new(),
            insertion_order: Vec::new(),
            rotation: VecDeque::new(),
            wrr_remaining: BTreeMap::new(),
            total_granted: 0,
            total_released: 0,
        }
    }

    /// The active policy.
    pub fn policy(&self) -> &FairnessPolicy {
        &self.policy
    }

    /// Try to acquire `weight` units of capacity for `tenant`.
    /// Returns a `FairnessDecision` — never panics, never blocks.
    pub fn try_acquire(&mut self, tenant: TenantId, weight: u32) -> FairnessDecision {
        // Defensive: a zero-weight request is a no-op.
        if weight == 0 {
            let deficit = self.accounting_for(&tenant).deficit;
            return FairnessDecision::Granted {
                tenant,
                granted_weight: 0,
                deficit_after: deficit,
            };
        }

        // Register tenant if first appearance.
        if !self.accounting.contains_key(&tenant) {
            self.register_tenant(&tenant);
        }

        match &self.policy {
            FairnessPolicy::Fifo => self.try_acquire_fifo(tenant, weight),
            FairnessPolicy::FairShare { .. } => self.try_acquire_fair_share(tenant, weight),
            FairnessPolicy::PriorityWeighted { priority } => {
                let priority = *priority;
                self.try_acquire_priority(tenant, weight, priority)
            }
            FairnessPolicy::WeightedRoundRobin { .. } => self.try_acquire_wrr(tenant, weight),
        }
    }

    /// Release `weight` units of capacity back to the queue from `tenant`.
    /// Updates accounting + totals. If `weight > tenant.granted`, the
    /// over-release is silently capped (deficit can go more negative).
    pub fn release(&mut self, tenant: TenantId, weight: u32) {
        if !self.accounting.contains_key(&tenant) {
            self.register_tenant(&tenant);
        }
        let acct = self.accounting.get_mut(&tenant).expect("just registered");
        acct.released = acct.released.saturating_add(weight as u64);
        // Releasing decreases effective deficit (tenant returned capacity).
        acct.deficit = acct.deficit.saturating_sub(weight as i64);
        self.total_released = self.total_released.saturating_add(weight as u64);
    }

    /// Point-in-time snapshot for audit logging.
    pub fn snapshot(&self) -> FairnessSnapshot {
        FairnessSnapshot {
            policy: self.policy.clone(),
            accounting: self.accounting.clone(),
            total_granted: self.total_granted,
            total_released: self.total_released,
            rotation: self.rotation.iter().cloned().collect(),
        }
    }

    /// Operator override: set a registered tenant's priority value. Panics
    /// if the tenant is not registered (use `register_tenant` first or
    /// register via `try_acquire`).
    pub fn set_priority(&mut self, tenant: &TenantId, priority: u8) {
        let acct = self
            .accounting
            .get_mut(tenant)
            .expect("set_priority: tenant must be registered first");
        acct.priority = priority;
    }

    // ---- private ----

    fn accounting_for(&self, tenant: &TenantId) -> TenantAccounting {
        self.accounting
            .get(tenant)
            .cloned()
            .unwrap_or(TenantAccounting {
                granted: 0,
                released: 0,
                deficit: 0,
                priority: u8::MAX,
            })
    }

    fn register_tenant(&mut self, tenant: &TenantId) {
        let priority = match &self.policy {
            FairnessPolicy::PriorityWeighted { priority } => *priority,
            _ => u8::MAX,
        };
        self.accounting.insert(
            tenant.clone(),
            TenantAccounting {
                granted: 0,
                released: 0,
                deficit: 0,
                priority,
            },
        );
        self.insertion_order.push(tenant.clone());
        // For Fifo + WRR, the tenant joins the rotation.
        match &self.policy {
            FairnessPolicy::Fifo | FairnessPolicy::WeightedRoundRobin { .. } => {
                if !self.rotation.contains(tenant) {
                    self.rotation.push_back(tenant.clone());
                }
            }
            _ => {}
        }
    }

    fn grant(&mut self, tenant: &TenantId, weight: u32) -> FairnessDecision {
        let acct = self.accounting.get_mut(tenant).expect("must exist");
        acct.granted = acct.granted.saturating_add(weight as u64);
        acct.deficit = acct.deficit.saturating_sub(weight as i64);
        self.total_granted = self.total_granted.saturating_add(weight as u64);
        FairnessDecision::Granted {
            tenant: tenant.clone(),
            granted_weight: weight,
            deficit_after: acct.deficit,
        }
    }

    fn deny(&self, tenant: &TenantId, reason: DenyReason) -> FairnessDecision {
        FairnessDecision::Denied {
            tenant: tenant.clone(),
            reason,
            current_deficit: self.accounting_for(tenant).deficit,
        }
    }

    fn try_acquire_fifo(&mut self, tenant: TenantId, weight: u32) -> FairnessDecision {
        // Fifo allows any request (the rotation just orders them).
        // Refuse if there's already a higher-priority waiting tenant (there
        // isn't in Fifo, but defense in depth).
        self.grant(&tenant, weight)
    }

    fn try_acquire_fair_share(
        &mut self,
        tenant: TenantId,
        weight: u32,
    ) -> FairnessDecision {
        // For FairShare, "try_acquire(tenant, weight)" actually means
        // "record this request and serve the most-deficit tenant next".
        // We honor the call: bump the requesting tenant's deficit, then
        // decide who's served based on the new deficit ranking.
        let acct = self.accounting.get_mut(&tenant).expect("must exist");
        acct.deficit = acct.deficit.saturating_add(weight as i64);
        drop(acct);

        // Pick the tenant with the max deficit. Ties broken by FIFO.
        let pick = self
            .accounting
            .iter()
            .max_by(|(a_id, a), (b_id, b)| {
                a.deficit
                    .cmp(&b.deficit)
                    .then_with(|| {
                        // earlier insertion order = smaller index = lower
                        let ai = self
                            .insertion_order
                            .iter()
                            .position(|x| x == *a_id)
                            .unwrap_or(usize::MAX);
                        let bi = self
                            .insertion_order
                            .iter()
                            .position(|x| x == *b_id)
                            .unwrap_or(usize::MAX);
                        ai.cmp(&bi)
                    })
                    .then_with(|| a_id.cmp(b_id))
            })
            .map(|(id, _)| id.clone());

        match pick {
            Some(picked) if picked == tenant => {
                // The caller is the most-deficit tenant — grant them and
                // clear their deficit for this round.
                let acct = self.accounting.get_mut(&picked).expect("must exist");
                let granted = acct.deficit.max(0) as u32;
                let weight = if weight == 0 { 0 } else { granted.max(weight) };
                let decision = self.grant(&picked, weight);
                // Zero out the granted portion of the deficit.
                let acct = self.accounting.get_mut(&picked).expect("must exist");
                if weight as i64 <= acct.deficit {
                    acct.deficit -= weight as i64;
                } else {
                    acct.deficit = 0;
                }
                decision
            }
            Some(picked) => {
                // Someone else has higher deficit — refuse this request.
                let blocking_priority = self.accounting_for(&picked).priority;
                self.deny(
                    &tenant,
                    DenyReason::LowerPriority {
                        blocking: picked,
                        blocking_priority,
                    },
                )
            }
            None => self.grant(&tenant, weight),
        }
    }

    fn try_acquire_priority(
        &mut self,
        tenant: TenantId,
        weight: u32,
        policy_priority: u8,
    ) -> FairnessDecision {
        // The policy_priority is the default for newly-registered tenants;
        // a tenant's actual priority may have been changed since
        // registration. Always use the stored priority for comparison.
        let tenant_priority = self.accounting_for(&tenant).priority;

        // Find the lowest-priority value across all tenants; this tenant
        // can only be served if its priority equals that minimum.
        let min_priority = self
            .accounting
            .values()
            .map(|a| a.priority)
            .min()
            .unwrap_or(policy_priority);

        if tenant_priority > min_priority {
            // Find the highest-priority blocking tenant (lowest number).
            let blocker = self
                .accounting
                .iter()
                .filter(|(id, a)| a.priority == min_priority && *id != &tenant)
                .min_by(|(a_id, _), (b_id, _)| {
                    let ai = self
                        .insertion_order
                        .iter()
                        .position(|x| x == *a_id)
                        .unwrap_or(usize::MAX);
                    let bi = self
                        .insertion_order
                        .iter()
                        .position(|x| x == *b_id)
                        .unwrap_or(usize::MAX);
                    ai.cmp(&bi)
                })
                .map(|(id, _)| id.clone());
            return match blocker {
                Some(blocking) => self.deny(
                    &tenant,
                    DenyReason::LowerPriority {
                        blocking,
                        blocking_priority: min_priority,
                    },
                ),
                None => self.grant(&tenant, weight),
            };
        }

        // Within same priority group, FIFO order: if another tenant of
        // equal priority was registered earlier, defer to them.
        let same_priority_earlier = self
            .accounting
            .iter()
            .filter(|(id, a)| a.priority == tenant_priority && *id != &tenant)
            .any(|(id, _)| {
                let ti = self
                    .insertion_order
                    .iter()
                    .position(|x| x == &tenant)
                    .unwrap_or(usize::MAX);
                let oi = self
                    .insertion_order
                    .iter()
                    .position(|x| x == id)
                    .unwrap_or(usize::MAX);
                oi < ti
            });
        if same_priority_earlier {
            return self.deny(&tenant, DenyReason::QueueFull);
        }
        self.grant(&tenant, weight)
    }

    fn try_acquire_wrr(&mut self, tenant: TenantId, weight: u32) -> FairnessDecision {
        // For WRR, the rotation slot count comes from policy.weight, not
        // the call arg (the call arg is the requested weight, not slot
        // budget). Initial registration primes a tenant with `policy.weight`
        // slots; each grant consumes 1 slot.
        let policy_weight = match self.policy {
            FairnessPolicy::WeightedRoundRobin { weight: pw } => pw,
            _ => 1,
        };

        // Pop from front of rotation if current front has slots remaining;
        // otherwise skip it (its slots are exhausted). Refill rotation when
        // empty.
        if self.rotation.is_empty() {
            self.rotation.push_back(tenant.clone());
            self.wrr_remaining.insert(tenant.clone(), policy_weight.max(1));
        } else if !self.rotation.contains(&tenant) {
            // New tenant joins rotation with the policy's slot budget.
            self.rotation.push_back(tenant.clone());
            self.wrr_remaining.insert(tenant.clone(), policy_weight.max(1));
        }

        let target = self.rotation.front().cloned();
        match target {
            Some(front) if front == tenant => {
                // Caller is the current rotation front; consume one slot.
                // Compute the consumption + grant decision FIRST, then update
                // the slot map (avoids borrow-checker conflict on &mut self).
                let current_slots = self
                    .wrr_remaining
                    .get(&tenant)
                    .copied()
                    .unwrap_or(policy_weight.max(1));
                let consume = 1u32.min(current_slots);
                // For WRR, only consume 1 slot per grant; the policy
                // granted-weight is the slot count, but each grant is `weight`
                // (the caller-requested amount).
                let grant_weight = weight.max(1);
                let decision = self.grant(&tenant, grant_weight);
                // Now update slot tracking (no overlap with `grant` borrows).
                if consume >= current_slots {
                    self.wrr_remaining.remove(&tenant);
                    self.rotation.pop_front();
                } else {
                    self.wrr_remaining
                        .insert(tenant.clone(), current_slots - consume);
                }
                decision
            }
            Some(front) => self.deny(
                &tenant,
                DenyReason::LowerPriority {
                    blocking: front,
                    blocking_priority: 0,
                },
            ),
            None => self.grant(&tenant, weight),
        }
    }
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------- T-F01 ----------------

    #[test]
    fn fifo_round_robin() {
        let mut q = FairnessQueue::new(FairnessPolicy::Fifo);
        let a = TenantId::new("a");
        let b = TenantId::new("b");
        let c = TenantId::new("c");
        // First call from each tenant registers them.
        let r1 = q.try_acquire(a.clone(), 1);
        let r2 = q.try_acquire(b.clone(), 1);
        let r3 = q.try_acquire(c.clone(), 1);
        // All should be granted (Fifo has no contention).
        assert!(matches!(r1, FairnessDecision::Granted { .. }));
        assert!(matches!(r2, FairnessDecision::Granted { .. }));
        assert!(matches!(r3, FairnessDecision::Granted { .. }));
        // Snap shot shows all three with granted=1.
        let snap = q.snapshot();
        assert_eq!(snap.total_granted, 3);
        assert_eq!(snap.accounting.get(&a).unwrap().granted, 1);
        assert_eq!(snap.accounting.get(&b).unwrap().granted, 1);
        assert_eq!(snap.accounting.get(&c).unwrap().granted, 1);
    }

    // ---------------- T-F02 ----------------

    #[test]
    fn fair_share_equal_grants() {
        let mut q = FairnessQueue::new(FairnessPolicy::FairShare { weight: 3 });
        let a = TenantId::new("a");
        let b = TenantId::new("b");
        let c = TenantId::new("c");
        // Register A first, then ask for 3.
        q.try_acquire(a.clone(), 3);
        // Now B is at max deficit (3 vs A's 0 after grant).
        let r2 = q.try_acquire(b.clone(), 3);
        assert!(matches!(r2, FairnessDecision::Granted { .. }));
        // Now C.
        let r3 = q.try_acquire(c.clone(), 3);
        assert!(matches!(r3, FairnessDecision::Granted { .. }));
        // All three should have been granted 3.
        let snap = q.snapshot();
        for t in [&a, &b, &c] {
            assert_eq!(snap.accounting.get(t).unwrap().granted, 3);
        }
        assert_eq!(snap.total_granted, 9);
    }

    // ---------------- T-F03 ----------------

    #[test]
    fn priority_skips_higher_priority_tenant() {
        let mut q = FairnessQueue::new(FairnessPolicy::PriorityWeighted { priority: 1 });
        let a = TenantId::new("a");
        let r1 = q.try_acquire(a.clone(), 5);
        assert!(matches!(r1, FairnessDecision::Granted { .. }));

        // Register B first (any weight >= 1 registers), then bump its
        // priority value to 2 (lower priority) and try to acquire a
        // large chunk. B should be denied — A is at priority 1.
        let b = TenantId::new("b");
        let _ = q.try_acquire(b.clone(), 1); // registers B at policy priority 1
        q.set_priority(&b, 2);
        let r2 = q.try_acquire(b.clone(), 5);
        match r2 {
            FairnessDecision::Denied { reason, .. } => match reason {
                DenyReason::LowerPriority { blocking, .. } => {
                    assert_eq!(blocking, a);
                }
                other => panic!("expected LowerPriority, got {other:?}"),
            },
            other => panic!("expected Denied, got {other:?}"),
        }
    }

    // ---------------- T-F04 ----------------

    #[test]
    fn wrr_weighted_slots() {
        let mut q = FairnessQueue::new(FairnessPolicy::WeightedRoundRobin { weight: 2 });
        let a = TenantId::new("a");
        let b = TenantId::new("b");
        // First call from A — registers, goes to rotation front.
        let r1 = q.try_acquire(a.clone(), 1);
        assert!(matches!(r1, FairnessDecision::Granted { .. }));
        // B tries to acquire — should be denied (not at front).
        let r2 = q.try_acquire(b.clone(), 1);
        assert!(matches!(r2, FairnessDecision::Denied { .. }));
        // A's slot count: starts at 2, decremented to 1 by the first call.
        // A acquires again — slot goes to 0, A leaves rotation, B takes over.
        let r3 = q.try_acquire(a.clone(), 1);
        assert!(matches!(r3, FairnessDecision::Granted { .. }));
        // Now B can acquire.
        let r4 = q.try_acquire(b.clone(), 1);
        assert!(matches!(r4, FairnessDecision::Granted { .. }));
    }

    // ---------------- T-F05 ----------------

    #[test]
    fn release_updates_accounting() {
        let mut q = FairnessQueue::new(FairnessPolicy::Fifo);
        let a = TenantId::new("a");
        q.try_acquire(a.clone(), 5);
        q.release(a.clone(), 3);
        let snap = q.snapshot();
        let acct = snap.accounting.get(&a).unwrap();
        assert_eq!(acct.granted, 5);
        assert_eq!(acct.released, 3);
        assert_eq!(snap.total_released, 3);
    }

    // ---------------- T-F06 ----------------

    #[test]
    fn snapshot_serde_round_trip() {
        let mut q = FairnessQueue::new(FairnessPolicy::FairShare { weight: 5 });
        let a = TenantId::new("alpha");
        let b = TenantId::new("beta");
        q.try_acquire(a.clone(), 2);
        q.try_acquire(b.clone(), 3);
        q.release(a.clone(), 1);
        let snap = q.snapshot();
        let json = serde_json::to_string(&snap).expect("serialize");
        let back: FairnessSnapshot = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, snap);
    }
}
