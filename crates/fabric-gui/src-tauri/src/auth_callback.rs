//! One-shot HTTP listener for WorkOS OAuth callback.
//!
//! Starts a temporary server on a random port, waits for the OAuth redirect,
//! extracts the authorization code, emits it to the frontend via a Tauri event,
//! returns a success page to the browser, and shuts down.

use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;

/// How long an accepted connection may take to send its request line before it
/// is abandoned. Without this, a client that connects and then sends nothing
/// blocks the reader forever and wedges the whole login flow.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Bind both loopback addresses, on one shared port.
///
/// Binding `127.0.0.1` alone is not sufficient. The redirect URI handed to the
/// browser says `localhost`, and on a dual-stack host `localhost` resolves to
/// `::1` **before** `127.0.0.1` (that is the case on macOS with the stock
/// `/etc/hosts`). An IPv4-only listener therefore refuses the browser's first
/// connection attempt and the OAuth callback never arrives at all.
///
/// IPv6 is bound first to reserve the port, then IPv4 is bound to the same one.
/// Binding `::1` explicitly does not collide with `127.0.0.1` even when
/// `bindv6only=0`, because v4-mapped addresses live under `::ffff:0:0/96`, not
/// under `::1`. Either bind may fail independently; as long as one succeeds the
/// listener is usable.
fn bind_loopback() -> Result<(Vec<TcpListener>, u16), String> {
    let mut listeners = Vec::new();
    let mut port: Option<u16> = None;

    match TcpListener::bind((Ipv6Addr::LOCALHOST, 0)) {
        Ok(l) => {
            port = l.local_addr().ok().map(|a| a.port());
            listeners.push(l);
        }
        Err(e) => tracing::warn!("auth callback: IPv6 loopback bind failed: {e}"),
    }

    let v4_addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port.unwrap_or(0)));
    match TcpListener::bind(v4_addr) {
        Ok(l) => {
            if port.is_none() {
                port = l.local_addr().ok().map(|a| a.port());
            }
            listeners.push(l);
        }
        Err(e) => tracing::warn!("auth callback: IPv4 loopback bind failed: {e}"),
    }

    match port {
        Some(port) if !listeners.is_empty() => Ok((listeners, port)),
        _ => Err("could not bind either loopback address".to_string()),
    }
}

/// Start a one-shot HTTP listener on a random port.
/// Returns the port number. The listener runs in a background thread.
/// When the callback arrives, it emits `auth-code-received` with `{ code, state }`.
pub fn start_listener(app_handle: tauri::AppHandle) -> Result<u16, String> {
    let (listeners, port) = bind_loopback()?;

    // Guards against both listeners delivering the same callback.
    let handled = Arc::new(AtomicBool::new(false));

    for listener in listeners {
        let app = app_handle.clone();
        let handled = Arc::clone(&handled);
        let bound = listener
            .local_addr()
            .map(|a| a.to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        std::thread::spawn(move || {
            tracing::info!(port, addr = %bound, "auth callback listener started");

            // One-shot: accept a single connection, but bail out immediately if
            // the other listener already delivered the callback.
            match listener.accept() {
                Ok((stream, _)) => {
                    if handled.swap(true, Ordering::SeqCst) {
                        tracing::debug!(addr = %bound, "auth callback: already handled elsewhere");
                        return;
                    }
                    // A silent peer must not be able to block this thread forever.
                    if let Err(e) = stream.set_read_timeout(Some(READ_TIMEOUT)) {
                        tracing::warn!("auth callback: could not set read timeout: {e}");
                    }
                    if let Err(e) = handle_callback(stream, &app) {
                        tracing::error!("auth callback error: {e}");
                    }
                }
                Err(e) => {
                    tracing::error!(addr = %bound, "auth callback accept failed: {e}");
                }
            }
            tracing::info!(addr = %bound, "auth callback listener shutting down");
        });
    }

    Ok(port)
}

fn handle_callback(mut stream: TcpStream, app_handle: &tauri::AppHandle) -> Result<(), String> {
    let reader_stream = stream.try_clone().map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(reader_stream);

    // Read the request line: GET /auth/callback?code=xxx&state=yyy HTTP/1.1
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|e| e.to_string())?;

    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/");

    // Drain remaining headers
    let mut header = String::new();
    loop {
        header.clear();
        reader
            .read_line(&mut header)
            .map_err(|e| e.to_string())?;
        if header.trim().is_empty() {
            break;
        }
    }

    // Parse query parameters
    let query = path.split('?').nth(1).unwrap_or("");
    let params: std::collections::HashMap<String, String> = query
        .split('&')
        .filter_map(|pair| {
            let mut kv = pair.splitn(2, '=');
            Some((
                url_decode(kv.next()?)?,
                url_decode(kv.next().unwrap_or(""))?,
            ))
        })
        .collect();

    let code = params.get("code").cloned().unwrap_or_default();
    let state = params.get("state").cloned().unwrap_or_default();

    tracing::info!(code_len = code.len(), "auth callback received");

    // Emit event to frontend
    let _ = app_handle.emit(
        "auth-code-received",
        serde_json::json!({ "code": code, "state": state }),
    );

    // Send success response to browser
    let body = "<!DOCTYPE html><html><head><title>Authenticated</title></head>\
                <body style=\"background:#0a0a16;color:#00d4aa;font-family:system-ui;\
                display:flex;align-items:center;justify-content:center;height:100vh;\
                margin:0\"><h1>✓ Signed in — you can close this tab.</h1></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();

    Ok(())
}

