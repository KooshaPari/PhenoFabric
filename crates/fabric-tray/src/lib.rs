//! fabric-tray: System tray for Phenotype Fabric daemon management.
//!
//! This library module exposes testable types. The binary crate (`main.rs`)
//! contains the tray event loop and platform-specific logic.

use std::net::TcpStream;
use std::time::Duration;

/// Daemon health status derived from polling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonStatus {
    /// Daemon is not running.
    Stopped,
    /// Daemon is running and healthy.
    Healthy,
    /// Daemon is running but not responding to health checks.
    Degraded,
    /// Daemon process exists but hasn't been polled yet.
    Starting,
}

impl DaemonStatus {
    pub fn label(&self) -> &'static str {
        match self {
            DaemonStatus::Stopped => "Stopped",
            DaemonStatus::Healthy => "Running",
            DaemonStatus::Degraded => "Degraded",
            DaemonStatus::Starting => "Starting...",
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self, DaemonStatus::Healthy | DaemonStatus::Degraded)
    }
}

/// Create a simple tray icon (colored circle based on status).
pub fn create_icon(status: DaemonStatus) -> tray_icon::Icon {
    let (r, g, b) = match status {
        DaemonStatus::Healthy => (0x22, 0xC5, 0x5E),
        DaemonStatus::Degraded => (0xEF, 0xBF, 0x04),
        DaemonStatus::Stopped => (0xEF, 0x44, 0x44),
        DaemonStatus::Starting => (0x60, 0xA5, 0xFA),
    };

    let mut rgba = vec![0u8; 32 * 32 * 4];
    for y in 0..32u32 {
        for x in 0..32u32 {
            let i = (y * 32 + x) as usize * 4;
            let cx = (x as f64 - 16.0) / 16.0;
            let cy = (y as f64 - 16.0) / 16.0;
            let dist = (cx * cx + cy * cy).sqrt();
            if dist < 0.85 {
                rgba[i] = r;
                rgba[i + 1] = g;
                rgba[i + 2] = b;
                rgba[i + 3] = 255;
            } else if dist < 0.95 {
                let alpha = ((0.95 - dist) / 0.10 * 255.0) as u8;
                rgba[i] = r;
                rgba[i + 1] = g;
                rgba[i + 2] = b;
                rgba[i + 3] = alpha;
            }
        }
    }

    tray_icon::Icon::from_rgba(rgba, 32, 32).expect("valid icon")
}

/// Check daemon health by connecting to the wire server TCP port.
pub fn check_daemon_health(port: u16) -> DaemonStatus {
    let addr = format!("127.0.0.1:{}", port);
    let Ok(addr) = addr.parse() else {
        return DaemonStatus::Degraded;
    };

    match TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
        Ok(mut stream) => {
            use std::io::Write;
            let request = r#"{"type":"health_check"}"#;
            let _ = stream.write_all(request.as_bytes());
            let _ = stream.write_all(b"\n");

            let mut buf = [0u8; 1024];
            match std::io::Read::read(&mut stream, &mut buf) {
                Ok(n) if n > 0 => {
                    let response = String::from_utf8_lossy(&buf[..n]);
                    if response.contains("healthy") || response.contains("ok") {
                        DaemonStatus::Healthy
                    } else {
                        DaemonStatus::Degraded
                    }
                }
                _ => DaemonStatus::Degraded,
            }
        }
        Err(_) => DaemonStatus::Stopped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_status_labels() {
        assert_eq!(DaemonStatus::Stopped.label(), "Stopped");
        assert_eq!(DaemonStatus::Healthy.label(), "Running");
        assert_eq!(DaemonStatus::Degraded.label(), "Degraded");
        assert_eq!(DaemonStatus::Starting.label(), "Starting...");
    }

    #[test]
    fn daemon_status_is_running() {
        assert!(DaemonStatus::Healthy.is_running());
        assert!(DaemonStatus::Degraded.is_running());
        assert!(!DaemonStatus::Stopped.is_running());
        assert!(!DaemonStatus::Starting.is_running());
    }

    #[test]
    fn icon_creation_produces_valid_icon() {
        // Should not panic
        let _icon = create_icon(DaemonStatus::Healthy);
        let _icon = create_icon(DaemonStatus::Degraded);
        let _icon = create_icon(DaemonStatus::Stopped);
        let _icon = create_icon(DaemonStatus::Starting);
    }

    #[test]
    fn check_daemon_health_unreachable_port() {
        // Port 1 (unprivileged but almost certainly no daemon) should return Stopped
        let status = check_daemon_health(1);
        assert_eq!(status, DaemonStatus::Stopped);
    }
}
