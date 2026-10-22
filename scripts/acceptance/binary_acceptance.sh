#!/usr/bin/env bash
# Cross-platform binary acceptance test.
#
# A binary that COMPILES is not a binary that WORKS. The release workflow
# only asserted that each expected binary file existed in the target
# directory; it never executed the macOS or Windows artifacts. A
# platform-specific defect -- a missing shared library, a broken
# linker, a startup panic in the daemon -- would ship green and only
# surface on a user's machine.
#
# This script runs the REAL binaries on the runner that natively hosts
# the target platform, so the artifact that gets published is proven to
# start before it is published.
#
# Usage: scripts/acceptance/binary_acceptance.sh <target-triple> <bin-dir>
#
#   <target-triple>  e.g. x86_64-unknown-linux-gnu, aarch64-apple-darwin
#   <bin-dir>        directory containing the built binaries (release/)
#
# Assertions:
#   1. Every expected binary exists and is runnable (`--version` exits 0
#      and prints a non-empty version string).
#   2. The daemon starts with an explicit db path and listen address,
#      answers a health probe, and shuts down cleanly.
#
# Exits non-zero on the first failed assertion.
#
# Portability: this runs on Linux, macOS and Windows runners, so it must
# not assume GNU coreutils. `seq` (absent from some minimal images),
# `mktemp -d -t` (BSD and GNU spell the template differently), and
# `sha256sum` (absent from stock macOS, which ships `shasum` instead)
# are all avoided or shimmed below.

set -uo pipefail

TARGET="${1:-}"
BIN_DIR="${2:-}"

if [ -z "$TARGET" ] || [ -z "$BIN_DIR" ]; then
  echo "usage: $0 <target-triple> <bin-dir>" >&2
  exit 2
fi

# The single source of truth for release contents. Must match
# EXPECTED_RELEASE_BINS in .github/workflows/release.yml and the
# post-publish verification step.
EXPECTED="fabric fabric-daemon fabric-tui fabric-tray fabric-graph-cli"

# Windows executables carry a .exe suffix; Unix ones do not.
case "$TARGET" in
  *-windows-*) SUFFIX=".exe" ;;
  *)            SUFFIX="" ;;
esac

# The daemon binds a second, distinct socket for the OAuth HTTP callback
# (127.0.0.1:9401 by default). That bind failure is logged but not
# fatal, so it cannot break this test.
PORT="${FABRIC_TEST_PORT:-19400}"
LISTEN="127.0.0.1:${PORT}"

# How long the daemon must stay alive after answering the health probe.
#
# A liveness check taken at a single instant is not a liveness check: a
# daemon that answers the probe and then exits one second later would
# sail through it. Re-checking after a short settle window turns that
# class of defect into a visible failure.
SETTLE_SECS=3

# How long the daemon may take to exit after SIGTERM before it is
# considered stuck. A daemon that ignores the shutdown signal is not a
# daemon that can be stopped, and it must fail the gate rather than be
# silently tolerated.
STOP_GRACE_SECS=15

# `mktemp -d` without a template works on both BSD and GNU. The BSD form
# requires `-t` for a named template, but the plain form does not, so
# avoid `-t` entirely.
WORK="$(mktemp -d 2>/dev/null)" || {
  echo "FAIL: could not create a scratch directory" >&2
  exit 1
}

DAEMON_PID=""
DAEMON_LOG="${WORK}/daemon.log"

