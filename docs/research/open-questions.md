# Open Questions

## Architecture

- What is the narrowest canonical graph core that can represent compute/data/I/O without becoming a universal ontology?
- Which node/port metadata is stable across all OSes, and which belongs in adapter extensions?
- Can graph transactions remain understandable when a requested link expands into several transformed subgraphs?
- What parts of coordinator state require consensus versus local authority and eventual reconciliation?

## Same-host and device fabric

- Which NVIDIA/AMD/Intel and OS combinations permit practical cross-process/cross-VM GPU sharing beyond capture?
- Can a second GPU’s encoder/copy engine help without PCIe transfer and synchronization costs exceeding benefit?
- What PCIe P2P/IOMMU paths are actually supported on the user’s current and future boards?
- Where can vhost-user/virtiofs/IVSHMEM be generalized safely beyond their initial device types?

## Real-time

- Which resource reservations measurably protect ASIO/Ableton on Windows under VM/agent load?
- Can networked DSP ever be admitted at musician-monitoring latency on ordinary wired LAN, or only for look-ahead/offline nodes?
- What is the right clock authority when display, audio interface, MacBook audio and remote nodes all differ?
- How should GPU render/encode/copy contention be admitted for 120/144-Hz HDR routes?

## Surfaces

- Which Windows applications expose reliable RAIL/RemoteApp behavior outside enterprise server assumptions?
- How should local proxy windows preserve modal ownership, z-order and accessibility trees?
- Can protected video or UAC flows be detected before the user encounters a blank surface?
- How many proxy windows can share an atlas before damage tracking and interaction become worse than dedicated streams?

## Compute/data

- Which workload boundaries provide enough semantic information for correct relocation without application rewrite?
- Can trace-derived region formation outperform static developer annotations robustly?
- What consistency models are needed beyond immutable objects and single-writer authority?
- How should the runtime price interference imposed on a foreground game/audio island?
- What execution can be rematerialized from intent instead of migrated as opaque state?

## Product

- Is Phenotype Fabric the correct public name and repo boundary, or should it become the deeper NVMS successor?
- What minimum packaged slice has standalone adoption value outside the AGSLAG ecosystem?
- Which components can be distributed without requiring users to run a central server?
