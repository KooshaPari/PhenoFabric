# Capability Probe Benchmark

**Spec:** PF-WP-010
**State:** evidence (pre-implementation)
**Date:** 2026-09-01
**Environment:** `phenotype-fabric` R0 reference environment

## Budget

| Probe scope | Budget | Rationale |
|:--|:--|:--|
| Cold (first call, process start) | < 100 ms | Must not block a shell invocation |
| Warm (subsequent calls, same process) | < 1 ms | Path planning loop needs fast refresh |
| Full (all subsystems) | < 1 s | Daemon can schedule during idle windows |

## Methodology

### Measurement setup

The benchmark runner (`cargo bench -p fabric-capability`) measures wall-clock time
using `std::time::Instant`. Each probe is run 3 times and the median is reported.
The cold measurement is performed on a freshly spawned process.

Reference environment:
- AMD Ryzen 9 7950X (16c/32t, 2 × CCD, 1 NUMA node)
- 64 GB DDR5-5600
- Samsung 980 Pro 2 TB NVMe
- Linux 6.10.0 (kernel)
- Apple M1 Pro MacBook (16 GB, macOS 14) as secondary target

### Metrics collected

- `cold_cpu_probe_us`: wall-clock µs for the CPU/NUMA/cache probe (sub-100ms budget)
- `warm_cpu_probe_us`: wall-clock µs for a repeated probe in the same process
- `full_probe_us`: wall-clock µs for all subsystems (sub-1s budget)
- `memory_bytes`: bytes allocated during the probe (target: < 1 MB)

## Planned benchmarks

| Benchmark | Target | Status |
|:--|:--|:--|
| `bench_cpu_probe` | < 100 ms cold, < 1 ms warm | TODO |
| `bench_full_probe` | < 1 s | TODO |
| `bench_topology_hash` | < 10 ms | TODO |
| `bench_sign_descriptor` | < 5 ms | TODO |
| `bench_verify_descriptor` | < 2 ms | TODO |

## Expected results (pre-implementation)

Based on similar tools (lstopo, nvidia-smi):

| Subsystem | Expected cold | Expected warm | Risk |
|:--|:--|:--|:--|
| CPU/NUMA/cache (`/proc`) | 5–20 ms | < 0.5 ms | Low |
| GPU (PCIe + nvidia-smi) | 50–200 ms | < 5 ms | Medium (nvidia-smi spawn) |
| Display (xrandr/wlr-randr) | 20–100 ms | < 2 ms | Medium (XR/Wayland) |
| Audio (PipeWire) | 10–50 ms | < 1 ms | Low |
| Storage (`lsblk`) | 5–20 ms | < 0.5 ms | Low |
| Network (`ip -j`) | 2–10 ms | < 0.5 ms | Low |
| **Full probe** | **100–400 ms** | **< 5 ms** | **Medium** |

The full probe is expected to complete well within the 1s budget on the reference
environment. The main risk is `nvidia-smi` (which spawns a subprocess on each call).

## Probes that exceed budget

If a probe exceeds its budget, the library returns `Error::BudgetExceeded` and
the descriptor includes a `confidence` field reduced to reflect partial data.

The daemon (PF-WP-080) handles this by scheduling the probe during idle windows
(when `w` < 0.3, measured via `/proc/loadavg`).

## Load sensitivity

On a heavily-loaded system (all cores pinned by agent builds + Ableton + gaming):

| Scenario | Expected impact |
|:--|:--|
| Idle | Baseline |
| 1 core pinned | +5–15 ms (negligible) |
| All cores pinned | +50–200 ms (may breach budget) |
| NVMe under heavy write | +10–30 ms |

## TODO

- [ ] Implement `bench_cpu_probe` in `crates/fabric-capability/benches/`
- [ ] Run on reference environment and record actual numbers
- [ ] Update this file with real benchmark results
- [ ] Add `bench_sign_descriptor` and `bench_verify_descriptor`
- [ ] Document nvidia-smi subprocess overhead
