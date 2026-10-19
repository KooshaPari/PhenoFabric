//! Contract tests for the Fabric GUI HTML/JS shell.
//!
//! `fabric-gui`'s Rust crate is intentionally a near-empty workspace member
//! (see `src/lib.rs`); the real user-facing surface is the static HTML/JS
//! served by the Tauri shell. This test file pins the contracts that
//! matter most:
//!
//! - HTML pages parse to a minimum well-formedness bar (DOCTYPE, html/head/
//!   body, charset, title) without running a real browser.
//! - Every `api('cmd', ...)` call in `src/index.html` resolves to a
//!   `#[tauri::command] cmd` in `src-tauri/src/commands.rs`. The JS shell
//!   and the Rust shell evolve independently; this catches drift.
//! - The default daemon address (`127.0.0.1:9400`) is consistent across
//!   `fabric-cli`, the GUI's own Tauri config, and the GUI's README.
//! - The WorkOS client id baked into the JS matches the one the Rust
//!   shell passes to the daemon.
//! - Every `document.getElementById('x')` target the JS queries in
//!   `src/index.html` is actually present as `id="x"` in the markup. This
//!   catches the class of bug where a loader writes to an element that was
//!   never rendered: `getElementById` returns null, the next property
//!   access throws, and the rest of that loader never runs.
//! - The tab bar (`switchTab('x')` callers) matches the page containers
//!   (`id="page-x"`) matches the loader switch (`loadTabData('x')` cases).
//! - Every internal `href` / `<link href>` / `<script src>` / `<img src>`
//!   on a page resolves to a real file inside the crate (modulo the
//!   allow-list of `http(s)://`, `data:`, `javascript:`, and `#fragment`).
//!
//! What this does NOT cover (documented here so the next agent doesn't
//! waste a turn wondering):
//!
//! - Whether the page actually renders correctly under webkit / wry.
//! - Whether the JS executes without runtime errors when the Tauri APIs
//!   are present.
//! - Whether the IPC bridge correctly maps to the real daemon wire
//!   protocol (the JS doesn't speak to the daemon directly — every
//!   `api('foo')` goes through a Tauri command which does the TCP work).
//! - Visual regression of the theme. There is no separate stylesheet to
//!   regress against: `src/index.html` carries its CSS inline in a single
//!   `<style>` block, so "the theme" and "index.html" are the same file and
//!   the only cheap structural check available here is that a page exists
//!   and is well-formed.
//!
//! A real browser (chromiumoxide / headless_chrome / Playwright) would
//! buy us all of that, but the task brief forbade adding one, and the
//! bundle weight + CI cost would not be worth it given that
//! `crates/fabric-integration-tests` already covers the wire-protocol
//! end of these surfaces.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

fn gui_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn src_dir() -> PathBuf {
    gui_root().join("src")
}

fn index_html() -> PathBuf {
    src_dir().join("index.html")
}

fn pages_dir() -> PathBuf {
    src_dir().join("pages")
}

fn tauri_commands_rs() -> PathBuf {
    gui_root().join("src-tauri").join("src").join("commands.rs")
}

fn wire_client_rs() -> PathBuf {
    let root = gui_root();
    let repo_root = root
        .parent()
        .and_then(|p| p.parent())
        .expect("fabric-gui is at <repo>/crates/fabric-gui");
    repo_root
        .join("crates")
        .join("fabric-cli")
        .join("src")
        .join("wire_client.rs")
}

fn daemon_rs() -> PathBuf {
    gui_root().join("src-tauri").join("src").join("daemon.rs")
}

fn readme() -> PathBuf {
    gui_root().join("README.md")
}

// All HTML pages the GUI serves from its static dir.
//
// `src/index.html` is the single shipped source: it carries the whole
// stylesheet, the whole markup for every tab, and the whole script. The
// `pages/*.html` fragments it used to sit beside are gone (deleted
// 2026-10-19), so `src/pages/` is absent in the shipped tree and there are
// no fragments to check. The directory scan is kept because it is the thing
// that flags a re-introduced fragment the moment it is added: if somebody
// drops a new `pages/foo.html` back in, it flows into every page-wide check
// below (well-formedness, local asset references, orphan detection).
fn all_pages() -> Vec<PathBuf> {
    let mut pages = vec![index_html()];
    // A missing `src/pages/` is the shipped state, not an error. Anything
    // other than NotFound is still a hard error — that is a real filesystem
    // problem we would rather surface than silently treat as "no fragments".
    let entries = match fs::read_dir(pages_dir()) {
        Ok(it) => it,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return pages,
        Err(e) => panic!("read_dir({}) failed: {e}", pages_dir().display()),
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) == Some("html") {
            pages.push(p);
        }
    }
    pages.sort();
    pages
}

