# Benchmark Plan

These are initial gates, not historical results. Gates are tightened after the first calibrated baseline rather than weakened after a failure.

| ID | Domain | Scenario | Metrics | Load matrix | Initial gate |
|---|---|---|---|---|---|
| PF-BM-001 | Input | Local focus transfer among host and 1–8 VMs | switch p50/p95/p99/max; stuck/lost/duplicate events; broker CPU | idle, cargo build, game, hotplug, target crash | p95 ≤50 ms after prepared; zero correctness failures in 1M randomized transitions |
| PF-BM-002 | Same-host video | VFIO VM to Linux display via LG/KVMFR and alternatives | input→photon; capture/compositor stage; frame age; CPU/GPU/copies | 1440p60/120/144 SDR/HDR; idle and mixed | LG selected when valid; no unexplained codec/copy; p99 frame age bounded to declared profile |
| PF-BM-003 | LAN video | Main/VM to M1 Pro over wired LAN | stage latency, dropped/late frames, bitrate, encode/decode utilization, thermals | desktop 4:4:4, game, HDR video, Mac local load | 1440p120 preferred profile meets no-stall/frame-age target or compiler selects explicit lower profile |
| PF-BM-004 | Color/HDR | Source intent to C27HG70 and M1 Pro | metadata, bit depth, transfer/gamut mapping, clipping, banding, color error where instrumented | SDR, PQ, desktop text, game, video | Transform is explicit and no worse than native/reference path within declared tolerance |
| PF-BM-005 | Audio local | Ableton/ASIO protected island | xruns, callback time/deadline, round trip, scheduler/DPC spikes | 64/128/256 samples; compile, inference, storage, game E2E | zero xruns in 4 h within admitted reference envelope |
| PF-BM-006 | Audio LAN | PC↔Mac/bench clocked audio/MIDI | one/two-way latency, drift, ASRC ratio, jitter, glitches, MIDI ordering | 1/8/24 h; induced jitter/skew | stable buffer and zero discontinuities within admitted profile |
| PF-BM-007 | Queue isolation | Bulk files/clipboard during input/audio/video | RT p99/worst and bulk throughput | 1/2.5/10GbE; loss/jitter; SSD saturation | bulk cannot violate admitted RT routes |
| PF-BM-008 | Compute placement | Build/test/inference/encode/compress across nodes | completion, transfer, queue, energy, cache, foreground externality | cold/warm data; CPU/GPU pressure; LAN/WAN | selected plan beats local/default by policy margin or explains no move |
| PF-BM-009 | Object plane | SHM/cache/replica/spill/restore | copy count, lookup, throughput, recovery, authority correctness | memory pressure, node crash, corrupt replica | no authority/version error and positive cache value |
| PF-BM-010 | Adaptive regions | atomic vs static vs fusion/fission | decision cost, completion, migrations, oscillation, wasted work | local/VM/LAN/WAN RTT and changing load | adaptive beats both baselines on target corpus without SLO regression |
| PF-BM-011 | Seamless surfaces | 1–32 Windows/Linux app windows | interaction latency, window correctness, encoder sessions, bandwidth, IME/DPI errors | static/motion/video/popups/protected surfaces | route-specific compatibility recorded; deterministic fallback |
| PF-BM-012 | Agent realms | N ephemeral VM/native realms | time-to-ready, resource overhead, cleanup, focus theft, evidence completeness | 1/8/32 realms and provider failures | zero unauthorized focus; bounded resource leakage |
| PF-BM-013 | WAN | Roaming and impaired media/control | resume, quality adaptation, latency distribution, route safety | 20–150ms RTT, loss, jitter, bandwidth steps | no stale focus; graceful explicit degradation and recovery |
| PF-BM-014 | OOB | BIOS/no-boot/hang/driver failure | time-to-control, success, action count | each important host and failure type | documented successful recovery path |

## Reference topology

- Main PC: Linux host target with Ryzen 7 5800X, RTX 3090 Ti, optional GTX 1080 Ti, 64 GB DDR4 and QEMU/VFIO Windows guest.
- Displays: Samsung C27HG70 2560×1440 high-refresh HDR display and 2021 M1 Pro MacBook Pro panel.
- Bench: 1–5 PCs with software path and at least one OOB path.
- Remote: at least one WAN node with controllable impairment.
- Foreground: representative game and Ableton Live 12 Suite project.
- Background: cargo builds, game E2E, inference, storage transfer and agent-created realms.

Hardware and driver versions are captured per run; the plan does not assume the current topology is the final supported matrix.
