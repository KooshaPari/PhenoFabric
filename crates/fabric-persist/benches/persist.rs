//! Criterion benchmarks for fabric-persist SQLite operations.

use std::sync::Arc;
use std::thread;

use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fabric_graph::model::{
    CapabilityRef, Edge, EdgeId, IntentId, LinkMetrics, Node, NodeId, RoutePlan, RoutePlanId,
    RouteStep, Topology, TopologyEpoch, TopologyMeta, TrustLevel,
};
use fabric_graph::surface::{
    CaptureDirection, LeaseState, SurfaceHandle, SurfaceLease, SurfaceProtocol, SurfaceSpec,
};
use fabric_graph::LocalityTier;
use fabric_persist::Persist;

// ---------------------------------------------------------------------------
// Existing benchmarks: topology persistence basics
// ---------------------------------------------------------------------------

/// Build a topology with 50 nodes and 49 chain edges for persistence benchmarks.
fn bench_topology_50(n: usize) -> Topology {
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
        compiled_at: Utc::now(),
        expires_at: Utc::now() + chrono::Duration::hours(1),
        tags: vec!["bench".into()],
    }
}

/// Benchmark saving a topology to SQLite.
fn bench_save_topology(c: &mut Criterion) {
    let topo = bench_topology_50(50);

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
    let topo = bench_topology_50(50);

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
    let topo = bench_topology_50(50);
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

// ---------------------------------------------------------------------------
// New benchmarks: 100-node save/recover, save_lease, concurrent_reads
// ---------------------------------------------------------------------------

/// Build a topology with `n` nodes, each with capabilities.
fn bench_topology_100(n: usize) -> Topology {
    let mut topo = Topology::new();
    topo.meta = TopologyMeta {
        name: format!("bench-persist-{n}"),
        ..Default::default()
    };

    for i in 0..n {
        let tier = match i % 4 {
            0 => LocalityTier::L0SameProcess,
            1 => LocalityTier::L1SameNuma,
            2 => LocalityTier::L3PcieP2P,
            _ => LocalityTier::L6Lan,
        };
        let cap_id = format!("sha256:cap-{i}");
        let node = Node::new(NodeId::new(format!("node-{i}")), tier)
            .with_capability(CapabilityRef::new(cap_id).with_trust(TrustLevel::Attested));
        topo.add_node(node);
    }

    for i in 0..n.saturating_sub(1) {
        let edge = Edge::new(
            EdgeId::new(format!("e-{i}-{}", i + 1)),
            NodeId::new(format!("node-{i}")),
            NodeId::new(format!("node-{}", i + 1)),
            LocalityTier::L6Lan,
        )
        .with_metrics(LinkMetrics {
            latency_us: Some(50.0 + i as f64),
            bandwidth_bps: Some(1_000_000_000),
            packet_loss: Some(0.001),
            jitter_us: Some(2.0),
        });
        topo.add_edge(edge).unwrap();
    }

    topo
}

/// Benchmark saving a 100-node topology to SQLite.
fn bench_save_topology_100nodes(c: &mut Criterion) {
    let topo = bench_topology_100(100);

    c.bench_function("save_topology_100nodes", |b| {
        b.iter_with_setup(
            || Persist::open_memory().unwrap(),
            |persist| {
                persist.save_topology(black_box(&topo)).unwrap();
            },
        );
    });
}

/// Benchmark recovering full state from a 100-node topology with leases and plans.
fn bench_recover_state_100nodes(c: &mut Criterion) {
    let topo = bench_topology_100(100);
    let plan = bench_plan();

    c.bench_function("recover_state_100nodes", |b| {
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

/// Create a sample SurfaceLease for persistence benchmarks.
fn bench_lease() -> SurfaceLease {
    SurfaceLease {
        handle: SurfaceHandle::new(),
        spec: SurfaceSpec {
            name: "bench-display".into(),
            protocol: SurfaceProtocol::WebRtc,
            capture: Some(CaptureDirection::Sink),
            locality_floor: LocalityTier::L3PcieP2P,
            refresh_hz: Some(60),
            audio_sample_rate_hz: Some(48000),
            requires_rt_island: false,
            strict_epoch_binding: true,
            min_host_trust: TrustLevel::Bootstrap,
            expires_at: None,
        },
        current: None,
        history: vec![],
        state: LeaseState::Pending,
        exit_reason: None,
        created_at: Utc::now(),
        terminated_at: None,
    }
}

/// Benchmark saving a single lease to SQLite.
fn bench_save_lease(c: &mut Criterion) {
    c.bench_function("save_lease", |b| {
        b.iter_with_setup(
            || Persist::open_memory().unwrap(),
            |persist| {
                let lease = bench_lease();
                persist.save_lease(black_box(&lease)).unwrap();
            },
        );
    });
}

/// Benchmark 10 concurrent reads while writing.
///
/// Simulates the daemon's production workload: multiple reader threads
/// polling topology/leases while a writer thread persists updates.
fn bench_concurrent_reads_10(c: &mut Criterion) {
    c.bench_function("concurrent_reads_10_while_writing", |b| {
        b.iter_with_setup(
            || {
                let topo = bench_topology_100(100);
                let persist = Persist::open_memory().unwrap();
                persist.save_topology(&topo).unwrap();
                Arc::new(persist)
            },
            |persist| {
                let handles: Vec<_> = (0..10)
                    .map(|i| {
                        let p = Arc::clone(&persist);
                        thread::spawn(move || {
                            // Each reader loads topology and recovers state
                            let topo = p.load_topology().unwrap();
                            black_box(&topo);
                            // Alternate between load_topology and recover_state
                            if i % 2 == 0 {
                                let state = p.recover_state().unwrap();
                                black_box(&state);
                            }
                        })
                    })
                    .collect();

                // Meanwhile, the main thread writes a new plan
                let plan = bench_plan();
                persist.save_route_plan(black_box(&plan)).unwrap();

                for h in handles {
                    h.join().unwrap();
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
    bench_save_topology,
    bench_load_topology,
    bench_recover_state,
    bench_save_topology_100nodes,
    bench_recover_state_100nodes,
    bench_save_lease,
    bench_concurrent_reads_10,
);
criterion_main!(benches);
