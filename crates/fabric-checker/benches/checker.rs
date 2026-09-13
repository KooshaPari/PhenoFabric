//! Criterion benchmarks for fabric-checker admission checks.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fabric_checker::{check, run_all};
use fabric_checker::manifest::CheckerManifest;
use fabric_capability::descriptor::{
    AudioCapabilities, AudioDevice, Capabilities, ComputeCapabilities, DisplayCapabilities,
    DisplayInfo, GpuInfo, AcceleratorCapabilities, HardwareCodecMatrix, InputCapabilities,
    NetworkCapabilities, NetworkInterface, StorageCapabilities, StorageDevice,
};
use fabric_capability::descriptor::CapabilityDescriptor;
use uuid::Uuid;

/// Build a descriptor that should pass most checks (16 GiB RAM, 8 cores, 1 TiB SSD).
fn passing_descriptor() -> CapabilityDescriptor {
    CapabilityDescriptor {
        node_id: Uuid::now_v7(),
        epoch: 1,
        schema_version: "phenotype.fabric.capability_descriptor/1".to_string(),
        probed_at: chrono::Utc::now(),
        probe_version: "0.1.0".to_string(),
        topology_hash: "bench-hash".to_string(),
        capabilities: Capabilities {
            compute: Some(ComputeCapabilities {
                processor: "bench-cpu".to_string(),
                cores_physical: 8,
                cores_logical: 8,
                numa_nodes: 1,
                cache: vec![],
                memory_bytes: 16 * 1024 * 1024 * 1024,
                memory_bandwidth_mbps: None,
                hyperthread_pairs: vec![],
                tdp_watts: None,
            }),
            storage: Some(StorageCapabilities {
                devices: vec![StorageDevice {
                    path: "/dev/sda".to_string(),
                    size_bytes: 1024 * 1024 * 1024 * 1024,
                    is_ssd: true,
                    read_iops_approx: None,
                    write_iops_approx: None,
                }],
                network_mounts: vec![],
            }),
            audio: Some(AudioCapabilities {
                backend: "pulse".to_string(),
                sinks: vec![AudioDevice {
                    name: "default".to_string(),
                    sample_rates: vec![44100, 48000],
                    channels: 2,
                }],
                sources: vec![AudioDevice {
                    name: "default".to_string(),
                    sample_rates: vec![44100, 48000],
                    channels: 1,
                }],
                midi_ports: 0,
            }),
            network: Some(NetworkCapabilities {
                interfaces: vec![NetworkInterface {
                    name: "eth0".to_string(),
                    mac_address: Some("00:11:22:33:44:55".to_string()),
                    link_speed_mbps: Some(10_000),
                    mtu: 1500,
                    rdma_capable: false,
                    zerocopy_capable: false,
                    rss_queues: 4,
                    ipv4: Some("10.0.0.2".to_string()),
                    ipv6: None,
                }],
            }),
            display: Some(DisplayCapabilities {
                displays: vec![DisplayInfo {
                    name: "primary".to_string(),
                    width_px: 1920,
                    height_px: 1080,
                    refresh_hz: Some(60.0),
                    hdr: false,
                    edid_hash: None,
                }],
                wayland: true,
                x11: true,
                hdr_max_nits: None,
            }),
            input: Some(InputCapabilities {
                keyboards: vec!["keyboard0".to_string()],
                mice: vec!["mouse0".to_string()],
                touchscreens: vec![],
                gamepads: vec![],
            }),
            accelerator: Some(AcceleratorCapabilities {
                gpus: vec![GpuInfo {
                    vendor: "NVIDIA".to_string(),
                    model: "RTX 4090".to_string(),
                    vram_bytes: Some(8 * 1024 * 1024 * 1024),
                    compute_capability: Some("8.9".to_string()),
                    driver_version: Some("535.0".to_string()),
                }],
                hardware_codecs: HardwareCodecMatrix {
                    h264_decode: true,
                    h264_encode: true,
                    h265_decode: true,
                    h265_encode: true,
                    av1_decode: false,
                    av1_encode: false,
                    vp9_decode: true,
                    vp9_encode: false,
                },
                npu_present: false,
            }),
            ..Default::default()
        },
        signatures: vec![],
    }
}

/// Build a manifest that should pass (modest requirements).
fn passing_manifest() -> CheckerManifest {
    CheckerManifest {
        memory_bytes: 8 * 1024 * 1024 * 1024,
        cpu_cores: 4,
        storage_bytes: 100 * 1024 * 1024 * 1024,
        os_families: vec![],
        arches: vec![],
        audio: true,
        network_peers: vec![],
        headless: false,
        realtime_island: false,
    }
}

/// Build a manifest that should fail (requires more memory than host has).
fn failing_manifest() -> CheckerManifest {
    CheckerManifest {
        memory_bytes: 128 * 1024 * 1024 * 1024, // 128 GiB (host has 16)
        cpu_cores: 64,
        storage_bytes: 100 * 1024 * 1024 * 1024,
        os_families: vec![],
        arches: vec![],
        audio: false,
        network_peers: vec![],
        headless: false,
        realtime_island: false,
    }
}

/// Build a manifest that exercises many check rules.
fn full_manifest() -> CheckerManifest {
    CheckerManifest {
        memory_bytes: 8 * 1024 * 1024 * 1024,
        cpu_cores: 4,
        storage_bytes: 100 * 1024 * 1024 * 1024,
        os_families: vec!["linux".to_string()],
        arches: vec!["x86_64".to_string()],
        audio: true,
        network_peers: vec!["10.0.0.1".to_string()],
        headless: false,
        realtime_island: false,
    }
}

/// Benchmark: check a manifest that should pass.
fn bench_check_compatible(c: &mut Criterion) {
    let descriptor = passing_descriptor();
    let manifest = passing_manifest();

    c.bench_function("check_compatible", |b| {
        b.iter(|| {
            let decision = check(black_box(&descriptor), black_box(&manifest));
            black_box(&decision);
        });
    });
}

/// Benchmark: check a manifest that should fail (memory insufficient).
fn bench_check_incompatible(c: &mut Criterion) {
    let descriptor = passing_descriptor();
    let manifest = failing_manifest();

    c.bench_function("check_incompatible", |b| {
        b.iter(|| {
            let decision = check(black_box(&descriptor), black_box(&manifest));
            assert!(!decision.is_admissible());
            black_box(&decision);
        });
    });
}

/// Benchmark: run_all with a manifest that exercises many check rules.
fn bench_run_all_10_checks(c: &mut Criterion) {
    let descriptor = passing_descriptor();
    let manifest = full_manifest();

    c.bench_function("run_all_10_checks", |b| {
        b.iter(|| {
            let outcomes = run_all(black_box(&descriptor), black_box(&manifest));
            black_box(&outcomes);
        });
    });
}

criterion_group!(
    benches,
    bench_check_compatible,
    bench_check_incompatible,
    bench_run_all_10_checks,
);
criterion_main!(benches);
