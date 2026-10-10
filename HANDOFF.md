# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file.

## Where the work is

Long Time Comin' is a Builder project built in one session on 2026-10-10 from `BRIEF.md` (Sam's request, word for word) and `PLAN.md`. Repository `sgilson7/long-time-comin`, public; a push to `main` runs `./scripts/check.sh` in Chromium on GitHub Actions and deploys `dist/` to Pages only if it passes. Sam approved the repository, Pages and the deploys in advance.

## Live (2026-10-10)

- Game: https://sgilson7.github.io/long-time-comin/ (Actions runs 38061649808 and 38061792472, a re-run of the same commit, both passed check and deploy, after the G9 fix; an earlier docs-only run, 38061274573, failed on the flake G9 fixes). `BUILDER_ORIGIN=https://sgilson7.github.io/long-time-comin tests/browser/check.py` passed in Chromium, Firefox and WebKit against the live site.
- Scholar site: https://sgilson7.github.io/projects/long-time-comin/, commit "Add Long Time Comin' to the games" in `~/Documents/website` (pushed to `scholar-site`); `make check` (57 pages, no findings), `make packet` (44 routes clean), `make gate` passed, then `make deploy`. `make gate-live` failed once straight after the publish and passed on the next run; the first run was probably served cached pages (not confirmed).

## What exists

- **Core** (`crates/core/src`): `player` (run, up to four jumps, dash, grapple, parry, health), `orbs` (volleys and their orbs and waves), `weapons`, `enemy` (five kinds, the Bailiff's phases), `arena` (the seeded builder for seven archetypes and the court), `level` (boxes, grid, rays), `route` (the zone graph), `meta` (the record), `world` (one arena's tick), `draw`, `render` (four passes: solid, facade, ink, glow), `game` (modes, clock, camera, HUD and screen JSON), `content`, `balance`. `fx`, `geom`, `rng`, `mesh` are Food Truck Circus's, with Ring, Blob, Panel and Facade meshes added.
- **Data**: 3 guns of 3 levels, 47 patterns (23 of them zone two's), 16 enemies (seven and a boss in each built zone), two built zones with their routes and encounters (16 and 12), 12 upgrades, the palette, the story (sentencing, 10 counts, addenda, an interlude, the verdict), copy, controls, sounds, display, shaders.
- **Page**: `app.js` (loop and modes), `hud.js`, `screens.js` (sentencing, Holding, map, indictment, verdict), `sound.js`, Iron Vector's `input.js`, and `gl.js` (Iron Vector's, with the passes).
- **Art**: `tools/art.py` → `web/art/` (title, sentencing, indictment, verdict, 3 guns, 8 icons), TikZ kept in `tools/art/`.
- **Probes** (ignored tests): `crates/core/tests/run_probe.rs` runs a scripted player through whole routes; `crates/core/tests/draw_cost.rs` prints each scene's instances and vertices.

## What passed

`./scripts/check.sh` on 2026-10-10 at commit "Take the pointer-lock fallback under automation": fmt, clippy with warnings as errors, 88 core tests and 11 boundary tests, the release wasm build, the wasm import audit, and the browser check in Chromium, Firefox and WebKit (`BUILDER CHECK PASSED`). Ten behaviours were broken on purpose and each test failed (SECOND-ORDER B1).

## Round 3 (2026-10-10): the playtest

A subagent played seven runs through the playtest harness (`cargo run --release -q -p ltc_core --example playtest -- SESSION_DIR help`) and recommended changes; SECOND-ORDER H1 to P9 record what was done and measured. To measure difficulty again, run consecutive autopilot runs through the harness (commands `start`, `go main`, `fight 150`, `holding`).

## Round 4 (2026-10-10): the see-through camera, the Clerk, zone two

`BRIEF.md` has the request; `PLAN.md` (Round 4) the design; `SECOND-ORDER.md` rows R4 and Z1 to Z11 what was chosen and found, and B2 the break tests.

- **Camera.** The shader cuts the arena's scenery out of a cone from the lens to the head and a sphere round the lens (`balance.rs` `SEE_*`). The reported fault had a cause of its own: since round two the camera started every arena behind the shut entry gate, which was not solid (Z1); it is now.
- **Zone one's seventh enemy:** the Clerk of Minutes (`blinker`), in three new encounters.
- **Zone two, the White Waste (2046):** `zone.json` now holds a route per zone; a run crosses from the Bailiff through an interlude into the ice; a run can start on the ice once the Bailiff has been beaten. Arenas in `crates/core/src/arena/ice.rs`; enemies' models in `crates/core/src/draw/beasts.rs`; held formations in `orbs.rs` (`Shape::Form`, `Launch`, `Rehold`); the Injunction (R) in `player.rs` and `world.rs` (`WallPlane`); vents in `arena.rs` (`Vent`) and `world.rs`.
- **Art:** `interlude.png`, `testimony.png` (the verdict), `icons/wall.png`.
- **Harness:** `record <json>` starts from a saved record, `start 1` starts on the ice, `north` leaves the interlude, `tap wall`; the autopilot raises the wall at blue orbs.

## What remains

- Sam's playtest (`TRIAGE.md`). Every feel and balance number is the agent's guess (SECOND-ORDER T1); the probe shows the curve (no wins with no upgrades, about half with 62 Clemency of upgrades), not whether it is fun (T2, P2).
- Sam's reading of the story's words (T3).
- Zones three and four and the final boss are sealed. `zone.json` lists them; a third zone needs its entry in `zone.json` (scene, route, archetypes, names, encounters), a builder in `arena.rs`'s style, its enemies and an interlude in `story.json`.
- Not built: music, a sensitivity setting, enemy pathfinding (walkers walk straight and turn at walls).

## What the next session must not assume

- That CI checked Firefox and WebKit: it runs Chromium only. Run the three-engine check locally before pushing.
- That CI's Chromium is fast: it draws on SwiftShader. The walk to the first gate waits up to 120 s, and instance count is what costs there (G7).
- That `scripts/serve.py` survives a rebuild: `build.sh` deletes `dist/`.
- That a newly built native program will start on this Mac (Z13). If `cargo test` or the harness sits with no CPU time, check `syspolicyd` (`ps -o time,%cpu,command -p $(pgrep syspolicyd)`); `sudo killall syspolicyd` restarts it. Until then CI runs the tests on a pull request, and `scripts/break_tests.py` runs there through `breaks.yml`.
