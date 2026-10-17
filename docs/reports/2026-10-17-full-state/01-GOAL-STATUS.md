# 01 — Goal Status in Detail

**Commit:** `c9eb1a3` · **Date:** 2026-10-16 PDT
**Evidence marks:** `[#]` re-verified in this audit · `[?]` lane-reported · `[X]` absent/contradicted

---

## A1 — Single device usable

### What exists

| Component | Evidence |
|---|---|
| Daemon binary | `crates/fabric-daemon/src/main.rs:81-244` — constructs config, binds wire server `127.0.0.1:9400`, ctrlc handler `[?]` |
| CLI, 11 subcommands | `crates/fabric-cli/src/main.rs`; dispatch fns at `commands/{cap,auth,graph,route,workspace,network,surface,probe,status,check}.rs` `[?]` |
| TUI | `crates/fabric-cli/src/tui/{mod.rs,views.rs}` `[?]` |
| GUI (Tauri + 9 HTML pages) | `crates/fabric-gui/`; pages total 2,608 lines (`dashboard.html` 490, `settings.html` 324, `auth.html` 294, `network.html` 291, `streaming.html` 285, `topology.html` 252, `logs.html` 230, `routes.html` 220, `leases.html` 218) `[?]` |
| Web frontend | `crates/fabric-web/` `[?]` |
| In-process e2e smoke | `crates/fabric-integration-tests/tests/e2e_smoke.rs` — health, topology, routes, leases over loopback `[?]` |

### What is missing

1. **No first-run flow that works as documented.** The only path that boots a working
   daemon is `cargo run -p fabric-daemon` **without** `--config`. With the shipped
   `config/settings.toml`, the daemon refuses `auth_start` with `workos_not_configured`.
   No document guides the user toward the path that works.
2. **No automated GUI smoke.** Nine HTML pages exist. Whether the Tauri sidecar launches
   the bundled `fabric-daemon-*` binary, wires the wire protocol, and renders real state is
   **UNKNOWN**. There is no Playwright, Selenium, or WebDriver harness in the repo.
3. **No composite-task proof.** No test drives a capture → route → display chain on one
   machine and asserts an observable result. The pieces exist; the chain does not.
4. **Acceptance needs a human.** The WorkOS AuthKit sign-in requires a real browser, a real
   email inbox for a 6-digit code, and a redirect. No headless, device-code, or API-only
   path exists.

### Verifiable without a second machine?

Partly. Wire protocol and CLI are verifiable in-process. GUI visual and interactive
fidelity is not. Composite single-machine task is **UNVERIFIED**.

### The `install.sh` problem

`install.sh:6` sets `REPO="KooshaPari/PhenoFabric"`. Lane-reported as mismatched against the
workspace's declared identity. A fresh cloner following `install.sh` as written is reported
to 404 on the release artifact. **Treat the installer as untested by a real external user.**

---

## A2 — Two devices exchange frames

### What exists and is CI-green at `c9eb1a3`

| Capability | Commit | State |
|---|---|---|
| Frame dispatch (`session_init` / `frame_data` / `frame_ack`) | `b4ebffb` | CI 3/3 green `[#]` |
| Cancel, teardown, idle expiry, concurrency cap | `485726b` | CI 3/3 green `[#]` |
| Disconnect + reconnect/resume | `0a97383` + `75e9b97` | CI 3/3 green `[#]` |
| Per-session input ownership enforcement | `c9eb1a3` | CI 3/3 green `[#]` |

Two-process-on-one-machine tests are real: `tests/two_node.rs` (5 tests) and
`tests/two_node_advanced.rs` (5 tests) start two actual `fabric-daemon` processes on
`127.0.0.1:0`. Federation library is implemented (860 lines across
`federation/{mod,discovery,merge,state,types}.rs`). Network primitives STUN/UPnP/Tailscale
are real (1,449 lines).

### What is missing

1. **The federation library is not wired into the wire protocol.** It carries
   `#[allow(dead_code)]` at `federation/mod.rs:9-13` — the code itself admits it is not
   reachable. There is no `federation_sync_*` message arm in `protocol.rs`.
2. **No shipped two-device procedure.** Nothing tells a user how to pair two machines.
3. **All "two node" tests are loopback.** Both daemons bind `127.0.0.1`. This proves the
   wire plane, not cross-device behaviour.
4. **Transport is always TCP** regardless of what the route compiler selected (see A3).

### Verifiable without a second machine?

Yes, via a Linux network-namespace harness with a veth pair and a userspace STUN stub —
that is the recommended path. Requires `#[cfg(target_os = "linux")]` gating.

---

## A3 — Transport selection

### What exists

- `LocalityTier` enum: `crates/fabric-capability/src/locality.rs:35`, with descriptions at
  `:68` (e.g. `L8Oob => "Out-of-band (IPMI, KVM-over-IP, WoL)"`).
- A 9-stage transport catalog: `crates/fabric-graph/src/multihop/stages.rs:80-275`
  (`shm_copy`, `quic_transport`, `tcp_transport`, `unix_socket`, and encode stages).
- Tier→stage selection: `select_stages` at
  `crates/fabric-graph/src/multihop/mod.rs:196-219`.

### The defect, quoted

```rust
let transport_id = match tier {
    LocalityTier::L0SameProcess | LocalityTier::L1SameNuma | LocalityTier::L2CrossNumaShm => {
        "unix_socket"
    }
    LocalityTier::L5Loopback => "unix_socket",
    LocalityTier::L6Lan | LocalityTier::L7Wan => "quic_transport",
    _ => "tcp_transport",   // ← L3, L4, L8 all land here
};
```

`crates/fabric-graph/src/multihop/mod.rs:211` `[#]` verified directly.

L3, L4 and L8 — the PCIe, RDMA and out-of-band tiers, the tiers that justify the whole
locality-first architecture — all silently resolve to a TCP label. No error, no warning.

### The deeper defect

**`RouteExecutor` does not exist.** Verified by workspace-wide search: no
`execute_route`, `run_route`, `apply_route`, `run_plan`, or `executor` in
`fabric-daemon/src/` or `fabric-graph/src/` `[#]`.

Consequence: the compiled `RoutePlan` is dead data. The `"quic_transport"` and
`"unix_socket"` strings are **catalog labels, not implementations**. No adapter exists for
either. Every byte still crosses the TCP wire server at
`crates/fabric-daemon/src/wire/mod.rs:39-160`.

Even if `select_stages` returned the perfect answer, nothing would act on it. **Fixing the
catch-all alone changes nothing.**

### `CopyPath`

Defined and carried in a descriptor field, never pattern-matched by any consumer.
Dead-but-wired. Its evident intent was to control whether a surface copy goes through
shared memory or the wire; nothing reads it.

### Verifiable without a second machine?

The *executor seam* and a *Unix-domain-socket adapter* are verifiable on one Linux box.
QUIC is verifiable on one machine. SHM requires Linux. **PCIe, RDMA, IVSHMEM, KVMFR,
vhost, DMA-BUF are UNKNOWN** — no RDMA NIC and no VFIO-passthrough GPU on this host.

---

## The structural blocker, stated once

> The multihop compiler emits `TransportStage` records whose `id` is a string, and there is
> no `RouteExecutor` anywhere in the codebase. The tier-to-stage selection is compiled but
> never executed. Every byte that actually moves between daemons still goes through the
> single TCP wire server. Therefore A2's "two devices" is really "two processes on one
> device", and A3's locality tier cannot produce a measurable effect because the chosen
> transport is never instantiated.

Every A2/A3 effort that does not begin with the executor seam is building on a foundation
that has not been cast.
