# 0028 — Testdata Verification Pattern

## Status

Accepted — 2026-09-05.

## Date

2026-09-05.

## Deciders

phenotype-fabric working group.

## Traceability

- PF-WP-011 capability-probe vs NVMS-manifest cross-checker (spec 018 / ADR-0027).
- `cmd/checker/testdata/` — 12 fixture pairs (7 descriptors + 5 manifests).
- `crates/fabric-capability/examples/verify_fixtures.rs` — executable regression gate.
- `crates/fabric-capability/tests/fixtures_roundtrip.rs` — Rust integration test.

## Context

Across the 2026-09-01 → 2026-09-05 session chain, three Rust crates
(`fabric-workspace`, `fabric-cli`, `fabric-checker`) hit cascading type-mismatch
loops because the developers (myself included) wrote against **invented** type
shapes instead of reading the authoritative Rust source. The same pattern
hit the testdata layer: the first 12 fixtures I committed used invented field
names (`cpu_count_physical`, `memory_total_bytes`, `numa_topology`) that don't
exist in `fabric_capability::Capabilities`/`ComputeCapabilities`.

When the fixtures only exist as JSON on disk, the drift is invisible until a
runtime consumer tries to deserialize them. By then the error is reported far
from its source and the fix loop is expensive. We need a verification gate
that catches field-shape drift at the moment fixtures are authored, not at
runtime.

## Decision

Every JSON fixture under `cmd/checker/testdata/` (and any future fixture
directory) **must round-trip through the authoritative Rust `serde::Deserialize`
impl** of the corresponding domain type, verified by two complementary gates:

1. **`cargo run --example verify_fixtures -p fabric-capability -- <dir>`**
   — an example binary that loads every `*descriptor*.json` and `*manifest*.json`
   file in the given directory, runs `serde_json::from_str` against
   `fabric_capability::CapabilityDescriptor` (and `serde_json::Value` for
   manifests in the JSON-only round), and reports per-file OK / parse-error.
   Exits non-zero on any parse-error. This is the **executable regression
   gate** that any future fixture edit must pass.

2. **`cargo test -p fabric-capability --test fixtures_roundtrip`** — a
   Rust integration test that loads the same fixture files and asserts both
   `serde_json::from_str::<CapabilityDescriptor>` success AND that the parsed
   values expose the expected semantic fields (`processor`, `cores_physical`,
   `memory_bytes`, etc.). This is the **semantic regression gate**.

Both gates are run by the existing `spec-validation.yml` CI on every PR and
must pass before merge.

## Field-name drift detected and fixed (2026-09-05)

The first round of fixtures used invented field names. The verifier caught them
all in a single `cargo run`. The corrected fields, grounded by reading
`crates/fabric-capability/src/descriptor.rs` lines 13–200, are:

| Invented (pre-ADR-0028) | Real (`ComputeCapabilities`) |
|:--|:--|
| `cpu_count_physical` | `cores_physical: u16` |
| `cpu_count_logical` | `cores_logical: u16` |
| `cpu_arch` | `processor: String` |
| `memory_total_bytes` | `memory_bytes: u64` |
| `numa_topology` | `numa_nodes: u8` |
| (missing) | `cache: Vec<CacheLevel>` |
| (missing) | `memory_bandwidth_mbps: Option<u32>` |
| (missing) | `hyperthread_pairs: Vec<(u8,u8)>` |
| (missing) | `tdp_watts: Option<u16>` |

The `numa_topology` field is **intentionally dropped** — the real struct has
just `numa_nodes: u8` because the fabric doesn't need to model the full NUMA
topology for R0 (a future revision may add it as `Vec<NumaNode>`).

## Alternatives considered

- **YAML fixtures** — rejected. Stdlib-only JSON avoids a Rust YAML dep
  (the existing `serde_yaml` in `phenotype-nvms-adapter` is for translating
  the upstream NVMS v0.2 manifest format, not for our own fixtures).
- **Proptest / quickcheck** — rejected. We are testing data-shape round-trip,
  not arbitrary property invariants. The deterministic JSON fixtures + verify
  example + Rust integration test pattern is cheaper and more readable.
- **Schema-only validation** (`jsonschema` crate) — rejected as primary gate.
  JSON Schema catches structural shape but cannot verify that a Rust type's
  `Deserialize` impl accepts the data. The `serde_json::from_str` round-trip
  is the strongest signal that the fixture is actually consumable.

## Consequences

- **Positive**: any future fixture edit that introduces field-name drift is
  caught immediately by `cargo run --example verify_fixtures` — no
  manual review of the JSON required.
- **Positive**: the verifier output is human-readable and grouped by file,
  so the fix path is obvious (`OK ...` vs `PARSE-ERR missing field
  memory_bytes`).
- **Positive**: the Rust integration test asserts semantic values, so a
  fixture that parses but has the wrong value (e.g. `cores_physical: 1`
  when the test expects `cores_physical >= 8`) still fails the gate.
- **Cost**: ~100 lines of verifier + ~50 lines of Rust test. Negligible
  compared to the cost of chasing drift downstream.
- **Convention**: future fixtures are added by writing JSON that passes the
  verifier first; if the Rust type doesn't accept the JSON, fix the JSON,
  not the Rust type.

## References

- `crates/fabric-capability/examples/verify_fixtures.rs` — the executable gate
- `crates/fabric-capability/tests/fixtures_roundtrip.rs` — the semantic gate
- `cmd/checker/testdata/` — the 12 fixtures (7 descriptors + 5 manifests)
- `adr/0027-checker-decision-taxonomy.md` — the reason-code taxonomy the
  fixtures encode
- `crates/fabric-capability/src/descriptor.rs` lines 13–200 — the authoritative
  type surface
