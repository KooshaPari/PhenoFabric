# 06 — Forward Path

**Commit:** `c9eb1a3` · **Effort unit:** focused engineering sessions (one uninterrupted block, one concern)
**No dates. No hour counts.** Effort is a range because the project forbids fabricated durations.

---

## The ordering principle

> Smallest remaining effort, fastest useful outcome, fewest dependencies.

A small candidate is not the same as finishing a large repository. Each stage below is scoped
so it can be accepted or rejected on its own evidence.

---

## Stage overview

| Stage | Work | Sessions | Depends on | Acceptance evidence |
|---|---|---|---|---|
| **P0** | Fix 3 silent-no-op BLOCKERs | 1–2 | none | each now fails loudly or works |
| **P1** | `RouteExecutor` seam + TCP fallback | 1 | none | plan executes; delta provably zero |
| **P2a** | `unix_socket` adapter + benchmark | 2 | P1 | two daemons cross via UDS, measured |
| **P2b** | Federation wire arm | 1–2 | P1 | `sync_topology` round-trips on the wire |
| **P2c** | First-run quickstart | 1 | none | clone → running in ≤3 commands |
| **P3** | Netns harness + replay | 2 | P2b | frames cross namespaces |
| **P4** | QUIC then SHM adapters | 7–10 | P2a | 3 measurable transport deltas |
| **P5** | GUI headless smoke + composite test | 2–3 | none | page asserts live daemon state |
| **P6** | Human WorkOS sign-in | minutes (human) | P5 | scoped user state observed |

**Path to a usable CVP: P0 + P1 + P2a + P2b + P2c = 7–11 sessions.**

---

## P0 — Fix the three silent no-ops

**Why first:** they are small, isolated, and each removes a false capability claim. They also
establish the discipline of failing loudly, which the rest of the work depends on.

| Item | File | Fix |
|---|---|---|
| B3 `self-update` exits 0 | `fabric-capture/src/main.rs:45-49` | wrap the whole arm in the cfg, not just the call; return an explicit "feature not compiled in" error |
| B1 `send_keys` types nothing | `fabric-terminal/src/web_main.rs:272-303` | implement the tmux invocation, or return 501 so the caller knows |
| B2 `broadcast_local` injects nothing | `fabric-terminal/src/relay.rs:12-29` | implement the four documented steps, or return an error |

**Acceptance:** a test asserts each path either performs its side effect or returns a
non-success status. No path may return `Ok(())` having done nothing.

---

## P1 — The keystone: `RouteExecutor`

This is the single most important piece of work in the project. Everything in A2 and A3 is
built on it, and nothing currently exists.

```
crates/fabric-graph/src/multihop/executor.rs   (new)

pub trait RouteExecutor {
    async fn execute(&self, plan: &RoutePlan, buf: &mut FrameBuffer) -> Result<ExecOutcome>;
}
```

1. Define the trait. One method, takes a compiled plan and a buffer.
2. Implement `TcpExecutor` by delegating to the existing wire server. Behaviour must be
   **byte-identical** to today.
3. Call the executor from the daemon's session loop where `RoutePlan` is currently dropped.
4. Add a test proving `execute()` with a TCP plan produces the same bytes as the direct path.

**Why a TCP fallback first:** it proves the seam without changing behaviour, so the change
cannot regress anything. It also makes the honest point that **today the transport delta is
provably zero**, which is a real finding, not a gap to hide.

**Acceptance:** `RoutePlan` is no longer dead data; a test asserts the executor is invoked.

---

## P2a — `unix_socket` adapter + first honest benchmark

1. `UnixSocketExecutor` using `std::os::unix::net` — no new dependency.
2. Test: two daemons on one Linux host, frames cross via UDS instead of TCP loopback.
3. Benchmark harness `tests/transport_compare.rs`: same frame stream at 1280×720, TCP vs UDS.

**Acceptance:** a measured delta exists. This is the first point at which "transport
selection" is more than a string in a catalog.

**Note:** `unix_socket` is currently a *label* with no implementation, so this stage also
fixes the `select_stages` L0/L1/L2/L5 mapping by giving it something real to point at.

