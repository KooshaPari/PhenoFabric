# 2026-10-20 — G-05: fix the Linux-only c_link_smoke failure, publish v0.1.3

## Session goal

Fix G-05, the portable C-link integration test that failed on `ubuntu-latest`
while passing on macOS, and publish a corrected tagged release.

The published `v0.1.2` remains genuinely broken and is not repaired by this
session: its CLI archives ship only 3 of the 5 binaries `install.sh`
requires. The correction ships as `v0.1.3`.

## Root cause of the Linux failure

CI run `39331325950` failed on `ubuntu-latest` in the `c_link_smoke`
integration test. The test compiles a C program and links it against the
native libraries rustc reports in its `native-static-libs` diagnostic.

The diagnostic is colourised because the CI job sets
`CARGO_TERM_COLOR: always`:

```
\x1b[1m\x1b[92mnote\x1b[0m: native-static-libs: -lgcc_s -lutil ...
```

There is no literal `note:` substring in that line. The word `note` is
wrapped in SGR escapes and the following colon is wrapped in its own
sequence. The old parser looked for a literal `"note:"` prefix, found
nothing, and hard-failed by its own design. On macOS the same test passed
because the local shell did not set `CARGO_TERM_COLOR=always`.

So the bug was never Linux-specific in the code. It was triggered on Linux
because CI forces colour and the parser only understood plain text.

## The fix

Two layers, deliberately redundant, so that neither is load-bearing alone.

### Layer 1 — the parser copes with colourised output

1. Strip ANSI SGR escapes before interpreting the text, so the payload is
   compared as plain text. (`tests/c_link_support/ansi.rs`)
2. Recognise the listing by its own shape, anchored to a rustc diagnostic
   prefix, so it is still found if the decoration differs — and, just as
   importantly, is NOT found when a line merely mentions the marker in
   prose. (`tests/c_link_support/marker.rs`)

### Layer 2 — the child cargo emits plain output

`CARGO_TERM_COLOR=never` and `TERM=dumb` are forced on the child cargo
invocation, so the note is readable in a CI log. This is the approach taken
by PR #54 (`731e2b8`), which was verified green on `ubuntu-24.04` in CI run
`39353001542`.

### Why both

The override makes the note readable; the parser makes the override
unnecessary for correctness. The evidence that the parser alone suffices:
with the override removed from the child invocation and
`CARGO_TERM_COLOR=always` set in the environment, `c_link_smoke` still
passes 20/20 locally.

The override is therefore an ergonomic measure. The correctness boundary is
the parser, and it is pinned by a colour-parity test that feeds the parser
the byte-exact colourised payload captured from the failing CI job log and
requires it to yield the same flags as the plain payload.

## Mutation testing: what is actually load-bearing

Neither defence was claimed to be load-bearing without evidence.

| Mutation | Result | Reading |
|---|---|---|
| ANSI stripping → no-op | colour-parity test FAILS; flag list comes back as `-lc\x1b[0m` | ANSI handling IS load-bearing |
| Marker rule → literal `note:` prefix | test SURVIVES | the marker rule is defence-in-depth, NOT the fix |
| Marker rule → earlier whitespace-delimited form | prose false-positive test FAILS | the hardened marker rule IS load-bearing |

The third row is why the marker rule is worth its strictness even though
the second row shows the old rule would have worked once ANSI stripping was
in place.

## Why the marker rule is biased towards rejecting

The two failure directions are not symmetric:

- A **missed** listing makes the parser find no flags and the test
  hard-fails loudly. Safe.
- A **wrongly recognised** listing injects junk into the C link command
  line and may link against the wrong libraries. Silent, dangerous.

So the rule admits only the shapes rustc actually emits — bare marker,
diagnostic prefix, or prefix plus a source location — and rejects
`notnative-static-libs:` and prose mentions.

## Refactor

The former 748-line `c_link_smoke.rs` was split by concern into
`tests/c_link_support/`:

| File | Lines | Concern |
|---|---|---|
| `c_link_smoke.rs` | 197 | test entrypoint and doc |
| `ansi.rs` | 159 | ANSI SGR stripping |
| `marker.rs` | 230 | strict `native-static-libs:` recognition |
| `flags.rs` | 394 | native-library flag parsing and filtering |
| `toolchain.rs` | 198 | C compiler and staticlib discovery |
| `c_program.rs` | 143 | extracted C source |
| `mod.rs` | 26 | module root |

