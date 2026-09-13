//! Criterion benchmarks for fabric-persist SQLite operations.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fabric_graph::model::{
    Edge, EdgeId, IntentId, LinkMetrics, Node, NodeId, RoutePlan, RoutePlanId, RouteStep,
    Topology, TopologyEpoch, TopologyMeta,
};
use fabric_graph::LocalityTier;
use fabric_persist::Persist;

/// Build a topology with 50 nodes and 49 chain edges for persistence benchmarks.
fn bench_topology(n: usize) -> Topology {
    let mut topo = Topology::new();
    topo.meta = TopologyMeta {
        name: "bench-persist-topo".into(),
        ..Default::default()
    };

    for i in 0..n {
        let tier = match i % 3 {
            0 => LocalityTier::L1SameNuma,
            1 => LocalityTier::L2CrossNumaShm,
            _ => LocalityTier::L6Lan,
        };
        topo.add_node(Node::new(NodeId::new(format!("node-{i}")), tier));
    }

    for i in 0..n.saturating_sub(1) {
        let edge = Edge::new(
            EdgeId::new(format!("e-{i}-{}", i + 1)),
            NodeId::new(format!("node-{i}")),
            NodeId::new(format!("node-{}", i + 1)),
            LocalityTier::L6Lan,
        )
        .with_metrics(LinkMetrics {
            latency_us: Some(100.0 + i as f64),
            bandwidth_bps: Some(1_000_000_000),
            packet_loss: Some(0.0),
            jitter_us: Some(5.0),
        });
        topo.add_edge(edge).unwrap();
    }

    topo
}

/// Create a sample route plan for persistence benchmarks.
fn bench_plan() -> RoutePlan {
    RoutePlan {
        id: RoutePlanId::new(),
        intent_id: IntentId::new(),
        topology_epoch: TopologyEpoch(1),
        steps: vec![
            RouteStep {
                node: NodeId::new("node-0"),
                capability_id: Some("sha256:cap0".into()),
                via_edge: None,
                action: "execute".into(),
            },
            RouteStep {
                node: NodeId::new("node-1"),
                capability_id: None,
                via_edge: Some(EdgeId::new("e-0-1")),
                action: "route".into(),
            },
        ],
        estimated_latency_us: Some(150.0),
        score: None,
        compiled_at: chrono::Utc::now(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        tags: vec!["bench".into()],
    }
}

/// Benchmark saving a topology to SQLite.
fn bench_save_topology(c: &mut Criterion) {
    let topo = bench_topology(50);

    c.bench_function("save_topology_50nodes", |b| {
        b.iter_with_setup(
            || Persist::open_memory().unwrap(),
            |persist| {
                persist.save_topology(black_box(&topo)).unwrap();
            },
        );
    });
}

/// Benchmark loading a topology from SQLite.
fn bench_load_topology(c: &mut Criterion) {
    let topo = bench_topology(50);

    c.bench_function("load_topology_50nodes", |b| {
        b.iter_with_setup(
            || {
                let persist = Persist::open_memory().unwrap();
                persist.save_topology(&topo).unwrap();
                persist
            },
            |persist| {
                let loaded = persist.load_topology().unwrap().unwrap();
                black_box(&loaded);
            },
        );
    });
}

/// Benchmark full state recovery from SQLite (topology + leases + plans).
fn bench_recover_state(c: &mut Criterion) {
    let topo = bench_topology(50);
    let plan = bench_plan();

    c.bench_function("recover_state", |b| {
        b.iter_with_setup(
            || {
                let persist = Persist::open_memory().unwrap();
                persist.save_topology(&topo).unwrap();
                persist.save_route_plan(&plan).unwrap();
                persist
            },
            |persist| {
                let state = persist.recover_state().unwrap();
                black_box(&state);
            },
        );
    });
}

criterion_group!(
    benches,
    bench_save_topology,
    bench_load_topology,
    bench_recover_state,
);
criterion_main!(benches);
