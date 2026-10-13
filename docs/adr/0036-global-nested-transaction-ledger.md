# ADR 0036 — Global nested transaction history with bounded authority domains

**Status:** Proposed; SQLite authority-domain slice committed, not distributed adoption  
**Date:** 2026-10-12  
**Related:** ADR 0007 (no universal database), ADR 0015 (ShareCLI boundary), ADR 0016 (ecosystem authority), ADR 0021 (object authority), ADR 0034 (SQLite), ShareCLI ADR 0009.

## Context

Humans and software agents must participate in the same Actor parent abstraction. The transaction ledger is **one global logical nested/tree-like history**, not individual agent logs or a generic append-only activity stream. Each consequential commitment has exactly one containment parent, but work, effects, resources, results and dependencies may reference other branches.

This clarifies, but does not abolish, ADR 0007: global semantic identity and provenance do not imply a single physically centralized database or universal product authority. AgilePlus, AGSLAG, thegent, Tracera, SessionLedger, ShareCLI and Fabric retain their stated ownership boundaries. Their signed/versioned evidence may be referenced rather than imported as freely writable authoritative state.

## Decision

1. Establish persistent Scope, Actor, Intent, WorkItem, ExecutionAttempt, ConsumerSubscription, Reservation, LogicalEffect, ObjectVersion, Observation and AcceptanceDecision identities. Human and SoftwareAgent share the same Actor superclass; no implicit privilege based on subclass.
2. Enforce ancestry/parent membership in an authority domain. Child work cannot broaden effective authority or multiply resource entitlement. Shared producers remain single execution objects referenced from multiple consumer branches.
3. Serialize only competing invariants at the relevant authority boundary; do not build a world-wide lock or interpret the root as a perpetually open SQL transaction.
4. Operations may require prepare/commit reservations at multiple domains. A placement proposal is not a lease. Activation and publication must recheck grant freshness and use target-side fencing and expected object-head version.
5. Failed remote operations can have unknown outcomes. Preserve stable logical-effect keys and reconcile before retry; a workflow run ID or transport request ID is not a new effect.
6. Cancellation, revocation, physical reclamation, and acceptance are separate transactions. Deadline expiry fences authority, **not** necessarily running processes or occupied capacity.
7. Closed candidate subtransactions are conditional on parent commit. Externally observed child effects require explicit compensation/forward recovery classification. Parent acceptance seals required-child membership/version.
8. An immutable event is an attributable historical claim, not immutable truth. Contradictory later observations supersede conclusions with new events. Telemetry, proposals, verified enforcement and accepted outcomes have different evidence classes.
9. Project encrypted/secrecy-restricted payloads through access-controlled references, not universal readable immutable metadata. Offline replicas can only spend explicitly partitioned/delegated rights; no arbitrary reconciling of conflicting rights after reconnection.

### Concrete experimental implementation

`fabric-persist` migration V4 adds:
- one root per local authority domain; persistent nested scopes;
- finite physical-capacity domains and reservation/reclamation projections;
- actor-neutral append-only event records with update/delete rejection;
- an outbox inserted in the same SQLite transaction as each authoritative change.

The `transaction_ledger` module demonstrates scope creation, capacity definition, admission, and reclaim with unit tests. **It does not implement cross-host consensus, actual grant verification, complete ledger projections, lease fencing at effect targets, parent sealing, producer subscriptions or automatic OS reclamation.** An event receipt string is not independently verified by this module; callers must verify trusted adapter receipts before release. No part of this change claims end-to-end enforcement.

### Cross-repo exchange

ShareCLI uses a standalone local authority domain and may join Fabric only through an explicit adoption procedure preserving origin, grant scope and epochs. Fabric owns topology and remote placement; ShareCLI owns local admission/pressure and existing runtime. PhenoRegistry indexes the ownership and decision, not live reservations or event storage.

Existing SessionLedger operational events and Tracera evidence are referenced as separate owning domain facts, not merged into an unbounded event database.

## Falsifiers / next gate

- Native two-consumer shared execution with cancellation and supervisor restart; no duplicate physical cost or lost consumer obligation.
- Concurrent reservation writers never exceed hard capacity, and SQL event/outbox counts remain atomically consistent under crash injection.
- Expired or revoked worker cannot publish at target; reserved capacity is not released until observed stop/reclaim.
- A lost successful response does not produce a second external mutation.
- Parent completion refuses stale required-child membership.
- Simulated partition refuses duplicate spending of shared global rights; no false claims of immediate offline revocation.
- Same acceptance conditions for human and software agent with equal grants.

Benchmark a plain SQL/outbox baseline before considering custom distributed ledger storage. Keep consensus and event-metadata traffic away from interactive RT/audio paths.
