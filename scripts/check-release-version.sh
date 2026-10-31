#!/usr/bin/env bash
# Fail-closed check that every released binary reports the release version.
#
# Why this exists: v0.1.7 shipped five binaries that all answered `--version`
# with "0.1.0" under a v0.1.7 tag. Three of the five crates that produce those
# binaries do NOT inherit [workspace.package] - they declare their own version
# literal - so a gate that only reads [workspace.package] cannot see them.
#
# This script is the single source of truth for the rule. release.yml calls it.
# It is deliberately standalone so it can be tested against a mutated tree
# without a GitHub runner.
#
# Usage: scripts/check-release-version.sh <release-version>
#   <release-version> may be "v0.1.8" or "0.1.8".
#
# Exit 0 = every released binary compiles with the release version.
# Exit 1 = at least one would report the wrong version. DO NOT PUBLISH.

set -euo pipefail

RELEASE_VERSION="${1:-}"
if [ -z "$RELEASE_VERSION" ]; then
  echo "usage: $0 <release-version>" >&2
  exit 2
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

rel_ms="${RELEASE_VERSION#v}"

# Crates that produce a released binary.
RELEASED_CRATES="fabric-cli fabric-tui fabric-graph-cli fabric-tray fabric-daemon"

ws_ms=$(sed -n '/^\[workspace\.package\]/,/^\[/s/^version *= *"\([^"]*\)".*/\1/p' Cargo.toml | head -1)
if [ -z "$ws_ms" ]; then
  echo "::error::could not read [workspace.package] version from Cargo.toml" >&2
  exit 1
fi
if [ "$ws_ms" != "$rel_ms" ]; then
  echo "::error::release version '${RELEASE_VERSION}' does not match [workspace.package] version '${ws_ms}'." >&2
  echo "::error::Shipping this would publish binaries that report ${ws_ms} under a ${rel_ms} tag, and self-update would never converge." >&2
  echo "::error::Bump [workspace.package] version in Cargo.toml to ${rel_ms} and re-tag." >&2
  exit 1
fi

meta=$(cargo metadata --no-deps --format-version 1)
failed=0
for crate in $RELEASED_CRATES; do
  eff=$(printf '%s' "$meta" | python3 -c '
import json,sys
want=sys.argv[1]
meta=json.load(sys.stdin)
for p in meta["packages"]:
    if p["name"]==want:
        print(p["version"]); break
' "$crate")
  if [ -z "$eff" ]; then
    echo "::error::crate '${crate}' not found in cargo metadata - cannot verify its version" >&2
    failed=1
    continue
  fi
  if [ "$eff" != "$rel_ms" ]; then
    echo "::error::crate '${crate}' compiles with version ${eff}, but the release tag says ${rel_ms}." >&2
    echo "::error::Its binary would answer --version with ${eff}. Bump crates/${crate}/Cargo.toml to ${rel_ms} (or set version.workspace = true)." >&2
    failed=1
  else
    echo "version OK: ${crate} = ${eff}"
  fi
done

if [ "$failed" -ne 0 ]; then
  echo "::error::release blocked: at least one released binary would report the wrong version." >&2
  exit 1
fi
echo "version OK: workspace and all ${RELEASED_CRATES} are ${rel_ms}"
