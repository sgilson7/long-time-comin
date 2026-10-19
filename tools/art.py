#!/usr/bin/env python3
"""The game's 2D art, drawn as TikZ and compiled, not generated as pixels
(the diagrams-as-code prompt in advanced-prompt-engineering-prompts.md §8,
as Food Truck Circus's postcards and Gear Master 2D's portraits are made).

Each picture is composed from sub-pictures defined once below (a building
with its windows and rooftop clutter, a cloud, an orb with its halo, the
Defendant, the Bailiff, a juror, a gun) and placed by the picture's own
function. The look follows Sam_Comm_final.png in the vagrancy repository:
a warm yellow sky with outlined clouds, salmon, coral and pink buildings,
sage and teal greens, flat fills, everything outlined in one plum ink, and
rooftop clutter (air-conditioning boxes, antennas, tanks, billboards).

    python3 tools/art.py            # every picture: web/art/*.png, tools/art/*.tex
    python3 tools/art.py title      # one

Facts each picture must get right (the prompt's list of what an image
generator gets wrong), and where this file enforces each:

- text: no words in any picture; the page sets every word over the art
  from data/copy.json and data/story.json (no \\node with text below).
- the clock: every clock in the game is stopped at 21:14, the minute of
  the Stilling (story.json, count IV): `clock` draws the hour hand at
  277 degrees and the minute hand at 84 degrees, clockwise from twelve.
  The one exception is the ending's tower, far off through Chumbles'
  window: it reads 21:15, one minute on (`clock(..., minute=15)`: the hour
  hand at 277.5 degrees, the minute hand at 90).
- the orbs: violet, magenta and cyan hurt; gold ones are the ones Objection
  parries, and only gold orbs carry the gold ring (`orb(..., gold=True)`),
  as in the game.
- the Defendant: dark coat, orange scarf and trim, as data/palette.json's
  player_coat and player_trim (`defendant`).
- the Bailiff: a robe, a clock for a face, a crown, two hands with gavels
  and a ring of hours, as crates/core/src/draw.rs draws it (`bailiff`).
- the ice (zone two): cobalt orbs are the blue ones the Injunction breaks,
  and carry a pale rim (`orb(..., rim=True)`); ember orbs do not.
- the Witness: a hooded robe grown into ice, one great gold eye, a crown
  of shards, arms of ice, as crates/core/src/draw/beasts.rs draws it
  (`witness`).
- the house (zone three): its agents are all one man in a charcoal suit,
  white shirt, black tie and dark glasses (`agent`), as
  crates/core/src/draw/house.rs draws them; the Executor is a gunmetal
  tank with ember weak points on its turret's back (`executor`).
- the lullaby (the ending): the god asleep in a bubble of stopped time,
  drawn grey inside it as the game greys time stopped; its eyes are shut;
  the Defendant plays a brass trumpet at the edge of the god's heart
  (`lullaby`).
- Chumbles (the last mission): a long-haired lynx-point cat, cream with a
  white ruff and chest, grey-blue points (mask, ears, legs, a plumed
  tail), darker ear tips and a darker mask round big blue eyes, tabby
  lines on the forehead, a pink nose, white whiskers (`chumbles_face`,
  `chumbles_loaf`). In `waking` he looks down from very close through the
  summit breaking like a plate; in `chumbles` he sleeps in a loaf on the
  sofa in the sun, eyes shut, beside the empty treat bag.
- colour is never the only cue: every shape has the plum outline.
- repeatable: every random choice comes from a seeded generator, so the
  same script draws the same pictures.
"""
import math, random, shutil, subprocess, sys, tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "web" / "art"
SRC = ROOT / "tools" / "art"

# Sam_Comm_final's palette, by eye from the picture, and the game's own.
COLOURS = {
    "ink": "3B2340", "sky": "EFC46A", "skyhi": "F5DA92", "skylow": "E9B060", "cloud": "F2D58A",
    "wallA": "E88A73", "wallB": "EE9F7E", "wallC": "E47A7E", "wallD": "F0B07E", "wallE": "D9765F", "wallF": "E99A8A",
    "greenA": "8DBB7E", "greenB": "6FA77A", "greenC": "A9C97F", "greenD": "5E9C6A",
    "window": "F6E7A0", "windark": "8C5A64", "metal": "8D9A93", "metaldark": "5E6B66", "wood": "B9835A",
    "paving": "E2BD8C", "ground": "D3A77D", "stone": "E7C9A0", "clock": "F8EBC8",
    "court": "9E4F55", "courtdark": "6E3542", "goldtrim": "F2B53A", "paper": "F6E7C4", "paperdark": "E3CC98",
    "violet": "7B2FF7", "magenta": "FF2E88", "cyan": "13CFE3", "gold": "FFB000",
    "coat": "2F4858", "coatdark": "22323F", "trim": "F26B38", "skin": "E0AC69", "hair": "3B2340", "gun": "3E3A46",
    "robe": "2D2240", "bossface": "F8EBC8", "eye": "FF2E88", "red": "B3262E", "heart": "FF4F6D",
    "jump": "8DE36B", "lamp": "FFE6A3", "night": "4A3357", "nightsky": "6B4A6E",
    # zone two, the White Waste: data/palette.json's ice set
    "icesky": "A7C4E2", "icefog": "8FAFD2", "icesnow": "EEF5F9", "icesnowdark": "CFDDE7", "icewhite": "E4F1F8",
    "iceblue": "A9D3EA", "icedeep": "6FA9D2", "iceglow": "8FE3FF", "icerock": "5E6E82", "icerockdark": "3F4C5F",
    "icered": "D94A3A", "hull": "A8392F", "hulldark": "6C2A27", "icemetal": "8A97A6", "canvas": "E9C47C",
    "auroragreen": "5CF2B0", "aurorapink": "F27AD0", "cobalt": "1F4FFF", "ember": "FF6A1A",
    "witnessrobe": "1E2742", "witnessice": "CDEBFA", "navy": "1C2A44", "water": "2C6690", "dawn": "F7C89A",
    # zone three, the Glass House: data/palette.json's house set
    "housesky": "1C2440", "housenight": "2A3358", "paperwall": "7A2E3A", "paperwall2": "2F5A4A", "panel": "5A3424",
    "hwood": "8A4F2E", "hwooddark": "4E2A1C", "marble": "E9DFC9", "marbledark": "3D3436", "brass": "E0A93E",
    "candle": "FFD27A", "moon": "9FC6FF", "carpet": "9C2F3C", "cream": "F1E6CC", "labwhite": "DCE7E6", "labfluid": "7CFFB2",
    "grassnight": "2E4A3A", "hedge": "1F3A2C", "glass": "BFEFFF", "suit": "33363D", "suitdark": "1E2026",
    "shirt": "EDEDE6", "agentskin": "D8C3A8", "shades": "0E0F12", "tankhull": "3B4249", "tankplate": "5A646F",
    "tanktread": "22262B", "tankweak": "FF7A1A", "tankeye": "FFF3B0", "tankiris": "FF4A3A", "halo": "FFC94A",
    "scarlet": "FF2E3A", "chalk": "FFF6DA", "lime": "B6FF3A", "sawsteel": "E8F0F5",
    # zone four, the Ascent, and what comes after it
    "dusk": "E8A27A", "duskhi": "F6C9A0", "dusklow": "B07A8C", "astone": "B9A79A", "astonedark": "7D6C68",
    "arock": "8A7A80", "amoss": "7FA06A", "asnow": "F4EDF0", "doorlight": "FFE9A8",
    "flesh": "3E2A5C", "fleshdark": "1A1028", "eldglow": "B6FF6A", "void": "05030A", "violetdeep": "9B5DE5",
    # zone five, inside the Idiot God: data/palette.json's god set
    "godsky": "3A1458", "godfog": "2A0C40", "godflesh": "7A3A6E", "godfleshpale": "C27BA8", "godstone": "6E6A9A",
    "godstonedark": "433F70", "dancer": "B98AE0", "stilled": "D8D2E8", "seam": "FFE27A", "trumpet": "E8B84A",
    "trumpetdark": "9A6A1A",
    # the last mission: home on a Saturday morning (data/palette.json's
    # home_* set) and Chumbles, a long-haired lynx-point cat (cat_*)
    "catcream": "EADCC4", "catwhite": "FAF7F1", "catpoint": "8C8490", "catpointdark": "514956", "catpointlight": "B9AFB2",
    "cattabby": "6E6676", "cateye": "5FA0DA", "catpupil": "141018", "catnose": "DC8F8F", "catear": "E3AFAA",
    "treat": "B8743A", "treatbag": "C8553D", "treatbagdark": "8E3A2C", "sofa": "5E7A8A", "sofadark": "3E5664",
    "sunlight": "FFF6E2", "homewall": "EDE2CE", "homewindow": "FFF2C8", "hometrim": "FBF7EE", "ceiling": "F7E2B0",
    "homefloor": "B98A5E", "homefloordark": "8A6240", "morningsky": "CFE3EE", "summitsky": "2A1830",
    # the Water Gap: data/palette.json's river set
    "riversky": "F2B97C", "riverfog": "E9A98A", "sandstone": "D9825B", "sandstonedark": "B5603F", "sandstonepale": "EBA97A",
    "canyonshadow": "8E4A3E", "riverwater": "3E8FA0", "riverfoam": "E8F4F0", "riverdeep": "2A5F73", "riverwood": "9A6A44",
    "riverwooddark": "6E4A33", "river": "2F8F5B", "bench": "F3D9A4", "oakbark": "2E2230", "oakeye": "FFC23A",
    "oakleaf": "5E3A52", "fire": "FF7A2A", "firehi": "FFD36A", "smoke": "6E5A66", "smokepale": "A8939C",
    # the Far Shore and the Vault
    "ocean": "1F6F8B", "oceandeep": "134B60", "seasky": "8FA9C2", "stormsky": "4B4E6D", "stormcloud": "2E3048",
    "sail": "E9D9B4", "piratered": "B3262E", "hulldk": "4E3020", "biolume": "7CFFB2", "dagonteal": "1E5A5A",
    "vaultwhite": "E8EEF2", "chrome": "9AA6B2", "blackglass": "121826", "vaultcyan": "13CFE3", "vaultviolet": "7B2FF7",
    "grey1": "D8D8D8", "grey2": "B0B0B0", "grey3": "8A8A8A", "grey4": "5E5E5E",
}
WALLS = ["wallA", "wallB", "wallC", "wallD", "wallE", "wallF"]
GREENS = ["greenA", "greenB", "greenC", "greenD"]
SIGNS = ["magenta", "cyan", "gold", "trim", "violet"]

PREAMBLE = r"""\documentclass[tikz,border=0pt]{standalone}
\usepackage{xcolor}
\usetikzlibrary{calc}
% Palette: Sam_Comm_final.png by eye, and data/palette.json.
""" + "\n".join(f"\\definecolor{{{k}}}{{HTML}}{{{v}}}" for k, v in COLOURS.items()) + r"""
% One ink for every outline (the TikZ house style and the game's ink pass).
\newcommand{\Ink}{1.1pt}
\tikzset{inked/.style={draw=ink, line width=\Ink, line join=round, line cap=round}}
"""


def f(x):
    return f"{x:.3f}".rstrip("0").rstrip(".")


def P(x, y):
    return f"({f(x)},{f(y)})"


def rect(x0, y0, x1, y1, fill, lw=None):
    w = f"line width={lw}pt," if lw else ""
    return f"\\filldraw[inked,{w}fill={fill}] {P(x0, y0)} rectangle {P(x1, y1)};\n"


def circle(x, y, r, fill, extra=""):
    return f"\\filldraw[inked,fill={fill}{extra}] {P(x, y)} circle ({f(r)});\n"


def poly(points, fill, extra=""):
    path = " -- ".join(P(x, y) for x, y in points)
    return f"\\filldraw[inked,fill={fill}{extra}] {path} -- cycle;\n"


def line(a, b, lw=1.1, colour="ink"):
    return f"\\draw[{colour}, line width={lw}pt, line cap=round] {P(*a)} -- {P(*b)};\n"


# ---- sub-pictures ---------------------------------------------------------

def sky(W, H, top="skyhi", mid="sky", low="skylow"):
    s = f"\\fill[{mid}] (0,0) rectangle {P(W, H)};\n"
    s += f"\\fill[{top}] (0,{f(H * 0.78)}) rectangle {P(W, H)};\n"
    s += f"\\fill[{low}] (0,0) rectangle {P(W, H * 0.3)};\n"
    return s


def cloud(x, y, s, fill="cloud"):
    """Puffs, outlined, as the clouds over Sam_Comm_final's city."""
    puffs = [(-1.0, 0, 0.55), (-0.45, 0.25, 0.7), (0.2, 0.35, 0.8), (0.85, 0.15, 0.6), (1.35, -0.05, 0.45)]
    out = ""
    for dx, dy, r in puffs:
        out += circle(x + dx * s, y + dy * s, r * s, fill)
    out += f"\\fill[{fill}] {P(x - 1.0 * s, y - 0.15 * s)} rectangle {P(x + 1.35 * s, y + 0.05 * s)};\n"
    out += line((x - 1.25 * s, y - 0.15 * s), (x + 1.6 * s, y - 0.15 * s))
    return out


def clock(x, y, r, fill="clock", minute=14):
    """A clock face stopped at 21:14: the hour hand at 277°, the minute
    hand at 84°, clockwise from twelve. Only the ending's tower passes
    minute=15 (21:15: the hour hand at 277.5°, the minute hand at 90°)."""
    s = circle(x, y, r, fill)
    s += f"\\draw[ink, line width=1.6pt] {P(x, y)} circle ({f(r * 0.86)});\n"
    s += f"\\foreach \\k in {{0,...,11}} {{ \\fill[ink] ($({f(x)},{f(y)})+(\\k*30:{f(r * 0.74)})$) circle ({f(r * 0.06)}); }}\n"
    for deg, length, w in [(270 + minute * 0.5, 0.5, 3.2), (minute * 6, 0.72, 2.2)]:
        a = math.radians(90 - deg)
        s += line((x, y), (x + math.cos(a) * r * length, y + math.sin(a) * r * length), lw=w)
    s += circle(x, y, r * 0.07, "ink")
    return s


def building(rng, x, y0, w, h, wall=None, detail=True, lit_pct=45):
    wall = wall or rng.choice(WALLS)
    s = rect(x, y0, x + w, y0 + h, wall)
    s += rect(x - 0.08, y0 + h, x + w + 0.08, y0 + h + 0.22, wall)
    cols = max(1, int((w - 0.3) / 0.55))
    rows = max(1, int((h - 0.6) / 0.75))
    gap_x = (w - cols * 0.32) / (cols + 1)
    for i in range(cols):
        for j in range(rows):
            wx = x + gap_x + i * (0.32 + gap_x)
            wy = y0 + 0.45 + j * 0.75
            if wy + 0.42 > y0 + h - 0.1:
                continue
            fill = "window" if rng.random() * 100 < lit_pct else "windark"
            s += f"\\filldraw[inked,line width=0.6pt,fill={fill}] {P(wx, wy)} rectangle {P(wx + 0.32, wy + 0.42)};\n"
            if detail and rng.random() < 0.06:
                s += rect(wx - 0.05, wy - 0.28, wx + 0.37, wy - 0.02, "metal", lw=0.7)
    if detail:
        s += roof(rng, x, y0 + h + 0.22, w)
        if rng.random() < 0.4:
            g = rng.choice(GREENS)
            vx = x + rng.uniform(0.2, w - 0.5)
            s += poly([(vx, y0 + h + 0.1), (vx + 0.35, y0 + h + 0.1), (vx + 0.25, y0 + h - rng.uniform(1.5, 4)), (vx + 0.05, y0 + h - rng.uniform(1, 3))], g)
    return s


def roof(rng, x, y, w):
    s = ""
    for _ in range(rng.randint(1, 3)):
        kind = rng.choice(["ac", "ac", "tank", "antenna", "billboard"])
        px = x + rng.uniform(0.2, max(0.3, w - 1.0))
        if kind == "ac":
            s += rect(px, y, px + 0.6, y + 0.4, "metal", lw=0.9)
            s += circle(px + 0.3, y + 0.2, 0.13, "metaldark", ",line width=0.6pt")
        elif kind == "tank":
            for lx in (0.08, 0.62):
                s += line((px + lx, y), (px + lx, y + 0.5), lw=0.9)
            s += rect(px, y + 0.5, px + 0.7, y + 1.15, "wood", lw=0.9)
            s += poly([(px - 0.05, y + 1.15), (px + 0.75, y + 1.15), (px + 0.35, y + 1.45)], "wallE", ",line width=0.9pt")
        elif kind == "antenna":
            top = y + rng.uniform(1.0, 2.2)
            s += line((px + 0.3, y), (px + 0.3, top), lw=0.9)
            for k in range(1, 4):
                yy = y + (top - y) * k / 4
                s += line((px + 0.05, yy), (px + 0.55, yy), lw=0.7)
        else:
            c = rng.choice(SIGNS)
            s += line((px + 0.2, y), (px + 0.2, y + 0.6), lw=1)
            s += line((px + 1.2, y), (px + 1.2, y + 0.6), lw=1)
            s += rect(px, y + 0.6, px + 1.4, y + 1.3, c)
            s += f"\\draw[paper, line width=1.5pt] {P(px + 0.25, y + 0.95)} -- {P(px + 0.55, y + 1.1)} -- {P(px + 0.85, y + 0.85)} -- {P(px + 1.15, y + 1.05)};\n"
    return s