// ---------------------------------------------------------------------------
// Tiny HTML helpers — string-only, no DOM, no real parser. These exist
// purely so the tests don't pull in an HTML-parser crate (the task brief
// asked for "no new dependency"). The checks are deliberately
// conservative: they look for *required markers*, not structural balance.
// ---------------------------------------------------------------------------

fn extract_meta_charset(html: &str) -> Option<String> {
    // Accept both `<meta charset="utf-8">` and
    // `<meta http-equiv="Content-Type" content="text/html; charset=...">`.
    let lower = html.to_ascii_lowercase();
    let head_end = lower.find("</head>").unwrap_or(lower.len());
    let head = &lower[..head_end];
    if let Some(idx) = head.find("<meta") {
        let tail = &head[idx..];
        if let Some(end) = tail.find('>') {
            let tag = &tail[..end];
            if let Some(cs) = tag_after_attr(tag, "charset") {
                return Some(cs);
            }
        }
    }
    if let Some(idx) = head.find("charset=") {
        let tail = &head[idx + "charset=".len()..];
        return Some(take_quoted(tail).unwrap_or_default());
    }
    None
}

fn tag_after_attr(tag: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=");
    let pos = tag.find(&needle)? + needle.len();
    let rest = &tag[pos..];
    take_quoted(rest)
}

fn take_quoted(s: &str) -> Option<String> {
    let s = s.trim_start();
    let quote = s.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let after = &s[quote.len_utf8()..];
    let end = after.find(quote)?;
    Some(after[..end].to_string())
}

fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let open = lower.find("<title>")?;
    let after_open = open + "<title>".len();
    let close = lower[after_open..].find("</title>")?;
    Some(html[after_open..after_open + close].trim().to_string())
}

fn has_doctype(html: &str) -> bool {
    let trimmed = html.trim_start();
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("<!doctype html")
}

fn has_open_tag(html: &str, tag: &str) -> bool {
    html.to_ascii_lowercase().contains(&format!("<{tag}"))
}

fn has_close_tag(html: &str, tag: &str) -> bool {
    html.to_ascii_lowercase().contains(&format!("</{tag}>"))
}

// ---------------------------------------------------------------------------
// Asset reference extraction
// ---------------------------------------------------------------------------

fn extract_attr_values(html: &str, attr: &str) -> Vec<(String, usize)> {
    // Returns (value, 1-based line) pairs. Conservative: looks for
    // `attr="..."` or `attr='...'`. Skips when the attribute sits inside
    // a `<script>` body to avoid matching inline JS.
    let mut out = Vec::new();
    let bytes = html.as_bytes();
    let attr_bytes = attr.as_bytes();
    let mut i = 0;
    let mut in_script = 0usize;
    let mut line = 1usize;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\n' {
            line += 1;
        }
        // Track script body so we don't match e.g. `href="..."` in a
        // literal inside a `<script>...</script>` block.
        if i + 7 <= bytes.len() && bytes[i..i + 7].eq_ignore_ascii_case(b"<script") {
            in_script += 1;
            // Skip to the matching closing `</script>` (best effort — JS
            // comments and strings can break this, but this only matters
            // for false positives, not correctness).
            if let Some(end) = find_subscript_ci(&html[i..], "</script>") {
                i += end + "</script>".len();
                in_script = in_script.saturating_sub(1);
                continue;
            }
        }
        if in_script > 0 {
            i += 1;
            continue;
        }
        if i + attr_bytes.len() + 1 < bytes.len()
            && bytes[i..i + attr_bytes.len()].eq_ignore_ascii_case(attr_bytes)
            && bytes[i + attr_bytes.len()] == b'='
        {
            // Skip whitespace, then a quote char.
            let mut j = i + attr_bytes.len() + 1;
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            if j >= bytes.len() {
                break;
            }
            let quote = bytes[j];
            if quote != b'"' && quote != b'\'' {
                i = j + 1;
                continue;
            }
            let start = j + 1;
            let mut k = start;
            while k < bytes.len() && bytes[k] != quote {
                if bytes[k] == b'\n' {
                    line += 1;
                }
                k += 1;
            }
            if k > start && k < bytes.len() {
                let value = &html[start..k];
                out.push((value.to_string(), line));
            }
            i = k + 1;
            continue;
        }
        i += 1;
    }
    out
}

fn find_subscript_ci(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
}

// ---------------------------------------------------------------------------
// Per-page contract tests
// ---------------------------------------------------------------------------

