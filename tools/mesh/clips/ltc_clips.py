"""Copied into Long Time Comin from Style Lab (branch ltc-diver, commit
d30f1ab, tools/mesh/ltc_clips.py) and changed in one place: an airborne
clip's keys keep the hips at standing height (see `key`). Rebuild with
tools/mesh/make_clips.sh.

Animate the diver, Long Time Comin's player, headless: every move the game
draws the Defendant making, as in-place clips on the costumed character.

    blender -b notes/costumes/final/diver.blend --python tools/mesh/ltc_clips.py -- OUTDIR [--fast] [--only a,b] [--no-render]

What the game asks for comes from its own code (player.rs, draw.rs,
weapons.json): run and backpedal and strafe; a jump, an air jump (up to four),
falling and landing; the dash; the hook fired and pulling; the gold parry; the
Injunction's wall; the ground pound's drop, slam and bounce; the time stop;
the Bench laid; Summary Judgment's slash and lunge; grabbing an injured enemy,
holding it and throwing it; swimming and sinking in the sea; a hurt, a death
and a Recess; each gun held, fired and reloaded, with the second fires that
change how a gun is handled (the carbine's burst, the Big Iron's fanned
hammer); the trumpet; waking in the bedroom; and the Holding's figure at rest.

Every clip is in place: the game moves the body, the clip moves the limbs. A
held prop rides rigidly on the right hand (wrist.R), as the game draws a
weapon at the grip; the right hand follows a target on an aim pivot at the
chest, the left hand reaches the prop's fore grip, both by IK with poles. Each
clip is then baked to plain bone keys (an action named diver__<clip>), so it
plays without the targets. Writes OUTDIR/ltc.blend, OUTDIR/ltc_clips.json
(each clip's frames, loop, prop and whether it is on the ground, for the clip
check) and, unless --no-render, OUTDIR/clips/<clip>.mp4 and a mid-clip still
per clip in OUTDIR/frames/.
"""
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE))
import stage  # noqa: E402

args = sys.argv[sys.argv.index("--") + 1:]
OUT = Path(args[0])
FAST = "--fast" in args
ONLY = set(args[args.index("--only") + 1].split(",")) if "--only" in args else None
RENDER = "--no-render" not in args
FPS = 30
R = math.radians
OB = bpy.data.objects
rig = OB["diver_rig"]
body = OB["diver"]
PROPS = json.loads((HERE / "ltc_props.json").read_text())
over = json.loads((HERE / "costume_stage.json").read_text())
for k in ("world", "world_strength", "exposure"):
    stage.CFG[k] = over[k]
stage.CFG["lights"]["rim"]["energy"] = over["rim_energy"]
META = {}


def say(*a):
    print("ltc_clips:", *a, flush=True)


def W(name, end="head"):
    b = rig.data.bones[name]
    return rig.matrix_world @ (b.head_local if end == "head" else b.tail_local)


CHEST = W("spine01")
SHOULDER_R = W("upperarm01.R")
SHOULDER_L = W("upperarm01.L")
HEAD = W("head")
HIP = W("root")
# the rig stands on its lead soles: lowest point of the dressed figure at rest
dg = bpy.context.evaluated_depsgraph_get()
LOW = 1e9
for ob in [o for o in OB if o.get("costume") or o == body]:
    e = ob.evaluated_get(dg)
    m = e.to_mesh()
    if len(m.vertices):
        LOW = min(LOW, min((ob.matrix_world @ v.co).z for v in m.vertices))
    e.to_mesh_clear()
BASE = -LOW
say("chest", tuple(round(x, 3) for x in CHEST), "base", round(BASE, 3))

# ---- poses (bone: XYZ degrees; ".*" mirrors to the right side) --------------
# signs (from the lab's rig probes): a negative hip turn swings the leg
# forward, a positive knee turn bends it, a positive shoulder X raises the
# arm forward, a negative shoulder Z lowers the left arm from the A pose,
# a positive spine or neck turn bends forward
GRIP = {f"finger{f}-{k}.*": (a, 0, 0) for f in (2, 3, 4, 5) for k, a in ((1, 55), (2, 70), (3, 45))}
GRIP.update({"finger1-2.*": (25, 0, 0), "finger1-3.*": (30, 0, 0)})
RELAX = {f"finger{f}-{k}.*": (a, 0, 0) for f in (2, 3, 4, 5) for k, a in ((1, 12), (2, 18), (3, 10))}
OPEN = {f"finger{f}-{k}.*": (a, 0, 0) for f in (2, 3, 4, 5) for k, a in ((1, -4), (2, 2), (3, 2))}
FIST = {f"finger{f}-{k}.*": (a, 0, 0) for f in (2, 3, 4, 5) for k, a in ((1, 80), (2, 95), (3, 70))}
FIST.update({"finger1-2.*": (40, 0, 0), "finger1-3.*": (45, 0, 0)})


def side(pose, s):
    """Keep only one side's hand keys from a ".*" hand pose, as that side."""
    out = {}
    for k, v in pose.items():
        b = k[:-2]
        out[f"{b}.{s}"] = v if s == "L" else (v[0], -v[1], -v[2])
    return out


ARMS_DOWN = {"upperarm01.*": (0, 0, -30), "lowerarm01.*": (16, 0, 0)}
STANCE = {"upperleg01.L": (-6, 0, 4), "upperleg01.R": (4, 0, -4), "lowerleg01.*": (10, 0, 0), "foot.*": (-4, 0, 0),
          "spine03": (4, 0, 0)}
HOLD_HANDS = {**side(GRIP, "R"), **side(GRIP, "L")}


def mirror(pose):
    out = {}
    for k, v in pose.items():
        if k.endswith(".*"):
            b = k[:-2]
            out[b + ".L"] = v
            out[b + ".R"] = (v[0], -v[1], -v[2])
        else:
            out[k] = v
    return out


BONES = [b.name for b in rig.pose.bones if not b.name.startswith((
    "oris", "levator", "risorius", "temporalis", "orbicularis", "oculi", "tongue", "special", "jaw", "eye"))]

# ---- the aim pivot, the targets and the IK ------------------------------------
def empty(name, parent=None, loc=(0, 0, 0)):
    e = OB.get(name) or bpy.data.objects.new(name, None)
    if e.name not in bpy.context.scene.collection.objects:
        bpy.context.scene.collection.objects.link(e)
    e.parent = parent
    e.location = loc
    e.rotation_mode = "XYZ"
    return e


PIVOT = empty("ltc_pivot")
PIVOT.parent, PIVOT.parent_type, PIVOT.parent_bone = rig, "BONE", "spine01"
bpy.context.view_layer.update()
PIVOT.matrix_world = Matrix.Translation(CHEST + Vector((-0.06, -0.05, -0.04)))
AIM = empty("ltc_aim", PIVOT)
T_R = empty("ltc_target_R", AIM)
P_R = empty("ltc_pole_R", AIM, (-0.6, 0.35, -0.4))
T_L = empty("ltc_target_L")
P_L = empty("ltc_pole_L", AIM, (0.6, 0.1, -0.5))
IK = {}
for s, t, p in (("R", T_R, P_R), ("L", T_L, P_L)):
    pb = rig.pose.bones[f"lowerarm02.{s}"]
    for c in [c for c in pb.constraints if c.name == "ltc_ik"]:
        pb.constraints.remove(c)
    ik = pb.constraints.new("IK")
    ik.name, ik.target, ik.pole_target, ik.chain_count = "ltc_ik", t, p, 5
    ik.pole_angle = R(-90)
    ik.influence = 0.0
    IK[s] = ik


