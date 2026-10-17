//! Route execution seam for multi-hop route plans.
//!
//! This module is the keystone that connects the multihop **compiler**
//! (`compile_multihop`, which produces a `RoutePlan` + per-hop transport
//! stages) to the **runtime**, where something actually has to push bytes
//! across the wire.
//!
//! ## Why this exists (observable-vs-silent)
//!
//! Prior to this module, the multihop compiler produced plans that named
//! per-hop transports (`unix_socket`, `quic_transport`, `tcp_transport`,
//! …) but nothing executed them. The result is that a plan claiming
//! `quic_transport` for an L6 hop would, today, still cross the wire as
//! plain TCP — there is no `RouteExecutor` anywhere to do otherwise.
//! That silent degradation was structurally invisible: the caller had no
//! way to ask "did the transport the plan named actually run?".
//!
//! The seam introduced here makes that question *answerable*. `RouteExecutor`
//! implementations MUST name the transport that actually served the
//! request in `ExecOutcome::transport`, and an unimplemented transport
//! MUST surface as `ExecError::UnsupportedTransport { requested }` rather
//! than a silent substitution to a fallback.
//!
//! ## The honest statement
//!
//! `TcpExecutor` is the only fully-implemented executor today. It accepts
//! the `tcp_transport` adapter and reports `tcp_transport` as the
//! transport that served the request. It does **not** open a socket —
//! the seam is the deliverable, and the statement "today the transport
//! delta is provably zero" remains accurate. Bytes still cross the wire
//! the same way they did before this module existed; the difference is
//! that the caller can now *observe* that fact.

use super::stages::TransportStage;
use thiserror::Error;

/// Result of a successful route execution.
///
/// `transport` is the **actual** transport that served the request, not
/// the one the plan named. When the two differ, this struct is the
/// mechanism by which the difference becomes observable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecOutcome {
    /// Adapter id of the transport that actually served the request
    /// (e.g. `"tcp_transport"`).
    pub transport: String,
    /// Logical bytes "sent" through the seam. The seam does not open a
    /// socket; this is the payload length reported for observability.
    pub bytes_sent: usize,
    /// Stage ids visited, in the order they were dispatched.
    pub stages_executed: Vec<String>,
}

/// Errors raised by [`RouteExecutor::execute`].
///
/// The keystone enum is [`ExecError::UnsupportedTransport`]: when the
/// plan names a transport that the executor has no adapter for, the
/// error carries the requested name back to the caller. Silent fallback
/// is structurally impossible.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExecError {
    /// The plan (or the selected stages) requested a transport that
    /// this executor has no adapter for. The seam is explicit: `requested`
    /// is the adapter id the caller asked for.
    ///
    /// This includes `unix_socket`, `quic_transport`, `shm_copy`,
    /// encode/decode stages. (`identity` is treated as a no-op
    /// passthrough — see `TcpExecutor` for the rationale.)
    #[error("no transport adapter implemented for requested transport: {requested}")]
    UnsupportedTransport { requested: String },

    /// The caller handed the executor an empty stage list. This is a
    /// programmer error — the multihop compiler never produces this —
    /// but the seam guards against it explicitly rather than panicking.
    #[error("empty plan: no stages to execute")]
    EmptyPlan,
}

/// The execution contract for a multi-hop route plan.
///
/// Implementations are responsible for dispatching a payload through
/// the per-hop stages that the multihop compiler selected. The trait
/// is intentionally narrow: it accepts the compiled stages and a
/// payload, and returns either an [`ExecOutcome`] naming the transport
/// that actually ran or an [`ExecError`] that refuses to silently
/// substitute a fallback.
///
/// Implementations MUST:
/// - place the *actual* transport that served the request in
///   `ExecOutcome::transport`;
/// - return `UnsupportedTransport` for any stage type they cannot
///   handle, never silently fall back to a different transport.
pub trait RouteExecutor: Send + Sync {
    /// Execute the per-hop stages compiled by `select_stages`, carrying
    /// `payload`. Returns either an [`ExecOutcome`] or an [`ExecError`].
    fn execute(
        &self,
        stages: &[TransportStage],
        payload: &[u8],
    ) -> impl std::future::Future<Output = Result<ExecOutcome, ExecError>> + Send;
}

