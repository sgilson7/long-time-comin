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

## Round 4: the see-through camera, a seventh enemy, zone two (2026-10-10)

The request is in `BRIEF.md`. Choices the request leaves open, taken by the agent and logged in `SECOND-ORDER.md` (rows Z1 on):

- **See-through camera.** The fragment shader cuts the static scene (never a body, an orb or a pickup) out of a cone from the eye to the player's head, solid near the axis and screen-door transparent at the rim, and out of a sphere round the lens; the floor under the feet is kept. The cone's sizes come from `balance.rs` as a scene uniform.
- **Zone one's seventh enemy: the Clerk of Minutes.** A new kind, `blinker`: it hovers, and at the end of each rest it vanishes and reappears elsewhere in sight of you, then winds up. Its patterns are fired at once, in zone one's style.
- **The story.** After the Bailiff kneels, a run does not end: an interlude (the years between) has the Defendant use what they know of the next fifteen years to build a company, and fund an expedition north in 2046 to find the Observatory's key (count I's key "given to someone else, a long time before") before their younger self can use it. Zone two is the White Waste; its boss is the Witness, the court's watcher in the ice. Beating it ends the run in this build; zones three and four stay sealed.
- **Zones in data.** `zone.json` holds a route, archetypes, names, encounters and a scene (sky, fog, light, ink) for each built zone. A route belongs to one zone; a run carries the body, the gun and the rewards from zone one into zone two and heals in the interlude. Once the Bailiff has been beaten, the Holding also offers to start a run in 2046. Zone two's camp offers an expedition cache: one of two rewards.
- **Zone two's arenas.** Five archetypes after Returnal's and Saros's arenas: the Crevasse (a chasm with ice bridges at two heights), the Stepped Shelf (terraces up to a plateau), the Wreck (an icebreaker frozen in: through the hull, over the deck, up the wheelhouse), the Antenna Field (pylons and a ring catwalk), the Blue Hall (an ice cave of arches and a ledge round it); the camp, the weather station (the rest) and the Witness's hollow (the boss). New verticality: updraft vents that throw you up, a chasm you can fall into (the fall returns you, at a cost), overhangs, a raised ring.
- **The Injunction.** Bought in the Holding (as the Writ of Reach is). R projects a wall in front of you for 0.5 s; blue orbs (zone one's cyan and zone two's cobalt) that meet it break; every other orb passes. Cooldown in `balance.rs`.
- **Held formations.** A new volley shape, `form`: orbs placed in a shape (ring, grid, line, column, star, spiral, arc) round the enemy, in front of it, round you or over you, held, then released all at once or one after another, outward, inward, at you, down, along, or round; an orb may stop again and set off a second time. Zone two's seven enemies and the Witness use only these, and no pattern of zone one.
- **Zone two's enemies.** Cairn (turret), Rime Kite (floater), Hauler (walker), Shardling (moth that bursts into a held ring), Auger (a new kind, `burrower`: under the ice between attacks, unhurtable), Prism (floater), Aurora (high floater); the Witness (boss, three phases).

## Round 5: the Glass House (2026-10-11)

The request is in `BRIEF.md`. Choices it leaves open, taken by the agent and logged in `SECOND-ORDER.md` (rows M1 on):

