# Changelog

All notable changes to Phenotype Fabric will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.5] - 2026-10-21

### Fixed

- **Release**: a failed Docker build can no longer produce a GitHub Release.
  `github-release` depended only on `build`, `build-gui` and `identity`, so
  `docker-build` was outside the release's dependency graph. The `v0.1.4`
  release is the proof: `Build Docker image` failed, `Create GitHub Release`
  succeeded with all ten assets, no container image was ever published (the
  `ghcr.io` package API returns 404 for it), and the release body still told
  users the image existed. `github-release` now needs `docker-build`.
- **Release**: the release body no longer promises a Docker image that was
  never published. The `docker` push job now runs before the release is
  created and exports a `pushed` output, and the body's Docker sentence is
  generated from it. When the image reached the registry the body names the
  `build-<identity>` tag; when it did not, the body says the image was built
  and verified but not published to the container registry.
- **Release**: the `verify` job's Docker pull is now fail-closed instead of
  best-effort. It depends on `docker` so it can read `pushed`; when the push
  claimed success the image must be pullable or the release fails, and when
  the push did not happen the pull is skipped with an explicit warning. This
  separates an expected registry-credential issue from a genuinely broken
  published image.
- **Docker image**: the image now builds from the real workspace sources
  instead of a hand-maintained Cargo target-stub layer that had drifted away
  from the actual crate graph. The stub generator, its scratch directories
  and the stub layer itself are gone; the builder compiles
  `cargo build --release --locked --bin fabric --bin fabric-daemon` against
  the real `crates/`, `examples/` and `docs/` tree.
- **Docker image**: `docs/` is now copied into the builder. Without it,
  `fabric-frame-transport`'s `include_str!` of
  `docs/spec/wire-protocol-schema.json` failed the build.
- **Docker image**: the runtime base is now `debian:trixie-slim`, matching
  the `rust:1.99-slim` builder's Debian release and glibc 2.41. It was
  `bookworm` (glibc 2.36), so a binary linked against the newer glibc could
  not run on the older runtime.
- **Docker image**: the runtime installs trixie's `libssl3t64`. The daemon's
  `reqwest` / `native-tls` path needs OpenSSL, and the package name differs
  between the two Debian releases.
- **CI**: three assertions in `ci.yml` reported correct images and correct
  release archives as missing their binaries. `grep -q` exits on the first
  match and closes the pipe, so the upstream `printf` died of `SIGPIPE`; with
  `set -o pipefail` enabled in those steps the pipeline reported failure and
  the binaries were wrongly recorded as missing. The assertions now use a
  here-string, which has no pipeline to break.

### Added

- **CI**: the `docker-build` job in `ci.yml` now runs on every push. It
  builds the image, checks that both binaries are present, and **executes**
  both binaries inside the image with `--version`. A binary that is present
  but cannot load its shared libraries is just as broken as a missing one,
  so the execution check is what makes this job meaningful.
- **CI**: the `docker-build` job in `release.yml` is promoted from
  best-effort (`continue-on-error: true`) to a real gate. The image is
  verified before the image tar is uploaded, so the bytes that reach the
  registry are proven to work. The downstream registry push stays
  best-effort, because a missing registry token must not fail a release
  whose binaries and assets are already correct.

## [0.1.4] - 2026-10-21

### Fixed

- **Release publication**: the GitHub release actually carries its assets.
  `download-artifact`'s `pattern` input is a single minimatch pattern, not a
  list, so the previous multi-line `cli-*` / `gui-*` block scalar matched
  nothing and the release published with zero assets while the job exited 0.
  It now uses `{cli-*,gui-*}`, which matches the eight CLI and GUI artifacts
  and still excludes the docker artifact.
- **Post-publish verification**: the `verify` job's tag-extraction step wrote
  `version=` while all four of its consumers read
  `steps.version.outputs.tag`, so `TAG` was empty and the verification was a
  no-op that could never have caught a partial archive. The step now writes
  `tag=`.
- **Partial-release guard**: a new step fails the release job *before* the
  release is created if any of the ten expected assets is missing, so a
  partial publish is prevented rather than merely undetected.

### Added

- `program/scripts/check_workflows.py`: a static lint for GitHub Actions
  workflows. It catches step-output name mismatches, multi-line
  `download-artifact` patterns, and patterns that cover one release artifact
  prefix but not the other. Wired into the Spec Validation workflow.
