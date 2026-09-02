# Scenario: Gaming Under Background Agent Load

## Intent

Play at the best valid high-refresh/HDR mode while agents build code, run tests and perform inference. Background throughput is valuable but cannot create frame/input spikes beyond the gaming profile.

## Runtime graph

```mermaid
flowchart LR
  KB[Keyboard/mouse/controller] --> GAME[Game VM / process]
  GAME --> GPU0[RTX 3090 Ti render]
  GPU0 --> DISP[C27HG70]
  GAME --> AUD[Local game audio]
  BUILD[Cargo builds] --> CPUPOOL[Background CPU pool / remote node]
  TEST[Game E2E agent] --> GPU1[Spare GPU or remote node]
  VISION[Video analysis] --> REMGPU[Remote/available accelerator]
  REC[Recording encode] --> ENCODER[Available encoder/copy path]
```

## Scheduler rules

- game input/render/audio are RT1/RT2 and remain local unless the whole game is intentionally streamed;
- shader compilation, asset preprocessing, test instances, recording and vision analysis are independent candidate regions;
- CPU affinity, GPU engine occupancy, memory bandwidth, PCIe copies, encoder sessions and thermal headroom enter admission;
- if the render GPU approaches the frame-time risk threshold, agent GPU work moves, pauses or falls to a lower service class;
- bulk transfers throttle before frame queues grow;
- HDR and refresh degrade only through the declared gaming ladder and are shown to the user.

## Required evidence

Frame-time distribution, input-to-photon, frame age, encoder/copy/GPU occupancy, background completion impact and every scheduler relocation are captured together. Average FPS alone is rejected.
