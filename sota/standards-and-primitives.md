# Standards and Primitive Map

## Purpose

The packaged product is built by composing standards and specialized mechanisms. This map prevents the product shell from inventing proprietary replacements where a stable primitive already exists.

| Domain | Preferred primitives | Adapter role | Non-negotiable caveat |
|---|---|---|---|
| Linux input | evdev, libevdev, uinput, libei/EIS, XDG portals | Acquire physical events, create stable virtual endpoints, obtain compositor-authorized capture/injection | Wayland consent and compositor capability cannot be bypassed honestly. |
| Windows input | Raw Input, SendInput, VHF, HID APIs | Acquire per-device input; initial injection; eventual true virtual devices | Privileged driver must remain small, signed and auditable. |
| macOS input | IOHID, CGEvent/Event Taps, Accessibility | Acquire/inject endpoint events | Secure Input and TCC are hard platform boundaries. |
| Linux media | PipeWire, SPA, DMA-BUF, DRM/KMS | Local graph, buffer negotiation and display/capture integration | Real-time callback paths cannot block on control policy. |
| Pro audio | JACK, ASIO, CoreAudio, MIDI/OSC, PTP concepts | Local DSP graph and endpoint clocks | Independent clocks require measured drift and ASRC/elastic buffering. |
| VM local | virtio, vhost-user, IVSHMEM/KVMFR, virtiofs, VFIO | Cross-realm input/media/object data planes | IOMMU/ACS/topology and guest cooperation determine what is possible. |
| GPU local | DMA-BUF, CUDA IPC, D3D shared resources, Metal IOSurface-like primitives | Share buffers and fences without codecs | Cross-vendor/cross-OS portability is limited; probe rather than assume. |
| LAN/WAN media | RTP/RTCP concepts, QUIC datagrams/streams, WebRTC/SRT lessons | Pacing, congestion control, loss recovery, NAT traversal | Audio/video/input need separate queues and policy. |
| Fabric data | UCX, libfabric, RDMA, GPUDirect, NIXL | High-performance object transfer | Registration, pinning and topology costs enter the optimizer. |
| Schema/API | Protobuf/gRPC, JSON Schema, OpenAPI, MCP adapter | Stable typed contracts for humans, agents and products | Cross-product authority remains separate despite shared IDs. |
| Observability | OpenTelemetry, Prometheus exposition, structured events | Stage timings, traces, SLO evidence | Instrumentation overhead is measured and disabled/aggregated on RT callbacks. |

## Rule

A primitive is selected because it removes stages or preserves semantics at a specific locality tier. It is not selected merely because it is fashionable or cross-platform.
