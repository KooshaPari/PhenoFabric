# Phenotype Fabric — Product Requirements Document

**Version:** 0.1  
**Status:** Specified  
**Date:** 2026-08-28  
**Working repository:** `phenotype-fabric`  
**Product class:** Distributed interactive operating environment and heterogeneous runtime fabric

---

## Product vision

Phenotype Fabric makes a heterogeneous collection of physical computers, VMs, operating-system sessions, accelerators, storage, displays, audio devices, input devices, and network locations feel like one coherent computer. It provides a packaged UI, CLI, SDK, and agent API, while compiling each requested connection or execution decision onto the most local and efficient valid mechanism.

The product must remain useful before its hardest research goals are solved. Its first releases unify discovery, workspaces, focus, surfaces, real-time media, and workload placement through existing proven adapters. Later releases introduce a data-locality plane, adaptive execution-region fusion/fission, and selected fine-grained interposition.

## Problem statement

Current multi-device and multi-VM workflows are fragmented:

- software KVMs route input but not displays or audio;
- remote desktops transport displays but treat each machine as a separate island;
- Looking Glass is excellent for one same-host VFIO path but does not govern a fleet;
- audio graph tools provide powerful node/port routing but do not generalize to compute, windows, files, or heterogeneous resources;
- cluster schedulers optimize batch tasks but ignore the human’s live seat, color, audio, input, and application-window experience;
- per-window remoting makes applications look local but generally leaves compute/data placement fixed;
- VM, host, PCIe, LAN, WAN, and KVM-over-IP paths expose different UX and APIs;
- background agent workloads can steal CPU, GPU, memory bandwidth, storage I/O, encoder sessions, and network capacity from games or Ableton;
- agents may create N VMs or services without a safe, trivial way to publish them to the user;
- physically available compute and memory are underused because execution is bound to the realm where a process began.

The product must collapse the user-facing complexity without collapsing the physical distinctions required for correct performance decisions.

## Target users

### Primary

1. **Power user with several devices and VMs**  
   Wants every realm to be reachable as naturally as a local application, with one keyboard/mouse, workspace switching, and a laptop usable as another display.

2. **Developer running agent fleets**  
   Wants background builds, tests, game E2E runs, inference, and transient VMs scheduled across all available hardware without degrading the foreground seat.

3. **Musician/creator**  
   Wants Ableton Live 12 Suite, audio interfaces, MIDI, plugins, windows, and monitoring protected by explicit real-time deadlines and routable across selected endpoints.

4. **Gamer**  
   Wants native-feeling frame pacing, HDR, high refresh, low input latency, controller fidelity, and no unexplained latency spikes when the rest of the fabric is busy.

5. **Operator of bench and remote machines**  
   Wants OS-level control when healthy and BIOS/crash access through out-of-band paths.

### Secondary

- AI agents that publish surfaces, request compute, and inspect results;
- platform engineers studying heterogeneous placement and shared-memory paths;
- small studios/labs with mixed Windows, Linux, and macOS workstations;
- ecosystem services such as ShareCLI, thegent, AgilePlus, Tracera, and the ledgers.

## Jobs to be done

| ID | Job |
|---|---|
| JTBD-01 | Move between desk and couch while preserving the same logical workspace in one action. |
| JTBD-02 | Use the MacBook as a third monitor, independent computer, or input seat without rebuilding configuration. |
| JTBD-03 | Switch keyboard/mouse/microphone input focus and display/audio output focus independently or atomically. |
| JTBD-04 | Place a Windows application beside native macOS applications while it continues executing on Windows. |
| JTBD-05 | Let an agent create a VM and publish a desktop/window/report without taking user focus. |
| JTBD-06 | Keep gaming or Ableton stable while agent builds/tests/inference consume remaining resources. |
| JTBD-07 | Route work to the best CPU/GPU/memory/storage location based on total completion cost, not nominal device speed. |
| JTBD-08 | Use same-host shared memory/DMA instead of encoding or network stacks where physically possible. |
| JTBD-09 | Degrade quality predictably rather than producing stalls, audio xruns, or hidden contention. |
| JTBD-10 | Recover any important physical machine through a fallback console when its OS path fails. |
| JTBD-11 | Understand why a route or placement was chosen and what would improve it. |
| JTBD-12 | Replay and prove performance, policy, and resource decisions later. |

## Product principles

