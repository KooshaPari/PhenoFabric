# Unified Product Shell and Workspace Model

## Meta

- **ID:** `001-unified-product-shell`
- **Created:** 2026-08-28
- **State:** specified
- **Scope:** Cross-platform packaged UI, CLI, SDK, installer, discovery, graph canvas, simple workspace controls
- **Requirement traces:** `PF-FR-001`, `PF-FR-002`, `PF-FR-003`, `PF-FR-004`, `PF-FR-005`, `PF-FR-006`
- **Intent traces:** `INT-P001`–`INT-P008`

## Context

The requested experience must be one product even though the underlying implementation composes many local and network data planes. Without a coherent shell, identity model, installer, and workspace transaction model, the result remains a collection of tools.

## Problem Statement

Current tools expose backend-specific concepts and configuration. Users must remember which machine is the host, which streamer is active, how to switch input, where audio is routed, and how an agent-created VM is reached.

## Goals

- Ship one recognizable application identity across Linux, Windows, and macOS.
- Expose simple workspace actions and an expert graph canvas over the same state.
- Discover devices, realms, sessions, seats, surfaces, resources, and adapters.
- Persist and transactionally activate versioned workspaces.
- Provide diagnostics, update/rollback, and an emergency local-control mode.
- Keep all media and bulk payloads out of the coordinator hot path.

## Non-Goals

- Implement every media, KVM, or scheduler backend inside the shell.
- Require the graph canvas for ordinary daily use.
- Erase platform permission differences.
- Claim branding is final.

## User and System Outcomes

The feature must improve the packaged experience while preserving the physical and authority boundaries in `README.md`, `DOMAIN_MODEL.md`, and `ecosystem/boundaries.md`. It is accepted only through evidence produced under the mixed-load reference scenarios.

## Functional Requirements

- `PF-FR-001`
- `PF-FR-002`
- `PF-FR-003`
- `PF-FR-004`
- `PF-FR-005`
- `PF-FR-006`

## Technical Approach

1. Define canonical identity and discovery contracts before UI.
2. Build a local-first coordinator and endpoint simulator.
3. Implement graph/workspace schema and transactional patches.
4. Create platform-native shell wrappers around a shared core API.
5. Package adapters as capability providers with independent health.
6. Add signed update, repair, rollback, and diagnostic export.

## Data and Control Boundaries

- Control metadata is versioned, authenticated, and auditable.
- High-bandwidth payloads remain in direct/shared/peer data planes where possible.
- Cross-product integrations use stable IDs and events, not shared database tables.
- Real-time threads never perform dynamic policy evaluation.
- Failure must preserve or restore a safe route/state.

## Success Criteria

- One installer launches a usable shell on all three platforms.
- A desk/couch workspace can be activated in one action.
- Adapter failure degrades capabilities without crashing the shell.
- Workspace activation is idempotent and reversible.
- An expert can inspect why every visible route exists.

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
| UI becomes a patchbay only experts understand | High | Provide simple modes, templates, command palette, and progressive disclosure. |
| Cross-platform parity creates lowest-common-denominator UX | High | Share semantics and schemas, not pixel-identical implementation. |
| Coordinator becomes data bottleneck | High | Control metadata only; peer-to-peer data planes. |
| Privileged installation damages trust | High | Narrow helpers, signed artifacts, repair/uninstall paths. |

## Work Packages

| WP ID | Description | Depends On | State |
|---|---|---|---|
| WP-001 | Identity, registry, and topology schema | — | Planned |
| WP-002 | Endpoint discovery and simulator | WP-001 | Planned |
| WP-003 | Desired graph and workspace transaction engine | WP-001 | Planned |
| WP-004 | CLI and public API baseline | WP-002, WP-003 | Planned |
| WP-005 | Graph canvas and simple workspace UX | WP-004 | Planned |
| WP-006 | Platform shell integrations | WP-004 | Planned |
| WP-007 | Installer, updates, repair, rollback | WP-006 | Planned |
| WP-008 | Cross-platform acceptance and accessibility | WP-005, WP-007 | Planned |

## Traces

- Parent PRD: [`../../PRD.md`](../../PRD.md)
- HLD: [`../../HLD.md`](../../HLD.md)
- LLD: [`../../LLD.md`](../../LLD.md)
- Verification: [`../../verification/requirements-traceability-matrix.md`](../../verification/requirements-traceability-matrix.md)
- Human source: [`../../intent/000-source-prompts.md`](../../intent/000-source-prompts.md)
