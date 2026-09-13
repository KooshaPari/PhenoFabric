# Glossary

| Term | Definition |
|---|---|
| **Principal** | Human, agent, service, or automation identity allowed to request or operate resources. |
| **Device** | Physical computer, thin client, phone, display appliance, KVM appliance, or accelerator enclosure. |
| **Realm** | Independently executing OS environment: bare-metal OS, VM, container desktop, cloud machine, or sandbox. |
| **Session** | Logged-in graphical, terminal, or service session inside a realm. |
| **Seat** | Coherent interactive target comprising input focus and optional visual/audio endpoints. |
| **Surface** | Desktop, monitor, application window, popup tree, video plane, audio stream, terminal, report, or agent artifact. |
| **Endpoint** | Physical or virtual source/sink such as keyboard, GPU, display, microphone, speaker, storage mount, or MIDI port. |
| **Node** | Graph object that owns capabilities and ports. A node may represent a device, realm, process, application, transform, or hardware resource. |
| **Port** | Typed graph connection point with formats, timing, capacity, security, and locality constraints. |
| **Link** | Desired or active connection between compatible ports. |
| **Route** | A compiled implementation of one or more links through selected transports and transforms. |
| **Execution region** | Fused set of tasks/operations assigned as one placement unit to amortize routing and state movement. |
| **Fission** | Splitting an execution region when different hardware, contention, or deadline constraints justify finer placement. |
| **Fusion** | Combining neighboring placement decisions when they repeatedly choose the same node or share state. |
| **Object** | Versioned data item with identity, residency, mutability, consistency, and durability semantics. |
| **Residency** | Where an object or replica currently lives: RAM, VRAM, persistent storage, remote cache, or absent. |
| **Authority** | Which component owns the canonical mutable state or decision for an entity. |
| **Workspace** | Saved graph layout, routes, focus policy, display placement, quality profile, and resource reservations. |
| **Zone** | Physical user location such as main desk, couch, bench, studio, or remote office. |
| **Locality tier** | Same process, same OS, same host/different realm, same PCIe fabric, LAN, WAN, or out-of-band. |
| **Hard real-time island** | Work whose deadlines must be protected ahead of throughput work, such as live audio callbacks. |
| **Soft real-time** | Work with deadlines where occasional degradation is tolerable but latency spikes are harmful. |
| **Fencing token** | Monotonically increasing token that prevents stale focus/route holders from continuing after a lease changes. |
| **Semantic remoting** | Forwarding application/window objects or protocol state rather than encoding an entire desktop as pixels. |
| **Pixel proxy** | Native local proxy window whose remote application remains executing elsewhere and is captured as pixels. |
| **Intent mobility** | Recreating or reopening equivalent logical work in another location rather than migrating opaque process state. |
| **Rematerialization** | Recreating a workload from declarative state, checkpoints, artifacts, and credentials near the best resources. |
| **Transport compiler** | Optimizer that maps abstract links onto concrete shared-memory, device, network, codec, or hardware paths. |
| **Quality profile** | Workload-aware preferences for latency, frame rate, chroma, HDR, color accuracy, audio quality, and bandwidth. |
| **SOTA** | State of the art, including current products, protocols, research systems, and relevant primitives. |
