//! HTTP listener loop and request/response parsing.
//!
//! Hand-rolled HTTP/1.1 over `std::net::TcpListener` because the wire
//! server already uses the same primitive and pulling in axum/hyper/
//! tiny_http for a single GET is not worth the dep-graph churn. The
//! parser is deliberately tiny: it reads the request line, the headers
//! up to the blank line, dispatches by method + path, and writes a
//! `Connection: close` response.

use super::dispatch::{process_callback, HttpResponse};
use crate::coordinator::Coordinator;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Start the OAuth/HTTP callback listener.
///
/// Blocking. Returns when the coordinator's shutdown flag is set. Designed
/// to run on a dedicated `std::thread` alongside the wire server.
///
/// `request_timeout` caps how long a single connection is allowed to
/// stall; the listener itself does not impose a session-level timeout
/// (the state TTL handles replay).
pub fn run_callback_server(
    listener: TcpListener,
    coordinator: std::sync::Arc<Coordinator>,
    runtime: std::sync::Arc<tokio::runtime::Runtime>,
    request_timeout: Duration,
) {
    if let Err(e) = listener.set_nonblocking(true) {
        error!(error = %e, "callback listener: failed to set non-blocking");
        return;
    }

    info!(
        addr = ?listener.local_addr().ok(),
        "OAuth callback listener started"
    );

    loop {
        if coordinator.is_shutting_down() {
            info!("callback listener shutting down");
            break;
        }

        match listener.accept() {
            Ok((stream, peer)) => {
                let coord = coordinator.clone();
                let rt = runtime.clone();
                std::thread::spawn(move || {
                    if let Err(e) = handle_http_connection(stream, coord, rt, request_timeout) {
                        debug!(peer = %peer, error = %e, "callback connection error");
                    }
                });
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => {
                warn!(error = %e, "callback listener: accept error");
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }

    info!("callback listener stopped");
}

fn handle_http_connection(
    stream: TcpStream,
    coordinator: std::sync::Arc<Coordinator>,
    runtime: std::sync::Arc<tokio::runtime::Runtime>,
    timeout: Duration,
) -> std::io::Result<()> {
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));

    let mut reader_stream = stream.try_clone()?;
    let mut writer = stream;
    let mut reader = BufReader::new(&mut reader_stream);

    // --- Read request line ---
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let request_line = request_line.trim_end_matches(['\r', '\n']).to_string();

    if request_line.is_empty() {
        return write_response(
            &mut writer,
            400,
            "Bad Request",
            "text/plain",
            b"empty request",
        );
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    let _http_version = parts.next().unwrap_or("");

    // --- Read headers until blank line ---
    loop {
        let mut header = String::new();
        let n = reader.read_line(&mut header)?;
        if n == 0 {
            break;
        }
        let trimmed = header.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
    }

    // --- Dispatch ---
    if method != "GET" {
        return write_response(
            &mut writer,
            405,
            "Method Not Allowed",
            "text/plain",
            b"only GET is supported",
        );
    }

    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p, q),
        None => (target, ""),
    };

    if path != "/auth/callback" {
        return write_response(&mut writer, 404, "Not Found", "text/plain", b"not found");
    }

    let params = parse_query(query);
    let code = params.get("code").map(String::as_str);
    let state = params.get("state").map(String::as_str);
    // The browser-driven callback does not include `code_verifier` (the
    // daemon minted it server-side at `auth_start` and bound it to the
    // state). We pass `None` so the dispatch uses the minted verifier.
    // A caller-supplied verifier here would mean the caller already
    // knows the daemon's secret, which would let them spoof any login --
    // see dispatch::process_callback for the matching-mismatch guard.
    let code_verifier: Option<&str> = None;

    let response = runtime.block_on(process_callback(&coordinator, code, state, code_verifier));

    write_html_response(&mut writer, &response)
}

fn write_response(
    writer: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    writer.write_all(head.as_bytes())?;
    writer.write_all(body)?;
    writer.flush()?;

    // Graceful close for `Connection: close`: FIN the write side so the peer
    // sees end-of-stream rather than a reset. Do NOT drain the read side here
    // -- that was tried and made the tests measurably worse, because holding
    // the connection open to drain races the caller's own EOF handling. The
    // correct client behaviour is to read Content-Length bytes and stop, so
    // the server does not need to keep the connection alive for it.
    let _ = writer.shutdown(Shutdown::Write);
    Ok(())
}

fn write_html_response(writer: &mut TcpStream, resp: &HttpResponse) -> std::io::Result<()> {
    write_response(
        writer,
        resp.status,
        resp.reason,
        "text/html; charset=utf-8",
        resp.body.as_bytes(),
    )
}

// ---------------------------------------------------------------------------
// Query parsing
// ---------------------------------------------------------------------------

fn parse_query(query: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    if query.is_empty() {
        return out;
    }
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };
        out.insert(url_decode(k), url_decode(v));
    }
    out
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                if let (Some(h), Some(l)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                    out.push((h << 4) | l);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
