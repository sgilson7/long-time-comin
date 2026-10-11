# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file.

## Where the work is

Long Time Comin' is a Builder project built in one session on 2026-10-10 from `BRIEF.md` (Sam's request, word for word) and `PLAN.md`. Repository `sgilson7/long-time-comin`, public; a push to `main` runs `./scripts/check.sh` in Chromium on GitHub Actions and deploys `dist/` to Pages only if it passes. Sam approved the repository, Pages and the deploys in advance.

## Live (2026-10-10)

- Game: https://sgilson7.github.io/long-time-comin/ (Actions runs 38061649808 and 38061792472, a re-run of the same commit, both passed check and deploy, after the G9 fix; an earlier docs-only run, 38061274573, failed on the flake G9 fixes). `BUILDER_ORIGIN=https://sgilson7.github.io/long-time-comin tests/browser/check.py` passed in Chromium, Firefox and WebKit against the live site.
- Scholar site: https://sgilson7.github.io/projects/long-time-comin/, commit "Add Long Time Comin' to the games" in `~/Documents/website` (pushed to `scholar-site`); `make check` (57 pages, no findings), `make packet` (44 routes clean), `make gate` passed, then `make deploy`. `make gate-live` failed once straight after the publish and passed on the next run; the first run was probably served cached pages (not confirmed).

## What exists

- **Core** (`crates/core/src`): `player` (run, up to four jumps, dash, grapple, parry, health), `orbs` (volleys and their orbs and waves), `weapons`, `enemy` (five kinds, the Bailiff's phases), `arena` (the seeded builder for seven archetypes and the court), `level` (boxes, grid, rays), `route` (the zone graph), `meta` (the record), `world` (one arena's tick), `draw`, `render` (four passes: solid, facade, ink, glow), `game` (modes, clock, camera, HUD and screen JSON), `content`, `balance`. `fx`, `geom`, `rng`, `mesh` are Food Truck Circus's, with Ring, Blob, Panel and Facade meshes added.
- **Data**: 4 guns of 4 levels, 77 patterns (23 zone two's, 30 zone three's), 25 enemies (seven and a boss in each built zone, and the house's mini-boss), three built zones with their routes and encounters (16, 12, 12), 27 upgrades in four branches, the palette, the story (sentencing, 10 counts, addenda, two interludes, the verdict), copy, controls, sounds, display, shaders.
- **Page**: `app.js` (loop and modes), `hud.js`, `screens.js` (sentencing, Holding, map, indictment, verdict), `sound.js`, Iron Vector's `input.js`, and `gl.js` (Iron Vector's, with the passes).
- **Art**: `tools/art.py` → `web/art/` (title, sentencing, indictment, verdict, 3 guns, 8 icons), TikZ kept in `tools/art/`.
- **Probes** (ignored tests): `crates/core/tests/run_probe.rs` runs a scripted player through whole routes; `crates/core/tests/draw_cost.rs` prints each scene's instances and vertices.

## What passed

`./scripts/check.sh` on 2026-10-10 at commit "Take the pointer-lock fallback under automation": fmt, clippy with warnings as errors, 88 core tests and 11 boundary tests, the release wasm build, the wasm import audit, and the browser check in Chromium, Firefox and WebKit (`BUILDER CHECK PASSED`). Ten behaviours were broken on purpose and each test failed (SECOND-ORDER B1).

## Round 3 (2026-10-10): the playtest

A subagent played seven runs through the playtest harness (`cargo run --release -q -p ltc_core --example playtest -- SESSION_DIR help`) and recommended changes; SECOND-ORDER H1 to P9 record what was done and measured. To measure difficulty again, run consecutive autopilot runs through the harness (commands `start`, `go main`, `fight 150`, `holding`).

## Round 4 (2026-10-10): the see-through camera, the Clerk, zone two

**Live:** pull request #1 (zone-two) passed the full check on CI (run 38085164272: fmt, clippy, 124 core and 11 boundary tests, the wasm build and audit, the browser check) and the break tests (run 38085164277: all twenty breaks fail their tests); merged to `main`, run 38085439338 passed check and deploy. Build 23046805 (commit 6e2b5ee) is live, and `BUILDER_ORIGIN=https://sgilson7.github.io/long-time-comin tests/browser/check.py` passed in Chromium, Firefox and WebKit against it. The core tests could not run on this Mac until Sam restarted `syspolicyd` (Z13); afterwards they passed locally too (124 and 11). Scholar site: the project page rewritten for zone two (commit 8234be5 in `~/Documents/website`, left unpushed under the site's rule 8, beside another session's unpushed commit 156cb5d whose page was already live); `make check` (59 pages, no findings), `make packet`, `make gate`, `make deploy` (build 5f37daac) and `make gate-live` all passed.

