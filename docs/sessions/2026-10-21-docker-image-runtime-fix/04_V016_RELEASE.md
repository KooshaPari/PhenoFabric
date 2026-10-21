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
