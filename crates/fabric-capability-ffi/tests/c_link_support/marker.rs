//! Recognition of the `native-static-libs:` marker in rustc output.
//!
//! # What this module is responsible for
//!
//! Given one line of rustc output, decide whether that line carries the
//! `native-static-libs:` listing, and if so, return the text that follows the
//! marker.
//!
//! This is a SEPARATE concern from the ANSI stripping in [`crate::ansi`] and
//! from the flag filtering in [`crate::flags`], and it is separated from both
//! so that each concern can be read — and tested — on its own.
//!
//! # Why the rule is as strict as it is
//!
//! The marker is a plain-text substring, so a naive "does this line contain
//! the marker" rule would also match a line that merely MENTIONS the marker
//! in prose. That would inject junk tokens into the C link command line,
//! which may link against the wrong libraries or corrupt the command.
//!
//! A missed listing is the opposite failure: the parser finds no flags at
//! all and the test HARD FAILS with the full compiler output. Loud and safe.
//!
//! So the rule is deliberately biased towards rejecting. It admits only the
//! shapes rustc actually emits:
//!
//! ```text
//! native-static-libs: -lm                 (bare marker)
//! note: native-static-libs: -lm            (diagnostic prefix)
//! note: src/lib.rs:10:5: native-static-libs: -lm   (prefix + location)
//! ```
//!
//! and rejects everything else, including `notnative-static-libs:` and
//! `error: could not find native-static-libs: -lm`.

/// The marker rustc prints before the library list.
pub const PREFIX: &str = "native-static-libs:";

/// The rustc diagnostic levels that may introduce the marker.
///
/// A listing is only recognised when it sits directly behind one of these
/// prefixes (or at the very start of a line). That is what separates a real
/// listing from a line that merely MENTIONS the marker in prose.
const DIAGNOSTIC_PREFIXES: [&str; 4] = ["note:", "warning:", "error:", "help:"];

/// Return `true` if `text` is a rustc source location such as
/// `src/lib.rs:10:5:` or `src/lib.rs:10:`.
///
/// rustc renders a source location as `path:line:column:` (the column is
/// optional). We accept it so that a future rustc which annotates the
/// `native-static-libs` note with a span does not silently break the parser.
///
/// The check is deliberately strict, on both halves:
///
///   * the trailing segment must be a plain number (the line or column), and
///   * the segment before it must be a non-empty string that LOOKS like a
///     file path — it must contain a `.` or a `/`.
///
/// The second condition is what rejects a bare numeric pair such as
/// `10:5`, which is far more likely to be prose or an unrelated number than
/// a real location. Prose such as `could not find` is rejected by the first
/// condition, because it has no numeric segment at all.
fn is_source_location(text: &str) -> bool {
    // The trailing colon that rustc puts after the location is optional here.
    let text = text.strip_suffix(':').unwrap_or(text);

    // Split from the right: `path`, then `line`, then an optional `column`.
    let mut segments = text.rsplit(':');

    // The first segment we pull back is the line number. It must be numeric.
    let Some(line) = segments.next() else {
        return false;
    };
    if !is_all_digits(line) {
        return false;
    }

    // The next segment is either the column (when present) or the path.
    let Some(next) = segments.next() else {
        // Only one segment: a bare number is not a location.
        return false;
    };

    if is_all_digits(next) {
        // That was the column. The one after it is the path.
        match segments.next() {
            Some(path) => looks_like_path(path),
            None => false,
        }
    } else {
        // That was the path.
        looks_like_path(next)
    }
}

