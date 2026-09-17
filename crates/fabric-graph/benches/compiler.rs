//! Criterion benchmarks for fabric-graph route compiler, negotiation, and failover.

mod helpers;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fabric_graph::compile;
use fabric_graph::failover::replan;
use fabric_graph::model::NodeId;
use fabric_graph::multihop::{builtin_stages, compile_multihop};
use fabric_graph::negotiation::negotiate;

// ---------------------------------------------------------------------------
// Original benchmarks (compile at various scales)
// ---------------------------------------------------------------------------

/// Benchmark route compilation on a small (10-node) mesh topology.
fn bench_compile_small_graph(c: &mut Criterion) {
    let topo = helpers::build_mesh_topology(10);
    let intent = helpers::any_node_intent();

    c.bench_function("compile_small_10nodes", |b| {
        b.iter(|| {
            let plan = compile(black_box(&topo), black_box(&intent)).unwrap();
            black_box(&plan);
        });
    });
}

/// Benchmark route compilation on a medium (100-node) mesh topology.
fn bench_compile_medium_graph(c: &mut Criterion) {
    let topo = helpers::build_mesh_topology(100);
    let intent = helpers::any_node_intent();

    c.bench_function("compile_medium_100nodes", |b| {
        b.iter(|| {
            let plan = compile(black_box(&topo), black_box(&intent)).unwrap();
            black_box(&plan);
        });
    });
}

/// Benchmark route compilation on a large (1000-node) mesh topology.
fn bench_compile_large_graph(c: &mut Criterion) {
    let topo = helpers::build_mesh_topology(1000);
    let intent = helpers::any_node_intent();

    c.bench_function("compile_large_1000nodes", |b| {
        b.iter(|| {
            let plan = compile(black_box(&topo), black_box(&intent)).unwrap();
            black_box(&plan);
        });
    });
}

/// Benchmark negotiating a single intent against a topology.
fn bench_negotiate_single_intent(c: &mut Criterion) {
    let topo = helpers::build_mesh_topology(50);
    let intent = helpers::any_node_intent();

    c.bench_function("negotiate_single_intent_50nodes", |b| {
        b.iter(|| {
            let result = negotiate(black_box(&topo), black_box(&intent));
            black_box(&result);
        });
    });
}

/// Benchmark multihop compile on a 4-node chain.
fn bench_multihop_compile_4node(c: &mut Criterion) {
    let topo = helpers::build_4node_chain();
    let intent = helpers::any_node_intent();
    let catalog = builtin_stages();

    c.bench_function("multihop_compile_4node_chain", |b| {
        b.iter(|| {
            let result = compile_multihop(
                black_box(&topo),
                &NodeId::new("node-0"),
                &NodeId::new("node-3"),
                black_box(&intent),
                &catalog,
            )
            .unwrap();
            black_box(&result);
        });
    });
}

/// Benchmark failover replan when a node fails.
fn bench_failover_replan(c: &mut Criterion) {
    let topo = helpers::build_mesh_topology(20);
    let intent = helpers::any_node_intent();
    let original_plan = compile(&topo, &intent).unwrap();

    // Blacklist node-5 (a mid-chain node)
    let blacklist = vec![NodeId::new("node-5")];

    c.bench_function("failover_replan_20nodes", |b| {
        b.iter(|| {
            let outcome = replan(
                black_box(&topo),
                black_box(&intent),
                black_box(&original_plan),
                black_box(&blacklist),
            )
            .unwrap();
            black_box(&outcome);
        });
    });
}

// ---------------------------------------------------------------------------
// New benchmarks: topology_compile, negotiate, scoring, failover, fairness
// ---------------------------------------------------------------------------

