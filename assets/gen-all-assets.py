#!/usr/bin/env python3
"""Generate ALL brand assets for Phenotype Fabric.

Produces: favicons, apple-touch-icon, PWA icons, SVG logo, dark/light logos,
DMG background, splash screen, and social cards from brand colors.

Usage:
    python3 assets/gen-all-assets.py
"""
import math
import os
import sys

# ---------------------------------------------------------------------------
# Colour palette (RGBA tuples)
# ---------------------------------------------------------------------------
BG_DARK = (26, 26, 46)          # #1a1a2e  primary background
ACCENT = (0, 212, 170)          # #00d4aa  teal/cyan accent
SECONDARY = (100, 100, 200)     # #6464c8  purple-blue
WHITE = (240, 240, 245)         # #f0f0f5  near white text
DARK_TEXT = (42, 42, 62)        # #2a2a3e  dark text
SUBTLE = (58, 58, 80)           # #3a3a50  borders / inactive
LINE_COLOR = (80, 80, 140, 100) # connections
BG_LIGHT = (248, 248, 250)      # #f8f8fa  light background

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _ensure_pillow():
    """Import Pillow, installing it automatically if missing."""
    try:
        from PIL import Image, ImageDraw, ImageFont, ImageFilter
        return Image, ImageDraw, ImageFont, ImageFilter
    except ImportError:
        print("  [setup] Installing Pillow ...")
        os.system(f"{sys.executable} -m pip install Pillow -q")
        from PIL import Image, ImageDraw, ImageFont, ImageFilter
        return Image, ImageDraw, ImageFont, ImageFilter


Image = ImageDraw = ImageFont = ImageFilter = None  # filled by _ensure_pillow


def _load_font(size: int):
    """Try system fonts, return best available."""
    candidates = [
        "/System/Library/Fonts/SFCompact.ttf",
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/Supplemental/Helvetica.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
        "C:/Windows/Fonts/arial.ttf",
    ]
    for fp in candidates:
        if os.path.exists(fp):
            try:
                return ImageFont.truetype(fp, size)
            except Exception:
                continue
    try:
        return ImageFont.truetype("Arial", size)
    except Exception:
        return ImageFont.load_default()


def _draw_topology(draw, w, h, cx, cy, ring_r, inner_r, count_outer=8, count_inner=6, alpha_base=100, width=2):
    """Draw the signature topology network (nodes + connections)."""
    import random
    rng = random.Random(42)  # deterministic
    nodes = []
    # Outer ring
    for i in range(count_outer):
        angle = 2 * math.pi * i / count_outer - math.pi / 2
        x = cx + int(ring_r * math.cos(angle))
        y = cy + int(ring_r * math.sin(angle))
        nodes.append((x, y))
    # Inner ring
    for i in range(count_inner):
        angle = 2 * math.pi * i / count_inner
        x = cx + int(inner_r * math.cos(angle))
        y = cy + int(inner_r * math.sin(angle))
        nodes.append((x, y))
    # Connections
    for i, (x1, y1) in enumerate(nodes):
        for j, (x2, y2) in enumerate(nodes):
            if j <= i:
                continue
            dist = math.sqrt((x2 - x1) ** 2 + (y2 - y1) ** 2)
            max_dist = ring_r * 1.3
            if dist < max_dist:
                a = max(20, int(alpha_base * (1 - dist / max_dist)))
                draw.line([(x1, y1), (x2, y2)], fill=(*SECONDARY, a), width=width)
    # Nodes
    for i, (x, y) in enumerate(nodes):
        r = 14 if i < count_outer else 9
        # Glow
        for gr in range(r + 16, r, -1):
            ga = int(50 * (1 - (gr - r) / 16))
            draw.ellipse([x - gr, y - gr, x + gr, y + gr], fill=(*ACCENT, ga))
        draw.ellipse([x - r, y - r, x + r, y + r], fill=(*ACCENT, 200))
    return nodes


