# Architecture / Abstraction-Level Design

`ALD` is used here as **Architecture- and Abstraction-Level Design**: the stable conceptual layers between human intent and physical mechanisms. It complements the system-wide HLD and component/hot-path LLD.

## A0 — Human and agent intent

Examples: “move my seat to the couch,” “show this Windows application on the Mac,” “run this game E2E without disturbing Ableton,” or “place this operation on the best available hardware.” Intent states desired outcomes and hard/soft constraints, not a product or transport.

## A1 — Semantic entities

```text
Principal → Workspace → Device → Realm → Session → Seat → Surface
                  └→ Resource / Task / ExecutionRegion / Object
```

These identities are stable across transient network addresses, process IDs and display order. Cross-product IDs link AgilePlus work, thegent tasks, Tracera evidence and ledgers.

## A2 — Desired graph

Nodes expose typed ports. Links carry intent, service class, format/quality, security, authority, consistency, mobility and fallback policy.

```text
source port ── desired link/policy ──▶ sink port
```

A desired link is not a stream and not a direct pointer to one backend.

## A3 — Compiled graph

The compiler expands a desired link into stages and prunes invalid alternatives:

```text
capture? → memory transition? → transform? → encode? → transport?
→ decode? → compose? → inject/store?
```

Every stage may disappear. The selected graph includes resource reservations, clocks, fencing and rollback.

## A4 — Execution and object regions

Tasks may be decomposed to processes, functions, syscalls, operations or accelerator kernels where an interposition point exists. Fine-grained decisions fuse into regions when sharing state/destination and split when locality, parallelism, deadline or capability diverges.

Objects expose authority, version, consistency and residency. Location transparency is a user/API abstraction; the runtime cost model remains location-aware.

## A5 — Locality backends

| Tier | Boundary | Example mechanisms |
|---|---|---|
| L0 | same process | direct calls, borrowed buffers, captured GPU graph |
| L1 | same OS | SHM/memfd, DMA-BUF, GPU handles, PipeWire, local IPC |
| L2 | same host/cross-realm | KVMFR/IVSHMEM, virtio/vhost, virtiofs, uinput/virtual devices |
| L3 | PCIe/coherent fabric | P2P DMA, BAR, GPUDirect, real CXL capabilities |
| L4 | isolated same-host | loopback/direct media only when sharing is impossible |
| L5–L6 | LAN/Wi-Fi | QUIC/RTP-like, UCX/RDMA where real, raw/light/codec media |
| L7 | WAN | congestion-controlled media, coarse compute, direct/relay |
| L8 | preboot/failed OS | physical KVM-over-IP, serial/power/virtual media |

## A6 — Physical resources

CPUs, NUMA nodes, caches, RAM/VRAM, GPUs/NPUs/FPGAs, codecs, copy engines, NICs, storage, displays, interfaces and clocks are modeled from probes and benchmarks. Product/model names are hints, not guarantees.

## A7 — Evidence and adaptation

The runtime observes actual stage cost, deadline misses, quality, resource pressure and failures. It updates route/scheduler models, fuses/fissions regions and emits evidence references. Tracera/AgilePlus decide semantic acceptance; Fabric owns physical execution facts.

## Invariants across levels

1. Hard constraints are never weakened silently during lowering.
2. Every compiled edge traces back to desired intent and source prompt/requirement.
3. A higher-level identity is not replaced by a lower-level address or PID.
4. A lower-level failure has an explicit rollback/fallback at the next stable abstraction.
5. No abstraction claims physics disappeared; it only hides management complexity.
