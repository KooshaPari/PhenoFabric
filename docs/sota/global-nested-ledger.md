# SOTA and competing approaches — global nested transactional actor execution

**Status:** candidate evaluation, **not** exhaustive validated coverage. Source review does not prove adoption or native effectiveness.

| Reference | Candidate mechanism | Where it could fail | Required local comparison |
|---|---|---|---|
| [Event Sourcing pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing) | immutable history & projected state | after-the-fact events do not enforce authorization | crash-consistent state+event+outbox |
| [PostgreSQL serializability](https://www.postgresql.org/docs/current/transaction-iso.html) | conflicting resource invariants | does not atomically govern OS/process/provider effects | local SQL authority baseline |
| [SQLite transactions](https://sqlite.org/lang_transaction.html) | embedded per-domain atomicity | no cross-host consensus by itself | existing Fabric V4 migration |
| [Temporal Activities](https://docs.temporal.io/activity-definition) | persistent workflow retries | duplicate external effects unless idempotent | stable logical-effect tests |
| [Google Chubby](https://research.google.com/archive/chubby-osdi06.pdf) | fencing tokens | stale executor accepted by effect target | delayed-writer publication fixture |
| [I-confluence](https://arxiv.org/abs/1402.2237) | coordinate only when invariant requires | wrong domain ownership assumption | competing delegated reservations |
| [Certificate Transparency RFC 9162](https://www.rfc-editor.org/rfc/rfc9162.html) | tamper-evident append-only history | false assertions still appendable; split views | evidence correctness/fork detection |

**Differentiation hypothesis:** actor-neutral tree history plus resource and authority-preserving work transformations across standalone and distributed placement. Must beat a simpler queue/SQL/effect-broker implementation in accepted useful outcomes, correctness and operational cost—not novelty claims. See [verification](../../verification/requirements-traceability-matrix.md).
