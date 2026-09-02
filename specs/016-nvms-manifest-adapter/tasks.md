# Tasks 016 — NVMS Manifest Adapter (R0.5)

| ID | Task | Phase | Definition of done |
|:--|:--|:--|:--|
| PF-WP-011.01 | Define `RequiredCapabilities` YAML struct + `nvms_to_capability_descriptor` | P0, P1 | `cargo test -p phenotype-nvms-adapter` passes; minimal NVMS YAML produces a `CapabilityDescriptor` with 6 fields populated |
| PF-WP-011.02 | Implement `AdapterError` taxonomy (UnknownField, InvalidType, MissingRequired, VersionMismatch) | P1 | `cargo test -p phenotype-nvms-adapter errors` passes; all four error variants reachable in tests |
| PF-WP-011.03 | Round-trip + provenance block (sha256 of source, adapter version) | P2 | `cargo test -p phenotype-nvms-adapter roundtrip` passes; `descriptor_to_nvms_bounds(d)` reproduces source fields |
| PF-WP-011.04 | Integration tests against archived fixtures (`minimal.yaml`, `full.yaml`) | P3, P4 | `cargo test -p phenotype-nvms-adapter` (full suite) passes; future-version YAML returns `Err(VersionMismatch)` |
