# PhenoFabric — State & Forward Docket v1.0

**Subject:** `phenotype-fabric` (`git@github.com:KooshaPari/PhenoFabric.git`)
**Snapshot ID:** `pf-2026-10-17T08:55Z-58176d1`
**Commit under assessment:** `58176d1` (full SHA in `snapshot.json`)
**Observation window:** 2026-10-17T03:18Z – 08:55Z
**Generated:** 2026-10-17T08:55Z / 2026-10-17 01:55 PDT
**Baseline for deltas:** `pf-2026-10-16T21:51Z-c9eb1a3` (the audit snapshot)
**Contract:** State & Forward Docket v1.0 (`handbook/specs/assessment/state-forward-docket-v1/CONTRACT.md`, branch `docs/high-velocity-state-forward-docket-20261016`)

> **Publication status.** This docket is a working-tree/pushed-branch artefact. It is **not** a
> release, not installed, not accepted by any consumer, and no human has validated it. Those
> are separate outcomes and are not claimed.

---

## 0. Two-minute cockpit

### The three questions, kept separate

| Question | Answer |
|---|---|
| **Implementation** — what is built? | 22 crates, 53,825 production LOC, 771 unit + 66 integration tests, clippy clean. Transport selection now has a real second adapter; federation is reachable from the wire. |
| **Usability** — which beneficiary completes which journey on which artefact? | **One journey, partially.** A developer can clone, build, run the daemon unauthenticated, and get wire/CLI responses. Two-device journeys remain impossible. No GUI journey is proven. |
| **Validation** — has a real person used it and did it help? | **NO.** Zero user sessions. Zero feedback. Nothing in this docket is user-validated. |

### Progress bars

Definitions: leaves are weighted predicates in `snapshot.json`. `P` = accepted pass weight,
`F` = fail weight, `U` = unknown/unexecuted/stale weight.
**Assessed pass rate = P/(P+F). Assessment coverage = (P+F)/(P+F+U). Verified satisfaction = P/(P+F+U).**

```
GOAL BRANCH                P/(P+F)   COVERAGE   VERIFIED   STATE
A1 single-device usable    [#######-]  8/9      [######--] 6/9   code done; needs a human
A2 two-device usable       [###-----]  4/8      [###-----] 4/8   wire real; cross-device unproven
A3 transport selection     [#####---]  5/9      [#####---] 5/9   executor + 2 adapters; PCIe/RDMA UNKNOWN
Cross-cutting assurance    [########]  8/8      [########] 8/8   gates green at 58176d1
----------------------------------------------------------------------
PRODUCT (unique leaves)    [######--] 25/34    [#####---] 23/34
```

`U` is non-zero on A1 (human acceptance unrun), A2 (cross-device unrun), A3 (5 hardware-gated
transports). A zero denominator would be undefined, not 100%; none occurs here.

### Delta from the audit baseline

| Indicator | Baseline `c9eb1a3` | Now `58176d1` | Δ |
|---|---|---|---|
| Unit tests passing | 751 | **771** | +20 |
| Integration tests passing | 63 | **66** | +3 |
| `RouteExecutor` exists | **NO** | **YES** | earned |
| Non-TCP adapter implemented | **NO** | **YES** (`unix_socket`) | earned |
| Federation reachable from wire | **NO** | **YES** | earned |
| Silent-no-op BLOCKERs | 3 | **0** | earned |
| First-run documentation | absent | `FIRST-RUN.md` | earned |
| **Usable journeys** | **0** | **0** | unchanged |
| **Human validation** | **0** | **0** | unchanged |

