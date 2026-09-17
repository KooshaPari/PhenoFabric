use uiautomation::core::UIAutomation;
use uiautomation::types::ControlType;

use super::super::TabInfo;
use super::capture::capture_active_tab_via_uia;
use super::extract::diag;

/// Try to infer a descriptive tab title from terminal content.
/// Terminal programs display shell name + working directory in the title bar.
/// The current prompt is always near the BOTTOM of the terminal buffer.
/// We check the last 20 non-empty lines for common prompt patterns:
///   bash/zsh:  user@host: ~/path$
///   PowerShell: PS C:\path>
///   cmd:       C:\path>
///   conda:     (envname) PS C:\path>
fn infer_title_from_content(hwnd: isize, content: &str) -> Option<String> {
    // Search ALL non-empty lines in reverse to find the last prompt pattern.
    // The prompt can be anywhere — after old output, before capture agent output, etc.
    let non_empty: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

    // Log a few lines around the end for debugging.
    let total = non_empty.len();
    for i in (total.saturating_sub(5)..total).rev() {
        diag(
            hwnd,
            &format!(
                "  infer: line[{}/{}]: \"{}\"",
                i,
                total,
                &non_empty[i].chars().take(120).collect::<String>()
            ),
        );
    }

    // Search in reverse — the last matching prompt is the current one.
    for line in non_empty.iter().rev() {
        let trimmed = line.trim();
        if trimmed.len() < 3 {
            continue;
        }

        // PowerShell: "PS C:\Users\koosh>" or "(envname) PS C:\path>"
        // May have content after the prompt: "(base) PS C:\Users\koosh> 45!231..."
        if let Some(ps_pos) = trimmed.rfind("PS ") {
            let prefix = if ps_pos > 0 { &trimmed[..ps_pos] } else { "" };
            let path_part = &trimmed[ps_pos + 3..];
            // Find the closing > that marks end of path
            if let Some(gt_pos) = path_part.find('>') {
                let cwd = path_part[..gt_pos].trim();
                let display = format!("{}PS {}", prefix, cwd);
                if display.len() > 3 && display.len() < 80 {
                    return Some(display);
                }
            }
        }

        // Git Bash with MSYS2-style prompt: "koosh@DESKTOP-NAME: ~/path>"
        // or just "koosh@DESKTOP:~/path>"
        if trimmed.contains('@') && trimmed.ends_with('>') && trimmed.contains(':') {
            if let Some(at_pos) = trimmed.find('@') {
                let user = &trimmed[..at_pos];
                let rest = &trimmed[at_pos + 1..];
                if let Some(colon_pos) = rest.find(':') {
                    let cwd = rest[colon_pos + 1..].trim_end_matches('>').trim();
                    if !cwd.is_empty() && user.len() < 30 && cwd.len() < 80 {
                        let host = &rest[..colon_pos];
                        return Some(format!("{}@{}: {}", user, host, cwd));
                    }
                }
            }
        }

        // Bare "~>" prompt (Git Bash) — strip trailing ^C, etc.
        if trimmed.starts_with("~>") {
            return Some("~>".to_string());
        }

        // Git Bash with $: "koosh@DESKTOP:~/path$"
        if trimmed.contains('@') && trimmed.contains('$') {
            if let Some(at_pos) = trimmed.find('@') {
                let user = &trimmed[..at_pos];
                let rest = &trimmed[at_pos + 1..];
                if let Some(colon_pos) = rest.find(':') {
                    let cwd = rest[colon_pos + 1..]
                        .trim_end_matches('$')
                        .trim_end_matches('#')
                        .trim();
                    if !cwd.is_empty() && user.len() < 30 && cwd.len() < 80 {
                        let host = &rest[..colon_pos];
                        return Some(format!("{}@{}: {}", user, host, cwd));
                    }
                }
            }
        }
        // cmd.exe: "C:\Users\koosh>" — just a bare path prompt
        if trimmed.len() > 2
            && trimmed.ends_with('>')
            && trimmed.chars().nth(1) == Some(':')
            && trimmed.contains('\\')
        {
            let cwd = trimmed.trim_end_matches('>').trim();
            if cwd.len() < 80 {
                return Some(format!("cmd {}", cwd));
            }
        }
    }
    None
}

