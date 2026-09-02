# Packaging, Deployment, Upgrade, and Recovery

## Meta

- **ID:** `012-packaging-operations`
- **Created:** 2026-08-28
- **State:** specified
- **Scope:** Cross-platform install, signed components, configuration, coordinator topology, updates, rollback, diagnostics, OOB recovery
- **Requirement traces:** `PF-FR-001`, `PF-FR-003`, `PF-FR-006`, `PF-FR-084`, `PF-FR-088`
- **Intent traces:** `INT-P001`–`INT-P008`

## Context

The target is a packaged product, not a research code dump. Deep platform integration introduces drivers, services, permissions, and failure modes that must be installable and reversible.

## Problem Statement

A technically excellent mesh fails as a product if setup requires manually configuring every backend, if updates strand VMs, or if removing a driver breaks input/display/audio.

## Goals

- Provide one installer per platform with capability-driven optional components.
- Support local-first setup and secure fleet enrollment.
- Version and migrate configuration and schemas.
- Roll back failed upgrades.
- Export diagnostics and restore safe local control.
- Document home, lab, WAN, and managed deployment topologies.

## Non-Goals

- Require Kubernetes for a home installation.
- Install every privileged helper by default.
- Make cloud availability a prerequisite for local work.
- Auto-update kernel/driver components without rollback.

## User and System Outcomes

The feature must improve the packaged experience while preserving the physical and authority boundaries in `README.md`, `DOMAIN_MODEL.md`, and `ecosystem/boundaries.md`. It is accepted only through evidence produced under the mixed-load reference scenarios.

## Functional Requirements

- `PF-FR-001`
- `PF-FR-003`
- `PF-FR-006`
- `PF-FR-084`
- `PF-FR-088`

## Technical Approach

1. Component manifest with capability and privilege declarations.
2. Signed packages and platform-native service management.
3. First-run topology discovery and guided policy.
4. Versioned config/schema migrations.
5. A/B or previous-version rollback for daemon/shell; explicit driver rollback.
6. Recovery mode and OOB inventory.
7. Release channels and compatibility certification.

## Data and Control Boundaries

- Control metadata is versioned, authenticated, and auditable.
- High-bandwidth payloads remain in direct/shared/peer data planes where possible.
- Cross-product integrations use stable IDs and events, not shared database tables.
- Real-time threads never perform dynamic policy evaluation.
- Failure must preserve or restore a safe route/state.

## Success Criteria

- A clean machine can join the fabric without backend-specific manual editing for the supported baseline.
- Uninstall restores native input/display/audio behavior.
- Failed update rolls back without losing workspace state.
- Local routes continue during cloud/control outage where policy allows.
- Diagnostic bundle identifies broken adapter or permission.

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
| Driver signing/distribution blocks release | High | Prototype user-space path first; isolate optional drivers. |
| Auto-detection chooses dangerous defaults | High | Dry-run, explicit privilege grant, conservative fallback. |
| Schema migration corrupts workspaces | High | Immutable backup, migration validation, rollback. |
| OOB devices become attack surface | High | Separate management network and credential policy. |

## Work Packages

| WP ID | Description | Depends On | State |
|---|---|---|---|
| WP-001 | Component/privilege manifest | — | Planned |
| WP-002 | Linux packaging and service lifecycle | WP-001 | Planned |
| WP-003 | Windows packaging and driver lifecycle | WP-001 | Planned |
| WP-004 | macOS packaging/notarization/permissions | WP-001 | Planned |
| WP-005 | Enrollment, config, and schema migration | WP-002, WP-003, WP-004 | Planned |
| WP-006 | Update, repair, rollback, uninstall | WP-005 | Planned |
| WP-007 | Recovery/OOB and diagnostic bundle | WP-005 | Planned |
| WP-008 | Release channels and clean-machine acceptance | WP-006, WP-007 | Planned |

## Traces

- Parent PRD: [`../../PRD.md`](../../PRD.md)
- HLD: [`../../HLD.md`](../../HLD.md)
- LLD: [`../../LLD.md`](../../LLD.md)
- Verification: [`../../verification/requirements-traceability-matrix.md`](../../verification/requirements-traceability-matrix.md)
- Human source: [`../../intent/000-source-prompts.md`](../../intent/000-source-prompts.md)
