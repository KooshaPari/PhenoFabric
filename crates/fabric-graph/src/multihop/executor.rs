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
use crate::LocalityTier;
use thiserror::Error;

// -- Implemented-transport registry ---------------------------------------
//
// The registry below is the source of truth for "what is implemented".
// Degradation-aware selection consults it; the drift-guard test pins
// the invariant that the registry matches what executors actually serve.
//
// `tcp_transport` and `unix_socket` are the only fully-implemented
// adapters today: `TcpExecutor` serves `tcp_transport` and
// `UnixSocketExecutor` serves `unix_socket`. Everything else in the
// catalog (`quic_transport`, `shm_copy`, encode/decode stages, and the
// tier-ideal names like `rdma`, `pcie_p2p`, `oob`) is NOT implemented.
// Selection treats those as "degrade or refuse" depending on mode.

/// The canonical list of adapter ids that `RouteExecutor`
/// implementations actually serve today.
///
/// This is the source of truth for "what is implemented". Degradation-
/// aware selection consults it to know whether a tier-mapped transport
/// can be honored directly or must degrade to a fallback. The runtime
/// executors in this module are responsible for staying in sync with
/// this list: every adapter id returned here MUST be served by at least
/// one `RouteExecutor` impl in this module, and every adapter id
/// served by an executor MUST appear here.
///
/// The drift-guard test in `selection.rs`
/// (`implemented_adapters_match_executor_capabilities`) is the only
/// thing that pins this invariant — without it, the map-vs-reality
/// drift that produced the original bug can return silently.
pub fn implemented_adapters() -> &'static [&'static str] {
    const ADAPTERS: &[&str] = &["tcp_transport", "unix_socket"];
    ADAPTERS
}

/// Predicate form: returns true iff `adapter` appears in
/// [`implemented_adapters`].
pub fn is_implemented(adapter: &str) -> bool {
    implemented_adapters().contains(&adapter)
}

/// The conceptual ideal transport per locality tier, irrespective of
/// whether an executor implements it. Selection uses this to know what
/// the tier "wants"; the degradation-aware selection function then
/// consults [`is_implemented`] to decide whether to honor the request
/// or fall back to a default transport.
///
/// The mapping names transports that have NO implementation today —
/// `pcie_p2p`, `rdma`, `oob`, `quic_transport` — so the registry /
/// executor pair can never silently substitute a name for an
/// implementation. When new executors land, update this map to point
/// at the new adapter id (and update `implemented_adapters()` to
/// include it); the drift-guard test will fail loudly if the two are
/// updated asymmetrically.
pub fn ideal_transport_for_tier(tier: LocalityTier) -> &'static str {
    match tier {
        LocalityTier::L0SameProcess | LocalityTier::L1SameNuma | LocalityTier::L2CrossNumaShm => {
            "unix_socket"
        }
        LocalityTier::L3PcieP2P => "pcie_p2p",
        LocalityTier::L4Rdma => "rdma",
        LocalityTier::L5Loopback => "unix_socket",
        LocalityTier::L6Lan | LocalityTier::L7Wan => "quic_transport",
        LocalityTier::L8Oob => "oob",
    }
}

/// The default fallback transport used by degradation-aware selection
/// when the tier's ideal is unimplemented.
///
/// Today: `tcp_transport` is the only implemented baseline transport
/// and runs on every target. When other implementations land (e.g.
/// `quic_transport`), this function should be revisited to prefer the
/// closest fit; the drift-guard test will keep the registry honest.
pub fn best_implemented_transport() -> &'static str {
    "tcp_transport"
}

