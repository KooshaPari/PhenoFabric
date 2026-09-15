//! Windows console enumeration and screen buffer capture.
//!
//! Uses Win32 APIs to find PowerShell/Console windows and read their
//! screen buffer content.

/// Information about a discovered console window.
#[derive(Debug, Clone)]
pub struct ConsolePane {
    /// Window handle (HWND). Stored as usize for portability; cast to HWND when calling Win32.
    pub hwnd: usize,
    /// Process ID that owns this window.
    pub process_id: u32,
    /// Window title.
    pub title: String,
    /// Friendly name derived from process and title.
    pub pid_name: String,
}

/// Content captured from a console screen buffer.
#[derive(Debug, Clone)]
pub struct CapturedContent {
    pub lines: Vec<String>,
    pub width: u32,
    pub height: u32,
}

// ============================================================================
// Windows implementation
// ============================================================================

#[cfg(target_os = "windows")]
mod os_specific {
    use super::*;
    use windows_sys::Win32::Foundation::{HANDLE, HWND, LPARAM, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Console::{
        AttachConsole, FreeConsole, GetConsoleScreenBufferInfo, GetStdHandle,
        ReadConsoleOutputCharacterW, CONSOLE_SCREEN_BUFFER_INFO, COORD, STD_OUTPUT_HANDLE,
    };
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, IsWindowVisible,
    };

    const CONSOLE_CLASSES: &[&str] = &[
        "ConsoleWindowClass",
        "PseudoConsoleWindow",
        "CASCADIA_HOSTING_WINDOW_CLASS",
        "CASCADIA_HOSTING",
    ];

    /// Cast usize to HWND pointer.
    unsafe fn usize_to_hwnd(v: usize) -> HWND {
        std::ptr::with_exposed_provenance_mut::<core::ffi::c_void>(v)
    }

    unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> i32 {
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }

        let mut process_id: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut process_id);
        if process_id == 0 {
            return 1;
        }

        let title_len = GetWindowTextLengthW(hwnd);
        if title_len == 0 {
            return 1;
        }

        let mut title_buf: Vec<u16> = vec![0; (title_len + 1) as usize];
        let copied = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_len + 1);
        if copied == 0 {
            return 1;
        }

        let title = String::from_utf16_lossy(&title_buf[..copied as usize]);

        if !is_console_window(hwnd) {
            return 1;
        }

        let pane = ConsolePane {
            hwnd: hwnd as usize,
            process_id,
            title: title.clone(),
            pid_name: format!("win-{}-{}", process_id, sanitize_title(&title, 32)),
        };

        let panes = &mut *(lparam as *mut Vec<ConsolePane>);
        panes.push(pane);
        1
    }

    fn is_console_window(hwnd: HWND) -> bool {
        let mut class_buf: [u16; 256] = [0; 256];
        let class_len = unsafe { GetClassNameW(hwnd, class_buf.as_mut_ptr(), 256) };
        if class_len > 0 {
            let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);
            return CONSOLE_CLASSES.iter().any(|&c| c == class_name);
        }
        false
    }

    pub(super) fn enumerate_windows() -> anyhow::Result<Vec<ConsolePane>> {
        unsafe {
            let mut panes: Vec<ConsolePane> = Vec::new();
            let lparam = &mut panes as *mut Vec<ConsolePane> as LPARAM;
            EnumWindows(Some(enum_windows_callback), lparam);
            tracing::debug!(count = panes.len(), "Enumerated console windows");
            Ok(panes)
        }
    }

    pub(super) fn capture_buffer(hwnd_usize: usize) -> Option<CapturedContent> {
        unsafe {
            let hwnd: HWND = usize_to_hwnd(hwnd_usize);

            let mut process_id: u32 = 0;
            GetWindowThreadProcessId(hwnd, &mut process_id);
            if process_id == 0 {
                tracing::debug!(hwnd = hwnd_usize, "capture_buffer: no process_id");
                return None;
            }

            let my_pid = GetCurrentProcessId();
            let need_reattach = process_id != my_pid;

            tracing::debug!(
                hwnd = hwnd_usize,
                target_pid = process_id,
                my_pid = my_pid,
                need_reattach,
                "capture_buffer: attempting AttachConsole"
            );

            if need_reattach {
                FreeConsole();
                if AttachConsole(process_id) == 0 {
                    let err = std::io::Error::last_os_error();
                    tracing::debug!(
                        hwnd = hwnd_usize,
                        target_pid = process_id,
                        error = %err,
                        "capture_buffer: AttachConsole FAILED"
                    );
                    AttachConsole(my_pid);
                    return None;
                }
            }

            let h_console: HANDLE = GetStdHandle(STD_OUTPUT_HANDLE);

            if h_console == INVALID_HANDLE_VALUE || h_console.is_null() {
                tracing::debug!(
                    hwnd = hwnd_usize,
                    handle = h_console as isize,
                    "capture_buffer: GetStdHandle returned invalid handle"
                );
                if need_reattach {
                    FreeConsole();
                    AttachConsole(my_pid);
                }
                return None;
            }

            let mut csbi: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();
            if GetConsoleScreenBufferInfo(h_console, &mut csbi) == 0 {
                let err = std::io::Error::last_os_error();
                tracing::debug!(
                    hwnd = hwnd_usize,
                    error = %err,
                    "capture_buffer: GetConsoleScreenBufferInfo FAILED"
                );
                if need_reattach {
                    FreeConsole();
                    AttachConsole(my_pid);
                }
                return None;
            }

            let width = (csbi.srWindow.Right - csbi.srWindow.Left + 1) as u32;
            let height = (csbi.srWindow.Bottom - csbi.srWindow.Top + 1) as u32;
            let width = width.min(512);
            let height = height.min(256);

            let mut lines: Vec<String> = Vec::with_capacity(height as usize);

            for row in 0..height {
                let mut char_buf: Vec<u16> = vec![0; width as usize];
                let mut chars_read: u32 = 0;
                let coord = COORD {
                    X: csbi.srWindow.Left,
                    Y: csbi.srWindow.Top + row as i16,
                };

                let result = ReadConsoleOutputCharacterW(
                    h_console,
                    char_buf.as_mut_ptr(),
                    width,
                    coord,
                    &mut chars_read,
                );

                if result != 0 && chars_read > 0 {
                    let line = String::from_utf16_lossy(&char_buf[..chars_read as usize]);
                    lines.push(line.trim_end().to_string());
                } else {
                    lines.push(String::new());
                }
            }

            if need_reattach {
                FreeConsole();
                AttachConsole(my_pid);
            }

            Some(CapturedContent { lines, width, height })
        }
    }

    /// Callback for enumerating PseudoConsoleWindow HWNDs (no title required).
    static mut PC_HWNDS: Option<Vec<usize>> = None;

    unsafe extern "system" fn enum_pc_callback(hwnd: HWND, _lparam: LPARAM) -> i32 {
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }

        let mut class_buf: [u16; 256] = [0; 256];
        let class_len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), 256);
        if class_len == 0 {
            return 1;
        }

        let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);
        if class_name == "PseudoConsoleWindow" {
            if let Some(ref mut hwnds) = PC_HWNDS {
                hwnds.push(hwnd as usize);
            }
        }

        1
    }

    /// Find all PseudoConsoleWindow HWNDs without requiring titles.
    pub(super) fn enumerate_pseudo_console_hwnds() -> Vec<usize> {
        unsafe {
            PC_HWNDS = Some(Vec::new());
            EnumWindows(Some(enum_pc_callback), 0);
            PC_HWNDS.take().unwrap_or_default()
        }
    }
}

