# Video, HDR, Color, and Frame-Pacing Architecture

## Pipeline decomposition

```text
render → acquire → memory transition → optional transform
→ optional encode → transport → optional decode
→ memory transition → compose → scanout → panel
```

Every stage is separately measured and removable.

## Same-host fast path

For VFIO Windows guest to Linux host:

```text
guest GPU capture → KVMFR/IVSHMEM → host DMA-BUF/import/upload → compositor
```

No codec, chroma conversion, or network path is expected when KVMFR/shared-memory capability is valid. Standard IVSHMEM may require more CPU/memory traffic than KVMFR GPU DMA.

## Network path

A source surface may negotiate:

- raw/RGBA/P010 on very fast controlled paths;
- AV1 10-bit;
- HEVC Main10;
- H.264 4:4:4 where available;
- lossless/intra modes for text/color workflows.

Codec choice considers actual hardware availability and contention. The 2021 M1 Pro provides hardware H.264/HEVC/ProRes support; AV1 hardware decode should not be assumed for that generation.

## Quality profiles

| Profile | Priority order |
|---|---|
| Desktop text | 4:4:4, sharp scaling, low latency, then frame rate |
| Gaming | frame pacing/refresh/input, HDR, then chroma/bitrate |
| Color-critical | bit depth, primaries/transfer, controlled transform, 4:4:4 |
| Video playback | source cadence, HDR, codec efficiency, audio sync |
| Monitoring | reliability and readable text over maximum fidelity |

## HDR/color descriptor

```yaml
color:
  bit_depth: 10
  primaries: bt2020
  transfer: pq
  matrix: bt2020nc
  range: full
  mastering_luminance_nits: [0.005, 1000]
  content_light_level: optional
  icc_profile_hash: optional
```

Sink capabilities include EDID/display API data and measured calibration where available. Marketing “HDR” is not sufficient.

## Reference displays

- Samsung C27HG70: 2560×1440, high-refresh VA/QLED-class gaming display with HDR capability; validate exact firmware/mode/port behavior.
- 2021 M1 Pro MacBook Pro: Liquid Retina XDR, P3, high luminance, ProMotion up to 120 Hz; platform HDR/EDR path must be validated.

Their different peak brightness, tone behavior, gamut, and refresh require sink-specific mapping.

## Frame pacing

Telemetry includes:

- source present interval;
- capture age;
- encoder queue;
- network delivery and late loss;
- decoder queue;
- compositor present;
- display refresh/VRR state.

Average FPS alone is insufficient. p95/p99 frame time and missed presentation deadlines drive adaptation.

## Degradation ladder

The policy may reduce:

1. optional transforms/effects;
2. bitrate at constant cadence;
3. chroma where profile permits;
4. resolution;
5. frame rate;
6. HDR only if explicitly allowed.

It must not silently increase latency through buffer growth.

## Measurement boundary

Report at least:

- capture callback to client enqueue;
- client enqueue to present request;
- glass-to-glass;
- input-to-photon.

The first must never be labeled as the last.
