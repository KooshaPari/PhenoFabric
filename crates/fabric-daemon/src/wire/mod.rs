//! Wire transport server for fabric-daemon.
//!
//! Listens on a TCP socket and handles incoming wire messages.
//! Implements the server side of spec 025.

mod auth_handlers;
pub mod frame_handlers;
pub mod frame_resume;
pub mod frame_session_lifecycle;
pub mod frame_session_state;
mod handlers;
mod network_handlers;
pub mod protocol;
mod surface_handlers;

use crate::auth::middleware::routes::auth_error_response;
use crate::auth::middleware::set_current_user;
use crate::auth::AuthMiddleware;
use crate::coordinator::Coordinator;
use frame_session_lifecycle::FrameSessionDisconnect;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Error type for wire server operations.
#[derive(Debug, thiserror::Error)]
pub enum WireServerError {
    #[error("io error: {0}")]
    Io(String),
}

/// Start the wire transport server.
///
/// This is a blocking function that runs until the shutdown flag is set.
pub fn run_wire_server(
    listener: TcpListener,
    coordinator: Arc<Coordinator>,
    max_connections: usize,
    request_timeout_ms: u64,
    auth: Arc<AuthMiddleware>,
    runtime: Arc<tokio::runtime::Runtime>,
) -> Result<(), WireServerError> {
    // Use non-blocking mode so the accept loop periodically returns, allowing
    // the shutdown flag to be checked. Without this, a blocking `accept()`
    // would hang indefinitely, preventing `stop_daemon` / `thread::join()`.
    listener
        .set_nonblocking(true)
        .map_err(|e| WireServerError::Io(format!("failed to set listener to non-blocking: {e}")))?;

    let timeout = Duration::from_millis(request_timeout_ms);
    let mut active_connections: usize = 0;

    info!(
        addr = ?listener.local_addr().ok(),
        max_connections,
        "wire server started"
    );

    loop {
        if coordinator.is_shutting_down() {
            info!("wire server shutting down, rejecting new connections");
            break;
        }

        // Idle-expiry sweep. Driven from this loop rather than from a
        // per-connection reader thread because this loop ticks on a
        // wall-clock cadence whether or not any client is sending: a session
        // belonging to a client that vanished without closing must still be
        // reaped, and that client will send nothing to trigger it.
        //
        // The loop sleeps ~100 ms per tick when the listener has nothing to
        // accept, so this scans at roughly 10 Hz. The timeout is far larger
        // (see `frame_session_state::DEFAULT_IDLE_TIMEOUT_MS`), so the sweep
        // cost is a hash-map scan of at most
        // `MAX_CONCURRENT_SESSIONS` entries per tick.
        {
            let expired = frame_handlers::sweep_expired_frame_sessions(&coordinator);
            if !expired.is_empty() {
                info!(
                    count = expired.len(),
                    sessions = ?expired,
                    idle_timeout_ms = frame_session_state::DEFAULT_IDLE_TIMEOUT_MS,
                    "expired idle frame sessions"
                );
            }
        }

        // Accept with a short timeout so we can check shutdown flag periodically.
        match listener.accept() {
            Ok((stream, peer_addr)) => {
                if active_connections >= max_connections {
                    warn!(
                        peer = %peer_addr,
                        active_connections,
                        max_connections,
                        "connection limit reached, rejecting"
                    );
                    let mut stream = stream;
                    let _ = writeln!(
                        stream,
                        "{{\"error\":\"server_busy\",\"message\":\"max connections reached\"}}"
                    );
                    continue;
                }

                active_connections += 1;
                debug!(peer = %peer_addr, active_connections, "new connection");

                // Set the accepted connection back to blocking mode.
                // The listener is non-blocking (for shutdown checks), but
                // handler threads need blocking reads with timeouts.
                stream.set_nonblocking(false).ok();

                // Disable Nagle. This is a request/response protocol carrying
                // small framing messages, and Nagle interacts badly with the
                // peer's delayed-ACK timer: a small write can sit un-sent until
                // an ACK arrives, which costs roughly 40 ms on loopback. That
                // shows up directly as tail latency, which is the thing this
                // product is supposed to measure honestly.
                if let Err(e) = stream.set_nodelay(true) {
                    debug!(error = %e, "could not set TCP_NODELAY on wire connection");
                }

                let coord = coordinator.clone();
                let auth = auth.clone();
                let rt = runtime.clone();
                let handle = std::thread::spawn(move || {
                    // handle_connection is async (process_message awaits auth
                    // handlers); drive it to completion on this thread through
                    // the shared tokio runtime.
                    rt.block_on(handle_connection(stream, coord, timeout, &auth));
                    // Decrement is handled by Drop of a counter or we accept the leak
                    // for now -- in production, use an AtomicUsize counter.
                });

                // Detach the thread (we don't join here -- fire and forget).
                drop(handle);
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                // No connection available or accept timed out; check shutdown flag.
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => {
                error!("accept error: {e}");
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }

    info!("wire server stopped");
    Ok(())
}

/// Handle a single TCP connection.
async fn handle_connection(
    stream: TcpStream,
    coordinator: Arc<Coordinator>,
    timeout: Duration,
    auth: &AuthMiddleware,
) {
    let peer = stream
        .peer_addr()
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "unknown".into());

    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));

    let reader_stream = match stream.try_clone() {
        Ok(s) => s,
        Err(e) => {
            debug!(peer = %peer, error = %e, "failed to clone stream");
            return;
        }
    };
    let reader = BufReader::new(reader_stream);
    let mut writer = stream;

    // Sessions this connection has touched, so a drop can mark them dormant.
    //
    // The daemon's disconnect policy is "dormant until the idle sweep reclaims
    // it" (see `frame_session_state`'s module docs), and acting on that policy
    // requires knowing which sessions belonged to the connection that just
    // died. The protocol deliberately does not bind a session to a socket --
    // that is what let state survive reconnects in the first place -- so this
    // is a *best-effort observation* of what flowed over this connection, not
    // ownership. A session named on this connection may also be in use on
    // another; marking it dormant is a marker that any subsequent accepted
    // traffic clears, and it does not by itself make the session unusable.
    //
    // It is deliberately populated only from frame-plane messages. Heartbeats
    // and health checks say nothing about sessions and must not mark anything.
    let mut touched_sessions: Vec<u64> = Vec::new();

    for line in reader.lines() {
        if coordinator.is_shutting_down() {
            let _ = writeln!(writer, "{{\"error\":\"shutting_down\"}}");
            break;
        }

        let line = match line {
            Ok(l) => l,
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                // No data available or read timed out -- keep connection open.
                // Sleep briefly to avoid busy-spinning, then retry.
                std::thread::sleep(Duration::from_millis(50));
                continue;
            }
            Err(e) => {
                debug!(peer = %peer, error = %e, "read error, closing connection");
                break;
            }
        };

        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        // Record which frame session this connection is carrying, so a drop can
        // mark exactly those dormant. Best-effort by design: a line that does
        // not parse, or that names no session, simply records nothing.
        note_session_in_line(&line, &mut touched_sessions);

        debug!(peer = %peer, len = line.len(), "received message");

        // --- Auth middleware validation ---
        // Parse the message for auth checking. If JSON is invalid, let
        // process_message handle the validation error downstream.
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&line) {
            match auth.validate_message(&parsed).await {
                Ok(Some(user)) => {
                    debug!(peer = %peer, user_id = %user.user_id, "auth: authenticated");
                    // Attach user to the message for downstream handlers.
                    let mut msg = parsed.clone();
                    if let Err(e) = set_current_user(&mut msg, &user) {
                        warn!(peer = %peer, error = %e, "failed to set auth user on message");
                    }
                    // Re-serialize for process_message (user field attached).
                    let re_serialized = msg.to_string();
                    let response = protocol::process_message(&re_serialized, &coordinator).await;
                    if let Some(resp) = response {
                        if let Err(e) = writeln!(writer, "{resp}") {
                            debug!(peer = %peer, error = %e, "write error");
                            break;
                        }
                    }
                }
                Ok(None) => {
                    // Public route or auth disabled -- proceed normally.
                    let response = protocol::process_message(&line, &coordinator).await;
                    if let Some(resp) = response {
                        if let Err(e) = writeln!(writer, "{resp}") {
                            debug!(peer = %peer, error = %e, "write error");
                            break;
                        }
                    }
                }
                Err(e) => {
                    warn!(peer = %peer, error = %e, "auth: rejected");
                    let resp = auth_error_response(&e);
                    if let Err(write_err) = writeln!(writer, "{resp}") {
                        debug!(peer = %peer, error = %write_err, "write error");
                        break;
                    }
                }
            }
        } else {
            // Invalid JSON -- let process_message return the validation error.
            let response = protocol::process_message(&line, &coordinator).await;
            if let Some(resp) = response {
                if let Err(e) = writeln!(writer, "{resp}") {
                    debug!(peer = %peer, error = %e, "write error");
                    break;
                }
            }
        }
    }

    debug!(peer = %peer, "connection closed");

    // The disconnect policy, applied. The sessions this connection was
    // carrying are marked dormant -- NOT destroyed. They keep every counter,
    // stay resumable via `session_resume`, and are reclaimed by the ordinary
    // idle sweep if the client never returns. See the "Disconnect policy"
    // section of `frame_session_state`'s module docs for why dormant rather
    // than torn down.
    //
    // Skipped on daemon shutdown: every session is going away regardless, and
    // marking them dormant would only add log noise on the way out.
    if !coordinator.is_shutting_down() && !touched_sessions.is_empty() {
        let dormant = coordinator
            .frame_sessions_mut()
            .mark_disconnected_many(&touched_sessions);
        if !dormant.is_empty() {
            info!(
                peer = %peer,
                sessions = ?dormant,
                "frame sessions marked disconnected; awaiting resume or idle expiry"
            );
        }
    }
}

