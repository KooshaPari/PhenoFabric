//! Criterion benchmarks for fabric-graph route compiler, negotiation, and failover.

mod helpers;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fabric_graph::compile;
use fabric_graph::multihop::{builtin_stages, compile_multihop};
use fabric_graph::negotiation::negotiate;
use fabric_graph::failover::{replan, FailoverOutcome};
use fabric_graph::model::{NodeId, RoutePlan, TopologyEpoch};

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

criterion_group!(
    benches,
    bench_compile_small_graph,
    bench_compile_medium_graph,
    bench_compile_large_graph,
    bench_negotiate_single_intent,
    bench_multihop_compile_4node,
    bench_failover_replan,
);
criterion_main!(benches);
