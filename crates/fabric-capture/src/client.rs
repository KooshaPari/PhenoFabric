//! HTTP client for posting captured console content to tf-web.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Serialize;

/// Pane data payload sent to tf-web.
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct PaneData {
    /// Unique pane identifier (e.g. "win-12345").
    pub pane_id: String,
    /// Window title (PowerShell process name or CWD).
    pub title: String,
    /// Console content lines (top to bottom).
    pub lines: Vec<String>,
    /// Console width in characters.
    pub width: u32,
    /// Console height in rows.
    pub height: u32,
    /// Source identifier ("windows-capture").
    pub source: String,
    /// Windows process ID.
    pub process_id: u32,
}

/// Request body for the pane update endpoint.
#[derive(Serialize)]
#[allow(dead_code)]
struct PaneUpdateRequest {
    panes: Vec<PaneData>,
}

/// Response body from tf-web.
#[derive(serde::Deserialize, Debug)]
#[allow(dead_code)]
struct ApiResponse {
    success: bool,
    error: Option<String>,
}

/// POST captured pane data to tf-web.
///
/// Sends a batch of pane updates to the `/api/panes` endpoint.
/// The tf-web server merges these into its pane cache.
#[allow(dead_code)]
pub async fn post_panes(url: &str, token: &str, panes: &[PaneData]) -> Result<()> {
    let client = Client::new();

    let endpoint = format!("{}/api/ingest", url.trim_end_matches('/'));

    let response = client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&PaneUpdateRequest {
            panes: panes.to_vec(),
        })
        .send()
        .await
        .context("Failed to send request to tf-web")?;

    let status = response.status();
    let body: ApiResponse = response
        .json()
        .await
        .context("Failed to parse tf-web response")?;

    if !status.is_success() || !body.success {
        let error_msg = body.error.unwrap_or_else(|| format!("HTTP {}", status));
        anyhow::bail!("tf-web returned error: {}", error_msg);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pane_data_serialization() {
        let pane = PaneData {
            pane_id: "win-12345".to_string(),
            title: "PowerShell".to_string(),
            lines: vec!["Hello, world!".to_string(), "Prompt>".to_string()],
            width: 80,
            height: 24,
            source: "windows-capture".to_string(),
            process_id: 12345,
        };

        let json = serde_json::to_string(&pane).unwrap();
        assert!(json.contains("win-12345"));
        assert!(json.contains("PowerShell"));
        assert!(json.contains("windows-capture"));
    }

    #[test]
    fn test_pane_update_request_serialization() {
        let panes = vec![PaneData {
            pane_id: "win-12345".to_string(),
            title: "PowerShell".to_string(),
            lines: vec![],
            width: 80,
            height: 24,
            source: "windows-capture".to_string(),
            process_id: 12345,
        }];

        let req = PaneUpdateRequest { panes };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("panes"));
    }
}
