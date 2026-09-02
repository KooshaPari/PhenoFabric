# Phenotype Fabric

**Working name:** `phenotype-fabric`  
**Document set version:** 0.1.0-draft  
**Created:** 2026-08-28  
**Status:** Research and architecture baseline; not an implementation-complete claim  
**Scope owner:** Runtime substrate for distributed compute, data locality, interactive I/O, surfaces, transport selection, and real-time quality of service

> A packaged, all-in-one computing environment that makes connected physical machines, operating-system sessions, virtual machines, accelerators, displays, audio devices, input devices, storage, and remote locations appear as one coherent interactive computer—while preserving the physical facts needed to make good placement decisions.

## The product in one sentence

Phenotype Fabric exposes one graph, one workspace model, and one UI/API while compiling each link onto the least-expensive valid execution path: direct calls, shared memory, DMA-BUF, IVSHMEM/KVMFR, virtio/vhost, PCIe peer paths, LAN transports, WAN transports, or out-of-band hardware.

## What is unified

The product shell unifies:

- device, realm, session, application, process, task, data-object, and surface discovery;
- keyboard, pointer, touch, pen, controller, MIDI, microphone, audio, video, clipboard, file, and storage routing;
- application and desktop presentation independent of execution location;
- heterogeneous CPU, GPU, NPU, FPGA, memory, storage, and network placement;
- hard and soft real-time reservations;
- topology-aware transport and codec selection;
- workspace snapshots such as **desk**, **couch**, **bench**, **music**, **game**, and **remote**;
- human and agent control through the same typed API;
- evidence, telemetry, audit, and reproducible benchmark output.

The implementation is intentionally **not monolithic**. It wraps and composes proven data planes—Looking Glass/KVMFR, PipeWire/JACK, evdev/uinput/libei, RDP/RAIL, Xpra/Waypipe, Sunshine/Moonlight-class media, platform virtual-display APIs, QUIC, virtio, SMB/NFS/SFTP, Syncthing-class synchronization, and KVM-over-IP—behind stable capabilities.

## What is not unified

This project must not become a universal database or absorb adjacent products:

- **AgilePlus** owns governed intent, requirements, work packages, acceptance criteria, and evidence gates.
- **thegent** owns labor/agent delegation and execution intent.
- **AGSLAG** owns economic allocation, priority, risk, and whether work deserves resources.
- **Tracera** owns cross-artifact traceability and evidence relationships.
- **SessionLedger** owns durable operational session history and replay.
- **ShareCLI** remains a standalone, useful OS-adjacent process/agent runtime and supervisor; it may become a privileged fabric client and local adapter.
- **NVMS** is the natural ecosystem boundary for runtime inventory and isolation; this specification materially expands the substrate expected beneath it.
- **labs-compute** remains the bounded research incubator for experimental kernels, device mesh techniques, and prototypes until a capability proves a stable product contract.

These systems share identifiers and event contracts but retain independent authority and storage.

## Non-negotiable design rules

1. **One product surface, many optimized data planes.**
2. **Remote does not mean network.** Same-process, same-OS, same-host, PCIe-local, LAN, WAN, and out-of-band routes are distinct compilation targets.
3. **Every stage is guilty until justified.** Copies, serialization, codecs, color conversion, resampling, kernel crossings, and relays are optional graph stages, never assumed.
4. **Atomic placement is a capability floor, not the default granularity.** The runtime may interpose at syscall/function/kernel/I/O granules, then fuse neighboring decisions into execution regions when distribution overhead dominates.
5. **Presentation and execution are independent.** A window may live on the MacBook while its process, DSP graph, GPU kernels, and data execute elsewhere.
6. **Physics remains visible to the optimizer.** Location transparency must never become performance opacity.
7. **Real-time islands preempt background throughput.** Ableton, live monitoring, MIDI, input, and game frame deadlines are protected from compilers, agents, inference, indexing, and bulk transfers.
8. **Color and audio semantics are first-class.** HDR metadata, bit depth, primaries, transfer functions, clock domains, latency, and sample accuracy are not “stream settings.”
9. **Agents may publish surfaces but may not steal focus.**
10. **Every ambitious claim requires a benchmark, falsification condition, and fallback.**

