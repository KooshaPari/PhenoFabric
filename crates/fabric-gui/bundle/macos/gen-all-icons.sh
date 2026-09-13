#!/usr/bin/env bash
# Generate Fabric.icns from icon.png using macOS sips + iconutil
set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ICON_PNG="$SCRIPT_DIR/icon.png"
ICONSET_DIR="$SCRIPT_DIR/Fabric.iconset"
OUTPUT_ICNS="$SCRIPT_DIR/Fabric.icns"

if [[ ! -f "$ICON_PNG" ]]; then
    echo "ERROR: $ICON_PNG not found. Run gen-icon.py first."
    exit 1
fi

rm -rf "$ICONSET_DIR"
mkdir -p "$ICONSET_DIR"

echo "==> Resizing icon to all macOS sizes..."

# Each line: output_name pixel_size
SIZES="
icon_16x16 16
icon_16x16@2x 32
icon_32x32 32
icon_32x32@2x 64
icon_128x128 128
icon_128x128@2x 256
icon_256x256 256
icon_256x256@2x 512
icon_512x512 512
icon_512x512@2x 1024
"

echo "$SIZES" | while read -r name pixels; do
    [[ -z "$name" ]] && continue
    sips -z "$pixels" "$pixels" "$ICON_PNG" --out "$ICONSET_DIR/${name}.png" >/dev/null 2>&1
    echo "    ${name}: ${pixels}x${pixels}"
done

echo "==> Creating Fabric.icns..."
iconutil -c icns "$ICONSET_DIR" -o "$OUTPUT_ICNS"

# Clean up iconset directory
rm -rf "$ICONSET_DIR"

echo "==> Icon written to $OUTPUT_ICNS"
ls -lh "$OUTPUT_ICNS"