def clocktower(x, y0, w, h, minute=14):
    s = rect(x, y0, x + w, y0 + h, "stone")
    s += rect(x - 0.25, y0 + h, x + w + 0.25, y0 + h + 0.35, "wallE")
    s += rect(x - 0.1, y0 + h + 0.35, x + w + 0.1, y0 + h + w * 1.05, "stone")
    s += clock(x + w / 2, y0 + h + 0.35 + w * 0.52, w * 0.4, minute=minute)
    s += poly([(x - 0.3, y0 + h + w * 1.05), (x + w + 0.3, y0 + h + w * 1.05), (x + w / 2, y0 + h + w * 1.05 + w * 0.9)], "wallE")
    for j in range(int(h / 1.2)):
        s += rect(x + w * 0.4, y0 + 0.6 + j * 1.2, x + w * 0.6, y0 + 1.1 + j * 1.2, "windark", lw=0.7)
    return s


def orb(x, y, r, colour, gold=False, glow=True, rim=False):
    """An orb as the game draws it: a halo, an inked core, a highlight.
    Only a gold orb (one Objection parries) carries the gold ring; a blue
    orb (one the Injunction breaks) a pale rim."""
    s = ""
    if glow:
        s += f"\\fill[{colour}, opacity=0.22] {P(x, y)} circle ({f(r * 1.9)});\n"
        s += f"\\fill[{colour}, opacity=0.3] {P(x, y)} circle ({f(r * 1.35)});\n"
    s += circle(x, y, r, colour)
    s += f"\\fill[white, opacity=0.75] {P(x - r * 0.35, y + r * 0.35)} circle ({f(r * 0.28)});\n"
    if gold:
        s += f"\\draw[gold, line width=2.2pt] {P(x, y)} ellipse ({f(r * 1.6)} and {f(r * 0.5)});\n"
        s += f"\\draw[ink, line width=0.7pt] {P(x, y)} ellipse ({f(r * 1.75)} and {f(r * 0.62)});\n"
    if rim:
        s += f"\\draw[icewhite, line width=1.6pt] {P(x, y)} ellipse ({f(r * 0.5)} and {f(r * 1.3)});\n"
    return s


def orb_ring(cx, cy, rx, ry, n, r, colour, phase=0.0, gold_every=0):
    s = ""
    for k in range(n):
        a = phase + 2 * math.pi * k / n
        # back half smaller: the ring is seen tilted
        depth = 0.75 + 0.25 * math.sin(a)
        s += orb(cx + rx * math.cos(a), cy + ry * math.sin(a), r * depth, colour if not (gold_every and k % gold_every == 0) else "gold",
                 gold=bool(gold_every and k % gold_every == 0))
    return s


def orb_spiral(cx, cy, arms, per_arm, r0, step, size, colours, turn=0.32):
    s = ""
    for a in range(arms):
        for k in range(per_arm):
            ang = 2 * math.pi * a / arms + k * turn
            rad = r0 + k * step
            s += orb(cx + rad * math.cos(ang), cy + rad * math.sin(ang) * 0.6, size * (0.6 + 0.4 * k / per_arm), colours[a % len(colours)])
    return s


def defendant(x, y, s, lean=0.0, gun=True, falling=False):
    """The Defendant: a long dark coat, an orange scarf, the gun held out.
    (x, y) is between the feet; s scales a 2.0-tall figure."""
    def T(px, py):
        # lean the figure about its feet
        return (x + (px + py * lean) * s, y + py * s)
    out = ""
    # legs
    legs = [(-0.18, 0.0), (0.18, 0.0)] if not falling else [(-0.3, 0.15), (0.25, -0.05)]
    for lx, ly in legs:
        out += poly([T(lx - 0.09, ly), T(lx + 0.09, ly), T(lx * 0.4 + 0.08, 0.9), T(lx * 0.4 - 0.08, 0.9)], "coatdark")
        out += poly([T(lx - 0.12, ly - 0.02), T(lx + 0.16, ly - 0.02), T(lx + 0.16, ly + 0.09), T(lx - 0.12, ly + 0.09)], "hair")
    # the coat: a skirt that flares, a body
    out += poly([T(-0.42, 0.45), T(0.42, 0.45), T(0.27, 1.05), T(-0.27, 1.05)], "coat")
    out += poly([T(-0.27, 1.0), T(0.27, 1.0), T(0.3, 1.62), T(-0.3, 1.62)], "coat")
    out += poly([T(-0.28, 1.0), T(0.28, 1.0), T(0.28, 1.08), T(-0.28, 1.08)], "trim")
    # the scarf, trailing behind
    out += f"\\filldraw[inked,fill=trim] {P(*T(-0.12, 1.6))} .. controls {P(*T(-0.6, 1.75))} and {P(*T(-0.9, 1.45))} .. {P(*T(-1.35, 1.62))} -- {P(*T(-1.3, 1.5))} .. controls {P(*T(-0.85, 1.32))} and {P(*T(-0.55, 1.55))} .. {P(*T(0.1, 1.5))} -- cycle;\n"
    # head and hair
    hx, hy = T(0, 1.83)
    out += circle(hx, hy, 0.2 * s, "skin")
    out += f"\\filldraw[inked,fill=hair] {P(hx - 0.21 * s, hy)} arc (180:0:{f(0.21 * s)}) -- {P(hx + 0.12 * s, hy + 0.02 * s)} -- {P(hx - 0.05 * s, hy + 0.1 * s)} -- cycle;\n"
    if gun:
        sx, sy = T(0.22, 1.5)
        gx, gy = T(0.85, 1.42)
        out += line((sx, sy), (gx, gy), lw=3.2 * s / 1.2, colour="coat")
        out += poly([(gx - 0.05 * s, gy - 0.08 * s), (gx + 0.55 * s, gy - 0.03 * s), (gx + 0.55 * s, gy + 0.08 * s), (gx - 0.05 * s, gy + 0.1 * s)], "gun")
        out += f"\\fill[cyan] {P(gx + 0.1 * s, gy + 0.08 * s)} rectangle {P(gx + 0.35 * s, gy + 0.12 * s)};\n"
    return out


def bailiff(x, y, s):
    """The Bailiff of Hours: robe, a clock for a face, crown, two hands with
    gavels, a ring of hours. (x, y) is the robe's hem centre; s scales."""
    out = ""
    # the ring of hours, behind
    out += f"\\draw[goldtrim, line width={f(3 * s)}pt] {P(x, y + 2.3 * s)} ellipse ({f(2.6 * s)} and {f(0.7 * s)});\n"
    out += f"\\draw[ink, line width=0.8pt] {P(x, y + 2.3 * s)} ellipse ({f(2.7 * s)} and {f(0.78 * s)});\n"
    out += poly([(x - 1.3 * s, y), (x + 1.3 * s, y), (x + 0.55 * s, y + 2.6 * s), (x - 0.55 * s, y + 2.6 * s)], "robe")
    out += f"\\draw[goldtrim, line width={f(2.4 * s)}pt] {P(x - 1.25 * s, y + 0.12 * s)} -- {P(x + 1.25 * s, y + 0.12 * s)};\n"
    out += poly([(x - 0.75 * s, y + 2.35 * s), (x + 0.75 * s, y + 2.35 * s), (x + 0.65 * s, y + 2.65 * s), (x - 0.65 * s, y + 2.65 * s)], "goldtrim")
    out += clock(x, y + 3.25 * s, 0.62 * s, "bossface")
    out += circle(x, y + 3.25 * s, 0.08 * s, "eye")
    # the crown
    cy0 = y + 3.85 * s
    out += rect(x - 0.42 * s, cy0, x + 0.42 * s, cy0 + 0.22 * s, "goldtrim")
    for k in range(5):
        px = x - 0.36 * s + k * 0.18 * s
        out += poly([(px - 0.07 * s, cy0 + 0.22 * s), (px + 0.07 * s, cy0 + 0.22 * s), (px, cy0 + 0.5 * s)], "goldtrim")
    # the hands and gavels
    for side in (-1, 1):
        hx, hy = x + side * 1.75 * s, y + 2.0 * s + (0.35 * s if side > 0 else 0)
        out += circle(hx, hy, 0.3 * s, "bossface")
        out += line((hx, hy + 0.25 * s), (hx, hy + 1.05 * s), lw=2.5 * s, colour="courtdark")
        out += f"\\filldraw[inked,fill=goldtrim] {P(hx - 0.42 * s, hy + 1.0 * s)} rectangle {P(hx + 0.42 * s, hy + 1.35 * s)};\n"
    for k in range(12):
        a = 2 * math.pi * k / 12
        px, py = x + 2.6 * s * math.cos(a), y + 2.3 * s + 0.7 * s * math.sin(a)
        out += poly([(px, py - 0.14 * s), (px + 0.1 * s, py), (px, py + 0.14 * s), (px - 0.1 * s, py)], "goldtrim", ",line width=0.7pt")
    return out


def juror(x, y, s, colour="courtdark"):
    out = poly([(x - 0.35 * s, y), (x + 0.35 * s, y), (x, y + 0.95 * s)], colour)
    out += f"\\fill[eye] {P(x - 0.08 * s, y + 0.55 * s)} circle ({f(0.035 * s)}) {P(x + 0.08 * s, y + 0.55 * s)} circle ({f(0.035 * s)});\n"
    return out


def tree(rng, x, y, s):
    out = rect(x - 0.08 * s, y, x + 0.08 * s, y + 0.9 * s, "wood")
    for dx, dy, r in [(-0.3, 1.0, 0.45), (0.25, 1.1, 0.5), (0, 1.45, 0.45)]:
        out += circle(x + dx * s, y + dy * s, r * s, rng.choice(GREENS))
    return out


def skyline(rng, W, y0, depth_rows=((0.6, 3.0, 6.5, 20), (0.0, 4.5, 9.5, 14))):
    out = ""
    for lit, hmin, hmax, n in depth_rows:
        x = -0.5
        while x < W:
            w = rng.uniform(1.4, 2.6)
            h = rng.uniform(hmin, hmax)
            out += building(rng, x, y0, w, h, detail=True, lit_pct=40 + lit * 20)
            x += w + rng.uniform(-0.2, 0.25)
    return out


# ---- pictures --------------------------------------------------------------

def title():
    """Key art. The Waiting City at dusk, the clock tower stopped at 21:14,
    the Defendant dropping in through a ring of orbs, the Bailiff looming
    over the skyline. The lower left stays calm for the page's title box."""
    W, H = 32.0, 18.0
    rng = random.Random(2031)
    s = sky(W, H)
    for (cx, cy, cs) in [(4, 15.6, 1.4), (11, 16.5, 1.1), (19.5, 15.8, 1.6), (27.5, 16.4, 1.2), (30.5, 14.2, 0.9)]:
        s += cloud(cx, cy, cs)
    s += bailiff(22.0, 7.5, 2.0)
    s += skyline(rng, W, 2.2, depth_rows=((0.2, 5.0, 10.0, 0), (0.8, 3.0, 7.5, 0)))
    s += clocktower(13.6, 2.2, 1.9, 9.0)
    # the street
    s += f"\\filldraw[inked,fill=paving] (-0.5,-0.5) -- (-0.5,2.4) -- {P(W + 0.5, 2.4)} -- {P(W + 0.5, -0.5)} -- cycle;\n"
    for k in range(9):
        s += f"\\fill[ground] {P(1 + k * 3.6, 0.8)} rectangle {P(2.6 + k * 3.6, 1.0)};\n"
    for tx in (1.2, 9.6, 29.4):
        s += tree(rng, tx, 2.2, 1.3)
    # orbs: a tilted ring round the falling figure, a spiral from the Bailiff
    s += orb_spiral(22.0, 13.4, 4, 9, 1.3, 0.62, 0.36, ["violet", "magenta", "cyan", "violet"])
    s += orb_ring(17.0, 9.4, 4.4, 1.3, 14, 0.42, "magenta", phase=0.4, gold_every=5)
    s += orb_ring(17.0, 9.4, 6.4, 1.9, 18, 0.32, "cyan", phase=0.1)
    # the time rift the Defendant fell through
    s += f"\\fill[white, opacity=0.35] (17.6,17.5) .. controls (17.0,14) and (17.8,12.5) .. (17.1,10.6) -- (16.7,10.8) .. controls (17.2,12.6) and (16.4,14.2) .. (17.0,17.6) -- cycle;\n"
    s += defendant(16.9, 8.6, 1.25, lean=0.12, falling=True)
    return s, (W, H)


def sentencing():
    """The Tribunal of the Last Hour: tiered jurors, the judge's bench, the
    great clock at 21:14, and the Defendant small in a pool of light. The
    right side is plain wall, under the page's sheet."""
    W, H = 32.0, 18.0
    rng = random.Random(81)
    s = f"\\fill[court] (0,0) rectangle {P(W, H)};\n"
    for k in range(9):
        s += f"\\fill[courtdark] {P(k * 3.8, 0)} rectangle {P(k * 3.8 + 0.6, H)};\n"
    s += clock(9.5, 13.2, 3.4)
    # the tiers of jurors
    for tier in range(4):
        y = 1.6 + tier * 1.7
        s += rect(-0.2, y - 0.4, 19.5 - tier * 0.6, y, "courtdark")
        x = 0.4
        while x < 19 - tier * 0.6:
            s += juror(x, y, 1.1 + tier * 0.05, rng.choice(["courtdark", "robe", "ink"]))
            x += rng.uniform(0.75, 1.0)
    # the bench
    s += rect(5.5, 5.8, 13.5, 8.6, "courtdark")
    s += rect(5.3, 8.6, 13.7, 9.0, "goldtrim")
    s += bailiff(9.5, 8.9, 0.85)
    # the light and the Defendant
    s += f"\\fill[lamp, opacity=0.35] (9.0,17.9) -- (10.2,17.9) -- (12.8,0.0) -- (6.4,0.0) -- cycle;\n"
    s += f"\\fill[lamp, opacity=0.5] (9.6,0.6) ellipse (2.3 and 0.45);\n"
    s += defendant(9.6, 0.6, 1.05, gun=False)
    return s, (W, H)


def indictment():
    """The Bailiff's desk in the Holding: the indictment's pages, a gavel, a
    lamp, a stopped pocket watch, the city at night through the window. The
    right side is the page's sheet."""
    W, H = 32.0, 18.0
    rng = random.Random(14)
    s = f"\\fill[night] (0,0) rectangle {P(W, H)};\n"
    # the window onto the city at night
    s += rect(1.2, 7.0, 15.0, 17.0, "nightsky", lw=2.5)
    s += f"\\begin{{scope}}\\clip (1.2,7.0) rectangle (15.0,17.0);\n"
    x = 0.8
    while x < 15.5:
        w, h = rng.uniform(1.1, 2.2), rng.uniform(2.5, 7.5)
        s += building(rng, x, 7.0, w, h, wall=rng.choice(["courtdark", "robe", "court"]), detail=True, lit_pct=55)
        x += w + 0.1
    s += clocktower(7.2, 7.0, 1.3, 6.0)
    s += "\\end{scope}\n"
    for k in range(1, 4):
        s += line((1.2 + k * 3.45, 7.0), (1.2 + k * 3.45, 17.0), lw=3)
    s += line((1.2, 12.0), (15.0, 12.0), lw=3)
    # the desk
    s += poly([(-0.5, 0), (W + 0.5, 0), (W + 0.5, 6.5), (-0.5, 6.5)], "wood")
    s += rect(-0.5, 6.3, W + 0.5, 6.8, "wallE")
    # pages, fanned
    for k, (px, py, rot) in enumerate([(3.0, 2.0, -8), (4.3, 2.4, 4), (5.7, 1.8, -3)]):
        s += f"\\begin{{scope}}[rotate around={{{rot}:({f(px + 2.2)},{f(py + 1.6)})}}]\n"
        s += rect(px, py, px + 4.4, py + 3.2, "paper")
        for j in range(6):
            s += line((px + 0.4, py + 2.6 - j * 0.42), (px + 4.0 - (j % 3) * 0.6, py + 2.6 - j * 0.42), lw=1.4, colour="paperdark")
        s += f"\\draw[red, line width=2pt, rotate around={{-14:({f(px + 3.3)},{f(py + 0.8)})}}] {P(px + 2.6, py + 0.45)} rectangle {P(px + 4.1, py + 1.1)};\n"
        s += "\\end{scope}\n"
    # the gavel
    s += poly([(10.6, 3.1), (14.2, 4.5), (14.0, 4.9), (10.4, 3.5)], "wood")
    s += f"\\filldraw[inked,fill=goldtrim,rotate around={{21:(14.4,4.8)}}] (13.6,4.2) rectangle (15.2,5.4);\n"
    # the lamp and its light
    s += f"\\fill[lamp, opacity=0.25] (11.3,15.0) -- (7.0,6.5) -- (16.5,6.5) -- cycle;\n"
    s += line((13.4, 6.6), (13.4, 13.8), lw=4)
    s += poly([(12.0, 13.6), (14.8, 13.6), (14.0, 15.0), (12.8, 15.0)], "goldtrim")
    # a pocket watch, stopped at 21:14
    s += clock(8.8, 1.5, 0.95)
    s += f"\\draw[goldtrim, line width=2.4pt] (8.8,2.45) .. controls (8.0,3.6) and (6.8,3.1) .. (6.2,2.6);\n"
    # orbs hanging in the dark, as if the arena had followed
    for (ox, oy, r, c) in [(18.0, 14.8, 0.5, "violet"), (19.4, 16.2, 0.35, "magenta"), (17.2, 16.9, 0.28, "cyan")]:
        s += orb(ox, oy, r, c)
    return s, (W, H)