## Reference user environment

The first validation environment is deliberately hostile rather than idle:

- one main multi-GPU PC hosting a Linux/virtualization environment and multiple disjoint-I/O VMs;
- one to five bench-test PCs in the room;
- other PCs across the WAN;
- a 2021 M1 Pro MacBook Pro used both as its own computer and as a third display/seat;
- a Samsung C27HG70 2560×1440 high-refresh HDR monitor;
- concurrent agent development that may compile Rust, run games for end-to-end tests, execute inference, or generate transient VMs;
- foreground gaming or Ableton Live 12 Suite use;
- movement between desk and couch without manually reconstructing sessions.

## Documentation map

- [`PRD.md`](PRD.md) — product requirements and user journeys.
- [`SPECIFICATION.md`](SPECIFICATION.md) — normative system specification and conformance rules.
- [`HLD.md`](HLD.md) — high-level system architecture.
- [`ALD.md`](ALD.md) — architecture/abstraction lowering from intent to physical routes.
- [`LLD.md`](LLD.md) — low-level component and hot-path design.
- [`UX_SPECIFICATION.md`](UX_SPECIFICATION.md) — simple workspace and expert graph interaction contracts.
- [`PLAN.md`](PLAN.md) and [`ROADMAP.md`](ROADMAP.md) — phased program and product/research horizons.
- [`TRACEABILITY.md`](TRACEABILITY.md) — exact prompt-to-requirement-to-evidence chain.
- [`FUNCTIONAL_REQUIREMENTS.md`](FUNCTIONAL_REQUIREMENTS.md) — traceable functional requirements.
- [`NON_FUNCTIONAL_REQUIREMENTS.md`](NON_FUNCTIONAL_REQUIREMENTS.md) — performance, quality, reliability, and security requirements.
- [`intent/`](intent/) — verbatim human prompts and formal intent synthesis.
- [`specs/`](specs/) — AgilePlus-shaped feature specifications, plans, and task catalogs.
- [`adr/`](adr/) — architecture decisions and rejected alternatives.
- [`architecture/`](architecture/) — component, protocol, platform, and graph details.
- [`sota/`](sota/) — 2026 state-of-the-art and competitive analysis.
- [`work/`](work/) — WBS, DAG, PERT, critical path, milestones, and research/build order.
- [`research/`](research/) — hypotheses, experiments, source discipline, and open questions.
- [`verification/`](verification/) — benchmark, fault, security, compatibility, and acceptance gates.
- [`ecosystem/`](ecosystem/) — product boundaries and event-contract integration.
- [`examples/`](examples/) — concrete graph/workspace examples.
- [`operations/`](operations/) — packaging, deployment, upgrades, SLOs, and recovery.
- [`risks/`](risks/) — feasibility, threat, licensing, and vendor-dependency registers.
- [`references/`](references/) — bibliography and source-confidence register.
- [`INDEX.md`](INDEX.md) and [`TREE.txt`](TREE.txt) — complete navigation and repository tree.
- [`VALIDATION_REPORT.md`](VALIDATION_REPORT.md) and [`MANIFEST.sha256`](MANIFEST.sha256) — package checks and file hashes.

## Honest status

This package is a **complete planning and specification baseline**, not proof that transparent arbitrary-binary fine-grained distributed execution has already been solved. The architecture deliberately separates:

- capabilities that can be integrated now;
- capabilities requiring platform-specific implementation;
- research hypotheses requiring measurement;
- mechanisms that should remain selective because their coordination cost is usually worse than local execution.

The first product can deliver substantial value without solving the hardest research layer. The user-facing shell, graph model, same-host fast paths, workspace switching, agent realm publication, and load-aware process/task placement are independently viable.
