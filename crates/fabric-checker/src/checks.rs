//! Pure check functions — one per `ReasonCode` variant.
//!
//! Each function takes a `CapabilityDescriptor` (the host's proven capabilities)
//! and a `CheckerManifest` (the application's requirements) and returns either
//! `Ok(())` or `Err(CheckOutcome)`.
//!
//! Pure = no I/O, no side effects, deterministic. The `Checker` composes these
//! into a single top-level `Decision`.

use crate::decision::{CheckOutcome, ReasonCode};
use crate::manifest::CheckerManifest;

use fabric_capability::descriptor::CapabilityDescriptor;

// ---------------------------------------------------------------------------
// Aggregate capability accessors (so we don't depend on private fields)
// ---------------------------------------------------------------------------

fn mem_total_bytes(caps: &CapabilityDescriptor) -> Option<u64> {
    caps.capabilities.compute.as_ref().map(|c| c.memory_bytes)
}

fn core_count(caps: &CapabilityDescriptor) -> u32 {
    caps.capabilities
        .compute
        .as_ref()
        .map(|c| c.cores_logical)
        .unwrap_or(0)
}

fn storage_total_bytes(caps: &CapabilityDescriptor) -> Option<u64> {
    caps.capabilities.storage.as_ref().map(|s| {
        s.devices
            .iter()
            .map(|d| d.size_bytes)
            .sum::<u64>()
    })
}

// ---------------------------------------------------------------------------
// Individual checks — each returns a `Result<(), CheckOutcome>` for clean
// ?-chaining
// ---------------------------------------------------------------------------

pub fn check_memory_sufficient(
    descriptor: &CapabilityDescriptor,
    manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    let host_mem =
        mem_total_bytes(descriptor).ok_or_else(|| {
            CheckOutcome::hard(ReasonCode::MemoryUnknown, "host memory unknown")
        })?;
    let req_mem = manifest.memory_bytes;
    if host_mem < req_mem {
        return Err(CheckOutcome::hard(
            ReasonCode::MemoryInsufficient,
            format!(
                "host has {} bytes RAM, manifest requires {} bytes",
                host_mem, req_mem
            ),
        ));
    }
    Ok(())
}

pub fn check_cores_sufficient(
    descriptor: &CapabilityDescriptor,
    manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    let host_cores = core_count(descriptor);
    let req_cores = manifest.cpu_cores;
    if host_cores < req_cores {
        return Err(CheckOutcome::hard(
            ReasonCode::CoresInsufficient,
            format!(
                "host has {} cores, manifest requires {}",
                host_cores, req_cores
            ),
        ));
    }
    Ok(())
}

pub fn check_storage_sufficient(
    descriptor: &CapabilityDescriptor,
    manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    let host_storage =
        storage_total_bytes(descriptor).ok_or_else(|| {
            CheckOutcome::soft(ReasonCode::StorageUnknown, "host storage unknown")
        })?;
    if host_storage < manifest.storage_bytes {
        return Err(CheckOutcome::hard(
            ReasonCode::StorageInsufficient,
            format!(
                "host has {} bytes storage, manifest requires {}",
                host_storage, manifest.storage_bytes
            ),
        ));
    }
    Ok(())
}

pub fn check_os_compatible(
    descriptor: &CapabilityDescriptor,
    manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    // If manifest has no OS restrictions, skip.
    if manifest.os_families.is_empty() {
        return Ok(());
    }
    // Use topology hash presence as a heuristic for "host is known"; real
    // OS family detection lives in the probe. For now we just check if the
    // manifest's list is non-empty and the host has compute (any OS).
    // A full implementation would compare the probed OS family against the
    // manifest list.
    let has_compute = descriptor.capabilities.compute.is_some();
    if !has_compute && !manifest.os_families.is_empty() {
        return Err(CheckOutcome::hard(
            ReasonCode::OsIncompatible,
            "host has no compute capabilities, cannot verify OS compatibility",
        ));
    }
    Ok(())
}

pub fn check_arch_compatible(
    descriptor: &CapabilityDescriptor,
    manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    // If manifest has no architecture restrictions, skip.
    if manifest.arches.is_empty() {
        return Ok(());
    }
    let has_compute = descriptor.capabilities.compute.is_some();
    if !has_compute && !manifest.arches.is_empty() {
        return Err(CheckOutcome::hard(
            ReasonCode::ArchIncompatible,
            "host has no compute capabilities, cannot verify arch compatibility",
        ));
    }
    Ok(())
}