/// Compile a route plan from a 100-node topology with 20 intents.
///
/// Measures the throughput of compiling multiple intents against a
/// capability-rich topology, simulating a real startup scenario where
/// many surfaces need route plans simultaneously.
fn bench_topology_compile_100nodes_20intents(c: &mut Criterion) {
    let topo = helpers::build_capability_topology(100);
    let intents = helpers::build_intent_batch(20);

    c.bench_function("topology_compile_100nodes_20intents", |b| {
        b.iter(|| {
            for intent in black_box(&intents) {
                let plan = compile(&topo, intent).unwrap();
                black_box(&plan);
            }
        });
    });
}

/// Score 50 candidate routes against an intent.
///
/// Uses a 100-node topology where most nodes pass hard filters,
/// so negotiate() scores ~50+ candidates.
fn bench_negotiate_50_candidates(c: &mut Criterion) {
    let topo = helpers::build_capability_topology(100);
    // Use a broad intent that passes most nodes' hard filters
    let intent = helpers::any_node_intent();

    c.bench_function("negotiate_50_candidates", |b| {
        b.iter(|| {
            let result = negotiate(black_box(&topo), black_box(&intent));
            black_box(&result.candidates.len());
        });
    });
}

/// Measure locality scoring function throughput across all nodes.
///
/// Iterates score_locality() on every node in a flat 200-node topology
/// to measure the raw throughput of the scoring hot path.
fn bench_scoring_locality(c: &mut Criterion) {
    let topo = helpers::build_flat_topology(200, fabric_graph::LocalityTier::L6Lan);
    let intent = helpers::any_node_intent();

    c.bench_function("scoring_locality_200nodes", |b| {
        b.iter(|| {
            helpers::bench_score_locality_throughput(black_box(&topo), black_box(&intent));
        });
    });
}

/// Re-plan around 5 failed nodes in a 100-node graph.
///
/// Measures failover replan latency with a realistic failure scenario
/// where 5 nodes out of 100 become unavailable simultaneously.
fn bench_failover_replan_100nodes_5failed(c: &mut Criterion) {
    let topo = helpers::build_capability_topology(100);
    let intent = helpers::any_node_intent();
    let original_plan = compile(&topo, &intent).unwrap();

    // 5 failed nodes spread across the topology
    let blacklist = vec![
        NodeId::new("node-5"),
        NodeId::new("node-15"),
        NodeId::new("node-35"),
        NodeId::new("node-65"),
        NodeId::new("node-85"),
    ];

    c.bench_function("failover_replan_100nodes_5failed", |b| {
        b.iter(|| {
            let outcome = replan(
                black_box(&topo),
                black_box(&intent),
                black_box(&original_plan),
                black_box(&blacklist),
            )
            .unwrap();
            black_box(&outcome);
        });
    });
}

/// Enqueue and dequeue 1000 fairness decisions.
///
/// Measures the throughput of the FairnessQueue with 20 tenants
/// each making 50 acquire/release cycles (1000 total decisions).
fn bench_fairness_queue_1000decisions(c: &mut Criterion) {
    c.bench_function("fairness_queue_1000decisions", |b| {
        b.iter_with_setup(
            || {
                // Fresh queue with 20 tenants
                helpers::build_fairness_queue(20, 0)
            },
            |mut queue| {
                for i in 0..1000 {
                    let tenant = fabric_graph::TenantId::new(format!("tenant-{}", i % 20));
                    let decision = queue.try_acquire(black_box(tenant.clone()), 1);
                    black_box(&decision);
                    queue.release(tenant, 1);
                }
            },
        );
    });
}

// ---------------------------------------------------------------------------
// Register all benchmarks
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_compile_small_graph,
    bench_compile_medium_graph,
    bench_compile_large_graph,
    bench_negotiate_single_intent,
    bench_multihop_compile_4node,
    bench_failover_replan,
    bench_topology_compile_100nodes_20intents,
    bench_negotiate_50_candidates,
    bench_scoring_locality,
    bench_failover_replan_100nodes_5failed,
    bench_fairness_queue_1000decisions,
);
criterion_main!(benches);
