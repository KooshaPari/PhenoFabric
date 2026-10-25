#!/usr/bin/env bash
# Regression test: release sidecars must name the file, not a path.
#
# GNU coreutils' `sha256sum` escapes a filename that contains a
# backslash by prefixing the WHOLE OUTPUT LINE with a backslash:
#
#     sha256sum 'D:\a\_temp\gui\phenotype-fabric-gui-windows-x64.msi'
#     \a9179aaa...  phenotype-fabric-gui-windows-x64.msi
#                     ^
#                     the leading backslash lands on the DIGEST field
#
# The GUI job staged its installers under "${RUNNER_TEMP}/gui-artifacts",
# which on the Windows runner is "D:\a\_temp\gui-artifacts" -- an
# absolute path full of backslashes. So the sidecar it published had a
# backslash-prefixed digest, and every comparison against the real bytes
# failed. The v0.1.7 republish run 40243421768 died there, AFTER the
# Docker manifest push and the required-asset gate had both succeeded.
#
# A user running `sha256sum -c phenotype-fabric-gui-windows-x64.msi.sha256`
# would have been told their download was corrupt -- when it was fine,
# and when the real digest was sitting in the file one character away.
#
# So the sidecar MUST be generated from inside the artifact directory
# with a RELATIVE name. This test proves that, by extracting the real
# `run:` block out of release.yml and running it against a fixture whose
# directory path contains a backslash -- exactly the Windows situation.
#
# Usage: scripts/acceptance/test_release_sidecar_paths.sh
#
# Exit codes:
#   0 -- the sidecar generation behaves correctly
#   1 -- at least one case regressed
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
WORKFLOW="${REPO_ROOT}/.github/workflows/release.yml"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

failed=0
total=0

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

echo "Testing release sidecar generation (release.yml: Generate installer checksums)"
echo

# ── extract the real `run:` block from the workflow ─────────────────
# The test runs the REAL script, not a copy of it. If the workflow
# changes, this test changes with it.
if [ ! -f "$WORKFLOW" ]; then
  echo "FATAL: workflow not found: $WORKFLOW" >&2
  exit 1
fi

python3 - "$WORKFLOW" > "$WORK/sidecar_step.sh" << 'PY'
import sys
import yaml

workflow = yaml.safe_load(open(sys.argv[1]))
jobs = workflow["jobs"]
found = None
for job in jobs.values():
    for step in job.get("steps", []) or []:
        name = step.get("name", "")
        if name == "Generate installer checksums":
            found = step.get("run", "")
            break
    if found:
        break

if found is None:
    sys.stderr.write("could not find the 'Generate installer checksums' step\n")
    sys.exit(1)

# The step reads the staged directory from the job env. Substitute the
# placeholder so the extracted script can be run standalone.
sys.stdout.write(found.replace("${{ env.gui_artifacts_dir }}", '"$STAGE"'))
PY
extract_rc=$?
if [ "$extract_rc" -ne 0 ] || [ ! -s "$WORK/sidecar_step.sh" ]; then
  echo "FATAL: could not extract the 'Generate installer checksums' step" >&2
  exit 1
fi

echo "  (extracted the real step from release.yml)"
echo

# ── the fixture ─────────────────────────────────────────────────────
# The directory name contains a backslash, reproducing the Windows
# runner's absolute path. On Linux a backslash is a legal filename
# character, so this is exactly the situation GNU coreutils escapes.
STAGE="${WORK}/win\\dir\\gui-artifacts"
mkdir -p "$STAGE"
printf 'fake msi bytes' > "${STAGE}/phenotype-fabric-gui-windows-x64.msi"
printf 'fake exe bytes' > "${STAGE}/phenotype-fabric-gui-windows-x64.exe"

export STAGE
# The step also writes to $GITHUB_ENV (the checksum_dir marker). Give it
# a real file so the extracted step runs unmodified.
export GITHUB_ENV="${WORK}/github_env"
export GITHUB_OUTPUT="${WORK}/github_output"
: > "$GITHUB_ENV"
: > "$GITHUB_OUTPUT"

