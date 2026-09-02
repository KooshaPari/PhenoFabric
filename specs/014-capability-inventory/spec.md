# Capability Inventory and Topology Probe

## Meta

- **ID:** `014-capability-inventory`
- **Created:** 2026-09-01
- **State:** specified
- **Scope:** Detect, classify, sign, and publish a graph-native
  capability inventory covering CPU/NUMA/cache/memory, GPU/NPU/codec/
  display/PCIe, audio/MIDI/input/storage/NIC endpoints, and link-layer
  RTT/jitter/loss/bandwidth and copy paths. First R0 *runtime* code
  (Rust library + Go reference adapter via NVMS).
- **Release gate:** R0 — must be complete before PF-WP-020 (route
  compiler) begins.
- **Requirement traces:** `PF-FR-007`, `PF-FR-008`, `PF-FR-009`
- **Intent traces:** `INT-P012`–`INT-P014`
- **Work package:** `PF-WP-010` (6 tasks: PF-WP-010.01 through
  PF-WP-010.06)
- **Depends on:** `013-fabric-program-baseline` (PF-WP-000)

## Context

Fabric's placement engine cannot make good decisions without a
trustworthy, current picture of what the system has. The inventory
must:
- Discover hardware, not just kernel-exposed abstractions
- Express topology as a graph (nodes, edges, weights)
- Sign every delta so a malicious peer cannot inject false capabilities
- Be cheap to refresh (sub-second for the small set, seconds for full)
- Be reproducible across reboots and power-state changes

The "deliberately hostile" reference environment (multi-GPU Linux + 1–5
bench PCs + WAN peers + MacBook + Samsung HDR + concurrent agent
builds + foreground Ableton) is the validation target.

## Problem Statement

Current tooling (lscpu, lstopo, nvidia-smi, lsusb, pactl, etc.) emits
heterogeneous, non-versioned, un-signed output. The Fabric placement
engine needs one canonical, signed, versioned capability descriptor
per node that includes:

- **Compute:** CPU model, cores, threads, NUMA domains, L1/L2/L3
  cache topology, memory size and bandwidth, hyperthread pairs, TDP
- **Accelerator:** GPU model, VRAM, compute capability, NPU presence,
  hardware codec support, AV1/H.264/H.265/VP9 encode-decode matrix
- **Display:** connected displays, EDID, HDR capability, refresh rate,
  pixel clock, color space, multi-display topology, virtual displays
- **PCIe:** device tree, lane allocation, generation, peer-to-peer
  capability
- **Audio:** input/output devices, sample rates, channels, JACK /
  PipeWire / CoreAudio / WASAPI backend, MIDI ports
- **Input:** keyboards, mice, touchscreens, gamepads, tablets, raw
  evdev devices
- **Storage:** mounted filesystems, IOPS/bandwidth per device, SSD
  vs HDD, network mounts
- **Network:** NICs, link speed, MTU, RSS queues, RDMA capability,
  zerocopy support
- **Link metrics:** RTT, jitter, loss, bandwidth to each adjacent node
  in the topology, and the copy path (shared memory, PCIe P2P, RDMA,
  loopback, LAN, WAN)

## Goals

- One signed, versioned `CapabilityDescriptor` JSON per node
- One topology graph representation that includes weighted edges
  (RTT, jitter, loss, bandwidth, copy-domain)
- Sub-100ms cold probe for the small (CPU/NUMA/cache) set
- Sub-1s full probe
- Reproducible: same hardware + same kernel state + same probe version
  → byte-identical descriptor (modulo timestamps)
- Cross-language binding: Rust core, Go reference adapter, C FFI for
  ShareCLI

## Non-Goals

- Cross-node synchronization (that's PF-WP-020 — route compiler
  consumes these descriptors, not this WP)
