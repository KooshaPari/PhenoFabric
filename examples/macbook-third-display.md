# Scenario: M1 Pro MacBook as a Third Display

## Requested link

```text
source realm: Windows VM on main PC
source mode: extend
sink: MacBook built-in panel
quality: desktop-text or gaming profile
HDR: preserve when complete path is validated
refresh: prefer 120 Hz, allow explicit lower fallback
```

## Route candidates

1. Same-host shared path is impossible because the sink is a separate machine.
2. Semantic per-window path is chosen if the user drags one application rather than a monitor.
3. For an extended monitor, provision a stable Windows virtual display and use a direct wired encoded path.
4. Prefer the hardware codec profile actually supported end-to-end by the source GPU, network and 2021 M1 Pro decode/presentation stack.
5. Do not assume AV1 hardware support from Apple Silicon generation alone; benchmark the exact codec/mode.
6. Record HDR/color/scale transform and presentation timing.

## Layout

The source OS sees a persistent display identity associated with `device://macbook/panel`. The user can place normal Windows windows on it. On macOS the sink can be full-screen, borderless or a managed workspace window. Mac native apps remain accessible by workspace policy.

## Fallback ladder

120-Hz 10-bit profile → lower chroma/bitrate with refresh preserved → 60-Hz HDR → 60-Hz SDR/desktop → semantic individual app → full desktop fallback. The exact order changes by workload profile.
