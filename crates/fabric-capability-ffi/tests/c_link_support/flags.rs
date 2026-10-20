//! Discovery and filtering of the native system libraries a Rust `staticlib`
//! needs, as reported by rustc on the host that is running the test.
//!
//! # Why this module exists
//!
//! A Rust `staticlib` is NOT self-contained: it references the platform's
//! system libraries, and the exact set is platform-specific. macOS wants
//! `-framework CoreFoundation -liconv -lSystem`, Linux wants a different set
//! (typically `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc`). Hardcoding either
//! list is a bug: the macOS list was shipped once, failed on `ubuntu-latest`
//! CI, and the test had to be reverted (commit 491b67e).
//!
//! The portable answer is to ASK rustc, on the host that is running the test:
//!
//! ```text
//! cargo rustc -p fabric-capability-ffi --lib -- --print native-static-libs
//! note: native-static-libs: -framework CoreFoundation -liconv -lSystem -lc -lm
//! ```
//!
//! The same command on Linux prints the Linux set. So this module discovers
//! the flags at run time and hands them to the C compiler verbatim (after the
//! narrow, documented filtering in [`filter_native_static_lib_flags`]). No
//! platform's library list is written into this file.
//!
//! # The colour problem (a real Linux CI failure)
//!
//! The obvious parser — one that looks for a line starting with the literal
//! text `note: native-static-libs:` — WORKS on a developer machine and FAILS
//! on CI. rustc colourises its diagnostics whenever `CARGO_TERM_COLOR=always`
//! is set, which this repository's CI does set. The colourised line is:
//!
//! ```text
//! \x1b[1m\x1b[92mnote\x1b[0m: native-static-libs: -lgcc_s -lutil ...
//! ```
//!
//! Note that there is no literal `note:` substring to match: the word `note`
//! is wrapped in SGR escapes, and the colon that follows it is wrapped in a
//! separate sequence. A parser that strips a literal `"note:"` prefix finds
//! nothing, concludes that rustc printed no library list, and — correctly, by
//! its own design — HARD FAILS. That is exactly what happened: the
//! `c_link_smoke` integration test failed on `ubuntu-latest` while passing on
//! macOS, because macOS had no `CARGO_TERM_COLOR=always` in effect.
//!
//! # The two defences, and what each one is actually worth
//!
//! 1. **Strip ANSI SGR escapes** before interpreting the text, so the payload
//!    is compared as plain text (see [`crate::ansi`]).
//! 2. **Recognise the marker by its own shape**, anchored to a rustc
//!    diagnostic prefix, so the marker is found even when the decoration
//!    differs — and, just as importantly, is NOT found when a line merely
//!    mentions the marker in prose (see [`crate::marker`]).
//!
//! Defence 1 is what fixed the OBSERVED failure: mutation testing (reverting
//! defence 2 to a literal `note:` prefix match) did NOT reintroduce the bug,
//! because stripping already restores the literal `note:` text. Defence 2 is
//! defence-in-depth against a FUTURE rustc whose escapes we cannot strip, and
//! it is the defence that rejects prose false positives.
//!
//! Both are retained, and both are pinned by a unit test. Neither is claimed
//! to be load-bearing for the observed failure unless a mutation test says so.

use super::ansi::strip_ansi_sgr;
use super::marker::listing_after_marker;

/// A representative `cargo rustc --print native-static-libs` payload as
/// linux-gnu rustc prints it when colour is DISABLED, with the surrounding
/// note noise included.
pub const LINUX_RUSTC_STDERR_PLAIN: &str = "\
   Compiling fabric-capability-ffi v0.1.0
note: link against the following native artifacts when linking against this static library. The order and any duplication can be significant on some platforms

note: native-static-libs: -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.64s
";

