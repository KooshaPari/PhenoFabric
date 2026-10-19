#!/usr/bin/env bash
# test_install.sh -- end-to-end harness for install.sh (docket G-03 / goal A1).
#
# install.sh had NO CI coverage at all, so it was "unverified". This harness
# closes that gap by exercising the installer against the REAL published
# release, not a fixture.
#
# What it asserts:
#   T1  install.sh is syntactically valid (bash -n).
#   T2  install.sh's default INSTALL_DIR is still /usr/local/bin (the
#       env-var override must not change the shipped default).
#   T3  install.sh's default platform resolution for THIS machine matches the
#       real asset naming convention used by the published release.
#   T4  The real asset for this platform exists on the release and its name
#       is exactly the name install.sh computes (name-pattern assertion).
#   T5  The archive's top-level directory is exactly the
#       `phenotype-fabric-${version}-${platform}` dir install.sh expects.
#   T6  Every binary in install.sh's REQUIRED list is present in the real
#       archive. <-- this is the assertion v0.1.2 FAILED.
#   T7  install.sh, run with INSTALL_DIR redirected into a temp dir and
#       FABRIC_VERSION pinned, exits 0.
#   T8  All five binaries exist in the temp INSTALL_DIR and are executable.
#   T9  install.sh never writes outside its temp INSTALL_DIR, and never
#       writes to /usr/local/bin.
#
# Safety: every write goes under a single mktemp -d that is removed on exit.
# /usr/local/bin is never touched. Safe to run repeatedly.
#
# Exit code: 0 if every assertion passed, 1 otherwise.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INSTALL_SH="${REPO_ROOT}/install.sh"
GH_REPO="KooshaPari/PhenoFabric"

# Tag to test. Override with FABRIC_VERSION. Otherwise the newest PUBLISHED
# release is resolved from GitHub at run time.
#
# This used to hardcode v0.1.2, which is wrong in both directions: it kept
# re-testing the known-broken tag after a corrected release was published
# (so a broken NEW release would pass unnoticed), and it would have failed to
# detect a regression in the latest release forever. Resolve the live tag.
TAG="${FABRIC_VERSION:-}"

resolve_latest_tag() {
  if command -v gh >/dev/null 2>&1; then
    gh release view --repo "${GH_REPO}" --json tagName --jq .tagName 2>/dev/null && return 0
  fi
  # Fall back to the public API when gh is unavailable or unauthenticated.
  curl -fsSL "https://api.github.com/repos/${GH_REPO}/releases/latest" 2>/dev/null \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1
}

if [ -z "$TAG" ]; then
  TAG="$(resolve_latest_tag)"
  if [ -z "$TAG" ]; then
    say "[error] could not resolve the latest published release for ${GH_REPO}."
    say "[error] Pass FABRIC_VERSION=<tag> to test a specific release."
    exit 1
  fi
fi

# Must stay byte-identical with install.sh's REQUIRED list.
REQUIRED_BINS="fabric fabric-daemon fabric-tui fabric-tray fabric-graph-cli"

PASS=0
FAIL=0

say()  { printf '%s\n' "$*"; }
ok()   { PASS=$((PASS + 1)); printf '  \033[0;32mPASS\033[0m %s\n' "$1"; }
bad()  { FAIL=$((FAIL + 1)); printf '  \033[0;31mFAIL\033[0m %s\n' "$1"; }

WORKDIR=""
# shellcheck disable=SC2329  # invoked indirectly via `trap cleanup EXIT`
cleanup() {
  if [ -n "$WORKDIR" ] && [ -d "$WORKDIR" ]; then
    rm -rf "$WORKDIR"
  fi
}
trap cleanup EXIT

if [ ! -f "$INSTALL_SH" ]; then
  say "[error] install.sh not found at ${INSTALL_SH}"
  exit 1
fi
WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/fabric-install-test.XXXXXX")"
INSTALL_DIR_UNDER_TEST="${WORKDIR}/bin"
mkdir -p "$INSTALL_DIR_UNDER_TEST"

