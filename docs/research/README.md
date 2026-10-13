# Research Set

- [Research program](research-program.md)
- [Hypotheses and falsification](hypotheses.md)
- [Experiment catalog](experiments.md)
- [Open questions](open-questions.md)
- [Source register](source-register.md)
- [Literature/prior-art map](literature-map.md)
- [Graduation gates](graduation-gates.md)

The research program exists to narrow claims. The product can ship useful workspaces, routes and explicit workload placement while atomic interposition or advanced memory-fabric work remains rejected or experimental.

---

## 2026-10-12 global nested immutable transactions

[Genesis](../genesis/global-nested-ledger.md) · [SOTA candidate comparison](../sota/global-nested-ledger.md) · [INT-P009..012 verbatim](../../intent/000-source-prompts.md) · [spec 027](../../specs/027-global-nested-ledger/spec.md).

Competing baselines are an ordinary SQL transactional domain with outbox and effect brokers, queue/after-the-fact logs, long-running workflow orchestration, and a custom distributed ledger. No exhaustive SOTA or throughput claim is established by the current code. Evaluate against native fault/cancellation/epoch/partition tests.
