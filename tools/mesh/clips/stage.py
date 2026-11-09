"""The stage every veteran render uses: the pack's light (an overcast key from
the upper left, a warm low fill, a cool rim from behind right), a grey studio
world, a ground that takes shadows, and camera helpers. Values come from
tools/mesh/stage.json; this file holds no colour of its own."""
import json
import math
from pathlib import Path

import bpy
from mathutils import Vector

CFG = json.loads((Path(__file__).with_name("stage.json")).read_text())


def lin(h):
    h = h.lstrip("#")
    c = [int(h[i:i + 2], 16) / 255 for i in (0, 2, 4)]
    return [(x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4) for x in c]


def gpu(scene):
    try:
        prefs = bpy.context.preferences.addons["cycles"].preferences
        prefs.compute_device_type = "METAL"
        prefs.get_devices()
        for d in prefs.devices:
            d.use = True
        scene.cycles.device = "GPU"
    except Exception:
        pass


def light(name, kind, energy, colour, loc, aim, size):
    data = bpy.data.lights.new(name, kind)
    data.energy = energy
    data.color = lin(colour)
    if kind == "AREA":
        data.size = size
    ob = bpy.data.objects.new(name, data)
    bpy.context.scene.collection.objects.link(ob)
    ob.location = loc
    d = Vector(aim) - Vector(loc)
    ob.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    return ob


def build(engine="CYCLES", ground=True, samples=None):
    scene = bpy.context.scene
    for ob in [o for o in bpy.data.objects if o.type in ("LIGHT", "CAMERA") or o.name.startswith("stage_")]:
        bpy.data.objects.remove(ob, do_unlink=True)
    scene.render.engine = engine
    if engine == "CYCLES":
        gpu(scene)
        scene.cycles.samples = samples or CFG["samples"]
        scene.cycles.use_denoising = True
    else:
        scene.eevee.taa_render_samples = samples or CFG["eevee_samples"]
    scene.view_settings.view_transform = "AgX"
    scene.view_settings.look = "AgX - Medium High Contrast"
    scene.view_settings.exposure = CFG.get("exposure", 0.0)
    world = bpy.data.worlds.new("stage_world") if "stage_world" not in bpy.data.worlds else bpy.data.worlds["stage_world"]
    world.use_nodes = True
    bg = world.node_tree.nodes["Background"]
    bg.inputs[0].default_value = lin(CFG["world"]) + [1.0]
    bg.inputs[1].default_value = CFG["world_strength"]
    scene.world = world
    for name, l in CFG["lights"].items():
        light(f"stage_{name}", l["kind"], l["energy"], l["colour"], l["loc"], l["aim"], l.get("size", 1.0))
    if ground:
        bpy.ops.mesh.primitive_plane_add(size=CFG["ground_size"], location=(0, 0, 0))
        g = bpy.context.active_object
        g.name = "stage_ground"
        m = bpy.data.materials.new("stage_ground")
        m.use_nodes = True
        p = m.node_tree.nodes["Principled BSDF"]
        nt = m.node_tree
        noise = nt.nodes.new("ShaderNodeTexNoise")
        noise.inputs["Scale"].default_value = CFG["ground_noise_scale"]
        noise.inputs["Detail"].default_value = 8.0
        ramp = nt.nodes.new("ShaderNodeValToRGB")
        ramp.color_ramp.elements[0].color = lin(CFG["ground_dark"]) + [1.0]
        ramp.color_ramp.elements[1].color = lin(CFG["ground"]) + [1.0]
        nt.links.new(noise.outputs["Fac"], ramp.inputs[0])
        nt.links.new(ramp.outputs[0], p.inputs["Base Color"])
        # expansion joints every few metres so travel reads
        grid = nt.nodes.new("ShaderNodeTexBrick")
        grid.inputs["Scale"].default_value = CFG["ground_joint_scale"]
        grid.inputs["Mortar Size"].default_value = 0.004
        grid.inputs["Color1"].default_value = (1, 1, 1, 1)
        grid.inputs["Color2"].default_value = (1, 1, 1, 1)
        grid.inputs["Mortar"].default_value = (0.55, 0.55, 0.55, 1)
        mult = nt.nodes.new("ShaderNodeMix")
        mult.data_type, mult.blend_type = "RGBA", "MULTIPLY"
        mult.inputs[0].default_value = 1.0
        nt.links.new(ramp.outputs[0], mult.inputs[6])
        nt.links.new(grid.outputs["Color"], mult.inputs[7])
        nt.links.new(mult.outputs[2], p.inputs["Base Color"])
        p.inputs["Roughness"].default_value = 0.9
        g.data.materials.append(m)
    return scene


