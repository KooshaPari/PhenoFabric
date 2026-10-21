# Session: Docker image runtime fix -- the v0.1.6 release

Part of the `2026-10-21-docker-image-runtime-fix` session.
Read `00_SESSION_OVERVIEW.md` first for the goal and the background,
and `03_POST_RELEASE_ACCEPTANCE.md` for the defects that caused this
release.

---

## Goal

Publish `v0.1.6`: a release whose Docker image actually runs the primary
daemon workflow, with CI and release gates that exercise the real
container interface, and independently verify the published artifacts.

The released `v0.1.5` tag is not rewritten. It stays exactly where it
is.

## Commits

| SHA | Subject |
|---|---|
| `6d3c348` | `fix(docker): make the published image actually run the daemon workflow` |
| `0c905c6` | `ci(release): gate on the one validated acceptance script, not a copy` |

Both are pushed to `main` and both carry the ledger trailers
(`tx-agent: jcode`, `tx-validated: test`).

---

## The three defects, and what asserts each one

| Defect | Symptom | Asserted by |
|---|---|---|
| Relative `state.db` resolved under `/` | container exits 1, `unable to open database file: state.db` | acceptance assertion 1; `Dockerfile` `WORKDIR` + absolute `--db` |
| Daemon bound `127.0.0.1:9400` | published port unreachable, CLI gets `Broken pipe` | acceptance assertion 2; default `--listen 0.0.0.0:9400` |
| `cmd_health` looped until EOF | health check printed healthy JSON then hung forever, so the container was reported unhealthy | acceptance assertion 3; `cmd_health` reads exactly one line with a 5s timeout; `crates/fabric-daemon/tests/health_probe.rs` |

Defect 3 is the one an earlier draft of the changelog got wrong: it
blamed QEMU emulation of the `arm64` host. The real cause was in the
probe's own code. The changelog has been corrected.

---

## Evidence: the assertions were observed to FAIL, not just to pass

An assertion that has only ever been seen to pass proves nothing. Each
of these was run against the known-broken artifact.

### The Rust regression test

`crates/fabric-daemon/tests/health_probe.rs` has three tests:

- `health_probe_terminates_instead_of_hanging`
- `health_probe_fails_when_no_daemon_is_listening`
- `daemon_answers_health_check_on_a_kept_open_connection`

Run against the reverted `v0.1.5` `cmd_health`, the first test FAILED
after hanging for 20 seconds (the probe never returned). Run against the
fix, all three pass.

### The container acceptance script

`scripts/acceptance/docker_acceptance.sh` was run against BOTH images
with the same three assertions:

Against the PUBLISHED `ghcr.io/kooshapari/phenofabric:v0.1.5`
(digest `sha256:ecc618d165e912729611567fb87e89275c3db6212e78f9bae8a719ce57793e42`):

```
=== assertion 1: default docker run keeps the daemon running ===
FAIL: the container exited during startup
--- container logs ---
    ERROR fabric_daemon: failed to create coordinator: database error: database error: unable to open database file: state.db
=== EXIT: 1 ===
```

Against the rebuilt image:

```
=== assertion 1: default docker run keeps the daemon running ===
PASS: container is still running after a default docker run
=== assertion 2: the CLI reaches a healthy daemon over the published port ===
PASS: CLI reaches a healthy daemon over the published port 9400
    Status:            healthy
    Uptime:            5s
    Daemon addr:       127.0.0.1:9400
    Topology epoch:    0
    Active:            0 leases, 0 plans
=== assertion 3: the image HEALTHCHECK reaches 'healthy' ===
PASS: the image HEALTHCHECK reports healthy
=== ALL ASSERTIONS PASS ===
```

The same script, the same three assertions, opposite results.

---

## Evidence: native Ubuntu x86_64 CI

The container assertions are only meaningful on native Linux; an
emulated run proves less. Both CI runs below are native
`ubuntu-latest` x86_64.

| Run | Commit | Workflows | Result |
|---|---|---|---|
| `39665041604` | `6d3c348` | CI, E2E, Spec Validation, Dependency Audit | all `success` |
| `39665861551` | `0c905c6` | CI, E2E, Spec Validation, Dependency Audit | all `success` |

From run `39665861551`, job `Docker image builds`, step `Verify the
daemon starts and serves the wire protocol`:

```
PASS: container is still running after a default docker run
PASS: CLI reaches a healthy daemon over the published port 9400
    Status:            healthy
PASS: the image HEALTHCHECK reports healthy
=== ALL ASSERTIONS PASS ===
```

## Evidence: repository gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 1539 passed, 0 failed, 2 ignored |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --all -- --check` | clean |
| `check_links.py` | all cross-doc links valid |
| `check_workflows.py` | all 5 workflows passed lint |
| `check_manifest.py` (strict) | 1133 files, all digests verified |

---

## A defect found in the acceptance script itself

While unifying the workflow copies onto the shared script, two real bugs
in the script surfaced. Both were fixed before the release.

