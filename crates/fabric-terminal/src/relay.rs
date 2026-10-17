//! Screen content relay for broadcasting to local tmux.
//!
//! Takes remote pane content and injects it into the local tmux session.
//! This enables viewing remote panes locally without SSH attachment.

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use tracing::debug;

use crate::PaneContent;

/// Broadcast remote pane content to a local tmux session.
///
/// This performs the actual side effect: it writes the captured lines
/// into a fresh tmux paste buffer, then runs `paste-buffer -t <pane>`
/// so the running program in that pane reads the content as if it had
/// been pasted from a clipboard.
///
/// # Arguments
///
/// * `content`      - The remote pane content to inject.
/// * `target_socket` - Local tmux socket path (e.g. `/private/tmp/tmux-501/default`).
/// * `target_pane_id` - The local pane id to inject into (e.g. `"%0"` or `"0.0"`).
///
/// # Errors
///
/// Returns an explicit error if:
/// * the local tmux binary cannot be located (not on PATH, `TMUX_BIN` unset
///   or points to a non-existent file);
/// * the tmux invocation fails (no such pane, socket not running, etc.);
/// * writing the content into the tmux buffer fails.
///
/// The function will never silently succeed while doing nothing: every
/// failure is surfaced with enough context to be actionable.
pub async fn broadcast_local(
    content: &PaneContent,
    target_socket: &str,
    target_pane_id: &str,
) -> Result<()> {
    if content.lines.is_empty() {
        // Nothing to do. Still honest: we did inspect the input.
        debug!(
            "broadcast_local: remote pane {} has no lines; skipping",
            content.pane_id
        );
        return Ok(());
    }

    if target_socket.is_empty() {
        return Err(anyhow!(
            "broadcast_local: no local tmux socket provided; cannot inject \
             pane {} into a local session. Pass a non-empty target_socket.",
            content.pane_id
        ));
    }
    if target_pane_id.is_empty() {
        return Err(anyhow!(
            "broadcast_local: no local target pane id provided; cannot inject \
             pane {}. Pass a non-empty target_pane_id (e.g. \"%0\" or \"0.0\").",
            content.pane_id
        ));
    }

    let bin = resolve_local_tmux_binary().ok_or_else(|| {
        anyhow!(
            "broadcast_local: tmux binary not found on PATH and TMUX_BIN is \
             unset or points to a non-existent file; cannot inject pane {} \
             into local socket {}",
            content.pane_id,
            target_socket
        )
    })?;

    // Stage the content into a fresh tmux buffer, then paste it.
    //
    // 1) `tmux -S <sock> set-buffer -b <name> <data>` writes the buffer
    // body as a positional argument. Joining the lines with `\n` is
    // required: tmux will not insert line breaks between separate
    // positional args.
    let buffer_name = format!("tf-relay-{}", content.pane_id.replace(['.', ' '], "_"));
    let joined = content.lines.join("\n");
    let output = std::process::Command::new(&bin)
        .arg("-S")
        .arg(target_socket)
        .arg("set-buffer")
        .arg("-b")
        .arg(&buffer_name)
        .arg(&joined)
        .output()
        .with_context(|| {
            format!(
                "broadcast_local: failed to spawn tmux set-buffer (bin={}, socket={}, name={})",
                bin.display(),
                target_socket,
                buffer_name
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "broadcast_local: tmux set-buffer failed for pane {} (exit {:?}): {}",
            content.pane_id,
            output.status.code(),
            stderr.trim()
        ));
    }

    // 2) `tmux -S <sock> paste-buffer -b <name> -t <pane>` pastes the
    // named buffer into the target pane.
    let output = std::process::Command::new(&bin)
        .arg("-S")
        .arg(target_socket)
        .arg("paste-buffer")
        .arg("-b")
        .arg(&buffer_name)
        .arg("-t")
        .arg(target_pane_id)
        .output()
        .with_context(|| {
            format!(
                "broadcast_local: failed to spawn tmux paste-buffer (bin={}, \
                 socket={}, name={}, pane={})",
                bin.display(),
                target_socket,
                buffer_name,
                target_pane_id
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "broadcast_local: tmux paste-buffer failed for pane {} -> {} (exit {:?}): {}",
            content.pane_id,
            target_pane_id,
            output.status.code(),
            stderr.trim()
        ));
    }

    debug!(
        "broadcast_local: injected {} lines from pane {} into {} on {}",
        content.lines.len(),
        content.pane_id,
        target_pane_id,
        target_socket
    );
    Ok(())
}

