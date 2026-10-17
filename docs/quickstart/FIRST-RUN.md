# Quickstart — first-run guide for Phenotype Fabric

**Read this before you try anything else.** The rest of the repository is large
and aspirational; this document is the only one that has been checked against
the source we have today. Every command below was reasoned through against the
crate source, not copy-pasted from elsewhere. Where a command was not actually
run (this is a docs-only change), the step is marked **(UNVERIFIED — see
verification note below)**.

This is for a **competent engineer** who already has:
- `git`, `cargo` (Rust 1.81 or newer, per the workspace `rust-version`), and
  the ability to compile a 22-crate workspace locally;
- about 10–30 GB of disk free for `target/`;
- patience for the first `cargo build` to take a long time.

If you do not have a WorkOS account, **stop after Section 1**. Section 2 is not
optional for the authenticated path — it requires credentials only the WorkOS
dashboard issues.

---

## Section 1 — Unauthenticated local mode (this works today)

This mode binds the daemon to `127.0.0.1:9400`, has `auth.enabled = false`
(set by `DaemonConfig::default()`), and accepts every wire message without
any token check. **It is for local development and inspection only — never
expose it to a network you do not fully trust.** There is no authentication,
no rate-limiting beyond what the daemon enforces internally, and no
multi-tenant isolation.

This path works today because `DaemonConfig::default()` (§`crates/fabric-daemon/src/config.rs:177`) ships with `auth.enabled = false`, a `listen` address of `127.0.0.1:9400`, and the full set of bootstrap routes (`health_check`, `status_check`, `auth_start`, `auth_complete`, `auth_email`, `auth_status`, `auth_verify`, `auth_logout`) registered as `public_routes`. The shipped `config/settings.toml`, by contrast, sets `auth.enabled = true` and a narrow `public_routes = ["health_check", "status_check"]`, which is what you would run against an **authenticated** daemon — see Section 2.

### Step 1 — build the workspace

From the repository root:

```bash
cargo build --workspace
```

This builds all the workspace members listed in the root `Cargo.toml` **except**
`crates/fabric-gui/src-tauri` (which is excluded from the workspace). What you
get is the daemon and CLI binaries plus the library crates; do not expect a
GUI app — see Section 3.

The first build will compile a lot of crates, including several with
substantial native dependencies (image codecs, tracing formatters, crypto).
On a fast machine expect 5–15 minutes; on a laptop, longer. There is no
shipped prebuilt.

### Step 2 — start the daemon in one terminal

```bash
cargo run -p fabric-daemon
```

No flags, no `--config`, no environment variables. This branch in
`crates/fabric-daemon/src/main.rs:111` falls through to
`DaemonConfig::default()` and binds on `127.0.0.1:9400`. You should see
log lines roughly like:

```
INFO fabric-daemon starting
INFO daemon ready addr=127.0.0.1:9400 max_connections=64
INFO auth middleware initialized enabled=false
INFO workos client secret load completed source=unset
```

(Exact wording depends on the tracing format. The four key signals are:
`daemon ready`, the listen address, `auth middleware initialized enabled=false`,
and `workos client secret load completed source=unset`. If `enabled=true`
appears, you accidentally pointed the daemon at `config/settings.toml` —
Ctrl-C and run the simpler command above.)

The process will not exit on its own. Leave it running.

### Step 3 — prove it works from a second terminal

The CLI binary is named `fabric` and is the `[[bin]]` of the `fabric-cli`
crate, so the package selector is `-p fabric-cli`:

```bash
cargo run -p fabric-cli -- status
```

This sends a `health_check` request over TCP to `127.0.0.1:9400` (the CLI
default in `crates/fabric-cli/src/wire_client.rs:11`), then prints a
human-readable summary of the daemon's health. Expected output, based on
`crates/fabric-cli/src/commands/status.rs`:

```
Status:           healthy
Uptime:           0s
Daemon addr:      127.0.0.1:9400
Topology epoch:   0
Active:           0 leases, 0 plans
```

`Uptime` increments after the first second. Anything other than
`Status: healthy` or `Status: degraded` is a red flag — Ctrl-C both processes
and try again. The daemon's own `status` subcommand does the same thing:

```bash
cargo run -p fabric-daemon -- status
```

…which is handy when you have only one terminal available.

### Step 4 — explore

The CLI has **13** subcommands (verified by reading `crates/fabric-cli/src/main.rs:34-54`):

`cap`, `auth`, `graph`, `route`, `workspace`, `network`, `surface`, `probe`, `status`, `check`, `version`, `tui`, `completions`.

Run `cargo run -p fabric-cli -- <subcommand> --help` for each. Several
subcommands will accept any input in unauthenticated local mode (that is by
design when `auth.enabled = false`) and will refuse to do anything useful
about pairing, federation, or transport — see Section 3.

