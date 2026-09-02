# Observability and Evidence Operations

## Three data rates

1. **RT path:** fixed-size counters/timestamps written lock-free; no policy, allocation or blocking.
2. **Local aggregation:** stage histograms, pressure, topology and route health.
3. **Durable evidence:** selected raw traces, summaries, manifests and cross-product references.

## Core signals

- graph transaction state and lease/fencing token;
- desired link versus compiled stages;
- buffer memory domains, copies, formats and fences;
- input event age and pressed-state transitions;
- audio callback, xrun, clock drift and buffer occupancy;
- frame capture/encode/network/decode/composition/presentation age;
- CPU scheduling, IRQ/DPC, GPU engines, VRAM, PCIe and memory bandwidth;
- object authority/residency/transfer/cache/spill;
- task candidate cost terms, chosen plan and regret sample;
- adapter/helper version and privilege boundary;
- agent realm TTL/budget/surface/focus events.

## Privacy

Content payloads are excluded by default. Titles, clipboard formats, file paths, screenshots and audio may be sensitive. Evidence collection is policy-scoped, redacted and explicit, with short retention for raw media unless a test requires it.