- `program/scripts/requirements.txt`: the Python dependencies for the
  validation scripts.

### Changed

- The Spec Validation workflow now installs PyYAML. Without it,
  `check_openapi.py` hit its ImportError guard and printed
  "WARNING: PyYAML not installed; skipping OpenAPI check" -- the OpenAPI
  validation had been silently inert. It now runs for real.
- `regenerate_manifest.py` warns about untracked files, since the manifest
  file list comes from `git ls-files` and silently drops anything not yet
  staged.
- The Docker build's dependency-stub layer now creates
  `crates/fabric-cli/src/lib.rs`. `fabric-cli` declares both a `[[bin]]` and a
  `[lib]`, and the stub layer only created the binary, so cargo could not
  resolve the workspace and the Docker build failed with "can't find lib
  `fabric_cli`".
- The `docker` push job is now `continue-on-error: true`, matching
  `docker-build`. Previously a Docker failure turned an otherwise complete
  release run red even though the release and its verification had passed.

## [0.1.3] - 2026-10-20

### Fixed

- **G-05 (Linux-only CI failure)**: the `c_link_smoke` integration test
  failed on Ubuntu because CI sets `CARGO_TERM_COLOR=always`, which inserts
  ANSI escapes into rustc's `native-static-libs:` diagnostic and broke the
  marker match. The test's diagnostic parser now strips ANSI SGR sequences
  and recognises the rustc diagnostic shape strictly, and the child cargo
  invocation forces `CARGO_TERM_COLOR=never` and `TERM=dumb`. Verified green
  on real `ubuntu-24.04` CI.

### Changed

- Split the 748-line `c_link_smoke.rs` into focused modules under
  `tests/c_link_support/` (ansi, marker, flags, toolchain, c_program), all
  below the 500-line limit.

### Known issues

- This release was published with **zero assets**. The asset-upload step
  used a multi-line `download-artifact` pattern that matched nothing, and
  the verification job read an output name the extraction step never wrote.
  The binaries themselves were built correctly -- the run artifacts contain
  all five required binaries in every CLI archive -- but they were never
  uploaded. Use 0.1.4 instead.

## [0.1.2] - 2026-10-16

### Fixed

- Replaced the retired `macos-13` GitHub-hosted runner with `macos-15-intel`.

### Known issues

- The published Linux archive is **incomplete**: it carries only three of the
  five binaries `install.sh` requires, missing `fabric` and
  `fabric-graph-cli`. Use 0.1.4 instead.

## [0.1.1] - 2026-10-10

### Added

- SQLite nested authority-domain and immutable outbox slice in
  `fabric-persist`, with tests for the ledger transaction path.

### Fixed

- Closed three of four pre-existing `spec-validation.yml` infrastructure gaps.

## [0.1.0] - 2026-09-13

### Added

- **Core crates**: fabric-graph (topology compiler), fabric-capability (descriptor model + probe),
  fabric-checker (constraint checker), fabric-persist (SQLite storage), fabric-workspace (seat lease FSM)
- **Transport**: frame-transport (encode/decode, multihop routing), daemon wire server (TCP JSON protocol)
- **GPU surface**: fabric-surface-mojo (MLIR/FFI bridge for GPU compute)
- **GUI surfaces**:
  - fabric-tray (macOS system tray with tao)
  - fabric-tui (ratatui terminal dashboard, 4 tabs)
  - fabric-gui (egui native desktop, 4 panels, dark/light mode)
  - fabric-web (Leptos + WebRTC data channels)
  - macOS .app bundle with icon, Info.plist, ad-hoc codesign
- **CLI**: fabric-cli (11 commands: topology, route, workspace, surface, check, probe, status, etc.)
  - graph-cli (dedicated graph compiler CLI)
- **Orchestration**: fabric-orchestrator (end-to-end pipeline with CLI)
- **Integration**: fabric-integration-tests (10 cross-crate tests), nvms-adapter
- **Examples**: full-demo with 6-node topology, Rust binary, shell script
- **Daemon**: fabric-daemon with wire server, config, hot-reload
- **DevOps**: Dockerfile, docker-compose, systemd unit, GitHub Actions CI/CD
- **Testing**: 464 tests, fuzz targets, proptest properties, criterion benchmarks
- **Documentation**: 35+ ADRs, mdBook site, architecture docs, Go adapter guide
- **Mojo GPU**: fabric-surface-mojo crate with MLIR bridge
