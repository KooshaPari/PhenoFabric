#!/usr/bin/env bash
# sign-and-notarize.sh -- Sign Fabric.app with Developer ID, notarize with Apple, and staple the ticket.
#
# Usage:
#   ./sign-and-notarize.sh \
#     --developer-id "Developer ID Application: Phenotype (TEAMID)" \
#     --apple-id "you@example.com" \
#     --team-id "TEAMID" \
#     --password "xxxx-xxxx-xxxx-xxxx"
#
# Optional flags:
#   --app PATH          Path to Fabric.app (default: ./Fabric.app relative to this script)
#   --entitlements PATH Path to entitlements file (default: fabric.entitlements next to this script)
#   --skip-notarize     Sign only, skip notarization
#   --skip-staple       Sign and notarize, skip stapling
#
set -euo pipefail

# ──────────────────────────────────────────────────────────────────────────────
# Defaults
# ──────────────────────────────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_PATH=""
ENTITLEMENTS_PATH=""
DEVELOPER_ID=""
APPLE_ID=""
TEAM_ID=""
PASSWORD=""
SKIP_NOTARIZE=false
SKIP_STAPLE=false

# ──────────────────────────────────────────────────────────────────────────────
# Argument parsing
# ──────────────────────────────────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
    case "$1" in
        --app)          APP_PATH="$2"; shift 2 ;;
        --entitlements) ENTITLEMENTS_PATH="$2"; shift 2 ;;
        --developer-id) DEVELOPER_ID="$2"; shift 2 ;;
        --apple-id)     APPLE_ID="$2"; shift 2 ;;
        --team-id)      TEAM_ID="$2"; shift 2 ;;
        --password)     PASSWORD="$2"; shift 2 ;;
        --skip-notarize) SKIP_NOTARIZE=true; shift ;;
        --skip-staple)   SKIP_STAPLE=true; shift ;;
        --help|-h)
            head -13 "$0" | tail -12
            exit 0
            ;;
        *)
            echo "ERROR: Unknown flag: $1" >&2
            exit 1
            ;;
    esac
done

# ──────────────────────────────────────────────────────────────────────────────
# Resolve defaults
# ──────────────────────────────────────────────────────────────────────────────
if [[ -z "$APP_PATH" ]]; then
    APP_PATH="$SCRIPT_DIR/Fabric.app"
fi
if [[ -z "$ENTITLEMENTS_PATH" ]]; then
    ENTITLEMENTS_PATH="$SCRIPT_DIR/fabric.entitlements"
fi

# ──────────────────────────────────────────────────────────────────────────────
# Validation
# ──────────────────────────────────────────────────────────────────────────────
missing_flags=()
if [[ -z "$DEVELOPER_ID" ]]; then missing_flags+=(--developer-id); fi
if [[ "$SKIP_NOTARIZE" == false ]]; then
    if [[ -z "$APPLE_ID" ]];     then missing_flags+=(--apple-id); fi
    if [[ -z "$TEAM_ID" ]];      then missing_flags+=(--team-id); fi
    if [[ -z "$PASSWORD" ]];     then missing_flags+=(--password); fi
