# PhenoFabric — Full State and Forward Path

**Repository:** `phenotype-fabric` (`KooshaPari/PhenoFabric`)
**Commit audited:** `c9eb1a3` ("feat(wire): enforce per-session input ownership on the frame plane")
**Report date:** 2026-10-16 21:51 PDT
**Audit mode:** seven independent read-only lanes + parent-side independent re-verification
**Scope exclusions (operator-set):** scaling, sunset, user-feedback-driven change, intent evolution

---

## How to read this set

| File | Contents |
|---|---|
| `00-EXECUTIVE-SUMMARY.md` | This file. State, goals, the one structural finding, forward path |
| `01-GOAL-STATUS.md` | A1 / A2 / A3 in detail with evidence and gap specifics |
| `02-USABILITY-REALITY.md` | What a user can actually do today, journey by journey |
| `03-STUB-INVENTORY.md` | Every stub, fake, silent no-op, and dead path in production |
| `04-QUALITY-REALITY.md` | Whether the test suite proves what it appears to prove |
| `05-DOCS-TRUST.md` | Which documentation can be trusted and which will mislead |
| `06-FORWARD-PATH.md` | Dependency graph, staged plan, effort ranges, decision points |
| `07-METHOD-AND-LIMITS.md` | How this audit was done and what it did not verify |

**Legend for all bars in this set**

- Bars are `filled/total` in discrete units, never invented percentages.
- Units are stated per bar. Where a denominator is unmeasurable the bar is replaced with `UNKNOWN`.
- `[#]` = verified with a command or file read in this audit. `[?]` = reported by a lane, not independently re-verified. `[X]` = contradicted or absent.

---

## 1. The one-paragraph state of the project

PhenoFabric is **not a collection of missing features**. It is a large body of real,
compiling, tested Rust — 52,353 production lines across 209 files, 22 workspace crates,
751 unit tests and 63 integration tests passing, clippy clean — whose **activation paths
are not connected to each other**. The single structural reason the product is not usable
today is that the multi-hop route compiler produces `TransportStage` records whose `id` is
a string like `"quic_transport"` or `"shm_copy"`, and **there is no `RouteExecutor`
anywhere in the codebase** `[#]` — verified absent by workspace-wide search. The selected
transport is therefore never instantiated. Every byte that actually moves between daemons
goes through the single TCP wire server. That one missing seam is why A2's "two devices"
is really "two processes on one device", and why A3's locality tiers cannot produce a
measurable effect even in principle.

The second-order finding is that the project's own documentation is a **decision and
specification layer, not a status layer**, and it is being read as the latter. The README
self-declares "not an implementation-complete claim" and `STATUS.md` says the lifecycle
state is `specified`, while release notes and spec files describe delivered capability.
55% of sampled documentary claims are wrong or stale. This is not dishonesty by the code;
it is drift between a large written intent and a partially-built system.

---

## 2. Goal status at a glance

| Goal | Definition | State | Gate evidence | Remaining effort |
|---|---|---|---|---|
| **A1** | Single device usable | Code complete; **acceptance blocked on a human** | Daemon + CLI + TUI + GUI + web all exist `[#]`; end-to-end interactive proof absent `[#]` | 2–4 focused sessions (agent) + 1 human sign-in |
| **A2** | Two devices exchange frames | Wire plane real; **cross-device path not wired** | Frame dispatch, cancel/teardown, reconnect/resume, ownership all CI-green `[#]`; federation not in wire protocol `[?]` | 3–6 focused sessions |
| **A3** | Transport selection | **Not implemented** | Tier→stage selection compiles; no executor exists `[#]` | 4–5 sessions (honest minimum) → 12–20 (full) → UNKNOWN (hardware) |

### A1 — Single device usable

```
code presence      [##########] 10/10  daemon, CLI, TUI, tray, GUI, web all exist
test coverage      [#####.....]  5/10  wire + CLI covered; GUI interactive never tested
documented flow    [..........]  0/10  no first-run quickstart that works as written
human acceptance   [..........]  0/1   requires a real browser sign-in
```

### A2 — Two devices

```
wire plane         [##########] 10/10  dispatch, cancel, reconnect/resume, ownership
cross-device path  [##........]  2/10  federation is a library, not a wire arm
real 2-machine     [..........]  0/1   no shipped pairing procedure
```