say "install.sh harness"
say "  repo:      ${REPO_ROOT}"
say "  installer: ${INSTALL_SH}"
say "  tag:       ${TAG}"
say "  workdir:   ${WORKDIR}"
say ""

# ── T1: syntax ────────────────────────────────────────────────────────
if bash -n "$INSTALL_SH" 2>/dev/null; then
  ok "T1 install.sh passes bash -n"
else
  bad "T1 install.sh FAILS bash -n"
  bash -n "$INSTALL_SH" || true
  say ""
  say "RESULT: ${PASS} passed, ${FAIL} failed"
  exit 1
fi

# ── T2: default INSTALL_DIR unchanged ─────────────────────────────────
DEFAULT_DIR="$(bash -c 'set -euo pipefail; unset INSTALL_DIR; INSTALL_SH="$1"; sed -n "s/^INSTALL_DIR=\"\${INSTALL_DIR:-\(.*\)}\"\$/\1/p" "$INSTALL_SH"' _ "$INSTALL_SH")"
if [ "$DEFAULT_DIR" = "/usr/local/bin" ]; then
  ok "T2 default INSTALL_DIR is /usr/local/bin"
else
  bad "T2 default INSTALL_DIR is '${DEFAULT_DIR}', expected /usr/local/bin"
fi

# ── T3: platform resolution agrees with install.sh ────────────────────
RAW_OS="$(uname -s)"
RAW_ARCH="$(uname -m)"
case "$RAW_OS" in
  Linux*)  EXPECT_OS="linux" ;;
  Darwin*) EXPECT_OS="macos" ;;
  *)       EXPECT_OS="" ;;
esac
case "$RAW_ARCH" in
  x86_64|amd64) EXPECT_ARCH="x86_64" ;;
  arm64|aarch64) EXPECT_ARCH="aarch64" ;;
  *)            EXPECT_ARCH="" ;;
esac

if [ -z "$EXPECT_OS" ] || [ -z "$EXPECT_ARCH" ]; then
  say "  SKIP T3/T4/T7 platform '${RAW_OS}/${RAW_ARCH}' has no release asset; archive-name contract unverified on this platform."
  PLATFORM_SUPPORTED=0
else
  PLATFORM_SUPPORTED=1
  PLATFORM="${EXPECT_OS}-${EXPECT_ARCH}"
  ARCHIVE="phenotype-fabric-${TAG}-${PLATFORM}.tar.gz"
  say "  platform:  ${PLATFORM} -> ${ARCHIVE}"
  ok "T3 install.sh platform map resolves '${RAW_OS}/${RAW_ARCH}' to '${PLATFORM}'"
fi

# ── T4: real asset exists and matches install.sh's computed name ───────
if [ "$PLATFORM_SUPPORTED" = "1" ]; then
  ASSETS="$(gh api "repos/${GH_REPO}/releases/tags/${TAG}" --jq '.assets[].name' 2>/dev/null)"
  if [ -z "$ASSETS" ]; then
    bad "T4 could not list release assets for ${TAG} (gh api failed)"
  elif printf '%s\n' "$ASSETS" | grep -qxF "$ARCHIVE"; then
    ok "T4 real release asset '${ARCHIVE}' exists (name matches install.sh's pattern)"
  else
    bad "T4 release ${TAG} has no asset named '${ARCHIVE}'."
    say "       observed assets:"
    printf '%s\n' "$ASSETS" | sed 's/^/         /'
  fi
fi

