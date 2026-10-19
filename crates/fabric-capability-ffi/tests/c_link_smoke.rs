//! True cross-language link smoke test for `fabric-capability-ffi`.
//!
//! Compiles a small C program against `include/fabric_capability.h`, links
//! it against the `fabric-capability-ffi` static library, runs the binary,
//! and asserts on the exit code. This catches ABI bugs at the level a real
//! C consumer would experience them — wrong symbol naming, wrong calling
//! convention, mismatched enum widths, missing functions, phantom symbols,
//! and so on.
//!
//! # The artefact problem
//!
//! `cargo test --lib` does NOT build a `staticlib` artefact even when the
//! crate declares `crate-type = ["rlib", "staticlib"]`. CI failure mode:
//! the test panics because no `libfabric_capability_ffi.a` exists in
//! `target/debug/`. To survive a cold checkout, the test must BUILD the
//! staticlib itself.
//!
//! We invoke `cargo rustc -p fabric-capability-ffi --lib --target-dir <scratch>`
//! to a separate target directory. The separate target dir avoids the cargo
//! lock that the parent `cargo test` holds during test execution (verified
//! empirically, see the in-tree design note). That invocation also emits the
//! `staticlib`. After the build, we locate the staticlib at
//! `<scratch>/debug/libfabric_capability_ffi.a` and link against it.
//!
//! # The link-flag problem (the reason a previous attempt was reverted)
//!
//! A Rust `staticlib` is NOT self-contained. It references the platform's
//! system libraries, and the exact set is platform-specific: macOS wants
//! `-framework CoreFoundation -liconv -lSystem`, Linux wants a different set
//! (typically `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc`).
//!
//! Hardcoding either list is a bug: the macOS list was shipped once, failed
//! on `ubuntu-latest` CI, and the test had to be reverted (commit 491b67e).
//! The portable answer is to ASK RUSTC, on the host that is running the test,
//! what the archive actually needs:
//!
//! ```text
//! cargo rustc -p fabric-capability-ffi --lib -- --print native-static-libs
//! note: native-static-libs: -framework CoreFoundation -liconv -lSystem -lc -lm
//! ```
//!
//! The same command on Linux prints the Linux set. So this test discovers the
//! flags at run time and passes them to the C compiler verbatim (after the
//! narrow, documented filtering in `filter_native_static_lib_flags`). No
//! platform's library list is written into this file.
//!
//! rustc writes the note to STDERR, and re-emits it on warm/cached runs, so
//! the discovery step is stable regardless of whether the target dir was just
//! built or reused.
//!
//! # The C compiler problem
//!
//! Per the project contract: if no C compiler is available, this test must
//! FAIL LOUDLY with a clear message naming what is missing — never silently
//! skip. The previous attempt's silent skip is exactly what produced a test
//! that "passed locally with stale artefacts" but said nothing on CI.
//!
//! # The symbol-tooling problem
//!
//! We deliberately do NOT use `nm`/`objdump`/`llvm-nm` to assert absence of
//! a symbol. The portable way to assert that a header-declared symbol
//! resolves is to actually compile and link C that uses (or, in the case
//! of a phantom, would use) it. The `c_link_smoke` test here does the
//! positive half: every header-declared function is called by the C
//! program below, so a missing symbol causes a link error. The negative
//! half (no phantom symbol) is asserted in `tests/abi.rs` by a portable
//! header-string check, which is the most robust portable assertion.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The C program is intentionally small. It exercises every FFI function
/// the library exports and exits 0 only if each call returns the expected
/// outcome. If the C ↔ Rust contract breaks (ABI drift, header missing
/// symbol, wrong enum value, etc.) the C program will fail to compile,
/// fail to link, or return non-zero at runtime.
///
/// IMPORTANT: this program is the test. If it changes, the assertions it
/// pins change. Every branch that does NOT exit with EXIT_FAILURE
/// represents a passed contract assertion.
const C_PROGRAM: &str = r#"
// fabric-capability-ffi cross-language smoke test.
// Every branch that does NOT exit with EXIT_FAILURE represents a passed
// contract assertion.

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fabric_capability.h"

#define PASS(name) do { printf("  ok %s\n", name); fflush(stdout); } while (0)
#define FAIL(name, why) do { fprintf(stderr, "FAIL %s: %s\n", name, why); fflush(stderr); exit(EXIT_FAILURE); } while (0)

