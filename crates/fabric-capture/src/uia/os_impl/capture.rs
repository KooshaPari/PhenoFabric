use uiautomation::core::UIAutomation;
use uiautomation::types::{Handle, TreeScope};

use super::extract::{diag, try_extract_text, try_extract_via_text_pattern};

/// Try UIA capture on a specific HWND.
/// This is the main capture function that tries multiple strategies.
pub(crate) fn capture_via_uia(hwnd: isize) -> Option<String> {
    diag(hwnd, "--- capture_via_uia called ---");

    let automation = match UIAutomation::new() {
        Ok(a) => a,
        Err(e) => {
            diag(hwnd, &format!("UIAutomation::new() FAILED: {e}"));
            return None;
        }
    };

    // Strategy A: Try the HWND directly.
    let handle = Handle::from(hwnd);
    let element = match automation.element_from_handle(handle) {
        Ok(e) => {
            let ct = e
                .get_control_type()
                .map(|c| format!("{:?}", c))
                .unwrap_or_else(|_| "err".into());
            let cn = e.get_classname().unwrap_or_default();
            diag(
                hwnd,
                &format!("element_from_handle OK, classname={cn}, control_type={ct}"),
            );
            e
        }
        Err(e) => {
            diag(hwnd, &format!("element_from_handle FAILED: {e}"));
            return None;
        }
    };

    // Try to capture text from this element.
    if let Some(text) = try_extract_via_text_pattern(hwnd, &element) {
        return Some(text);
    }
    if let Some(text) = try_extract_text(&automation, hwnd, &element) {
        diag(hwnd, &format!("Captured text ({} chars)", text.len()));
        return Some(text);
    }

    // Strategy B: Enumerate child HWNDs and try UIA on each.
    diag(hwnd, "Trying child HWNDs...");
    let children = enum_child_hwnds(hwnd);
    diag(hwnd, &format!("Found {} child HWNDs", children.len()));

    for child_hwnd in &children {
        let mut child_class = [0u16; 256];
        let child_class_len = unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::GetClassNameW(
                *child_hwnd as windows_sys::Win32::Foundation::HWND,
                child_class.as_mut_ptr(),
                256,
            )
        };
        let child_class_name = String::from_utf16_lossy(&child_class[..child_class_len as usize]);
        diag(
            hwnd,
            &format!(
                "  Child HWND {:#x} class=\"{}\"",
                child_hwnd, child_class_name
            ),
        );

        if let Ok(child_element) = automation.element_from_handle(Handle::from(*child_hwnd)) {
            let ct = child_element
                .get_control_type()
                .map(|c| format!("{:?}", c))
                .unwrap_or_else(|_| "err".into());
            diag(hwnd, &format!("  Child element: control_type={ct}"));

            if let Some(text) = try_extract_via_text_pattern(hwnd, &child_element) {
                diag(
                    hwnd,
                    &format!(
                        "  Captured from child via ITextProvider ({} chars)",
                        text.len()
                    ),
                );
                return Some(text);
            }
            if let Some(text) = try_extract_text(&automation, hwnd, &child_element) {
                diag(
                    hwnd,
                    &format!("  Captured from child ({} chars)", text.len()),
                );
                return Some(text);
            }
        }
    }

    // Strategy C: Try finding ALL descendants with a broader search.
    diag(hwnd, "Trying full descendant search for value elements...");
    if let Ok(true_cond) = automation.create_true_condition() {
        if let Ok(all) = element.find_all(TreeScope::Subtree, &true_cond) {
            diag(hwnd, &format!("Full tree has {} elements", all.len()));
            for (i, elem) in all.iter().enumerate().take(50) {
                let ct = elem
                    .get_control_type()
                    .map(|c| format!("{:?}", c))
                    .unwrap_or_else(|_| "err".into());
                let cn = elem.get_classname().unwrap_or_default();
                let aid = elem.get_automation_id().unwrap_or_default();
                let name = elem.get_name().unwrap_or_default();
                let name_preview = if name.len() > 80 {
                    format!("{}...", &name[..77])
                } else {
                    name.clone()
                };
                diag(
                    hwnd,
                    &format!(
                        "  [{i}] type={ct} class=\"{cn}\" aid=\"{aid}\" name=\"{name_preview}\""
                    ),
                );

                if name.len() > 50 && ct != "Window" && ct != "TitleBar" {
                    diag(
                        hwnd,
                        &format!("  >>> LONG NAME FOUND: {} chars", name.len()),
                    );
                    return Some(name);
                }
                if cn == "TermControl" || ct == "Text" {
                    if let Some(text) = try_extract_via_text_pattern(hwnd, elem) {
                        return Some(text);
                    }
                }
            }
        }
    }

    // Strategy D: Search from the desktop root.
    diag(hwnd, "Trying desktop root search...");
    if let Ok(root) = automation.get_root_element() {
        if let Ok(cond) = automation.create_true_condition() {
            if let Ok(all) = root.find_all(TreeScope::Subtree, &cond) {
                diag(
                    hwnd,
                    &format!("Desktop tree has {} total elements", all.len()),
                );
                for (i, elem) in all.iter().enumerate().take(500) {
                    let ct = elem
                        .get_control_type()
                        .map(|c| format!("{:?}", c))
                        .unwrap_or_else(|_| "err".into());
                    let cn = elem.get_classname().unwrap_or_default();
                    let name = elem.get_name().unwrap_or_default();
                    if cn.contains("Terminal")
                        || cn.contains("CASCADIA")
                        || name.contains('\n')
                        || (name.len() > 100 && ct != "Window")
                    {
                        let preview = if name.len() > 120 {
                            format!("{}...", &name[..117].replace('\n', "\\n"))
                        } else {
                            name.replace('\n', "\\n")
                        };
                        diag(
                            hwnd,
                            &format!("  [D:{i}] type={ct} class=\"{cn}\" name=\"{preview}\""),
                        );
                    }
                    if name.contains('\n') && name.len() > 50 {
                        diag(
                            hwnd,
                            &format!(
                                "  >>> TERMINAL CONTENT FOUND via desktop ({} chars)",
                                name.len()
                            ),
                        );
                        return Some(name);
                    }
                    if cn == "TermControl" || (ct == "Text" && cn.contains("Term")) {
                        if let Some(text) = try_extract_via_text_pattern(hwnd, elem) {
                            return Some(text);
                        }
                    }
                }
            }
        }
    }

    // Strategy E: Try element_from_point at the center of the WT window.
    diag(hwnd, "Trying element_from_point...");
    unsafe {
        use windows_sys::Win32::Foundation::{HWND, RECT};
        use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowRect;
        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd as HWND, &mut rect) != 0 {
            let cx = (rect.left + rect.right) / 2;
            let cy = (rect.top + rect.bottom) / 2;
            diag(
                hwnd,
                &format!(
                    "  Window rect: ({},{})-({},{}) center=({},{})",
                    rect.left, rect.top, rect.right, rect.bottom, cx, cy
                ),
            );
            let point = uiautomation::types::Point::new(cx, cy);
            match automation.element_from_point(point) {
                Ok(point_elem) => {
                    let ct = point_elem
                        .get_control_type()
                        .map(|c| format!("{:?}", c))
                        .unwrap_or_else(|_| "err".into());
                    let cn = point_elem.get_classname().unwrap_or_default();
                    let name = point_elem.get_name().unwrap_or_default();
                    diag(
                        hwnd,
                        &format!(
                            "  Point element: type={ct} class=\"{cn}\" name_len={}",
                            name.len()
                        ),
                    );
                    if let Some(text) = try_extract_via_text_pattern(hwnd, &point_elem) {
                        return Some(text);
                    }
                    if let Ok(true_cond) = automation.create_true_condition() {
                        if let Ok(all) = point_elem.find_all(TreeScope::Subtree, &true_cond) {
                            diag(hwnd, &format!("  Point subtree has {} elements", all.len()));
                            for (i, e) in all.iter().enumerate().take(100) {
                                let ct2 = e
                                    .get_control_type()
                                    .map(|c| format!("{:?}", c))
                                    .unwrap_or_else(|_| "err".into());
                                let cn2 = e.get_classname().unwrap_or_default();
                                let n2 = e.get_name().unwrap_or_default();
                                let preview = if n2.len() > 100 {
                                    format!("{}...", &n2[..97].replace('\n', "\\n"))
                                } else {
                                    n2.replace('\n', "\\n")
                                };
                                diag(
                                    hwnd,
                                    &format!(
                                        "    [E:{i}] type={ct2} class=\"{cn2}\" name=\"{preview}\""
                                    ),
                                );
                                if let Some(text) = try_extract_via_text_pattern(hwnd, e) {
                                    return Some(text);
                                }
                                if n2.contains('\n') && n2.len() > 50 {
                                    diag(
                                        hwnd,
                                        &format!(
                                            "    >>> TERMINAL CONTENT via point ({} chars)",
                                            n2.len()
                                        ),
                                    );
                                    return Some(n2);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    diag(hwnd, &format!("  element_from_point FAILED: {e}"));
                }
            }
        }
    }

    diag(hwnd, "No content captured via UIA");
    None
}

/// Capture the currently active tab's content using element_from_point.
/// Tries multiple points in the content area to find the TermControl.
pub(crate) fn capture_active_tab_via_uia(wt_hwnd: isize) -> Option<String> {
    let automation = UIAutomation::new().ok()?;

    let handle = Handle::from(wt_hwnd);
    let root = automation.element_from_handle(handle).ok()?;
    let rect = root.get_bounding_rectangle().ok()?;
    let center_x = (rect.get_left() + rect.get_right()) / 2;
    let center_y = (rect.get_top() + rect.get_bottom()) / 2;

    diag(
        wt_hwnd,
        &format!(
            "capture_active_tab: center=({},{}), rect=({},{})-({},{})",
            center_x,
            center_y,
            rect.get_left(),
            rect.get_top(),
            rect.get_right(),
            rect.get_bottom()
        ),
    );

    // Try multiple points in the content area to find the TermControl.
    // After keyboard cycling, the tab strip may shift and the center might
    // hit a non-TermControl element. Try offset points as fallback.
    let points = [
        (center_x, center_y),
        (center_x, center_y + 50),
        (center_x - 50, center_y),
        (center_x + 50, center_y),
        (center_x, center_y - 100),
        (center_x - 200, center_y + 100),
        (center_x + 200, center_y + 100),
    ];

    for (px, py) in &points {
        let point = uiautomation::types::Point::new(*px, *py);
        let element = match automation.element_from_point(point) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let ct = element
            .get_control_type()
            .map(|c| format!("{:?}", c))
            .unwrap_or_else(|_| "err".into());
        let cn = element.get_classname().unwrap_or_default();
        diag(
            wt_hwnd,
            &format!(
                "capture_active_tab: point=({},{}) class=\"{}\", ct={}",
                px, py, cn, ct
            ),
        );

        if let Some(text) = try_extract_via_text_pattern(wt_hwnd, &element) {
            return Some(text);
        }
    }

    // Last resort: search the entire window tree for a TermControl element.
    diag(
        wt_hwnd,
        "  All points failed, searching tree for TermControl...",
    );
    let term_matcher = automation
        .create_matcher()
        .from_ref(&root)
        .classname("TermControl")
        .depth(20)
        .timeout(2000);
    if let Ok(terms) = term_matcher.find_all() {
        if let Some(term) = terms.first() {
            diag(wt_hwnd, "  Found TermControl via classname search");
            if let Some(text) = try_extract_via_text_pattern(wt_hwnd, term) {
                return Some(text);
            }
        }
    }

    None
}

/// Find all visible windows matching WT class names.
pub(crate) fn enum_terminals() -> Vec<WindowInfo> {
    use std::sync::Mutex;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetWindowTextLengthW, GetWindowTextW, IsWindowVisible,
    };

    const WT_CLASSES: &[&str] = &[
        "CASCADIA_HOSTING_WINDOW_CLASS",
        "CASCADIA_HOSTING",
        "CascadiaMainWindow",
    ];

    static FOUND: Mutex<Vec<(String, String, isize)>> = Mutex::new(Vec::new());
    FOUND.lock().unwrap().clear();

    unsafe extern "system" fn callback(hwnd: HWND, _lparam: isize) -> i32 {
        if IsWindowVisible(hwnd) == 0 {
            return 1;
        }
        let mut class_buf = [0u16; 256];
        let class_len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), 256);
        if class_len == 0 {
            return 1;
        }
        let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);

        let title_len = GetWindowTextLengthW(hwnd);
        let mut title_buf = vec![0u16; (title_len + 1).max(2) as usize];
        let actual_len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_len + 1);
        let title = String::from_utf16_lossy(&title_buf[..actual_len as usize]);

        if WT_CLASSES
            .iter()
            .any(|c| c.eq_ignore_ascii_case(&class_name))
        {
            FOUND
                .lock()
                .unwrap()
                .push((class_name, title, hwnd as isize));
        }
        1
    }

    unsafe {
        EnumWindows(Some(callback), 0);
    }

    FOUND
        .lock()
        .unwrap()
        .drain(..)
        .map(|(_class, title, hwnd)| WindowInfo { hwnd, title })
        .collect()
}

pub(crate) struct WindowInfo {
    pub(crate) hwnd: isize,
    pub(crate) title: String,
}

/// Enumerate all visible Windows Terminal windows and attempt UIA capture.
pub(crate) fn enumerate_and_capture() -> Vec<(isize, String, Option<String>)> {
    let terminals = enum_terminals();
    terminals
        .into_iter()
        .map(|t| {
            let content = capture_via_uia(t.hwnd).map(|t| super::super::trim_terminal_content(&t));
            (t.hwnd, t.title, content)
        })
        .collect()
}

/// Enumerate immediate child HWNDs of a parent window.
fn enum_child_hwnds(parent: isize) -> Vec<isize> {
    use std::sync::Mutex;
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::EnumChildWindows;

    static CHILDREN: Mutex<Vec<isize>> = Mutex::new(Vec::new());
    CHILDREN.lock().unwrap().clear();

    unsafe extern "system" fn child_callback(hwnd: HWND, _lparam: isize) -> i32 {
        CHILDREN.lock().unwrap().push(hwnd as isize);
        1
    }

    unsafe {
        EnumChildWindows(parent as HWND, Some(child_callback), 0);
    }
    CHILDREN.lock().unwrap().clone()
}
