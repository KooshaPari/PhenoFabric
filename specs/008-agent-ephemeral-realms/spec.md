# Agent-Created Realms and Non-Stealing Surface Publication

## Meta

- **ID:** `008-agent-ephemeral-realms`
- **Created:** 2026-08-28
- **State:** specified
- **Scope:** Realm provisioning, enrollment, TTL, surface publication, attention requests, resource budgets, fallback consoles
- **Requirement traces:** `PF-FR-026`, `PF-FR-037`, `PF-FR-085`, `PF-FR-086`, `PF-FR-087`
- **Intent traces:** `INT-P001`–`INT-P008`

## Context

Agents may create an arbitrary number of VMs or other items and need to make them trivially accessible to the user. Automation must not turn that convenience into focus theft or privilege escalation.

## Problem Statement

Current agent runtimes create terminals, containers, VMs, browsers, and reports with inconsistent discovery and no unified publication or cleanup contract.

## Goals

- Allow agents to request realms under authorized constraints.
- Enroll realms with short-lived identity and capability descriptors.
- Publish desktops, windows, terminals, reports, logs, and artifacts.
- Request attention without stealing focus.
- Apply TTL, budget, cleanup, and fallback-console policy.
- Link runs and surfaces to ecosystem IDs and evidence.

## Non-Goals

- Give agents unrestricted human input or filesystem access.
- Require all agent outputs to become pixel streams.
- Keep expired realms alive silently.
- Make the fabric the labor/planning authority.

## User and System Outcomes

The feature must improve the packaged experience while preserving the physical and authority boundaries in `README.md`, `DOMAIN_MODEL.md`, and `ecosystem/boundaries.md`. It is accepted only through evidence produced under the mixed-load reference scenarios.

## Functional Requirements

- `PF-FR-026`
- `PF-FR-037`
- `PF-FR-085`
- `PF-FR-086`
- `PF-FR-087`

## Technical Approach

1. Define RealmRequest, Publication, AttentionRequest, and TTL policies.
2. Integrate thegent/ShareCLI/NVMS provisioning adapters.
3. Create enrollment bootstrap and short-lived certificates.
4. Publish semantic surfaces before desktops where possible.
5. Add notification/pin/portal/takeover interaction.
6. Export lifecycle events to SessionLedger/Tracera.

## Data and Control Boundaries

- Control metadata is versioned, authenticated, and auditable.
- High-bandwidth payloads remain in direct/shared/peer data planes where possible.
- Cross-product integrations use stable IDs and events, not shared database tables.
- Real-time threads never perform dynamic policy evaluation.
- Failure must preserve or restore a safe route/state.

## Success Criteria

- An agent creates a realm and publishes a usable surface without manual address discovery.
- No agent can seize focus without explicit policy.
- Expired realm access is revoked immediately and resources are reclaimed.
- Every realm has a fallback console or explicit absence.
- Run, requirement, and evidence IDs remain linked.

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
| Realm explosion exhausts resources | High | Admission, budgets, quotas, TTL, pressure-aware cleanup. |
| Agent publishes malicious UI | High | Origin labels, sandbox, no implicit focus, capability boundaries. |
| Bootstrap secret leaks | High | ECDH pairing/bootstrap tokens with short expiry. |
| Pixel surfaces hide richer semantics | High | Prefer terminal/report/artifact types. |

## Work Packages

| WP ID | Description | Depends On | State |
|---|---|---|---|
| WP-001 | Realm and publication schemas | — | Planned |
| WP-002 | Provisioning adapter contract | WP-001 | Planned |
| WP-003 | Enrollment and short-lived identity | WP-001 | Planned |
| WP-004 | Surface catalog and attention workflow | WP-001, WP-003 | Planned |
| WP-005 | TTL, budget, cleanup, and fallback | WP-002, WP-003 | Planned |
| WP-006 | thegent/ShareCLI/NVMS integration | WP-002 | Planned |
| WP-007 | SessionLedger/Tracera event export | WP-004, WP-005 | Planned |
| WP-008 | Abuse, scale, and failure testing | WP-006, WP-007 | Planned |

## Traces

- Parent PRD: [`../../PRD.md`](../../PRD.md)
- HLD: [`../../HLD.md`](../../HLD.md)
- LLD: [`../../LLD.md`](../../LLD.md)
- Verification: [`../../verification/requirements-traceability-matrix.md`](../../verification/requirements-traceability-matrix.md)
- Human source: [`../../intent/000-source-prompts.md`](../../intent/000-source-prompts.md)
