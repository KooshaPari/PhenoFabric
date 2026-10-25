#!/usr/bin/env bash
# Regression test for scripts/release/resolve_version.sh.
#
# The resolver exists because every release job used to derive the
# version from `${GITHUB_REF#refs/tags/}`. That expression is correct
# ONLY for a tag push. On a workflow_dispatch run it yields the literal
# string "refs/heads/main", which would become:
#
#   - the artifact filename prefix ("phenotype-fabric-refs/heads/main-..."),
#   - the Docker image tag, and
#   - the name of the GitHub release that `gh release download` looks for.
#
# The v0.1.7 release needed to be re-run from a fixed commit, and the
# dispatch path could not even name the version it was republishing.
#
# So this test has to prove four things:
#
#   1. A tag-push run resolves the version from GITHUB_REF.
#   2. An explicit `version` input wins, and lets a dispatch name the
#      version it publishes.
#   3. A run that cannot determine a version FAILS. It never guesses.
#   4. A version whose tag does not exist in origin FAILS, and a
#      malformed / injected version FAILS.
#
# (3) and (4) are the ones that matter. A resolver that only ever
# succeeds is indistinguishable from no resolver at all.
#
# `git` is stubbed so the test needs no network and no remote.
#
# Usage: scripts/acceptance/test_resolve_version.sh
#
# Exit codes:
#   0 -- the script behaves correctly on every fixture
#   1 -- at least one case regressed
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="${SCRIPT_DIR}/../release/resolve_version.sh"

if [ ! -f "$TARGET" ]; then
  echo "FATAL: target script not found: $TARGET" >&2
  exit 1
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

failed=0
total=0

# ── the stub ─────────────────────────────────────────────────────────
# Stubs `git ls-remote` so the "does this tag exist" gate can be
# exercised without a network. The stub honours the SAME exit-code
# contract the real command uses:
#
#   0  -- the ref matched
#   2  -- no ref matched (the tag is genuinely absent)
#   128 -- the lookup itself failed (network, auth, bad remote)
#
# The distinction matters: the resolver must report a missing tag and a
# lookup failure differently, and must not treat one as the other.
mkdir -p "$WORK/bin"
cat > "$WORK/bin/git" << 'STUB_EOF'
#!/usr/bin/env bash
# Stubbed `git ls-remote --exit-code --tags origin refs/tags/<tag>`.
#
# STUB_TAGS      - space-separated list of tags that "exist"
# STUB_EXIT      - forced exit code for the lookup (simulates failure)
# STUB_CALLS     - file to append the invocation to, for assertions
if [ "${STUB_FORCE_EXIT:-}" != "" ]; then
  echo "git ls-remote $*" >> "${STUB_CALLS:-/dev/null}"
  exit "${STUB_FORCE_EXIT}"
fi

echo "git ls-remote $*" >> "${STUB_CALLS:-/dev/null}"