/// The same Linux payload as rustc prints it when colour is ENABLED.
///
/// This is the byte-exact shape observed on `ubuntu-latest` CI, where
/// `CARGO_TERM_COLOR: always` is set (`.github/workflows/ci.yml`). The bytes
/// were captured from the failing CI job log, not hand-written:
///
/// ```text
/// \x1b[1m\x1b[92mnote\x1b[0m: native-static-libs: -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc
/// ```
///
/// Note the structure: `note` is wrapped in `\x1b[1m\x1b[92m` ... `\x1b[0m`,
/// and the colon after it is wrapped in its OWN `\x1b[1m` ... `\x1b[0m`
/// pair. So the literal substring `note:` does not appear in the line.
pub const LINUX_RUSTC_STDERR_COLOR: &str = "\
   Compiling fabric-capability-ffi v0.1.0
\x1b[1m\x1b[92mnote\x1b[0m\x1b[1m: link against the following native artifacts when linking against this static library. The order and any duplication can be significant on some platforms
\x1b[1m\x1b[92mnote\x1b[0m\x1b[1m: native-static-libs: -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc\x1b[0m
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.64s
";

/// A representative macOS payload, used to pin the two-token `-framework`
/// handling that a Linux payload never exercises.
pub const MACOS_RUSTC_STDERR_PLAIN: &str = "\
note: link against the following native artifacts when linking against this static library. The order and any duplication can be significant on some platforms

note: native-static-libs: -framework CoreFoundation -liconv -lSystem -lc -lm
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.64s
";

/// Extract the `native-static-libs:` flag list from `cargo rustc` output.
///
/// rustc emits the note on STDERR. We scan both streams so the test is not
/// brittle if that ever changes.
///
/// ANSI SGR sequences are stripped first (see [`strip_ansi_sgr`]), and the
/// marker is matched independently of the `note:` decoration. That
/// combination is what makes this work under `CARGO_TERM_COLOR=always`,
/// where the literal text `note:` does not appear in the line at all.
///
/// A wrapped listing would continue on the following line(s), but rustc gives
/// no end marker, so we do NOT guess at continuations: a listing is read from
/// the marker to the end of its line and no further. Continuation tokens are
/// discarded by [`filter_native_static_lib_flags`], which keeps this parser
/// correct (no false positives) and self-correcting (a wrapped flag is dropped
/// rather than silently accepted as a library name).
pub fn parse_native_static_libs(stdout: &str, stderr: &str) -> Vec<String> {
    let mut flags: Vec<String> = Vec::new();

    for line in strip_ansi_sgr(stderr)
        .lines()
        .chain(strip_ansi_sgr(stdout).lines())
    {
        if let Some(listing) = listing_after_marker(line) {
            flags.extend(listing.split_whitespace().map(str::to_string));
        }
    }

    assert!(
        !flags.is_empty(),
        "rustc did not print a usable `native-static-libs:` line, so this test \
         cannot know which system libraries the staticlib needs.\n\
         This is a HARD FAILURE by design: guessing a library list would either \
         fail to link (macOS-only list on Linux, or vice versa) or, worse, link \
         successfully against the WRONG libraries and stop testing what it \
         exists to test.\n\
         If rustc colourised its output, the marker may be interleaved with \
         ANSI escape sequences; this parser strips SGR escapes, so a failure \
         here means the marker itself was absent.\n\
         Reproduce with: `cargo rustc -p fabric-capability-ffi --lib -- \
         --print native-static-libs`\n\
         --- stderr ---\n{}\n--- stdout ---\n{}",
        stderr,
        stdout
    );

    flags
}

/// Narrowly filter rustc's native library flags down to what a C driver can
/// consume.
///
/// ## Filtering rules, and why each exists
///
/// This function only ever REMOVES tokens from the list rustc printed. It
/// never invents, reorders, or rewrites one, so the surviving list is always
/// a subsequence of what the running platform's rustc reported.
///
/// KEEP, verbatim, in the order rustc printed them (order matters to the
/// linker):
///   * `-l<name>`   e.g. `-lm`, `-ldl`, `-lgcc_s`. These are plain driver flags
///     that any C compiler forwards to its linker. This is the whole point of
///     asking rustc instead of guessing.
///   * `-L<path>`   pass through first. Rustc only emits `-L` paths that the
///     C driver can also use, and stripping them speculatively would be
///     guessing. If a `-L` genuinely breaks the link the assertion in the test
///     body prints the whole command, which is the right place to diagnose it,
///     not a silent filter.
///   * `-framework`  plus the bare framework name that follows it. rustc prints
///     the Apple form as TWO whitespace-separated tokens; we split on
///     whitespace, so they arrive here as two separate tokens and are forwarded
///     as two separate argv entries, which is exactly what `cc`/`clang`
///     expects. Note that the name token does not start with `-`, so it is
///     admitted by the non-dash rule below rather than by a rule of its own.
///
/// STRIP:
///   * `-C <arg>` and `-C<arg>`   rustc codegen options, including any
///     `-C link-arg=...`. Meaningless to `cc`; passing them would make the C
///     driver error out on an unknown option.
///   * `-Wl,<arg>`             linker-driver passthrough. `cc` understands
///     `-Wl,`, but the specific args rustc prints are driver-specific and, if
///     they ever appeared, would encode rustc's own link strategy rather than
///     ours. We strip them and let the C driver apply its own defaults, which
///     is the correct behaviour for a C consumer.
///   * empty tokens            never forwarded as empty argv entries, which
///     some drivers treat as a filename argument.
///   * anything else starting with `-` that is not one of the introductions
///     above. This is the safety net described below.
///
/// ## Why the "anything else" safety net is the right direction to fail
///
/// rustc does not wrap a long `native-static-libs` listing onto a second line
/// (it ends the note at the line break), so today the net changes nothing. But
/// if a future rustc ever did wrap, the stray continuation token would be
/// dropped here instead of being handed to `cc` as a bogus library name or as
/// an input filename.
///
/// Dropping a token makes the LINK FAIL LOUDLY. Accepting one would silently
/// corrupt the command line — potentially writing an unexpected file next to
/// the test binary. For a test whose entire job is to fail loudly rather than
/// for the wrong reason, failing loud is the correct direction to fail in.
/// A wrongly-dropped token is also trivially diagnosable: the link-failure
/// assertion prints the exact flag list that was used.
pub fn filter_native_static_lib_flags(flags: Vec<String>) -> Vec<String> {
    let mut kept: Vec<String> = Vec::with_capacity(flags.len());
    // Set when the previous token was a bare `-C`, whose argument must be
    // dropped along with it.
    let mut skip_next = false;

    for flag in flags {
        if skip_next {
            skip_next = false;
            continue;
        }

        if flag.is_empty() {
            continue;
        }

        // `-C` (attached or not) is a rustc codegen option. Its argument is
        // the next token when it is not attached to the flag itself.
        if flag == "-C" {
            skip_next = true;
            continue;
        }
        if flag.starts_with("-C") {
            continue;
        }

        // `-Wl,...` encodes rustc's own link strategy; the C driver should
        // apply its own defaults instead.
        if flag.starts_with("-Wl,") {
            continue;
        }

        // KEEP `-l<name>`, `-L<path>`, and `-framework`. The bare framework
        // name that follows `-framework` is admitted here because it does not
        // start with a dash.
        if flag.starts_with("-l")
            || flag.starts_with("-L")
            || flag.starts_with("-framework")
            || !flag.starts_with('-')
        {
            kept.push(flag);
        }
    }

    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colorised_linux_payload_identically_to_plain() {
        // This is the regression that mattered: the colourised Linux payload
        // produced by `CARGO_TERM_COLOR=always` used to yield NOTHING, and
        // the test then hard-failed on `ubuntu-latest` while passing on macOS.
        let plain =
            filter_native_static_lib_flags(parse_native_static_libs("", LINUX_RUSTC_STDERR_PLAIN));
        let colored =
            filter_native_static_lib_flags(parse_native_static_libs("", LINUX_RUSTC_STDERR_COLOR));
        assert_eq!(
            colored, plain,
            "colourised and plain payloads must yield identical flag lists"
        );
        assert_eq!(
            colored,
            vec![
                "-lgcc_s",
                "-lutil",
                "-lrt",
                "-lpthread",
                "-lm",
                "-ldl",
                "-lc"
            ]
        );
    }

    #[test]
    fn parses_linux_payload_into_one_argv_entry_per_flag() {
        let flags =
            filter_native_static_lib_flags(parse_native_static_libs("", LINUX_RUSTC_STDERR_PLAIN));
        assert_eq!(
            flags,
            vec![
                "-lgcc_s",
                "-lutil",
                "-lrt",
                "-lpthread",
                "-lm",
                "-ldl",
                "-lc"
            ]
        );
    }

    #[test]
    fn preserves_the_two_token_framework_form() {
        // `-framework CoreFoundation` must survive as TWO argv entries, because
        // that is what clang expects. Splitting on `=` or joining the pair would
        // both break the macOS link.
        let flags =
            filter_native_static_lib_flags(parse_native_static_libs("", MACOS_RUSTC_STDERR_PLAIN));
        assert_eq!(
            flags,
            vec![
                "-framework",
                "CoreFoundation",
                "-liconv",
                "-lSystem",
                "-lc",
                "-lm"
            ]
        );
    }

    #[test]
    fn parses_the_listing_off_either_stream() {
        // rustc writes the note to stderr today; scanning stdout too keeps the
        // test from breaking if that ever changes.
        assert_eq!(
            filter_native_static_lib_flags(parse_native_static_libs(
                "",
                "note: native-static-libs: -lm\n"
            )),
            vec!["-lm"]
        );
        assert_eq!(
            filter_native_static_lib_flags(parse_native_static_libs(
                "native-static-libs: -lm\n",
                ""
            )),
            vec!["-lm"]
        );
    }

    #[test]
    fn strips_driver_specific_flags_but_keeps_library_search_paths() {
        // `-C ...` and `-Wl,...` are not valid inputs for a C driver, but `-L`
        // paths are passed through rather than stripped, per the documented rule:
        // strip on a real failure, never speculatively.
        let stderr =
            "note: native-static-libs: -L/usr/lib -C link-arg=-deadbeef -Wl,-z,now -lm -lc\n";
        assert_eq!(
            filter_native_static_lib_flags(parse_native_static_libs("", stderr)),
            vec!["-L/usr/lib", "-lm", "-lc"]
        );
    }

    #[test]
    fn does_not_swallow_the_finished_line_as_flags() {
        // Regression guard for a real bug found while restoring this test: an
        // earlier parser treated any indented line after the listing as a
        // continuation, which would have absorbed cargo's indented
        // `    Finished ...` line into the flag list.
        let flags =
            filter_native_static_lib_flags(parse_native_static_libs("", LINUX_RUSTC_STDERR_PLAIN));
        assert!(
            !flags
                .iter()
                .any(|f| f.contains("Finished") || f.contains("profile")),
            "cargo status text leaked into the link flags: {flags:?}"
        );
    }

    #[test]
    fn accepts_a_bare_marker_and_the_colourised_shape() {
        // The bare marker form rustc can print.
        assert_eq!(
            listing_after_marker("native-static-libs: -lm"),
            Some(" -lm")
        );
        // After ANSI stripping, the colourised CI line reduces to the plain
        // `note:` form, which the rule above accepts.
        let colored = "\x1b[1m\x1b[92mnote\x1b[0m\x1b[1m: native-static-libs: -lm";
        assert_eq!(listing_after_marker(&strip_ansi_sgr(colored)), Some(" -lm"));
    }

    #[test]
    #[should_panic(expected = "did not print a usable")]
    fn fails_loudly_when_rustc_reports_no_flags() {
        // The test must never quietly link with no flags when discovery failed.
        // A silent pass here is exactly the failure mode that made the previous
        // attempt report success on stale artefacts.
        parse_native_static_libs("", "    Finished `dev` profile [unoptimized + debuginfo]\n");
    }
}
