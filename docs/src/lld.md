# Low-Level Design Overview

This document defines the hot-path rules and component contracts. Detailed component files live under `architecture/`.

## Runtime processes

### `pf-coordinator`

Unprivileged service responsible for desired graph, registry, topology epochs, plans, leases, policies, and audit references.

### `pf-endpointd`

Per-device agent. Mostly unprivileged; communicates with narrowly scoped helpers:

- `pf-input-helper`
- `pf-display-helper`
- `pf-audio-helper`
- `pf-resource-helper`
- `pf-oob-adapter`

### `pf-rt-engine`

Optional local real-time process that owns preallocated rings, scheduling cycles, clock mapping, and hot-path graph execution. No dynamic allocation or blocking control-plane I/O on RT threads.

### `pf-objectd`

Local object/residency manager. Owns immutable object mappings, cache index, spill/restore, transfer and prefetch.

### `pf-workerd`

Executes admitted task/execution-region plans and reports events/results.

### `pf-shell`

Platform-native UI that uses the same public API as CLI/agents.

## Graph transaction

```text
client proposes GraphPatch(base_version=N)
  coordinator validates identity and schema
  compiler resolves affected links
  endpoints PREPARE resources
  RT admission validates reservations
  coordinator allocates fencing token N+1
  endpoints COMMIT(token=N+1, activation_time=T)
  previous route drains/releases
  desired graph version becomes N+1
```

Commit messages are idempotent. Endpoint state includes `last_applied_token`.

## Hot-path message classes

| Class | Delivery | Typical transport | Loss behavior |
|---|---|---|---|
| Input/MIDI | ordered where semantics require, otherwise timestamped datagrams | shared ring / QUIC datagram | reconcile state; never replay stale motion |
| Audio | timestamped cyclic buffers | shared ring / RTP-like datagrams | bounded concealment; no unbounded retransmission |
| Video | frame/chunk datagrams plus reliable control | KVMFR/raw/codec over QUIC | discard late deltas, request recovery frame |
| Control | reliable ordered | local IPC / QUIC stream | retry idempotently |
| Object metadata | reliable | control RPC | content hash verifies |
| Object payload | direct bulk stream/RDMA/shared mapping | selected per route | resumable/chunk verified |
| Telemetry | sampled/batched | local ring + stream | shed before RT impact |

## Route-stage interface

Each stage advertises:

```yaml
stage:
  id: string
  kind: capture|transform|copy|encode|transport|decode|compose|inject|store
  input_types: []
  output_types: []
  locality: L0-L8
  fixed_cost_us: number
  per_byte_cost: number
  queue_model: string
  resource_claims: []
  timing_class: RT0|RT1|RT2|RT3|RT4|Bulk|Background
  security_properties: []
  failure_modes: []
```

The compiler may compose stages only when formats, clocks, memory domains, ownership, and policy are compatible.

## Placement cost model

For candidate node `n` and operation/region `r`:

\[
C(n,r) =
T_q + T_c + T_s + T_d + T_x + T_y + T_o
+ \lambda_r R_{deadline}
+ \lambda_p R_{pressure}
+ \lambda_f R_{failure}
+ \lambda_e E
\]

Where:

- `T_q`: predicted queue delay;
- `T_c`: compute time;
- `T_s`: state preparation/migration;
- `T_d`: required input data movement;
- `T_x`: synchronization/coordination;
- `T_y`: result movement;
- `T_o`: routing/interposition overhead;
- `R_deadline`: deadline miss risk;
- `R_pressure`: impact on protected workloads;
- `R_failure`: availability/security/topology uncertainty;
- `E`: optional energy/thermal cost.

Hard constraints prune before scoring.

## Adaptive granularity

Each fine-grained placement point has a local baseline and remote candidate. Distribution is eligible only when:

\[
T_{local} - T_{remote\_compute}
>
T_{marshal} + T_{state} + T_{data} + T_{queue} + T_{sync} + T_{return} + margin
\]

The runtime caches decisions by code/data/topology/pressure signature.

### Fusion

Fuse adjacent decisions when:

- same destination repeatedly wins;
- mutable state is shared;
- boundary overhead is material;
- dependencies are dense;
- failure semantics align.

### Fission

Split a region when:

- an accelerator-specific kernel appears;
- data locality diverges;
- parallelism becomes available;
- resource pressure changes;
- a deadline/quality class changes;
- a small result follows a large remote-resident input.

## RT execution

RT threads:

- use preallocated buffers;
- avoid locks or use bounded lock-free structures;
- never perform DNS, disk I/O, allocation, logging formatting, or policy evaluation;
- operate on immutable compiled schedules;
- report counters through lock-free rings;
- fail safe when a node exceeds its budget.

Control threads build a new schedule and atomically swap it at a cycle boundary.

## Clock model

Each endpoint exports a monotonic clock sample and quality. The runtime maintains affine mappings:

\[
t_{global} = a \cdot t_{local} + b
\]

Audio devices remain their own clock domains. Drift is handled by bounded buffers and ASRC; wall-clock synchronization alone is not treated as sample-clock identity.

## Buffer model

```yaml
buffer:
  id: uuid
  media_type: video|audio|object|input
  memory_domain: cpu|gpu|shared|device
  device_id: optional
  format: typed
  immutable: true
  ownership: producer|consumer|shared
  sync:
    kind: fence|event|sequence|cycle
    handle: opaque
```

Copy count and domain transitions are measured per route.

## Security

Every route request carries:

- principal identity;
- capability token;
- source/sink grants;
- purpose and TTL;
- topology epoch;
- desired constraints;
- audit correlation ID.

A route cannot infer permissions from network reachability.

## API baseline

The public API supports:

- list/query nodes, ports, objects, workspaces, resources, and routes;
- propose/validate/commit graph patches;
- request placement;
- publish surfaces;
- request attention;
- explain route/placement;
- create/destroy ephemeral realms;
- subscribe to events/metrics;
- export diagnostic/evidence bundles.

See `architecture/api.md` and `architecture/schemas/`.
