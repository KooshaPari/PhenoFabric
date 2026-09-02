# Scenario: Remote Worldwide PC

## Route compilation

1. Authenticate device and resolve direct overlay/NAT path.
2. Probe current RTT, jitter, loss, bandwidth and decoder/encoder pressure.
3. Select full desktop, semantic app or terminal/artifact surface based on intent.
4. Allocate separate input, audio, video, control and bulk queues.
5. Apply congestion pacing and bounded FEC/retransmission by stream semantics.
6. Keep authoritative local RT audio/game paths independent.
7. Replan on roaming or link changes without accepting stale focus leases.

## Compute placement

WAN is normally job/agent/service-granularity. Atomic calls are possible only as an experimental capability when state/result sizes and latency make them profitable. The optimizer prefers remote execution when data and accelerator are already remote and the returned result is small.

## Failure

A relay may preserve reachability but is visible in the route explanation and cost. If the software endpoint fails, the user may switch to OOB KVM where installed. No public port-forwarding of privileged OOB services is assumed.
