#!/usr/bin/env bash
# create-dmg.sh -- Create a professional DMG installer for Fabric.app
#
# Creates a read-only DMG with an Applications symlink and properly sized window.
#
# Usage:
#   ./create-dmg.sh --app Fabric.app --output Fabric.dmg
#
# Optional flags:
#   --volume-name NAME   Volume name shown when DMG is mounted (default: "Phenotype Fabric")
#   --background PATH    Path to a background image (PNG, 1920x1080 recommended)
#   --window-size W H    Window width and height in pixels (default: 640 400)
#   --icon-size SIZE     Icon size in pixels (default: 80)
#   --app-icon X Y       Position of the app icon (default: 140 200)
#   --shortcut-icon X Y  Position of the Applications shortcut (default: 500 200)
#
set -euo pipefail

# ──────────────────────────────────────────────────────────────────────────────
# Defaults
# ──────────────────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_PATH=""
OUTPUT_PATH=""
VOLUME_NAME="Phenotype Fabric"
BACKGROUND=""
WINDOW_W=640
WINDOW_H=400
ICON_SIZE=80
APP_ICON_X=140
APP_ICON_Y=200
SHORTCUT_X=500
SHORTCUT_Y=200

# ──────────────────────────────────────────────────────────────────────────────
# Argument parsing
# ──────────────────────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --app)            APP_PATH="$2"; shift 2 ;;
        --output)         OUTPUT_PATH="$2"; shift 2 ;;
        --volume-name)    VOLUME_NAME="$2"; shift 2 ;;
        --background)     BACKGROUND="$2"; shift 2 ;;
        --window-size)    WINDOW_W="$2"; WINDOW_H="$3"; shift 3 ;;
        --icon-size)      ICON_SIZE="$2"; shift 2 ;;
        --app-icon)       APP_ICON_X="$2"; APP_ICON_Y="$3"; shift 3 ;;
        --shortcut-icon)  SHORTCUT_X="$2"; SHORTCUT_Y="$3"; shift 3 ;;
        --help|-h)
            head -15 "$0" | tail -14
            exit 0
            ;;
        *)
            echo "ERROR: Unknown flag: $1" >&2
            exit 1
            ;;
    esac
done

# ──────────────────────────────────────────────────────────────────────────────
# Validation
# ──────────────────────────────────────────────────────────────────────────────
if [[ -z "$APP_PATH" ]]; then
    echo "ERROR: --app is required." >&2
    echo "Run with --help for usage." >&2
    exit 1
fi
if [[ -z "$OUTPUT_PATH" ]]; then
    OUTPUT_PATH="$(basename "$APP_PATH" .app).dmg"
fi
if [[ ! -d "$APP_PATH" ]]; then
    echo "ERROR: .app bundle not found: $APP_PATH" >&2
    exit 1
fi
if [[ -n "$BACKGROUND" && ! -f "$BACKGROUND" ]]; then
    echo "ERROR: Background image not found: $BACKGROUND" >&2
    exit 1
fi

APP_NAME="$(basename "$APP_PATH" .app)"

# ──────────────────────────────────────────────────────────────────────────────
# Clean up any previous DMG
# ──────────────────────────────────────────────────────────────────────────────
rm -f "$OUTPUT_PATH"

# ──────────────────────────────────────────────────────────────────────────────
# Create a temporary working directory
# ──────────────────────────────────────────────────────────────────────────────
WORK_DIR=$(mktemp -d)
trap 'rm -rf "$WORK_DIR"' EXIT

echo "==> Creating DMG staging area..."

# Copy the .app into the working directory
cp -R "$APP_PATH" "$WORK_DIR/$(basename "$APP_PATH")"

# Create the Applications symlink
ln -s /Applications "$WORK_DIR/Applications"

# ──────────────────────────────────────────────────────────────────────────────
# Set DMG window appearance via AppleScript
# ──────────────────────────────────────────────────────────────────────────────
DMG_TEMP="${WORK_DIR}/temp.dmg"

if [[ -n "$BACKGROUND" ]]; then
    echo "==> Using background image: $BACKGROUND"
    cp "$BACKGROUND" "$WORK_DIR/background.png"
fi

