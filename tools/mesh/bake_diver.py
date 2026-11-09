"""Bake the diver (Long Time Comin's player) for the browser, headless.

    blender -b tools/mesh/src/ltc.blend --python tools/mesh/bake_diver.py -- [--fast]

Copied from Style Lab's tools/mesh/bake_game.py (style-lab branch ltc-diver,
commit d30f1ab) and changed for one model, the diver, read from Style Lab's
ltc.blend (copied to tools/mesh/src/ with its ltc_clips.json):

- what cannot be seen is cut: the body under the suit (its MASK modifiers),
  the hair, teeth, tongue, brows and lashes inside the helmet;
- the parts are joined by material, so each material is one draw with one
  baked colour texture, and decimated toward TRI_BUDGET triangles, the suit
  (whose creases are modelled) less than the rest;
- the face bones, toes, metacarpals and fingertips are dropped;
- every clip is sampled to bone matrices for the animation texture, as in
  Espiochum; each held prop's grip is written per frame (position, muzzle
  and up, in the figure's frame) for the core, and each frame's bounds for
  the volume check.

Game frame: game (x, y, z) = Blender (-x, z, y); heading zero faces -z.

Writes web/mesh/diver.bin, web/mesh/diver_anim.bin, web/mesh/tex/diver_*.jpg,
data/mesh/diver.json (the manifest the page reads), data/clips.json (the
clip table the core reads) and data/mesh/diver_bounds.json (per frame, for
scripts/check_diver.py).
"""
import json
import math
import struct
import sys
from pathlib import Path

import bpy
import numpy as np
from mathutils import Matrix, Vector

HERE = Path(__file__).parent
GAME = HERE.parent.parent
SRC = HERE / "src"
WEB = GAME / "web" / "mesh"
TEX = WEB / "tex"
DATA = GAME / "data" / "mesh"
for d in (WEB, TEX, DATA):
    d.mkdir(parents=True, exist_ok=True)
args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
FAST = "--fast" in args
META = json.loads((SRC / "ltc_clips.json").read_text())
OB = bpy.data.objects
rig = OB["diver_rig"]
A = Matrix(((-1, 0, 0, 0), (0, 0, 1, 0), (0, 1, 0, 0), (0, 0, 0, 1)))   # Blender to game
AI = A.inverted()
# inside the helmet, never seen
SKIP = ("short02", "teeth", "tongue", "eyebrow", "eyelashes", "high-poly")
DROP = ("oris", "levator", "risorius", "temporalis", "orbicularis", "oculi", "tongue", "special", "breast", "toe",
        "metacarpal", "jaw", "eye")
TRI_BUDGET = 26000
# how much more of its detail a material keeps than the rest
DETAIL = {"diver_suit": 2.0, "diver_copper": 1.4, "diver_brass": 1.1, "diver_skin": 0.5, "diver_boot": 0.45}
# gaits, and the axis of the figure's frame they travel along (for the stride)
GAITS = {"run": 2, "backpedal": 2, "strafe_left": 0, "strafe_right": 0}
TEX_PX = 512 if FAST else 1024
SMALL_PX = 256 if FAST else 512
# the upper body's layer: spine01 and every bone under it; its seam is
# spine01's parent, where the upper layer is set onto the lower
UPPER_FROM = "spine01"
ARMS = ("clavicle", "shoulder", "upperarm", "lowerarm", "wrist", "finger")
LEGS = ("upperleg", "lowerleg", "foot")
GLASS = "diver_glass"


def say(*a):
    print("bake_diver:", *a, flush=True)


def lin_to_srgb(c):
    return [(12.92 * x if x <= 0.0031308 else 1.055 * x ** (1 / 2.4) - 0.055) for x in c]


def rest():
    if rig.animation_data:
        rig.animation_data.action = None
    rig.location, rig.rotation_euler = (0, 0, 0), (0, 0, 0)
    for pb in rig.pose.bones:
        pb.rotation_mode = "XYZ"
        pb.rotation_euler = (0, 0, 0)
        pb.location = (0, 0, 0)
        pb.scale = (1, 1, 1)
        # the clips are baked to plain keys (IK and all): play them bare
        for c in pb.constraints:
            c.mute = True
    bpy.context.view_layer.update()


def kept_bones():
    out = []
    for b in rig.data.bones:
        if b.name.startswith(DROP) or (b.name.startswith("finger") and "-3." in b.name):
            continue
        out.append(b.name)
    return out


