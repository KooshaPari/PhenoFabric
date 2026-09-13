# Go Reference Adapter Integration Guide

## Overview

The Go reference adapter (`cmd/wire/` + `cmd/checker/`) communicates with the
Rust `fabric-daemon` over TCP. The daemon listens on `127.0.0.1:9400` and
accepts line-delimited JSON messages.

Two protocol layers exist:

1. **Daemon simple JSON** -- `{"type": "<msg_type>", ...}\n` (native format)
2. **Spec 025 WireEnvelope** -- structured envelope wrapping payloads with
   metadata (used by `cmd/wire` TCPTransport)

The Go `cmd/wire` package implements WireClient/WireServer interfaces over TCP.
The `cmd/checker` uses this for replan requests instead of shelling out to
`fabric-graph-cli` via `os/exec`.

## Wire Protocol Messages

### Envelope (spec 025)

Every wire message is a `WireEnvelope`:

```json
{
  "envelope_id": "<32-hex-char UUIDv4>",
  "tenant_id": "ops-phenotype-default",
  "msg_type": "<message_type>",
  "payload": { ... },
  "signature": "",
  "sent_at_unix_ms": 1725900001000
}
```

### Message Types

| `msg_type` | Direction | Purpose | Response |
|---|---|---|---|
| `probe.request` | Go -> Daemon | Request node CapabilityDescriptor | `probe.response` |
| `probe.response` | Daemon -> Go | Return descriptor JSON | -- |
| `replan.request` | Go -> Daemon | Topology-driven failover replan | `replan.response` |
| `replan.response` | Daemon -> Go | Outcome: `replaced` / `no_replacement` / `error` | -- |
| `surface.invalidate` | Daemon -> Go | Push lease invalidation (fire-and-forget) | -- |
| `heartbeat` | Bidirectional | Liveness probe | `heartbeat_ack` |
| `error` | Either | Error envelope (codec-generated) | -- |

### Daemon Simple JSON (direct TCP)

| `type` | Purpose |
|---|---|
| `health_check` | Health check |
| `topology_request` | Get topology |
| `routes_request` | Get routes |
| `capabilities_request` | Get capabilities |
| `compile_request` | Multihop route compile (needs `source`, `destination`) |
| `heartbeat` | Liveness |

## Example: Go Client at localhost:9400

```go
package main

import (
    "context"
    "fmt"
    "log"
    "time"

    "github.com/phenotype/fabric/cmd/wire"
)

func main() {
    target := wire.NodeAddress{Scheme: "tcp", Host: "127.0.0.1", Port: 9400}
    client := wire.NewTCPTransport()
    defer client.Close()

    ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
    defer cancel()

    // Probe request (spec 025 envelope).
    probeEnv, _ := wire.NewEnvelopeFromPayload(
        wire.MsgTypeProbeRequest, "ops-phenotype-default",
        wire.ProbeRequest{RequestID: "11223344556677889900aabbccddeeff"},
    )
    resp, err := client.Send(ctx, target, probeEnv)
    if err != nil {
        log.Fatalf("probe: %v", err)
    }
    fmt.Printf("Probe response: msg_type=%s\n", resp.MsgType)

    // Replan via wire.
    rr, err := wire.ReplanViaWire(ctx, client, target, "ops-phenotype-default",
        topoJSON, intentJSON, oldPlanJSON, []string{"host-1"})
    if err != nil {
        log.Fatalf("replan: %v", err)
    }
    fmt.Printf("Outcome: %s\n", rr.Outcome)
}
```

## Cross-Language Parity Testing

Go and Rust share test fixtures in `cmd/wire/testdata/` (8 JSON files) to
verify wire-format compatibility.

1. **Go round-trip tests** (`cmd/wire/wire_test.go`): unmarshal fixture, re-marshal, verify determinism
2. **Rust fixture verification** (`crates/fabric-capability/examples/verify_fixtures.rs`): load same fixtures, verify serde deserialization

| Layer | Tests | Focus |
|---|---|---|
| Rust surface_runtime | 11 | Wire reason mapping, invalidation creation |
| Go wire constructors | 18 | Envelope round-trip, reason mapping |
| Go checker (E2E + unit) | 27 | Trust, replan, resource checks |
| **Total** | **56** | |

```bash
cd cmd/wire && go test -v ./...          # Go wire tests
cd cmd/checker && go test -v ./...       # Go checker tests
cargo test -p fabric-capability          # Rust fixture verification
```

## Build Instructions

### Go

```bash
cd phenotype-fabric
cd cmd/checker && go build -o ../../bin/checker .
cd ../wire && go build ./...
```

### Rust (fabric-daemon)

```bash
cargo build -p fabric-daemon --release
cargo run -p fabric-daemon -- start       # Default: 127.0.0.1:9400
cargo run -p fabric-daemon -- health      # Check health
cargo test -p fabric-daemon               # Daemon tests
```

### Smoke Test

```bash
# Terminal 1: Start daemon.
cargo run -p fabric-daemon -- start

# Terminal 2: Health check.
echo '{"type":"health_check"}' | nc 127.0.0.1 9400

# Terminal 3: Run checker with wire replan.
bin/checker -descriptor host.json -manifest app.yaml \
    -daemon-addr 127.0.0.1:9400 \
    -topology topo.json -intent intent.json -old-plan plan.json
```

## References

- `specs/025-wire-transport/spec.md` -- wire transport contract
- `architecture/spec-024-025-wire-mapping.md` -- Rust to Go wire mapping
- `docs/adr/0029-rust-vs-go-port-policy.md` -- Go is canonical for wire transport
- `docs/adr/0028-testdata-verification-pattern.md` -- fixture verification pattern
- `crates/fabric-daemon/src/wire/` -- Rust wire server
- `cmd/wire/` -- Go wire transport package
- `cmd/checker/` -- Go capability checker
