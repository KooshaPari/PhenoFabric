//! UI Automation-based content capture for Windows Terminal.
//!
//! Windows Terminal uses ConPTY (pseudo-console) which prevents traditional
//! `ReadConsoleOutputCharacterW` capture. This module uses the Windows UI
//! Automation API with ITextProvider to read terminal content via
//! accessibility interfaces (the same mechanism used by screen readers).
//!
//! Architecture:
//!   WT HWND -> XAML Island -> TerminalControl -> ITextProvider -> GetText()

/// Maximum number of lines to capture.
#[allow(dead_code)]
const MAX_LINES: usize = 200;

#[cfg(target_os = "windows")]
mod os_impl;

/// Trim trailing whitespace from each line and remove trailing empty lines.
/// Terminal buffers are padded with spaces, making 907 lines when only ~30 are real.
#[cfg(target_os = "windows")]
fn trim_terminal_content(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let trimmed: Vec<String> = lines.iter().map(|l| l.trim_end().to_string()).collect();
    let mut end = trimmed.len();
    while end > 0 && trimmed[end - 1].is_empty() {
        end -= 1;
    }
    trimmed[..end].join("\n")
}

/// Information about a single Windows Terminal tab found via UIA.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
#[derive(Debug, Clone)]
pub struct TabInfo {
    pub index: usize,
    pub title: String,
}

/// Attempt UIA-based capture of a Windows Terminal window.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn capture_windows_terminal(hwnd: isize) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        os_impl::capture_via_uia(hwnd).map(|t| trim_terminal_content(&t))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = hwnd;
        None
    }
}

/// Capture all tabs in a Windows Terminal window by clicking each tab via UIA.
/// Returns (tab_title, content) for each tab that yielded content.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn capture_all_tabs_uia(wt_hwnd: isize) -> Vec<(TabInfo, String)> {
    #[cfg(target_os = "windows")]
    {
        os_impl::capture_all_tabs_via_uia(wt_hwnd)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = wt_hwnd;
        Vec::new()
    }
}

/// Enumerate all visible Windows Terminal windows and attempt UIA capture.
/// Returns list of (hwnd, title, captured_content) for each terminal.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn capture_all_terminals() -> Vec<(isize, String, Option<String>)> {
    #[cfg(target_os = "windows")]
    {
        os_impl::enumerate_and_capture()
    }
    #[cfg(not(target_os = "windows"))]
    {
        Vec::new()
    }
}
