# Experiment Catalog

| ID | Experiment | Compared systems / variables | Primary metrics | Matrix | Promotion gate |
|---|---|---|---|---|---|
| RE-001 | Same-host surface route shootout | LG/KVMFR vs Parsec vs Sunshine/Moonlight vs RDP/SPICE on the same VM | capture→enqueue, input→photon, CPU/GPU/copy engines, frame pacing, quality | Idle and mixed game/compiler load; 60/120/144 Hz; SDR/HDR | Select minimum valid path; document stage count. |
| RE-002 | Input switch torture | Central broker vs QEMU grabs; randomized held keys/buttons/raw mouse | stuck state, switch p50/p95/p99/max, lost/duplicated events | 1M transitions, target crash, coordinator crash, hotplug | Zero stuck/lost ownership failures. |
| RE-003 | MacBook sink matrix | HEVC/H.264/available codecs, 8/10-bit, 60/120 Hz, scale modes | decode time, compositor delay, thermals, dropped frames, power | M1 Pro local workload plus remote desktop/game | Choose supported profiles; no assumed AV1 hardware. |
| RE-004 | HDR/color chain | Windows/Linux source to C27HG70 and M1 Pro | DeltaE/gamut coverage where instrumented, clipping, metadata, banding | SDR, PQ HDR, desktop text, game, video | Controlled policy no worse than native baseline within tolerance. |
| RE-005 | Ableton isolation | Ableton project at several buffer sizes with cargo build, game E2E, inference and disk I/O | xruns, callback deadline, round-trip latency, DPC/scheduler spikes | Local and routed audio; CPU/GPU/network saturation | No xruns within admitted envelope; honest rejection otherwise. |
| RE-006 | Network audio clock soak | Independent PC/Mac clocks over wired LAN | drift estimate error, buffer occupancy, ASRC ratio, glitch count | 1h/8h/24h, induced jitter and clock skew | Stable occupancy and zero audible discontinuity. |
| RE-007 | Independent queue isolation | Bulk clipboard/file transfer during input/audio/video | input p99, audio xrun, video frame age, transfer throughput | Shared vs isolated queues; congestion and packet loss | RT classes unaffected within gate. |
| RE-008 | Explicit placement corpus | Rust builds, tests, game E2E, encoding, inference, compression | completion, queue, transfer, energy, foreground externality | Main PC/Mac/bench/remote nodes with cold/warm data | Cost model picks measured winner above threshold. |
| RE-009 | Object residency | Immutable object cache, spill and replicas | copy count, lookup, transfer, corruption recovery, space | SHM, SSD, LAN, WAN, node loss | Correct authority/version and positive cache benefit. |
| RE-010 | Fusion/fission | Synthetic micro-ops and real build/inference traces | decision overhead, completion, migrations, oscillation | RTT/locality/load sweeps | Adaptive policy beats atomic-every-time and static-region baselines. |
| RE-011 | Prefetch prediction | Known workflow transitions and adversarial changes | cold-start p95, hit rate, bytes wasted, eviction externality | Build, agent tool, game E2E, Ableton project | Net p95 gain inside waste budget. |
| RE-012 | Seamless app compatibility | Representative Windows/Linux apps including browser, IDE, video, Blender, dialogs | window graph correctness, IME, DPI, cursor, latency, encoder sessions | RAIL/Xpra/Waypipe/pixel proxy/full desktop | Per-app route and fallback recorded. |
| RE-013 | Surface atlas | 1–32 proxy windows with static/motion mixes | encoder sessions, bandwidth, interaction latency, artifacts | Dedicated vs atlas, damage rates | Atlas beneficial for static mix; dedicated for high-motion as selected. |
| RE-014 | WAN impairment | Desktop/app/audio/input over latency/loss/jitter/bandwidth transitions | p50/p95/p99, recovery, quality changes, disconnect time | Network emulator and real remote node | No unsafe focus; stable adaptation and resumability. |
| RE-015 | Partition/fencing | Coordinator and link partitions during route/focus transaction | dual ownership, stale acceptance, rollback time | Prepare/commit boundaries and clock skew | No stale-token action accepted. |
| RE-016 | Agent realm lifecycle | Provision N VMs/sandboxes, publish surfaces, expire/restore | time-to-ready, cleanup leaks, focus theft, evidence completeness | 1/8/32 realms and provider failures | No focus theft; bounded cleanup and fallback. |
| RE-017 | OOB recovery | OS hang, driver crash, no boot, wrong display, network path failure | time to control/recover, action count | PiKVM/JetKVM/software fallbacks | Defined recovery coverage for each important host. |
| RE-018 | Security adversarial suite | Pairing MITM, stale tokens, malicious adapter, clipboard exfiltration, input logger, OOB exposure | exploit success, detection, containment, audit | Local/LAN/WAN and privilege boundaries | No critical open finding at promotion. |
| RE-019 | FUSE coalescing/offload | ShareCLI-like repeated build/read patterns | overhead, duplicate bytes/work, completion, correctness | FUSE on/off, cache cold/warm, same-host/LAN | Graduate only positive bounded workload. |
| RE-020 | Atomic mediation envelope | Function/syscall/kernel candidates across locality tiers | break-even granularity, marshal/sync cost, failures | same process/OS/host/RDMA/LAN/WAN | Publish empirical granularity map, not universal claim. |
| RE-021 | User workflow comparison | Separate apps vs unified shell for desk/couch/bench/agent tasks | actions, errors, completion, cognitive load, recovery | Expert and ordinary modes | Unified shell materially improves workflow without hiding failures. |

## Reproduction contract

Each run records commit/version, OS/kernel/driver/firmware, topology, power/thermal state, display/audio modes, network impairment, background load, raw samples, clocks, trace IDs and failed/aborted runs. Graphs without raw evidence are illustrative only.
