# Security, Identity, Leases, Audit, and Evidence

## Meta

- **ID:** `009-security-identity-evidence`
- **Created:** 2026-08-28
- **State:** specified
- **Scope:** Mutual identity, capability authorization, privileged helper isolation, route leases, evidence bundles, OOB isolation
- **Requirement traces:** `PF-FR-080`, `PF-FR-081`, `PF-FR-082`, `PF-FR-083`, `PF-FR-084`, `PF-FR-087`, `PF-FR-088`
- **Intent traces:** `INT-P001`–`INT-P008`

## Context

A system that captures and redirects every key, screen, audio stream, file, and compute workload is more privileged than an ordinary remote desktop. Security cannot be bolted on.

## Problem Statement

Network reachability, shared accounts, or one long-lived secret are insufficient. Platform helpers require elevated permissions, and stale routes can cause dangerous split-brain behavior.

## Goals

- Mutually authenticate devices and principals.
- Authorize each route/action with narrow capabilities.
- Isolate privileged helpers and drivers.
- Use leases/fencing for exclusive resources.
- Produce tamper-evident decision/evidence records.
- Secure OOB management separately.

## Non-Goals

- Use one credential for the whole fleet.
- Trust LAN location.
- Record private payloads by default.
- Expose OOB management directly to the Internet.

## User and System Outcomes

The feature must improve the packaged experience while preserving the physical and authority boundaries in `README.md`, `DOMAIN_MODEL.md`, and `ecosystem/boundaries.md`. It is accepted only through evidence produced under the mixed-load reference scenarios.

## Functional Requirements

- `PF-FR-080`
- `PF-FR-081`
- `PF-FR-082`
- `PF-FR-083`
- `PF-FR-084`
- `PF-FR-087`
- `PF-FR-088`

## Technical Approach

1. Device identity with platform key stores and revocable certificates.
2. Capability tokens bound to principal, action, source, sink, purpose, TTL, and topology epoch.
3. Small audited privileged helper protocols.
4. Lease authority with fencing and safe local priority.
5. Structured audit plus content-addressed evidence artifacts.
6. Dedicated management-network policy for KVM/OOB.

## Data and Control Boundaries

- Control metadata is versioned, authenticated, and auditable.
- High-bandwidth payloads remain in direct/shared/peer data planes where possible.
- Cross-product integrations use stable IDs and events, not shared database tables.
- Real-time threads never perform dynamic policy evaluation.
- Failure must preserve or restore a safe route/state.

## Success Criteria

- Compromising one endpoint credential does not grant fleet-wide control.
- Stale route holders are rejected after failover/partition.
- Sensitive route activation is visible and attributable.
- Diagnostic bundles redact payloads and secrets by default.
- OOB access remains available during OS failure but isolated from normal data plane.

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
| Privilege surface is too broad | High | Split helpers by device class; deny-by-default protocols. |
| Certificate/lease service outage blocks local work | High | Local safe mode and bounded offline leases. |
| Audit becomes surveillance | High | Metadata-only defaults and explicit payload capture. |
| Pairing is socially engineered | High | Out-of-band confirmation, device display, revocation UI. |

## Work Packages

| WP ID | Description | Depends On | State |
|---|---|---|---|
| WP-001 | Threat model and capability matrix | — | Planned |
| WP-002 | Device identity and pairing | WP-001 | Planned |
| WP-003 | Authorization token and policy engine | WP-001, WP-002 | Planned |
| WP-004 | Privileged helper isolation | WP-001 | Planned |
| WP-005 | Lease/fencing service | WP-003 | Planned |
| WP-006 | Audit/evidence bundle format | WP-003, WP-004 | Planned |
| WP-007 | OOB management isolation | WP-001, WP-002 | Planned |
| WP-008 | Red-team, partition, and recovery validation | WP-005, WP-006, WP-007 | Planned |

## Traces

- Parent PRD: [`../../PRD.md`](../../PRD.md)
- HLD: [`../../HLD.md`](../../HLD.md)
- LLD: [`../../LLD.md`](../../LLD.md)
- Verification: [`../../verification/requirements-traceability-matrix.md`](../../verification/requirements-traceability-matrix.md)
- Human source: [`../../intent/000-source-prompts.md`](../../intent/000-source-prompts.md)
