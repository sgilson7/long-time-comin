# Second-order notebook

`B3 Build in self-questioning`. One row per assumption, surprise or divergence, written when it happened. *Divergence*: assumed, found, changed. *Finding*: something learned. *Worklist*: Sam's to do.

| # | Kind | Row |
|---|---|---|
| S1 | divergence | "Shifu" read as Sifu, where every death ages you. Kept as time served: each death adds years to the sentence, 1, then 2 more, then 3 more (`meta.rs`, `record_death`), shown in the Holding and on the indictment. If Sam meant something else by it, it is one function. |
| S2 | divergence | The title is the folder Sam made for the project, *LongTimeComin*, as "Long Time Comin'"; the repository is `sgilson7/long-time-comin`. |
| S3 | divergence | The request asks for the first zone fully built and three bosses then a final one. The route shows all four zones and their bosses; zones two to four are sealed (`zone.json`, `built: false`) and the game says so. Beating the Bailiff ends the run in a verdict and banks Clemency. |
| S4 | divergence | Side paths rejoin the next main node instead of being dead ends you walk back from (Returnal's side rooms are reached and left by the same door). Assumed: in a graph of separate arenas, walking back through a cleared arena is dead time. Each side path is one or two harder fights with its reward at the end, shown on the map and above its gate before you choose. |
| S5 | divergence | One gun carried at a time (Returnal, Tower of Guns); a gun on the floor waits for E and the gun you put down stays where it was, so a swap can be undone. |
| S6 | finding | Sentinels standing on a roof fired their rings flat, over the head of a player below, so they could never hit. Rings now tilt their plane through the target, up to 35° (`orbs.rs`, `RING_TILT`); orbs across the ring still fly level. Test `a_ring_fired_from_a_roof_tilts_down_through_the_target`; breaking the tilt fails it. |
| G1 | divergence | Two world tests first failed because, with no waves left, the arena counted as cleared on its first tick and cleared its orbs: the tests' setup, not the game. The tests mark the arena cleared before adding their orb. |
| G2 | finding | The first standing-still test (at the rooftops' entrance) survived two minutes: both sentinels stood behind rooftop blocks, which is cover doing its job. The test now stands in the open plaza, where a body that does not move dies within a minute. |
| G3 | finding | The camera pulled in against a pillar until the Defendant filled the screen (found by screenshot). Closer than 1.6 m to the head, the body is not drawn (`CAMERA_HIDE_BODY`). |
| G4 | finding | Headless Chromium on SwiftShader drew the 19,000-instance arenas at a few frames a second, so the walk took far longer than its seconds. With `--use-angle=metal` it runs at 60. The browser check waits on what the page shows, not on a time. |
| A1 | divergence | The art is generated TikZ (`tools/art.py`), as the request asks: title, sentencing, indictment, verdict, the three guns, eight icons. A Holding picture was drawn into the plan and dropped: the Holding sits over the live 3D crater, which says more. |
| A2 | finding | pdftoppm cannot write transparency; icons and guns go through `pdftocairo -transp` so their corners are clear on the page's panels. |
| B1 | finding | Rule 6, run as a script over ten behaviours: jump count, dash invulnerability, step-up, ring tilt, wave height, the parry, the side path's rejoin, the years added, the boss's phase change, the interact key. Each break failed its test (messages: `a build with 1 jumps`, `p.invulnerable()`, `standing on the kerb`, the tilt pitch, `hit_ground && !hit_air`, `the orb did not land`, `seed 0: rejoins the next main node`, years 1 ≠ 6, `d.phase_changed`, gun index). |
| T1 | worklist | Every number in `crates/core/src/balance.rs`, `data/patterns.json`, `data/enemies.json` and `data/weapons.json` is the agent's guess until Sam plays it: run speed, jump heights, air control, orb speeds and sizes, enemy health, boss health (4,200), upgrade costs. |
| T2 | worklist | Sam: whether the Bailiff can be beaten by a person with the starting Carbine, and how many runs the upgrades should take. No scripted player dodges, so no test shows a person can win. |
| T3 | worklist | Sam: the story's words (`data/story.json`), especially the ten counts and the verdict. |
