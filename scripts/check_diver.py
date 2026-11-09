#!/usr/bin/env python3
"""The diver's clips, held to the game's player volume, and to Style Lab's
clip check.

    python3 scripts/check_diver.py              every check, each proven on a planted defect
    python3 scripts/check_diver.py --plant top  one planted defect (the check must fail)

1. The volume (no Blender needed). For every frame of every clip, the bake
   (tools/mesh/bake_diver.py) wrote where the shader draws the diver, in the
   figure's frame: its top, its lowest point, and how far the core (hips,
   torso, head, helmet), the arms and the legs reach out from the upright
   axis (data/mesh/diver_bounds.json). The rules (tools/mesh/diver_volume.json)
   hold the top to PLAYER_HEIGHT and the core to PLAYER_RADIUS, read from
   crates/core/src/balance.rs, each with a named allowance, and the arms and
   legs to a named reach. A clip that leaves the volume by design is named
   there with its reason and the reach measured, and fails if it grows past
   what was measured.
2. Style Lab's clip check (tools/mesh/ltc_clip_check.py: no limb through the
   body, nothing under the floor, no prop in the torso) against the game's
   copy of the clips (tools/mesh/src/ltc.blend). It needs Blender; without it
   this part is skipped and says so.

Each part is first run with each of its planted defects, which must fail,
then on the clips as they are, which must pass.
"""
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BOUNDS = ROOT / "data" / "mesh" / "diver_bounds.json"
RULES = ROOT / "tools" / "mesh" / "diver_volume.json"
TOP, LOW, CORE, ARMS, LEGS = range(5)
PLANTS = ("top", "ground", "core", "arms", "legs", "exempt")


def balance_mm(name):
    """A length in balance.rs written as ONE * a / b, in millimetres."""
    text = (ROOT / "crates" / "core" / "src" / "balance.rs").read_text()
    m = re.search(rf"pub const {name}: i32 = ONE \* (\d+) / (\d+);", text)
    if not m:
        raise SystemExit(f"check_diver: {name} not found in balance.rs as ONE * a / b")
    return int(m.group(1)) * 1000 // int(m.group(2))


def problems_of(clips, rules):
    height = balance_mm("PLAYER_HEIGHT")
    radius = balance_mm("PLAYER_RADIUS")
    a = rules["allow_mm"]
    out = []
    for name, frames in clips.items():
        worst = [max(f[k] for f in frames) for k in range(5)]
        worst[LOW] = min(f[LOW] for f in frames)
        ex = rules["exempt"].get(name)
        if ex:
            # out of the volume by design: held to what was measured
            for k, key in ((TOP, "top_mm"), (CORE, "core_mm"), (ARMS, "arms_mm"), (LEGS, "legs_mm")):
                if key in ex and worst[k] > ex[key] + a["measured"]:
                    out.append(f"exempt {name}: {key[:-3]} {worst[k]} mm, past the {ex[key]} mm measured")
            continue
        if worst[TOP] > height + a["top"]:
            out.append(f"top: {name} reaches {worst[TOP]} mm, over PLAYER_HEIGHT {height} + {a['top']}")
        if worst[LOW] < -a["ground"]:
            out.append(f"ground: {name} goes {-worst[LOW]} mm under the feet")
        if worst[CORE] > radius + a["core"]:
            out.append(f"core: {name} reaches {worst[CORE]} mm out, past PLAYER_RADIUS {radius} + {a['core']}")
        if worst[ARMS] > rules["arm_reach_mm"]:
            out.append(f"arms: {name} reach {worst[ARMS]} mm, past {rules['arm_reach_mm']}")
        if worst[LEGS] > rules["leg_reach_mm"]:
            out.append(f"legs: {name} reach {worst[LEGS]} mm, past {rules['leg_reach_mm']}")
    return out


def planted(clips, rules, kind):
    """The bounds with one defect put in: the first clip not exempt (or the
    first exempt one) made to break its rule in one frame."""
    clips = {k: [list(f) for f in v] for k, v in clips.items()}
    if kind == "exempt":
        name = next(iter(rules["exempt"]))
        key = next(k for k in ("core_mm", "top_mm", "arms_mm", "legs_mm") if k in rules["exempt"][name])
        clips[name][0][{"top_mm": TOP, "core_mm": CORE, "arms_mm": ARMS, "legs_mm": LEGS}[key]] += 500
        return clips
    name = next(n for n in clips if n not in rules["exempt"])
    f = clips[name][0]
    if kind == "top":
        f[TOP] += 500
    elif kind == "ground":
        f[LOW] = -300
    elif kind == "core":
        f[CORE] += 500
    elif kind == "arms":
        f[ARMS] = rules["arm_reach_mm"] + 100
    elif kind == "legs":
        f[LEGS] = rules["leg_reach_mm"] + 100
    return clips


def blender():
    b = os.environ.get("BLENDER") or shutil.which("blender")
    mac = "/Applications/Blender.app/Contents/MacOS/Blender"
    return b or (mac if Path(mac).exists() else None)


def clip_check(b, plant=None):
    cmd = [b, "-b", str(ROOT / "tools/mesh/src/ltc.blend"), "--python", str(ROOT / "tools/mesh/ltc_clip_check.py"),
           "--", str(ROOT / "tools/mesh/src")] + (["--plant", plant] if plant else [])
    out = subprocess.run(cmd, capture_output=True, text=True).stdout
    return [l for l in out.splitlines() if l.startswith("ltc_clip_check:")]


def main():
    clips = json.loads(BOUNDS.read_text())["clips"]
    rules = json.loads(RULES.read_text())
    if "--plant" in sys.argv:
        kind = sys.argv[sys.argv.index("--plant") + 1]
        found = problems_of(planted(clips, rules, kind), rules)
        print("\n".join(found) or "no problem found")
        sys.exit(1 if found else 0)
    fail = False
    for kind in PLANTS:
        found = problems_of(planted(clips, rules, kind), rules)
        ok = any(p.startswith(kind) for p in found)
        print(f"check_diver: volume plant {kind} {'caught' if ok else 'NOT caught'}")
        fail |= not ok
    found = problems_of(clips, rules)
    for p in found:
        print("check_diver: problem:", p)
    print(f"check_diver: volume {'PASS' if not found else 'FAIL'} ({len(clips)} clips)")
    fail |= bool(found)
    b = blender()
    if b is None:
        print("check_diver: no Blender here: Style Lab's clip check skipped (it runs where Blender is)")
    else:
        for d in ("self", "ground", "prop"):
            out = clip_check(b, d)
            ok = any(f"problem: {d}:" in l for l in out) and any("FAIL" in l for l in out)
            print(f"check_diver: clip plant {d} {'caught' if ok else 'NOT caught'}")
            fail |= not ok
        out = clip_check(b)
        passed = any(l.endswith(" problems") and "PASS" in l for l in out)
        for l in out:
            if "problem:" in l:
                print(l)
        print(f"check_diver: clip check {'PASS' if passed else 'FAIL'}")
        fail |= not passed
    sys.exit(1 if fail else 0)


main()