# Write the AppleScript that configures the Finder window
APPLESCRIPT=""
if [[ -n "$BACKGROUND" ]]; then
    APPLESCRIPT="\n"
    APPLESCRIPT+="tell application \"Finder\"\n"
    APPLESCRIPT+="tell disk \"${VOLUME_NAME}\"\n"
    APPLESCRIPT+="open\n"
    APPLESCRIPT+="set current view of container window to icon view\n"
    APPLESCRIPT+="set toolbar visible of container window to false\n"
    APPLESCRIPT+="set statusbar visible of container window to false\n"
    APPLESCRIPT+="set the bounds of container window to {100, 100, $((100 + WINDOW_W)), $((100 + WINDOW_H))}\n"
    APPLESCRIPT+="set viewOptions to the icon view options of container window\n"
    APPLESCRIPT+="set arrangement of viewOptions to not arranged\n"
    APPLESCRIPT+="set icon size of viewOptions to ${ICON_SIZE}\n"
    APPLESCRIPT+="set background picture of viewOptions to file \"background.png\"\n"
    APPLESCRIPT+="set position of item \"${APP_NAME}.app\" of container window to {$APP_ICON_X, $APP_ICON_Y}\n"
    APPLESCRIPT+="set position of item \"Applications\" of container window to {$SHORTCUT_X, $SHORTCUT_Y}\n"
    APPLESCRIPT+="close\n"
    APPLESCRIPT+="open\n"
    APPLESCRIPT+="update without registering applications\n"
    APPLESCRIPT+="delay 2\n"
    APPLESCRIPT+="end tell\n"
    APPLESCRIPT+="end tell"
elif [[ "$APP_ICON_X" != "140" || "$APP_ICON_Y" != "200" || "$SHORTCUT_X" != "500" || "$SHORTCUT_Y" != "200" ]]; then
    # Custom positions but no background
    APPLESCRIPT="\n"
    APPLESCRIPT+="tell application \"Finder\"\n"
    APPLESCRIPT+="tell disk \"${VOLUME_NAME}\"\n"
    APPLESCRIPT+="open\n"
    APPLESCRIPT+="set current view of container window to icon view\n"
    APPLESCRIPT+="set toolbar visible of container window to false\n"
    APPLESCRIPT+="set statusbar visible of container window to false\n"
    APPLESCRIPT+="set the bounds of container window to {100, 100, $((100 + WINDOW_W)), $((100 + WINDOW_H))}\n"
    APPLESCRIPT+="set viewOptions to the icon view options of container window\n"
    APPLESCRIPT+="set arrangement of viewOptions to not arranged\n"
    APPLESCRIPT+="set icon size of viewOptions to ${ICON_SIZE}\n"
    APPLESCRIPT+="set position of item \"${APP_NAME}.app\" of container window to {$APP_ICON_X, $APP_ICON_Y}\n"
    APPLESCRIPT+="set position of item \"Applications\" of container window to {$SHORTCUT_X, $SHORTCUT_Y}\n"
    APPLESCRIPT+="close\n"
    APPLESCRIPT+="open\n"
    APPLESCRIPT+="update without registering applications\n"
    APPLESCRIPT+="delay 2\n"
    APPLESCRIPT+="end tell\n"
    APPLESCRIPT+="end tell"
fi

# ──────────────────────────────────────────────────────────────────────────────
# Create the DMG
# ──────────────────────────────────────────────────────────────────────────────
echo "==> Creating DMG..."

hdiutil create \
    -volname "$VOLUME_NAME" \
    -srcfolder "$WORK_DIR" \
    -ov \
    -format UDZO \
    -imagekey zlib-level=9 \
    "$DMG_TEMP"

# ──────────────────────────────────────────────────────────────────────────────
# Apply Finder window layout (if AppleScript provided)
# ──────────────────────────────────────────────────────────────────────────────
if [[ -n "$APPLESCRIPT" ]]; then
    echo "==> Applying Finder window layout..."
    # Write AppleScript to temp file
    echo "$APPLESCRIPT" > "$WORK_DIR/set-layout.scpt"

    # Mount the DMG read-write for layout modification
    RW_DMG="${WORK_DIR}/temp-rw.dmg"
    hdiutil convert "$DMG_TEMP" \
        -format UDRW \
        -o "$RW_DMG"

    MOUNT_OUTPUT=$(hdiutil attach -readwrite -noverify \
        -noautoopen -mountpoint "$WORK_DIR/mnt" "$RW_DMG" 2>&1)

    MOUNT_DEV=$(echo "$MOUNT_OUTPUT" | grep '/dev/disk' | head -1 | awk '{print $1}')

    # Run the AppleScript
    osascript "$WORK_DIR/set-layout.scpt" 2>/dev/null || true

    # Sync and unmount
    sync
    hdiutil detach "$MOUNT_DEV" -force 2>/dev/null || true

    # Convert back to compressed read-only format
    hdiutil convert "$RW_DMG" \
        -format UDZO \
        -imagekey zlib-level=9 \
        -o "$OUTPUT_PATH"
else
    # No layout to apply, just rename the temp DMG
    mv "$DMG_TEMP" "$OUTPUT_PATH"
fi

# ──────────────────────────────────────────────────────────────────────────────
# Done
# ──────────────────────────────────────────────────────────────────────────────
echo ""
echo "=== DMG Created ==="
echo "  Output: $OUTPUT_PATH"
if command -v du &>/dev/null; then
    SIZE=$(du -h "$OUTPUT_PATH" | cut -f1)
    echo "  Size:   $SIZE"
fi
echo ""
echo "To mount: open \"$OUTPUT_PATH\""
echo "To sign:  codesign --force --sign \"Developer ID Application: ...\" \"$OUTPUT_PATH\""
