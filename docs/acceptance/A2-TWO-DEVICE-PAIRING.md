# A2: Two-Device Pairing — Acceptance Notes

This document describes how to pair two `fabric-daemon` instances over
the wire protocol, and explicitly separates what the implementation
proves from what it does not.

## What this gives you

A second daemon is reachable from a first daemon. The first daemon can
ship its own topology view to the second, and the second's wire
handler merges that view with its own. Both sides report the merged
view back to the caller of the pair request.

## The wire message

```
type:     "federation_pair_request"
peers:    ["host:port", ...]            # required, non-empty
federation_id:    "string"              # optional, defaults to daemon config
merge_strategy:   "MergeAll" | "LocalPrimary" | "PeerPrimary"   # optional
```

Response:

```
type:    "federation_pair_response"
results: [{peer, reachable, peer_count, max_epoch,
           federation_ids, merged_node_ids, error}]
```

Each entry in `results` corresponds to one address in `peers`, in
input order. `reachable: false` populates `error` with a free-text
string; the other fields are null/empty in that case.

## How to invoke

There is no GUI or CLI path yet (this lane ships the library
mechanism only). The two ways the daemon knows about a peer:

1. **Send the wire message directly.** Open a TCP connection to the
   daemon's `listen_addr` (default `127.0.0.1:9400` from
   `config/settings.toml`), write the request line, and read the
   response line:

   ```bash
   printf '%s\n' \
     '{"type":"federation_pair_request","federation_id":"peer-A","merge_strategy":"MergeAll","peers":["127.0.0.1:9400"]}' \
     | nc 127.0.0.1 9400
   ```

   This asks the local daemon (listening on `127.0.0.1:9400`) to pair
   with the peer address listed in the `peers` field
   (here, the same machine on port `9400` — replace with the actual
   peer address). The response is a single JSON object line.

2. **Use the daemon's startup config.** Set
   `[federation].enabled = true` and `[federation].static_peers = [...]`
   in `config/settings.toml`. When the wire server starts, it will
   spawn a detached thread that runs the same pair-once exchange
   against each peer in `static_peers` and logs the outcome at info
   level. The auto-pair path does not block the accept loop and does
   not cache the merged view; the message flow is identical to the
   runtime path.

The two daemons must both be running. The `listen_addr` in
`config/settings.toml` (default `127.0.0.1:9400`) is the address the
peer will dial.

## How the two daemons must differ

Each daemon's `peer_snapshot` is sent with a `federation_id` prefix.
Without distinct prefixes, the merge logic prefixes every node with
`<federation_id>/`, and the two daemons would still be distinguishable
on the receiving side, but the prefixing is the disambiguator when
two daemons have the same node names.

The auto-pair path picks up `federation_id` from `[federation].federation_id`
in the daemon's own config (default `"node-local"`). When the runtime
path is used, the caller passes `federation_id` in the request body.
For two real machines, set distinct values on each side.

## Verified behaviour (the integration test)

`crates/fabric-integration-tests/tests/federation_pairing.rs` exercises
the runtime path on loopback. The test starts two in-process daemons
on ephemeral ports via `start_two_daemons()`, sends a real
`federation_pair_request` JSON line to daemon A over TCP, and asserts
on the response. The four cases:

| Test | What it asserts |
|---|---|
| `federation_pair_request_pairs_two_loopback_daemons` | The happy path: A dials B, the peer returns a `federation_sync_response`, the merged view contains both A's prefixed node and B's local node. |
| `federation_pair_request_reports_unreachable_peer` | When A is asked to pair with `127.0.0.1:1` (a closed port), the outcome is `reachable: false` with a populated `error`; the daemon does not panic and does not crash. |
| `federation_pair_request_missing_peers_is_a_validation_error` | A request without the required `peers` field is rejected by the schema validator before the handler runs. |
| `federation_pair_request_empty_peers_is_a_handler_error` | A request with an empty `peers` array is rejected by the handler with a `federation_pair_error` carrying `error: missing_peers`. |

The test exercises:

* the new `federation_pair_request` request type is accepted by the
  schema validator (i.e., the schema registry at
  `crates/fabric-frame-transport/src/schema.rs` recognises it);
* the wire handler at `crates/fabric-daemon/src/wire/handlers.rs`
  dials the configured peer over TCP, ships a
  `federation_sync_request` carrying the local snapshot, and reads
  the peer's `federation_sync_response`;
* the per-peer `PairOutcome` is exposed through the
  `federation_pair_response` payload that the caller can read;