def verdict():
    """The Court of Hours after the fight: the Bailiff kneels, its clock face
    cracked, dawn coming up over the city through the broken wall, the
    Defendant standing."""
    W, H = 32.0, 18.0
    rng = random.Random(48)
    s = sky(W, H, top="skyhi", mid="sky", low="wallD")
    s += f"\\fill[white, opacity=0.4] (8.5,6.0) circle (3.6);\n"
    s += circle(8.5, 6.0, 2.4, "skyhi")
    for (cx, cy, cs) in [(3.5, 14.8, 1.2), (13, 16.0, 1.0), (7.0, 12.4, 0.8)]:
        s += cloud(cx, cy, cs)
    s += skyline(rng, 17.0, 3.0, depth_rows=((0.4, 3.0, 7.0, 0),))
    # the broken wall of the court
    s += poly([(16.5, 0), (W + 0.5, 0), (W + 0.5, H), (21.0, H), (19.5, 13.0), (17.8, 10.5), (18.6, 7.0), (16.8, 4.0)], "court")
    for k in range(5):
        s += rect(22.5 + k * 2.0, 0, 23.1 + k * 2.0, H, "courtdark")
    # the floor
    s += poly([(-0.5, 0), (W + 0.5, 0), (W + 0.5, 3.0), (-0.5, 3.0)], "courtdark")
    s += f"\\draw[goldtrim, line width=2pt] (2,1.5) ellipse (9 and 0.8);\n"
    # the Bailiff kneeling: lower, tilted, its ring dropped
    s += f"\\begin{{scope}}[rotate around={{-12:(25.5,2.2)}}]\n" + bailiff(25.5, 1.5, 1.35) + "\\end{scope}\n"
    s += f"\\draw[skyhi, line width=2.4pt] (24.6,6.6) -- (25.1,5.7) -- (24.9,5.1) -- (25.5,4.6);\n"
    s += defendant(12.5, 2.4, 1.6, gun=True)
    for (ox, oy, r, c) in [(15.5, 9.0, 0.3, "gold"), (19.5, 12.0, 0.25, "violet"), (14.0, 12.8, 0.2, "cyan")]:
        s += orb(ox, oy, r, c, gold=(c == "gold"))
    return s, (W, H)


def gun_art(kind):
    """A gun seen from the side, muzzle to the right, as draw.rs builds it."""
    W, H = 10.0, 4.0
    s = ""
    if kind == "carbine":
        s += poly([(0.6, 1.6), (6.6, 1.6), (6.6, 2.5), (0.6, 2.7)], "gun")
        s += rect(6.6, 1.9, 9.4, 2.25, "metaldark")
        s += poly([(1.6, 1.6), (2.4, 1.6), (2.0, 0.4), (1.3, 0.5)], "gun")
        s += poly([(4.0, 1.6), (4.6, 1.6), (4.5, 0.8), (3.9, 0.8)], "metaldark")
        s += rect(2.6, 2.5, 5.8, 2.78, "shotcarbine" if False else "lamp")
        s += poly([(0.0, 1.7), (0.6, 1.6), (0.6, 2.7), (0.0, 2.5)], "wood")
    elif kind == "scattergun":
        s += poly([(0.4, 1.3), (5.0, 1.3), (5.0, 2.8), (0.4, 2.6)], "gun")
        s += rect(5.0, 2.2, 9.4, 2.65, "metal")
        s += rect(5.0, 1.6, 9.4, 2.05, "metal")
        s += circle(9.4, 2.43, 0.2, "ink")
        s += circle(9.4, 1.83, 0.2, "ink")
        s += poly([(1.2, 1.3), (2.2, 1.3), (1.8, 0.2), (0.9, 0.3)], "wood")
        s += rect(1.6, 2.8, 4.4, 3.05, "gold")
    elif kind == "big_iron":
        s += rect(4.6, 2.1, 9.6, 2.55, "metaldark")
        s += rect(9.3, 2.0, 9.7, 2.65, "metaldark")
        s += f"\\filldraw[inked,fill=gun,rounded corners=3pt] (3.0,1.6) rectangle (4.9,3.0);\n"
        for k in range(3):
            s += f"\\draw[ink, line width=0.8pt] {P(3.2, 1.85 + k * 0.4)} -- {P(4.7, 1.85 + k * 0.4)};\n"
        s += poly([(2.2, 2.9), (3.1, 3.0), (3.1, 2.6), (2.6, 2.5)], "gun")
        s += poly([(1.9, 2.0), (3.2, 1.7), (2.6, 0.2), (1.4, 0.4)], "wood")
        s += circle(2.15, 1.2, 0.12, "goldtrim")
        s += rect(4.8, 2.55, 9.4, 2.7, "lamp")
    elif kind == "ripsaw":
        s += poly([(0.4, 1.4), (5.6, 1.4), (5.6, 2.8), (0.4, 2.6)], "gun")
        s += poly([(1.3, 1.4), (2.2, 1.4), (1.9, 0.3), (1.0, 0.4)], "gun")
        s += rect(5.6, 2.6, 8.0, 3.0, "metaldark")
        teeth = []
        for k in range(24):
            a = 2 * math.pi * k / 24
            r = 1.55 if k % 2 == 0 else 1.25
            teeth.append((7.8 + r * math.cos(a), 1.9 + r * math.sin(a)))
        s += poly(teeth, "sawsteel")
        s += circle(7.8, 1.9, 0.55, "metal")
        s += circle(7.8, 1.9, 0.18, "ink")
        s += rect(2.0, 2.8, 4.8, 3.05, "chalk")
    elif kind == "filibuster":
        # three barrels round one axis, and a strip that heats from cream to orange
        s += poly([(0.0, 1.7), (0.6, 1.6), (0.6, 2.7), (0.0, 2.5)], "wood")
        s += poly([(0.6, 1.5), (5.0, 1.5), (5.0, 2.85), (0.6, 2.7)], "gun")
        s += poly([(1.6, 1.5), (2.4, 1.5), (2.0, 0.35), (1.3, 0.45)], "gun")
        s += poly([(3.6, 1.5), (4.3, 1.5), (4.2, 0.7), (3.5, 0.7)], "metaldark")
        for y in (1.62, 1.98, 2.34):
            s += rect(5.0, y, 9.3, y + 0.24, "metaldark")
        s += rect(5.0, 1.45, 5.55, 2.75, "gun")
        s += rect(8.6, 1.5, 8.95, 2.7, "metal")
        s += rect(1.3, 2.75, 2.3, 3.0, "lamp")
        s += rect(2.35, 2.75, 3.35, 3.0, "gold")
        s += rect(3.4, 2.75, 4.4, 3.0, "trim")
    elif kind == "redactor":
        # a long, slim rifle; a black bar across the muzzle; a strip struck through
        s += poly([(0.0, 1.8), (0.5, 1.7), (0.5, 2.6), (0.0, 2.45)], "wood")
        s += poly([(0.5, 1.7), (6.6, 1.7), (6.8, 2.6), (0.5, 2.6)], "gun")
        s += poly([(1.5, 1.7), (2.3, 1.7), (1.9, 0.5), (1.2, 0.6)], "gun")
        s += poly([(3.8, 1.7), (4.4, 1.7), (4.6, 0.8), (4.0, 0.8)], "metaldark")
        s += rect(6.6, 2.0, 9.0, 2.28, "metaldark")
        s += rect(9.0, 1.3, 9.4, 3.0, "ink")
        s += rect(1.0, 2.6, 5.4, 2.9, "lamp")
        for (a, b) in ((1.2, 2.3), (2.6, 4.0), (4.3, 5.0)):
            s += rect(a, 2.64, b, 2.86, "ink")
    else:
        s += rect(1.0, 1.4, 7.6, 2.8, "gun")
        s += circle(8.2, 2.1, 1.0, "cyan")
        s += f"\\fill[white, opacity=0.7] (7.9,2.45) circle (0.25);\n"
        s += f"\\fill[cyan, opacity=0.25] (8.2,2.1) circle (1.6);\n"
        s += poly([(2.6, 1.4), (3.4, 1.4), (3.1, 0.3), (2.3, 0.4)], "metaldark")
        for k in range(4):
            s += rect(1.6 + k * 1.3, 2.8, 2.2 + k * 1.3, 3.2, "metal")
    return s, (W, H)


def icon(kind):
    """A 4 cm square icon for an upgrade, a reward or a map node."""
    W, H = 4.0, 4.0
    s = circle(2.0, 2.0, 1.85, "paper")
    if kind == "heart":
        s += "\\filldraw[inked,fill=heart] (2,0.75) .. controls (0.2,2.2) and (0.9,3.6) .. (2,2.8) .. controls (3.1,3.6) and (3.8,2.2) .. (2,0.75) -- cycle;\n"
    elif kind == "jump":
        for k in range(3):
            y = 0.9 + k * 0.85
            s += poly([(0.9, y), (2.0, y + 0.75), (3.1, y), (3.1, y + 0.4), (2.0, y + 1.15), (0.9, y + 0.4)], "jump")
    elif kind == "speed":
        for k in range(2):
            x = 0.8 + k * 1.1
            s += poly([(x, 1.0), (x + 0.6, 1.0), (x + 1.4, 2.0), (x + 0.6, 3.0), (x, 3.0), (x + 0.8, 2.0)], "cyan")
    elif kind == "gun":
        s += poly([(0.6, 1.8), (2.9, 1.8), (2.9, 2.5), (0.6, 2.6)], "gun")
        s += rect(2.9, 2.0, 3.6, 2.3, "metaldark")
        s += poly([(1.0, 1.8), (1.6, 1.8), (1.3, 1.0), (0.8, 1.1)], "gun")
        s += rect(1.3, 2.5, 2.6, 2.7, "gold")
    elif kind == "parry":
        s += orb(2.4, 2.4, 0.55, "gold", gold=True, glow=False)
        s += "\\draw[ink, line width=2.4pt, -{Latex[length=3mm]}] (1.1,1.0) .. controls (0.6,2.0) and (1.2,3.1) .. (1.8,3.2);\n" if False else "\\draw[ink, line width=2.4pt] (1.0,0.9) .. controls (0.5,2.0) and (1.1,3.1) .. (1.8,3.25);\n\\filldraw[inked,fill=ink] (1.8,3.25) -- (1.45,3.5) -- (1.55,3.0) -- cycle;\n"
    elif kind == "hook":
        s += "\\draw[trim, line width=3pt] (0.8,0.8) -- (2.6,2.6);\n"
        s += poly([(2.4, 2.4), (3.3, 2.7), (2.7, 3.3)], "trim")
        s += "\\draw[ink, line width=2pt] (2.6,2.6) .. controls (3.4,2.4) and (3.4,1.6) .. (2.8,1.4);\n"
    elif kind == "dash":
        for k in range(3):
            s += f"\\draw[cyan, line width=2.5pt, opacity={f(0.4 + 0.3 * k)}] {P(0.6 + k * 0.4, 2.0 + (k - 1) * 0.6)} -- {P(1.6 + k * 0.4, 2.0 + (k - 1) * 0.6)};\n"
        s += circle(2.9, 2.0, 0.55, "coat")
    elif kind == "pierce":
        # a shot through two plates
        s += rect(1.3, 0.9, 1.6, 3.1, "metal")
        s += rect(2.3, 0.9, 2.6, 3.1, "metal")
        s += "\\draw[trim, line width=3pt] (0.6,2.0) -- (3.0,2.0);\n"
        s += poly([(3.0, 1.7), (3.5, 2.0), (3.0, 2.3)], "trim")
    elif kind == "pulse":
        s += "\\draw[cyan, line width=2.6pt] (2,2) circle (1.25);\n"
        s += "\\draw[ink, line width=0.8pt] (2,2) circle (1.42);\n"
        s += circle(2.0, 2.0, 0.5, "coat")
        for a in (30, 150, 270):
            import math as _m
            x, y = 2 + 1.25 * _m.cos(_m.radians(a)), 2 + 1.25 * _m.sin(_m.radians(a))
            s += circle(x, y, 0.22, "violet")
    elif kind == "mend":
        s += orb(2.0, 1.7, 0.6, "gold", gold=True, glow=False)
        s += "\\filldraw[inked,fill=heart] (2,2.55) .. controls (1.2,3.2) and (1.5,3.8) .. (2,3.4) .. controls (2.5,3.8) and (2.8,3.2) .. (2,2.55) -- cycle;\n"
    elif kind == "wall":
        # a pane of light, a blue orb breaking on it, an ember one passing
        s += f"\\filldraw[inked,fill=iceglow,fill opacity=0.8] (1.4,0.8) rectangle (2.4,3.2);\n"
        s += "\\draw[icewhite, line width=1.5pt] (1.6,1.0) -- (1.6,3.0);\n"
        s += orb(0.85, 2.4, 0.38, "cobalt", glow=False, rim=True)
        for (x, y) in ((1.25, 2.9), (1.2, 1.9), (1.35, 2.55)):
            s += poly([(x, y), (x + 0.18, y + 0.1), (x + 0.05, y - 0.15)], "cobalt", ",line width=0.6pt")
        s += orb(3.2, 1.5, 0.32, "ember", glow=False)
        s += "\\draw[ember, line width=1.4pt, opacity=0.7] (1.0,1.6) -- (2.8,1.5);\n"
    elif kind == "pound":
        # a gavel's head coming straight down onto a shockwave
        s += f"\\filldraw[inked,fill=goldtrim] (1.3,2.3) rectangle (2.7,3.0);\n"
        s += rect(1.85, 3.0, 2.15, 3.6, "wood")
        s += "\\draw[ink, line width=1.6pt] (2.0,2.2) -- (2.0,1.6);\n"
        for k, r in enumerate((0.55, 1.0, 1.45)):
            s += f"\\draw[halo, line width={f(2.4 - k * 0.6)}pt] (2,1.2) ellipse ({f(r)} and {f(r * 0.3)});\n"
    elif kind == "saw":
        teeth = []
        for k in range(20):
            a = 2 * math.pi * k / 20
            r = 1.55 if k % 2 == 0 else 1.2
            teeth.append((2 + r * math.cos(a), 2 + r * math.sin(a)))
        s += poly(teeth, "sawsteel")
        s += circle(2.0, 2.0, 0.5, "metal")
        s += circle(2.0, 2.0, 0.16, "ink")
    elif kind == "conduct":
        pts = []
        for k in range(10):
            a = math.pi / 2 + math.pi * k / 5
            r = 1.4 if k % 2 == 0 else 0.6
            pts.append((2 + r * math.cos(a), 2.1 + r * math.sin(a)))
        s += poly(pts, "gold")
        s += poly([(1.4, 0.4), (1.9, 0.4), (2.0, 1.0), (1.5, 1.0)], "carpet")
        s += poly([(2.1, 0.4), (2.6, 0.4), (2.5, 1.0), (2.0, 1.0)], "carpet")
    elif kind == "trigger":
        s += "\\draw[ink, line width=2.4pt] (2.0,1.6) circle (1.0);\n"
        s += poly([(2.2, 3.3), (1.4, 1.9), (2.0, 1.9), (1.7, 0.7), (2.7, 2.3), (2.1, 2.3)], "gold")
    elif kind == "plating":
        for k in range(3):
            y = 2.6 - k * 0.75
            s += f"\\filldraw[inked,fill=metal,rounded corners=3pt] {P(0.9 + k * 0.25, y)} rectangle {P(3.1 - k * 0.25, y + 0.6)};\n"
            s += circle(1.2 + k * 0.25, y + 0.3, 0.08, "ink")
            s += circle(2.8 - k * 0.25, y + 0.3, 0.08, "ink")
    elif kind == "stop":
        # a clock face, its hands stopped, and a hand held up before it
        s += circle(2.0, 2.0, 1.25, "clock")
        s += "\\draw[ink, line width=1.8pt] (2,2) -- (1.15,2.1);\n\\draw[ink, line width=1.2pt] (2,2) -- (2.95,2.1);\n"
        s += "\\draw[violetdeep, line width=2.2pt] (2,2) circle (1.5);\n"
    elif kind == "gavel":
        s += poly([(0.9, 0.9), (2.4, 2.4), (2.1, 2.7), (0.6, 1.2)], "wood")
        s += f"\\filldraw[inked,fill=goldtrim,rotate around={{45:(2.6,2.6)}}] (1.9,2.25) rectangle (3.3,2.95);\n"
    elif kind == "contempt":
        # a body, and the ring it bursts in
        s += "\\draw[trim, line width=2.6pt] (2,2) circle (1.2);\n"
        for k in range(8):
            a = 2 * math.pi * k / 8
            s += f"\\draw[trim, line width=2pt] {P(2 + 0.62 * math.cos(a), 2 + 0.62 * math.sin(a))} -- {P(2 + 0.98 * math.cos(a), 2 + 0.98 * math.sin(a))};\n"
        s += circle(2.0, 2.0, 0.45, "gun")
    elif kind == "purse":
        # a tied purse, and a coin
        s += "\\filldraw[inked,fill=goldtrim] (2,0.8) .. controls (0.6,0.8) and (0.8,2.4) .. (1.6,2.6) -- (2.4,2.6) .. controls (3.2,2.4) and (3.4,0.8) .. (2,0.8) -- cycle;\n"
        s += poly([(1.5, 2.6), (2.5, 2.6), (2.8, 3.2), (1.2, 3.2)], "goldtrim")
        s += "\\draw[ink, line width=1.4pt] (1.55,2.65) -- (2.45,2.65);\n"
        s += circle(2.0, 1.6, 0.35, "gold")
    elif kind == "bench":
        # a plank in the air, its gold edges, a shot going through it
        s += "\\draw[ember, line width=2pt] (1.0,0.7) -- (2.6,3.3);\n"
        s += "\\filldraw[inked,fill=bench] (0.6,1.7) -- (3.4,1.7) -- (3.4,2.3) -- (0.6,2.3) -- cycle;\n"
        s += "\\draw[goldtrim, line width=1.8pt] (0.7,2.25) -- (3.3,2.25);\n\\draw[goldtrim, line width=1.8pt] (0.7,1.75) -- (3.3,1.75);\n"
        s += circle(2.0, 2.95, 0.42, "coat")
    elif kind == "blade":
        # Summary Judgment: a great blade swung, and the arc of light it cuts
        s += "\\draw[iceglow, line width=3.2pt, opacity=0.85] (0.55,2.9) .. controls (0.9,0.9) and (2.6,0.5) .. (3.5,1.4);\n"
        s += "\\draw[chalk, line width=1.4pt] (0.75,2.7) .. controls (1.05,1.1) and (2.55,0.75) .. (3.3,1.45);\n"
        s += poly([(1.2, 1.0), (3.15, 2.95), (3.4, 3.4), (2.95, 3.15), (1.0, 1.2)], "sawsteel")
        s += "\\draw[metal, line width=0.8pt] (1.15,1.15) -- (3.15,3.15);\n"
        s += poly([(0.55, 1.3), (1.3, 0.55), (1.45, 0.7), (0.7, 1.45)], "goldtrim")
        s += poly([(0.95, 0.8), (0.55, 0.4), (0.4, 0.55), (0.8, 0.95)], "wood")
        s += circle(0.42, 0.42, 0.13, "goldtrim")
    elif kind == "sealed":
        # a letter, and the court's wax seal on it
        s += rect(0.7, 1.1, 3.3, 2.9, "lamp")
        s += "\\draw[ink, line width=1pt] (0.7,2.9) -- (2,1.9) -- (3.3,2.9);\n"
        s += circle(2.0, 1.75, 0.45, "heart")
    return s, (W, H)


