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

Live: PR #4 passed both workflows (check 38122273822, breaks 38122273789) and merged as 04f7cb3; the main run 38123497839 deployed it, and the live browser check against https://sgilson7.github.io/long-time-comin passed in chromium, firefox and webkit. The website was deployed as build 853fa91c; `make gate-live` passed.

## Round 8 (2026-10-11): the Defendant's moves

`./scripts/check.sh` passed locally (169 core and 11 boundary tests, the browser check in three engines), and every break in `scripts/break_tests.py` fails its test. `BRIEF.md` has the request, `PLAN.md` (Round 8) the design, `SECOND-ORDER.md` R8, Q13 to Q17, B6 and T9 what was chosen and found.

- **Hook and pound**: an attack's `move` (`content.rs` `Lunge`); `enemy.rs` `lunge_spot`, `lunge_step`, `hooking`; numbers `LUNGE_*` in `balance.rs`; drawn in `draw/house.rs` `double`.
- **The stop a quarter as often**: an attack's `once_in`; `enemy.rs` `pass_over`.
- **The dome**: `Form::Dome` and `Launch::Gather` (`orbs.rs`); the patterns `the_world`, `queued_knives`, `the_world_2`; the wall is `injunction_echo`.
- The Defendant's health is 4,000.

Live: PR #5 passed both workflows (check 38152218054, breaks 38152218086) and merged as 3d295fe; the main run 38153832412 deployed it, and the live browser check passed in chromium, firefox and webkit. The website's page was left as it is: nothing on it describes what changed.

## Round 9 (2026-10-11): the mountain's creatures, the wall's wards, the Escort fix

`./scripts/check.sh` passed locally (176 core and 11 boundary tests, the browser check in three engines), and every break in `scripts/break_tests.py` fails its test. `BRIEF.md` has the request, `PLAN.md` (Round 9) the design, `SECOND-ORDER.md` R9, Q18 to Q25, B7 and T10 what was chosen and found.

- **Creatures**: kind `beast` and its `Gait` (`content.rs`), moved by `Enemy::beast` (`enemy.rs`), the bite by `Enemy::head` and `contact`; numbers `BOUND_*`, `HOP_*`, `WHEEL_*`, `BITE_*`, `SURGE_*` in `balance.rs`; drawn in `draw/fauna.rs`.
- **Corruption**: `World::corrupt` from the node's step (`CORRUPT_*`), given to each enemy as it spawns.
- **Wards**: `palette.json` `wards`, `Volley::ward` and `Orb::ward`, `Build::wards`; the upgrades `standing_order` and `permanent_injunction`.
- **The Escort fix**: an elite is never the arena's boss (`World::spawn`), so felling the Escort no longer ends zone three.
- **Harness**: `hit_by` cleared each tick and set by waves; the autopilot avoids drops and leads moving targets.

Live: PR #6 passed both workflows (check 38158683100, breaks 38158683105) and merged as 7d009d4; the main run 38160592662 deployed it, and the live browser check passed in chromium, firefox and webkit. The website's page was updated and deployed as build 8b75088c; `make gate-live` passed.

## Round 10 (2026-10-11): the Great Horned, and big fights

`./scripts/check.sh` passed locally (179 core and 11 boundary tests, the browser check in three engines), and every break in `scripts/break_tests.py` fails its test. `BRIEF.md` has the request, `PLAN.md` (Round 10) the design, `SECOND-ORDER.md` R10, Q26 to Q29, B8 and T11 what was chosen and found.

- **The Great Horned**: `Gait::Perch`, `Enemy::perching`, `perches`, `swoop_tell` (`enemy.rs`); `PERCH_*` and `SWOOP_*` in `balance.rs`; drawn in `draw/fauna.rs` `owl`; zone four's `minibosses`.
- **Fifteen at most**: `MAX_ALIVE`, `World::room`, `wave_open`.
- **Big waves**: zone four's `encounters` in `zone.json`.

Live: PR #7 passed both workflows (check 38163993223, breaks 38163993270) and merged as c053414; the main run 38165969745 deployed it, and the live browser check passed in chromium, firefox and webkit. The website's page was updated and deployed as build f93857c1; `make gate-live` passed.

## Round 11 (2026-10-11): the Idiot God

`./scripts/check.sh` passed locally (207 core and 11 boundary tests, the browser check in chromium, firefox and webkit), and each of the round's 36 new breaks fails its test. `BRIEF.md` has the request, `PLAN.md` (Round 11) the design, `SECOND-ORDER.md` R11, Q30 to Q43, B9 and T12 what was chosen and found. Two parts were built by agents in their own worktrees and merged: the arenas (`arena/god.rs`) and the drawing (`draw/god.rs`).

