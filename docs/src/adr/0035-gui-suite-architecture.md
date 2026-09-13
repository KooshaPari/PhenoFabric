# ADR-0035 — Phenotype Fabric GUI Suite

## Status

Proposed

## Date

2026-09-12

## Context

R3 deliverables (fabric-daemon, fabric-persist, fabric-cli, wire transport) provide the
backend for a complete application GUI. The product spec (001) defines the GUI as
WP-005 "Graph canvas and simple workspace UX." The ROADMAP Horizon 1 lists
"graph/workspace UI and desk/couch templates."

The user wants four GUI surfaces:

1. **Tray app** — always-running system tray icon for daemon lifecycle and quick actions
2. **TUI** — terminal-based interactive UI for graph/routes/workspaces (ratatui)
3. **Native GUI** — desktop window with Parsec-style surface streaming (egui + custom renderer)
4. **Web frontend** — remote hosted, multi-tenant, Parsec-style surface streaming (Leptos/WASM)

The Parsec analogy means options 3 and 4 are not just dashboards — they stream
live surface frames (video) from the daemon's SurfaceRegistry, similar to how
Parsec streams remote desktop frames. The daemon already has:
- SurfaceRegistry with lease tracking (spec 024)
- Multihop compiler with HEVC/AV1 encode/decode stages (ADR-0032)
- Wire transport with probe/replan message types (spec 025)
- Health endpoint returning daemon state

## Decision

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    fabric-daemon                         │
│  ┌─────────────┐ ┌──────────────┐ ┌──────────────────┐  │
│  │ Coordinator  │ │ WireServer   │ │ SurfaceRegistry  │  │
│  │ (topology,   │ │ (TCP, JSON   │ │ (leases, frame   │  │
│  │  leases,     │ │  wire proto) │ │  streaming)      │  │
│  │  plans)      │ │              │ │                  │  │
│  └──────┬───────┘ └──────┬───────┘ └────────┬─────────┘  │
│         │                │                   │            │
│         └────────────────┼───────────────────┘            │
│                          │                                │
│              HTTP/JSON + frame transport                  │
└──────────────────────┬───────────────────────────────────┘
                       │
        ┌──────────────┼──────────────┬──────────────┐
        │              │              │              │
   ┌────▼────┐   ┌─────▼─────┐  ┌────▼────┐   ┌────▼────┐
   │ Tray App│   │    TUI    │  │ Native  │   │  Web    │
   │ (tray-  │   │ (ratatui) │  │  GUI    │   │ Frontend│
   │  icon)  │   │           │  │ (egui)  │   │ (Leptos)│
   └─────────┘   └───────────┘  └─────────┘   └─────────┘
```

### Layer 1: Tray App (`fabric-tray`)

- **Crate:** `crates/fabric-tray/`
- **Framework:** `tray-icon` + `tao` (event loop)
- **Responsibilities:**
  - System tray icon with status indicator (green/yellow/red)
  - Menu: Start/Stop daemon, Health status, Open TUI, Open GUI, Open Web
  - Spawns/monitors `fabric-daemon` child process
  - Polls daemon health endpoint periodically
  - Shows desktop notifications on daemon events (epoch change, lease failure)
- **Platform:** macOS, Linux, Windows (via `tray-icon` cross-platform)
- **No window** — tray icon only, menu-driven

### Layer 2: TUI (`fabric tui`)

- **Crate:** `fabric-cli` subcommand (no new crate)
- **Framework:** `ratatui` + `crossterm`
- **Views:**
  - Topology graph (node/edge table with health status)
  - Route plans (compiled routes with score breakdown)
  - Workspaces (create/list/show/delete)
  - Leases (active/expired/revoked with handle, spec, epoch)
  - Daemon health (uptime, epoch, counts)
- **Mode:** Direct API calls to fabric-graph in-process (no daemon needed)
- **Keybind:** `q` quit, `Tab` switch view, `j/k` navigate, `Enter` select, `n` new workspace

### Layer 3: Native GUI (`fabric-gui`)

- **Crate:** `crates/fabric-gui/`
- **Framework:** `egui` + `eframe` + custom surface renderer
- **Responsibilities:**
  - Topology graph canvas (zoomable, pannable, node health heatmap)
  - Workspace management panel
  - Route visualization (animated flow on edges)
  - **Surface streaming viewport** — Parsec-style:
    - Connects to daemon wire server
    - Receives encoded frames (HEVC/AV1) via frame transport
    - Decodes and renders in egui texture
    - Forwards input events (keyboard/mouse) back to daemon
  - Lease inspector panel
  - Health/status bar
- **Transport:** Frame channel over TCP (daemon → GUI), separate from wire proto
- **Deferred:** Full WebRTC transport deferred to R5 (local TCP first)

### Layer 4: Web Frontend (`fabric-web`)

- **Crate:** `crates/fabric-web/` (Leptos WASM app)
- **Framework:** Leptos + WASM
- **Responsibilities:**
  - Same views as native GUI
  - Multi-tenant: user authentication, workspace isolation
  - Remote surface streaming via WebRTC
  - Hosted deployment (SaaS model)
- **Transport:** WebRTC for frames, WebSocket for control
- **Deferred:** R5+ (requires WebRTC stack, auth, deployment infra)

### Shared: Frame Transport Protocol

All GUI surfaces that stream frames use a common frame transport:

```
FrameTransport:
  - Header: { frame_id, timestamp_us, width, height, format, surface_handle }
  - Body: encoded frame bytes (HEVC/AV1/raw)
  - Control: { input_event, ack, resize, keyframe_request }
```

This is a new wire message type added to the daemon's wire server.

### Crate Layout

```
crates/
  fabric-tray/          # System tray app (new)
  fabric-gui/           # Native GUI with surface streaming (new)
  fabric-web/           # Leptos WASM app (new)
  fabric-cli/           # TUI added as subcommand (existing)
  fabric-daemon/        # Frame transport added to wire_server (existing)
```

## Consequences

### Positive
- Tray app provides immediate always-on daemon management
- TUI gives power users full graph/route inspection without leaving terminal
- Native GUI enables Parsec-like surface streaming for local/LAN use
- Web frontend enables remote multi-tenant access
- Shared frame transport protocol works across all GUI surfaces
- Leptos enables Rust-on-both-sides (no API translation layer)

### Negative
- Four GUI surfaces to maintain (mitigated: shared backend, shared frame protocol)
- Surface streaming is complex (mitigated: daemon already has HEVC/AV1 stages)
- WebRTC for web is deferred (mitigated: TCP frame transport works for local first)
- egui has limited styling (mitigated: functional-first, not pixel-perfect)

### Risks
- Frame transport latency could exceed Parsec-like expectations (mitigated: local TCP, zero-copy where possible)
- Cross-platform tray icon behavior varies (mitigated: `tray-icon` abstracts this)
- Leptos WASM bundle size could be large (mitigated: code splitting, lazy loading)

## References

- Spec 001: Unified Product Shell (WP-005)
- Spec 019: Surface Plane
- Spec 024: Surface Plane Runtime
- Spec 025: Wire Transport Contract
- ADR-0032: Multihop Route Compiler (HEVC/AV1 stages)
- ADR-0033: Fabric Daemon Architecture
- ROADMAP.md Horizon 1