/// The only fully-implemented executor today.
///
/// Accepts the `tcp_transport` adapter and reports `tcp_transport` as
/// the transport that served the request. For every other stage — the
/// passthrough `identity` stage, `unix_socket`, `quic_transport`,
/// `shm_copy`, encode/decode stages — it returns
/// [`ExecError::UnsupportedTransport`] naming the requested adapter.
///
/// This implementation does not open a socket. The seam is the
/// deliverable; the byte-level behavior at the wire is unchanged.
/// Once a real adapter is implemented for another transport, a sibling
/// executor (e.g. `QuicExecutor`) becomes a thin shim over it.
#[derive(Debug, Default, Clone, Copy)]
pub struct TcpExecutor;

impl RouteExecutor for TcpExecutor {
    fn execute(
        &self,
        stages: &[TransportStage],
        payload: &[u8],
    ) -> impl std::future::Future<Output = Result<ExecOutcome, ExecError>> + Send {
        let outcome = if stages.is_empty() {
            Err(ExecError::EmptyPlan)
        } else {
            // The multihop compiler's `select_stages` always prepends
            // the `identity` passthrough stage before the transport
            // decision. Treat `identity` as a no-op (semantically it
            // is) so the L6 observation test surfaces `quic_transport`,
            // not `identity`.
            let transport = stages
                .iter()
                .rev()
                .find(|s| s.id.0 != "identity")
                .ok_or_else(|| ExecError::UnsupportedTransport {
                    requested: stages
                        .last()
                        .map(|s| s.id.0.clone())
                        .unwrap_or_else(|| "<empty>".to_string()),
                });

            match transport {
                Ok(stage) if stage.adapter == "tcp_transport" => Ok(ExecOutcome {
                    transport: stage.adapter.clone(),
                    bytes_sent: payload.len(),
                    stages_executed: stages.iter().map(|s| s.id.0.clone()).collect(),
                }),
                Ok(stage) => Err(ExecError::UnsupportedTransport {
                    requested: stage.adapter.clone(),
                }),
                Err(e) => Err(e),
            }
        };

        async move { outcome }
    }
}

#[cfg(test)]
mod tests {
    use super::super::select_stages;
    use super::*;
    use crate::multihop::stages::builtin_stages;
    use crate::LocalityTier;

