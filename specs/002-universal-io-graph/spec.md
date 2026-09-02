# Universal I/O Graph, Seats, and Focus

## Meta

- **ID:** `002-universal-io-graph`
- **Created:** 2026-08-28
- **State:** specified
- **Scope:** Typed ports, graph transactions, input/output leases, HID/MIDI/audio/video/clipboard/file endpoint semantics
- **Requirement traces:** `PF-FR-010`, `PF-FR-011`, `PF-FR-015`, `PF-FR-016`, `PF-FR-020`, `PF-FR-021`, `PF-FR-022`, `PF-FR-023`, `PF-FR-024`, `PF-FR-025`, `PF-FR-026`
- **Intent traces:** `INT-P001`–`INT-P008`

## Context

PipeWire/JACK demonstrate that linked nodes and typed ports can make complex routing inspectable and composable. The requested product generalizes that model beyond audio while preserving hard timing and platform security.

## Problem Statement

Deskflow-class tools route input but not output; remote desktops bundle video/audio/input; VM input pass-through binds devices directly; application-level routes lack one transactional authority.

## Goals

- Represent all connectable capabilities as typed nodes and ports.
- Separate desired links from compiled routes.
- Implement transactional graph changes and fenced exclusive focus.
- Support absolute and raw-relative pointer modes.
- Prevent stuck keys, stale focus, and recursive routes.
- Allow independent and coupled input/output focus switching.

## Non-Goals

- Force all port types through one wire format.
- Treat eventual consistency as sufficient for exclusive input.
- Bypass Wayland/macOS permission models.
- Tunnel all special devices as generic USB.

## User and System Outcomes

The feature must improve the packaged experience while preserving the physical and authority boundaries in `README.md`, `DOMAIN_MODEL.md`, and `ecosystem/boundaries.md`. It is accepted only through evidence produced under the mixed-load reference scenarios.

## Functional Requirements

- `PF-FR-010`
- `PF-FR-011`
- `PF-FR-015`
- `PF-FR-016`
- `PF-FR-020`
- `PF-FR-021`
- `PF-FR-022`
- `PF-FR-023`
- `PF-FR-024`
- `PF-FR-025`
- `PF-FR-026`

## Technical Approach

1. Define port type system, format negotiation, timing, and memory-domain descriptors.
2. Build Linux evdev/libevdev acquisition and persistent uinput endpoints.
3. Add libei/EIS and XDG portal mediation for Wayland.
4. Implement Windows Raw Input/SendInput prototype and Virtual HID path.
5. Implement macOS CGEvent/Event Tap endpoint with explicit permissions.
6. Build focus lease/fencing state machine and pressed-control ledger.
7. Compile hotkeys and workspace changes into graph transactions.

## Data and Control Boundaries

- Control metadata is versioned, authenticated, and auditable.
- High-bandwidth payloads remain in direct/shared/peer data planes where possible.
- Cross-product integrations use stable IDs and events, not shared database tables.
- Real-time threads never perform dynamic policy evaluation.
- Failure must preserve or restore a safe route/state.

## Success Criteria

- One keyboard/mouse bundle routes across host, VMs, Mac, and LAN peers.
- No stuck modifiers/buttons in one million randomized switches.
- Raw mouse capture cannot trigger cursor-edge switching.
- Stale tokens cannot inject after focus transfer.
- Emergency local input works if coordinator or network fails.

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
| Input router becomes a keylogger-class security boundary | High | Minimize privileged code, visible state, capability grants, audit. |
| Platform injection differs from hardware HID | High | Use staged SendInput/CGEvent prototypes and virtual HID where required. |
| Graph transaction stalls RT input | High | Precompile and swap routes atomically. |
| Multiple coordinators create split brain | High | Single lease authority and fencing tokens. |

## Work Packages

| WP ID | Description | Depends On | State |
|---|---|---|---|
| WP-001 | Port type and graph transaction schemas | — | Planned |
| WP-002 | Linux evdev/uinput broker | WP-001 | Planned |
| WP-003 | Wayland libei/portal adapter | WP-002 | Planned |
| WP-004 | Windows input endpoint | WP-001 | Planned |
| WP-005 | macOS input endpoint | WP-001 | Planned |
| WP-006 | Lease, fencing, and pressed-state reconciliation | WP-002, WP-004, WP-005 | Planned |
| WP-007 | Hotkeys, OSD, and break-glass path | WP-006 | Planned |
| WP-008 | Fuzz, fault, and latency validation | WP-003, WP-007 | Planned |

## Traces

- Parent PRD: [`../../PRD.md`](../../PRD.md)
- HLD: [`../../HLD.md`](../../HLD.md)
- LLD: [`../../LLD.md`](../../LLD.md)
- Verification: [`../../verification/requirements-traceability-matrix.md`](../../verification/requirements-traceability-matrix.md)
- Human source: [`../../intent/000-source-prompts.md`](../../intent/000-source-prompts.md)