def owner_bone(name, kept):
    b = rig.data.bones.get(name)
    while b is not None and b.name not in kept:
        b = b.parent
    return b.name if b else kept[0]


def under(name, top):
    b = rig.data.bones[name]
    while b is not None:
        if b.name == top:
            return True
        b = b.parent
    return False


def parts():
    out = []
    for ob in OB:
        if ob.type != "MESH" or ob.hide_render:
            continue
        if not (ob.name == "diver" or ob.name.startswith("diver.")):
            continue
        if any(k in ob.name for k in SKIP):
            continue
        out.append(ob)
    return out


def evaluated_copy(ob):
    """A copy with every modifier but the armature applied (the body's masks
    cut what the suit covers), in world space, unparented."""
    cp = ob.copy()
    cp.data = ob.data.copy()
    bpy.context.scene.collection.objects.link(cp)
    cp.parent = None
    cp.matrix_world = ob.matrix_world.copy()
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = cp
    cp.select_set(True)
    if cp.data.shape_keys:
        bpy.ops.object.shape_key_remove(all=True, apply_mix=True)
    for m in list(cp.modifiers):
        if m.type in ("ARMATURE", "PARTICLE_SYSTEM"):
            cp.modifiers.remove(m)
            continue
        try:
            bpy.ops.object.modifier_apply(modifier=m.name)
        except RuntimeError:
            cp.modifiers.remove(m)
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    return cp


def tris_of(ob):
    return sum(len(p.vertices) - 2 for p in ob.data.polygons)


def join(obs, name):
    bpy.ops.object.select_all(action="DESELECT")
    for o in obs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = obs[0]
    if len(obs) > 1:
        bpy.ops.object.join()
    j = bpy.context.view_layer.objects.active
    j.name = name
    return j


def decimate(ob, ratio):
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    if ratio < 0.999:
        d = ob.modifiers.new("dec", "DECIMATE")
        d.ratio = ratio
        bpy.ops.object.modifier_apply(modifier="dec")
    # the decimated surface's own normals: smooth, sharp past 40 degrees
    if ob.data.has_custom_normals:
        bpy.ops.mesh.customdata_custom_splitnormals_clear()
    bpy.ops.object.shade_smooth_by_angle(angle=math.radians(40))


def ratios(groups):
    """Each group's decimation ratio: one scale for all, times its detail,
    never above 1, so the total comes to TRI_BUDGET."""
    tris = {k: tris_of(o) for k, o in groups.items()}
    lo, hi = 0.0, 1.0
    for _ in range(40):
        k = (lo + hi) / 2
        total = sum(min(1.0, k * DETAIL.get(g, 1.0)) * t for g, t in tris.items())
        lo, hi = (k, hi) if total < TRI_BUDGET else (lo, k)
    return {g: min(1.0, lo * DETAIL.get(g, 1.0)) for g in tris}, tris


def bake_colour(ob, mat, name, px):
    """Bake a group's base colour (its procedural material) to a texture,
    unwrapping it first if it has no UVs."""
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 4 if FAST else 16
    scene.cycles.device = "CPU"
    bpy.ops.object.select_all(action="DESELECT")
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    if not ob.data.uv_layers:
        bpy.ops.object.mode_set(mode="EDIT")
        bpy.ops.mesh.select_all(action="SELECT")
        bpy.ops.uv.smart_project(angle_limit=math.radians(60), island_margin=0.004)
        bpy.ops.object.mode_set(mode="OBJECT")
    img = bpy.data.images.new(f"bake_{name}", px, px)
    nt = mat.node_tree
    # a metal's diffuse colour bakes black: bake it as if it were not metal
    bsdf = next((n for n in nt.nodes if n.type == "BSDF_PRINCIPLED"), None)
    metal = None
    if bsdf is not None and not bsdf.inputs["Metallic"].is_linked:
        metal = bsdf.inputs["Metallic"].default_value
        bsdf.inputs["Metallic"].default_value = 0.0
    tn = nt.nodes.new("ShaderNodeTexImage")
    tn.image = img
    nt.nodes.active = tn
    scene.render.bake.use_pass_direct = scene.render.bake.use_pass_indirect = False
    scene.render.bake.use_pass_color = True
    # the islands' colour carried out into the gaps, so a mipmap of the
    # small figure far off does not average the black between them in
    scene.render.bake.margin_type = "EXTEND"
    scene.render.bake.margin = 64
    bpy.ops.object.bake(type="DIFFUSE", use_clear=True)
    nt.nodes.remove(tn)
    # the bake leaves linear values in the image's bytes; the page reads the
    # texture as sRGB, so encode them
    buf = np.empty(px * px * 4, dtype=np.float32)
    img.pixels.foreach_get(buf)
    rgba = buf.reshape(-1, 4)
    c = np.clip(rgba[:, :3], 0.0, 1.0)
    rgba[:, :3] = np.where(c <= 0.0031308, 12.92 * c, 1.055 * np.power(c, 1 / 2.4) - 0.055)
    img.pixels.foreach_set(buf)
    if metal is not None:
        bsdf.inputs["Metallic"].default_value = metal
    img.file_format = "JPEG"
    img.filepath_raw = str(TEX / f"{name}.jpg")
    bpy.context.scene.render.image_settings.quality = 85
    img.save()
    return f"{name}.jpg"


