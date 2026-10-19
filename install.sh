#!/usr/bin/env bash
# install.sh — curl installer for Phenotype Fabric
# Usage: curl -fsSL https://raw.githubusercontent.com/KooshaPari/PhenoFabric/main/install.sh | bash
set -euo pipefail

REPO="KooshaPari/PhenoFabric"
# Binaries installed into INSTALL_DIR. These are the real cargo [[bin]]
# target names and MUST match the release archive contents in
# .github/workflows/release.yml:
#   - "fabric"         is the [[bin]] of the fabric-cli crate
#     (crates/fabric-cli/Cargo.toml), NOT the crate name "fabric-cli"
#   - "fabric-graph-cli" is the crate/bin name; there is no "fabric-graph"
#     binary (fabric-graph is a library crate with no [[bin]])
#   - "fabric-gui" is deliberately absent: the GUI is a Tauri app shipped as
#     a separate .dmg/.deb/.msi/.AppImage asset by the build-gui job, and
#     crates/fabric-gui is a library-only crate with no cargo binary target.
# Keeping a name here that can never exist in the archive just produces a
# permanent "Skipped (not in archive)" warning.
BINS="fabric fabric-daemon fabric-tui fabric-tray fabric-graph-cli"
# All five names in BINS are REQUIRED. Every one of them is a real cargo
# [[bin]] target (verified against `cargo metadata`) and every one of them is
# packaged into the release archives by .github/workflows/release.yml. If one
# is absent from the extracted archive the install is PARTIAL: the user gets a
# broken toolchain (e.g. no `fabric` at all) while the script still reports
# success. A missing required binary is therefore a hard error, not a warning.
#
# Observed on v0.1.2: all three published .tar.gz archives were missing
# `fabric` and `fabric-graph-cli`; this script silently installed 3 of 5
# binaries, printed "Done.", and exited 0. That is the defect this fixes.
#
# INSTALL_DIR is overridable so the installer can be exercised end-to-end
# against a real release without writing to /usr/local/bin. The default is
# unchanged.
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"

# ── Colors ────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RESET='\033[0m'

info()  { printf "${GREEN}[info]${RESET}  %s\n" "$1"; }
warn()  { printf "${YELLOW}[warn]${RESET}  %s\n" "$1"; }
die()   { printf "${RED}[error]${RESET} %s\n" "$1" >&2; exit 1; }

# ── Detect OS ─────────────────────────────────────────────────────────
_detect_os() {
  local os
  os=$(uname -s)
  case "$os" in
    Linux*)  OS="linux" ;;
    Darwin*) OS="macos" ;;
    MINGW*|MSYS*|CYGWIN*) OS="windows" ;;
    *) die "Unsupported OS: $os" ;;
  esac
}

# ── Detect arch ───────────────────────────────────────────────────────
_detect_arch() {
  local arch
  arch=$(uname -m)
  case "$arch" in
    x86_64|amd64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    *) die "Unsupported architecture: $arch" ;;
  esac
}

# ── Map to release platform ───────────────────────────────────────────
_platform() {
  echo "${OS}-${ARCH}"
}

# ── Fetch latest release tag ─────────────────────────────────────────
_latest_version() {
  local url
  if [ -n "${FABRIC_VERSION:-}" ]; then
    echo "$FABRIC_VERSION"
    return
  fi
  url="https://api.github.com/repos/${REPO}/releases/latest"
  if command -v curl &>/dev/null; then
    curl -fsSL "$url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name":\s*"\([^"]*\)".*/\1/'
  elif command -v wget &>/dev/null; then
    wget -qO- "$url" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name":\s*"\([^"]*\)".*/\1/'
  else
    die "Neither curl nor wget found. Install one and retry."
  fi
}

# ── Download and extract ──────────────────────────────────────────────
download_and_install() {
  local version=$1
  local platform=$2
  local archive="phenotype-fabric-${version}-${platform}.tar.gz"
  local url="https://github.com/${REPO}/releases/download/${version}/${archive}"
  local tmpdir
  tmpdir=$(mktemp -d)

  info "Downloading ${archive} ..."
  if command -v curl &>/dev/null; then
    curl -fSL -o "${tmpdir}/${archive}" "$url" || die "Download failed. Check that version '${version}' exists."
  else
    wget -qO "${tmpdir}/${archive}" "$url" || die "Download failed. Check that version '${version}' exists."
  fi

  info "Extracting ..."
  tar xzf "${tmpdir}/${archive}" -C "${tmpdir}"

  local dist_dir="${tmpdir}/phenotype-fabric-${version}-${platform}"
  if [ ! -d "$dist_dir" ]; then
    die "Expected directory not found after extraction: $dist_dir"
  fi

  # Preflight: refuse to install anything at all if the archive is incomplete.
  # Doing this before the copy loop means a partial install never leaves the
  # user with a half-populated INSTALL_DIR.
  local missing=""
  for bin in $BINS; do
    if [ ! -f "${dist_dir}/${bin}" ]; then
      missing="${missing} ${bin}"
    fi
  done
  if [ -n "$missing" ]; then
    local archive_contents
    archive_contents="$(find "$dist_dir" -mindepth 1 -maxdepth 1 -exec basename {} \; | tr '\n' ' ')"
    rm -rf "$tmpdir"
    die "Archive ${archive} is missing REQUIRED binaries:${missing} -- nothing was installed. Contents of archive dir: ${archive_contents}"
  fi

  info "Installing to ${INSTALL_DIR} ..."
  for bin in $BINS; do
    install -m 755 "${dist_dir}/${bin}" "${INSTALL_DIR}/${bin}"
    info "  Installed ${bin}"
  done

  rm -rf "$tmpdir"
  info "Done."
}

# ── Main ──────────────────────────────────────────────────────────────
main() {
  info "Phenotype Fabric installer"

  _detect_os
  _detect_arch
  local platform
  platform=$(_platform)
  info "Detected platform: ${platform}"

  if [ ! -d "$INSTALL_DIR" ]; then
    die "Install directory ${INSTALL_DIR} does not exist. Create it or run with sudo."
  fi

  # Check write permissions
  if [ ! -w "$INSTALL_DIR" ]; then
    warn "No write permission to ${INSTALL_DIR}. Retrying with sudo (INSTALL_DIR is inherited)."
    exec sudo env "INSTALL_DIR=${INSTALL_DIR}" bash "$0" "$@"
  fi

  local version
  version=$(_latest_version)
  if [ -z "$version" ]; then
    die "Could not determine latest version."
  fi
  info "Latest version: ${version}"

  download_and_install "$version" "$platform"

  echo
  # Resolve each binary against INSTALL_DIR rather than $PATH: INSTALL_DIR is
  # not necessarily on PATH, and reporting the destination we actually wrote
  # to beats a stale shadowing copy earlier in PATH.
  local installed_count=0
  local required_count
  required_count=$(echo "$BINS" | wc -w | tr -d ' ')
  info "Installed binaries:"
  for bin in $BINS; do
    if [ -x "${INSTALL_DIR}/${bin}" ]; then
      printf "  %-20s %s\n" "$bin" "${INSTALL_DIR}/${bin}"
      installed_count=$((installed_count + 1))
    else
      printf "  %-20s %s\n" "$bin" "MISSING"
    fi
  done
  echo
  if [ "$installed_count" -ne "$required_count" ]; then
    die "Installed ${installed_count} of ${required_count} required binaries into ${INSTALL_DIR}."
  fi
  echo
  info "Get started: fabric --help"
}

main "$@"
