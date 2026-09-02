# 0025 — Route and Lease Semantic Model

## Status

Accepted — 2026-09-02.

## Date

2026-09-02.

## Deciders

GLM-backed Codex chat continuation session (2026-09-01 → 2026-09-02).

## Supersedes

None.

## Traceability

- ADR-0003 (typed graph)
- ADR-0004 (locality-first route selection)
- ADR-0006 (projections of shared identity)
- ADR-0012 (leases and fencing tokens)
- ADR-0022 (control/data plane separation)
- spec `015-route-compiler-cli`

## Context

The Fabric route compiler must satisfy four independent concerns at once:
**typed graph traversal**, **locality-aware scoring**, **lease-based exclusivity**,
and **plan validation**. A premature decision here locks in the entire R1
surface area, so the model has to be explicit and minimal.

PF-WP-020 (`crates/fabric-graph`) is the first concrete implementation. It
exposes `negotiate`, `compile`, `plan_sequence`, and `plan_parallel`. The
question is: what is the **minimum semantic model** that captures the four
concerns, and what gets deferred to R1?

## Decision

### 1. Three-stage pipeline

The route pipeline is **filter → score → compile**:

1. **Filter** (hard requirements): A candidate node is **rejected** if
   any hard requirement is unmet. Hard requirements include:
   - `min_trust`: `node.trust().satisfies(intent.min_trust)` must be true.
   - `requires_rt`: If the intent requires real-time, the node must
     belong to a known RT island.
   - `latency_budget_us`: The minimum of `link.effective_bw(max_latency)`
     across all reachable links must be `Some`.
   - `cpu_cores` / `min_ram` / `tags` (matched against `IntentRequirements::matches_node`).

2. **Score** (soft ranking): Each surviving candidate is given a
   `Score { total, breakdown, reason, rank }`. The default weights are
   `locality=0.40`, `latency=0.25`, `bandwidth=0.20`, `trust=0.10`,
   `rt_bonus=0.05`. A preferred node (if specified) gets `+3.0` to its
   locality component. RT islands get full RT bonus only if `requires_rt`
   is true.

3. **Compile** (plan validation): The highest-scored candidate is
   turned into a `RoutePlan` containing the selected node, the entry
   link, the local node's capabilities, and a `Score` record. The plan
   is then validated by `validate_compiled_plan`, which re-checks the
   hard requirements from the original intent.

### 2. Locality tiers are not absolute

ADR-0004 says "search L0..L8 in order of physical cost." PF-WP-020
implements this as a **scoring input**, not a hard sequence. The
`LocalityTier` (`L0SameProcess` … `L8OutOfBand`) is converted to a
`f64` via `as_f64()` (L0=0.0, L8=8.0), then scaled by the locality
weight. An L4 candidate can out-score an L2 candidate if its link
quality is much better. This matches the "tier order is a heuristic,
not an absolute rule" wording in ADR-0004.

### 3. Leases are separate from plans

ADR-0012 says "exclusive resources use one authoritative lease service
and monotonically increasing fencing tokens." PF-WP-020 does **not**
implement a lease service. `fabric-graph` produces `RoutePlan`s; the
lease acquisition step is the caller's responsibility (and belongs to
PF-WP-030 / R1). The `SeatLease` / `RouteLease` types are **deferred**
to R1.

This separation is intentional: route plans should be reproducible
(pure functions of `topology × intent × epoch`), while leases are
mutable state with fencing tokens. Mixing them forces a lease
acquisition into every compile call and breaks `cargo test` determinism.

### 4. Topology is a snapshot, not a stream

The `Topology` type is a snapshot of the capability graph at a
specific `TopologyEpoch`. `negotiate` and `compile` are **pure**:
they take a `&Topology` and return a `RoutePlan` (or a
`NegotiationResult` with no candidates). The epoch is recorded in
the plan so downstream consumers can detect "this plan was compiled
against a stale topology."

R1 will introduce an `Authority` that tracks topology deltas and
notifies subscribers. Until then, the caller is responsible for
refreshing the topology before recompiling.

### 5. Planner semantics

