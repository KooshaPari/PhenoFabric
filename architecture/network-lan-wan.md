# LAN, WAN, Relay, and Clock Transport

## Transport goals

- direct peer path first;
- separate reliable control, deadline-sensitive datagrams, and bulk object streams;
- end-to-end encryption independent of relay;
- measurable congestion, loss, jitter, and queueing;
- path migration without rebuilding user intent;
- no unbounded retransmission on real-time channels.

## LAN profiles

### Controlled wired LAN

May support:

- raw/light-compressed media;
- PCM/multichannel audio;
- QUIC datagrams;
- optional jumbo MTU after validation;
- PTP-like clock quality;
- optional RDMA/UCX/NIXL experiments.

### Wi-Fi/routed LAN

Use adaptive codec/bitrate, pacing, conservative jitter buffers, and path probes. Wireless link rate is not usable throughput.

## WAN

- ICE/STUN/TURN-like discovery or equivalent;
- direct NAT traversal;
- blind relay fallback;
- congestion control;
- adaptive bitrate/resolution/FPS;
- bounded selective FEC based on loss pattern;
- independent audio/video/input channels;
- coarse compute granularity by default.

## Channel model

| Channel | Reliability | Priority |
|---|---|---|
| Control/lease | Reliable ordered | High |
| Input/MIDI | Timestamped; selective reliability | Highest interactive |
| Audio | Deadline datagrams | RT |
| Video | Deadline datagrams + reliable metadata | RT |
| Clipboard metadata | Reliable | Normal |
| Object/file | Reliable resumable | Bulk |
| Telemetry | Sampled/best effort | Low |

## Congestion and queues

Each path uses explicit pacing and bounded queues. Late media is dropped rather than accumulating latency. Bulk transfer consumes residual budget.

## Clock quality

Endpoints report:

- monotonic clock source;
- offset uncertainty;
- drift estimate;
- synchronization method;
- last update;
- audio device clock relationships.

Clock mapping supports correlation but does not make independent audio hardware synchronous.

## Relay security

Relay learns connection metadata but cannot decrypt payload. Relay credentials do not grant endpoint capabilities. Route explanations show when relay is active.

## Path changes

Wi-Fi/Ethernet/VPN changes trigger a new path candidate. A live migration is prepared and committed only when expected benefit exceeds transition risk.
