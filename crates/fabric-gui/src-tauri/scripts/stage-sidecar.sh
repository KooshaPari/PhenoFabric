#!/usr/bin/env bash
# Stage the fabric-daemon release binary as a Tauri sidecar.
#
# The GUI resolves the daemon by looking next to its own executable, then in
# PATH. A Finder-launched .app gets launchd's minimal PATH
# (/usr/bin:/bin:/usr/sbin:/sbin), so PATH lookup never finds a locally
# installed daemon and the app cannot start one at all. Tauri's `externalBin`
# copies a staged binary into Contents/MacOS/ next to the GUI executable,
# which is exactly where the resolver looks first.
#
# Tauri requires the staged file to carry the target-triple suffix:
#   binaries/fabric-daemon-<host-triple>
# It strips the suffix when bundling. Run from `beforeBuildCommand`.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# scripts -> src-tauri -> fabric-gui -> crates -> repo root
root="$(cd "$here/../../../.." && pwd)"

triple="$(rustc -vV | awk '/^host: /{print $2}')"
if [ -z "$triple" ]; then
  echo "stage-sidecar: could not determine host target triple" >&2
  exit 1
fi

src="$root/target/release/fabric-daemon"
dst_dir="$here/../binaries"
dst="$dst_dir/fabric-daemon-$triple"

# Always go through cargo: it is incremental and cheap when the binary is
# already current, and it is the only thing that knows whether the sources
# changed. Checking only for a missing file would happily ship a stale daemon.
echo "stage-sidecar: building fabric-daemon (release)"
(cd "$root" && cargo build --release --bin fabric-daemon)

if [ ! -x "$src" ]; then
  echo "stage-sidecar: expected binary not found at $src after build" >&2
  exit 1
fi

mkdir -p "$dst_dir"
cp "$src" "$dst"
chmod +x "$dst"

echo "stage-sidecar: staged $dst"
