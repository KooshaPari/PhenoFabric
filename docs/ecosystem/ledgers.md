# Ledger Integrations

| Ledger | Fabric publishes | Fabric consumes | Authority rule |
|---|---|---|---|
| SessionLedger | run/session IDs, stage summaries, logs/artifact bundles, route lifecycle | prior run context and replay references | SessionLedger owns durable session history; Fabric owns current execution state. |
| ResearchLedger | experiment runs, source/version fingerprints, result refs | claims, sources, confidence and open questions | ResearchLedger owns research knowledge; Fabric owns reproduced runtime measurements. |
| RepoLedger | build/test/runtime associations, repo/artifact hashes | repo identity, language/toolchain, cache hints | RepoLedger owns repository inventory; Fabric may cache it. |
| hwLedger | utilization/capability evidence and failure observations | asset identity, acquisition/cost/repair history | hwLedger owns hardware capital record; Fabric probes live capability. |

## Shared identifiers

Identifiers are namespaced and globally unique enough for cross-reference. An event includes source product, schema version, subject ID, causation/correlation IDs, occurred/observed timestamps, content hash and evidence URI. A ledger can reject, reconcile or supersede events without writing Fabric’s store.

---

## The global nested transaction model is not another product ledger

User-confirmed 2026-10-12 contract: one logical global nested immutable history across Human/SoftwareAgent actors, supporting local ShareCLI and distributed Fabric work. A global relationship model links authoritative facts from distinct product stores. Neither `fabric-research-ledger` nor ShareCLI's compactable terminal-session observation log is automatically the shared transactional authority.

**Separation:** one owning scope per commitment and explicit cross-branch references; Fabric's local SQLite `fabric-persist` slice owns only its authority-domain reservations/commit/outbox. SessionLedger still owns operational session history, Tracera still owns evidence/product relationships, and PhenoRegistry owns the inventory/owner index. No product silently obtains full write authority to another product's state.

**Next protocol:** explicit schema/version, actor/grant and origin identity, effect/correlation key, epoch, parent/ref links, visibility/consent classification, idempotency/reclaim evidence. Native target effect enforcement and offline adoption are open.