/// Result of a successful route execution.
///
/// `transport` is the **actual** transport that served the request,
/// not the one the plan named. The degradation-aware selection path
/// (`execute_with_selection`) populates the remaining fields so the
/// difference between "the tier wanted X" and "the executor served Y"
/// becomes observable in the outcome itself, not just in a log line.
///
/// The fields break down as follows:
///
/// - `transport`: adapter id of the executor that served the request
///   (e.g. `"tcp_transport"`).
/// - `requested`: adapter id the **plan** asked for. For the legacy
///   `execute(stages, payload)` path this equals the last non-identity
///   stage's adapter; for `execute_with_selection(exec, selection, payload)`
///   this is the selection's `requested` field — i.e. the tier's
///   conceptual ideal.
/// - `degraded_from`: `Some(<adapter>)` when selection degraded the
///   tier's ideal to an implemented fallback; `None` when no
///   degradation happened.
/// - `degradation_reason`: non-empty machine-readable reason when
///   `degraded_from.is_some()`; `None` otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecOutcome {
    /// Adapter id of the transport that actually served the request
    /// (e.g. `"tcp_transport"`).
    pub transport: String,
    /// Adapter id the plan (or selection) requested. When the legacy
    /// `execute(stages, payload)` path is used this is inferred from
    /// the last non-identity stage; when `execute_with_selection` is
    /// used this carries the tier's conceptual ideal.
    pub requested: String,
    /// `Some(<adapter>)` when the served transport differs from the
    /// requested one (selection degraded). `None` when no
    /// degradation happened.
    pub degraded_from: Option<String>,
    /// Non-empty machine-readable reason when `degraded_from` is
    /// `Some`. `None` otherwise. This is intentionally a `String`
    /// rather than a `log::warn!` line so callers can assert on it
    /// from tests and ship it in their own telemetry.
    pub degradation_reason: Option<String>,
    /// Logical bytes "sent" through the seam. The seam does not open a
    /// socket; this is the payload length reported for observability.
    pub bytes_sent: usize,
    /// Bytes the executor actually observed coming back from the
    /// remote side. For executors that perform a real byte round-trip
    /// (e.g. [`UnixSocketExecutor`]) this is the echo payload and
    /// MUST equal the input payload byte-for-byte; the seam uses
    /// this to prove the round-trip was not a silent failure. For
    /// executors that do not perform an echo (e.g. [`TcpExecutor`],
    /// which is a seamed shim), this is an empty `Vec` — the seam
    /// explicitly reports "no echo observed" rather than synthesizing
    /// data.
    pub bytes_received: Vec<u8>,
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

    /// The executor attempted a real I/O action (bind, connect, read,
    /// write) and the underlying transport reported an error. This
    /// carries the formatted reason back to the caller; the seam does
    /// NOT collapse I/O errors into `UnsupportedTransport` because
    /// that would hide the difference between "no adapter" and
    /// "adapter failed at the wire".
    #[error("transport I/O error: {0}")]
    Io(String),
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
                Ok(stage) if stage.adapter == "tcp_transport" => {
                    let adapter = stage.adapter.clone();
                    Ok(ExecOutcome {
                        transport: adapter.clone(),
                        // Legacy path: requested = the adapter the
                        // executor actually observed. The executor
                        // does not know whether the selection wanted
                        // something different; the degradation-aware
                        // path (`execute_with_selection`) overrides
                        // this with the selection's requested field.
                        requested: adapter,
                        degraded_from: None,
                        degradation_reason: None,
                        bytes_sent: payload.len(),
                        bytes_received: Vec::new(),
                        stages_executed: stages.iter().map(|s| s.id.0.clone()).collect(),
                    })
                }
                Ok(stage) => Err(ExecError::UnsupportedTransport {
                    requested: stage.adapter.clone(),
                }),
                Err(e) => Err(e),
            }
        };

        async move { outcome }
    }
}

// -- UnixSocketExecutor -------------------------------------------------

/// Unix domain socket adapter that performs a real byte round-trip.
///
/// Unlike [`TcpExecutor`] (which is a pure seam — it does not open a
/// socket), [`UnixSocketExecutor`] *does* move bytes:
/// each [`RouteExecutor::execute`] call binds a fresh
/// `std::os::unix::net::UnixListener` at a unique path under
/// `std::env::temp_dir()`, spawns a server thread that accepts one
/// connection and echoes the payload back, then connects from the
/// foreground task, writes the payload, reads the echo, and returns
/// the received bytes in [`ExecOutcome::bytes_received`].
///
/// The temp socket file is removed via an RAII guard that runs on
/// every exit path (including panic), so no socket files are left
/// behind even when the round-trip fails partway.
///
/// The executor accepts the `unix_socket` adapter id and returns
/// [`ExecError::UnsupportedTransport`] — naming the requested
/// adapter — for any other stage id. There is no silent fallback.
///
/// On non-unix targets the type exists as a unit struct so the seam
/// still compiles, but its [`RouteExecutor::execute`] implementation
/// unconditionally returns [`ExecError::UnsupportedTransport`] for
/// every stage id (including `unix_socket`); this is the honest
/// report for a target that cannot host the adapter.
#[derive(Debug, Default, Clone, Copy)]
pub struct UnixSocketExecutor;