int main(void) {
    // --- error_message: every code must return a non-NULL non-empty UTF-8-ish string.
    for (FabricError c = 0; c <= FABRIC_NULL_POINTER; c++) {
        const char *m = fabric_capability_error_message(c);
        if (!m) FAIL("error_message NULL", "returned NULL");
        if (strlen(m) == 0) FAIL("error_message empty", "returned empty string");
    }
    PASS("error_message");

    // --- free_string(NULL): must be a safe no-op.
    fabric_capability_free_string(NULL);
    PASS("free_string(NULL)");

    // --- to_json(NULL): must return NULL.
    if (fabric_capability_to_json(NULL) != NULL) FAIL("to_json(NULL)", "did not return NULL");
    PASS("to_json(NULL)");

    // --- generate_key: must populate 32 bytes; NULL out returns FABRIC_NULL_POINTER.
    unsigned char seed[32];
    if (fabric_capability_generate_key(NULL) != FABRIC_NULL_POINTER)
        FAIL("generate_key(NULL)", "did not return FABRIC_NULL_POINTER");
    if (fabric_capability_generate_key(seed) != FABRIC_OK)
        FAIL("generate_key(ok)", "did not return FABRIC_OK");
    PASS("generate_key");

    // --- key_id: must return a NUL-terminated heap string (must NULL-safe).
    if (fabric_capability_key_id(NULL) != NULL) FAIL("key_id(NULL)", "did not return NULL");
    char *kid = fabric_capability_key_id(seed);
    if (!kid) FAIL("key_id(ok)", "returned NULL");
    if (strlen(kid) == 0) FAIL("key_id(ok)", "returned empty string");
    // Determinism: same seed -> same fingerprint.
    char *kid2 = fabric_capability_key_id(seed);
    if (!kid2 || strcmp(kid, kid2) != 0)
        FAIL("key_id deterministic", "two calls produced different fingerprints");
    fabric_capability_free_string(kid);
    fabric_capability_free_string(kid2);
    PASS("key_id");

    // --- sign + verify round-trip with the generated seed.
    // The FFI's sign and verify both take a 32-byte SEED. Verify derives the
    // verification key from the seed internally, so the same 32 bytes used
    // to sign must also verify the signed descriptor.
    const char *desc =
        "{\"node_id\":\"00000000-0000-7000-8000-000000000001\","
        "\"epoch\":1,"
        "\"schema_version\":\"phenotype.fabric.capability_descriptor/1\","
        "\"probed_at\":\"2026-01-01T00:00:00Z\","
        "\"probe_version\":\"0.1.0\","
        "\"topology_hash\":\"deadbeef\","
        "\"capabilities\":{},"
        "\"signatures\":[]}";
    char *signed_desc = NULL;
    FabricError e = fabric_capability_sign_json(desc, seed, &signed_desc);
    if (e != FABRIC_OK) FAIL("sign_json(ok)", "did not return FABRIC_OK");
    if (!signed_desc) FAIL("sign_json(ok)", "out_signed was NULL");
    if (!strstr(signed_desc, "\"signatures\""))
        FAIL("sign_json(ok)", "signed descriptor lacks signatures field");
    PASS("sign_json");

    // Same-seed verify must succeed: the descriptor was signed with this
    // exact seed, and the FFI's verify derives the matching verification key.
    FabricError vself = fabric_capability_verify_json(signed_desc, seed);
    if (vself != FABRIC_OK)
        FAIL("verify_json(same seed)", "verify under the same seed that signed did not return FABRIC_OK");
    PASS("verify_json(same seed)");

    // --- NULL handling for sign_json.
    char *placeholder = NULL;
    if (fabric_capability_sign_json(NULL, seed, &placeholder) != FABRIC_NULL_POINTER)
        FAIL("sign_json(NULL desc)", "did not return FABRIC_NULL_POINTER");
    if (fabric_capability_sign_json(desc, NULL, &placeholder) != FABRIC_NULL_POINTER)
        FAIL("sign_json(NULL key)", "did not return FABRIC_NULL_POINTER");
    if (fabric_capability_sign_json(desc, seed, NULL) != FABRIC_NULL_POINTER)
        FAIL("sign_json(NULL out)", "did not return FABRIC_NULL_POINTER");
    PASS("sign_json(NULL *)");

    // --- verify_json NULL handling.
    if (fabric_capability_verify_json(NULL, seed) != FABRIC_NULL_POINTER)
        FAIL("verify_json(NULL desc)", "did not return FABRIC_NULL_POINTER");
    if (fabric_capability_verify_json(signed_desc, NULL) != FABRIC_NULL_POINTER)
        FAIL("verify_json(NULL seed)", "did not return FABRIC_NULL_POINTER");
    PASS("verify_json(NULL *)");

    // --- Cross-seed rejection: a different 32-byte seed must not verify.
    unsigned char other_seed[32];
    memset(other_seed, 0xCC, sizeof(other_seed));
    FabricError verr = fabric_capability_verify_json(signed_desc, other_seed);
    if (verr == FABRIC_OK)
        FAIL("verify_json(other seed)", "forged signature verified under a different seed");
    PASS("verify_json(rejects other seed)");

    // --- The FFI's contract for the key is "a pointer to 32 bytes" — the
    // length is fixed by the signature, not by the input. There is no
    // `bad key len` runtime error to surface here: the consumer must
    // guarantee the buffer size. We instead verify that the key bytes
    // are actually USED (round-trip via sign+verify below).

    // --- Serde error on malformed descriptor.
    char *out_buf = NULL;
    if (fabric_capability_sign_json("not json {", seed, &out_buf) != FABRIC_SERDE)
        FAIL("sign_json(bad desc)", "did not return FABRIC_SERDE");
    PASS("sign_json(Serde on bad desc)");

    // --- Header must NOT export `fabric_capability_free_key` (phantom fix).
    // The C compile would NOT directly catch a phantom declaration, but
    // if the header declared a phantom AND the program called it, the link
    // step would fail. We rely on tests/abi.rs for the portable absence
    // assertion (header string search). Here we pin the positive half:
    // every symbol we use below MUST resolve at link time.

    fabric_capability_free_string(signed_desc);
    printf("c_link_smoke: all FFI contracts verified from C\n");
    return 0;
}
"#;

