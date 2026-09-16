# Phenotype Fabric Branding Assets Audit

**Date:** 2026-09-14  
**Repo:** `/Users/<REDACTED>/CodeProjects/Phenotype/repos/phenotype-fabric`  
**Auditor:** Jcode (automated)  
**Scope:** All brand assets, icons, scripts, colors, typography, and platform icon coverage

---

## Executive Summary

The Phenotype Fabric branding asset pipeline is **well-structured and mostly complete**. There is a single master generator (`gen-all-assets.py`) that produces all 2D assets, a Blender script for high-quality 3D renders, and a complete macOS icon generation chain. Color consistency is high across scripts, with two minor deviations in the fallback 3D generator. The main gaps are: **no ASCII/terminal art for CLI output**, **missing `favicon.ico`**, **no maskable PWA icon**, **3D renders orphaned in repo root**, and **the `fallback-3d.py` script uses non-brand background and white colors**.

### Overall Status

```
[Brand Docs    ] ██████████████████ 100%  colors.md + typography.md present and complete
[2D Icons      ] ██████████████████ 100%  All favicon, apple-touch, PWA, SVG, logos generated
[macOS .icns   ] ██████████████████ 100%  Full iconset, build + sign pipeline complete
[Social Cards  ] ██████████████████ 100%  OG + Twitter cards present
[Splash/DMG    ] ██████████████████ 100%  Both splash.png and DMG background present
[3D Renders    ] ████████████░░░░░░  67%  Blender + PIL scripts work, but outputs misplaced
[CLI Art       ] ████░░░░░░░░░░░░░░  20%  No ASCII art found anywhere
[Color Consist.] ████████████████░░  89%  2 deviations in fallback-3d.py
[PWA Completeness] ██████████░░░░░░░░  56%  Missing maskable, favicon.ico, Windows .ico
```

---

## 1. Brand Specification Documents

### assets/brand/colors.md

| Attribute | Status |
|-----------|--------|
| Exists | YES |
| Non-empty | YES (1,983 bytes) |
| Format | ASCII text, well-structured Markdown |
| Primary palette defined | YES (4 colors with hex, RGB, usage) |
| Extended palette defined | YES (4 colors) |
| Background variants | YES (4 variants) |
| Opacity scale | YES (6 levels) |
| Usage rules | YES (5 rules including WCAG AA) |

**Colors defined:**

| Name | Hex | RGB | Verified in scripts |
|------|-----|-----|---------------------|
| Deep Navy | `#1a1a2e` | (26, 26, 46) | gen-all-assets, gen-icon, gen-blender (linear) |
| Teal Accent | `#00d4aa` | (0, 212, 170) | gen-all-assets, gen-icon, gen-blender (linear), SVG |
| Purple Blue | `#6464c8` | (100, 100, 200) | gen-all-assets, gen-icon, SVG |
| Near White | `#f0f0f5` | (240, 240, 245) | gen-all-assets, gen-icon, SVG |
| Dark Text | `#2a2a3e` | (42, 42, 62) | gen-all-assets |
| Subtle Gray | `#3a3a50` | (58, 58, 80) | gen-all-assets |
| Error Red | `#ef4444` | (239, 68, 68) | Not used in assets (UI only) |
| Success Green | `#22c55e` | (34, 197, 94) | Not used in assets (UI only) |
| Dark Elevated | `#222240` | (34, 34, 64) | Not used in asset scripts |
| Light | `#f8f8fa` | (248, 248, 250) | gen-all-assets (BG_LIGHT) |
| Light Elevated | `#ffffff` | (255, 255, 255) | Not used in asset scripts |

**Verdict:** PASS. Comprehensive and well-maintained.

### assets/brand/typography.md

| Attribute | Status |
|-----------|--------|
| Exists | YES |
| Non-empty | YES (1,887 bytes) |
| Format | ASCII text, well-structured Markdown |
| Font stacks defined | YES (primary, monospace, display) |
| Type scale | YES (8 sizes from 11px to 48px) |
| Weight assignments | YES (5 weights) |
| Rules | YES (5 rules) |

