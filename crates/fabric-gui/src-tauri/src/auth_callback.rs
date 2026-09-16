//! One-shot HTTP listener for WorkOS OAuth callback.
//!
//! Starts a temporary server on a random port, waits for the OAuth redirect,
//! extracts the authorization code, emits it to the frontend via a Tauri event,
//! returns a success page to the browser, and shuts down.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use tauri::Emitter;

/// Start a one-shot HTTP listener on a random port.
/// Returns the port number. The listener runs in a background thread.
/// When the callback arrives, it emits `auth-code-received` with `{ code, state }`.
pub fn start_listener(app_handle: tauri::AppHandle) -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("bind failed: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();

    std::thread::spawn(move || {
        tracing::info!(port, "auth callback listener started");
        listener.set_nonblocking(false).ok();

        match listener.accept() {
            Ok((stream, _)) => {
                if let Err(e) = handle_callback(stream, &app_handle) {
                    tracing::error!("auth callback error: {e}");
                }
            }
            Err(e) => {
                tracing::error!("auth callback accept failed: {e}");
            }
        }
        tracing::info!("auth callback listener shutting down");
    });

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
