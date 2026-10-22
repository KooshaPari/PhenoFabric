#!/usr/bin/env bash
# Negative tests for scripts/acceptance/binary_acceptance.sh.
#
# A release gate that cannot fail is not a gate. These tests deliberately
# break each thing the acceptance script is supposed to catch and assert
# that the script exits NON-ZERO every time.
#
# This is a self-test, not a product script: it lives outside
# scripts/acceptance/ because it is about the gate's own behaviour rather
# than about a release artifact.
#
# Usage: tests/acceptance/binary_acceptance_negative.sh

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ACCEPTANCE="${REPO_ROOT}/scripts/acceptance/binary_acceptance.sh"
REAL_BINS="${REPO_ROOT}/target/release"

if [ ! -x "$ACCEPTANCE" ]; then
  echo "FAIL: acceptance script not found or not executable: $ACCEPTANCE" >&2
  exit 2
fi

if [ ! -d "$REAL_BINS" ]; then
  echo "FAIL: release binaries not found: $REAL_BINS" >&2
  echo "Build them first: cargo build --release --bins" >&2
  exit 2
fi

EXPECTED="fabric fabric-daemon fabric-tui fabric-tray fabric-graph-cli"
TARGET="aarch64-apple-darwin"

PASSED=0
FAILED=0

# Stage a copy of the real binaries so a test can corrupt one of them
# without touching the build output.
stage_binaries() {
  local dest="$1"
  rm -rf "$dest"
  mkdir -p "$dest"
  local bin
  for bin in $EXPECTED; do
    cp "${REAL_BINS}/${bin}" "${dest}/${bin}" || return 1
    chmod +x "${dest}/${bin}"
  done
}

# Run the acceptance script and assert it fails.
expect_failure() {
  local name="$1" dir="$2" expected_pattern="$3"
  local output rc
  output="$("$ACCEPTANCE" "$TARGET" "$dir" 2>&1)"
  rc=$?
  if [ "$rc" -eq 0 ]; then
    echo "FAIL: ${name}: the acceptance script exited 0 but should have failed"
    printf '%s\n' "$output" | sed 's/^/    /'
    FAILED=$((FAILED + 1))
    return
  fi
  if [ -n "$expected_pattern" ] && ! printf '%s' "$output" | grep -qi -- "$expected_pattern"; then
    echo "FAIL: ${name}: the script failed but did not report '${expected_pattern}'"
    printf '%s\n' "$output" | sed 's/^/    /'
    FAILED=$((FAILED + 1))
    return
  fi
  echo "PASS: ${name}: exited ${rc} and reported the defect"
  printf '%s\n' "$output" | sed -n '1,4p' | sed 's/^/    /'
  PASSED=$((PASSED + 1))
}

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "=== negative tests for binary_acceptance.sh ==="

# ── Case 1: a binary that exits non-zero on --version ────────────────
stage_binaries "$WORK/case1"
cat > "$WORK/case1/fabric-tui" <<'EOF'
#!/bin/sh
if [ "$1" = "--version" ]; then
  echo "simulated startup failure" >&2
  exit 42
fi
exit 0
EOF
chmod +x "$WORK/case1/fabric-tui"
expect_failure "non-zero --version exit" "$WORK/case1" "exited"

# ── Case 2: a binary that prints nothing on --version ────────────────
stage_binaries "$WORK/case2"
cat > "$WORK/case2/fabric-graph-cli" <<'EOF'
#!/bin/sh
exit 0
EOF
chmod +x "$WORK/case2/fabric-graph-cli"
expect_failure "empty --version output" "$WORK/case2" "printed nothing"

# ── Case 3: a binary that hangs on --version ─────────────────────────
# This is the real `fabric-tray` failure mode: a GUI app that ignores the
# flag and enters its event loop. Without the watchdog in the acceptance
# script this case would wedge the runner until the job timeout.
stage_binaries "$WORK/case3"
cat > "$WORK/case3/fabric-tray" <<'EOF'
#!/bin/sh
if [ "$1" = "--version" ]; then
  sleep 600
fi
exit 0
EOF
chmod +x "$WORK/case3/fabric-tray"
expect_failure "hanging --version" "$WORK/case3" "did not exit"

# ── Case 4: a missing binary ─────────────────────────────────────────
stage_binaries "$WORK/case4"
rm -f "$WORK/case4/fabric-daemon"
expect_failure "missing binary" "$WORK/case4" "missing"

# ── Case 5: a daemon that never reports healthy ──────────────────────
stage_binaries "$WORK/case5"
cat > "$WORK/case5/fabric-daemon" <<'EOF'
#!/bin/sh
case "$1" in
  --version) echo "fabric-daemon 0.1.0"; exit 0 ;;
  start)    echo "simulated bind failure" >&2; exit 1 ;;
  health)   echo "unhealthy"; exit 1 ;;
esac
exit 0
EOF
chmod +x "$WORK/case5/fabric-daemon"
expect_failure "daemon never becomes healthy" "$WORK/case5" "did not report healthy"

# ── Case 6: a daemon that answers once and then exits ────────────────
# The acceptance script asserts the daemon is still alive after the
# health probe. A daemon that answers a single request and exits would
# pass a naive "is it healthy" check but is not a working daemon.
stage_binaries "$WORK/case6"
cat > "$WORK/case6/fabric-daemon" <<'EOF'
#!/bin/sh
case "$1" in
  --version) echo "fabric-daemon 0.1.0"; exit 0 ;;
  start)    sleep 2; exit 0 ;;
  health)   echo '{"status":"healthy"}'; exit 0 ;;
esac
exit 0
EOF
chmod +x "$WORK/case6/fabric-daemon"
expect_failure "daemon exits after answering" "$WORK/case6" "exited"

# ── Case 7: a daemon that ignores the shutdown signal ────────────────
stage_binaries "$WORK/case7"
cat > "$WORK/case7/fabric-daemon" <<'EOF'
#!/bin/sh
case "$1" in
  --version) echo "fabric-daemon 0.1.0"; exit 0 ;;
  start)    trap '' TERM; sleep 120 & wait ;;
  health)   echo '{"status":"healthy"}'; exit 0 ;;
esac
exit 0
EOF
chmod +x "$WORK/case7/fabric-daemon"
expect_failure "daemon ignores shutdown signal" "$WORK/case7" "did not stop"

# ── Case 8: the positive control ─────────────────────────────────────
# The gate must PASS on the untouched binaries. Without this, a script
# that fails on everything would satisfy every case above.
stage_binaries "$WORK/case8"
output="$("$ACCEPTANCE" "$TARGET" "$WORK/case8" 2>&1)"
rc=$?
if [ "$rc" -ne 0 ]; then
  echo "FAIL: positive control: the acceptance script failed on GOOD binaries (exit ${rc})"
  printf '%s\n' "$output" | sed 's/^/    /'
  FAILED=$((FAILED + 1))
else
  echo "PASS: positive control: the acceptance script passes on good binaries"
  PASSED=$((PASSED + 1))
fi

echo "=============================================="
echo "passed: ${PASSED}  failed: ${FAILED}"
if [ "$FAILED" != "0" ]; then
  echo "RESULT: FAIL -- the acceptance gate does not fail closed on every defect"
  exit 1
fi
echo "RESULT: PASS -- the acceptance gate fails closed on every defect"
