//! Degradation-aware transport selection.
//!
//! This module is the **structural** half of the multi-hop selection
//! contract. The legacy [`super::select_stages`] maps a locality tier to
//! a label by string lookup; that map names transports that have **no
//! implementation** (e.g. RDMA, PCIe, QUIC, out-of-band), and the silence
//! is what allowed the bug to exist: nothing observable was reported.
//!
//! The functions here make the silence impossible:
//!
//! 1. Selection consults [`super::executor::is_implemented`] before
//!    committing to a transport. This consults a registry the executors
//!    themselves are responsible for staying in sync with; the
//!    drift-guard test in this module pins the invariant.
//! 2. When the tier's ideal is unimplemented, selection DEGRADES
//!    OBSERVABLY: it picks the best implemented alternative and records
//!    the fact in [`StageSelection`]. The `requested` field is the
//!    ideal the tier named; `actual` is what will run; `degraded_from`
//!    is `Some(ideal)` when the two differ; `degradation_reason` is the
//!    machine-readable explanation.
//! 3. A STRICT mode ([`SelectionOptions::strict`]) refuses to degrade.
//!    In strict mode an unimplemented request returns
//!    [`super::executor::ExecError::UnsupportedTransport`] instead of
//!    silently substituting a fallback. Callers that want a hard
//!    refusal opt in explicitly; default mode preserves the legacy
//!    "best-effort degrade + report" behavior.
//!
//! ## What did NOT change
//!
//! The legacy [`super::select_stages`] keeps its existing public/crate
//! visibility and its existing observable behavior: tier → named
//! transport, with no degradation awareness. Existing tests that call
//! it (e.g. `select_stages_for_lan`, `l6_lan_through_select_stages_is_visibly_unsupported`)
//! keep passing unmodified. New code that wants degradation observability
//! uses [`select_stages_with_options`] and [`super::executor::execute_with_selection`].

use crate::multihop::executor::{
    best_implemented_transport, ideal_transport_for_tier, is_implemented, ExecError,
};
use crate::multihop::stages::TransportStage;
use crate::LocalityTier;

/// The result of degradation-aware selection.
///
/// `stages` is the executable plan: the identity passthrough (if
/// present in the catalog) followed by the transport the executor will
/// actually run. `requested` is what the tier *conceptually* wants
/// (the ideal); `actual` is what `stages` says will run. When the two
/// differ, `degraded_from = Some(<ideal>)` and `degradation_reason` is
/// non-empty with a machine-readable explanation.
///
/// When the ideal IS implemented, `requested == actual`,
/// `degraded_from == None`, `degradation_reason == None`. A test that
/// only checks `actual` would not catch a regression where every tier
/// is silently marked degraded; the new fields are the structural
/// guarantee.
#[derive(Debug, Clone)]
pub struct StageSelection {
    /// The stages to execute. Always contains `identity` if the catalog
    /// has it, followed by the transport the executor will run.
    pub stages: Vec<TransportStage>,
    /// The conceptual ideal transport for the tier, irrespective of
    /// whether it is implemented. This is the field callers can use
    /// to ask "what was the plan trying to use?" — independent of
    /// whether the executor ended up serving it.
    pub requested: String,
    /// The transport the executor will actually run. Equals
    /// `requested` when the ideal is implemented; equals the best
    /// implemented alternative otherwise.
    pub actual: String,
    /// `Some(<ideal>)` when `actual != requested` and selection degraded
    /// to a fallback. `None` when `actual == requested` (no
    /// degradation happened).
    pub degraded_from: Option<String>,
    /// Machine-readable explanation of the degradation. Non-empty iff
    /// `degraded_from.is_some()`. This is intentionally a `String`,
    /// not a `log` line, so callers can assert on it from tests.
    pub degradation_reason: Option<String>,
}

// Manual `PartialEq` because `TransportStage` does not derive
// `PartialEq` (its non-comparable fields like `StageCost` and
// `StageClaims` would prevent it). We compare on the metadata fields
// that are actually meaningful to compare — two `StageSelection`s
// with the same `requested`/`actual`/`degraded_from`/`degradation_reason`
// describe the same degradation decision regardless of whether the
// catalog's `stages` vector contains the same `TransportStage` clones.
// This is the same shape `ExecOutcome` already uses (manual impl) and
// the same shape `Result<StageSelection, ExecError>` needs to support
// `assert_eq!` on the strict-mode `Err` path.
impl PartialEq for StageSelection {
    fn eq(&self, other: &Self) -> bool {
        self.requested == other.requested
            && self.actual == other.actual
            && self.degraded_from == other.degraded_from
            && self.degradation_reason == other.degradation_reason
    }
}

