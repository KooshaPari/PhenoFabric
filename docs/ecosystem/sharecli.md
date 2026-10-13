# ShareCLI Integration

## Current identity

ShareCLI remains a standalone OS/kernel-adjacent agent runtime and declarative process supervisor. Its current public description includes process discovery, CPU/memory/network/FD and syscall-relevant I/O observation, speculative coalescing, FUSE I/O sharing, an agent mesh, thermal gating, hot reload, health, metrics and plugins.

## Relationship

```text
ShareCLI                           Phenotype Fabric
---------                          ----------------
process/service intent  ────────▶  TaskSpec / RegionSpec
host pressure telemetry ────────▶  resource pressure/cost model
FUSE/coalesce evidence  ────────▶  research and object-placement hints
process lifecycle       ◀────────  selected node/realm execution adapter
health/logs             ────────▶  SessionLedger/Tracera refs
```

## Non-collapse rule

ShareCLI must remain useful without Fabric. A simple `sharecli.toml` starts and monitors processes on one host. When Fabric is present, optional placement, locality, RT class and published-surface fields become available. ShareCLI does not embed the global graph coordinator or pretend to be the distributed OS.

## Proposed extension

```toml
[[process]]
name = "compiler"
command = ["cargo", "build", "--workspace"]

[process.fabric]
service_class = "background"
objective = "min_completion_without_rt_violation"
prefer_data = ["repo://current", "cache://cargo"]
capabilities = ["rust-toolchain"]
publish = ["terminal", "logs"]
```

ShareCLI submits a declarative workload; Fabric chooses execution and object paths; thegent may have authorized the work; Tracera and SessionLedger receive evidence/history references.

---

## Transactional integration proposal — 2026-10-12

- ShareCLI local work coordinator owns real admission, coalescing and pressure logic. Its standalone local scope must remain independently meaningful.
- Fabric integrates external placement/object epochs into the **same logical global nested history**, not a second agent-local ledger, without importing ShareCLI's authority.
- Both Human and SoftwareAgent use Actor; role/grant/scope determines effective permission.
- Candidate work plan and placement quote are not authoritative lease or successful OS actuation. Prepare/commit a resource reservation, dispatch at effect broker, observe real target result, then accept and publish with current epoch.
- Producer sharing preserves each consumer's cancellation, freshness and output rights; parent budget is not duplicated by child agents.
- Standalone root adoption across hosts must be an explicit authority transition; disconnected rights and unknown outcomes require bounded reconciliation.

See [Fabric ADR 0036](../adr/0036-global-nested-transaction-ledger.md), [ShareCLI ADR 0009](https://github.com/KooshaPari/ShareCLI/blob/feat/global-nested-ledger-20261012/docs/adr/0009-global-nested-ledger-local-admission.md), and [spec 027](../../specs/027-global-nested-ledger/spec.md). No production cross-host execution is asserted.
