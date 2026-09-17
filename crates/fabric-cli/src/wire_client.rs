//! Simple TCP wire client for communicating with fabric-daemon.
//!
//! The daemon speaks a line-based JSON protocol over TCP (spec 025).
//! Each request is a single JSON line; the response is a single JSON line.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Default daemon wire server address.
pub const DEFAULT_DAEMON_ADDR: &str = "127.0.0.1:9400";

/// Connection timeout for the wire client.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// Read/write timeout for wire operations.
const IO_TIMEOUT: Duration = Duration::from_secs(5);

/// Error type for wire client operations.
#[derive(Debug, thiserror::Error)]
pub enum WireClientError {
    #[error("connection refused at {addr} — is fabric-daemon running?")]
    ConnectionRefused { addr: String },

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("daemon returned error: {0}")]
    DaemonError(String),

    #[error("invalid response from daemon: {0}")]
    InvalidResponse(String),
}

/// Send a JSON message to the daemon wire server and return the parsed response.
///
/// Opens a TCP connection, sends the message as a single line, reads the
/// response, and closes the connection (one-shot, no keep-alive).
pub fn send_message(
    addr: &str,
    message: &serde_json::Value,
) -> Result<serde_json::Value, WireClientError> {
    let stream = match TcpStream::connect_timeout(
        &addr
            .parse()
            .map_err(|e: std::net::AddrParseError| WireClientError::DaemonError(e.to_string()))?,
        CONNECT_TIMEOUT,
    ) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
            return Err(WireClientError::ConnectionRefused {
                addr: addr.to_string(),
            });
        }
        Err(e) => return Err(WireClientError::Io(e)),
    };

    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;

    // Send message as a single JSON line.
    let mut writer = BufReader::new(&stream);
    let msg_str =
        serde_json::to_string(message).map_err(|e| WireClientError::DaemonError(e.to_string()))?;
    writer.get_mut().write_all(msg_str.as_bytes())?;
    writer.get_mut().write_all(b"\n")?;
    writer.get_mut().flush()?;

    // Read response line.
    let mut reader = BufReader::new(&stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    if response_line.trim().is_empty() {
        return Err(WireClientError::InvalidResponse(
            "empty response from daemon".into(),
        ));
    }

    let response: serde_json::Value = serde_json::from_str(response_line.trim())
        .map_err(|e| WireClientError::InvalidResponse(e.to_string()))?;

    // Check for error responses.
    if let Some(err) = response.get("error").and_then(|v| v.as_str()) {
        let msg = response
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(WireClientError::DaemonError(format!("{}: {}", err, msg)));
    }

    Ok(response)
}

/// Send a health_check message to the daemon.
pub fn health_check(addr: &str) -> Result<serde_json::Value, WireClientError> {
    let msg = serde_json::json!({"type": "health_check"});
    send_message(addr, &msg)
}

/// Send a routes_request message to the daemon.
pub fn routes_request(addr: &str) -> Result<serde_json::Value, WireClientError> {
    let msg = serde_json::json!({"type": "routes_request"});
    send_message(addr, &msg)
}

/// Send a topology_request message to the daemon.
pub fn topology_request(addr: &str) -> Result<serde_json::Value, WireClientError> {
    let msg = serde_json::json!({"type": "topology_request"});
    send_message(addr, &msg)
}

/// Send a capabilities_request message to the daemon.
pub fn capabilities_request(addr: &str) -> Result<serde_json::Value, WireClientError> {
    let msg = serde_json::json!({"type": "capabilities_request"});
    send_message(addr, &msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_addr_is_localhost() {
        assert_eq!(DEFAULT_DAEMON_ADDR, "127.0.0.1:9400");
    }

    #[test]
    fn send_message_connection_refused() {
        // No daemon running on a random port — should get ConnectionRefused.
        let result = send_message("127.0.0.1:59999", &serde_json::json!({"type": "heartbeat"}));
        assert!(result.is_err());
        match result.unwrap_err() {
            WireClientError::ConnectionRefused { addr } => {
                assert!(addr.contains("59999"));
            }
            other => panic!("expected ConnectionRefused, got {:?}", other),
        }
    }
}