### A3 — Transport selection

```
tier taxonomy      [##########] 10/10  LocalityTier + 9-stage catalog real
selection logic    [##########] 10/10  select_stages compiles correct-looking plans
executor           [..........]  0/10  RouteExecutor does not exist anywhere
non-TCP adapter    [..........]  0/10  every named adapter is a string, not code
```

---

## 3. What is genuinely finished

Stated plainly so the work is not understated:

| Area | Evidence |
|---|---|
| Wire frame plane | dispatch, ack, cancel, teardown, idle expiry, concurrency cap, reconnect/resume, per-session input ownership — all CI-green |
| Zero production stubs of the classic kind | no `todo!()`, no `unimplemented!()`, zero in production paths `[#]` |
| No mocks or fakes in production code | clean across the workspace `[#]` |
| Graph engine | 165 tests; leases, failover, multihop, surface registry |
| Frame codec | 151 tests; encode/decode, bounds, truncation, max size |
| Capability layer | 50 tests; signing, trust root, descriptors, locality |
| Network primitives | STUN, UPnP, Tailscale clients are real implementations, not stubs |
| CI | 3 workflows, honest except 3 localized soft spots |

---

## 4. What is actually broken

The findings that matter, ordered by user impact. Full detail in `03-STUB-INVENTORY.md`.

| # | Location | Defect | Severity |
|---|---|---|---|
| 1 | `fabric-terminal/src/web_main.rs:272-303` | HTTP `POST /api/panes/{id}/send` returns 200, types nothing into the pane | BLOCKER |
| 2 | `fabric-terminal/src/relay.rs:12-29` | `broadcast_local` logs and returns `Ok(())`; injects nothing, despite a docstring promising remote-pane viewing | BLOCKER |
| 3 | `fabric-capture/src/main.rs:45-49` | `self-update` subcommand silently exits 0 when its feature is off (the feature is never enabled by any crate) | BLOCKER |
| 4 | `fabric-graph/src/multihop/mod.rs:211` | `_ => "tcp_transport"` catch-all labels PCIe/RDMA/shm requests as TCP | STRUCTURAL |
| 5 | workspace-wide | `RouteExecutor` absent, so no selected transport is ever instantiated | STRUCTURAL |
| 6 | `install.sh:6` | Installer targets a repo path that does not match the workspace | BROKEN ONBOARDING |

All three BLOCKERs share one shape: **a stub is wired into a real production entry point
and reports success without doing the work.** That is the single most dangerous class of
defect in this repository, because it is invisible to the test suite and to `cargo check`.

---

## 5. Time to finish, honestly

No dates and no hour counts are given, because the project forbids fabricated durations and
because a wall-clock estimate would imply a staffing assumption that is not mine to make.
Effort is expressed as **focused engineering sessions**, one session being one uninterrupted
block on one concern.

| Milestone | Sessions | Prerequisite | Verifiable without a 2nd machine? |
|---|---|---|---|
| Fix the 3 BLOCKERs | 1–2 | none | yes |
| **Executor seam + TCP fallback** | 1 | none | yes |
| Working first-run quickstart | 1 | none | yes |
| `unix_socket` adapter + first benchmark | 2 | executor seam | yes |
| **= honest A3 minimum** | **4–5** | | yes |
| Federation wire arm (`sync_topology` on the wire) | 1–2 | executor seam | yes, via Linux netns harness |
| Namespace harness + replay two-node tests | 2 | federation arm | yes |
| **= honest A2** | **3–6** | | yes |
| GUI headless smoke + composite task test | 2–3 | none | yes |
| **= honest A1** | **2–4** + 1 human sign-in | | partly |
| QUIC adapter + benchmark | 3–4 | unix_socket adapter | yes |
| SHM adapter + benchmark | 4–6 | QUIC or unix_socket | Linux only |
| **= full A3 ambition** | **12–20** | | yes |
| PCIe / RDMA / IVSHMEM / vhost / DMA-BUF | **UNKNOWN** | hardware not present on this host | no |

**The honest answer to "how long to a usable CVP":** the smallest credible CVP is
**executor seam + one non-TCP adapter + first-run quickstart + federation wire arm**, which
is roughly **7–11 focused sessions** and closes A2 and makes A3 honest rather than paper.
A1's *code* portion is inside that; A1's *acceptance* additionally needs one human browser
sign-in that no agent can perform.

