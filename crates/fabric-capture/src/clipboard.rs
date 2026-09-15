//! Clipboard-based capture for Windows Terminal tabs.
//!
//! Strategy: Send WM_KEYDOWN/WM_KEYUP messages directly to the WT window
//! handle via SendMessageW. This avoids global input injection (SendInput)
//! which would steal keyboard input from the user.

#[cfg(target_os = "windows")]
pub mod win {
    use std::ptr;
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::System::DataExchange::*;
    use windows_sys::Win32::System::Memory::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    const CF_UNICODETEXT: u32 = 13;
    const WM_KEYDOWN: u32 = 0x0100;
    const WM_KEYUP: u32 = 0x0101;

    const VK_CONTROL: u16 = 0x11;
    const VK_A: u16 = 0x41;
    const VK_C: u16 = 0x43;
    const VK_1: u16 = 0x31;
    const VK_2: u16 = 0x32;
    const VK_3: u16 = 0x33;
    const VK_4: u16 = 0x34;
    const VK_5: u16 = 0x35;
    const VK_6: u16 = 0x36;
    const VK_7: u16 = 0x37;
    const VK_8: u16 = 0x38;
    const VK_9: u16 = 0x39;

    /// Send a single key press and release to a window via SendMessage.
    /// `vk` is the virtual key code (e.g. VK_A, VK_CONTROL).
    /// `scan` is the hardware scan code for the key.
    unsafe fn send_key_to_window(hwnd: HWND, vk: u16, scan: u16) {
        // lparam layout: repeat(1) | scan(16) | ext(1) | reserved(1) | prev(1) | up(1)
        let lparam_down: isize = 1 | ((scan as isize) << 16);
        let lparam_up: isize = 1 | ((scan as isize) << 16) | (1 << 30) | (1 << 31);

        SendMessageW(hwnd, WM_KEYDOWN, vk as usize, lparam_down);
        std::thread::sleep(std::time::Duration::from_millis(10));
        SendMessageW(hwnd, WM_KEYUP, vk as usize, lparam_up);
    }

    /// Send a Ctrl+Key combination to a specific window via SendMessage.
    /// All messages target `hwnd` directly -- no global input injection.
    unsafe fn send_ctrl_key_to_window(hwnd: HWND, vk: u16) {
        let scan_ctrl: u16 = 0x1D;
        let scan_key: u16 = match vk {
            VK_A => 0x1E,
            VK_C => 0x2E,
            VK_1 => 0x02, VK_2 => 0x03, VK_3 => 0x04,
            VK_4 => 0x05, VK_5 => 0x06, VK_6 => 0x07,
            VK_7 => 0x08, VK_8 => 0x09, VK_9 => 0x0A,
            _ => 0x00,
        };

        // Ctrl down
        let lp_ctrl: isize = 1 | ((scan_ctrl as isize) << 16);
        SendMessageW(hwnd, WM_KEYDOWN, VK_CONTROL as usize, lp_ctrl);
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Key down
        send_key_to_window(hwnd, vk, scan_key);
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Ctrl up
        let lp_ctrl_up: isize = lp_ctrl | (1 << 30) | (1 << 31);
        SendMessageW(hwnd, WM_KEYUP, VK_CONTROL as usize, lp_ctrl_up);
    }

    /// Check if Windows Terminal is the foreground window.
    /// Safety belt: only capture when WT is visible and focused.
    unsafe fn is_wt_foreground(wt_hwnd: HWND) -> bool {
        let fg = GetForegroundWindow();
        if fg == wt_hwnd {
            return true;
        }
        // Also accept if the foreground is a child of WT (e.g. a dialog).
        if !fg.is_null() && IsChild(wt_hwnd, fg) != 0 {
            return true;
        }
        tracing::debug!(
            wt_hwnd = wt_hwnd as isize,
            fg = fg as isize,
            "WT is not foreground, skipping clipboard capture"
        );
        false
    }

