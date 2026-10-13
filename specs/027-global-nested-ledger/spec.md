# Spec 027 — Global nested immutable transaction contract (proposed)

**Provenance:** [verbatim INT-P009..012](../../intent/000-source-prompts.md); [ADR 0036](../../docs/adr/0036-global-nested-transaction-ledger.md); [FRs PF-FR-089..094](../../FUNCTIONAL_REQUIREMENTS.md).

## Purpose

Make ShareCLI and PhenoFabric participate in **one global logical recursively nested transaction history** whose Actor parent includes humans and software agents. Avoid a separate journal for every agent, a universal product database, and implicit permissions derived from actor type.

## Domain model

`Actor { id, kind: Human|SoftwareAgent, claims/grants_by_reference }` is separate from role and scoped effective grants. Each transaction/Scope has one canonical parent and versioned required-child membership. References cross branches for Producer, WorkItem, ResourceDomain, LogicalEffect, ObjectVersion, Evidence and ConsumerSubscription. A permanent scope/root is not one forever-open SQL transaction.

A delegated grant must constrain scope/effect/epoch and attenuate its parent. Dividing work among child actors cannot duplicate parent budgets; one physical producer may satisfy several independent consumer commitments only under certified equivalence and separate output permission.

## Proposed operation protocol

1. Record user/agent intent and authoritative parent scope.
2. Authenticate actor and scoped grant with expiry/revocation/epoch check; classify uncertain effects.
3. Propose transformation; verify equivalence, permission, resource and temporal constraints.
4. Atomically reserve capacity, append the authoritative change/event and outbox in the relevant authority domain.
5. Dispatch via a trusted effect broker; record attempted/observed outcomes independently.
6. Verify actual outputs/physical reclamation; require recipient-side fencing for publication.
7. Accept against the current required-child membership revision; otherwise revise, compensate or mark unknown.

### Failure semantics

Cancelled subscription ≠ producer stop. Revoked grant ≠ consumer no longer interested. Lease expiry ≠ physical reclamation. Lost acknowledgement ≠ no external effect. An externally committed child cannot be universally rolled back by a parent abort. An immutable event is historical evidence and can be superseded by subsequent contradictory observations.

## Interfaces (provisional)

`ScopeRef`, `ActorRef`, `GrantRef`, `ConsumerRef`, `EffectKey`, `ResourceReservationRef`, `VersionedObjectRef`, `Epoch`, `DecisionReceipt`. Origin/product, scope ancestry, trust zone and schema version must be explicit; no secret payloads in universally readable event metadata.

**Existing local experimental implementation:** `crates/fabric-persist/src/transaction_ledger.rs` and SQLite migration V4 (scope creation, capacity, reservation, reclamation reference, append-only event + outbox). It does **not** independently verify grants or reclamation receipts, provide cross-host consensus, or perform live ShareCLI supervisor integration.

## Acceptance / negative oracles

| ID | Expected observable outcome | Counterexample |
|---|---|---|
| PF-AT-TXN-01 | Human and SoftwareAgent with equal effective grants receive identical action decisions | ActorKind changes permission without explicit policy |
| PF-AT-TXN-02 | Child delegation consumes parent resource budget and does not multiply capacity | 100 children reserve 100× |
| PF-AT-TXN-03 | Cancel A preserves B's valid qualified shared producer | stale empty-consumer shutdown |
| PF-AT-TXN-04 | Unknown effect reconciled/deduped before retry | success response lost, duplicate side effect |
| PF-AT-TXN-05 | Stale epoch rejected at destination | old worker publishes after migration |
| PF-AT-TXN-06 | Parent acceptance requires current sealed child set | required child added while accept commits |
| PF-AT-TXN-07 | Local reserved projection/event/outbox consistent after crash | projection without event or event without reservation |
| PF-AT-TXN-08 | Standalone root adoption does not duplicate delegated rights | partitioned hosts each spend original allowance |

**Release state:** all acceptance IDs remain unverified at product level. Unit tests exercise small subsets. Results must cite exact checkout, platform, toolchain and evidence class.
