//! Toolchain discovery for the C link smoke test: locating a C compiler and
//! building the `fabric-capability-ffi` staticlib together with the list of
//! native system libraries that archive needs.
//!
//! # The artefact problem
//!
//! `cargo test --lib` does NOT build a `staticlib` artefact even when the
//! crate declares `crate-type = ["rlib", "staticlib"]`. CI failure mode: the
//! test panics because no `libfabric_capability_ffi.a` exists in
//! `target/debug/`. To survive a cold checkout, the test must BUILD the
//! staticlib itself.
//!
//! We invoke `cargo rustc -p fabric-capability-ffi --lib --target-dir <scratch>`
//! to a separate target directory. The separate target dir avoids the cargo
//! lock that the parent `cargo test` holds during test execution. That
//! invocation also emits the `staticlib`, and — because of the
//! `-- --print native-static-libs` flag — it prints the exact, platform
//! specific list of system libraries the archive needs.
//!
//! # The link-flag problem
//!
//! A Rust `staticlib` is NOT self-contained. It references the platform's
//! system libraries, and the exact set is platform-specific: macOS wants
//! `-framework CoreFoundation -liconv -lSystem`, Linux wants a different set
//! (typically `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc`). Hardcoding either
//! list is a bug, so the list is DISCOVERED at run time on the host that runs
//! the test. See [`super::flags`] for the parsing rules.
//!
//! # The C compiler problem
//!
//! Per the project contract: if no C compiler is available, this test must
//! FAIL LOUDLY with a clear message naming what is missing — never silently
//! skip. A previous attempt's silent skip is exactly what produced a test
//! that "passed locally with stale artefacts" but said nothing on CI.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::flags::{filter_native_static_lib_flags, parse_native_static_libs};

/// Locate the workspace root from the manifest dir. The manifest dir is the
/// crate's `CARGO_MANIFEST_DIR`; workspace root is two levels up
/// (`crates/<crate>/Cargo.toml` -> workspace root).
pub fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("CARGO_MANIFEST_DIR should be <workspace>/crates/<crate>")
        .to_path_buf()
}

/// The workspace `Cargo.toml` path.
pub fn manifest_path() -> PathBuf {
    workspace_root().join("Cargo.toml")
}

/// Pick a C compiler binary. The returned `Command` is configured with the
/// compiler as argv[0] only — caller adds flags.
///
/// Detection order (per platform):
///   * macOS: prefer `xcrun --sdk macosx clang` because invoking the bare
///     `clang` binary omits the SDKROOT/sysroot plumbing that `xcrun` sets
///     up, which causes system headers (`stdio.h`, etc.) to be missing.
///     This is the exact failure mode a previous attempt hit on this host:
///     it called `/Applications/Xcode.app/.../clang` directly and the C
///     compile failed with `'stdio.h' file not found`.
///   * All platforms: fall back to `cc`, `clang`, `gcc` in turn.
///
/// Returns `None` if NO compiler is found, which the caller MUST treat as
/// a hard failure (per the project contract: never silently skip).
pub fn pick_c_compiler() -> Option<Command> {
    let mut candidates: Vec<Command> = Vec::new();

    if cfg!(target_os = "macos") {
        // Probe whether `xcrun --sdk macosx clang` works at all. We use it
        // as the literal command (NOT just locating clang and invoking it
        // directly) because the bare binary has no SDK plumbing.
        if let Ok(out) = Command::new("xcrun")
            .args(["--sdk", "macosx", "clang", "--version"])
            .output()
        {
            if out.status.success() {
                let mut c = Command::new("xcrun");
                c.arg("--sdk").arg("macosx").arg("clang");
                candidates.push(c);
            }
        }
    }

    for name in ["cc", "clang", "gcc"] {
        if let Ok(out) = Command::new(name).arg("--version").output() {
            if out.status.success() {
                candidates.push(Command::new(name));
                break;
            }
        }
    }

    candidates.into_iter().next()
}