1. **Graph-native:** resources and interactive endpoints are nodes with typed ports and policy-bearing links.
2. **Topology-compiled:** abstract intent is recompiled when topology, load, or capability changes.
3. **Locality-maximal:** the runtime searches from direct/local paths outward before paying network/codec costs.
4. **Deadline-aware:** hard/soft real-time and throughput classes are explicit.
5. **Data-aware:** execution follows data when movement dominates compute benefit.
6. **Adaptive granularity:** atomic decisions are possible; fused regions are normal.
7. **Semantics before pixels:** use native/semantic application protocols when available; pixel projection is fallback.
8. **One shell, replaceable adapters:** product coherence does not require one implementation technology.
9. **Non-stealing agents:** automation requests attention; it does not seize the seat.
10. **Inspectable optimization:** route plans, cost terms, pressure, and degradation are visible.

## User experience

### Unified graph canvas

The primary UI shows devices, realms, applications, resources, and endpoints as nested nodes. Users may:

- drag an application window to a display sink;
- connect a microphone to a VM or application;
- route an audio bus to local speakers and a remote recorder;
- pin an agent test surface to a corner;
- move a workload or execution region to another resource;
- inspect selected path stages, latency budget, color/audio format, and fallback;
- save the resulting graph as a workspace.

### Simple workspace controls

Powerful internals must not force patchbay use for ordinary actions. The shell provides:

- global input cycle;
- global output cycle;
- coupled seat switch;
- local emergency-return chord;
- workspace launcher;
- surface palette;
- device/realm search;
- route health badge;
- “why here?” placement explanation.

### Agent experience

Agents use typed APIs to:

- request a realm with constraints and TTL;
- publish a desktop/window/terminal/report;
- request compute or data placement;
- subscribe to completion and health;
- request—not force—user attention;
- attach evidence identifiers to runs and artifacts.

## Epics

### E1 — Packaged product shell

A single installer and coherent UI/CLI/API across Linux, Windows, and macOS.

Stories:
- E1.1: device and realm discovery;
- E1.2: graph canvas and simple workspace mode;
- E1.3: command palette and global hotkeys;
- E1.4: adapter lifecycle and capability diagnostics;
- E1.5: signed update, rollback, and recovery;
- E1.6: accessibility and keyboard-only operation.

### E2 — Universal I/O graph

Typed node/port/link model for video, audio, HID, MIDI, clipboard, file, storage, terminal, telemetry, and compute/data.

Stories:
- E2.1: graph object model and schema;
- E2.2: format/capability negotiation;
- E2.3: clock and latency metadata;
- E2.4: graph transactions and rollback;
- E2.5: workspace serialization;
- E2.6: recursive route prevention.

### E3 — Locality and route compiler

Discover the least-expensive valid route for every link.

Stories:
- E3.1: same-process and same-OS paths;
- E3.2: same-host cross-realm shared-memory/virtio paths;
- E3.3: PCIe/P2P/DMA capability graph;
- E3.4: LAN direct media/data paths;
- E3.5: WAN congestion-controlled paths;
- E3.6: OOB hardware fallback;
- E3.7: dynamic replan without unsafe mid-stream changes.

### E4 — Seats, input, and output focus

Treat input and output focus as lease-governed transactions.

Stories:
- E4.1: evdev/uinput/libei host broker;
- E4.2: Windows and macOS endpoint agents;
- E4.3: pressed-key/button ledger;
- E4.4: raw-relative versus absolute-pointer modes;
- E4.5: fenced focus leases;
- E4.6: independent/coupled input-output switching;
- E4.7: emergency local control.

### E5 — Surfaces and application presentation

Make desktops and applications appear wherever requested.

Stories:
- E5.1: full desktop takeover/mirror;
- E5.2: virtual display extend mode;
- E5.3: semantic application forwarding;
- E5.4: pixel-proxy window fallback;
- E5.5: owned-window graph for popups/modals;
- E5.6: mixed DPI, IME, drag/drop, clipboard;
- E5.7: protected-surface fallback.

### E6 — Real-time audio, MIDI, and media quality

Protect professional audio and interactive media.

Stories:
- E6.1: PipeWire/JACK-style audio graph;
- E6.2: WASAPI/ASIO/CoreAudio adapters;
- E6.3: network clock estimation and ASRC;
- E6.4: timestamped MIDI/OSC;
- E6.5: RT scheduling/reservations;
- E6.6: xrun and latency instrumentation;
- E6.7: independent audio/video buffering and synchronization.

### E7 — HDR, color, and high-refresh video

Preserve source intent and adapt to sink capabilities.

Stories:
- E7.1: EDID/ICC/display capability inventory;
- E7.2: 8/10-bit and HDR metadata negotiation;
- E7.3: workload-specific 4:4:4/4:2:2/4:2:0 policy;
- E7.4: tone/gamut mapping;
- E7.5: frame pacing, VRR, and refresh negotiation;
- E7.6: hardware encode/decode pressure accounting;
- E7.7: capture-to-enqueue and true glass-to-glass measurement.

### E8 — Heterogeneous compute and data fabric

