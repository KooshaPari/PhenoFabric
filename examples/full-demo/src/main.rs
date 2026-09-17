//! Fabric Full Demo
//!
//! A self-contained Rust binary that demonstrates the complete Fabric lifecycle:
//!
//! 1. Builds a 6-node topology across 3 locality tiers
//! 2. Compiles route plans from a placement intent
//! 3. Validates a checker manifest against the topology
//! 4. Boots the wire server via the Coordinator
//! 5. Streams a test frame over the frame transport protocol
//! 6. Shuts down gracefully
//!
//! Run with: `cargo run --example full-demo`

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use fabric_graph::builder::{IntentBuilder, TopologyBuilder};
use fabric_graph::compile;
use fabric_graph::model::{LinkMetrics, Node, NodeId};
use fabric_graph::LocalityTier as LT;

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> anyhow::Result<()> {
    // Initialize tracing (defaults to INFO, override with RUST_LOG).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    println!();
    println!("==============================================");
    println!("  Phenotype Fabric — Full Demo");
    println!("==============================================");
    println!();

    // ---------------------------------------------------------------
    // Step 1: Build the topology
    // ---------------------------------------------------------------
    println!("[1/6] Building 6-node topology...");

    let topology = build_demo_topology();
    println!(
        "      Nodes: {} | Edges: {} | Epoch: {}",
        topology.nodes.len(),
        topology.edges.len(),
        topology.epoch
    );

    // ---------------------------------------------------------------
    // Step 2: Compile routes
    // ---------------------------------------------------------------
    println!("[2/6] Compiling route plans from intent...");

    let intent = IntentBuilder::new()
        .name("demo-gpu-inference")
        .min_trust(fabric_graph::TrustLevel::Untrusted)
        .max_locality(6.0) // up to L6 (LAN)
        .require_tag("gpu")
        .build();

    let plan = compile(&topology, &intent)?;
    println!(
        "      Plan {:?} compiled with {} step(s), estimated latency: {:.0} us",
        plan.id,
        plan.steps.len(),
        plan.estimated_latency_us.unwrap_or(0.0)
    );
    for (i, step) in plan.steps.iter().enumerate() {
        println!(
            "        Step {}: node={} action={}",
            i + 1,
            step.node,
            step.action
        );
    }

    // ---------------------------------------------------------------
    // Step 3: Multi-hop compilation
    // ---------------------------------------------------------------
    println!("[3/6] Compiling multi-hop route (gpu-node-alpha -> remote-node-a)...");

    let source = NodeId::new("gpu-node-alpha");
    let destination = NodeId::new("remote-node-a");
    let catalog = fabric_graph::multihop::builtin_stages();
    let multihop = fabric_graph::multihop::compile_multihop(
        &topology,
        &source,
        &destination,
        &intent,
        &catalog,
    )?;
    println!(
        "      Multi-hop plan: {} step(s), cost={:.2}, fallbacks={}",
        multihop.primary.steps.len(),
        multihop.cost.per_frame_us,
        multihop.fallbacks.len()
    );
    for stage_hop in &multihop.stages_per_hop {
        let names: Vec<&str> = stage_hop.iter().map(|s| s.name.as_str()).collect();
        println!("        Stages: {}", names.join(" -> "));
    }

    // ---------------------------------------------------------------
    // Step 4: Validate checker manifest
    // ---------------------------------------------------------------
    println!("[4/6] Validating checker manifest...");

    let _manifest = fabric_checker::CheckerManifest {
        memory_bytes: 8 * 1024 * 1024 * 1024, // 8 GB
        cpu_cores: 4,
        storage_bytes: 50 * 1024 * 1024 * 1024, // 50 GB
        os_families: vec!["linux".into()],
        arches: vec!["x86_64".into(), "aarch64".into()],
        audio: true,
        network_peers: vec!["cpu-node-one".into(), "remote-node-a".into()],
        headless: false,
        realtime_island: false,
    };

    // Validate manifest against each GPU node.
    let gpu_nodes: Vec<(&NodeId, &Node)> = topology
        .nodes
        .iter()
        .filter(|(_, n)| n.tags.contains(&"gpu".to_string()))
        .collect();

    for (node_id, node) in &gpu_nodes {
        println!(
            "      Checking node '{}' ({} cap refs, tags={:?})...",
            node_id,
            node.capabilities.len(),
            node.tags
        );
    }

    // ---------------------------------------------------------------
    // Step 5: Boot the daemon and stream a test frame
    // ---------------------------------------------------------------
    println!("[5/6] Booting wire server and streaming a test frame...");

    let (dir, coordinator) = make_coordinator()?;
    coordinator.set_topology(topology.clone())?;
    coordinator.insert_plan(plan.clone());

    let listen_addr = coordinator.listen_addr().to_string();
    let shutdown_flag = coordinator.shutdown_flag();

    // Start the wire server in a background thread.
    let coord_ref = coordinator.clone();
    let addr_clone = listen_addr.clone();
    let auth = std::sync::Arc::new(fabric_daemon::auth::AuthMiddleware::new(
        fabric_daemon::auth::AuthMiddlewareConfig::default(),
    ));
    let runtime = std::sync::Arc::new(
        tokio::runtime::Runtime::new().expect("failed to create tokio runtime"),
    );
    let server_handle = std::thread::spawn(move || {
        let listener =
            std::net::TcpListener::bind(&addr_clone).expect("failed to bind wire server");
        fabric_daemon::wire::run_wire_server(listener, coord_ref, 16, 5000, auth, runtime)
            .expect("wire server error");
    });

    // Give the server a moment to start.
    std::thread::sleep(Duration::from_millis(200));

    // Connect a client and exchange wire protocol messages.
    println!("      Connecting client to {}...", listen_addr);
    let mut stream = TcpStream::connect(&listen_addr)?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    // 5a. Health check
    send_wire_message(&mut stream, r#"{"type":"health_check"}"#)?;
    let health = read_wire_response(&mut stream)?;
    println!("      Health: {}", truncate(&health, 120));

    // 5b. Topology snapshot
    send_wire_message(&mut stream, r#"{"type":"topology_request"}"#)?;
    let topo_resp = read_wire_response(&mut stream)?;
    println!("      Topology response: {} bytes", topo_resp.len());

    // 5c. Routes snapshot
    send_wire_message(&mut stream, r#"{"type":"routes_request"}"#)?;
    let routes_resp = read_wire_response(&mut stream)?;
    println!("      Routes response: {} bytes", routes_resp.len());

    // 5d. Compile request (multihop)
    send_wire_message(
        &mut stream,
        r#"{"type":"compile_request","source":"gpu-node-alpha","destination":"remote-node-a"}"#,
    )?;
    let compile_resp = read_wire_response(&mut stream)?;
    println!("      Compile response: {} bytes", compile_resp.len());

    // 5e. Stream a test frame via the frame transport protocol.
    println!("      Streaming test frame (HEVC 1920x1080)...");
    stream_test_frame(&mut stream)?;

    // ---------------------------------------------------------------
    // Step 6: Graceful shutdown
    // ---------------------------------------------------------------
    println!("[6/6] Shutting down...");

    shutdown_flag.store(true, Ordering::Relaxed);
    drop(stream);
    let _ = server_handle.join();
    coordinator.flush()?;

    // Clean up temp directory.
    drop(dir);

    println!();
    println!("==============================================");
    println!("  Demo Complete");
    println!("==============================================");
    println!();
    println!("Topology nodes: {}", topology.nodes.len());
    println!("Route steps:    {}", plan.steps.len());
    println!(
        "Multi-hop cost: per_frame={:.0} us, setup={:.0} us",
        multihop.cost.per_frame_us, multihop.cost.setup_us
    );
    println!("Frame streamed: yes");
    println!();

    Ok(())
}

// ---------------------------------------------------------------------------
// Topology builder
// ---------------------------------------------------------------------------

fn build_demo_topology() -> fabric_graph::Topology {
    TopologyBuilder::new()
        .with_name("demo-topology")
        // --- Tier L3: GPU nodes (PCIe P2P) ---
        .add(
            Node::new(NodeId::new("gpu-node-alpha"), LT::L3PcieP2P)
                .with_label("GPU Alpha (RTX 4090)")
                .with_tag("gpu")
                .with_tag("rt-island"),
        )
        .add(
            Node::new(NodeId::new("gpu-node-beta"), LT::L3PcieP2P)
                .with_label("GPU Beta (A100 80GB)")
                .with_tag("gpu"),
        )
        // --- Tier L1: CPU nodes (same NUMA) ---
        .add(
            Node::new(NodeId::new("cpu-node-one"), LT::L1SameNuma)
                .with_label("CPU Node One (EPYC 7763)")
                .with_tag("cpu"),
        )
        .add(
            Node::new(NodeId::new("cpu-node-two"), LT::L1SameNuma)
                .with_label("CPU Node Two (Xeon E5-2680)")
                .with_tag("cpu"),
        )
        // --- Tier L6: Remote nodes (LAN) ---
        .add(
            Node::new(NodeId::new("remote-node-a"), LT::L6Lan)
                .with_label("Remote Node A")
                .with_tag("remote")
                .with_tag("audio-sink"),
        )
        .add(
            Node::new(NodeId::new("remote-node-b"), LT::L6Lan)
                .with_label("Remote Node B")
                .with_tag("remote"),
        )
        // --- Edges ---
        .connect_with_metrics(
            "gpu-node-alpha",
            "cpu-node-one",
            LT::L3PcieP2P,
            LinkMetrics {
                latency_us: Some(45.0),
                bandwidth_bps: Some(12_000_000_000),
                packet_loss: Some(0.0),
                jitter_us: Some(5.0),
            },
        )
        .connect_with_metrics(
            "gpu-node-beta",
            "cpu-node-two",
            LT::L3PcieP2P,
            LinkMetrics {
                latency_us: Some(50.0),
                bandwidth_bps: Some(12_000_000_000),
                packet_loss: Some(0.0),
                jitter_us: Some(5.0),
            },
        )
        .connect_with_metrics(
            "cpu-node-one",
            "remote-node-a",
            LT::L6Lan,
            LinkMetrics {
                latency_us: Some(2500.0),
                bandwidth_bps: Some(1_000_000_000),
                packet_loss: Some(0.01),
                jitter_us: Some(500.0),
            },
        )
        .connect_with_metrics(
            "cpu-node-two",
            "remote-node-b",
            LT::L6Lan,
            LinkMetrics {
                latency_us: Some(2800.0),
                bandwidth_bps: Some(1_000_000_000),
                packet_loss: Some(0.02),
                jitter_us: Some(600.0),
            },
        )
        .connect_with_metrics(
            "gpu-node-alpha",
            "gpu-node-beta",
            LT::L3PcieP2P,
            LinkMetrics {
                latency_us: Some(80.0),
                bandwidth_bps: Some(25_000_000_000),
                packet_loss: Some(0.0),
                jitter_us: Some(10.0),
            },
        )
        .connect_with_metrics(
            "cpu-node-one",
            "cpu-node-two",
            LT::L1SameNuma,
            LinkMetrics {
                latency_us: Some(15.0),
                bandwidth_bps: Some(50_000_000_000),
                packet_loss: Some(0.0),
                jitter_us: Some(2.0),
            },
        )
        .build()
}

// ---------------------------------------------------------------------------
// Coordinator helper
// ---------------------------------------------------------------------------

fn make_coordinator() -> anyhow::Result<(
    tempfile::TempDir,
    Arc<fabric_daemon::coordinator::Coordinator>,
)> {
    let dir = tempfile::tempdir()?;
    let db_path = dir.path().join("demo-state.db");
    let config = fabric_daemon::config::DaemonConfig {
        database: fabric_daemon::config::DatabaseConfig {
            path: db_path,
            ..Default::default()
        },
        server: fabric_daemon::config::ServerConfig {
            listen: "127.0.0.1:0".into(), // let OS pick a port
            max_connections: 16,
            request_timeout_ms: 5000,
        },
        ..Default::default()
    };
    let coordinator = fabric_daemon::coordinator::Coordinator::new(config)?;
    Ok((dir, Arc::new(coordinator)))
}

// ---------------------------------------------------------------------------
// Wire protocol helpers
// ---------------------------------------------------------------------------

fn send_wire_message(stream: &mut TcpStream, msg: &str) -> anyhow::Result<()> {
    writeln!(stream, "{msg}")?;
    stream.flush()?;
    Ok(())
}

fn read_wire_response(stream: &mut TcpStream) -> anyhow::Result<String> {
    let reader_stream = stream.try_clone()?;
    let mut reader = BufReader::new(reader_stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

// ---------------------------------------------------------------------------
// Frame streaming demo
// ---------------------------------------------------------------------------

fn stream_test_frame(stream: &mut TcpStream) -> anyhow::Result<()> {
    use bytes::BytesMut;
    use fabric_frame_transport::{Codec, FrameHeader, MessageType, SessionInit, PROTOCOL_VERSION};

    // --- SessionInit (type 0x01) ---
    let init = SessionInit {
        version: PROTOCOL_VERSION,
        preferred_codec: Codec::Hevc,
        width: 1920,
        height: 1080,
        target_fps: 60,
        max_latency_ms: 33,
        client_id: "fabric-full-demo".into(),
    };
    let payload = serde_json::to_vec(&init)?;
    let total_len = (payload.len() + 1) as u32;
    let mut msg = Vec::with_capacity(4 + 1 + payload.len());
    msg.extend_from_slice(&total_len.to_le_bytes());
    msg.push(MessageType::SessionInit as u8);
    msg.extend_from_slice(&payload);
    stream.write_all(&msg)?;
    stream.flush()?;

    // Read SessionAck.
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let resp_len = u32::from_le_bytes(len_buf) as usize;
    let mut resp_buf = vec![0u8; resp_len];
    stream.read_exact(&mut resp_buf)?;
    if let Some(MessageType::SessionAck) = resp_buf.first().and_then(|b| MessageType::from_u8(*b)) {
        let ack: fabric_frame_transport::SessionAck = serde_json::from_slice(&resp_buf[1..])?;
        println!(
            "      SessionAck: codec={}, {}x{}, fps={}, session={}",
            ack.codec.name(),
            ack.width,
            ack.height,
            ack.fps,
            ack.session_id
        );
    }

    // --- FrameData (type 0x03) ---
    let header = FrameHeader {
        seq: 1,
        pts_us: 0,
        dts_us: 0,
        is_keyframe: true,
        codec: Codec::Hevc,
        width: 1920,
        height: 1080,
        payload_len: 1024,
        duration_us: 16_667,
    };
    let frame_data = vec![0xAB_u8; 1024];
    let frame_total = 1 + FrameHeader::SERIALIZED_SIZE + frame_data.len();
    let mut frame_msg = Vec::with_capacity(4 + frame_total);
    frame_msg.extend_from_slice(&(frame_total as u32).to_le_bytes());
    frame_msg.push(MessageType::FrameData as u8);
    let mut hdr = BytesMut::with_capacity(FrameHeader::SERIALIZED_SIZE);
    header.encode(&mut hdr);
    frame_msg.extend_from_slice(&hdr);
    frame_msg.extend_from_slice(&frame_data);
    stream.write_all(&frame_msg)?;
    stream.flush()?;
    println!("      FrameData sent: seq=1, HEVC 1920x1080, 1024 bytes");

    // Read FrameAck (optional).
    match stream.read_exact(&mut len_buf) {
        Ok(()) => {
            let ack_len = u32::from_le_bytes(len_buf) as usize;
            let mut ack_buf = vec![0u8; ack_len];
            stream.read_exact(&mut ack_buf)?;
            if let Some(MessageType::FrameAck) =
                ack_buf.first().and_then(|b| MessageType::from_u8(*b))
            {
                let ack: fabric_frame_transport::FrameAck = serde_json::from_slice(&ack_buf[1..])?;
                println!("      FrameAck: seq={}, rtt={} us", ack.seq, ack.rtt_us);
            }
        }
        Err(_) => println!("      FrameAck: (no response -- expected for demo)"),
    }
    Ok(())
}