Every file is under the 500-line limit. The C program body is
byte-identical to the previous version (verified: 6198 bytes, exact
match), so the refactor is a pure move plus the parsing fix.

## Verification

### Local (rerun independently by the orchestrator)

- `cargo test -p fabric-capability-ffi --test c_link_smoke` under
  `CARGO_TERM_COLOR=always`: 20 passed, 0 failed
- same, with the child colour override removed: 20 passed, 0 failed
- `cargo test --workspace`: 0 failures
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo fmt --all -- --check`: clean
- `cargo doc -p fabric-capability-ffi --no-deps --document-private-items`:
  clean
- `check_links.py`, `regenerate_manifest.py`, `check_manifest.py`: pass
  (1124 tracked entries, all digests verified)

### Linux (real CI, not inferred)

| Run | Workflow | Result |
|---|---|---|
| `39355149493` | CI | success — 8/8 jobs, `ubuntu-24.04` image |
| `39355149475` | E2E | success — 2/2 jobs |
| `39355149563` | Spec Validation | success — 3/3 jobs |

All three on commit `2c441d9`. The `Integration tests` job log shows
`Running tests/c_link_smoke.rs` followed by `test c_link_smoke ... ok` and
`test result: ok. 20 passed; 0 failed`. The same log visibly contains ANSI
SGR escapes around the cargo `Running` line, which confirms the fix held
under the exact colourised condition that broke v0.1.2.

The `Release archive contents` job in the same run confirmed all five
required release binaries exist and that the tarball it builds contains
all five.

## The v0.1.2 defect (context for the release)

`v0.1.2` published archives containing only 3 of the 5 required binaries.
Verified directly against the published asset:

```
phenotype-fabric-v0.1.2-linux-x86_64.tar.gz
  fabric-tray      PRESENT
  fabric-daemon    PRESENT
  fabric-tui       PRESENT
  fabric           MISSING
  fabric-graph-cli MISSING
