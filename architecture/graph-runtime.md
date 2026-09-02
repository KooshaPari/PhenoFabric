# Graph Runtime

## Core abstraction

```text
Node → typed Port → desired Link → compiled Route → measured Evidence
```

A graph edit is declarative. The runtime may implement a link with one direct stage or a chain.

## Port contract

```yaml
port:
  id: pf://port/...
  direction: in|out|duplex
  type: video|audio|midi|hid|clipboard|file|storage|object|compute|telemetry
  formats: [...]
  timing:
    class: RT0|RT1|RT2|RT3|RT4|Bulk|Background
    deadline_us: optional
    period_us: optional
    max_jitter_us: optional
    clock_domain: optional
  memory_domains: [cpu, gpu, dma_buf, kvmfr, shared, device]
  security_labels: [...]
  locality: L0-L8
  capacity: {...}
```

## Desired link

A link states what should be connected, not how.

```yaml
link:
  source: pf://surface/win-vm/ableton/main-window/video
  sink: pf://device/macbook/panel/video
  policy:
    latency: realtime
    hdr: preserve
    chroma: 444_preferred
    frame_rate: 120_preferred
    fallback: [whole_desktop, local_only]
```

## Compiled route

```yaml
route:
  link_id: ...
  topology_epoch: 1842
  stages:
    - windows_graphics_capture
    - p010_to_hevc_main10
    - quic_datagram_transport
    - videotoolbox_decode
    - metal_edr_present
  claims:
    copies: 2
    memory_transitions: [d3d_texture_to_encoder, decoded_surface_to_metal]
    codec: HEVC Main10
    hdr: preserved_with_sink_tone_map
  reservations: [...]
  fallback_routes: [...]
```

## Graph transaction semantics

- validate all changed links together;
- prepare resources without changing active output;
- enforce exclusive-resource leases;
- commit on a frame/cycle/timestamp boundary;
- release prior route after new route becomes ready;
- abort cleans prepared resources;
- graph patch and route plan are independently versioned.

## RT schedule

The graph compiler produces an immutable local schedule:

```text
cycle N:
  wait/input event
  run ready nodes by dependency
  signal buffers/fences
  publish counters
  swap schedule only at boundary
```

Control changes are built off-thread. No heap allocation or blocking control RPC occurs on RT processing threads.

## Loop and recursion defense

Each surface/route carries:

- origin realm/surface;
- route ID;
- parent route;
- hop count;
- capture-exclusion token;
- content fingerprint hints.

A proposed route is rejected if it returns a surface to an ancestor capture scope unless an explicit bounded mirror policy allows it.

## Graph simplification

The compiler applies:

1. identity-transform removal;
2. adjacent format-transform fusion;
3. shared-clock resampler elimination;
4. copy-stage elimination where ownership/synchronization allows;
5. local bypass of network/codec stages;
6. static-source frame suppression;
7. common subroute sharing where isolation permits.

## Explainability

The UI can display:

```text
Selected: KVMFR → DMA-BUF import → compositor
Rejected:
- HEVC loopback: +encode/decode, GPU encoder reserved for game
- CPU SHM upload: +one CPU copy, higher RAM bandwidth
- physical HDMI capture: recovery-only policy
```
