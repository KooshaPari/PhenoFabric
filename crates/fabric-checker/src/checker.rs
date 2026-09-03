//! Top-level `Checker` — composes all individual check functions into a
//! single `Decision`. Order of composition matters: hard requirements
//! (Severity::Reject) short-circuit; soft warnings (Severity::AdmitWithNotes)
//! accumulate without failing the decision.

use crate::decision::{CheckOutcome, Decision, ReasonCode, Severity};
use crate::checks;

use fabric_capability::descriptor::CapabilityDescriptor;
use phenotype_nvms_adapter::BoundManifest;

/// Runs the full check suite against a host descriptor and a manifest.
///
/// Returns a `Decision` with `Admit` if all reject-level checks pass,
/// `AdmitWithNotes` if only notes-level checks failed, or `Reject` if any
/// reject-level check failed.
pub fn check(
    descriptor: &CapabilityDescriptor,
    manifest: &BoundManifest,
) -> Decision {
    let outcomes = run_all(descriptor, manifest);
    collapse(outcomes)
}

/// Public for testing — runs every check and returns the raw outcomes.
pub fn run_all(
    descriptor: &CapabilityDescriptor,
    manifest: &BoundManifest,
) -> Vec<CheckOutcome> {
    let fns: Vec<fn(&CapabilityDescriptor, &BoundManifest) -> Result<(), CheckOutcome>> = vec![
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
    fns
        .into_iter()
        .map(|f| f(descriptor, manifest))
        .filter_map(|r| r.err())
        .collect()
}

/// Collapse a list of `CheckOutcome` failures into a top-level `Decision`.
/// Any `Reject` severity → `Decision::Reject`. Otherwise, if any
/// `AdmitWithNotes` → `Decision::AdmitWithNotes`. Otherwise `Decision::Admit`.
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
    use fabric_capability::descriptor::CapabilityDescriptor;

    fn empty_descriptor() -> CapabilityDescriptor {
        CapabilityDescriptor::default()
    }

    fn empty_manifest() -> BoundManifest {
        BoundManifest {
            required: phenotype_nvms_adapter::RequiredCapabilities::default(),
            provenance: None,
        }
    }

    #[test]
    fn empty_outcomes_admit() {
        let d = collapse(vec![]);
        assert_eq!(d, Decision::Admit);
    }

    #[test]
    fn notes_only_admit_with_notes() {
        let d = collapse(vec![CheckOutcome::fail(
            ReasonCode::SignatureMissing,
            Severity::AdmitWithNotes,
            "no sigs".to_string(),
        )]);
        assert!(matches!(d, Decision::AdmitWithNotes { .. }));
    }

    #[test]
    fn reject_short_circuits() {
        let d = collapse(vec![
            CheckOutcome::fail(
                ReasonCode::SignatureMissing,
                Severity::AdmitWithNotes,
                "no sigs".to_string(),
            ),
            CheckOutcome::fail(
                ReasonCode::MemoryInsufficient,
                Severity::Reject,
                "out of memory".to_string(),
            ),
        ]);
        assert!(matches!(d, Decision::Reject { .. }));
    }

    #[test]
    fn run_all_on_empty_inputs_produces_only_notes() {
        // The default descriptor has epoch=0 and no signatures, so
        // check_signature_valid and check_epoch_current both produce notes.
        // All other checks should pass on empty inputs.
        let descriptor = empty_descriptor();
        let manifest = empty_manifest();
        let outcomes = run_all(&descriptor, &manifest);
        // Should have notes but no rejects.
        let d = collapse(outcomes);
        assert!(matches!(d, Decision::AdmitWithNotes { .. }));
    }
}
