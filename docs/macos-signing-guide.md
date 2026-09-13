# macOS Code Signing and Notarization Guide

Complete distribution pipeline for Phenotype Fabric desktop app on macOS.

## Overview

Fabric.app (built from `crates/fabric-gui/`) must be code-signed, notarized, and packaged as a DMG for distribution. This guide covers every step from certificate setup through CI automation.

**Bundle identifiers:**
- App: `com.phenotype.fabric`
- Bundle display name: `Phenotype Fabric`
- Minimum macOS: 11.0

---

## 1. Apple Developer Account Requirements

### Membership
- Enroll in the [Apple Developer Program](https://developer.apple.com/programs/) ($99/year)
- Organization or Individual account both work for code signing
- Team Agent or Admin role required for certificate management

### What you need
| Item | Where to find | Purpose |
|------|--------------|---------|
| Team ID | Apple Developer account sidebar (10-char alphanumeric) | Identifies your org to Apple |
| Apple ID | Your Apple account email | Notarization authentication |
| App-Specific Password | [appleid.apple.com](https://appleid.apple.com) > Sign-In and Security > App-Specific Passwords | Used with `notarytool` |
| Certificate | Apple Developer > Certificates, Identifiers & Profiles | Code signing identity |

---

## 2. Code Signing Certificates

### Developer ID Application (required for distribution)

Used for apps distributed outside the Mac App Store. This is the certificate you need for Fabric.app.

1. Go to [Certificates page](https://developer.apple.com/account/resources/certificates/list)
2. Click the "+" to create a new certificate
3. Select **Developer ID Application** (under "Software")
4. Follow the CSR instructions:
   ```bash
   # Generate a Certificate Signing Request
   openssl req -new -newkey rsa:2048 -nodes \
     -keyout /tmp/fabric-signing.key \
     -out /tmp/fabric-signing.csr \
     -subj "/CN=Phenotype Fabric Signing/O=Phenotype"
   ```
5. Upload the `.csr` to the Apple Developer portal
6. Download the resulting `.cer` file
7. Double-click to install in Keychain Access
8. Verify it appears under **My Certificates** in Keychain Access as "Developer ID Application: ..."

### Mac Developer (for development/testing only)

For local development and internal testing only. Apps signed with this certificate will **not** pass Gatekeeper on other machines.

```bash
# Ad-hoc signing (no certificate, local testing only)
codesign --force --sign - Fabric.app
```

### Installing certificates from CI

For CI environments, export the certificate as a `.p12` file:

1. In Keychain Access, right-click the certificate > Export
2. Choose **Personal Information Exchange (.p12)**
3. Set a strong password
4. Store the `.p12` file and password as GitHub Secrets

```bash
# Install certificate in CI from secrets
security import certificate.p12 \
  -k ~/Library/Keychains/signing-db.keychain-db \
  -P "$CERTIFICATE_PASSWORD" \
  -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple: \
  -k "" ~/Library/Keychains/signing-db.keychain-db
```

---

## 3. Entitlements for eframe Apps

eframe/egui apps require specific entitlements to function correctly under hardened runtime. Without these, macOS will kill the process or deny JIT compilation.

**File:** `crates/fabric-gui/bundle/macos/fabric.entitlements`

| Entitlement | Why it's needed |
|-------------|------------------|
| `com.apple.security.cs.allow-jit` | eframe uses wgpu which may use JIT compilation for shader pipelines |
| `com.apple.security.cs.allow-unsigned-executable-memory` | Required by eframe's OpenGL/Metal rendering backend |
| `com.apple.security.network.client` | Fabric connects to remote nodes, surface proxies, and LAN peers |
| `com.apple.security.cs.disable-library-validation` | Allows loading native libraries (dylibs) not signed by Apple or your team |

### Entitlements file location

```
crates/fabric-gui/bundle/macos/fabric.entitlements
```

### Viewing current entitlements of a signed binary

```bash
codesign -d --entitlements - Fabric.app/Contents/MacOS/fabric-gui
```

---

## 4. Code Signing Commands

### Ad-hoc signing (local development)

```bash
# Sign without a certificate - for local testing only
codesign --force --sign - Fabric.app
```

### Signing with Developer ID (distribution)

```bash
SIGNING_IDENTITY="Developer ID Application: Your Name (TEAMID)"

# Sign all binaries and frameworks inside the bundle first
codesign --force --sign "$SIGNING_IDENTITY" \
  --timestamp \
  --options runtime \
  --entitlements fabric.entitlements \
  Fabric.app/Contents/MacOS/fabric-gui

# Sign the bundle itself
codesign --force --sign "$SIGNING_IDENTITY" \
  --timestamp \
  --options runtime \
  --entitlements fabric.entitlements \
  Fabric.app
```

### Flags explained

| Flag | Purpose |
|------|---------|
| `--force` | Replace existing signatures |
| `--timestamp` | Include a secure timestamp (required for notarization) |
| `--options runtime` | Enable hardened runtime (required for notarization) |
| `--entitlements` | Path to the entitlements plist |

### Verifying the signature

```bash
# Detailed verification
codesign --verify --verbose=2 Fabric.app

# Verify the signature meets Gatekeeper requirements
spctl --assess --type execute Fabric.app

# Check the signing chain
codesign -dvvv Fabric.app
```

---

## 5. Notarization via notarytool

Starting with macOS 13, `altool` is deprecated. Use `notarytool` (shipped with Xcode Command Line Tools).

### Prerequisites
- Xcode Command Line Tools: `xcode-select --install`
- An app-specific password from [appleid.apple.com](https://appleid.apple.com)

### Submit for notarization

```bash
# Store credentials (one-time)
xcrun notarytool store-credentials "phenotype-notarization" \
  --apple-id "you@example.com" \
  --team-id "XXXXXXXXXX" \
  --password "xxxx-xxxx-xxxx-xxxx"

# Submit the signed .app
xcrun notarytool submit Fabric.app \
  --keychain-profile "phenotype-notarization" \
  --wait

# Or submit a zip/dmg directly
xcrun notarytool submit Fabric.dmg \
  --keychain-profile "phenotype-notarization" \
  --wait
```

### Monitor submission status

```bash
# List recent submissions
xcrun notarytool list --keychain-profile "phenotype-notarization"

# Get details of a specific submission
xcrun notarytool info <submission-id> \
  --keychain-profile "phenotype-notarization"

# Fetch the log of a failed submission
xcrun notarytool log <submission-id> \
  --keychain-profile "phenotype-notarization"
```

### Staple the notarization ticket

After successful notarization, staple the ticket to the bundle:

```bash
# Staple to the .app
xcrun stapler staple Fabric.app

# Or staple to a .dmg
xcrun stapler staple Fabric.dmg
```

### Verify stapling

```bash
# Check the stapled ticket
xcrun stapler validate Fabric.app

# Full Gatekeeper assessment
spctl --assess --type execute Fabric.app
```

### Common notarization failures

| Error | Fix |
|-------|-----|
| `The binary is not signed` | Ensure hardened runtime is enabled (`--options runtime`) |
| `The signature does not include a secure timestamp` | Add `--timestamp` flag to codesign |
| `The executable requests the ... entitlement` | Entitlements must match what Apple allows for Developer ID |
| `The bundle format is ambiguous` | Ensure the .app bundle structure is correct |
| `Invalid signature` | Re-sign from scratch: remove `Contents/_CodeSignature` and re-sign |

---

## 6. Gatekeeper Considerations

### Quarantine attribute

When users download Fabric.app via a browser, macOS adds a quarantine extended attribute:

```bash
# Check quarantine attribute
xattr -l Fabric.app

# Remove quarantine (for testing only)
xattr -r -d com.apple.quarantine Fabric.app
```

**Important:** Do not ship with the quarantine attribute removed. Users should see the Gatekeeper prompt. Notarization ensures Gatekeeper passes.

### Bypassing Gatekeeper (development only)

```bash
# Allow an app that was blocked by Gatekeeper
spctl --add --label "Approved" Fabric.app

# Or temporarily disable Gatekeeper (admin password required)
sudo spctl --master-disable
# Re-enable when done
sudo spctl --master-enable
```

### Security assessment flow

```
User downloads Fabric.dmg
  -> Finder mounts DMG
  -> User drags Fabric.app to Applications
  -> macOS adds quarantine attribute
  -> User double-clicks Fabric.app
  -> Gatekeeper checks:
     1. Code signature valid? (codesign --verify)
     2. Signed by Developer ID? (not ad-hoc)
     3. Hardened runtime enabled?
     4. Notarized? (Apple's servers check ticket)
     5. Stapled ticket present? (offline fallback)
  -> If all pass: app launches normally
  -> If any fail: "App can't be opened because Apple cannot check it for malicious software"
```

---

## 7. DMG Creation

### Basic DMG

```bash
hdiutil create \
  -volname "Phenotype Fabric" \
  -srcfolder Fabric.app \
  -ov -format UDZO \
  Fabric.dmg
```

### Professional DMG with Applications symlink

Use the dedicated script:

```bash
./crates/fabric-gui/bundle/macos/create-dmg.sh \
  --app Fabric.app \
  --output Fabric.dmg \
  --volume-name "Phenotype Fabric"
```

See `create-dmg.sh` for full details on window layout, background images, and Applications symlink.

### Signing the DMG

```bash
# Sign the DMG with the same identity
codesign --force --sign "$SIGNING_IDENTITY" \
  --timestamp \
  Fabric.dmg
```

---

## 8. CI Automation (GitHub Actions)

### Storing certificates in GitHub Secrets

Create these repository secrets:

| Secret name | Content |
|-------------|--------|
| `MACOS_CERTIFICATE_P12` | Base64-encoded `.p12` certificate file |
| `MACOS_CERTIFICATE_PASSWORD` | Password for the `.p12` file |
| `MACOS_SIGNING_IDENTITY` | Full certificate identity string (e.g., `Developer ID Application: Phenotype (TEAMID)`) |
| `APPLE_ID` | Apple ID email for notarization |
| `APPLE_TEAM_ID` | Apple Developer Team ID |
| `APPLE_APP_PASSWORD` | App-specific password for notarization |

### Base64-encoding the certificate

```bash
# On your local machine, encode the .p12 file
base64 -i certificate.p12 | pbcopy
# Paste as MACOS_CERTIFICATE_P12 in GitHub Secrets
```

### GitHub Actions workflow

```yaml
name: macOS Release

on:
  push:
    tags:
      - 'v*'

jobs:
  macos-sign-notarize:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-darwin

      - name: Build release binary
        run: cargo build --release -p fabric-gui

      - name: Create .app bundle
        run: ./crates/fabric-gui/bundle/macos/build-app.sh

      - name: Import signing certificate
        env:
          CERTIFICATE_P12: ${{ secrets.MACOS_CERTIFICATE_P12 }}
          CERTIFICATE_PASSWORD: ${{ secrets.MACOS_CERTIFICATE_PASSWORD }}
        run: |
          echo "$CERTIFICATE_P12" | base64 --decode > /tmp/certificate.p12
          security create-keychain -p "" signing-db.keychain-db
          security default-keychain -s signing-db.keychain-db
          security unlock-keychain -p "" signing-db.keychain-db
          security import /tmp/certificate.p12 \
            -k signing-db.keychain-db \
            -P "$CERTIFICATE_PASSWORD" \
            -T /usr/bin/codesign
          security set-key-partition-list \
            -S apple-tool:,apple: \
            -k "" signing-db.keychain-db
          security list-keychain -d user -s signing-db.keychain-db

      - name: Sign and notarize
        env:
          SIGNING_IDENTITY: ${{ secrets.MACOS_SIGNING_IDENTITY }}
          APPLE_ID: ${{ secrets.APPLE_ID }}
          TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
          APP_PASSWORD: ${{ secrets.APPLE_APP_PASSWORD }}
        run: |
          ./crates/fabric-gui/bundle/macos/sign-and-notarize.sh \
            --developer-id "$SIGNING_IDENTITY" \
            --apple-id "$APPLE_ID" \
            --team-id "$TEAM_ID" \
            --password "$APP_PASSWORD"

      - name: Create DMG
        run: |
          ./crates/fabric-gui/bundle/macos/create-dmg.sh \
            --app Fabric.app \
            --output Fabric-${{ github.ref_name }}.dmg

      - name: Sign DMG
        env:
          SIGNING_IDENTITY: ${{ secrets.MACOS_SIGNING_IDENTITY }}
        run: |
          codesign --force --sign "$SIGNING_IDENTITY" \
            --timestamp \
            Fabric-${{ github.ref_name }}.dmg

      - name: Upload release artifact
        uses: softprops/action-gh-release@v2
        with:
          files: Fabric-${{ github.ref_name }}.dmg
```

### Key CI considerations

1. **Keychain management:** CI creates a temporary keychain, imports the certificate, and cleans up after the job
2. **Hardened runtime:** Always pass `--options runtime` during signing
3. **Timestamp:** Always include `--timestamp` for notarization
4. **Cache notarytool credentials:** `store-credentials` can be done once per CI run if the profile is saved
5. **Universal binary:** If building for both x86_64 and aarch64, use `lipo` to create a fat binary before bundling:
   ```bash
   lipo -create target-x86_64/release/fabric-gui \
     target-aarch64/release/fabric-gui \
     -output Fabric.app/Contents/MacOS/fabric-gui
   ```

---

## 9. Quick Reference

### Full signing + notarization + DMG pipeline

```bash
# 1. Build
cargo build --release -p fabric-gui

# 2. Create .app bundle
./crates/fabric-gui/bundle/macos/build-app.sh

# 3. Sign with Developer ID + hardened runtime + entitlements
./crates/fabric-gui/bundle/macos/sign-and-notarize.sh \
  --developer-id "Developer ID Application: Phenotype (TEAMID)" \
  --apple-id "you@example.com" \
  --team-id "TEAMID" \
  --password "xxxx-xxxx-xxxx-xxxx"

# 4. Create DMG
./crates/fabric-gui/bundle/macos/create-dmg.sh \
  --app Fabric.app \
  --output Fabric-v0.1.0.dmg

# 5. Sign the DMG
codesign --force --sign "Developer ID Application: Phenotype (TEAMID)" \
  --timestamp \
  Fabric-v0.1.0.dmg
```

### Verifying everything worked

```bash
# Verify .app signature
codesign --verify --verbose=2 Fabric.app

# Verify notarization
spctl --assess --type execute Fabric.app

# Verify DMG signature
codesign --verify --verbose=2 Fabric-v0.1.0.dmg

# Check stapled ticket
xcrun stapler validate Fabric.app

# Full quarantine simulation
xattr -r -w com.apple.quarantine "0081;\x$(date +%s);Safari;" Fabric.app
open Fabric.app  # Should pass Gatekeeper
```

---

## 10. Troubleshooting

| Problem | Solution |
|---------|----------|
| `errSecInternalComponent` | Keychain is locked or certificate not trusted. Unlock keychain and re-import. |
| App killed on launch | Missing entitlements or hardened runtime not enabled. Check with `codesign -d --entitlements -`. |
| "Cannot be opened because the developer cannot be verified" | Notarization failed or ticket not stapled. Run `xcrun notarytool log` to check. |
| `The executable does not have the hardened runtime enabled` | Re-sign with `--options runtime`. |
| DMG won't mount after signing | DMG was signed after creation with wrong flags. Re-create DMG, then sign. |
| Gatekeeper blocks app with quarantine | Expected for unnotarized apps. Notarize and staple to fix. |
| `no identity found` for codesign | Certificate not in keychain. Import the `.p12` file. |
| JIT compilation fails under hardened runtime | Ensure `com.apple.security.cs.allow-jit` entitlement is present. |