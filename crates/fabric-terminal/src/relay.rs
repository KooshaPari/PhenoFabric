//! Screen content relay for broadcasting to local tmux.
//!
//! Takes remote pane content and injects it into the local tmux session.
//! This enables viewing remote panes locally without SSH attachment.

use anyhow::Result;
use tracing::debug;

use crate::PaneContent;

/// Broadcast remote pane content to a local tmux session.
pub async fn broadcast_local(content: &PaneContent) -> Result<()> {
    // The simplest approach: write the content to a tmux pane
    // using send-keys with the content
    debug!(
        "Broadcasting pane {} ({} lines) to local",
        content.pane_id,
        content.lines.len()
    );

    // For now, we just log the content
    // In production, this would:
    // 1. Find the matching local pane (by mapping remote -> local)
    // 2. Clear the local pane
    // 3. Write the remote content line by line
    // 4. Set the cursor position

    Ok(())
}

/// Map a remote pane ID to a local pane ID.
#[allow(dead_code)]
pub fn map_pane_id(remote_id: &str, pane_map: &[(String, String)]) -> Option<String> {
    for (remote, local) in pane_map {
        if remote == remote_id {
            return Some(local.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_pane_id() {
        let mapping = vec![
            ("0.0".to_string(), "0.0".to_string()),
            ("0.1".to_string(), "0.1".to_string()),
        ];

        assert_eq!(map_pane_id("0.0", &mapping), Some("0.0".to_string()));
        assert_eq!(map_pane_id("0.1", &mapping), Some("0.1".to_string()));
        assert_eq!(map_pane_id("0.2", &mapping), None);
    }
}