# ---- zone two ---------------------------------------------------------------

def ice_sky(W, H):
    s = f"\\fill[icefog] (0,0) rectangle {P(W, H)};\n"
    s += f"\\fill[icesky] (0,{f(H * 0.55)}) rectangle {P(W, H)};\n"
    return s


def aurora(rng, x0, x1, y0, n, colour, amp=1.2):
    """Curtains of light: tall thin panes rising and falling, faint."""
    s = ""
    for k in range(n):
        x = x0 + (x1 - x0) * k / max(1, n - 1)
        wave = math.sin(k * 0.55)
        h = 2.2 + abs(wave) * 2.4
        y = y0 + wave * amp
        s += f"\\fill[{colour}, opacity={f(0.18 + 0.12 * (k % 3))}] {P(x, y)} rectangle {P(x + (x1 - x0) / n * 0.9, y + h)};\n"
    return s


def mountain(x, y0, w, h, rock="icerock"):
    s = poly([(x - w / 2, y0), (x + w / 2, y0), (x, y0 + h)], rock)
    s += poly([(x - w * 0.21, y0 + h * 0.58), (x + w * 0.21, y0 + h * 0.58), (x + w * 0.08, y0 + h * 0.5), (x - w * 0.02, y0 + h * 0.6), (x - w * 0.1, y0 + h * 0.52)], "icewhite", ",line width=0.6pt")
    s += poly([(x - w * 0.21, y0 + h * 0.58), (x, y0 + h), (x + w * 0.21, y0 + h * 0.58)], "icewhite")
    return s


def ship(x, y, s, sail=True):
    """The expedition's icebreaker: a red hull, a white bridge, two masts."""
    out = poly([(x - 3.2 * s, y + 0.9 * s), (x + 2.6 * s, y + 0.9 * s), (x + 3.4 * s, y + 1.8 * s), (x - 3.6 * s, y + 1.8 * s)], "hull")
    out += rect(x - 3.1 * s, y + 0.9 * s, x + 2.5 * s, y + 1.12 * s, "hulldark", lw=0.8)
    out += rect(x + 0.4 * s, y + 1.8 * s, x + 2.0 * s, y + 2.7 * s, "icewhite")
    for k in range(3):
        out += rect(x + 0.6 * s + k * 0.45 * s, y + 2.2 * s, x + 0.85 * s + k * 0.45 * s, y + 2.45 * s, "lamp", lw=0.6)
    out += rect(x + 2.3 * s, y + 1.8 * s, x + 2.7 * s, y + 2.9 * s, "icered")
    for mx, top in ((-2.0, 4.6), (-0.4, 3.9)):
        out += line((x + mx * s, y + 1.8 * s), (x + mx * s, y + top * s), lw=1.6 * s)
        out += line((x + (mx - 0.7) * s, y + (top - 1.1) * s), (x + (mx + 0.7) * s, y + (top - 1.1) * s), lw=1.0 * s)
    return out


def floe(x, y, w):
    return poly([(x, y), (x + w, y), (x + w * 0.9, y + 0.35), (x + w * 0.15, y + 0.4)], "icewhite")


def witness(x, y, s):
    """The Witness: a robe grown into ice, one great eye, a crown of
    shards, arms of ice lowered. (x, y) is the robe's hem centre."""
    out = ""
    for k in range(7):
        a = k / 6
        px = x - 1.6 * s + a * 3.2 * s
        h = (1.6 + (k % 3) * 0.7) * s
        out += poly([(px - 0.35 * s, y), (px + 0.35 * s, y), (px + 0.1 * s, y + h)], "witnessice")
    out += poly([(x - 1.2 * s, y), (x + 1.2 * s, y), (x + 0.45 * s, y + 3.0 * s), (x - 0.45 * s, y + 3.0 * s)], "witnessrobe")
    out += poly([(x - 0.75 * s, y + 2.8 * s), (x + 0.75 * s, y + 2.8 * s), (x, y + 4.4 * s)], "witnessrobe")
    out += f"\\filldraw[inked,fill=gold] {P(x, y + 3.35 * s)} ellipse ({f(0.34 * s)} and {f(0.22 * s)});\n"
    out += circle(x, y + 3.35 * s, 0.1 * s, "ink")
    for k in range(5):
        px = x - 0.6 * s + k * 0.3 * s
        out += poly([(px - 0.08 * s, y + 4.0 * s), (px + 0.08 * s, y + 4.0 * s), (px, y + 4.7 * s + (k % 2) * 0.25 * s)], "iceglow")
    for side in (-1, 1):
        sx, sy = x + side * 0.55 * s, y + 2.5 * s
        hx, hy = x + side * 1.9 * s, y + 0.9 * s
        out += poly([(sx, sy + 0.15 * s), (hx + side * 0.1 * s, hy + 0.1 * s), (hx - side * 0.15 * s, hy - 0.15 * s), (sx - side * 0.05 * s, sy - 0.2 * s)], "witnessice")
        out += poly([(hx - 0.15 * s, hy - 0.2 * s), (hx + 0.15 * s, hy - 0.2 * s), (hx, hy - 0.7 * s)], "iceglow")
    return out


def interlude():
    """The Years Between: the Waiting City's harbour at dawn, a tower with
    the company's floor lit, a clock stopped at 21:14, and the expedition's
    ship sailing north into colder light between the ice floes. The right
    side is cold sky and sea, under the page's sheet."""
    W, H = 32.0, 18.0
    rng = random.Random(2046)
    s = f"\\fill[icesky] (0,0) rectangle {P(W, H)};\n"
    # the sky warms toward the city on the left
    for k in range(12):
        x0 = k * 1.4
        s += f"\\fill[dawn, opacity={f(0.85 - k * 0.07)}] {P(x0, 0)} rectangle {P(x0 + 1.45, H)};\n"
    s += f"\\fill[skyhi, opacity=0.6] (0,12) rectangle (9,18);\n"
    for (cx, cy, cs) in [(3.0, 15.5, 1.3), (9.5, 16.4, 1.0)]:
        s += cloud(cx, cy, cs)
    s += aurora(rng, 18.0, 31.5, 12.0, 16, "auroragreen")
    s += aurora(rng, 20.0, 31.5, 13.4, 10, "aurorapink", amp=0.8)
    # the far ice to the north
    for (mx, mh, mw) in [(20.5, 4.8, 6.0), (25.0, 6.2, 7.0), (29.5, 4.4, 5.0)]:
        s += mountain(mx, 5.2, mw, mh)
    # the city: towers, the clock, the company's tower with one floor lit
    x = -0.4
    while x < 12.5:
        w, h = rng.uniform(1.2, 2.2), rng.uniform(3.5, 8.5)
        s += building(rng, x, 5.2, w, h, detail=True, lit_pct=35)
        x += w + 0.1
    s += clocktower(5.6, 5.2, 1.5, 7.5)
    s += rect(9.2, 5.2, 11.6, 16.2, "wallC")
    for j in range(13):
        fill = "lamp" if j == 10 else "windark"
        for i in range(3):
            s += f"\\filldraw[inked,line width=0.6pt,fill={fill}] {P(9.45 + i * 0.72, 5.8 + j * 0.78)} rectangle {P(9.85 + i * 0.72, 6.3 + j * 0.78)};\n"
    s += rect(9.05, 16.2, 11.75, 16.55, "wallE")
    s += line((10.4, 16.55), (10.4, 17.8), lw=1.2)
    # the harbour: the quay, the sea, floes, the ship going north
    s += rect(-0.5, 4.2, 13.5, 5.3, "stone")
    s += poly([(-0.5, -0.5), (W + 0.5, -0.5), (W + 0.5, 5.2), (13.5, 5.2), (13.5, 4.2), (-0.5, 4.2)], "water")
    for k in range(10):
        y = 0.6 + (k % 5) * 0.75
        x0 = rng.uniform(0, W - 2)
        s += f"\\draw[iceglow, line width=1.4pt, opacity=0.6] {P(x0, y)} -- {P(x0 + rng.uniform(0.8, 2.2), y)};\n"
    for (fx_, fy, fw) in [(15.0, 2.0, 1.6), (19.0, 3.4, 1.1), (23.5, 1.4, 2.2), (27.0, 3.8, 1.4), (29.5, 1.0, 1.8)]:
        s += floe(fx_, fy, fw)
    s += ship(17.2, 1.8, 1.25)
    s += f"\\draw[icewhite, line width=1.8pt] (12.8,2.4) .. controls (14.0,2.2) and (15.0,2.5) .. (13.8,2.3);\n"
    # the wake behind the ship
    for k in range(4):
        s += f"\\draw[icewhite, line width=1.2pt, opacity={f(0.8 - k * 0.18)}] {P(13.0 - k * 1.2, 2.25 - k * 0.12)} -- {P(13.9 - k * 1.2, 2.3 - k * 0.12)};\n"
    return s, (W, H)


def testimony():
    """The Witness steps down: the hollow in the ice under the aurora, the
    Witness lowering its arms, a pillar gone clear with a sleeper in a red
    coat inside, and the Defendant holding up a key on its string. The
    right side is the ice wall, under the page's sheet."""
    W, H = 32.0, 18.0
    rng = random.Random(1439)
    s = ice_sky(W, H)
    s += aurora(rng, 0.5, 31.5, 10.5, 28, "auroragreen", amp=1.5)
    s += aurora(rng, 3.0, 26.0, 12.5, 18, "aurorapink", amp=1.0)
    for (mx, mh, mw) in [(3.0, 5.5, 7.0), (9.0, 7.0, 8.0), (15.0, 5.0, 6.0)]:
        s += mountain(mx, 4.0, mw, mh, rock="icerockdark")
    # the ice wall of the hollow on the right
    s += poly([(16.5, 0), (W + 0.5, 0), (W + 0.5, H), (20.5, H), (19.4, 13.5), (18.0, 11.0), (18.8, 7.5), (16.9, 4.2)], "icedeep")
    for k in range(6):
        x0 = 20.5 + k * 1.9
        s += poly([(x0, 0), (x0 + 0.8, 0), (x0 + 0.5, H), (x0 + 0.2, H)], "iceblue")
        s += f"\\fill[iceglow, opacity=0.35] {P(x0 + 0.3, 2)} rectangle {P(x0 + 0.45, 14)};\n"
    # the frozen lake
    s += poly([(-0.5, 0), (W + 0.5, 0), (W + 0.5, 4.0), (-0.5, 4.0)], "water")
    s += f"\\draw[iceglow, line width=2pt, opacity=0.7] (8,2.0) ellipse (8.5 and 1.0);\n"
    for k in range(8):
        a = rng.uniform(0, math.pi)
        x0, y0 = 8 + rng.uniform(-6, 6), rng.uniform(0.5, 3.5)
        s += f"\\draw[icewhite, line width=0.9pt] {P(x0, y0)} -- {P(x0 + math.cos(a) * 1.4, y0 + math.sin(a) * 0.3)} -- {P(x0 + math.cos(a) * 2.2, y0 + math.sin(a) * 0.15 - 0.2)};\n"
    # the Witness, its arms lowered
    s += witness(12.2, 3.6, 2.0)
    # the pillar gone clear, the sleeper in her red coat inside
    s += f"\\filldraw[inked,fill=witnessice,fill opacity=0.85] (3.2,3.4) -- (5.6,3.4) -- (5.9,11.0) -- (4.4,12.2) -- (2.9,11.0) -- cycle;\n"
    s += poly([(3.9, 4.4), (4.9, 4.4), (4.75, 8.4), (4.05, 8.4)], "icered", ",fill opacity=0.85")
    s += circle(4.4, 9.0, 0.42, "skin", ",fill opacity=0.85")
    s += f"\\filldraw[inked,fill=hair,fill opacity=0.85] (3.98,9.1) arc (180:0:0.42) -- cycle;\n"
    s += f"\\draw[goldtrim, line width=1pt] (4.15,8.4) -- (4.4,7.6) -- (4.65,8.4);\n"
    s += f"\\fill[iceglow, opacity=0.4] (3.3,4.0) -- (3.6,4.0) -- (3.9,11.0) -- (3.6,11.0) -- cycle;\n"
    # the Defendant, holding up the key
    ds, dx = 1.9, 7.4
    s += defendant(dx, 3.4, ds, gun=False)
    hx, hy = dx + 0.6 * ds, 3.4 + 2.3 * ds
    s += line((dx + 0.25 * ds, 3.4 + 1.5 * ds), (hx, hy), lw=4.0, colour="coat")
    s += f"\\draw[ink, line width=0.8pt] {P(hx, hy)} -- {P(hx, hy - 0.6)};\n"
    s += f"\\filldraw[inked,fill=goldtrim] {P(hx, hy - 0.85)} circle (0.22);\n"
    s += f"\\filldraw[inked,fill=goldtrim] {P(hx - 0.06, hy - 1.05)} rectangle {P(hx + 0.06, hy - 1.7)};\n"
    s += f"\\filldraw[inked,fill=goldtrim] {P(hx + 0.06, hy - 1.6)} rectangle {P(hx + 0.24, hy - 1.5)};\n"
    s += f"\\fill[gold, opacity=0.3] {P(hx, hy - 1.2)} circle (0.8);\n"
    # the last orbs hanging, a held ring breaking up
    for k in range(9):
        a = 2 * math.pi * k / 9
        c = "cobalt" if k % 2 == 0 else "ember"
        s += orb(12.2 + math.cos(a) * 4.2, 9.5 + math.sin(a) * 1.2, 0.26, c, rim=(c == "cobalt"))
    return s, (W, H)


# ---- zone three ---------------------------------------------------------------

