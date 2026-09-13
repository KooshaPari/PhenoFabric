# System and Platform Requirements

## Reference hardware

The initial lab must support:

- Linux virtualization host on the main PC;
- Windows 11 GPU-passthrough guest;
- RTX 3090 Ti and optional GTX 1080 Ti/secondary GPU paths;
- 64 GB host memory baseline;
- 1/2.5 GbE LAN baseline, with higher-speed paths treated as optional experiments;
- 2021 M1 Pro MacBook Pro client/sink;
- Samsung C27HG70 HDR/high-refresh display;
- one to five bench PCs;
- at least one KVM-over-IP/OOB endpoint;
- WAN-connected node.

These are validation targets, not hard product minimums.

## Required platform facilities

### Linux

- evdev/libevdev and uinput;
- libei/EIS and XDG RemoteDesktop/InputCapture/Clipboard portals where available;
- PipeWire; JACK compatibility where required;
- KVM/QEMU/libvirt;
- memfd/shared memory, Unix sockets, SCM_RIGHTS;
- DMA-BUF and fences where drivers permit;
- cgroups v2, cpusets, pressure stall information, eBPF/perf interfaces as permitted;
- nftables and an authenticated overlay or direct secure transport;
- optional KVMFR/Looking Glass;
- optional RDMA/UCX/libfabric experiments.

### Windows

- Windows Graphics Capture and/or Desktop Duplication;
- IddCx virtual display driver;
- Core Audio/WASAPI and ASIO bridge strategy;
- Raw Input and SendInput for prototype; Virtual HID Framework driver for higher-fidelity endpoint;
- ETW/performance counters/job objects/CPU sets;
- DXGI/D3D shared-resource primitives where permitted;
- RDP/RAIL/RemoteApp adapter where available;
- service and signed-driver installation lifecycle.

### macOS

- AppKit/SwiftUI shell;
- ScreenCaptureKit where capture is required;
- VideoToolbox and Metal;
- Core Audio;
- CGEvent/Event Taps with Accessibility permissions;
- Network.framework/QUIC-compatible transport as selected;
- ColorSync/EDR/HDR presentation;
- notarized privileged helper only where unavoidable.

## Network

- direct LAN discovery must not be the sole identity mechanism;
- direct paths preferred, relay only when required;
- QUIC datagrams/reliable streams or equivalent multiplexed transport;
- DSCP/traffic-class support where permitted;
- measured bandwidth, RTT, jitter, loss, reordering, and MTU;
- clock synchronization quality descriptor;
- path migration support where transport supports it;
- relay must remain payload-blind under end-to-end encryption.

## Required external integration adapters

- Looking Glass/KVMFR;
- Sunshine/Moonlight-class stream;
- Parsec launch/focus adapter where installed;
- Deskflow-compatible software-KVM bridge during migration;
- RDP/RAIL;
- Xpra;
- Waypipe;
- PipeWire/JACK;
- SMB/NFS/SFTP and object transfer;
- Syncthing-class sync;
- PiKVM/JetKVM/NanoKVM/Comet-class OOB interface;
- ShareCLI process/runtime interface;
- ecosystem event contracts.

## Unsupported assumptions

- uniform coherent memory across unrelated devices;
- GPU peer-to-peer availability merely because two GPUs share a chassis;
- stable HDR behavior without end-to-end platform tests;
- macOS APIs allowing arbitrary background input/capture without permission;
- Wayland compositors exposing identical capture/input semantics;
- consumer GPUs supporting unlimited encoder sessions;
- WAN latency/bandwidth sufficient for fine-grained synchronous execution;
- live migration compatibility between arbitrary CPU/GPU/device topologies.