`BRIEF.md` has the request; `PLAN.md` (Round 4) the design; `SECOND-ORDER.md` rows R4 and Z1 to Z11 what was chosen and found, and B2 the break tests.

- **Camera.** The shader cuts the arena's scenery out of a cone from the lens to the head and a sphere round the lens (`balance.rs` `SEE_*`). The reported fault had a cause of its own: since round two the camera started every arena behind the shut entry gate, which was not solid (Z1); it is now.
- **Zone one's seventh enemy:** the Clerk of Minutes (`blinker`), in three new encounters.
- **Zone two, the White Waste (2046):** `zone.json` now holds a route per zone; a run crosses from the Bailiff through an interlude into the ice; a run can start on the ice once the Bailiff has been beaten. Arenas in `crates/core/src/arena/ice.rs`; enemies' models in `crates/core/src/draw/beasts.rs`; held formations in `orbs.rs` (`Shape::Form`, `Launch`, `Rehold`); the Injunction (R) in `player.rs` and `world.rs` (`WallPlane`); vents in `arena.rs` (`Vent`) and `world.rs`.
- **Art:** `interlude.png`, `testimony.png` (the verdict), `icons/wall.png`.
- **Harness:** `record <json>` starts from a saved record, `start 1` starts on the ice, `north` leaves the interlude, `tap wall`; the autopilot raises the wall at blue orbs.

## Round 5 (2026-10-11): the Glass House

**Checked:** `./scripts/check.sh` passed locally on 2026-10-11 before the pull request (fmt, clippy, 150 core and 11 boundary tests, the wasm build and audit, the browser check in Chromium, Firefox and WebKit). The 31 new breaks in `scripts/break_tests.py` were run locally and each failed its test (B3). **Live:** pull request #2 passed the check on CI (run 38116058576) and the break tests (run 38115050236, after one break was repointed at a renamed test); merged, run 38116251693 passed check and deploy, and `BUILDER_ORIGIN=https://sgilson7.github.io/long-time-comin tests/browser/check.py` passed in Chromium, Firefox and WebKit against the live site. Scholar site: the page rewritten for zone three (commit 19a60c8 in `~/Documents/website`, unpushed under the site's rule 8); `make check` (62 pages, no findings), `make packet`, `make gate`, `make deploy` (build f0836aba) and `make gate-live` passed. `BRIEF.md` has the request; `PLAN.md` (Round 5) the design; `SECOND-ORDER.md` rows R5, M1 to M15, B3 and T7 what was chosen and found.

- **Story.** The Witness's interlude (WHAT THE WITNESS SAW) carries the run home to the house on Calder Hill in 2056; the verdict is now the Executor's orders, signed by the Defendant. Zone four (2081) stays sealed.
- **The house** (`crates/core/src/arena/house.rs`): nine rooms, walled and roofed 15 m up. `arena.nav` holds each house room's walk grid (`nav.rs`), `arena.panes` the Conservatory's glass.
- **Agents** (`enemy.rs` kind `agent`, `squad.rs`): seven roles; cover and peeking, crossfire and pincer timing, rings while a Handler lives, flanking to your back. New pattern mechanics in `orbs.rs`: `bounce`, `fall`/`plant`, `sweep`, the `lane` form and `Launch::Close`.
- **The Section Chief** (elite, in the Ballroom on main fight four): calls agents (`summon`, `rally` in `enemies.json`; `world.rs` `calls_step`).
- **The Executor** (`tank.rs`, kind `tank`): drive and turret, armour and weak points per phase, cannons grown per phase, a fourth phase outlasted; the glass breaks in `world.rs` `glass_step`. Models in `crates/core/src/draw/house.rs`.
- **The ground pound** (`player.rs`, key C, action bit 1<<12) and **the Ripsaw** (`weapons.rs`, `ShotKind::Saw`, `Lodge`; the rev in `world.rs` `saws_step`).
- **Upgrades and rewards**: level IV guns, Hair Trigger and Plating, fifteen new upgrades; the Holding draws its tree by `branch` (`upgrades.json`, copy `holding_branches`).
- **Art**: `interlude_house.png`, `orders.png` (the verdict), `guns/ripsaw.png`, icons `pound`, `saw`, `conduct`, `trigger`, `plating`.
- **Harness**: `goto <node>|boss` and `break` (via `Game::go_to` and `Game::break_weak_points`); it reports who hurt you after each command.
- **Fixed on the way**: an orb's hit never reached the page's notes, so the hurt sound and a Recess's callout played only for falls (M12).

