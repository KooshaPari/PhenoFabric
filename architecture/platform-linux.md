# Linux Platform Design

## Roles

Linux is the first deep-integration host because it provides the strongest combination of KVM/QEMU, evdev/uinput, PipeWire, libei portals, cgroups, topology visibility, shared memory, DMA-BUF, and open adapter integration.

## Components

### Unprivileged endpoint

- registry/probe client;
- PipeWire graph adapter;
- Wayland/X11 session adapter;
- process/resource telemetry;
- object/cache daemon;
- local transport;
- shell.

### Privileged helpers

- input grab/uinput;
- selected topology/device probes;
- KVMFR/device permissions;
- CPU/cgroup/RT scheduling operations;
- optional eBPF/interposition;
- OOB adapter credentials.

Helpers use Unix sockets, peer credentials, SCM_RIGHTS, versioned messages, and allowlists.

## Input

- libevdev for event normalization;
- EVIOCGRAB for owned devices;
- uinput for persistent virtual endpoints;
- libei/EIS and portals for compositor-mediated capture/injection;
- X11 fallback isolated from Wayland assumptions.

## Video

- Looking Glass/KVMFR for passed-through Windows VM;
- DMA-BUF import/export where supported;
- PipeWire screen/window capture for compositor-mediated paths;
- Sunshine/Moonlight-class adapter for network stream;
- virtual display/compositor strategy for headless surfaces.

## Audio

PipeWire is the default graph. JACK compatibility is used for professional applications. The RT engine follows PipeWire/JACK principles but does not require replacing them.

## Compute/resource

- `/sys`, hwloc-like topology, PCI tree, IOMMU groups;
- cgroups v2/cpusets/PSI;
- perf/eBPF where permitted;
- GPU vendor APIs and DRM telemetry;
- io_uring/local async I/O;
- KVM/QEMU/libvirt realm lifecycle.

## VM fast paths

- IVSHMEM/KVMFR;
- virtio-input/network/block/balloon;
- virtiofs;
- vhost-user;
- QEMU migration/checkpoint adapters;
- complete IOMMU-group passthrough where required.

## Wayland truth

Always-on capture/input may require explicit portal grants and compositor support. The product must surface missing/expired permission rather than silently falling back to unsafe hacks.