# ---- props ---------------------------------------------------------------------
def stuff(name):
    s = PROPS["stuffs"][name]
    m = bpy.data.materials.get(f"prop_{name}") or bpy.data.materials.new(f"prop_{name}")
    m.use_nodes = True
    p = m.node_tree.nodes["Principled BSDF"]
    p.inputs["Base Color"].default_value = stage.lin(s["colour"]) + [1.0]
    p.inputs["Roughness"].default_value = s["rough"]
    p.inputs["Metallic"].default_value = s["metal"]
    if s.get("glass"):
        p.inputs["Transmission Weight"].default_value = 1.0
    return m


def make_prop(name):
    spec = PROPS["props"][name]
    root = empty(f"prop_{name}")
    for i, (kind, size, at, mat) in enumerate(spec["parts"]):
        if kind == "box":
            bpy.ops.mesh.primitive_cube_add(size=1, location=at)
            ob = bpy.context.active_object
            ob.scale = size
        elif kind in ("cyl", "cyl_y", "cyl_z"):
            rot = {"cyl": (R(90), 0, 0), "cyl_y": (0, R(90), 0), "cyl_z": (0, 0, 0)}[kind]
            bpy.ops.mesh.primitive_cylinder_add(radius=size[0], depth=size[1], location=at, rotation=rot, vertices=24)
            ob = bpy.context.active_object
        elif kind == "cone":
            bpy.ops.mesh.primitive_cone_add(radius1=size[0], radius2=size[1], depth=size[2], location=at,
                                            rotation=(R(90), 0, 0), vertices=32)
            ob = bpy.context.active_object
        else:
            bpy.ops.mesh.primitive_uv_sphere_add(radius=1, location=at, segments=24, ring_count=12)
            ob = bpy.context.active_object
            ob.scale = size
        bpy.ops.object.transform_apply(scale=True)
        ob.name = f"prop_{name}_{i}"
        ob.data.materials.append(stuff(mat))
        bv = ob.modifiers.new("bevel", "BEVEL")
        bv.width, bv.segments = 0.003, 2
        for p in ob.data.polygons:
            p.use_smooth = kind != "box"
        ob.parent = root
    root["fore"] = spec.get("fore", [0, -0.2, 0])
    return root


PROP = {n: make_prop(n) for n in PROPS["props"]}


def show(name):
    for n, root in PROP.items():
        for ob in [root] + list(root.children):
            ob.hide_render = ob.hide_viewport = n != name


# where each prop's grip sits from the pivot at the chest (metres: x to the
# figure's left, y back, z up), its yaw across the body (degrees, muzzle
# toward the left), and where the left hand holds it
HOLDS = {
    "carbine": ((-0.04, -0.25, -0.02), 22, "fore"),
    "redactor": ((-0.04, -0.25, -0.02), 22, "fore"),
    "filibuster": ((-0.04, -0.25, -0.08), 22, "fore"),
    "scattergun": ((-0.04, -0.25, -0.02), 22, "fore"),
    "launcher": ((-0.05, -0.24, 0.0), 18, "fore"),
    "ripsaw": ((-0.04, -0.25, -0.04), 22, "fore"),
    "big_iron": ((-0.02, -0.4, 0.06), 4, None),
    "trumpet": ((0.0, -0.31, 0.31), 0, "fore"),
    "knife": ((-0.1, -0.3, -0.1), 0, None),
}


def reset_pose():
    for pb in rig.pose.bones:
        pb.rotation_mode = "XYZ"
        pb.rotation_euler = (0, 0, 0)
        pb.location = (0, 0, 0)
    rig.location = (0, 0, BASE)
    rig.rotation_euler = (0, 0, 0)
    for ob in (AIM, T_R, T_L):
        if ob.animation_data:
            ob.animation_data.action = None
    if rig.animation_data:
        rig.animation_data.action = None
    AIM.location, AIM.rotation_euler = (0, 0, 0), (0, 0, 0)


def arm(prop, base_pose=None):
    """Put `prop` in the right hand and the left hand on its fore grip.
    Returns the world point of anything the clip needs (hammer, cylinder)."""
    reset_pose()
    for root in PROP.values():
        root.parent = None
        root.matrix_world = Matrix.Identity(4)
    show(prop)
    if prop is None or prop not in HOLDS:
        IK["R"].influence = IK["L"].influence = 0.0
        return
    at, yaw, fore = HOLDS[prop]
    pose = mirror({**(base_pose or {}), **HOLD_HANDS})
    for n, v in pose.items():
        if n in rig.pose.bones:
            rig.pose.bones[n].rotation_euler = [R(a) for a in v]
    T_R.location = at
    T_R.rotation_euler = (0, 0, 0)
    ik = IK["R"]
    ik.use_rotation = False
    ik.influence = 1.0
    IK["L"].influence = 0.0
    bpy.context.view_layer.update()
    # the forearm's way at the grip becomes the target's, so aim and recoil
    # (turning the pivot) turn the hand and the prop with it
    fa = (rig.matrix_world @ rig.pose.bones["lowerarm02.R"].matrix).to_3x3().normalized()
    T_R.matrix_world = Matrix.Translation(T_R.matrix_world.translation) @ fa.to_4x4()
    ik.use_rotation = True
    bpy.context.view_layer.update()
    root = PROP[prop]
    aimrot = Matrix.Rotation(R(yaw), 4, "Z")
    hand = rig.matrix_world @ rig.pose.bones["wrist.R"].head
    root.matrix_world = Matrix.Translation(hand + Vector((0, 0, -0.035))) @ aimrot
    bpy.context.view_layer.update()
    mw = root.matrix_world.copy()
    root.parent, root.parent_type, root.parent_bone = rig, "BONE", "wrist.R"
    bpy.context.view_layer.update()
    root.matrix_world = mw
    if fore:
        T_L.parent = root
        T_L.matrix_parent_inverse = Matrix.Identity(4)
        T_L.location = root["fore"]
        IK["L"].influence = 1.0
    bpy.context.view_layer.update()
    gap_r = (rig.matrix_world @ rig.pose.bones["wrist.R"].head - T_R.matrix_world.translation).length
    gap_l = (rig.matrix_world @ rig.pose.bones["wrist.L"].head - T_L.matrix_world.translation).length if fore else 0
    if gap_r > 0.02 or gap_l > 0.04:
        say(f"  {prop}: a hand short of its grip by R {gap_r * 1000:.0f} mm, L {gap_l * 1000:.0f} mm")


# ---- keying ---------------------------------------------------------------------------
SOLID = [o for o in OB if (o.get("costume") and o.get("layer_name") != "hair") or o == body]
CURRENT = {"ground": True}


def lowest():
    """The lowest point of the dressed figure as posed now."""
    d = bpy.context.evaluated_depsgraph_get()
    d.update()
    low = 1e9
    for ob in SOLID:
        e = ob.evaluated_get(d)
        m = e.to_mesh()
        if len(m.vertices):
            low = min(low, min((ob.matrix_world @ v.co).z for v in m.vertices))
        e.to_mesh_clear()
    return low


