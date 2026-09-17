//! Top-level `Checker` — composes all individual check functions into a
//! single `Decision`. Order of composition matters: hard requirements
//! (Severity::Reject) short-circuit; soft warnings (Severity::AdmitWithNotes)
//! accumulate without failing the decision.

use crate::checks;
use crate::decision::{CheckOutcome, Decision, Severity};
use crate::manifest::CheckerManifest;

use fabric_capability::descriptor::CapabilityDescriptor;

type CheckFn = fn(&CapabilityDescriptor, &CheckerManifest) -> Result<(), CheckOutcome>;

/// Runs the full check suite against a host descriptor and a manifest.
///
/// Returns a `Decision` with `Admit` if all reject-level checks pass,
/// `AdmitWithNotes` if only notes-level checks failed, or `Reject` if any
/// reject-level check failed.
pub fn check(descriptor: &CapabilityDescriptor, manifest: &CheckerManifest) -> Decision {
    let outcomes = run_all(descriptor, manifest);
    collapse(outcomes)
}

/// Public for testing — runs every check and returns the raw outcomes.
pub fn run_all(descriptor: &CapabilityDescriptor, manifest: &CheckerManifest) -> Vec<CheckOutcome> {
    let fns: Vec<CheckFn> = vec![
        checks::check_memory_sufficient,
        checks::check_cores_sufficient,
        checks::check_storage_sufficient,
        checks::check_os_compatible,
        checks::check_arch_compatible,
        checks::check_audio_capable,
        checks::check_network_reachable,
        checks::check_display_available,
        checks::check_realtime_safety,
        checks::check_signature_valid,
        checks::check_epoch_current,
        checks::check_schema_supported,
        checks::check_node_id_present,
        checks::check_topology_hash_present,
        checks::check_probe_freshness,
        checks::check_audio_io,
        checks::check_storage_io,
        checks::check_bandwidth_sufficient,
        checks::check_input_devices,
        checks::check_gpu_driver_present,
    ];
    fns.into_iter()
        .map(|f| f(descriptor, manifest))
        .filter_map(|r| r.err())
        .collect()
}

