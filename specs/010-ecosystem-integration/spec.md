# Phenotype Ecosystem Integration and Product Boundaries

## Meta

- **ID:** `010-ecosystem-integration`
- **Created:** 2026-08-28
- **State:** specified
- **Scope:** AgilePlus, thegent, AGSLAG, Tracera, SessionLedger, ShareCLI, NVMS, labs-compute, event contracts
- **Requirement traces:** `PF-FR-086`, `PF-FR-087`
- **Intent traces:** `INT-P001`–`INT-P008`

## Context

The fabric is not an unrelated project. It fills the physical execution/data/I/O substrate beneath an existing synthetic-enterprise architecture, but consolidation must not erase independent authorities.

## Problem Statement

Overlapping repos can create duplicate schedulers, inventories, graphs, telemetry, and public claims. Putting everything into ShareCLI or one universal database would reduce clarity and external usefulness.

## Goals

- Define authority and repository boundaries.
- Create stable shared IDs and event envelopes.
- Integrate ShareCLI without absorbing its standalone supervisor product.
- Expose runtime capabilities to thegent/NVMS.
- Receive authorized constraints from AgilePlus/AGSLAG.
- Export evidence to Tracera/SessionLedger.
- Keep research in labs-compute until graduation.

## Non-Goals

- Create a universal database/ontology.
- Move project planning into the runtime.
- Move labor dispatch into the fabric.
- Move economic allocation into the scheduler.
- Duplicate raw operational history or traceability stores.

## User and System Outcomes

The feature must improve the packaged experience while preserving the physical and authority boundaries in `README.md`, `DOMAIN_MODEL.md`, and `ecosystem/boundaries.md`. It is accepted only through evidence produced under the mixed-load reference scenarios.

## Functional Requirements

- `PF-FR-086`
- `PF-FR-087`

## Technical Approach

1. Publish authority matrix and event contracts.
2. Define TaskSpec/PlacementRequest/RouteDecision/RunEvidence envelopes.
3. Build ShareCLI adapter for process discovery, coalescing, thermal, FUSE, and supervision.
4. Build NVMS inventory and isolation boundary.
5. Link AgilePlus FR/WP IDs and thegent run IDs.
6. Export summaries/evidence references to Tracera/SessionLedger.
7. Define labs-compute graduation criteria.

## Data and Control Boundaries

- Control metadata is versioned, authenticated, and auditable.
- High-bandwidth payloads remain in direct/shared/peer data planes where possible.
- Cross-product integrations use stable IDs and events, not shared database tables.
- Real-time threads never perform dynamic policy evaluation.
- Failure must preserve or restore a safe route/state.

## Success Criteria

- Each canonical entity has one owning product.
- Integrations work through versioned contracts without shared database access.
- ShareCLI remains installable/useful alone.
- Runtime decisions can be traced from human intent to work to run to evidence.
- Experimental fine-grained mechanisms do not become product claims before proof.

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
| Boundary debates block implementation | High | Start with contracts and adapters; defer repo merge decisions. |
| Duplicate telemetry/evidence | High | Raw local metrics, summarized events, canonical trace ownership. |
| ShareCLI and fabric schedulers conflict | High | Explicit delegation and priority hierarchy. |
| AGSLAG economic objective leaks into RT hot path | High | Translate budgets/policies ahead of time; no economic reasoning on RT thread. |

## Work Packages

| WP ID | Description | Depends On | State |
|---|---|---|---|
| WP-001 | Authority and entity ownership matrix | — | Planned |
| WP-002 | Shared ID/event envelope schema | WP-001 | Planned |
| WP-003 | ShareCLI adapter | WP-002 | Planned |
| WP-004 | NVMS/thegent runtime contracts | WP-002 | Planned |
| WP-005 | AgilePlus/AGSLAG policy inputs | WP-002 | Planned |
| WP-006 | Tracera/SessionLedger evidence outputs | WP-002 | Planned |
| WP-007 | labs-compute graduation process | WP-001 | Planned |
| WP-008 | End-to-end intent-to-evidence demonstration | WP-003, WP-004, WP-005, WP-006 | Planned |

## Traces

- Parent PRD: [`../../PRD.md`](../../PRD.md)
- HLD: [`../../HLD.md`](../../HLD.md)
- LLD: [`../../LLD.md`](../../LLD.md)
- Verification: [`../../verification/requirements-traceability-matrix.md`](../../verification/requirements-traceability-matrix.md)
- Human source: [`../../intent/000-source-prompts.md`](../../intent/000-source-prompts.md)
