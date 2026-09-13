"""
Phenotype Fabric 2D Fallback Logo Generator
Uses PIL to create polished 2D approximations of the 3D logo.

Output files (same as Blender script):
    logo-render.png       2048x2048
    logo-render-wide.png  2048x1024
    logo-render-icon.png   512x512
    splash-3d.png         1200x800
"""

from PIL import Image, ImageDraw, ImageFont, ImageFilter
import math
import os

OUTPUT_DIR = os.path.dirname(os.path.abspath(__file__))

# ── Colors ───────────────────────────────────────────────────────────────────

TEAL = (0, 212, 170)       # #00d4aa
NAVY = (26, 26, 46)        # #1a1a2e
PURPLE = (100, 100, 200)   # #6464c8
DARK_BG = (15, 15, 30)     # near-black with blue tint
WHITE = (255, 255, 255)


def hex_to_rgb(h):
    h = h.lstrip('#')
    return tuple(int(h[i:i+2], 16) for i in (0, 2, 4))


def radial_gradient(size, center, radius, color_inner, color_outer):
    """Create a radial gradient image."""
    img = Image.new('RGBA', size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    steps = 80
    for i in range(steps, 0, -1):
        t = i / steps
        r = int(radius * t)
        c = tuple(int(color_inner[j] * (1 - t) + color_outer[j] * t) for j in range(3))
        draw.ellipse([center[0] - r, center[1] - r, center[0] + r, center[1] + r],
                      fill=(*c, 255))
    return img


def draw_3d_block(draw, x, y, w, h, depth, color, highlight, shadow):
    """Draw a pseudo-3D rectangular block."""
    # Front face
    draw.rectangle([x, y, x + w, y + h], fill=color)
    # Top face (lighter)
    top_pts = [(x, y), (x + depth, y - depth), (x + w + depth, y - depth), (x + w, y)]
    draw.polygon(top_pts, fill=highlight)
    # Right face (darker)
    right_pts = [(x + w, y), (x + w + depth, y - depth),
                 (x + w + depth, y + h - depth), (x + w, y + h)]
    draw.polygon(right_pts, fill=shadow)


def draw_f_letter(img, cx, cy, scale=1.0):
    """Draw a 3D-styled 'F' at center (cx, cy)."""
    draw = ImageDraw.Draw(img)
    s = scale

    # F dimensions
    stem_w = int(45 * s)
    stem_h = int(320 * s)
    bar_h = int(50 * s)
    top_w = int(180 * s)
    mid_w = int(130 * s)
    depth = int(20 * s)

    # Position so F is centered
    sx = cx - int(stem_w / 2)
    sy = cy - int(stem_h / 2)

    # 3D shadow layer (offset down-right)
    shadow_off = int(8 * s)
    dark_teal = tuple(max(0, c - 60) for c in TEAL)
    draw_3d_block(draw, sx + shadow_off, sy + shadow_off, stem_w, stem_h, depth,
                  dark_teal, tuple(max(0, c - 40) for c in dark_teal),
                  tuple(max(0, c - 80) for c in dark_teal))

    # Main stem
    draw_3d_block(draw, sx, sy, stem_w, stem_h, depth, TEAL,
                  tuple(min(255, c + 40) for c in TEAL),
                  tuple(max(0, c - 40) for c in TEAL))

    # Top bar
    draw_3d_block(draw, sx, sy, top_w, bar_h, depth, TEAL,
                  tuple(min(255, c + 50) for c in TEAL),
                  tuple(max(0, c - 30) for c in TEAL))

    # Middle bar
    mid_y = sy + int(stem_h * 0.42)
    draw_3d_block(draw, sx, mid_y, mid_w, bar_h, depth, TEAL,
                  tuple(min(255, c + 40) for c in TEAL),
                  tuple(max(0, c - 35) for c in TEAL))


def draw_node(draw, x, y, radius, color=PURPLE, glow_radius=None):
    """Draw a glowing sphere-like node."""
    if glow_radius is None:
        glow_radius = radius * 3
    # Glow (soft circle)
    for i in range(int(glow_radius), 0, -1):
        t = i / glow_radius
        alpha = int(30 * (1 - t))
        c = tuple(max(0, min(255, int(color[j] * 0.5))) for j in range(3))
        draw.ellipse([x - i, y - i, x + i, y + i], fill=(*c, alpha))

    # Solid sphere with gradient shading
    steps = 12
    for i in range(steps, 0, -1):
        t = i / steps
        r = int(radius * t)
        # Highlight offset (upper-left)
        hx = x - int(radius * 0.3)
        hy = y - int(radius * 0.3)
        # Blend toward white near center
        c = tuple(int(color[j] * (1 - t * 0.3) + 255 * t * 0.3) for j in range(3))
        draw.ellipse([hx - r, hy - r, hx + r, hy + r], fill=(*c, 255))


def draw_connection(draw, x1, y1, x2, y2, color=TEAL, width=2):
    """Draw a line with slight glow."""
    # Glow layer
    draw.line([(x1, y1), (x2, y2)], fill=(*tuple(int(c * 0.4) for c in color), 80), width=width + 4)
    # Core line
    draw.line([(x1, y1), (x2, y2)], fill=(*color, 200), width=width)


def draw_background(img, style="default"):
    """Draw gradient background."""
    draw = ImageDraw.Draw(img)
    w, h = img.size

    # Vertical gradient: dark navy to darker
    for y in range(h):
        t = y / h
        r = int(NAVY[0] * (1 - t * 0.4))
        g = int(NAVY[1] * (1 - t * 0.4))
        b = int(NAVY[2] * (1 - t * 0.3))
        draw.line([(0, y), (w, y)], fill=(r, g, b, 255))

    # Subtle radial glow at center
    cx, cy = w // 2, h // 2
    glow_r = min(w, h) // 3
    glow = radial_gradient((w, h), (cx, cy), glow_r,
                           (0, 40, 60), (0, 0, 0))
    img.paste(Image.alpha_composite(Image.new('RGBA', img.size, (0, 0, 0, 0)), glow),
              (0, 0), glow)


def place_nodes(img, scale=1.0):
    """Draw topology nodes on the image."""
    w, h = img.size
    draw = ImageDraw.Draw(img)

    # Node positions (normalized 0-1 then scaled)
    nodes_raw = [
        (0.62, 0.40), (0.38, 0.52), (0.65, 0.58), (0.32, 0.62),
        (0.72, 0.50), (0.45, 0.35), (0.55, 0.65), (0.28, 0.45),
        (0.58, 0.32), (0.42, 0.68),
    ]

    nodes = [(int(nx * w), int(ny * h)) for nx, ny in nodes_raw]

    # Connections
    edges = [
        (0, 1), (0, 2), (1, 3), (2, 4), (3, 5),
        (4, 6), (5, 7), (6, 8), (7, 9), (8, 9),
        (0, 4), (1, 5),
    ]

    for e in edges:
        draw_connection(draw, nodes[e[0]][0], nodes[e[0]][1],
                        nodes[e[1]][0], nodes[e[1]][1], TEAL, 2)

    # Draw nodes
    for i, (nx, ny) in enumerate(nodes):
        r = int((6 + (i % 3) * 2) * scale)
        draw_node(draw, nx, ny, r, PURPLE)


def generate_square(size=2048):
    """Generate the main square logo render."""
    img = Image.new('RGBA', (size, size), DARK_BG)
    draw_background(img)

    cx, cy = size // 2, size // 2
    draw_f_letter(img, cx, cy, scale=size / 1024)
    place_nodes(img, scale=size / 1024)

    return img


def generate_wide(w=2048, h=1024):
    """Generate wide landscape logo."""
    img = Image.new('RGBA', (w, h), DARK_BG)
    draw_background(img)

    cx, cy = w // 3, h // 2  # F on left third
    draw_f_letter(img, cx, cy, scale=h / 800)

    # Nodes spread across right side
    draw_obj = ImageDraw.Draw(img)
    nodes_raw = [
        (0.55, 0.30), (0.70, 0.45), (0.60, 0.60), (0.80, 0.35),
        (0.75, 0.65), (0.85, 0.50), (0.65, 0.75), (0.90, 0.40),
    ]
    nodes = [(int(nx * w), int(ny * h)) for nx, ny in nodes_raw]
    edges = [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 7),
             (0, 3), (1, 4)]

    for e in edges:
        draw_connection(draw_obj, nodes[e[0]][0], nodes[e[0]][1],
                        nodes[e[1]][0], nodes[e[1]][1], TEAL, 2)
    for i, (nx, ny) in enumerate(nodes):
        draw_node(draw_obj, nx, ny, 8, PURPLE)

    return img