## Round 6 (2026-10-11): a test save, the boss rush, second fires

**Live:** pull request #3 passed check (run 38117985724) and break tests (run 38117985758); merged, run 38118787560 passed check and deploy; the live three-engine browser check passed. Scholar site page updated (commit in `~/Documents/website`, unpushed), `make deploy` build d19e51c7, `make gate-live` passed. `./scripts/check.sh` passed locally (154 core and 11 boundary tests, the browser check in three engines, now also driving the test save and the boss rush). `BRIEF.md` has the request, `PLAN.md` (Round 6) the design, `SECOND-ORDER.md` R6, N1 to N4 and B4 what was chosen.

- **Test save**: `Meta::everything`, `Game::test_save`, the Holding's `#test-save` (asks twice).
- **Boss rush**: `Route::rush`, `Game::start_rush`, `Run.rush`; the Holding's `#start-rush` once `Meta::rush_open`.
- **Second fires**: `weapons.json` `alt`, `content::Alt`, `weapons::alt_trigger`, the Ripsaw's in `world.rs` `detonate`; action `alt` (1<<13) on the right mouse button; the parry is Q.
- **Stronger guns**: the Scattergun, the Launcher and the Ripsaw, about 30% (N4).

## Round 7 (2026-10-11): the Ascent

`./scripts/check.sh` passed locally (165 core and 11 boundary tests, the browser check in three engines). `BRIEF.md` has the request, `PLAN.md` (Round 7) the design, `SECOND-ORDER.md` R7, Q1 to Q12, B5 and T8 what was chosen and found.

- **Zone four** (`zone.json` `the_ascent`): arenas in `crates/core/src/arena/ascent.rs`, built at a height (`arena::build_at`, `ASCENT_PER_STEP`), walked by agents on the nav grid; seven assassins and the Defendant (`enemies.json`), patterns in `patterns.json`.
- **The Defendant**: kind `double` (`enemy.rs` `skirmish`, `reposition`), attacks with `stop_ms`; the world's `stop` (`world.rs` `TimeStop`) freezes all but its caster.
- **The ending**: `World::ending`, `ending_step`, `melt`; `draw::azathoth`; the shaders' `uFx` (melt, grey), sent in the view values (`fx`, indices 23 and 24).
- **Your time stop**: `Build::stop_ticks`, from `Meta::beaten` and the Stopped Clocks (`unlock` in `upgrades.json`).
- **Grab and throw**: `World::grab_step`, `throw`, `ShotKind::Thrown`.
- **The Big Iron**: `GunLevel::mag`/`shell`, `Held::reload_step`, the fan (`AltKind::Fan`).
- **Mini-bosses**: the Pioneer (kind `mech`, zone two) and the Escort (an elite tank, zone three); `zone.json` `minibosses`.
- **Art**: `door.png` (the verdict), `guns/big_iron.png`, `icons/stop.png`.
- **Harness**: `fell` (via `Game::fell_boss`).

## What remains

- Sam's playtest (`TRIAGE.md`). Every feel and balance number is the agent's guess (SECOND-ORDER T1); the probe shows the curve (no wins with no upgrades, about half with 62 Clemency of upgrades), not whether it is fun (T2, P2).
- Sam's reading of the story's words (T3).
- Zone five (the Idiot God, the heart of it, the trumpet) is sealed, and the hidden zone six (a harder new game and the cat) is not listed; both are described in `BRIEF.md`'s seventh request. Zone five needs its entry in `zone.json`, a builder in `arena.rs`'s style, its enemies and an interlude in `story.json` keyed `the_ascent`.
- Sam's playtest of zone three (T7), and whether the Executor moves as the Atlas of Tyr does (M2).
- Not built: music, a sensitivity setting, pathfinding for zones one and two's walkers (they walk straight and turn at walls; the house's agents walk a grid).

## What the next session must not assume

- That CI checked Firefox and WebKit: it runs Chromium only. Run the three-engine check locally before pushing.
- That CI's Chromium is fast: it draws on SwiftShader. The walk to the first gate waits up to 120 s, and instance count is what costs there (G7).
- That `scripts/serve.py` survives a rebuild: `build.sh` deletes `dist/`.
- That a newly built native program will start on this Mac (Z13). If `cargo test` or the harness sits with no CPU time, check `syspolicyd` (`ps -o time,%cpu,command -p $(pgrep syspolicyd)`); `sudo killall syspolicyd` restarts it. Until then CI runs the tests on a pull request, and `scripts/break_tests.py` runs there through `breaks.yml`.