The daemon itself has three subcommands (`start`, `health`, `status`) — all
visible via `cargo run -p fabric-daemon -- --help`.

### What "verified" means here

The commands above were checked against the source: `DaemonConfig::default`,
the daemon's startup branch, the CLI's status dispatcher, and the wire
client's default address were all read directly. They were **not run end-to-end**
during this docs change (the requirements were: "leave changes in the working
tree, do NOT run the daemon"). The `cargo check --workspace --all-targets`
gate did succeed, which means the code backing every command above
compiles cleanly. **Empirical daemon startup is left to you.**

---

## Section 2 — Authenticated mode (requires WorkOS credentials)

This is the path the shipped `config/settings.toml` was written for. It will
not work without credentials, and those credentials are not obtainable
without a WorkOS account.

### What you need

A single environment variable, populated with the confidential-client
OAuth secret that WorkOS issues when you create an AuthKit project:

```
WORKOS_CLIENT_SECRET=...
```

That secret:
- **Cannot be retrieved via the WorkOS API.** It is shown **once** at project
  creation time and on explicit rotation; if you lose it you must rotate it.
  This is documented in `docs/runbooks/workos-infisical-shared.md` and is a
  WorkOS policy, not a Phenotype limitation.
- **Must not be hard-coded anywhere.** `DaemonConfig::save` deliberately uses
  `#[serde(skip_serializing)]` on the secret fields so they cannot be
  persisted to a TOML config file by accident
  (`crates/fabric-daemon/src/config.rs:159, 167`).
- **Must come from your WorkOS dashboard.** The Infisical-based fallback
  path used by the Phenotype org is described in
  `docs/runbooks/workos-infisical-shared.md`; a non-Phenotype user does not
  have access to that Infisical project.

### The honest statement

**This path is not completable by someone without a WorkOS account.** Do not
treat the existence of `config/settings.toml` as a sign that you can
self-issue a `WORKOS_CLIENT_SECRET` and proceed. The shipped config has
`auth.enabled = true`, `workos_client_id` set to a Phenotype-org AuthKit
client ID, and a `workos_redirect_uri` of `http://localhost:9400/auth/callback`
— all of which are deployment-specific and would also need to be re-pointed
for any new deployment.

### What to do with a real `WORKOS_CLIENT_SECRET`

If (and only if) you have one:

1. Make sure no daemon is already running on `127.0.0.1:9400` — either
   Ctrl-C the Section 1 daemon or check with `lsof -i :9400`.
2. Set the env var and the daemon will pick it up via
   `DaemonConfig::load_env_secrets()` (`crates/fabric-daemon/src/config.rs:274`).
   `WORKOS_CLIENT_ID` and `WORKOS_REDIRECT_URI` are also accepted from the
   environment.
3. Start the daemon. You can run it either without `--config` (still uses
   the default config, just with the env values overlaid) or against the
   shipped config:

   ```bash
   # Option A — defaults + env overlay
   WORKOS_CLIENT_SECRET=... WORKOS_CLIENT_ID=... WORKOS_REDIRECT_URI=... \
       cargo run -p fabric-daemon

   # Option B — explicit config file + env overlay
   WORKOS_CLIENT_SECRET=... cargo run -p fabric-daemon -- \
       start --config config/settings.toml
   ```

   Note that `config/settings.toml` uses `[server] listen_addr = "..."`,
   but the daemon's `ServerConfig` field is `listen` (not `listen_addr`);
   serde silently ignores the unknown field, so the daemon binds to the
   default `127.0.0.1:9400`. That is a *second* documentation bug
   (see below) but it does not stop the daemon from starting.

4. From a second terminal:

   ```bash
   cargo run -p fabric-cli -- auth start
   ```

   This issues an `auth_start` wire message. With `[auth] enabled = true` and
   `public_routes = ["health_check", "status_check"]` loaded from the config,
   the daemon will treat it as an authenticated request and either:
   - Reject it with an authentication-required error (because `auth_start` is
     no longer in `public_routes` once the file's list overrides the default
     — this is the most likely outcome today, and it is a config-vs-source
     disagreement worth flagging to the daemon team); **OR**
   - Generate an authorize URL via WorkOS, which you would then open in a
     browser to log in.

   The exact error you observe depends on the daemon build; both behaviors
   are consistent with the current code. **Either way, the only way
   forward is to go through WorkOS's hosted UI.**

5. See `docs/runbooks/workos-infisical-shared.md` for the broader WorkOS /
   Infisical workflow, and `STATUS.md` for the lifecycle state of the
   authenticated capability.

---

## Section 3 — What does NOT work yet (do not waste hours here)

This is the most valuable section of this document. A new user must not
chase paths that look plausible but lead nowhere.

### 3.1 — Two-device pairing has no shipped procedure

A grep across `crates/` for `pair_with`, `paired_node`, `register_peer`,
`pairing`, and `peer_register` returns nothing. The federation subsystem in
`crates/fabric-daemon/src/federation/discovery.rs` does have a
`sync_topology(peer_addr)` path that consumes the `federation.peers` list
from the daemon config (see `DaemonConfig::federation` at
`crates/fabric-daemon/src/config.rs:229`), **but** there is:
- no end-user CLI command to add a peer;
- no interactive handshake or verification step;
- no documentation of how two daemons on two hosts actually recognize each
  other securely.

**Do not interpret the existence of `federation` configuration as evidence
of a working two-device story.** Configs alone are not products.

### 3.2 — Transport selection compiles, but only TCP is wired

`fabric-frame-transport` defines the wire framing and protocol version.
`fabric-graph/src/multihop/executor.rs` defines a `RouteExecutor` trait and
explicitly documents this state in code comments
(`crates/fabric-graph/src/multihop/executor.rs:122-131`, verbatim):

> The only fully-implemented executor today. Accepts the `tcp_transport`
> adapter and reports `tcp_transport` as the transport that served the
> request. For every other stage — the passthrough `identity` stage,
> `unix_socket`, `quic_transport`, `shm_copy`, encode/decode stages — it
> returns `ExecError::UnsupportedTransport` naming the requested adapter.

The daemon's wire server also only listens on TCP
(`crates/fabric-daemon/src/main.rs:198-205` uses `TcpListener::bind`). There
is no QUIC, no Unix-socket, no shared-memory, no IVSHMEM path wired today.
The compiler accepts plans for any of those — that is by design so the
specification compiles — but `cargo run` them and you get
`ExecError::UnsupportedTransport`.

### 3.3 — The GUI is not built by `cargo build --workspace`

`crates/fabric-gui` is in the workspace as a *library*, but
`crates/fabric-gui/src-tauri` is **excluded** from the workspace
(root `Cargo.toml` lines 3-6). The Tauri desktop app is a separate build
target with its own dependency graph and a `tauri build` flow rather than
a `cargo build --bin` flow. **No fabric-gui binary will appear in `target/`
after a workspace build.** If you need the GUI app, follow the Tauri build
instructions in `crates/fabric-gui/README.md` and `crates/fabric-gui/Makefile`
and treat it as a separate component.

### 3.4 — Other undocumented or partially-implemented areas

These are listed in `STATUS.md` as well; they are reproduced here so the
quickstart is self-contained:

- Cross-OS process migration is *explicitly non-general* by design
  (`STATUS.md:24`). Use semantic handoff, checkpoint, VM migration, or
  rematerialization instead.
- Hard real-time isolation is platform-specific research
  (`STATUS.md:25`); do not expect measured deadline protection under
  adversarial contention yet.
- Atomic syscall/function placement is a research hypothesis
  (`STATUS.md:23`), not a shipped capability.

### 3.5 — Documentation bugs we already know about

These were found during the audit that produced this document. They are
**not fixed here** because fixing them belongs to the daemon/config lane,
not the docs lane. They are listed so you do not chase them:

- `config/settings.toml` uses `[server] listen_addr`, but the daemon's
  `ServerConfig` struct field is named `listen`. The unknown key is
  silently ignored by serde, so the daemon does not bind to the address
  the config appears to set. This file also has a `public_routes = [...]`
  override that conflicts with what `DaemonConfig::default()` would
  install, and the file's narrower list blocks auth-bootstrap routes
  (`auth_start` and friends) when the file is loaded.
- The macro/CLI delivery story (cmd/, fabric-checker, fabric-capture, the
  `examples/full-demo/` workspace member) is shipped as Go, with the rest
  of the project in Rust. That is intentional and not a bug — see the
  `cmd/` directory.

---

## Where to go next

- `README.md` — product overview, design rules, honest status line.
- `STATUS.md` — capability-by-capability readiness with the evidence each
  one needs before an implementation claim can be made.
- `docs/runbooks/workos-infisical-shared.md` — the authoritative
  WorkOS/Infisical credential workflow (Phenotype org only).
- `architecture/` — the protocol, system-context, graph-runtime, and
  transport-selection documents that the source code is supposed to
  satisfy.
- `examples/` — concrete graph and workspace scenarios; note that some
  examples require non-default capabilities that are not yet wired
  (see Section 3).

When in doubt about whether a feature actually ships, grep the source
under `crates/`. **The source is the contract.** Specs and design
documents are the aspiration.
