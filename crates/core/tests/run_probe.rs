//! A scripted player run through whole routes, to see where zone one
//! kills and whether it can be won. Not part of the check (ignored): it
//! prints, and what it measures is balance, which is Sam's to judge.
//!
//!     cargo test -p ltc_core --test run_probe -- --ignored --nocapture
//!     RUNS=20 UPGRADES=constitution_1,spare_leap cargo test ...
//!
//! The bot aims perfectly at the nearest enemy, circles it at about twelve
//! metres, dashes sideways from an orb about to touch it, jumps a ground
//! wave about to reach it, and walks through the main gate once an arena is
//! clear. It never takes a side path and never parries. It is a better shot
//! and a worse dodger than a person.

use ltc_core::balance::*;
use ltc_core::content::{Content, Files};
use ltc_core::fx::{self, int, ONE};
use ltc_core::geom::{yaw_pitch_of, V3};
use ltc_core::meta::Meta;
use ltc_core::player::{Controls, Player};
use ltc_core::route::{NodeKind, Route};
use ltc_core::weapons::Held;
use ltc_core::world::{Exit, Outcome, World};

fn content() -> Content {
    Content::parse(&Files {
        weapons: include_str!("../../../data/weapons.json"),
        patterns: include_str!("../../../data/patterns.json"),
        enemies: include_str!("../../../data/enemies.json"),
        zone: include_str!("../../../data/zone.json"),
        upgrades: include_str!("../../../data/upgrades.json"),
        palette: include_str!("../../../data/palette.json"),
        story: include_str!("../../../data/story.json"),
        copy: include_str!("../../../data/copy.json"),
    })
    .unwrap()
}

fn exits(r: &Route, at: usize) -> Vec<Exit> {
    r.nodes[at]
        .next
        .iter()
        .enumerate()
        .map(|(k, &to)| Exit {
            to,
            main: k == 0,
            reward: r.nodes[to].reward,
            sealed: r.nodes[to].sealed,
        })
        .collect()
}

/// One tick of the bot's choices.
fn bot(w: &World, c: &Content, t: i32, stuck: bool) -> (Controls, i32, i32) {
    let p = &w.player;
    let chest = p.chest();
    let mut ctl = Controls::default();
    let mut yaw = p.yaw;
    let mut pitch = p.pitch;
    if w.cleared {
        if let Some(g) = w.gates.iter().find(|g| g.main && g.to.is_some()) {
            let (y, _) = yaw_pitch_of(g.pos.sub(p.pos));
            yaw = y;
            pitch = 0;
            ctl.move_z = ONE;
            if t % 50 == 0 {
                ctl.jump_pressed = true;
            }
        }
        return (ctl, yaw, pitch);
    }
    let target = w
        .enemies
        .iter()
        .filter(|e| e.hurtable())
        .min_by_key(|e| e.pos.sub(chest).len());
    if let Some(e) = target {
        // aim from the camera's eye: shots follow its ray
        let mut wc = w.clone();
        let (mut y, mut pt) = yaw_pitch_of(e.pos.sub(chest));
        for _ in 0..4 {
            wc.player.yaw = y;
            wc.player.pitch = pt.clamp(PITCH_MIN, PITCH_MAX);
            let eye = ltc_core::game::play_camera(&wc, ONE).eye;
            let r = yaw_pitch_of(e.pos.sub(eye));
            y = r.0;
            pt = r.1;
        }
        yaw = y;
        pitch = pt.clamp(PITCH_MIN, PITCH_MAX);
        ctl.fire = true;
        let d = e.pos.sub(chest).len_xz();
        let to = e.pos.sub(chest);
        let seen = w.arena.level.ray(chest, to.norm(), to.len()).is_none();
        ctl.move_x = if (t / 150) % 2 == 0 { ONE } else { -ONE };
        ctl.move_z = if !seen || d > int(16) {
            ONE
        } else if d < int(9) {
            -ONE
        } else {
            0
        };
        let _ = c;
    }
    // an orb a few ticks from touching: dash across its path
    for o in &w.orbs {
        let soon = o.pos.add(o.dir.scale(o.speed * 8));
        if soon.dist(chest) < int(2) || o.pos.dist(chest) < int(2) {
            ctl.dash_pressed = true;
            ctl.move_x = if (t / 150) % 2 == 0 { ONE } else { -ONE };
            break;
        }
    }
    if stuck {
        // not getting anywhere: jump and go round
        ctl.jump_pressed = t % 20 == 0;
        ctl.move_x = if (t / 90) % 2 == 0 { ONE } else { -ONE };
    }
    for wv in &w.waves {
        let d = p.pos.sub(wv.centre).len_xz();
        if d > wv.radius && d - wv.radius < wv.speed * 10 && p.grounded {
            ctl.jump_pressed = true;
        }
    }
    (ctl, yaw, pitch)
}

