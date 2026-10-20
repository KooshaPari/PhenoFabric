//! Removal of ANSI SGR (Select Graphic Rendition) escape sequences from
//! compiler output.
//!
//! # Why this exists
//!
//! rustc colourises its diagnostics whenever `CARGO_TERM_COLOR=always` is
//! set. This repository's CI sets it (`.github/workflows/ci.yml`), so on CI
//! the diagnostic that carries the native library list arrives decorated:
//!
//! ```text
//! \x1b[1m\x1b[92mnote\x1b[0m: native-static-libs: -lgcc_s -lutil ...
//! ```
//!
//! The word `note` is wrapped in SGR escapes and the colon after it is
//! wrapped in its own separate sequence. There is therefore NO literal
//! `note:` substring to match, and any parser that strips a literal `"note:"`
//! prefix finds nothing at all.
//!
//! This module is the single place where that decoration is removed, so the
//! rest of the discovery pipeline can treat compiler output as plain text.

/// Remove ANSI SGR (Select Graphic Rendition) escape sequences from `text`.
///
/// rustc colourises diagnostics with SGR sequences: `ESC [ <params> m`, where
/// `ESC` is `0x1b` and `<params>` is a run of digits and `;`. We remove the
/// whole sequence, including the terminating `m`.
///
/// Only SGR sequences are removed. Other escape families (cursor movement,
/// OSC, etc.) are left untouched: rustc does not emit them in this note, and
/// silently mangling unrelated bytes would be worse than leaving them.
pub fn strip_ansi_sgr(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;

    while i < bytes.len() {
        // A CSI sequence starts with ESC '[' and ends at its final byte.
        // For SGR, that final byte is 'm'.
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            let mut j = i + 2;
            // Parameter bytes: 0x30-0x3F (digits, ';', ':', '<', '=', '>', '?').
            while j < bytes.len() && (0x30..=0x3f).contains(&bytes[j]) {
                j += 1;
            }
            // Intermediate bytes: 0x20-0x2F.
            while j < bytes.len() && (0x20..=0x2f).contains(&bytes[j]) {
                j += 1;
            }
            // Final byte: 0x40-0x7E.
            if j < bytes.len() && (0x40..=0x7e).contains(&bytes[j]) {
                if bytes[j] == b'm' {
                    // SGR: drop the whole sequence.
                    i = j + 1;
                    continue;
                }
                // Some other CSI sequence: keep it verbatim by copying
                // through the final byte.
                out.push_str(&text[i..=j]);
                i = j + 1;
                continue;
            }
            // Unterminated sequence: treat the ESC as an ordinary byte.
        }

        // Copy one UTF-8 character. `text` is valid UTF-8, so the byte at
        // `i` starts a character whose length we can determine from its
        // leading byte.
        let ch_len = utf8_char_len(bytes[i]);
        let end = (i + ch_len).min(bytes.len());
        out.push_str(&text[i..end]);
        i = end;
    }

    out
}

/// Length in bytes of the UTF-8 character starting with `first`.
///
/// Falls back to 1 for a byte that cannot start a multi-byte sequence, which
/// keeps the caller from ever splitting a character.
fn utf8_char_len(first: u8) -> usize {
    match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_sgr_sequences_and_keeps_the_payload() {
        let colored = "\x1b[1m\x1b[92mnote\x1b[0m\x1b[1m: native-static-libs: -lm\x1b[0m";
        assert_eq!(
            strip_ansi_sgr(colored),
            "note: native-static-libs: -lm",
            "SGR escapes must be removed so the marker is plain text"
        );
    }

    #[test]
    fn strips_the_exact_colourised_form_seen_on_linux_ci() {
        // Byte-exact shape captured from the failing `ubuntu-latest` job log
        // of run 39331325950. The literal text `note:` does NOT appear in
        // this string, which is precisely why the old parser found nothing.
        let ci_line = "\x1b[1m\x1b[92mnote\x1b[0m: native-static-libs: -lgcc_s -lutil";
        assert_eq!(
            strip_ansi_sgr(ci_line),
            "note: native-static-libs: -lgcc_s -lutil"
        );
        assert!(
            !ci_line.contains("note:"),
            "the captured CI line must not contain a literal 'note:' — if it \
             does, the fixture no longer reproduces the original bug"
        );
    }

    #[test]
    fn strips_bare_escape_and_reset_sequences() {
        assert_eq!(strip_ansi_sgr("\x1b[0m\x1b[1mplain\x1b[0m"), "plain");
        assert_eq!(strip_ansi_sgr("no escapes here"), "no escapes here");
        assert_eq!(strip_ansi_sgr(""), "");
    }

    #[test]
    fn strips_multi_parameter_sgr_sequences() {
        assert_eq!(strip_ansi_sgr("\x1b[1;31;40mred\x1b[0m"), "red");
        assert_eq!(
            strip_ansi_sgr("\x1b[38;2;255;0;0mtruecolor\x1b[0m"),
            "truecolor"
        );
    }

    #[test]
    fn preserves_non_sgr_escape_sequences_verbatim() {
        // A cursor-movement sequence is not SGR; it must survive untouched
        // rather than being silently swallowed.
        let text = "\x1b[2Kvisible";
        assert_eq!(strip_ansi_sgr(text), text);
    }

    #[test]
    fn preserves_non_ascii_text() {
        // The stripper walks bytes; it must not split multi-byte characters.
        let text = "note: caf\u{e9} \u{1f600} native-static-libs: -lm";
        assert_eq!(strip_ansi_sgr(text), text);
    }

    #[test]
    fn handles_a_truncated_escape_sequence_without_panicking() {
        // A trailing ESC with nothing after it must not panic or loop.
        assert_eq!(strip_ansi_sgr("text\x1b"), "text\x1b");
        assert_eq!(strip_ansi_sgr("text\x1b["), "text\x1b[");
    }
}
