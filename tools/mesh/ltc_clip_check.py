"""Hold every diver clip to the clip rules, headless: no limb through the body,
nothing under the floor, no held thing inside the torso.

    blender -b OUTDIR/ltc.blend --python tools/mesh/ltc_clip_check.py -- OUTDIR [--json REPORT] [--plant self|ground|prop]

For every frame of every clip in OUTDIR/ltc_clips.json, each kept bone of the
body becomes a capsule: its rest head and tail moved by that frame's pose, its
radius the median distance of the body's own skin from that bone. Limits are
in ltc_clip_rules.json. The check fails when, in any frame:

  self    an arm or leg capsule passes into a torso capsule deeper than the two
          overlap in the rest pose, by more than self_mm (bones next to each
          other in the chain are skipped);
  ground  the lowest point of the soles, or of any capsule, is more than
          ground_mm under the floor (z = 0);
  prop    more than prop_share of the held prop's points lie inside a torso
          capsule shrunk to prop_inset of its radius.

--plant puts one defect into the first clip it fits (on the ground; holding
a prop) before checking, so
scripts/prove-checks.sh can require each part to fail: 'self' swings the
right upper arm into the chest, 'ground' sinks the body 0.25 m, 'prop' sets
the held prop inside the chest. The check's own code is the same either way.
"""
import json
import math
import sys
from pathlib import Path

import bpy
from mathutils import Matrix, Vector

HERE = Path(__file__).parent
args = sys.argv[sys.argv.index("--") + 1:]
OUT = Path(args[0])
REPORT = Path(args[args.index("--json") + 1]) if "--json" in args else None
PLANT = args[args.index("--plant") + 1] if "--plant" in args else None
RULES = json.loads((HERE / "ltc_clip_rules.json").read_text())
META = json.loads((OUT / "ltc_clips.json").read_text())
OB = bpy.data.objects
rig = OB["diver_rig"]
body = OB["diver"]


def say(*a):
    print("ltc_clip_check:", *a, flush=True)


TORSO = ["spine05", "spine04", "spine03", "spine02", "spine01"]
LIMBS = ["upperarm02.L", "lowerarm01.L", "lowerarm02.L", "wrist.L", "upperarm02.R", "lowerarm01.R", "lowerarm02.R", "wrist.R",
         "lowerleg01.L", "lowerleg02.L", "lowerleg01.R", "lowerleg02.R", "foot.L", "foot.R"]
ALL = TORSO + LIMBS + ["neck01", "head", "upperleg01.L", "upperleg02.L", "upperleg01.R", "upperleg02.R", "upperarm01.L",
                       "upperarm01.R"]
ALL = [b for b in ALL if b in rig.data.bones]

# ---- rest radii from the body's own skin ---------------------------------------------
for ob in OB:
    if ob.animation_data is None and ob == rig:
        rig.animation_data_create()
rig.animation_data.action = None
for pb in rig.pose.bones:
    pb.rotation_mode = "XYZ"
    pb.rotation_euler = (0, 0, 0)
    pb.location = (0, 0, 0)
loc0 = rig.location.copy()
bpy.context.view_layer.update()
groups = {g.index: g.name for g in body.vertex_groups}
pts_of = {b: [] for b in ALL}
MW = body.matrix_world
for v in body.data.vertices:
    for g in v.groups:
        n = groups.get(g.group)
        if n in pts_of and g.weight > 0.5:
            pts_of[n].append(MW @ v.co)


def seg_dist(p, a, b):
    ab = b - a
    t = max(0.0, min(1.0, (p - a).dot(ab) / max(ab.length_squared, 1e-12)))
    return (a + ab * t - p).length


def rest_seg(b):
    bone = rig.data.bones[b]
    return rig.matrix_world @ bone.head_local, rig.matrix_world @ bone.tail_local