def generate_icon(size=512):
    """Generate close-up icon."""
    img = Image.new('RGBA', (size, size), DARK_BG)
    draw_background(img)

    cx, cy = size // 2, size // 2
    draw_f_letter(img, cx, cy, scale=size / 512)

    # Minimal nodes close to F
    draw_obj = ImageDraw.Draw(img)
    nodes_raw = [(0.18, 0.25), (0.82, 0.35), (0.20, 0.75), (0.80, 0.70)]
    nodes = [(int(nx * size), int(ny * size)) for nx, ny in nodes_raw]
    edges = [(0, 1), (0, 2), (1, 3), (2, 3)]

    for e in edges:
        draw_connection(draw_obj, nodes[e[0]][0], nodes[e[0]][1],
                        nodes[e[1]][0], nodes[e[1]][1], TEAL, 2)
    for nx, ny in nodes:
        draw_node(draw_obj, nx, ny, 8, PURPLE)

    return img


def generate_splash(w=1200, h=800):
    """Generate splash screen with angled feel."""
    img = Image.new('RGBA', (w, h), DARK_BG)
    draw_background(img)

    cx, cy = int(w * 0.35), h // 2
    draw_f_letter(img, cx, cy, scale=min(w, h) / 700)

    draw_obj = ImageDraw.Draw(img)
    # Diagonal node constellation
    nodes_raw = [
        (0.50, 0.25), (0.60, 0.40), (0.55, 0.55), (0.70, 0.30),
        (0.65, 0.65), (0.80, 0.50), (0.75, 0.70), (0.85, 0.60),
        (0.90, 0.45), (0.95, 0.55),
    ]
    nodes = [(int(nx * w), int(ny * h)) for nx, ny in nodes_raw]
    edges = [(0, 1), (1, 2), (2, 4), (0, 3), (3, 5),
             (4, 6), (5, 7), (6, 8), (7, 9), (8, 9)]

    for e in edges:
        draw_connection(draw_obj, nodes[e[0]][0], nodes[e[0]][1],
                        nodes[e[1]][0], nodes[e[1]][1], TEAL, 2)
    for i, (nx, ny) in enumerate(nodes):
        draw_node(draw_obj, nx, ny, 7 + (i % 2) * 2, PURPLE)

    return img


# ── Generate all variants ────────────────────────────────────────────────────

def save(img, name):
    path = os.path.join(OUTPUT_DIR, name)
    img.save(path, "PNG")
    print(f"  [OK] {name} ({img.size[0]}x{img.size[1]})")


print("Phenotype Fabric 2D Fallback - Generating logos...")

save(generate_square(2048), "logo-render.png")
save(generate_wide(2048, 1024), "logo-render-wide.png")
save(generate_icon(512), "logo-render-icon.png")
save(generate_splash(1200, 800), "splash-3d.png")

print("Phenotype Fabric 2D Fallback - Done!")
