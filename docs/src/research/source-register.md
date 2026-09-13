# Source Register

| ID | Source | Class | URL | Used for | Confidence | Checked |
|---|---|---|---|---|---|---|
| SRC-001 | PipeWire: Graph Scheduling | primary documentation | https://docs.pipewire.org/page_scheduling.html | Graph activation, RT threads, buffers/IO in shared memory. | high | 2026-08-28 |
| SRC-002 | PipeWire: Latency | primary documentation | https://docs.pipewire.org/page_latency.html | Latency propagation, quantum/rate and mismatch. | high | 2026-08-28 |
| SRC-003 | JACK2 | primary project | https://jackaudio.org/ | Sample-synchronous low-latency audio/MIDI graph. | high | 2026-08-28 |
| SRC-004 | NetJACK2 | primary source | https://github.com/jackaudio/jack2 | Network JACK transport and buffering. | high | 2026-08-28 |
| SRC-005 | Looking Glass documentation | primary documentation | https://looking-glass.io/docs/ | KVMFR/IVSHMEM path and configuration. | high | 2026-08-28 |
| SRC-006 | Linux dma-buf | kernel documentation | https://docs.kernel.org/driver-api/dma-buf.html | Shared DMA buffers and fences. | high | 2026-08-28 |
| SRC-007 | QEMU IVSHMEM | primary documentation | https://www.qemu.org/docs/master/system/devices/ivshmem.html | Inter-VM/host shared memory device. | high | 2026-08-28 |
| SRC-008 | QEMU vhost-user | primary documentation | https://www.qemu.org/docs/master/interop/vhost-user.html | Shared-memory backend protocol. | high | 2026-08-28 |
| SRC-009 | XDG RemoteDesktop portal | standard documentation | https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html | Wayland-authorized screen/input/clipboard session. | high | 2026-08-28 |
| SRC-010 | libei/EIS | primary documentation | https://libinput.pages.freedesktop.org/libei/ | Emulated input mediation. | high | 2026-08-28 |
| SRC-011 | Legion | primary project/papers | https://legion.stanford.edu/ | Logical regions and mapping. | high | 2026-08-28 |
| SRC-012 | StarPU | primary project/papers | https://starpu.gitlabpages.inria.fr/ | Heterogeneous task scheduling/data replicas. | high | 2026-08-28 |
| SRC-013 | PaRSEC | primary project/papers | https://icl.utk.edu/parsec/ | Distributed task graph runtime. | high | 2026-08-28 |
| SRC-014 | Ray Core objects | primary documentation | https://docs.ray.io/en/latest/ray-core/objects.html | Shared-memory object store, references and transfer. | high | 2026-08-28 |
| SRC-015 | UCX | primary project | https://openucx.org/ | Transport abstraction over SHM/TCP/RDMA/GPU. | high | 2026-08-28 |
| SRC-016 | libfabric | primary project | https://ofiwg.github.io/libfabric/ | Fabric communication abstraction. | high | 2026-08-28 |
| SRC-017 | NIXL | primary source | https://github.com/ai-dynamo/nixl | Heterogeneous transfer library. | medium-high | 2026-08-28 |
| SRC-018 | NVIDIA GPUDirect RDMA | vendor technical documentation | https://docs.nvidia.com/cuda/gpudirect-rdma/ | GPU direct device/network access and topology constraints. | high | 2026-08-28 |
| SRC-019 | CXL Consortium | standards organization | https://www.computeexpresslink.org/ | CXL protocol/specification family. | high | 2026-08-28 |
| SRC-020 | Microsoft RDP/RemoteApp | vendor technical documentation | https://learn.microsoft.com/en-us/windows-server/remote/remote-desktop-services/ | Windows session/application remoting. | high | 2026-08-28 |
| SRC-021 | WSLg | primary source | https://github.com/microsoft/wslg | RAIL/VAIL-style Linux GUI integration. | high | 2026-08-28 |
| SRC-022 | Sunshine | primary project | https://docs.lizardbyte.dev/projects/sunshine/ | Open host for Moonlight-compatible streaming. | high | 2026-08-28 |
| SRC-023 | Moonlight | primary project | https://moonlight-stream.org/ | Client ecosystem and capabilities. | high | 2026-08-28 |
| SRC-024 | Parsec | vendor product/docs | https://parsec.app/ | Commercial interactive desktop transport. | medium-high | 2026-08-28 |
| SRC-025 | Deskflow | primary source | https://github.com/deskflow/deskflow | Cross-platform software KVM. | high | 2026-08-28 |
| SRC-026 | Apple MacBook Pro 2021 specs | vendor technical specs | https://support.apple.com/kb/SP858 | M1 Pro display/media hardware baseline. | high | 2026-08-28 |
| SRC-027 | Samsung C27HG70 specs | vendor technical specs | https://www.samsung.com/us/computing/monitors/gaming/27-chg70-gaming-monitor-with-quantum-dot-lc27hg70qqnxza/ | Reference HDR display baseline. | medium-high | 2026-08-28 |
| SRC-028 | FreeRDP | primary project | https://www.freerdp.com/ | Open RDP/RAIL implementation. | high | 2026-08-28 |
| SRC-029 | Xpra | primary project | https://github.com/Xpra-org/xpra | Linux application/session forwarding. | high | 2026-08-28 |
| SRC-030 | Waypipe | primary project | https://gitlab.freedesktop.org/mstoeckl/waypipe | Wayland application proxy. | high | 2026-08-28 |
| SRC-031 | QUIC RFC 9000 | standard | https://www.rfc-editor.org/rfc/rfc9000 | Multiplexed secure transport. | high | 2026-08-28 |
| SRC-032 | RTP RFC 3550 | standard | https://www.rfc-editor.org/rfc/rfc3550 | Timestamped real-time media and feedback. | high | 2026-08-28 |
| SRC-033 | IEEE 1588/PTP | standard organization | https://standards.ieee.org/ieee/1588/6825/ | Precision time synchronization. | high | 2026-08-28 |
| SRC-034 | AgilePlus PRD/PLAN | user repository | https://github.com/KooshaPari/AgilePlus | Spec/work/evidence governance format. | high | 2026-08-28 |
| SRC-035 | ShareCLI README/ADRs | user repository | https://github.com/KooshaPari/sharecli | OS-adjacent agent runtime and scope boundary. | high | 2026-08-28 |

## Source policy

Primary specifications, kernel/project documentation and papers support mechanism claims. Vendor pages support product availability/capability descriptions but not comparative latency or quality. Community reports identify test cases; they do not establish SLOs. Every material product claim must be benchmarked on the target topology before automatic routing.
