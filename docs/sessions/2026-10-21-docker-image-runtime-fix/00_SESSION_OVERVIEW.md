# Session: Docker image runtime fix

**Date:** 2026-10-21
**Scope:** the `v0.1.4` release workflow was red because the Docker build's
hand-maintained Cargo dependency-stub layer had drifted out of sync with the
workspace. This session replaces that design and proves the resulting image
on real Ubuntu CI.

## Goal

Make the Docker image build from a design that cannot drift, and prove on
real Ubuntu x86_64 CI that the image both builds and actually runs. Then
decide whether a `v0.1.5` tag is required.

## What was wrong

The Dockerfile used a two-stage dependency-cache scheme:

1. `COPY` only the workspace manifests, hand-write a stub source file for
   every declared cargo target, `cargo build --release` to warm the cache.
2. `COPY` the real sources and rebuild.

The stub list drifted. The `v0.1.4` release's Docker build failed with 7
distinct missing-target errors, for example:

```
error: can't find lib `fabric_cli` at path `/build/crates/fabric-cli/src/lib.rs`
error: can't find bin `fabric-tui` at path `/build/crates/fabric-tui/src/main.rs`
error: can't find bin `tf-web` at path `/build/crates/fabric-terminal/src/web_main.rs`
```

### Why the stub layer could not simply be repaired

Cargo refuses to resolve the workspace **at all** when any declared target's
source file is absent. `cargo metadata` and `cargo read-manifest` both fail.
So the stub list had to be exhaustive, and a crate may declare several
`[[bin]]`s, a `[lib]`, and `[[bench]]`es. Every new target broke the build
again.

### Why the stub layer was not worth repairing

Every crate the image needs is a path dependency of another workspace crate.
The cache was therefore warmed with **stub** crates rather than the real
ones, and the second build recompiled everything regardless. The layer
provided no useful caching and created a permanent maintenance burden.

## The replacement

The builder stage copies the real sources and builds only the two binaries
the runtime stage consumes:

```dockerfile
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY examples/ examples/
COPY docs/ docs/

RUN cargo build --release --locked --bin fabric --bin fabric-daemon
```

Cargo builds only the requested targets and their dependency closure, so
the terminal/TUI/GUI crates are never compiled. A green build still proves
the whole workspace resolves, because cargo validates all declared targets.

## Defects found by running the build (not by inspection)

All three were found by actually building on native CI, or by executing the
resulting image.

### 1. `docs/` was not copied into the builder

Two crates in the image's dependency closure embed documentation into the
binary:

| Crate | File | Embedded path |
|---|---|---|
| `fabric-frame-transport` | `src/schema.rs:7` | `include_str!("../../../docs/spec/wire-protocol-schema.json")` |
| `fabric-daemon` | `src/secret_loader/review_response.rs:406` | `include_str!("../../../../docs/runbooks/workos-infisical-shared.md")` |

The build failed with:

```
error: couldn't read `crates/fabric-frame-transport/src/../../../docs/spec/wire-protocol-schema.json`:
  No such file or directory (os error 2)
```

**Fix:** copy the whole `docs/` tree. Copying the tree rather than the two
individual files is deliberate: it is the only way to stop this class of
failure recurring when a new `include_str!` is added.

### 2. The runtime base did not match the builder's Debian release

| Stage | Image | Debian | glibc |
|---|---|---|---|
| builder | `rust:1.99-slim` | 13 "trixie" | 2.41 |
| runtime (before) | `debian:bookworm-slim` | 12 "bookworm" | 2.36 |

A binary linked against glibc 2.41 cannot start on glibc 2.36. The image
could have built successfully and still been unrunnable, which the previous
CI job could not have detected because it only checked that the binary
*files* existed.

**Fix:** the runtime is now `debian:trixie-slim`, matching the builder.

### 3. The OpenSSL package name differs between the two releases

`reqwest` (a `fabric-daemon` dependency) uses `native-tls`, which links
OpenSSL. The binaries therefore need `libssl.so.3` at runtime.

| Release | Package | Provides |
|---|---|---|
| trixie | `libssl3t64` | `/usr/lib/x86_64-linux-gnu/libssl.so.3` |
| bookworm | `libssl3` | `/usr/lib/x86_64-linux-gnu/libssl.so.3` |

**Fix:** the runtime installs `libssl3t64` (the package that exists on
trixie). Before this fix the runtime stage installed **no** OpenSSL at all,
so the binaries would have failed with
`error while loading shared libraries: libssl.so.3`.

## Evidence discipline

Two distinct kinds of "it works" are recorded here, and they are not
interchangeable.

| Claim | How it was established | Status |
|---|---|---|
| The pruned context builds both binaries | `cargo build --release --locked --bin fabric --bin fabric-daemon` in a directory containing only `Cargo.toml`, `Cargo.lock`, `crates/`, `examples/`, `docs/` | Verified (native macOS arm64) |
| Both binaries execute | `fabric --version` and `fabric-daemon --version` on the built binaries | Verified (native macOS arm64) |
| The `include_str!` targets survive `.dockerignore` | A real `docker build` context probe listing the context contents | Verified |
| The image builds on Linux x86_64 | CI job "Docker image builds" on `ubuntu-latest` | See "CI evidence" below |
| The image's binaries run on Linux x86_64 | CI job step "Verify the binaries load and run inside the image" | See "CI evidence" below |

Local `docker build` under Colima QEMU emulation reached a linker SIGSEGV
that appears to be an emulation artifact, not a project defect, and the
emulated build was too slow to be a useful gate. Local emulation was
therefore **not** treated as evidence. The native CI run is the authority.

