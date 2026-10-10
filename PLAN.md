# Plan

Stage 2. `B2 Require step-by-step justification`. Approved in advance (BRIEF.md, Autonomy).

## The game in one paragraph

**Long Time Comin'** (after the folder Sam made for it). In 2081 you committed a crime the court calls the Stilling. The Tribunal of the Last Hour sentenced you to Corrective Return: to be sent back to the night before and stop yourself. The court's Bailiff aimed badly and you arrived fifty years early, in 2031, so you have to cross the half century on foot. Each zone is a decade. Each death, the Bailiff collects you, the court reads the next count of the indictment, adds the years to your sentence (Sifu's ageing, as time served), and returns you to the Holding, where the Clemency your run earned buys survivability and new mechanics.

## Inspirations, and what each contributes

| Source | What it gives this game |
| --- | --- |
| Returnal, Saros | third-person bullet hell; large readable orbs; ground waves to jump; dash with invulnerability; levelled guns on side paths; parrying one colour of shot back (Saros) |
| Realm of the Mad God | dense radial patterns: rings, spirals, flowers, curling orbs |
| Hades | runs through a route of rooms; three bosses then a final boss; a hub between runs with permanent upgrades; story told on death |
| Risk of Rain 2 (Loader) | a grappling hook that pulls you and keeps the momentum |
| Tower of Guns | one gun at a time, picked up and swapped; fast arenas; strong verticality |
| Sifu | each death costs years |
| Iron Vector | the engine: Q16.16 core, instanced draws, box collision, the core computing every matrix, pointer-lock input, fixed 60 Hz ticks drawn interpolated |
| Food Truck Circus | the meshes (sphere, cylinder, cone), flight physics habits, TikZ art pipeline from `Sam_Comm_final.png` |

## Architecture (Builder, from Iron Vector and Food Truck Circus)

- `crates/core` (`ltc_core`): every rule, Q16.16 integers, no floats, clocks or I/O. `fx`, `geom`, `rng`, `mesh` copied from Food Truck Circus.
- `crates/shim` (`ltc_shim`): one call per function, no branches.
- `web/`: WebGL 2. The renderer gains two passes over Iron Vector's: an ink outline (the inverted hull, the plum line of the TikZ art) and an additive glow for orbs. The core says which pass each draw is in.
- `data/`: weapons, enemies, patterns, the zone, upgrades, story, copy, controls, sounds, shaders.

## Modules

| Module | Holds |
| --- | --- |
| `balance` | every feel number |
| `content` | parsing of data files |
| `level` | solid boxes, a grid over them, ray and sphere queries |
| `arena` | building an arena from an archetype and a seed: floor, walls, platforms, stairs, cover, gates, spawn points, and the decoration (skyline, rooftop clutter, plants, signs) |
| `player` | walking, multi-jump, dash, grapple, parry, health |
| `orbs` | enemy orbs and ground waves; volleys from patterns |
| `weapons` | the three guns, levels, player shots, splash |
| `enemy` | enemy kinds, movement, attack schedules; the boss's phases |
| `route` | the zone graph for a run: main path, detours that rejoin, rewards |
| `meta` | saved progress: Clemency, upgrades, deaths, counts revealed |
| `world` | one arena in play: tick, collisions, waves of enemies, pickups, gates |
| `draw` | instances for the player, enemies, orbs, shots, effects |
| `game` | modes, the clock, input latch, camera, HUD and screen JSON |

## The run

Zone one, *The Waiting City, 2031*. Main path: Arrival → five fights → the Antechamber (rest) → the Bailiff. Between main nodes, three or four detours of one or two harder fights each, rejoining the main path at the next node. A detour's last node holds its reward, shown on the map before you choose: a levelled gun, +max health, +speed, +1 jump (up to four). Each run's graph comes from a seed, so runs differ. Zones two to four (2048 The Flood Years, 2064 The Glass Decade, 2081 The Night of the Stilling) are on the route and sealed in the MVP.

## Combat

- Guns: Carbine (automatic, accurate), Scattergun (pellets, close), Hourglass Launcher (slow orb, splash). Levels 1 to 3 raise damage and add a trait. One gun carried; a pickup swaps it.
- Orbs: volleys from `data/patterns.json` (ring, aimed fan, sphere, ground wave), with speed, acceleration, curl (turning), spin between repeats, size and colour. Gold orbs can be parried once *Objection* is bought.
- Enemies: Sentinel (turret, spirals), Drifter (floats, aimed fans), Warden (walks, ground waves), Lantern (floats high, flowers and gold orbs), Moth (fast, touches). The Bailiff has three phases.

## Meta progression (`data/upgrades.json`)

Constitution I–III (max health), Fleet I–II (speed), Spare Leap (start with two jumps), Second Wind Dash (three dash charges), Standing Armory (start with a level-two Carbine), the Writ of Reach (grappling hook), Objection (parry gold orbs), Recess (survive one killing blow a run).

## Art

TikZ through `tools/art.py`, compiled with pdflatex and pdftoppm, after the diagrams-as-code prompt and `Sam_Comm_final.png`: plum ink outlines, flat fills, warm yellow sky, salmon and coral walls, sage greens. Title, the sentencing, the indictment page, the Holding, the zone card, the Bailiff, the three guns, the verdict.

## Order of work

1. Core maths, level, player movement with tests.
2. Orbs, weapons, enemies, world tick with tests.
3. Arena generation and decoration; route; meta.
4. Game modes, render, shim, page; play it.
5. Art, sound, story.
6. Check, repository, Pages, scholar site.
