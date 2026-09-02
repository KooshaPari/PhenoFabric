# Locality-Aware Route Compiler and Transport Plane

## Meta

- **ID:** `004-locality-route-compiler`
- **Created:** 2026-08-28
- **State:** specified
- **Scope:** L0-L8 locality tiers, stage elimination, shared-memory/PCIe/LAN/WAN/OOB paths, dynamic replanning
- **Requirement traces:** `PF-FR-012`, `PF-FR-013`, `PF-FR-014`, `PF-FR-015`, `PF-FR-017`, `PF-FR-041`, `PF-FR-044`, `PF-FR-045`
- **Intent traces:** `INT-P001`–`INT-P008`

## Context

The product must make the same request work everywhere while using radically different implementations beneath it. “Remote” must not automatically mean encoded network streaming.

## Problem Statement

General remote-desktop stacks perform capture, conversion, encode, network, decode, upload, and composition even when source and sink share a host. Static backend choices ignore topology, load, quality, and fallback.

## Goals

- Define locality tiers and transport-stage contracts.
- Select the least-expensive valid route for each link.
- Eliminate unnecessary copies and transforms.
- Integrate KVMFR/IVSHMEM, DMA-BUF, virtio/vhost, direct LAN, WAN, and OOB paths.
- Account for PCIe/root-complex, encoder/copy engine, memory, and NIC pressure.
- Replan safely as topology or load changes.

## Non-Goals

- Invent a single universal transport.
- Assume zero-copy across incompatible memory/security domains.
- Use WAN relays for reachable LAN peers.
- Switch live routes without preparation and rollback.

## User and System Outcomes

The feature must improve the packaged experience while preserving the physical and authority boundaries in `README.md`, `DOMAIN_MODEL.md`, and `ecosystem/boundaries.md`. It is accepted only through evidence produced under the mixed-load reference scenarios.

## Functional Requirements

- `PF-FR-012`
- `PF-FR-013`
- `PF-FR-014`
- `PF-FR-015`
- `PF-FR-017`
- `PF-FR-041`
- `PF-FR-044`
- `PF-FR-045`

## Technical Approach

1. Model each route as composable stages with fixed/per-byte/queue/resource costs.
2. Probe memory domains, DMA/P2P, codecs, direct networking, and permissions.
3. Implement same-host video/input/audio/object reference routes first.
4. Add LAN QUIC and optional RDMA/UCX/NIXL experiments.
5. Add WAN congestion control, NAT traversal, blind relay, and FEC policy.
6. Use prepare/commit/abort and activation timestamps.
7. Persist route explanations and benchmark evidence.

## Data and Control Boundaries

- Control metadata is versioned, authenticated, and auditable.
- High-bandwidth payloads remain in direct/shared/peer data planes where possible.
- Cross-product integrations use stable IDs and events, not shared database tables.
- Real-time threads never perform dynamic policy evaluation.
- Failure must preserve or restore a safe route/state.

## Success Criteria

- Same-host eligible routes contain no codec or network stage.
- LAN/WAN routes adapt without unbounded queues.
- Failure during preparation preserves the existing route.
- Compiler decisions outperform static defaults on benchmark matrix.
- Route explanation identifies all copies, conversions, buffers, and relays.

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
| Cost model is too complex or stale | High | Start empirical, cache by topology/load class, expose manual pinning. |
| P2P/DMA unavailable due IOMMU/driver/topology | High | Probe; fall back without claim inflation. |
| Dynamic route change causes visible glitch | High | Warm prepare and cycle/frame-boundary commit. |
| Vendor codec session limits appear under load | High | Inventory sessions and reserve capacity. |

## Work Packages

| WP ID | Description | Depends On | State |
|---|---|---|---|
| WP-001 | Locality and stage schema | — | Planned |
| WP-002 | Topology/capability probes | WP-001 | Planned |
| WP-003 | Same-OS/same-host route implementations | WP-002 | Planned |
| WP-004 | KVMFR/IVSHMEM/virtio adapters | WP-003 | Planned |
| WP-005 | LAN direct transport and quality negotiation | WP-001, WP-002 | Planned |
| WP-006 | WAN/NAT/relay transport | WP-005 | Planned |
| WP-007 | Compiler cost model and explanations | WP-003, WP-005 | Planned |
| WP-008 | Transactional replan and fallback | WP-004, WP-006, WP-007 | Planned |
| WP-009 | Topology and contention benchmark matrix | WP-008 | Planned |

## Traces

- Parent PRD: [`../../PRD.md`](../../PRD.md)
- HLD: [`../../HLD.md`](../../HLD.md)
- LLD: [`../../LLD.md`](../../LLD.md)
- Verification: [`../../verification/requirements-traceability-matrix.md`](../../verification/requirements-traceability-matrix.md)
- Human source: [`../../intent/000-source-prompts.md`](../../intent/000-source-prompts.md)
