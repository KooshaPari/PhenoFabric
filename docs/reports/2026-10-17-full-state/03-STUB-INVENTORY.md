# 03 — Stub, Fake, and Silent No-Op Inventory

**Commit:** `c9eb1a3` · **Scope:** production code only (`crates/*/src/**`), 52,353 LOC across 209 files
**Tests, examples, benches excluded** from both findings and the LOC denominator

---

## Headline

| Severity | Count | Meaning |
|---|---|---|
| BLOCKER | 3 | wired into a real entry point, reports success, does nothing |
| DEGRADED | 9 | works but limited, or a flag with no effect |
| COSMETIC | 6 | stale comment, dead `pub fn`, no behavioural impact |
| **Total** | **18** | **0.034% of production LOC** |

**Classic stubs: zero.** No `todo!()`, no `unimplemented!()`, no `panic!("not implemented")`,
no `.expect("TODO")` anywhere in production source. All 38 `panic!` occurrences in
`crates/**/src/**` live inside `#[cfg(test)]` blocks or `#[test]` functions.

This matters: PhenoFabric does not have the usual "unfinished code loudly marked as
unfinished" problem. It has the opposite and more dangerous problem — **finished-looking code
that does nothing.**

---

## BLOCKERs

### B1 — `send_keys` accepts input and types nothing

`crates/fabric-terminal/src/web_main.rs:272-303`, registered as
`POST /api/panes/{pane_id}/send` at line 688.

The function body carries its own admission:

```
// In production: execute tmux send-keys via subprocess
// For now: just log and broadcast to connected WebSockets
```

`for now` has shipped. An HTTP client POSTs keystrokes, receives **200 OK**, and the
keystrokes are never typed into the pane. WebSocket subscribers receive an `input` event, so
a UI that only listens to the socket may *appear* to work while the underlying terminal
never changes.

### B2 — `broadcast_local` promises remote pane injection, injects nothing

`crates/fabric-terminal/src/relay.rs:12-29`. The entire body is one `tracing::debug!` and
`Ok(())`.

Its own docstring (`relay.rs:1-4`) promises: *"Takes remote pane content and injects it into
the local tmux session. This enables viewing remote panes locally without SSH attachment."*

Comments at lines 22-26 enumerate the four steps a real implementation would take (find the
local pane, clear it, write content line by line, set cursor). None execute. It is called
from `crates/fabric-terminal/src/main.rs:173`, so any data path ending here silently succeeds.

### B3 — `self-update` exits 0 having done nothing

`crates/fabric-capture/src/main.rs:45-49`.

The `#[cfg(feature = "self-update")]` gate is placed **inside** the match arm, gating only
the `self_update::run(...)` call — not the trailing `return Ok(())`. And **no crate in the
workspace ever enables the feature**. Result: `tf-win-capture self-update` matches the arm,
skips the work, returns `Ok(())`, exits 0. The user is told nothing.

The same feature, declared for `fabric-terminal` at `Cargo.toml:19` and gated at
`web_main.rs:27,540`, is likewise never enabled — that body is dead in every published build.

### Why these three share a shape

All three are **stubs wired into real production entry points that report success.** They are
invisible to `cargo check`, invisible to `cargo clippy`, and invisible to the test suite,
because nothing asserts on the side effect that never happens.

This is the highest-value class of fix in the repository: each is small, each removes a false
capability claim, and together they teach the codebase to fail loudly.

---

## DEGRADED

| Location | Finding |
|---|---|
| `fabric-terminal/Cargo.toml:19` + `web_main.rs:27,540` | `self-update` feature gates body code; never enabled; dead in every build |
| `fabric-orchestrator/Cargo.toml:20,30` | `surface-mojo` feature declared with an optional dep, but **no `#[cfg(feature = ...)]` exists anywhere in the crate** — the flag has literally no effect on compilation |
| `fabric-terminal/Cargo.toml:18` | `ssh` feature gates `dep:ssh2`; **is** enabled by default — this one works |
| `fabric-capture/src/main.rs:48,52` | Early-exit arms for `--version` and `self-update` |
| plus 5 further placeholder/deferred items | placeholder hardcoded values, deferred port checks, missing OS-family comparison, no WebSocket auth, dead `pub fn`s |

Full line-level detail is in the lane output retained with this audit session.

---

## Structural findings (not counted above, reported separately)

| Finding | Location | Why it matters |
|---|---|---|
| Silent tier catch-all | `fabric-graph/src/multihop/mod.rs:211` | L3/L4/L8 all resolve to `"tcp_transport"` with no signal |
| No executor | workspace-wide `[#]` | `RouteExecutor` absent; selected transport never instantiated |
| `CopyPath` dead-but-wired | descriptor field | Carried, never matched; whatever it controlled is uncontrolled |
| Federation not on the wire | `fabric-daemon/src/federation/mod.rs:9-13` | `#[allow(dead_code)]` — the code admits unreachability |

---

## The honest summary

18 findings in 52,353 lines is a **low** density, and the classic-stub count is **zero**. The
repository is not sloppy.

But the three BLOCKERs are all in user-facing surfaces (`fabric-terminal` HTTP API,
`fabric-terminal` relay, `fabric-capture` CLI), all silent, and all report success. A user
who encounters them concludes the feature works and the *system* is at fault — the worst
possible failure mode, because it sends them debugging the wrong layer.

**Fix order:** B3 (smallest, isolated), B1, B2. All three are 1-2 sessions total.
