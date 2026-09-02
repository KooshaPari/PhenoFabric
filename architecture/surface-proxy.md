# Surface Presentation and Native Proxy Windows

## Presentation modes

1. **Takeover:** one realm fills the current zone.
2. **Extend:** sink becomes a virtual monitor of source realm.
3. **Portal:** one application/window tree appears as a local proxy.
4. **Mirror/pin:** surface stays visible without following input focus.
5. **Audio-only/control-only:** independent nonvisual routes.

## Semantic-first adapter order

- Windows RAIL/RemoteApp/FreeRDP/WinApps-compatible path;
- Linux Xpra/X2Go for X11;
- Waypipe/compositor protocol for Wayland;
- platform/hypervisor integration such as Coherence/Seamless/WSLg-like mechanisms;
- pixel proxy;
- full desktop fallback.

## Owned-window graph

```yaml
window:
  id: source-window-id
  app_id: ...
  parent: optional
  owner: optional
  kind: toplevel|dialog|popup|menu|tooltip|tool
  modality: none|parent|application
  bounds_logical: [...]
  z_group: ...
  capture_surface: ...
```

The source is authoritative for ownership/modality; the sink compositor is authoritative for local top-level placement.

## Pixel proxy capture

Windows prototype path:

```text
source app moved to parking virtual display
→ Windows Graphics Capture
→ damage detection
→ surface atlas or dedicated stream
→ QUIC/shared path
→ VideoToolbox/Metal or local decoder/compositor
→ native AppKit window
```

## Surface atlas

Static and low-motion windows share an atlas. Metadata identifies crop, scale, color, damage, and timestamps. High-motion/HDR windows may receive dedicated streams. This avoids one encoder session per window.

## Minimize/occlusion

Local minimize does not necessarily minimize the source window, because many apps stop/throttle rendering. The source may remain on a hidden parking display with reduced capture priority.

## Input

- pointer translated through logical coordinate and scale;
- raw mode available for relevant surface;
- raw keys and semantic text/IME are separate;
- drag/drop uses file/object promises;
- modal proxy raises with parent.

## Protected surfaces

UAC secure desktop, DRM/protected video, credential UI, macOS secure input, or unavailable Wayland capture causes explicit failure. Options: local confirmation, whole desktop, or OOB.

## Recursion

Capture exclusion plus origin/hop ancestry prevents a proxy from being recaptured into itself or bounced between devices.

## Process migration claim

Moving a proxy changes presentation, not process location. UI must never label it live migration unless process/VM state actually moves.