def key(f, pose, loc=(0, 0, 0), rot=(0, 0, 0), ik=None, aim=None, kick=(0, 0, 0), hips=False, settle=None):
    """One key: bone angles, the whole body's offset and turn (in place),
    IK influence (right, left), the aim pivot's turn (degrees: pitch up,
    roll, yaw left) and the prop's kick back along the aim (metres)."""
    p = mirror(pose)
    for name in BONES:
        pb = rig.pose.bones[name]
        pb.rotation_euler = [R(a) for a in p.get(name, (0, 0, 0))]
        pb.keyframe_insert("rotation_euler", frame=f)
    e = [R(a) for a in rot]
    off = Vector(loc)
    if not CURRENT["ground"]:
        # Long Time Comin's copy: in the air the core carries the body, so an
        # airborne clip keeps the hips at standing height and moves only the
        # limbs and the turn (Style Lab's lifted the figure to a jump's
        # height for its stills; in the game that height was counted twice)
        off.z = 0.0
    if hips:
        # turn about the hips, not the feet: `loc` is then the hips' move
        from mathutils import Euler
        h = Vector((HIP.x, HIP.y, HIP.z))
        off = off + h - Euler(e, "XYZ").to_matrix() @ h
    rig.location = (off.x, off.y, off.z + BASE)
    rig.rotation_euler = e
    if settle if settle is not None else CURRENT["ground"]:
        # on the ground: the lowest point of the figure rests on the floor,
        # whatever the knees and hips are doing (the height given is ignored)
        if ik is not None:
            for s_, v in zip("RL", ik):
                IK[s_].influence = v
        bpy.context.view_layer.update()
        rig.location.z -= lowest()
    rig.keyframe_insert("location", frame=f)
    rig.keyframe_insert("rotation_euler", frame=f)
    if ik is not None:
        for s, v in zip("RL", ik):
            IK[s].influence = v
            rig.keyframe_insert(f'pose.bones["lowerarm02.{s}"].constraints["ltc_ik"].influence', frame=f)
    a = aim or (0, 0, 0)
    AIM.rotation_euler = (R(-a[0]), R(a[1]), R(a[2]))
    AIM.location = kick
    AIM.keyframe_insert("rotation_euler", frame=f)
    AIM.keyframe_insert("location", frame=f)


def begin(name, frames, loop, prop=None, ground=True, view="three_quarter", base_pose=None):
    arm(prop, base_pose)
    rig.animation_data_create()
    rig.animation_data.action = bpy.data.actions.new(f"diver__{name}__work")
    AIM.animation_data_create()
    AIM.animation_data.action = bpy.data.actions.new(f"diver__{name}__aim")
    META[name] = {"frames": frames, "loop": loop, "prop": prop, "ground": ground, "view": view}
    CURRENT["ground"] = ground
    root = PROP.get(prop)
    if root is not None and root.parent == rig:
        # the prop's place in the right hand, for the clip check and the bake
        wrist = rig.matrix_world @ rig.pose.bones["wrist.R"].matrix
        META[name]["in_hand"] = [list(r) for r in (wrist.inverted() @ root.matrix_world)]


def ease(loop):
    for ob in (rig, AIM, T_L):
        ad = ob.animation_data
        if not ad or not ad.action:
            continue
        for fc in getattr(ad.action, "fcurves", []):
            for kp in fc.keyframe_points:
                kp.interpolation = "BEZIER"
                kp.handle_left_type = kp.handle_right_type = "AUTO_CLAMPED"
            if loop and not fc.modifiers:
                fc.modifiers.new("CYCLES")


# ---- gaits --------------------------------------------------------------------------
def stride_frames(speed):
    return max(14, round((0.7 + 0.5 * speed) / speed * FPS))


def gait(name, speed, swing, knee, lean, bob, prop="carbine", back=False, extra=None, view="side", lift=0.0):
    """One stride in place, contact, down, passing, up for each foot; the gun
    held throughout, as the game draws it."""
    n = stride_frames(speed)
    begin(name, n, True, prop, view=view, base_pose=STANCE)
    sgn = -1 if back else 1
    for k in range(9):
        f = 1 + n * k / 8
        s = 1 if (k % 8) < 4 else -1
        ph = k % 4
        lead = [(-swing, 8), (-swing * 0.3, 25), (swing * 0.5, 15), (swing * 0.85, knee * 0.4)][ph]
        trail = [(swing * 0.8, knee * 0.4), (swing * 0.1, knee), (-swing * 0.55, knee * 1.05), (-swing * 0.95, knee * 0.5)][ph]
        L, Rt = (lead, trail) if s > 0 else (trail, lead)
        L, Rt = (L[0] * sgn, L[1]), (Rt[0] * sgn, Rt[1])
        pose = {**HOLD_HANDS, "spine05": (lean * 0.4, 0, 0), "spine03": (lean * 0.6, s * 4, 0), "neck01": (-lean * 0.5, 0, 0),
                "upperleg01.L": (L[0], 0, 2), "upperleg01.R": (Rt[0], 0, -2), "lowerleg01.L": (L[1], 0, 0),
                "lowerleg01.R": (Rt[1], 0, 0), "foot.L": (-L[1] * 0.3, 0, 0), "foot.R": (-Rt[1] * 0.3, 0, 0), **(extra or {})}
        z = [0.0, -bob, 0.0, bob][ph] + lift
        key(f, pose, (0, 0, z), ik=(1, 1), aim=(0, 0, -s * 2))
    ease(True)
    return n


def strafe(name, s):
    """Side steps to the figure's left (s=1) or right (s=-1), facing ahead."""
    n = 18
    begin(name, n, True, "carbine", view="front", base_pose=STANCE)
    for k in range(5):
        f = 1 + n * k / 4
        a = [0, 1, 0, -1, 0][k]
        open_ = [1, 0, -1, 0, 1][k]
        pose = {**HOLD_HANDS, "spine03": (4, 0, -s * 3),
                "upperleg01.L": (-6 * a, 0, 10 * open_ + 2), "upperleg01.R": (6 * a, 0, -10 * open_ - 2),
                "lowerleg01.L": (18 + 22 * max(a, 0), 0, 0), "lowerleg01.R": (18 + 22 * max(-a, 0), 0, 0),
                "foot.*": (-6, 0, 0)}
        key(f, pose, (0, 0, -0.02 * abs(open_)), rot=(0, s * 3 * open_, 0), ik=(1, 1))
    ease(True)


# ---- clip definitions ---------------------------------------------------------------
CLIPS = {}


def clip(fn):
    CLIPS[fn.__name__] = fn
    return fn


