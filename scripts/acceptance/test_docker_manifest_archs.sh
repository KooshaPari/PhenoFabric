#!/usr/bin/env bash
# Regression test for scripts/acceptance/docker_manifest_archs.sh.
#
# The manifest check exists to catch a multi-arch publish that SUCCEEDS
# but leaves an architecture out of the published manifest list. A test
# for it has to prove two things:
#
#   1. A manifest containing both architectures passes.
#   2. A manifest MISSING an architecture fails.
#
# (2) is the one that matters. A check that only ever passes is
# indistinguishable from no check at all.
#
# `docker buildx imagetools inspect` is stubbed so the test needs no
# registry, no network, and no Docker daemon.
#
# Usage: scripts/acceptance/test_docker_manifest_archs.sh
#
# Exit codes:
#   0 -- the script behaves correctly on every fixture
#   1 -- at least one case regressed
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="${SCRIPT_DIR}/docker_manifest_archs.sh"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

failed=0
total=0

# ── the stub ─────────────────────────────────────────────────────────
# Emulates `docker buildx imagetools inspect <ref> --format '{{json .Manifest}}'`
# by printing the fixture named by $STUB_MANIFEST. It also enforces the
# real flag rules of buildx, so a regression to the unsupported `-f`
# shorthand is caught here rather than in a release.
mkdir -p "$WORK/bin"
cat > "$WORK/bin/docker" << 'STUB_EOF'
#!/usr/bin/env bash
if [ "$1" = "buildx" ] && [ "$2" = "imagetools" ]; then
  # Emulate buildx's real flag handling: `imagetools inspect` accepts
  # `--format` but NOT the `-f` shorthand. Real buildx exits 125 with
  #     ERROR: unknown shorthand flag: 'f' in -f
  # The v0.1.7 release failed because the script used `-f` and hid the
  # error behind `2>/dev/null`. Rejecting `-f` here means a regression
  # back to the shorthand is caught by the test suite.
  shift 2
  # Record the argv so a test can assert which flag form was used.
  printf '%s\n' "$@" > "$STUB_ARGV_FILE"
  saw_format=0
  for arg in "$@"; do
    case "$arg" in
      -f|--format)
        saw_format=1
        ;;
      -f*)
        echo "ERROR: unknown shorthand flag: 'f' in $arg" >&2
        exit 125
        ;;
    esac
  done
  if [ "$saw_format" != "1" ]; then
    echo "ERROR: no --format flag given" >&2
    exit 125
  fi

  # A stubbed registry can be told to be "not ready yet" for the first
  # $STUB_UNREADABLE_CALLS invocations. That models the registry
  # propagation delay that can follow a just-completed push.
  #
  # The count lives in a FILE because the checker invokes `docker` as a
  # separate process on each retry; a shell variable would not survive.
  count=0
  if [ -f "$STUB_COUNT_FILE" ]; then
    count="$(cat "$STUB_COUNT_FILE")"
  fi
  if [ "$count" -lt "${STUB_UNREADABLE_CALLS:-0}" ]; then
    echo $((count + 1)) > "$STUB_COUNT_FILE"
    exit 1
  fi
  cat "$STUB_MANIFEST"
  exit 0
fi
exit 1
STUB_EOF
chmod +x "$WORK/bin/docker"
export PATH="$WORK/bin:$PATH"

# The stub counts invocations in a file, so the counter survives across
# separate `docker` invocations within one checker run.
export STUB_COUNT_FILE="$WORK/stub_count"
export STUB_ARGV_FILE="$WORK/stub_argv"

# ── fixtures ────────────────────────────────────────────────────────
cat > "$WORK/both.json" << 'EOF'
{"manifests":[
  {"platform":{"os":"linux","architecture":"amd64"},"digest":"sha256:a"},
  {"platform":{"os":"linux","architecture":"arm64"},"digest":"sha256:b"}
]}
EOF

cat > "$WORK/only_amd64.json" << 'EOF'
{"manifests":[
  {"platform":{"os":"linux","architecture":"amd64"},"digest":"sha256:a"}
]}
EOF

cat > "$WORK/only_arm64.json" << 'EOF'
{"manifests":[
  {"platform":{"os":"linux","architecture":"arm64"},"digest":"sha256:b"}
]}
EOF

# BuildKit's provenance/SBOM attestations carry the platform
# `unknown/unknown`. They are not images and must not be mistaken for
# an architecture.
cat > "$WORK/with_attestation.json" << 'EOF'
{"manifests":[
  {"platform":{"os":"unknown","architecture":"unknown"},"digest":"sha256:c"},
  {"platform":{"os":"linux","architecture":"amd64"},"digest":"sha256:a"},
  {"platform":{"os":"linux","architecture":"arm64"},"digest":"sha256:b"}
]}
EOF

# A manifest with no image entries at all.
cat > "$WORK/no_entries.json" << 'EOF'
{"manifests":[]}
EOF

# ── cases ───────────────────────────────────────────────────────────
# check <name> <fixture> <expected-rc> <expected-substring>
#
# The retry window is disabled for the ordinary cases so they stay fast
# and so a broken fixture fails immediately instead of sleeping.
check() {
  name="$1"; fixture="$2"; want_rc="$3"; want_text="$4"
  total=$((total + 1))
  rm -f "$STUB_COUNT_FILE"
  out="$(STUB_UNREADABLE_CALLS=0 \
         DOCKER_MANIFEST_READ_ATTEMPTS=1 \
         DOCKER_MANIFEST_READ_DELAY_SECONDS=0 \
         STUB_MANIFEST="$WORK/${fixture}" bash "$TARGET" "example.invalid/img:tag" 2>&1)"
  rc=$?
  ok=1
  if [ "$rc" != "$want_rc" ]; then
    ok=0
    echo "  [FAIL] ${name}: exit ${rc}, want ${want_rc}"
  elif ! printf '%s\n' "$out" | grep -q -- "$want_text"; then
    ok=0
    echo "  [FAIL] ${name}: output did not mention '${want_text}'"
    printf '%s\n' "$out" | sed 's/^/         /'
  else
    echo "  [ok  ] ${name}: exit ${rc}, mentioned '${want_text}'"
  fi
  if [ "$ok" != "1" ]; then
    failed=$((failed + 1))
  fi
}

