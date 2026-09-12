# ADR-0032: Multi-Hop Route Compiler

## Status

Proposed

## Date

2026-09-12

## Context

R3 requires multi-hop route plans that span multiple topology hops with
transport stage selection at each hop. The existing `compile()` function
in `fabric-graph` produces single-hop plans — it finds the best direct
path from source to destination. Multi-hop compilation needs:

1. Path finding through intermediate topology nodes (BFS shortest path)
2. Transport stage selection per hop based on locality tier gap
3. Cost computation across all hops and stages
4. Validation (cycle detection, epoch matching, node existence)
5. Fallback route generation for failover resilience

## Decision

Introduce `fabric_graph::multihop` as a submodule of `fabric-graph`
with five components:

1. **`stages.rs`** — Transport stage catalog. 9 built-in stages:
   identity, shm_copy, hevc/av1 encode/decode, quic/tcp/unix_socket.
   Each stage declares `StageCost` (setup_us, per_frame_us, memory_bytes,
   bandwidth_mbps), `StageClaims` (codec, gpu_encoder/decoder, rt_capable),
   and `MediaFormat` (RawVideo, Hevc, Av1, PcmAudio, OpusAudio, DmaBuffer, Any).

2. **`cost.rs`** — `RouteCost` composite scoring from topology edge metrics.
   `compute_route_cost()` sums per-hop edge costs + stage costs. Budget
   constraint checking via `meets_budget()`.

3. **`validate.rs`** — `validate_multihop()` checks cycle detection (visited
   set), node existence in topology, edge existence between consecutive
   hops, and topology epoch matching.

4. **`fallback.rs`** — `generate_fallbacks()` produces alternative routes
   by trying alternative edges at each hop and degraded paths (skipping
   intermediate hops where single-hop transport is viable).

5. **`mod.rs`** — `compile_multihop()` orchestrates: BFS pathfinding →
   per-hop stage selection (stages matching the locality tier gap between
   hops) → route plan construction → validation → cost computation →
   fallback generation.

## Alternatives considered

### A. Extend existing `compile()` with multi-hop support
Rejected: the existing `compile()` is a single-purpose function tightly
coupled to `IntentRequirements` hard-filter + soft-scoring. Adding multi-hop
logic would bloat the function beyond 500 lines and break the single-responsibility
principle.

### B. Separate `fabric-multihop` crate
Rejected: the multi-hop compiler operates entirely within `fabric-graph`
types (`Topology`, `NodeId`, `RoutePlan`, `RouteStep`). A separate crate
would create a circular dependency or require duplicating model types.

### C. Graph-theory library (petgraph) for pathfinding
Rejected: BFS on `Topology.nodes` + `Topology.edges` is 20 lines.
Adding petgraph as a dependency for one BFS is overkill. Custom BFS
is trivially correct and avoids dependency version conflicts.

## Consequences

- `fabric_graph::multihop` is the canonical multi-hop compilation path.
- `compile()` (single-hop) remains unchanged — callers who don't need
  multi-hop behavior are unaffected.
- Transport stage catalog is extensible via `Vec<TransportStage>` parameter.
- Fallback generation is opt-in — callers who don't need alternatives
  can ignore the `fallbacks` field.
- 24 unit tests across all submodules. All pass.

## References

- Spec 024 (surface-plane runtime) — uses `compile_multihop` for
  multi-hop route plans in the surface registry.
- ADR-0004 (locality-first route selection) — stage selection follows
  the locality tier ordering.
- ADR-0009 (shared memory before codecs) — shm_copy and identity stages
  are preferred for same-process and same-host paths.
- ADR-0030 (route failover model) — fallback routes feed into the
  failover replan path.
