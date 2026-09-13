"""
Phenotype Fabric 3D Logo Generator
Renders a stylized "F" with topology network nodes.

Usage (headless):
    blender --background --python gen-blender-logo.py

Output files:
    logo-render.png       2048x2048  front angle
    logo-render-wide.png  2048x1024  landscape
    logo-render-icon.png   512x512   close-up icon
    splash-3d.png         1200x800   splash screen angle
"""

import bpy
import math

# ── Helpers ──────────────────────────────────────────────────────────────────

class Vec3:
    """Minimal 3D vector for Blender math."""
    __slots__ = ('x', 'y', 'z')

    def __init__(self, tup):
        self.x, self.y, self.z = tup

    def __sub__(self, o):
        return Vec3((self.x - o.x, self.y - o.y, self.z - o.z))

    def __add__(self, o):
        return Vec3((self.x + o.x, self.y + o.y, self.z + o.z))

    def __truediv__(self, s):
        return Vec3((self.x / s, self.y / s, self.z / s))

    def dot(self, o):
        return self.x * o.x + self.y * o.y + self.z * o.z

    def cross(self, o):
        return Vec3((
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        ))

    @property
    def length(self):
        return math.sqrt(self.x**2 + self.y**2 + self.z**2)

    def normalized(self):
        l = self.length
        if l < 1e-8:
            return Vec3((0, 0, 0))
        return Vec3((self.x / l, self.y / l, self.z / l))

    def as_tuple(self):
        return (self.x, self.y, self.z)


# ── Clear Scene ──────────────────────────────────────────────────────────────

bpy.ops.object.select_all(action='SELECT')
bpy.ops.object.delete(use_global=False)

for col in list(bpy.data.collections):
    bpy.data.collections.remove(col)

for block in list(bpy.data.meshes):
    if block.users == 0:
        bpy.data.meshes.remove(block)
for block in list(bpy.data.materials):
    if block.users == 0:
        bpy.data.materials.remove(block)
for block in list(bpy.data.cameras):
    if block.users == 0:
        bpy.data.cameras.remove(block)
for block in list(bpy.data.lights):
    if block.users == 0:
        bpy.data.lights.remove(block)


# ── Materials ────────────────────────────────────────────────────────────────

