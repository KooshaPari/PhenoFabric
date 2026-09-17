//! Background tmux capture task for tf-web.
//!
//! Periodically runs `tf-mux list-panes` and `tf-mux capture <pane_id>` to
//! populate the pane cache and broadcast changes to connected WebSocket clients.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::{AppState, CachedPane};

/// Parsed pane info from `tf-mux list-panes` output.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PaneEntry {
    pub pane_id: String,
    pub tab_id: String,
}

/// Locate the `tf-mux` binary.
///
/// Resolution order:
/// 1. `TMUX_BIN` env var
/// 2. `zig/zig-out/bin/tf-mux` relative to current directory
/// 3. Standard zig build locations relative to current directory
/// 4. `tf-mux` on system PATH via `which`
pub(crate) fn find_tf_mux_binary() -> Option<PathBuf> {
    // 1. Explicit env var
    if let Ok(val) = std::env::var("TMUX_BIN") {
        let p = PathBuf::from(&val);
        if p.exists() {
            return Some(p);
        }
        tracing::warn!("TMUX_BIN set to {:?} but file not found", val);
    }

    // 2. Relative to cwd (typical development layout)
    if let Ok(cwd) = std::env::current_dir() {
        let candidates: &[&str] = &[
            "zig/zig-out/bin/tf-mux",
            "zig/zig-out/tf-mux",
            "target/debug/tf-mux",
            "target/release/tf-mux",
        ];
        for rel in candidates {
            let path = cwd.join(rel);
            if path.exists() {
                return Some(path);
            }
        }
    }

    // 3. System PATH
    if let Ok(output) = std::process::Command::new("which").arg("tf-mux").output()
        && output.status.success()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let path = PathBuf::from(stdout.trim());
        if path.exists() {
            return Some(path);
        }
    }

    None
}

/// Parse tab-separated output from `tf-mux list-panes`.
///
/// Expected format per line: `<pane_id>\t<window_name>`
pub(crate) fn parse_list_panes_output(output: &str) -> Vec<PaneEntry> {
    let mut panes = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 && !parts[0].is_empty() {
            panes.push(PaneEntry {
                pane_id: parts[0].to_string(),
                tab_id: parts[1].to_string(),
            });
        }
    }
    panes
}

/// Blocking capture loop. Runs on a dedicated OS thread.
///
/// - Every `poll_ms` milliseconds, runs `tf-mux list-panes` to get pane list.
/// - For each pane, runs `tf-mux capture <pane_id>` and stores result in cache.
pub(crate) fn run_capture_loop_blocking(state: Arc<AppState>, poll_ms: u64) {
    let tmux_bin = find_tf_mux_binary();
    let bin = match tmux_bin {
        Some(p) => p,
        None => {
            tracing::warn!(
                "tf-mux binary not found (set TMUX_BIN env var). \
                 Capture loop disabled."
            );
            return;
        }
    };

    let broadcast_tx = state.broadcast_tx.clone();

    tracing::info!(
        "tmux capture loop started (blocking): bin={:?}, poll={}ms",
        bin,
        poll_ms,
    );

    loop {
        std::thread::sleep(Duration::from_millis(poll_ms));

        // --- list-panes ---
        let result = std::process::Command::new(&bin)
            .arg("list-panes")
            .env("TMUX_SOCKET", &state.socket_path)
            .output();

        let (ok, stdout, stderr) = match result {
            Ok(o) => (
                o.status.success(),
                String::from_utf8_lossy(&o.stdout).into_owned(),
                String::from_utf8_lossy(&o.stderr).into_owned(),
            ),
            Err(e) => {
                tracing::error!("Failed to execute {:?}: {}", bin, e);
                continue;
            }
        };

        if !ok {
            tracing::debug!(
                "list-panes returned non-zero: stderr={}",
                stderr.chars().take(200).collect::<String>()
            );
            continue;
        }

        let entries = parse_list_panes_output(&stdout);
        if entries.is_empty() {
            tracing::debug!("list-panes returned 0 panes, stdout_len={}", stdout.len());
            continue;
        }

        tracing::info!("capture loop: found {} panes", entries.len());

        // --- capture each pane ---
        let mut panes: HashMap<String, CachedPane> =
            HashMap::with_capacity(entries.len());

        for entry in &entries {
            let cap_result = std::process::Command::new(&bin)
                .arg("capture")
                .arg(&entry.pane_id)
                .env("TMUX_SOCKET", &state.socket_path)
                .output();

            let capture_out = match cap_result {
                Ok(o) if o.status.success() => {
                    String::from_utf8_lossy(&o.stdout).into_owned()
                }
                _ => {
                    tracing::debug!(
                        "capture failed for pane {}, skipping",
                        entry.pane_id
                    );
                    continue;
                }
            };

            let lines: Vec<String> = capture_out
                .lines()
                .map(|l| l.to_string())
                .collect();
            let height = lines.len() as u32;
            let width = lines.iter().map(|l| l.len() as u32).max().unwrap_or(0);

            panes.insert(
                entry.pane_id.clone(),
                CachedPane {
                    pane_id: entry.pane_id.clone(),
                    workspace_id: "local".to_string(),
                    tab_id: entry.tab_id.clone(),
                    lines,
                    width,
                    height,
                    cursor_row: 0,
                    cursor_col: 0,
                    updated_at: now_epoch_secs(),
                    source: "tmux".to_string(),
                },
            );
        }

        // --- update cache and broadcast ---
        if let Ok(mut cache) = state.pane_cache.write() {
            *cache = panes;
            tracing::debug!("capture loop: pane cache updated");
        }
        // Broadcast a "all" message so WebSocket clients know to re-fetch
        let _ = broadcast_tx.send("all".to_string());
    }
}

