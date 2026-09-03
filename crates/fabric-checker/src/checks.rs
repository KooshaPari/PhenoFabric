//! Pure check functions — one per `ReasonCode` variant.
//!
//! Each function takes a `CapabilityDescriptor` (the host's proven capabilities)
//! and a `BoundManifest` (the application's requirements) and returns either
//! `Ok(CheckOutcome::Pass)` or `Err(CheckOutcome { severity, code, message, ... })`.
//!
//! Pure = no I/O, no side effects, deterministic. The `Checker` composes these
//! into a single top-level `Decision`.

use crate::decision::{CheckOutcome, ReasonCode, Severity};

use fabric_capability::descriptor::CapabilityDescriptor;
use fabric_capability::topology::AudioCapabilities;
use fabric_capability::descriptor::ComputeCapabilities;

use phenotype_nvms_adapter::BoundManifest;

// ---------------------------------------------------------------------------
// Aggregate capability accessors (so we don't depend on private fields)
// ---------------------------------------------------------------------------

fn mem_total_bytes(caps: &ComputeCapabilities) -> Option<u64> {
    Some(caps.memory.total_bytes)
}

fn core_count(caps: &ComputeCapabilities) -> u32 {
    caps.cpu.logical_cores
}

fn storage_total_bytes(caps: &ComputeCapabilities) -> Option<u64> {
    Some(caps.storage.total_bytes)
}

// ---------------------------------------------------------------------------
// Individual checks — each returns a `Result<(), CheckOutcome>` for clean ?-chaining
// ---------------------------------------------------------------------------