/// Build the `fabric-capability-ffi` staticlib into a SCRATCH target dir
/// that is OUTSIDE the workspace target dir. The scratch dir avoids the
/// cargo lock that the parent `cargo test` holds during test execution.
///
/// The build is done with `cargo rustc ... -- --print native-static-libs`
/// rather than `cargo build` because that single invocation gives us BOTH:
///
///   1. the `staticlib` archive we link against, and
///   2. the exact, platform-specific list of system libraries the archive
///      needs, as reported by rustc on THIS host.
///
/// Returns `(staticlib_path, native_static_libs_flags)`. Asserts that the
/// build succeeded, that the artefact exists, and that the
/// `native-static-libs` line was actually printed. All three are required for
/// the test to mean anything, and a missing flags line is a HARD FAILURE, not
/// a skip: silently linking without the flags would let the test pass for the
/// wrong reason on some hosts and fail on others.
///
/// No `--release`: the flags we read must describe the same debug archive we
/// link, so the profile has to match.
pub fn build_staticlib_and_native_libs(scratch_target_dir: &Path) -> (PathBuf, Vec<String>) {
    // The `CARGO` env var is set by the parent cargo invocation; fall back
    // to `cargo` on PATH if it is missing (e.g. if someone runs the test
    // binary directly without going through cargo).
    let cargo = std::env::var_os("CARGO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cargo"));

    // `--message-format=short` keeps the compile log terse. Note that it does
    // NOT suppress the `native-static-libs` note, which we read off stderr.
    //
    // We force plain output from the child cargo, AND we parse colourised
    // output. Both are deliberate, and both are needed:
    //
    //   * Forcing `CARGO_TERM_COLOR=never` (and `TERM=dumb`) on the child is
    //     what makes the NOTE READABLE in a CI log, where the surrounding job
    //     sets `CARGO_TERM_COLOR=always` and would otherwise wrap the
    //     diagnostic in escape bytes. This is the change from PR #54, and it
    //     was verified green on `ubuntu-24.04` in CI run 39353001542.
    //
    //   * Parsing colourised output anyway is defence in depth. The parser
    //     does not depend on the override being present, so a future change
    //     that drops it — or a different caller that invokes the parser on
    //     genuinely colourised output — cannot silently reintroduce the
    //     original Linux-only failure. This is pinned by the colour-parity
    //     test in [`crate::flags`], which feeds the parser the byte-exact
    //     colourised payload captured from the failing CI job log and
    //     requires it to yield the same flags as the plain payload.
    //
    // The override is therefore an ergonomic measure, not the correctness
    // boundary. The correctness boundary is the parser.
    let out = Command::new(&cargo)
        .env("CARGO_TERM_COLOR", "never")
        .env("TERM", "dumb")
        .arg("rustc")
        .arg("--manifest-path")
        .arg(manifest_path())
        .arg("--target-dir")
        .arg(scratch_target_dir)
        .arg("-p")
        .arg("fabric-capability-ffi")
        .arg("--lib")
        .arg("--message-format=short")
        .arg("--")
        .arg("--print")
        .arg("native-static-libs")
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "failed to invoke `cargo rustc --print native-static-libs` \
                 (cargo={:?}): {e}. Either the CARGO env var is unset (run via \
                 `cargo test`) or cargo itself is not on PATH.",
                cargo
            )
        });

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        out.status.success(),
        "`cargo rustc -p fabric-capability-ffi --lib -- --print \
         native-static-libs` failed with status {:?}.\n\
         The C link test cannot run without both the staticlib and its native \
         library list.\n--- stderr ---\n{}\n--- stdout ---\n{}",
        out.status,
        stderr,
        stdout
    );

    // Locate the staticlib in the scratch target dir. Cargo places it at
    // `<target>/<profile>/lib<name>.a` on Unix, `<target>/<profile>/<name>.lib`
    // on Windows. The crate's library name is `fabric_capability_ffi`.
    let staticlib_name = if cfg!(target_os = "windows") {
        "fabric_capability_ffi.lib"
    } else {
        "libfabric_capability_ffi.a"
    };
    let staticlib = scratch_target_dir.join("debug").join(staticlib_name);

    assert!(
        staticlib.exists(),
        "staticlib not found at expected path after build: {}. \
         This means cargo rustc succeeded but the expected artefact was not \
         produced. Cargo crate-type may have been changed; it must include \
         `staticlib` for this test to work.",
        staticlib.display()
    );

    let native_libs = filter_native_static_lib_flags(parse_native_static_libs(&stdout, &stderr));

    (staticlib, native_libs)
}
