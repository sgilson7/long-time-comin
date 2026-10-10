# Handoff

Stage 9, written by the agent. `C5 Consolidate in your own words`. A fresh session starts from this file.

## Where the work is

Long Time Comin' is a Builder project built in one session on 2026-10-10 from `BRIEF.md` (Sam's request, word for word) and `PLAN.md`. Repository `sgilson7/long-time-comin`, public; a push to `main` runs `./scripts/check.sh` in Chromium on GitHub Actions and deploys `dist/` to Pages only if it passes. Sam approved the repository, Pages and the deploys in advance.

## Live (2026-10-10)

- Game: https://sgilson7.github.io/long-time-comin/ (Actions run 38060673004: check and deploy passed). `BUILDER_ORIGIN=https://sgilson7.github.io/long-time-comin tests/browser/check.py` passed in Chromium, Firefox and WebKit against the live site.
- Scholar site: https://sgilson7.github.io/projects/long-time-comin/, commit "Add Long Time Comin' to the games" in `~/Documents/website` (pushed to `scholar-site`); `make check` (57 pages, no findings), `make packet` (44 routes clean), `make gate` passed, then `make deploy`. `make gate-live` failed once straight after the publish and passed on the next run; the first run was probably served cached pages (not confirmed).

## What exists

- **Core** (`crates/core/src`): `player` (run, up to four jumps, dash, grapple, parry, health), `orbs` (volleys and their orbs and waves), `weapons`, `enemy` (five kinds, the Bailiff's phases), `arena` (the seeded builder for seven archetypes and the court), `level` (boxes, grid, rays), `route` (the zone graph), `meta` (the record), `world` (one arena's tick), `draw`, `render` (four passes: solid, facade, ink, glow), `game` (modes, clock, camera, HUD and screen JSON), `content`, `balance`. `fx`, `geom`, `rng`, `mesh` are Food Truck Circus's, with Ring, Blob, Panel and Facade meshes added.
- **Data**: 3 guns of 3 levels, 21 patterns, 6 enemies, the zone and its 10 encounters, 11 upgrades, the palette, the story (sentencing, 10 counts, addenda, verdict), copy, controls, sounds, display, shaders.
- **Page**: `app.js` (loop and modes), `hud.js`, `screens.js` (sentencing, Holding, map, indictment, verdict), `sound.js`, Iron Vector's `input.js`, and `gl.js` (Iron Vector's, with the passes).
- **Art**: `tools/art.py` → `web/art/` (title, sentencing, indictment, verdict, 3 guns, 8 icons), TikZ kept in `tools/art/`.
- **Probes** (ignored tests): `crates/core/tests/run_probe.rs` runs a scripted player through whole routes; `crates/core/tests/draw_cost.rs` prints each scene's instances and vertices.

## What passed

`./scripts/check.sh` on 2026-10-10 at commit "Draw each facade's windows in the shader": fmt, clippy with warnings as errors, 88 core tests and 11 boundary tests, the release wasm build, the wasm import audit, and the browser check in Chromium, Firefox and WebKit (`BUILDER CHECK PASSED`). Ten behaviours were broken on purpose and each test failed (SECOND-ORDER B1).

## What remains

- Sam's playtest (`TRIAGE.md`). Every feel and balance number is the agent's guess (SECOND-ORDER T1); the probe shows the curve (no wins with no upgrades, about half with 62 Clemency of upgrades), not whether it is fun (T2, P2).
- Sam's reading of the story's words (T3).
- Zones two to four, their bosses and the final boss are sealed. `zone.json` lists them; `route.rs` builds one zone.
- Not built: music, a sensitivity setting, enemy pathfinding (walkers walk straight and turn at walls).

## What the next session must not assume

- That CI checked Firefox and WebKit: it runs Chromium only. Run the three-engine check locally before pushing.
- That CI's Chromium is fast: it draws on SwiftShader. The walk to the first gate waits up to 120 s, and instance count is what costs there (G7).
- That `scripts/serve.py` survives a rebuild: `build.sh` deletes `dist/`.
