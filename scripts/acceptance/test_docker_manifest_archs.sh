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
# Emulates `docker buildx imagetools inspect <ref> -f '{{json .Manifest}}'`
# by printing the fixture named by $STUB_MANIFEST.
mkdir -p "$WORK/bin"
cat > "$WORK/bin/docker" << 'STUB_EOF'
#!/usr/bin/env bash
if [ "$1" = "buildx" ] && [ "$2" = "imagetools" ]; then
  cat "$STUB_MANIFEST"
  exit 0
fi
exit 1
STUB_EOF
chmod +x "$WORK/bin/docker"
export PATH="$WORK/bin:$PATH"

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
check() {
  name="$1"; fixture="$2"; want_rc="$3"; want_text="$4"
  total=$((total + 1))
  out="$(STUB_MANIFEST="$WORK/${fixture}" bash "$TARGET" "example.invalid/img:tag" 2>&1)"
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

echo ""
passed=$((total - failed))
echo "${passed}/${total} cases passed"

if [ "$failed" != "0" ]; then
  echo "FAIL: ${failed} case(s) regressed" >&2
  exit 1
fi
exit 0