# ── run the real step ───────────────────────────────────────────────
( bash "$WORK/sidecar_step.sh" ) > "$WORK/step.log" 2>&1
step_rc=$?

if [ "$step_rc" -ne 0 ]; then
  fail "the sidecar step succeeds against a backslash path" "exit=${step_rc}; log:"
  sed 's/^/        /' "$WORK/step.log" | head -10
else
  pass "the sidecar step succeeds against a backslash path"
fi

# ── case 1: a sidecar exists for every installer ───────────────────
for name in phenotype-fabric-gui-windows-x64.msi phenotype-fabric-gui-windows-x64.exe; do
  if [ -f "${STAGE}/${name}.sha256" ]; then
    pass "sidecar exists for ${name}"
  else
    fail "sidecar exists for ${name}" "not found"
  fi
done

# ── case 2: the sidecar's digest field is a bare 64-hex digest ─────
# This is the bug. The escaped form put a leading backslash in the
# digest field, so the digest could never equal the real one.
for name in phenotype-fabric-gui-windows-x64.msi phenotype-fabric-gui-windows-x64.exe; do
  sidecar="${STAGE}/${name}.sha256"
  if [ ! -f "$sidecar" ]; then
    continue
  fi
  sum="$(cut -d' ' -f1 < "$sidecar")"
  if printf '%s' "$sum" | grep -Eq '^[0-9a-f]{64}$'; then
    pass "sidecar digest for ${name} is a bare 64-hex digest"
  else
    fail "sidecar digest for ${name} is a bare 64-hex digest" "got: ${sum}"
  fi
done

# ── case 3: the sidecar names the BARE filename ─────────────────────
# A sidecar naming "D:\a\_temp\gui\file.msi" is useless to a user who
# downloaded the file to their own directory.
for name in phenotype-fabric-gui-windows-x64.msi phenotype-fabric-gui-windows-x64.exe; do
  sidecar="${STAGE}/${name}.sha256"
  if [ ! -f "$sidecar" ]; then
    continue
  fi
  named="$(awk '{print $2}' "$sidecar" | head -1)"
  if [ "$named" = "$name" ]; then
    pass "sidecar for ${name} names the bare filename"
  else
    fail "sidecar for ${name} names the bare filename" "got: ${named}"
  fi
done

# ── case 4: the digest matches the real bytes ──────────────────────
for name in phenotype-fabric-gui-windows-x64.msi phenotype-fabric-gui-windows-x64.exe; do
  sidecar="${STAGE}/${name}.sha256"
  if [ ! -f "$sidecar" ]; then
    continue
  fi
  actual="$(sha256sum "${STAGE}/${name}" | cut -d' ' -f1)"
  said="$(cut -d' ' -f1 < "$sidecar")"
  if [ "$actual" = "$said" ]; then
    pass "sidecar digest matches the real bytes for ${name}"
  else
    fail "sidecar digest matches the real bytes for ${name}" "actual=${actual} sidecar=${said}"
  fi
done

# ── case 5: 'sha256sum -c' accepts it (what a user runs) ───────────
# This is the user-facing contract: the published sidecar must verify
# with the standard tool, from the directory the file was downloaded to.
cp "${STAGE}/phenotype-fabric-gui-windows-x64.msi" "$WORK/userdir-check.msi" 2>/dev/null || true
mkdir -p "$WORK/userdir"
cp "${STAGE}/phenotype-fabric-gui-windows-x64.msi" "$WORK/userdir/phenotype-fabric-gui-windows-x64.msi"
cp "${STAGE}/phenotype-fabric-gui-windows-x64.msi.sha256" "$WORK/userdir/phenotype-fabric-gui-windows-x64.msi.sha256"
if ( cd "$WORK/userdir" && sha256sum -c phenotype-fabric-gui-windows-x64.msi.sha256 >/dev/null 2>&1 ); then
  pass "sha256sum -c verifies the published sidecar"
else
  fail "sha256sum -c verifies the published sidecar" "verification failed"
