# Session: Docker image runtime fix -- post-release acceptance

Part of the `2026-10-21-docker-image-runtime-fix` session.
Read `00_SESSION_OVERVIEW.md` first for the goal and the background.

---

## Post-release acceptance: the v0.1.5 image was broken

The v0.1.5 release was published and its gates were green. The gates
were nonetheless insufficient: every Docker check in CI ran the binaries
with `--entrypoint`, which OVERRIDES the image's default command. A green
"the binaries load and run" result therefore says nothing about whether
`docker run <image>` works at all.

Re-running the actual container acceptance path found three independent
defects in the published image.

### Defect 1: a plain `docker run` died immediately

```
ERROR fabric_daemon: failed to create coordinator: database error:
unable to open database file: state.db
```

The daemon opens its SQLite database at a RELATIVE path (`state.db`).
The image had no `WORKDIR`, so the path resolved against `/`, which the
unprivileged `fabric` user cannot write. Every container exited at once.

Fix: `WORKDIR /var/lib/fabric` plus an absolute `--db` path in the
image's default command.

### Defect 2: the published port reached nothing

The daemon binds `127.0.0.1:9400` by default. That is right for a local
daemon and useless in a container: `docker run -p 9400:9400` published a
port nothing was listening on, and a CLI in a second container failed
with `status check failed ... Broken pipe`.

Fix: the image's default command passes `--listen 0.0.0.0:9400`. The
container's network namespace still isolates the daemon from the host;
the port is only reachable where the operator publishes it with `-p`.

### Defect 3: the health probe hung forever

The first diagnosis was wrong, and worth recording because the wrong
diagnosis looked plausible: the local probe image is `arm64/linux` on an
`arm64` host, so it was NOT running under QEMU, and the "5 second
timeout is too short for emulation" explanation did not hold.

The real cause is a bug in the probe itself. `fabric-daemon health` read
lines from the daemon socket in a loop until EOF:

```rust
for line in reader.lines() {
    match line {
        Ok(l) => { println!("{l}"); }
        ...
    }
}
```

The daemon's wire server deliberately KEEPS a connection open so a client
can send more messages over it, so EOF never arrives. The probe printed
the correct healthy JSON and then blocked forever. Observed directly:

```
Health check exceeded timeout (15s): {"status":"healthy","uptime_s":50,...}
Health check exceeded timeout (15s): {"status":"healthy","uptime_s":95,...}
```

Every Docker health check timed out and killed a perfectly healthy
daemon. The container was reported "unhealthy" while the daemon was
fine -- which makes an orchestrator restart a working daemon in a loop.
Raising the timeout from 5s to 15s did not fix it, because the probe
never returns at any timeout.

Fix: `fabric-daemon health` is now a one-shot request/response. It
writes the request, reads exactly ONE response line, and exits, with an
explicit 5s read/write timeout so a probe that cannot answer fails
loudly instead of hanging. The 15s Docker timeout stays as the outer
backstop.

The daemon binary was rebuilt and the behaviour confirmed: the probe now
returns and exits.

### The regression test, and the negative check that gives it meaning

`crates/fabric-daemon/tests/health_probe.rs` asserts the probe
TERMINATES -- not merely that it prints the right JSON, since printing
correctly and then hanging IS the bug. Three tests:

| Test | Asserts |
|---|---|
| `health_probe_terminates_instead_of_hanging` | The probe exits on its own, exits 0, and reports healthy |
| `health_probe_fails_when_no_daemon_is_listening` | The probe exits non-zero against a dead daemon |
| `daemon_answers_health_check_on_a_kept_open_connection` | The daemon answers twice on one kept-open connection (the server-side half of the contract) |

The negative check: reverting `cmd_health` to the v0.1.5 implementation
and re-running the suite produced

```
test health_probe_terminates_instead_of_hanging ... FAILED
the health probe HUNG: it did not exit within 20s (elapsed 20.0s).
```

and restoring the fix made it pass. A regression test that has only ever
been seen to pass proves nothing, so the failing run is the evidence that
the test actually covers the defect.

### CI assertions added

Both `ci.yml` and `release.yml` now run three assertions against the
image, the last two of which the v0.1.5 image fails:

1. A plain `docker run` must still be running after a grace period.
   (v0.1.5 failed: the container had exited.)
2. A CLI in a SEPARATE container must reach a healthy daemon over the
   published port. (v0.1.5 failed: "Broken pipe".)
3. The image's own `HEALTHCHECK` must reach the `healthy` state.
   (v0.1.5 failed: the probe hung and the container stayed
   "starting"/"unhealthy" forever.)

Assertion 3 is the only one that can see defect 3, because the CLI is a
different binary with its own one-shot client. In `release.yml` the
assertions run against the exact image bytes intended for publication,
so a broken image is never pushed to the registry.

### The negative control, run against the published image

`scripts/acceptance/docker_acceptance.sh` holds the same three assertions
that CI and the release workflow now run, so the evidence can be
reproduced locally against any tag.

Against the FIXED local image (`fabric-ci:probe`, rebuilt with the
one-shot health probe):

```
PASS: container is still running after a default docker run
PASS: CLI reaches a healthy daemon over the published port 9400
    Status:            healthy
    Uptime:            5s
    Active:            0 leases, 0 plans
PASS: the image HEALTHCHECK reports healthy
=== ALL ASSERTIONS PASS ===
```

Against the PUBLISHED broken image (`ghcr.io/kooshapari/phenofabric:v0.1.5`,
digest `sha256:ecc618d165e912729611567fb87e89275c3db6212e78f9bae8a719ce57793e42`):

```
FAIL: the container exited during startup
=== EXIT: 1 ===
```

with the container's own state and log:

```
status: exited
exit code: 1
ERROR fabric_daemon: failed to create coordinator: database error:
      unable to open database file: state.db
```

The same script, the same three assertions, opposite results. That is
what makes the green run meaningful: the assertions are not vacuous.
