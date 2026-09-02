# Video, HDR, Color, and Frame-Pacing Test Plan

## Sink profiles

- Samsung C27HG70: native mode, supported high-refresh modes, SDR and OS HDR path.
- 2021 M1 Pro MacBook Pro: native scaled modes, ProMotion behavior, SDR/EDR/HDR presentation and concurrent local workload.

Capabilities are probed and recorded; product names are not treated as complete capability descriptors.

## Content classes

| Class | Priority |
|---|---|
| Desktop/text | 4:4:4, sharp edges, low static loss, stable scale |
| Gaming | input/frame age, pacing, refresh, VRR where possible, HDR preservation |
| Color-critical | controlled transform, 10-bit path, gamut/transfer/luminance metadata |
| Video playback | source cadence, decoder efficiency, HDR metadata and A/V sync |

## Tests

- raw/shared path versus H.264/HEVC/available codec profiles;
- 8-bit/10-bit and chroma modes;
- 60/120/144-Hz source/sink combinations;
- source and sink mixed DPI/scaling;
- GPU render/encode/copy contention and decoder load;
- tone/gamut mapping to SDR and differing HDR sinks;
- late-frame bursts during compilation, inference and network impairment;
- sleep/wake, display hotplug and mode changes;
- exact route-stage and color-transform explanation.

## Evidence

High-speed camera or photodiode for visible latency, frame-capture/timestamp traces, HDR metadata capture, calibrated test patterns and visual comparison. Screenshots alone cannot prove HDR luminance, frame pacing or latency.
