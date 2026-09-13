#!/usr/bin/env python3
"""Generate a 1024x1024 PNG icon for Phenotype Fabric GUI."""
import sys
import os


def install_pillow():
    """Install Pillow if not available."""
    try:
        from PIL import Image, ImageDraw, ImageFont
        return Image, ImageDraw, ImageFont
    except ImportError:
        print("==> Installing Pillow...")
        os.system("pip3 install Pillow")
        from PIL import Image, ImageDraw, ImageFont
        return Image, ImageDraw, ImageFont


def main():
    Image, ImageDraw, ImageFont = install_pillow()

    SIZE = 1024
    BG = (26, 26, 46)          # #1a1a2e
    ACCENT = (0, 212, 170)     # teal/cyan
    NODE_COLOR = (100, 100, 200)
    WHITE = (240, 240, 245)
    LINE_COLOR = (80, 80, 140)

    img = Image.new("RGBA", (SIZE, SIZE), (*BG, 255))
    draw = ImageDraw.Draw(img)

    # Draw subtle radial gradient overlay
    for r in range(SIZE // 2, 0, -2):
        alpha = int(20 * (1 - r / (SIZE // 2)))
        cx, cy = SIZE // 2, SIZE // 2
        draw.ellipse(
            [cx - r, cy - r, cx + r, cy + r],
            fill=(*ACCENT, alpha)
        )

    # === Draw topology network ===
    import math

    # Node positions in a ring around the F
    nodes = []
    ring_r = 320
    ring_count = 8
    for i in range(ring_count):
        angle = 2 * math.pi * i / ring_count - math.pi / 2
        x = SIZE // 2 + int(ring_r * math.cos(angle))
        y = SIZE // 2 + int(ring_r * math.sin(angle))
        nodes.append((x, y))

    # Inner ring
    inner_r = 200
    inner_count = 6
    for i in range(inner_count):
        angle = 2 * math.pi * i / inner_count
        x = SIZE // 2 + int(inner_r * math.cos(angle))
        y = SIZE // 2 + int(inner_r * math.sin(angle))
        nodes.append((x, y))

    # Draw connections
    for i, (x1, y1) in enumerate(nodes):
        for j, (x2, y2) in enumerate(nodes):
            if j <= i:
                continue
            dist = math.sqrt((x2 - x1) ** 2 + (y2 - y1) ** 2)
            if dist < 420:
                draw.line([(x1, y1), (x2, y2)], fill=(*LINE_COLOR, 100), width=2)

    # Draw nodes
    for i, (x, y) in enumerate(nodes):
        r = 18 if i < ring_count else 12
        # Glow
        for gr in range(r + 20, r, -2):
            alpha = int(60 * (1 - (gr - r) / 20))
            draw.ellipse(
                [x - gr, y - gr, x + gr, y + gr],
                fill=(*ACCENT, alpha)
            )
        # Solid node
        draw.ellipse([x - r, y - r, x + r, y + r], fill=(*ACCENT, 220))

    # === Draw stylized F ===
    font_size = 480
    font = None
    font_paths = [
        "/System/Library/Fonts/SFCompact.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/System/Library/Fonts/SFNS.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
    ]
    for fp in font_paths:
        if os.path.exists(fp):
            try:
                font = ImageFont.truetype(fp, font_size)
                break
            except Exception:
                continue

    if font is None:
        try:
            font = ImageFont.truetype("Arial", font_size)
        except Exception:
            font = ImageFont.load_default()

    # Measure text
    bbox = draw.textbbox((0, 0), "F", font=font)
    tw = bbox[2] - bbox[0]
    th = bbox[3] - bbox[1]
    tx = (SIZE - tw) // 2 - bbox[0]
    ty = (SIZE - th) // 2 - bbox[1]

    # Shadow
    draw.text((tx + 6, ty + 6), "F", fill=(0, 0, 0, 120), font=font)
    # Main letter
    draw.text((tx, ty), "F", fill=(*WHITE, 240), font=font)

    # === Outer ring accent ===
    ring_outer = 460
    for i in range(360):
        angle = math.radians(i)
        x = SIZE // 2 + int(ring_outer * math.cos(angle))
        y = SIZE // 2 + int(ring_outer * math.sin(angle))
        if i % 3 == 0:
            draw.ellipse([x - 2, y - 2, x + 2, y + 2], fill=(*ACCENT, 60))

    # Save
    output_dir = os.path.dirname(os.path.abspath(__file__))
    output_path = os.path.join(output_dir, "icon.png")
    img.save(output_path, "PNG")
    print(f"==> Icon saved to {output_path} ({SIZE}x{SIZE})")


if __name__ == "__main__":
    main()