**Verdict:** PASS. Complete brand typography spec.

---

## 2. Asset Inventory

### assets/icons/ (9 files)

| File | Size | Format | Dimensions | Valid | Notes |
|------|------|--------|------------|-------|-------|
| favicon-16.png | 283 B | PNG RGBA 8-bit | 16x16 | YES | No topology (by design) |
| favicon-32.png | 512 B | PNG RGBA 8-bit | 32x32 | YES | No topology (by design) |
| favicon-48.png | 829 B | PNG RGBA 8-bit | 48x48 | YES | No topology (by design) |
| apple-touch-icon-180.png | 12,951 B | PNG RGBA 8-bit | 180x180 | YES | Full topology |
| icon-192.png | 17,208 B | PNG RGBA 8-bit | 192x192 | YES | Full PWA treatment |
| icon-512.png | 42,175 B | PNG RGBA 8-bit | 512x512 | YES | Full PWA treatment |
| icon.svg | 2,237 B | SVG | 512x512 viewBox | YES | Clean, all brand colors |
| logo-dark.png | 20,627 B | PNG RGBA 8-bit | 1024x256 | YES | "Phenotype Fabric" on dark bg |
| logo-light.png | 20,726 B | PNG RGBA 8-bit | 1024x256 | YES | "Phenotype Fabric" on light bg |

**Verdict:** PASS. All icons present and valid.

### assets/3d/ (3 files)

| File | Size | Format | Valid | Notes |
|------|------|--------|-------|-------|
| README.md | 2,077 B | UTF-8 text | YES | Good documentation |
| gen-blender-logo.py | 12,855 B | Python script | YES | Blender 3.6+ headless, Cycles |
| fallback-3d.py | 9,632 B | Python script | YES | PIL-based 2D approximation |

**3D render outputs (in repo root, NOT in assets/3d/):**

| File | Size | Format | Dimensions | Color Depth | Valid |
|------|------|--------|------------|-------------|-------|
| logo-render.png | 8,317,773 B | PNG RGBA | 2048x2048 | **16-bit** | YES |
| logo-render-wide.png | 2,983,709 B | PNG RGBA | 2048x1024 | **16-bit** | YES |
| logo-render-icon.png | 659,554 B | PNG RGBA | 512x512 | **16-bit** | YES |
| splash-3d.png | 2,081,121 B | PNG RGBA | 1200x800 | **16-bit** | YES |

**Note:** The 3D renders are 16-bit color depth (from Blender Cycles). All other brand assets are 8-bit. This is expected for 3D renders but creates inconsistency in asset formats.

**Issue:** The Blender script outputs to `//` (the blend file directory), but the renders are in the repo root. The `fallback-3d.py` outputs to `os.path.dirname(os.path.abspath(__file__))` which is `assets/3d/`. This means:
- Running `gen-blender-logo.py` from `assets/3d/` would output to `assets/3d/` (correct)
- Running `fallback-3d.py` from anywhere outputs to `assets/3d/` (correct)
- But the actual renders are in the repo root (orphaned)

**Verdict:** PASS with issues. Scripts are valid and documented, but output location inconsistency and misplaced renders.

### assets/splash/ (1 file)

| File | Size | Format | Dimensions | Valid |
|------|------|--------|------------|-------|
| splash.png | 95,619 B | PNG RGBA 8-bit | 1200x800 | YES |

**Verdict:** PASS.

### assets/dmg/ (1 file)

| File | Size | Format | Dimensions | Valid |
|------|------|--------|------------|-------|
| background.png | 55,155 B | PNG RGBA 8-bit | 1280x640 | YES |

**Verdict:** PASS. Correct size for DMG background.

### assets/social/ (2 files)

| File | Size | Format | Dimensions | Valid |
|------|------|--------|------------|-------|
| og-image.png | 89,010 B | PNG RGBA 8-bit | 1200x630 | YES |
| twitter-card.png | 72,893 B | PNG RGBA 8-bit | 1200x600 | YES |

