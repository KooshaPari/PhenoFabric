# Tasks — Spec 019 Surface Plane (PF-WP-015, R1)

| ID | Sub-task | Definition of done |
|:--|:--|:--|
| S019-01 | `SurfaceKind` enum with 8 variants | Compiles, derives Clone+PartialEq+Eq+Debug, JSON-serializable, unit test in `surface.rs` |
| S019-02 | `SurfaceSpec` struct (5 fields) | Compiles, derives Serialize+Deserialize, JSON round-trip test, unit test asserts default values |
| S019-03 | `LeaseState` enum (4 variants) | Same as S019-01 |
| S019-04 | `SurfaceLease` struct + `try_transition` FSM | FSM rejects 2 invalid transitions (test), accepts 4 valid (test) |
| S019-05 | `RouteBinding` struct | Compiles, JSON round-trip test |
| S019-06 | `fabric-graph::decision` module (Decision, Severity, ReasonCode, String() methods) | Compiles, `String()` round-trip test, 6 ReasonCode variants defined |
| S019-07 | `admit(&SurfaceSpec, &RoutePlan) -> Decision` | Compiles, 4 acceptance-criteria tests pass (locality downgrade, epoch stale, trust below min, no matching step) |
| S019-08 | `crates/fabric-graph/tests/surface_admit.rs` | 4 tests, all green |
| S019-09 | `crates/fabric-graph/tests/surface_lease_fsm.rs` | 6 tests, all green |
| S019-10 | `cmd/checker` Go: `surface` subcommand | 2 Go tests green, decisions match Rust output byte-for-byte for shared inputs |
| S019-11 | `cargo test --workspace` clean | 96 Rust + 18 Go total, 0 failures |
| S019-12 | `cargo run --example verify_fixtures` clean | 7/7 descriptors + 5/5 manifests still parse (no fixture regressions) |
| S019-13 | 4/4 spec checks | manifest ✓ schemas ✓ openapi ✓ links ✓ |

## Cross-deps

- S019-04 depends on S019-03
- S019-07 depends on S019-01, S019-02, S019-06
- S019-10 depends on S019-07 (so Go can be tested against Rust output)
- S019-11 depends on S019-08, S019-09, S019-10
- S019-12 depends on S019-11 (cargo run example is part of cargo workspace)

## Definition of done (overarching)

- All 13 sub-tasks green
- Spec 019 status moves from `draft` to `accepted`
- Manifest regen + 4/4 spec checks green
- Commit + meta/PHENOTYPE_ARCHITECTURE.md addendum
