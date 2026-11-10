# Goal map, observed gaps and CVP selection

**Snapshot:** PF-SFD-20261023-READ-001. Evidence class: source inspection + historical authored reports.

## Mature product goal tree

The PRD currently specifies **JTBD-01..12** and epics **E1..E12**, including desk/couch workspace continuity, MacBook as an additional independent seat, separate/atomic input-output routing, Windows application presentation on macOS, agent-published surfaces, gaming/Ableton isolation, heterogeneous placement, local fast paths, graceful quality degradation, fallback console, route explainability and decision replay. These remain the mature target, not an implicit promise that every feature exists.

| Goal family | Intended benefit | Current source assessment | Next discriminating evidence |
|---|---|---|---|
| E1/E11: install, API, identity/security | User can safely start and operate Fabric | Daemon/CLI appears source-backed; GUI and authenticated federation not accepted | Fresh-build and release test; peer rejection |
| E2/E4/E5: I/O, seats, windows | Multi-device seamless workspace | No complete real two-device journey verified | Two hosts; complete input/display focus task |
| E3: route and locality | Select least-cost physical path | TCP executor exists; non-TCP options return unsupported | Negative unsupported test then real adapter benchmark |
| E6/E7/E9: latency, media, real-time | Foreground gaming/music remains stable | Design/research claimed; no physical contention evidence inspected | Instrumented user workload under background stress |
| E8/E10: distributed placement/agent realms | Place workloads and publish ephemeral realms | Broad specification; task/realm lifecycle not proven here | One real placed task with recovery/TTL |
| E12: ecosystem interop | Compose without authority duplication | Boundaries specified for AgilePlus, Tracera, SessionLedger, ShareCLI | One contract-verified consuming journey |

## Evidence-bound unfinished inventory

| Gap ID | Exact locus | Present behavior / concern | Closure oracle |
|---|---|---|---|
| PF-SFD-G01 | `docs/quickstart/FIRST-RUN.md` §3.1 vs `docs/acceptance/A2-TWO-DEVICE-PAIRING.md`, `wire/mod.rs`, `federation/pairing.rs` | The quickstart categorically says pairing has no shipped procedure despite newer raw-wire pairing | Quickstart acknowledges one-shot pairing accurately and distinguishes non-delivered guarantees |
| PF-SFD-G02 | `crates/fabric-daemon/src/federation/pairing.rs`; acceptance A2 | One-shot exchange, no peer authentication, no retained merged topology | Unauthorized peer rejected; authenticated identities; view persists and reappears in probe; real-host proof |
| PF-SFD-G03 | `config/settings.toml` [server] `listen_addr` vs `ServerConfig.listen` | Wrong config key silently falls back to default address | Deserialize non-default `listen`, assert bind address, reject/flag obsolete key |
| PF-SFD-G04 | `config/settings.toml` [auth] `public_routes` | Narrow override may prevent auth bootstrap according to quickstart; runtime not exercised | Negative unauthenticated access plus correct bootstrap with real WorkOS test identity |
| PF-SFD-G05 | `crates/fabric-daemon/src/wire/mod.rs` active_connections | Counter increments on accepted connection; code comment says decrement is not yet implemented | Connect/disconnect repeatedly beyond max and verify released capacity; no false server_busy |
| PF-SFD-G06 | `crates/fabric-graph/src/multihop/executor.rs` | TCP supported, other transport stages return `UnsupportedTransport` | At least one true alternative adapter with trace and cost evidence or explicit scope deferral |
| PF-SFD-G07 | `Cargo.toml`, `crates/fabric-gui/src-tauri` | Not built by workspace; separately packaged GUI unverified | Fresh native install and completed primary journey |
| PF-SFD-G08 | Existing quickstart + no execution in this audit | Health/status described but unverified in current environment | Capture clean build and process/CLI status output on exact SHA |

**G05 is a source-visible risk, not a reproduced customer incident.** The accepted exit condition must be reproduced with an appropriate test before declaring a fix. G02 is security critical; do not expose insecure pairing to the public network in an attempt to qualify the CVP.

## CVP/MVP decision and alternative hypotheses

Candidate **CVP-F1:** two authenticated machines can be intentionally paired, show a persistent truthful combined topology and one meaningful cross-machine action, survive restart/rejoin, and display actionable failure when a peer is unreachable. This is a *proposed* identity-bearing subset, **not accepted scope**. It is more faithful to the product than an unauthenticated status command or one-shot topology echo.

Alternative A: ship useful single-node placement/graph first, treating federation as a later CVP. Alternative B: keep peer view ephemeral by design and build a safe repeated pull-through UI. Alternative C: reuse an existing trusted transport/tunnel while preserving Fabric's graph semantics. Decide with the beneficiary outcome and a discriminating end-to-end pilot, not with source line counts or diagram size.

**No percentage progress bar** is computed: the mature denominator across all JTBDs has not been mapped to accepted, equally/unequally weighted completion predicates. A false 100% in a small test package is not full product progress.

## No new product acceptance

All source-backed implementation observations are subject to runtime/host checks. This docset and the quickstart correction are documentation movement. No physical/installed/real-user value delta is claimed.
