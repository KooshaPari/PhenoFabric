# Cross-Platform Continuity, Audio, Clipboard, and Files

## Meta

- **ID:** `007-platform-continuity-adapters`
- **Created:** 2026-08-28
- **State:** specified
- **Scope:** Linux/Windows/macOS endpoint adapters, PipeWire/JACK/CoreAudio/WASAPI, typed clipboard, file send/sync/mount, platform permissions
- **Requirement traces:** `PF-FR-050`, `PF-FR-051`, `PF-FR-052`, `PF-FR-053`, `PF-FR-054`, `PF-FR-055`, `PF-FR-083`, `PF-FR-088`
- **Intent traces:** `INT-P001`–`INT-P008`

## Context

The requested experience includes Apple-like continuity across heterogeneous operating systems, professional audio, cursor movement, clipboard, network drives, and seamless file movement.

## Problem Statement

Each platform has different input, capture, virtual display, audio, clipboard, file, permission, and secure-surface models. Pretending one lowest-level API works everywhere is false.

## Goals

- Provide equivalent product semantics through platform-native adapters.
- Implement audio/MIDI as first-class graph ports.
- Preserve clock domains and bounded latency.
- Implement typed clipboard with provenance and loop suppression.
- Support explicit send, persistent sync, and live mount modes.
- Make permission and capability loss visible.

## Non-Goals

- Clone AirDrop internals as the core protocol.
- Assume network mounts replace object transfer or sync.
- Encode local audio that can use shared/direct paths.
- Hide required macOS/Wayland user authorization.

## User and System Outcomes

The feature must improve the packaged experience while preserving the physical and authority boundaries in `README.md`, `DOMAIN_MODEL.md`, and `ecosystem/boundaries.md`. It is accepted only through evidence produced under the mixed-load reference scenarios.

## Functional Requirements

- `PF-FR-050`
- `PF-FR-051`
- `PF-FR-052`
- `PF-FR-053`
- `PF-FR-054`
- `PF-FR-055`
- `PF-FR-083`
- `PF-FR-088`

## Technical Approach

1. Implement platform capability matrix and simulators.
2. Build local audio graph adapters and clock descriptors.
3. Prototype network audio with timestamping, elastic buffers, and ASRC.
4. Define MIME-aware clipboard objects and sensitivity policy.
5. Integrate direct file transfer, Syncthing-class sync, and SMB/NFS/SFTP mounts.
6. Map permission lifecycle into health and remediation UI.

## Data and Control Boundaries

- Control metadata is versioned, authenticated, and auditable.
- High-bandwidth payloads remain in direct/shared/peer data planes where possible.
- Cross-product integrations use stable IDs and events, not shared database tables.
- Real-time threads never perform dynamic policy evaluation.
- Failure must preserve or restore a safe route/state.

## Success Criteria

- Local audio routes avoid unnecessary encode/resample.
- Network audio exposes drift, buffer, and xrun state.
- Clipboard text/image/HTML/file-list representations round-trip without loops.
- Files can be sent, synced, or mounted through one product UX.
- Revoked platform permission yields a specific degraded capability.

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
| Network audio clock drift produces xruns | High | ASRC, bounded buffer, topology-specific admission. |
| Clipboard leaks secrets | High | Sensitivity policy, expiry, provenance, opt-in WAN replication. |
| Mount semantics differ | High | Keep send/sync/mount separate in domain model. |
| Platform updates break private APIs | High | Use public APIs and versioned capability probes. |

## Work Packages

| WP ID | Description | Depends On | State |
|---|---|---|---|
| WP-001 | Platform capability/permission matrix | — | Planned |
| WP-002 | Linux PipeWire/JACK and portal adapters | WP-001 | Planned |
| WP-003 | Windows audio/clipboard/file adapters | WP-001 | Planned |
| WP-004 | macOS audio/clipboard/file adapters | WP-001 | Planned |
| WP-005 | Network audio clock and buffer engine | WP-002, WP-003, WP-004 | Planned |
| WP-006 | Typed clipboard and direct transfer | WP-002, WP-003, WP-004 | Planned |
| WP-007 | Sync and mount integrations | WP-006 | Planned |
| WP-008 | Cross-platform continuity acceptance | WP-005, WP-007 | Planned |

## Traces

- Parent PRD: [`../../PRD.md`](../../PRD.md)
- HLD: [`../../HLD.md`](../../HLD.md)
- LLD: [`../../LLD.md`](../../LLD.md)
- Verification: [`../../verification/requirements-traceability-matrix.md`](../../verification/requirements-traceability-matrix.md)
- Human source: [`../../intent/000-source-prompts.md`](../../intent/000-source-prompts.md)
