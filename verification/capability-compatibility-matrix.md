# Capability Probe Compatibility Matrix

**Spec:** PF-WP-010
**State:** evidence (pre-implementation)
**Date:** 2026-09-01
**Schema:** `capability.schema.json` v1.0.0

This document records the hardware configurations verified to produce
a stable, signed `CapabilityDescriptor`.

## Stability criteria

A descriptor is considered "stable" when:

1. The `topology_hash` is byte-identical across N ≥ 3 probe runs.
2. The `epoch` does not change between runs (no transient fields leaking).
3. The signature is valid against the node's verification key.
4. The schema validates against `capability.schema.json` v1.0.0.

## Verified configurations

| OS | Kernel | CPU | Result | Date |
|:--|:--|:--|:--|:--|
| macOS 14.5 | Darwin 23.5.0 | Apple M1 Pro 16c | TODO | — |
| Ubuntu 24.04 | Linux 6.10.0 | AMD Ryzen 9 7950X | TODO | — |
| Windows 11 | 10.0.22631 | (out of R0 scope) | n/a | — |

## Probe subsystem availability

| Subsystem | Linux | macOS | Windows (R1+) |
|:--|:--:|:--:|:--:|
| CPU/NUMA/cache | ✅ `/proc`, `/sys` | ⚠️ sysctl only (no NUMA) | TODO |
| GPU (NVIDIA) | ✅ `nvidia-smi` | n/a (no NVIDIA on M-series) | TODO |
| GPU (AMD) | ✅ `rocm-smi` | n/a | TODO |
| GPU (Intel) | ✅ `intel_gpu_top` | ✅ Apple integrated | TODO |
| Display (X11) | ✅ `xrandr`, EDID | n/a | n/a |
| Display (Wayland) | ✅ `wlr-randr` | n/a | n/a |
| Display (macOS) | n/a | ✅ `coregraphics` | n/a |
| Audio (PipeWire) | ✅ `pactl`, wpctl | n/a | n/a |
| Audio (CoreAudio) | n/a | ✅ `coreaudio` | n/a |
| Audio (WASAPI) | n/a | n/a | TODO |
| MIDI (PipeWire) | ✅ wpctl | n/a | n/a |
| MIDI (CoreMIDI) | n/a | ✅ `midishell` | n/a |
| Storage (`lsblk`) | ✅ | n/a | TODO |
| Network (`ip -j`) | ✅ | ✅ `ifconfig` | TODO |
| RDMA | ⚠️ `ibstat` (InfiniBand) | n/a | TODO |
| PCIe P2P | ✅ `lspci -tv` | n/a | TODO |

Legend: ✅ supported · ⚠️ partial · n/a not applicable · TODO planned

## Known issues

| Issue | Impact | Workaround | Fixed in |
|:--|:--|:--|:--|
| NUMA not exposed on older kernels (pre-3.8) | `numa_nodes = 0` | Library returns `Error::Unsupported` | n/a |
| Apple Silicon has 1 NUMA domain | Loss of granularity | OK for R0 (no real NUMA) | n/a |
| `nvidia-smi` blocks on broken driver | Probe hangs | Per-call timeout (5s) | n/a |
| PCIe P2P requires `CONFIG_PCI_P2PDMA` | Always false on old kernels | Library reads `/proc/config.gz` | n/a |
| Some monitors have no EDID | `edid_hash = None` | Use `DisplayInfo::width_px/height_px` only | n/a |

## Test plan (R0 exit gate)

1. Run probe 3 times on the reference environment.
2. Verify the three descriptors are byte-identical (modulo `probed_at` and `epoch`).
3. Verify the topology_hash is identical across all 3.
4. Verify the signature on each is valid.
5. Verify the descriptor validates against the JSON schema.

This will be run as part of the R0 acceptance gate per `specs/013-fabric-program-baseline/tasks.md`.
