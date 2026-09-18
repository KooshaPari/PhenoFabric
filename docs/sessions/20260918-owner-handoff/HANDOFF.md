# PhenoFabric — Owner Session Handoff

**Written:** 2026-09-18 (Pacific) · **Author session:** Jcode on `Kooshas-Laptop` · **Handoff to:** new owner session running on the operator's desktop

This document transfers ownership of `KooshaPari/PhenoFabric` and all future work on it. Read it **before touching anything**. Every claim here is either verified-with-date or explicitly marked UNKNOWN. Do not upgrade an UNKNOWN to a pass.

---

## 0. Read order

1. This file (top to bottom).
2. `/Users/kooshapari/CodeProjects/docs/docs-5/products/PhenoFabric/START-HERE.md` (authority entrypoint).
3. `.../PhenoFabric/STATE.md` (dated current state — observation date 2026-09-16, so it predates everything in §5).
4. `.../PhenoFabric/NEXT-ACTIONS.md` (bounded work with acceptance criteria).
5. `.../PhenoFabric/DOSSIER.md`, `PROOF-AND-GRADE.md`, `PILOT.json`.

`DOSSIER.md`, `PILOT.json` and prior audit verdicts are **context, not proof**. A green local suite is not installation, not adoption, not user acceptance.

---

## 1. Machine topology and how to reach this host

The new session runs on the operator's **desktop**. The repo, the Rust/toolchain provisioning, the Tauri bundle and the installed `/Applications/Phenotype Fabric.app` currently live on **this laptop**. SSH over when a task needs local-only state (the checkout, its build cache, the installed `.app`, or the macOS GUI build).

| Fact | Value | Verified |
|---|---|---|
| Hostname | `Kooshas-Laptop.local` | 2026-09-18 |
| Local user | `kooshapari` | 2026-09-18 |
| Tailscale node | `kooshas-laptop` = `100.112.14.98` (online) | 2026-09-18 |
| LAN address | `192.168.1.23` | 2026-09-18 |
| sshd | listening on port 22 (loopback verified) | 2026-09-18 |
| `~/.ssh/authorized_keys` | 3 keys: `kooshapari-desk-forge-recovery`, `kooshapari-desk-wsm3d`, `moshi-pair:host_f201d2…` | 2026-09-18 |
| Hardware | Apple M1 Pro, 16 GB, 1 TB, iGPU | 2026-09-18 |
| OS | macOS 27.0 (build 26A5353q) | 2026-09-18 |

Connect to this laptop (from the desktop):

```bash
ssh kooshapari@100.112.14.98        # Tailscale, preferred
ssh kooshapari@192.168.1.23         # same LAN only
ssh kooshapari@Kooshas-Laptop.local # mDNS on the same LAN
```

**The desktop→laptop direction was verified working**, not assumed: running
`ssh -o BatchMode=yes kooshapari@100.112.14.98 "echo ...; hostname"` *from the desktop* returned
`Kooshas-Laptop.local`. The laptop already holds `kooshapari-desk-forge-recovery` and
`kooshapari-desk-wsm3d` in `~/.ssh/authorized_keys`, so no new key exchange is needed.

### The desktop is Windows, and it is online

This corrects an earlier draft of this document, which claimed no desktop node was online. That was
**wrong**. Verified 2026-09-18 from this laptop:

| Fact | Value | Verified |
|---|---|---|
| Desktop hostname | `kooshapari-desk` | 2026-09-18 |
| Desktop Tailscale | `kooshapari-desk` = `100.96.135.160` — **active**, direct `192.168.1.159:41641` | 2026-09-18 |
| Desktop OS | **Windows 11** (Windows NT 10.0.28120.0), PowerShell 5.1 | 2026-09-18 |
| Reach it from here | `ssh desk` (alias is in this laptop's `~/.ssh/config`; user `koosh` with `~/.ssh/id-git`, **not** `kooshapari`) | 2026-09-18 |
| Desktop→laptop SSH | **verified working** | 2026-09-18 |
| Desktop has | git 2.55.0, node, Docker Desktop, `wsl`, PowerShell 5.1 | 2026-09-18 |
| Desktop lacks | **Rust (`cargo`/`rustc` absent on the Windows side)**, posh-git/VS Code `code` | 2026-09-18 |
| WSL2 distros | `FedoraLinux-44` (**Running**), `podman-default` (**Running**), `docker-desktop` (Stopped) | 2026-09-18 |

Other tailnet peers at handoff: `cachyos` (100.97.123.10, offline 41d), `kooshapari-desk-1`
(100.84.189.31, offline 136d), `kooshapari-desk-2` (100.122.128.84, offline 16d), `iphone182`
(online).

### Consequences of the desktop being Windows (read this before assigning work)

1. **The clean-machine install smoke (§6.6) must use the Windows installer**, not the macOS `.dmg`.
   `release.yml` already builds an MSI and an NSIS `.exe` for `x86_64-pc-windows-msvc`. The `.dmg`
   can only be installed on a Mac, so do not attempt that test from the desktop.
2. **You have native Linux on the desktop via WSL2** — `FedoraLinux-44`, kernel
   `6.18.40.1-microsoft-standard-WSL2`, **x86_64** (the same CPU arch as GitHub's `ubuntu-latest`
   runners). It has git, gcc, cc, pkg-config, dnf and python3; `cargo`/`rustc`/`rustup` are absent
   and must be installed. This is the best available host for reproducing Linux CI, and better than
   the Docker route in §9.
3. **WSL builds must live on the WSL-native filesystem** (`~/work/...`), not under `/mnt/c`. Building
   a 22-crate workspace across the Windows mount is dramatically slower.
4. **The laptop is still the only host with a provisioned Rust toolchain, the build cache and the
   installed macOS app.** Use `ssh kooshapari@100.112.14.98` for those.

Wake-on-LAN is configured for a *separate* host, `00:81:2a:ee:d4:9b` @ `192.168.1.62`, via `~/bin/wake-fabric-machine.sh` and `~/bin/wake-monitor.sh` (documented in `docs/sessions/20260917-doc-review/06_WOL_SETUP.md`). That is **not** this laptop (`192.168.1.23`). Do not confuse the two.

### Preferred working model

- **Do repo work where the repo is.** Clone/fetch from GitHub on the desktop for read and light edits, but run `cargo` builds/tests and the Tauri bundle here — the toolchain and Linux-GUI-free macOS build path are provisioned here.
- **Heavy work placement.** This laptop is 1TB/16GB/1 iGPU. A stronger box (5.5TB/64GB/2 real GPUs) exists. Rust compilation here is CPU-bound and fits; nothing in this repo currently needs a GPU. Decide placement before starting heavy work and record which placement served the job.
- **CI billing is exhausted** (see §10). Do not rely on GitHub Actions to tell you whether a change is good. Verify locally, or reproduce Linux in Docker (§9).

---

## 2. Repository identity and locations

| Item | Value |
|---|---|
| Repo path (this host) | `/Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-fabric` |
| Remote | `git@github.com:KooshaPari/PhenoFabric.git` (SSH) |
| Default branch | `main` |
| HEAD at handoff | `76656c2` |
| GitHub repo ID | `1363521465` (public) |
| Tags | `v0.1.0-nightly` — **local only; never pushed**. `git ls-remote --tags origin` returns nothing, and `gh release list` is empty. **There are zero published releases and zero remote tags.** |
| Workspace | 22 members, 249 `.rs` files, ~106.7k LOC under `crates/` + `examples/` |
| Live docs-5 dossier | `/Users/kooshapari/CodeProjects/docs/docs-5/products/PhenoFabric/` |

The repo is a Cargo workspace, **not** a top-level Python project. There is no `cli.py`.

Members:

```
fabric-capability        fabric-capability-ffi   fabric-checker
fabric-graph             fabric-graph-cli        fabric-persist
fabric-daemon            fabric-cli              fabric-tray
fabric-gui               fabric-web              fabric-frame-transport
phenotype-nvms-adapter   phenotype-manifest      fabric-integration-tests
fabric-tui               fabric-workspace        fabric-surface-mojo
fabric-orchestrator      fabric-terminal         fabric-capture
fabric-research-ledger   examples/full-demo
```

`phenotype-manifest` is a **workspace member**, so `phenotype-nvms-adapter`'s path dependencies are self-contained. The workspace does not depend on any external git URL or on a checkout outside this repo. This was a real defect earlier (a 404 git dep) and is fixed.

---

## 3. Ownership and governance contract

You now own this repository's work.

| Rule | Consequence |
|---|---|
| **One owner chat per repository** | Do not spawn a second permanent owner chat. |
| **No history rewrite** | `git push --force`, `git reset --hard`, `git clean -fd`, `git branch -D` are blocked. Fix forward. |
| **Ledger trailers** | Commits carry `tx-agent`, `tx-validated`, plus `tx-scope`/`tx-intent`. |
| **Release policy** | Anything not verified by the actual user stays `-nightly` / `-dev`. Never promote to a stable version string on the strength of a local test run. |
| **Honest evidence states** | Keep built / installed / launched / user-verified separate. Installed is not launched; launched is not verified. |
| **Dated evidence** | Historical results keep their observation date and never silently become a fresh pass. |
| **No fabricated progress** | Report UNKNOWN where evidence is missing. No invented percentages or ETAs. |
| **Preserve other sessions' work** | Check `git status` before any `checkout`/`stash`. Never discard work you did not write. |
| **Screenshots** | No whole-desktop capture. Agent-started isolated processes only. |

Authority: source truth belongs to Git; repo identity to observed GitHub state; execution truth to a qualified run with a retained receipt. A doc calling itself "canonical" is an observation, not authority.

---

## 4. Verified current state (dated)

All rows verified 2026-09-18 on this host unless noted.

| Area | State | Evidence |
|---|---|---|
| Working tree | clean; `main` was `76656c2` when this handoff's measurements were taken, then advanced by the handoff's own doc commits (`6a17806`, `10aca92`, `d2d545b`) | `git status --porcelain` → 0 |
| Toolchain resolution | stable **1.97.1** inside the repo (`rust-toolchain.toml` pins `stable`); the *default* toolchain on this host is nightly 1.99 | `rustc --version` in repo dir |
| `cargo check --workspace` | passes | local |
| Unit tests (`--lib`) | **451 passed, 0 failed** | `cargo test --workspace --lib` |
| Integration tests (`--test '*'`) | **174 passed, 0 failed** across 23 test binaries | `cargo test --workspace --test '*'` |
| `cargo clippy --workspace --all-targets` | 0 errors (warnings only, from dependencies) | local |
| `cargo clippy -p fabric-terminal -p fabric-capture --features self-update` | 0 errors | local |
| `fabric-capture` release profile override | **effective** — its own units compile at `-C opt-level=s` (2 units) while all 63 others use `opt-level=3`; binary 1.42 MB | `cargo build -p fabric-capture --release --verbose` |
| Cargo "profiles for the non root package will be ignored" warning | **gone** (was emitted on every cargo invocation; `fabric-capture` carried an ignored member `[profile.release]`) | manifest inspection |
| Tauri bundle | builds; `/Applications/Phenotype Fabric.app` installed, `CFBundleShortVersionString = 0.1.0-nightly` | `defaults read` |
| Published release artifacts | **none** — no remote tags, no GitHub releases, no downloadable installer for any platform | `git ls-remote --tags origin`; `gh release list` |
| GitHub Actions CI on `main` | **FAILING** — `Check` **passes** on Linux (1m13s); `Clippy`, `Unit tests`, `Integration tests` fail with exit 101. All three of those pass on macOS. | run `35323422107` |
| Root cause of the Linux-only failures | **UNKNOWN at message level**, but narrowed to lint deltas + test failures (not a build failure) — see §6.1 | annotations API exposes only `exit code 101`; job logs are 403 |
| Login card rendering (visual) | **UNVERIFIED** | no successful GUI automation (§6.2) |
| SSO end-to-end (system-browser → callback → token) | **UNVERIFIED** — code path exists, never observed succeeding | — |
| Clean-machine install | **NOT RUN** | — |

Do not restate "tests pass" without naming the platform. The honest sentence is: **unit + integration suites pass on macOS; the Linux CI equivalent fails for an unknown reason.**

---

## 5. What landed in the handing-off session

| Commit | Summary |
|---|---|
| `fd6459a` | Handoff corrections: the desktop is an online **Windows** node with WSL2 Fedora (verified), the `v0.1.0-nightly` tag is **local-only** with zero published releases, and the clean-machine install must use the Windows installer. Also moved `fabric-capture`'s ignored member `[profile.release]` to a supported root package override. |
| `0d33b36` | Handoff: corrected a stale HEAD value and two section cross-references. |
| `d2d545b` | Handoff: narrowed the Linux CI failure (`Check` passes on Linux ⇒ not a build break) and recorded the Docker/QEMU/sshfs environment traps. |
| `10aca92` | Linked the handoff from `INDEX.md`. |
| `6a17806` | The handoff document itself. |
| `76656c2` | Removed stale `tf-ci.yml` and `tf-release.yml` (both referenced `rust/` and `rust/windows-capture/` paths that never existed after the terminal-fabric absorption in `e65cdfc`; both failed on every push, and `tf-release.yml` collided with `release.yml` on the same `v*` trigger). Folded the one unique piece — a clippy pass over `fabric-terminal`/`fabric-capture` with `self-update` — into `ci.yml`. |
| `9b2394f` | Restored the manual `Default` impls in `crates/fabric-daemon/src/config.rs`. Commit `1c21232` ("final clippy cleanup") had replaced all 8 with `derive(Default)`, which silently zeroed operational defaults: listen address `""`, database path `""`, `wal_mode=false`, log level `""`, empty `public_routes`, etc. Fixed 3 failing unit tests (`default_config_is_valid`, `save_and_load_roundtrip`, `coordinator_config_snapshot`) **and** a real first-boot defect: with no config file the daemon would bind to an empty address and use an empty DB path. |
| `6da7489` | `ci.yml`: `libpango-1.0-dev` → `libpango1.0-dev` (the former does not exist on the runner image). |
| `b51eab7` | `ci.yml`: added the GTK3/pango/cairo dev packages the Tauri GUI build needs on Linux. |
| `6840d4f` | `v0.1.0-nightly` release: version bumped in `Cargo.toml` + `tauri.conf.json`, Tauri `.app` + DMG built, installed to `/Applications`, tagged. |
| `53079f6` | SSO white-screen fix: WorkOS sends `X-Frame-Options: DENY`, so the iframe popup could never render. Replaced with a system-browser flow — new `crates/fabric-gui/src-tauri/src/auth_callback.rs` runs a one-shot HTTP listener on a random port, captures the OAuth redirect, emits `auth-code-received` to the frontend; frontend calls `openAuthInBrowser()` and completes on the event; dead iframe overlay (HTML+CSS+JS) removed; `start_auth_listener` command added. |
| `eff2433` | Workspace build repair: `phenotype-nvms-adapter` pointed at a 404 git URL (`nanovms.git`); repointed to the in-repo `phenotype-manifest` member. Added the `env` feature to workspace `clap` (needed by `fabric-capture`). |

All pushed to `origin/main`.

---

## 6. Pending work, in priority order

Ordering rule: smallest remaining effort, fastest useful outcome, fewest dependencies. `NEXT-ACTIONS.md` is authoritative if it disagrees.

### 6.1 Unblock Linux CI — do this first

Everything below is untrustworthy while CI is red, and the fix is small and self-contained. The suites pass on macOS and fail on Linux for clippy, unit, and integration. **Reproduce on the native Linux host that is now known to exist** — WSL2 Fedora on the desktop (§9) — read the real errors, fix forward.

**Already-narrowed diagnosis (verified, run `35323422107`, 2026-09-18).** The `Check` job — `cargo check --workspace --all-targets` — **passes on Linux in 1m13s**. That rules out a build/link failure, missing system library, and missing dependency. So the failures are two *separate* problems, not one:

| Job | Result | What that implies |
|---|---|---|
| `Check` (`cargo check --workspace`) | **passes** | the workspace compiles cleanly on Linux; no missing pkg-config library |
| `Clippy` (`-- -D warnings`) | fails, exit 101 | **Linux-only lint warnings**, promoted to errors by `-D warnings` |
| `Unit tests` (`--lib`) | fails, exit 101 | **real test failures** on Linux (compile already proven fine by `Check`) |
| `Integration tests` (`--test '*'`) | fails, exit 101 | **real test failures** on Linux |

That also means: do **not** assume all three share one root cause. Fix the lint deltas and the test failures independently.

Known lint traps in this workspace, both of which behave differently per platform:
- `crates/fabric-workspace/src/lib.rs` carries `#![deny(missing_docs)]` and `#![warn(rust_2018_idioms)]`. Under CI's `-D warnings` the `warn` becomes an error, and `missing_docs` fires per-platform — an item that only exists on one target must be documented on that target.
- Only six `#[cfg(target_os = "linux")]` blocks exist workspace-wide (`fabric-capability/src/probe.rs` ×4, `fabric-tray/src/main.rs` ×2). They were inspected and are correctly cfg-gated, so the linux-only warnings are most likely *not* there — look at `#[cfg(target_os = "macos")]` code whose Linux counterpart is missing, and at macOS-only helpers that become dead code on Linux.

**Acceptance:** `cargo clippy --workspace --all-targets -- -D warnings` and both test commands produce 0 failures under Linux. Note: job logs are **403** for this identity (`Must have admin rights to Repository`) — the annotations API only exposes `exit code 101`. The Docker reproduction is the available path; do not assume the failure is unfixable, and do not guess at it.

### 6.2 Make the GUI actually verifiable (largest honest gap)

Two independently written features have **never been observed working**: the rebuilt login card and the system-browser SSO flow. The blocker was never the code — it was that the Tauri WebView is invisible to macOS automation (`screencapture`, System Events, and Accessibility APIs all return 0 windows). The display also slept and could not be woken remotely.

Resolve the observation problem before writing more GUI code. Options: a real interactive session, a VNC/screen-share path, driver-level capture of the WebView process, or a Linux/CI-hosted WebDriver run against the same bundle.

**Acceptance:** a screenshot or video of the running app showing the populated login card; then a recorded successful SSO round-trip reaching an authenticated state. Until then the GUI is `-nightly` and unverified, and no release note may claim SSO works.

### 6.3 `CUR-1363521465-A1` — qualify the authentication contract

Test success, denial, cancellation, expired session, unavailable network, and native callback; compare expected local/offline behavior against accepted policy. **Acceptance:** no demo success, no credential leak, no irrecoverable login loop; actual scoped user state observed.

### 6.4 `CUR-1363521465-A2` — prove one real two-node path

The 2-node harness work (F.5 phases, `68ef121`, `88ecb74`, `f0f8f11`) built the integration scaffolding and split `two_node.rs` at the 500-line limit. Acceptance still requires genuine producer/consumer endpoints with negotiated capabilities, cancellation, disconnect/reconnect, and correct focus/input/output ownership — **no graph-only fixture standing in for working devices.**

### 6.5 `CUR-1363521465-A3` — locality and contention behavior

Choose shared-memory/local transport where supported and qualified network adapters otherwise; measure tail latency, quality, and foreground impact. **Acceptance:** no hidden copies or queues, no unsupported-capability pass, real Mac/PC workflow proof.

### 6.6 Clean-machine install smoke

Install the `v0.1.0-nightly` **Windows** installer on the desktop (a machine without the repo), launch, and confirm the bundled frontend loads. Report signing/SmartScreen and missing-resource errors.

Do **not** attempt this with the macOS `.dmg`: the desktop is Windows (§1), so the `.dmg` cannot be installed there. `release.yml` already produces an MSI and an NSIS `.exe` for `x86_64-pc-windows-msvc`, but **no `v0.1.0-nightly` Windows artifact has been built or published** — the only tag is `v0.1.0-nightly`, and verifying its release assets is part of this item. If no Windows installer exists, producing one is the prerequisite.

This is the smallest gap to a verifiable *installed* product on the machine the owner actually uses.

### 6.7 `-nightly` → nothing yet

There is no path to a stable version string until 6.2 and 6.6 are both done and user-verified. Do not bump the version for a green test run.

---

## 7. Open risks and unknowns

- **Linux CI failures are UNKNOWN at the message level.** Three jobs fail with rust exit 101; logs are inaccessible (403). Everything else in §4 was verified on macOS only, so Linux-specific `#[cfg(target_os = "linux")]` code paths and dependency resolution are effectively unqualified.
- **A prior "clippy cleanup" commit silently broke runtime defaults.** Commit `1c21232` passed a local macOS clippy run while zeroing `ServerConfig::listen`, `DatabaseConfig::path`, `wal_mode`, `LoggingConfig::level` and `AuthConfig::public_routes`. It was caught only because three unit tests happened to assert those values. **Treat "clippy clean" as a lint signal, not a behaviour signal**, and be suspicious of commit messages that claim lint-driven API simplification.
- **`.github/workflows/` had two permanently-red workflows for the whole lifetime of the repo.** They were never noticed because CI was already red for other reasons. Re-check that every workflow file references paths that exist.
- **The GUI has no automated observation path** (§6.2). This is a process gap, not a code gap, and it is why two shipped features are unverified.
- **Only three in-tree TODOs exist, and one is security-relevant:**
  - `crates/fabric-terminal/src/web_main.rs:367` — `// TODO: proper auth via query param`. **This is the one that matters.** The `tf-web` HTTP surface authenticates via a query parameter. Treat that as an unqualified auth path: do not expose `tf-web` off-host until it is replaced, and do not cite it as evidence that Fabric's authentication is implemented.
  - `crates/fabric-cli/src/tui/mod.rs:319` — `// TODO: poll daemon health via TCP` (TUI shows static/absent health).
  - `crates/fabric-gui/src/index.html:2120` — `// TODO: implement search overlay` (cosmetic).
- **Nothing has been published.** The `v0.1.0-nightly` tag exists only in this laptop's clone; it was never pushed, and there are no GitHub releases and no downloadable artifacts for any platform. The "release" is therefore a locally built `.app` plus a local tag — treat it as an unversioned local build, not as a shipped release. Pushing the tag would trigger `release.yml` (workspace bins + Tauri GUI + crates.io publish + Docker), which has never been exercised; expect that first run to surface failures.
- **`gh` API access is asymmetric.** `gh run view` and `git push` work; `gh run view --log` and the job-logs API return 403 for this identity. CI diagnosis must go through the reproduction path, not the API.

---

## 8. Resume commands (run on this host)

```bash
cd /Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-fabric
git fetch --all && git status --short

# Fast confidence loop (macOS). Toolchain resolves to stable 1.97.1 from
# rust-toolchain.toml regardless of the host default nightly.
cargo check --workspace                     # ~1 min
cargo test --workspace --lib                 # 451 tests
cargo test --workspace --test '*'            # 174 tests, 23 binaries
cargo clippy --workspace --all-targets       # expect 0 errors

# Linux CI reproduction (definitive; see §9)
bash ~/.jcode/scratch/repro-linux-ci.sh

# GUI bundle + install
cd crates/fabric-gui/src-tauri && cargo tauri build
# -> target/release/bundle/{macos,dmg}/
ditto "target/release/bundle/macos/Phenotype Fabric.app" "/Applications/Phenotype Fabric.app"
```

Notes: keep scratch, targets and worktrees under `$JCODE_SCRATCH_DIR`, **not** `/tmp` — `/tmp` is RAM-backed on this host. Long suites take minutes; run them in the background with progress-emitting output rather than blocking.

Version strings live in **two** places and must be changed together: `crates/fabric-gui/src-tauri/Cargo.toml` and `crates/fabric-gui/src-tauri/tauri.conf.json`.

---

## 9. Linux CI reproduction (the tool that unblocks §6.1)

### Preferred: native Linux via WSL2 on the desktop

The desktop has `FedoraLinux-44` under WSL2, **native x86_64** — the same CPU architecture as
GitHub's `ubuntu-latest` runners. Use this instead of Docker. Run it from this laptop with:

```bash
# write a bash script locally, copy it over, execute by path
# (nested quoting through bash -> ssh -> powershell -> wsl -> bash silently truncates output,
#  so never inline a multi-command string)
cat > /tmp/step.sh <<'EOS'
set -uo pipefail
export PATH="$HOME/.cargo/bin:$PATH"
cd ~/work/phenofabric
cargo clippy --workspace --all-targets -- -D warnings; echo "CLIPPY=$?"
EOS
scp -o BatchMode=yes /tmp/step.sh desk:C:/Windows/Temp/step.sh
ssh -o BatchMode=yes desk "powershell -NoProfile -Command \"wsl -d FedoraLinux-44 -- bash /mnt/c/Windows/Temp/step.sh\""
```

Install Rust in the distro first, then clone into the **WSL-native** filesystem
(`~/work/...`, never `/mnt/c` — the Windows mount makes a 22-crate build crawl):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
  --profile minimal --component clippy --component rustfmt
git clone https://github.com/KooshaPari/PhenoFabric.git ~/work/phenofabric   # public repo, https is fine
```

You will also need the Linux build deps translated to Fedora names (`libxcb-devel`, `libxkbcommon-devel`,
`wayland-devel`, `glib2-devel`, `atk-devel`, `at-spi2-atk-devel`, `dbus-devel`, `libsoup3-devel`,
`gtk3-devel`, `gdk-pixbuf2-devel`, `pango-devel`, `cairo-devel`) plus **`openssl-devel` and
`libssh2-devel`** — `openssl-sys` and `libssh2-sys` are in the Linux dependency tree (via
`reqwest`/`native-tls` and `ssh2`) but not the macOS one, so the Linux build genuinely needs system
OpenSSL. Fedora is not Ubuntu: if a package name differs, `dnf search <term>`. rustc/clippy version
is what drives lint results and both get `stable`, so lint findings transfer; flag anything that
looks Fedora-specific rather than a genuine Linux-vs-macOS delta.

### Fallback: Docker on this laptop

A working reproduction script exists at `~/.jcode/scratch/repro-linux-ci.sh`. It runs the official `rust:1.97-bookworm` image, installs the same GUI packages `ci.yml` does, mounts the repo, and runs clippy (`-D warnings`), unit tests and integration tests under Linux, writing logs to `/tmp/repro-{clippy,unit,integ}.log`.

```bash
bash ~/.jcode/scratch/repro-linux-ci.sh
tail -100 /tmp/repro-clippy.log
```

Docker is available on this host via `colima`. The script uses a container-local `CARGO_TARGET_DIR` so it does not pollute the macOS `target/`, and mounts the host cargo registry to avoid re-downloading crates.

**Environment caveats found at handoff (these cost real time — read before retrying):**

- `colima status` reports **`runtime: QEMU`** and **`mountType: sshfs`**. Compiling a 22-crate workspace across an sshfs mount under QEMU is extremely slow. If the reproduction stalls, copy the repo *into* the container (`docker cp`) and build from container-local storage rather than building over the bind mount.
- The cached `rust:1.90` and `rust:1.98` images on this host are **`linux/amd64`**, so they run under QEMU emulation on this arm64 machine — avoid them. Pull an explicit `--platform linux/arm64` image.
- A prior attempt to run clippy in a container (`fabric-clippy`, `rust:1.98`) produced **no usable result**. It died in rustup, not in the code:
  `error: could not download file from 'https://static.rust-lang.org/dist/channel-rust-stable.toml.sha256' ... tls handshake eof`.
  Cause: the repo's `rust-toolchain.toml` pins `channel = "stable"`, so rustup tries to *install* stable inside the container instead of using the image's built-in toolchain. Network to `static.rust-lang.org` was later confirmed working (HTTP 200) from a container, so this was environment/emulation-flavoured, not a hard block.
  **Workarounds, in order:** (a) `rustup toolchain install stable --profile minimal --component clippy` as an explicit first step so the failure is visible; (b) set `RUSTUP_TOOLCHAIN` to the image's own toolchain to bypass the `rust-toolchain.toml` download entirely — but note this changes the compiler version relative to CI, so verify version-sensitive findings against an image matching CI's stable.
- Clean up leftovers: `docker rm -f fabric-clippy` (a stopped container from the earlier attempt still exists).

**Do not "fix" Linux CI by weakening the workflow** (removing `-D warnings`, marking tests ignored, or deleting jobs). The macOS/stable toolchain agrees the code is clean; find what Linux does differently.

---

## 10. Environment quirks and failure modes

- **Actions billing is exhausted.** Per the operator's standing policy, do not treat a billed-runner failure as a blocking bug, and do not dispatch agent swarms that trigger CI. Prefer local verification and the Docker reproduction.
- **`gh` cannot read job logs** on this repo (403, needs admin). It can read run lists, job lists, and annotations (which only say `exit code 101`). Plan around it.
- **Two toolchains coexist.** The host default is nightly 1.99; entering the repo selects stable 1.97.1 via `rust-toolchain.toml`. Clippy output therefore differs if you run it from outside the repo directory. Always `cd` first.
- **The working tree is clean at handoff** — the only untracked path is this handoff directory. An earlier session reported core dumps (`core.16371`, `core.20300`) at the repo root; **they are not present now and that report is unverified**. If a dump reappears, treat it as evidence the process under test is crashing and investigate the origin rather than deleting it.
- **Version must be bumped in two files** (`Cargo.toml` + `tauri.conf.json`) or the bundle reports a stale version.
- **The sibling portfolio repos are separate concerns.** `khostty` (`KooshaPari/Khostty`) and `Melosviz` (`KooshaPari/Melosviz`) are their own repos with their own handoffs. This repo is `PhenoFabric` only. At handoff: `khostty` at `5a85344f0` with 7 dirty files; `Melosviz` at `c6f7212` with 17 dirty files and a completed owner handoff. **Do not touch their dirty files from here.**

---

## 11. What "done" means for this product

Parent outcome: **the installed fabric completes a real multi-device workflow with safe state, recovery, and measured performance.**

Hard gates before success can be claimed (`verification/acceptance-gates.md` is the fuller statement):

1. A real multi-device workflow runs end-to-end, with observed transport and device effects — not a graph-only fixture.
2. Authentication is qualified across success, denial, cancellation, expiry and network loss, with no credential leak and no irrecoverable loop.
3. The published artifact installs and runs on a clean machine.
4. Linux CI is green, so "passes" is true on both platforms.
5. Locality and contention behavior are measured: tail latency, quality, foreground impact, no hidden copies.
6. Known limits are displayed and experimental modules are labelled; unsupported secure surfaces, remote DSP, or atomic routing are never implied by an adjacent demo.

---

## 12. First actions for the new owner

1. `git fetch --all`; confirm `main` is `d2d545b` or later and that the tree is clean. (The §4 measurements were taken at `76656c2`.)
2. Run §8's fast confidence loop to confirm the macOS numbers in §4 still hold.
3. **Take §6.1** — run the Docker reproduction, read `/tmp/repro-clippy.log`, and fix the Linux-only failures. It is self-contained, needs no other machine, and unblocks the trustworthiness of everything else.
4. Then **§6.6** (clean-machine install) from the desktop — smallest path to a verifiable installed product, and it needs a machine you already have.
5. Resolve **§6.2**'s observation problem before writing any more GUI code. Two unverified features already exist; a third should not join them.
6. Record results with dates. Update `docs-5/products/PhenoFabric/STATE.md` only after a verified pass, with a new observation date.
7. Keep the version `-nightly`. Do not promote it.

Do not reopen closed audits, do not create a competing ledger, and do not restart work whose evidence is already dated above.