* failure of a single peer is reported per-outcome, not raised as a
  process-level error.

The test serialises via a static `PAIRING_GUARD: Mutex<()>` because
the pair handler briefly holds the dispatcher's `Mutex<DaemonConfig>`
while reading the `federation` config block. This is a guard against
process-wide lock contention, not a guard against port reuse; the
harness already binds ephemeral ports per daemon.

## What this does NOT prove

Honest scoping. Pairing in this lane is intentionally limited to the
smallest mechanism that connects two daemons:

* **Cross-machine semantics.** The integration test runs both daemons
  in the same process on loopback. It does not exercise path MTU
  discovery, fragmented writes, NAT rebinding, or hostile peer
  behaviour. Real two-machine pairing would still need an integration
  test against two physical hosts, which is out of scope for this
  lane.

* **Peer discovery / NAT traversal.** The peer list is what the
  caller (or the config) supplies. `static_peers` is the address list
  and nothing else. The `network` module in the daemon (STUN/UPnP/
  Tailscale) is dead-code per its module header and is not invoked
  here.

* **Pairing state.** The peer view is reported once per call and
  discarded. The local daemon's `coordinator.topology_snapshot()`
  continues to return the local view; subsequent `probe_request`
  calls do not see the merged view. If you need the merged view to
  persist across calls, that is a separate increment that would
  touch the `coordinator::state` and the `FederationState` machinery.

* **Authentication of the peer.** The wire plane currently relies on
  the request-handler middleware (auth) for caller identity, but the
  outbound dial from daemon A to daemon B does not present any token
  or challenge. The peer's `federation_sync_request` handler is
  reached without verifying that A is who it claims to be. A future
  increment should bind a shared secret or mutual TLS to the pair
  handshake.

* **Concurrent pair operations.** The `PAIRING_GUARD` mutex
  serialises tests within a process but does not constrain how many
  pair requests a daemon can serve concurrently in production. The
  pair handler holds `Mutex<DaemonConfig>` only while reading two
  short strings, and the outbound TCP operations do not hold any
  daemon lock, so concurrent calls do not deadlock. Stress-testing
  this is a separate task.

## Files touched

| Path | What changed |
|---|---|
| `config/settings.toml` | New `[federation]` section. Defaults preserve existing single-node behaviour. |
| `crates/fabric-daemon/src/config.rs` | `DaemonConfig` now carries `FederationConfig`. `FederationConfig` gained `federation_id` and `static_peers` fields. The dead-code allow on the federation config block was removed. |
| `crates/fabric-daemon/src/federation/pairing.rs` | **New.** Library-level pairing driver: `PairOutcome` and `pair_with_peers`. |
| `crates/fabric-daemon/src/federation/mod.rs` | Re-exports the pairing API. |
| `crates/fabric-daemon/src/federation/discovery.rs` | Renamed `peers` to `static_peers` to match the new field name. |
| `crates/fabric-daemon/src/wire/handlers.rs` | New `handle_federation_pair_request` async handler. |
| `crates/fabric-daemon/src/wire/protocol.rs` | New match arm routing `federation_pair_request` to the handler. |
| `crates/fabric-daemon/src/wire/mod.rs` | Auto-pair at wire-server startup, gated on `[federation].enabled` and a non-empty `static_peers`. |
| `crates/fabric-frame-transport/src/schema.rs` | New `federation_pair_request` request type in the schema registry, with required `peers` field and `array` type. **This file is in `fabric-frame-transport`, not `fabric-daemon`; the change is a 3-line data-only addition that is genuinely required to make pairing reachable through the existing wire protocol. Treat it as in-scope-by-necessity.** |
| `crates/fabric-integration-tests/tests/federation_pairing.rs` | **New.** End-to-end tests for the runtime pair path. |
| `docs/acceptance/A2-TWO-DEVICE-PAIRING.md` | **New.** This file. |

## Out-of-scope-by-necessity deviation

Adding `federation_pair_request` to the schema registry required
editing `crates/fabric-frame-transport/src/schema.rs`. This file is
outside the strict scope defined for the lane (`federation/`, `wire/`,
integration tests, `docs/acceptance/`, `config/settings.toml`).
The change is a 3-line data-only addition to a static `&[&str]` table
— it does not change the behaviour of any other message type — and
without it the schema validator rejects the new message type with
code `UNKNOWN_TYPE` before the handler ever runs. Treat the change as
required infrastructure for the new wire message, in the same
category as editing the wire dispatcher or the federation module.
