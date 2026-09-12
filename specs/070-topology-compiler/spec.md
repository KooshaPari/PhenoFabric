# Spec 070 — Multi-Hop Topology Graph Compiler (PF-WP-070, R3)

**Status:** draft
**WP:** PF-WP-070
**Release gate:** R3
**Depends on:** specs/004 (locality route compiler), specs/015 (route compiler CLI)
**Enables:** real transport stage selection, multi-machine routing

## 1. Background

The R1/R2 route compiler (`fabric-graph/src/compile.rs`) is single-hop: it picks one destination node and builds a route plan with one step. This works for same-machine placement but cannot express:

- source → relay → destination (multi-hop routes);
- format transcoding stages (HEVC encode → transport → decode);
- locality-tier progression (L0 shared-memory → L4 LAN → L8 WAN);
- fallback route chains.

R3 extends the compiler to generate multi-hop route plans with real transport stages.

## 2. Scope

**In:**
- Multi-hop route plan generation (source → stages → destination)
- Transport stage catalog (encode, decode, transport, transform)
- Locality-aware stage selection (prefer L0 before L4 before L8)
- Fallback route generation
- Route plan validation (cycle detection, resource feasibility)
- Cost model for multi-hop routes

**Out:**
- Runtime route execution (R3.1)
- Dynamic route modification (R3.1)
- Route mirroring/splitting (R4)

## 3. Transport Stage Catalog

```rust
pub struct TransportStage {
    pub id: StageId,
    pub name: String,
    pub input_format: MediaFormat,
    pub output_format: MediaFormat,
    pub locality_tier: LocalityTier,
    pub cost: StageCost,
    pub claims: StageClaims,
    pub adapter: String, // e.g. "hevc_encoder", "quic_transport", "pipeWire"
}

pub struct StageCost {
    pub setup_us: u64,
    pub per_frame_us: f64,
    pub memory_bytes: u64,
    pub bandwidth_mbps: f64,
}

pub struct StageClaims {
    pub codec: Option<String>,
    pub gpu_encoder: bool,
    pub gpu_decoder: bool,
    pub rt_capable: bool,
}
```

### Built-in stages

| Stage | Input | Output | Tier | RT? |
|-------|-------|--------|------|-----|
| `identity` | any | same | L0 | yes |
| `shm_copy` | any | any | L0 | yes |
| `dma_buf_export` | GPU tex | DMA handle | L0 | yes |
| `kvmfr_import` | DMA handle | GPU tex | L0 | yes |
| `hevc_encode` | raw video | HEVC | L0-L4 | no |
| `hevc_decode` | HEVC | raw video | L0-L4 | no |
| `av1_encode` | raw video | AV1 | L0-L4 | no |
| `av1_decode` | AV1 | raw video | L0-L4 | no |
| `opus_encode` | PCM audio | Opus | L0-L8 | yes |
| `opus_decode` | Opus | PCM audio | L0-L8 | yes |
| `quic_transport` | encoded | encoded | L4-L8 | no |
| `tcp_transport` | encoded | encoded | L4-L8 | no |
| `unix_socket` | any | any | L0 | yes |
| `pipeWire_bridge` | audio | audio | L0 | yes |

## 4. Multi-Hop Route Generation

```rust
/// Generate multi-hop route plans from source to destination.
pub fn compile_multihop(
    topology: &Topology,
    source: &NodeId,
    destination: &NodeId,
    intent: &Intent,
    catalog: &[TransportStage],
) -> Vec<RoutePlan>;
```

### Algorithm

1. **Find path**: BFS/Dijkstra from source to destination through topology edges
2. **For each hop** (source → node₁ → node₂ → ... → destination):
   a. Determine format at this hop (from intent or source capability)
   b. Select transport stages that bridge the locality tier gap
   c. Score each stage combination
3. **Build route plan**: sequence of (node, stages) tuples
4. **Validate**: no cycles, all stages feasible, resource claims satisfiable
5. **Score**: total cost across all hops + stages
6. **Generate fallback**: alternative paths with higher cost

### Stage selection

For a hop from node A (tier L0) to node B (tier L4):
- If same format: no transcoding needed, just transport
- If different format: select encode stage at source, decode stage at destination
- If GPU → CPU: select texture download stage
- Always prefer stages at the lowest locality tier that satisfies the constraint

## 5. Cost Model

```rust
pub fn route_cost(plan: &RoutePlan, topology: &Topology) -> RouteCost {
    let mut cost = RouteCost::default();
    for step in &plan.steps {
        cost.setup_us += step.stages.iter().map(|s| s.cost.setup_us).sum::<u64>();
        cost.per_frame_us += step.stages.iter().map(|s| s.cost.per_frame_us).sum::<f64>();
        cost.memory_bytes += step.stages.iter().map(|s| s.cost.memory_bytes).sum::<u64>();
        // Add inter-hop transport cost from topology edge
        if let Some(edge) = topology.find_edge(&step.from_node, &step.node) {
            cost.latency_us += edge.link_metrics.rtt_us.unwrap_or(0);
            cost.bandwidth_mbps = cost.bandwidth_mbps.min(
                edge.link_metrics.bandwidth_mbps.unwrap_or(f64::MAX)
            );
        }
    }
    cost
}
```

## 6. Fallback Generation

For each primary route plan, generate up to 2 fallback plans:
1. **Alternative path**: different topology path, same stages
2. **Degraded path**: same path, lower-quality stages (e.g., lossy codec)

Fallbacks are stored in the route plan as `fallback_plans: Vec<RoutePlan>`.

## 7. Validation

```rust
impl RoutePlan {
    pub fn validate_multihop(&self, topology: &Topology) -> Result<(), RouteError> {
        // 1. No cycles (node appears at most once)
        // 2. All nodes exist in topology
        // 3. All edges exist between consecutive nodes
        // 4. All stages are feasible at their nodes
        // 5. Resource claims are satisfiable
        // 6. Total cost is within budget (if specified)
        // 7. No RT stage exceeds deadline
    }
}
```

## 8. Explainability

```text
Route: main-pc → bench-pc-2
  Hop 1: main-pc → router (LAN, L4)
    Stages: av1_encode (GPU encoder, 2ms/frame)
            quic_transport (LAN, 0.5ms)
  Hop 2: router → bench-pc-2 (LAN, L4)
    Stages: av1_decode (GPU decoder, 1ms)
  Total: 3.5ms/frame, 50MB GPU memory
  Fallback: main-pc local only (L0, 0ms, no transport)
```

## 9. Dependencies

- `fabric-graph` (topology, intent, route plan types)
- `fabric-capability` (locality tiers, media formats)
- `petgraph` (graph algorithms for pathfinding)

## 10. File Locations

| File | Purpose |
|------|---------|
| `crates/fabric-graph/src/compiler/multihop.rs` | Multi-hop compilation |
| `crates/fabric-graph/src/compiler/stages.rs` | Transport stage catalog |
| `crates/fabric-graph/src/compiler/cost.rs` | Cost model |
| `crates/fabric-graph/src/compiler/fallback.rs` | Fallback generation |
| `crates/fabric-graph/src/compiler/validate.rs` | Route validation |
| `crates/fabric-graph/tests/multihop_compiler.rs` | Integration tests |