def agent(x, y, s, umbrella=False):
    """One of the Continuance's men: a charcoal suit, a white shirt, a black
    tie, dark glasses, the same face as every other. (x, y) between the
    feet; s scales a 2.0-tall figure."""
    def T(px, py):
        return (x + px * s, y + py * s)
    out = ""
    for lx in (-0.14, 0.14):
        out += poly([T(lx - 0.09, 0), T(lx + 0.09, 0), T(lx + 0.07, 0.95), T(lx - 0.07, 0.95)], "suitdark")
        out += poly([T(lx - 0.11, -0.02), T(lx + 0.15, -0.02), T(lx + 0.15, 0.07), T(lx - 0.11, 0.07)], "shades")
    out += poly([T(-0.3, 0.85), T(0.3, 0.85), T(0.32, 1.62), T(-0.32, 1.62)], "suit")
    out += poly([T(-0.1, 1.62), T(0.1, 1.62), T(0, 1.3)], "shirt")
    out += poly([T(-0.03, 1.58), T(0.03, 1.58), T(0.05, 1.12), T(0, 1.05), T(-0.05, 1.12)], "shades")
    for sx in (-1, 1):
        out += poly([T(sx * 0.3, 1.6), T(sx * 0.42, 1.55), T(sx * 0.38, 0.95), T(sx * 0.28, 0.98)], "suit")
    hx, hy = T(0, 1.82)
    out += circle(hx, hy, 0.19 * s, "agentskin")
    out += f"\\filldraw[inked,fill=shades] {P(hx - 0.2 * s, hy + 0.02 * s)} arc (180:0:{f(0.2 * s)}) -- cycle;\n"
    out += rect(hx - 0.16 * s, hy - 0.04 * s, hx + 0.16 * s, hy + 0.04 * s, "shades")
    if umbrella:
        ux, uy = T(0.38, 1.0)
        out += line((ux, uy), (ux, uy + 1.5 * s), lw=1.2)
        out += f"\\filldraw[inked,fill=shades] {P(ux - 0.75 * s, uy + 1.45 * s)} arc (180:0:{f(0.75 * s)}) -- cycle;\n"
    return out


def rain(rng, W, H, n=120):
    s = ""
    for _ in range(n):
        x, y = rng.uniform(0, W), rng.uniform(0, H)
        s += f"\\draw[moon, line width=0.6pt, opacity=0.55] {P(x, y)} -- {P(x - 0.25, y - 0.9)};\n"
    return s


def mansion(rng, x0, y0, w, h):
    """The house on Calder Hill: a long front of lit windows, a roof of
    gables and chimneys, the glass conservatory on its right."""
    s = rect(x0, y0, x0 + w, y0 + h, "marbledark")
    s += rect(x0 - 0.2, y0 + h, x0 + w + 0.2, y0 + h + 0.35, "hwooddark")
    for k in range(5):
        gx = x0 + w * (k + 0.5) / 5
        s += poly([(gx - 1.3, y0 + h + 0.35), (gx + 1.3, y0 + h + 0.35), (gx, y0 + h + 2.1)], "hwooddark")
        s += rect(gx + 0.5, y0 + h + 0.9, gx + 0.85, y0 + h + 2.4, "marbledark")
    for j in range(3):
        for i in range(int(w / 1.1)):
            wx = x0 + 0.35 + i * 1.1
            wy = y0 + 0.5 + j * h / 3.2
            lit = rng.random() < 0.7
            fill = "candle" if lit else "housenight"
            s += f"\\filldraw[inked,line width=0.7pt,fill={fill}] {P(wx, wy)} rectangle {P(wx + 0.55, wy + h / 4.6)};\n"
            s += f"\\draw[inked,line width=0.5pt] {P(wx + 0.275, wy)} -- {P(wx + 0.275, wy + h / 4.6)};\n"
            if lit and rng.random() < 0.35:
                s += f"\\fill[labfluid, opacity=0.5] {P(wx + 0.05, wy + 0.05)} rectangle {P(wx + 0.5, wy + h / 4.6 - 0.05)};\n"
    # the door, lit
    dx = x0 + w * 0.45
    s += f"\\filldraw[inked,fill=candle] {P(dx, y0)} -- {P(dx, y0 + 1.4)} arc (180:0:0.45) -- {P(dx + 0.9, y0)} -- cycle;\n"
    # the conservatory: a glass wing with ribs
    cx = x0 + w
    s += f"\\filldraw[inked,fill=glass,fill opacity=0.55] {P(cx, y0)} rectangle {P(cx + 3.6, y0 + h * 0.75)};\n"
    s += f"\\filldraw[inked,fill=glass,fill opacity=0.55] {P(cx, y0 + h * 0.75)} -- {P(cx + 1.8, y0 + h * 1.05)} -- {P(cx + 3.6, y0 + h * 0.75)} -- cycle;\n"
    for k in range(7):
        rx = cx + k * 0.6
        s += f"\\draw[ink, line width=0.7pt] {P(rx, y0)} -- {P(rx, y0 + h * 0.75)};\n"
    s += f"\\fill[candle, opacity=0.25] {P(cx + 0.1, y0 + 0.1)} rectangle {P(cx + 3.5, y0 + h * 0.7)};\n"
    return s


def interlude_house():
    """What the Witness saw: the house on Calder Hill in the rain at
    night, ten years of laboratories lit in its windows, and up the lawn,
    in a line, the grey men with one face, under black umbrellas. The right
    side is lawn and rain, under the page's sheet."""
    W, H = 32.0, 18.0
    rng = random.Random(2056)
    s = f"\\fill[housesky] (0,0) rectangle {P(W, H)};\n"
    s += f"\\fill[housenight] (0,{f(H * 0.55)}) rectangle {P(W, H * 0.8)};\n"
    s += f"\\filldraw[inked,fill=moon] (27.5,15.6) circle (1.1);\n"
    s += f"\\fill[moon, opacity=0.18] (27.5,15.6) circle (2.4);\n"
    for (cx, cy, cs) in [(6, 16.2, 1.4), (15, 16.8, 1.1), (22, 15.8, 1.2)]:
        s += cloud(cx, cy, cs, fill="housenight")
    # the hill and the house on it
    s += f"\\filldraw[inked,fill=grassnight] (-0.5,0) -- (-0.5,6.8) .. controls (6,8.2) and (14,8.0) .. (20,6.4) .. controls (25,5.2) and (29,4.8) .. (32.5,4.6) -- (32.5,0) -- cycle;\n"
    s += mansion(rng, 2.0, 7.4, 10.0, 4.6)
    for k in range(6):
        tx = 0.6 + k * 2.6 + rng.uniform(-0.4, 0.4)
        if 1.5 < tx < 16.5:
            continue
        s += f"\\filldraw[inked,fill=hedge] {P(tx, 6.9)} circle (0.9);\n"
    # a path down the lawn, and the grey men coming up it in a line
    s += f"\\fill[marbledark, opacity=0.5] (6.5,7.4) -- (7.6,7.4) -- (16,0) -- (12.5,0) -- cycle;\n"
    for k in range(7):
        t = k / 6
        ax = 8.2 + t * 7.0
        ay = 6.4 - t * 6.0
        s += agent(ax, ay, 0.85 + t * 0.9, umbrella=True)
    # one scientist's lab coat on the steps, fallen
    s += poly([(6.8, 7.45), (7.7, 7.45), (7.9, 7.65), (6.6, 7.7)], "labwhite")
    s += rain(rng, W, H, 160)
    return s, (W, H)


def executor(x, y, s, eyes=True, closed=True):
    """The Executor seen side on, muzzle to the left: two treads, a hull, a
    turret with its long gun, ember weak points on the turret's back, the
    cannons it grew and (eyes) the eyes of its last phase, shut."""
    def T(px, py):
        return (x + px * s, y + py * s)
    out = ""
    out += f"\\filldraw[inked,fill=tanktread,rounded corners=4pt] {P(*T(-4.2, 0))} rectangle {P(*T(4.2, 1.3))};\n"
    for k in range(7):
        out += circle(*T(-3.6 + k * 1.2, 0.65), 0.45 * s, "tankplate")
    out += poly([T(-4.5, 1.3), T(4.3, 1.3), T(4.3, 2.4), T(-3.4, 2.4)], "tankhull")
    out += poly([T(-2.0, 2.4), T(2.2, 2.4), T(2.6, 3.6), T(-1.6, 3.6)], "tankplate")
    out += rect(*T(-8.0, 2.85), *T(-1.9, 3.2), "tanktread")
    out += rect(*T(-8.4, 2.75), *T(-7.7, 3.3), "tanktread")
    for (wx, wy) in ((2.3, 2.9), (2.4, 3.3)):
        out += f"\\fill[tankweak, opacity=0.35] {P(*T(wx, wy))} circle ({f(0.45 * s)});\n"
        out += circle(*T(wx, wy), 0.2 * s, "tankweak")
    # the cannons it grew, out of the hull and up the turret like wings
    for k in range(6):
        a = math.radians(40 + k * 18)
        bx, by = T(0.6, 3.5)
        tx, ty = bx + math.cos(a) * 3.2 * s, by + math.sin(a) * 3.2 * s
        out += line((bx, by), (tx, ty), lw=3.0 * s, colour="tanktread")
        out += circle(tx, ty, 0.25 * s, "halo")
        if eyes:
            out += f"\\filldraw[inked,fill=tankeye] {P(tx, ty + 0.5 * s)} ellipse ({f(0.32 * s)} and {f((0.05 if closed else 0.2) * s)});\n"
    for k in range(4):
        a = math.radians(160 - k * 22)
        bx, by = T(-0.4, 3.4)
        tx, ty = bx + math.cos(a) * 2.6 * s, by + math.sin(a) * 2.6 * s
        out += line((bx, by), (tx, ty), lw=2.6 * s, colour="tanktread")
    if eyes:
        for k in range(5):
            ex, ey = T(-3.0 + k * 1.5, 1.85)
            out += f"\\filldraw[inked,fill=tankeye] {P(ex, ey)} ellipse ({f(0.34 * s)} and {f((0.05 if closed else 0.22) * s)});\n"
        out += f"\\draw[halo, line width={f(2.2 * s)}pt] {P(*T(0.3, 7.6))} ellipse ({f(1.8 * s)} and {f(0.45 * s)});\n"
    return out


def orders():
    """The Executor's orders: the conservatory wrecked, its glass on the
    floor, rain through the broken roof, the Executor settled with its eyes
    shut, and the Defendant reading the typed sheet signed in their own
    hand. The right side is the house's wall, under the page's sheet."""
    W, H = 32.0, 18.0
    rng = random.Random(2081)
    s = f"\\fill[housesky] (0,0) rectangle {P(W, H)};\n"
    # the glass wall's iron frame, its panes gone, the lawn beyond
    s += f"\\fill[grassnight] (0,0) rectangle (32,6.5);\n"
    for k in range(10):
        s += f"\\filldraw[inked,fill=hedge] {P(rng.uniform(0, 30), rng.uniform(5.5, 7.0))} circle ({f(rng.uniform(0.6, 1.3))});\n"
    for k in range(9):
        x0 = k * 2.4
        s += f"\\draw[tanktread, line width=2.2pt] {P(x0, 0)} -- {P(x0, 16)};\n"
        if rng.random() < 0.45:
            # a broken pane's teeth hanging from the frame
            s += f"\\filldraw[inked,fill=glass,fill opacity=0.5,line width=0.6pt] {P(x0, 12)} -- {P(x0 + 2.4, 12)} -- {P(x0 + 1.9, 11.2)} -- {P(x0 + 1.3, 11.7)} -- {P(x0 + 0.7, 10.9)} -- cycle;\n"
    for y in (4.0, 8.0, 12.0, 16.0):
        s += f"\\draw[tanktread, line width=2.2pt] (0,{f(y)}) -- (21,{f(y)});\n"
    # the floor: marble, shards of glass across it
    s += poly([(-0.5, -0.5), (W + 0.5, -0.5), (W + 0.5, 3.0), (-0.5, 3.0)], "marble")
    for i in range(16):
        s += f"\\fill[marbledark] {P(i * 2.0, 0)} rectangle {P(i * 2.0 + 1.0, 1.5)};\n"
        s += f"\\fill[marbledark] {P(i * 2.0 + 1.0, 1.5)} rectangle {P(i * 2.0 + 2.0, 3.0)};\n"
    for _ in range(40):
        x, y = rng.uniform(0, 22), rng.uniform(0.2, 2.8)
        a = rng.uniform(0, math.pi)
        r = rng.uniform(0.15, 0.45)
        s += poly([(x, y), (x + math.cos(a) * r, y + math.sin(a) * r * 0.4), (x + math.cos(a + 1.9) * r, y + 0.08)], "glass", ",fill opacity=0.8,line width=0.5pt")
    s += executor(8.8, 2.0, 1.1)
    # the Defendant, reading
    ds, dx = 1.8, 15.6
    s += defendant(dx, 1.2, ds, gun=False)
    hx, hy = dx + 0.45 * ds, 1.2 + 1.45 * ds
    s += line((dx + 0.25 * ds, 1.2 + 1.5 * ds), (hx, hy), lw=4.0, colour="coat")
    s += f"\\filldraw[inked,fill=cream,rotate around={{-12:{P(hx, hy)}}}] {P(hx - 0.1, hy - 0.2)} rectangle {P(hx + 1.3, hy + 1.5)};\n"
    for k in range(5):
        y0 = hy + 1.25 - k * 0.22
        s += f"\\draw[ink, line width=0.6pt, rotate around={{-12:{P(hx, hy)}}}] {P(hx + 0.1, y0)} -- {P(hx + 1.1, y0)};\n"
    s += f"\\draw[ink, line width=1.0pt, rotate around={{-12:{P(hx, hy)}}}] {P(hx + 0.15, hy + 0.05)} .. controls {P(hx + 0.4, hy + 0.35)} and {P(hx + 0.6, hy - 0.1)} .. {P(hx + 1.0, hy + 0.15)};\n"
    s += rain(rng, 21.0, H, 110)
    return s, (W, H)


def door():
    """The Door: the summit's circle of stones running like wax, the sky
    torn open over it and the Idiot God pouring through, a mass of violet
    bubbles and eyes round a black maw, and the Defendant small against
    it, drawn up toward the mouth. The right side is the torn sky, under
    the page's sheet."""
    W, H = 32.0, 18.0
    rng = random.Random(2081)
    s = f"\\fill[dusk] (0,0) rectangle {P(W, H)};\n"
    s += f"\\fill[duskhi] (0,{f(H * 0.35)}) rectangle {P(W, H * 0.55)};\n"
    s += f"\\fill[dusklow] (0,0) rectangle {P(W, H * 0.3)};\n"
    # the tear in the sky, and what is through it
    s += f"\\filldraw[inked,fill=void] (-1,17.8) .. controls (3,12) and (15,11) .. (21,17.8) -- cycle;\n"
    for k in range(34):
        a = rng.uniform(0.2, math.pi - 0.2)
        r = rng.uniform(2.0, 9.0)
        x, y = 10 + math.cos(a) * r, 18.5 - math.sin(a) * r * 0.55
        b = rng.uniform(0.6, 1.6)
        s += circle(x, y, b, "flesh" if k % 3 else "fleshdark")
        if k % 2 == 0:
            s += f"\\filldraw[inked,fill=chalk] {P(x, y)} ellipse ({f(b * 0.45)} and {f(b * 0.28)});\n"
            s += circle(x, y, b * 0.16, "eldglow")
            s += circle(x, y, b * 0.07, "void")
    # the maw, a ring of teeth round the dark
    s += circle(10, 14.4, 2.4, "void")
    for k in range(18):
        a = 2 * math.pi * k / 18
        x0, y0 = 10 + math.cos(a) * 2.4, 14.4 + math.sin(a) * 2.4
        x1, y1 = 10 + math.cos(a) * 1.5, 14.4 + math.sin(a) * 1.5
        s += poly([(x0 - 0.18, y0), (x0 + 0.18, y0), (x1, y1)], "chalk", ",line width=0.6pt")
    s += f"\\fill[eldglow, opacity=0.07] (10,14.4) circle (5.5);\n"
    # the mountain's summit, melting: stones that sag and run
    s += f"\\filldraw[inked,fill=arock] (-0.5,0) -- (-0.5,4.2) .. controls (6,6.2) and (12,6.6) .. (18,5.8) .. controls (24,5.0) and (28,4.2) .. (32.5,3.8) -- (32.5,0) -- cycle;\n"
    for k in range(9):
        x = 0.5 + k * 1.9
        h = rng.uniform(1.6, 2.6)
        lean = rng.uniform(-0.5, 0.5)
        top = 5.4 + rng.uniform(-0.4, 0.4)
        s += f"\\filldraw[inked,fill=astone] {P(x, top - 0.5)} -- {P(x + 0.8, top - 0.5)} .. controls {P(x + 0.9 + lean, top + h * 0.6)} and {P(x + 0.7 + lean, top + h)} .. {P(x + 0.4 + lean, top + h)} .. controls {P(x + lean, top + h)} and {P(x - 0.1, top + h * 0.6)} .. cycle;\n"
        for d in range(2):
            dx = x + 0.2 + d * 0.4
            s += f"\\filldraw[inked,fill=astonedark,line width=0.6pt] {P(dx, top - 0.5)} .. controls {P(dx + 0.05, top - 1.4)} and {P(dx + 0.15, top - 1.6)} .. {P(dx + 0.1, top - 1.9)} .. controls {P(dx - 0.05, top - 1.6)} and {P(dx - 0.1, top - 1.2)} .. cycle;\n"
    # the Long Door, still lit, and the Defendant drawn up toward the maw
    s += rect(4.6, 5.8, 4.9, 8.6, "astonedark") + rect(6.1, 5.8, 6.4, 8.6, "astonedark") + rect(4.5, 8.6, 6.5, 8.95, "astone")
    s += f"\\fill[doorlight, opacity=0.8] (4.9,5.8) rectangle (6.1,8.6);\n"
    s += defendant(8.4, 9.2, 1.2, lean=0.25, gun=False, falling=True)
    for k in range(6):
        s += f"\\draw[eldglow, line width=0.8pt, opacity=0.6] {P(8.8 + k * 0.1, 11.5 + k * 0.3)} -- {P(9.6 + k * 0.1, 13.2 + k * 0.1)};\n"
    return s, (W, H)