    /// Read text from the Windows clipboard (CF_UNICODETEXT).
    unsafe fn read_clipboard_text() -> Option<String> {
        for attempt in 0..5 {
            if OpenClipboard(ptr::null_mut()) != 0 {
                break;
            }
            if attempt == 4 {
                tracing::debug!("read_clipboard: OpenClipboard FAILED after 5 attempts");
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        let data = GetClipboardData(CF_UNICODETEXT);
        if data.is_null() {
            tracing::debug!("read_clipboard: clipboard empty (no CF_UNICODETEXT)");
            CloseClipboard();
            return None;
        }

        let p = GlobalLock(data);
        if p.is_null() {
            tracing::debug!("read_clipboard: GlobalLock FAILED");
            CloseClipboard();
            return None;
        }

        let text = {
            let mut len = 0;
            let mut ptr = p as *const u16;
            while *ptr != 0 {
                len += 1;
                ptr = ptr.add(1);
            }
            let slice = std::slice::from_raw_parts(p as *const u16, len);
            String::from_utf16_lossy(slice)
        };

        GlobalUnlock(data);
        CloseClipboard();
        Some(text)
    }

    /// Capture a single tab by index (1-9) using clipboard.
    ///
    /// Sends Ctrl+Number (switch tab), Ctrl+A (select all), Ctrl+C (copy)
    /// directly to the WT window handle via SendMessage. No global input
    /// injection is performed.
    ///
    /// Returns None if WT is not the foreground window or capture fails.
    pub fn capture_tab_by_index(wt_hwnd: isize, tab_index: u8) -> Option<String> {
        let hwnd = wt_hwnd as HWND;
        let vk = match tab_index {
            1 => VK_1, 2 => VK_2, 3 => VK_3, 4 => VK_4,
            5 => VK_5, 6 => VK_6, 7 => VK_7, 8 => VK_8, 9 => VK_9,
            _ => return None,
        };

        unsafe {
            // Safety: only capture when WT is the foreground window.
            if !is_wt_foreground(hwnd) {
                return None;
            }

            // Switch to the interactive desktop (scheduled tasks may run
            // on Winlogon desktop where SendMessage has no effect).
            switch_to_input_desktop();

            // Step 1: Ctrl+Number to switch to the target tab.
            send_ctrl_key_to_window(hwnd, vk);
            std::thread::sleep(std::time::Duration::from_millis(200));

            // Step 2: Ctrl+A to select all content in the terminal.
            send_ctrl_key_to_window(hwnd, VK_A);
            std::thread::sleep(std::time::Duration::from_millis(150));

            // Step 3: Ctrl+C to copy the selected content to clipboard.
            send_ctrl_key_to_window(hwnd, VK_C);
            std::thread::sleep(std::time::Duration::from_millis(200));

            // Step 4: Read the clipboard.
            let text = read_clipboard_text();
            tracing::debug!(
                tab = tab_index,
                text_len = text.as_ref().map(|s| s.len()).unwrap_or(0),
                "capture_tab_by_index (SendMessage)"
            );
            text
        }
    }

    /// Capture all visible tabs from the Windows Terminal (indices 1-9).
    ///
    /// Each tab is captured by sending Ctrl+Number, Ctrl+A, Ctrl+C via
    /// SendMessage directly to the WT window handle.
    pub fn capture_all_tabs(wt_hwnd: isize) -> Vec<(u8, String, Vec<String>)> {
        let mut results = Vec::new();

        for tab_idx in 1..=9u8 {
            if let Some(text) = capture_tab_by_index(wt_hwnd, tab_idx) {
                let trimmed = text.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }

                let lines: Vec<String> = trimmed.lines().map(String::from).collect();
                let title_hint = lines
                    .first()
                    .map(|l| {
                        let clean = l.trim();
                        if clean.len() > 60 {
                            format!("{}...", &clean[..57])
                        } else {
                            clean.to_string()
                        }
                    })
                    .unwrap_or_else(|| format!("Tab {}", tab_idx));

                results.push((tab_idx, title_hint, lines));
            }
        }

        results
    }

    /// Switch to the interactive input desktop.
    /// Scheduled tasks may run on Winlogon desktop where SendMessage
    /// has no effect on user windows.
    unsafe fn switch_to_input_desktop() {
        const GENERIC_ALL: u32 = 0x10000000;
        const FALSE: i32 = 0;

        let input_desktop = OpenInputDesktop(0, FALSE, GENERIC_ALL);
        if !input_desktop.is_null() {
            let tid = GetCurrentThreadId();
            let switched = SetThreadDesktop(input_desktop);
            tracing::debug!(
                switched = switched,
                err = windows_sys::Win32::Foundation::GetLastError(),
                tid = tid,
                "switch_to_input_desktop"
            );
        } else {
            tracing::debug!(
                err = windows_sys::Win32::Foundation::GetLastError(),
                "OpenInputDesktop FAILED"
            );
        }
    }
}

// Non-Windows stub: clipboard capture is not available.
#[cfg(not(target_os = "windows"))]
pub mod win {
    /// Capture a single tab -- not available on this platform.
    pub fn capture_tab_by_index(_wt_hwnd: isize, _tab_index: u8) -> Option<String> {
        None
    }

    /// Capture all tabs -- not available on this platform.
    pub fn capture_all_tabs(_wt_hwnd: isize) -> Vec<(u8, String, Vec<String>)> {
        Vec::new()
    }
}
