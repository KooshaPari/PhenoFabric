# Functional Requirements

Requirement IDs are stable trace keys. Each requirement maps to one or more source-intent IDs in `intent/002-human-to-requirement-map.md` and to verification rows in `verification/requirements-traceability-matrix.md`.

## Product shell and discovery

| ID | Requirement | Priority |
|---|---|---|
| PF-FR-001 | The system shall provide a single packaged UI, CLI, SDK, and agent-facing API across supported platforms. | P0 |
| PF-FR-002 | The system shall discover physical devices, realms, sessions, seats, surfaces, resources, and endpoint capabilities. | P0 |
| PF-FR-003 | The system shall preserve stable IDs across reconnects and distinguish identity from transient address or display order. | P0 |
| PF-FR-004 | The system shall present both a simplified workspace UI and an expert graph/patchbay UI. | P0 |
| PF-FR-005 | The system shall serialize, version, activate, diff, and roll back workspace graphs. | P0 |
| PF-FR-006 | The system shall expose health, pressure, and route status without requiring users to inspect each backend. | P0 |

## Graph and routing

| ID | Requirement | Priority |
|---|---|---|
| PF-FR-010 | Every connectable capability shall be represented by typed ports with format, timing, security, and locality metadata. | P0 |
| PF-FR-011 | A desired link shall be distinct from its compiled route. | P0 |
| PF-FR-012 | Route compilation shall search locality tiers from least expensive to most remote, subject to policy. | P0 |
| PF-FR-013 | The compiler shall eliminate unnecessary copy, serialization, codec, color-conversion, resampling, and relay stages. | P0 |
| PF-FR-014 | The compiler shall produce a route explanation with considered alternatives and rejected constraints. | P1 |
| PF-FR-015 | Route changes shall be transactional with prepare, validate, commit, abort, and rollback. | P0 |
| PF-FR-016 | Recursive capture/route loops shall be detected and blocked. | P0 |
| PF-FR-017 | Routes shall replan when topology/capabilities change, but shall not silently violate hard constraints. | P0 |

## Input and focus

| ID | Requirement | Priority |
|---|---|---|
| PF-FR-020 | A central broker shall route physical and virtual keyboard, mouse, controller, pen, touch, and MIDI input. | P0 |
| PF-FR-021 | Input focus shall be governed by exclusive leases and monotonically increasing fencing tokens. | P0 |
| PF-FR-022 | Focus transfer shall synthesize releases for all held keys/buttons before revoking the old route. | P0 |
| PF-FR-023 | Absolute desktop pointer and raw-relative capture modes shall be explicit and mutually safe. | P0 |
| PF-FR-024 | Users shall independently cycle input focus, output focus, or both. | P0 |
| PF-FR-025 | A local break-glass input path shall remain available. | P0 |
| PF-FR-026 | Agents shall not acquire human input focus without an explicit grant or policy. | P0 |

## Displays, windows, and surfaces

| ID | Requirement | Priority |
|---|---|---|
| PF-FR-030 | The system shall support takeover, extend, portal, mirror/pin, and audio-only presentation modes. | P0 |
| PF-FR-031 | A sink device may advertise a panel as a dynamic virtual monitor for another realm. | P0 |
| PF-FR-032 | The system shall prefer semantic per-application remoting when compatible and fall back to pixel proxies. | P1 |
| PF-FR-033 | Pixel-proxy mode shall preserve owned-window relationships, modals, popups, z-order intent, and minimize semantics. | P1 |
| PF-FR-034 | The system shall support mixed-DPI coordinate translation and raw/semantic text input. | P1 |
| PF-FR-035 | Protected or unavailable surfaces shall fail explicitly and offer a whole-desktop or out-of-band fallback. | P0 |
| PF-FR-036 | Surface location shall be independent from process and compute location. | P0 |
| PF-FR-037 | The system shall allow an agent to publish a surface without forcing it visible or focused. | P0 |

## Video, HDR, and color

| ID | Requirement | Priority |
|---|---|---|
| PF-FR-040 | Display capability descriptors shall include resolution, scale, refresh, VRR, bit depth, primaries, transfer functions, luminance, and ICC/EDID data where available. | P0 |
| PF-FR-041 | Video links shall negotiate raw or encoded formats based on locality, source intent, sink capability, and resource pressure. | P0 |
| PF-FR-042 | Workload profiles shall distinguish desktop-text, gaming, color-critical, and video cadence priorities. | P0 |
| PF-FR-043 | HDR metadata shall be preserved where possible and explicit tone/gamut mapping shall be recorded where not. | P0 |
| PF-FR-044 | Encoder, decoder, copy-engine, VRAM, PCIe, and memory-bandwidth pressure shall influence route selection. | P0 |
| PF-FR-045 | The system shall expose frame pacing, capture, encode, network, decode, composition, and display-stage telemetry. | P0 |
| PF-FR-046 | Quality degradation shall follow declared profiles rather than uncontrolled queue growth. | P0 |

