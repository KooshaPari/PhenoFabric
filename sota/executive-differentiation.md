# Executive Differentiation

## Category definition

Phenotype Fabric is a packaged **distributed interactive operating environment**: a universal node/port/link graph that jointly compiles human/agent I/O, application surfaces, compute tasks, object residency and resource policy across local processes, operating systems, VMs, PCIe, LAN, WAN and OOB paths.

It is not just:

- a remote desktop;
- a software KVM;
- an audio patchbay;
- a VM console;
- a cluster scheduler;
- a distributed filesystem;
- a process supervisor;
- or an agent orchestrator.

## User-visible edge

| User expectation | Existing category behavior | Fabric behavior |
|---|---|---|
| “This MacBook is my third PC display.” | Install a display product and separately solve input/audio/files. | Sink advertises display/audio/input capabilities; one workspace link provisions the best source display and routes the seat. |
| “Put this Windows app beside Mac apps.” | Remote full desktop, RemoteApp-specific infrastructure, or early pixel proxy. | Semantic app route first, pixel proxy fallback, full desktop only when required. |
| “I moved to the couch.” | Reconnect tools and rearrange windows manually. | Activate `couch`; the seat and surfaces relocate while execution stays or moves only when beneficial. |
| “Do not let agents ruin Ableton/game latency.” | Process priority or manual shutdown. | RT admission, CPU/GPU/encoder/memory/network reservations, and active relocation of competing work. |
| “Use all my hardware.” | Run jobs per host or build a separate cluster. | Unified resource inventory, data-aware placement, adaptive regions and explicit locality costs. |
| “Why is this here?” | Opaque vendor heuristics. | Explain candidate plans, constraints, rejected alternatives, uncertainty and measured result. |

## Technical edge

1. **Cross-plane optimization:** execution, data and presentation are solved together rather than by independent products.
2. **Locality compilation:** every codec, copy, serialization, transition and hop must justify itself.
3. **Adaptive granularity:** atomic placement is possible, while fusion/fission avoids paying atomic routing overhead by default.
4. **Real-time protection:** service classes extend through CPU, GPU, copy engines, codecs, memory, storage, network, thermals and power.
5. **Semantic-to-pixel ladder:** native application semantics when available, universal pixel projection when not.
6. **Authority-safe ecosystem:** shared IDs/events connect AgilePlus, thegent, Tracera, ledgers, AGSLAG, NVMS and ShareCLI without a universal database.
7. **One product shell:** users install and operate one system even though specialized data planes remain modular.

## Honest boundary

The product can hide location from the user; it cannot repeal latency, coherence, driver, secure-surface, anti-cheat, audio-clock or platform-permission constraints. Its credibility comes from exposing and optimizing those constraints, not claiming they disappeared.