Bars: `[####....]` newly earned · `[========]` retained · `[........]` remaining/unmeasured.
Nothing was lost or invalidated in this window; one regression was reverted and parked
(`wyvern`'s payload work, §7).

### What a person can use today

```sh
cargo run -p fabric-daemon            # unauthenticated local mode, binds 127.0.0.1:9400
cargo run -p fabric-cli -- status     # returns real JSON from a real daemon
```

That is the complete list. Everything else is either unproven or impossible without a second
machine, a WorkOS account, or hardware this host does not have.

---

## 1. Intent — the mature product, then its projections

**Mature intended product** (README.md:9, pinned): a packaged all-in-one computing environment
that makes connected machines, OS sessions, VMs, accelerators, displays, audio, input, storage
and remote locations appear as **one coherent interactive computer**, while preserving the
physical facts needed for good placement decisions.

**Closest Viable Product (CVP)** — the smallest living slice that preserves that identity:
one machine runs the daemon; a second machine connects over a selected non-TCP-by-default
transport; a user drives input on one and sees output on the other. Identity-preserving
because it exercises placement, transport selection and interactive I/O together.

**MVP** (narrower, honours the same identity): one machine, daemon + GUI, a user performs one
real task and observes a durable result.

### Intent archaeology — what the project actually asked for

| Source | Authority class | Status |
|---|---|---|
| `README.md:6` — "Research and architecture baseline; not an implementation-complete claim" | direct requirement | **accepted, accurate** |
| `STATUS.md:4` — lifecycle `specified` | direct requirement | **accepted, accurate** |
| 37 ADRs in `docs/adr/` | accepted design | mixed: many partially or wholly unimplemented |
| A1/A2/A3 acceptance goals | operator-set acceptance | **A1 needs a human; A2/A3 unaccepted** |
| Release notes R0/R1, `specs/025`, `DEVELOPMENT_GUIDE.md` | historical claim | **contradicted by code** (§8) |

**Unresolved contradiction (recorded, not resolved):** the README and `STATUS.md` describe a
specification baseline, while release notes describe delivered capability. 55% of sampled
documentary claims are wrong or stale. This docket is the current status layer; the release
notes are not.

**Superseded intent:** none identified in this window.

---

## 2. Usability — the named-beneficiary audit

| # | Beneficiary | Journey | Artefact | Environment | Can they complete it? | Evidence |
|---|---|---|---|---|---|---|
| U1 | developer | clone → build → daemon responds | `fabric-daemon` | macOS, local | **YES** | unauthenticated mode works |
| U2 | developer | query daemon via CLI | `fabric-cli` | local | **YES** | 11 subcommands dispatch |
| U3 | developer | first-run as documented | `install.sh` | clean machine | **UNKNOWN** | installer repo-identity claim unverified |
| U4 | developer | single-device real task | daemon + GUI | local | **NO** | GUI not built by `cargo build --workspace`; zero GUI tests |
| U5 | developer | two-machine topology sync | two daemons | two hosts | **NO** | wire arm exists; no pairing procedure shipped |
| U6 | developer | drive input on A, see output on B | frame plane | two hosts | **NO** | input ownership enforced; no cross-host path |
| U7 | operator | authenticated session | WorkOS | browser | **NO** | needs a human sign-in (§9) |
| U8 | developer | non-TCP transport actually used | executor | one host | **PARTIAL** | `unix_socket` works; tier still selects TCP for L3/L4/L8 |

**Technical E2E proof ≠ user validation.** U1 and U2 are proven by real execution. U5 is proven
only as loopback (two processes, one host) — that is the wire arm, not cross-device behaviour.
**No journey has been validated by an actual user.** None is claimed.

---

## 3. Capability and coverage matrix

| Capability | Intended | Implemented | Reachable | Exercised | Useful | Accepted |
|---|---|---|---|---|---|---|
| Wire frame plane (dispatch, ack, cancel, reclaim) | ✔ | ✔ | ✔ | ✔ | partial | — |
| Reconnect / resume | ✔ | ✔ | ✔ | ✔ | partial | — |
| Per-session input ownership | ✔ | ✔ | ✔ | ✔ | partial | — |
| RouteExecutor seam | ✔ | ✔ | ✔ | ✔ | — | — |
| `unix_socket` transport | ✔ | ✔ | ✔ | ✔ | — | — |
| `tcp_transport` | ✔ | ✔ | ✔ | ✔ | — | — |
| `quic_transport` | ✔ | **✘** | ✘ | ✘ | ✘ | — |
| `shm_copy` | ✔ | **✘** | ✘ | ✘ | ✘ | — |
| PCIe / RDMA / vhst / DMA-BUF | ✔ | **✘** | ✘ | ✘ | ✘ | — |
| Federation topology merge | ✔ | ✔ | ✔ | ✔ | partial | — |
| Graph engine (leases, failover, multihop) | ✔ | ✔ | partial | ✔ | — | — |
| Capability layer (signing, trust root) | ✔ | ✔ | ✔ | ✔ | — | — |
| Desktop GUI | ✔ | ✔ | **✘** | ✘ | ✘ | — |
| Web frontend | ✔ | ✔ | ✘ | partial | ✘ | — |
| Authenticated sessions | ✔ | ✔ | ✔ | **✘** | ✘ | — |
| C ABI (`fabric-capability-ffi`) | ✔ | ✔ | ? | **✘** | ✘ | — |

Legend: ✔ proven · ✘ absent/unproven · — not assessed · ? unknown. "Reachable" means callable
from a production entry point. "Exercised" means a test or run observed it.

**Zero-test crates (4):** `fabric-gui` (2,608 LOC of GUI), `fabric-tui`, `phenotype-manifest`,
`fabric-capability-ffi` (a C ABI). These are unverified surface; the workspace pass count hides
them.

---

## 4. Unfinished inventory

Scored at `58176d1`. Effort in focused sessions; ranges, not invented hours.

| ID | Location | Goal | Present behaviour | User consequence | Closure oracle | Effort |
|---|---|---|---|---|---|---|
| G-01 | `multihop/mod.rs:211` | A3 | `_ => "tcp_transport"`: L3/L4/L8 silently resolve to TCP | a caller asking for RDMA silently gets TCP | tier either errors or reports the transport actually used | S |
| G-02 | `fabric-gui` | A1 | 2,608 LOC GUI, **zero tests** | nothing proves it renders live state | headless smoke asserting live daemon state | M |
| G-03 | `install.sh` | A1 | repo-identity claim unverified | a new user may 404 | verified end-to-end install on a clean machine | S |
| G-04 | two-device pairing | A2 | no shipped procedure | two machines cannot be paired by a user | documented + tested pairing flow | M |
| G-05 | `fabric-capability-ffi` | — | C ABI, zero tests | no proof the ABI is correct | ABI-level test | M |
| G-06 | PCIe/RDMA/shm adapters | A3 | catalog labels only | locality claim unbacked | adapter + measured delta | **UNKNOWN** |
| G-07 | daemon signal handling | ops | not wired as implied | Ctrl-C behaviour differs from expectation | signal test | S |
| G-08 | graceful shutdown | ops | does not drain connections | in-flight frames can be lost | drain test | S |
| G-09 | concurrent SQLite WAL | data | untested | corruption risk unquantified | two-writer test | S |
| G-10 | wire version mismatch | wire | no test | opaque failure between versions | clear refusal test | S |
| G-11 | capability revocation | security | revocation recorded, not enforced on live handle | a revoked capability may persist | revocation test on open session | M |
| G-12 | docs drift | — | 55% of sampled claims wrong/stale | users misled | claims corrected or retracted | M |
| G-13 | release notes R0/R1, spec 025 | — | describe undelivered capability | stakeholder misreads progress | annotate as planning records | S |
| G-14 | `deny.yml` `\|\| true` ×3, `spec-validation.yml:109` | assurance | checks report, cannot fail | a vulnerable dep passes | remove the waivers | S |
| G-15 | payload decoding | A2 | **PARKED** (`wyvern`): broke 16 tests, decoder unwired | frame bodies are not decoded | re-integrate AND wire into handlers | M |

**Closed in this window:** G-16 `RouteExecutor` absent → **closed**; G-17 `unix_socket` absent
→ **closed**; G-18 federation unreachable → **closed**; G-19 three silent no-ops → **closed**;
G-20 first-run docs absent → **partially closed** (written, not user-tested).

**Not deferred as "post-MVP":** every item above is in scope. G-06 is the only genuinely
hardware-gated one, and it is marked UNKNOWN rather than deferred.

---

## 5. Assurance

| Gate | Method | Observed at `58176d1` |
|---|---|---|
| `cargo check --workspace --all-targets` | executed | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | executed | exit 0, 0 errors |
| `cargo test --workspace --lib` | executed | **771 passed, 0 failed** |
| `cargo test -p fabric-integration-tests` | executed | **66 passed, 0 failed** |
| `cargo fmt --all -- --check` | executed | exit 0 |
| manifest digest | executed | 1100 files, all digests verified |
| CI / Spec Validation / E2E | executed | see `snapshot.json` |

**Known assurance weaknesses** (separate from verdicts):
- `cargo test --workspace --lib` skips ~83 main-only unit tests — the reported count understates
  the suite and overstates coverage.
- ~73% of "integration" tests never cross an OS boundary.
- Two CI checks are `|| true`: they report and cannot fail.
- No negative controls for the hardware-gated transports because they do not exist.
- **Applicability ≠ execution ≠ verdict.** A passing fixture qualifies a fixture, not a product.

---

## 6. Forward path

Dependency-ordered. `S` ≈ one focused session.

```
DONE  P0 no-op BLOCKERs        P1 RouteExecutor seam
      P2a unix_socket          P2b federation wire arm
      P2c first-run docs
──────────────────────────────────────────────────────────
NEXT  G-01 transport fallback truth        S   no deps   [operator decision]
      G-14 remove CI waivers               S   no deps
      G-07/08/09/10 lifecycle+data tests   2S  no deps
      G-02 GUI headless smoke              2S  no deps
      G-04 two-device pairing procedure    2S  needs P2b ✔
      G-11 revocation enforcement          1S  no deps
      G-12/13 docs truth pass              1S  no deps
      G-15 payload decoding (re-do)        2S  needs wiring
      G-05 C ABI tests                     1S  no deps
LATER G-06 PCIe / RDMA / shm           UNKNOWN   hardware-gated
      CVP completion (2 machines, real E2E)  4–6S + 2 real hosts
```

### Time-boxed milestones

| Horizon | Deliverable | Depends on |
|---|---|---|
| **30 min** | G-01 decision applied + G-14 waivers removed; CI re-verified | operator answer on G-01 |
| **1 hour** | + G-07/08/09/10 lifecycle and data tests green | none |
| **4 hours** | + GUI headless smoke (G-02) + revocation enforcement (G-11) + docs truth pass | none |
| **24 hours** | + two-device pairing procedure (G-04) + C ABI tests + payload decoding redone | a second host for the pair test |
| **CVP** | two machines, real transport, real input/output, documented pairing | **4–6 sessions + 2 real machines + operator sign-in** |
| **Mature** | + QUIC/SHM adapters with measured deltas | PCIe/RDMA **UNKNOWN** |

**Aggregate agent effort vs elapsed time.** The 4-hour band assumes parallel lanes on disjoint
files, which is how this window ran. Elapsed critical-path time is longer than aggregate effort
because CI runs ~4 minutes per push, gates are serial, and the manifest ordering is a strict
sequence. Review and rework are not included and have historically been ~30% of lane output
(all three lanes this window left work unfinished that I had to complete).

**Scenarios:** low = 4 sessions to CVP (2 hosts available, no surprises); likely = 6–8 (lane
rework, one regression cycle); high = 12+ (hardware for G-06 turns out to be required for
acceptance). These are engineering ranges with stated assumptions, **not** statistical P50/P90.

---

## 7. Decisions, blockers, risks

### Blockers with precise consequences

| Blocker | Precisely prevents | Does not prevent |
|---|---|---|
| **No WorkOS sign-in by a human** | A1 acceptance; any authenticated journey | unauthenticated local work, all wire/graph/transport work |
| **No second physical machine** | real cross-device proof (A2 acceptance) | loopback wire-arm proof, which is already green |
| **No RDMA/PCIe hardware on this host** | G-06, the A3 upper bound | unix_socket + QUIC + SHM on one Linux host |
| **Operator decision G-01 (A vs B)** | closing the silent-degradation finding | the executor seam, which is done |

### Decisions taken in this window

| Decision | Rationale |
|---|---|
| Revert and park `wyvern`'s payload work | it broke 16 existing tests and its decoder was proven unused by clippy; regressing green for unverifiable gain is not acceptable |
| Complete the lanes' unfinished work rather than revert it | the underlying work was sound (+20 unit, +3 integration) and the gaps were small and precisely identifiable |
| Leave `_ => "tcp_transport"` unchanged | the A-vs-B choice is the operator's; inventing intent would bake a guess into the architecture |
| Report `unix_socket` as proven, quic/shm as absent | a catalog label is not an implementation |

### Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Lane work is unfinished more often than reported | high, observed 3/3 this window | independent gating of every lane; never trust a lane's self-report |
| Manifest ordering is a strict sequence that fails silently | medium, bit twice | codified: `add -A` → regenerate → check → `add MANIFEST` last |
| Docs drift re-accumulates | medium | this docket set becomes the status layer |
| GUI is entirely unverified | medium | G-02 |
| Payload decoding is parked, not done | medium | G-15 |

---

## 8. Documentation trust

| Document class | Trustworthy as current state? |
|---|---|
| `README.md` status lines, `STATUS.md` | **yes** |
| `docs/adr/` (37) | yes as *decisions*; a large fraction unimplemented |
| `docs/reports/2026-10-17-full-state/` | **yes** — the audit |
| This docket | **yes** for the instant it describes; superseded on change |
| Release notes R0/R1, `specs/025`, `DEVELOPMENT_GUIDE.md` | **no** |
| `docs/runbooks/workos-infisical-shared.md` | **no** — commands fail as written |

---

## 9. Lineage

**Not a fork.** PhenoFabric is a first-party project (`KooshaPari/PhenoFabric`, created
2026-09-10). There is no upstream to diverge from and **no fork acceptance criteria apply**.
This is recorded explicitly because the Docket contract requires the distinction: applying
upstream-parity or migration obligations here would be inventing requirements.

---

## 10. Limits of this docket

- No GUI was launched. The GUI is **UNKNOWN**, not broken.
- No second machine was used. Cross-device is **UNPROVEN**.
- No human used any of this. **Zero user validation.**
- Lanes did not run cargo (build contention); all test counts are the coordinator's own
  executed runs at `58176d1`.
- Hardware-gated transports are **UNKNOWN**, not estimated.
- This is not an exhaustive audit of the portfolio, the local machine, or every historical
  conversation.
