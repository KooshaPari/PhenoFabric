#!/usr/bin/env bash
# install.sh — curl installer for Phenotype Fabric
# Usage: curl -fsSL https://raw.githubusercontent.com/KooshaPari/PhenoFabric/main/install.sh | bash
set -euo pipefail

REPO="KooshaPari/PhenoFabric"
BINS="fabric-daemon fabric-cli fabric-tui fabric-gui fabric-tray fabric-graph"
INSTALL_DIR="/usr/local/bin"

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

  info "Installing to ${INSTALL_DIR} ..."
  for bin in $BINS; do
    if [ -f "${dist_dir}/${bin}" ]; then
      install -m 755 "${dist_dir}/${bin}" "${INSTALL_DIR}/${bin}"
      info "  Installed ${bin}"
    else
      warn "  Skipped ${bin} (not in archive)"
    fi
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
    warn "No write permission to ${INSTALL_DIR}. Retrying with sudo ..."
    exec sudo bash "$0" "$@"
  fi

  local version
  version=$(_latest_version)
  if [ -z "$version" ]; then
    die "Could not determine latest version."
  fi
  info "Latest version: ${version}"

  download_and_install "$version" "$platform"

  echo
  info "Installed binaries:"
  for bin in $BINS; do
    if command -v "$bin" &>/dev/null; then
      printf "  %-20s %s\n" "$bin" "$(command -v $bin)"
    fi
  done
  echo
  info "Get started: fabric-cli --help"
}

main "$@"
