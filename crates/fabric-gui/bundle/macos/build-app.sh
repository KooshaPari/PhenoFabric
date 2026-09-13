#!/usr/bin/env bash
# Build a macOS .app bundle for Phenotype Fabric GUI
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CRATE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../../" && pwd)"
APP_NAME="Fabric"
BUNDLE_ID="com.phenotype.fabric"
VERSION="0.1.0"
ICON_NAME="Fabric.icns"

APP_DIR="$SCRIPT_DIR/$APP_NAME.app"

echo "==> Building fabric-gui in release mode..."
cd "$REPO_ROOT"
cargo build --release -p fabric-gui 2>&1

BINARY_SRC="target/release/fabric-gui"
if [[ ! -f "$BINARY_SRC" ]]; then
    echo "ERROR: Binary not found at $BINARY_SRC"
    exit 1
fi

echo "==> Creating $APP_NAME.app bundle..."
rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS"
mkdir -p "$APP_DIR/Contents/Resources"

echo "==> Copying binary..."
cp "$BINARY_SRC" "$APP_DIR/Contents/MacOS/fabric-gui"
chmod +x "$APP_DIR/Contents/MacOS/fabric-gui"

echo "==> Copying icon..."
ICNS_PATH="$SCRIPT_DIR/$ICON_NAME"
if [[ -f "$ICNS_PATH" ]]; then
    cp "$ICNS_PATH" "$APP_DIR/Contents/Resources/$ICON_NAME"
else
    echo "WARNING: $ICON_NAME not found at $ICNS_PATH; building without icon"
fi

echo "==> Generating Info.plist..."
cat > "$APP_DIR/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>Phenotype Fabric</string>
    <key>CFBundleExecutable</key>
    <string>fabric-gui</string>
    <key>CFBundleIconFile</key>
    <string>${ICON_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
PLIST

echo "==> Ad-hoc signing..."
codesign --force --sign - "$APP_DIR" 2>&1

echo ""
echo "=== Build Complete ==="
echo "  .app bundle: $APP_DIR"
echo "  Contents:"
find "$APP_DIR" -type f | sed "s|^|    |"
echo ""
echo "To run: open \"$APP_DIR\""
