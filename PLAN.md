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

## Round 7: the Ascent (2026-10-11)

The request is in `BRIEF.md`. Choices it leaves open (SECOND-ORDER rows Q1 on):

- **Two mini-bosses in "the previous two missions"**, read as zones two and three, the two before zone four: the Pioneer, a mech after Iron Vector's frames (the expedition's walker, taken by the ice), in the White Waste's Antenna Field; and the Escort, a small Executor, in the Glass House's Great Hall. A zone's route now lists its mini-bosses (`zone.json` `minibosses`). The Pioneer glides round you at range, quick-boosts sideways, ascends and hovers, and fires held formations as zone two's enemies do; the Escort drives as the Executor does, with two phases of weak points and no last phase to outlast.
- **The story.** The Executor's orders become the house's interlude, THE ORDERS: the Defendant has thought of letting themself die, but the court always sends them back, so this one must kill that one. They climb Mount Meridian to the Observatory on its summit, where the Long Door stands in a circle of stones. Zone four is the Ascent, the evening of 14 March 2081, always at 21:14. Its boss is the Defendant. After it falls, the Idiot God bursts into the sky, the summit melts, and you are drawn into its maw; the revelation (THE DOOR) says the sin was the ritual that opened the door and let it in; the stopped evening is the lock; the Tribunal knew only that time had stopped. Zone five (the Idiot God) is sealed; zone six is not listed.
- **The climb.** Zone four's arenas are terraces cut into the mountain, open to the drop on three sides; each node's arena stands higher than the last (12 m a step of the map), and what is below is drawn that far down: the mountain's flanks, cloud, and the city's lights far under. Archetypes: the Switchback, the Span (two crags and a bridge over a gap), the Spire (platforms round a pillar), the Ruined Chapel, the Ledge (an overhang and a cave). The Trailhead (arrival), the Hermitage (rest), the Circle (the boss: trilithons round the Long Door). A fall off an edge returns you at a cost.
- **Seven assassins**, people who walk the grid as the house's agents do and with the same roles, wearing the Defendant's own coat under a clock-face mask: the Proxy, the Shade, the Watcher, the Bellringer, the Sower, the Knife, the Second. All their patterns are new.
- **The Defendant** (kind `double`): moves as you do, running round you, dashing, jumping, with your coat and your gun. Its time stop: as an attack's windup ends, time stops for you and everything but it for a few seconds (the picture goes grey); it blinks to places round you and lays held formations that do not move while time is stopped, then time resumes and they fly. Three phases, assassins called at each change.
- **The time stop, yours** (bought from the Holding once the Defendant is beaten: the Stopped Hour, then three Stopped Clocks): T stops time for 3 s, 4, 5 or 6; enemies, orbs and waves hold still, and your shots hang where they are until time resumes. 30 s cooldown.
- **Grab and throw** (G, every zone): within 3.5 m in front of you, an enemy at half health or less (not a boss, a mini-boss or a tank) is lifted; G again throws it along your aim, and what it hits takes heavy damage, with a burst round it. The thrown enemy dies.
- **The Big Iron** (from zone four's side paths, levels I to IV): six rounds, very heavy and slow shots (70 to 110 damage, 55 m/s, a fat round), reloaded one shell at a time whenever you are not firing (0.4 s a shell, so 2.4 s for six), the cylinder swung out while it loads. Its second fire fans the hammer: every round left, one every 80 ms, a little wild.

## Round 8: the Defendant's moves (2026-10-11)

The request is in `BRIEF.md`. Choices it leaves open (SECOND-ORDER rows Q13 on):

- **The golden parry.** The Defendant had no real guard: what read as one was a gold glow 5 m across drawn round it through every cast, and the gold `parry_bait` volley in every phase. The glow now shows only in the windup and is a third the size, and `parry_bait` is in the last phase only.
- **The hook** (`"move": "hook"` on an attack, `Lunge::Hook`): in its windup a chain flies from its hand to a spot on the floor 4 m beside you; as it casts, the chain pulls it there at 36 m/s, over anything in the way, and its cast waits until it arrives (at most a second), then fires a ring and a short fan.
- **The ground pound** (`Lunge::Pound`): in its windup it leaps 6 m and comes over where you stood, which is marked on the floor; as it casts it drops at 40 m/s, and two shockwaves (jump them) and a ring go out from where it lands.
- **The wall** (data only, `injunction_echo`): two standing walls of blue orbs, nine by three, a second apart, flying along the line to you. Your Injunction breaks them; otherwise step round the ends.
- **The stop, a quarter as often**: an attack's `once_in` (4 for each stop) takes it the first time it comes up and every fourth time after.
- **The dome**: a held formation `dome` (rows of rings over a half sphere round you, each ring turned half a gap from the one below) and a launch `gather` (in to the centre, and spent there, so a dome closes on the spot it was laid round and does not come out the other side). Each stop lays an inner shell 4.2 to 4.5 m out and an outer one 8 to 9 m out, which reach you about half a second apart; the dash's 13 ticks of invulnerability cover one shell, not both, and two charges cover both. The middle phase adds knives laid in rings where it stood, which fly at you later; the last adds a doubled outer shell and a grid that falls from above.
- **Kept hard**: its health goes from 3,600 to 4,000 (SECOND-ORDER Q16).

## Round 9: the mountain's creatures, the wall's wards (2026-10-11)

The request is in `BRIEF.md`. Choices it leaves open (SECOND-ORDER rows Q18 on):

- **Seven creatures** (`enemies.json`, kind `beast` with a `gait`, and one walker), each with two patterns of its own:
  - the Hart, a stag whose antlers end in clock hands: stands, then every 3 s bounds in a high arc to a place at its range further round you (`bound`); a fan of tines, and a hoofbeat wave on landing;
  - the Rook, a crag bird with eyes along its wings: circles high over you and dives as it winds up (`wheel`); a fast diving burst, a ring that rains down;
  - the Toad: short low hops toward its range (`hop`); a lobbed glob that bursts into a ring where it lands, a curling croak;
  - the Gaper, a man-eating plant: rooted; it lunges on its stalk and bites when you come within 2.5 m of its reach (`root`, `bite_cm`); seeds and a bloom;
  - the Freshet, a mountain stream risen as a body of water: rushes round you along the ground in sweeping curves (`surge`); two waves and a falling spray;
  - the Sporecap, a great toadstool: rooted, breathes slow spores that linger, and bursts into spores when it dies;
  - the Ursine, a bear of the high slopes: walks you down (kind `walker`); a close maul and a roar.
- **Corruption grows with the climb**: each arena's creatures carry `corrupt` from 1/6 at the trailhead to full at the last main fight (`CORRUPT_*`). Drawn (`draw/fauna.rs`) as eyes opening over the body (the Idiot God), tendrils from the back, a gold ring of hours turning backward and, past half, an afterimage that lags behind it where time stutters (the ritual).
- **The waves**: mostly creatures (more than two thirds), with enemies from below (the Auger, the Rime Kite, Shardlings, the Lantern, the Clerk, the Drifter) and the assassins in some waves.
- **The wall's wards**: an orb's colour gives the ward the Injunction needs to break it (`palette.json` `wards`): blue and cyan with the Injunction; violet, magenta and lime with a Standing Order; scarlet, ember and chalk with a Permanent Injunction (`upgrades.json`, effect `wall_ward`, its amount the ward it opens). Gold stays the parry's. An orb your wall breaks has a pale rim.

## Round 10: the Great Horned, and big fights (2026-10-11)

The request is in `BRIEF.md`. Choices it leaves open (SECOND-ORDER rows Q26 on):

- **The Great Horned** (`enemies.json`, a beast of gait `perch`, elite; zone four's mini-boss on its third main fight, in the Spire): an owl 3.4 m across, ear tufts like horns, a face disc round two gold eyes. Six dead snags stand round the arena, 8 m high (`perches` in `enemy.rs`); it sits 4 s on one, firing down (talons, a rain of feathers, a screech), then glides to another. One move in two it spreads its wings for 0.8 s and swoops instead, at 22 m/s, through where you stood and 6 m past, striking with its talons (its contact damage), then climbs to its next snag. It holds its fire in flight and calls up to two Rooks.
- **Big fights**: zone four's waves are 8 creatures at the first tier, 10 to 14 after (`zone.json`), most of them light: the Toad, Rook, Sporecap and Hart lost health and shots to suit coming ten at a time, the Hart's hoofbeat lost its ground wave, and the creatures' Clemency was halved.
- **Fifteen at most** (`MAX_ALIVE`): a wave larger than the room left comes in as room is made, and calls and a boss's summons are cut to the room left.


## Round 11: the Idiot God (2026-10-11)

The request is in `BRIEF.md`. Choices it leaves open (SECOND-ORDER rows R11 and Q30 on):

- **What "the latest Ultrakill chapters" lends.** Layer 8, *Fraud* (February 2026), is described as trick art: rooms that are not what they look like. With it, the games of impossible space: *Manifold Garden* (fall off the world and come down on it again from above; the world repeating above and below you), *Antichamber* and *Portal* (a doorway leads somewhere it cannot), the Penrose stair of Escher's *Relativity* (a stair that climbs forever), *Hyperbolica* (space that curves away from you), and Half-Life's Xen (islands in a void).
- **The story.** Beating the Defendant no longer ends the run. THE DOOR (the revelation) becomes the summit's interlude, and its button falls into the maw. Zone five is *The Idiot God*, its year "Outside". At its heart the Defendant understands what the older self did and chooses to do it better: to stop time in a bubble round the god alone, and keep it asleep. The ending (the verdict) is the Defendant playing the trumpet for ever.
- **The spaces** (`crates/core/src/arena/god.rs`), each a floor in a void, the same square and gates as the other zones so the route, spawn and reach rules apply:
  - **Falling comes round.** Off any edge you fall, and come down again from the sky over the same arena, unhurt, keeping your speed; enemies too. The arena is drawn again above and below you, fading into the dark, so you see the floor you will land on overhead.
  - **Space curves.** Far off, the scene bends up and round you as if you stood in a bowl, and breathes; near you it is flat, so aiming is unchanged (`scene.vert`, `uFx`).
  - **Seams.** A doorframe's two faces lead elsewhere: through one, you come out of its partner turned as it is turned, keeping your speed. Orbs go through seams too.
  - Archetypes: the **Fold** (islands, and the Waiting City hanging upside down overhead), the **Penrose Stair** (four flights round a well, each climbing; the top landing's seam is the bottom landing, so it climbs for ever), the **Doors** (free-standing frames paired as seams), the **Lattice** (Xen: rocks in the void, crystals, updrafts), the **Orrery** (rings of floor round a sun, the rings' gaps fallen through); the **Gullet** (arrival), the **Still Point** (rest: a stopped clock, the trumpet's case), the **Heart** (the boss).
- **Seven enemies, each from another art style or game** (drawn in `draw/god.rs`; the renderer gains three passes for styles a flat shape cannot give):
  - the **Glyph**, an eight-bit invader of cubes, flat to you, two frames: marches sideways in steps, steps nearer, reverses; fires columns;
  - the **Tesseract**, a vector-display hypercube in glowing lines, turning through four dimensions: folds away and unfolds elsewhere (a blinker); fires from its corners;
  - the **Etching**, an engraved figure in one-bit dither (Return of the Obra Dinn): moves only while you are not looking at it, and then fast;
  - the **Render**, a PlayStation-era hound whose corners snap to a coarse grid: turns on the spot, then charges straight, like tank controls;
  - the **Toon**, a rubber-hose cartoon with pie-cut eyes and white gloves, its ink boiling: every Toon bounces on one shared beat, and fires on landing;
  - the **Reprise**, you in photographic negative: walks the path you walked three seconds ago, firing your Carbine's bursts; stand still and it reaches you;
  - the **Recursion**, a Sierpinski pyramid in gold: killed, it splits into three smaller, and those into three smaller again.
- **The Idiot God** (kind `god`): a blind mass the size of the room over the Heart's north side, in three phases with a bar each (named, coloured), then the lullaby:
  - THE BLIND MASS: froth (rings that curl into flowers), eyes opening (aimed fans from everywhere above), Glyphs called;
  - THE DANCERS: the mindless dancers ring the god and pipe (lanes, turning stars, domes that close), Toons and Tesseracts called;
  - THE DREAMER: its eyes all open, the picture's colours turn over; orbs that fly out and come back along their path (time folding), Reprises and Etchings called.
- **The lullaby.** When the last bar empties the god does not die. The Defendant takes out a trumpet. A staff of five lines crosses the screen, and five notes after the ocarina's (the left mouse button, Q, E, R and F, which the trumpet frees from your gun and abilities). The dancers pipe a phrase (listen); then the same phrase scrolls again and you play it in time (play). A phrase played well (three in four of its notes on time) puts the god a verse deeper asleep; played badly, it stirs and the phrase comes again. Four verses, longer each time. Through all of it the god fires on the beat (rings every other beat, a ground wave to jump every eighth), so you dodge while you play: move, jump and dash stay yours. A bubble of stopped time grows round the god with each verse. After the fourth, the bubble closes, time stops round it, and the last verse plays once more over the stopped god; then the ending.
- **Bars.** A boss's HUD bar shows its phase's share of its health, with a pip for each bar left, and the phase's name. The lullaby's bar is the sleep.

## Round 13: the last sentence (2026-10-12)

The request is in `BRIEF.md`. Choices it leaves open (SECOND-ORDER rows R13 and Q44 on):

- **What opens it.** The Idiot God put to sleep (its id in the record's `beaten`). The Holding then shows a button, *The last sentence*, under the boss rush.
- **The story.** The clock on the mountain moved to 21:15; the court noticed, found the Defendant still at the summit, and did not understand the trumpet. It takes another of you from another timeline, younger and angrier, and sends them back with the first sentence. That one is you. The brief is read on the interlude's screen before the crater. Between the zones the mission has its own short interludes (THE YEARS AGAIN, THE KEY AGAIN, THE ORDERS AGAIN). At the summit, the Defendant's fall does not let the god in, because the first of you is already playing it to sleep; both cannot be, and reality breaks (REALITY BREAKS: the summit comes apart like a dropped plate, and you wake in bed on a Saturday morning, Chumbles sitting on your chest). The ending (BREAKFAST): he is fed, sleeps on the sofa in the sun, and far away the trumpet still plays.
- **One go.** The mission starts in the crater and goes through zones one to four without the Holding between; dying ends it as any run does (a death on the record). It opens no later starts.
- **New game plus.** Every fight's waves send half as many again (more of their own kinds), the next wave comes when three of the last are left, and up to 22 enemies are up at once (15 otherwise). Fourteen of the simpler enemies' patterns have remixes (`remix` in `patterns.json`) fired in their place: zone one's turrets, drifters and wardens, the Rime Kite's snowfall, the Grey Man's bursts, the Hart, the Rook and the Toad. Bosses and mini-bosses are as they were.
- **Home** (zone `home`, after the Idiot God in `zone.json`, marked `mission` so it is not one of the zones a run crosses): Your Bedroom (arrival), The Landing (rest: water, or a reward), Downstairs (the boss). In the bedroom Chumbles walks from beside the bed out of the door.
- **Chumbles** (kind `cat`, `crates/core/src/draw/cat.rs` after Sam's three photos: a long-haired lynx point, cream with a white ruff, grey-blue mask, ears, paws and plumed tail, blue eyes). He appears by the back door at a third of his size, screams to go outside or for treats (the HUD says which), and grows to full size over three seconds, unhurtable meanwhile; full size, his body is a 2.3 m sphere. Five bars, each named and each with its own gait (a new per-phase `gait`), so the fight changes shape every 20%:
  - THE ZOOMIES (charges straight at you, tank-style): claw swipes in sweeping fans, shed fur that curls, hisses;
  - THE POUNCE (bounds round you; pounces land where you stood with a ground wave and rings), a tail lash that sweeps both ways, kneading waves, two balls of yarn called;
  - THE COUNTER (sits up on the furniture, leaps between perches and swoops): things knocked off the counter rain from above, lobbed hairballs that burst, a glare that rings you;
  - 3 AM (creeps at you only while you are not looking): a laser-pointer lane and a sweeping dot, two-colour spirals, eyes in the dark that close in, more yarn;
  - TREATS (charges again, everything at once): treat rain, a frenzy of spirals and waves, a second scream, three balls of yarn.
  When the last bar empties he is not killed: he is fed, shrinks back to his own size and eats, and sleeps; then the ending.

## Round 14: the rewrite (2026-10-15)

The request is in `BRIEF.md`; the packet is `long_time_comin_rewrite_v2/` in Sam's folder (not committed). It is data only: `apply_rewrite.py --check`, then `--apply`, which replaces the narrative text of `data/story.json`, 40 keys of `data/copy.json` and the first boss's display name. The first boss is the Interdictor, a machine carrying an unsigned transfer warrant; the Bailiff of Hours remains the court's courier and is not fought. The internal id `bailiff` stays everywhere (`zone.json`, `draw.rs`, saves, tests). Nothing else is reworded: strings outside the packet that still say Bailiff are reported (SECOND-ORDER Q54 to Q57), not changed.

## Round 15: the arsenal (2026-10-15)

The request and Sam's answers to the design are in `BRIEF.md`; the design, with in-game screenshots, is the design journal's Round 15 tab. Choices and findings are SECOND-ORDER R15, W1 to W9, B15 and T15.

- **The Filibuster** (`filibuster`, from zone one): an automatic rifle that spins up. A spin gun's level (`cold_ms`, `spin_ms`, `bleed_ms`, `hot_spread_deg`) fires every `cold_ms` when it starts and every `rate_ms` once held `spin_ms`, its spread widening; let go, the spin bleeds away. `Held.spin` (0 to ONE) and `Held.charge` (thousandths of a tick) in `weapons.rs`. Its second fire, **Cloture** (`AltKind::Cloture`), is one lance of `damage` plus `spin_damage` times the spin, which it spends.
- **The Redactor** (`redactor`, from zone two): an automatic rifle with less damage whose rounds erase the first enemy orb they meet of a ward up to `erase_ward` (blue at I, two wards at II, every orb but gold from III), going on through `through` more (`world.rs` `shots_step`, `erasable`). Its second fire, **Redaction** (`AltKind::Redaction`, `ShotKind::Bar`), is a flat bar 3 m wide that slides forward 0.9 s, erasing every orb but gold and hitting each enemy it crosses once (`world.rs` `bar_step`).
- **Guns to level V** (`balance.rs` `MAX_GUN_LEVEL`); the Holding's Standing Armory starts one at III at most.
- **Exhibits** (`upgrades.json`, effect `exhibit`, `gun`): B the Scattergun and C the Hourglass Launcher at once; D the Filibuster, E the Redactor, F the Ripsaw and G the Big Iron once the boss of the zone that first offers it is beaten. **Carry in** (`meta.rs` `carry`, `carry_choices`, `carried`; `Game::carry`; the shim's `carry`; the Holding's row in `screens.js`) chooses the gun every run starts with, the rush and the last mission included.
- **Contempt** (`Reward::Contempt`, `Build.contempt`): an enemy you kill bursts for 25% of its maximum health on every other enemy within 3 m (1.5 m more each further Contempt, up to three); what it kills bursts too; a boss never bursts (`world.rs` `contempt`, called from `fell`).
- **Side paths** (`route.rs`): rewards rolled from each zone's `rewards` table (a name listed twice comes twice as often), nothing twice in a zone and nothing the body could not use (`Have`); a two-fight path holds a choice of two (`Node.extra`, `choose`); one path in four (`sealed_pct`) is sealed, shown as Sealed until cleared, holding a gun a level higher or two rewards both yours; the **Purse** (`Reward::Purse`) is three times the zone's side-path Clemency. A gun is `weapon_steps` levels above the gun in hand when it is taken.
- **Art and sound**: `guns/filibuster.png`, `guns/redactor.png`, icons `contempt`, `purse`, `sealed`; the two models and the bar in `draw.rs`; sounds `fire_filibuster`, `fire_redactor`, `cloture`, `redaction`, `contempt`.

## Round 16: the Blaster, the tree, and the river designed (2026-10-18)

The request is in `BRIEF.md`; the journal's Round 16 tab has the entries. SECOND-ORDER R16, X1 to X4, B16 and T16.

- **The Blaster.** The zone-three mini-boss, a small Executor, is renamed everywhere, its id too (`blaster`, patterns `blaster_*`): unlike the Interdictor's rename, no packet asked for the id to stay, and no save holds a mini-boss's id (`meta.beaten` keeps bosses only).
- **The tree** (`crates/core/src/tree.rs`, numbers `TREE_*` in `balance.rs`): every upgrade that needs nothing stands on a ring round a hub; each upgrade stands one step further out than the one it needs, on a line to it; each branch takes a wedge of the circle as wide as its outermost upgrades (a petal reaching a little past its outermost upgrade), in the order of `copy.json` `holding_branches`. The core gives every place as a percentage, each wedge as an SVG path and each line's ends; the page only draws them (`screens.js` `holding`). Rules are unchanged: what may be bought is still `meta.rs`'s. Clicking an upgrade picks it; the card under the tree reads the upgrade pointed at or picked, and its button buys the one picked.
- **The river** (designed, not built): `SECOND-ORDER` T16 holds the questions; the design is the journal's Round 16 tab.
