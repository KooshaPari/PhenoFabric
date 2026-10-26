# Session: release platform hardening -- the v0.1.7 release

The first release published through the hardened, fail-closed release
pipeline. It is the first PhenoFabric release that carries a native
Linux ARM64 CLI archive, an aggregate `SHA256SUMS` manifest, and a real
multi-architecture Docker manifest list.

The released `v0.1.6` tag is not rewritten. It stays exactly where it
is.

---

## What changed since v0.1.6

| SHA | Subject |
|---|---|
| `317013f` | `fix(release): close three fail-open holes in the checksum gates` |
| `d87e00a` | `feat(release): publish a real multi-architecture Docker manifest list` |
| `ab822c6` | `feat(release): build Linux ARM64 natively and close five platform-list holes` |

(`b7cdae7`, `c25f88b`, `81dc93f` and `1092f6c` are also in range: they
are the drift-proof test, the macOS x86_64 hard gate, the manifest
regeneration, and the v0.1.6 evidence record respectively.)

---

## The four gaps this release closes

### 1. Linux ARM64 was never shipped

`v0.1.6` published exactly 10 assets:

```
phenotype-fabric-gui-linux-x64.AppImage
phenotype-fabric-gui-linux-x64.deb
phenotype-fabric-gui-macos-arm64.dmg
phenotype-fabric-gui-macos-x64.dmg
phenotype-fabric-gui-windows-x64.exe
phenotype-fabric-gui-windows-x64.msi
phenotype-fabric-v0.1.6-linux-x86_64.tar.gz
phenotype-fabric-v0.1.6-macos-aarch64.tar.gz
phenotype-fabric-v0.1.6-macos-x86_64.tar.gz
phenotype-fabric-v0.1.6-windows-x86_64.zip
```

There is **no `linux-aarch64` CLI archive**. The `aarch64-unknown-linux-gnu`
target was in the build matrix but had been disabled, because the x86_64
`ubuntu-latest` runner cannot reach the arm64 package indexes.

The root cause was not a mirror outage. `archive.ubuntu.com` does not
publish `binary-arm64` at all -- that path returns 404 permanently. arm64
packages live on `ports.ubuntu.com/ubuntu-ports`. GitHub's own ARM64
runner image uses that mirror (see `actions/runner-images`
`images/ubuntu/scripts/build/configure-apt.sh`), so a **native**
`ubuntu-24.04-arm` runner already points at the mirror that carries
arm64 packages, and no `dpkg --add-architecture arm64` cross-compile
dance is needed.

`linux-aarch64` is now built natively on `ubuntu-24.04-arm`.

### 2. No aggregate checksum manifest

Every individual archive in `v0.1.6` had a `.sha256` sidecar, but there
was no single `SHA256SUMS` file, so a user had to download N sidecars to
verify N downloads. `release.yml` now generates and publishes a combined
`SHA256SUMS` alongside the sidecars.

### 3. The Docker image was single-architecture

The published image was amd64-only, and the release body did not describe
it as multi-architecture. It is now a real manifest list with
`linux/amd64` and `linux/arm64` entries.

### 4. Five platform lists could disagree

Five independent places in the pipeline declared the release platforms:
the CLI build matrix, the required-release-asset gate, the post-publish
archive existence check, the post-publish archive contents check, and the
release body table. Nothing forced them to agree, and the drift test
that was supposed to catch it silently produced empty lists when a step
was renamed, deleted, or unreadable -- so it reported success while
checking nothing.

`program/scripts/test_release_platforms.py` now compares all five and
fails closed. It is wired into `.github/workflows/spec-validation.yml`.

---

## Evidence

### The drift test fails closed

Ten seeded defects were injected into the five platform lists. All ten
were detected, each for the correct reason. The failure mode for a
renamed, deleted, or unreadable step is now a hard failure, not an empty
list that compares equal to nothing.

### The unified contents checker

The post-publish contents check previously excluded Windows from the
main platform loop and handled it in a separate hardcoded block. All
five CLI platforms now go through one auditable loop, including the
Windows `.exe` suffix handling. Six synthetic archive cases were
exercised against the unified checker and all six behaved correctly.

### Native ARM64 CI evidence

Check run `119636443950`, job `Native acceptance linux-aarch64`:

| Property | Value |
|---|---|
| Runner label | `ubuntu-24.04-arm` |
| Runner image | `ubuntu24-arm64` |
| Target | `aarch64-unknown-linux-gnu` |

The logs show `fabric`, `fabric-daemon`, `fabric-tui`, `fabric-tray` and
`fabric-graph-cli` actually running and passing. The real `aarch64`
daemon started, answered a health probe, and stopped cleanly on native
ARM64 hardware. This is native evidence, not emulation.

### Repository gates

| Gate | Result |
|---|---|
| `check_workflows.py` | all 5 workflows passed lint |
| `check_links.py` | all cross-doc links valid |
| `check_manifest.py --strict` | 1140 files, all digests verified |
| `test_release_checksums.py` | 18/18 cases passed |
| `test_docker_manifest_archs.sh` | 5/5 cases passed |
| `test_release_platforms.py` | all five platform lists agree |

### CI on the release commit

| Run | Workflow | Result |
|---|---|---|
| `39721711796` | CI | `success` |
| `39721711797` | E2E | `success` |
| `39721711854` | Spec Validation | `success` |

SonarCloud Code Analysis reports `failure` on `ab822c6`. The identical
failure is present on the prior commit `d87e00a`, so it is pre-existing
and is not caused by these changes.

---

## Publication

### The v0.1.7 republish