## `.dockerignore` note

The `.dockerignore` contains a `*.md` rule, which looks like it would strip
the embedded runbook. It does not: Docker's matcher does not treat `*` as
crossing directory separators, so `*.md` matches only root-level files. The
runbook was never at risk. The intent is now recorded in the file with an
explicit `!docs/` exception so a future simplification of the pattern list
does not break the build.

`docs/book/` is generated output (git-ignored, ~6 MB) and is excluded; it is
not present in a clean CI checkout either.

## CI changes

`.github/workflows/ci.yml`, job `docker-build` (runs on every push):

1. The image builds. This proves the workspace resolves.
2. Both binaries are present at `/usr/local/bin/`.
3. **Both binaries are executed inside the image** (`--version`), and the
   output is checked for `error while loading shared libraries`. A binary
   that is present but cannot load is just as broken as a missing binary.

`.github/workflows/release.yml`, job `docker-build`:

- Promoted from `continue-on-error: true` to a real gate. The brittleness
  that justified the exemption is gone, and the `ci.yml` job now proves the
  image works on every push, so a tag can only be cut on a commit whose
  Docker image is already proven.
- The same execution assertion was added before the image tar is uploaded,
  so the bytes that reach the registry are proven to work.
- The downstream `docker` push job stays best-effort: a missing registry
  token must not fail a release whose binaries and assets are already
  correct.

## CI evidence

| Run | Commit | Result |
|---|---|---|
| 39545339580 | `178a355` | FAIL — `couldn't read .../docs/spec/wire-protocol-schema.json` |
| 39547036357 | `1240110` | Image built and both binaries compiled. Job failed in the *assertion* step with `printf: write error: Broken pipe` — see below. |
| 39547651066 | `288ffd8` | CANCELLED — not a failure. A later push cancelled the in-flight run; the Docker build was 65% through cargo when it was cancelled. |
| **39547767036** | **`aed03ba`** | **PASS — all nine jobs green on native Ubuntu x86_64.** |

### The assertion bug in run 39547036357

The image built. Both binaries compiled. The job then failed in
"Verify the image contains the expected binaries" with:

```
printf: write error: Broken pipe
##[error]Process completed with exit code 1.
```

The image was correct; the assertion was wrong.

`grep -q` exits as soon as it finds a match, which closes the pipe early.
`printf` then dies of `SIGPIPE`, and because the step enables
`set -o pipefail`, the pipeline reports failure. The result is a **false**
"binary is missing" report against a perfectly good image.

Reproduced locally against a 50,000-entry listing:

```
$ bash -e piptest.sh
missing=' fabric fabric-daemon'
```

The listing of a real image or a real release archive is large enough to
trigger this reliably, so the assertion fails on correct input.

Three sites were affected, all of which enable `pipefail`:

| Site | Effect before the fix |
|---|---|
| `ci.yml` — verify the image contains the expected binaries | The assertion that failed this run |
| `ci.yml` — check the published release for a partial archive | Reports every binary in a correct archive as missing |
| The two new binary-execution assertions in `ci.yml` and `release.yml` | Would have had the same defect |

Each was changed from `printf ... | grep -q` to a here-string
(`grep -q ... <<<"$listing"`), which has no pipeline and therefore no
`SIGPIPE`. Re-verified against the same 50,000-entry listing: the
assertion now reports the binaries as present.

The two `printf ... | grep -q` sites in `release.yml`'s "Verify CLI
archives contain the expected binaries" step were deliberately left alone:
that step does not enable `pipefail`, so the pattern is safe there. The
distinction was checked per step, not per file.

### The green run: 39547767036 (commit `aed03ba`)

All nine jobs passed on native Ubuntu x86_64:

```
Clippy: completed/success
Format: completed/success
Check: completed/success
Docker image builds: completed/success
Integration tests: completed/success
Unit tests: completed/success
Release build: completed/success
Release archive contents: completed/success
E2E smoke tests: completed/success
```

The Docker job's own log, read back from the run, not inferred from the
job's green checkmark:

```
OK Docker image contains /usr/local/bin/fabric
OK Docker image contains /usr/local/bin/fabric-daemon
OK fabric --version runs inside the image: fabric 0.1.0
OK fabric-daemon --version runs inside the image: fabric-daemon 0.1.0
```

That is the four facts this work existed to establish: the image builds
from the real workspace, both binaries are present, and both binaries
**execute** inside the runtime stage, which is the only way to catch a
missing shared library.

The build log also confirms the two Debian fixes landed as intended:

```
#10 [runtime 2/6] RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates curl libssl3t64 && rm -rf /var/lib/apt/lists/*
#10 libssl3t64 is already the newest version (3.5.7-1~deb13u3).
```

Run 39547651066 (`288ffd8`) was cancelled, not failed: a subsequent push
cancelled the in-flight run while the Docker build was 65% through `cargo
build`. Its `X` marker on the build step is a cancellation artifact, not a
build error, and it is recorded here so the cancelled run is not later
mistaken for a red build.

## Repository gates

| Gate | Command | Result |
|---|---|---|
| Workflow lint | `program/scripts/check_workflows.py` | Pass (5 files) |
| Link check | `program/scripts/check_links.py` | Pass |
| Manifest | `program/scripts/regenerate_manifest.py` then `check_manifest.py` | Pass (1127 entries) |
| OpenAPI | `program/scripts/check_openapi.py` | Pass |
| JSON schemas | `program/scripts/check_json_schemas.py` | Pass (6 files) |

The manifest hashes `git ls-files`, so new files must be staged before
regenerating or CI reports a mismatch.
