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
