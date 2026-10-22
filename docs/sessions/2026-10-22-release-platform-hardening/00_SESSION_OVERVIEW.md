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

_(filled in after the release workflow completes)_
