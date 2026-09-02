# Plan 016 — NVMS Manifest Adapter (R0.5)

## Phases

| Phase | Scope | Exit criterion |
|:--|:--|:--|
| **P0** | RequiredCapabilities struct + serde_yaml deserialization | `cargo test -p phenotype-nvms-adapter required` passes |
| **P1** | Adapter API + AdapterError taxonomy | `cargo test -p phenotype-nvms-adapter api` passes |
| **P2** | Roundtrip + provenance | `cargo test -p phenotype-nvms-adapter roundtrip` passes |
| **P3** | Integration tests against archived fixtures | `cargo test -p phenotype-nvms-adapter` (all) passes |
| **P4** | Version mismatch detection | `cargo test -p phenotype-nvms-adapter version` passes; future-version YAML returns `Err(VersionMismatch)` |

## WP DAG

```
PF-WP-014.* (R0 — already shipped)
  └─> PF-WP-011.01 ──> PF-WP-011.02 ──> PF-WP-011.03
                                              │
                                              └─> PF-WP-011.04 (integration)
```

## Acceptance

The spec is complete when:
1. All four sub-tasks merged
2. The adapter is published as a path-dependency crate that other Fabric crates can depend on
3. The integration tests pass against the actual `nanovms/crates/phenotype-manifest` fixtures
4. The release-gate evidence is filed in `releases/2026-09-01-R0.5.md`
