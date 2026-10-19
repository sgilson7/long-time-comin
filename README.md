# Long Time Comin'

A third-person bullet-hell roguelike that runs in the browser. You committed a crime in 2081 the court calls the Stilling, and were sentenced to go back and stop yourself. The court's Bailiff aimed fifty years early. You land in 2031 and have to cross the half century on foot, one arena at a time, through rings, spirals and flowers of large orbs, north across the ice, and home to a house under siege.

**Play:** https://sgilson7.github.io/long-time-comin/

- **Runs.** Zone one, *The Waiting City, 2031*, is a graph of arenas: one main path from the crater where you land to the Court of Hours, and side paths of harder fights that rejoin it and end in a reward you can see on the map before you choose (a better gun, more health, more speed, another jump, a passive such as Contempt, under which what you kill bursts, or a purse of Clemency). A longer side path holds a choice of two, and some are sealed until you clear them. Every run rolls its own graph and its rewards.
- **Seven guns.** Carbine (automatic, accurate), Scattergun (pellets, close), Hourglass Launcher (a slow orb that bursts), the Filibuster (an automatic rifle that fires faster the longer it is held), on the ice the Redactor (an automatic rifle whose rounds strike enemy orbs from the air), in the house the Ripsaw (after Saros: a click throws a saw blade that lodges in what it hits, and holding the trigger revs every lodged blade), and on the mountain the Big Iron. Levels I to V; each has a second fire on the right mouse button (a burst, a slug, a cluster of bombs, the saws' shatter, the hammer fanned, a lance that spends the spin, a bar that wipes orbs away); one carried at a time, as in Returnal and Tower of Guns. You choose the gun you carry in from those whose Exhibit you have bought in the Holding, and every gun a side path offers is a level or two above the one in your hand.
- **Orbs.** Enemies fire volleys from `data/patterns.json`: rings, aimed fans, spirals (each repeat turned a little), flowers (orbs curling both ways), blooms (fast out, then hanging), spheres, and ground waves you jump. A ring fired from a roof tilts through you.
- **Moving.** Run, up to four jumps, a dash that orbs pass through, and air control close to the ground's, after Returnal, Saros and Iron Vector.
- **The Bailiff of Hours**, the zone's boss, has three phases and calls help at each. Beating it does not end the run: an interlude (the years between, a company built on knowing what comes next) carries you into zone two.
- **Zone two**, *The White Waste, 2046*: an expedition to the ice to fetch the Observatory's key. Five new arenas (a crevasse with ice bridges, stepped plateaus, a frozen icebreaker, a field of radio masts round a catwalk, an ice cave), updraft vents, seven new enemies and the Witness. Their patterns are held formations: orbs placed in a ring, grid, line, column, star, spiral or ribbon, held, then released together or in turn, outward, inward, at you or at where you are heading. Once the Bailiff has been beaten, a run can start on the ice. Beating the Witness does not end the run either: it tells you what you will do, and cannot finish the sentence.
- **Zone three**, *The Glass House, 2056*: your own house on Calder Hill, ten years of laboratories, raided by the Continuance, men in grey suits who all have one face. Rooms after Resident Evil's mansion and Ultrakill's labs (a great hall with twin stairs, a dining room, a library of aisles, a laboratory broken through into its basement, a long gallery, a ballroom), with ceilings 15 m up. Seven new enemies are people: they walk the floors and stairs, hide out of your sight and step out to fire, wait for a partner so two of them fire on the same tick from either side of you, go round to your back, ring you, lob mines over cover and close walls of orbs on either side of you. The Section Chief calls them in batches. The Executor, a tank, comes in through the conservatory's glass wall; only the glowing weak points on its back take damage, each phase grows more cannons, and its fourth phase, wheels within wheels covered in eyes, cannot be hurt: outlast it.
- **Death.** The court reads the next count of your indictment and adds years to your sentence (Sifu's count: 1, then 2 more, then 3 more). You return to the Holding, where the Clemency the run earned buys upgrades: more health and speed, a double jump, a third dash, a better starting gun, Recess (survive one killing blow), **Gavel Drop** (C in the air: a ground pound that sends a shockwave along the floor and bounces you back up past where you started), second levels of the hook, the parry and the wall, and more, the **Writ of Reach** (a grappling hook after Risk of Rain 2's Loader: hold F and keep the speed when you let go), **Objection** (a parry after Saros: gold orbs that reach you inside it fly back at whoever fired them) and the **Injunction** (R: a wall in front of you for half a second that breaks blue orbs).
- **The camera** never hides you: scenery between it and you is cut away in the shader.
- **Zone four**, *The Ascent, 2081*: the evening of the Stilling, always 21:14. You climb Mount Meridian to the Observatory, each arena higher up the mountain than the last. The mountain's own creatures are turning: harts whose antlers end in clock hands, rooks with eyes along their wings, toads, a man-eating plant, a river risen as a body of water, toadstools and bears, taken by the ritual's clockwork and the Idiot God's eyes, more the higher you climb. With them come some enemies from the zones below and the assassins in your own coat that your older self sent. At the summit, in a circle of stones round the Long Door, is the Defendant: you, older, with all your abilities and one more: they stop time, hold you still, step round you and lay their shots, then let time go. Beating them gives you the time stop (T). What comes after is the revelation.
- **Zone five**, *The Idiot God, Outside*: inside the thing your ritual let in. The spaces are impossible, after Ultrakill's trick-art layer and games like Manifold Garden, Antichamber and Hyperbolica: fall off any floor and you come down on it again from above, the arena repeating above and below you; gold doorways lead to other doorways, for you and for orbs; a stair of four flights climbs for ever; far off, space curls up into a bowl and breathes. Seven new enemies each look as if they came from another game: an eight-bit invader that marches, a vector-display tesseract that folds through four dimensions, an engraving in one-bit dither that moves only when you are not looking at it, a PlayStation-era hound with tank controls, rubber-hose cartoons that bounce on one beat, a photographic negative of you that walks the path you walked three seconds ago, and a golden fractal that splits into smaller copies of itself. The Idiot God has a bar for each of its three phases. When the last empties, you take out a trumpet: its dancers pipe a verse, and you play it back on five keys (left mouse, Q, E, R, F) in time, after the ocarina, while it fires on the beat. Four verses put it to sleep in a bubble of stopped time.
- **The River**, a second timeline out of 2056, chosen on the Holding's timeline: past the Executor, a run goes down the river Calder instead of up the mountain. *The Water Gap, 2066* is all platforming, four long sections with no fights: the Spillway (the house's old weir), the Narrows (a slot canyon), the Falls (a cascade 120 m down) and the Shallows (Lake Stillwater's flooded wood). You cross them with the hook, Bound Over (Shift and Space together: a long low leap), the ground pound, and **the Bench** (V, bought in the Holding: a plank you lay under you in the air, stand on and shoot through), past levers you hook to open gates for a few seconds, rotten floors you pound through, roots that rise and sink, and walls of the river's own green orbs that no gun or wall can erase. The water drowns you: back to the last mooring post, a tenth of your health the poorer. At the lake's edge the Gallows Oak: hook up 150 m of its trunk while its eye opens just above you, and at the crown fight the Summons, a floating knot of vines after Armored Core 6's Balteus, with a Husk to break before its health and a Bloom at half health.
- **The Far Shore and the Vault**, the River's last two zones. Where the Oak's roots were there is a light under Lake Stillwater: an anomaly, with an ocean inside it. *The Far Shore* is four long sections, fought: seven pirate ships at anchor to fight across (the Drowned Fleet: deckhands, gunners, powder monkeys, lookouts, bosuns, and their cockatrices, chimeras and an Enslaved Dragon), the fleet running through a hurricane (gusts, lightning, storm harpies), a rest in a capsized hull, and everything under the water, wreckage spiralling down into a whirlpool, where Dagon rises in the eye. Beyond it, at the bottom of the lake, *the Vault*: an alien base older than the Oak, down its shaft, through its halls of machines, to the Eraser, which you charge pylon by pylon while your older self comes through to stop you. It fires, and erases the Defendant, and the Stilling, and you: THE GREY ENDING.
- **Mini-bosses**: the Pioneer (a mech after Iron Vector's) on the ice, the Blaster (a small Executor) and the Section Chief in the house.
- **Grab and throw** (G): lift an enemy at half health or less and throw it into the others.
- **The Big Iron**: a six-shot hand cannon, heavy and slow, loaded one shell at a time; its second fire fans the hammer.
- **The Holding** lays its 41 upgrades out as a passive tree after Path of Exile 2's: a ring round a hub, each branch bursting out in its own petal (Body, Motion, Arms, Writs). Its timeline is where a run starts: the years of the main line, and the River forking after 2056. Once the game is beaten it offers a **boss rush**: each zone's boss, one after another. A **test save** button loads a record with everything bought and every zone open. A faint ♪ under the credits starts the Idiot God's lullaby at once, and a faint 🐾 beside it fights Chumbles with everything bought, for practice: winning, dying or abandoning either returns to the Holding and changes nothing on the record.
- **The last sentence**, opened by putting the Idiot God to sleep: another of you is sent back, and must cross zones one to four in one go, harder (every fight's waves half as many again, more of them at once, and fourteen of the simpler enemies' patterns remixed). At the summit reality breaks, and you wake in your bedroom on a Saturday morning (zone `home`): your cat, Chumbles, walks out of the door, and downstairs he screams for treats, grows to three times his size, and fights in five bars, each moving differently. He is the hardest boss in the game.
- **The route** has four bosses and then a final one, the Idiot God, at the heart of zone five.

## Controls

| Input | Does |
| --- | --- |
| W A S D | Move |
| Mouse | Look and aim |
| Left mouse (hold) | Fire (the Ripsaw: click to throw, hold to rev) |
| Right mouse | The gun's second fire: burst, slug, cluster, shatter, fan, lance or bar |
| Space | Jump, and again in the air |
| Shift | Dash: orbs pass through you |
| E | Take a gun from the floor |
| F (hold) | Grappling hook, once bought |
| Q | Parry gold orbs, once bought |
| R | The Injunction: a wall that breaks blue orbs, once bought |
| C (in the air) | Ground pound, once bought |
| G | Grab an injured enemy; G again throws it |
| T | Stop time, once the Defendant is beaten |
| Tab or M | Map |
| Esc | Pause |
| N | Sound on or off |

## How it is built

Long Time Comin' is a [Builder](https://github.com/sgilson7/builder-setting) project, and its parents are Sam's other games.

- **From Iron Vector:** the 3D engine. Q16.16 fixed-point maths with no floats in the core, one instanced draw per mesh, the core computing every matrix, the camera and the HUD, pointer-lock input, box collision moved one axis at a time, and a fixed 60 ticks a second drawn between ticks.
- **From Food Truck Circus:** the sphere, cylinder and cone meshes, the build and check scripts, and the TikZ art pipeline after `Sam_Comm_final.png`.
- **New here:** two render passes, an ink outline (the inverted hull: each inked shape drawn again a little larger with its front faces culled, which leaves the plum line of the TikZ art) and an additive glow for the orbs; toon-banded light; a seeded arena builder that walls each square of the city in facades with lit windows, rooftop clutter, ivy, shops, a skyline out to the clouds and a clock tower stopped at 21:14.

The layers:

- **`crates/core`** holds every rule: movement, orbs and patterns, guns, enemies, the boss, arenas, the route, the record, the camera and the HUD. No input or output, no floats, no clock.
- **`crates/shim`** moves values across the WebAssembly boundary and decides nothing.
- **`web/`** is a static page with a WebGL 2 canvas. It draws what the core returns.
- **`data/`** holds every gun, enemy, pattern, zone rule, upgrade, colour, word, sound recipe and shader.
- **`tools/art.py`** draws every picture as TikZ; the sources are kept in `tools/art/`.

```bash
./scripts/setup.sh   # once: Playwright and three browser engines
./scripts/check.sh   # fmt, clippy, the core tests, the wasm build and audit, the browser check
python3 scripts/serve.py 8000
```

A push to `main` runs the same check in Chromium and deploys `dist/` to GitHub Pages if it passes.

`BRIEF.md` holds the request, `PLAN.md` the decisions, `SECOND-ORDER.md` what was found while building, and `HANDOFF.md` where the work stands.

## Credits

Made by Sam Gilson. Inspired by Returnal, Saros, Realm of the Mad God, Hades, Risk of Rain 2, Tower of Guns and Sifu; it uses nothing of theirs.