`plan_sequence(intents: &[Intent])` produces a list of `RoutePlan`s,
one per intent, in order, assuming sequential execution. `plan_parallel`
produces a `RoutePlan` for every intent under the assumption that all
are running simultaneously. Both are **scheduling hints**: they do
not reserve resources. A real scheduler (PF-WP-040) will read the
plans and either accept, reject, or renegotiate them.

### 6. Score breakdown is part of the plan

A `RoutePlan` includes the `Score { total, breakdown, reason, rank }`
of its chosen candidate. This is for **evidence** (per the Fabric
NFR: every decision must be auditable). A user can ask "why was this
node chosen?" and the answer is in the `reason` field. The `rank` is
the position of this candidate in the full scored list (rank 0 is
the best). This is **not** a global ranking — it is the rank within
the candidates that survived the hard filter.

## Alternatives considered

### A. Single-stage compile (filter + score in one pass)

A single `compile(&topology, &intent) -> Result<RoutePlan>` that does
hard filtering and soft scoring in one traversal. **Rejected** because:
- The two passes have different error semantics (filter errors are
  "no candidate" and should be silent; score is a `Result` of `f64`).
- The test suite needs to assert on `Score { breakdown }`, which
  requires the soft pass to be observable.
- Hard filters are stable across recompiles within an epoch; soft
  scores can vary. Mixing them obscures the source of non-determinism.

### B. Leases in `fabric-graph`

Implementing lease acquisition in `RoutePlan` so that `compile`
returns a `Result<(RoutePlan, LeaseToken)>`. **Rejected** because:
- Breaks test determinism (every test would have to mint a lease).
- Forces a global lease service into the R0 crate, which is a
  stateful, network-aware component. R0 should be pure.
- The "filter → score → lease" pipeline is cleaner and matches the
  real-world contract: "tell me if it's possible" vs. "do it."

### C. Locality tier as a hard sequence

Compile iterates L0 → L8 and returns the first tier that has a
matching node. **Rejected** because it bakes the heuristic into the
algorithm. The current `Score`-based approach lets future weights
adjust without touching compile logic. It also matches the
"heuristic, not rule" wording in ADR-0004.

### D. Topology as a stream

`Topology` is a live subscription; `negotiate` queries the stream.
**Rejected** because R0 is local-only and tests need deterministic
inputs. The `TopologyEpoch` is a marker for "this snapshot is stale"
without the cost of a stream API.

## Consequences

### Positive

- `fabric-graph` is **pure** and **deterministic**: same input → same
  output. This makes the 43 tests trivially reproducible.
- The filter / score / lease pipeline matches the R1 architecture
  without forcing a lease service into R0.
- `Score { breakdown, reason, rank }` is observable, which satisfies
  the Fabric NFR on auditable decisions.
- `LocalityTier` is data (`u8` 0..8), not behavior. This means the
  scoring function is a pure function of `(tier, link_metrics)`.

### Negative

- The `SeatLease` / `RouteLease` types are deferred, so PF-WP-030
  cannot start until R1 begins. This is a **one-cycle delay** for
  the lease service.
- `plan_sequence` / `plan_parallel` are hints, not schedules. A
  caller that treats them as a schedule will be surprised when
  PF-WP-040 introduces the real scheduler.
- `TopologyEpoch` is a marker, not a stream. Callers that need
  push-based updates must poll the topology. R1 will fix this with
  the `Authority` API.

### Risks

- The default score weights (`locality=0.40`, etc.) are **untested
  on real workloads**. R0.5 must include a benchmark of these
  weights against the reference environment (multi-GPU Linux + bench
  PCs + MacBook).
- The hard filter rejects candidates that *would* work with a
  relaxed trust model. If a future intent needs "trusted OR
  self-attested," `IntentRequirements` will need a `trust_or` field.
- `plan_sequence` assumes intents are **independent** except for
  the "previously selected nodes are unavailable" constraint. This
  is wrong for the data graph (where the next step depends on the
  previous step's output). R1 will introduce a DAG-aware planner.

## References

- spec `015-route-compiler-cli` — concrete feature spec
- work `tasks.json` — PF-WP-020 (12 atomic tasks)
- `crates/fabric-graph/src/{model,score,negotiation,compile,planner}.rs` — implementation
- ADR-0003, ADR-0004, ADR-0006, ADR-0012, ADR-0022 — upstream constraints