`v0.1.7` was tagged at `b0938bd` and the release run `39724488642` built
and pushed every artifact correctly, then failed in the Docker manifest
verification step. The defect was `docker buildx imagetools inspect -f`,
which buildx does not support (`-f` is an unknown shorthand flag; exit
`125`). The fix is `--format '{{json .Manifest}}'`, landed in `d867b99`.

That fix alone could not be released, because the tag is the only source
of the release workflow's code: a tag-push rerun re-checks-out `b0938bd`,
which still contains the broken script. (Confirmed empirically: the
`v0.1.6` release run's `head_sha` equals the `v0.1.6` tag target.) A
`workflow_dispatch` run checks out the dispatched ref instead, but the
workflow had no way to NAME the version it was republishing.

So the release workflow gained a `version` dispatch input and a shared
resolver, `scripts/release/resolve_version.sh`, which every job sources.
The resolver resolves the version from the input when given and from
`GITHUB_REF` on a tag push, validates its shape, and requires the named
tag to exist in origin. The tag itself is never moved, deleted, or
rewritten: the workflow contains no `git push` of any kind, and
`softprops/action-gh-release` is now bound to the resolved tag with an
explicit `tag_name`.

Four defects were found and fixed while getting the republish through,
each caught by a fail-closed gate that had every right to refuse:

| Run | Failure | Cause |
|---|---|---|
| `40149865321` | `cargo-deny advisories` | `RUSTSEC-2026-0387` (crypto-bigint), issued 2026-10-23 -- one day AFTER the original run |
| `40180377333` | required-asset gate | the gate derived the version from `github.ref_name`, i.e. the branch "main" |
| `40243421768` | per-file checksum | GNU `sha256sum` escaped the Windows absolute path, prefixing the sidecar's DIGEST field with a backslash |
| `40245060938` | (the new test, not the release) | the test compared digests the wrong way; fixed in `aef85f7` |

The `RUSTSEC-2026-0387` case is worth stating plainly: it was a real
vulnerability in a real transitive dependency, and the gate was right to
block the release. There was no upstream upgrade path -- `crypto-bigint`
0.5.5 is the last release in the 0.5 line, the fix landed in 0.7.6, and
`jsonwebtoken` 11.1.0 (the latest) pins `p256 = "0.13.2"`, which pins
`crypto-bigint ^0.5`. The dependency was removed at its source instead
of the advisory being added to the `deny.toml` ignore list: the
`jsonwebtoken` `aws_lc_rs` backend implements every algorithm the daemon
uses and depends on none of `crypto-bigint` / `p256` / `p384` / `rsa` /
`ed25519-dalek`. Switching backends removed 26 pure-Rust crypto crates
from `Cargo.lock`. The cost is that `aws-lc-sys` builds AWS-LC's C core
through CMake, so the Dockerfile builder stage now installs `cmake`.

### Regression tests added

Each defect above is now covered by a test that fails closed.

| Test | Covers |
|---|---|
| `scripts/acceptance/test_docker_manifest_archs.sh` | the manifest check fails when an architecture is missing (10/10) |
| `scripts/acceptance/test_resolve_version.sh` | the resolver fails when it cannot determine a version, when the tag is absent, and on a malformed version (28/28) |
| `program/scripts/test_release_version_source.py` | the workflow never derives a version from the git ref, and every job reading a version output resolves it itself |
| `scripts/acceptance/test_release_sidecar_paths.sh` | sidecars name the bare file, not a path, and survive a backslash-containing directory (16/16) |

Every one of them has a negative control: reverting the fix makes the
test fail. The sidecar test's control was verified in a Linux container
(the CI environment), where GNU coreutils' escaping actually applies --
10 of 16 cases fail with the original code.

### Known pre-existing failure, not addressed here

`cargo test --workspace` (the full scope, which also runs doc-tests)
fails `error_message_table_is_complete_and_nonempty` in
`fabric-capability-ffi`:

```
error message for code Ok must be UTF-8
```

`FabricError::Ok` maps to the literal `"success"`, so the returned
pointer must be bad. This is PRE-EXISTING and unrelated to the release
work:

- It reproduces on the pristine `b0938bd` tag in a separate clean git
  worktree.
- `cargo test --workspace --lib` passes.
- `cargo test --workspace --test '*'` passes (4/4 runs) -- and this is
  the scope CI actually runs.
- `cargo test -p fabric-capability-ffi --test abi_surface` alone passes
  (8/8 runs).

It is outside the release path: the release workflow runs
`cargo build --release --bins`, which does not compile tests, and the
release binaries depend on `fabric-capability`, not on the FFI wrapper.
The mechanism was not determined; a delegated investigation did not
conclude before the release work was finished.

### Repository gates on the republish commit

| Gate | Result |
|---|---|
| `check_workflows.py` | all 5 workflows passed lint |
| `check_links.py` | all cross-doc links valid |
| `check_manifest.py --strict` | 1145 files, all digests verified |
| `test_release_checksums.py` | 18/18 cases passed |
| `test_docker_manifest_archs.sh` | 10/10 cases passed |
| `test_release_platforms.py` | all five platform lists agree |
| `test_resolve_version.sh` | 28/28 cases passed |
| `test_release_version_source.py` | pass |
| `test_release_sidecar_paths.sh` | 16/16 cases passed |
| `cargo deny check advisories/licenses/bans/sources` | all ok |

### CI on the republish commit `aef85f7`

| Workflow | Result |
|---|---|
| CI | `success` |
| E2E | `success` |
| Spec Validation | `success` |
| Dependency Audit | `success` |

_(publication evidence filled in below once the release workflow completes)_