/// Note the frame session a wire line refers to, if it refers to one.
///
/// Only the frame-plane messages that *name* an existing session count:
/// `session_init` does not (the id is assigned by the daemon in response), and
/// `heartbeat` / `health_check` and friends do not (they say nothing about
/// sessions). `session_close` / `session_cancel` do name one but are excluded
/// deliberately: they remove the session, so there is nothing left to mark
/// dormant, and counting them would make the drop path re-flag a dead id.
///
/// The duplicate check keeps the list bounded on a long-lived connection that
/// streams thousands of frames: the first occurrence of an id is all that
/// matters for a drop.
fn note_session_in_line(line: &str, sessions: &mut Vec<u64>) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return;
    };
    let Some(msg_type) = value.get("type").and_then(|v| v.as_str()) else {
        return;
    };
    // Match the dispatcher's naming (snake_case and PascalCase are both
    // accepted on the wire) without pulling in the normalization helper, which
    // lives in the transport crate's private validation module.
    let normalized = msg_type.to_ascii_lowercase().replace('_', "");
    if !matches!(
        normalized.as_str(),
        "framedata" | "frameack" | "sessionresume"
    ) {
        return;
    }
    if let Some(session_id) = value.get("session_id").and_then(|v| v.as_u64()) {
        if !sessions.contains(&session_id) {
            sessions.push(session_id);
        }
    }
}

#[cfg(test)]
#[path = "wire_connection_tests.rs"]
mod tests;