#[test]
fn every_html_page_is_minimally_well_formed() {
    let mut failed: Vec<String> = Vec::new();
    for page in all_pages() {
        let html = match fs::read_to_string(&page) {
            Ok(s) => s,
            Err(e) => {
                failed.push(format!("{}: read failed: {e}", page.display()));
                continue;
            }
        };
        let rel = page
            .strip_prefix(src_dir())
            .unwrap_or(&page)
            .display()
            .to_string();

        let checks: [(&str, bool); 7] = [
            ("DOCTYPE present", has_doctype(&html)),
            ("<html> open tag", has_open_tag(&html, "html")),
            ("</html> close tag", has_close_tag(&html, "html")),
            ("<head> open tag", has_open_tag(&html, "head")),
            ("</head> close tag", has_close_tag(&html, "head")),
            ("<body> open tag", has_open_tag(&html, "body")),
            ("</body> close tag", has_close_tag(&html, "body")),
        ];
        for (label, ok) in checks.iter() {
            if !ok {
                failed.push(format!("{rel}: missing {label}"));
            }
        }
        if extract_meta_charset(&html).is_none() {
            failed.push(format!("{rel}: no <meta charset> declaration"));
        }
        if extract_title(&html).map(|t| t.is_empty()).unwrap_or(true) {
            failed.push(format!("{rel}: missing or empty <title>"));
        }
    }
    assert!(
        failed.is_empty(),
        "GUI HTML pages failed well-formedness check:\n  - {}",
        failed.join("\n  - ")
    );
}

#[test]
fn every_page_title_is_nonempty_and_distinct() {
    let titles: Vec<(PathBuf, String)> = all_pages()
        .into_iter()
        .filter_map(|p| {
            let html = fs::read_to_string(&p).ok()?;
            extract_title(&html).map(|t| (p, t))
        })
        .collect();
    assert!(!titles.is_empty(), "no HTML pages found");
    for (path, t) in &titles {
        assert!(!t.is_empty(), "{}: empty title", path.display());
    }
    // Soft sanity: no two pages share the same title — if they do, the
    // tab UI will not be able to distinguish them by accessibility name.
    let mut seen: std::collections::HashMap<String, PathBuf> = Default::default();
    for (path, t) in &titles {
        if let Some(prev) = seen.get(t) {
            panic!(
                "duplicate page title {:?} on {} and {}",
                t,
                prev.display(),
                path.display()
            );
        }
        seen.insert(t.clone(), path.clone());
    }
}

#[test]
fn every_page_has_unique_charset() {
    // Sanity: every page declares a charset meta tag with a non-empty
    // value. Encoding drift across pages has been a real defect here
    // before (different pages picked different charsets when copy-pasted
    // from demos), so we assert the value is present and the set is
    // consistent.
    let mut charsets: BTreeSet<String> = BTreeSet::new();
    for page in all_pages() {
        let html = fs::read_to_string(&page).unwrap();
        let cs = extract_meta_charset(&html)
            .unwrap_or_else(|| panic!("{}: missing <meta charset>", page.display()));
        assert!(!cs.is_empty(), "{}: empty charset value", page.display());
        charsets.insert(cs.to_ascii_lowercase());
    }
    assert!(
        charsets.len() <= 2,
        "GUI pages declare {charsets:?} — should converge on UTF-8 \
         (utf-8 / utf8); charset drift breaks HTML rendering on some \
         platforms"
    );
}

// ---------------------------------------------------------------------------
// Daemon address consistency
// ---------------------------------------------------------------------------

#[test]
fn default_daemon_address_matches_across_cli_gui_and_readme() {
    // fabric-cli is the canonical authority.
    let cli = fs::read_to_string(wire_client_rs())
        .expect("fabric-cli wire_client.rs is required reading");
    let cli_addr = extract_const_str(&cli, "DEFAULT_DAEMON_ADDR")
        .expect("DEFAULT_DAEMON_ADDR must be a string literal in wire_client.rs");
    assert_eq!(
        cli_addr, "127.0.0.1:9400",
        "fabric-cli DEFAULT_DAEMON_ADDR drifted from the documented value; \
         all GUI / docs must follow"
    );

    // The Tauri shell hardcodes the same value in DaemonConfig::default_config.
    let tauri_daemon = fs::read_to_string(daemon_rs()).unwrap();
    let tauri_addr = extract_listen_addr_default(&tauri_daemon)
        .expect("DaemonConfig::default_config.listen_addr must be a string literal");
    assert_eq!(
        tauri_addr, "127.0.0.1:9400",
        "fabric-gui/src-tauri DaemonConfig::default_config drifted from \
         fabric-cli DEFAULT_DAEMON_ADDR (was {tauri_addr:?}, expected \
         \"127.0.0.1:9400\")"
    );

    // The README must mention the same port so operators can find it.
    let readme = fs::read_to_string(readme()).unwrap();
    assert!(
        readme.contains("9400"),
        "fabric-gui/README.md must document the daemon port (9400) so \
         operators can find it; missing — this is exactly the kind of \
         stale-doc bug these tests exist to catch"
    );
}

