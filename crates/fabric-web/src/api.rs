//! API client for fetching data from the Fabric daemon wire server.

use serde::{Deserialize, Serialize};

/// Topology response.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopologyResponse {
    pub nodes: Vec<TopologyNode>,
}

/// Simplified node for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNode {
    pub id: String,
    pub label: Option<String>,
    pub locality: String,
    pub cap_count: usize,
    pub tags: Vec<String>,
}

/// Routes response.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutesResponse {
    pub routes: Vec<RoutePlan>,
}

/// Route plan for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePlan {
    pub intent: String,
    pub steps: usize,
    pub cost: f64,
    pub trust_level: String,
}

/// Capabilities response.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilitiesResponse {
    pub capabilities: Vec<CapEntry>,
}

/// Capability entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapEntry {
    pub node_name: String,
    pub descriptor_id: String,
    pub trust: String,
}

/// Health response.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HealthResponse {
    pub daemon_healthy: bool,
    pub node_count: usize,
    pub edge_count: usize,
    pub cap_count: usize,
    pub route_count: usize,
}

/// Error response from API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
}

/// Fetch JSON from the daemon API.
pub async fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T, String> {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};

    let window = web_sys::window().ok_or("No window object")?;

    let opts = RequestInit::new();
    opts.set_method("GET");

    let request = Request::new_with_str_and_init(url, &opts)
        .map_err(|e| format!("Failed to create request: {e:?}"))?;

    let resp_val = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {e:?}"))?;

    let resp: Response = resp_val.into();

    if !resp.ok() {
        return Err(format!("HTTP {} {}", resp.status(), resp.status_text()));
    }

    let text_val = JsFuture::from(resp.text()
        .map_err(|e| format!("Failed to read response: {e:?}"))?)
        .await
        .map_err(|e| format!("Failed to read response text: {e:?}"))?;

    let text = text_val
        .as_string()
        .ok_or("Response text is not a string")?;

    serde_json::from_str(&text).map_err(|e| format!("JSON parse error: {e}"))
}

/// Default daemon API base URL.
pub fn daemon_base_url() -> String {
    let window = web_sys::window().expect("no window");
    let location = window.location();
    let host = location.host().unwrap_or_else(|_| "127.0.0.1:7833".to_string());
    format!("http://{host}/api")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_response_default() {
        let h = HealthResponse::default();
        assert!(!h.daemon_healthy);
        assert_eq!(h.node_count, 0);
    }

    #[test]
    fn topology_response_default() {
        let t = TopologyResponse::default();
        assert!(t.nodes.is_empty());
    }

    #[test]
    fn serialization_roundtrip() {
        let health = HealthResponse {
            daemon_healthy: true,
            node_count: 3,
            edge_count: 5,
            cap_count: 2,
            route_count: 1,
        };
        let json = serde_json::to_string(&health).unwrap();
        let parsed: HealthResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.daemon_healthy);
        assert_eq!(parsed.node_count, 3);
    }

    #[test]
    fn route_plan_serialization() {
        let route = RoutePlan {
            intent: "gpu-inference".to_string(),
            steps: 3,
            cost: 1.5,
            trust_level: "Attested".to_string(),
        };
        let json = serde_json::to_string(&route).unwrap();
        let parsed: RoutePlan = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.intent, "gpu-inference");
        assert_eq!(parsed.steps, 3);
    }

    #[test]
    fn cap_entry_serialization() {
        let cap = CapEntry {
            node_name: "gpu-node-1".to_string(),
            descriptor_id: "abc123".to_string(),
            trust: "Audited".to_string(),
        };
        let json = serde_json::to_string(&cap).unwrap();
        let parsed: CapEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.node_name, "gpu-node-1");
    }
}
