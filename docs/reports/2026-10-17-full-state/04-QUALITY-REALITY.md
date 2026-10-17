# 04 — Quality Reality

**Commit:** `c9eb1a3` · **Observed in audit:** 751 unit, 63 integration, clippy clean, fmt clean, manifest 1086 entries verified

**Question answered:** does the passing test count prove the project is ready?

**Short answer:** no. The pass count is real. The *readiness claim* it is used to support is
overstated, for three specific and fixable reasons.

---

## 1. Four crates have zero tests

Of 22 workspace members:

| Crate | Unit | Integration | Total |
|---|---|---|---|
| `fabric-graph` | 113 | 52 | 165 |
| `fabric-frame-transport` | 147 | 4 | 151 |
| `fabric-daemon` | 406 | 8 | 414 |
| `fabric-capability` | 21 | 29 | 50 |
| `fabric-persist` | 36 | 0 | 36 |
| `fabric-cli` | 22 | 13 | 35 |
| `fabric-checker` | 17 | 0 | 17 |
| `fabric-graph-cli` | 7 | 8 | 15 |
| `phenotype-nvms-adapter` | 13 | 0 | 13 |
| `fabric-web` | 5 | 0 | 5 |
| `fabric-tray` | 4 | 0 | 4 |
| **`fabric-gui`** | **0** | **0** | **0** |
| **`fabric-tui`** | **0** | **0** | **0** |
| **`phenotype-manifest`** | **0** | **0** | **0** |
| **`fabric-capability-ffi`** | **0** | **0** | **0** |

`fabric-gui` is the **desktop GUI** — 2,608 lines of HTML plus a Tauri shell — with zero
tests. `fabric-capability-ffi` is a **C ABI** with zero tests. These are the two most
externally-facing surfaces in the workspace and neither has a single assertion.

The workspace-wide pass count hides them completely.

---

## 2. The 751 figure is inflated

`cargo test --workspace --lib` is what CI runs (`ci.yml:211`). `--lib` only compiles library
targets. Modules reachable only from `main.rs` are **silently skipped**.

Affected: `fabric-capture`, `fabric-tui`, and several `fabric-daemon` modules (`metrics`,
`logging`, `input_guard`, `rate_limiter`, `secret_loader/*`).

Estimated **~83 unit tests never execute** under the CI command. The true unit test count is
higher than 751, but the number CI reports is not the number that ran against the library.

This is not fraud; it is a flag mismatch. But it means "751 passing" is not the coverage
statement it looks like.

---

## 3. Most "integration" tests are unit tests renamed

Of 27 `tests/*.rs` files, only **6 actually cross a real OS boundary** (real subprocess,
real socket, real file). Roughly **73%** of the "integration" coverage is single-process
public-API calls that never leave the test binary.

Both "two node" test files start two real daemons — but both bind `127.0.0.1`. They cross a
process boundary, not a machine boundary. They are honest about what they do; they are not
evidence of cross-device behaviour.

---

## 4. What is clean

| Dimension | Finding |
|---|---|
| Mocks / fakes in production code | **None.** Verified workspace-wide |
| Classic stubs | **Zero** (`todo!`, `unimplemented!`) |
| Property-based testing | Real `proptest` usage, not decorative |
| E2E | Real TCP and real SQLite, not simulated |
| CI | 3 workflows, mostly honest |

---

## 5. CI soft spots

| Workflow | Line | Issue |
|---|---|---|
| `deny.yml` | 74, 78, 82 | `govulncheck ... \|\| true` on three lines — a vulnerable dependency cannot fail this check |
| `spec-validation.yml` | 109 | `go run . testdata/... \|\| true` |
| `e2e.yml` | — | gating depends on GitHub branch protection, not on the workflow itself |
| `release.yml` | — | `continue-on-error: true` on macos-x86_64 and docker-build — documented and intentional |

The `|| true` instances matter: they mean two checks *report* rather than *gate*. A report
that cannot fail is not a control.

---

## 6. The five most important untested behaviours

Judged by user impact:

1. **Daemon signal handling is not wired.** Ctrl-C behaviour is not what the handler implies.
2. **Graceful shutdown does not drain connections.** In-flight frames can be lost on stop.
3. **Concurrent SQLite WAL writers** — no test crosses two writers.
4. **Wire protocol version mismatch** — no test asserts a clear refusal between versions.
5. **Capability revocation does not propagate to a live handle** — revocation is recorded,
   not enforced on an open session.

Every one of these is a data-loss or security-shaped gap, and every one is invisible to the
current suite.

---

## Verdict

| Claim | Verdict |
|---|---|
| "Test suite passes" | **TRUE** — 751 unit, 63 integration, observed |
| "Code quality is high" | **TRUE** — clippy clean, no mocks in production, no classic stubs |
| "The suite proves readiness" | **FALSE** — four zero-test crates, ~83 skipped unit tests, 73% of "integration" is in-process |
| "Green CI means shippable" | **FALSE** — two checks are `\|\| true`, gating is external |

The correct reading: this is a **well-engineered codebase with an under-powered verification
net in exactly the places that face users.** The tests are honest; the summary metric is
misleading.