## Audio and MIDI

| ID | Requirement | Priority |
|---|---|---|
| PF-FR-050 | Audio, MIDI, OSC, microphone, and control surfaces shall be first-class graph port types. | P0 |
| PF-FR-051 | Audio routes shall carry sample format, rate, channels, clock domain, target latency, and xrun policy. | P0 |
| PF-FR-052 | Local routes shall avoid encoding/resampling when compatible direct/shared paths exist. | P0 |
| PF-FR-053 | Network routes shall estimate clock drift and apply bounded elastic buffering and asynchronous sample-rate conversion when required. | P1 |
| PF-FR-054 | Audio and video may share synchronization intent but shall retain independent buffering and transport. | P0 |
| PF-FR-055 | MIDI/control events shall be timestamped and not tunneled as generic USB unless device fidelity requires it. | P1 |
| PF-FR-056 | The system shall provide protected RT audio islands for Ableton-class workloads. | P0 |

## Compute and data plane

| ID | Requirement | Priority |
|---|---|---|
| PF-FR-060 | The system shall inventory CPU topology, NUMA, caches, accelerators, memory domains, storage, NICs, PCIe paths, thermals, and power state. | P0 |
| PF-FR-061 | Tasks and objects shall declare capabilities, locality, security, consistency, deadline, and resource constraints. | P0 |
| PF-FR-062 | Placement shall minimize predicted total completion cost including queueing, compute, state movement, data transfer, synchronization, result return, contention, and risk. | P0 |
| PF-FR-063 | The runtime shall track object identity, versions, authority, replicas, caches, and residency. | P0 |
| PF-FR-064 | The runtime shall move compute to data or data to compute based on measured total cost. | P0 |
| PF-FR-065 | Placement granularity may reach task, function, syscall, operation, or accelerator-kernel boundaries where interposition exists. | P2 |
| PF-FR-066 | The runtime shall fuse repeated fine-grained decisions into execution regions and split regions when conditions diverge. | P1 |
| PF-FR-067 | The runtime shall support prefetch, replication, speculative execution, cancellation, and cache eviction policies. | P1 |
| PF-FR-068 | The runtime shall support mobility levels: surface projection, semantic handoff, application checkpoint, process checkpoint, VM migration, and workload rematerialization. | P1 |
| PF-FR-069 | The runtime shall never represent remote memory or VRAM as uniform local capacity without visible latency/consistency semantics. | P0 |

## QoS and contention

| ID | Requirement | Priority |
|---|---|---|
| PF-FR-070 | Work shall be classified into RT0, RT1, RT2, RT3, RT4, Bulk, and Background service classes. | P0 |
| PF-FR-071 | Resource reservations shall extend across CPU, GPU, encoder/decoder, memory bandwidth, storage I/O, network, thermal, and power constraints. | P0 |
| PF-FR-072 | Background builds, tests, inference, and agents shall be subordinate to declared foreground deadlines. | P0 |
| PF-FR-073 | The scheduler shall evict or relocate competing work before distributing tightly coupled game/audio critical paths. | P0 |
| PF-FR-074 | The system shall support explicit degradation ladders for frame rate, chroma, bitrate, resolution, and optional effects. | P0 |
| PF-FR-075 | The system shall expose admission-control failure rather than overcommit a hard real-time route. | P0 |

## Security, agents, recovery, and integration

| ID | Requirement | Priority |
|---|---|---|
| PF-FR-080 | Device enrollment shall use mutually authenticated identities and revocable short-lived credentials. | P0 |
| PF-FR-081 | Permissions shall be capability-scoped by view, input, audio, microphone, clipboard, file, USB, migration, compute, and focus actions. | P0 |
| PF-FR-082 | Privileged capture, injection, driver, and KVM components shall be isolated from the UI/control process. | P0 |
| PF-FR-083 | Clipboard and file objects shall preserve MIME/types, provenance, sensitivity, expiry, and loop suppression. | P0 |
| PF-FR-084 | Important physical devices shall support an out-of-band recovery route. | P0 |
| PF-FR-085 | Ephemeral realms shall have owner, purpose, TTL, resource budget, fallback console, and cleanup policy. | P0 |
| PF-FR-086 | The system shall emit stable events and evidence references for AgilePlus, thegent, Tracera, SessionLedger, AGSLAG, ShareCLI, NVMS, and ledgers without assuming shared storage. | P0 |
| PF-FR-087 | All focus, route, privilege, migration, and placement decisions shall be auditable. | P0 |
| PF-FR-088 | The system shall continue local operation during loss of cloud/coordination services where policy permits. | P0 |