def lullaby():
    """The Lullaby: inside the Idiot God, its heart. The god asleep in a
    bubble of stopped time, grey within it, every eye shut, the orbs it
    last fired hanging still; its dancers circling outside, slow; the
    Defendant at the edge of the heart's floor playing the trumpet toward
    it, notes drifting from the bell. The right side is under the page's
    sheet."""
    W, H = 32.0, 18.0
    rng = random.Random(9999)
    s = f"\\fill[godfog] (0,0) rectangle {P(W, H)};\n"
    s += f"\\fill[godsky] (0,{f(H * 0.4)}) rectangle {P(W, H)};\n"
    # the god's insides, curving round: walls of flesh above and below, and
    # its shut eyes far off
    s += f"\\filldraw[inked,fill=godflesh] (-1,18.5) .. controls (6,14.5) and (22,14.2) .. (33,18.5) -- cycle;\n"
    s += f"\\filldraw[inked,fill=flesh] (-1,-0.5) .. controls (8,4.2) and (24,4.0) .. (33,-0.5) -- cycle;\n"
    for _ in range(26):
        x, y = rng.uniform(0, W), rng.choice([rng.uniform(15.6, 17.8), rng.uniform(0.2, 2.2)])
        r = rng.uniform(0.25, 0.6)
        s += f"\\filldraw[inked,fill=godfleshpale,line width=0.6pt] {P(x, y)} ellipse ({f(r)} and {f(r * 0.45)});\n"
        s += f"\\draw[ink, line width=0.7pt] {P(x - r * 0.8, y)} .. controls {P(x, y - r * 0.35)} .. {P(x + r * 0.8, y)};\n"
    # the dancers, far round the god, slow, piping
    for k in range(7):
        a = 2 * math.pi * k / 7 + 0.4
        x, y = 12.0 + math.cos(a) * 9.5, 10.0 + math.sin(a) * 4.6
        if x > 21.5:
            continue
        b = 0.55 + 0.15 * math.sin(a)
        s += f"\\filldraw[inked,fill=dancer] {P(x, y)} .. controls {P(x - b, y + b * 1.2)} and {P(x + b * 0.4, y + b * 2.2)} .. {P(x + b * 0.3, y + b * 1.1)} .. controls {P(x + b, y + b * 0.8)} and {P(x + b * 0.8, y - b * 0.4)} .. cycle;\n"
        s += line((x + b * 0.2, y + b * 0.9), (x + b * 1.6, y + b * 0.4), lw=1.6, colour="godstonedark")
    # the bubble of stopped time: grey inside, the god asleep in it
    cx, cy, R = 14.0, 10.6, 5.6
    s += f"\\fill[stilled, opacity=0.55] {P(cx, cy)} circle ({f(R)});\n"
    for k in range(30):
        a = rng.uniform(0, 2 * math.pi)
        r = rng.uniform(0.0, 3.6)
        x, y = cx + math.cos(a) * r, cy + math.sin(a) * r * 0.8
        b = rng.uniform(0.7, 1.5)
        s += circle(x, y, b, "godstone" if k % 3 else "godstonedark")
        if k % 2 == 0:
            # an eye, shut
            s += f"\\filldraw[inked,fill=stilled,line width=0.7pt] {P(x, y)} ellipse ({f(b * 0.45)} and {f(b * 0.16)});\n"
            s += f"\\draw[ink, line width=0.9pt] {P(x - b * 0.45, y)} .. controls {P(x, y - b * 0.2)} .. {P(x + b * 0.45, y)};\n"
    # its maw, shut
    s += f"\\draw[ink, line width=1.6pt] {P(cx - 1.6, cy - 1.2)} .. controls {P(cx, cy - 1.7)} .. {P(cx + 1.6, cy - 1.2)};\n"
    # what it fired last, hanging where it was
    for _ in range(14):
        a = rng.uniform(0, 2 * math.pi)
        r = rng.uniform(4.2, 5.4)
        s += orb(cx + math.cos(a) * r, cy + math.sin(a) * r, 0.22, "stilled", glow=False)
    s += f"\\draw[chalk, line width=2.4pt] {P(cx, cy)} circle ({f(R)});\n"
    s += f"\\draw[ink, line width=1.1pt] {P(cx, cy)} circle ({f(R + 0.12)});\n"
    s += f"\\fill[white, opacity=0.35] {P(cx - R * 0.45, cy + R * 0.55)} ellipse (1.4 and 0.5);\n"
    # the heart's floor, a disc of dream-stone, and the Defendant at its edge
    s += f"\\filldraw[inked,fill=godstone] (-0.5,3.6) .. controls (4,4.6) and (9,4.4) .. (12.5,3.2) -- (12.5,2.4) .. controls (9,3.4) and (4,3.6) .. (-0.5,2.6) -- cycle;\n"
    x0, y0, sc = 4.4, 4.2, 1.6
    s += defendant(x0, y0, sc, lean=-0.06, gun=False)
    # the trumpet at the lips, its bell toward the god
    mx, my = x0 + 0.22 * sc, y0 + 1.8 * sc
    bx, by = mx + 2.4, my + 0.7
    s += line((mx, my), (bx - 0.5, by - 0.12), lw=3.4, colour="trumpetdark")
    s += line((mx, my), (bx - 0.5, by - 0.12), lw=2.0, colour="trumpet")
    s += poly([(bx - 0.7, by - 0.06), (bx + 0.3, by + 0.55), (bx + 0.45, by - 0.5)], "trumpet")
    for k in range(3):
        vx = mx + 0.6 + k * 0.22
        vy = my + 0.18 + k * 0.05
        s += rect(vx - 0.05, vy, vx + 0.05, vy + 0.28, "trumpetdark", lw=0.6)
    s += line((x0 + 0.25 * sc, y0 + 1.45 * sc), (mx + 0.7, my + 0.1), lw=3.0, colour="coat")
    # the notes, drifting toward the bubble
    for k in range(5):
        nx = bx + 0.7 + k * 0.62
        ny = by + 0.5 + math.sin(k * 1.3) * 0.5 + k * 0.35
        s += f"\\filldraw[inked,fill=seam,line width=0.7pt,rotate around={{-20:{P(nx, ny)}}}] {P(nx, ny)} ellipse (0.2 and 0.14);\n"
        s += line((nx + 0.18, ny + 0.02), (nx + 0.18, ny + 0.75), lw=1.0)
        if k % 2 == 0:
            s += line((nx + 0.18, ny + 0.75), (nx + 0.45, ny + 0.55), lw=1.0)
    return s, (W, H)


# ---- the last mission ---------------------------------------------------------

def smooth(points, fill, extra="", ink=True):
    """A closed curve through the points, inked (or only filled)."""
    pts = " ".join(P(x, y) for x, y in points)
    if ink:
        return f"\\filldraw[inked,fill={fill}{extra}] plot[smooth cycle, tension=0.7] coordinates {{{pts}}};\n"
    return f"\\fill[{fill}{extra}] plot[smooth cycle, tension=0.7] coordinates {{{pts}}};\n"


def fluff(cx, cy, rx, ry, n, depth, a0=0.0, a1=360.0, shape=None):
    """Points round an ellipse with a long-haired edge: every other point
    pushed out by depth (a fraction of the radius). shape(angle) scales the
    radius by direction, for cheeks that flare."""
    pts = []
    for k in range(n + 1 if a1 - a0 < 360 else n):
        a = math.radians(a0 + (a1 - a0) * k / n)
        m = (1 + depth) if k % 2 == 0 else 1.0
        if shape:
            m *= shape(a)
        pts.append((cx + math.cos(a) * rx * m, cy + math.sin(a) * ry * m))
    return pts


def chumbles_face(x, y, s, asleep=False):
    """Chumbles' head, face on: grey-blue ears with pink insides, dark tips
    and white tufts; a fluffy head, the darker lynx-point mask round big
    blue eyes (or shut, asleep), tabby lines on his forehead, a pink nose,
    a white muzzle and chin, white whiskers. (x, y) is the middle of his
    face; s scales a head 2.5 wide."""
    def T(px, py):
        return (x + px * s, y + py * s)

    def Ts(points):
        return [T(px, py) for px, py in points]

    def lerp(a, b, t):
        return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)

    k = s / 3.0  # line widths follow the size
    out = ""
    # the ears, behind the head
    for side in (-1, 1):
        b0, b1, tip = (side * 1.3, 0.2), (side * 0.18, 0.95), (side * 1.08, 1.86)
        out += poly(Ts([b0, b1, tip]), "catpoint", f",line width={f(1.6 * k)}pt")
        out += poly(Ts([lerp(b0, tip, 0.68), tip, lerp(b1, tip, 0.68)]), "catpointdark", f",line width={f(1.2 * k)}pt")
        mid = ((b0[0] + b1[0]) / 2, (b0[1] + b1[1]) / 2 + 0.05)
        out += poly(Ts([lerp(mid, b0, 0.55), lerp(mid, tip, 0.62), lerp(mid, b1, 0.55)]), "catear", f",line width={f(0.9 * k)}pt")
        # white tufts out of the ear, and the lynx tip
        for j in range(4):
            ox = mid[0] + side * (0.12 * j - 0.18)
            out += f"\\draw[catwhite, line width={f(1.6 * k)}pt, line cap=round] {P(*T(ox, mid[1] - 0.05))} .. controls {P(*T(ox + side * 0.05, mid[1] + 0.35))} .. {P(*T(ox - side * (0.1 - 0.05 * j), mid[1] + 0.55 + 0.08 * j))};\n"
        out += poly(Ts([(tip[0] - 0.06, tip[1] - 0.05), (tip[0] + side * 0.02, tip[1] + 0.14), (tip[0] + 0.06, tip[1] - 0.05)]), "catpointdark", f",line width={f(0.8 * k)}pt")
    # the head: fluffy, its cheeks flaring at the bottom
    head = fluff(0, 0.05, 1.28, 1.05, 44, 0.05,
                 shape=lambda a: 1 + 0.22 * max(0.0, -math.sin(a)) * abs(math.cos(a)) ** 0.6 + (0.06 if math.sin(a) < 0.2 else 0))
    out += poly(Ts(head), "catpointlight", f",line width={f(1.6 * k)}pt")
    # cream cheeks low on each side
    out += smooth(Ts([(0, -0.25), (0.6, -0.12), (1.1, -0.2), (1.38, -0.5), (1.0, -0.82), (0.4, -0.98), (0, -1.0), (-0.4, -0.98), (-1.0, -0.82), (-1.38, -0.5), (-1.1, -0.2), (-0.6, -0.12)]), "catcream", ink=False)
    # the mask: over the eyes, down the nose
    out += smooth(Ts([(0, 0.98), (0.4, 0.86), (0.85, 0.6), (1.06, 0.3), (0.9, -0.02), (0.48, -0.12), (0.24, -0.42), (-0.24, -0.42), (-0.48, -0.12), (-0.9, -0.02), (-1.06, 0.3), (-0.85, 0.6), (-0.4, 0.86)]), "catpoint", ink=False)
    # tabby lines on the forehead
    for j, dx in enumerate((-0.27, -0.09, 0.09, 0.27)):
        out += f"\\draw[cattabby, line width={f(2.4 * k)}pt, line cap=round] {P(*T(dx, 0.9 - 0.04 * abs(j - 1.5)))} .. controls {P(*T(dx + 0.02, 0.75))} .. {P(*T(dx * 0.7, 0.55))};\n"
    for side in (-1, 1):
        out += f"\\draw[cattabby, line width={f(2.0 * k)}pt, line cap=round] {P(*T(side * 0.55, 0.8))} .. controls {P(*T(side * 0.48, 0.68))} .. {P(*T(side * 0.5, 0.58))};\n"
    # the eyes, set in the darker patches of the mask
    for side in (-1, 1):
        ex, ey = T(side * 0.5, 0.26)
        tilt = side * 12
        pw, ph = (0.36, 0.22) if asleep else (0.42, 0.32)
        out += f"\\fill[catpointdark, opacity=0.85, rotate around={{{tilt}:{P(ex, ey)}}}] {P(ex, ey)} ellipse ({f(pw * s)} and {f(ph * s)});\n"
        out += f"\\fill[catpointdark, opacity=0.85, rotate around={{{tilt}:{P(ex, ey)}}}] {P(ex + side * 0.3 * s, ey + 0.02 * s)} ellipse ({f(0.25 * s)} and {f(0.1 * s)});\n"
        # the pale line under each eye, as his has
        if not asleep:
            out += f"\\draw[catwhite, line width={f(2.2 * k)}pt, line cap=round] {P(*T(side * 0.2, 0.0))} .. controls {P(*T(side * 0.5, -0.06))} .. {P(*T(side * 0.82, 0.14))};\n"
        if asleep:
            out += f"\\draw[ink, line width={f(2.4 * k)}pt, line cap=round] {P(*T(side * 0.22, 0.27))} .. controls {P(*T(side * 0.42, 0.12))} and {P(*T(side * 0.62, 0.14))} .. {P(*T(side * 0.8, 0.33))};\n"
        else:
            out += f"\\filldraw[inked,line width={f(1.8 * k)}pt,fill=cateye,rotate around={{{tilt}:{P(ex, ey)}}}] {P(ex, ey)} ellipse ({f(0.31 * s)} and {f(0.25 * s)});\n"
            out += f"\\fill[catpupil] {P(ex, ey)} ellipse ({f(0.075 * s)} and {f(0.2 * s)});\n"
            out += f"\\fill[white] {P(ex - 0.1 * s, ey + 0.09 * s)} circle ({f(0.065 * s)});\n"
            out += f"\\fill[white, opacity=0.6] {P(ex + 0.12 * s, ey - 0.1 * s)} circle ({f(0.03 * s)});\n"
    # the muzzle: white pads and chin, the pink nose, the mouth
    out += smooth(Ts([(0, -0.32), (0.28, -0.36), (0.45, -0.52), (0.36, -0.72), (0.12, -0.8), (0, -0.92), (-0.12, -0.8), (-0.36, -0.72), (-0.45, -0.52), (-0.28, -0.36)]), "catwhite", ink=False)
    out += poly(Ts([(-0.15, -0.26), (0.15, -0.26), (0.03, -0.42), (-0.03, -0.42)]), "catnose", f",line width={f(1.3 * k)}pt")
    out += f"\\draw[ink, line width={f(1.3 * k)}pt, line cap=round] {P(*T(0, -0.42))} -- {P(*T(0, -0.52))} .. controls {P(*T(-0.04, -0.6))} and {P(*T(-0.16, -0.62))} .. {P(*T(-0.22, -0.55))};\n"
    out += f"\\draw[ink, line width={f(1.3 * k)}pt, line cap=round] {P(*T(0, -0.52))} .. controls {P(*T(0.04, -0.6))} and {P(*T(0.16, -0.62))} .. {P(*T(0.22, -0.55))};\n"
    # white whiskers, each with a fine ink edge so they read on white
    for side in (-1, 1):
        for j in range(3):
            bx, by = T(side * 0.3, -0.5 - j * 0.07)
            ex, ey = T(side * (1.75 - j * 0.1), -0.35 - j * 0.28)
            mx, my = T(side * 1.1, -0.36 - j * 0.12)
            for col, lw in (("ink", 2.0), ("catwhite", 1.1)):
                out += f"\\draw[{col}, line width={f(lw * k)}pt, line cap=round] {P(bx, by)} .. controls {P(mx, my)} .. {P(ex, ey)};\n"
    return out


def chumbles_ruff(x, y, s):
    """The white ruff under his chin and down his chest."""
    pts = fluff(x, y - 0.95 * s, 1.55 * s, 1.25 * s, 30, 0.1)
    return poly(pts, "catwhite", f",line width={f(1.6 * s / 3)}pt")


def chumbles_tail(points, s):
    """A plumed grey tail along a path: one band, thick in the middle, its
    outer edge long-haired, darker at the tip."""
    n = len(points)
    outer, inner = [], []
    for i, (px, py) in enumerate(points):
        a = points[min(n - 1, i + 1)]
        b = points[max(0, i - 1)]
        tx, ty = a[0] - b[0], a[1] - b[1]
        d = math.hypot(tx, ty) or 1
        nx, ny = -ty / d, tx / d
        t = i / (n - 1)
        w = s * (0.1 + 0.27 * math.sin(math.pi * min(1.0, 0.04 + t * 0.92)))
        hair = 1.2 if i % 2 == 0 else 1.0
        outer.append((px + nx * w * hair, py + ny * w * hair))
        hair = 1.12 if i % 2 == 1 else 1.0
        inner.append((px - nx * w * hair, py - ny * w * hair))
    ex, ey = points[-1]
    tip = (ex + (points[-1][0] - points[-2][0]) * 0.8, ey + (points[-1][1] - points[-2][1]) * 0.8)
    band = outer + [tip] + inner[::-1]
    out = smooth(band, "catpoint", f",line width={f(1.4 * s / 3)}pt")
    # the darker tip
    m = int(n * 0.78)
    out += f"\\begin{{scope}}\\clip plot[smooth cycle, tension=0.7] coordinates {{{' '.join(P(*q) for q in band)}}};\n"
    out += smooth(outer[m:] + [tip] + inner[m:][::-1], "catpointdark", ink=False)
    out += "\\end{scope}\n"
    # a few long hairs of the plume, lighter
    for i in range(4, n - 4, 5):
        out += f"\\draw[catpointlight, line width={f(1.4 * s / 3)}pt, line cap=round] {P(*points[i])} .. controls {P(*points[i + 1])} .. {P(*outer[i + 2])};\n"
    return out