- **The story.** Beating the Witness no longer ends the run. Its interlude, WHAT THE WITNESS SAW: the Defendant takes the key; the Witness says they will do a thing so terrible that time stops, and that it can say no more. Ten years in the house on the hill (2046 to 2056), its labs spending the company's money to find how time can be kept from breaking. Then the raid: the Continuance, men in identical grey coats, come through the grounds and kill the scientists. Zone three is the Glass House, 2056; its boss is the Executor, a tank. Beaten, its orders are found, signed in the Defendant's own hand, dated 2081: time must break, because if it does not, everything else will, which is much worse. Zone four (2081) stays sealed; the verdict follows the Executor.
- **The house's rooms** (`crates/core/src/arena/house.rs`), the same square and gates as before so the route and the reach tests apply, walled, with ceilings 15 m up: the Great Hall (twin stairs to a gallery round the room), the Dining Room (a long table to stand on, a minstrels' gallery), the Library (aisles of shelves that make lanes, a mezzanine), the Laboratory (a broken floor into a basement lab, catwalks, specimen tanks), the Long Gallery (display cases, a dais, an upper walk); the Vestibule (arrival), the Safe Room (rest, a typewriter), the Ballroom (the mini-boss) and the Conservatory (the boss: a wall of glass to the north, the lawn outside). Verticality here is stairs, galleries, a lower floor, tables and shelves to stand on, and the ground pound's bounce.
- **Humanoid enemies.** A new kind, `agent`, walks a navigation grid (`nav.rs`, half-metre cells over every floor a body can stand on, joined where a body can step or drop). A squad director (`squad.rs`) gives each agent its next place as it rests: cover (out of your sight, near a spot it can step out to and see you), your back (outside your view, behind where you face), a ring round you, or straight at you. It also times attacks together: a crossfire (two agents at least 90° apart round you wind up together and fire on the same tick), a pincer (one in front holds your eye while a flanker reaches your back), and a ring (three or more fire at once from round you). Seven: the Grey Man (rifle, cover and crossfire), the Outrider (flanker), the Marksman (high places, a sight line, its aim locks before it fires), the Warder (a riot shield that stops shots from in front), the Sapper (lobs canisters over cover that land and become held rings), the Cleaner (runs at you, lunges, leaves a line of held orbs), the Handler (while it lives the squad rests less and rings you). Their patterns are new: bursts that sweep, orbs that ricochet off walls, lobbed mines, and lanes (two walls of orbs either side of you, open at one end).
- **The mini-boss, the Section Chief**, in the Ballroom on the main path's fourth fight: an armoured agent with 1800 health who calls agents in batches while he lives, more below half health; when he falls his squad withdraws.
- **The Executor.** A new kind, `tank`. It could not be confirmed what the Atlas of Tyr does (SECOND-ORDER M2), so it moves as a heavy tracked tank: it turns on the spot to a heading, drives in long arcs with momentum, reverses, and now and then rams in a straight line; its turret turns on its own, slower, to keep its guns on you. Its hull and turret take no damage. Each phase opens weak points (glowing vents on the back of the turret first, then on the new cannons' backs); destroying every one of a phase's weak points starts the next, and each phase grows more cannons from new places (`enemies.json` lists each phase's mounts and weak points). The fourth phase grows wheels of eyes and a crown of cannons round it, rises off the floor and fires everything; it cannot be hurt, and surviving it for 40 s beats it. At the start it drives in through the Conservatory's glass wall: the panes shatter into falling shards and the camera shakes.
- **The ground pound** (Gavel Drop, bought in the Holding; key C): in the air, drop straight down fast; on landing a shockwave hurts every enemy and weak point near you, then you bounce back up to a little over the height you started from, with your air jumps back.
- **The Ripsaw**, a fourth gun found on the house's side paths: a click throws a saw blade that lodges in what it hits; holding fire revs every lodged saw for damage over time until its teeth are spent, then it comes back. Levels add saws.
- **More levels.** Every gun gains a level IV, found only in the house. Two new rewards that last the run: Hair Trigger (guns fire faster) and Plating (hits do less). The Holding gains fifteen upgrades in four branches (Body, Motion, Arms, Writs), shown as columns, each upgrade under the one it needs: Constitution IV, Fleet III, a second Spare Leap, Fourth Step, Armory II, Gavel Drop and Gavel II, the Writ of Reach II, Objection II, Injunction II, Appeal (a second Recess), Precedent (start with Piercing Rounds), Good Conduct (more Clemency), Bail (more health on a clear), and the Lien (a Ripsaw waits at the start of every run).

## Round 6: a test save, the boss rush, second fires, stronger guns (2026-10-11)

The request is in `BRIEF.md`. Choices it leaves open (SECOND-ORDER rows N1 on):

- **The test save** is a button in the Holding (asked twice, as the reset is, since it replaces the record): every upgrade bought, every count read, every built zone open, the game beaten (so the boss rush is open), and 5,000 Clemency (`Meta::everything`).
- **The boss rush** opens in the Holding once a run has beaten the game: the Bailiff's court, then the Witness Stand, then the Conservatory, each boss alone, health restored between and no interlude, with the body and gun the Holding gives a run. Its end has its own verdict line; it banks Clemency and counts in the record (`rush_wins`).
- **Second fires** on the right mouse button, each with its own cooldown and 25% more damage per gun level (`weapons.json` `alt`): the Carbine's three-round burst (22 a round, piercing one), the Scattergun's slug (90, piercing three), the Launcher's cluster (five bombs in a fan), the Ripsaw's shatter (every lodged blade bursts for 60 and half its teeth left, and comes back). The parry, which had the right mouse button and Q, keeps Q.
- **Stronger guns**, the Carbine unchanged: the Scattergun's pellets 8 to 11 damage (were 6 to 8), the Launcher's hits 38 to 60 and bursts 34 to 56 (were 30 to 46 and 26 to 42), the Ripsaw's throw 24 to 32, rev 5 to 8 and teeth 180 to 300 (were 18 to 24, 4 to 6, 140 to 240): about 30% more each.
