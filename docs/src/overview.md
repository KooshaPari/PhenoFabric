# Phenotype Fabric

**A packaged, all-in-one computing environment that makes connected physical machines, operating-system sessions, virtual machines, accelerators, displays, audio devices, input devices, storage, and remote locations appear as one coherent interactive computer---while preserving the physical facts needed to make good placement decisions.**

Phenotype Fabric exposes one graph, one workspace model, and one UI/API while compiling each link onto the least-expensive valid execution path: direct calls, shared memory, DMA-BUF, IVSHMEM/KVMFR, virtio/vhost, PCIe peer paths, LAN transports, WAN transports, or out-of-band hardware.

## What is unified

- Device, realm, session, application, process, task, data-object, and surface discovery
- Keyboard, pointer, touch, pen, controller, MIDI, microphone, audio, video, clipboard, file, and storage routing
- Application and desktop presentation independent of execution location
- Heterogeneous CPU, GPU, NPU, FPGA, memory, storage, and network placement
- Hard and soft real-time reservations
- Topology-aware transport and codec selection
- Workspace snapshots (desk, couch, bench, music, game, remote)
- Human and agent control through the same typed API
- Evidence, telemetry, audit, and reproducible benchmark output

## Design principles

1. **One product surface, many optimized data planes.**
2. **Remote does not mean network.** Same-process, same-OS, same-host, PCIe-local, LAN, WAN, and out-of-band routes are distinct compilation targets.
3. **Every stage is guilty until justified.** Copies, serialization, codecs, color conversion, resampling, kernel crossings, and relays are optional graph stages, never assumed.
4. **Atomic placement is a capability floor, not the default granularity.**
5. **Presentation and execution are independent.**
6. **Physics remains visible to the optimizer.**
7. **Real-time islands preempt background throughput.**
8. **Color and audio semantics are first-class.**
9. **Agents may publish surfaces but may not steal focus.**
10. **Every ambitious claim requires a benchmark, falsification condition, and fallback.**

## Book contents

- **Getting Started** -- Product requirements, specification, and glossary
- **Architecture** -- High-level design, abstraction layers, domain model, and requirements
- **Architecture Decision Records** -- All accepted and proposed ADRs
- **Research** -- Capability models, hypotheses, experiments, and graduation gates
- **Ecosystem** -- Integrations with AGSLAG, AgilePlus, thegent, Tracera, and more
- **Releases** -- Release notes and changelogs