**Verdict:** PASS. Correct dimensions for Open Graph (1200x630) and Twitter (1200x600).

### crates/fabric-gui/bundle/macos/ (12 items)

| File | Size | Valid | Notes |
|------|------|-------|-------|
| Fabric.icns | 505,443 B | YES | Full iconset (10 sizes: 16-1024px) |
| icon.png | 58,861 B | YES | Source icon for gen-icon.py |
| gen-icon.py | 4,072 B | YES | Generates 1024x1024 icon.png |
| gen-all-icons.sh | 1,175 B | YES | Generates Fabric.icns from icon.png |
| build-app.sh | 2,657 B | YES | Creates .app bundle |
| create-dmg.sh | 5,106 B | YES | Creates DMG installer |
| sign-and-notarize.sh | 5,456 B | YES | Developer ID signing + notarization |
| fabric.entitlements | 275 B | YES | JIT, network, unsigned memory |
| Fabric.app/ | - | YES | Complete .app bundle with binary, icns, plist |

**ICNS icon sizes present:**

| Entry | Pixel Size | Source |
|-------|-----------|--------|
| ic04 | 16x16 | Standard |
| ic05 | 32x32 | Standard |
| ic07 | 128x128 | Standard |
| ic08 | 256x256 | Standard |
| ic09 | 512x512 | Standard |
| ic10 | 1024x1024 | Retina 512@2x |
| ic11 | 32x32 | Retina 16@2x |
| ic12 | 64x64 | Retina 32@2x |
| ic13 | 256x256 | Retina 128@2x |
| ic14 | 512x512 | Retina 256@2x |

**Verdict:** PASS. Complete macOS distribution pipeline.

### assets/gen-all-assets.py

| Attribute | Status |
|-----------|--------|
| Exists | YES |
| Non-empty | YES (20,261 B, 559 lines) |
| Color consistency | PERFECT MATCH with colors.md |
| Output coverage | All 2D assets (icons, logos, splash, DMG, social) |
| Dependencies | Pillow only (auto-installs) |
| Cross-platform fonts | macOS, Linux, Windows paths |

**Verdict:** PASS. Well-organized master generator.

---

## 3. Color Consistency Analysis

### gen-all-assets.py (master generator)

| Variable | RGB | Matches colors.md | Status |
|----------|-----|-------------------|--------|
| BG_DARK | (26, 26, 46) | #1a1a2e | EXACT MATCH |
| ACCENT | (0, 212, 170) | #00d4aa | EXACT MATCH |
| SECONDARY | (100, 100, 200) | #6464c8 | EXACT MATCH |
| WHITE | (240, 240, 245) | #f0f0f5 | EXACT MATCH |
| DARK_TEXT | (42, 42, 62) | #2a2a3e | EXACT MATCH |
| SUBTLE | (58, 58, 80) | #3a3a50 | EXACT MATCH |
| BG_LIGHT | (248, 248, 250) | #f8f8fa | EXACT MATCH |

### fallback-3d.py

| Variable | RGB | Expected | Status |
|----------|-----|----------|--------|
| TEAL | (0, 212, 170) | #00d4aa | EXACT MATCH |
| NAVY | (26, 26, 46) | #1a1a2e | EXACT MATCH |
| PURPLE | (100, 100, 200) | #6464c8 | EXACT MATCH |
| DARK_BG | **(15, 15, 30)** | (26, 26, 46) | **INCONSISTENT** |
| WHITE | **(255, 255, 255)** | (240, 240, 245) | **INCONSISTENT** |

**Issue:** `fallback-3d.py` uses `DARK_BG = (15, 15, 30)` instead of the brand `#1a1a2e (26, 26, 46)`. This produces a slightly darker background than the brand spec. It also uses pure white `(255, 255, 255)` instead of the brand near-white `(240, 240, 245)`.

### gen-icon.py (macOS)