def chumbles_loaf(x, y, s):
    """Chumbles asleep in a loaf: a cream body, the back shading grey,
    white ruff, paws tucked, the plumed tail curled round the front, his
    head turned to the viewer with its eyes shut. (x, y) is where his
    chest meets the cushion; s scales his head."""
    def T(px, py):
        return (x + px * s, y + py * s)
    k = s / 3.0
    out = ""
    # the body, long behind him to the right, a loaf on the cushion
    body = fluff(1.35 * s + x, 0.95 * s + y, 2.85 * s, 1.15 * s, 48, 0.035,
                 shape=lambda a: 1.0 if math.sin(a) > -0.2 else 0.92)
    out += poly(body, "catcream", f",line width={f(1.6 * k)}pt")
    out += f"\\begin{{scope}}\\clip {' -- '.join(P(*p) for p in body)} -- cycle;\n"
    out += f"\\fill[catpointlight, opacity=0.75] {P(*T(1.6, 2.6))} ellipse ({f(2.6 * s)} and {f(0.8 * s)});\n"
    out += "\\end{scope}\n"
    # a hind leg's haunch, the fur lines of the flank
    out += f"\\draw[ink, line width={f(1.2 * k)}pt] {P(*T(2.6, 0.2))} .. controls {P(*T(2.3, 1.3))} and {P(*T(3.4, 1.6))} .. {P(*T(3.9, 0.9))};\n"
    for j in range(3):
        out += f"\\draw[catpointlight, line width={f(1.6 * k)}pt, line cap=round] {P(*T(1.0 + j * 0.6, 1.25 - j * 0.05))} -- {P(*T(1.25 + j * 0.6, 0.95 - j * 0.05))};\n"
    # the ruff and the paws tucked under it
    out += chumbles_ruff(*T(0, 2.3), s * 0.8)
    # the tail, round the front of him
    keys = [(4.1, 1.05), (3.75, 0.4), (2.2, 0.22), (0.4, 0.25), (-0.75, 0.48), (-1.1, 1.0)]
    path = []
    for j in range(len(keys) - 1):
        p0, p1, p2, p3 = keys[max(0, j - 1)], keys[j], keys[j + 1], keys[min(len(keys) - 1, j + 2)]
        for i in range(7):
            u = i / 7
            # Catmull-Rom through the keys
            path.append(T(*[0.5 * (2 * p1[c] + (-p0[c] + p2[c]) * u + (2 * p0[c] - 5 * p1[c] + 4 * p2[c] - p3[c]) * u * u + (-p0[c] + 3 * p1[c] - 3 * p2[c] + p3[c]) * u ** 3) for c in (0, 1)]))
    path.append(T(*keys[-1]))
    out += chumbles_tail(path, s)
    # his head, a little down and turned to us, eyes shut
    out += f"\\begin{{scope}}[rotate around={{-6:{P(*T(0, 2.45))}}}]\n" + chumbles_face(*T(0, 2.45), s * 0.95, asleep=True) + "\\end{scope}\n"
    return out


def summit_night(rng, W, H):
    """The summit under a night that never ends: the deep sky, stars, the
    clock hung in it at 21:14, and the circle of standing stones."""
    s = f"\\fill[night] (0,0) rectangle {P(W, H)};\n"
    s += f"\\fill[summitsky] (0,{f(H * 0.62)}) rectangle {P(W, H)};\n"
    s += f"\\fill[nightsky] (0,0) rectangle {P(W, H * 0.4)};\n"
    s += f"\\fill[dusklow, opacity=0.7] (0,{f(H * 0.3)}) rectangle {P(W, H * 0.4)};\n"
    for _ in range(90):
        sx, sy = rng.uniform(0, W), rng.uniform(H * 0.35, H)
        s += f"\\fill[chalk, opacity={f(rng.uniform(0.5, 1.0))}] {P(sx, sy)} circle ({f(rng.uniform(0.03, 0.09))});\n"
    s += f"\\fill[chalk, opacity=0.15] (3.8,14.0) circle (2.3);\n"
    s += clock(3.8, 14.0, 1.5)
    # the hill and the ring of stones on it
    s += f"\\filldraw[inked,fill=arock] (-0.5,0) -- (-0.5,5.4) .. controls (7,7.0) and (16,7.2) .. (24,6.0) .. controls (28,5.4) and (31,5.0) .. (32.5,4.8) -- (32.5,0) -- cycle;\n"
    stones = []
    for i in range(14):
        a = 2 * math.pi * i / 14 + 0.2
        stones.append((math.sin(a), a))
    for depth, a in sorted(stones, reverse=True):
        px = 11.5 + math.cos(a) * 10.5
        py = 4.6 + depth * 1.6
        sc = 1.0 - depth * 0.3
        w, h = 1.0 * sc, rng.uniform(2.2, 3.0) * sc
        lean = rng.uniform(-0.15, 0.15)
        s += f"\\filldraw[inked,fill=astone] {P(px - w / 2, py)} -- {P(px + w / 2, py)} -- {P(px + w / 2 + lean, py + h * 0.85)} .. controls {P(px + w / 2 + lean, py + h)} and {P(px - w / 2 + lean, py + h)} .. {P(px - w / 2 + lean, py + h * 0.85)} -- cycle;\n"
        s += f"\\fill[astonedark, opacity=0.7] {P(px + w * 0.15, py)} -- {P(px + w / 2, py)} -- {P(px + w / 2 + lean, py + h * 0.85)} -- {P(px + w * 0.15 + lean, py + h * 0.85)} -- cycle;\n"
    s += f"\\fill[night, opacity=0.35] (0,0) rectangle {P(W, H)};\n"
    return s


def waking():
    """Waking: the summit comes apart like a dropped plate. Its stones,
    stars and stopped clock break into shards, each a little out of place,
    with white light in the cracks; through the hole in the middle, a sunny
    bedroom ceiling and Chumbles looking down from very close, sitting on
    your chest. The right side is under the page's sheet."""
    W, H = 32.0, 18.0
    rng = random.Random(2115)
    cx, cy, rx, ry = 11.0, 9.2, 6.4, 7.6
    scene = summit_night(random.Random(2114), W, H)
    # the shards: rays out from the hole, broken into rings
    n = 11
    rings = [1.0, 1.55, 2.3, 6.0]
    angles = [2 * math.pi * i / n + rng.uniform(-0.18, 0.18) for i in range(n)]
    V = [[None] * len(rings) for _ in range(n)]
    for i, a in enumerate(angles):
        for j, r in enumerate(rings):
            aj = a + rng.uniform(-0.08, 0.08) * (j > 0)
            rj = r * (rng.uniform(0.9, 1.08) if j < 3 else 1)
            V[i][j] = (cx + math.cos(aj) * rx * rj, cy + math.sin(aj) * ry * rj)
    # a jagged point on each edge of the hole
    mids = []
    for i in range(n):
        a, b = V[i][0], V[(i + 1) % n][0]
        m = rng.uniform(0.35, 0.65)
        px, py = a[0] + (b[0] - a[0]) * m, a[1] + (b[1] - a[1]) * m
        pull = rng.uniform(-0.12, 0.12)
        mids.append((cx + (px - cx) * (1 + pull), cy + (py - cy) * (1 + pull)))
    hole = []
    for i in range(n):
        hole += [V[i][0], mids[i]]
    hole_path = " -- ".join(P(*p) for p in hole) + " -- cycle"
    s = f"\\fill[white] (0,0) rectangle {P(W, H)};\n"
    # the bedroom through the hole: a warm ceiling in the morning, a light,
    # and Chumbles
    s += f"\\begin{{scope}}\\clip {hole_path};\n"
    s += f"\\fill[ceiling] (0,0) rectangle {P(W, H)};\n"
    for k in range(3):
        x0 = 13.0 + k * 2.6
        s += f"\\fill[sunlight, opacity=0.55] {P(x0, 18)} -- {P(x0 + 1.4, 18)} -- {P(x0 - 6.0, 0)} -- {P(x0 - 7.8, 0)} -- cycle;\n"
    s += f"\\fill[white, opacity=0.5] (11.1,15.3) circle (1.6);\n"
    s += f"\\filldraw[inked,fill=hometrim] (11.1,15.3) ellipse (1.0 and 0.55);\n"
    s += f"\\filldraw[inked,fill=homewindow,line width=0.8pt] (11.1,15.3) ellipse (0.6 and 0.3);\n"
    # his shoulders, the ruff, and his face, very close
    fx, fy, fs = cx + 0.1, cy - 1.7, 3.4
    s += poly(fluff(fx, fy - 4.5, 7.0, 3.6, 40, 0.04), "catcream", ",line width=1.6pt")
    s += chumbles_ruff(fx, fy, fs)
    s += chumbles_face(fx, fy, fs)
    s += "\\end{scope}\n"
    # the light in the cracks
    s += f"\\draw[white, line width=6pt, line join=round, opacity=0.8] {hole_path};\n"
    # the shards, each pushed out from the hole and turned a little
    for i in range(n):
        for j in range(len(rings) - 1):
            i2 = (i + 1) % n
            if j == 0:
                pts = [V[i][0], mids[i], V[i2][0], V[i2][1], V[i][1]]
            else:
                pts = [V[i][j], V[i2][j], V[i2][j + 1], V[i][j + 1]]
            mx = sum(p[0] for p in pts) / len(pts)
            my = sum(p[1] for p in pts) / len(pts)
            dx, dy = (mx - cx) / rx, (my - cy) / ry
            d = math.hypot(dx, dy) or 1
            push = [rng.uniform(0.35, 0.6), rng.uniform(0.18, 0.3), rng.uniform(0.08, 0.14)][j]
            turn = rng.uniform(-1, 1) * [5.0, 2.5, 1.0][j]
            path = " -- ".join(P(*p) for p in pts) + " -- cycle"
            tf = f"shift={{({f(dx / d * push)},{f(dy / d * push)})}}, rotate around={{{f(turn)}:{P(mx, my)}}}"
            s += f"\\begin{{scope}}[{tf}]\n\\begin{{scope}}\\clip {path};\n{scene}\\end{{scope}}\n"
            s += f"\\draw[ink, line width=1.8pt, line join=round] {path};\n"
            s += f"\\draw[white, line width=0.8pt, opacity=0.5, line join=round] {path};\n\\end{{scope}}\n"
    return s, (W, H)


def chumbles():
    """The ending: Chumbles asleep in a loaf in the sun on the blue-grey
    sofa, eyes shut, tail curled round, the treat bag empty on its side
    and treats left by it; morning light through the window, and very far
    off through it a clock tower reading 21:15. The right side is the
    room, under the page's sheet."""
    W, H = 32.0, 18.0
    rng = random.Random(2215)
    s = f"\\fill[homewall] (0,0) rectangle {P(W, H)};\n"
    # the window, and the town very far off through it
    wx0, wy0, wx1, wy1 = 2.2, 9.6, 12.4, 17.0
    s += rect(wx0 - 0.35, wy0 - 0.35, wx1 + 0.35, wy1 + 0.35, "hometrim", lw=1.4)
    s += f"\\begin{{scope}}\\clip {P(wx0, wy0)} rectangle {P(wx1, wy1)};\n"
    s += f"\\fill[morningsky] {P(wx0, wy0)} rectangle {P(wx1, wy1)};\n"
    s += f"\\fill[homewindow] {P(wx0, wy0)} rectangle {P(wx1, wy0 + 2.6)};\n"
    s += cloud(5.0, 15.6, 0.55, fill="hometrim")
    s += cloud(10.6, 16.2, 0.4, fill="hometrim")
    x = wx0 - 0.3
    while x < wx1:
        w, h = rng.uniform(0.35, 0.7), rng.uniform(0.5, 1.4)
        s += f"\\filldraw[ink,line width=0.5pt,fill={rng.choice(WALLS)}] {P(x, wy0)} rectangle {P(x + w, wy0 + 1.2 + h)};\n"
        x += w + 0.02
    # the clock tower, small, its clock one minute on
    tx, tw, th = 8.1, 0.95, 1.9
    s += f"\\filldraw[ink,line width=0.5pt,fill=stone] {P(tx, wy0)} rectangle {P(tx + tw, wy0 + th)};\n"
    s += f"\\filldraw[ink,line width=0.5pt,fill=stone] {P(tx - 0.08, wy0 + th)} rectangle {P(tx + tw + 0.08, wy0 + th + tw * 1.3)};\n"
    s += clock(tx + tw / 2, wy0 + th + tw * 0.65, tw * 0.48, minute=15)
    s += f"\\filldraw[ink,line width=0.5pt,fill=wallE] {P(tx - 0.12, wy0 + th + tw * 1.3)} -- {P(tx + tw + 0.12, wy0 + th + tw * 1.3)} -- {P(tx + tw / 2, wy0 + th + tw * 2.3)} -- cycle;\n"
    s += "\\end{scope}\n"
    s += line(((wx0 + wx1) / 2, wy0), ((wx0 + wx1) / 2, wy1), lw=4, colour="hometrim")
    s += line((wx0, (wy0 + wy1) / 2), (wx1, (wy0 + wy1) / 2), lw=4, colour="hometrim")
    s += f"\\draw[ink, line width=1.1pt] {P(wx0, wy0)} rectangle {P(wx1, wy1)};\n"
    # the floor
    s += rect(-0.5, -0.5, W + 0.5, 2.2, "homefloor")
    for k in range(1, 4):
        s += line((-0.5, k * 0.55), (W + 0.5, k * 0.55), lw=0.7, colour="homefloordark")
    # the sofa: its back, cushions, seat, and arm
    s += f"\\filldraw[inked,fill=sofa,rounded corners=10pt] (-1,3.0) rectangle (27,10.2);\n"
    for k in range(3):
        s += f"\\filldraw[inked,fill=sofa,rounded corners=8pt] {P(1.4 + k * 8.2, 5.0)} rectangle {P(9.4 + k * 8.2, 9.8)};\n"
    s += f"\\filldraw[inked,fill=sofa,rounded corners=8pt] (-1,1.4) rectangle (27,5.4);\n"
    s += line((-1, 4.2), (27, 4.2), lw=1.0, colour="sofadark")
    s += f"\\filldraw[inked,fill=sofadark] (1.0,0.2) rectangle (1.6,1.4);\n\\filldraw[inked,fill=sofadark] (24.4,0.2) rectangle (25.0,1.4);\n"
    s += f"\\filldraw[inked,fill=sofa,rounded corners=10pt] (-1,1.2) rectangle (2.2,7.4);\n"
    # the sunlight: shafts from the window, a bright patch on the seat
    s += f"\\fill[sunlight, opacity=0.28] {P(wx0, wy1)} -- {P(wx1, wy1)} -- (22.5,5.4) -- (9.5,5.4) -- cycle;\n"
    for k in range(3):
        x0 = 4.6 + k * 3.4
        s += f"\\fill[sunlight, opacity=0.22] {P(x0, wy1)} -- {P(x0 + 1.3, wy1)} -- {P(x0 + 12.2, 5.4)} -- {P(x0 + 10.6, 5.4)} -- cycle;\n"
    s += f"\\fill[homewindow, opacity=0.75] (9.0,5.4) -- (22.6,5.4) -- (21.4,3.0) -- (7.6,3.0) -- cycle;\n"
    # the empty treat bag on its side, and the treats left
    bx, by = 5.4, 4.6
    s += f"\\filldraw[inked,fill=treatbag] {P(bx, by)} -- {P(bx + 2.6, by - 0.15)} -- {P(bx + 2.9, by + 0.35)} -- {P(bx + 2.7, by + 0.9)} -- {P(bx + 2.95, by + 1.5)} -- {P(bx + 0.4, by + 1.75)} -- {P(bx + 0.1, by + 1.2)} -- {P(bx - 0.15, by + 0.6)} -- cycle;\n"
    s += f"\\filldraw[inked,fill=treatbagdark] {P(bx + 2.6, by - 0.15)} -- {P(bx + 3.25, by + 0.05)} -- {P(bx + 3.0, by + 0.6)} -- {P(bx + 3.3, by + 1.15)} -- {P(bx + 2.95, by + 1.5)} -- {P(bx + 2.7, by + 0.9)} -- {P(bx + 2.9, by + 0.35)} -- cycle;\n"
    s += f"\\draw[ink, line width=0.8pt] {P(bx + 0.9, by + 0.2)} -- {P(bx + 1.2, by + 1.1)};\n\\draw[ink, line width=0.8pt] {P(bx + 1.9, by + 1.5)} -- {P(bx + 1.6, by + 0.9)};\n"
    s += f"\\filldraw[inked,fill=hometrim,line width=0.7pt] {P(bx + 1.0, by + 0.9)} .. controls {P(bx + 1.4, by + 1.25)} and {P(bx + 1.8, by + 1.25)} .. {P(bx + 2.1, by + 0.95)} -- {P(bx + 2.35, by + 1.15)} -- {P(bx + 2.35, by + 0.75)} -- {P(bx + 2.1, by + 0.95)} .. controls {P(bx + 1.8, by + 0.65)} and {P(bx + 1.4, by + 0.65)} .. cycle;\n"
    for (tx_, ty_, rot) in ((bx + 3.9, by - 0.05, 20), (bx + 4.7, by + 0.25, -35), (bx + 3.6, by - 0.6, 70)):
        s += f"\\filldraw[inked,fill=treat,line width=0.8pt,rotate around={{{rot}:{P(tx_, ty_)}}}] {P(tx_, ty_)} ellipse (0.22 and 0.15);\n"
    # Chumbles, in the sun
    s += chumbles_loaf(12.6, 4.75, 1.55)
    # a last warmth over everything
    s += f"\\fill[sunlight, opacity=0.12] (0,0) rectangle {P(W, H)};\n"
    return s, (W, H)


