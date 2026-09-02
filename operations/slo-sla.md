# Service-Level Objectives and Product Profiles

SLOs attach to a complete fingerprint and declared profile. A generic “low latency” claim is prohibited.

| Class | SLI | Initial objective | Scope |
|---|---|---|---|
| Control | Prepared local focus switch latency | p95 ≤50 ms; zero stuck/lost ownership errors | Prepared endpoints; exact hardware profile |
| Control | Route transaction correctness | 100% old-route preservation on failed prepare; stale token acceptance = 0 | Fault/partition corpus |
| Same-host video | Avoidable stage count | 0 codec/network stages when a valid SHM/DMA path exists | Supported same-host route matrix |
| Interactive video | Frame age / pacing | Per profile; no uncontrolled queue growth; p99 and burst limits published | Resolution/refresh/HDR and load profile |
| Input | Input-to-photon | Profile-specific; measured hardware loop, not software-only claim | same-host/LAN/WAN reported separately |
| Audio RT | Xruns | 0 within admitted reference soak and fault envelope | Declared rate/buffer/project/hardware/load |
| Audio network | Clock/buffer stability | No discontinuity; bounded occupancy and ASRC ratio | Declared network impairment envelope |
| Compute | Placement regret | Selected completion cost within calibrated tolerance of measured best valid candidate | Benchmark corpus; uncertainty reported |
| Object | Authority/version correctness | 0 split-authority or stale-version delivery | Crash/corruption/partition suite |
| Agent | Unauthorized focus acquisitions | 0 | All realm/surface workflows |
| Operations | OOB recovery coverage | Every important physical host has a tested path for listed failure classes | Inventory policy |
| Upgrade | Failed upgrade recovery | 100% rollback to operable compatible version in test matrix | Supported version transitions |

## Profiles

- `creator-rt`: audio deadline dominates; reject routes rather than overcommit.
- `gaming`: input/frame pacing dominates; color/HDR preserved within declared ladder.
- `desktop-text`: 4:4:4 and scale/text clarity dominate.
- `color-critical`: controlled transforms and bit depth dominate.
- `agent-throughput`: completion/throughput dominates within foreground reservations.
- `bulk`: consumes residual capacity and is the first to throttle.
- `recovery`: reachability and correctness dominate quality.

The initial values are design objectives. Achieved numbers are published only from versioned evidence.
