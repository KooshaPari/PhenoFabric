# ADR 0009 — Prefer shared memory and device DMA before compression

**Status:** Accepted  
**Date:** 2026-08-28  
**Deciders:** Project owner and architecture maintainers  
**Supersedes:** —  
**Traceability:** `INT-P001`–`INT-P008`; see `../intent/prompt-map.yaml`

---

## Context

Looking Glass/KVMFR demonstrates that same-host VFIO presentation can avoid compression and color conversion.

The decision must preserve the package-level goal while remaining honest about locality, distribution cost, platform constraints, and ecosystem authority.

## Decision

For eligible same-OS and same-host paths, use direct references, SHM, DMA-BUF, IVSHMEM/KVMFR, virtio/vhost, or device handles before codec stages. Codec paths are fallback when isolation/format/distance requires them.

### Enforced rules

1. The decision is represented in schemas, APIs, and acceptance tests rather than documentation alone.
2. Any exception names the topology, reason, expiry/review date, and affected requirements.
3. The selected design must expose measurement and rollback.
4. Implementation may use different platform mechanisms while retaining the same semantics.
5. Performance claims require adversarial concurrent-load evidence.

## Alternatives considered

- **Alternative 1:** Always AV1/HEVC
- **Alternative 2:** CPU copies through generic IPC
- **Alternative 3:** Use standard IVSHMEM only
- **Alternative 4:** Physical capture for all VMs

These alternatives remain valid comparison baselines. They are not dismissed merely because the selected design is more ambitious.

## Consequences

- Lower latency and resource use are possible.
- Buffer ownership and fences become security/correctness boundaries.
- Zero-copy is claimed only when measured end to end.

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
