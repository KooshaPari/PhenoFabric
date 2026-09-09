# Tasks 021 — Trust-Root Implementation

**Status:** draft
**Spec:** [spec.md](./spec.md) · [plan.md](./plan.md)

| ID | Title | Depends | Status | Notes |
|:--|:--|:--|:--|:--|
| S021-01 | ADR-0031 (trust-root model) Accepted | — | done | this turn |
| S021-02 | spec 021 meta.json + spec.md + plan.md + tasks.md | S021-01 | done | this turn |
| S021-03 | `trust_root.rs` skeleton: constants + enums + structs | Phase 0 read | in-progress | — |
| S021-04 | `TrustError` + `From<Error>` impl | S021-03 | pending | — |
| S021-05 | `Authority::canonical_bytes` | S021-03 | pending | — |
| S021-06 | `TrustStore::new(root)` — anchor insertion | S021-04 | pending | — |
| S021-07 | `TrustStore::add_authority` — verify parent signature | S021-06 | pending | — |
| S021-08 | `TrustStore::set_revocation_list` — verify root signature | S021-06 | pending | — |
| S021-09 | `TrustStore::verify_chain` — full algorithm | S021-07, S021-08 | pending | spec §4 |
| S021-10 | 5 unit tests in `trust_root.rs` | S021-09 | pending | spec §7 |
| S021-11 | 3 integration tests in `tests/trust_root_chain.rs` | S021-09 | pending | spec §7 |
| S021-12 | Wire `pub mod trust_root` + re-exports in `lib.rs` | S021-09 | pending | — |
| S021-13 | 4/4 spec checks + MANIFEST regen | S021-10, S021-11, S021-12 | pending | — |
| S021-14 | Commit + WORKLOG + meta/ARCHITECTURE | S021-13 | pending | — |

## Definition of done

- `cargo test -p fabric-capability` all green (existing 11+6+6+6 + new 5 unit + 3 integration = 31 tests).
- `cargo test --workspace` 138 Rust pass (was 130; +8 = 5 unit + 3 integration).
- `go test ./... cmd/{capprobe, checker}` unchanged (18 pass).
- 4/4 spec checks pass.
- MANIFEST.sha256 regen'd for all new/changed files.
- WORKLOG has a 2026-09-08 entry describing the trust-root landing.
- meta/PHENOTYPE_ARCHITECTURE.md has the corresponding addendum.
- Indexes updated: adr/INDEX.md (row 0031), specs/INDEX.md (row 021).

## Cross-deps

- spec 019 surface plane (spec.rs `Signature` shape) — same signature
  shape is reused; no schema change.
- spec 020 route lease integration — does NOT call `verify_chain` in
  R1; R2 topology-validation hook.
- ADR-0023 (signature format) — unchanged.
- ADR-0028 (fixture verification) — unchanged.
- ADR-0029 (Go vs Rust port policy) — N/A; this is a new Rust-only module.
