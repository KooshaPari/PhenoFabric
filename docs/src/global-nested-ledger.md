# Global nested transaction contract

**Proposed — not native product acceptance.**

The canonical [user intent](../../intent/000-source-prompts.md) requires one global logical recursively nested immutable history across both Human and SoftwareAgent actors. These two actor types share one Actor parent class.

The established repository boundaries remain separate: ShareCLI owns local admission and shared work; PhenoFabric owns cross-host resource placement and object epochs; PhenoRegistry indexes repository ownership.

Read the canonical [spec 027](../../specs/027-global-nested-ledger/spec.md), [ADR 0036](../adr/0036-global-nested-transaction-ledger.md), [requirements PF-FR-089–094](../../FUNCTIONAL_REQUIREMENTS.md), [genesis](../genesis/global-nested-ledger.md), [SOTA](../sota/global-nested-ledger.md), and [verification matrix](../../verification/requirements-traceability-matrix.md).

**Current state:** local SQLite scope/event/reservation/outbox experiment with unit tests. Authenticated grants, native effect enforcement, cross-host adoption, and release acceptance remain pending.