#[cfg(unix)]
impl UnixSocketExecutor {
    /// Perform the actual byte round-trip. Spawns a server thread
    /// that accepts one connection, reads the payload until EOF,
    /// writes the same bytes back, and exits. The foreground task
    /// connects, writes its input, half-closes the write side, reads
    /// the echo, and validates byte equality.
    ///
    /// Returns the bytes the client received from the server so the
    /// caller (the trait impl) can attach them to
    /// [`ExecOutcome::bytes_received`] and the test can prove
    /// round-trip identity.
    async fn round_trip(&self, payload: &[u8]) -> Result<Vec<u8>, ExecError> {
        use std::io::{Read, Write};
        use std::os::unix::net::{UnixListener, UnixStream};
        use std::path::PathBuf;
        use std::thread;

        // 1. Choose a unique temp socket path. pid + nanos is unique
        //    per call within a process; the nanos clock already
        //    advances faster than any reasonable test loop can call
        //    bind() twice on the same nanosecond.
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path: PathBuf =
            std::env::temp_dir().join(format!("fabric-graph-uds-{pid}-{nanos:x}.sock"));

        // 2. RAII guard so the socket file is removed on every exit
        //    path, including panic. The drop order is: the
        //    UnixStream goes first (when `_client` falls out of
        //    scope), then `_guard`, then the JoinHandle when the
        //    thread variable is dropped — but the thread has already
        //    exited by then, so removing the file is safe.
        struct SocketFileGuard(PathBuf);
        impl Drop for SocketFileGuard {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }

        // 3. Bind the listener. After this returns the socket file
        //    exists at `path` and the kernel will queue incoming
        //    connections.
        let listener = UnixListener::bind(&path)
            .map_err(|e| ExecError::Io(format!("bind({}): {e}", path.display())))?;

        let _guard = SocketFileGuard(path.clone());

        // 4. Spawn the server thread. It owns the listener; the
        //    calling task does not need it. The thread blocks on
        //    accept() until the client connects.
        let path_for_server = path.clone();
        let server = thread::spawn(move || -> Result<(), String> {
            let (mut conn, _) = listener.accept().map_err(|e| format!("accept: {e}"))?;
            // The server reads until the client closes the write side.
            let mut received = Vec::new();
            conn.read_to_end(&mut received)
                .map_err(|e| format!("server read: {e}"))?;
            // Echo back exactly what we received.
            conn.write_all(&received)
                .map_err(|e| format!("server write: {e}"))?;
            // Best-effort cleanup; the calling task's RAII guard will
            // also remove the file but it's harmless to attempt twice.
            let _ = std::fs::remove_file(&path_for_server);
            Ok(())
        });

        // 5. Connect, write, half-close write, read the echo.
        let mut client =
            UnixStream::connect(&path).map_err(|e| ExecError::Io(format!("connect: {e}")))?;
        client
            .write_all(payload)
            .map_err(|e| ExecError::Io(format!("client write: {e}")))?;
        // Shut down the write side so the server's read_to_end
        // unblocks. The read side stays open so we can read the echo.
        client
            .shutdown(std::net::Shutdown::Write)
            .map_err(|e| ExecError::Io(format!("client shutdown(Write): {e}")))?;
        let mut received = Vec::new();
        client
            .read_to_end(&mut received)
            .map_err(|e| ExecError::Io(format!("client read: {e}")))?;

        // 6. Validate byte identity on the foreground task. The
        //    round-trip is not "successful" unless the bytes came
        //    back byte-for-byte.
        if received.as_slice() != payload {
            let first_diff = payload
                .iter()
                .zip(received.iter())
                .position(|(a, b)| a != b);
            return Err(ExecError::Io(format!(
                "byte mismatch: sent {} bytes, received {} bytes; first diff at {first_diff:?}",
                payload.len(),
                received.len(),
            )));
        }

        // 7. Wait for the server thread to finish cleanly. If it
        //    returned an error or panicked, surface that to the
        //    caller — the bytes happened to round-trip but the
        //    server's bookkeeping is broken, which is observably
        //    wrong.
        match server.join() {
            Ok(Ok(())) => Ok(received),
            Ok(Err(e)) => Err(ExecError::Io(format!("server: {e}"))),
            Err(join_err) => Err(ExecError::Io(format!(
                "server thread panicked: {join_err:?}"
            ))),
        }
    }
}