fn now_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_output() {
        assert!(parse_list_panes_output("").is_empty());
        assert!(parse_list_panes_output("   \n  \n").is_empty());
    }

    #[test]
    fn test_parse_single_pane() {
        let out = "%0\tmain\n";
        let panes = parse_list_panes_output(out);
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].pane_id, "%0");
        assert_eq!(panes[0].tab_id, "main");
    }

    #[test]
    fn test_parse_multiple_panes() {
        let out = "%0\tmain\n%1\tmain\n%2\teditor\n";
        let panes = parse_list_panes_output(out);
        assert_eq!(panes.len(), 3);
        assert_eq!(panes[0].pane_id, "%0");
        assert_eq!(panes[1].pane_id, "%1");
        assert_eq!(panes[2].pane_id, "%2");
        assert_eq!(panes[2].tab_id, "editor");
    }

    #[test]
    fn test_parse_ignores_malformed_lines() {
        let out = "%0\tmain\nbadline\n%1\tok\n";
        let panes = parse_list_panes_output(out);
        assert_eq!(panes.len(), 2);
        assert_eq!(panes[0].pane_id, "%0");
        assert_eq!(panes[1].pane_id, "%1");
    }

    #[test]
    fn test_parse_extra_tabs_ignored() {
        let out = "%0\tmain\textra_data\n";
        let panes = parse_list_panes_output(out);
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].pane_id, "%0");
        assert_eq!(panes[0].tab_id, "main");
    }

    #[test]
    fn test_parse_no_tab_line_ignored() {
        let out = "just_text\n%0\tmain\n";
        let panes = parse_list_panes_output(out);
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].pane_id, "%0");
    }

    #[test]
    fn test_parse_leading_trailing_whitespace() {
        let out = "  %0\tmain  \n";
        let panes = parse_list_panes_output(out);
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].pane_id, "%0");
        assert_eq!(panes[0].tab_id, "main");
    }

    #[test]
    fn test_find_binary_not_found() {
        // When TMUX_BIN points to nonexistent file, should return None
        // (unless tf-mux happens to be in zig output)
        // We just verify the function doesn't panic
        let _ = find_tf_mux_binary();
    }

    #[test]
    fn test_run_capture_loop_missing_binary() {
        // With no tf-mux available, the loop should return immediately
        // without panicking or hanging
        let (broadcast_tx, _) = tokio::sync::broadcast::channel::<String>(16);
        let state = Arc::new(AppState {
            agents: tokio::sync::RwLock::new(HashMap::new()),
            connections: tokio::sync::RwLock::new(HashMap::new()),
            pane_cache: std::sync::RwLock::new(HashMap::new()),
            broadcast_tx,
            socket_path: "/tmp/nonexistent-tmux-socket".to_string(),
            tmux_bin: std::path::PathBuf::from("/nonexistent/tf-mux"),
            pane_cache_path: std::env::temp_dir().join("tf-test-panes.json"),
        });

        // Set TMUX_BIN to nonexistent path so loop exits immediately
        // SAFETY: This test runs single-threaded; no concurrent env access.
        unsafe { std::env::set_var("TMUX_BIN", "/nonexistent/path/tf-mux"); }
        let handle = std::thread::spawn(move || {
            run_capture_loop_blocking(state, 100);
        });
        // The function should return immediately since binary doesn't exist
        handle.join().expect("capture loop should exit cleanly");
        unsafe { std::env::remove_var("TMUX_BIN"); }
    }
}