- **The song**: `song.rs` (the lullaby's parts, judging, the beat, the finale), `World::lullaby`, the god's `floor_hp`; the data is the god's `lullaby` in `enemies.json`.
- **Impossible space**: `Arena::wrap` and `repeated`, `Seam` (`arena.rs`), `World::wrap_and_seams`, orbs and shots through seams; the bend in `scene.vert` (`uFx.z`, by each shape's centre).
- **Styles**: `PASS_DITHER`, `PASS_PS1`, `PASS_NEGATIVE` and `PASS_ORDER` (`render.rs`), `Batch::style`; the shaders.
- **The seven**: gaits `march`, `unseen`, `charge`, `bounce`, `echo` (`enemy.rs`); `splits`; the world tells each enemy the clock (`beat`), whether you look at it (`seen`) and where you were (`trail`).
- **HUD**: a bar per phase (`story.json` `bars`), the staff (`Game::song_hud`, `#staff`).
- **The ending**: `story.json` `interludes.the_ascent` (THE DOOR), the verdict THE LULLABY, `web/art/lullaby.png`.

Live: PR #8 passed both workflows (check 38179829228, breaks 38179829239, after one round-seven break was repointed and the break workflow's limit raised to 90 minutes: it took 43) and merged as 742bb98; the main run 38182675685 deployed it, and the live browser check passed in chromium, firefox and webkit. The website's page was updated (commit cbf864e in `~/Documents/website`, unpushed under its rule) and deployed as build 23b9c76a; `make gate-live` passed.

## What remains

- Sam's playtest (`TRIAGE.md`). Every feel and balance number is the agent's guess (SECOND-ORDER T1); the probe shows the curve (no wins with no upgrades, about half with 62 Clemency of upgrades), not whether it is fun (T2, P2).
- Sam's reading of the story's words (T3).
- The hidden zone six (a harder new game and the cat) is not listed; it is described in `BRIEF.md`'s seventh request.
- Sam's playtest of zone five (T12): the lullaby's keys and timing, the god's length, whether the spaces are trippy enough.
- Sam's playtest of zone three (T7), and whether the Executor moves as the Atlas of Tyr does (M2).
- Not built: music, a sensitivity setting, pathfinding for zones one and two's walkers (they walk straight and turn at walls; the house's agents walk a grid).

## What the next session must not assume

- That CI checked Firefox and WebKit: it runs Chromium only. Run the three-engine check locally before pushing.
- That CI's Chromium is fast: it draws on SwiftShader. The walk to the first gate waits up to 120 s, and instance count is what costs there (G7).
- That `scripts/serve.py` survives a rebuild: `build.sh` deletes `dist/`.
- That a newly built native program will start on this Mac (Z13). If `cargo test` or the harness sits with no CPU time, check `syspolicyd` (`ps -o time,%cpu,command -p $(pgrep syspolicyd)`); `sudo killall syspolicyd` restarts it. Until then CI runs the tests on a pull request, and `scripts/break_tests.py` runs there through `breaks.yml`.

## Round 12 (2026-10-11): the lullaby practice

A faint ♪ at the foot of the Holding starts the Idiot God's lullaby at once (`Game::practice_lullaby`, SECOND-ORDER R12). Nothing from it is kept on the record. `./scripts/check.sh` passed (BUILDER CHECK PASSED, the browser check in Chromium, Firefox and WebKit now clicks the ♪ and abandons it); the two new breaks each fail their test.

## Round 13 (2026-10-12): the last sentence, and Chumbles

The last mission (PLAN.md Round 13; SECOND-ORDER R13, Q44 to Q51, B11, B12, T13), opened by putting the Idiot God to sleep: zones one to four in one go as new game plus, reality breaking at the summit, and the home zone (`zone.json` `home`, marked `mission`): the bedroom, the landing, and Chumbles downstairs (kind `cat`, five bars each with its own gait). The house is `arena/home.rs` and Chumbles `draw/cat.rs`, each built by a subagent in its own worktree and merged; the two story pictures are `waking` and `chumbles`. A faint 🐾 in the Holding opens his five bar names; each fights him with everything bought from that bar, nothing kept (Q52, B13). The playtest harness gains `mission`, `home` and `testsave`.

`./scripts/check.sh` passed before the merge to main (BUILDER CHECK PASSED; 227 core tests, the browser check in Chromium, Firefox and WebKit, which now plays the 🐾 and opens the mission's brief). The round's 22 new breaks each fail their test (B11, B12); four older breaks whose lines this round changed were repointed.

Live (2026-10-12): PR #10 merged as f472978 once its check passed (its break run, all breaks failing their tests, finished after); main's run 38274461715 deployed it, and `tests/browser/check.py` against https://sgilson7.github.io/long-time-comin passed in Chromium, Firefox and WebKit. The scholar site's page was rewritten shorter, on the game's design decisions (website commit 0d71292, build 9375e52c); its first `make deploy` was refused because the Pages repository had a newer publish from another session (build a9a6d21b, which differed from this build only in build ids), so `.pages` was reset to it and the publish re-run; `make gate-live` passed.

## Round 14 (2026-10-15): the rewrite

Sam's prose packet (`long_time_comin_rewrite_v2/`, kept out of the repository) applied with its own installer on branch `narrative/clearer-stilling`: `data/story.json` (all narrative text), 40 keys of `data/copy.json`, and the first boss's name in `data/enemies.json` (The Interdictor; id `bailiff` and all mechanics unchanged). `./scripts/check.sh` passed (BUILDER CHECK PASSED: fmt, clippy, 228 core and 11 boundary tests, the wasm build and audit, the browser check in Chromium, Firefox and WebKit). Every story branch and both endings were read through the playtest harness, and the main screens checked in Chromium at two sizes (SECOND-ORDER R14, Q53). Left for Sam: the Codex line that still says "from the Bailiff on" (Q54), art that shows the courier and the boss as one figure (Q55), an addendum that claims progress whether or not there was any (Q56), the README's first-boss line (Q57).

Live (2026-10-15): PR #12 passed its check (run 38684142976) and merged as 3cefdb3; main's run 38684567129 passed check and deploy, and `tests/browser/check.py` against https://sgilson7.github.io/long-time-comin passed in Chromium, Firefox and WebKit. The scholar site's page was not changed.

## Round 15 (2026-10-15): the arsenal

Designed first, in the design journal's Round 15 tab, and built after Sam's answers (both in `BRIEF.md`), on branch `arsenal`. Two rifles in the Carbine's family: the Filibuster (spins up the longer it is held; Cloture spends the spin on one lance) and the Redactor (less damage, its rounds erase enemy orbs; Redaction is a bar that wipes them away). Six Exhibits in the Holding's Arms column and a Carry in row choose the gun every run starts with. Guns go to level V; the Holding's Armory to III; every gun a side path offers is a level or two above the gun in hand. Contempt (what you kill bursts) on side paths; side-path rewards rolled from a table, nothing twice in a zone and nothing the body could not use, a choice of two on a long path, sealed rewards, and the Purse. `PLAN.md` (Round 15) has the design; SECOND-ORDER R15, W1 to W9, B15 and T15 what was chosen and found.

Checked locally: `./scripts/check.sh` steps 1 to 5 passed (fmt, clippy with warnings as errors, 241 core and 11 boundary tests, the release wasm build, the import audit). Step 6, the browser check, timed out on its first click in headless Chromium on this Mac: other sessions' builds and browser tests held the processor (the title drew 2 frames a second, and the live site, unchanged, drew the same), so the browser check was left to CI, which runs the same script before any deploy. `python3 scripts/break_tests.py arsenal "cache of two" "second fire's cooldown"`: every break fails its test (B15). In-game screenshots of each new gun are in the journal (W9).

Live (2026-10-15): PR #13 passed its check (run 38758384806, the browser check in Chromium included) and the break tests (run 38758384932, all 178 breaks fail their tests); merged as 12340c8; main's run 38767955545 passed check and deploy, and the live `data/weapons.json` holds the Filibuster. `tests/browser/check.py` against the live site timed out on its first click from this Mac, for the same reason as the local step 6 (other sessions holding the processor); it is worth running again when the Mac is quiet. The scholar site's page was not changed.

## Round 16 (2026-10-18): the Blaster, the tree, the river designed

The Escort is the Blaster (id `blaster` too, SECOND-ORDER X1). The Holding's shop is a passive tree after Path of Exile 2's: `crates/core/src/tree.rs` places every upgrade on a ring round a hub or a step out from what it needs, each branch in its own petal (`TREE_*` in `balance.rs`); `screens.js` draws it, and a card under it buys the upgrade picked (X2, X3). The river branch after the Executor (the Water Gap: four platforming sections, the Bench, Bound Over, the Gallows Oak and the Summons) is designed in the journal's Round 16 tabs with Sam's questions (T16) and not built.

Checked locally: `./scripts/check.sh` passed (BUILDER CHECK PASSED: fmt, clippy, 244 core and 11 boundary tests, the wasm build and audit, the browser check in Chromium, Firefox and WebKit, which now counts the tree's wedges and lines). `python3 scripts/break_tests.py "tree:"`: each of the four new breaks fails its test (B16).