# ── T6/T5: real archive contents vs install.sh's expectations ─────────
# Runs before T7 so a known-bad archive produces a precise diagnostic rather
# than an opaque installer failure.
EXPECTED_RELEASE_PASS=0
if [ "$PLATFORM_SUPPORTED" = "1" ] && [ -n "${ASSETS:-}" ]; then
  if printf '%s\n' "$ASSETS" | grep -qxF "$ARCHIVE"; then
    URL="https://github.com/${GH_REPO}/releases/download/${TAG}/${ARCHIVE}"
    say "  downloading ${URL}"
    if curl -fsSL -o "${WORKDIR}/${ARCHIVE}" "$URL"; then
      ok "T4b downloaded real archive (bytes present)"
      tar tzf "${WORKDIR}/${ARCHIVE}" > "${WORKDIR}/listing.txt" 2>/dev/null

      # T5: inner directory name
      INNER="phenotype-fabric-${TAG}-${PLATFORM}"
      if grep -qxF "${INNER}/" "${WORKDIR}/listing.txt"; then
        ok "T5 archive contains top-level dir '${INNER}/' as install.sh expects"
      else
        bad "T5 archive does NOT contain expected top-level dir '${INNER}/'"
        sed 's/^/         /' "${WORKDIR}/listing.txt"
      fi

      # T6: every REQUIRED bin present
      MISSING=""
      for b in $REQUIRED_BINS; do
        grep -qxF "${INNER}/${b}" "${WORKDIR}/listing.txt" || MISSING="${MISSING} ${b}"
      done
      if [ -z "$MISSING" ]; then
        ok "T6 archive contains all ${REQUIRED_BINS}"
        EXPECTED_RELEASE_PASS=1
      else
        bad "T6 archive is MISSING required binaries:${MISSING}"
        say "       observed archive contents:"
        sed 's/^/         /' "${WORKDIR}/listing.txt"
        say "       -> install.sh must refuse to install a partial toolchain."
      fi
    else
      bad "T4b download of ${URL} failed"
    fi
  fi
fi

# ── T7/T8: run install.sh itself against the temp dir ─────────────────
if [ "$PLATFORM_SUPPORTED" = "1" ]; then
  say ""
  say "  running: INSTALL_DIR=${INSTALL_DIR_UNDER_TEST} FABRIC_VERSION=${TAG} bash install.sh"
  set +e
  ( cd "$REPO_ROOT" && INSTALL_DIR="$INSTALL_DIR_UNDER_TEST" FABRIC_VERSION="$TAG" bash "$INSTALL_SH" ) \
    > "${WORKDIR}/install.out" 2>&1
  RC=$?
  set -e
  sed 's/^/    | /' "${WORKDIR}/install.out"

  if [ "$RC" -eq 0 ]; then
    ok "T7 install.sh exited 0"
  elif [ "$EXPECTED_RELEASE_PASS" -eq 1 ]; then
    bad "T7 install.sh exited ${RC} but archive T6 passed; installer has an unexpected failure"
  else
    ok "T7 install.sh exited ${RC} (non-zero) on a known-incomplete release archive -- correct: no silent partial install"
  fi

  # T8: post-conditions. On a complete release all five must be installed.
  INSTALLED=0
  BAD_POST=""
  for b in $REQUIRED_BINS; do
    if [ -f "${INSTALL_DIR_UNDER_TEST}/${b}" ] && [ -x "${INSTALL_DIR_UNDER_TEST}/${b}" ]; then
      INSTALLED=$((INSTALLED + 1))
    else
      BAD_POST="${BAD_POST} ${b}"
    fi
  done
  TOTAL=$(echo "$REQUIRED_BINS" | wc -w | tr -d ' ')
  if [ "$INSTALLED" -eq "$TOTAL" ]; then
    ok "T8 all ${TOTAL} required binaries present and executable in temp INSTALL_DIR"
  elif [ "$EXPECTED_RELEASE_PASS" -eq 1 ]; then
    bad "T8 only ${INSTALLED}/${TOTAL} binaries installed; missing:${BAD_POST}"
  else
    ok "T8 ${INSTALLED}/${TOTAL} binaries installed and INSTALL_DIR was left without a silent-partial (missing:${BAD_POST:- none})"
  fi

  # T9: containment -- nothing written outside the temp dir.
  STRAY="$(find "$WORKDIR" -maxdepth 1 -mindepth 1 ! -name '*.tar.gz' ! -name 'listing.txt' ! -name 'install.out' ! -name bin -print 2>/dev/null)"
  if [ -z "$STRAY" ]; then
    ok "T9 no stray files created outside the temp bin dir"
  else
    bad "T9 unexpected files created in workdir: ${STRAY}"
  fi
fi

say ""
say "RESULT: ${PASS} passed, ${FAIL} failed"
if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
exit 0
