# Latency Measurement Methodology

## Do not conflate stage and glass-to-glass latency

```text
input capture → route → remote injection → application → render queue → capture
→ transform/encode → transport → decode → compositor → scanout → panel
```

Capture-to-client-enqueue is useful diagnostic data but excludes compositor scheduling, scanout and panel response. User-facing claims use input-to-photon or glass-to-glass evidence.

## Clocks

- Same-host stages use one monotonic clock where possible.
- Cross-host software timestamps require clock mapping and uncertainty bounds.
- PTP-like synchronization is used on controlled LAN when available; otherwise two-way estimation reports asymmetry uncertainty.
- Hardware high-speed camera/photodiode/LED loop establishes end-to-end ground truth.
- Audio loopback uses physical or calibrated digital loop and reports converter buffering.

## Sampling

- Warm-up and cold-start cohorts are separate.
- At least 10,000 frame/input samples for stable interactive distributions where practical.
- Report p50/p95/p99/max, late-frame count and consecutive-late bursts.
- Keep raw timestamps; do not reconstruct only from aggregated logs.
- Mixed-load traces align CPU/GPU/encoder/copy/memory/network pressure to each outlier.

## Queue age

The central media metric is **age at presentation**, not merely per-stage service time. A fast decoder consuming an old queued frame is not low latency.
