#!/usr/bin/env bash
# Verify that a published Docker manifest list really contains the
# architectures the release body claims it contains.
#
# Usage: scripts/acceptance/docker_manifest_archs.sh <image-ref>
#
# Why this exists
# ---------------
# The multi-arch Docker publish step can succeed while producing a
# manifest list that is missing an architecture. `docker/build-push-action`
# with `platforms: linux/amd64,linux/arm64` reports success when it has
# pushed SOMETHING; if the arm64 half failed to build under QEMU, or if
# the platform list was silently truncated, the registry ends up holding
# a single-arch image under a multi-arch tag. The release body would
# then advertise a linux/arm64 image that does not exist.
#
# The same class of bug already bit this pipeline twice: v0.1.4 shipped
# a release body promising a Docker image that was never published, and
# a `download-artifact` pattern matched nothing while the job exited 0.
# A build step that succeeds is not evidence that the thing it claims to
# have produced exists. This script is that evidence.
#
# Exit codes:
#   0 -- the manifest list contains every required architecture
#   1 -- an architecture is missing, or the manifest could not be read
#   2 -- bad usage
set -uo pipefail

IMAGE="${1:-}"
REQUIRED_ARCHS="${2:-linux/amd64 linux/arm64}"

# How long to keep retrying an unreadable manifest.
#
# A container registry does not always make a just-pushed manifest list
# readable at the instant the push returns. The publish step finished at
# 06:03:49.474 and the verification step first read the registry at
# 06:03:47.228 -- within a second of the push -- and the manifest was
# not yet visible.
#
# That was the fail-closed gate working as designed: it refused to let
# an unverified image through. But a false negative must not be mistaken
# for proof of a defect, so the read is retried within a bounded window.
#
# So the check must distinguish two very different failures:
#
#   * "the registry cannot show me this manifest yet" -- a TRANSIENT
#     condition that resolves on its own, and
#   * "the manifest exists and is missing an architecture" -- a REAL
#     defect that must fail the release.
#
# Only the second kind may fail immediately. The first kind gets a
# bounded retry window. This does not weaken the gate: a manifest that
# genuinely lacks an architecture still fails on the FIRST successful
# read, and a manifest that never becomes readable still fails when the
# window expires.
#
# Set DOCKER_MANIFEST_READ_ATTEMPTS=1 to disable the retry entirely
# (used by the regression test, where the registry is stubbed).
READ_ATTEMPTS="${DOCKER_MANIFEST_READ_ATTEMPTS:-30}"
READ_DELAY_SECONDS="${DOCKER_MANIFEST_READ_DELAY_SECONDS:-10}"

if [ -z "$IMAGE" ]; then
  echo "usage: $0 <image-ref> [required-archs]" >&2
  exit 2
fi

echo "=== manifest architectures for ${IMAGE} ==="
echo "required: ${REQUIRED_ARCHS}"

# `docker buildx imagetools inspect` reads the manifest list straight
# from the registry -- it does not pull any layers, so it works for a
# list this runner cannot run natively. The `--format '{{json .Manifest}}'`
# form yields the raw manifest, from which each entry's platform can be
# read exactly.
#
# The flag MUST be the long form `--format`. The `-f` shorthand is NOT
# accepted by `docker buildx imagetools inspect`:
#
#     $ docker buildx imagetools inspect <ref> -f '{{json .Manifest}}'
#     ERROR: unknown shorthand flag: 'f' in -f
#
# This was verified against the exact buildx the CI runner uses
# (v0.38.0, installed by docker/setup-buildx-action@v4): the shorthand
# is rejected with exit code 125, and the long form returns the
# manifest correctly. `imagetools inspect` simply never registered a
# `f` shorthand -- only `inspect` (the single-image variant) does.
#
# The failure was silent and total. The script sent the flag error to
# /dev/null, saw empty output, and reported "could not read the
# manifest" -- a message that pointed at the registry when the real
# fault was the command line. That cost a failed v0.1.7 release.
# Stderr is therefore NO LONGER suppressed: a broken invocation must be
# visible rather than masquerading as a registry problem.
#
# Attestation manifests (BuildKit's provenance/SBOM sidecars) carry the
# platform `unknown/unknown` and are not real images. They must be
# filtered out or they would be mistaken for architectures.
#
# The read is retried, because an unreadable manifest right after a push
# is a registry propagation delay rather than proof of a defect. The
# retry loop ONLY covers the unreadable case: once a manifest reads
# successfully, its contents are judged immediately and are never
# retried away.
manifest_json=""
attempt=1
while [ "$attempt" -le "$READ_ATTEMPTS" ]; do
  manifest_json="$(docker buildx imagetools inspect "$IMAGE" --format '{{json .Manifest}}')"
  if [ -n "$manifest_json" ]; then
    if [ "$attempt" -gt 1 ]; then
      echo "manifest became readable after ${attempt} attempt(s)"
    fi
    break
  fi
  if [ "$attempt" -eq "$READ_ATTEMPTS" ]; then
    break
  fi
  echo "manifest for ${IMAGE} is not readable yet (attempt ${attempt}/${READ_ATTEMPTS}); retrying in ${READ_DELAY_SECONDS}s" >&2
  sleep "$READ_DELAY_SECONDS"
  attempt=$((attempt + 1))
done

if [ -z "$manifest_json" ]; then
  echo "FAIL: could not read the manifest for ${IMAGE} from the registry" >&2
  echo "      (tried ${READ_ATTEMPTS} time(s); the manifest never became readable)" >&2
  exit 1
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "FAIL: jq is required to parse the manifest" >&2
  exit 1
fi

# The jq program is kept on ONE line. A backslash line-continuation
# inside a single-quoted string is passed to jq literally, which makes
# jq fail to compile the filter -- the script then reports "no real
# image entries" for a perfectly good manifest.
found="$(printf '%s' "$manifest_json" \
  | jq -r '.manifests[]? | select(.platform.os != "unknown") | "\(.platform.os)/\(.platform.architecture)"' \
  | sort -u)"

if [ -z "$found" ]; then
  echo "FAIL: the manifest contains no real image entries." >&2
  echo "--- raw manifest ---" >&2
  printf '%s\n' "$manifest_json" | head -40 >&2
  exit 1
fi

echo "found: ${found}"

failed=0
for want in $REQUIRED_ARCHS; do
  if ! printf '%s\n' "$found" | grep -qx "$want"; then
    echo "FAIL: manifest is missing ${want}" >&2
    failed=1
  else
    echo "OK ${want}"
  fi
done

if [ "$failed" = "1" ]; then
  echo "FAIL: the published manifest list does not contain every required architecture." >&2
  exit 1
fi

echo "=== ALL ASSERTIONS PASS ==="