cleanup() {
  if [ -n "$DAEMON_PID" ]; then
    kill "$DAEMON_PID" >/dev/null 2>&1 || true
    # Bounded reap. A plain `wait` here would block forever if the
    # daemon ignores SIGTERM, wedging the runner at script exit
    # instead of failing the gate.
    waited=0
    while [ "$waited" -lt 3 ]; do
      kill -0 "$DAEMON_PID" >/dev/null 2>&1 || break
      sleep 1
      waited=$((waited + 1))
    done
    kill -9 "$DAEMON_PID" >/dev/null 2>&1 || true
    wait "$DAEMON_PID" 2>/dev/null || true
  fi
  rm -rf "$WORK" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# Poll a pid until it exits or the budget is exhausted.
# Sets `poll_rc` to the exit status, or 124 on timeout.
poll_until_exit() {
  local pid="$1" budget="$2" waited=0
  while [ "$waited" -lt "$budget" ]; do
    if ! kill -0 "$pid" >/dev/null 2>&1; then
      wait "$pid" 2>/dev/null
      poll_rc=$?
      return 0
    fi
    sleep 1
    waited=$((waited + 1))
  done
  kill -9 "$pid" >/dev/null 2>&1 || true
  wait "$pid" 2>/dev/null || true
  poll_rc=124
  return 0
}

echo "=== binary acceptance: target=${TARGET} bin-dir=${BIN_DIR} ==="

# ── Assertion 1: every expected binary exists and runs ─────────────────
echo "=== assertion 1: every expected binary exists and runs ==="
for bin in $EXPECTED; do
  path="${BIN_DIR}/${bin}${SUFFIX}"
  if [ ! -f "$path" ]; then
    echo "FAIL: expected binary missing: $path"
    exit 1
  fi
  # On Unix the archive must ship a runnable binary. Windows .exe files
  # carry no POSIX exec bit, so only check -x there.
  if [ -z "$SUFFIX" ] && [ ! -x "$path" ]; then
    echo "FAIL: binary is not executable: $path"
    exit 1
  fi

  # Actually RUN it. `--version` exits 0 and prints a version string.
  # This is the check that was missing: the file existed, but nothing
  # proved it could execute on this platform.
  #
  # A binary that hangs (for example a GUI app that ignores unknown
  # flags and enters its event loop) would otherwise wedge the runner
  # until the job timeout. The timeout keeps the failure honest and
  # fast: a binary that cannot report its version is not shippable.
  out=""
  if command -v timeout >/dev/null 2>&1; then
    out="$(timeout 30 "$path" --version 2>&1)"
    rc=$?
  else
    # No coreutils `timeout` (stock macOS, minimal Windows Git-Bash).
    # Fall back to a background process with a manual watchdog.
    "$path" --version >"${WORK}/ver.out" 2>&1 &
    vpid=$!
    poll_until_exit "$vpid" 30
    rc="$poll_rc"
    out="$(cat "${WORK}/ver.out" 2>/dev/null || true)"
  fi

  if [ "$rc" = "124" ]; then
    echo "FAIL: ${bin}${SUFFIX} --version did not exit within 30s (it hung)"
    exit 1
  fi
  if [ "$rc" != "0" ]; then
    echo "FAIL: ${bin}${SUFFIX} --version exited ${rc}"
    printf '%s\n' "$out" | sed 's/^/    /'
    exit 1
  fi
  if [ -z "$out" ]; then
    echo "FAIL: ${bin}${SUFFIX} --version printed nothing"
    exit 1
  fi
  echo "PASS: ${bin}${SUFFIX} runs --version -> ${out}"
done

# ── Assertion 2: the daemon starts, serves health, and stops ───────────
echo "=== assertion 2: the daemon starts, answers health, and stops cleanly ==="

DAEMON="${BIN_DIR}/fabric-daemon${SUFFIX}"
DB="${WORK}/state.db"

# Start the daemon in the background. The db path and listen address are
# explicit: a relative db path resolves against the current working
# directory, which is exactly the class of defect that broke the
# v0.1.5 Docker image.
"$DAEMON" start \
  --db "$DB" \
  --listen "$LISTEN" \
  >"$DAEMON_LOG" 2>&1 &
DAEMON_PID=$!

# Wait for the daemon to bind and answer. `health` is the same probe the
# container HEALTHCHECK uses, so this proves the published probe path
# works natively on this platform.
healthy=0
out=""
waited=0
while [ "$waited" -lt 30 ]; do
  if ! kill -0 "$DAEMON_PID" >/dev/null 2>&1; then
    break
  fi
  if out="$("$DAEMON" health --connect "$LISTEN" 2>&1)"; then
    if printf '%s' "$out" | grep -qi 'healthy'; then
      healthy=1
      break
    fi
  fi
  sleep 1
  waited=$((waited + 1))
done

if [ "$healthy" != "1" ]; then
  echo "FAIL: the daemon did not report healthy on ${TARGET}"
  echo "--- daemon log ---"
  sed 's/^/    /' "$DAEMON_LOG" 2>/dev/null || true
  exit 1
fi
echo "PASS: daemon reports healthy on ${TARGET}: ${out}"

# The daemon must still be running after answering -- a daemon that
# answers once and exits would pass the check above.
#
# Two checks, because one instant is not enough. The first catches a
# daemon that dies immediately; the second catches one that answers and
# then dies during the settle window.
if ! kill -0 "$DAEMON_PID" >/dev/null 2>&1; then
  echo "FAIL: the daemon exited immediately after answering the health probe"
  echo "--- daemon log ---"
  sed 's/^/    /' "$DAEMON_LOG" 2>/dev/null || true
  exit 1
fi

sleep "$SETTLE_SECS"
if ! kill -0 "$DAEMON_PID" >/dev/null 2>&1; then
  echo "FAIL: the daemon exited within ${SETTLE_SECS}s of answering the health probe"
  echo "--- daemon log ---"
  sed 's/^/    /' "$DAEMON_LOG" 2>/dev/null || true
  exit 1
fi

# Shut it down and confirm it actually stopped.
#
# SIGTERM first, then poll for exit within the grace window. There is
# deliberately no bare `wait` on the daemon: `wait` blocks until the
# child exits, so a daemon that ignores SIGTERM would hang the script
# for as long as it lives and then be reported as a clean shutdown.
#
# The brace group with `2>/dev/null` swallows bash's "Terminated: 15"
# job-control notice, which the shell prints when it reports the
# signal death of a background child. That notice is expected here --
# the daemon is being asked to stop -- and it otherwise reads like a
# failure in the release log. The daemon's own output goes to
# DAEMON_LOG, not to this script's stderr, so nothing else is hidden.
{
  kill "$DAEMON_PID" >/dev/null 2>&1 || true
  stopped=0
  waited=0
  while [ "$waited" -lt "$STOP_GRACE_SECS" ]; do
    if ! kill -0 "$DAEMON_PID" >/dev/null 2>&1; then
      stopped=1
      break
    fi
    sleep 1
    waited=$((waited + 1))
  done
  wait "$DAEMON_PID" 2>/dev/null || true
} 2>/dev/null

if [ "$stopped" != "1" ]; then
  echo "FAIL: the daemon did not stop within ${STOP_GRACE_SECS}s of SIGTERM"
  echo "--- daemon log ---"
  sed 's/^/    /' "$DAEMON_LOG" 2>/dev/null || true
  kill -9 "$DAEMON_PID" >/dev/null 2>&1 || true
  wait "$DAEMON_PID" 2>/dev/null || true
  exit 1
fi

DAEMON_PID=""
echo "PASS: daemon stops cleanly on ${TARGET}"

echo "=== ALL ASSERTIONS PASS (${TARGET}) ==="