```

Cause: the packaging loop asked for a bin name that is not a real cargo
`[[bin]]` target, and `2>/dev/null || true` swallowed the copy error, so
the archive shipped without the binary and CI stayed green.

Fixed in `309345b` (already on `main` before this session): the packaging
loop now hard-fails on a missing binary, `install.sh` preflights every
required binary before copying anything, and a post-publish job verifies
the CONTENTS of every published archive.

## Release: v0.1.3

Tagged `v0.1.3` at `2c441d9` and pushed the tag. Release workflow run
`39524991323`.

Version choice: `v0.1.3` rather than re-tagging `v0.1.2`. A published
GitHub release's assets cannot be replaced in place by re-tagging, and
`v0.1.2` is what the documented install command resolves to today, so a
new version is the only way to actually correct what users download. The
`install.sh` and `program/scripts/test_install.sh` version resolution is
dynamic (newest published release), so no version pin needed updating.

## Notes for the next session

- The `docker-build` job is brittle: it failed on the v0.1.2 run because
  the Docker build context did not carry the `benches/` directories that
  several `Cargo.toml` files reference via `[[bench]]`. Those files exist
  now, so the failure should not recur, but the job is still marked
  best-effort and `continue-on-error`.
- `publish-crates` is a no-op until `CRATES_IO_TOKEN` is wired on the repo.
  Nothing is published to crates.io today.
- A1 still needs one human browser run. A2/A3 remain code-complete.

## Release: v0.1.3 was published EMPTY (2026-10-21)

The v0.1.3 release workflow run `39524991323` reported **failure**, but
the failure was in the post-publish `verify` job, and the *release itself*
had been created with **zero assets**.

Two independent defects made that invisible:

1. `download-artifact`'s `pattern` input is a SINGLE minimatch pattern
   (`new Minimatch(inputs.pattern)` in actions/download-artifact v8
   `src/download-artifact.ts`). The job passed a YAML block scalar:

       pattern: |
         cli-*
         gui-*

   which is the literal two-line string `"cli-*\ngui-*"`. The run log shows
   `Found 9 artifact(s)` then `Total of 0 artifact(s) downloaded`, and
   `find artifacts` failed with `find: 'artifacts': No such file or
   directory`. `softprops/action-gh-release` with `files: artifacts/*` does
   NOT fail on an empty glob, so the release published with no binaries and
   the job exited 0.

2. The `verify` job's `Extract version from tag` step wrote `version=...`
   while all four of its consumers read `steps.version.outputs.tag`. `TAG`
   therefore expanded to the empty string, so `gh release download ""`
   reported "no assets to download". The post-publish verification was a
   no-op that could never have caught the empty archive.

### The G-05 build fix itself was correct

Independently downloading the run artifacts of `39524991323` and listing
their contents shows every CLI archive carried all five required binaries:

```
cli-linux-x86_64/phenotype-fabric-v0.1.3-linux-x86_64.tar.gz
  fabric  fabric-daemon  fabric-tui  fabric-tray  fabric-graph-cli
cli-macos-aarch64/...   -> all five
cli-macos-x86_64/...    -> all five
cli-windows-x86_64/...  -> all five (.exe)
```

Only the *upload* step dropped them. This is why the fix could not be
verified by inspecting the published release: there was nothing published
to inspect.

## Release: v0.1.4

Fixes committed in `d01eeba`, `8a0d89c`, `ca9205b`, `7ab123d`, `23e85fd`:

- `pattern: '{cli-*,gui-*}'` -- one minimatch pattern using brace
  expansion. Verified against the real v0.1.3 artifact names with
  minimatch directly: it matches exactly the 8 `cli-*`/`gui-*` artifacts
  and excludes `KooshaPari~PhenoFabric~<id>.dockerbuild`. (`cli-*` alone
  would match only 4 and silently drop every GUI installer; `cli-*\ngui-*`
  matches 0.)
- The `verify` job's step now writes `tag=`, matching its consumers. The
  `docker` job already wrote `version=` and read `.version` -- correct as
  written, left alone.
- A new `Assert required release assets are present` step fails the release
  job BEFORE the release is created if any of the ten expected assets is
  missing, so a partial publish is prevented rather than merely undetected.
- `program/scripts/check_workflows.py` -- a static lint for both defect
  classes plus the partial-prefix pitfall, wired into Spec Validation.
  Verified it flags the pre-fix `release.yml` (5 findings), flags a
  `cli-*`-only pattern, and passes the fixed file.
- The Spec Validation workflow now installs PyYAML. Without it
  `check_openapi.py` hit its ImportError guard and printed "WARNING:
  PyYAML not installed; skipping OpenAPI check" -- the OpenAPI validation
  had been silently INERT. It now runs for real.
- `regenerate_manifest.py` warns about untracked files, since the manifest
  file list comes from `git ls-files` and silently drops anything not yet
  staged (this bit us once: `requirements.txt` was regenerated before
  being staged, and CI failed with "1 added, 0 removed").

CI on `23e85fd`: CI `39530950640`, E2E `39530950657`, Spec Validation
`39530950692` -- all green, with the OpenAPI check confirmed live
("OK architecture/openapi.yaml (openapi version: 3.1.0)").

### Empirical notes worth keeping

- `gh release download` ORs multiple `--pattern` flags together (verified
  against v0.1.2: a matching `.tar.gz` pattern plus a non-matching `.zip`
  pattern still downloaded the `.tar.gz` files and exited 0). The workflow
  comment previously claimed AND semantics; corrected.
- `download-artifact` `pattern` is NOT newline-separated. The
  `@actions/glob` package accepts newline-separated patterns via
  `glob.create(patterns.join('\n'))`, but `download-artifact` does not use
  that -- it uses minimatch directly. The two look alike and are not.

## Notes for the next session

- The `docker-build` job is brittle: it failed on the v0.1.2 run because
  the Docker build context did not carry the `benches/` directories that
  several `Cargo.toml` files reference via `[[bench]]`. Those files exist
  now, so the failure should not recur, but the job is still marked
  best-effort and `continue-on-error`.
- `publish-crates` is a no-op until `CRATES_IO_TOKEN` is wired on the repo.
  Nothing is published to crates.io today.
- A1 still needs one human browser run. A2/A3 remain code-complete.

## v0.1.4 published and independently verified (2026-10-21)

Tagged `v0.1.4` at `ea024b5`. Release workflow run `39532120442`.

### What the run proved

| Job | Result |
|---|---|
| Build CLI linux-x86_64 / macos-aarch64 / macos-x86_64 / windows-x86_64 | success |
| Build GUI linux-x64 / macos-arm64 / macos-x64 / windows-x64 | success |
| Create GitHub Release | success |
| Post-publish verification | success |
| Build Docker image | failure (pre-existing, see below) |
| Push Docker image | failure (consequence of the above) |

**All 10 assets are published.** The run's overall `failure` conclusion came
ONLY from the two Docker jobs.

The post-publish verification did real work this time: its log shows all four
CLI archive listings with the five expected binaries, and the final line
`All release archives contain the expected binaries: fabric fabric-daemon
fabric-tui fabric-tray fabric-graph-cli`. The docker-pull step inside that job
is `continue-on-error: true` by design (the release job only depends on
`github-release` + `identity`), so its `denied` error from ghcr.io does not
affect the job's success.

### Independent verification (not relying on the workflow's own report)

Downloaded all 10 published assets with `gh release download` and inspected
them directly:

- All four CLI archives contain **all five** required binaries.
  - `phenotype-fabric-v0.1.4-linux-x86_64.tar.gz`: fabric, fabric-daemon,
    fabric-tui, fabric-tray, fabric-graph-cli
  - `phenotype-fabric-v0.1.4-macos-aarch64.tar.gz`: same five
  - `phenotype-fabric-v0.1.4-macos-x86_64.tar.gz`: same five
  - `phenotype-fabric-v0.1.4-windows-x86_64.zip`: the same five with `.exe`
- The five Linux binaries are genuine `ELF 64-bit LSB pie executable,
  x86-64` files, not empty placeholders, and are mode 0755.
- The five macOS binaries are `Mach-O 64-bit executable arm64`.
- The published macOS `fabric` binary actually RUNS: `fabric --version` ->
  `fabric 0.1.0`, and `fabric --help` prints its subcommand surface.
- GUI assets are real installers: AppImage is a static-pie ELF, `.deb` is
  `Debian binary package (format 2.0)` with `Version: 0.1.4` in its control
  file, `.exe` is a `PE32 executable (GUI) ... Nullsoft Installer`, `.msi` is
  a `Composite Document File V2 Document`, `.dmg` files are zlib-compressed
  disk images.
- `program/scripts/test_install.sh` against the **published** release:
  **10 passed, 0 failed** (T1-T9 plus the archive-name check), with
  `install.sh` downloading from GitHub, extracting, and installing all five
  binaries into a temp dir.

### Cross-check: v0.1.2 really was broken

Downloaded `phenotype-fabric-v0.1.2-linux-x86_64.tar.gz` and listed it:
`fabric-daemon`, `fabric-tray`, `fabric-tui`, `README.md` -- only 3 of the 5
required binaries. `fabric` and `fabric-graph-cli` are absent. Confirmed
independently, not taken on faith from the earlier session's notes.

### Two more defects found and fixed

**1. The Dockerfile's stub layer was missing `fabric-cli`'s lib target.**

The v0.1.4 Docker build failed with:

    4.499 error: can't find lib `fabric_cli` at path
    `/build/crates/fabric-cli/src/lib.rs`

`crates/fabric-cli/Cargo.toml` declares BOTH a `[[bin]]` (`fabric` ->
`src/main.rs`) and a `[lib]` (`fabric_cli` -> `src/lib.rs`). The Dockerfile's
dependency-stub layer creates `crates/fabric-cli/src/main.rs` but never
`src/lib.rs`, so cargo cannot even resolve the workspace. Differential proof
(run locally against a scaffolded copy of the builder stage):

- without the stub: `error: can't find lib 'fabric_cli' at path .../crates/fabric-cli/src/lib.rs`
- with the stub: resolves cleanly

Fix: add `echo "pub fn _stub() {}" > crates/fabric-cli/src/lib.rs` to the
Dockerfile stub layer.

**2. The `docker` job was not `continue-on-error`, unlike `docker-build`.**

`docker-build` is marked best-effort, but the downstream `docker` push job
was not, so a Docker failure turned an otherwise complete release run red.
That is exactly what happened on the v0.1.4 run: `Create GitHub Release` and
`Post-publish verification` both succeeded, yet the run was reported as
`failure`. Fix: mark the `docker` job `continue-on-error: true`.

### Defective releases annotated

- `v0.1.3` (0 assets): release body now leads with a warning that it is
  broken and points at v0.1.4, explaining both root causes.
- `v0.1.2` (incomplete Linux archive): release body now leads with a warning
  naming the two missing binaries and points at v0.1.4.

Neither release was deleted or re-tagged: a published release's assets cannot
be replaced in place, and the tags carry real build history.