# ---- the Water Gap ------------------------------------------------------------

def gallows_oak(x, y0, s, burning=False):
    """The Gallows Oak: a black trunk, roots into the water, a dark crown,
    gold eyes in the bark (shut when it burns), as draw/river.rs draws its eye."""
    out = ""
    out += f"\\filldraw[inked,fill=oakbark] {P(x - 1.1 * s, y0)} .. controls {P(x - 0.8 * s, y0 + 4 * s)} and {P(x - 1.2 * s, y0 + 7 * s)} .. {P(x - 0.6 * s, y0 + 9 * s)} -- {P(x + 0.6 * s, y0 + 9 * s)} .. controls {P(x + 1.2 * s, y0 + 7 * s)} and {P(x + 0.8 * s, y0 + 4 * s)} .. {P(x + 1.1 * s, y0)} -- cycle;\n"
    for k in range(5):
        a = -0.9 + k * 0.45
        out += f"\\draw[ink, line width=3pt] {P(x + a * s, y0 + 0.2 * s)} .. controls {P(x + a * 2.5 * s, y0 - 0.2 * s)} .. {P(x + a * 4.2 * s, y0 - 0.5 * s)};\n"
    for (cx, cy, r) in [(0, 10.2, 2.6), (-2.2, 9.4, 1.9), (2.3, 9.6, 2.0), (-1.0, 11.6, 1.7), (1.3, 11.4, 1.8)]:
        out += f"\\filldraw[inked,fill=oakleaf] {P(x + cx * s, y0 + cy * s)} circle ({f(r * s)});\n"
    for (ex, ey) in [(-0.3, 2.6), (0.35, 4.6), (-0.2, 6.8)]:
        if burning:
            out += f"\\draw[ink, line width=1.4pt] {P(x + (ex - 0.25) * s, y0 + ey * s)} -- {P(x + (ex + 0.25) * s, y0 + ey * s)};\n"
        else:
            out += f"\\filldraw[inked,fill=oakeye] {P(x + ex * s, y0 + ey * s)} ellipse ({f(0.32 * s)} and {f(0.16 * s)});\n"
            out += circle(x + ex * s, y0 + ey * s, 0.07 * s, "ink")
    return out


def downriver():
    """Downriver: the Calder in its sandstone canyon below the house, the
    old weir, the Defendant on its wall, a Bench in the air over the race
    with green orbs of the river's own, and far down the gorge, over the
    lake, the Gallows Oak. The right side is canyon wall, under the sheet."""
    W, H = 32.0, 18.0
    rng = random.Random(2066)
    s = f"\\fill[riversky] (0,0) rectangle {P(W, H)};\n"
    s += f"\\fill[riverfog] (0,{f(H * 0.45)}) rectangle {P(W, H * 0.7)};\n"
    # the far lake and the Oak on its shore, small down the gap
    s += f"\\filldraw[inked,fill=riverwater] (9,7.2) -- (17,7.2) -- (16,6.4) -- (10,6.4) -- cycle;\n"
    s += gallows_oak(13.2, 7.0, 0.42)
    # the canyon walls in strata, either side, stepping in toward the gap
    for side in (-1, 1):
        for k in range(6):
            top = 18.0 - k * 0.7
            if side < 0:
                xs = [(-0.5, 0), (5.5 + k * 0.6, 0), (8.6 + k * 0.1, top * 0.42 + 4), (-0.5, top)]
            else:
                xs = [(32.5, 0), (24.0 - k * 0.6, 0), (17.6 - k * 0.1, top * 0.42 + 4), (32.5, top)]
            col = ["sandstone", "sandstonedark", "sandstonepale"][k % 3]
            s += poly(xs, col)
    # the race, and the broken weir across it
    s += f"\\filldraw[inked,fill=riverwater] (5,0) -- (25,0) -- (17.2,6.4) -- (9.0,6.4) -- cycle;\n"
    for k in range(26):
        x = rng.uniform(7, 23); y = rng.uniform(0.3, 6.0)
        w = rng.uniform(0.4, 1.4) * (1 - y / 7)
        s += f"\\draw[riverfoam, line width=1.2pt] {P(x, y)} -- {P(x + w, y + 0.05)};\n"
    for (x0, x1) in [(5.5, 10.2), (11.8, 14.8), (16.6, 21.0)]:
        s += f"\\filldraw[inked,fill=sandstonepale] {P(x0, 2.4)} rectangle {P(x1, 3.4)};\n"
        s += f"\\fill[riverfoam, opacity=0.7] {P(x0, 2.4)} rectangle {P(x1, 2.0)};\n"
    s += defendant(8.2, 3.4, 1.15, lean=0.15)
    # a Bench in the air over the second gap, a curtain of the river's orbs
    s += f"\\filldraw[inked,fill=bench] (12.6,5.2) rectangle (14.4,5.55);\n\\draw[goldtrim, line width=1.6pt] (12.65,5.5) -- (14.35,5.5);\n"
    for r in range(4):
        for c in range(7):
            if 2 <= c <= 3 and 1 <= r <= 2:
                continue  # the hole to go through
            s += orb(16.0 + c * 0.55, 4.0 + r * 0.55, 0.2, "river", glow=False)
    # a hook ring on a pole, and a lever on the bank
    s += f"\\draw[ink, line width=2pt] (19.4,2.0) -- (19.4,5.8);\n\\draw[goldtrim, line width=2.2pt] (19.4,6.1) circle (0.3);\n"
    return s, (W, H)


def oak_burns():
    """The Oak burns: across Lake Stillwater at dawn the Gallows Oak burns
    to the waterline, its eyes shut, smoke going up; on the near shore the
    Defendant, small, watching. The right side is lake, under the sheet."""
    W, H = 32.0, 18.0
    rng = random.Random(2067)
    s = f"\\fill[riversky] (0,0) rectangle {P(W, H)};\n"
    s += f"\\fill[dawn] (0,{f(H * 0.5)}) rectangle {P(W, H * 0.62)};\n"
    # smoke, then the lake
    for k in range(9):
        cx = 9.0 + rng.uniform(-1.5, 2.5) + k * 0.6; cy = 9.5 + k * 1.0
        s += f"\\fill[{'smoke' if k % 2 else 'smokepale'}, opacity=0.75] {P(cx, cy)} circle ({f(1.0 + k * 0.25)});\n"
    s += f"\\filldraw[inked,fill=riverwater] (-0.5,0) rectangle (32.5,6.2);\n"
    s += f"\\filldraw[inked,fill=sandstonedark] (2,6.2) .. controls (6,7.0) and (12,7.1) .. (18,6.2) -- cycle;\n"
    s += gallows_oak(9.5, 6.4, 0.75, burning=True)
    # fire up the trunk and in the crown
    for k in range(16):
        fx = 9.5 + rng.uniform(-1.6, 1.6); fy = 6.6 + rng.uniform(0, 8.5)
        r = rng.uniform(0.35, 0.8)
        s += f"\\fill[fire] {P(fx, fy)} circle ({f(r)});\n\\fill[firehi] {P(fx, fy + 0.1)} circle ({f(r * 0.5)});\n"
    # its reflection, broken by the water
    for k in range(10):
        y = 5.6 - k * 0.5
        s += f"\\draw[fire, line width=2.5pt, opacity={f(0.7 - k * 0.06)}] {P(8.6 + rng.uniform(-0.4, 0.4), y)} -- {P(10.4 + rng.uniform(-0.4, 0.4), y)};\n"
    # the near shore, and the Defendant on it
    s += f"\\filldraw[inked,fill=sandstone] (-0.5,0) -- (-0.5,2.4) .. controls (4,2.9) and (8,2.4) .. (11,1.4) -- (11,0) -- cycle;\n"
    s += defendant(5.0, 2.4, 1.0)
    return s, (W, H)


def ship_side(x, y, s, sails=True):
    """A ship side on, bow to the right: hull, masts, sails, a red flag."""
    out = poly([(x - 3 * s, y + 1.0 * s), (x + 3.4 * s, y + 1.0 * s), (x + 2.6 * s, y), (x - 2.6 * s, y)], "hulldk")
    for k, mx in enumerate((-1.6, 0.2, 1.8)):
        h = (3.2 if k == 1 else 2.5) * s
        out += f"\\draw[ink, line width=1.4pt] {P(x + mx * s, y + 1.0 * s)} -- {P(x + mx * s, y + 1.0 * s + h)};\n"
        if sails:
            out += poly([(x + (mx - 0.8) * s, y + 1.0 * s + h * 0.85), (x + (mx + 0.8) * s, y + 1.0 * s + h * 0.85), (x + (mx + 0.7) * s, y + 1.0 * s + h * 0.3), (x + (mx - 0.7) * s, y + 1.0 * s + h * 0.3)], "sail")
    out += poly([(x + 0.2 * s, y + 4.2 * s), (x + 0.9 * s, y + 4.0 * s), (x + 0.2 * s, y + 3.8 * s)], "piratered")
    return out


def anomaly():
    """The anomaly: under the burnt Oak's roots, a door of light on the lake
    bed; the Defendant swimming down to it; through it, ships on another sea."""
    W, H = 32.0, 18.0
    s = f"\\fill[oceandeep] (0,0) rectangle {P(W, H)};\n"
    s += f"\\fill[ocean] (0,{f(H * 0.55)}) rectangle {P(W, H)};\n"
    # the lake bed and the Oak's dead roots
    s += f"\\filldraw[inked,fill=sandstonedark] (-0.5,0) -- (-0.5,3.0) .. controls (8,3.6) and (20,2.6) .. (32.5,3.2) -- (32.5,0) -- cycle;\n"
    for k in range(7):
        a = 4 + k * 3.6
        s += f"\\draw[ink, line width=4pt] (16,9) .. controls {P(a, 7)} .. {P(a + 1.2, 2.8)};\n"
    s += f"\\filldraw[inked,fill=oakbark] (14.8,9) rectangle (17.2,18.5);\n"
    # the door: rings of light, and through it another sea and ships
    for r, op in ((5.2, 0.15), (4.2, 0.25), (3.3, 0.4)):
        s += f"\\fill[biolume, opacity={op}] (16,4.6) ellipse ({f(r)} and {f(r * 0.45)});\n"
    s += f"\\filldraw[inked,fill=seasky] (16,4.6) ellipse (2.6 and 1.2);\n"
    s += f"\\begin{{scope}}\\clip (16,4.6) ellipse (2.6 and 1.2);\n\\fill[ocean] (13,3.4) rectangle (19,4.5);\n" + ship_side(15.2, 4.3, 0.22) + ship_side(17.0, 4.4, 0.16) + "\\end{scope}\n"
    s += defendant(9.5, 10.0, 1.0, lean=0.5, falling=True)
    for k in range(9):
        s += f"\\draw[riverfoam, line width=1pt] {P(10 + k * 0.25, 11.2 + k * 0.6)} circle ({f(0.08 + k * 0.01)});\n"
    return s, (W, H)


def descent():
    """The descent: the drained whirlpool's floor, and under the lake bed a
    white building older than the Oak, its halls lit with nothing."""
    W, H = 32.0, 18.0
    s = f"\\fill[oceandeep] (0,0) rectangle {P(W, H)};\n"
    s += f"\\filldraw[inked,fill=sandstonedark] (-0.5,9) .. controls (8,10.2) and (22,9.4) .. (32.5,10) -- (32.5,7.5) -- (-0.5,7.5) -- cycle;\n"
    # the Vault under the bed: white ribs, black glass, a shaft of cyan light
    s += f"\\filldraw[inked,fill=blackglass] (4,0) -- (4,7.5) -- (24,7.5) -- (24,0) -- cycle;\n"
    for k in range(6):
        x = 5.5 + k * 3.4
        s += f"\\filldraw[inked,fill=vaultwhite] {P(x, 0)} rectangle {P(x + 0.7, 7.5)};\n"
    s += f"\\fill[vaultcyan, opacity=0.35] (13,0) rectangle (15,10.5);\n"
    for y in (1.5, 3.5, 5.5):
        s += f"\\draw[vaultcyan, line width=2pt] (4.2,{f(y)}) -- (23.8,{f(y)});\n"
    s += f"\\filldraw[inked,fill=vaultviolet] (14,2.2) circle (0.9);\n"
    s += defendant(14.0, 11.0, 0.9, lean=0.0, falling=True)
    return s, (W, H)


def grey_ending():
    """The grey ending: an ordinary evening in Meridian, the clock tower
    reading 21:16, and the Defendant at the bottom of the picture going
    grey from the hands, as time stopped is drawn grey."""
    W, H = 32.0, 18.0
    rng = random.Random(2081)
    s = sky(W, H)
    s += skyline(rng, W, 3.0)
    s += clocktower(22.5, 3.0, 2.6, 9.0, minute=16)
    s += f"\\fill[grey1, opacity=0.55] (0,0) rectangle {P(W, 6.0)};\n"
    s += defendant(8.0, 1.0, 1.4)
    # going grey: the figure washed over, flakes rising
    s += f"\\fill[grey2, opacity=0.65] (6.6,0.9) rectangle (9.6,5.6);\n"
    for k in range(26):
        x = 6.2 + rng.uniform(0, 4.2); y = 2.0 + rng.uniform(0, 6.5)
        g = ["grey1", "grey2", "grey3", "grey4"][k % 4]
        s += f"\\fill[{g}] {P(x, y)} rectangle {P(x + 0.18, y + 0.18)};\n"
    return s, (W, H)


PICTURES = {
    "title": title,
    "sentencing": sentencing,
    "indictment": indictment,
    "verdict": verdict,
    "interlude": interlude,
    "testimony": testimony,
    "interlude_house": interlude_house,
    "orders": orders,
    "door": door,
    "lullaby": lullaby,
    "waking": waking,
    "chumbles": chumbles,
    "downriver": downriver,
    "oak_burns": oak_burns,
    "anomaly": anomaly,
    "descent": descent,
    "grey_ending": grey_ending,
}
for g in ("carbine", "scattergun", "launcher", "ripsaw", "big_iron", "filibuster", "redactor"):
    PICTURES[f"guns/{g}"] = (lambda g=g: gun_art(g))
for k in ("heart", "jump", "speed", "gun", "parry", "hook", "dash", "gavel", "pierce", "pulse", "mend", "wall", "pound", "saw", "conduct", "trigger", "plating", "stop", "contempt", "purse", "sealed", "bench", "blade"):
    PICTURES[f"icons/{k}"] = (lambda k=k: icon(k))

# dpi for each kind: big pictures fill a screen, icons are small
DPI = {"guns": 90, "icons": 160}


def document(body, size):
    W, H = size
    return PREAMBLE + "\\begin{document}\n\\begin{tikzpicture}\n" + f"\\clip (0,0) rectangle {P(W, H)};\n" + body + "\\end{tikzpicture}\n\\end{document}\n"


def main():
    only = set(sys.argv[1:])
    for name, make in PICTURES.items():
        if only and name not in only and name.split("/")[0] not in only:
            continue
        body, size = make()
        tex = document(body, size)
        folder = name.split("/")[0] if "/" in name else ""
        dpi = DPI.get(folder, 150)
        with tempfile.TemporaryDirectory() as tmp:
            t = Path(tmp)
            (t / "art.tex").write_text(tex)
            r = subprocess.run(["pdflatex", "-interaction=nonstopmode", "-halt-on-error", "art.tex"], cwd=t, capture_output=True, text=True)
            if r.returncode != 0:
                print(r.stdout[-3000:])
                raise SystemExit(f"{name}: pdflatex failed")
            # small pictures sit on the page's own panels, so their corners are clear
            clear = ["-transp"] if folder else []
            subprocess.run(["pdftocairo", "-png", *clear, "-r", str(dpi), "-singlefile", str(t / "art.pdf"), str(t / "art")], check=True)
            out = OUT / f"{name}.png"
            out.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(t / "art.png", out)
            src = SRC / f"{name}.tex"
            src.parent.mkdir(parents=True, exist_ok=True)
            src.write_text(tex)
        print("wrote", out)


if __name__ == "__main__":
    main()