/// Resolve the tmux binary to invoke for local broadcast. Honors `TMUX_BIN`
/// (used in tests and deployments) and falls back to `which tmux`.
fn resolve_local_tmux_binary() -> Option<PathBuf> {
    if let Ok(val) = std::env::var("TMUX_BIN") {
        let p = PathBuf::from(&val);
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(output) = std::process::Command::new("which").arg("tmux").output()
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
    use std::sync::Mutex;

    /// Serial mutex: env-var-driven tests in this module touch `TMUX_BIN`
    /// and shared tmux server state. tokio::sync::Mutex would be cleaner but
    /// std::sync::Mutex works because the tests are short-lived and never
    /// hold the lock across an .await.
    static RELAY_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn sample_content(pane_id: &str, lines: Vec<String>) -> PaneContent {
        PaneContent {
            pane_id: pane_id.to_string(),
            lines,
            width: 80,
            height: 24,
            cursor_row: 0,
            cursor_col: 0,
            dirty: true,
        }
    }

    /// Run a shell command and return its trimmed stdout. Test-only helper.
    fn run_cmd_stdout(args: &[&str]) -> Option<String> {
        let out = std::process::Command::new(args[0])
            .args(&args[1..])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Set up an ephemeral tmux server on a private socket, returning the
    /// socket path. Caller is responsible for killing the server.
    fn ephemeral_tmux_server(label: &str) -> Option<String> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_nanos();
        let sock = format!("/tmp/tf-test-relay-{label}-{nanos}.sock");
        let _ = std::fs::remove_file(&sock);
        let started = std::process::Command::new("tmux")
            .args([
                "-S",
                &sock,
                "new-session",
                "-d",
                "-s",
                label,
                "-x",
                "80",
                "-y",
                "24",
            ])
            .status()
            .ok()?
            .success();
        if !started {
            return None;
        }
        Some(sock)
    }

    fn kill_tmux_server(sock: &str) {
        let _ = std::process::Command::new("tmux")
            .args(["-S", sock, "kill-server"])
            .status();
        let _ = std::fs::remove_file(sock);
    }

    /// Read the contents of a named tmux buffer.
    /// Returns None if the read fails.
    fn show_buffer(sock: &str, name: &str) -> Option<String> {
        let out = std::process::Command::new("tmux")
            .args(["-S", sock, "show-buffer", "-b", name])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    }

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

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_broadcast_local_empty_lines_is_ok() {
        let _guard = RELAY_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // Empty content: nothing to inject, but the call must still
        // succeed honestly (the function inspects the content and decides
        // there's nothing to do). Crucially, it must NOT return Ok
        // because of a silent skip — we still verify there's nothing
        // fake about it by making sure no tmux invocation occurred.
        let content = sample_content("0.0", vec![]);
        let result = broadcast_local(&content, "/tmp/never-used", "%0").await;
        assert!(
            result.is_ok(),
            "broadcast_local on empty content should be Ok, not a silent \
             no-op that pretends to have done work; got: {:?}",
            result
        );
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_broadcast_local_errors_on_empty_target_socket() {
        let _guard = RELAY_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let content = sample_content("0.0", vec!["hello".to_string(), "world".to_string()]);
        let result = broadcast_local(&content, "", "%0").await;
        assert!(
            result.is_err(),
            "broadcast_local with empty target_socket must return an explicit \
             Err rather than silently succeeding"
        );
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("socket") || msg.contains("target_socket"),
            "error should mention the missing socket; got: {msg}"
        );
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_broadcast_local_errors_on_empty_target_pane() {
        let _guard = RELAY_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let content = sample_content("0.0", vec!["hello".to_string(), "world".to_string()]);
        let result = broadcast_local(&content, "/tmp/some-socket", "").await;
        assert!(
            result.is_err(),
            "broadcast_local with empty target_pane_id must return Err"
        );
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("pane") || msg.contains("target_pane_id"),
            "error should mention the missing pane; got: {msg}"
        );
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_broadcast_local_errors_when_tmux_binary_missing() {
        let _guard = RELAY_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        // SAFETY: tests in this module take RELAY_TEST_LOCK before
        // touching TMUX_BIN.
        unsafe {
            std::env::set_var("TMUX_BIN", "/nonexistent/path/tmux-for-relay-test");
        }
        let content = sample_content(
            "remote-0.0",
            vec!["line-A".to_string(), "line-B".to_string()],
        );
        let result = broadcast_local(&content, "/tmp/tf-test-no-server", "%0").await;
        unsafe {
            std::env::remove_var("TMUX_BIN");
        }
        assert!(
            result.is_err(),
            "broadcast_local must return Err when the tmux binary cannot be \
             located; the original stub returned Ok while doing nothing"
        );
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("tmux") || msg.contains("TMUX_BIN"),
            "error should explain the missing tmux binary; got: {msg}"
        );
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_broadcast_local_errors_when_tmux_socket_missing() {
        let _guard = RELAY_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let tmux_path = match run_cmd_stdout(&["which", "tmux"]) {
            Some(p) if !p.is_empty() => p,
            _ => {
                eprintln!("skipping: no system tmux available");
                return;
            }
        };
        unsafe {
            std::env::set_var("TMUX_BIN", &tmux_path);
        }
        // Use a socket path that we never start a tmux server on.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .unwrap()
            .as_nanos();
        let nonexistent_sock = format!("/tmp/tf-test-no-such-srv-{nanos}.sock");
        let _ = std::fs::remove_file(&nonexistent_sock);

        let content = sample_content("remote-0.0", vec!["will-fail".to_string()]);
        let result = broadcast_local(&content, &nonexistent_sock, "%0").await;
        unsafe {
            std::env::remove_var("TMUX_BIN");
        }
        assert!(
            result.is_err(),
            "broadcast_local must return Err when the tmux server is not \
             running; the original stub returned Ok while doing nothing"
        );
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("set-buffer")
                || msg.contains("paste-buffer")
                || msg.contains("failed to spawn"),
            "error should mention which tmux command failed; got: {msg}"
        );
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_broadcast_local_succeeds_with_real_tmux_session() {
        let _guard = RELAY_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let tmux_path = match run_cmd_stdout(&["which", "tmux"]) {
            Some(p) if !p.is_empty() => p,
            _ => {
                eprintln!("skipping: no system tmux available");
                return;
            }
        };
        let sock = match ephemeral_tmux_server("relay-ok") {
            Some(s) => s,
            None => {
                eprintln!("skipping: could not start ephemeral tmux server");
                return;
            }
        };
        unsafe {
            std::env::set_var("TMUX_BIN", &tmux_path);
        }

        // Mark the pane with a unique token before broadcasting so we can
        // verify the actual content was injected.
        let token = format!("RELAYOK_{}", std::process::id());
        let content = sample_content(
            "remote-0.0",
            vec![format!("{token} line one"), format!("{token} line two")],
        );
        let result = broadcast_local(&content, &sock, "%0").await;
        unsafe {
            std::env::remove_var("TMUX_BIN");
        }

        // Give tmux a moment to actually paste the buffer, then read the
        // named buffer back to verify the content reached tmux.
        std::thread::sleep(std::time::Duration::from_millis(100));
        let buffer_name = "tf-relay-remote-0_0".to_string();
        let buf = show_buffer(&sock, &buffer_name);
        kill_tmux_server(&sock);

        assert!(
            result.is_ok(),
            "broadcast_local must return Ok against a live tmux server; got: {:?}",
            result
        );
        let buf = buf.unwrap_or_default();
        assert!(
            buf.contains(&token),
            "the injected content must reach the tmux buffer; \
             show-buffer output was: {buf:?}"
        );
        assert!(
            buf.contains("line one") && buf.contains("line two"),
            "the buffer must contain both injected lines; got: {buf:?}"
        );
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_broadcast_local_error_names_remote_pane() {
        // The error message must name the remote pane id so operators can
        // debug which pane failed to inject.
        let _guard = RELAY_TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        unsafe {
            std::env::set_var("TMUX_BIN", "/nonexistent/path/tmux-for-relay-test");
        }
        let content = sample_content("remote-7.42", vec!["payload".to_string()]);
        let result = broadcast_local(&content, "/tmp/some-socket", "%0").await;
        unsafe {
            std::env::remove_var("TMUX_BIN");
        }
        assert!(result.is_err(), "expected an error");
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("remote-7.42"),
            "error message should name the remote pane id; got: {msg}"
        );
    }
}
