# ADR 0015 — Keep ShareCLI standalone and integrate it as a privileged runtime client

**Status:** Accepted  
**Date:** 2026-08-28  
**Deciders:** Project owner and architecture maintainers  
**Supersedes:** —  
**Traceability:** `INT-P001`–`INT-P008`; see `../intent/prompt-map.yaml`

---

## Context

ShareCLI already has OS-adjacent process discovery, coalescing, FUSE, mesh, thermal, supervision, health, and telemetry value outside this product.

The decision must preserve the package-level goal while remaining honest about locality, distribution cost, platform constraints, and ecosystem authority.

## Decision

Do not absorb ShareCLI. Define adapters through which it supplies process/resource signals and executes delegated local actions. The fabric owns cross-device placement and interactive I/O; ShareCLI owns its standalone local runtime/supervisor contract.

### Enforced rules

1. The decision is represented in schemas, APIs, and acceptance tests rather than documentation alone.
2. Any exception names the topology, reason, expiry/review date, and affected requirements.
3. The selected design must expose measurement and rollback.
4. Implementation may use different platform mechanisms while retaining the same semantics.
5. Performance claims require adversarial concurrent-load evidence.

## Alternatives considered

- **Alternative 1:** Rename the entire fabric ShareCLI
- **Alternative 2:** Fork/reimplement ShareCLI features
- **Alternative 3:** Make ShareCLI depend on all fabric services

These alternatives remain valid comparison baselines. They are not dismissed merely because the selected design is more ambitious.

## Consequences

- Independent adoption remains credible.
- Overlap requires explicit delegation.
- Shared crates/protocols may be extracted if proven.

### Positive

- The architecture remains internally coherent.
- Product and evidence boundaries are explicit.
- The implementation can be evaluated against rejected alternatives.

### Negative / cost

- More contracts and measurement infrastructure are required.
- Some conveniences are delayed until platform-specific safety and rollback exist.
- The selected approach may be superseded if benchmark or adoption evidence contradicts it.

## Falsification and review

This ADR must be reviewed when:

- a vertical slice shows the alternative is materially simpler or faster;
- a platform API/security change invalidates the mechanism;
- coordination overhead exceeds the benefit in the target scenarios;
- product/repository ownership changes;
- the user-facing workflow cannot remain simple.

## References

- [`../PRD.md`](../PRD.md)
- [`../HLD.md`](../HLD.md)
- [`../LLD.md`](../LLD.md)
- [`../intent/000-source-prompts.md`](../intent/000-source-prompts.md)
- [`../verification/acceptance-gates.md`](../verification/acceptance-gates.md)
