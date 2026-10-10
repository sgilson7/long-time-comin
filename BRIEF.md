# Brief

Stage 1, written by the person. `C1 Frame the problem and set the rules`, `B1 Establish context`.

## The request, word for word

> Please use the Builder repo as a base in the directory above, and use the work and repos of Sam gilson at sgilson7.github.io, which can all be found one directory up. I want to make a 3rd person shooter game in the style of saros and returnal and relam of the mad god, so enemies release these intricate patterns of large orbs that move outwards and in crazy patterns, and you have to dodge them while hitting back, like a bullet hell style game. I want the physics and 3d design to come from iron vector and food truck circus. the gameplay should be roguelike, with a map you try and reach the end of, and every time you die you are reset, and there are 3 bosses along the way then a final boss, kinda like hades. after each run you get some points to spend that make your overall survivability higher, and unlock new mechanics for interacting with the game, stuff like a grappling hook like the loader from risk of rain 2, or parrying specific colors of shots and shooting them back like in saros. the story of the game should be you comitted some great sin in the future and as punishment were sent back in time to stop yourself from committing the sin, but you were sent 50 years too early, so the game is like shifu in that regard. every time you die, the terms of your indictment and case start to  appear and send you back to the start of the game, where you can spend points you acquired from the success of your previous run to improve yourself before starting a new run. tower of guns is another example for an inspiration you can use. I want the MVP built with 3 different guns, and the first zone fully built out, with as much detail in the environment as saros / returnal (i.e nodal areas in a graph structure, where there is one main path that leads from start to end, and each node can potentially have side paths that provide benefits to completing them like leveled up versions of guns (like returnal / saros), max health pickups, speed pickups, extra jumps up to a potential quad jump you are allowed to do. when you are in the air, you should be able to have control of yourself pretty well like saros and returnal and iron vector. please build out the MVP and deploy it to my website. you have approval for creating a public github repo, github pages, and approval to deploy to my website when its ready. any time you need to make 2D art and any kind of asset for the game, use the tikz latex prompt you can find in other repos, and use sam comm final as an inspiration image, alongside the menu screens of all of the sgilson7 games which use a similar build pipeline for images. now build the mvp for the game and deploy it, dont stop till its deployed, good luck

## The fourth request, word for word (2026-10-10)

> the game and story and difficulty are really great so far. the only change i have is making so if your camera goes inside of an object, it doesnt obfuscate your view, instead shows you a transparency / you dont see the object. also add one new enemy type to the first zone.  then I want you to build the second area, where after you defeat the baliff of hours, you become a successful business owner in this time frame, then you fund an expedition into the icy wastes to find something to thelp you in your quest to stop yourself, which is the setting for the second time frame. for the 2nd area, generate all new art assets, types of verticality, new arena layouts taken from returnal / saros if need be or other arena based games, one new ability in the style of the hook; maybe something like a wall you can project in front of you for half a second that destroys specifically blue pellets. also generate the same number of enemies as the 1st area as for the second; their shot patterns should be completely unique in the second area, and the shot patterns should use more delay based firing patterns like the shape floats in mid air, fires out the outside ring of shots, then fires the inside ring at double the speed of the outside ring. please work until the 2nd zone is finished and deployed

Done means, for this round: a camera inside or behind an object sees through it; a seventh enemy in zone one; zone two (after the Bailiff, the years as a business owner, the expedition to the ice) built as zone one is, with its own art, verticality, layouts, seven enemies and a boss whose patterns are all new and built on held formations; a projected wall (bought in the Holding) that breaks blue orbs for half a second; deployed. Approval for the deploy is the standing one above.

## The tool

A third-person bullet-hell shooter that runs in a desktop browser with a keyboard and mouse. Roguelike runs across a graph of arenas; meta-progression between runs; a story told by an indictment that is revealed one count at a time, on each death.

## Done means

- Three guns, each with levels found on side paths.
- Zone one fully built: a node graph with one main path from the arrival to the zone's boss, and side paths whose rewards are a levelled gun, max health, speed, or an extra jump (up to four jumps).
- Enemies fire large orbs in patterns (rings, spirals, flowers, waves, aimed bursts), and the player dodges and shoots back, with good air control.
- Death resets the run, shows the next count of the indictment, and returns to the start, where points earned by the run buy survivability and new mechanics (a Loader-style grappling hook; parrying gold orbs back at their source).
- The full route (three bosses then a final boss) is laid out; zones two to four are sealed in the MVP.
- Art made as TikZ in the style of `Sam_Comm_final.png` and the other games' menu screens.
- A public repository, GitHub Pages, and a page on Sam's website.

## Decisions that stay with the person

- Feel and difficulty: every number in `crates/core/src/balance.rs` is the agent's guess until Sam plays it.
- The story's words (`data/story.json`).
- Zones two to four.

## Autonomy

Sam approved in advance: creating the public repository `sgilson7/long-time-comin`, GitHub Pages for it, and deploying the game and its page on the scholar site when the check passes. The plan is approved in advance. Where it leaves a choice open the agent takes its recommendation and records it in `SECOND-ORDER.md`.

## Allowed core dependencies

`serde`, `serde_json` (reference default).