/// Click at a screen coordinate using SendInput mouse events.
fn click_at_point(x: i32, y: i32) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetSystemMetrics;

    let screen_w = unsafe { GetSystemMetrics(0) };
    let screen_h = unsafe { GetSystemMetrics(1) };
    let absolute_x = (x * 65535) / screen_w;
    let absolute_y = (y * 65535) / screen_h;

    let inputs = [
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: absolute_x,
                    dy: absolute_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE | MOUSEEVENTF_LEFTDOWN,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT {
                    dx: absolute_x,
                    dy: absolute_y,
                    mouseData: 0,
                    dwFlags: MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_MOVE | MOUSEEVENTF_LEFTUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];

    unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<INPUT>() as i32,
        );
    }
}

/// Send Ctrl+Number to switch to a specific tab in Windows Terminal.
/// Ctrl+1 switches to tab 1, Ctrl+2 to tab 2, etc.
/// Uses SendInput for hardware-level key injection.
unsafe fn send_ctrl_number(tab_index: u8) {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;

    let vk_control: u16 = 0x11;
    let vk_number: u16 = 0x30 + tab_index as u16; // VK_1 = 0x31, VK_9 = 0x39

    let inputs = [
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk_control,
                    wScan: 0x1D,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk_number,
                    wScan: (tab_index as u16 + 1),
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk_number,
                    wScan: (tab_index as u16 + 1),
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk_control,
                    wScan: 0x1D,
                    dwFlags: KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        },
    ];

    SendInput(
        inputs.len() as u32,
        inputs.as_ptr(),
        std::mem::size_of::<INPUT>() as i32,
    );
}

