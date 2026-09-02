# Route Compiler

## Objective

Choose the lowest total-cost route that satisfies hard security, timing, quality, compatibility, and authority constraints.

## Pipeline

```mermaid
flowchart LR
    I[Link / placement intent] --> C[Capability match]
    C --> L[Locality expansion]
    L --> P[Hard prune]
    P --> G[Candidate graph generation]
    G --> S[Cost + risk score]
    S --> A[Admission]
    A --> R[Prepare / commit]
    R --> M[Measure / learn]
    M --> S
```

## Candidate generation

The compiler maintains stage catalogs with:

- input/output type and format;
- memory-domain support;
- locality;
- fixed and variable cost;
- resource claims;
- timing behavior;
- security properties;
- failure modes;
- adapter/version evidence.

Candidate generation is bounded by locality tier, maximum stage count, policy, and known-good templates. It is not unrestricted path search over every possible conversion.

## Hard constraints

Examples:

- source/sink authorization;
- no WAN for confidential surface;
- HDR must not become SDR;
- desktop profile requires 4:4:4;
- RT0 route deadline ≤ 2.67 ms;
- source and sink must share a root complex for a P2P candidate;
- no driver install on managed Mac;
- no relay;
- fallback must exist;
- object authority may not move.

Candidates violating a hard constraint are not assigned a large cost; they are removed.

## Cost terms

```text
fixed setup
+ queue delay
+ per-frame/per-buffer processing
+ memory-domain transitions
+ bytes moved × path cost
+ synchronization/barrier cost
+ contention impact on protected resources
+ expected loss/recovery cost
+ failure/uncertainty margin
+ energy/thermal policy
```

Quality is either hard-constrained or represented as loss penalty. The compiler never hides a quality loss inside “performance.”

## Hysteresis

Replanning requires:

- material improvement over current route;
- confidence above threshold;
- minimum dwell time;
- no in-progress protected transaction;
- prepared fallback.

This prevents oscillation when GPU/load/network metrics fluctuate.

## Adaptive learning

The system records predicted versus actual:

- capture/transform/encode/decode times;
- queue delay;
- copy bytes;
- network delivery;
- frame/audio deadline results;
- CPU/GPU/engine pressure.

Models begin as empirical tables and exponentially weighted estimates. Complex ML is unnecessary until the benchmark corpus proves value.

## Manual control

Users/policies may:

- pin a transport or resource;
- forbid a stage/vendor;
- require/forbid relay;
- fix quality profile;
- cap bandwidth/power;
- retain route until failure;
- request an experiment comparing candidates.

Manual pinning appears in explanations and is not silently overridden.