# Find the tag argument (the last argument matching refs/tags/*).
tag=""
for arg in "$@"; do
  case "$arg" in
    refs/tags/*) tag="${arg#refs/tags/}" ;;
  esac
done

for known in ${STUB_TAGS:-}; do
  if [ "$known" = "$tag" ]; then
    echo "deadbeef	refs/tags/${tag}"
    exit 0
  fi
done

# No ref matched: the real command exits 2 here.
exit 2
STUB_EOF
chmod +x "$WORK/bin/git"

# ── helpers ─────────────────────────────────────────────────────────
# Runs the resolver in a controlled environment and captures
# stdout/stderr plus the exit code.
#
# The resolver FAILS by calling `exit 1`. Because it is SOURCED, that
# `exit` terminates the shell that sourced it. So the resolver is sourced
# inside a nested command substitution: its `exit` ends only that
# substitution, and the substitution's own status becomes the resolver's
# exit code. Sourcing it in the SAME shell would abort the harness
# before it could report anything.
#
# Emits "<resolver output>\nEXIT=<code>". On success the output also
# carries "TAG=<tag> VERSION=<version>"; on failure the resolver exits
# before printing them, which is exactly the signal the assertions want.
run_resolver() {
  local ref="$1" input="$2" allow="$3" tags="$4" force_exit="$5"
  local out
  out=$(
    export PATH="${WORK}/bin:${PATH}"
    export GITHUB_REF="${ref}"
    export INPUT_VERSION="${input}"
    export RELEASE_REF_ALLOW_NON_TAG="${allow}"
    export STUB_TAGS="${tags}"
    export STUB_FORCE_EXIT="${force_exit}"
    export STUB_CALLS="${WORK}/calls"
    : > "${WORK}/calls"

    rc=0
    resolver_output=$(
      # shellcheck disable=SC1090
      source "$TARGET" 2>&1
      printf '\nTAG=%s VERSION=%s DOCKER_TAG=%s' \
        "${RELEASE_TAG:-}" "${RELEASE_VERSION:-}" "${RELEASE_DOCKER_TAG:-}"
    ) || rc=$?

    printf '%s' "$resolver_output"
    printf '\nEXIT=%s' "$rc"
  )
  printf '%s' "$out"
}

pass() {
  total=$((total + 1))
  echo "  PASS  $1"
}

fail() {
  total=$((total + 1))
  failed=$((failed + 1))
  echo "  FAIL  $1"
  if [ -n "${2:-}" ]; then
    printf '        %s\n' "$2"
  fi
}

assert_contains() {
  local haystack="$1" needle="$2" label="$3"
  case "$haystack" in
    *"$needle"*) pass "$label" ;;
    *) fail "$label" "expected to contain: ${needle}" ;;
  esac
}

assert_not_contains() {
  local haystack="$1" needle="$2" label="$3"
  case "$haystack" in
    *"$needle"*) fail "$label" "expected NOT to contain: ${needle}" ;;
    *) pass "$label" ;;
  esac
}

echo "Testing scripts/release/resolve_version.sh"
echo

# ── case 1: tag push resolves from GITHUB_REF ──────────────────────────
out=$(run_resolver "refs/tags/v0.1.7" "" "0" "v0.1.7" "")
assert_contains "$out" "TAG=v0.1.7" "tag push resolves the version from GITHUB_REF"
assert_contains "$out" "VERSION=0.1.7" "tag push derives the bare version"
assert_contains "$out" "EXIT=0" "tag push exits 0"

# ── case 1b: the Docker image tag KEEPS the leading "v" ────────────────
# The container image is published as
# ghcr.io/kooshapari/phenofabric:v0.1.7 -- WITH the "v". The Tauri
# bundle version is 0.1.7 -- WITHOUT it, because "v0.1.7" is not a
# valid Debian/RPM package version. These are different values and
# conflating them publishes the image under a tag nobody pulls.
out=$(run_resolver "refs/tags/v0.1.7" "" "0" "v0.1.7" "")
assert_contains "$out" "DOCKER_TAG=v0.1.7" "the Docker image tag keeps the leading v"
assert_not_contains "$out" "DOCKER_TAG=0.1.7" "the Docker image tag is not the bare semver"
assert_contains "$out" "VERSION=0.1.7" "the Tauri bundle version is the bare semver"

# ── case 2: explicit input wins over GITHUB_REF ───────────────────────
out=$(run_resolver "refs/heads/main" "v0.1.7" "0" "v0.1.7" "")
assert_contains "$out" "TAG=v0.1.7" "explicit version input wins on a dispatch"
assert_contains "$out" "EXIT=0" "dispatch with a version input exits 0"

# ── case 3: a dispatch with NO version input fails ─────────────────────
# This is the exact defect that blocked the v0.1.7 republish. It must
# NOT silently produce "refs/heads/main" as a version.
out=$(run_resolver "refs/heads/main" "" "0" "v0.1.7" "")
assert_contains "$out" "EXIT=1" "dispatch without a version input FAILS"
assert_not_contains "$out" "TAG=refs/heads/main" "dispatch without a version input never yields refs/heads/main as a version"
assert_contains "$out" "Cannot determine the release version" "dispatch without a version input explains why"

# ── case 4: a version whose tag does not exist FAILS ───────────────────
out=$(run_resolver "refs/heads/main" "v9.9.9" "0" "v0.1.7" "")
assert_contains "$out" "EXIT=1" "a version whose tag is absent FAILS"
assert_contains "$out" "does not exist in origin" "a version whose tag is absent explains why"

# ── case 5: a lookup FAILURE is reported differently from a missing tag ─
out=$(run_resolver "refs/heads/main" "v0.1.7" "0" "v0.1.7" "128")
assert_contains "$out" "EXIT=1" "a tag lookup failure FAILS"
assert_contains "$out" "Could not look up tag" "a tag lookup failure is not reported as a missing tag"
assert_not_contains "$out" "does not exist in origin" "a tag lookup failure does not claim the tag is absent"

# ── case 6: malformed and injected versions are rejected ───────────────
for bad in "../../etc/passwd" "v0.1.7;rm -rf /" "refs/heads/main" "v0.1.7 extra" "0.1.7" "v0.1" "vv0.1.7" "v0.1.7 && echo pwned"; do
  out=$(run_resolver "refs/heads/main" "$bad" "1" "v0.1.7" "")
  if printf '%s' "$out" | grep -q "EXIT=1"; then
    pass "rejects malformed version: ${bad}"
  else
    fail "rejects malformed version: ${bad}" "got: ${out}"
  fi
done

# ── case 7: RELEASE_REF_ALLOW_NON_TAG permits a brand-new version ──────
out=$(run_resolver "refs/heads/main" "v9.9.9" "1" "v0.1.7" "")
assert_contains "$out" "EXIT=0" "RELEASE_REF_ALLOW_NON_TAG=1 permits an untagged version"
assert_contains "$out" "TAG=v9.9.9" "RELEASE_REF_ALLOW_NON_TAG=1 preserves the requested version"

# ── case 8: the tag lookup actually happened ───────────────────────────
# A resolver that never checks the tag would pass case 2 by accident.
: > "${WORK}/calls"
run_resolver "refs/heads/main" "v0.1.7" "0" "v0.1.7" "" > /dev/null
if grep -q "ls-remote" "${WORK}/calls" 2>/dev/null; then
  pass "the resolver actually looks the tag up in origin"
else
  fail "the resolver actually looks the tag up in origin" "no ls-remote call was recorded"
fi

# ── case 9: sourcing the resolver does not change the caller's shell ───
# `set -euo pipefail` inside a SOURCED script would silently change the
# behaviour of every command the caller runs afterwards.
out=$(
  set +e
  export PATH="${WORK}/bin:${PATH}"
  export GITHUB_REF="refs/tags/v0.1.7"
  export INPUT_VERSION=""
  export STUB_TAGS="v0.1.7"
  before="$(set +o)"
  # shellcheck disable=SC1090
  source "$TARGET" > /dev/null 2>&1
  after="$(set +o)"
  if [ "$before" = "$after" ]; then echo "OPTS-PRESERVED"; else echo "OPTS-CHANGED"; fi
)
assert_contains "$out" "OPTS-PRESERVED" "sourcing the resolver preserves the caller's shell options"

# ── summary ───────────────────────────────────────────────────────────
echo
echo "─────────────────────────────────────────────"
if [ "$failed" -eq 0 ]; then
  echo "All $total resolver test cases passed."
  exit 0
fi
echo "$failed of $total resolver test cases FAILED."
exit 1