#[test]
#[ignore]
fn probe_whole_runs() {
    let c = content();
    let runs: u64 = std::env::var("RUNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8);
    let upgrades: Vec<String> = std::env::var("UPGRADES")
        .map(|s| s.split(',').map(|x| x.to_string()).collect())
        .unwrap_or_default();
    let meta = Meta {
        owned: upgrades,
        ..Meta::default()
    };
    let mut wins = 0;
    for seed in 0..runs {
        let route = Route::build(&c, seed * 31 + 7);
        let (build, gun_level) = meta.build(&c);
        let mut player = Player::new(V3::ZERO, 0, build);
        let mut held = Held::new(0, gun_level);
        let mut at = route.start();
        let mut log = Vec::new();
        let mut t = 0;
        let outcome = loop {
            let node = &route.nodes[at];
            let mut w = World::new(&c, node, &exits(&route, at), player.clone(), held, seed);
            let start_hp = w.player.health;
            let mut ticks = 0;
            let mut was = w.player.pos;
            let mut stuck = 0;
            while w.outcome == Outcome::Playing && ticks < 60 * 300 {
                if ticks % 60 == 0 {
                    let wants_to_move = w.cleared || w.enemies.iter().any(|e| e.hurtable());
                    stuck = if wants_to_move && w.player.pos.dist(was) < int(2) {
                        90
                    } else {
                        (stuck - 60).max(0)
                    };
                    was = w.player.pos;
                }
                stuck -= 1;
                let (ctl, yaw, pitch) = bot(&w, &c, t, stuck > 0);
                w.player.yaw = yaw;
                w.player.pitch = pitch;
                // the camera the game would use, so shots go where the bot aims
                let cam = ltc_core::game::play_camera(&w, ONE);
                w.tick(&c, &ctl, cam.eye, cam.fwd);
                ticks += 1;
                t += 1;
            }
            let name = match node.kind {
                NodeKind::Fight => node.archetype.clone(),
                k => format!("{k:?}"),
            };
            log.push(format!(
                "{name}(t{}) {}→{} hp {}s",
                node.tier,
                start_hp,
                w.player.health,
                ticks / 60
            ));
            match w.outcome {
                Outcome::Through(to) => {
                    player = w.player.clone();
                    held = w.held;
                    at = to;
                }
                o => {
                    if o == Outcome::Playing {
                        let left = w.alive_enemies();
                        let boss = w.boss.map(|b| w.enemies[b].health_pct());
                        log.push(format!(
                            "timed out, {left} left, boss {boss:?}%, cleared {}, waves left {}, player at {:?}",
                            w.cleared,
                            w.waves_left.len(),
                            w.player.pos.scale(ONE / 64).scale(64 * ONE / 65536)
                        ));
                        for e in w.enemies.iter().filter(|e| e.alive()) {
                            let to = e.pos.sub(w.player.chest());
                            let blocked = w.arena.level.ray(w.player.chest(), to.norm(), to.len()).is_some();
                            let inside = w.arena.level.sphere_hits(e.pos, c.enemies[e.def].radius / 2);
                            log.push(format!(
                                "  {} at ({}, {}, {}) m {:?} hp {} blocked {blocked} inside {inside}",
                                c.enemies[e.def].id,
                                e.pos.x / ONE,
                                e.pos.y / ONE,
                                e.pos.z / ONE,
                                e.state,
                                e.hp
                            ));
                        }
                    }
                    break o;
                }
            }
        };
        if outcome == Outcome::Won {
            wins += 1;
        }
        println!("run {seed}: {outcome:?}\n   {}", log.join("\n   "));
    }
    println!("won {wins} of {runs}");
    let _ = fx::ONE;
}