/// Collapse a list of `CheckOutcome` failures into a top-level `Decision`.
/// Any `Severity::Reject` outcome → `Decision::Reject`. Otherwise, if any
/// `Severity::AdmitWithNotes` → `Decision::AdmitWithNotes`. Otherwise
/// `Decision::Admit`.
pub fn collapse(outcomes: Vec<CheckOutcome>) -> Decision {
    if outcomes.is_empty() {
        return Decision::Admit;
    }
    let mut notes: Vec<CheckOutcome> = Vec::new();
    for o in outcomes {
        match o.severity {
            Severity::Reject => {
                return Decision::Reject {
                    reason_code: o.code,
                    reason_message: o.message,
                };
            }
            Severity::AdmitWithNotes => {
                notes.push(o);
            }
        }
    }
    if notes.is_empty() {
        Decision::Admit
    } else {
        Decision::AdmitWithNotes { notes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::ReasonCode;
    use fabric_capability::descriptor::{
        AcceleratorCapabilities, AudioCapabilities, Capabilities, ComputeCapabilities,
        DisplayCapabilities, DisplayInfo, GpuInfo, HardwareCodecMatrix, InputCapabilities,
        NetworkCapabilities, NetworkInterface, StorageCapabilities, StorageDevice,
    };
    use uuid::Uuid;

    /// Helper to build a minimal valid descriptor that passes basic checks.
    fn base_descriptor() -> CapabilityDescriptor {
        CapabilityDescriptor {
            node_id: Uuid::now_v7(),
            epoch: 1,
            schema_version: "phenotype.fabric.capability_descriptor/1".to_string(),
            probed_at: chrono::Utc::now(),
            probe_version: "0.1.0".to_string(),
            topology_hash: "abc123".to_string(),
            capabilities: Capabilities {
                compute: Some(ComputeCapabilities {
                    processor: "test".to_string(),
                    cores_physical: 8,
                    cores_logical: 8,
                    numa_nodes: 1,
                    cache: vec![],
                    memory_bytes: 16 * 1024 * 1024 * 1024, // 16 GiB
                    memory_bandwidth_mbps: None,
                    hyperthread_pairs: vec![],
                    tdp_watts: None,
                }),
                storage: Some(StorageCapabilities {
                    devices: vec![StorageDevice {
                        path: "/dev/sda".to_string(),
                        size_bytes: 1024 * 1024 * 1024 * 1024, // 1 TiB
                        is_ssd: true,
                        read_iops_approx: None,
                        write_iops_approx: None,
                    }],
                    network_mounts: vec![],
                }),
                ..Default::default()
            },
            signatures: vec![],
        }
    }

    fn empty_manifest() -> CheckerManifest {
        // Headless by default so display checks are skipped unless a test
        // explicitly sets headless: false.
        CheckerManifest {
            headless: true,
            ..CheckerManifest::default()
        }
    }

    // ----- collapse tests -----

    #[test]
    fn empty_outcomes_admit() {
        let d = collapse(vec![]);
        assert_eq!(d, Decision::Admit);
    }

    #[test]
    fn notes_only_admit_with_notes() {
        let d = collapse(vec![CheckOutcome::soft(
            ReasonCode::SignatureMissing,
            "no sigs",
        )]);
        assert!(matches!(d, Decision::AdmitWithNotes { .. }));
    }

    #[test]
    fn reject_short_circuits() {
        let d = collapse(vec![
            CheckOutcome::soft(ReasonCode::SignatureMissing, "no sigs"),
            CheckOutcome::hard(ReasonCode::MemoryInsufficient, "out of memory"),
        ]);
        assert!(matches!(d, Decision::Reject { .. }));
    }

    #[test]
    fn run_all_on_empty_inputs_produces_only_notes() {
        // The default descriptor has epoch=0 and no signatures, so
        // check_signature_valid and check_epoch_current both produce notes.
        // All other checks should pass on empty inputs.
        let descriptor = base_descriptor();
        let manifest = empty_manifest();
        let outcomes = run_all(&descriptor, &manifest);
        // Should have notes but no rejects.
        let d = collapse(outcomes);
        assert!(matches!(d, Decision::AdmitWithNotes { .. }));
    }

    // ----- audio check tests -----

    #[test]
    fn test_audio_required_but_host_has_none() {
        let mut descriptor = base_descriptor();
        // No audio capabilities on the host.
        descriptor.capabilities.audio = None;

        let manifest = CheckerManifest {
            audio: true,
            ..empty_manifest()
        };

        let d = check(&descriptor, &manifest);
        assert!(matches!(d, Decision::Reject { .. }));
    }

    #[test]
    fn test_audio_required_host_has() {
        let mut descriptor = base_descriptor();
        descriptor.capabilities.audio = Some(AudioCapabilities {
            backend: "alsa".to_string(),
            sinks: vec![],
            sources: vec![fabric_capability::descriptor::AudioDevice {
                name: "mic0".to_string(),
                sample_rates: vec![44100, 48000],
                channels: 1,
            }],
            midi_ports: 0,
        });

        let manifest = CheckerManifest {
            audio: true,
            ..empty_manifest()
        };

        let d = check(&descriptor, &manifest);
        // audio check passes; other checks (epoch, sig, schema) produce notes.
        assert!(d.is_admissible());
    }

    // ----- display check tests -----

    #[test]
    fn test_display_required_but_missing() {
        let mut descriptor = base_descriptor();
        // No display capabilities.
        descriptor.capabilities.display = None;

        let manifest = CheckerManifest {
            headless: false,
            ..empty_manifest()
        };

        let d = check(&descriptor, &manifest);
        // Should be rejected because display is required but missing.
        let reject = match d {
            Decision::Reject { reason_code, .. } => reason_code,
            _ => panic!("expected Reject, got {:?}", d),
        };
        assert_eq!(reject, ReasonCode::DisplayRequiredButMissing);
    }

    #[test]
    fn test_display_not_required_ok() {
        let mut descriptor = base_descriptor();
        descriptor.capabilities.display = None;

        // headless: true (already the default from empty_manifest)
        let manifest = empty_manifest();

        let d = check(&descriptor, &manifest);
        // Headless app doesn't need display, should be admissible.
        assert!(d.is_admissible());
    }

    // ----- realtime island tests -----

    #[test]
    fn test_realtime_island_missing_soft() {
        let mut descriptor = base_descriptor();
        descriptor.capabilities.topology = None;

        let manifest = CheckerManifest {
            realtime_island: true,
            ..empty_manifest()
        };

        let d = check(&descriptor, &manifest);
        // RT island not verified → AdmitWithNotes.
        assert!(matches!(d, Decision::AdmitWithNotes { .. }));
    }

    // ----- network check tests -----

    #[test]
    fn test_network_host_has_interface() {
        let mut descriptor = base_descriptor();
        descriptor.capabilities.network = Some(NetworkCapabilities {
            interfaces: vec![NetworkInterface {
                name: "eth0".to_string(),
                mac_address: Some("00:11:22:33:44:55".to_string()),
                link_speed_mbps: Some(1000),
                mtu: 1500,
                rdma_capable: false,
                zerocopy_capable: false,
                rss_queues: 4,
                ipv4: Some("10.0.0.1".to_string()),
                ipv6: None,
            }],
        });

        let manifest = CheckerManifest {
            network_peers: vec!["peer1".to_string()],
            ..empty_manifest()
        };

        let d = check(&descriptor, &manifest);
        // Network has interface → soft note only, admissible.
        assert!(d.is_admissible());
    }

    #[test]
    fn test_network_no_interface_reject() {
        let mut descriptor = base_descriptor();
        descriptor.capabilities.network = None;

        let manifest = CheckerManifest {
            network_peers: vec!["peer1".to_string()],
            ..empty_manifest()
        };

        let d = check(&descriptor, &manifest);
        // No network interface → Reject.
        assert!(matches!(d, Decision::Reject { .. }));
    }

    // ----- storage check tests -----

    #[test]
    fn test_storage_required_but_insufficient() {
        let mut descriptor = base_descriptor();
        descriptor.capabilities.storage = Some(StorageCapabilities {
            devices: vec![StorageDevice {
                path: "/dev/sda".to_string(),
                size_bytes: 100 * 1024 * 1024, // 100 MiB
                is_ssd: true,
                read_iops_approx: None,
                write_iops_approx: None,
            }],
            network_mounts: vec![],
        });

        let manifest = CheckerManifest {
            storage_bytes: 1024 * 1024 * 1024, // 1 GiB
            ..empty_manifest()
        };

        let d = check(&descriptor, &manifest);
        assert!(matches!(d, Decision::Reject { .. }));
    }

    // ----- GPU driver tests -----

    #[test]
    fn test_gpu_driver_missing_soft() {
        let mut descriptor = base_descriptor();
        descriptor.capabilities.accelerator = Some(AcceleratorCapabilities {
            gpus: vec![GpuInfo {
                vendor: "NVIDIA".to_string(),
                model: "RTX 4090".to_string(),
                vram_bytes: Some(24 * 1024 * 1024 * 1024),
                compute_capability: Some("8.9".to_string()),
                driver_version: None, // No driver version!
            }],
            npu_present: false,
            hardware_codecs: HardwareCodecMatrix {
                av1_encode: true,
                av1_decode: true,
                h264_encode: true,
                h264_decode: true,
                h265_encode: true,
                h265_decode: true,
                vp9_encode: true,
                vp9_decode: true,
            },
        });

        let manifest = empty_manifest();
        let d = check(&descriptor, &manifest);
        // GPU driver missing → soft advisory.
        assert!(d.is_admissible());
        assert!(matches!(d, Decision::AdmitWithNotes { .. }));
    }

    // ----- input device tests -----

    #[test]
    fn test_input_devices_empty_with_display_soft() {
        let mut descriptor = base_descriptor();
        descriptor.capabilities.input = Some(InputCapabilities {
            keyboards: vec![],
            mice: vec![],
            touchscreens: vec![],
            gamepads: vec![],
        });
        descriptor.capabilities.display = Some(DisplayCapabilities {
            displays: vec![DisplayInfo {
                name: "monitor0".to_string(),
                width_px: 1920,
                height_px: 1080,
                refresh_hz: Some(60.0),
                hdr: false,
                edid_hash: None,
            }],
            wayland: true,
            x11: false,
            hdr_max_nits: None,
        });

        let manifest = CheckerManifest {
            headless: false,
            ..empty_manifest()
        };

        let d = check(&descriptor, &manifest);
        // Input empty + display present → soft advisory.
        assert!(d.is_admissible());
        assert!(matches!(d, Decision::AdmitWithNotes { .. }));
    }

    // ----- bandwidth advisory tests -----

    #[test]
    fn test_bandwidth_no_link_speed_soft() {
        let mut descriptor = base_descriptor();
        descriptor.capabilities.network = Some(NetworkCapabilities {
            interfaces: vec![NetworkInterface {
                name: "eth0".to_string(),
                mac_address: None,
                link_speed_mbps: None, // No link speed!
                mtu: 1500,
                rdma_capable: false,
                zerocopy_capable: false,
                rss_queues: 1,
                ipv4: None,
                ipv6: None,
            }],
        });

        let manifest = empty_manifest();
        let d = check(&descriptor, &manifest);
        // Bandwidth advisory → soft.
        assert!(d.is_admissible());
        assert!(matches!(d, Decision::AdmitWithNotes { .. }));
    }

    // ----- memory check tests -----

    #[test]
    fn test_memory_insufficient_rejects() {
        let mut descriptor = base_descriptor();
        descriptor.capabilities.compute = Some(ComputeCapabilities {
            processor: "test".to_string(),
            cores_physical: 4,
            cores_logical: 4,
            numa_nodes: 1,
            cache: vec![],
            memory_bytes: 512 * 1024 * 1024, // 512 MiB
            memory_bandwidth_mbps: None,
            hyperthread_pairs: vec![],
            tdp_watts: None,
        });

        let manifest = CheckerManifest {
            memory_bytes: 1024 * 1024 * 1024, // 1 GiB required
            ..empty_manifest()
        };

        let d = check(&descriptor, &manifest);
        assert!(matches!(d, Decision::Reject { .. }));
    }

    // ----- cores check tests -----

    #[test]
    fn test_cores_insufficient_rejects() {
        let mut descriptor = base_descriptor();
        descriptor.capabilities.compute = Some(ComputeCapabilities {
            processor: "test".to_string(),
            cores_physical: 2,
            cores_logical: 2,
            numa_nodes: 1,
            cache: vec![],
            memory_bytes: 8 * 1024 * 1024 * 1024,
            memory_bandwidth_mbps: None,
            hyperthread_pairs: vec![],
            tdp_watts: None,
        });

        let manifest = CheckerManifest {
            cpu_cores: 8,
            ..empty_manifest()
        };

        let d = check(&descriptor, &manifest);
        assert!(matches!(d, Decision::Reject { .. }));
    }
}