fi
if [[ ${#missing_flags[@]} -gt 0 ]]; then
    echo "ERROR: Missing required flags: ${missing_flags[*]}" >&2
    echo "Run with --help for usage information." >&2
    exit 1
fi

if [[ ! -d "$APP_PATH" ]]; then
    echo "ERROR: .app bundle not found: $APP_PATH" >&2
    echo "Run build-app.sh first to create the .app bundle." >&2
    exit 1
fi

if [[ ! -f "$ENTITLEMENTS_PATH" ]]; then
    echo "ERROR: Entitlements file not found: $ENTITLEMENTS_PATH" >&2
    exit 1
fi

echo "==> Configuration:"
echo "    App:          $APP_PATH"
echo "    Entitlements: $ENTITLEMENTS_PATH"
echo "    Signing ID:   $DEVELOPER_ID"
echo "    Notarize:     $([ "$SKIP_NOTARIZE" == true ] && echo "no" || echo "yes")"
echo ""

# ──────────────────────────────────────────────────────────────────────────────
# Step 1: Clean existing signatures
# ──────────────────────────────────────────────────────────────────────────────
echo "==> [1/4] Cleaning existing signatures..."
codesign --remove-signature "$APP_PATH" 2>/dev/null || true
rm -rf "$APP_PATH/Contents/_CodeSignature"

# ──────────────────────────────────────────────────────────────────────────────
# Step 2: Sign the bundle
# ──────────────────────────────────────────────────────────────────────────────
echo "==> [2/4] Signing with hardened runtime and entitlements..."

# Sign the main binary first
codesign --force \
    --sign "$DEVELOPER_ID" \
    --timestamp \
    --options runtime \
    --entitlements "$ENTITLEMENTS_PATH" \
    "$APP_PATH/Contents/MacOS/fabric-gui"

# Sign the bundle
codesign --force \
    --sign "$DEVELOPER_ID" \
    --timestamp \
    --options runtime \
    --entitlements "$ENTITLEMENTS_PATH" \
    "$APP_PATH"

echo "    Verifying signature..."
codesign --verify --verbose=2 "$APP_PATH"
echo "    Signature OK."

# ──────────────────────────────────────────────────────────────────────────────
# Step 3: Notarize
# ──────────────────────────────────────────────────────────────────────────────
if [[ "$SKIP_NOTARIZE" == true ]]; then
    echo "==> [3/4] Skipping notarization (--skip-notarize)."
    echo ""
    echo "=== Signing Complete (not notarized) ==="
    echo "  App: $APP_PATH"
    echo ""
    echo "To notarize later:"
    echo "  xcrun notarytool submit \"$APP_PATH" --keychain-profile "phenotype-notarization" --wait"
    echo "  xcrun stapler staple \"$APP_PATH""
    exit 0
fi

echo "==> [3/4] Submitting for notarization..."
echo "    Apple ID: $APPLE_ID"
echo "    Team ID:  $TEAM_ID"
echo ""

# Try to use stored credentials first, fall back to inline auth
if xcrun notarytool submit "$APP_PATH" \
    --apple-id "$APPLE_ID" \
    --team-id "$TEAM_ID" \
    --password "$PASSWORD" \
    --wait 2>&1 | tee /tmp/notary-output.txt; then
    NOTARY_SUCCESS=true
else
    NOTARY_SUCCESS=false
fi

if [[ "$NOTARY_SUCCESS" == false ]]; then
    echo ""
    echo "ERROR: Notarization failed. Attempting to fetch log..."
    # Extract submission ID from output if possible
    SUBMISSION_ID=$(grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' /tmp/notary-output.txt | head -1 || true)
    if [[ -n "$SUBMISSION_ID" ]]; then
        echo "Submission ID: $SUBMISSION_ID"
        xcrun notarytool log "$SUBMISSION_ID" \
            --apple-id "$APPLE_ID" \
            --team-id "$TEAM_ID" \
            --password "$PASSWORD" 2>&1 || true
    fi
    exit 1
fi

echo "    Notarization successful."

# ──────────────────────────────────────────────────────────────────────────────
# Step 4: Staple the ticket
# ──────────────────────────────────────────────────────────────────────────────
if [[ "$SKIP_STAPLE" == true ]]; then
    echo "==> [4/4] Skipping stapling (--skip-staple)."
    echo ""
    echo "=== Signing + Notarization Complete ==="
    echo "  App: $APP_PATH"
    exit 0
fi

echo "==> [4/4] Stapling notarization ticket..."
xcrun stapler staple "$APP_PATH"

echo "    Verifying stapled ticket..."
xcrun stapler validate "$APP_PATH"

# ──────────────────────────────────────────────────────────────────────────────
# Done
# ──────────────────────────────────────────────────────────────────────────────
echo ""
echo "=== Signing + Notarization Complete ==="
echo "  App: $APP_PATH"
echo ""
echo "Gatekeeper check:"
spctl --assess --type execute "$APP_PATH" 2>&1 && echo "  PASSED" || echo "  Note: spctl may require the quarantine attribute."
echo ""
echo "To create a DMG:"
echo "  $SCRIPT_DIR/create-dmg.sh --app \"$APP_PATH\" --output Fabric.dmg"
