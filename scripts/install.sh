#!/usr/bin/env bash
# terminal-fabric installer for macOS/Linux
# Usage: curl -sSL https://raw.githubusercontent.com/KooshaPari/terminal-fabric/main/scripts/install.sh | bash
set -euo pipefail

REPO="KooshaPari/terminal-fabric"
BINARY="tf-web"
VERSION="${1:-latest}"

echo "Installing terminal-fabric..."

# Detect platform
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  darwin) PLATFORM="apple-darwin" ;;
  linux)  PLATFORM="unknown-linux-gnu" ;;
  *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  arm64|aarch64) ARCH="aarch64" ;;
  x86_64|amd64)  ARCH="x86_64" ;;
  *)             echo "Unsupported arch: $ARCH"; exit 1 ;;
esac

# Get latest version if not specified
if [ "$VERSION" = "latest" ]; then
  VERSION=$(curl -sL "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"v([^"]+)".*/\1/')
fi

echo "Downloading $BINARY v$VERSION for $ARCH-$PLATFORM..."

# Download
URL="https://github.com/$REPO/releases/download/v$VERSION/${BINARY}-${ARCH}-${PLATFORM}.tar.gz"
curl -sL "$URL" | tar xz -C /tmp

# Install to /usr/local/bin
chmod +x "/tmp/$BINARY"
sudo mv "/tmp/$BINARY" /usr/local/bin/

echo "Installed $BINARY v$VERSION to /usr/local/bin/$BINARY"