---

## P2b — Federation wire arm

The federation library (860 lines) carries `#[allow(dead_code)]` because nothing calls it.

1. Add `federation_sync_request` / `federation_sync_response` to `wire/protocol.rs`.
2. Route through `network_handlers`; invoke `sync_topology` in the session loop.
3. Drop the `#[allow(dead_code)]` on `federation/mod.rs:14` **only when** the arm exists.

**Acceptance:** a topology merge round-trips over the real wire.

**Coordination:** this touches `wire/`, which is the active development area. Sequence it
after P1 so the executor exists first.

---

## P2c — First-run quickstart

One session, highest usability-per-effort in the project.

1. A quickstart that uses the path that actually works (`cargo run -p fabric-daemon`, no
   `--config`) and says plainly that this is unauthenticated local mode.
2. A second section for the authenticated path that states the required env var and that it
   needs a WorkOS secret.
3. Fix `install.sh:6` or mark the installer unmaintained.
4. A `fabric-cli` tutorial for the 11 subcommands.

**Acceptance:** a person who has never seen the repo can start the daemon and run
`fabric-cli status` in three commands.

---

## P3 — Namespace harness

1. Two Linux netns joined by a veth pair; two daemons, one per namespace.
2. Userspace STUN stub returning the veth peer IP.
3. Replay the loopback two-node tests under the harness.
4. `#[cfg(target_os = "linux")]` gate so macOS/Windows CI skips cleanly.

**Acceptance:** A2 accepted without two physical machines.

---

## P4 — QUIC then SHM

| Step | Work | Sessions |
|---|---|---|
| QUIC adapter | `quinn` behind `quic-transport` feature; same trait shape | 3–4 |
| SHM adapter | `memfd_create` + `memmap2`, Linux only | 4–6 |

**Acceptance:** three measurable transport deltas (TCP, UDS, QUIC), then a fourth (SHM).

---

## P5 — GUI proof

`fabric-gui` has zero tests and 2,608 lines of HTML. Nothing proves it renders live state.

1. Add `headless_chrome` or Playwright as a dev-dependency.
2. Load `index.html`, navigate to `dashboard.html`, assert JS successfully calls the wire server.
3. One composite test: start daemon, drive a synthetic surface through `SurfaceRegistry`, open
   the GUI headlessly, assert the surface appears.

**Acceptance:** A1 code portion closed with evidence rather than assertion.

---

## P6 — The one thing no agent can do

The WorkOS AuthKit sign-in requires a real browser, a real email inbox, and a redirect.

| Step | Who |
|---|---|
| Open authorize URL | human |
| Submit email at `significant-vessel-93-staging.authkit.app` | human |
| Retrieve 6-digit code from inbox | human |
| Submit code, allow redirect to local callback | human |
| Everything after the callback | daemon |

There is **no headless, device-code, or API-only path**. A1 acceptance is blocked on this and
on nothing else.

---

## What is deliberately NOT in this plan

| Excluded | Why |
|---|---|
| Scaling work | operator scope exclusion |
| Sunset / deprecation | operator scope exclusion |
| User-feedback-driven change | operator scope exclusion, and the product is untested by a real user |
| Intent evolution | operator scope exclusion |
| PCIe / RDMA / IVSHMEM / vhost / DMA-BUF | hardware not present on this host — **UNKNOWN effort** |
| Rewriting anything already green | would regress a working baseline for no gain |

---

## The one decision the owner should make

`select_stages` returns `"tcp_transport"` for L3/L4/L8. Two defensible repairs:

| Option | Behaviour | Cost | Consequence |
|---|---|---|---|
| **A. Fail loudly** | requesting PCIe/RDMA returns an explicit "unsupported" error | small | honest; may break callers that silently relied on fallback |
| **B. Observable fallback** | falls back to TCP but returns the actual transport used | small | honest *and* backward-compatible |

**Recommendation: B.** It preserves behaviour, removes the silence, and gives the benchmark
harness something to record. There is no repo evidence stating the intended behaviour, so this
is a genuine product decision, not a technical one. Recorded as **UNKNOWN intent** rather than
guessed.
