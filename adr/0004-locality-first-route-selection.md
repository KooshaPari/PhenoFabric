# ADR 0004 — Search locality tiers before remote transports

**Status:** Accepted  
**Date:** 2026-08-28  
**Deciders:** Project owner and architecture maintainers  
**Supersedes:** —  
**Traceability:** `INT-P001`–`INT-P008`; see `../intent/prompt-map.yaml`

---

## Context

Same-host streaming through a codec/network stack wastes latency, memory bandwidth, GPU engines, and power when shared-memory/device paths exist.

The decision must preserve the package-level goal while remaining honest about locality, distribution cost, platform constraints, and ecosystem authority.

## Decision

The route compiler searches L0 same-process through L8 OOB in order of physical cost, prunes by constraints, then scores valid candidates. Tier order is a heuristic, not an absolute rule.

### Enforced rules

1. The decision is represented in schemas, APIs, and acceptance tests rather than documentation alone.
2. Any exception names the topology, reason, expiry/review date, and affected requirements.
3. The selected design must expose measurement and rollback.
4. Implementation may use different platform mechanisms while retaining the same semantics.
5. Performance claims require adversarial concurrent-load evidence.

## Alternatives considered

- **Alternative 1:** Always use one network protocol
- **Alternative 2:** Backend manually pinned forever
- **Alternative 3:** Encode all surfaces for uniformity
- **Alternative 4:** Treat loopback as free

These alternatives remain valid comparison baselines. They are not dismissed merely because the selected design is more ambitious.

## Consequences

- Same-host paths can use direct/shared/DMA mechanisms.
- WAN paths remain codec/congestion aware.
- Topology probing and explanations are required.

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