/// Return `true` if `text` is a non-empty run of ASCII digits.
fn is_all_digits(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

/// Return `true` if `text` looks like a file path rather than a word.
///
/// A path contains a `.` (an extension, e.g. `lib.rs`) or a `/` (a directory
/// separator). Requiring one of those is what rejects a bare numeric pair
/// such as `10:5`, which is far more likely to be prose or an unrelated
/// number than a real location.
fn looks_like_path(text: &str) -> bool {
    !text.is_empty() && (text.contains('.') || text.contains('/'))
}

/// Return the text that follows the `native-static-libs:` marker on `line`.
///
/// The marker is accepted only when it is
///
///   * at the very start of the line (rustc printed the bare marker), or
///   * directly preceded by a rustc diagnostic prefix — `note:`, `warning:`,
///     `error:` or `help:` — optionally followed by a source location.
///
/// This rejects the two false-positive shapes that a "whitespace-delimited
/// marker" rule would accept:
///
///   * a marker glued to a word (`notnative-static-libs:`), and
///   * a line that mentions the marker in prose
///     (`error: could not find native-static-libs: -lm`), which would
///     otherwise contribute a bogus listing to the flag list.
///
/// # Why false negatives are acceptable but false positives are not
///
/// A listing we fail to recognise causes the parser to find NO flags, and the
/// test then HARD FAILS with the full compiler output. That is loud and safe.
///
/// A listing we wrongly recognise contributes junk tokens to the C link
/// command line, which may link against the WRONG libraries or corrupt the
/// command. That is silent and dangerous.
///
/// So the rule is biased towards rejecting: it admits only the shapes rustc
/// actually emits, rather than anything that merely contains the marker.
///
/// It still accepts every shape rustc actually emits, whether colourised or
/// not, because ANSI stripping restores the plain `note:` text first.
pub fn listing_after_marker(line: &str) -> Option<&str> {
    let (before, after) = line.split_once(PREFIX)?;

    // Trailing whitespace is not significant: rustc emits `note: <marker>`.
    let before = before.trim_end();

    if before.is_empty() {
        return Some(after);
    }

    // Strip a rustc diagnostic prefix, if the line carries one.
    let rest = DIAGNOSTIC_PREFIXES
        .iter()
        .find_map(|prefix| before.strip_suffix(prefix))
        .unwrap_or(before)
        .trim();

    if rest.is_empty() {
        return Some(after);
    }

    // The only thing allowed between the diagnostic prefix and the marker is
    // a source location.
    if is_source_location(rest) {
        return Some(after);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_a_rustc_source_location() {
        assert!(is_source_location("src/lib.rs:10:5:"));
        assert!(is_source_location("src/lib.rs:10:5"));
        assert!(is_source_location("src/lib.rs:10:"));
        assert!(is_source_location("src/lib.rs:10"));
        // A bare number is not a location: it has no path.
        assert!(!is_source_location("10:5:"));
        // Prose is not a location.
        assert!(!is_source_location("could not find"));
        assert!(!is_source_location("error"));
        assert!(!is_source_location(""));
    }

    #[test]
    fn rejects_a_marker_that_is_glued_to_a_word() {
        // `notnative-static-libs:` must not be treated as the marker.
        assert_eq!(listing_after_marker("notnative-static-libs: -lm"), None);
        assert_eq!(
            listing_after_marker("native-static-libs: -lm"),
            Some(" -lm")
        );
        assert_eq!(
            listing_after_marker("note: native-static-libs: -lm"),
            Some(" -lm")
        );
    }

    #[test]
    fn rejects_a_marker_mentioned_in_prose() {
        // A line that merely MENTIONS the marker must not contribute a
        // listing. Under a "whitespace-delimited marker" rule this line
        // would have contributed the bogus token `-lm` to the flag list.
        assert_eq!(
            listing_after_marker("error: could not find native-static-libs: -lm"),
            None
        );
        assert_eq!(
            listing_after_marker("note: see the native-static-libs: documentation"),
            None
        );
        // A diagnostic prefix followed by a source location is still accepted.
        assert_eq!(
            listing_after_marker("note: src/lib.rs:10:5: native-static-libs: -lm"),
            Some(" -lm")
        );
    }

    #[test]
    fn accepts_a_bare_marker() {
        // The bare marker form rustc can print.
        assert_eq!(
            listing_after_marker("native-static-libs: -lm"),
            Some(" -lm")
        );
    }
}