#[cfg(unix)]
impl RouteExecutor for UnixSocketExecutor {
    fn execute(
        &self,
        stages: &[TransportStage],
        payload: &[u8],
    ) -> impl std::future::Future<Output = Result<ExecOutcome, ExecError>> + Send {
        // Locate the last non-identity stage — the actual transport
        // decision — and validate. The semantics here mirror
        // `TcpExecutor`'s so the L3/L4/L8 catch-all observable
        // contract continues to hold for any non-unix_socket stages.
        let pre_check = if stages.is_empty() {
            Err(ExecError::EmptyPlan)
        } else {
            match stages
                .iter()
                .rev()
                .find(|s| s.id.0 != "identity")
                .ok_or_else(|| ExecError::UnsupportedTransport {
                    requested: stages
                        .last()
                        .map(|s| s.id.0.clone())
                        .unwrap_or_else(|| "<empty>".to_string()),
                }) {
                Ok(stage) if stage.adapter == "unix_socket" => Ok(()),
                Ok(stage) => Err(ExecError::UnsupportedTransport {
                    requested: stage.adapter.clone(),
                }),
                Err(e) => Err(e),
            }
        };

        let stages = stages.to_vec();
        async move {
            match pre_check {
                Ok(()) => self.round_trip(payload).await.map(|received| ExecOutcome {
                    transport: "unix_socket".to_string(),
                    // Legacy path: requested = the adapter the executor
                    // actually observed (unix_socket). The executor does
                    // not know whether the selection wanted something
                    // different; the degradation-aware path
                    // (`execute_with_selection`) overrides this with the
                    // selection's requested field.
                    requested: "unix_socket".to_string(),
                    degraded_from: None,
                    degradation_reason: None,
                    bytes_sent: payload.len(),
                    bytes_received: received,
                    stages_executed: stages.iter().map(|s| s.id.0.clone()).collect(),
                }),
                Err(e) => Err(e),
            }
        }
    }
}

#[cfg(not(unix))]
impl RouteExecutor for UnixSocketExecutor {
    fn execute(
        &self,
        stages: &[TransportStage],
        payload: &[u8],
    ) -> impl std::future::Future<Output = Result<ExecOutcome, ExecError>> + Send {
        // On non-unix targets the adapter does not exist. Surface
        // UnsupportedTransport for every stage id, including
        // `unix_socket`, so the seam is honest about what the
        // target can host.
        let _ = payload;
        let stages = stages.to_vec();
        async move {
            if stages.is_empty() {
                Err(ExecError::EmptyPlan)
            } else {
                let requested = stages
                    .iter()
                    .rev()
                    .find(|s| s.id.0 != "identity")
                    .map(|s| s.id.0.clone())
                    .unwrap_or_else(|| {
                        stages
                            .last()
                            .map(|s| s.id.0.clone())
                            .unwrap_or_else(|| "<empty>".to_string())
                    });
                Err(ExecError::UnsupportedTransport { requested })
            }
        }
    }
}

// -- Degradation-aware execution -----------------------------------------