def make_material(name, base_color, metallic=0.0, roughness=0.5, emission=0.0):
    """Create a Principled BSDF material."""
    mat = bpy.data.materials.new(name=name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = (*base_color, 1)
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Roughness"].default_value = roughness
    if emission > 0 and "Emission Strength" in bsdf.inputs:
        bsdf.inputs["Emission Strength"].default_value = emission
        bsdf.inputs["Emission Color"].default_value = (*base_color, 1)
    return mat


mat_teal = make_material("TealMetal", (0.0, 0.831, 0.667), metallic=0.85, roughness=0.15)
mat_navy = make_material("DarkNavy", (0.102, 0.102, 0.180), metallic=0.1, roughness=0.6)
mat_purple = make_material("PurpleNode", (0.392, 0.392, 0.784), metallic=0.3, roughness=0.3, emission=0.6)
mat_purple_dim = make_material("PurpleNodeDim", (0.292, 0.292, 0.584), metallic=0.3, roughness=0.4, emission=0.3)
mat_edge = make_material("EdgeMat", (0.0, 0.631, 0.467), metallic=0.5, roughness=0.3, emission=0.2)

# ── F geometry constants ─────────────────────────────────────────────────────

F_THICKNESS = 0.35
F_WIDTH = 2.0
F_HEIGHT = 4.0
F_DEPTH = 0.6
F_BAR_LEN = 1.4
F_MID_BAR_LEN = 1.0

# ── Create the "F" letter from boxes ────────────────────────────────────────

parts = []

def add_box(name, size, location):
    bpy.ops.mesh.primitive_cube_add(size=1, location=location)
    obj = bpy.context.active_object
    obj.name = name
    obj.scale = (size[0] / 2, size[1] / 2, size[2] / 2)
    bpy.ops.object.transform_apply(scale=True)
    parts.append(obj)
    return obj

# Vertical stem
add_box("F_stem", (F_THICKNESS, F_DEPTH, F_HEIGHT), (0, 0, 0))
# Top horizontal bar
add_box("F_top_bar", (F_WIDTH, F_DEPTH, F_THICKNESS),
        (F_WIDTH / 2 - F_THICKNESS / 2, 0, F_HEIGHT / 2 - F_THICKNESS / 2))
# Middle horizontal bar
add_box("F_mid_bar", (F_MID_BAR_LEN, F_DEPTH, F_THICKNESS),
        (F_MID_BAR_LEN / 2 - F_THICKNESS / 2, 0, 0.3))

# Join all F parts into one object
bpy.ops.object.select_all(action='DESELECT')
for part in parts:
    part.select_set(True)
bpy.context.view_layer.objects.active = parts[0]
bpy.ops.object.join()

f_obj = bpy.context.active_object
f_obj.name = "Fabric_F"
f_obj.data.materials.append(mat_teal)

# Bevel modifier for softer edges
bevel = f_obj.modifiers.new(name="Bevel", type='BEVEL')
bevel.width = 0.04
bevel.segments = 3
bevel.profile = 0.7

# ── Base platform ────────────────────────────────────────────────────────────

bpy.ops.mesh.primitive_cylinder_add(radius=3.5, depth=0.3,
                                    location=(0.5, 0, -F_HEIGHT / 2 - 0.15))
base = bpy.context.active_object
base.name = "Base_Platform"
base.data.materials.append(mat_navy)

bevel_b = base.modifiers.new(name="Bevel", type='BEVEL')
bevel_b.width = 0.05
bevel_b.segments = 2

# ── Topology network nodes ───────────────────────────────────────────────────

node_positions = [
    (2.5, 1.0, 1.8),
    (-1.5, 2.5, 0.5),
    (3.0, -1.5, 0.0),
    (-2.0, -1.0, 2.2),
    (3.8, 0.5, -0.8),
    (-0.5, 3.5, 1.0),
    (1.5, -2.5, -0.3),
    (-2.5, 0.5, 2.8),
    (2.0, 3.0, -0.5),
    (-1.0, -3.0, 0.8),
]

node_objects = []

for i, pos in enumerate(node_positions):
    radius = 0.12 + (i % 3) * 0.04
    bpy.ops.mesh.primitive_uv_sphere_add(radius=radius, location=pos)
    node = bpy.context.active_object
    node.name = f"Node_{i}"
    node.data.materials.append(mat_purple if i % 2 == 0 else mat_purple_dim)
    node_objects.append(node)

# ── Connections between nodes ────────────────────────────────────────────────

connections = [
    (0, 1), (0, 2), (1, 3), (2, 4), (3, 5),
    (4, 6), (5, 7), (6, 8), (7, 9), (8, 9),
    (0, 4), (1, 5), (3, 7), (2, 6),
]

f_center = Vec3((0.5, 0, 0))

for conn in connections:
    start = Vec3(node_positions[conn[0]])
    end = Vec3(node_positions[conn[1]])

    direction = end - start
    length = direction.length
    if length < 0.01:
        continue

    mid = (start + end) / 2

    bpy.ops.mesh.primitive_cylinder_add(
        radius=0.02, depth=length,
        location=mid.as_tuple()
    )
    edge = bpy.context.active_object
    edge.name = f"Edge_{conn[0]}_{conn[1]}"
    edge.data.materials.append(mat_edge)

    # Align cylinder to point from start to end (default cylinder is along Z)
    up = Vec3((0, 0, 1))
    dnorm = direction.normalized()
    if dnorm.dot(up) < 0.999:
        rot = up.cross(dnorm).normalized()
        angle = math.acos(max(-1, min(1, up.dot(dnorm))))
        edge.rotation_euler = (rot.x * angle, rot.y * angle, rot.z * angle)

    bevel_e = edge.modifiers.new(name="Bevel", type='BEVEL')
    bevel_e.width = 0.005
    bevel_e.segments = 1

# Connect some nodes to F
f_connections = [(1, 0), (7, 0)]
for node_idx, _ in f_connections:
    start = Vec3(node_positions[node_idx])
    direction = f_center - start
    length = direction.length
    if length < 0.01:
        continue
    mid = (start + f_center) / 2

    bpy.ops.mesh.primitive_cylinder_add(
        radius=0.015, depth=length,
        location=mid.as_tuple()
    )
    edge = bpy.context.active_object
    edge.name = f"NodeToF_{node_idx}"
    edge.data.materials.append(mat_edge)

    up = Vec3((0, 0, 1))
    dnorm = direction.normalized()
    if dnorm.dot(up) < 0.999:
        rot = up.cross(dnorm).normalized()
        angle = math.acos(max(-1, min(1, up.dot(dnorm))))
        edge.rotation_euler = (rot.x * angle, rot.y * angle, rot.z * angle)

# ── Mini floating accent spheres ─────────────────────────────────────────────

accent_positions = [
    (0.8, 0.5, 2.5),
    (-0.5, 0.3, -1.5),
    (1.2, -0.4, 0.8),
]

for i, pos in enumerate(accent_positions):
    bpy.ops.mesh.primitive_uv_sphere_add(radius=0.08, location=pos)
    accent = bpy.context.active_object
    accent.name = f"Accent_{i}"
    accent.data.materials.append(mat_purple)

# ── Lighting ─────────────────────────────────────────────────────────────────

def add_light(name, light_type, energy, location, rotation, color=(1, 1, 1)):
    bpy.ops.object.light_add(type=light_type, location=location, rotation=rotation)
    light = bpy.context.active_object
    light.name = name
    light.data.energy = energy
    light.data.color = color
    return light

# Key light (warm, upper right)
add_light("Key_Light", 'AREA', 800, (5, -3, 6), (0.5, 0, 0.8), (1.0, 0.95, 0.9))

# Fill light (cool, left)
add_light("Fill_Light", 'AREA', 300, (-4, 2, 3), (0.3, 0, -0.6), (0.85, 0.9, 1.0))

# Rim light (behind, edge separation)
add_light("Rim_Light", 'AREA', 500, (-2, -5, 4), (0.2, 0.3, 0.5), (0.95, 0.95, 1.0))

# Subtle uplight
bpy.ops.object.light_add(type='POINT', location=(0, 0, -3))
ambient = bpy.context.active_object
ambient.name = "Ambient_Up"
ambient.data.energy = 80
ambient.data.color = (0.7, 0.7, 1.0)

# ── Camera ───────────────────────────────────────────────────────────────────

bpy.ops.object.camera_add(
    location=(5, -5, 3),
    rotation=(math.radians(70), 0, math.radians(45))
)
cam = bpy.context.active_object
cam.name = "Main_Camera"
cam.data.lens = 50
cam.data.clip_end = 100
bpy.context.scene.camera = cam

# Track-to constraint so camera looks at the F
track = cam.constraints.new(type='TRACK_TO')
track.target = f_obj
track.track_axis = 'TRACK_NEGATIVE_Z'
track.up_axis = 'UP_Y'

# ── Render Settings ──────────────────────────────────────────────────────────

scene = bpy.context.scene
scene.render.engine = 'CYCLES'
scene.cycles.samples = 128
scene.cycles.use_denoising = True
scene.render.image_settings.file_format = 'PNG'
scene.render.image_settings.color_mode = 'RGBA'
scene.render.image_settings.color_depth = '16'
scene.render.film_transparent = True

# ── Render Function ──────────────────────────────────────────────────────────

def render_variant(name, width, height, cam_location, cam_rotation=None):
    cam.location = cam_location
    if cam_rotation:
        cam.rotation_euler = cam_rotation
    for c in cam.constraints:
        if c.name == 'Track To':
            c.target = f_obj
    scene.render.resolution_x = width
    scene.render.resolution_y = height
    scene.render.resolution_percentage = 100
    scene.render.filepath = f"//{name}.png"
    bpy.ops.render.render(write_still=True)
    print(f"  [OK] {name}.png ({width}x{height})")


# ── Render All Variants ─────────────────────────────────────────────────────

print("Phenotype Fabric 3D Logo - Rendering...")

render_variant("logo-render", 2048, 2048,
               (5, -5, 3), (math.radians(70), 0, math.radians(45)))
render_variant("logo-render-wide", 2048, 1024,
               (6, -4, 2.5), (math.radians(75), 0, math.radians(40)))
render_variant("logo-render-icon", 512, 512,
               (3, -2, 2), (math.radians(65), 0, math.radians(50)))
render_variant("splash-3d", 1200, 800,
               (7, -3, 4), (math.radians(60), 0, math.radians(35)))

print("Phenotype Fabric 3D Logo - Done!")