---

## 6. The forward path, in dependency order

```
                        ┌─────────────────────────────┐
                        │  P0  Fix 3 BLOCKERs         │  1-2 sessions, no deps
                        │  (silent no-op success)     │  removes false claims
                        └──────────────┬──────────────┘
                                       │
                        ┌──────────────▼──────────────┐
                        │  P1  RouteExecutor seam     │  1 session, no deps
                        │  + TCP fallback impl        │  THE keystone
                        └──────────────┬──────────────┘
                                       │
                 ┌─────────────────────┼─────────────────────┐
                 │                     │                     │
      ┌──────────▼─────────┐ ┌─────────▼──────────┐ ┌────────▼─────────┐
      │ P2a unix_socket    │ │ P2b federation     │ │ P2c first-run    │
      │ adapter + bench    │ │ wire arm           │ │ quickstart       │
      │ 2 sessions         │ │ 1-2 sessions       │ │ 1 session        │
      └──────────┬─────────┘ └─────────┬──────────┘ └────────┬─────────┘
                 │                     │                     │
                 │           ┌─────────▼──────────┐          │
                 │           │ P3 netns harness   │          │
                 │           │ + replay tests     │          │
                 │           │ 2 sessions         │          │
                 │           └─────────┬──────────┘          │
                 │                     │                     │
                 │           ┌─────────▼──────────┐          │
                 │           │ A2 ACCEPTED        │          │
                 │           └────────────────────┘          │
                 │                                           │
      ┌──────────▼─────────┐                       ┌─────────▼────────┐
      │ P4 QUIC then SHM   │                       │ P5 GUI headless  │
      │ 3-4 then 4-6       │                       │ smoke + composite│
      └──────────┬─────────┘                       │ 2-3 sessions     │
                 │                                 └────────┬─────────┘
                 │                                          │
      ┌──────────▼─────────┐                       ┌────────▼─────────┐
      │ A3 (honest min)    │                       │ A1 code complete │
      └────────────────────┘                       └────────┬─────────┘
                                                            │
                                                   ┌────────▼─────────┐
                                                   │ HUMAN sign-in    │
                                                   │ A1 ACCEPTED      │
                                                   └──────────────────┘
```

**Priority rule applied:** smallest remaining effort, fastest useful outcome, fewest
dependencies. P0 and P1 are each single-session and unblock everything else. P2c
(first-run quickstart) is one session and is the difference between "a repo" and "a thing
someone can start".

---

## 7. Corrections to earlier claims made in this ownership session

Recorded because the operator is entitled to know which prior statements were wrong.

| Earlier claim | Reality | How determined |
|---|---|---|
| "`fabric-gui`'s 13 tests never run because the crate is excluded" | **Wrong.** `fabric-gui` is a workspace member (`Cargo.toml:17`); only `src-tauri` is excluded (`:5`). The real defect is that `fabric-gui` has **zero tests**. | direct read `[#]` |
| "WorkOS now returns a real `/bootstrap` session" | **Misleading.** `/bootstrap` is the WorkOS AuthKit *sign-in page* at `significant-vessel-93-staging.authkit.app/bootstrap`. The fabric daemon never serves `/bootstrap`. | lane 06 `[?]` |
| "A1 needs only a live WorkOS run" | **True but incomplete.** It needs a live run *performed by a human*; no headless, device-code, or API-only path exists. | lane 06 `[?]` |
| "`select_stages` maps L3/L4/L8 to `tcp_transport`" | **Right in substance, wrong in location.** The mapping is in `fabric-graph/src/multihop/mod.rs:211`, not `fabric-frame-transport`. And `"tcp_transport"` is a catalog label, not a transport implementation. | lane 07 + direct read `[#]` |

---

## 8. What this report does not claim

- It does not claim the tests were observed passing in this audit; lanes did not run cargo
  to avoid build contention with concurrent work. Test counts are from the parent's own
  verified runs on `c9eb1a3`.
- It does not claim the GUI works or does not work. **No GUI was launched.** Nine HTML
  pages totalling 2,608 lines exist; whether they render correct live state is `UNKNOWN`.
- It does not claim effort ranges are calendar time. They are focused sessions.
- It does not claim the hardware-gated transports are reachable. They are `UNKNOWN`.