Place work and objects across all connected resources.

Stories:
- E8.1: CPU/GPU/NPU/memory/storage/network inventory;
- E8.2: immutable object references and residency;
- E8.3: task/process/function/kernel placement;
- E8.4: data prefetch and replication;
- E8.5: adaptive fusion/fission;
- E8.6: speculative execution and cancellation;
- E8.7: checkpoint, migration, handoff, and rematerialization hierarchy;
- E8.8: “why here?” cost explanation.

### E9 — Real-time and scaled resource governance

Prevent throughput work from destroying foreground performance.

Stories:
- E9.1: service classes RT0–Bulk;
- E9.2: CPU affinity and core shielding;
- E9.3: GPU/encoder/decoder reservations;
- E9.4: memory-bandwidth and storage-I/O pressure;
- E9.5: network pacing and class isolation;
- E9.6: graceful quality degradation;
- E9.7: thermal/power-aware scheduling.

### E10 — Agent and ephemeral realm lifecycle

Safely expose automation-created environments.

Stories:
- E10.1: short-lived enrollment and certificates;
- E10.2: TTL and cleanup;
- E10.3: surface publication;
- E10.4: non-stealing attention;
- E10.5: fallback console;
- E10.6: per-principal capability grants;
- E10.7: evidence and run identifiers.

### E11 — Security, identity, audit, and evidence

Protect an unusually privileged system.

Stories:
- E11.1: mutual identity and pairing;
- E11.2: route capabilities and least privilege;
- E11.3: privileged helper isolation;
- E11.4: clipboard/file/mic/camera policy;
- E11.5: focus and route audit;
- E11.6: signed telemetry/evidence;
- E11.7: Tracera/SessionLedger integration.

### E12 — Ecosystem integration

Expose stable contracts without absorbing adjacent products.

Stories:
- E12.1: ShareCLI process/resource adapter;
- E12.2: NVMS runtime inventory;
- E12.3: thegent TaskSpec placement contract;
- E12.4: AgilePlus requirement/work/evidence IDs;
- E12.5: Tracera evidence links;
- E12.6: AGSLAG resource budget/utility constraints;
- E12.7: ledger event contracts.

## Success metrics

### User-facing

- A desk-to-couch workspace transition requires one explicit action and no manual reconnection.
- Any enrolled realm is visible through at least one healthy surface path and one recovery path.
- Warm focus switch p95 is under 50 ms for input and under 250 ms to first valid visible frame for prepared output routes.
- A user can game or run Ableton while adversarial agent workloads run without violating declared frame/audio deadlines.
- The MacBook can act as a third display at its negotiated native/logical mode and can independently retain local applications.
- Route and placement decisions are explainable in plain language.

### Technical

- Same-host paths avoid codecs and network stacks whenever an eligible shared-memory/device path exists.
- No stuck keys/buttons across randomized focus-switch stress.
- Stale fencing tokens cannot produce input or output after lease transfer.
- Audio RT tests report zero xruns within accepted load envelopes.
- HDR/color tests preserve declared metadata or report an explicit conversion.
- All significant copies, transformations, queue waits, and data movements are measured.
- Adaptive fusion reduces fine-grained routing overhead below the local-benefit threshold.
- Failures preserve the previous route or move to a declared fallback without hidden split-brain behavior.

## Non-goals

- Pretending distributed RAM/VRAM is uniform local memory.
- Arbitrary transparent live migration of every process across different operating systems.
- Routing every syscall remotely by default.
- Reimplementing every codec, remote desktop, KVM, filesystem, or audio server.
- Making hard real-time guarantees over uncontrolled WAN links.
- Giving agents implicit access to human input, microphone, files, credentials, or focus.
- Replacing AgilePlus, thegent, AGSLAG, Tracera, SessionLedger, ShareCLI, or infrastructure deployment tools.
- Building a universal ontology merely to simplify diagrams.

## Release strategy

The product is staged so each release has independent value:

1. **R0 — Measurement and adapter lab:** topology inventory, benchmark harness, Looking Glass/PipeWire/evdev adapters.
2. **R1 — Unified local seat:** graph UI, workspaces, input/output focus, local VM and physical-machine surfaces.
3. **R2 — LAN/WAN fabric:** direct/relay transport, virtual displays, files/clipboard, agent realm publication.
4. **R3 — Real-time media:** audio/MIDI graph, HDR/color policy, resource reservations.
5. **R4 — Compute/data placement:** object residency, process/task placement, prefetch, build/test/inference demonstrations.
6. **R5 — Adaptive granularity:** selective interposition, execution regions, fusion/fission, predictive placement.
7. **R6 — Mature ecosystem product:** signed packaging, stable APIs, compatibility certification, evidence loop.