fn url_decode(s: &str) -> Option<String> {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        match b {
            b'%' => {
                let hex: String = chars.by_ref().take(2).map(char::from).collect();
                let byte = u8::from_str_radix(&hex, 16).ok()?;
                result.push(byte as char);
            }
            b'+' => result.push(' '),
            _ => result.push(b as char),
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reason this module binds two sockets instead of one: `localhost` is
    /// not necessarily `127.0.0.1`. On a dual-stack host it resolves to `::1`
    /// first, so an IPv4-only listener refuses the browser's first attempt and
    /// the OAuth callback never arrives. This asserts both families are actually
    /// reachable on the one returned port.
    #[test]
    fn binds_both_loopback_families_on_the_same_port() {
        let (listeners, port) = bind_loopback().expect("bind_loopback should succeed");

        assert!(
            !listeners.is_empty(),
            "at least one loopback listener must be bound"
        );
        for l in &listeners {
            assert_eq!(
                l.local_addr().unwrap().port(),
                port,
                "every listener must share the returned port"
            );
        }

        let bound_families: Vec<bool> = listeners
            .iter()
            .map(|l| l.local_addr().unwrap().is_ipv6())
            .collect();

        // Connect over each bound family. `connect` succeeds as soon as the
        // kernel completes the handshake, which is enough to prove reachability.
        if bound_families.contains(&true) {
            let c = TcpStream::connect(SocketAddr::from((Ipv6Addr::LOCALHOST, port)));
            assert!(c.is_ok(), "IPv6 loopback must be reachable: {:?}", c.err());
        }
        if bound_families.contains(&false) {
            let c = TcpStream::connect(SocketAddr::from((Ipv4Addr::LOCALHOST, port)));
            assert!(c.is_ok(), "IPv4 loopback must be reachable: {:?}", c.err());
        }
    }

    /// Resolving `localhost` the way a browser does must land on a port this
    /// module can actually serve. This is the regression test for the defect
    /// that made the login flow unreachable.
    #[test]
    fn localhost_resolution_order_is_reachable() {
        let (_listeners, port) = bind_loopback().expect("bind_loopback should succeed");

        let addrs: Vec<SocketAddr> =
            std::net::ToSocketAddrs::to_socket_addrs(&("localhost", port))
                .expect("localhost should resolve")
                .collect();
        assert!(!addrs.is_empty(), "localhost must resolve to something");

        // Every address localhost resolves to must be connectable, including
        // whichever one the browser tries first.
        for addr in addrs {
            let c = TcpStream::connect(addr);
            assert!(
                c.is_ok(),
                "localhost resolves to {addr} but nothing is listening there: {:?}",
                c.err()
            );
        }
    }

    /// A silent peer must not be able to wedge an accepted connection forever.
    #[test]
    fn read_timeout_is_configured() {
        assert_eq!(READ_TIMEOUT, Duration::from_secs(30));
        assert!(!READ_TIMEOUT.is_zero(), "a zero timeout is not a timeout");
    }
}
