# Seamless Surface and Application Presentation

## Meta

- **ID:** `006-seamless-surface-presentation`
- **Created:** 2026-08-28
- **State:** specified
- **Scope:** Full desktops, virtual monitors, semantic app remoting, native pixel proxies, window graphs, MacBook third-display workflow
- **Requirement traces:** `PF-FR-030`, `PF-FR-031`, `PF-FR-032`, `PF-FR-033`, `PF-FR-034`, `PF-FR-035`, `PF-FR-036`, `PF-FR-037`, `PF-FR-040`, `PF-FR-043`
- **Intent traces:** `INT-P001`–`INT-P008`

## Context

The user wants applications and desktops to visibly live anywhere, including a Windows application beside native Mac apps and the MacBook as a third monitor.

## Problem Statement

Whole-desktop streaming is universal but coarse. Per-window pixel capture can look native but struggles with popups, IME, protected surfaces, DPI, and encoder scaling. Semantic protocols are richer but platform/edition constrained.

## Goals

- Support takeover, extend, portal, mirror/pin, and audio-only modes.
- Create stable per-sink virtual display identities.
- Prefer RAIL/Xpra/Waypipe-style semantic remoting.
- Implement pixel-proxy fallback with owned-window graph.
- Preserve DPI, input, IME, drag/drop, clipboard, and focus semantics.
- Support HDR/color/high-refresh negotiation.

## Non-Goals

- Claim the process migrates because its window moved.
- Bypass DRM/secure desktop/permission boundaries.
- Allocate one encoder session per static window indefinitely.
- Require all applications to support semantic remoting.

## User and System Outcomes

The feature must improve the packaged experience while preserving the physical and authority boundaries in `README.md`, `DOMAIN_MODEL.md`, and `ecosystem/boundaries.md`. It is accepted only through evidence produced under the mixed-load reference scenarios.

## Functional Requirements

- `PF-FR-030`
- `PF-FR-031`
- `PF-FR-032`
- `PF-FR-033`
- `PF-FR-034`
- `PF-FR-035`
- `PF-FR-036`
- `PF-FR-037`
- `PF-FR-040`
- `PF-FR-043`

## Technical Approach

1. Implement full-display adapter and virtual-display lifecycle.
2. Integrate semantic protocols by platform.
3. Create source-owned window graph and local proxy model.
4. Use damage tracking and adaptive surface atlas for pixel fallback.
5. Separate raw key from semantic text/IME channels.
6. Add capture exclusion and route ancestry.
7. Validate MacBook third-display and Windows-app-on-Mac scenarios.

## Data and Control Boundaries

- Control metadata is versioned, authenticated, and auditable.
- High-bandwidth payloads remain in direct/shared/peer data planes where possible.
- Cross-product integrations use stable IDs and events, not shared database tables.
- Real-time threads never perform dynamic policy evaluation.
- Failure must preserve or restore a safe route/state.

## Success Criteria

- A Windows app can be placed beside Mac apps with correct modal/popup behavior for the supported app set.
- The MacBook acts as a stable third display without manual display recreation.
- Minimizing a proxy does not unintentionally stop source rendering.
- Protected surfaces fail explicitly and fall back safely.
- HDR/color state is visible and verified.

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
| Window graph APIs are inconsistent | High | Per-platform adapters and whole-desktop fallback. |
| Encoder session exhaustion | High | Atlas/multi-surface streams, semantic protocols, static-damage suppression. |
| Mixed DPI causes coordinate errors | High | Logical/physical coordinate model and test matrix. |
| Mac/Windows shortcut and IME mismatch | High | Dual raw/semantic input channels. |

## Work Packages

| WP ID | Description | Depends On | State |
|---|---|---|---|
| WP-001 | Surface/window graph schema | — | Planned |
| WP-002 | Virtual display lifecycle | WP-001 | Planned |
| WP-003 | Full desktop and extend modes | WP-002 | Planned |
| WP-004 | RAIL/Xpra/Waypipe semantic adapters | WP-001 | Planned |
| WP-005 | Pixel proxy and surface atlas prototype | WP-001, WP-003 | Planned |
| WP-006 | DPI/IME/drag-drop/clipboard semantics | WP-004, WP-005 | Planned |
| WP-007 | HDR/color/high-refresh integration | WP-003, WP-005 | Planned |
| WP-008 | MacBook and overlapping-window acceptance | WP-006, WP-007 | Planned |

## Traces

- Parent PRD: [`../../PRD.md`](../../PRD.md)
- HLD: [`../../HLD.md`](../../HLD.md)
- LLD: [`../../LLD.md`](../../LLD.md)
- Verification: [`../../verification/requirements-traceability-matrix.md`](../../verification/requirements-traceability-matrix.md)
- Human source: [`../../intent/000-source-prompts.md`](../../intent/000-source-prompts.md)