def surface(mat):
    """(tint, metal, rough) from a material's principled BSDF."""
    b = next((n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED"), None)
    if b is None:
        return [0.5, 0.5, 0.5], 0.0, 0.6
    base = b.inputs["Base Color"]
    tint = [1.0, 1.0, 1.0] if base.is_linked else list(base.default_value)[:3]
    return tint, float(b.inputs["Metallic"].default_value), float(b.inputs["Roughness"].default_value)


def stride_of(feet, axis):
    """How far the ground goes by in one cycle of a gait, from the planted
    foot: each frame, the lower foot is on the ground and moves back as fast
    as the body goes on; its median speed times the cycle's frames."""
    n = len(feet)
    speeds = []
    for f in range(n):
        a, b = feet[f], feet[(f + 1) % n]
        low = 0 if a[0][1] <= a[1][1] else 1
        speeds.append(abs(b[low][axis] - a[low][axis]))
    speeds.sort()
    return int(round(speeds[n // 2] * n * 1000))


def half(x):
    return struct.pack("<e", max(-65000.0, min(65000.0, x)))


def main():
    rest()
    kept = kept_bones()
    index = {n: i for i, n in enumerate(kept)}
    upper = [1 if under(n, UPPER_FROM) else 0 for n in kept]
    seam = index[owner_bone(rig.data.bones[UPPER_FROM].parent.name, kept)]
    arm_bone = np.array([1 if n.startswith(ARMS) else 0 for n in kept], dtype=bool)
    leg_bone = np.array([1 if n.startswith(LEGS) else 0 for n in kept], dtype=bool)

    # ---- the parts, joined by material and decimated ----------------------------------
    by_mat = {}
    for ob in parts():
        mat = ob.active_material
        cp = evaluated_copy(ob)
        if not cp.data.polygons:
            bpy.data.objects.remove(cp, do_unlink=True)
            continue
        by_mat.setdefault(mat.name, []).append(cp)
    groups = {m: join(obs, f"bake_{m}") for m, obs in by_mat.items()}
    rat, before = ratios(groups)
    for m, ob in groups.items():
        decimate(ob, rat[m])
        say(m, before[m], "->", tris_of(ob), "triangles")

    # ---- vertices, textures ------------------------------------------------------------
    blob = bytearray()
    pmeta = []
    allV, allJ, allW = [], [], []
    for m, ob in sorted(groups.items()):
        mat = bpy.data.materials[m]
        tint, metal, rough = surface(mat)
        glass = m == GLASS
        tex = None
        if not glass and mat.node_tree and any(
                n.type == "BSDF_PRINCIPLED" and n.inputs["Base Color"].is_linked for n in mat.node_tree.nodes):
            px = TEX_PX if tris_of(ob) > 2000 else SMALL_PX
            tex = bake_colour(ob, mat, f"diver_{m.replace('diver_', '')}", px)
        me = ob.data
        uv = me.uv_layers.active.data if me.uv_layers else None
        groups_ix = {g.index: g.name for g in ob.vertex_groups}
        me.calc_loop_triangles()
        normals = me.corner_normals
        verts, vmap, idx = [], {}, []
        for tri in me.loop_triangles:
            for li, vi in zip(tri.loops, tri.vertices):
                n_b = normals[li].vector
                u = tuple(uv[li].uv) if uv else (0.0, 0.0)
                key = (vi, round(u[0], 4), round(u[1], 4), round(n_b.x, 2), round(n_b.y, 2), round(n_b.z, 2))
                if key not in vmap:
                    v = me.vertices[vi]
                    p = A @ v.co
                    n = (A.to_3x3() @ n_b).normalized()
                    ws = {}
                    for g in v.groups:
                        gname = groups_ix.get(g.group)
                        if gname is None or g.weight <= 0.0 or gname not in rig.data.bones:
                            continue
                        o = owner_bone(gname, kept)
                        ws[o] = ws.get(o, 0.0) + g.weight
                    top = sorted(ws.items(), key=lambda kv: -kv[1])[:4] or [(kept[0], 1.0)]
                    tot = sum(w for _, w in top)
                    j = [index[k] for k, _ in top] + [0] * (4 - len(top))
                    wq = [int(round(255 * w / tot)) for _, w in top] + [0] * (4 - len(top))
                    wq[0] += 255 - sum(wq)
                    vmap[key] = len(verts)
                    verts.append((p, n, u, j, wq))
                idx.append(vmap[key])
        off = len(blob)
        for p, n, u, j, w in verts:
            blob += struct.pack("<3f", *p) + struct.pack("<3b", *(int(max(-127, min(127, c * 127))) for c in n)) + b"\0"
            blob += struct.pack("<2f", u[0], u[1]) + bytes(j) + bytes(w)
            allV.append(tuple(p))
            allJ.append(j)
            allW.append([x / 255 for x in w])
        stride = 12 + 4 + 8 + 4 + 4
        ioff = len(blob)
        big = len(verts) > 65535
        for i in idx:
            blob += struct.pack("<I" if big else "<H", i)
        while len(blob) % 4:
            blob += b"\0"
        pmeta.append({"name": m, "vertices": len(verts), "offset": off, "stride": stride, "indices": len(idx),
                      "index_offset": ioff, "index_bytes": 4 if big else 2, "texture": tex, "tint": lin_to_srgb(tint),
                      "metal": metal, "rough": rough, "glass": 1 if glass else 0})
        say(m, len(verts), "vertices", len(idx) // 3, "triangles", tex or "flat")
    (WEB / "diver.bin").write_bytes(bytes(blob))

    V = np.array(allV, dtype=np.float64)
    V1 = np.concatenate([V, np.ones((len(V), 1))], axis=1)
    J = np.array(allJ, dtype=np.int64)
    W = np.array(allW, dtype=np.float64)
    owner = J[np.arange(len(J)), np.argmax(W, axis=1)]
    arm_vert = arm_bone[owner]
    leg_vert = leg_bone[owner]
    core_vert = ~(arm_vert | leg_vert)

    # ---- bones ------------------------------------------------------------------------
    bones = []
    for n in kept:
        b = rig.data.bones[n]
        par = b.parent
        while par is not None and par.name not in index:
            par = par.parent
        bones.append({"name": n, "head": list(A @ b.head_local), "parent": index[par.name] if par else -1})
    pivot = A @ rig.data.bones[UPPER_FROM].head_local

    # ---- clips ------------------------------------------------------------------------
    rest_inv = {n: rig.data.bones[n].matrix_local.inverted() for n in kept}
    anim = bytearray()
    clips, bounds = [], {}
    row = 0
    rig.animation_data_create()
    for name, meta in META.items():
        rest()
        rig.animation_data.action = bpy.data.actions[f"diver__{name}"]
        n = meta["frames"]
        grips, bnd, feet = [], [], []
        # where the right hand holds a gun: the clip's own grip when it holds
        # a gun or the trumpet, else the hold's (the game's gun stays in hand
        # through a slash, a swim or a fall)
        own = meta.get("in_hand") and meta.get("prop") not in ("knife", "bed")
        hand = Matrix(meta["in_hand"] if own else META["holding_idle"]["in_hand"])
        for f in range(1, n + 1):
            bpy.context.scene.frame_set(f)
            mw = rig.matrix_world
            Ms = np.zeros((len(kept), 3, 4))
            for bi, bn in enumerate(kept):
                pb = rig.pose.bones[bn]
                m = A @ (mw @ pb.matrix @ rest_inv[bn]) @ AI
                for r_ in range(3):
                    for c_ in range(4):
                        anim += half(m[r_][c_])
                        Ms[bi, r_, c_] = m[r_][c_]
            # every vertex where the shader puts it this frame
            P = np.zeros((len(V), 3))
            for k in range(4):
                P += W[:, k:k + 1] * np.einsum("nij,nj->ni", Ms[J[:, k]], V1)
            reach = np.hypot(P[:, 0], P[:, 2])
            bnd.append([int(round(P[:, 1].max() * 1000)), int(round(P[:, 1].min() * 1000)),
                        int(round(reach[core_vert].max() * 1000)), int(round(reach[arm_vert].max() * 1000)),
                        int(round(reach[leg_vert].max() * 1000))])
            if name in GAITS:
                feet.append([list(A @ (mw @ rig.pose.bones[f"foot.{s}"].head)) for s in "LR"])
            # the prop's own frame (ltc_props.json): x right, the muzzle
            # toward -y, z up; carried to the game's frame
            g = mw @ rig.pose.bones["wrist.R"].matrix @ hand
            at = A @ g.translation
            muzzle = (A.to_3x3() @ (g.to_3x3() @ Vector((0, -1, 0)))).normalized()
            up = (A.to_3x3() @ (g.to_3x3() @ Vector((0, 0, 1)))).normalized()
            grips.append([int(round(at.x * 1000)), int(round(at.y * 1000)), int(round(at.z * 1000))]
                         + [int(round(c * 10000)) for c in muzzle] + [int(round(c * 10000)) for c in up])
        entry = {"name": name, "frames": n, "loop": meta["loop"], "row": row, "prop": meta.get("prop"),
                 "grip": grips}
        if feet:
            entry["stride_mm"] = stride_of(feet, GAITS[name])
        clips.append(entry)
        bounds[name] = bnd
        row += n
        say(name, n, "frames")
    rig.animation_data.action = None
    (WEB / "diver_anim.bin").write_bytes(bytes(anim))

    texels = 3
    manifest = {
        "_about": "The diver, baked by tools/mesh/bake_diver.py from Style Lab's ltc.blend. Vertex layout: position 3 x f32, normal 3 x i8 and a pad, uv 2 x f32, joints 4 x u8, weights 4 x u8 (sum 255). The animation file holds, for each frame of each clip in order (data/clips.json), every bone's skin matrix in the figure's frame as three rows of four half floats. Bones flagged upper take the record's upper rows, set onto the lower rows at the seam bone. Each part is one material: its baked colour texture (or none), tint, metalness, roughness, and whether it is the helmet's glass.",
        "model": "diver", "bin": "mesh/diver.bin", "anim": "mesh/diver_anim.bin", "fps": 30,
        "bones": bones, "upper": upper, "seam": seam, "parts": pmeta, "anim_rows": row,
        "anim_width": len(kept) * texels, "texels_per_bone": texels,
        "attrib": {"position": [3, 0], "normal": [3, 12], "uv": [2, 16], "joints": [4, 24], "weights": [4, 28]},
    }
    (DATA / "diver.json").write_text(json.dumps(manifest))
    (DATA / "diver_bounds.json").write_text(json.dumps({
        "_about": "Each clip's frames, as the shader draws them in the figure's frame, in millimetres: [top, lowest, and the horizontal reach from the upright axis of the core (hips, torso, head and helmet), the arms and the legs]. Written by tools/mesh/bake_diver.py; read by scripts/check_diver.py.",
        "clips": bounds}))
    (GAME / "data" / "clips.json").write_text(json.dumps({
        "_about": "The diver's clips, in the order of the animation file's rows. Frames at 30 a second; a loop's last frame joins its first. grip: per frame, where the right hand holds a gun (the clip's own prop when it is a gun or the trumpet, else the hold's grip on the wrist) in the figure's frame (x right, y up, -z ahead): position in millimetres, then the muzzle's direction and the prop's up, each in ten-thousandths. pivot: where the upper body bends for the aim (spine01's head at rest), in millimetres. Written by tools/mesh/bake_diver.py; the core reads it to choose a clip, a frame and where a gun is held.",
        "fps": 30, "pivot": [int(round(c * 1000)) for c in pivot], "clips": clips}, separators=(",", ":")))
    say(len(kept), "bones,", len(clips), "clips,", row, "frames,", len(blob), "bytes mesh,", len(anim), "bytes anim")


main()
