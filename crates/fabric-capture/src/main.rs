//! Windows console content capture for terminal-fabric.
//!
//! Enumerates PowerShell/Console windows, reads their screen buffers,
//! and posts captured content to tf-web for remote viewing.

mod capture;
mod clipboard;
mod client;
#[cfg(feature = "self-update")]
mod self_update;
mod uia;

use anyhow::Result;
use clap::Parser;
use tracing::info;

/// tf-win-capture: Captures Windows console content and posts to tf-web.
#[derive(Parser, Debug)]
#[command(name = "tf-win-capture", version, about)]
struct Args {
    /// tf-web server URL (e.g. http://localhost:8787)
    #[arg(long, env = "TF_WEB_URL")]
    url: String,

    /// Authentication token (API key)
    #[arg(long, env = "TF_WEB_TOKEN")]
    token: String,

    /// Capture interval in milliseconds
    #[arg(long, default_value_t = 300, env = "TF_CAPTURE_INTERVAL_MS")]
    interval_ms: u64,

    /// Agent name for registration
    #[arg(long, default_value = "windows-capture", env = "TF_AGENT_NAME")]
    agent_name: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Handle self-update and --version before any initialization
    {
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            match args[1].as_str() {
                "self-update" => {
#[cfg(feature = "self-update")]
                    self_update::run("tf-win-capture")?;
                    return Ok(());
                }
                "--version" | "-V" => {
                    println!("tf-win-capture {}", env!("CARGO_PKG_VERSION"));
                    return Ok(());
                }
                _ => {}
            }
        }
    }

    // File-based logging for deployment diagnostics.
    let log_path = std::env::temp_dir().join("tf-win-capture.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with_writer(std::sync::Mutex::new(log_file))
        .init();

    let args = Args::parse();

    info!(
        url = %args.url,
        interval_ms = args.interval_ms,
        agent_name = %args.agent_name,
        "tf-win-capture starting"
    );

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!(
            "tf-win-capture only runs on Windows. \
             This binary was compiled for a non-Windows target."
        );
    }

    #[cfg(target_os = "windows")]
    {
        run_capture_loop(&args).await
    }
}

