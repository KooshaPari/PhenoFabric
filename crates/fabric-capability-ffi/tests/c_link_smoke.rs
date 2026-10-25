//! True cross-language link smoke test for `fabric-capability-ffi`.
//!
//! Compiles a small C program against `include/fabric_capability.h`, links
//! it against the `fabric-capability-ffi` static library, runs the binary,
//! and asserts on the exit code. This catches ABI bugs at the level a real
//! C consumer would experience them — wrong symbol naming, wrong calling
//! convention, mismatched enum widths, missing functions, phantom symbols,
//! and so on.
//!
//! # Where the pieces live
//!
//! The supporting code is split out by concern under
//! `tests/c_link_support/`:
//!
//! * [`c_link_support::c_program`] — the C source compiled and linked here.
//! * [`c_link_support::toolchain`] — C compiler detection and the staticlib
//!   build that produces both the archive and its native library list.
//! * [`c_link_support::flags`] — parsing and filtering that library list.
//! * [`c_link_support::marker`] — recognising the `native-static-libs:`
//!   marker in a line of rustc output.
//! * [`c_link_support::ansi`] — stripping ANSI colour escapes from rustc's
//!   output, which is what makes the discovery work under the CI job's
//!   `CARGO_TERM_COLOR=always`.
//!
//! # The artefact problem
//!
//! `cargo test --lib` does NOT build a `staticlib` artefact even when the
//! crate declares `crate-type = ["rlib", "staticlib"]`. CI failure mode:
//! the test panics because no `libfabric_capability_ffi.a` exists in
//! `target/debug/`. To survive a cold checkout, the test must BUILD the
//! staticlib itself. See [`c_link_support::toolchain`].
//!
//! # The link-flag problem
//!
//! A Rust `staticlib` is NOT self-contained. It references the platform's
//! system libraries, and the exact set is platform-specific: macOS wants
//! `-framework CoreFoundation -liconv -lSystem`, Linux wants a different set
//! (typically `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc`).
//!
//! Hardcoding either list is a bug: the macOS list was shipped once, failed
//! on `ubuntu-latest` CI, and the test had to be reverted (commit 491b67e).
//! The portable answer is to ASK RUSTC, on the host that is running the test,
//! what the archive actually needs. See [`c_link_support::flags`].
//!
//! # The colour problem (why this failed on Linux CI)
//!
//! The parser must cope with COLOURED rustc output. This repository's CI sets
//! `CARGO_TERM_COLOR: always`, so on `ubuntu-latest` rustc prints the
//! library list as:
//!
//! ```text
//! \x1b[1m\x1b[92mnote\x1b[0m: native-static-libs: -lgcc_s -lutil ...
//! ```
//!
//! There is no literal `note:` substring there, so a parser that strips a
//! literal `"note:"` prefix finds nothing and hard-fails. That is precisely
//! the failure observed in CI run 39331325950, after the test had been
//! passing on macOS.
//!
//! Two independent measures now guard against that:
//!
//!   * The child cargo is forced to emit plain output
//!     (`CARGO_TERM_COLOR=never`, `TERM=dumb`) so the note is readable in a
//!     CI log — see [`c_link_support::toolchain`]. This is PR #54, verified
//!     green on `ubuntu-24.04` in CI run 39353001542.
//!   * The parser copes with colourised output regardless, by stripping ANSI
//!     SGR escapes ([`c_link_support::ansi`]) and recognising the listing by
//!     its own shape rather than by the decoration around it
//!     ([`c_link_support::marker`]).
//!
//! The second measure is what makes the first one unnecessary for
//! correctness: the override is an ergonomic measure, not the correctness
//! boundary. The parser is the correctness boundary, and it is pinned by
//! tests that feed it the byte-exact colourised payload captured from the
//! failing CI job log.
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

use std::path::PathBuf;
use std::process::Command;

mod c_link_support;

use c_link_support::c_program::C_PROGRAM;
use c_link_support::toolchain::{build_staticlib_and_native_libs, pick_c_compiler};

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

    // A monotonic timestamp, used to give the throwaway C staging dir a
    // unique name. The cargo target dir below deliberately does NOT use it.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    // 2. Build the staticlib into a scratch target dir (no lock contention).
    //
    // The dir name MUST be stable across runs, or the "reuse the build cache"
    // optimisation below is dead code: cargo keys its cache on the target
    // dir path, so a per-run name forces a full cold rebuild (~24s, ~830MB
    // of target tree) every single time and leaves the old tree behind.
    // That is not theoretical — it leaked 32GB of duplicate cargo target
    // trees into the scratch dir before this was fixed.
    //
    // Stability is safe here because the target dir is only ever used to
    // build this one crate at one profile with one toolchain, and cargo
    // itself handles invalidation (it fingerprints its inputs and rebuilds
    // when they change). The PID is deliberately NOT in the name: it was
    // there to avoid concurrent-run collisions, but `cargo` takes a build
    // lock on the target dir, so two concurrent runs serialise on the lock
    // instead of corrupting each other. A stable name plus cargo's own
    // locking is what makes the cache actually work.
    let scratch_target_dir = std::env::temp_dir().join("fabric_capability_ffi_c_link_smoke_target");
    std::fs::create_dir_all(&scratch_target_dir)
        .expect("create scratch target dir for staticlib build");

    let (staticlib, native_libs) = build_staticlib_and_native_libs(&scratch_target_dir);
    eprintln!(
        "c_link_smoke: rustc reported native-static-libs = {}",
        native_libs.join(" ")
    );

    // 3. Stage the C source. Use a per-test scratch dir so concurrent test
    // runs don't collide on the same .c/.o file. This one IS deleted at the
    // end of the run, so a per-run name costs nothing.
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
    // The scratch_target_dir is deliberately KEPT, so a subsequent run can
    // reuse the build cache — cargo will reuse the cached artefacts, making
    // repeated runs ~3s instead of ~24s.
    //
    // This only works because the dir name is stable across runs (see step
    // 2). The dir is a full cargo target tree (~830MB), not "only the
    // staticlib" as an earlier comment here claimed — but it is ONE tree
    // that is reused, rather than a new ~830MB tree leaking on every run.

    eprintln!("c_link_smoke OK — stdout:\n{}", stdout);
}
