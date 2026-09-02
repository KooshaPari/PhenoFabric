# Adaptive Heterogeneous Compute and Data Fabric

## Meta

- **ID:** `003-compute-data-fabric`
- **Created:** 2026-08-28
- **State:** specified
- **Scope:** Resource inventory, task/object model, placement, residency, fusion/fission, mobility, predictive prefetch
- **Requirement traces:** `PF-FR-060`, `PF-FR-061`, `PF-FR-062`, `PF-FR-063`, `PF-FR-064`, `PF-FR-065`, `PF-FR-066`, `PF-FR-067`, `PF-FR-068`, `PF-FR-069`
- **Intent traces:** `INT-P001`–`INT-P008`

## Context

The user intends all connected instances to act as one distributed compute plane while preserving the possibility of atomic routing. The runtime must exploit all available hardware without pretending data movement and synchronization are free.

## Problem Statement

Traditional cluster schedulers place jobs; local OS schedulers place threads; accelerator runtimes place kernels; remote desktop fixes presentation. None jointly optimizes execution, data, foreground deadlines, and visible surfaces across same-host, LAN, and WAN boundaries.

## Goals

- Inventory CPU/GPU/NPU/FPGA, memory, storage, NIC, PCIe, NUMA, thermal, and power topology.
- Represent tasks/execution regions and versioned objects with residency.
- Choose placement by total completion cost and protected-workload impact.
- Support atomic interposition where available and adaptive region fusion/fission.
- Prefetch, replicate, spill, restore, and cancel based on predicted use.
- Support surface projection, semantic handoff, checkpoint, VM migration, and rematerialization as distinct mobility levels.

## Non-Goals

- Present remote memory/VRAM as uniform local memory.
- Promise transparent arbitrary cross-OS process migration.
- Route each syscall remotely by default.
- Replace application-specific distributed algorithms.
- Make the coordinator carry object payloads.

## User and System Outcomes

The feature must improve the packaged experience while preserving the physical and authority boundaries in `README.md`, `DOMAIN_MODEL.md`, and `ecosystem/boundaries.md`. It is accepted only through evidence produced under the mixed-load reference scenarios.

## Functional Requirements

- `PF-FR-060`
- `PF-FR-061`
- `PF-FR-062`
- `PF-FR-063`
- `PF-FR-064`
- `PF-FR-065`
- `PF-FR-066`
- `PF-FR-067`
- `PF-FR-068`
- `PF-FR-069`

## Technical Approach

1. Build topology/resource descriptor and pressure telemetry.
2. Create immutable object references, residency catalog, local shared-memory store, and spill tiers.
3. Expose explicit TaskSpec/RegionSpec API before transparent interposition.
4. Integrate process/task workloads through ShareCLI/NVMS/thegent adapters.
5. Implement candidate pruning and cost-based placement with explainability.
6. Add fusion/fission learning from actual cost.
7. Prototype FUSE/syscall/eBPF/runtime interposition only after explicit-task baselines.
8. Add checkpoint/rematerialization and selected VM migration adapters.

## Data and Control Boundaries

- Control metadata is versioned, authenticated, and auditable.
- High-bandwidth payloads remain in direct/shared/peer data planes where possible.
- Cross-product integrations use stable IDs and events, not shared database tables.
- Real-time threads never perform dynamic policy evaluation.
- Failure must preserve or restore a safe route/state.

## Success Criteria

- Rust builds, agent tests, inference, and media helpers are placed across available nodes with measured improvement.
- Data transfer cost can make a slower local node correctly win.
- Repeated fine-grained decisions fuse and reduce overhead below threshold.
- Foreground game/audio routes improve because competing work is moved away.
- Every placement explains inputs, alternatives, and uncertainty.
- Node loss does not corrupt authoritative objects.

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
| Transparent interposition overhead erases benefit | High | Explicit regions first; measure; fuse aggressively; retain as selective capability. |
| Object plane duplicates filesystems/databases | High | Limit to runtime objects and bridge conventional storage. |
| Prediction errors cause thrash | High | Hysteresis, minimum residency, cooldown, confidence margin. |
| Cross-node mutable state is unsound | High | Single authority or explicit consistency; immutable replication preferred. |
| Hardware capability claims are wrong | High | Probe and benchmark topology instead of trusting names. |

## Work Packages

| WP ID | Description | Depends On | State |
|---|---|---|---|
| WP-001 | Resource/topology descriptor and probes | — | Planned |
| WP-002 | Object identity, residency, local shared store | WP-001 | Planned |
| WP-003 | TaskSpec and execution-region API | WP-001 | Planned |
| WP-004 | Baseline cost model and scheduler | WP-002, WP-003 | Planned |
| WP-005 | ShareCLI/NVMS/thegent execution adapters | WP-003 | Planned |
| WP-006 | Prefetch, replication, spill, and result routing | WP-002, WP-004 | Planned |
| WP-007 | Fusion/fission and predictive placement | WP-004, WP-006 | Planned |
| WP-008 | Selective interposition and mobility experiments | WP-005, WP-007 | Planned |
| WP-009 | Adversarial mixed-workload evaluation | WP-008 | Planned |

## Traces

- Parent PRD: [`../../PRD.md`](../../PRD.md)
- HLD: [`../../HLD.md`](../../HLD.md)
- LLD: [`../../LLD.md`](../../LLD.md)
- Verification: [`../../verification/requirements-traceability-matrix.md`](../../verification/requirements-traceability-matrix.md)
- Human source: [`../../intent/000-source-prompts.md`](../../intent/000-source-prompts.md)