def _draw_f(draw, cx, cy, size, font_size, fill=None, shadow=True):
    """Draw the stylised 'F' glyph centred at (cx, cy)."""
    fill = fill or (*WHITE, 240)
    font = _load_font(font_size)
    bbox = draw.textbbox((0, 0), "F", font=font)
    tw = bbox[2] - bbox[0]
    th = bbox[3] - bbox[1]
    tx = cx - tw // 2 - bbox[0]
    ty = cy - th // 2 - bbox[1]
    if shadow:
        draw.text((tx + 5, ty + 5), "F", fill=(0, 0, 0, 100), font=font)
    draw.text((tx, ty), "F", fill=fill, font=font)


def _draw_radial_gradient(img, cx, cy, max_r, color, max_alpha=20):
    """Subtle radial glow overlay."""
    overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(overlay)
    for r in range(max_r, 0, -3):
        a = int(max_alpha * (1 - r / max_r))
        d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(*color, a))
    return Image.alpha_composite(img, overlay)


def _draw_background_pattern(img, alpha_factor=1.0):
    """Draw the subtle topology background pattern on the given image."""
    overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(overlay)
    w, h = img.size
    cx, cy = w // 2, h // 2
    _draw_topology(d, w, h, cx, cy,
                   ring_r=int(w * 0.35),
                   inner_r=int(w * 0.2),
                   count_outer=10, count_inner=8,
                   alpha_base=int(70 * alpha_factor), width=max(1, w // 500))
    return Image.alpha_composite(img, overlay)


# ---------------------------------------------------------------------------
# Asset generators
# ---------------------------------------------------------------------------

def gen_icon(size, out_path, include_topology=True):
    """Generate a square icon at the given size."""
    img = Image.new("RGBA", (size, size), (*BG_DARK, 255))
    cx, cy = size // 2, size // 2
    img = _draw_radial_gradient(img, cx, cy, size // 2, ACCENT, max_alpha=18)

    if include_topology:
        overlay = Image.new("RGBA", (size, size), (0, 0, 0, 0))
        d = ImageDraw.Draw(overlay)
        _draw_topology(d, size, size, cx, cy,
                       ring_r=int(size * 0.38),
                       inner_r=int(size * 0.22),
                       count_outer=8, count_inner=6,
                       alpha_base=80, width=max(1, size // 400))
        img = Image.alpha_composite(img, overlay)

    draw = ImageDraw.Draw(img)
    _draw_f(draw, cx, cy, size, int(size * 0.48))
    img.save(out_path, "PNG")
    print(f"  {os.path.basename(out_path):>30s}  {size}x{size}")


def gen_apple_touch_icon(out_path):
    """180x180 Apple Touch Icon with gradient."""
    size = 180
    img = Image.new("RGBA", (size, size), (*BG_DARK, 255))
    cx, cy = size // 2, size // 2
    img = _draw_radial_gradient(img, cx, cy, size // 2, ACCENT, max_alpha=24)
    overlay = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(overlay)
    _draw_topology(d, size, size, cx, cy,
                   ring_r=int(size * 0.38), inner_r=int(size * 0.2),
                   count_outer=6, count_inner=4, alpha_base=60, width=1)
    img = Image.alpha_composite(img, overlay)
    draw = ImageDraw.Draw(img)
    _draw_f(draw, cx, cy, size, int(size * 0.48))
    img.save(out_path, "PNG")
    print(f"  {'apple-touch-icon-180.png':>30s}  {size}x{size}")


def gen_pwa_icon(size, out_path):
    """PWA icon with full brand treatment."""
    img = Image.new("RGBA", (size, size), (*BG_DARK, 255))
    cx, cy = size // 2, size // 2
    img = _draw_radial_gradient(img, cx, cy, size // 2, ACCENT, max_alpha=20)

    overlay = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    d = ImageDraw.Draw(overlay)
    _draw_topology(d, size, size, cx, cy,
                   ring_r=int(size * 0.36), inner_r=int(size * 0.2),
                   count_outer=10, count_inner=8, alpha_base=80, width=max(1, size // 500))
    img = Image.alpha_composite(img, overlay)

    # Outer accent ring
    ring_overlay = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    rd = ImageDraw.Draw(ring_overlay)
    ring_r = int(size * 0.46)
    for deg in range(360):
        angle = math.radians(deg)
        x = cx + int(ring_r * math.cos(angle))
        y = cy + int(ring_r * math.sin(angle))
        if deg % 3 == 0:
            rd.ellipse([x - 2, y - 2, x + 2, y + 2], fill=(*ACCENT, 50))
    img = Image.alpha_composite(img, ring_overlay)

    draw = ImageDraw.Draw(img)
    _draw_f(draw, cx, cy, size, int(size * 0.46))
    img.save(out_path, "PNG")
    print(f"  {os.path.basename(out_path):>30s}  {size}x{size}")


def gen_svg(out_path):
    """Generate an SVG version of the logo."""
    svg = """\
<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
  <!-- Background -->
  <rect width="512" height="512" rx="64" fill="#1a1a2e"/>

  <!-- Topology connections -->
  <g opacity="0.35" stroke="#6464c8" stroke-width="1.5" fill="none">
    <line x1="256" y1="116" x2="360" y2="186"/>
    <line x1="256" y1="116" x2="152" y2="186"/>
    <line x1="360" y1="186" x2="380" y2="310"/>
    <line x1="152" y1="186" x2="132" y2="310"/>
    <line x1="256" y1="116" x2="380" y2="310"/>
    <line x1="256" y1="116" x2="132" y2="310"/>
    <line x1="380" y1="310" x2="256" y2="396"/>
    <line x1="132" y1="310" x2="256" y2="396"/>
    <line x1="360" y1="186" x2="256" y2="396"/>
    <line x1="152" y1="186" x2="256" y2="396"/>
    <line x1="380" y1="310" x2="152" y2="186"/>
    <line x1="132" y1="310" x2="360" y2="186"/>
    <!-- Inner connections -->
    <line x1="200" y1="196" x2="312" y2="196"/>
    <line x1="312" y1="196" x2="312" y2="316"/>
    <line x1="312" y1="316" x2="200" y2="316"/>
    <line x1="200" y1="316" x2="200" y2="196"/>
    <line x1="200" y1="196" x2="312" y2="316"/>
    <line x1="312" y1="196" x2="200" y2="316"/>
  </g>

  <!-- Topology nodes -->
  <g fill="#00d4aa" opacity="0.8">
    <!-- Outer ring -->
    <circle cx="256" cy="116" r="10"/>
    <circle cx="360" cy="186" r="10"/>
    <circle cx="380" cy="310" r="10"/>
    <circle cx="256" cy="396" r="10"/>
    <circle cx="132" cy="310" r="10"/>
    <circle cx="152" cy="186" r="10"/>
    <!-- Inner ring -->
    <circle cx="200" cy="196" r="7"/>
    <circle cx="312" cy="196" r="7"/>
    <circle cx="312" cy="316" r="7"/>
    <circle cx="200" cy="316" r="7"/>
  </g>

  <!-- Radial glow -->
  <defs>
    <radialGradient id="glow" cx="50%" cy="50%" r="50%">
      <stop offset="0%" stop-color="#00d4aa" stop-opacity="0.12"/>
      <stop offset="100%" stop-color="#00d4aa" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="512" height="512" rx="64" fill="url(#glow)"/>

  <!-- Stylised F -->
  <text x="256" y="310" text-anchor="middle" font-family="Helvetica Neue, Helvetica, Arial, sans-serif" font-size="260" font-weight="300" fill="#f0f0f5" opacity="0.92">F</text>
</svg>
"""
    with open(out_path, "w") as f:
        f.write(svg)
    print(f"  {'icon.svg':>30s}  vector")


def gen_logo(out_path, bg_color, text_color, w=1024, h=256):
    """Full logo: F icon + 'Phenotype Fabric' text on a given background."""
    img = Image.new("RGBA", (w, h), (*bg_color, 255))
    draw = ImageDraw.Draw(img)

    # --- Left: icon square ---
    icon_size = int(h * 0.8)
    icon_pad = int(h * 0.1)
    icon_x = icon_pad
    icon_y = icon_pad

    # Icon background
    draw.rounded_rectangle(
        [icon_x, icon_y, icon_x + icon_size, icon_y + icon_size],
        radius=int(icon_size * 0.18),
        fill=(*BG_DARK, 255),
    )

    # Mini topology on icon
    overlay = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    od = ImageDraw.Draw(overlay)
    cx = icon_x + icon_size // 2
    cy = icon_y + icon_size // 2
    _draw_topology(od, w, h, cx, cy,
                   ring_r=int(icon_size * 0.38),
                   inner_r=int(icon_size * 0.22),
                   count_outer=6, count_inner=4, alpha_base=50, width=1)
    img = Image.alpha_composite(img, overlay)
    draw = ImageDraw.Draw(img)

    # F glyph on icon
    _draw_f(draw, cx, cy, icon_size, int(icon_size * 0.50), fill=(*WHITE, 230))

    # --- Right: text ---
    text_x = icon_x + icon_size + int(h * 0.12)
    font_large = _load_font(int(h * 0.28))
    font_small = _load_font(int(h * 0.14))

    # "Phenotype"
    draw.text((text_x, int(h * 0.18)), "Phenotype", fill=(*text_color, 255), font=font_large)
    # "Fabric"
    draw.text((text_x, int(h * 0.52)), "Fabric", fill=(*ACCENT, 255), font=font_large)

    img.save(out_path, "PNG")
    label = os.path.basename(out_path)
    print(f"  {label:>30s}  {w}x{h}")


def gen_dmg_background(out_path):
    """1280x640 DMG installer background."""
    w, h = 1280, 640
    img = Image.new("RGBA", (w, h), (*BG_DARK, 255))
    img = _draw_background_pattern(img, alpha_factor=0.6)
    draw = ImageDraw.Draw(img)

    cx, cy = w // 2, h // 2

    # Large F glyph centred
    _draw_f(draw, cx, cy - 40, w, int(w * 0.22), shadow=True)

    # "Phenotype Fabric" below
    font_brand = _load_font(36)
    bbox = draw.textbbox((0, 0), "Phenotype Fabric", font=font_brand)
    tw = bbox[2] - bbox[0]
    draw.text((cx - tw // 2, cy + 100), "Phenotype Fabric", fill=(*WHITE, 200), font=font_brand)

    # --- Right side: Applications arrow hint ---
    arrow_x = w - 300
    arrow_y = h // 2 - 80

    # Dashed box representing Applications
    dash_font = _load_font(18)
    draw.rounded_rectangle(
        [arrow_x - 80, arrow_y - 60, arrow_x + 80, arrow_y + 60],
        radius=12,
        outline=(*SUBTLE, 150),
        width=2,
    )
    # "Applications" label inside box
    lbl = "Applications"
    lbbox = draw.textbbox((0, 0), lbl, font=dash_font)
    lw = lbbox[2] - lbbox[0]
    draw.text((arrow_x - lw // 2, arrow_y - 8), lbl, fill=(*SUBTLE, 180), font=dash_font)

    # Arrow from left to right
    arrow_start = cx + int(w * 0.15)
    arrow_end = arrow_x - 100
    arrow_cy = h // 2
    draw.line([(arrow_start, arrow_cy), (arrow_end, arrow_cy)],
              fill=(*ACCENT, 150), width=3)
    # Arrowhead
    draw.polygon([
        (arrow_end, arrow_cy),
        (arrow_end - 14, arrow_cy - 8),
        (arrow_end - 14, arrow_cy + 8),
    ], fill=(*ACCENT, 180))

    img.save(out_path, "PNG")
    print(f"  {'background.png (DMG)':>30s}  {w}x{h}")


def gen_splash(out_path, version="v0.1.0"):
    """1200x800 splash / about screen."""
    w, h = 1200, 800
    img = Image.new("RGBA", (w, h), (*BG_DARK, 255))
    img = _draw_background_pattern(img, alpha_factor=0.8)
    img = _draw_radial_gradient(img, w // 2, h // 2, int(h * 0.7), ACCENT, max_alpha=16)
    draw = ImageDraw.Draw(img)

    cx, cy = w // 2, h // 2

    # Large F
    _draw_f(draw, cx, cy - 60, w, int(w * 0.26), shadow=True)

    # "Phenotype Fabric"
    font_title = _load_font(52)
    title = "Phenotype Fabric"
    tbbox = draw.textbbox((0, 0), title, font=font_title)
    tw = tbbox[2] - tbbox[0]
    draw.text((cx - tw // 2, cy + 100), title, fill=(*WHITE, 240), font=font_title)

    # Version
    font_ver = _load_font(24)
    vbbox = draw.textbbox((0, 0), version, font=font_ver)
    vw = vbbox[2] - vbbox[0]
    draw.text((cx - vw // 2, cy + 170), version, fill=(*ACCENT, 200), font=font_ver)

    # Tagline
    font_tag = _load_font(18)
    tagline = "Graph-native workspace runtime"
    tgbbox = draw.textbbox((0, 0), tagline, font=font_tag)
    tgw = tgbbox[2] - tgbbox[0]
    draw.text((cx - tgw // 2, cy + 210), tagline, fill=(*SUBTLE, 200), font=font_tag)

    img.save(out_path, "PNG")
    print(f"  {'splash.png':>30s}  {w}x{h}")


def gen_og_image(out_path):
    """1200x630 Open Graph social card."""
    w, h = 1200, 630
    img = Image.new("RGBA", (w, h), (*BG_DARK, 255))
    img = _draw_background_pattern(img, alpha_factor=0.5)
    img = _draw_radial_gradient(img, w // 3, h // 2, int(h * 0.8), ACCENT, max_alpha=14)
    draw = ImageDraw.Draw(img)

    # --- Left side: icon ---
    icon_size = 200
    ix, iy = 80, (h - icon_size) // 2
    draw.rounded_rectangle(
        [ix, iy, ix + icon_size, iy + icon_size],
        radius=28,
        fill=(*BG_DARK, 255),
    )
    overlay = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    od = ImageDraw.Draw(overlay)
    _draw_topology(od, w, h, ix + icon_size // 2, iy + icon_size // 2,
                   ring_r=70, inner_r=42, count_outer=6, count_inner=4,
                   alpha_base=40, width=1)
    img = Image.alpha_composite(img, overlay)
    draw = ImageDraw.Draw(img)
    _draw_f(draw, ix + icon_size // 2, iy + icon_size // 2, icon_size, 90, fill=(*WHITE, 230))

    # --- Right side: text ---
    tx = ix + icon_size + 50
    font_title = _load_font(56)
    font_sub = _load_font(24)
    font_tag = _load_font(18)

    draw.text((tx, h // 2 - 80), "Phenotype", fill=(*WHITE, 255), font=font_title)
    draw.text((tx, h // 2 - 16), "Fabric", fill=(*ACCENT, 255), font=font_title)
    draw.text((tx, h // 2 + 60), "Graph-native workspace runtime", fill=(*SUBTLE, 220), font=font_tag)

    # URL
    font_url = _load_font(16)
    draw.text((tx, h - 48), "github.com/Phenotype/phenotype-fabric", fill=(*SUBTLE, 160), font=font_url)

    img.save(out_path, "PNG")
    print(f"  {'og-image.png':>30s}  {w}x{h}")


def gen_twitter_card(out_path):
    """1200x600 Twitter card."""
    w, h = 1200, 600
    img = Image.new("RGBA", (w, h), (*BG_DARK, 255))
    img = _draw_background_pattern(img, alpha_factor=0.5)
    img = _draw_radial_gradient(img, w // 2, h // 2, int(h * 0.8), ACCENT, max_alpha=14)
    draw = ImageDraw.Draw(img)

    cx, cy = w // 2, h // 2

    # Centred F
    _draw_f(draw, cx, cy - 60, w, 140, shadow=True)

    # "Phenotype Fabric"
    font_title = _load_font(48)
    title = "Phenotype Fabric"
    tbbox = draw.textbbox((0, 0), title, font=font_title)
    tw = tbbox[2] - tbbox[0]
    draw.text((cx - tw // 2, cy + 60), title, fill=(*WHITE, 240), font=font_title)

    # Tagline
    font_tag = _load_font(20)
    tagline = "Graph-native workspace runtime"
    tgbbox = draw.textbbox((0, 0), tagline, font=font_tag)
    tgw = tgbbox[2] - tgbbox[0]
    draw.text((cx - tgw // 2, cy + 120), tagline, fill=(*SUBTLE, 200), font=font_tag)

    # Bottom bar
    draw.rectangle([0, h - 4, w, h], fill=(*ACCENT, 180))

    img.save(out_path, "PNG")
    print(f"  {'twitter-card.png':>30s}  {w}x{h}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    global Image, ImageDraw, ImageFont, ImageFilter
    Image, ImageDraw, ImageFont, ImageFilter = _ensure_pillow()

    root = os.path.dirname(os.path.abspath(__file__))
    icons_dir = os.path.join(root, "icons")
    dmg_dir = os.path.join(root, "dmg")
    splash_dir = os.path.join(root, "splash")
    social_dir = os.path.join(root, "social")

    for d in (icons_dir, dmg_dir, splash_dir, social_dir):
        os.makedirs(d, exist_ok=True)

    print("=== Phenotype Fabric — Brand Asset Generator ===\n")

    # Favicons (simplified, no topology)
    print("[favicons]")
    gen_icon(16, os.path.join(icons_dir, "favicon-16.png"), include_topology=False)
    gen_icon(32, os.path.join(icons_dir, "favicon-32.png"), include_topology=False)
    gen_icon(48, os.path.join(icons_dir, "favicon-48.png"), include_topology=False)

    # Apple touch icon
    print("\n[apple-touch]")
    gen_apple_touch_icon(os.path.join(icons_dir, "apple-touch-icon-180.png"))

    # PWA icons
    print("\n[pwa]")
    gen_pwa_icon(192, os.path.join(icons_dir, "icon-192.png"))
    gen_pwa_icon(512, os.path.join(icons_dir, "icon-512.png"))

    # SVG
    print("\n[svg]")
    gen_svg(os.path.join(icons_dir, "icon.svg"))

    # Logos
    print("\n[logos]")
    gen_logo(os.path.join(icons_dir, "logo-dark.png"), bg_color=BG_DARK, text_color=WHITE)
    gen_logo(os.path.join(icons_dir, "logo-light.png"), bg_color=BG_LIGHT, text_color=DARK_TEXT)

    # DMG background
    print("\n[dmg]")
    gen_dmg_background(os.path.join(dmg_dir, "background.png"))

    # Splash
    print("\n[splash]")
    gen_splash(os.path.join(splash_dir, "splash.png"))

    # Social cards
    print("\n[social]")
    gen_og_image(os.path.join(social_dir, "og-image.png"))
    gen_twitter_card(os.path.join(social_dir, "twitter-card.png"))

    print("\n=== Done! All assets generated. ===")


if __name__ == "__main__":
    main()
