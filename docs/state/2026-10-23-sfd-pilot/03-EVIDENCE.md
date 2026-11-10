# Evidence index / source coverage / handoff

**Evidence class:** GitHub source at `73187462e6bc4202879374be60deb98e994b005a`; no daemon, hardware or app execution by this audit. Paths below are source locators, not immutable blob excerpts; the commit SHA is the binding baseline.

| Evidence ID | Source (relative to PhenoFabric root) | What it supports; what it does not |
|---|---|---|
| PF-E01 | [README.md](../../../README.md), [docs/PRD.md](../../PRD.md) | Mature identity/JTBD/Epic intent, not implemented user value |
| PF-E02 | [docs/quickstart/FIRST-RUN.md](../../quickstart/FIRST-RUN.md) | Prior source-checked local quickstart + run-not-performed disclosure; aging gap around pairing |
| PF-E03 | [Cargo.toml](../../../Cargo.toml) | Workspace membership + Tauri exclusion, not a passing build |
| PF-E04 | [crates/fabric-daemon/src/config.rs](../../../crates/fabric-daemon/src/config.rs), [config/settings.toml](../../../config/settings.toml) | Default and configured listen/auth/federation keys; not loaded-config runtime proof |
| PF-E05 | [crates/fabric-daemon/src/wire/mod.rs](../../../crates/fabric-daemon/src/wire/mod.rs), [crates/fabric-daemon/src/wire/handlers.rs](../../../crates/fabric-daemon/src/wire/handlers.rs) | Wire binding/auto-pair/connection counter code; no live network measurement |
| PF-E06 | [crates/fabric-daemon/src/federation/pairing.rs](../../../crates/fabric-daemon/src/federation/pairing.rs) | One-shot peer-dial code and limitations; not trusted multi-host integration |
| PF-E07 | [crates/fabric-integration-tests/tests/federation_pairing.rs](../../../crates/fabric-integration-tests/tests/federation_pairing.rs) | Four test functions exist; **not run by this audit** |
| PF-E08 | [docs/acceptance/A2-TWO-DEVICE-PAIRING.md](../../acceptance/A2-TWO-DEVICE-PAIRING.md) | Earlier developer's explicit non-goals and run claim; report, not independent test |
| PF-E09 | [crates/fabric-graph/src/multihop/executor.rs](../../../crates/fabric-graph/src/multihop/executor.rs) | Unsupported transport path in source, no observed runtime |
| PF-E10 | [STATUS.md](../../../STATUS.md) | 2026-08-28 state and later 2026-10-12 proposed ledger extension; dated record, not current execution |

**Authority hierarchy:** direct accepted user intent and current boundaries > accepted ADR/spec within scope > actual current implementation and matching executed evidence for runtime truth > dated docs/reports. GitHub search/file read can confirm code presence; it cannot simulate the host.

**Collection coverage:** selected files only. Did not audit all 23 crates, all direct historical conversations, CI log execution, target physical topology, third-party adapter research or previously prepared valid acceptance dossiers. Current worktree state outside GitHub and private secrets were inaccessible. Full product maturity, cost, CVP ETA, user usefulness and uptime remain **not evaluated**.

## Resume point

Branch `docs/sfd-pilot-20261023`; initial pinned source commit `73187462e6bc4202879374be60deb98e994b005a`. Read [cockpit](00-COCKPIT.md), [gap map](01-GOALS-AND-GAPS.md), [forward DAG](02-FORWARD-DAG.md), then the existing quickstart/acceptance docs. Before adopting results on newer source commits, recheck changes to config, federation, wire protocol and integration tests. Do not blindly carry the source-defined observations forward. Do not modify auth bootstrap or pair trust defaults without explicit security review.

## Publication levels

Writing the docset to a branch is documentation progress. A PR is a review candidate. A default-branch merge is publication, not proof of runtime availability. A current two-host task and first user acceptance are separate release gates. Explicit product/registry dual receipts must be filled from actual outcomes.