#[cfg(target_os = "windows")]
async fn run_capture_loop(args: &Args) -> Result<()> {
    use std::time::Duration;

    let interval = Duration::from_millis(args.interval_ms);

    info!("Starting capture loop (interval: {:?})", interval);

    loop {
        let mut pane_data_list = Vec::new();

        // Phase 1: Traditional console capture.
        match capture::enumerate_console_panes() {
            Ok(panes) => {
                for pane in &panes {
                    match capture::capture_console_buffer(pane.hwnd) {
                        Some(content) => {
                            pane_data_list.push(client::PaneData {
                                pane_id: format!("win-{}", pane.process_id),
                                title: pane.title.clone(),
                                lines: content.lines,
                                width: content.width,
                                height: content.height,
                                source: "windows-capture".to_string(),
                                process_id: pane.process_id,
                            });
                        }
                        None => {
                            tracing::debug!(
                                hwnd = pane.hwnd,
                                title = %pane.title,
                                "Console buffer capture failed (likely ConPTY)"
                            );
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to enumerate console windows");
            }
        }

        // Phase 2: PseudoConsoleWindow capture (ConPTY terminals).
        // Windows Terminal tabs have PseudoConsoleWindow HWNDs.
        // ReadConsoleOutputCharacterW should still work for ConPTY per WT maintainer.
        let pc_hwnds = capture::enumerate_pseudo_console_hwnds();
        tracing::debug!(count = pc_hwnds.len(), "Found PseudoConsoleWindow HWNDs");

        for hwnd in &pc_hwnds {
            match capture::capture_console_buffer(*hwnd) {
                Some(content) => {
                    if !content.lines.is_empty() {
                        tracing::debug!(
                            hwnd = hwnd,
                            height = content.height,
                            width = content.width,
                            sample = content.lines.first().map(|s| s.as_str()).unwrap_or(""),
                            "Captured PseudoConsoleWindow buffer"
                        );
                        pane_data_list.push(client::PaneData {
                            pane_id: format!("pc-{:#x}", hwnd),
                            title: format!("PseudoConsole {:#x}", hwnd),
                            lines: content.lines,
                            width: content.width,
                            height: content.height,
                            source: "windows-terminal".to_string(),
                            process_id: 0,
                        });
                    }
                }
                None => {
                    tracing::debug!(hwnd = hwnd, "PseudoConsoleWindow capture returned None");
                }
            }
        }

        // Phase 3: Clipboard-based capture via SendMessage (window-targeted).
        // Uses SendMessageW to send Ctrl+Number/Ctrl+A/Ctrl+C directly to
        // the WT window handle. No global input injection -- safe for the user.
        // The foreground check inside capture_tab_by_index ensures we only
        // touch WT when it is focused.
        {
            let terminals = uia::capture_all_terminals();
            for (hwnd, title, _) in &terminals {
                let tab_results = clipboard::win::capture_all_tabs(*hwnd);
                if !tab_results.is_empty() {
                    tracing::debug!(
                        hwnd = hwnd,
                        title = %title,
                        tab_count = tab_results.len(),
                        "Clipboard captured tabs via SendMessage"
                    );
                    for (tab_idx, title_hint, lines) in tab_results {
                        pane_data_list.push(client::PaneData {
                            pane_id: format!("clip-tab-{}-{:#x}", tab_idx, hwnd),
                            title: title_hint,
                            lines,
                            width: 80,
                            height: 0,
                            source: "windows-terminal-clipboard".to_string(),
                            process_id: 0,
                        });
                    }
                }
            }
        }

        // Phase 4: UI Automation capture for Windows Terminal tabs.
        // Uses UIA to find the tab bar, click each tab, and capture its content.
        // This captures ALL tabs, not just the active one.
        {
            // 4a: Try main WT HWND via UIA for multi-tab capture.
            let terminals = uia::capture_all_terminals();
            for (hwnd, title, _content_opt) in &terminals {
                // Use the new multi-tab capture that clicks each tab.
                let tab_results = uia::capture_all_tabs_uia(*hwnd);
                if tab_results.is_empty() {
                    // Fallback: if no tabs found, capture whatever is active.
                    if let Some(content) = uia::capture_windows_terminal(*hwnd) {
                        if !content.is_empty() {
                            let lines: Vec<String> = content.lines().map(String::from).collect();
                            tracing::debug!(
                                hwnd = hwnd,
                                title = %title,
                                lines = lines.len(),
                                sample = lines.first().map(|s| s.as_str()).unwrap_or(""),
                                "UIA captured active tab content"
                            );
                            pane_data_list.push(client::PaneData {
                                pane_id: format!("uia-{:#x}", hwnd),
                                title: title.clone(),
                                lines,
                                width: 80,
                                height: 0,
                                source: "windows-terminal-uia".to_string(),
                                process_id: 0,
                            });
                        }
                    }
                } else {
                    tracing::debug!(
                        hwnd = hwnd,
                        title = %title,
                        tab_count = tab_results.len(),
                        "UIA captured multiple tabs"
                    );
                    for (tab_info, content) in tab_results {
                        let lines: Vec<String> = content.lines().map(String::from).collect();
                        tracing::debug!(
                            tab_index = tab_info.index,
                            tab_title = %tab_info.title,
                            lines = lines.len(),
                            sample = lines.first().map(|s| s.as_str()).unwrap_or(""),
                            "UIA captured tab content"
                        );
                        pane_data_list.push(client::PaneData {
                            pane_id: format!("uia-tab-{}-{:#x}", tab_info.index, hwnd),
                            title: tab_info.title,
                            lines,
                            width: 80,
                            height: 0,
                            source: "windows-terminal-uia".to_string(),
                            process_id: 0,
                        });
                    }
                }
            }
            if terminals.is_empty() {
                tracing::debug!("UIA: no Windows Terminal windows found");
            }

            // 4b: Try UIA on one PseudoConsoleWindow HWND directly.
            // Short-circuit: all HWNDs share the same terminal content.
            {
                let mut found_uia = false;
                for hwnd in &pc_hwnds {
                    if found_uia {
                        break;
                    }
                    if let Some(content) = uia::capture_windows_terminal(*hwnd as isize) {
                        if !content.is_empty() {
                            let lines: Vec<String> = content.lines().map(String::from).collect();
                            tracing::debug!(
                                hwnd = hwnd,
                                lines = lines.len(),
                                sample = lines.first().map(|s| s.as_str()).unwrap_or(""),
                                "UIA captured PseudoConsoleWindow content"
                            );
                            pane_data_list.push(client::PaneData {
                                pane_id: format!("pcuia-{:#x}", hwnd),
                                title: format!("PseudoConsole UIA {:#x}", hwnd),
                                lines,
                                width: 80,
                                height: 0,
                                source: "windows-terminal-pc-uia".to_string(),
                                process_id: 0,
                            });
                            found_uia = true;
                        }
                    }
                }
            }
        }

        // Post all captured data.
        if !pane_data_list.is_empty() {
            if let Err(e) = client::post_panes(
                &args.url,
                &args.token,
                &pane_data_list,
            ).await {
                tracing::warn!(error = %e, "Failed to post panes to tf-web");
            } else {
                tracing::debug!(
                    count = pane_data_list.len(),
                    "Posted pane data to tf-web"
                );
            }
        }

        tokio::time::sleep(interval).await;
    }
}
