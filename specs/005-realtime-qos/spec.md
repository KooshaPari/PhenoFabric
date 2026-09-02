# Real-Time QoS, Admission, and Contention Control

## Meta

- **ID:** `005-realtime-qos`
- **Created:** 2026-08-28
- **State:** specified
- **Scope:** RT0-Bulk classes, resource reservations, Ableton/gaming protection, adaptive degradation, thermal/power policy
- **Requirement traces:** `PF-FR-056`, `PF-FR-070`, `PF-FR-071`, `PF-FR-072`, `PF-FR-073`, `PF-FR-074`, `PF-FR-075`
- **Intent traces:** `INT-P001`–`INT-P008`

## Context

Both source and sink hosts may be heavily loaded by compilers, agent tests, games, inference, and media. An architecture benchmarked only on idle machines would fail the actual use case.

## Problem Statement

OS priority alone does not reserve GPU engines, VRAM, memory bandwidth, storage, NIC capacity, encoder sessions, or network buffers. Background work can create latency spikes even when average utilization appears acceptable.

## Goals

- Define service classes and deadline/admission contracts.
- Reserve CPU, GPU, memory, storage, network, and codec capacity.
- Protect local Ableton and game critical paths.
- Relocate, throttle, or pause background work before deadline violation.
- Provide predictable quality degradation instead of stalls.
- Expose pressure, misses, xruns, frame pacing, and admission decisions.

## Non-Goals

- Claim hard RT over uncontrolled WAN.
- Distribute tightly coupled audio/game loops merely because another node is faster.
- Hide overcommit behind best-effort behavior.
- Rely solely on nice level or process priority.

## User and System Outcomes

The feature must improve the packaged experience while preserving the physical and authority boundaries in `README.md`, `DOMAIN_MODEL.md`, and `ecosystem/boundaries.md`. It is accepted only through evidence produced under the mixed-load reference scenarios.

## Functional Requirements

- `PF-FR-056`
- `PF-FR-070`
- `PF-FR-071`
- `PF-FR-072`
- `PF-FR-073`
- `PF-FR-074`
- `PF-FR-075`

## Technical Approach

1. Define RT0–RT4/Bulk/Background contracts.
2. Use platform CPU isolation/affinity and priority facilities.
3. Inventory GPU queues, codec/copy engines, VRAM, and memory pressure.
4. Integrate storage and network pacing.
5. Build admission controller and degradation ladders.
6. Integrate ShareCLI thermal/coalescing/pressure signals.
7. Test under adversarial mixed workloads.

## Data and Control Boundaries

- Control metadata is versioned, authenticated, and auditable.
- High-bandwidth payloads remain in direct/shared/peer data planes where possible.
- Cross-product integrations use stable IDs and events, not shared database tables.
- Real-time threads never perform dynamic policy evaluation.
- Failure must preserve or restore a safe route/state.

## Success Criteria

- Declared Ableton load runs with zero xruns within admitted envelope while agents compile/test.
- Game frame-time p99 remains within declared budget while background jobs relocate/throttle.
- Admission rejects impossible combinations before activation.
- Telemetry attributes deadline pressure to resource/stage.
- Degradation is monotonic, profile-driven, and reversible.

## Required Evidence

- unit and contract tests linked to requirement IDs;
- topology and version manifest;
- p50/p95/p99/worst performance results where applicable;
- concurrent-load scenario;
- fault/rollback result;
- security review for privileged/cross-device boundaries;
- Tracera-compatible evidence references;
- clean setup and teardown instructions.

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Consumer OS cannot provide hard guarantees | High | Define measured envelopes and local RT islands, not absolute marketing claims. |
| GPU interference is opaque | High | Use vendor telemetry plus empirical admission margins. |
| Quality ladder surprises user | High | Expose profile and active degradation state. |
| Over-reservation wastes resources | High | Borrow unused reservation with immediate preemption. |

## Work Packages

| WP ID | Description | Depends On | State |
|---|---|---|---|
| WP-001 | Service-class and deadline schema | — | Planned |
| WP-002 | CPU scheduling/isolation adapters | WP-001 | Planned |
| WP-003 | GPU/codec/VRAM pressure and reservations | WP-001 | Planned |
| WP-004 | Storage/network/memory pressure control | WP-001 | Planned |
| WP-005 | Admission and degradation engine | WP-002, WP-003, WP-004 | Planned |
| WP-006 | Ableton RT island prototype | WP-002, WP-005 | Planned |
| WP-007 | Gaming/frame-time protection prototype | WP-003, WP-005 | Planned |
| WP-008 | Mixed workload/fault validation | WP-006, WP-007 | Planned |

## Traces

- Parent PRD: [`../../PRD.md`](../../PRD.md)
- HLD: [`../../HLD.md`](../../HLD.md)
- LLD: [`../../LLD.md`](../../LLD.md)
- Verification: [`../../verification/requirements-traceability-matrix.md`](../../verification/requirements-traceability-matrix.md)
- Human source: [`../../intent/000-source-prompts.md`](../../intent/000-source-prompts.md)
