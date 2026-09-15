//! Sync state management for tracking screen changes.
//!
//! Maintains the last known state of each pane and detects changes.
//! This is what enables efficient diff-based synchronization.

use crate::PaneContent;
use std::collections::HashMap;

/// Tracks screen state across sync cycles.
pub struct SyncState {
    last_capture: HashMap<String, Vec<String>>,
    sequence: u64,
}

impl SyncState {
    pub fn new() -> Self {
        Self {
            last_capture: HashMap::new(),
            sequence: 0,
        }
    }

    /// Update with new pane content. Returns true if content changed.
    pub fn update(&mut self, content: &PaneContent) -> bool {
        let prev = self.last_capture.get(&content.pane_id);

        let changed = match prev {
            Some(old) => old != &content.lines,
            None => true,
        };

        if changed {
            self.last_capture
                .insert(content.pane_id.clone(), content.lines.clone());
            self.sequence += 1;
        }

        changed
    }

#[allow(dead_code)]
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Get all tracked pane IDs.
    #[allow(dead_code)]
    pub fn pane_ids(&self) -> Vec<&str> {
        self.last_capture.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_is_empty() {
        let state = SyncState::new();
        assert_eq!(state.sequence(), 0);
        assert!(state.pane_ids().is_empty());
    }

    #[test]
    fn test_first_update_always_changes() {
        let mut state = SyncState::new();
        let content = PaneContent {
            pane_id: "0.0".to_string(),
            lines: vec!["Hello".to_string()],
            width: 80,
            height: 24,
            cursor_row: 0,
            cursor_col: 5,
            dirty: true,
        };

        assert!(state.update(&content));
        assert_eq!(state.sequence(), 1);
    }

    #[test]
    fn test_same_content_no_change() {
        let mut state = SyncState::new();
        let content = PaneContent {
            pane_id: "0.0".to_string(),
            lines: vec!["Hello".to_string()],
            width: 80,
            height: 24,
            cursor_row: 0,
            cursor_col: 5,
            dirty: true,
        };

        state.update(&content.clone());
        assert!(!state.update(&content));
        assert_eq!(state.sequence(), 1);
    }

    #[test]
    fn test_different_content_detected() {
        let mut state = SyncState::new();
        let mut content = PaneContent {
            pane_id: "0.0".to_string(),
            lines: vec!["Hello".to_string()],
            width: 80,
            height: 24,
            cursor_row: 0,
            cursor_col: 5,
            dirty: true,
        };

        state.update(&content.clone());
        content.lines = vec!["World".to_string()];
        assert!(state.update(&content));
        assert_eq!(state.sequence(), 2);
    }
}
