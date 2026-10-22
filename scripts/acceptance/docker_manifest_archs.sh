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

if [ -z "$IMAGE" ]; then
  echo "usage: $0 <image-ref> [required-archs]" >&2
  exit 2
fi

echo "=== manifest architectures for ${IMAGE} ==="
echo "required: ${REQUIRED_ARCHS}"

# `docker buildx imagetools inspect` reads the manifest list straight
# from the registry -- it does not pull any layers, so it works for a
# list this runner cannot run natively. The `-f '{{json .Manifest}}'`
# form yields the raw manifest, from which each entry's platform can be
# read exactly.
#
# Attestation manifests (BuildKit's provenance/SBOM sidecars) carry the
# platform `unknown/unknown` and are not real images. They must be
# filtered out or they would be mistaken for architectures.
manifest_json="$(docker buildx imagetools inspect "$IMAGE" -f '{{json .Manifest}}' 2>/dev/null)"
if [ -z "$manifest_json" ]; then
  echo "FAIL: could not read the manifest for ${IMAGE} from the registry" >&2
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