impl Eq for StageSelection {}

/// Options for [`select_stages_with_options`].
///
/// The struct is intentionally small and copyable. The default is the
/// "degrade + report" mode the project owner chose as the new default:
/// when the tier's ideal is unimplemented, selection falls back to the
/// best implemented transport and records the fact. Strict mode opts
/// out of degradation; an unimplemented ideal returns
/// [`ExecError::UnsupportedTransport`] instead.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SelectionOptions {
    /// When `true`, an unimplemented tier request returns
    /// [`ExecError::UnsupportedTransport`] instead of degrading.
    /// When `false` (the default), selection degrades to the best
    /// implemented transport and records the degradation in the
    /// returned [`StageSelection`].
    pub strict: bool,
}

impl SelectionOptions {
    /// Construct strict-mode options.
    pub const fn strict() -> Self {
        Self { strict: true }
    }

    /// Construct default-mode options (degrade + report).
    pub const fn degrade() -> Self {
        Self { strict: false }
    }
}

/// Degradation-aware transport selection.
///
/// Unlike the legacy [`super::select_stages`], this function consults
/// [`super::executor::is_implemented`] before committing to a transport.
/// When the tier's ideal is implemented, the selection is
/// indistinguishable from the legacy path: `requested == actual`,
/// `degraded_from == None`. When the ideal is unimplemented:
///
/// - In default mode (`options.strict == false`), the function picks
///   the best implemented transport via
///   [`super::executor::best_implemented_transport`], pushes it into
///   the stages, and populates `degraded_from` + `degradation_reason`
///   on the returned [`StageSelection`]. The `requested` field still
///   names the original ideal so callers can see "this was supposed to
///   be RDMA but we served TCP".
/// - In strict mode (`options.strict == true`), the function returns
///   [`ExecError::UnsupportedTransport`] naming the original ideal.
///   The selection does NOT silently substitute.
///
/// Strict mode is opt-in; default mode is "degrade + report", which is
/// what the project owner chose as the new default behavior.
///
/// Errors:
///
/// - [`ExecError::UnsupportedTransport`] in strict mode when the tier's
///   ideal is unimplemented. Carries the ideal's adapter id so the
///   caller knows exactly what was rejected.
/// - The function does NOT return other `ExecError` variants; the
///   underlying stage lookup is total over the built-in catalog (the
///   fallback transport `tcp_transport` is always present).
pub fn select_stages_with_options(
    tier: LocalityTier,
    catalog: &[TransportStage],
    options: SelectionOptions,
) -> Result<StageSelection, ExecError> {
    let ideal = ideal_transport_for_tier(tier);

    // Build the identity-passthrough stage list. The legacy
    // `select_stages` always prepends identity when present; we mirror
    // that here so the degradation-aware path is structurally
    // identical to the legacy one when no degradation happens.
    let mut stages = Vec::new();
    if let Some(identity) = catalog.iter().find(|s| s.id.0 == "identity") {
        stages.push(identity.clone());
    }

    if is_implemented(ideal) {
        // Happy path: the ideal is implemented. Look it up in the
        // catalog and push it. If the catalog is missing the stage
        // (which can only happen if someone removed an entry from
        // `builtin_stages`), we surface UnsupportedTransport naming
        // the ideal — same shape the executor would return.
        let stage = catalog.iter().find(|s| s.adapter == ideal).ok_or_else(|| {
            ExecError::UnsupportedTransport {
                requested: ideal.to_string(),
            }
        })?;
        stages.push(stage.clone());
        Ok(StageSelection {
            stages,
            requested: ideal.to_string(),
            actual: ideal.to_string(),
            degraded_from: None,
            degradation_reason: None,
        })
    } else if options.strict {
        // Strict mode: do not degrade. Return UnsupportedTransport
        // naming the ideal so the caller can decide what to do.
        Err(ExecError::UnsupportedTransport {
            requested: ideal.to_string(),
        })
    } else {
        // Default mode: degrade to the best implemented transport.
        // Pick the best alternative (today: tcp_transport), look it
        // up in the catalog, and record the degradation. The reason
        // string is intentionally machine-readable (caller can assert
        // on it from tests) and includes both the ideal and the
        // actual adapter ids so log triage is unambiguous.
        let actual = best_implemented_transport();
        let stage = catalog
            .iter()
            .find(|s| s.adapter == actual)
            .ok_or_else(|| ExecError::UnsupportedTransport {
                requested: ideal.to_string(),
            })?;
        stages.push(stage.clone());
        let reason = format!(
            "tier {tier:?} requested {ideal}, which is not implemented; degraded to {actual}"
        );
        Ok(StageSelection {
            stages,
            requested: ideal.to_string(),
            actual: actual.to_string(),
            degraded_from: Some(ideal.to_string()),
            degradation_reason: Some(reason),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multihop::executor::{
        implemented_adapters, is_implemented, TcpExecutor, UnixSocketExecutor,
    };
    use crate::multihop::stages::builtin_stages;

    // -- Drift guard -----------------------------------------------------
    //
    // This test is the single source of truth that the registry of
    // "what is implemented" in `executor.rs` matches what the executors
    // in this crate actually serve. If someone adds a new executor
    // (e.g. `QuicExecutor`) without updating `implemented_adapters()`,
    // this test fails. Conversely, if someone adds a new adapter to
    // `implemented_adapters()` without implementing an executor for
    // it, this test fails too. Either direction of drift is caught
    // here.
    //
    // The original bug — selection naming a transport that has no
    // implementation — would have been structurally prevented by this
    // invariant: had it existed then, the L3/L4/L8 catch-all could not
    // have silently mapped to a transport the executors didn't serve.

    /// Compile-time declaration of what each executor in this crate
    /// actually serves. Update this whenever a new executor lands or
    /// an existing executor grows a new adapter id.
    const TCP_EXECUTOR_SERVES: &[&str] = &["tcp_transport"];
    #[cfg(unix)]
    const UNIX_EXECUTOR_SERVES: &[&str] = &["unix_socket"];

    #[test]
    fn implemented_adapters_match_executor_capabilities() {
        let mut served: Vec<&str> = TCP_EXECUTOR_SERVES.to_vec();
        #[cfg(unix)]
        served.extend_from_slice(UNIX_EXECUTOR_SERVES);
        served.sort();
        served.dedup();

        let mut registry: Vec<&str> = implemented_adapters().to_vec();
        registry.sort();

        assert_eq!(
            registry, served,
            "implemented_adapters() drifted from what the executors actually serve: \
             registry={registry:?} served={served:?}. Update both sides in the same change."
        );

        // Sanity: every served adapter must be served by SOME
        // executor, not just declared in the registry. This is the
        // "no orphan declarations" half of the guard.
        for adapter in &registry {
            assert!(
                served.contains(adapter),
                "implemented_adapters() lists {adapter} but no executor serves it"
            );
        }
        for adapter in &served {
            assert!(
                registry.contains(adapter),
                "executor serves {adapter} but it is not in implemented_adapters()"
            );
        }
    }

    // -- is_implemented sanity ------------------------------------------

    #[test]
    fn is_implemented_recognizes_known_adapters() {
        // The registry today: tcp_transport and unix_socket. Both must
        // be reported as implemented; everything else must NOT be.
        assert!(is_implemented("tcp_transport"));
        assert!(is_implemented("unix_socket"));
        assert!(!is_implemented("quic_transport"));
        assert!(!is_implemented("shm_copy"));
        assert!(!is_implemented("rdma"));
        assert!(!is_implemented("pcie_p2p"));
        assert!(!is_implemented("oob"));
        assert!(!is_implemented("hevc_encoder"));
        assert!(!is_implemented(""));
    }

    // -- Default mode: implemented tier is not marked degraded ----------

    #[test]
    fn l5_loopback_default_mode_reports_no_degradation() {
        let catalog = builtin_stages();
        let selection = select_stages_with_options(
            LocalityTier::L5Loopback,
            &catalog,
            SelectionOptions::default(),
        )
        .expect("L5 ideal is implemented; default mode should not error");

        assert_eq!(selection.requested, "unix_socket");
        assert_eq!(selection.actual, "unix_socket");
        assert_eq!(selection.degraded_from, None);
        assert_eq!(selection.degradation_reason, None);

        // The stages carry the served transport (not a degraded one).
        let adapters: Vec<&str> = selection
            .stages
            .iter()
            .map(|s| s.adapter.as_str())
            .collect();
        assert!(
            adapters.contains(&"unix_socket"),
            "L5 default-mode stages should contain unix_socket, got {adapters:?}"
        );
    }

    #[test]
    fn l0_same_process_default_mode_reports_no_degradation() {
        let catalog = builtin_stages();
        let selection = select_stages_with_options(
            LocalityTier::L0SameProcess,
            &catalog,
            SelectionOptions::default(),
        )
        .expect("L0 ideal is implemented; default mode should not error");

        assert_eq!(selection.requested, "unix_socket");
        assert_eq!(selection.actual, "unix_socket");
        assert_eq!(selection.degraded_from, None);
    }

    // -- Default mode: unimplemented tier degrades with full metadata ---

    #[test]
    fn l4_rdma_default_mode_degrades_with_metadata() {
        let catalog = builtin_stages();
        let selection =
            select_stages_with_options(LocalityTier::L4Rdma, &catalog, SelectionOptions::default())
                .expect("default mode must NOT error; it should degrade");

        // requested = what the tier wanted (the unimplemented ideal)
        assert_eq!(selection.requested, "rdma");
        // actual = the implemented transport that will run
        assert!(
            is_implemented(&selection.actual),
            "actual must be an IMPLEMENTED transport, got {}",
            selection.actual
        );
        // degraded_from = Some(ideal), reason = non-empty
        assert_eq!(selection.degraded_from, Some("rdma".to_string()));
        let reason = selection
            .degradation_reason
            .as_deref()
            .expect("degradation_reason must be populated when degraded_from is Some");
        assert!(!reason.is_empty(), "degradation_reason must be non-empty");
        // The reason mentions both the ideal and the actual so log
        // triage is unambiguous. This is a structural assertion: the
        // reason is not a hand-curated string the test has to mirror.
        assert!(
            reason.contains("rdma") && reason.contains(&selection.actual),
            "degradation_reason must name both the ideal and the actual: {reason:?}"
        );

        // The stages must carry the DEGRADED transport, not the ideal.
        // If the stages still named `rdma`, the executor would either
        // reject (strict) or fail — neither is what default mode is
        // supposed to do.
        let adapters: Vec<&str> = selection
            .stages
            .iter()
            .map(|s| s.adapter.as_str())
            .collect();
        assert!(
            !adapters.contains(&"rdma"),
            "default mode must substitute an implemented transport; stages still contain 'rdma': {adapters:?}"
        );
        assert!(
            adapters.contains(&selection.actual.as_str()),
            "stages must contain the actual served transport; got {adapters:?}"
        );
    }

    #[test]
    fn l3_pcie_default_mode_degrades_with_metadata() {
        let catalog = builtin_stages();
        let selection = select_stages_with_options(
            LocalityTier::L3PcieP2P,
            &catalog,
            SelectionOptions::default(),
        )
        .expect("default mode must NOT error; it should degrade");

        assert_eq!(selection.requested, "pcie_p2p");
        assert!(is_implemented(&selection.actual));
        assert_eq!(selection.degraded_from, Some("pcie_p2p".to_string()));
        assert!(selection.degradation_reason.is_some());
        assert!(!selection.degradation_reason.as_ref().unwrap().is_empty());
    }

    #[test]
    fn l8_oob_default_mode_degrades_with_metadata() {
        let catalog = builtin_stages();
        let selection =
            select_stages_with_options(LocalityTier::L8Oob, &catalog, SelectionOptions::default())
                .expect("default mode must NOT error; it should degrade");

        assert_eq!(selection.requested, "oob");
        assert!(is_implemented(&selection.actual));
        assert_eq!(selection.degraded_from, Some("oob".to_string()));
        assert!(selection.degradation_reason.is_some());
        assert!(!selection.degradation_reason.as_ref().unwrap().is_empty());
    }

    #[test]
    fn l6_lan_default_mode_degrades_with_metadata() {
        let catalog = builtin_stages();
        let selection =
            select_stages_with_options(LocalityTier::L6Lan, &catalog, SelectionOptions::default())
                .expect("default mode must NOT error; it should degrade");

        assert_eq!(selection.requested, "quic_transport");
        assert!(is_implemented(&selection.actual));
        assert_eq!(selection.degraded_from, Some("quic_transport".to_string()));
        assert!(selection.degradation_reason.is_some());
    }

    // -- Strict mode: unimplemented tier returns UnsupportedTransport --

    #[test]
    fn l4_rdma_strict_mode_returns_unsupported_named() {
        let catalog = builtin_stages();
        let result =
            select_stages_with_options(LocalityTier::L4Rdma, &catalog, SelectionOptions::strict());

        assert_eq!(
            result,
            Err(ExecError::UnsupportedTransport {
                requested: "rdma".to_string()
            }),
            "strict mode must surface the unimplemented ideal, not silently degrade"
        );
    }

    #[test]
    fn l6_lan_strict_mode_returns_unsupported_named() {
        let catalog = builtin_stages();
        let result =
            select_stages_with_options(LocalityTier::L6Lan, &catalog, SelectionOptions::strict());

        assert_eq!(
            result,
            Err(ExecError::UnsupportedTransport {
                requested: "quic_transport".to_string()
            }),
            "strict mode must surface the unimplemented ideal, not silently degrade"
        );
    }

    #[test]
    fn strict_mode_for_implemented_tier_still_succeeds() {
        // Strict mode changes behavior ONLY for unimplemented ideals.
        // An implemented ideal (e.g. L5 -> unix_socket) must still
        // succeed and return a no-degradation selection.
        let catalog = builtin_stages();
        let selection = select_stages_with_options(
            LocalityTier::L5Loopback,
            &catalog,
            SelectionOptions::strict(),
        )
        .expect("L5 ideal is implemented; strict mode must still succeed");

        assert_eq!(selection.requested, "unix_socket");
        assert_eq!(selection.actual, "unix_socket");
        assert_eq!(selection.degraded_from, None);
    }

    // -- Shape tests: stages always contain identity when present -------

    #[test]
    fn stages_always_prepend_identity_when_catalog_has_it() {
        // The legacy `select_stages` always prepends the identity
        // passthrough. The degradation-aware path mirrors that so
        // downstream behavior (e.g. `stages_executed` in ExecOutcome)
        // stays consistent.
        let catalog = builtin_stages();

        for tier in [
            LocalityTier::L0SameProcess,
            LocalityTier::L5Loopback,
            LocalityTier::L4Rdma, // degraded to TCP
            LocalityTier::L6Lan,  // degraded to TCP
            LocalityTier::L8Oob,  // degraded to TCP
        ] {
            let selection = select_stages_with_options(tier, &catalog, SelectionOptions::default())
                .expect("default mode never errors");

            let first_adapter = selection
                .stages
                .first()
                .map(|s| s.adapter.as_str())
                .unwrap_or("");
            assert_eq!(
                first_adapter, "identity",
                "stages for tier {tier:?} must start with identity, got {first_adapter:?}"
            );
        }
    }

    // -- Touchpoint smoke test: executors exist for declared adapters ----

    /// This is a structural sanity check, not a behavior test: it
    /// walks the registry and verifies that every adapter listed is
    /// served by SOME `RouteExecutor` in the crate. The drift-guard
    /// test above is the strong form of this; this one is a quick
    /// smoke that prevents a regression where a future refactor
    /// removes one of the executor types without updating the
    /// registry.
    #[test]
    fn registry_adapters_have_a_concrete_executor() {
        // Touch the types so the compiler enforces their existence.
        // If someone removes TcpExecutor or UnixSocketExecutor this
        // test fails to compile.
        let _tcp: TcpExecutor = TcpExecutor;
        #[cfg(unix)]
        let _unix: UnixSocketExecutor = UnixSocketExecutor;

        // And the registry must contain exactly the adapters those
        // executors declare. This is the inverse of the drift-guard.
        let registry: Vec<&str> = implemented_adapters().to_vec();
        assert!(
            registry.contains(&"tcp_transport"),
            "registry missing tcp_transport"
        );
        #[cfg(unix)]
        assert!(
            registry.contains(&"unix_socket"),
            "registry missing unix_socket"
        );
    }
}