1. The container id was captured with `2>&1`. Docker's platform warning
   ("the requested image's platform (linux/amd64) does not match the
   detected host platform (linux/arm64/v8)") was prepended to the id, so
   the id was unusable and every failure reported `page not found`
   instead of the real cause. The id is now read from stdout only.
2. The container was started with `--rm`, which auto-deletes it the
   moment it exits. A crashed daemon therefore lost its logs -- the one
   piece of information that explains the failure. The container is now
   removed by the `EXIT` trap instead.

After those fixes the negative control reports the actual defect rather
than a Docker error.

## A defect found in the release workflow

`release.yml` set `IMAGE=...` inside one step and used it in a later
step. Shell variables do not survive into a later step, so the
reference would have expanded to an empty string. The image reference is
now spelled out explicitly in the assertion step.

## Why the assertions were unified

The three container assertions existed in three places: an inline copy
in `ci.yml`, a near-identical inline copy in `release.yml`, and the
standalone script that was actually validated locally. CI was gating on
a hand-maintained duplicate. Both workflows now call the shared script,
so the gate runs the exact artifact that was validated.

---

## Publication

| Item | Value |
|---|---|
| Tag | `v0.1.6` (annotated), commit `ecf9e7a2a8407919981a15ce0e79cbbd974bb268` |
| Release workflow run | `39668118930` -- all 15 jobs `success` |
| Published at | 2026-10-21T20:51:56Z |
| Release URL | https://github.com/KooshaPari/PhenoFabric/releases/tag/v0.1.6 |
| Image digest | `sha256:c1f7ef0316d76277a0d0598a4ae4657fdce485336321ccf716ab67f91afb092e` |
| Build identity | Cargo.lock SHA-256 prefix `ef672c7354d0` |

The release workflow ran the assertions on the exact bytes it pushed.
From run `39668118930`, job `Build Docker image`, step `Verify the built
image starts and serves as a daemon`:

```
PASS: container is still running after a default docker run
PASS: CLI reaches a healthy daemon over the published port 9400
PASS: the image HEALTHCHECK reports healthy
=== ALL ASSERTIONS PASS ===
```

The `Post-publish verification` job then confirmed all 4 CLI archives
are present, that they contain the expected binaries, and that the
published image is pullable.

### The `v0.1.5` tag was not rewritten

| Ref | Before | After |
|---|---|---|
| `v0.1.5` tag object | `122d41985e2fe96c29856979177640960c5f04f7` | `122d41985e2fe96c29856979177640960c5f04f7` |
| `v0.1.5` commit | `f8b32c8b48d3fc9976b95fbab689d4c08159c862` | `f8b32c8b48d3fc9976b95fbab689d4c08159c862` |

Both the local and the remote tag are byte-identical to their pre-release
values. The released `v0.1.5` still exists and still points at the
broken image; `v0.1.6` is a new tag alongside it.

### crates.io: not published, by design

The `Publish crates` job is a deliberate no-op while
`CRATES_IO_TOKEN` is not configured on the repository. The job log
shows:

```
::warning::CRATES_IO_TOKEN secret is not configured on this repo. Skipping cargo publish.
```

This is an expected configuration state, not a defect, and it is not
part of what this release set out to fix. The crates.io API confirms
`phenotype-fabric` does not exist as a published crate.

---

## Independent verification of the PUBLISHED artifacts

Everything below was run against the artifacts as downloaded from the
registry and the release page, not against the local build.

### The published image passes the acceptance script

The image was removed locally and pulled fresh from GHCR, then handed
to the same script that gates CI:

```
=== image: ghcr.io/kooshapari/phenofabric:v0.1.6 ===
PASS: container is still running after a default docker run
PASS: CLI reaches a healthy daemon over the published port 9400
    Status:            healthy
    Uptime:            5s
    Daemon addr:       127.0.0.1:9400
PASS: the image HEALTHCHECK reports healthy
=== ALL ASSERTIONS PASS ===
```

This is the exact path that failed for `v0.1.5`.

### The published health probe terminates; the v0.1.5 one hangs

Both published probes were run against the SAME live published daemon.
This isolates the probe defect from every other change.

The `v0.1.6` probe:

```
{"status":"healthy","uptime_s":6,"topology_epoch":0,"active_leases":0,"active_plans":0}
EXIT: 0
real 0m0.252s
```

The `v0.1.5` probe against the same daemon never returned. Its
container was still `running` nine minutes later, and the daemon
container's health log recorded the timeout on every single attempt:

```
exit=-1 Health check exceeded timeout (5s): {"status":"healthy","uptime_s":466,...}
exit=-1 Health check exceeded timeout (5s): {"status":"healthy","uptime_s":501,...}
exit=-1 Health check exceeded timeout (5s): {"status":"healthy","uptime_s":536,...}
```

The daemon was reporting itself healthy in JSON the whole time while
the probe refused to finish. That is the `v0.1.5` defect, reproduced
from published artifacts, and it is not emulation: the daemon was
healthy and answering.

The `v0.1.6` daemon container's own health log, for contrast:

```
exit=0 {"status":"healthy","uptime_s":517,...}
exit=0 {"status":"healthy","uptime_s":547,...}
exit=0 {"status":"healthy","uptime_s":578,...}
```

### The published binaries

`phenotype-fabric-v0.1.6-linux-x86_64.tar.gz` was downloaded from the
release page and contains `fabric`, `fabric-daemon`, `fabric-tui`,
`fabric-graph-cli`, `fabric-tray` and a `README.md`.

Note on versions: the binaries report the workspace version `0.1.0`,
not the tag version. This is pre-existing behavior -- the released
`v0.1.5` binaries report `0.1.0` as well -- and the release workflow
derives the release name from the Git tag, not from
`CARGO_PKG_VERSION`. It is not a regression introduced by this
release, and nothing in the release gates asserts the two must match.

