use uiautomation::core::{UIAutomation, UIElement};
use uiautomation::types::ControlType;
use uiautomation::patterns::UITextPattern;

use super::super::MAX_LINES;

/// Diagnostic log to file.
pub(crate) fn diag(hwnd: isize, msg: &str) {
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("C:\\Users\\koosh\\Desktop\\tf-uia-debug.log")
        .and_then(|mut f| {
            use std::io::Write;
            writeln!(f, "[{:?}] hwnd={:#x} {}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis(),
                hwnd, msg)
        });
}

/// Try to extract text via ITextProvider (TextPattern) on an element.
/// This is the canonical way to read Windows Terminal content.
pub(crate) fn try_extract_via_text_pattern(hwnd: isize, element: &UIElement) -> Option<String> {
    match element.get_pattern::<UITextPattern>() {
        Ok(pattern) => {
            diag(hwnd, "  ITextPattern obtained!");
            // Try document range first (full scrollback buffer).
            match pattern.get_document_range() {
                Ok(range) => {
                    match range.get_text(100_000) {
                        Ok(text) if !text.is_empty() => {
                            diag(hwnd, &format!("  ITextProvider: got {} chars from document range", text.len()));
                            return Some(text);
                        }
                        Ok(_) => {
                            diag(hwnd, "  ITextProvider: document range returned empty text");
                        }
                        Err(e) => {
                            diag(hwnd, &format!("  ITextProvider: get_text failed: {e}"));
                        }
                    }
                }
                Err(e) => {
                    diag(hwnd, &format!("  ITextProvider: get_document_range failed: {e}"));
                }
            }
            // Try visible ranges as fallback.
            match pattern.get_visible_ranges() {
                Ok(ranges) => {
                    diag(hwnd, &format!("  ITextProvider: {} visible ranges", ranges.len()));
                    let mut all_text = String::new();
                    for (i, range) in ranges.iter().enumerate() {
                        match range.get_text(100_000) {
                            Ok(text) if !text.is_empty() => {
                                if !all_text.is_empty() { all_text.push('\n'); }
                                all_text.push_str(&text);
                            }
                            _ => {}
                        }
                        if i >= 50 { break; }
                    }
                    if !all_text.is_empty() {
                        diag(hwnd, &format!("  ITextProvider: got {} chars from visible ranges", all_text.len()));
                        return Some(all_text);
                    }
                }
                Err(e) => {
                    diag(hwnd, &format!("  ITextProvider: get_visible_ranges failed: {e}"));
                }
            }
            None
        }
        Err(e) => {
            diag(hwnd, &format!("  ITextPattern not available: {e}"));
            None
        }
    }
}

/// Try to extract text from an element and its descendants.
/// Returns terminal content (multi-line text) rather than window titles.
pub(crate) fn try_extract_text(
    automation: &UIAutomation,
    hwnd: isize,
    element: &UIElement,
) -> Option<String> {
    // Strategy 1: Check if the element's Name contains terminal content.
    // Terminal content has newlines; window titles are single-line.
    if let Ok(name) = element.get_name() {
        if name.contains('\n') && name.len() > 30 {
            diag(hwnd, &format!(
                "  Found multi-line name ({} chars): {}",
                name.len(),
                &name[..name.len().min(120)].replace('\n', "\\n")
            ));
            return Some(name);
        }
    }

    // Strategy 2: Use UIMatcher to find descendants with text content.
    // Try ControlType::Edit first (WT may expose terminal as Edit).
    let matcher = automation
        .create_matcher()
        .from_ref(element)
        .depth(20)
        .timeout(3000);

    // Try finding Edit controls.
    match matcher.control_type(ControlType::Edit).find_all() {
        Ok(elems) if !elems.is_empty() => {
            diag(hwnd, &format!("  Found {} Edit controls", elems.len()));
            let mut lines = Vec::new();
            for child in elems.iter() {
                if let Ok(name) = child.get_name() {
                    if !name.is_empty() {
                        for line in name.lines() {
                            let t = line.to_string();
                            if !t.is_empty() {
                                lines.push(t);
                            }
                        }
                    }
                }
                if lines.len() >= MAX_LINES {
                    break;
                }
            }
            if !lines.is_empty() {
                lines.truncate(MAX_LINES);
                return Some(lines.join("\n"));
            }
        }
        _ => {}
    }

    // Try finding Text controls.
    match automation
        .create_matcher()
        .from_ref(element)
        .control_type(ControlType::Text)
        .depth(20)
        .timeout(2000)
        .find_all()
    {
        Ok(elems) if !elems.is_empty() => {
            diag(hwnd, &format!("  Found {} Text controls", elems.len()));
            let mut lines = Vec::new();
            for child in elems.iter() {
                if let Ok(name) = child.get_name() {
                    if !name.is_empty() {
                        for line in name.lines() {
                            let t = line.to_string();
                            if !t.is_empty() {
                                lines.push(t);
                            }
                        }
                    }
                }
                if lines.len() >= MAX_LINES {
                    break;
                }
            }
            if !lines.is_empty() {
                lines.truncate(MAX_LINES);
                return Some(lines.join("\n"));
            }
        }
        _ => {}
    }

    // Try finding Pane controls (the terminal pane is often a Pane type).
    match automation
        .create_matcher()
        .from_ref(element)
        .control_type(ControlType::Pane)
        .depth(20)
        .timeout(2000)
        .find_all()
    {
        Ok(elems) => {
            diag(hwnd, &format!("  Found {} Pane controls", elems.len()));
            // Panes might contain the terminal content as multi-line names.
            for child in elems.iter() {
                if let Ok(name) = child.get_name() {
                    if name.contains('\n') && name.len() > 50 {
                        diag(hwnd, &format!(
                            "  Pane with multi-line name ({} chars)",
                            name.len()
                        ));
                        return Some(name);
                    }
                }
            }
        }
        _ => {}
    }

    None
}