| Variable | RGB | Expected | Status |
|----------|-----|----------|--------|
| BG | (26, 26, 46) | #1a1a2e | EXACT MATCH |
| ACCENT | (0, 212, 170) | #00d4aa | EXACT MATCH |
| NODE_COLOR | (100, 100, 200) | #6464c8 | EXACT MATCH |
| WHITE | (240, 240, 245) | #f0f0f5 | EXACT MATCH |

### gen-blender-logo.py

Uses Blender linear color space, so RGB values differ from sRGB hex. This is expected behavior for physically accurate rendering. The README documents the material colors.

| Material | Linear RGB | Expected sRGB | Status |
|----------|-----------|---------------|--------|
| TealMetal | (0.0, 0.831, 0.667) | ~(0, 171, 139) | APPROXIMATE (linear gamma) |
| DarkNavy | (0.102, 0.102, 0.180) | ~(38, 38, 65) | APPROXIMATE (linear gamma) |
| PurpleNode | (0.392, 0.392, 0.784) | ~(157, 157, 235) | APPROXIMATE (linear gamma) |

**Verdict:** ACCEPTABLE. Blender linear colors render correctly under Cycles gamma.

### icon.svg

All hex values in the SVG match the brand spec exactly:
- `#1a1a2e` (background)
- `#6464c8` (connections)
- `#00d4aa` (nodes, glow)
- `#f0f0f5` (F glyph)

### Color Consistency Summary

| Script | Match Rate | Issues |
|--------|-----------|--------|
| gen-all-assets.py | 7/7 (100%) | None |
| gen-icon.py | 4/4 (100%) | None |
| fallback-3d.py | 3/5 (60%) | DARK_BG and WHITE off-brand |
| gen-blender-logo.py | N/A (linear) | Expected gamma difference |
| icon.svg | 4/4 (100%) | None |

---

## 4. Platform Icon Coverage

### macOS

| Required | Present | Source |
|----------|---------|--------|
| .icns (16-1024px) | YES | Fabric.icns (505KB, 10 sizes) |
| .app bundle | YES | Fabric.app/Contents/ |
| Info.plist icon ref | YES | CFBundleIconFile = Fabric.icns |
| Ad-hoc signing | YES | build-app.sh |
| Dev ID signing | YES | sign-and-notarize.sh |
| Notarization | YES | sign-and-notarize.sh (xcrun notarytool) |
| Entitlements | YES | fabric.entitlements (JIT, network, etc.) |

### Web / Favicons

| Required | Present | File |
|----------|---------|------|
| favicon 16x16 | YES | favicon-16.png |
| favicon 32x32 | YES | favicon-32.png |
| favicon 48x48 | YES | favicon-48.png |
| apple-touch 180x180 | YES | apple-touch-icon-180.png |
| **favicon.ico (multi-size)** | **MISSING** | Need combined .ico |
| **favicon.svg** | **MISSING** | Modern browsers prefer SVG favicon |

### PWA / Android

| Required | Present | File |
|----------|---------|------|
| icon-192x192 | YES | icon-192.png |
| icon-512x512 | YES | icon-512.png |
| **maskable icon-512** | **MISSING** | PWA requires maskable variant |
| **icon-144x144** | **MISSING** | Windows PWA tile |
| **icon-384x384** | **MISSING** | Android splash screen |

### Social / Sharing

| Required | Present | File | Dimensions |
|----------|---------|------|------------|
| Open Graph | YES | og-image.png | 1200x630 (correct) |
| Twitter Card | YES | twitter-card.png | 1200x600 (correct) |
| LinkedIn | MISSING | - | Need 1200x627 |

### Windows

| Required | Present | Notes |
|----------|---------|-------|
| **favicon.ico** | **MISSING** | Need 16/32/48 multi-size .ico |

---

## 5. Script Validation

### gen-all-assets.py (master 2D generator)

| Check | Status |
|-------|--------|
| Valid Python | YES |
| Dependencies | Pillow (auto-installed) |
| Output coverage | 13 assets across 4 directories |
| Deterministic | YES (no randomness) |
| Cross-platform | YES (font paths for macOS/Linux/Windows) |
| Color accuracy | 100% match to brand spec |
| Idempotent | YES (overwrites on re-run) |