/// Use UIA to find the tab strip in a Windows Terminal window,
/// click each tab, and capture its content.
/// Returns (tab_title, content) for each tab that yielded content.
pub(crate) fn capture_all_tabs_via_uia(wt_hwnd: isize) -> Vec<(TabInfo, String)> {
    diag(wt_hwnd, "=== capture_all_tabs_via_uia ===");

    // SAFETY CHECK: Only capture if Windows Terminal is the foreground window.
    // This prevents mouse clicks and keyboard shortcuts from landing on whatever
    // the user is actively working in. If WT isn't focused, skip this cycle.
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        let fg = unsafe { GetForegroundWindow() };
        if fg != wt_hwnd as _ {
            diag(
                wt_hwnd,
                &format!(
                    "SKIPPING: WT not foreground (fg={}, wt={})",
                    fg as isize, wt_hwnd
                ),
            );
            return Vec::new();
        }
    }

    let automation = match UIAutomation::new() {
        Ok(a) => a,
        Err(e) => {
            diag(wt_hwnd, &format!("UIAutomation::new() FAILED: {e}"));
            return Vec::new();
        }
    };

    let handle = uiautomation::types::Handle::from(wt_hwnd);
    let root = match automation.element_from_handle(handle) {
        Ok(e) => e,
        Err(e) => {
            diag(wt_hwnd, &format!("element_from_handle FAILED: {e}"));
            return Vec::new();
        }
    };

    // Step 1: Find visible tab items via UIA.
    let tab_items = {
        let matcher = automation
            .create_matcher()
            .from_ref(&root)
            .control_type(ControlType::TabItem)
            .depth(15)
            .timeout(3000);

        match matcher.find_all() {
            Ok(items) if !items.is_empty() => {
                diag(wt_hwnd, &format!("Found {} TabItem controls", items.len()));
                items
            }
            _ => {
                diag(
                    wt_hwnd,
                    "No TabItem controls found, trying broader search...",
                );
                let matcher2 = automation
                    .create_matcher()
                    .from_ref(&root)
                    .depth(15)
                    .timeout(3000);
                match matcher2.find_all() {
                    Ok(all) => {
                        let tabs: Vec<_> = all
                            .iter()
                            .filter(|e| {
                                let ct = e.get_control_type().ok();
                                let aid = e.get_automation_id().unwrap_or_default();
                                let name = e.get_name().unwrap_or_default();
                                matches!(ct, Some(ControlType::TabItem))
                                    || aid.starts_with("Tab")
                                    || (name.starts_with("Tab ") && name.len() < 30)
                            })
                            .cloned()
                            .collect();
                        diag(
                            wt_hwnd,
                            &format!("Broad search found {} tab-like elements", tabs.len()),
                        );
                        tabs
                    }
                    Err(_) => Vec::new(),
                }
            }
        }
    };

    if tab_items.is_empty() {
        diag(wt_hwnd, "No tabs found via UIA");
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut seen_titles: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Step 2: Click each visible tab and capture its content.
    for (i, tab) in tab_items.iter().enumerate() {
        let tab_title = tab.get_name().unwrap_or_else(|_| format!("Tab {}", i + 1));
        diag(
            wt_hwnd,
            &format!("Switching to tab {}: \"{}\"", i, &tab_title),
        );

        let mut switched = false;
        if let Ok(invoke) = tab.get_pattern::<uiautomation::patterns::UIInvokePattern>() {
            if invoke.invoke().is_ok() {
                switched = true;
                diag(wt_hwnd, &format!("  Tab {} clicked via Invoke", i));
            }
        }
        if !switched {
            if let Ok(scroll) = tab.get_pattern::<uiautomation::patterns::UIScrollItemPattern>() {
                let _ = scroll.scroll_into_view();
            }
            if let Ok(rect) = tab.get_bounding_rectangle() {
                let center_x = (rect.get_left() + rect.get_right()) / 2;
                let center_y = (rect.get_top() + rect.get_bottom()) / 2;
                diag(
                    wt_hwnd,
                    &format!(
                        "  Tab {} rect: ({},{}) - ({},{}) center=({},{})",
                        i,
                        rect.get_left(),
                        rect.get_top(),
                        rect.get_right(),
                        rect.get_bottom(),
                        center_x,
                        center_y
                    ),
                );
                click_at_point(center_x, center_y);
                switched = true;
                diag(
                    wt_hwnd,
                    &format!(
                        "  Tab {} clicked via mouse at ({},{})",
                        i, center_x, center_y
                    ),
                );
            }
        }

        if !switched {
            diag(wt_hwnd, &format!("  Failed to switch to tab {}", i));
            continue;
        }

        std::thread::sleep(std::time::Duration::from_millis(500));

        if let Some(content) = capture_active_tab_via_uia(wt_hwnd) {
            if !content.is_empty() {
                let trimmed = super::super::trim_terminal_content(&content);
                let lines: Vec<String> = trimmed.lines().map(String::from).collect();
                diag(
                    wt_hwnd,
                    &format!("  Captured tab \"{}\": {} lines", &tab_title, lines.len()),
                );
                seen_titles.insert(tab_title.clone());
                results.push((
                    TabInfo {
                        index: i,
                        title: tab_title,
                    },
                    trimmed,
                ));
            } else {
                diag(
                    wt_hwnd,
                    &format!("  Tab \"{}\" returned empty content", &tab_title),
                );
            }
        } else {
            diag(
                wt_hwnd,
                &format!("  capture_via_uia returned None for tab \"{}\"", &tab_title),
            );
        }
    }

    // Step 3 REMOVED: Ctrl+Number keyboard shortcuts injected keystrokes into
    // whatever window had focus, causing random input on the user's machine.
    // UIA tab clicking (Step 2) handles visible tabs. For overflow tabs
    // that are scrolled off the tab strip, we scroll the strip and click.

    diag(
        wt_hwnd,
        &format!(
            "=== capture_all_tabs complete: {} tabs captured ===",
            results.len()
        ),
    );
    results
}
