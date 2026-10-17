# 02 — Usability Reality

**Commit:** `c9eb1a3` · **Method:** read-only journey tracing with file:line evidence
**Question answered:** what can a competent engineer who has just cloned this repo actually do?

---

## Journey summary

| # | Journey | Works? | Stop point |
|---|---|---|---|
| J1 | First run (clone, build, first command) | **PARTIAL** | three entry points disagree; the documented one fails |
| J2 | Single device (A1) | **NO** | shipped config refuses auth; working path is undocumented |
| J3 | Two devices (A2) | **NO** | no pairing procedure; federation not on the wire |
| J4 | GUI | **NO from a clone** | `src-tauri` excluded from workspace; sidecar not built by `cargo` |
| J5 | "What is the product?" | **PLAN-SHAPED** | product claim requires unshipped wire arm + external identity provider |

---

## J1 — First run

Three competing entry points, and they contradict each other:

| Path | What happens |
|---|---|
| `curl … install.sh \| bash` | Installer points at a different repo path than the workspace declares; release artifact lookup fails `[?]` |
| `cargo run -p fabric-daemon` | **Works.** Silently uses `DaemonConfig::default()`: `auth.enabled = false`, binds `127.0.0.1:9400`, accepts every message type |
| `cargo run -p fabric-daemon -- start --config config/settings.toml` | **Fails.** `WORKOS_CLIENT_SECRET` is empty and there is no programmatic way to obtain it |

The irony is precise: **the documented path fails and the undocumented path works.** A new
user who follows instructions is broken; a user who ignores them succeeds and never learns
they are unauthenticated.

Supporting evidence:

- `README.md:6` — "Status: Research and architecture baseline; not an implementation-complete claim"
- `STATUS.md:4` — "Lifecycle state: `specified`"
- `config/settings.toml` — ships `[auth] enabled = true` with empty secret
- `crates/fabric-daemon/src/config.rs:177-202` — `Default for AuthConfig` sets `enabled: false`
- `crates/fabric-cli/src/wire_client.rs:11` — `DEFAULT_DAEMON_ADDR = "127.0.0.1:9400"`

---

## J2 — Single device (A1)

**No.** The only end-to-end demo,
`examples/full-demo/run_demo.sh`, explicitly bypasses the shipped config. The shipped
config refuses `auth_start` with `workos_not_configured`. A1 is officially NO in the
project's own session log.

The composite task the product promises — capture, route, display on one machine — has no
test and no documented procedure.

---

## J3 — Two devices (A2)

**No.** No shipped two-device procedure exists. Federation library code carries
`#[allow(dead_code)]`. Both "two node" test files start both daemons on `127.0.0.1`, so they
prove the wire plane, not cross-device behaviour.

A user with two machines has no documented sequence to follow and no way to discover the
pairing handshake.

---

## J4 — GUI

**Not from a clone.** The GUI binary lives at `crates/fabric-gui/src-tauri/`, which root
`Cargo.toml:5` **excludes** from the workspace. `cargo build --workspace` does not build it.
The only documented delivery path is the `build-gui` GitHub Actions job producing OS
installers.

Note the precision here: `fabric-gui` **is** a workspace member (`Cargo.toml:17`) and has
**zero tests**. `src-tauri` is the excluded part. So the GUI is neither tested nor built by
the standard command.

---

## J5 — What is the product

The stated product (`README.md:9`):

> "A packaged, all-in-one computing environment that makes connected physical machines,
> operating-system sessions, virtual machines, accelerators, displays, audio devices, input
> devices, storage, and remote locations appear as one coherent interactive computer."

What the shipped tree actually is: a daemon, a CLI, a TUI, a tray app, a web frontend, and
an **in-process** federation and graph library.

The gap between the two is twofold:
1. the two-device wire arm that was never shipped, and
2. an identity provider (WorkOS) that a fresh cloner cannot provision.

So the product claim requires two things a new user cannot supply.

---

## The first thing that stops a new user

> The documented start command fails on an empty secret, and the command that works is not
> documented anywhere.

That is a one-session fix (`P2c` in the forward path) and it is the highest
usability-per-effort item in the project.

---

## Honest note on what was not tested

No GUI was launched in this audit. Nine HTML pages totalling 2,608 lines exist. Whether they
render correct state against a live daemon is **UNKNOWN**, not "broken". The distinction
matters: this report does not claim the GUI is non-functional, only that nothing proves it
is functional.
