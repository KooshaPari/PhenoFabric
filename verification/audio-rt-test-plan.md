# Real-Time Audio and MIDI Test Plan

## Reference workload

Ableton Live 12 Suite project with audio interface, virtual instruments/effects, automation, MIDI/controller input, monitoring and recording. Tests use 48 kHz and several buffer sizes; exact plugin set and interface are versioned with the evidence.

## Test families

1. Local ASIO/CoreAudio/PipeWire/JACK baseline.
2. Protected RT island with background cargo build, inference, storage and game E2E.
3. Audio routed to another device’s speakers while input remains local.
4. Microphone and MIDI returned from MacBook/remote endpoint.
5. Network audio clock drift and ASRC soak.
6. Remote DSP/plugin experiment with explicit graph slack.
7. Device hotplug, sample-rate change, sleep/wake and network interruption.
8. Admission failure when the route cannot meet deadline.

## Metrics

- callback execution and wake-up latency;
- xrun count and exact cause window;
- physical round-trip and one-way estimates;
- buffer occupancy and underrun/overrun margin;
- drift estimate and ASRC ratio variation;
- MIDI timestamp error/jitter/order;
- CPU/IRQ affinity and DPC-like platform spikes;
- background work displaced/throttled and completion impact.

## Acceptance

A route with hard xrun policy is admitted only when the full graph has margin. Zero xruns in the reference soak is necessary but not sufficient; injected contention and recovery must also pass. Remote DSP that cannot fit remains a look-ahead/offline route rather than being marketed as live monitoring.
