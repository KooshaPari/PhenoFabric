# Program Dependency DAG

```mermaid
flowchart LR
  W000[PF-WP-000 Baseline] --> W010[010 Capability]
  W000 --> W020[020 Graph core]
  W000 --> W160[160 Security]
  W000 --> W170[170 Evidence]
  W010 --> W030[030 Linux local]
  W020 --> W030
  W010 --> W040[040 VM fast paths]
  W020 --> W040
  W020 --> W050[050 Shell]
  W010 --> W060[060 Win/mac endpoints]
  W020 --> W060
  W030 --> W070[070 LAN media]
  W060 --> W070
  W030 --> W080[080 RT audio]
  W060 --> W080
  W060 --> W090[090 HDR/video]
  W070 --> W090
  W050 --> W100[100 Agent realms]
  W070 --> W100
  W010 --> W110[110 Object plane]
  W020 --> W110
  W010 --> W120[120 Scheduler]
  W110 --> W120
  W120 --> W130[130 Adaptive regions]
  W060 --> W140[140 Seamless apps]
  W070 --> W140
  W090 --> W140
  W050 --> W150[150 WAN/OOB]
  W070 --> W150
  W110 --> W190[190 Atomic research]
  W120 --> W190
  W170 --> W190
  W050 --> W180[180 Packaging]
  W060 --> W180
  W150 --> W180
  W160 --> W180
  W170 --> W180
  W080 --> W200[200 Acceptance]
  W090 --> W200
  W100 --> W200
  W120 --> W200
  W140 --> W200
  W150 --> W200
  W180 --> W200
```

## Parallel lanes

- Graph core, capability inventory, security and evidence begin after the documentation baseline.
- Linux local paths, VM fast paths, shell UX and Windows/macOS endpoints can overlap once their contracts stabilize.
- RT audio and HDR/video are independent specialist lanes sharing endpoint and telemetry contracts.
- Object plane/scheduler can progress without seamless-window completion.
- Atomic interposition is deliberately off the release critical path.

## Forbidden shortcuts

- Packaging cannot wait until the end; clean-machine skeletons start early even though final gate depends on later components.
- Pixel-proxy work cannot bypass semantic-protocol research.
- Adaptive scheduling cannot precede a measured explicit TaskSpec baseline.
- A UI demo cannot mark a transport complete without mixed-load and fault evidence.
