# Architecture Index

The architecture is intentionally split by authority and hot-path behavior.

| Document | Purpose |
|---|---|
| [`system-context.md`](system-context.md) | End-to-end planes, entities, and locality. |
| [`control-plane.md`](control-plane.md) | Registry, desired state, leases, plans, failover. |
| [`graph-runtime.md`](graph-runtime.md) | Nodes, ports, links, graph transactions, RT schedule. |
| [`route-compiler.md`](route-compiler.md) | Candidate generation, stage elimination, cost and fallback. |
| [`scheduler.md`](scheduler.md) | Compute placement, adaptive regions, admission and pressure. |
| [`object-plane.md`](object-plane.md) | Object identity, residency, consistency, transfer, spill. |
| [`realtime-audio.md`](realtime-audio.md) | PipeWire/JACK-derived audio/MIDI/clock design. |
| [`video-hdr.md`](video-hdr.md) | Capture, memory domains, codec policy, HDR/color/frame pacing. |
| [`input.md`](input.md) | evdev/uinput/libei, focus, raw relative input, platform endpoints. |
| [`surface-proxy.md`](surface-proxy.md) | Semantic remoting, pixel proxies, window graphs, recursion. |
| [`files-clipboard.md`](files-clipboard.md) | Typed clipboard and send/sync/mount/object semantics. |
| [`network-lan-wan.md`](network-lan-wan.md) | Direct, relay, congestion, clock, loss and path migration. |
| [`platform-linux.md`](platform-linux.md) | Linux adapters and privileged boundaries. |
| [`platform-windows.md`](platform-windows.md) | Windows capture/display/input/audio/workload adapters. |
| [`platform-macos.md`](platform-macos.md) | macOS sink/source, AppKit proxy, VideoToolbox/Metal/CoreAudio. |
| [`security.md`](security.md) | Identity, capabilities, helpers, audit and OOB isolation. |
| [`protocols.md`](protocols.md) | Control/data protocols and message classes. |
| [`api.md`](api.md) | Public UI/CLI/SDK/agent API. |
| [`state-machines.md`](state-machines.md) | Route, lease, realm, object, and workspace lifecycles. |
| [`schemas/`](schemas/) | Machine-readable contracts and examples. |

## Architectural test

A design change is acceptable only if it can answer:

1. What authority owns the state?
2. What graph projection contains it?
3. What is the finest and normal execution granule?
4. Where is the data authoritative and resident?
5. What physical boundaries are crossed?
6. What copies/transforms/queues are added?
7. What deadline and quality contract applies?
8. What security capability permits it?
9. What happens on partition, crash, or stale token?
10. What evidence would prove the claim?