# check_retry <name> <fixture> <unreadable_calls> <expected-rc> <expected-substring>
#
# Runs the checker with the retry window ENABLED and a stubbed registry
# that is unreadable for the first <unreadable_calls> invocations.
check_retry() {
  name="$1"; fixture="$2"; unreadable="$3"; want_rc="$4"; want_text="$5"
  total=$((total + 1))
  rm -f "$STUB_COUNT_FILE"
  out="$(STUB_UNREADABLE_CALLS="$unreadable" \
         DOCKER_MANIFEST_READ_ATTEMPTS=5 \
         DOCKER_MANIFEST_READ_DELAY_SECONDS=0 \
         STUB_MANIFEST="$WORK/${fixture}" bash "$TARGET" "example.invalid/img:tag" 2>&1)"
  rc=$?
  ok=1
  if [ "$rc" != "$want_rc" ]; then
    ok=0
    echo "  [FAIL] ${name}: exit ${rc}, want ${want_rc}"
  elif ! printf '%s\n' "$out" | grep -q -- "$want_text"; then
    ok=0
    echo "  [FAIL] ${name}: output did not mention '${want_text}'"
    printf '%s\n' "$out" | sed 's/^/         /'
  else
    echo "  [ok  ] ${name}: exit ${rc}, mentioned '${want_text}'"
  fi
  if [ "$ok" != "1" ]; then
    failed=$((failed + 1))
  fi
}

echo "=== docker_manifest_archs.sh regression test ==="

# The positive case: both architectures present must PASS.
check "both architectures present" both.json 0 "ALL ASSERTIONS PASS"

# The cases that matter: a missing architecture must FAIL.
check "amd64 only -> arm64 missing" only_amd64.json 1 "missing linux/arm64"
check "arm64 only -> amd64 missing" only_arm64.json 1 "missing linux/amd64"

# An attestation manifest must not be mistaken for an architecture.
check "attestation filtered out" with_attestation.json 0 "ALL ASSERTIONS PASS"

# A manifest with no image entries is a failure, not a vacuous pass.
check "no image entries" no_entries.json 1 "no real image entries"

# ── registry propagation delay ──────────────────────────────────────
# The v0.1.7 release was FAILED because the manifest was read 22ms after
# the push returned, before the registry had made the manifest list
# readable. The manifest was correct; the READ was too early.
#
# These cases prove the retry covers that, WITHOUT weakening the gate.

# The manifest becomes readable on the 3rd attempt: must PASS.
check_retry "propagation delay then readable" both.json 2 0 "ALL ASSERTIONS PASS"

# The registry NEVER becomes readable: must still FAIL after exhausting
# the retry window. A gate that retried forever would hang the release.
check_retry "never readable -> fail after retries" both.json 99 1 "could not read the manifest"

# A REAL defect must fail on the FIRST successful read, not be retried
# away. The manifest is readable but genuinely missing arm64: the retry
# window must not paper over that.
check_retry "missing arm64 fails even with retry" only_amd64.json 0 1 "missing linux/arm64"

# Same for the other direction.
check_retry "missing amd64 fails even with retry" only_arm64.json 0 1 "missing linux/amd64"

# ── the command form itself ────────────────────────────────────────
# The v0.1.7 release failed because the script invoked
#     docker buildx imagetools inspect <ref> -f '{{json .Manifest}}'
# and `imagetools inspect` has never accepted the `-f` shorthand. The
# stub rejects it exactly as real buildx does (exit 125, "unknown
# shorthand flag: 'f' in -f"), so a regression to the shorthand breaks
# every case above. This case additionally asserts the exact flag form
# that was used, so the requirement is visible rather than implied.
total=$((total + 1))
rm -f "$STUB_COUNT_FILE" "$STUB_ARGV_FILE"
stub_out="$(STUB_UNREADABLE_CALLS=0 \
             DOCKER_MANIFEST_READ_ATTEMPTS=1 \
             DOCKER_MANIFEST_READ_DELAY_SECONDS=0 \
             STUB_MANIFEST="$WORK/both.json" \
             bash "$TARGET" "example.invalid/img:tag" 2>&1)"
stub_rc=$?
stub_argv="$(cat "$STUB_ARGV_FILE" 2>/dev/null || true)"
if [ "$stub_rc" != "0" ]; then
  echo "  [FAIL] the checker failed against a healthy registry (rc=${stub_rc})"
  printf '%s\n' "$stub_out" | sed 's/^/         /'
  failed=$((failed + 1))
elif ! printf '%s\n' "$stub_argv" | grep -qx -- "--format"; then
  echo "  [FAIL] the checker did not pass --format to imagetools inspect"
  echo "         argv was: ${stub_argv}"
  failed=$((failed + 1))
elif printf '%s\n' "$stub_argv" | grep -qx -- "-f"; then
  echo "  [FAIL] the checker passed the unsupported -f shorthand"
  echo "         argv was: ${stub_argv}"
  failed=$((failed + 1))
else
  echo "  [ok  ] imagetools inspect is called with --format (not -f)"
fi

echo ""
passed=$((total - failed))
echo "${passed}/${total} cases passed"

if [ "$failed" != "0" ]; then
  echo "FAIL: ${failed} case(s) regressed" >&2
  exit 1
fi
exit 0