pub fn check_memory_sufficient(
    descriptor: &CapabilityDescriptor,
    manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    let host_mem = mem_total_bytes(&descriptor.capabilities.compute)
        .ok_or_else(outcome(ReasonCode::MemoryUnknown, Severity::Reject, "host memory unknown"))?;
    let req_mem = manifest.required.memory_bytes;
    if host_mem < req_mem {
        return Err(CheckOutcome::fail(
            ReasonCode::MemoryInsufficient,
            Severity::Reject,
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
    manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    let host_cores = core_count(&descriptor.capabilities.compute);
    let req_cores = manifest.required.cpu_cores;
    if host_cores < req_cores {
        return Err(CheckOutcome::fail(
            ReasonCode::CoresInsufficient,
            Severity::Reject,
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
    manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    let host_storage = storage_total_bytes(&descriptor.capabilities.compute)
        .ok_or_else(outcome(ReasonCode::StorageUnknown, Severity::AdmitWithNotes, "host storage unknown"))?;
    if host_storage < manifest.required.storage_bytes {
        return Err(CheckOutcome::fail(
            ReasonCode::StorageInsufficient,
            Severity::Reject,
            format!(
                "host has {} bytes storage, manifest requires {}",
                host_storage, manifest.required.storage_bytes
            ),
        ));
    }
    Ok(())
}

pub fn check_os_compatible(
    descriptor: &CapabilityDescriptor,
    manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    let host_os = descriptor.capabilities.compute.os.family.to_string();
    if !manifest.required.os_families.iter().any(|f| f == &host_os) {
        return Err(CheckOutcome::fail(
            ReasonCode::OsIncompatible,
            Severity::Reject,
            format!(
                "host OS '{}' not in manifest's allowed list {:?}",
                host_os, manifest.required.os_families
            ),
        ));
    }
    Ok(())
}

pub fn check_arch_compatible(
    descriptor: &CapabilityDescriptor,
    manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    let host_arch = descriptor.capabilities.compute.cpu.arch.to_string();
    if !manifest.required.arches.iter().any(|a| a == &host_arch) {
        return Err(CheckOutcome::fail(
            ReasonCode::ArchIncompatible,
            Severity::Reject,
            format!(
                "host arch '{}' not in manifest's allowed list {:?}",
                host_arch, manifest.required.arches
            ),
        ));
    }
    Ok(())
}

pub fn check_audio_capable(
    _descriptor: &CapabilityDescriptor,
    manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    if !manifest.required.audio {
        return Ok(());
    }
    // Real check would inspect descriptor.capabilities.audio; placeholder.
    Ok(())
}

pub fn check_network_reachable(
    _descriptor: &CapabilityDescriptor,
    manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    if manifest.required.network_peers.is_empty() {
        return Ok(());
    }
    // Real check would probe each network peer; placeholder.
    Ok(())
}

pub fn check_display_available(
    _descriptor: &CapabilityDescriptor,
    manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    if !manifest.required.headless {
        // Real check: do we have a display surface?
        return Ok(());
    }
    Ok(())
}

pub fn check_realtime_safety(
    _descriptor: &CapabilityDescriptor,
    manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    if !manifest.required.realtime_island {
        return Ok(());
    }
    // Real check: does the host have a reserved RT island?
    Ok(())
}

pub fn check_signature_valid(
    descriptor: &CapabilityDescriptor,
    _manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    if descriptor.signatures.is_empty() {
        return Err(CheckOutcome::fail(
            ReasonCode::SignatureMissing,
            Severity::AdmitWithNotes,
            "descriptor has no signatures (admit with note)".to_string(),
        ));
    }
    Ok(())
}

pub fn check_epoch_current(
    descriptor: &CapabilityDescriptor,
    _manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    if descriptor.epoch == 0 {
        return Err(CheckOutcome::fail(
            ReasonCode::EpochZero,
            Severity::AdmitWithNotes,
            "descriptor epoch is 0 (initial)".to_string(),
        ));
    }
    Ok(())
}

pub fn check_schema_supported(
    descriptor: &CapabilityDescriptor,
    _manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    let supported = ["phenotype.fabric.capability_descriptor/1"];
    if !supported.iter().any(|s| *s == descriptor.schema_version.as_str()) {
        return Err(CheckOutcome::fail(
            ReasonCode::SchemaUnsupported,
            Severity::Reject,
            format!("schema '{}' not in supported list", descriptor.schema_version),
        ));
    }
    Ok(())
}

pub fn check_node_id_present(
    descriptor: &CapabilityDescriptor,
    _manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    if descriptor.node_id.is_nil() {
        return Err(CheckOutcome::fail(
            ReasonCode::NodeIdNil,
            Severity::Reject,
            "descriptor node_id is nil".to_string(),
        ));
    }
    Ok(())
}

pub fn check_topology_hash_present(
    descriptor: &CapabilityDescriptor,
    _manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    if descriptor.topology_hash.is_empty() {
        return Err(CheckOutcome::fail(
            ReasonCode::TopologyHashMissing,
            Severity::AdmitWithNotes,
            "topology hash empty".to_string(),
        ));
    }
    Ok(())
}

pub fn check_probe_freshness(
    descriptor: &CapabilityDescriptor,
    _manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    let now = chrono::Utc::now();
    let age = now.signed_duration_since(descriptor.probed_at);
    if age.num_seconds() > 86_400 {
        return Err(CheckOutcome::fail(
            ReasonCode::ProbeStale,
            Severity::AdmitWithNotes,
            format!("probe is {} seconds old (admit with note)", age.num_seconds()),
        ));
    }
    Ok(())
}

pub fn check_audio_io(
    _descriptor: &CapabilityDescriptor,
    _manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    Ok(())
}

pub fn check_storage_io(
    _descriptor: &CapabilityDescriptor,
    _manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    Ok(())
}

pub fn check_bandwidth_sufficient(
    _descriptor: &CapabilityDescriptor,
    _manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    Ok(())
}

pub fn check_input_devices(
    _descriptor: &CapabilityDescriptor,
    _manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    Ok(())
}

pub fn check_gpu_driver_present(
    _descriptor: &CapabilityDescriptor,
    _manifest: &BoundManifest,
) -> Result<(), CheckOutcome> {
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn outcome(
    code: ReasonCode,
    severity: Severity,
    msg: impl Into<String>,
) -> impl FnOnce() -> CheckOutcome {
    move || CheckOutcome::fail(code, severity, msg.into())
}