- Capability authorization (that's spec 009 — security)
- Dynamic re-probing on kernel events (this WP provides the library;
  the daemon that calls it lives in PF-WP-080 or later)
- macOS / Windows probes in R0 (POSIX-first per `PRD.md`; macOS /
  Windows coverage is R1+)

## User and System Outcomes

After this WP completes, the Fabric repo has:
- `crates/fabric-capability/` Rust crate with the descriptor and
  probe API
- A `go.mod` + `cmd/capprobe` Go reference adapter that exercises
  the C FFI
- A C FFI shim in `crates/fabric-capability-ffi/`
- Integration with `nanovms` for the inventory backend (the
  provisional name per ADR-0020)
- A `verification/capability-compatibility-matrix.md` listing which
  hardware is verified to produce a stable descriptor
- A `verification/capability-benchmark.md` showing the probe
  cold/warm/full latencies on the reference environment

## Functional Requirements

- `PF-FR-007` — Each Fabric node publishes a signed capability
  descriptor with versioned schema.
- `PF-FR-008` — Probe latency is bounded per the evidence contract.
- `PF-FR-009` — Topology includes weighted edges (RTT, jitter, loss,
  bandwidth, copy-domain).

## Technical Approach

### PF-WP-010.01 — Define capability descriptor schemas

- Author `architecture/schemas/capability-descriptor-v1.json` as the
  canonical schema. Schema fields:
  - `node_id` (UUID v7 for time-ordering)
  - `epoch` (monotonic counter, incremented on any change)
  - `schema_version` (1.0.0)
  - `probed_at` (RFC3339 timestamp)
  - `probe_version` (semver of the probe binary)
  - `topology_hash` (BLAKE3 of the topology subgraph)
  - `capabilities` (object: compute, accelerator, display, pcie,
    audio, input, storage, network, link)
  - `signatures` (array: { key_id, alg, sig, signed_at })
- Validate with `jsonschema` crate in CI; export a typed Rust struct
  via `schemars`.
- Acceptance: every test in `crates/fabric-capability/tests/descriptor_*.rs`
  passes; schema validates.

### PF-WP-010.02 — Probe CPU/NUMA/cache/memory topology

- Implement the small set in Rust using:
  - `/proc/cpuinfo` for cores, threads, frequencies
  - `lscpu` JSON output (parse, not exec) for NUMA + cache
  - `/sys/devices/system/node/node*/` for NUMA distances
  - `getconf` or direct `sysinfo(2)` for memory
- Reject the probe if NUMA is not exposed (older kernels). The
  library returns `Error::Unsupported` and the adapter surfaces a
  clear "NUMA not exposed" message.
- Measure: cold probe (first call after process start) < 100ms;
  warm probe (subsequent) < 1ms.
- Acceptance: `cargo bench -p fabric-capability --bench cpu_probe`
  shows the latency on the reference environment, captured in
  `verification/capability-benchmark.md`.

### PF-WP-010.03 — Probe GPU/NPU/codec/display/PCIe topology

- Detect GPUs via `lspci` + `nvidia-smi` (NVIDIA), `rocm-smi` (AMD),
  `intel_gpu_top` (Intel). Optional: `vulkaninfo` for compute caps.
- Detect displays via `xrandr` (X11) or `wlr-randr` (Wayland) or
  read `/sys/class/drm/` directly. Parse EDID for HDR + color space.
- Detect PCIe via `lspci -tv` parsed as a tree. Lane count, generation,
  P2P capability (via `CONFIG_PCI_P2PDMA`).
- Detect codecs via `/dev/dri/renderD*` introspection + EGL/Vulkan
  queries. Fallback: a hard-coded matrix per vendor (NVIDIA, AMD,
  Intel, Apple) with confidence values.
- Acceptance: the descriptor includes a non-empty `accelerator` and
  `display` block on the reference environment.

### PF-WP-010.04 — Probe audio/MIDI/input/storage/NIC endpoints

- Audio: use the same backend as the spec 007 (PipeWire / JACK /
  CoreAudio / WASAPI) — for R0, POSIX means PipeWire only. List
  sinks, sources, sample rates, channels.
- MIDI: parse `/dev/snd/` + `amidi -l` + PipeWire MIDI bridge.
- Input: list `/dev/input/event*` and decode names from
  `/sys/class/input/`.
- Storage: `lsblk` JSON + `iotop`-style measurement (read+write 1MB
  and time it). Detect network mounts via `mount` output.
- NIC: `ip -j link` + `ethtool` for link speed and RDMA capability.
- Acceptance: descriptor includes all 4 sections populated on the
  reference environment.

### PF-WP-010.05 — Measure link RTT/jitter/loss/bandwidth and copy paths

- For each adjacent node, measure:
  - RTT: ICMP (with consent) or TCP handshake (fallback). 100 samples,
    record p50/p95/p99/worst.
  - Jitter: stddev of the RTT samples.
  - Loss: send 1000 ICMP/UDP probes, record dropped count.
  - Bandwidth: `iperf3` 5s single-stream + 5s multi-stream. Record
    p50 throughput.
  - Copy path classification: based on (src_node, dst_node, src_cap,
    dst_cap, link_metric), classify the available copy paths:
    - `L0 shared-memory` (same NUMA node)
    - `L1 cross-numa-shm`
    - `L2 pcie-p2p` (peer-to-peer DMA)
    - `L3 rdma` (RoCE / iWARP / IB)
    - `L4 loopback`
    - `L5 lan-tcp`
    - `L6 wan-tcp`
    - `L7 oob` (Wake-on-LAN, IPMI, KVM-over-IP)
  - The output is a `LinkMetrics` struct per edge.
- Acceptance: `verification/capability-link-metrics.md` shows the
  measurement methodology and a sample result.

### PF-WP-010.06 — Sign and publish descriptor deltas

- Sign with Ed25519 (signing key per node, generated at install time,
  stored in `~/.config/fabric/keys/`).
- Publish deltas (epoch incremented) via:
  - Local: filesystem watcher in `~/.local/share/fabric/`
  - LAN: mDNS / DNS-SD announcement
  - WAN: optional (default off; requires explicit consent)
- Acceptance: a tampered descriptor is rejected by the verifier
  in `fabric-capability::verify`. Test: `crates/fabric-capability/tests/signature.rs`.

## Risks and Open Questions

- **Risk:** Probe latency on a heavily-loaded system (Ableton running,
  agent builds active) is much worse than the budget. **Mitigation:**
  The budget is per-call, but the daemon can schedule probes during
  idle windows; this WP ships the library, not the scheduler.
- **Risk:** `nanovms` integration surface is undefined until ADR-0020
  is resolved. **Mitigation:** this WP wraps `nanovms` via its
  existing public API; if NVMS is later absorbed into Fabric, only the
  import path changes.
- **Open:** Should the C FFI be `unsafe extern "C"` or use `cbindgen`
  with a safe wrapper? Default to `cbindgen` for safety.

## Exit Gate (R0)

This WP is done when:
1. All 6 sub-tasks above are complete.
2. `cargo test -p fabric-capability` and
   `cargo test -p fabric-capability-ffi` are green.
3. The reference environment produces a stable, signed descriptor.
4. The benchmark + compatibility matrix evidence files are committed.
5. `work/build-order.md` is updated to mark PF-WP-010 as Done.
