# 3D Logo Renders

This directory contains scripts to generate Phenotype Fabric logo renders.

## Files

| File | Description |
|------|-------------|
| `gen-blender-logo.py` | Blender headless script — true 3D render with Cycles |
| `fallback-3d.py` | PIL-based 2D approximation (no Blender required) |
| `logo-render.png` | 2048x2048 front-angle render |
| `logo-render-wide.png` | 2048x1024 landscape render |
| `logo-render-icon.png` | 512x512 close-up icon |
| `splash-3d.png` | 1200x800 angled view for splash screen |

## Running the Blender Script

Requires **Blender 3.6+** installed (tested with Homebrew `blender`).

```bash
cd assets/3d
blender --background --python gen-blender-logo.py
```

The script runs entirely headless — no GUI window opens. It uses Cycles
path tracing at 128 samples with denoising. A typical render takes 2-5
minutes depending on hardware.

### What the Blender script creates

- A geometric **"F" letter** built from extruded cubes with a bevel modifier
- A dark navy **cylindrical platform** beneath the F
- **10 topology nodes** (UV spheres) connected by **cylinders** forming a network graph
- 3 **accent spheres** floating near the F
- **4-point lighting**: key (warm), fill (cool), rim (edge separation), ambient uplight
- **Camera** with track-to constraint for automatic framing

### Materials

| Name | Color | Properties |
|------|-------|------------|
| TealMetal | #00d4aa | Metallic 0.85, roughness 0.15 |
| DarkNavy | #1a1a2e | Metallic 0.1, roughness 0.6 |
| PurpleNode | #6464c8 | Metallic 0.3, emission 0.6 |
| EdgeMat | #00a178 | Metallic 0.5, emission 0.2 |

## Running the Fallback Script

No dependencies beyond Pillow:

```bash
cd assets/3d
python3 fallback-3d.py
```

Generates the same output filenames using 2D approximations:
- Pseudo-3D extruded blocks for the F letter
- Radial gradient glow on topology nodes
- Gradient background with subtle radial glow

## Regenerating

Delete the PNG files and re-run either script to regenerate.
The Blender script produces higher-quality output but requires Blender.