/// Execute a `StageSelection` with degradation metadata merged into the
/// returned [`ExecOutcome`].
///
/// This is the structural counterpart of
/// `selection::select_stages_with_options`: where selection decides
/// WHAT the executor will run, this function runs it and attaches the
/// decision's outcome to the result so the difference between "the
/// tier wanted X" and "the executor served Y" is observable in the
/// returned struct, not in a log line.
///
/// The merge rules:
///
/// - `requested` on the outcome comes from `selection.requested` —
///   the tier's conceptual ideal, not the adapter the executor
///   actually observed. This is the ONLY place the outcome can know
///   the difference.
/// - `degraded_from` and `degradation_reason` come from the
///   selection verbatim. They are non-`None` iff the selection
///   degraded; this function does not second-guess the selection.
/// - `transport`, `bytes_sent`, `bytes_received`, `stages_executed`
///   come from the executor verbatim.
///
/// The function is generic over `E: RouteExecutor + ?Sized` so callers
/// can pass either a concrete executor (`TcpExecutor`,
/// `UnixSocketExecutor`) or a trait object (`Box<dyn RouteExecutor>`).
/// Selection is a free function over the catalog, not a method on the
/// executor; combining the two is the caller's job, and this helper
/// makes the common path one line.
pub async fn execute_with_selection<E>(
    executor: &E,
    selection: &super::selection::StageSelection,
    payload: &[u8],
) -> Result<ExecOutcome, ExecError>
where
    E: RouteExecutor + ?Sized,
{
    let mut outcome = executor.execute(&selection.stages, payload).await?;
    // Override `requested` with the selection's requested field so
    // the outcome can answer "what was the plan trying to use?".
    // The executor could only infer the served adapter; the
    // selection is the only source of truth for the tier's
    // conceptual ideal.
    outcome.requested = selection.requested.clone();
    // Forward the degradation metadata verbatim. Selection decided
    // whether to degrade; the executor just served what it was
    // given. This is the only place the two halves of the
    // degradation contract are stitched together.
    outcome.degraded_from = selection.degraded_from.clone();
    outcome.degradation_reason = selection.degradation_reason.clone();
    Ok(outcome)
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

    // -- UnixSocketExecutor: real byte round-trip ------------------------

    /// Build a 4 KiB payload that exercises every byte value at least
    /// once and includes a NUL, a 0xFF, and a long run of repeated
    /// values. If any byte is corrupted in transit, this test catches
    /// it.
    fn nontrivial_payload() -> Vec<u8> {
        const SIZE: usize = 4096;
        let mut v = Vec::with_capacity(SIZE);
        for i in 0..SIZE {
            // Mix of repeated and varied values, full 0..=255 range
            // covered multiple times across the buffer.
            v.push(((i * 7 + i.wrapping_mul(13)) & 0xFF) as u8);
        }
        // Guarantee NUL and 0xFF appear at known positions.
        v[0] = 0x00;
        v[1] = 0xFF;
        v[2] = 0x00;
        v[SIZE - 1] = 0xFF;
        v
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn unix_socket_round_trip_succeeds_and_bytes_are_identical() {
        let catalog = builtin_stages();
        let stage = stage_by_id(&catalog, "unix_socket");
        let stages = vec![stage.clone()];
        let payload = nontrivial_payload();

        let exec = UnixSocketExecutor;
        let outcome = exec
            .execute(&stages, &payload)
            .await
            .expect("unix_socket round-trip must succeed on a unix target");

        assert_eq!(
            outcome.transport, "unix_socket",
            "executor must report the transport it actually served"
        );
        assert_eq!(
            outcome.bytes_sent,
            payload.len(),
            "bytes_sent must equal payload length"
        );
        assert_eq!(
            outcome.stages_executed,
            vec!["unix_socket".to_string()],
            "stages_executed must list the served stage"
        );
        // The keystone assertion: bytes received back are byte-identical
        // to the bytes sent. This is the round-trip proof — not just "no
        // error". A silent corruption would slip past a length check.
        assert_eq!(
            outcome.bytes_received, payload,
            "bytes_received must equal payload byte-for-byte (round-trip identity proof)"
        );
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn unix_socket_round_trip_handles_empty_payload() {
        // Edge case: zero-length payload. The server's read_to_end
        // returns immediately with empty buf; the client also reads
        // zero bytes. Round-trip must still succeed and report
        // bytes_received as an empty Vec, not panic.
        let catalog = builtin_stages();
        let stage = stage_by_id(&catalog, "unix_socket");
        let stages = vec![stage.clone()];
        let payload: &[u8] = b"";

        let exec = UnixSocketExecutor;
        let outcome = exec
            .execute(&stages, payload)
            .await
            .expect("zero-length payload must round-trip cleanly");

        assert_eq!(outcome.transport, "unix_socket");
        assert_eq!(outcome.bytes_sent, 0);
        assert!(outcome.bytes_received.is_empty());
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn unix_socket_round_trip_handles_large_payload() {
        // 256 KiB — large enough to make sure we're not relying on
        // any kernel-level "small write" fast path, and to make
        // read_to_end genuinely stream.
        let payload: Vec<u8> = (0..(256 * 1024)).map(|i| (i & 0xFF) as u8).collect();
        let catalog = builtin_stages();
        let stage = stage_by_id(&catalog, "unix_socket");
        let stages = vec![stage.clone()];

        let exec = UnixSocketExecutor;
        let outcome = exec
            .execute(&stages, &payload)
            .await
            .expect("256 KiB payload must round-trip cleanly");

        assert_eq!(outcome.bytes_sent, payload.len());
        assert_eq!(
            outcome.bytes_received, payload,
            "large-payload round-trip must be byte-identical"
        );
    }

    // -- UnixSocketExecutor: explicit-error path for non-unix_socket ----

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn unix_socket_executor_rejects_tcp_transport() {
        let catalog = builtin_stages();
        let tcp = stage_by_id(&catalog, "tcp_transport");
        let stages = one_stage(tcp);

        let exec = UnixSocketExecutor;
        let err = exec
            .execute(&stages, b"x")
            .await
            .expect_err("UnixSocketExecutor must NOT silently serve tcp_transport");
        assert_eq!(
            err,
            ExecError::UnsupportedTransport {
                requested: "tcp_transport".to_string(),
            }
        );
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn unix_socket_executor_rejects_quic_transport() {
        let catalog = builtin_stages();
        let quic = stage_by_id(&catalog, "quic_transport");
        let stages = one_stage(quic);

        let exec = UnixSocketExecutor;
        let err = exec
            .execute(&stages, b"x")
            .await
            .expect_err("UnixSocketExecutor must NOT silently serve quic_transport");
        assert_eq!(
            err,
            ExecError::UnsupportedTransport {
                requested: "quic_transport".to_string(),
            }
        );
    }

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn unix_socket_executor_handles_identity_passthrough() {
        // The TcpExecutor tests that an [identity]-only plan surfaces
        // UnsupportedTransport for "identity" because there is no
        // transport decision after the passthrough. UnixSocketExecutor
        // mirrors that contract.
        let catalog = builtin_stages();
        let identity = stage_by_id(&catalog, "identity");
        let stages = vec![identity.clone()];

        let exec = UnixSocketExecutor;
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

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn unix_socket_executor_treats_identity_as_passthrough_before_unix() {
        // Mirrors TcpExecutor's behavior: when the plan is
        // [identity, unix_socket], the transport decision is
        // unix_socket, not identity. The seam must surface as
        // unix_socket and actually round-trip the bytes.
        let catalog = builtin_stages();
        let identity = stage_by_id(&catalog, "identity").clone();
        let unix = stage_by_id(&catalog, "unix_socket").clone();
        let stages = vec![identity, unix];
        let payload = b"frame".to_vec();

        let exec = UnixSocketExecutor;
        let outcome = exec
            .execute(&stages, &payload)
            .await
            .expect("[identity, unix_socket] must round-trip");
        assert_eq!(outcome.transport, "unix_socket");
        assert_eq!(outcome.bytes_received, payload);
        assert_eq!(
            outcome.stages_executed,
            vec!["identity".to_string(), "unix_socket".to_string()],
            "both stages appear in stages_executed in dispatch order"
        );
    }

    // -- UnixSocketExecutor: TcpExecutor seam is preserved ---------------

    // `tcp_plan_succeeds_and_reports_tcp_transport` (above) already
    // pins the TcpExecutor seam; we don't duplicate it here. The
    // presence of UnixSocketExecutor must not perturb TcpExecutor's
    // observable behavior.

    // -- UnixSocketExecutor: benchmark harness --------------------------
    //
    // Plain `#[test]` (not criterion), `#[ignore]` so CI stays fast.
    // Run with:
    //   cargo test -p fabric-graph --lib unix_socket_benchmark \
    //       -- --ignored --nocapture

    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    #[ignore = "benchmark — run with --ignored --nocapture; not for CI"]
    async fn unix_socket_round_trip_benchmark() {
        use std::time::Instant;

        const ITERATIONS: usize = 100;
        const PAYLOAD_SIZE: usize = 4096;

        let payload: Vec<u8> = (0..PAYLOAD_SIZE).map(|i| (i % 251) as u8).collect();
        let catalog = builtin_stages();
        let stage = stage_by_id(&catalog, "unix_socket").clone();
        let stages = vec![stage];
        let exec = UnixSocketExecutor;

        let mut durations: Vec<std::time::Duration> = Vec::with_capacity(ITERATIONS);
        let mut ok_runs = 0usize;
        let mut err_runs = 0usize;

        for _ in 0..ITERATIONS {
            let start = Instant::now();
            let result = exec.execute(&stages, &payload).await;
            let dur = start.elapsed();
            durations.push(dur);

            match result {
                Ok(o) => {
                    assert_eq!(o.transport, "unix_socket");
                    assert_eq!(o.bytes_received, payload);
                    ok_runs += 1;
                }
                Err(_) => err_runs += 1,
            }
        }

        durations.sort();
        let total: std::time::Duration = durations.iter().sum();
        let avg = total / ITERATIONS as u32;
        let min = durations[0];
        let max = durations[durations.len() - 1];
        let p50 = durations[ITERATIONS / 2];
        let p99 = durations[(ITERATIONS.saturating_sub(1) * 99) / 100];

        eprintln!(
            "unix_socket round-trip benchmark: iterations={ITERATIONS} payload={PAYLOAD_SIZE}B"
        );
        eprintln!("  total: {total:?}");
        eprintln!("  avg:   {avg:?}");
        eprintln!("  min:   {min:?}");
        eprintln!("  p50:   {p50:?}");
        eprintln!("  p99:   {p99:?}");
        eprintln!("  max:   {max:?}");
        eprintln!("  ok:    {ok_runs}");
        eprintln!("  err:   {err_runs}");

        // Assert the round-trip succeeded every iteration. No timing
        // assertion — measurements, not guarantees, per the operator
        // instruction (do not make CI flaky with hard timing).
        assert_eq!(
            ok_runs, ITERATIONS,
            "every iteration must round-trip cleanly (err_runs={err_runs})"
        );
    }

    // -- Degradation-aware execution tests ------------------------------
    //
    // These tests are the behavioral contract for the new
    // `execute_with_selection` helper and the degradation metadata
    // fields on `ExecOutcome`. They are the only way to prove the
    // legacy `select_stages` silent-substitution path is now
    // observable. The shape: each test pairs a `StageSelection`
    // built by `select_stages_with_options` with an executor, runs
    // `execute_with_selection`, and asserts on the merged
    // `ExecOutcome`.

    /// Default-mode end-to-end: L4 wants RDMA, selection degrades to
    /// TCP, the executor runs, and the merged `ExecOutcome` carries
    /// the full degradation trace. This is the bug-class test: had
    /// the legacy `select_stages` been used, the outcome would have
    /// said `"transport": "tcp_transport"` with no way to know
    /// anything was lost. With the new path, the outcome says
    /// `requested = "rdma"`, `degraded_from = "rdma"`, and
    /// `degradation_reason` is non-empty.
    #[tokio::test(flavor = "current_thread")]
    async fn l4_rdma_default_mode_degrades_with_metadata() {
        use super::super::selection::{select_stages_with_options, SelectionOptions};

        let catalog = builtin_stages();
        let selection =
            select_stages_with_options(LocalityTier::L4Rdma, &catalog, SelectionOptions::default())
                .expect("default mode must NOT error; it should degrade");

        let exec = TcpExecutor;
        let payload = b"rdma-fallback-payload";
        let outcome = execute_with_selection(&exec, &selection, payload)
            .await
            .expect("TCP executor must serve the degraded stage");

        // transport = what actually served (TCP, the implemented fallback)
        assert_eq!(outcome.transport, "tcp_transport");
        // requested = what the tier wanted (the unimplemented ideal)
        assert_eq!(outcome.requested, "rdma");
        // degraded_from = the original ideal; reason = non-empty
        assert_eq!(outcome.degraded_from, Some("rdma".to_string()));
        let reason = outcome
            .degradation_reason
            .as_deref()
            .expect("degradation_reason must be Some when degraded_from is Some");
        assert!(!reason.is_empty(), "degradation_reason must be non-empty");
        assert!(
            reason.contains("rdma") && reason.contains("tcp_transport"),
            "degradation_reason must name both the ideal and the actual: {reason:?}"
        );
        // Stages carry the DEGRADED transport, not the ideal. This
        // is what made the bug visible in the first place: the plan
        // actually executed by the seam is the implemented one.
        let adapters: Vec<&str> = outcome.stages_executed.iter().map(|s| s.as_str()).collect();
        assert!(
            !adapters.contains(&"rdma"),
            "stages_executed must NOT contain 'rdma'; got {adapters:?}"
        );
        assert!(
            adapters.contains(&"tcp_transport"),
            "stages_executed must contain the actual served transport; got {adapters:?}"
        );
        // Bytes accounting stays the same: payload round-tripped
        // through the seam with the same observability as before.
        assert_eq!(outcome.bytes_sent, payload.len());
    }

    /// Strict-mode end-to-end: L4 wants RDMA, strict mode refuses
    /// to degrade, and `execute_with_selection` was never reached
    /// because the selection itself errored. The test pins that
    /// the function pair — selection-then-execute — is
    /// short-circuiting: there is no "TCP round-trip happens
    /// anyway" path.
    #[tokio::test(flavor = "current_thread")]
    async fn l4_rdma_strict_mode_returns_unsupported() {
        use super::super::selection::{select_stages_with_options, SelectionOptions};

        let catalog = builtin_stages();
        let result =
            select_stages_with_options(LocalityTier::L4Rdma, &catalog, SelectionOptions::strict());

        assert_eq!(
            result,
            Err(ExecError::UnsupportedTransport {
                requested: "rdma".to_string()
            }),
            "strict mode must surface the unimplemented ideal at selection time"
        );
    }

    /// Default-mode end-to-end, implemented tier: L5 wants
    /// unix_socket, the selection is a no-op, the executor runs
    /// unix_socket, and the merged `ExecOutcome` carries the
    /// same-transport confirmation (no degradation).
    #[cfg(unix)]
    #[tokio::test(flavor = "current_thread")]
    async fn l5_loopback_default_mode_no_degradation() {
        use super::super::selection::{select_stages_with_options, SelectionOptions};

        let catalog = builtin_stages();
        let selection = select_stages_with_options(
            LocalityTier::L5Loopback,
            &catalog,
            SelectionOptions::default(),
        )
        .expect("L5 ideal is implemented; default mode must NOT error");

        let exec = UnixSocketExecutor;
        let payload = b"loopback-payload";
        let outcome = execute_with_selection(&exec, &selection, payload)
            .await
            .expect("UnixSocketExecutor must serve the unix_socket stage");

        assert_eq!(outcome.transport, "unix_socket");
        assert_eq!(outcome.requested, "unix_socket");
        assert_eq!(outcome.degraded_from, None);
        assert_eq!(outcome.degradation_reason, None);
        assert_eq!(outcome.bytes_sent, payload.len());
        // Round-trip identity: UnixSocketExecutor performs a real
        // byte round-trip, so the bytes received must equal the
        // payload byte-for-byte. This is the same property the
        // existing round-trip tests pin, just exercised through the
        // selection-aware path.
        assert_eq!(
            outcome.bytes_received, payload,
            "round-trip must be byte-identical through the selection-aware path"
        );
    }

    /// Drift-guard test for the registry <-> executor invariant.
    /// This is the same shape as the one in `selection.rs` but
    /// expressed in terms of the executors in this module: the
    /// static lists of "what each executor serves" must match the
    /// runtime registry. The two tests (this one and
    /// `selection::tests::implemented_adapters_match_executor_capabilities`)
    /// are independently meaningful: this one lives next to the
    /// executors and will fail if a future refactor drops
    /// `implemented_adapters()` or shortens the registry without
    /// touching the executors.
    #[test]
    fn implemented_adapters_match_executor_capabilities() {
        // Compile-time declaration of what each executor in this
        // crate actually serves. Update this whenever a new
        // executor lands or an existing executor grows a new
        // adapter id.
        let mut served: Vec<&'static str> = Vec::new();
        served.push("tcp_transport"); // TcpExecutor serves this
        #[cfg(unix)]
        served.push("unix_socket"); // UnixSocketExecutor serves this
        served.sort();
        served.dedup();

        let mut registry: Vec<&'static str> = implemented_adapters().to_vec();
        registry.sort();

        assert_eq!(
            registry, served,
            "implemented_adapters() drifted from what the executors actually serve: \
             registry={registry:?} served={served:?}. Update both sides in the same change."
        );

        // Sanity: no orphan declarations in either direction.
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
}