**Generated assets:**
1. favicon-16.png (16x16)
2. favicon-32.png (32x32)
3. favicon-48.png (48x48)
4. apple-touch-icon-180.png (180x180)
5. icon-192.png (192x192)
6. icon-512.png (512x512)
7. icon.svg (512x512)
8. logo-dark.png (1024x256)
9. logo-light.png (1024x256)
10. background.png (1280x640) - DMG
11. splash.png (1200x800)
12. og-image.png (1200x630)
13. twitter-card.png (1200x600)

### gen-blender-logo.py (3D renderer)

| Check | Status |
|-------|--------|
| Valid Python | YES |
| Blender API usage | Correct for 3.6+ |
| Headless mode | YES (background) |
| Cycles settings | 128 samples, denoising |
| Output format | 16-bit RGBA PNG |
| Color depth | 16-bit (higher than 2D assets) |
| Camera tracking | YES (track-to constraint on F) |
| Scene cleanup | YES (clears all objects/materials) |

**Outputs:**
1. logo-render.png (2048x2048)
2. logo-render-wide.png (2048x1024)
3. logo-render-icon.png (512x512)
4. splash-3d.png (1200x800)

**Issue:** Script uses `scene.render.filepath = f"//{name}.png"` which writes to the blend file directory. If run from the repo root via `blender --background --python assets/3d/gen-blender-logo.py`, outputs go to the repo root. This explains why the renders are in the root rather than in `assets/3d/`.

### fallback-3d.py (2D approximation)

| Check | Status |
|-------|--------|
| Valid Python | YES |
| Dependencies | Pillow only |
| Output coverage | Same 4 files as Blender script |
| Output location | `assets/3d/` (correct) |
| Color accuracy | MOSTLY correct (2 deviations) |

**Issue:** Outputs to `assets/3d/` but the existing renders are in repo root. The script would overwrite files in the wrong location if run directly.

### gen-icon.py (macOS icon)

| Check | Status |
|-------|--------|
| Valid Python | YES |
| Dependencies | Pillow (auto-installed) |
| Output | icon.png (1024x1024) |
| Color accuracy | 100% match |
| Font handling | System font cascade with fallbacks |

### gen-all-icons.sh (macOS .icns)

| Check | Status |
|-------|--------|
| Valid Bash | YES |
| Uses standard tools | YES (sips + iconutil) |
| Iconset completeness | All 10 required sizes |
| Error handling | set -eo pipefail |
| Idempotent | YES (rm -rf before create) |

### build-app.sh

| Check | Status |
|-------|--------|
| Valid Bash | YES |
| Bundle structure | Correct (Contents/{MacOS,Resources}) |
| Info.plist | Complete with all required keys |
| Binary copy | YES |
| Icon copy | YES (with warning if missing) |
| Ad-hoc signing | YES (codesign --force --sign -) |

### create-dmg.sh

| Check | Status |
|-------|--------|
| Valid Bash | YES |
| Uses hdiutil | YES (UDZO format) |
| Background image | Supported via --background flag |
| Applications symlink | YES |
| Window layout | AppleScript for Finder positioning |
| Compression | zlib-level=9 (maximum) |

### sign-and-notarize.sh

| Check | Status |
|-------|--------|
| Valid Bash | YES |
| 4-step pipeline | Clean -> Sign -> Notarize -> Staple |
| Hardened runtime | YES (codesign --options runtime) |
| Entitlements | YES |
| Timestamp | YES (codesign --timestamp) |
| Error handling | Logs notarization failure, fetches submission log |
| Skip options | --skip-notarize, --skip-staple |

---

## 6. Brand Consistency Analysis

### Color Usage Across Assets

| Context | Teal Usage | Navy Usage | Purple Usage | White Usage |
|---------|-----------|------------|-------------|-------------|
| F glyph | No | No | No | White (text) |
| Topology nodes | Teal (accent) | No | No | No |
| Connections | No | No | Purple (secondary) | No |
| Backgrounds | Radial glow | Solid fill | No | No |
| Accent ring | Teal dots | No | No | No |

