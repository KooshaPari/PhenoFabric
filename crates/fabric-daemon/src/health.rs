//! Health check endpoint for fabric-daemon.

use serde::Serialize;
use std::time::Instant;

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub uptime_s: u64,
    pub topology_epoch: u64,
    pub active_leases: usize,
    pub active_plans: usize,
}

impl HealthResponse {
    /// Build a health response from current daemon state.
    pub fn new(
        start_time: Instant,
        topology_epoch: u64,
        active_leases: usize,
        active_plans: usize,
    ) -> Self {
        Self {
            status: "healthy",
            uptime_s: start_time.elapsed().as_secs(),
            topology_epoch,
            active_leases,
            active_plans,
        }
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| r#"{"status":"error"}"#.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn health_response_json() {
        let resp = HealthResponse {
            status: "healthy",
            uptime_s: 3600,
            topology_epoch: 42,
            active_leases: 7,
            active_plans: 3,
        };
        let json = resp.to_json();
        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"uptime_s\":3600"));
        assert!(json.contains("\"topology_epoch\":42"));
        assert!(json.contains("\"active_leases\":7"));
        assert!(json.contains("\"active_plans\":3"));
    }

    #[test]
    fn health_response_from_state() {
        let start = Instant::now() - Duration::from_secs(60);
        let resp = HealthResponse::new(start, 5, 3, 1);
        assert_eq!(resp.status, "healthy");
        assert!(resp.uptime_s >= 59 && resp.uptime_s <= 61);
        assert_eq!(resp.topology_epoch, 5);
        assert_eq!(resp.active_leases, 3);
        assert_eq!(resp.active_plans, 1);
    }
}
