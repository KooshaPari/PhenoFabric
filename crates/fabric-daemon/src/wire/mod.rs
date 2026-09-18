//! Wire transport server for fabric-daemon.
//!
//! Listens on a TCP socket and handles incoming wire messages.
//! Implements the server side of spec 025.

mod handlers;
pub mod protocol;

use crate::auth::middleware::routes::auth_error_response;
use crate::auth::middleware::set_current_user;
use crate::auth::AuthMiddleware;
use crate::coordinator::Coordinator;
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
                    handle_connection(stream, coord, timeout, &auth, &rt);
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
fn handle_connection(
    stream: TcpStream,
    coordinator: Arc<Coordinator>,
    timeout: Duration,
    auth: &AuthMiddleware,
    runtime: &tokio::runtime::Runtime,
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

        debug!(peer = %peer, len = line.len(), "received message");

        // --- Auth middleware validation ---
        // Parse the message for auth checking. If JSON is invalid, let
        // process_message handle the validation error downstream.
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&line) {
            match runtime.block_on(auth.validate_message(&parsed)) {
                Ok(Some(user)) => {
                    debug!(peer = %peer, user_id = %user.user_id, "auth: authenticated");
                    // Attach user to the message for downstream handlers.
                    let mut msg = parsed.clone();
                    if let Err(e) = set_current_user(&mut msg, &user) {
                        warn!(peer = %peer, error = %e, "failed to set auth user on message");
                    }
                    // Re-serialize for process_message (user field attached).
                    let re_serialized = msg.to_string();
                    let response = protocol::process_message(&re_serialized, &coordinator);
                    if let Some(resp) = response {
                        if let Err(e) = writeln!(writer, "{resp}") {
                            debug!(peer = %peer, error = %e, "write error");
                            break;
                        }
                    }
                }
                Ok(None) => {
                    // Public route or auth disabled -- proceed normally.
                    let response = protocol::process_message(&line, &coordinator);
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
            let response = protocol::process_message(&line, &coordinator);
            if let Some(resp) = response {
                if let Err(e) = writeln!(writer, "{resp}") {
                    debug!(peer = %peer, error = %e, "write error");
                    break;
                }
            }
        }
    }

    debug!(peer = %peer, "connection closed");
}