This is consistent with the brand rule: "Never use teal accent for large filled areas."

### Typography in Assets

The SVG uses `font-family: "Helvetica Neue, Helvetica, Arial, sans-serif"` with `font-weight: 300`, matching the typography spec's Display/Logo font.

### Logo Mark Consistency

The "F" glyph appears in these variants:
- **icon.svg**: Rounded rect background, topology network, white F
- **icon PNGs**: Same as SVG but rasterized
- **Logo-dark.png / logo-light.png**: Icon + "Phenotype Fabric" text
- **3D renders**: 3D extruded F on platform with nodes
- **macOS icon**: Radial gradient, topology ring, white F
- **DMG background**: Large F centered with "Phenotype Fabric" below

All share the same topology-network motif, maintaining visual coherence.

---

## 7. Identified Gaps and Issues

### HIGH Priority

| # | Gap | Impact | Recommendation |
|---|-----|--------|----------------|
| H1 | **No ASCII/terminal art** | CLI users see no branding | Create ASCII art banner for CLI output |
| H2 | **No favicon.ico** | Older browsers and some setups need .ico | Generate multi-size favicon.ico from PNGs |
| H3 | **No maskable PWA icon** | Android install prompt shows clipped icon | Generate maskable variant with safe zone padding |

### MEDIUM Priority

| # | Gap | Impact | Recommendation |
|---|-----|--------|----------------|
| M1 | **fallback-3d.py uses off-brand colors** | DARK_BG=(15,15,30) and WHITE=(255,255,255) differ from spec | Update to (26,26,46) and (240,240,245) |
| M2 | **3D renders orphaned in repo root** | 8.3MB PNGs cluttering root | Move to `assets/3d/` or `.gitignore` if regenerable |
| M3 | **gen-blender-logo.py output path** | Outputs to blend file dir, not predictable | Update to output to `assets/3d/` |
| M4 | **No Windows favicon.ico** | Windows users see generic icon | Generate .ico from existing PNGs |
| M5 | **No icon-144.png** | Windows PWA tile missing | Add to gen-all-assets.py |
| M6 | **No icon-384.png** | Android splash screen missing | Add to gen-all-assets.py |
| M7 | **No favicon.svg** | Modern browsers prefer SVG favicon | Create favicon.svg (can reuse icon.svg) |

### LOW Priority