def camera(loc, aim, lens=85.0, name="stage_cam"):
    data = bpy.data.cameras.new(name)
    data.lens = lens
    ob = bpy.data.objects.new(name, data)
    bpy.context.scene.collection.objects.link(ob)
    ob.location = loc
    d = Vector(aim) - Vector(loc)
    ob.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.camera = ob
    return ob


def track(cam, target_ob, bone=None):
    c = cam.constraints.new("TRACK_TO")
    c.target = target_ob
    if bone:
        c.subtarget = bone
    c.track_axis = "TRACK_NEGATIVE_Z"
    c.up_axis = "UP_Y"
    return c


def deg(*a):
    return tuple(math.radians(x) for x in a)


def reflective_world(scene, over):
    """A ground-to-sky gradient with a bright strip above the horizon: metal
    reflects it, as steel in a room reflects walls and windows."""
    w = scene.world
    nt = w.node_tree
    nt.nodes.clear()
    tc = nt.nodes.new("ShaderNodeTexCoord")
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    nt.links.new(tc.outputs["Generated"], sep.inputs[0])
    ramp = nt.nodes.new("ShaderNodeValToRGB")
    cr = ramp.color_ramp
    cr.elements[0].position, cr.elements[0].color = 0.0, lin(over["sky_ground"]) + [1]
    cr.elements[1].position, cr.elements[1].color = 1.0, lin(over["sky_top"]) + [1]
    e = cr.elements.new(0.52)
    e.color = lin(over["sky_horizon"]) + [1]
    e2 = cr.elements.new(0.49)
    e2.color = lin(over["sky_ground"]) + [1]
    mapr = nt.nodes.new("ShaderNodeMapRange")
    mapr.inputs["From Min"].default_value = -1.0
    nt.links.new(sep.outputs["Z"], mapr.inputs["Value"])
    nt.links.new(mapr.outputs["Result"], ramp.inputs[0])
    strip = nt.nodes.new("ShaderNodeMapRange")
    strip.interpolation_type = "SMOOTHSTEP"
    strip.inputs["From Min"].default_value = 0.25
    strip.inputs["From Max"].default_value = 0.33
    nt.links.new(sep.outputs["Z"], strip.inputs["Value"])
    strip2 = nt.nodes.new("ShaderNodeMapRange")
    strip2.interpolation_type = "SMOOTHSTEP"
    strip2.inputs["From Min"].default_value = 0.5
    strip2.inputs["From Max"].default_value = 0.42
    nt.links.new(sep.outputs["Z"], strip2.inputs["Value"])
    band = nt.nodes.new("ShaderNodeMath")
    band.operation = "MULTIPLY"
    nt.links.new(strip.outputs["Result"], band.inputs[0])
    nt.links.new(strip2.outputs["Result"], band.inputs[1])
    mix = nt.nodes.new("ShaderNodeMix")
    mix.data_type = "RGBA"
    nt.links.new(band.outputs[0], mix.inputs[0])
    nt.links.new(ramp.outputs[0], mix.inputs[6])
    mix.inputs[7].default_value = [c * over["strip_strength"] for c in lin(over["strip"])] + [1]
    bg = nt.nodes.new("ShaderNodeBackground")
    bg.inputs[1].default_value = over["world_strength"]
    nt.links.new(mix.outputs[2], bg.inputs[0])
    out = nt.nodes.new("ShaderNodeOutputWorld")
    nt.links.new(bg.outputs[0], out.inputs[0])