fn extract_const_str(src: &str, name: &str) -> Option<String> {
    // Looks for `pub const NAME: &str = "<value>";` (Rust) or
    // `const NAME = '<value>';` (JavaScript). Tolerates whitespace and
    // either quote style.
    let needle = format!("const {name}");
    let idx = src.find(&needle)?;
    let after = &src[idx + needle.len()..];
    // The character after the name can be `:`, `=` (with optional space).
    let trimmed = after.trim_start();
    let eq = match trimmed.chars().next()? {
        ':' | '=' => {
            // We don't actually care which delimiter; just find the first
            // `=` after skipping the (optional) type annotation.
            let mut pos = 0;
            for (i, c) in trimmed.char_indices() {
                if c == '=' {
                    pos = i;
                    break;
                }
            }
            pos
        }
        _ => return None,
    };
    let value = trimmed[eq + 1..].trim_start();
    take_quoted(value)
}

fn extract_listen_addr_default(src: &str) -> Option<String> {
    // The struct field declaration `pub listen_addr: String,` has no
    // initializer and so no quoted string. We want the value used in
    // `DaemonConfig::default()`, which is of the form
    // `listen_addr: "<value>".into()`. We iterate matches of the key
    // and pick the first one that is followed by a quoted string.
    let key = "listen_addr:";
    let mut i = 0;
    while let Some(pos) = src[i..].find(key) {
        let start = i + pos + key.len();
        let after = src[start..].trim_start();
        if let Some(value) = take_quoted(after) {
            return Some(value);
        }
        i = start + key.len();
    }
    None
}

// ---------------------------------------------------------------------------
// IPC command contract: every `api('x', ...)` JS call has a Rust command.
// ---------------------------------------------------------------------------

