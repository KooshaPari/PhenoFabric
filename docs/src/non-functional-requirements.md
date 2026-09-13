# Non-Functional Requirements

These are target contracts. A target is not a claim until the associated benchmark evidence exists.

## Latency and real-time behavior

| ID | Requirement / target |
|---|---|
| PF-NFR-001 | Same-host input routing adds no more than 0.25 ms p95 processing latency in the broker under supported load. |
| PF-NFR-002 | Prepared input-focus switch completes in under 50 ms p95 and never leaves a stuck key/button state. |
| PF-NFR-003 | Prepared output takeover reaches first valid presented frame in under 250 ms p95 on LAN/same-host paths. |
| PF-NFR-004 | Same-host VFIO video uses a non-codec path when KVMFR/shared-memory capability is healthy. |
| PF-NFR-005 | Wired-LAN optimized media targets under 10 ms median capture-to-client-enqueue and under 20 ms p95, separately from glass-to-glass measurement. |
| PF-NFR-006 | RT0 audio routes meet the configured callback deadline with zero xruns during the declared admission envelope. |
| PF-NFR-007 | RT1 input/MIDI packets are scheduled ahead of video and bulk traffic. |
| PF-NFR-008 | Scheduler/route decisions on an admitted hot path shall consume less than 1% of the corresponding task or frame budget, or be cached/fused. |
| PF-NFR-009 | No route change may create an unbounded buffer or queue. |
| PF-NFR-010 | WAN behavior is adaptive and measured; no hard real-time guarantee is made over uncontrolled links. |

## Throughput and scale

| ID | Requirement / target |
|---|---|
| PF-NFR-020 | A coordinator supports at least 512 concurrently registered realms and 10,000 graph objects in the initial scale target. |
| PF-NFR-021 | Graph updates not touching a real-time route shall not pause its processing thread. |
| PF-NFR-022 | The system supports at least 120 concurrently active agent/workload placements in the reference environment while protecting foreground reservations. |
| PF-NFR-023 | Object metadata operations are horizontally partitionable; object payloads never transit the coordinator by default. |
| PF-NFR-024 | Telemetry collection is bounded and shed before it interferes with RT workloads. |
| PF-NFR-025 | Route planning complexity is constrained by candidate pruning, locality ordering, and cached capability plans. |

## Video and color quality

| ID | Requirement / target |
|---|---|
| PF-NFR-030 | Desktop-text profiles require 4:4:4 or an explicit documented exception. |
| PF-NFR-031 | HDR routes preserve at least 10-bit transport where source and sink require it. |
| PF-NFR-032 | Source/sink color transforms are deterministic, profile-aware, and testable with reference patterns. |
| PF-NFR-033 | Frame pacing and p99 latency matter more than maximum average FPS. |
| PF-NFR-034 | The system reports whether HDR is preserved, tone-mapped, clipped, or converted to SDR. |
| PF-NFR-035 | Same-host raw/shared-memory video does not perform hidden chroma subsampling or color-space conversion. |

## Audio quality

| ID | Requirement / target |
|---|---|
| PF-NFR-040 | Local professional-audio routes support float32 and common 44.1/48/88.2/96/192 kHz modes subject to endpoint support. |
| PF-NFR-041 | Resampling is avoided within a shared clock domain and explicit elsewhere. |
| PF-NFR-042 | Clock drift, elastic-buffer fill, ASRC ratio, jitter, and xrun counters are observable. |
| PF-NFR-043 | Audio switching uses bounded fade/crossfade to avoid pops and duplicate playback. |
| PF-NFR-044 | Audio packet loss recovery does not add video-derived buffering automatically. |

## Reliability and consistency

| ID | Requirement / target |
|---|---|
| PF-NFR-050 | Exclusive input/output routes use leases and fencing tokens; stale holders are rejected. |
| PF-NFR-051 | Coordinator failover preserves committed state or clearly returns to local safe mode. |
| PF-NFR-052 | Failed route preparation leaves the current route intact. |
| PF-NFR-053 | Workspace activation is idempotent. |
| PF-NFR-054 | Agent realm cleanup is eventual but access revocation is immediate. |
| PF-NFR-055 | Recovery paths are tested with guest crash, host crash, network partition, and coordinator loss. |

## Security and privacy

| ID | Requirement / target |
|---|---|
| PF-NFR-060 | Cross-device traffic is mutually authenticated and encrypted; first pairing never returns a reusable shared secret over cleartext. |
| PF-NFR-061 | No universal credential grants access to all nodes or capabilities. |
| PF-NFR-062 | Platform secret stores protect long-lived device identity material. |
| PF-NFR-063 | Sensitive clipboard/mic/camera/file routes default to explicit policy and visible indication. |
| PF-NFR-064 | OOB devices reside on restricted management networks and are never directly internet-exposed. |
| PF-NFR-065 | Privileged helpers have minimal syscall/device access and an independently reviewable protocol. |
| PF-NFR-066 | Audit records are tamper-evident and reference evidence artifacts without exposing secrets. |

## Portability, operability, and maintainability

| ID | Requirement / target |
|---|---|
| PF-NFR-070 | Linux, Windows, and macOS expose equivalent product semantics even when implementation mechanisms differ. |
| PF-NFR-071 | An unavailable optional adapter reduces capability rather than preventing core startup. |
| PF-NFR-072 | Stable schemas are versioned and backwards-compatible within a major release. |
| PF-NFR-073 | Every platform adapter has a loopback simulator for CI. |
| PF-NFR-074 | The installer can repair, roll back, or remove privileged components. |
| PF-NFR-075 | A full diagnostic bundle is exportable without private payload data by default. |
| PF-NFR-076 | Source and generated claims are traceable to exact versions, tests, and topologies. |

## Resource efficiency

| ID | Requirement / target |
|---|---|
| PF-NFR-080 | Idle endpoint agents remain below 1% average CPU and use no active encode/decode session. |
| PF-NFR-081 | Data copies and memory-domain transitions are counted and exposed per route. |
| PF-NFR-082 | The compiler prefers existing hardware codec/copy engines only when their contention cost is lower than alternatives. |
| PF-NFR-083 | Background bulk transfers are paced to residual capacity. |
| PF-NFR-084 | Fine-grained distribution is automatically disabled/fused when observed overhead exceeds its benefit. |