fi

# ── case 6: the sidecar does NOT contain a backslash-escaped digest ─
# The literal regression: no line in any sidecar may begin with "\".
for sidecar in "${STAGE}"/*.sha256; do
  [ -f "$sidecar" ] || continue
  base="$(basename "$sidecar")"
  if grep -q '^\\' "$sidecar" 2>/dev/null; then
    fail "no sidecar contains a backslash-escaped line" "${base} starts with a backslash"
  else
    pass "no sidecar contains a backslash-escaped line (${base})"
  fi
done

# ── case 7: the FIX is what protects against GNU's escaping ───────
#
# GNU coreutils' `sha256sum` prefixes the whole output line with "\"
# when the filename contains a backslash. The Windows runner uses GNU
# coreutils (Git Bash), and $STAGE there is an absolute path with
# backslashes, so the sidecar's DIGEST field was backslash-prefixed and
# never matched the real bytes.
#
# macOS ships a BSD `sha256sum` that does NOT implement that escaping,
# so the bug does not reproduce here -- which is exactly why it survived
# until a real Windows release run. This case therefore asserts the
# PROPERTY that makes the fix work, and it does so by checking the
# sidecar's filename field is RELATIVE: a relative name cannot contain
# a backslash, so GNU has nothing to escape and no "\" can appear.
#
# It also runs the digest through GNU coreutils when one is available,
# so the escaping behaviour itself is exercised on this host.
for sidecar in "${STAGE}"/*.sha256; do
  [ -f "$sidecar" ] || continue
  base="$(basename "$sidecar")"
  named="$(awk '{print $2}' "$sidecar" | head -1)"
  case "$named" in
    /*|*"\\"*)
      fail "sidecar filename field is relative and backslash-free (${base})" "got: ${named}"
      ;;
    *)
      pass "sidecar filename field is relative and backslash-free (${base})"
      ;;
  esac
done

# If GNU coreutils is available, prove the escaping directly: the same
# bytes checksummed under a backslash-containing path produce an
# escaped line, while the relative form does not.
GNU_SUM=""
for cand in gsha256sum /opt/homebrew/opt/coreutils/libexec/gnubin/sha256sum; do
  if command -v "$cand" >/dev/null 2>&1; then
    GNU_SUM="$cand"
    break
  fi
done

if [ -n "$GNU_SUM" ]; then
  # Run inside a directory that actually holds a backslash-named file,
  # so the escaping behaviour is exercised for real.
  mkdir -p "${WORK}/gnucheck"
  printf 'bytes' > "${WORK}/gnucheck/file.msi"
  mkdir -p "${WORK}/gnucheck/win\\dir\\gui"
  printf 'bytes' > "${WORK}/gnucheck/win\\dir\\gui\\file.msi"

  # The escaped form: the same bytes under a backslash-containing path.
  escaped="$(cd "${WORK}/gnucheck" && "$GNU_SUM" 'win\dir\gui\file.msi' 2>/dev/null | head -1)"
  case "$escaped" in
    \\*)
      pass "GNU sha256sum escapes a backslash path (bug reproduced locally)"
      ;;
    *)
      fail "GNU sha256sum escapes a backslash path (bug reproduced locally)" "got: ${escaped}"
      ;;
  esac
  # The relative form the fix uses.
  rel="$(cd "${WORK}/gnucheck" && "$GNU_SUM" 'file.msi' 2>/dev/null | head -1)"
  case "$rel" in
    \\*)
      fail "GNU sha256sum does not escape a relative name" "got: ${rel}"
      ;;
    *)
      pass "GNU sha256sum does not escape a relative name"
      ;;
  esac
else
  echo "  note  GNU coreutils sha256sum not available on this host;"
  echo "        the escaping itself is asserted by the relative-name property above"
fi

echo
echo "─────────────────────────────────────────────"
if [ "$failed" -eq 0 ]; then
  echo "All $total sidecar-path test cases passed."
  exit 0
fi
echo "$failed of $total sidecar-path test cases FAILED."
exit 1