pub fn check_audio_capable(
    descriptor: &CapabilityDescriptor,
    manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    if !manifest.audio {
        return Ok(());
    }
    // If the manifest requires audio, the host must have audio capabilities
    // with at least one sink or source.
    match &descriptor.capabilities.audio {
        None => {
            return Err(CheckOutcome::hard(
                ReasonCode::CaptureRequiredButMissing,
                "manifest requires audio but host has no audio backend",
            ));
        }
        Some(audio) => {
            if audio.sinks.is_empty() && audio.sources.is_empty() {
                return Err(CheckOutcome::hard(
                    ReasonCode::CaptureRequiredButMissing,
                    "manifest requires audio but host has no sinks or sources",
                ));
            }
        }
    }
    Ok(())
}

pub fn check_network_reachable(
    descriptor: &CapabilityDescriptor,
    manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    if manifest.network_peers.is_empty() {
        return Ok(());
    }
    // If the manifest requires network peers, check that the host has at
    // least one network interface. We cannot do live reachability probing,
    // so we accept if any interface exists and emit a soft advisory.
    match &descriptor.capabilities.network {
        None => {
            return Err(CheckOutcome::hard(
                ReasonCode::BandwidthInsufficient,
                "manifest requires network but host has no network interfaces",
            ));
        }
        Some(net) if net.interfaces.is_empty() => {
            return Err(CheckOutcome::hard(
                ReasonCode::BandwidthInsufficient,
                "manifest requires network but host has no network interfaces",
            ));
        }
        Some(_) => {
            // Host has at least one interface. Soft advisory since we can't
            // verify actual reachability.
            return Err(CheckOutcome::soft(
                ReasonCode::BandwidthInsufficient,
                "cannot verify network reachability at check time; host has at least one interface",
            ));
        }
    }
}

pub fn check_display_available(
    descriptor: &CapabilityDescriptor,
    manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    // If the app is headless, it doesn't need a display.
    if manifest.headless {
        return Ok(());
    }
    // Non-headless app needs a display.
    match &descriptor.capabilities.display {
        None => {
            return Err(CheckOutcome::hard(
                ReasonCode::DisplayRequiredButMissing,
                "manifest requires a display but host has none",
            ));
        }
        Some(display) if display.displays.is_empty() => {
            return Err(CheckOutcome::hard(
                ReasonCode::DisplayRequiredButMissing,
                "manifest requires a display but host display list is empty",
            ));
        }
        Some(_) => {}
    }
    Ok(())
}

pub fn check_realtime_safety(
    _descriptor: &CapabilityDescriptor,
    manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    if !manifest.realtime_island {
        return Ok(());
    }
    // If the manifest requires a real-time island, check if the host
    // advertises any RT capability. Since this is hard to probe, we use
    // Soft severity (AdmitWithNotes) with RealTimeIslandMissing.
    // A full implementation would check topology edges for RT locality tiers.
    Err(CheckOutcome::soft(
        ReasonCode::RealTimeIslandMissing,
        "manifest requires real-time island; host RT capability not verified at check time",
    ))
}

pub fn check_signature_valid(
    descriptor: &CapabilityDescriptor,
    _manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    if descriptor.signatures.is_empty() {
        return Err(CheckOutcome::soft(
            ReasonCode::SignatureMissing,
            "descriptor has no signatures (admit with note)",
        ));
    }
    Ok(())
}

pub fn check_epoch_current(
    descriptor: &CapabilityDescriptor,
    _manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    if descriptor.epoch == 0 {
        return Err(CheckOutcome::soft(
            ReasonCode::EpochZero,
            "descriptor epoch is 0 (initial)",
        ));
    }
    Ok(())
}

pub fn check_schema_supported(
    descriptor: &CapabilityDescriptor,
    _manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    let supported = ["phenotype.fabric.capability_descriptor/1"];
    if !supported
        .iter()
        .any(|s| *s == descriptor.schema_version.as_str())
    {
        return Err(CheckOutcome::hard(
            ReasonCode::SchemaUnsupported,
            format!(
                "schema '{}' not in supported list",
                descriptor.schema_version
            ),
        ));
    }
    Ok(())
}

