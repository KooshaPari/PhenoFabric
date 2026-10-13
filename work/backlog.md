# Initial Backlog

The backlog is ordered by information value and adoption value, not by architectural glamour.

## Now

1. PF-WP-000 documentation and governance checks.
2. PF-WP-010 capability descriptors on the exact main PC, VMs, C27HG70 and M1 Pro.
3. PF-WP-170 calibrated latency/audio/video labs.
4. PF-WP-020 graph transaction and lease core.
5. PF-WP-030 central Linux input broker.
6. PF-WP-040 Looking Glass/SPICE/virtio route family.
7. PF-WP-050 minimal workspace/focus shell.

## Next

- Windows/macOS endpoints and MacBook display sink.
- Wired LAN transport and independent audio/input channels.
- Protected Ableton and game modes.
- Agent realm publication.
- Object plane and explicit TaskSpec placement.

## Later / research-gated

- Arbitrary native pixel-proxy windows.
- Cross-node DSP plugin placement beyond proven budget.
- Syscall/FUSE/function interposition.
- Learned placement policies.
- CXL/RDMA/GPUDirect-specific backends beyond available hardware.
- Process checkpoint and VM live migration adapters.

---

## Proposed PF-WP-TXN-027 (2026-10-12)

Source INT-P009..012, [spec 027](../specs/027-global-nested-ledger/spec.md), and [proposed FR-089..094](../FUNCTIONAL_REQUIREMENTS.md). Candidate local SQLite scopes/events/outbox exist; no permission-enforcing native integration.

**Now:** exact-branch CI, source-provenance and requirements/spec link validation. **Next:** authenticated grants, effect-target fencing, independently verified physical reclaim and native two-consumer/share/cancel/restart fixture. **Later (research-gated):** partition-safe offline rights, global logical adoption and performance comparison vs straightforward SQL/outbox/queue baselines.

No supersession of prior PF-WP IDs; task IDs TXN27-01..10 are listed in [spec tasks](../specs/027-global-nested-ledger/tasks.md).