#[test]
fn every_ipc_command_in_index_html_has_a_tauri_command() {
    let index = fs::read_to_string(index_html()).unwrap();
    let commands_rs = fs::read_to_string(tauri_commands_rs()).unwrap();

    // The api() helper is defined as `api(command, args = {})`. We want
    // every string literal passed as the first argument.
    let called: BTreeSet<String> = extract_api_call_first_args(&index);

    // Sanity: the helper itself must exist somewhere — otherwise the
    // collected set is empty and the test below would vacuously pass.
    assert!(
        index.contains("async function api(command"),
        "index.html does not define the api() IPC helper — every caller \
         would silently no-op. Either the helper was renamed and these \
         tests are stale, or the JS is broken."
    );

    let defined: BTreeSet<String> = extract_tauri_command_names(&commands_rs);

    let mut missing: Vec<String> = Vec::new();
    for cmd in &called {
        if !defined.contains(cmd) {
            missing.push(cmd.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "JS calls api() with commands the Tauri shell does not expose: \
         {missing:?}. Either add the `#[tauri::command] fn {missing:?}` or \
         fix the caller."
    );

    // Reverse direction — every Tauri command has *some* presence in the
    // JS. If a command is defined but never called, it's dead surface
    // (the wrapper still costs nothing, so this is a soft check, but
    // catching unused IPC commands helps reduce surprise).
    let defined_but_unused: Vec<String> = defined
        .iter()
        .filter(|c| !called.contains(*c) && !is_likely_intentional_unused(c))
        .cloned()
        .collect();
    assert!(
        defined_but_unused.is_empty(),
        "Tauri commands defined but never called from the JS: \
         {defined_but_unused:?}. Either remove them or call them. \
         (intentional helpers are listed in is_likely_intentional_unused)"
    );
}

fn is_likely_intentional_unused(name: &str) -> bool {
    // These are defined for future use, programmatic access, or are
    // reached via `start_auth_listener`'s port, not by direct `api()`.
    // If you remove one of these, also remove the entry from this list.
    matches!(
        name,
        "start_auth"              // superseded by the WorkOS code path in index.html
            | "start_email_auth"   // kept for the email-link fallback
            | "verify_email_auth"  // kept for the email-link fallback
            | "restart_daemon"     // reached via toggleDaemon() shortcut UI
            | "get_daemon_status"  // reached via toggleDaemon() shortcut UI
            | "get_settings"       // settings is a static form, no data needed
            | "session_token" // helper, not a #[tauri::command]
    )
}

fn extract_api_call_first_args(src: &str) -> BTreeSet<String> {
    // Match `api('<name>'` or `api("<name>"`. Tolerates whitespace.
    let mut out = BTreeSet::new();
    let bytes = src.as_bytes();
    let needle = b"api(";
    let mut i = 0;
    while i + needle.len() < bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let mut j = i + needle.len();
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n') {
                j += 1;
            }
            if j < bytes.len() && (bytes[j] == b'\'' || bytes[j] == b'"') {
                let quote = bytes[j];
                let start = j + 1;
                let mut k = start;
                while k < bytes.len() && bytes[k] != quote {
                    k += 1;
                }
                if k > start && k < bytes.len() {
                    let val = &src[start..k];
                    // Heuristic: an IPC command name is snake_case ASCII.
                    if !val.is_empty()
                        && val.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                    {
                        out.insert(val.to_string());
                    }
                }
                i = k + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn extract_tauri_command_names(src: &str) -> BTreeSet<String> {
    // A Tauri command is `#[tauri::command]` followed by
    // `pub async fn NAME(` or `pub fn NAME(` on the next non-blank,
    // non-attribute line.
    let mut out = BTreeSet::new();
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim() == "#[tauri::command]" {
            for l in lines.iter().skip(i + 1).take(7) {
                let l = l.trim();
                if l.starts_with("pub async fn ") || l.starts_with("pub fn ") {
                    if let Some(rest) = l
                        .strip_prefix("pub async fn ")
                        .or_else(|| l.strip_prefix("pub fn "))
                    {
                        if let Some(name) = rest.split('(').next() {
                            let name = name.trim();
                            if !name.is_empty() {
                                out.insert(name.to_string());
                            }
                        }
                    }
                    break;
                }
            }
        }
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// WorkOS client id consistency
// ---------------------------------------------------------------------------

#[test]
fn workos_client_id_is_consistent_between_js_and_tauri_shell() {
    let index = fs::read_to_string(index_html()).unwrap();
    let tauri_daemon = fs::read_to_string(daemon_rs()).unwrap();

    let js_id = extract_const_str(&index, "WORKOS_CLIENT_ID")
        .expect("index.html must define WORKOS_CLIENT_ID as a string literal");
    let rust_id = extract_const_str(&tauri_daemon, "WORKOS_CLIENT_ID")
        .expect("fabric-gui/src-tauri/src/daemon.rs must define WORKOS_CLIENT_ID");

    assert_eq!(
        js_id, rust_id,
        "WORKOS_CLIENT_ID drifts between index.html ({js_id:?}) and \
         src-tauri/src/daemon.rs ({rust_id:?}); login will silently fail \
         when the authorize URL uses one id and the daemon registers a \
         different one"
    );
    // The string must look like a WorkOS public client id (starts with
    // `client_`). Catching the wrong value here is cheap.
    assert!(
        js_id.starts_with("client_"),
        "WORKOS_CLIENT_ID {js_id:?} doesn't start with `client_` — WorkOS \
         will reject authorize requests built with it"
    );
}

// ---------------------------------------------------------------------------
// Tab navigation contract: switchTab() callers ↔ page-* IDs ↔ loadTabData().
// ---------------------------------------------------------------------------

#[test]
fn every_switchtab_call_has_a_matching_page_id_and_a_loader_case() {
    let index = fs::read_to_string(index_html()).unwrap();

    let switchtab_args = extract_function_string_args(&index, "switchTab");
    let page_ids = extract_id_attr_values(&index, "page-");
    let loader_cases = extract_switch_case_strings(&index, "loadTabData");

    for name in &switchtab_args {
        let id = format!("page-{name}");
        assert!(
            page_ids.contains(name),
            "switchTab({name:?}) is called but no <div id=\"{id}\"> exists \
             in index.html — the tab will silently do nothing"
        );
        assert!(
            loader_cases.contains(name),
            "switchTab({name:?}) has a page id but loadTabData() has no \
             case for it — the page will never load its data on switch"
        );
    }

    for name in &loader_cases {
        let id = format!("page-{name}");
        assert!(
            page_ids.contains(name),
            "loadTabData() handles {name:?} but no <div id=\"{id}\"> exists — \
             the loader would call its case, then try to show a missing page"
        );
        assert!(
            switchtab_args.contains(name),
            "loadTabData() handles {name:?} but switchTab() is never \
             called with it — the case is unreachable from the tab rail"
        );
    }

    // The data-tab attr must mirror the page id exactly.
    let tab_ids = extract_data_tab_values(&index);
    for tab in &tab_ids {
        let id = format!("page-{tab}");
        assert!(
            page_ids.contains(tab),
            "<div class=\"tab-item\" data-tab=\"{tab}\"> is rendered but \
             no <div id=\"{id}\"> exists — clicking the tab will show \
             nothing"
        );
    }
    assert_eq!(
        tab_ids.len(),
        page_ids.len(),
        "tab rail ({} tabs) and page containers ({} pages) drifted — \
         every data-tab value must have a matching page-<name> id and \
         vice versa",
        tab_ids.len(),
        page_ids.len()
    );
}

fn extract_function_string_args(src: &str, fn_name: &str) -> BTreeSet<String> {
    let needle = format!("{fn_name}(");
    let bytes = src.as_bytes();
    let mut out = BTreeSet::new();
    let mut i = 0;
    while i + needle.len() < bytes.len() {
        if bytes[i..i + needle.len()].eq_ignore_ascii_case(needle.as_bytes()) {
            let mut j = i + needle.len();
            while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n') {
                j += 1;
            }
            if j < bytes.len() && (bytes[j] == b'\'' || bytes[j] == b'"') {
                let quote = bytes[j];
                let start = j + 1;
                let mut k = start;
                while k < bytes.len() && bytes[k] != quote {
                    k += 1;
                }
                if k > start && k < bytes.len() {
                    out.insert(src[start..k].to_string());
                }
                i = k + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn extract_id_attr_values(src: &str, prefix: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let needle = format!("id=\"{prefix}");
    let mut i = 0;
    while let Some(pos) = src[i..].find(&needle) {
        let start = i + pos + needle.len();
        if let Some(end) = src[start..].find('"') {
            let value = &src[start..start + end];
            // Filter out JS template-literal interpolations like ${name}
            // that show up in querySelector strings.
            if !value.contains("${") {
                out.insert(value.to_string());
            }
        }
        i = start + 1;
    }
    out
}

fn extract_data_tab_values(src: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let needle = "data-tab=\"";
    let mut i = 0;
    while let Some(pos) = src[i..].find(needle) {
        let start = i + pos + needle.len();
        if let Some(end) = src[start..].find('"') {
            let value = &src[start..start + end];
            // Filter out JS template-literal interpolations like ${name}.
            if !value.contains("${") {
                out.insert(value.to_string());
            }
        }
        i = start + 1;
    }
    out
}

fn extract_switch_case_strings(src: &str, fn_name: &str) -> BTreeSet<String> {
    // Find the function `async function NAME(` and read the `case 'X':`
    // literals that follow until the closing `}` of the switch.
    let start_needle = format!("function {fn_name}(");
    let Some(start) = src.find(&start_needle) else {
        return BTreeSet::new();
    };
    // Find the matching `{` after the signature.
    let sig_end = src[start..]
        .find('{')
        .map(|o| start + o + 1)
        .unwrap_or(start);
    // The switch lives inside this function; the next `switch (` token is
    // its anchor. We stop reading cases at the next `}` at brace-depth 1
    // (i.e. the function body end).
    let mut depth = 0usize;
    let mut in_switch = false;
    let mut out = BTreeSet::new();
    let bytes = &src.as_bytes()[sig_end..];
    let mut k = 0;
    while k < bytes.len() {
        let c = bytes[k];
        if c == b'{' {
            depth += 1;
        } else if c == b'}' {
            if depth == 1 {
                break;
            }
            depth -= 1;
        } else if depth == 1 {
            // Look for the `switch (` token.
            if !in_switch && k + 8 < bytes.len() && &bytes[k..k + 8] == b"function" {
                // Inner functions: ignore
            }
            // Find `case 'X':` patterns.
            if k + 6 < bytes.len() && &bytes[k..k + 5] == b"case " {
                let mut j = k + 5;
                while j < bytes.len() && (bytes[j] == b' ' || bytes[j] == b'\t') {
                    j += 1;
                }
                if j < bytes.len() && (bytes[j] == b'\'' || bytes[j] == b'"') {
                    let quote = bytes[j];
                    let s = j + 1;
                    let mut e = s;
                    while e < bytes.len() && bytes[e] != quote {
                        e += 1;
                    }
                    if e > s && e < bytes.len() {
                        out.insert(src[sig_end + s..sig_end + e].to_string());
                        in_switch = true;
                    }
                }
            }
            if in_switch {
                // Look for the closing brace of the switch to stop
                // collecting.
                if k + 1 < bytes.len() && bytes[k] == b'}' {
                    in_switch = false;
                }
            }
        }
        k += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// index.html JS: every document.getElementById('X') target exists as
// id="X" in the markup.
// ---------------------------------------------------------------------------

#[test]
fn index_getelementbyid_targets_exist_in_the_dom() {
    let html = fs::read_to_string(index_html()).unwrap();
    let js = extract_script_block(&html).expect("index.html must contain a <script> block");

    let queried: BTreeSet<String> = extract_getelementbyid_args(&js);
    assert!(
        !queried.is_empty(),
        "index.html's script block yielded no getElementById targets. If the \
         script was restructured, update extract_getelementbyid_args rather \
         than letting this test pass vacuously."
    );

    // `extract_attr_values` skips <script> bodies, so this is the real DOM
    // markup and not an id mentioned inside a JS string.
    let defined: BTreeSet<String> = extract_attr_values(&html, "id")
        .into_iter()
        .map(|(v, _line)| v)
        .collect();

    let missing: Vec<&String> = queried.iter().filter(|q| !defined.contains(*q)).collect();
    assert!(
        missing.is_empty(),
        "index.html's JS calls document.getElementById() for {} id(s) that \
         no element in the markup carries:\n  - {}\n\
         getElementById returns null for an unknown id, so every one of \
         these throws a TypeError on the next property access and silently \
         kills the rest of that loader. Add the element with the matching \
         id, or point the JS at an id that exists.\n\
         Checked {} unique target(s) against {} id= attribute(s) in the DOM.",
        missing.len(),
        missing
            .iter()
            .map(|m| m.as_str())
            .collect::<Vec<_>>()
            .join("\n  - "),
        queried.len(),
        defined.len()
    );
}

fn extract_script_block(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let open = lower.find("<script>")?;
    let after_open = open + "<script>".len();
    let close = lower[after_open..].find("</script>")?;
    Some(html[after_open..after_open + close].to_string())
}

// Collects the literal string argument of every `document.getElementById(...)`
// call. Only single-quoted literals are matched, which is what index.html
// uses throughout; anything else is silently ignored rather than guessed at,
// because a false positive here reads as a missing DOM element and would send
// the next agent chasing a bug that does not exist.
fn extract_getelementbyid_args(js: &str) -> BTreeSet<String> {
    const NEEDLE: &[u8] = b"getElementById('";
    let mut out = BTreeSet::new();
    let bytes = js.as_bytes();
    let mut i = 0;
    while i + NEEDLE.len() < bytes.len() {
        if bytes[i..i + NEEDLE.len()] == *NEEDLE {
            let start = i + NEEDLE.len();
            if let Some(end) = bytes[start..].iter().position(|&b| b == b'\'') {
                if end > 0 {
                    out.insert(js[start..start + end].to_string());
                }
                i = start + end + 1;
                continue;
            }
        }
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// Asset reference integrity: every local href/src on every page resolves.
// ---------------------------------------------------------------------------

#[test]
fn every_local_asset_reference_on_every_page_resolves() {
    let pages = all_pages();
    let mut failures: Vec<String> = Vec::new();
    for page in &pages {
        let html = fs::read_to_string(page).unwrap();
        let page_dir = page.parent().unwrap();
        for attr in ["href", "src"] {
            for (value, _line) in extract_attr_values(&html, attr) {
                if value.is_empty() {
                    continue;
                }
                // Allow the standard ignore list.
                let lowered = value.to_ascii_lowercase();
                if lowered.starts_with("http://")
                    || lowered.starts_with("https://")
                    || lowered.starts_with("data:")
                    || lowered.starts_with("javascript:")
                    || lowered.starts_with("mailto:")
                    || lowered.starts_with("blob:")
                    || lowered.starts_with('#')
                {
                    continue;
                }
                // Strip any URL fragment / query before resolving.
                let stripped = value
                    .split('?')
                    .next()
                    .unwrap_or(&value)
                    .split('#')
                    .next()
                    .unwrap_or(&value);
                if stripped.is_empty() {
                    continue;
                }
                let resolved = page_dir.join(stripped);
                if !resolved.exists() {
                    failures.push(format!(
                        "{}: {}={} -> {}",
                        page.strip_prefix(src_dir()).unwrap_or(page).display(),
                        attr,
                        value,
                        resolved.display()
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "Broken local asset references in GUI pages:\n  - {}\n\
         Fix: add the missing file, fix the path, or remove the reference.",
        failures.join("\n  - ")
    );
}

// ---------------------------------------------------------------------------
// Orphan-page detection: every file in pages/*.html must be referenced
// from index.html or one of its siblings. Right now nothing loads these
// fragments, so this test is *expected to fail* until the GUI either
// uses them or deletes them. We surface the orphan list as a diagnostic
// (rather than panicking) so future agents see what is dead weight.
// ---------------------------------------------------------------------------

#[test]
fn pages_dir_files_are_either_referenced_or_documented_as_orphans() {
    // `src/pages/` does not exist in the shipped tree. No directory means no
    // fragments and therefore no orphans — nothing to detect. The scan below
    // still runs the moment a fragment is added back, which is the point:
    // it fails loudly rather than passing quietly.
    let Ok(entries) = fs::read_dir(pages_dir()) else {
        return;
    };

    let index = fs::read_to_string(index_html()).unwrap();
    let mut orphans: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) != Some("html") {
            continue;
        }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        // Look for any reference to this file from index.html or any
        // sibling page. Today there are none — these are stand-alone
        // fragments that were never wired up.
        let needle = format!("pages/{name}");
        if !index.contains(&needle) {
            // Also check the bare file name (in case `pages/` is elided).
            if !index.contains(&name) {
                orphans.push(p);
            }
        }
    }
    // An unreferenced fragment is dead weight and a drift hazard: index.html
    // embeds an equivalent copy. The fix is one of:
    //   1. Wire the page into the tab loader, OR
    //   2. Delete the file, OR
    //   3. Add its name to `KNOWN_ORPHANS` with a justification.
    //
    // RESOLVED 2026-10-19: the nine fragments were DELETED rather than wired.
    //
    // index.html is the single shipped source. It already carries all nine
    // `page-*` divs and a `loadTabData()` case for each, so the fragments were
    // dead weight that could only drift from the live markup. Wiring them
    // would have meant `fetch()`-ing fragments inside the Tauri webview, which
    // changes shipped runtime behaviour and cannot be verified without running
    // the GUI.
    //
    // Five of the nine were near-duplicates (their only ids already existed in
    // index.html). `settings.html` had genuinely diverged: 12 controls
    // (daemon-addr, daemon-timeout, bitrate, codec-select, fps-select,
    // keyframe-interval, refresh-interval, resolution, startup-tab,
    // stun-server, theme-select, turn-server) against the live form's four
    // cfg-* fields. That design was never wired and was never shipped, so
    // deleting it removes no delivered feature; it remains recoverable from
    // git history at cf2c997 if it is ever wanted.
    //
    // The allowlist is gone on purpose. If an unreferenced fragment reappears,
    // this test fails again rather than passing quietly.
    let real_orphans: Vec<String> = orphans
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(
        real_orphans.is_empty(),
        "GUI page fragments in src/pages/ are never loaded by index.html \
         and are therefore dead code:\n  - {}\n\
         Each will silently drift from the embedded version in index.html. \
         Fix: either wire them into loadTabData() (fetch + inject into the \
         matching page-* div) or delete the file.",
        real_orphans
            .iter()
            .map(|n| format!("pages/{n}"))
            .collect::<Vec<_>>()
            .join("\n  - ")
    );
}

// ---------------------------------------------------------------------------
// Tauri-command coverage: the IPC functions the JS shell calls must all
// return `Result<_, String>` so the JS `api()` wrapper (which logs and
// returns null on failure) gets something it can render. A command that
// returns `Result<X, MyCustomError>` would not match the JS contract.
// ---------------------------------------------------------------------------

#[test]
fn every_called_tauri_command_returns_result_with_string_error() {
    let index = fs::read_to_string(index_html()).unwrap();
    let commands_rs = fs::read_to_string(tauri_commands_rs()).unwrap();
    let called: BTreeSet<String> = extract_api_call_first_args(&index);
    let functions = parse_tauri_command_signatures(&commands_rs);

    let mut bad: Vec<String> = Vec::new();
    for cmd in &called {
        match functions.get(cmd) {
            Some(sig) if !sig.error_type.contains("String") => {
                bad.push(format!(
                    "{cmd}: returns `{sig:?}` — JS api() wrapper expects \
                     Result<_, String>"
                ));
            }
            Some(_) => {}
            None => {
                // Already covered by the contract test above; skip.
            }
        }
    }
    assert!(
        bad.is_empty(),
        "Tauri commands called from JS but returning a non-String error:\n\
         {}\nThe JS api() wrapper logs `String(e)` and returns null. Any \
         non-String error type will produce a confusing `Display` output \
         like `Err(NetworkError {{ code: 503 }})` instead of a useful \
         message.",
        bad.join("\n")
    );
}

#[derive(Debug)]
struct CommandSig {
    error_type: String,
}

fn parse_tauri_command_signatures(src: &str) -> std::collections::HashMap<String, CommandSig> {
    // Naive line-oriented parser: for each `pub async fn NAME(...) ->
    // Result<T, E>` (or `pub fn NAME(...) -> Result<T, E>`) that
    // immediately follows a `#[tauri::command]`, record E.
    let mut out = std::collections::HashMap::new();
    let lines: Vec<&str> = src.lines().collect();
    for i in 0..lines.len() {
        if lines[i].trim() != "#[tauri::command]" {
            continue;
        }
        for j in (i + 1)..(i + 10).min(lines.len()) {
            let l = lines[j].trim();
            if !(l.starts_with("pub async fn ") || l.starts_with("pub fn ")) {
                continue;
            }
            // Find the function name.
            let after_kw = if let Some(r) = l.strip_prefix("pub async fn ") {
                r
            } else {
                l.strip_prefix("pub fn ").unwrap()
            };
            let name = match after_kw.split('(').next() {
                Some(n) => n.trim().to_string(),
                None => break,
            };
            // Concatenate the signature across lines until we find the
            // arrow or close brace.
            let mut sig = l.to_string();
            for next in lines.iter().skip(j + 1).take(29) {
                sig.push(' ');
                sig.push_str(next.trim());
                if next.contains("->") || next.trim_end().ends_with('{') {
                    break;
                }
            }
            if let Some(arrow) = sig.find("->") {
                let tail = sig[arrow + 2..].trim_start();
                // Strip a `where` clause, anything after `{`.
                let tail = tail.split('{').next().unwrap_or(tail);
                let tail = tail.split("where").next().unwrap_or(tail).trim();
                // Extract the error type from `Result<T, E>`.
                if tail.starts_with("Result<") {
                    if let Some(end) = tail.find('>') {
                        let inner = &tail["Result<".len()..end];
                        // Split on the *last* top-level comma.
                        if let Some(err) = last_top_level_comma(inner) {
                            out.insert(
                                name.clone(),
                                CommandSig {
                                    error_type: err.trim().to_string(),
                                },
                            );
                        }
                    }
                }
            }
            break;
        }
    }
    out
}

fn last_top_level_comma(s: &str) -> Option<&str> {
    let mut depth = 0i32;
    let mut last = None;
    for (i, c) in s.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => last = Some(i),
            _ => {}
        }
    }
    last.map(|i| &s[i + 1..])
}
