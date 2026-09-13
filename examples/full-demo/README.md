# Fabric Full Demo

End-to-end demonstration of the Phenotype Fabric daemon, topology loading, route
compilation, and frame streaming over the wire protocol.

## What This Demo Does

1. **Builds a 6-node topology** across 3 locality tiers (GPU L3, CPU L1, remote L6)
2. **Loads the topology** into the fabric-daemon coordinator
3. **Compiles routes** for a sample intent requesting GPU compute
4. **Streams a test frame** over the wire transport protocol
5. **Validates the checker manifest** against a node's capability descriptor

## Topology Layout

```
  Tier L3 (PCIe P2P)          Tier L1 (Same NUMA)         Tier L6 (LAN)
  ┌─────────────────┐        ┌─────────────────┐        ┌─────────────────┐
  │  gpu-node-alpha  │◄──────►│  cpu-node-one    │◄──────►│  remote-node-a  │
  │  GPU: RTX 4090   │        │  CPU: 16 cores   │        │  CPU: 4 cores   │
  │  RAM: 24 GB      │        │  RAM: 32 GB      │        │  RAM: 8 GB      │
  └─────────────────┘        └─────────────────┘        └─────────────────┘
         │                           │                           │
         ▼                           ▼                           ▼
  ┌─────────────────┐        ┌─────────────────┐        ┌─────────────────┐
  │  gpu-node-beta   │◄──────►│  cpu-node-two   │◄──────►│  remote-node-b  │
  │  GPU: A100       │        │  CPU: 8 cores   │        │  CPU: 2 cores   │
  │  RAM: 40 GB      │        │  RAM: 16 GB     │        │  RAM: 4 GB      │
  └─────────────────┘        └─────────────────┘        └─────────────────┘
```

## Quick Start

### Option A: Shell Script (requires fabric-daemon + fabric binaries)

```bash
cd examples/full-demo
chmod +x run_demo.sh
./run_demo.sh
```

### Option B: Rust Binary (self-contained)

```bash
cargo run --example full-demo
```

The Rust binary exercises the same APIs as the shell script but runs entirely
in-process: it builds the topology, compiles routes, starts a wire server for
5 seconds, accepts one connection, serves a health check, then shuts down
gracefully.

## Files

| File | Description |
|------|-------------|
| `topology.json` | 6-node sample topology with edges and capability descriptors |
| `intent.json` | Placement intent: GPU compute, audio sink, 8 GB memory |
| `manifest.json` | Checker manifest for validating node fitness |
| `run_demo.sh` | Shell script that orchestrates the full demo via daemon + netcat |
| `src/main.rs` | Rust binary that exercises the same flow in-process |
| `Cargo.toml` | Cargo manifest for the Rust example binary |

## Wire Protocol Messages

The shell script sends these JSON messages over TCP (newline-delimited):

```jsonc
{"type": "health_check"}
// → {"status":"healthy","uptime_s":0,"topology_epoch":1,...}

{"type": "topology_request"}
// → {"type":"probe_response","nodes":[...],"edges":[...]}

{"type": "routes_request"}
// → {"type":"routes_response","routes":[...]}
```

## Requirements

- **Shell script**: `fabric-daemon` and `fabric` binaries on PATH, `nc` (netcat)
- **Rust binary**: Just `cargo run` — all dependencies pulled from workspace