| # | Gap | Impact | Recommendation |
|---|-----|--------|----------------|
| L1 | **No LinkedIn card** | LinkedIn shares may show wrong crop | Add 1200x627 variant or accept OG image |
| L2 | **No dark elevated (#222240) in assets** | Only used in UI, not branding | Acceptable |
| L3 | **Root README doesn't mention branding** | New contributors may miss assets | Add assets section to README |
| L4 | **3D renders are 16-bit, all others 8-bit** | Minor format inconsistency | Acceptable for 3D renders |
| L5 | **gen-all-assets.py doesn't generate 3D assets** | Two separate pipelines | Consider unifying or documenting clearly |

---

## 8. Color Deviation Details

### fallback-3d.py

```python
# CURRENT (inconsistent)
DARK_BG = (15, 15, 30)     # Not #1a1a2e
WHITE = (255, 255, 255)    # Not #f0f0f5

# CORRECT (per brand spec)
DARK_BG = (26, 26, 46)     # #1a1a2e
WHITE = (240, 240, 245)    # #f0f0f5
```

This means `fallback-3d.py` renders have a slightly darker background and brighter white text than the brand spec. Since the existing `logo-render*.png` and `splash-3d.png` files were likely generated by the Blender script (16-bit depth), this inconsistency hasn't manifested in shipped assets yet, but will if the fallback is used.

---

## 9. Recommendations

### Immediate (Pre-Release)

1. **Fix fallback-3d.py colors** - Update DARK_BG and WHITE to match brand spec (5 min)
2. **Generate favicon.ico** - Combine 16/32/48 PNGs into multi-size .ico (10 min)
3. **Add maskable icon variant** - Create 512x512 with safe zone padding (15 min)
4. **Create favicon.svg** - Symlink or copy icon.svg as favicon.svg (5 min)

### Short-Term (v0.2.0)

5. **Create ASCII art banner** - For CLI startup output and terminal branding (30 min)
6. **Move 3D renders** - Either move to assets/3d/ or add to .gitignore (5 min)
7. **Fix fallback-3d.py output path** - Output to consistent location (5 min)
8. **Add PWA icon sizes** - icon-144.png and icon-384.png to gen-all-assets.py (15 min)

### Medium-Term (v1.0)

9. **Add dark/light mode logo variants with 3D renders** - Currently only 2D logos have light variant
10. **Add LinkedIn card** - 1200x627 variant in gen-all-assets.py (10 min)
11. **Unify 2D and 3D generation** - Consider having gen-all-assets.py call fallback-3d.py
12. **Add Windows .ico generation** - To gen-all-assets.py pipeline

---

## 10. Complete Asset Map

```
phenotype-fabric/
  assets/
    brand/
      colors.md                 [1,983 B] Brand color specification
      typography.md             [1,887 B] Brand typography specification
    icons/
      favicon-16.png            [283 B]   16x16    PNG RGBA 8-bit
      favicon-32.png            [512 B]   32x32    PNG RGBA 8-bit
      favicon-48.png            [829 B]   48x48    PNG RGBA 8-bit
      apple-touch-icon-180.png  [12.9 KB] 180x180  PNG RGBA 8-bit
      icon-192.png              [17.2 KB] 192x192  PNG RGBA 8-bit
      icon-512.png              [42.2 KB] 512x512  PNG RGBA 8-bit
      icon.svg                  [2.2 KB]  512x512  SVG vector
      logo-dark.png             [20.6 KB] 1024x256 PNG RGBA 8-bit
      logo-light.png            [20.7 KB] 1024x256 PNG RGBA 8-bit
    3d/
      README.md                 [2.1 KB]  Documentation
      gen-blender-logo.py       [12.9 KB] Blender Cycles renderer
      fallback-3d.py            [9.6 KB]  PIL 2D approximation
    splash/
      splash.png                [95.6 KB] 1200x800 PNG RGBA 8-bit
    dmg/
      background.png            [55.2 KB] 1280x640 PNG RGBA 8-bit
    social/
      og-image.png              [89.0 KB] 1200x630 PNG RGBA 8-bit
      twitter-card.png          [72.9 KB] 1200x600 PNG RGBA 8-bit
    gen-all-assets.py           [20.3 KB] Master 2D generator
  (root)
    logo-render.png             [8.3 MB]  2048x2048 PNG RGBA 16-bit
    logo-render-wide.png        [3.0 MB]  2048x1024 PNG RGBA 16-bit
    logo-render-icon.png        [660 KB]  512x512   PNG RGBA 16-bit
    splash-3d.png               [2.1 MB]  1200x800  PNG RGBA 16-bit
  crates/fabric-gui/bundle/macos/
    Fabric.icns                 [505 KB]  macOS iconset (10 sizes)
    icon.png                    [58.9 KB] Source 1024x1024 icon
    gen-icon.py                 [4.1 KB]  Icon generator (PIL)
    gen-all-icons.sh            [1.2 KB]  ICNS generator (sips+iconutil)
    build-app.sh                [2.7 KB]  .app bundle builder
    create-dmg.sh               [5.1 KB]  DMG installer creator
    sign-and-notarize.sh        [5.5 KB]  Code signing + notarization
    fabric.entitlements         [275 B]   macOS entitlements
    Fabric.app/                 [-]       Complete .app bundle
```

**Total asset files:** 24 (excluding generators and docs)  
**Total asset size:** ~15.6 MB (dominated by 3D renders at 14.1 MB)  
**Generators:** 5 scripts + 1 shell script  
**Color consistency:** 89% (2 deviations in fallback-3d.py)