// ============================================================================
// Cross-platform public API
// ============================================================================

#[cfg(not(target_os = "windows"))]
pub fn enumerate_console_panes() -> anyhow::Result<Vec<ConsolePane>> {
    Ok(vec![])
}

#[cfg(not(target_os = "windows"))]
pub fn capture_console_buffer(_hwnd: usize) -> Option<CapturedContent> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn enumerate_pseudo_console_hwnds() -> Vec<usize> {
    vec![]
}

#[cfg(target_os = "windows")]
pub fn enumerate_console_panes() -> anyhow::Result<Vec<ConsolePane>> {
    os_specific::enumerate_windows()
}

#[cfg(target_os = "windows")]
pub fn capture_console_buffer(hwnd: usize) -> Option<CapturedContent> {
    os_specific::capture_buffer(hwnd)
}

/// Find all PseudoConsoleWindow HWNDs (no title required).
/// These are the ConPTY host windows created by OpenConsole.exe for each WT tab.
#[cfg(target_os = "windows")]
pub fn enumerate_pseudo_console_hwnds() -> Vec<usize> {
    os_specific::enumerate_pseudo_console_hwnds()
}

/// Sanitize a window title for use as a pane name component.
fn sanitize_title(title: &str, max_chars: usize) -> String {
    let cleaned: String = title
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if cleaned.len() > max_chars {
        format!("{}...", &cleaned[..max_chars])
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_title_short() {
        assert_eq!(sanitize_title("PowerShell", 20), "PowerShell");
    }

    #[test]
    fn test_sanitize_title_long() {
        let result = sanitize_title("Administrator: Very Long PowerShell Window Title", 15);
        assert!(result.len() <= 18);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_sanitize_title_strips_special_chars() {
        let result = sanitize_title("pwsh (C:\\Users\\test)", 30);
        assert!(!result.contains('\\'));
        assert!(!result.contains('('));
        assert!(!result.contains(')'));
    }

    #[test]
    fn test_enumerate_returns_empty_on_non_windows() {
        #[cfg(not(target_os = "windows"))]
        {
            let panes = enumerate_console_panes().unwrap();
            assert!(panes.is_empty());
        }
    }

    #[test]
    fn test_capture_returns_none_on_non_windows() {
        #[cfg(not(target_os = "windows"))]
        {
            let result = capture_console_buffer(0);
            assert!(result.is_none());
        }
    }
}