def still(name, pose, frames=60, prop="carbine", loc=(0, 0, 0), rot=(0, 0, 0), breathe=2.0, ik=(1, 1), aim=(0, 0, 0),
          view="three_quarter", ground=True):
    begin(name, frames, True, prop, ground=ground, view=view, base_pose=pose)
    for i, f in enumerate((1, frames // 2 + 1, frames + 1)):
        b = breathe if i == 1 else 0.0
        p = dict(pose)
        p["spine02"] = (pose.get("spine02", (0, 0, 0))[0] + b, 0, 0)
        key(f, p, loc, rot, ik=ik, aim=(aim[0] + b * 0.5, aim[1], aim[2]))
    ease(True)


@clip
def idle():
    still("idle", {**HOLD_HANDS, **STANCE})


@clip
def holding_idle():
    # the Holding's figure: the gun lowered, at ease
    still("holding_idle", {**HOLD_HANDS, **STANCE, "neck01": (6, 0, 0)}, aim=(-30, 0, 10), breathe=3.0)


@clip
def run():
    gait("run", 9.5, 40, 100, 14, 0.05, lift=0.04)


@clip
def backpedal():
    gait("backpedal", 4.0, 26, 70, -4, 0.025, back=True)


@clip
def strafe_left():
    strafe("strafe_left", 1)


@clip
def strafe_right():
    strafe("strafe_right", -1)


CROUCHED = {"spine05": (14, 0, 0), "spine03": (10, 0, 0), "upperleg01.*": (-50, 0, -4), "lowerleg01.*": (85, 0, 0),
            "foot.*": (-30, 0, 0)}
TUCK = {"spine05": (24, 0, 0), "spine03": (12, 0, 0), "upperleg01.*": (-80, 0, -6), "lowerleg01.*": (110, 0, 0),
        "foot.*": (20, 0, 0)}
SPREAD = {"upperleg01.L": (-14, 0, 8), "upperleg01.R": (10, 0, -8), "lowerleg01.*": (24, 0, 0), "foot.*": (25, 0, 0)}


@clip
def jump():
    begin("jump", 12, False, "carbine", ground=False)
    key(1, {**HOLD_HANDS, **STANCE}, ik=(1, 1), settle=True)
    key(4, {**HOLD_HANDS, **CROUCHED}, (0, 0, -0.16), ik=(1, 1), aim=(-4, 0, 0), settle=True)
    key(8, {**HOLD_HANDS, "upperleg01.*": (6, 0, -2), "lowerleg01.*": (6, 0, 0), "foot.*": (40, 0, 0)}, (0, 0, 0.12), ik=(1, 1), aim=(6, 0, 0))
    key(12, {**HOLD_HANDS, **SPREAD}, (0, 0, 0.3), ik=(1, 1), aim=(2, 0, 0))
    ease(False)


@clip
def air():
    begin("air", 24, True, "carbine", ground=False)
    for f, b in ((1, 0), (13, 1), (25, 0)):
        key(f, {**HOLD_HANDS, **SPREAD, "lowerleg01.L": (24 + 8 * b, 0, 0), "lowerleg01.R": (30 - 8 * b, 0, 0)},
            (0, 0, 0.4 + 0.01 * b), ik=(1, 1))
    ease(True)


@clip
def air_jump():
    # a second (third, fourth) jump in the air: a tuck and a full forward
    # somersault about the hips, the gun kept on the aim's side
    begin("air_jump", 18, False, "carbine", ground=False, view="side")
    key(1, {**HOLD_HANDS, **SPREAD}, (0, 0, 0.4), ik=(1, 1), hips=True)
    key(5, {**HOLD_HANDS, **TUCK}, (0, 0, 0.62), rot=(-90, 0, 0), ik=(1, 1), hips=True)
    key(10, {**HOLD_HANDS, **TUCK}, (0, 0, 0.86), rot=(-200, 0, 0), ik=(1, 1), hips=True)
    key(15, {**HOLD_HANDS, **TUCK}, (0, 0, 0.78), rot=(-300, 0, 0), ik=(1, 1), hips=True)
    key(18, {**HOLD_HANDS, **SPREAD}, (0, 0, 0.7), rot=(-360, 0, 0), ik=(1, 1), hips=True)
    ease(False)


@clip
def fall():
    begin("fall", 24, True, "carbine", ground=False)
    for f, b in ((1, 0), (13, 1), (25, 0)):
        key(f, {**HOLD_HANDS, "upperleg01.L": (-8 - 6 * b, 0, 6), "upperleg01.R": (4 + 6 * b, 0, -6),
                "lowerleg01.*": (18, 0, 0), "foot.*": (30, 0, 0), "spine03": (-4, 0, 0)}, (0, 0, 0.5), ik=(1, 1), aim=(-6, 0, 0))
    ease(True)


@clip
def land():
    begin("land", 12, False, "carbine")
    key(1, {**HOLD_HANDS, **SPREAD}, (0, 0, 0.06), ik=(1, 1))
    key(4, {**HOLD_HANDS, **CROUCHED, "spine05": (24, 0, 0)}, (0, 0, -0.2), ik=(1, 1), aim=(-10, 0, 0))
    key(12, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    ease(False)


@clip
def dash():
    # a burst ahead: a long low lunge, the coat's ghosts are the game's
    begin("dash", 10, False, "carbine", view="side")
    lunge = {**HOLD_HANDS, "spine05": (30, 0, 0), "spine03": (12, 0, 0), "neck01": (-26, 0, 0),
             "upperleg01.L": (-60, 0, 2), "lowerleg01.L": (70, 0, 0), "upperleg01.R": (30, 0, -2),
             "lowerleg01.R": (30, 0, 0), "foot.L": (-10, 0, 0), "foot.R": (30, 0, 0)}
    key(1, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    key(3, lunge, (0, 0, -0.08), ik=(1, 1), aim=(-8, 0, 0))
    key(8, lunge, (0, 0, -0.06), ik=(1, 1), aim=(-6, 0, 0))
    key(10, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    ease(False)


HOOK_ARM = {"upperarm01.L": (115, 0, -10), "lowerarm01.L": (4, 0, 0), "spine03": (-6, 0, -10), **side(FIST, "L")}


@clip
def hook_fire():
    begin("hook_fire", 12, False, "big_iron", view="three_quarter", base_pose=STANCE)
    key(1, {**STANCE, **side(GRIP, "R")}, ik=(1, 0))
    key(4, {**STANCE, **side(GRIP, "R"), **HOOK_ARM, "upperarm01.L": (100, 0, -16), "lowerarm01.L": (40, 0, 0)}, ik=(1, 0))
    key(6, {**STANCE, **side(GRIP, "R"), **HOOK_ARM}, ik=(1, 0))
    key(9, {**STANCE, **side(GRIP, "R"), **HOOK_ARM, "upperarm01.L": (120, 0, -8)}, ik=(1, 0))
    key(12, {**STANCE, **side(GRIP, "R"), **HOOK_ARM}, ik=(1, 0))
    ease(False)


@clip
def hook_pull():
    # hauled along the line: the hook arm ahead and up, the body trailing it
    begin("hook_pull", 20, True, "big_iron", ground=False, view="side")
    for f, b in ((1, 0), (11, 1), (21, 0)):
        key(f, {**side(GRIP, "R"), **HOOK_ARM, "upperarm01.L": (150, 0, -6), "spine05": (-8, 0, 0),
                "upperleg01.L": (16 + 6 * b, 0, 4), "upperleg01.R": (24 - 6 * b, 0, -4), "lowerleg01.*": (40, 0, 0),
                "foot.*": (40, 0, 0)}, (0, 0, 0.45), rot=(-28, 0, 0), ik=(1, 0), hips=True)
    ease(True)


@clip
def parry():
    # Objection: the left forearm swept across in front, the body turned into it
    begin("parry", 12, False, "carbine", base_pose=STANCE)
    sweep = {**HOLD_HANDS, **STANCE, "spine03": (6, 0, 18), "upperarm01.L": (85, 0, 0), "lowerarm01.L": (60, 0, 0), **side(OPEN, "L")}
    key(1, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    key(3, sweep, ik=(1, 0), aim=(0, 0, -18))
    key(7, {**sweep, "spine03": (6, 0, 8), "upperarm01.L": (90, 0, 14)}, ik=(1, 0), aim=(0, 0, -8))
    key(12, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    ease(False)


@clip
def wall():
    # the Injunction: the left palm thrust ahead, braced
    begin("wall", 14, False, "carbine", base_pose=STANCE)
    brace = {**HOLD_HANDS, "upperleg01.L": (-28, 0, 4), "lowerleg01.L": (34, 0, 0), "upperleg01.R": (16, 0, -4),
             "lowerleg01.R": (14, 0, 0), "spine03": (10, 0, -8), "upperarm01.L": (88, 0, 4), "lowerarm01.L": (6, 0, 0),
             "wrist.L": (-60, 0, 0), **side(OPEN, "L")}
    key(1, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    key(5, brace, (0, 0, -0.04), ik=(1, 0), aim=(-12, 0, 6))
    key(10, brace, (0, 0, -0.04), ik=(1, 0), aim=(-12, 0, 6))
    key(14, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    ease(False)


@clip
def pound_drop():
    # the gavel's fall: knees to the chest, the right fist raised to strike
    begin("pound_drop", 12, True, None, ground=False, view="side")
    for f, b in ((1, 0), (7, 1), (13, 0)):
        key(f, {**TUCK, **side(FIST, "R"), **side(FIST, "L"), "upperarm01.R": (160, 0, 10), "lowerarm01.R": (40, 0, 0),
                "upperarm01.L": (40, 0, -30), "lowerarm01.L": (90, 0, 0)}, (0, 0, 0.5 - 0.02 * b), rot=(8, 0, 0), hips=True)
    ease(True)


@clip
def pound_land():
    begin("pound_land", 16, False, None, view="three_quarter")
    slam = {**side(FIST, "R"), **side(FIST, "L"), "spine05": (40, 0, 0), "spine03": (20, 0, 0), "neck01": (-30, 0, 0),
            "upperleg01.*": (-70, 0, -14), "lowerleg01.*": (110, 0, 0), "foot.*": (-30, 0, 0),
            "upperarm01.R": (60, 0, 20), "lowerarm01.R": (20, 0, 0), "upperarm01.L": (20, 0, -50), "lowerarm01.L": (40, 0, 0)}
    key(1, {**TUCK, **side(FIST, "R"), "upperarm01.R": (160, 0, 10), "lowerarm01.R": (40, 0, 0)}, (0, 0, 0.2))
    key(4, slam, (0, 0, -0.34))
    key(10, slam, (0, 0, -0.33))
    key(16, {**CROUCHED, **side(FIST, "R")}, (0, 0, -0.18))
    ease(False)


@clip
def bounce():
    # up off the shockwave, to a little above where the drop began
    begin("bounce", 16, False, None, ground=False, view="side")
    key(1, {**CROUCHED, **ARMS_DOWN}, (0, 0, -0.18))
    key(5, {"upperleg01.*": (4, 0, -4), "lowerleg01.*": (6, 0, 0), "foot.*": (45, 0, 0), "upperarm01.*": (150, 0, -20),
            "lowerarm01.*": (10, 0, 0), **OPEN}, (0, 0, 0.3))
    key(16, {**SPREAD, **ARMS_DOWN}, (0, 0, 0.6))
    ease(False)


@clip
def time_stop():
    # the left hand raised, opened, then closed on the moment
    begin("time_stop", 36, False, "carbine", base_pose=STANCE)
    up = {**HOLD_HANDS, **STANCE, "upperarm01.L": (150, 0, -10), "lowerarm01.L": (20, 0, 0), "neck01": (-14, 0, 0),
          "spine03": (-6, 0, 0)}
    key(1, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    key(10, {**up, **side(OPEN, "L")}, ik=(1, 0), aim=(-20, 0, 0))
    key(20, {**up, **side(OPEN, "L"), "wrist.L": (0, 0, 20)}, ik=(1, 0), aim=(-20, 0, 0))
    key(24, {**up, **side(FIST, "L"), "upperarm01.L": (140, 0, -10)}, ik=(1, 0), aim=(-20, 0, 0))
    key(36, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    ease(False)


@clip
def bench():
    # the Bench asked for: the left palm swept down and out, where it is laid
    begin("bench", 18, False, "carbine", base_pose=STANCE)
    key(1, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    key(6, {**HOLD_HANDS, **STANCE, "upperarm01.L": (70, 0, 10), "lowerarm01.L": (60, 0, 0), **side(OPEN, "L")}, ik=(1, 0))
    key(12, {**HOLD_HANDS, **STANCE, "spine05": (14, 0, 0), "upperarm01.L": (50, 0, -30), "lowerarm01.L": (10, 0, 0),
             "wrist.L": (40, 0, 0), **side(OPEN, "L")}, (0, 0, -0.05), ik=(1, 0))
    key(18, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    ease(False)


SLASH_A = {**side(GRIP, "R"), "spine03": (4, 0, -30), "spine05": (0, 0, -10), "upperarm01.R": (70, 0, 70), "lowerarm01.R": (70, 0, 0)}
SLASH_B = {**side(GRIP, "R"), "spine03": (8, 0, 34), "spine05": (0, 0, 12), "upperarm01.R": (80, 0, -60), "lowerarm01.R": (10, 0, 0)}


@clip
def slash():
    # Summary Judgment: the diver's knife drawn across from right to left
    begin("slash", 14, False, "knife", base_pose=STANCE)
    key(1, {**STANCE, **side(GRIP, "R"), "upperarm01.R": (30, 0, 30), "lowerarm01.R": (60, 0, 0)}, ik=(0, 0))
    key(4, {**STANCE, **SLASH_A}, ik=(0, 0))
    key(8, {**STANCE, **SLASH_B}, ik=(0, 0))
    key(14, {**STANCE, **side(GRIP, "R"), "upperarm01.R": (30, 0, 30), "lowerarm01.R": (60, 0, 0)}, ik=(0, 0))
    ease(False)


@clip
def lunge_slash():
    begin("lunge_slash", 18, False, "knife", base_pose=STANCE, view="three_quarter")
    step = {"upperleg01.L": (-50, 0, 4), "lowerleg01.L": (50, 0, 0), "upperleg01.R": (24, 0, -4), "lowerleg01.R": (20, 0, 0),
            "foot.R": (20, 0, 0)}
    key(1, {**STANCE, **side(GRIP, "R"), "upperarm01.R": (30, 0, 30), "lowerarm01.R": (60, 0, 0)}, ik=(0, 0))
    key(5, {**SLASH_A, **step, "spine05": (18, 0, -10)}, (0, 0, -0.08), ik=(0, 0))
    key(10, {**SLASH_B, **step, "spine05": (22, 0, 12)}, (0, 0, -0.1), ik=(0, 0))
    key(18, {**STANCE, **side(GRIP, "R"), "upperarm01.R": (30, 0, 30), "lowerarm01.R": (60, 0, 0)}, ik=(0, 0))
    ease(False)


GRAB_UP = {"upperarm01.L": (150, 0, 6), "lowerarm01.L": (30, 0, 0), **side(FIST, "L"), "spine03": (-4, 0, -6)}


@clip
def grab():
    begin("grab", 10, False, "big_iron", base_pose=STANCE)
    key(1, {**STANCE, **side(GRIP, "R")}, ik=(1, 0))
    key(5, {**STANCE, **side(GRIP, "R"), "upperarm01.L": (90, 0, 4), "lowerarm01.L": (6, 0, 0), "spine05": (16, 0, 0),
            **side(OPEN, "L")}, (0, 0, -0.03), ik=(1, 0))
    key(10, {**STANCE, **side(GRIP, "R"), "upperarm01.L": (95, 0, 4), "lowerarm01.L": (20, 0, 0), **side(FIST, "L")}, ik=(1, 0))
    ease(False)


@clip
def hold_grabbed():
    still("hold_grabbed", {**STANCE, **side(GRIP, "R"), **GRAB_UP}, prop="big_iron", ik=(1, 0))


@clip
def throw():
    begin("throw", 14, False, "big_iron", base_pose=STANCE)
    key(1, {**STANCE, **side(GRIP, "R"), **GRAB_UP}, ik=(1, 0))
    key(4, {**STANCE, **side(GRIP, "R"), **GRAB_UP, "upperarm01.L": (165, 0, -10), "lowerarm01.L": (40, 0, 0), "spine03": (-14, 0, -10)}, ik=(1, 0))
    key(8, {**side(GRIP, "R"), "upperleg01.L": (-34, 0, 4), "lowerleg01.L": (30, 0, 0), "upperleg01.R": (16, 0, -4),
            "spine03": (22, 0, 16), "spine05": (14, 0, 0), "upperarm01.L": (60, 0, 6), "lowerarm01.L": (6, 0, 0), **side(OPEN, "L")},
        (0, 0, -0.04), ik=(1, 0))
    key(14, {**STANCE, **side(GRIP, "R")}, ik=(1, 0))
    ease(False)


@clip
def hurt():
    begin("hurt", 12, False, "carbine", base_pose=STANCE)
    key(1, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    key(3, {**HOLD_HANDS, **STANCE, "spine03": (-14, 0, 8), "spine05": (-6, 0, 0), "neck01": (-16, 0, 6)}, ik=(1, 1), aim=(14, 0, 8))
    key(12, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    ease(False)


@clip
def recess():
    # a fatal hit cancelled: thrown to one knee, and up again
    begin("recess", 30, False, "carbine", base_pose=STANCE)
    kneel = {**HOLD_HANDS, "spine05": (20, 0, 0), "neck01": (14, 0, 0), "upperleg01.L": (-80, 0, 0), "lowerleg01.L": (90, 0, 0),
             "upperleg01.R": (10, 0, -4), "lowerleg01.R": (120, 0, 0), "foot.R": (40, 0, 0)}
    key(1, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    key(5, {**HOLD_HANDS, "spine03": (-20, 0, 0)}, ik=(1, 1), aim=(20, 0, 0))
    key(12, kneel, (0, 0, -0.42), ik=(1, 1), aim=(-30, 0, 0))
    key(20, kneel, (0, 0, -0.42), ik=(1, 1), aim=(-20, 0, 0))
    key(30, {**HOLD_HANDS, **STANCE}, ik=(1, 1))
    ease(False)


@clip
def death():
    begin("death", 48, False, None, base_pose=STANCE, view="side")
    key(1, {**RELAX, **STANCE, **ARMS_DOWN})
    key(6, {**RELAX, **ARMS_DOWN, "spine03": (-24, 0, 6), "neck01": (-20, 0, 0)})
    key(16, {**RELAX, "spine05": (20, 0, 0), "upperleg01.*": (-80, 0, 0), "lowerleg01.*": (100, 0, 0), "foot.*": (40, 0, 0),
             **ARMS_DOWN}, (0, 0, -0.45), ik=(0, 0))
    key(30, {**RELAX, "spine05": (10, 0, 0), "upperleg01.*": (-40, 0, 0), "lowerleg01.*": (60, 0, 0), "upperarm01.*": (60, 0, -40),
             "lowerarm01.*": (30, 0, 0)}, (0, -0.4, -0.55), rot=(70, 0, 0), ik=(0, 0))
    key(40, {**RELAX, "upperleg01.*": (-6, 0, -4), "lowerleg01.*": (10, 0, 0), "upperarm01.*": (100, 0, -40),
             "lowerarm01.*": (20, 0, 0), "neck01": (-20, 0, 0)}, (0, -0.85, -0.68), rot=(90, 0, 0), ik=(0, 0))
    key(48, {**RELAX, "upperleg01.*": (-6, 0, -4), "lowerleg01.*": (10, 0, 0), "upperarm01.*": (100, 0, -40),
             "lowerarm01.*": (20, 0, 0), "neck01": (-20, 0, 0)}, (0, -0.85, -0.68), rot=(90, 0, 0), ik=(0, 0))
    ease(False)


@clip
def swim():
    # a stroke under the water, the body laid forward, a frog kick
    begin("swim", 36, True, None, ground=False, view="side")
    for k, f in enumerate((1, 10, 19, 28, 37)):
        a = [0, 1, 2, 1, 0][k]
        arms = [{"upperarm01.*": (150, 0, -10), "lowerarm01.*": (10, 0, 0)},
                {"upperarm01.*": (90, 0, -60), "lowerarm01.*": (20, 0, 0)},
                {"upperarm01.*": (20, 0, -40), "lowerarm01.*": (70, 0, 0)},
                {"upperarm01.*": (110, 0, -40), "lowerarm01.*": (75, 0, 0)},
                {"upperarm01.*": (150, 0, -10), "lowerarm01.*": (10, 0, 0)}][k]
        legs = {"upperleg01.*": (-40 * (a == 2), 0, -20 * (a == 2)), "lowerleg01.*": (80 * (a == 2) + 10, 0, 0), "foot.*": (40, 0, 0)}
        key(f, {**OPEN, **arms, **legs, "neck01": (-30, 0, 0)}, (0, 0, 0.6), rot=(60, 0, 0), hips=True)
    ease(True)


@clip
def sink():
    # the open sea: sinking slowly, arms drifting out
    begin("sink", 48, True, None, ground=False)
    for f, b in ((1, 0), (25, 1), (49, 0)):
        key(f, {**RELAX, "upperarm01.*": (20 + 10 * b, 0, 10 + 8 * b), "lowerarm01.*": (30, 0, 0), "upperleg01.L": (-10 - 6 * b, 0, 4),
                "upperleg01.R": (6 + 4 * b, 0, -4), "lowerleg01.*": (20 + 6 * b, 0, 0), "foot.*": (30, 0, 0)}, (0, 0, 0.6 + 0.02 * b))
    ease(True)


# ---- the guns -------------------------------------------------------------------
def fire(name, prop, frames, kicks, power, loop=False, shake=0.0):
    """Shots at the listed frames: the prop kicks back and its muzzle rises,
    the shoulders take it."""
    begin(name, frames, loop, prop, base_pose=STANCE)
    hold = {**HOLD_HANDS, **STANCE}
    key(1, hold, ik=(1, 1))
    for f in kicks:
        key(f, {**hold, "spine03": (4 - power * 0.6, 0, 0)}, ik=(1, 1), aim=(power, 0, 0), kick=(0, 0.02 * power / 5, 0))
        key(f + 2, hold, ik=(1, 1), aim=(power * 0.2, shake, 0))
    key(frames + 1 if loop else frames, hold, ik=(1, 1))
    ease(loop)


@clip
def carbine_fire():
    fire("carbine_fire", "carbine", 8, [1, 5], 3, loop=True)


@clip
def carbine_burst():
    fire("carbine_burst", "carbine", 16, [1, 3, 5], 4)


@clip
def redactor_fire():
    fire("redactor_fire", "redactor", 6, [1, 4], 2, loop=True)


@clip
def filibuster_fire():
    # spinning up: the rounds come faster the longer it is held
    fire("filibuster_fire", "filibuster", 40, [1, 9, 16, 22, 27, 31, 34, 36, 38], 3, shake=2.0)


@clip
def scattergun_fire():
    begin("scattergun_fire", 20, False, "scattergun", base_pose=STANCE)
    hold = {**HOLD_HANDS, **STANCE}
    key(1, hold, ik=(1, 1))
    key(2, {**hold, "spine03": (-6, 0, 0)}, ik=(1, 1), aim=(14, 0, 0), kick=(0, 0.06, 0))
    key(7, hold, ik=(1, 1), aim=(3, 0, 0))
    # the pump: the fore hand back and forward along the barrel
    tl = T_L.location.copy()
    T_L.animation_data_create()
    T_L.animation_data.action = bpy.data.actions.new("diver__scattergun_fire__fore")
    for f, dy in ((1, 0), (9, 0), (12, 0.1), (15, 0), (20, 0)):
        T_L.location = tl + Vector((0, dy, 0))
        T_L.keyframe_insert("location", frame=f)
    key(20, hold, ik=(1, 1))
    ease(False)


@clip
def launcher_fire():
    begin("launcher_fire", 24, False, "launcher", base_pose=STANCE)
    hold = {**HOLD_HANDS, **STANCE}
    key(1, hold, ik=(1, 1))
    key(3, {**hold, "spine03": (-12, 0, 0), "spine05": (-4, 0, 0), "upperleg01.R": (10, 0, -4)}, (0, 0.03, 0), ik=(1, 1),
        aim=(18, 0, 0), kick=(0, 0.08, 0))
    key(12, hold, ik=(1, 1), aim=(4, 0, 0))
    key(24, hold, ik=(1, 1))
    ease(False)


@clip
def ripsaw_fire():
    fire("ripsaw_fire", "ripsaw", 14, [1], 7)


@clip
def ripsaw_rev():
    # the trigger held: the lodged blades revved, the gun shuddering
    begin("ripsaw_rev", 12, True, "ripsaw", base_pose=STANCE)
    hold = {**HOLD_HANDS, **STANCE}
    for f, s in ((1, 0), (3, 1), (5, -1), (7, 1), (9, -1), (11, 1), (13, 0)):
        key(f, hold, ik=(1, 1), aim=(1.5 * s, 2 * s, 1 * s), kick=(0, 0.004 * abs(s), 0))
    ease(True)


@clip
def big_iron_fire():
    begin("big_iron_fire", 22, False, "big_iron", base_pose=STANCE)
    hold = {**STANCE, **side(GRIP, "R"), **ARMS_DOWN, "upperarm01.L": (10, 0, -24)}
    key(1, hold, ik=(1, 0))
    key(3, {**hold, "spine03": (-8, 0, 4)}, ik=(1, 0), aim=(30, 0, 0), kick=(0, 0.07, 0))
    key(14, hold, ik=(1, 0), aim=(4, 0, 0))
    key(22, hold, ik=(1, 0))
    ease(False)


def big_iron_point(name):
    root = PROP["big_iron"]
    return root.matrix_world @ Vector(PROPS["props"]["big_iron"][name])


@clip
def big_iron_fan():
    # Fan the hammer: the left palm sweeps the hammer back six times, fast
    begin("big_iron_fan", 40, False, "big_iron", base_pose=STANCE)
    hold = {**STANCE, **side(GRIP, "R"), **side(OPEN, "L"), "spine03": (6, 0, 6)}
    T_L.parent = PROP["big_iron"]
    T_L.matrix_parent_inverse = Matrix.Identity(4)
    T_L.animation_data_create()
    T_L.animation_data.action = bpy.data.actions.new("diver__big_iron_fan__fore")
    hm = Vector(PROPS["props"]["big_iron"]["hammer"])
    key(1, hold, ik=(1, 0))
    T_L.location = hm + Vector((0.0, 0.02, 0.1))
    T_L.keyframe_insert("location", frame=1)
    key(4, hold, ik=(1, 1))
    for i in range(6):
        f = 5 + i * 5
        T_L.location = hm + Vector((0.0, 0.03, 0.06))
        T_L.keyframe_insert("location", frame=f)
        T_L.location = hm + Vector((0.0, 0.12, 0.02))
        T_L.keyframe_insert("location", frame=f + 2)
        key(f + 2, hold, ik=(1, 1), aim=(16, 0, -2 + 2 * (i % 2)), kick=(0, 0.04, 0))
        key(f + 4, hold, ik=(1, 1), aim=(4, 0, 0))
    key(40, hold, ik=(1, 0))
    ease(False)


@clip
def big_iron_reload():
    # loads one shell at a time: the gate opened, the empties shaken out
    # (muzzle up), six shells fetched from the belt one by one and turned in,
    # the cylinder spun and the gun brought back up
    n = 170
    begin("big_iron_reload", n, False, "big_iron", base_pose=STANCE)
    hold = {**STANCE, **side(GRIP, "R")}
    belt = {"upperarm01.L": (-5, 0, -15), "lowerarm01.L": (35, 0, 0), "wrist.L": (0, 0, 20), **side(GRIP, "L"),
            "spine03": (8, 0, 10)}
    cyl = Vector(PROPS["props"]["big_iron"]["cylinder"])
    T_L.parent = PROP["big_iron"]
    T_L.matrix_parent_inverse = Matrix.Identity(4)
    T_L.location = cyl + Vector((0.07, 0.0, 0.0))
    key(1, hold, ik=(1, 0))
    key(10, {**hold, "spine03": (14, 0, 10)}, ik=(1, 0), aim=(-15, 70, 38), kick=(0.0, -0.05, 0.0))
    key(18, {**hold, "spine03": (6, 0, 10)}, ik=(1, 0), aim=(60, 40, 16), kick=(0.0, 0.06, 0.0))
    key(24, {**hold, "spine03": (14, 0, 10)}, ik=(1, 0), aim=(-15, 70, 38), kick=(0.0, -0.05, 0.0))
    for i in range(6):
        f = 28 + i * 20
        key(f, {**hold, **belt}, ik=(1, 0), aim=(-15, 70, 38), kick=(0.0, -0.05, 0.0))
        key(f + 10, {**hold, **side(GRIP, "L"), "spine03": (14, 0, 10)}, ik=(1, 1), aim=(-15, 70, 38), kick=(0.0, -0.05, 0.0))
        key(f + 15, {**hold, **side(GRIP, "L"), "spine03": (14, 0, 10)}, ik=(1, 1), aim=(-15, 64, 38), kick=(0.0, -0.05, 0.0))
    key(n - 10, {**hold, "spine03": (8, 0, 4)}, ik=(1, 0), aim=(-10, 10, 4), kick=(0.0, 0.05, -0.02))
    key(n, hold, ik=(1, 0))
    ease(False)


def reload_two_hand(name, prop, n, trips, tilt=(-14, 30, 10)):
    """The left hand leaves the fore grip for the belt and back `trips` times
    (a magazine; shells one by one), the gun tilted to show its side."""
    begin(name, n, False, prop, base_pose=STANCE)
    hold = {**HOLD_HANDS, **STANCE}
    belt = {"upperarm01.L": (-5, 0, -15), "lowerarm01.L": (35, 0, 0), "wrist.L": (0, 0, 20), **side(GRIP, "L"),
            "spine03": (8, 0, 8)}
    key(1, hold, ik=(1, 1))
    key(6, hold, ik=(1, 1), aim=tilt)
    span = (n - 14) / trips
    for i in range(trips):
        f = 6 + span * i
        key(f + span * 0.4, {**hold, **belt}, ik=(1, 0), aim=tilt)
        key(f + span * 0.9, hold, ik=(1, 1), aim=(tilt[0], tilt[1] * 0.8, tilt[2]))
    key(n - 4, hold, ik=(1, 1), aim=(4, 0, 0), kick=(0, 0.02, 0))
    key(n, hold, ik=(1, 1))
    ease(False)


@clip
def carbine_reload():
    reload_two_hand("carbine_reload", "carbine", 44, 1)


@clip
def scattergun_reload():
    reload_two_hand("scattergun_reload", "scattergun", 64, 3, tilt=(-10, 40, 10))


@clip
def launcher_reload():
    reload_two_hand("launcher_reload", "launcher", 48, 1, tilt=(-20, 20, 10))


@clip
def ripsaw_reload():
    reload_two_hand("ripsaw_reload", "ripsaw", 44, 1, tilt=(-10, 30, 10))


@clip
def aim_up():
    still("aim_up", {**HOLD_HANDS, **STANCE, "spine03": (-10, 0, 0), "neck01": (-24, 0, 0)}, frames=30, aim=(45, 0, 0), breathe=0.5)


@clip
def aim_down():
    still("aim_down", {**HOLD_HANDS, **STANCE, "spine03": (16, 0, 0), "neck01": (20, 0, 0)}, frames=30, aim=(-45, 0, 0), breathe=0.5)


@clip
def trumpet():
    # the lullaby: the trumpet's mouthpiece to the helmet's front port, the
    # right fingers on the valves, swaying with the beat (a bar of four)
    begin("trumpet", 64, True, "trumpet", base_pose=STANCE)
    hold = {**STANCE, **side(GRIP, "L"), **side(OPEN, "R"), "neck01": (-6, 0, 0)}
    for k in range(5):
        f = 1 + 16 * k
        s = [0, 1, 0, -1, 0][k]
        valves = {f"finger{d}-1.R": (40 if (k + d) % 2 else 10, 0, 0) for d in (2, 3, 4)}
        key(f, {**hold, **valves, "spine03": (2, 0, 4 * s), "spine05": (0, 0, 2 * s)}, ik=(1, 1), aim=(6 + 3 * abs(s), 0, 3 * s))
        if k < 4:
            key(f + 8, {**hold, "spine03": (6, 0, 2 * s)}, ik=(1, 1), aim=(2, 0, 2 * s))
    ease(True)


@clip
def wake():
    # the last mission: waking in the bedroom, sitting up, swinging the legs
    # out over the side and standing (the bed is a box for the stage); every
    # place here is where the hips go
    n = 96
    begin("wake", n, False, "bed", view="three_quarter")
    bed = PROP["bed"]
    bed.parent = None
    bed.matrix_world = Matrix.Translation((0.9, 0.45, 0.0)) @ Matrix.Rotation(R(90), 4, "Z")
    top = 0.48
    hz = HIP.z
    lie = {**RELAX, **ARMS_DOWN, "upperleg01.*": (-4, 0, -2), "lowerleg01.*": (6, 0, 0), "foot.*": (30, 0, 0)}
    sit = {**RELAX, "spine05": (24, 0, 0), "spine03": (10, 0, 0), "upperleg01.*": (-90, 0, -2), "lowerleg01.*": (10, 0, 0),
           "upperarm01.*": (10, 0, -40), "lowerarm01.*": (20, 0, 0)}
    edge = {**RELAX, "spine05": (14, 0, 0), "upperleg01.*": (-90, 0, -6), "lowerleg01.*": (90, 0, 0), "foot.*": (-6, 0, 0),
            "upperarm01.*": (40, 0, -24), "lowerarm01.*": (50, 0, 0)}
    rise = {**CROUCHED, **ARMS_DOWN, "spine05": (34, 0, 0)}
    stand = {**RELAX, **ARMS_DOWN, **STANCE}
    H = lambda x, y, z: (x, y - HIP.y, z - hz)
    key(1, lie, H(0.6, 0.45, top + 0.14), rot=(-90, 0, 90), hips=True)
    key(18, lie, H(0.6, 0.45, top + 0.14), rot=(-90, 0, 90), hips=True)
    key(38, sit, H(0.62, 0.45, top + 0.1), rot=(0, 0, 90), hips=True)
    key(56, edge, H(0.62, 0.0, top + 0.1), rot=(0, 0, 10), hips=True)
    key(74, rise, H(0.62, -0.3, hz - 0.2), rot=(0, 0, 0), hips=True)
    key(n, stand, H(0.62, -0.32, hz), rot=(0, 0, 0), hips=True)
    ease(False)


# ---- bake, render, save ---------------------------------------------------------
def lift(name):
    """Between keys the curves can sag: every frame where the figure dips
    under the floor is lifted onto it. Nothing is ever pulled down, so a jump
    or a run's flight stays in the air."""
    n = META[name]["frames"]
    scene = bpy.context.scene
    fixes = []
    for f in range(1, n + 1):
        scene.frame_set(f)
        low = lowest()
        if low < -0.002:
            fixes.append((f, rig.location.z - low))
    for f, z in fixes:
        scene.frame_set(f)
        rig.location.z = z
        rig.keyframe_insert("location", index=2, frame=f)
    return len(fixes)


def bake(name):
    """Bake the clip's final pose (IK and all) to plain bone keys on a fresh
    action, so it plays with the targets gone."""
    m = META[name]
    n = m["frames"]
    scene = bpy.context.scene
    scene.frame_start, scene.frame_end = 1, n
    bpy.context.view_layer.objects.active = rig
    for o in bpy.context.selected_objects:
        o.select_set(False)
    rig.select_set(True)
    bpy.ops.object.mode_set(mode="POSE")
    for pb in rig.pose.bones:
        want = pb.name in BONES
        if hasattr(pb, "select"):
            pb.select = want
        else:
            pb.bone.select = want
    work = rig.animation_data.action
    bpy.ops.nla.bake(frame_start=1, frame_end=n, only_selected=True, visual_keying=True, clear_constraints=False,
                     use_current_action=False, bake_types={"POSE", "OBJECT"})
    bpy.ops.object.mode_set(mode="OBJECT")
    baked = rig.animation_data.action
    baked.name = f"diver__{name}"
    baked.use_fake_user = True
    # the IK is in the baked pose now: switch it off for playing the bake
    for s in "RL":
        path = f'pose.bones["lowerarm02.{s}"].constraints["ltc_ik"].influence'
        for fc in [fc for fc in getattr(baked, "fcurves", []) if fc.data_path == path]:
            baked.fcurves.remove(fc)
        IK[s].influence = 0.0
    bpy.data.actions.remove(work)
    return baked


def cameras(view, ground=True):
    top = HEAD.z + 0.2
    mid = Vector((0, 0, top * 0.5 + (0.0 if ground else 0.45)))
    d = top * (2.2 if ground else 2.7)
    loc, aim = {
        "three_quarter": (Vector((-d * 0.72, -d * 0.75, mid.z + 0.3)), mid),
        "side": (Vector((-d * 1.05, -0.2, mid.z + 0.15)), mid + Vector((0, -0.2, 0))),
        "front": (Vector((-0.4, -d * 1.05, mid.z + 0.2)), mid),
    }[view]
    for ob in [o for o in OB if o.type == "CAMERA"]:
        bpy.data.objects.remove(ob, do_unlink=True)
    stage.camera(tuple(loc), tuple(aim), lens=34, name="stage_cam")


def render(name):
    m = META[name]
    scene = bpy.context.scene
    scene.frame_start, scene.frame_end = 1, m["frames"]
    cameras(m["view"], m["ground"] or name in ("death", "recess"))
    scene.render.resolution_x, scene.render.resolution_y = (640, 360) if FAST else (960, 540)
    (OUT / "frames").mkdir(parents=True, exist_ok=True)
    scene.frame_set(max(1, m["frames"] // 2))
    scene.render.image_settings.media_type = "IMAGE"
    scene.render.image_settings.file_format = "JPEG"
    scene.render.filepath = str(OUT / "frames" / f"{name}.jpg")
    bpy.ops.render.render(write_still=True)
    scene.render.image_settings.media_type = "VIDEO"
    scene.render.image_settings.file_format = "FFMPEG"
    scene.render.ffmpeg.format = "MPEG4"
    scene.render.ffmpeg.codec = "H264"
    scene.render.ffmpeg.constant_rate_factor = "MEDIUM"
    scene.render.filepath = str(OUT / "clips" / f"{name}.mp4")
    bpy.ops.render.render(animation=True)


def main():
    (OUT / "clips").mkdir(parents=True, exist_ok=True)
    scene = stage.build(engine="BLENDER_EEVEE", ground=True, samples=16 if FAST else 48)
    stage.reflective_world(scene, over)
    scene.render.fps = FPS
    for name, fn in CLIPS.items():
        if ONLY and name not in ONLY:
            continue
        fn()
        lifted = 0
        for _ in range(4):
            k = lift(name)
            lifted += k
            if not k:
                break
        if lifted:
            say(f"  {name}: {lifted} frames lifted onto the floor")
        bake(name)
        # play the bake for the render: the targets and IK are off
        for ob in (AIM, T_L):
            if ob.animation_data and ob.animation_data.action:
                ob.animation_data.action.use_fake_user = True
        if RENDER:
            render(name)
        say(name, META[name]["frames"], "frames")
    (OUT / "ltc_clips.json").write_text(json.dumps(META, indent=1))
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT / "ltc.blend"))
    say("saved", len(META), "clips")


main()
