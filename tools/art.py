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
- the orbs: violet, magenta and cyan hurt; gold ones are the ones Objection
  parries, and only gold orbs carry the gold ring (`orb(..., gold=True)`),
  as in the game.
- the Defendant: dark coat, orange scarf and trim, as data/palette.json's
  player_coat and player_trim (`defendant`).
- the Bailiff: a robe, a clock for a face, a crown, two hands with gavels
  and a ring of hours, as crates/core/src/draw.rs draws it (`bailiff`).
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


def clock(x, y, r, fill="clock"):
    """A clock face stopped at 21:14: the hour hand at 277°, the minute
    hand at 84°, clockwise from twelve."""
    s = circle(x, y, r, fill)
    s += f"\\draw[ink, line width=1.6pt] {P(x, y)} circle ({f(r * 0.86)});\n"
    s += f"\\foreach \\k in {{0,...,11}} {{ \\fill[ink] ($({f(x)},{f(y)})+(\\k*30:{f(r * 0.74)})$) circle ({f(r * 0.06)}); }}\n"
    for deg, length, w in [(277, 0.5, 3.2), (84, 0.72, 2.2)]:
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


def clocktower(x, y0, w, h):
    s = rect(x, y0, x + w, y0 + h, "stone")
    s += rect(x - 0.25, y0 + h, x + w + 0.25, y0 + h + 0.35, "wallE")
    s += rect(x - 0.1, y0 + h + 0.35, x + w + 0.1, y0 + h + w * 1.05, "stone")
    s += clock(x + w / 2, y0 + h + 0.35 + w * 0.52, w * 0.4)
    s += poly([(x - 0.3, y0 + h + w * 1.05), (x + w + 0.3, y0 + h + w * 1.05), (x + w / 2, y0 + h + w * 1.05 + w * 0.9)], "wallE")
    for j in range(int(h / 1.2)):
        s += rect(x + w * 0.4, y0 + 0.6 + j * 1.2, x + w * 0.6, y0 + 1.1 + j * 1.2, "windark", lw=0.7)
    return s


def orb(x, y, r, colour, gold=False, glow=True):
    """An orb as the game draws it: a halo, an inked core, a highlight.
    Only a gold orb (one Objection parries) carries the gold ring."""
    s = ""
    if glow:
        s += f"\\fill[{colour}, opacity=0.22] {P(x, y)} circle ({f(r * 1.9)});\n"
        s += f"\\fill[{colour}, opacity=0.3] {P(x, y)} circle ({f(r * 1.35)});\n"
    s += circle(x, y, r, colour)
    s += f"\\fill[white, opacity=0.75] {P(x - r * 0.35, y + r * 0.35)} circle ({f(r * 0.28)});\n"
    if gold:
        s += f"\\draw[gold, line width=2.2pt] {P(x, y)} ellipse ({f(r * 1.6)} and {f(r * 0.5)});\n"
        s += f"\\draw[ink, line width=0.7pt] {P(x, y)} ellipse ({f(r * 1.75)} and {f(r * 0.62)});\n"
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
    elif kind == "gavel":
        s += poly([(0.9, 0.9), (2.4, 2.4), (2.1, 2.7), (0.6, 1.2)], "wood")
        s += f"\\filldraw[inked,fill=goldtrim,rotate around={{45:(2.6,2.6)}}] (1.9,2.25) rectangle (3.3,2.95);\n"
    return s, (W, H)


PICTURES = {
    "title": title,
    "sentencing": sentencing,
    "indictment": indictment,
    "verdict": verdict,
}
for g in ("carbine", "scattergun", "launcher"):
    PICTURES[f"guns/{g}"] = (lambda g=g: gun_art(g))
for k in ("heart", "jump", "speed", "gun", "parry", "hook", "dash", "gavel"):
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