RADIUS = {}
for b in ALL:
    a, c = rest_seg(b)
    d = sorted(seg_dist(p, a, c) for p in pts_of[b]) or [0.04]
    RADIUS[b] = d[len(d) // 2]


def segs_now():
    M = rig.matrix_world
    return {b: (M @ rig.pose.bones[b].head, M @ rig.pose.bones[b].tail) for b in ALL}


def seg_seg(p1, q1, p2, q2):
    """Closest distance between two segments."""
    d1, d2, r = q1 - p1, q2 - p2, p1 - p2
    a, e, f = d1.dot(d1), d2.dot(d2), d2.dot(r)
    if a <= 1e-12 and e <= 1e-12:
        return r.length
    if a <= 1e-12:
        s, t = 0.0, max(0.0, min(1.0, f / e))
    else:
        c = d1.dot(r)
        if e <= 1e-12:
            t, s = 0.0, max(0.0, min(1.0, -c / a))
        else:
            b = d1.dot(d2)
            den = a * e - b * b
            s = max(0.0, min(1.0, (b * f - c * e) / den)) if den > 1e-12 else 0.0
            t = (b * s + f) / e
            if t < 0:
                t, s = 0.0, max(0.0, min(1.0, -c / a))
            elif t > 1:
                t, s = 1.0, max(0.0, min(1.0, (b - c) / a))
    return ((p1 + d1 * s) - (p2 + d2 * t)).length


def related(a, b):
    pa, pb = rig.data.bones[a], rig.data.bones[b]
    return pa.parent == pb or pb.parent == pa or (pa.parent and pa.parent == pb.parent)


PAIRS = [(l, t) for l in LIMBS for t in TORSO if l in RADIUS and not related(l, t)]
S0 = segs_now()
BASE = {(l, t): (RADIUS[l] + RADIUS[t]) - seg_seg(*S0[l], *S0[t]) for l, t in PAIRS}
say("bones", len(ALL), "pairs", len(PAIRS))

SOLES = [o for o in OB if o.name.startswith(("diver.sole_", "diver.toecap_", "diver.boot_"))]
dg = bpy.context.evaluated_depsgraph_get()


def lowest_sole():
    low = 1e9
    for ob in SOLES:
        e = ob.evaluated_get(dg)
        m = e.to_mesh()
        if len(m.vertices):
            low = min(low, min((ob.matrix_world @ v.co).z for v in m.vertices))
        e.to_mesh_clear()
    return low


LOCAL = {}
for _r in [o for o in OB if o.name.startswith("prop_") and o.type == "EMPTY"]:
    _r.parent = None
    _r.matrix_world = Matrix.Identity(4)
bpy.context.view_layer.update()


def prop_local(name):
    """A prop's points in its own frame (the grip at the origin)."""
    if name not in LOCAL:
        root = OB.get(f"prop_{name}")
        inv = root.matrix_world.inverted() if root else None
        LOCAL[name] = [inv @ ob.matrix_world @ v.co for ob in (root.children if root else []) if ob.type == "MESH"
                       for v in ob.data.vertices]
    return LOCAL[name]


def prop_points(name, in_hand):
    """The prop where the clip holds it: in the right hand, this frame."""
    wrist = rig.matrix_world @ rig.pose.bones["wrist.R"].matrix
    M = wrist @ Matrix(in_hand)
    return [M @ p for p in prop_local(name)]


# ---- the planted defect ----------------------------------------------------------------
names = list(META)
if PLANT:
    # a clip where the defect means something: on the ground, or holding a prop
    first = next(n for n in names if (META[n].get("in_hand") if PLANT == "prop" else META[n]["ground"]))
    act = bpy.data.actions[f"diver__{first}"]
    rig.animation_data.action = act
    n = META[first]["frames"]
    if PLANT == "self":
        # the right arm folded so the wrist ends in the chest (found by search)
        pu, pl = rig.pose.bones["upperarm01.R"], rig.pose.bones["lowerarm01.R"]
        for f in range(1, n + 1):
            bpy.context.scene.frame_set(f)
            pu.rotation_euler = (math.radians(60), math.radians(-90), 0)
            pl.rotation_euler = (math.radians(100), 0, 0)
            pu.keyframe_insert("rotation_euler", frame=f)
            pl.keyframe_insert("rotation_euler", frame=f)
    elif PLANT == "ground":
        for f in range(1, n + 1):
            bpy.context.scene.frame_set(f)
            rig.location.z -= 0.25
            rig.keyframe_insert("location", frame=f)
    elif PLANT == "prop":
        # the prop held 0.3 m back from the hand: into the chest
        hand = Matrix(META[first]["in_hand"])
        META[first]["in_hand"] = [list(r) for r in (Matrix.Translation((0.0, 0.0, 0.0)) @ hand)]
        bpy.context.scene.frame_set(1)
        wrist = rig.matrix_world @ rig.pose.bones["wrist.R"].matrix
        chest = rig.matrix_world @ rig.pose.bones["spine03"].head
        M = wrist.inverted() @ Matrix.Translation(chest) @ Matrix(hand).to_3x3().to_4x4()
        META[first]["in_hand"] = [list(r) for r in M]
    say("planted", PLANT, "in", first)

# ---- every frame of every clip ---------------------------------------------------------
problems, report = [], {}
for name in names:
    m = META[name]
    act = bpy.data.actions.get(f"diver__{name}")
    if act is None:
        problems.append(f"missing: {name} has no baked action")
        continue
    rig.animation_data.action = act
    worst = {"self": 0.0, "ground": 0.0, "prop": 0.0}
    where = {}
    for f in range(1, m["frames"] + 1):
        bpy.context.scene.frame_set(f)
        S = segs_now()
        for (l, t) in PAIRS:
            depth = (RADIUS[l] + RADIUS[t]) - seg_seg(*S[l], *S[t]) - max(BASE[(l, t)], 0.0)
            if depth > worst["self"]:
                worst["self"], where["self"] = depth, f"{l} into {t} at frame {f}"
        low_caps = min(min(a.z, b.z) - RADIUS[k] for k, (a, b) in S.items())
        low = min(lowest_sole(), low_caps + RULES["capsule_floor_allowance_mm"] / 1000)
        if -low > worst["ground"]:
            worst["ground"], where["ground"] = -low, f"frame {f}"
        if m.get("in_hand"):
            pts = prop_points(m["prop"], m["in_hand"])
            inside = 0
            for p in pts:
                for t in TORSO:
                    if seg_dist(p, *S[t]) < RADIUS[t] * RULES["prop_inset"]:
                        inside += 1
                        break
            share = inside / max(len(pts), 1)
            if share > worst["prop"]:
                worst["prop"], where["prop"] = share, f"frame {f}"
    report[name] = {"worst": worst, "where": where}
    if worst["self"] * 1000 > RULES["self_mm"]:
        problems.append(f"self: {name}: {where['self']}, {worst['self'] * 1000:.0f} mm past rest")
    if worst["ground"] * 1000 > RULES["ground_mm"]:
        problems.append(f"ground: {name}: {worst['ground'] * 1000:.0f} mm under the floor at {where['ground']}")
    if worst["prop"] > RULES["prop_share"]:
        problems.append(f"prop: {name}: {worst['prop'] * 100:.1f}% of the {m['prop']} inside the torso at {where['prop']}")
    say(f"{name}: self {worst['self'] * 1000:.0f} mm, ground {worst['ground'] * 1000:.0f} mm, prop {worst['prop'] * 100:.1f}%")

for p in problems:
    say("problem:", p)
say("PASS" if not problems else "FAIL", len(names), "clips,", len(problems), "problems")
if REPORT:
    REPORT.write_text(json.dumps({"problems": problems, "clips": report}, indent=1))