/// Locate the workspace root from the manifest dir. The manifest dir is the
/// crate's `CARGO_MANIFEST_DIR`; workspace root is two levels up
/// (`crates/<crate>/Cargo.toml` -> workspace root).
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("CARGO_MANIFEST_DIR should be <workspace>/crates/<crate>")
        .to_path_buf()
}

fn manifest_path() -> PathBuf {
    workspace_root().join("Cargo.toml")
}

/// Pick a C compiler binary. The returned `Command` is configured with the
/// compiler as argv[0] only — caller adds flags.
///
/// Detection order (per platform):
///   * macOS: prefer `xcrun --sdk macosx clang` because invoking the bare
///     `clang` binary omits the SDKROOT/sysroot plumbing that `xcrun` sets
///     up, which causes system headers (`stdio.h`, etc.) to be missing.
///     This is the exact failure mode the previous attempt hit on this
///     host: it called `/Applications/Xcode.app/.../clang` directly and
///     the C compile failed with `'stdio.h' file not found`.
///   * All platforms: fall back to `cc`, `clang`, `gcc` in turn.
///
/// Returns `None` if NO compiler is found, which the caller MUST treat as
/// a hard failure (per the project contract: never silently skip).
fn pick_c_compiler() -> Option<Command> {
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
fn build_staticlib_and_native_libs(scratch_target_dir: &Path) -> (PathBuf, Vec<String>) {
    // The `CARGO` env var is set by the parent cargo invocation; fall back
    // to `cargo` on PATH if it's missing (e.g. if someone runs the test
    // binary directly without going through cargo).
    let cargo = std::env::var_os("CARGO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("cargo"));

    // `--message-format=short` keeps the compile log terse. Note that it does
    // NOT suppress the `native-static-libs` note, which we read off stderr.
    let out = Command::new(&cargo)
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

    let native_libs = parse_native_static_libs(&stdout, &stderr);
    let native_libs = filter_native_static_lib_flags(native_libs);

    (staticlib, native_libs)
}

/// Extract the `native-static-libs:` flag list from `cargo rustc` output.
///
/// rustc emits the note on STDERR, formatted as `note: native-static-libs:
/// <flags>`. We scan both streams so the test is not brittle if that ever
/// changes, and we skip the `note: ` decoration itself rather than assuming
/// the prefix starts the line.
///
/// A wrapped listing would continue on the following line(s), but rustc gives
/// no end marker, so we do NOT guess at continuations: a listing is read from
/// the prefix to the end of its line and no further. Continuation tokens are
/// discarded by `filter_native_static_lib_flags`, which keeps this parser
/// correct (no false positives) and self-correcting (a wrapped flag is dropped
/// rather than silently accepted as a library name).
fn parse_native_static_libs(stdout: &str, stderr: &str) -> Vec<String> {
    const PREFIX: &str = "native-static-libs:";

    let mut flags: Vec<String> = Vec::new();

    for line in stderr.lines().chain(stdout.lines()) {
        // Skip rustc's `note: ` decoration so we match the payload wherever
        // it sits in the line.
        let payload = line
            .trim_start()
            .strip_prefix("note:")
            .unwrap_or_else(|| line.trim_start())
            .trim_start();

        if let Some(rest) = payload.strip_prefix(PREFIX) {
            flags.extend(rest.split_whitespace().map(str::to_string));
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
/// never invents, reorders, or rewrites one, so the surviving list is always a
/// subsequence of what the running platform's rustc reported.
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
fn filter_native_static_lib_flags(flags: Vec<String>) -> Vec<String> {
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

#[test]
fn c_link_smoke() {
    // 1. C compiler detection — fail loudly if missing (per project contract).
    let mut compile_cmd = pick_c_compiler().unwrap_or_else(|| {
        panic!(
            "c_link_smoke: no C compiler found on PATH. Tried \
             `xcrun --sdk macosx clang` (macOS), `cc`, `clang`, `gcc`. \
             Install a C toolchain (e.g. build-essential on Debian/Ubuntu, \
             Xcode Command Line Tools on macOS, or MSVC build tools on Windows) \
             and re-run. This test cannot be skipped."
        );
    });

    // 2. Build the staticlib into a scratch target dir (no lock contention).
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let scratch_target_dir = std::env::temp_dir().join(format!(
        "fabric_capability_ffi_link_{}_{}_{}",
        std::process::id(),
        stamp,
        // Avoid name clash between two test files using the same scratch
        // directory pattern.
        "c_link_smoke",
    ));
    let _ = std::fs::remove_dir_all(&scratch_target_dir);
    std::fs::create_dir_all(&scratch_target_dir)
        .expect("create scratch target dir for staticlib build");

    let (staticlib, native_libs) = build_staticlib_and_native_libs(&scratch_target_dir);
    eprintln!(
        "c_link_smoke: rustc reported native-static-libs = {}",
        native_libs.join(" ")
    );

    // 3. Stage the C source. Use a per-test scratch dir so concurrent test
    // runs don't collide on the same .c/.o file.
    let scratch_c = std::env::temp_dir().join(format!(
        "fabric_capability_ffi_c_link_{}_{}",
        std::process::id(),
        stamp
    ));
    let _ = std::fs::remove_dir_all(&scratch_c);
    std::fs::create_dir_all(&scratch_c).expect("create c scratch dir");
    let c_file = scratch_c.join("smoke.c");
    let bin_file = scratch_c.join(if cfg!(target_os = "windows") {
        "smoke.exe"
    } else {
        "smoke"
    });
    std::fs::write(&c_file, C_PROGRAM).expect("write C source");

    // 4. Locate the header.
    let header = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("include/fabric_capability.h");

    // 5. Compile + link against the staticlib.
    compile_cmd
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg(format!("-I{}", header.parent().unwrap().display()))
        .arg("-o")
        .arg(&bin_file)
        .arg(&c_file)
        .arg(&staticlib);

    // Ask rustc (via `build_staticlib_and_native_libs`) which system libraries
    // this archive needs ON THIS HOST, and forward them to the C driver. The
    // list is whatever the running platform's rustc printed, so this is correct
    // on macOS, on ubuntu-latest CI, and anywhere else, with no per-platform
    // branch and no hardcoded library name in this file.
    for flag in &native_libs {
        compile_cmd.arg(flag);
    }

    let compile_status = compile_cmd
        .status()
        .expect("cc compile invocation failed to launch");
    assert!(
        compile_status.success(),
        "C compile/link failed (staticlib={}, header={}, native_static_libs={:?}). \
         The flags above are exactly what rustc reported for this host; if the \
         link still fails, the filter in filter_native_static_lib_flags dropped \
         something needed, or the C driver needs a matching sysroot.",
        staticlib.display(),
        header.display(),
        native_libs
    );

    // 6. Run the linked binary and assert it exits 0.
    let run = Command::new(&bin_file)
        .output()
        .expect("run linked C binary");
    let stdout = String::from_utf8_lossy(&run.stdout);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(
        run.status.success(),
        "linked C binary exited non-zero (status={:?})\nstdout:\n{}\nstderr:\n{}",
        run.status,
        stdout,
        stderr
    );

    // 7. Cleanup.
    let _ = std::fs::remove_dir_all(&scratch_c);
    // Keep the scratch_target_dir so a subsequent run can reuse the build
    // cache — cargo will reuse the cached artefacts, making repeated runs
    // ~3s instead of ~24s. The dir is rooted in `std::env::temp_dir()`
    // and is small (only the staticlib).

    eprintln!("c_link_smoke OK — stdout:\n{}", stdout);
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests for the flag discovery pipeline.
//
// These are cheap, pure, and — crucially — they pin the LINUX-shaped flag
// string even when the host running them is macOS. The `c_link_smoke` test
// itself can only ever exercise the flag list of the machine it runs on, so
// these tests are what let a macOS developer gain confidence in the code path
// that `ubuntu-latest` CI will take. They do not claim to prove the Linux link
// succeeds; only CI can do that. What they do prove is that nothing between
// rustc's output and the C driver's argv drops, reorders, or corrupts a Linux
// flag list.
// ─────────────────────────────────────────────────────────────────────────────

/// A representative `cargo rustc --print native-static-libs` payload as
/// linux-gnu rustc prints it, with the surrounding note noise included.
const LINUX_RUSTC_STDERR: &str = "\
   Compiling fabric-capability-ffi v0.1.0
note: link against the following native artifacts when linking against this static library. The order and any duplication can be significant on some platforms

note: native-static-libs: -lgcc_s -lutil -lrt -lpthread -lm -ldl -lc
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.64s
";

/// A representative macOS payload, used to pin the two-token `-framework`
/// handling that a Linux payload never exercises.
const MACOS_RUSTC_STDERR: &str = "\
note: link against the following native artifacts when linking against this static library. The order and any duplication can be significant on some platforms

note: native-static-libs: -framework CoreFoundation -liconv -lSystem -lc -lm
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.64s
";

#[test]
fn parses_linux_native_static_libs_into_one_argv_entry_per_flag() {
    let flags = filter_native_static_lib_flags(parse_native_static_libs("", LINUX_RUSTC_STDERR));
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
    let flags = filter_native_static_lib_flags(parse_native_static_libs("", MACOS_RUSTC_STDERR));
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
        filter_native_static_lib_flags(parse_native_static_libs("native-static-libs: -lm\n", "")),
        vec!["-lm"]
    );
}

#[test]
fn strips_driver_specific_flags_but_keeps_library_search_paths() {
    // `-C ...` and `-Wl,...` are not valid inputs for a C driver, but `-L`
    // paths are passed through rather than stripped, per the documented rule:
    // strip on a real failure, never speculatively.
    let stderr = "note: native-static-libs: -L/usr/lib -C link-arg=-deadbeef -Wl,-z,now -lm -lc\n";
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
    let flags = filter_native_static_lib_flags(parse_native_static_libs("", LINUX_RUSTC_STDERR));
    assert!(
        !flags
            .iter()
            .any(|f| f.contains("Finished") || f.contains("profile")),
        "cargo status text leaked into the link flags: {flags:?}"
    );
}

#[test]
#[should_panic(expected = "did not print a usable")]
fn fails_loudly_when_rustc_reports_no_flags() {
    // The test must never quietly link with no flags when discovery failed.
    // A silent pass here is exactly the failure mode that made the previous
    // attempt report success on stale artefacts.
    parse_native_static_libs("", "    Finished `dev` profile [unoptimized + debuginfo]\n");
}
