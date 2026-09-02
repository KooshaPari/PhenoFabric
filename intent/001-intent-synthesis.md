# Formal Intent Synthesis

## Core human intent

The user is not asking for another remote desktop, software KVM, VM console, audio router, cluster scheduler, or process migration demo in isolation.

The requested product is a **single packaged interactive computing plane** in which:

1. all physical devices, VMs, sessions, and remote nodes are discoverable;
2. every display, window, keyboard, pointer, microphone, speaker, controller, MIDI endpoint, file system, and storage object can be connected through a common graph;
3. a window can appear on any chosen device while its process and compute remain elsewhere;
4. execution can be placed across all available CPU/GPU/memory/storage resources;
5. the smallest interceptable unit may be routed when profitable;
6. adjacent decisions are normally fused because distribution itself has cost;
7. same-process/same-OS/same-host/PCIe routes are aggressively different from LAN/WAN routes;
8. foreground gaming and Ableton receive protected real-time treatment even while agents run adversarial background work;
9. HDR, color, high refresh, audio quality, clock drift, and frame/sample deadlines are first-class;
10. human movement between desk and couch becomes a workspace/seat transition rather than a manual reconnection exercise;
11. agents can create N realms and publish useful surfaces without stealing control;
12. all complexity is hidden behind one coherent UI/API, not exposed as a bag of tools.

## Product interpretation

The most faithful interpretation is:

> A distributed operating/runtime fabric with a PipeWire/JACK-inspired graph model, a Citrix/RAIL-like surface layer, a Looking Glass/evdev-style same-host fast path, LAN/WAN media and object transports, and a heterogeneous compute/data scheduler whose granularity adapts to measured distribution cost.

This is broader than “location-transparent process migration.” Presentation, I/O, data, and execution are separately movable.

## Why “syscall granule” is retained but not literalized

The user clarified that the most atomic granule must be possible, but not commonly used. Therefore:

- the architecture must not hard-code process/job as the minimum placement unit;
- interposition points may include function, syscall, I/O request, GPU kernel, graph node, or memory/object operation;
- each decision has an explicit routing tax;
- repeated choices are fused into execution regions;
- region fission occurs only when a different hardware/data/deadline optimum appears;
- WAN generally produces coarse regions; same-host shared-memory/PCIe may justify finer ones.

This preserves the ambition without pretending that RPC-per-syscall is desirable.

## User-facing truth

The user should experience:

```text
“I opened/moved/used an application here.”
```

The runtime may implement:

```text
window on MacBook
process in Windows VM
audio DSP pinned to protected PC cores
shader compile on another CPU
capture encode on spare GPU
assets cached near the process
agent analysis on remote GPU
input sourced from current seat
```

The product must explain this only when asked.

## Technical truth

Underneath, the optimizer must never forget:

- data size and residency;
- mutable state and consistency;
- queueing and synchronization;
- PCIe/NUMA/root-complex topology;
- encoder/decoder/copy-engine contention;
- network RTT/jitter/loss/bandwidth;
- display and color capabilities;
- audio clock domains;
- OS/driver permission boundaries;
- migration and recovery cost;
- security grants;
- predicted future work;
- real-time deadlines;
- thermal and power state.

## Relationship to the existing ecosystem

The new fabric is the physical execution and I/O substrate, not the entire synthetic enterprise:

- AGSLAG determines whether and why resources should be allocated.
- AgilePlus formalizes intent, requirements, work packages, and acceptance evidence.
- thegent translates authorized work into labor/agent execution requests.
- Phenotype Fabric chooses physical placement, data movement, and interactive routes.
- ShareCLI supplies mature OS-adjacent process discovery, coalescing, pressure, FUSE, fleet, and supervision capabilities without losing its standalone product identity.
- SessionLedger preserves operational history.
- Tracera links requirements, decisions, runs, code, tests, and artifacts.
- NVMS remains the runtime-inventory/isolation product boundary; this work deepens its substrate contract.
- labs-compute contains risky prototypes until they prove a stable capability.

## Minimum coherent product

The minimum coherent product is not fine-grained distributed execution. It is:

1. packaged cross-platform shell;
2. common graph model;
3. device/realm/session/surface/resource discovery;
4. transactional focus and workspace switching;
5. Looking Glass/evdev/PipeWire local integration;
6. LAN/WAN desktop/application adapters;
7. agent realm publication;
8. load-aware process/task placement;
9. evidence and benchmark harness.

This delivers immediate value and creates the measurement substrate needed for harder research.
