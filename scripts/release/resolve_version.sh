#!/usr/bin/env bash
# Resolve the release version for a workflow run.
#
# WHY THIS EXISTS
# --------------
# Every release job previously derived the version from one expression:
#
#     tag=${GITHUB_REF#refs/tags/}
#
# That expression is only correct when the run was triggered by a tag
# push. On a `workflow_dispatch` run, `GITHUB_REF` is `refs/heads/<branch>`
# (or `refs/heads/main`), so `${GITHUB_REF#refs/tags/}` does NOT strip
# anything and yields the literal string "refs/heads/main". A dispatched
# release would then publish assets named
# "phenotype-fabric-refs/heads/main-linux-x86_64.tar.gz" and try to
# create a release for a tag named "refs/heads/main".
#
# That is not a hypothetical: the v0.1.7 release run (39724488642) built
# and pushed every artifact correctly, then failed in the manifest
# verification step. To recover, the release had to be re-run -- but the
# only source of the broken code was the tag itself, so the fix could
# never be reached from a tag push. A dispatch was the only way to build
# from the fixed commit, and dispatch had no way to name the version.
#
# THIS SCRIPT IS THE SINGLE SOURCE OF TRUTH for the version, so the
# "Extract version from tag" step in every job cannot drift from the
# others. It resolves in this order:
#
#   1. An explicit `version` workflow input, if non-empty.
#   2. `GITHUB_REF` when it is a tag ref.
#   3. Otherwise it FAILS (fail-closed). It never guesses.
#
# USAGE
# -----
#   source scripts/release/resolve_version.sh
#   echo "$RELEASE_TAG"     # e.g. v0.1.7
#   echo "$RELEASE_VERSION" # e.g. 0.1.7
#
# Environment:
#   GITHUB_REF        - the ref the workflow was triggered on (required)
#   INPUT_VERSION     - the `version` workflow_dispatch input, if any
#   RELEASE_REF_ALLOW_NON_TAG - set to "1" only to allow a version input
#                      that names a tag which does not yet exist locally.
#                      The default ("0") requires the tag to exist, which
#                      is what makes the republish path safe: it proves the
#                      version names a real, already-published tag rather
#                      than an arbitrary string.

# This script is SOURCED, not executed, so it must not leak shell options
# into the caller's shell. `set -euo pipefail` here would change the
# behaviour of every command the caller runs afterwards -- including
# commands that legitimately return non-zero. Save and restore instead.
__resolve_version_shell_opts="$(set +o)"
set -euo pipefail

# ── Resolve the version ───────────────────────────────────────────────────
RESOLVED_VERSION="${INPUT_VERSION:-}"

if [ -z "$RESOLVED_VERSION" ]; then
  case "${GITHUB_REF:-}" in
    refs/tags/*)
      RESOLVED_VERSION="${GITHUB_REF#refs/tags/}"
      ;;
    *)
      echo "::error::Cannot determine the release version." >&2
      echo "::error::GITHUB_REF is '${GITHUB_REF:-<unset>}', which is not a tag ref." >&2
      echo "::error::Run this workflow from a tag push, or pass the" >&2
      echo "::error::'version' workflow_dispatch input (e.g. version=v0.1.7)." >&2
      exit 1
      ;;
  esac
fi

# ── Validate the shape ────────────────────────────────────────────────────
# A version must look like "vMAJOR.MINOR.PATCH" with an optional
# pre-release/build suffix. This rejects path traversal ("../.."),
# shell metacharacters, and empty/garbage values before they can reach
# an artifact filename, a Docker tag, or a `gh release` invocation.
if ! printf '%s' "$RESOLVED_VERSION" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.-]+)?$'; then
  echo "::error::Invalid release version: '${RESOLVED_VERSION}'" >&2
  echo "::error::Expected a form like 'v0.1.7'." >&2
  exit 1
fi

RELEASE_TAG="$RESOLVED_VERSION"
RELEASE_VERSION="${RELEASE_TAG#v}"

# ── Prove the tag exists ──────────────────────────────────────────────────
# A republish must name a version that is REALLY released. If the tag
# does not exist in the repository, this workflow would create a GitHub
# release for a version nothing else references -- and the artifacts
# would be built from a commit that is not that version.
#
# The tag is looked up over the network via `git ls-remote` rather than
# `git rev-parse` because the runner's checkout is shallow (depth 1) and
# does not fetch tags by default, so a local lookup would fail for
# reasons unrelated to whether the tag exists.
#
# `git ls-remote --exit-code` exits 2 when NO ref matched (the tag is
# genuinely absent) and 128 on a lookup error (network, auth, bad remote).
# Those are different situations and must not be conflated: treating a
# transient network failure as "tag absent" would report a misleading
# error, and treating a genuinely absent tag as a lookup error would hide
# the real problem. Both fail closed either way, but they fail with
# different messages.
if [ "${RELEASE_REF_ALLOW_NON_TAG:-0}" != "1" ]; then
  ls_remote_status=0
  git ls-remote --exit-code --tags origin "refs/tags/${RELEASE_TAG}" >/dev/null 2>&1 || ls_remote_status=$?

  if [ "$ls_remote_status" -ne 0 ]; then
    if [ "$ls_remote_status" -eq 2 ]; then
      echo "::error::Tag '${RELEASE_TAG}' does not exist in origin." >&2
      echo "::error::Refusing to publish a release for a version that is not tagged." >&2
    else
      echo "::error::Could not look up tag '${RELEASE_TAG}' in origin" >&2
      echo "::error::(git ls-remote exited ${ls_remote_status}; this is a lookup" >&2
      echo "::error::failure, not a missing tag -- check network and remote access)." >&2
    fi
    echo "::error::Create the tag first, or set RELEASE_REF_ALLOW_NON_TAG=1" >&2
    echo "::error::to publish a brand-new version." >&2
    exit 1
  fi
  echo "Resolved tag '${RELEASE_TAG}' exists in origin."
fi

# Export so a caller can rely on these being set even if it did not
# assign them itself.
export RELEASE_TAG RELEASE_VERSION

# Restore the caller's shell options.
eval "$__resolve_version_shell_opts"
unset __resolve_version_shell_opts