pub fn check_node_id_present(
    descriptor: &CapabilityDescriptor,
    _manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    if descriptor.node_id.is_nil() {
        return Err(CheckOutcome::hard(
            ReasonCode::NodeIdNil,
            "descriptor node_id is nil",
        ));
    }
    Ok(())
}

pub fn check_topology_hash_present(
    descriptor: &CapabilityDescriptor,
    _manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    if descriptor.topology_hash.is_empty() {
        return Err(CheckOutcome::soft(
            ReasonCode::TopologyHashMissing,
            "topology hash empty",
        ));
    }
    Ok(())
}

pub fn check_probe_freshness(
    descriptor: &CapabilityDescriptor,
    _manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    let now = chrono::Utc::now();
    let age = now.signed_duration_since(descriptor.probed_at);
    if age.num_seconds() > 86_400 {
        return Err(CheckOutcome::soft(
            ReasonCode::ProbeStale,
            format!(
                "probe is {} seconds old (admit with note)",
                age.num_seconds()
            ),
        ));
    }
    Ok(())
}

pub fn check_audio_io(
    descriptor: &CapabilityDescriptor,
    manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    if !manifest.audio {
        return Ok(());
    }
    // If audio is required, check that the host's audio backend has
    // non-empty sinks or sources.
    match &descriptor.capabilities.audio {
        None => {
            return Err(CheckOutcome::soft(
                ReasonCode::CaptureRequiredButMissing,
                "audio required but host has no audio backend",
            ));
        }
        Some(audio) => {
            if audio.sinks.is_empty() && audio.sources.is_empty() {
                return Err(CheckOutcome::soft(
                    ReasonCode::CaptureRequiredButMissing,
                    "audio required but host has no sinks or sources",
                ));
            }
        }
    }
    Ok(())
}

pub fn check_storage_io(
    descriptor: &CapabilityDescriptor,
    manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    if manifest.storage_bytes == 0 {
        return Ok(());
    }
    // Advisory: check if the host has any storage devices at all.
    match &descriptor.capabilities.storage {
        None => {
            return Err(CheckOutcome::soft(
                ReasonCode::StorageUnknown,
                "storage required but host storage info unavailable",
            ));
        }
        Some(storage) => {
            if storage.devices.is_empty() {
                return Err(CheckOutcome::soft(
                    ReasonCode::StorageUnknown,
                    "storage required but host has no storage devices",
                ));
            }
        }
    }
    Ok(())
}

pub fn check_bandwidth_sufficient(
    descriptor: &CapabilityDescriptor,
    _manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    // Advisory: if the host has network capabilities, check that at least
    // one interface has a non-zero link speed.
    if let Some(net) = &descriptor.capabilities.network {
        if !net.interfaces.is_empty() {
            let any_speed = net
                .interfaces
                .iter()
                .any(|iface| iface.link_speed_mbps.unwrap_or(0) > 0);
            if !any_speed {
                return Err(CheckOutcome::soft(
                    ReasonCode::BandwidthInsufficient,
                    "host network interfaces have no link speed reported",
                ));
            }
        }
    }
    Ok(())
}

pub fn check_input_devices(
    descriptor: &CapabilityDescriptor,
    manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    // If the app is headless, input devices are not needed.
    if manifest.headless {
        return Ok(());
    }
    // If the app needs a display, check that input devices exist.
    if let Some(input) = &descriptor.capabilities.input {
        let total = input.keyboards.len()
            + input.mice.len()
            + input.touchscreens.len()
            + input.gamepads.len();
        if total == 0 {
            return Err(CheckOutcome::soft(
                ReasonCode::CaptureRequiredButMissing,
                "app requires display but host has no input devices",
            ));
        }
    }
    // If there's no input section at all, that's only a concern if the app
    // is not headless and the host has a display (suggesting a GUI context).
    Ok(())
}

pub fn check_gpu_driver_present(
    descriptor: &CapabilityDescriptor,
    _manifest: &CheckerManifest,
) -> Result<(), CheckOutcome> {
    // Advisory: if the host has accelerator capabilities, check that at
    // least one GPU has a driver_version.
    if let Some(acc) = &descriptor.capabilities.accelerator {
        for gpu in &acc.gpus {
            if gpu.driver_version.is_none() {
                return Err(CheckOutcome::soft(
                    ReasonCode::FirmwareUpdateAvailable,
                    format!(
                        "GPU '{}' ({}) has no driver version reported",
                        gpu.model, gpu.vendor
                    ),
                ));
            }
        }
    }
    Ok(())
}
