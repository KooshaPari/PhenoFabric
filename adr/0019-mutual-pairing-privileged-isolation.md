# ADR 0019 — Use mutual identity and narrow privileged helpers

**Status:** Accepted  
**Date:** 2026-08-28  
**Deciders:** Project owner and architecture maintainers  
**Supersedes:** —  
**Traceability:** `INT-P001`–`INT-P008`; see `../intent/prompt-map.yaml`

---

## Context

The system can capture/inject input, display, audio, clipboard, files, and device state, making broad daemon privilege unacceptable.

The decision must preserve the package-level goal while remaining honest about locality, distribution cost, platform constraints, and ecosystem authority.

## Decision

Pair through ephemeral key agreement and out-of-band confirmation. Store device keys in platform secret stores. Split privileged helpers by device class and use capability-bound, versioned protocols.

### Enforced rules

1. The decision is represented in schemas, APIs, and acceptance tests rather than documentation alone.
2. Any exception names the topology, reason, expiry/review date, and affected requirements.
3. The selected design must expose measurement and rollback.
4. Implementation may use different platform mechanisms while retaining the same semantics.
5. Performance claims require adversarial concurrent-load evidence.

## Alternatives considered

- **Alternative 1:** Plain HTTP secret bootstrap
- **Alternative 2:** One fleet password
- **Alternative 3:** Run UI as root/admin
- **Alternative 4:** Trust LAN address

These alternatives remain valid comparison baselines. They are not dismissed merely because the selected design is more ambitious.

## Consequences

- Compromise blast radius is reduced.
- Driver/helper packaging is more complex.
- Security review is a release gate.

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