    /// Look up a stage by id from the catalog. Panics if the catalog is
    /// missing the stage — this only happens if someone removes an entry
    /// from `builtin_stages`, which is a separate problem.
    fn stage_by_id<'a>(catalog: &'a [TransportStage], id: &str) -> &'a TransportStage {
        catalog
            .iter()
            .find(|s| s.id.0 == id)
            .unwrap_or_else(|| panic!("catalog missing stage id {id}"))
    }

    /// Build a synthetic 1-stage list to test the executor in isolation
    /// from `select_stages`. Useful for proving the explicit-error path
    /// without indirect reasoning through tier-to-transport mapping.
    fn one_stage(s: &TransportStage) -> Vec<TransportStage> {
        vec![s.clone()]
    }

    // -- TCP executor: success path ---------------------------------------

    #[tokio::test(flavor = "current_thread")]
    async fn tcp_plan_succeeds_and_reports_tcp_transport() {
        let catalog = builtin_stages();
        let tcp = stage_by_id(&catalog, "tcp_transport");
        let stages = one_stage(tcp);

        let exec = TcpExecutor;
        let outcome = exec
            .execute(&stages, b"hello-world")
            .await
            .expect("tcp_transport should succeed");

        assert_eq!(outcome.transport, "tcp_transport");
        assert_eq!(outcome.bytes_sent, 11);
        assert_eq!(outcome.stages_executed, vec!["tcp_transport".to_string()]);
    }

    // -- TCP executor: explicit-error path for non-tcp stages ------------

    #[tokio::test(flavor = "current_thread")]
    async fn quic_transport_returns_unsupported_named() {
        let catalog = builtin_stages();
        let quic = stage_by_id(&catalog, "quic_transport");
        let stages = one_stage(quic);

        let exec = TcpExecutor;
        let err = exec
            .execute(&stages, b"x")
            .await
            .expect_err("quic_transport must NOT silently succeed");

        assert_eq!(
            err,
            ExecError::UnsupportedTransport {
                requested: "quic_transport".to_string(),
            }
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn unix_socket_returns_unsupported_named() {
        let catalog = builtin_stages();
        let unix = stage_by_id(&catalog, "unix_socket");
        let stages = one_stage(unix);

        let exec = TcpExecutor;
        let err = exec
            .execute(&stages, b"x")
            .await
            .expect_err("unix_socket must NOT silently succeed");

        assert_eq!(
            err,
            ExecError::UnsupportedTransport {
                requested: "unix_socket".to_string(),
            }
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn shm_copy_is_unsupported_named() {
        let catalog = builtin_stages();
        let shm = stage_by_id(&catalog, "shm_copy");
        let stages = one_stage(shm);

        let exec = TcpExecutor;
        let err = exec
            .execute(&stages, b"x")
            .await
            .expect_err("shm_copy must NOT silently succeed");

        assert_eq!(
            err,
            ExecError::UnsupportedTransport {
                requested: "shm_copy".to_string(),
            }
        );
    }

    // -- L6: observable-vs-silent ----------------------------------------

    #[tokio::test(flavor = "current_thread")]
    async fn l6_lan_through_select_stages_is_visibly_unsupported() {
        // select_stages(L6Lan) currently returns [identity, quic_transport].
        // The seam must surface the L6 path as visibly NOT-TCP, not as a
        // silent success.
        let catalog = builtin_stages();
        let stages = select_stages(LocalityTier::L6Lan, &catalog);

        let ids: Vec<String> = stages.iter().map(|s| s.id.0.clone()).collect();
        assert_eq!(
            ids,
            vec!["identity".to_string(), "quic_transport".to_string()],
            "select_stages(L6Lan) shape changed; review the L6 observability contract",
        );

        let exec = TcpExecutor;
        let err = exec
            .execute(&stages, b"x")
            .await
            .expect_err("L6 must NOT silently succeed");

        assert_eq!(
            err,
            ExecError::UnsupportedTransport {
                requested: "quic_transport".to_string(),
            },
            "the L6 path must surface as quic_transport, not identity or tcp_transport",
        );
    }

    // -- L3/L4/L8: pins the catch-all to TCP, observable -----------------

    /// The `_ => "tcp_transport"` catch-all in `select_stages` for
    /// L3/L4/L8 is a known design choice the project owner has been
    /// asked to revisit. This test pins the *current* observable
    /// behavior so anyone changing the catch-all can see the impact on
    /// this seam: today L3/L4/L8 are honest TCP plans that the executor
    /// runs and reports as `tcp_transport`.
    #[tokio::test(flavor = "current_thread")]
    async fn l3_l4_l8_observe_tcp_catch_all() {
        let catalog = builtin_stages();
        let exec = TcpExecutor;
        let payload = b"frame";

        for tier in [
            LocalityTier::L3PcieP2P,
            LocalityTier::L4Rdma,
            LocalityTier::L8Oob,
        ] {
            let stages = select_stages(tier, &catalog);
            let ids: Vec<String> = stages.iter().map(|s| s.id.0.clone()).collect();
            assert_eq!(
                ids.last().map(String::as_str),
                Some("tcp_transport"),
                "select_stages({tier:?}) currently maps to tcp_transport — \
                 if this changes, review the L3/L4/L8 observability contract",
            );

            let outcome = exec
                .execute(&stages, payload)
                .await
                .expect("L3/L4/L8 currently goes through TCP and the executor reports it");

            assert_eq!(
                outcome.transport, "tcp_transport",
                "L3/L4/L8 tier {tier:?} must surface as tcp_transport, \
                 not falsely claim any other transport",
            );
            assert_eq!(outcome.bytes_sent, payload.len());
        }
    }

    // -- Identity alone + empty plan: edge cases -------------------------

    /// An `[identity]`-only plan has no transport decision after the
    /// passthrough. The seam must not silently succeed — there is no
    /// transport here. (The multihop compiler never emits this shape;
    /// the test guards the contract.)
    #[tokio::test(flavor = "current_thread")]
    async fn identity_alone_returns_unsupported_named() {
        let catalog = builtin_stages();
        let identity = stage_by_id(&catalog, "identity");
        let stages = vec![identity.clone()];

        let exec = TcpExecutor;
        let err = exec
            .execute(&stages, b"x")
            .await
            .expect_err("identity-only plan must NOT silently succeed");

        assert_eq!(
            err,
            ExecError::UnsupportedTransport {
                requested: "identity".to_string(),
            }
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn empty_plan_returns_empty_plan_error() {
        let exec = TcpExecutor;
        let err = exec
            .execute(&[], b"x")
            .await
            .expect_err("empty stage list must surface as EmptyPlan");
        assert_eq!(err, ExecError::EmptyPlan);
    }
}
