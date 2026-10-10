//! One arena in play. Each tick: the aim ray, the hook, the body, the gun,
//! the enemies and what they fire, every orb, wave and shot, the touches,
//! the waves of enemies, the pickups and the gates.

use crate::arena::{self, Arena, Side};
use crate::balance::*;
use crate::content::Content;
use crate::enemy::{self, Enemy};
use crate::fx::{self, int, ONE};
use crate::geom::{facing, right_of, v3, V3};
use crate::orbs::{self, Orb, Wave};
use crate::player::{Controls, Event, Player};
use crate::rng::Rng;
use crate::route::{Node, NodeKind, Reward};
use crate::weapons::{self, Held, Shot, ShotKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickupKind {
    Heal(i32),
    MaxHealth(i32),
    Speed(i32),
    Jump,
    Gun(usize, i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pickup {
    pub pos: V3,
    pub kind: PickupKind,
    pub taken: bool,
    pub age: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gate {
    pub pos: V3,
    pub yaw: i32,
    /// the node it leads to; the gate you came in by leads nowhere
    pub to: Option<usize>,
    pub main: bool,
    pub reward: Option<Reward>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FxKind {
    Spark,
    Burst,
    Flash,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    pub kind: FxKind,
    pub pos: V3,
    pub vel: V3,
    pub age: i32,
    pub life: i32,
    pub rgb: [i32; 3],
    pub size: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Playing,
    Died,
    Through(usize),
    Won,
}

/// Something the page should hear or show: a sound by name, a callout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub seq: u32,
    pub sound: &'static str,
    pub text: String,
}

/// A burst to resolve: where, how wide, its damage, whether it erases
/// orbs, its colour, and the enemy it hit directly (spared the splash).
type Blast = (V3, i32, i32, bool, [i32; 3], Option<usize>);

/// Effects alive at once, at most.
const MAX_EFFECTS: usize = 600;
/// a boss's fall plays out this long before the verdict
const WIN_TICKS: i32 = secs(5, 2);
/// the first wave comes this long after you arrive
const FIRST_WAVE: i32 = secs(1, 1);
/// a killed enemy leaves a heal this often, in percent
const HEAL_DROP_PCT: i32 = 12;
const HEAL_DROP: i32 = 8;

#[derive(Debug, Clone)]
pub struct World {
    pub arena: Arena,
    pub kind: NodeKind,
    pub player: Player,
    pub held: Held,
    pub enemies: Vec<Enemy>,
    pub orbs: Vec<Orb>,
    pub waves: Vec<Wave>,
    pub shots: Vec<Shot>,
    pub pickups: Vec<Pickup>,
    pub gates: Vec<Gate>,
    pub effects: Vec<Effect>,
    pub waves_left: Vec<Vec<usize>>,
    pub wave_timer: i32,
    pub cleared: bool,
    pub reward: Option<Reward>,
    pub rng: Rng,
    pub tick: i32,
    pub notes: Vec<Note>,
    /// carried from arena to arena, so the page hears every note once
    pub seq: u32,
    /// Clemency the kills here have earned
    pub earned: i32,
    pub outcome: Outcome,
    pub kills: i32,
    pub boss: Option<usize>,
    pub win_timer: i32,
    pub aim_point: V3,
    pub aim_enemy: Option<usize>,
    /// where the hook would land, for the reticle, if it would
    pub hook_target: Option<V3>,
    pub healed_at_fountain: bool,
    /// the pickup within reach that waits for the interact key, if any
    pub offer: Option<usize>,
}

/// Where a side's exit leads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exit {
    pub to: usize,
    pub main: bool,
    pub reward: Option<Reward>,
}

impl World {
    /// The arena for a node, with the body carried in from the last.
    pub fn new(c: &Content, node: &Node, exits: &[Exit], mut player: Player, held: Held, seed: u64) -> World {
        let sides: Vec<Side> = exits
            .iter()
            .map(|e| if e.main { Side::North } else { Side::East })
            .collect();
        let arena = arena::build(&node.archetype, node.seed, &sides, &c.pal);
        player.arrive(arena.start, arena.start_yaw);
        let mut gates = Vec::new();
        for (k, (pos, yaw)) in arena.gates.iter().enumerate() {
            let exit = if k == 0 { None } else { exits.get(k - 1) };
            gates.push(Gate {
                pos: *pos,
                yaw: *yaw,
                to: exit.map(|e| e.to),
                main: exit.is_none_or(|e| e.main),
                reward: exit.and_then(|e| e.reward),
            });
        }
        let waves_left = node
            .encounter
            .map(|e| c.encounters[e].waves.clone())
            .unwrap_or_default();
        let fight = matches!(node.kind, NodeKind::Fight | NodeKind::Boss);
        let mut w = World {
            arena,
            kind: node.kind,
            player,
            held,
            enemies: Vec::new(),
            orbs: Vec::new(),
            waves: Vec::new(),
            shots: Vec::new(),
            pickups: Vec::new(),
            gates,
            effects: Vec::new(),
            waves_left,
            wave_timer: FIRST_WAVE,
            cleared: !fight,
            reward: node.reward,
            rng: Rng::new(seed ^ node.seed),
            tick: 0,
            notes: Vec::new(),
            seq: 0,
            earned: 0,
            outcome: Outcome::Playing,
            kills: 0,
            boss: None,
            win_timer: 0,
            aim_point: V3::ZERO,
            aim_enemy: None,
            hook_target: None,
            healed_at_fountain: false,
            offer: None,
        };
        if node.kind == NodeKind::Boss {
            let boss = c.enemy(&c.zones[0].boss).unwrap_or(0);
            w.waves_left = vec![vec![boss]];
        }
        w
    }

    pub fn note(&mut self, sound: &'static str, text: String) {
        self.seq += 1;
        let seq = self.seq;
        self.notes.push(Note { seq, sound, text });
        if self.notes.len() > 24 {
            self.notes.remove(0);
        }
    }

    pub fn alive_enemies(&self) -> usize {
        self.enemies.iter().filter(|e| e.alive()).count()
    }

    /// The point the crosshair is on: the first solid or enemy along the
    /// camera's ray past the player's own body.
    fn aim(&mut self, c: &Content, eye: V3, fwd: V3) {
        let skip = eye.dist(self.player.chest()) + ONE / 2;
        let (t, enemy) = self.ray(c, eye, fwd, skip, AIM_RANGE);
        self.aim_point = eye.add(fwd.scale(t));
        self.aim_enemy = enemy;
        let reach = skip + GRAPPLE_RANGE;
        self.hook_target = if t < reach { Some(self.aim_point) } else { None };
    }

    /// Along a unit ray, the distance to the first solid or hurtable enemy
    /// at least `skip` away, and the enemy if one was hit first.
    fn ray(&self, c: &Content, o: V3, d: V3, skip: i32, max: i32) -> (i32, Option<usize>) {
        let start = o.add(d.scale(skip));
        let mut best = self
            .arena
            .level
            .ray(start, d, max - skip)
            .map_or(max, |h| h.t + skip);
        let mut who = None;
        for (i, e) in self.enemies.iter().enumerate() {
            if !e.alive() {
                continue;
            }
            if let Some(t) = ray_sphere(o, d, e.pos, c.enemies[e.def].radius) {
                if t >= skip && t < best {
                    best = t;
                    who = Some(i);
                }
            }
        }
        (best, who)
    }

    fn spawn(&mut self, c: &Content, def: usize) {
        let d = &c.enemies[def];
        let feet = self.player.pos;
        let air = matches!(d.kind, crate::content::Kind::Floater | crate::content::Kind::Moth);
        let pool = if air {
            &self.arena.air_spawns
        } else {
            &self.arena.ground_spawns
        };
        let fits: Vec<V3> = pool
            .iter()
            .copied()
            .filter(|p| {
                let dist = p.sub(feet).len_xz();
                dist > int(12) && dist < int(36)
            })
            .collect();
        let list = if fits.is_empty() { pool.clone() } else { fits };
        let mut at = if d.kind == crate::content::Kind::Boss || list.is_empty() {
            self.arena.centre
        } else {
            list[self.rng.range(0, list.len() as i32 - 1) as usize]
        };
        if !air && d.kind != crate::content::Kind::Boss {
            at.y += if d.kind == crate::content::Kind::Walker {
                d.radius
            } else {
                d.hover
            };
        }
        let orbit = if self.rng.chance(50) { 1 } else { -1 };
        let mut e = Enemy::new(def, d, at, orbit);
        e.home = self.arena.centre;
        if d.kind == crate::content::Kind::Boss {
            self.boss = Some(self.enemies.len());
        }
        self.enemies.push(e);
        self.effect(
            FxKind::Flash,
            at,
            V3::ZERO,
            enemy::SPAWN_TICKS,
            c.pal.c("enemy_core"),
            d.radius * 3,
        );
    }

    fn effect(&mut self, kind: FxKind, pos: V3, vel: V3, life: i32, rgb: [i32; 3], size: i32) {
        if self.effects.len() < MAX_EFFECTS {
            self.effects.push(Effect {
                kind,
                pos,
                vel,
                age: 0,
                life,
                rgb,
                size,
            });
        }
    }

    fn sparks(&mut self, at: V3, rgb: [i32; 3], n: i32, speed: i32) {
        for _ in 0..n {
            let dir = facing(fx::deg(self.rng.range(0, 359)), fx::deg(self.rng.range(-30, 70)));
            let s = speed * self.rng.range(40, 100) / 100;
            let life = self.rng.range(12, 30);
            self.effect(FxKind::Spark, at, dir.scale(s), life, rgb, ONE / 10);
        }
    }

    /// One tick of play. `eye` and `fwd` are the camera's, for the aim.
    pub fn tick(&mut self, c: &Content, ctl: &Controls, eye: V3, fwd: V3) {
        if self.outcome != Outcome::Playing {
            return;
        }
        self.tick += 1;
        self.aim(c, eye, fwd);

        // the hook
        if ctl.grapple_pressed
            && self.player.build.grapple
            && self.player.grapple.is_none()
            && self.player.grapple_cooldown == 0
        {
            match self.hook_target {
                Some(p) => self.player.start_grapple(p, self.aim_enemy),
                None => self.note("hook_miss", String::new()),
            }
        }
        if let Some(g) = self.player.grapple.as_mut() {
            if let Some(i) = g.enemy {
                if self.enemies[i].alive() {
                    g.anchor = self.enemies[i].pos;
                } else {
                    g.enemy = None;
                }
            }
        }

        // the body
        self.player.step(ctl, &self.arena.level);
        for ev in self.player.events.clone() {
            let sound = match ev {
                Event::Jump(n) if n > 1 => "air_jump",
                Event::Jump(_) => "jump",
                Event::Land => "land",
                Event::Dash => "dash",
                Event::Hurt(_) => "hurt",
                Event::Recess => "recess",
                Event::Fell => "fell",
                Event::GrappleOn => "hook",
                Event::GrappleOff => "hook_off",
                Event::Parry => "parry",
            };
            let text = match ev {
                Event::Recess => c.say("callout_recess"),
                Event::Fell => c.say("callout_fell"),
                _ => String::new(),
            };
            self.note(sound, text);
        }
        if let Some(f) = self.arena.fountain {
            if !self.healed_at_fountain && self.player.pos.dist(f) < int(4) {
                self.healed_at_fountain = true;
                let max = self.player.build.max_health;
                self.player.heal(max);
                self.sparks(f.add(v3(0, int(2), 0)), c.pal.c("pickup_heal"), 30, ONE / 6);
                self.note("pickup", c.say("callout_fountain"));
            }
        }

        // the gun
        let chest = self.player.chest();
        let muzzle = chest
            .add(right_of(self.player.yaw).scale(ONE * 4 / 10))
            .add(v3(0, ONE / 5, 0))
            .add(facing(self.player.yaw, 0).scale(ONE / 2));
        let gun = &c.guns[self.held.gun];
        let rgb = c.pal.c(&gun.colour);
        let shots = weapons::trigger(
            &mut self.held,
            gun,
            rgb,
            ctl.fire,
            muzzle,
            self.aim_point,
            &mut self.rng,
        );
        if !shots.is_empty() {
            let sound = match self.held.gun {
                0 => "fire_carbine",
                1 => "fire_scatter",
                _ => "fire_launcher",
            };
            self.note(sound, String::new());
            self.effect(FxKind::Flash, muzzle, V3::ZERO, 3, rgb, ONE / 3);
        }
        self.shots.extend(shots);

        // the waves of enemies
        self.waves_step(c);

        // the enemies and what they fire
        for i in 0..self.enemies.len() {
            if !self.enemies[i].alive() {
                continue;
            }
            let deeds = self.enemies[i].step(c, &self.arena.level, chest, i);
            let room = MAX_ORBS.saturating_sub(self.orbs.len());
            self.orbs.extend(deeds.salvo.orbs.into_iter().take(room));
            self.waves.extend(deeds.salvo.waves);
            if deeds.phase_changed {
                // the court clears its orbs while the Bailiff changes
                for o in std::mem::take(&mut self.orbs) {
                    self.effect(FxKind::Burst, o.pos, V3::ZERO, 14, o.rgb, o.radius * 2);
                }
                self.waves.clear();
                self.note("boss_phase", c.say("callout_phase"));
            }
            for s in deeds.summons {
                self.spawn(c, s);
            }
        }

        self.orbs_step(c, chest);
        self.shots_step(c);

        // touching an enemy hurts; a moth bursts on you
        let feet = self.player.pos;
        for i in 0..self.enemies.len() {
            let d = &c.enemies[self.enemies[i].def];
            if self.enemies[i].alive() && enemy::contact(&self.enemies[i], d, feet) {
                let dmg = d.contact;
                if d.kind == crate::content::Kind::Moth {
                    self.enemies[i].hp = 0;
                    let at = self.enemies[i].pos;
                    self.sparks(at, c.pal.c("enemy_eye"), 14, ONE / 5);
                    self.note("boom", String::new());
                }
                self.player.hurt(dmg);
            }
        }

        self.pickups_step(c, ctl);
        self.gates_step();
        self.effects_step();

        if let Some(b) = self.boss {
            if !self.enemies[b].alive() {
                if self.win_timer == 0 {
                    self.note("boss_down", c.say("callout_boss_down"));
                    for o in std::mem::take(&mut self.orbs) {
                        self.effect(FxKind::Burst, o.pos, V3::ZERO, 16, o.rgb, o.radius * 2);
                    }
                    self.waves.clear();
                    for e in self.enemies.iter_mut() {
                        e.hp = 0;
                    }
                }
                self.win_timer += 1;
                if self.win_timer % 8 == 1 {
                    let at = self.enemies[b].pos.add(v3(
                        int(self.rng.range(-3, 3)),
                        int(self.rng.range(-2, 2)),
                        int(self.rng.range(-3, 3)),
                    ));
                    self.effect(FxKind::Burst, at, V3::ZERO, 24, c.pal.c("boss_trim"), int(4));
                    self.sparks(at, c.pal.c("spark"), 10, ONE / 4);
                }
                if self.win_timer > WIN_TICKS {
                    self.outcome = Outcome::Won;
                }
            }
        }
        if !self.player.alive() && self.outcome == Outcome::Playing {
            self.outcome = Outcome::Died;
        }
    }

    fn waves_step(&mut self, c: &Content) {
        if self.cleared || self.kind == NodeKind::Arrival || self.kind == NodeKind::Rest {
            return;
        }
        let alive = self.alive_enemies();
        if !self.waves_left.is_empty() {
            let ready = alive <= WAVE_REMAINDER || self.enemies.is_empty();
            if ready {
                self.wave_timer -= 1;
                if self.wave_timer <= 0 {
                    let wave = self.waves_left.remove(0);
                    for def in wave {
                        self.spawn(c, def);
                    }
                    self.wave_timer = secs(3, 4);
                    if self.kind == NodeKind::Boss {
                        self.note("boss_intro", String::new());
                    } else {
                        self.note("wave", String::new());
                    }
                }
            }
        } else if alive == 0 && self.kind != NodeKind::Boss {
            self.cleared = true;
            self.orbs.clear();
            self.waves.clear();
            let heal = HEAL_ON_CLEAR;
            self.player.heal(heal);
            if let Some(r) = self.reward {
                let kind = match r {
                    Reward::Gun(g, l) => PickupKind::Gun(g, l),
                    Reward::Jump => PickupKind::Jump,
                    Reward::Health => PickupKind::MaxHealth(HEALTH_PICKUP),
                    Reward::Speed => PickupKind::Speed(SPEED_PICKUP_PCT),
                };
                let at = self.player.pos.add(facing(self.player.yaw, 0).scale(int(4)));
                let ground = self
                    .arena
                    .level
                    .floor_below(at.x, at.y + int(3), at.z)
                    .unwrap_or(0);
                self.pickups.push(Pickup {
                    pos: v3(at.x, ground, at.z),
                    kind,
                    taken: false,
                    age: 0,
                });
            }
            self.note("cleared", c.say("callout_cleared"));
        }
    }

    fn orbs_step(&mut self, c: &Content, chest: V3) {
        let level = &self.arena.level;
        let mut kept = Vec::with_capacity(self.orbs.len());
        let mut hurt = false;
        let mut parried: Vec<Orb> = Vec::new();
        let mut spent: Vec<Orb> = Vec::new();
        let parrying = self.player.parry_window > 0;
        for mut o in std::mem::take(&mut self.orbs) {
            if !o.step(level) {
                spent.push(o);
                continue;
            }
            if parrying && o.gold && orbs::within(o.pos, chest, PARRY_REACH) {
                parried.push(o);
                continue;
            }
            if !hurt && o.touches(self.player.pos) && !self.player.invulnerable() {
                hurt = true;
                spent.push(o);
                continue;
            }
            kept.push(o);
        }
        self.orbs = kept;
        if hurt {
            self.player.hurt(ORB_DAMAGE);
        }
        for o in spent {
            self.effect(FxKind::Burst, o.pos, V3::ZERO, 10, o.rgb, o.radius * 2);
        }
        let reflect = c.pal.c("shot_reflect");
        for o in parried {
            let target = self
                .enemies
                .get(o.owner)
                .filter(|e| e.alive())
                .map_or(self.aim_point, |e| e.pos);
            let dir = target.sub(o.pos).norm();
            self.shots.push(Shot {
                pos: o.pos,
                prev: o.pos,
                vel: dir.scale(PARRY_SPEED),
                damage: PARRY_DAMAGE,
                radius: o.radius,
                life: secs(3, 1),
                splash_radius: 0,
                splash: 0,
                pierce: 0,
                passed: Vec::new(),
                rgb: reflect,
                kind: ShotKind::Reflect,
                erases: false,
            });
            self.sparks(o.pos, reflect, 12, ONE / 5);
            self.note("reflect", c.say("callout_objection"));
        }
        let mut wave_hit = false;
        let feet = self.player.pos;
        let inv = self.player.invulnerable();
        self.waves.retain_mut(|w| {
            let alive = w.step();
            if !w.hit && !inv && w.touches(feet) {
                w.hit = true;
                wave_hit = true;
            }
            alive
        });
        if wave_hit {
            self.player.hurt(WAVE_DAMAGE);
        }
    }

    fn shots_step(&mut self, c: &Content) {
        let mut kept = Vec::with_capacity(self.shots.len());
        let mut blasts: Vec<Blast> = Vec::new();
        let mut killed: Vec<usize> = Vec::new();
        let mut hits: Vec<(V3, [i32; 3])> = Vec::new();
        for mut s in std::mem::take(&mut self.shots) {
            s.prev = s.pos;
            s.life -= 1;
            let len = s.vel.len();
            let dir = s.vel.norm();
            let wall = self.arena.level.ray(s.pos, dir, len).map(|h| h.t);
            let mut first: Option<(i32, usize)> = None;
            for (i, e) in self.enemies.iter().enumerate() {
                if !e.hurtable() || s.passed.contains(&i) {
                    continue;
                }
                let r = c.enemies[e.def].radius + s.radius;
                if let Some(t) = ray_sphere(s.pos, dir, e.pos, r) {
                    if t <= len && first.is_none_or(|(ft, _)| t < ft) {
                        first = Some((t, i));
                    }
                }
            }
            let enemy_first = match (first, wall) {
                (Some((te, _)), Some(tw)) => te <= tw,
                (Some(_), None) => true,
                _ => false,
            };
            if enemy_first {
                let (t, i) = first.unwrap_or((0, 0));
                let at = s.pos.add(dir.scale(t));
                if self.enemies[i].hurt(s.damage) {
                    killed.push(i);
                }
                hits.push((at, s.rgb));
                if s.kind == ShotKind::Bomb {
                    blasts.push((at, s.splash_radius, s.splash, s.erases, s.rgb, Some(i)));
                    continue;
                }
                if s.pierce > 0 {
                    s.pierce -= 1;
                    s.passed.push(i);
                } else {
                    continue;
                }
            } else if let Some(t) = wall {
                let at = s.pos.add(dir.scale(t));
                if s.kind == ShotKind::Bomb {
                    blasts.push((at, s.splash_radius, s.splash, s.erases, s.rgb, None));
                } else {
                    hits.push((at, s.rgb));
                }
                continue;
            }
            s.pos = s.pos.add(s.vel);
            if s.life <= 0 {
                if s.kind == ShotKind::Bomb {
                    blasts.push((s.pos, s.splash_radius, s.splash, s.erases, s.rgb, None));
                }
                continue;
            }
            kept.push(s);
        }
        self.shots = kept;
        for (at, rgb) in hits {
            self.sparks(at, rgb, 4, ONE / 8);
            self.note("hit", String::new());
        }
        for (at, radius, damage, erases, rgb, direct) in blasts {
            self.effect(FxKind::Burst, at, V3::ZERO, 18, rgb, radius * 2);
            self.sparks(at, rgb, 16, ONE / 4);
            self.note("boom", String::new());
            for i in 0..self.enemies.len() {
                if Some(i) == direct || !self.enemies[i].hurtable() {
                    continue;
                }
                let r = radius + c.enemies[self.enemies[i].def].radius;
                if orbs::within(self.enemies[i].pos, at, r) && self.enemies[i].hurt(damage) {
                    killed.push(i);
                }
            }
            if erases {
                self.orbs.retain(|o| !orbs::within(o.pos, at, radius));
            }
        }
        killed.sort();
        killed.dedup();
        for i in killed {
            self.kills += 1;
            let e = &self.enemies[i];
            let (at, def) = (e.pos, e.def);
            let d = &c.enemies[def];
            self.earned += d.clemency;
            self.effect(
                FxKind::Burst,
                at,
                V3::ZERO,
                22,
                c.pal.c("enemy_core"),
                d.radius * 4,
            );
            self.sparks(at, c.pal.c("spark"), 20, ONE / 4);
            self.note("kill", String::new());
            if self.rng.chance(HEAL_DROP_PCT) && Some(i) != self.boss {
                let ground = self.arena.level.floor_below(at.x, at.y, at.z).unwrap_or(0);
                self.pickups.push(Pickup {
                    pos: v3(at.x, ground, at.z),
                    kind: PickupKind::Heal(HEAL_DROP),
                    taken: false,
                    age: 0,
                });
            }
        }
    }

    fn pickups_step(&mut self, c: &Content, ctl: &Controls) {
        self.offer = None;
        let feet = self.player.pos;
        for k in 0..self.pickups.len() {
            if self.pickups[k].taken {
                continue;
            }
            self.pickups[k].age += 1;
            let p = self.pickups[k].clone();
            let near = feet.sub(p.pos).len_xz() < PICKUP_REACH && (feet.y - p.pos.y).abs() < int(3);
            if !near {
                continue;
            }
            match p.kind {
                PickupKind::Gun(gun, level) => {
                    self.offer = Some(k);
                    if !ctl.interact_pressed {
                        continue;
                    }
                    // the gun you put down stays where it was, to take back
                    let old = self.held;
                    self.held = Held::new(gun, level);
                    self.pickups[k].kind = PickupKind::Gun(old.gun, old.level);
                    self.pickups[k].age = 0;
                    let name = format!("{} {}", c.guns[gun].name, roman(level));
                    self.note("pickup", c.say("callout_gun").replace("{gun}", &name));
                    continue;
                }
                PickupKind::Heal(n) => {
                    self.player.heal(n);
                    self.note("heal", String::new());
                }
                PickupKind::MaxHealth(n) => {
                    self.player.build.max_health += n;
                    self.player.heal(n);
                    let text = c.say("callout_health").replace("{n}", &n.to_string());
                    self.note("pickup", text);
                }
                PickupKind::Speed(n) => {
                    self.player.build.speed_pct = (self.player.build.speed_pct + n).min(160);
                    self.note("pickup", c.say("callout_speed").replace("{n}", &n.to_string()));
                }
                PickupKind::Jump => {
                    if self.player.build.jumps < MAX_JUMPS {
                        self.player.build.jumps += 1;
                        let text = c
                            .say("callout_jump")
                            .replace("{n}", &self.player.build.jumps.to_string())
                            .replace("{max}", &MAX_JUMPS.to_string());
                        self.note("pickup", text);
                    } else {
                        let max = self.player.build.max_health;
                        self.player.heal(max);
                        self.note("pickup", c.say("callout_jump_full"));
                    }
                }
            }
            self.pickups[k].taken = true;
            self.sparks(p.pos.add(v3(0, ONE, 0)), c.pal.c("spark"), 16, ONE / 6);
        }
    }

    fn gates_step(&mut self) {
        if !self.cleared {
            return;
        }
        let feet = self.player.pos;
        for g in &self.gates {
            if let Some(to) = g.to {
                if feet.sub(g.pos).len_xz() < GATE_REACH && (feet.y - g.pos.y).abs() < int(2) {
                    self.outcome = Outcome::Through(to);
                    return;
                }
            }
        }
    }

    fn effects_step(&mut self) {
        let g = GRAVITY;
        self.effects.retain_mut(|e| {
            e.age += 1;
            if e.kind == FxKind::Spark {
                e.pos = e.pos.add(e.vel);
                e.vel.y -= g;
            }
            e.age < e.life
        });
    }
}

/// The distance along a unit ray to where it enters a sphere, if it does.
pub fn ray_sphere(o: V3, d: V3, c: V3, r: i32) -> Option<i32> {
    let oc = c.sub(o);
    let tca = oc.dot(d);
    let oc2 = oc.x as i64 * oc.x as i64 + oc.y as i64 * oc.y as i64 + oc.z as i64 * oc.z as i64;
    let r2 = r as i64 * r as i64;
    if oc2 <= r2 {
        return Some(0);
    }
    if tca < 0 {
        return None;
    }
    let d2 = oc2 - tca as i64 * tca as i64;
    if d2 > r2 {
        return None;
    }
    let thc = fx::isqrt((r2 - d2) as u64) as i32;
    Some((tca - thc).max(0))
}

pub fn roman(n: i32) -> &'static str {
    match n {
        1 => "I",
        2 => "II",
        3 => "III",
        _ => "IV",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;
    use crate::player::Build;
    use crate::route::Route;

    fn world_for(kind: NodeKind) -> (Content, World) {
        let c = content();
        let r = Route::build(&c, 11);
        let i = r.nodes.iter().position(|n| n.kind == kind).unwrap();
        let exits: Vec<Exit> = r.nodes[i]
            .next
            .iter()
            .enumerate()
            .map(|(k, &to)| Exit {
                to,
                main: k == 0,
                reward: r.nodes[to].reward,
            })
            .collect();
        let p = Player::new(V3::ZERO, 0, Build::default());
        let w = World::new(&c, &r.nodes[i], &exits, p, Held::new(0, 1), 1);
        (c, w)
    }

    fn eye_of(w: &World) -> (V3, V3) {
        let f = facing(w.player.yaw, w.player.pitch);
        (w.player.chest().sub(f.scale(int(4))), f)
    }

    #[test]
    fn a_ray_meets_a_sphere_at_its_near_surface() {
        let t = ray_sphere(V3::ZERO, v3(0, 0, -ONE), v3(0, 0, -int(10)), int(2)).unwrap();
        assert!((t - int(8)).abs() < 8, "{t}");
        assert!(ray_sphere(V3::ZERO, v3(0, 0, -ONE), v3(int(3), 0, -int(10)), int(2)).is_none());
        assert!(ray_sphere(V3::ZERO, v3(0, 0, ONE), v3(0, 0, -int(10)), int(2)).is_none());
    }

    #[test]
    fn the_crater_is_clear_from_the_start_and_its_gate_leads_on() {
        let (c, mut w) = world_for(NodeKind::Arrival);
        assert!(w.cleared);
        assert_eq!(w.gates.len(), 2);
        assert!(w.gates[0].to.is_none(), "the way you came is shut");
        // walk north to the gate
        let ctl = Controls {
            move_z: ONE,
            ..Controls::default()
        };
        for _ in 0..60 * 20 {
            let (eye, fwd) = eye_of(&w);
            w.tick(&c, &ctl, eye, fwd);
            if w.outcome != Outcome::Playing {
                break;
            }
        }
        assert_eq!(w.outcome, Outcome::Through(1), "at {:?}", w.player.pos);
    }

    #[test]
    fn standing_still_in_the_open_plaza_is_fatal_within_a_minute() {
        // In the open. At the entrance of the rooftops, cover can shield a
        // body from both first enemies (SECOND-ORDER G3), which is cover
        // doing its job.
        let c = content();
        let (node, seed) = (0..60u64)
            .find_map(|seed| {
                let r = Route::build(&c, seed);
                r.nodes
                    .iter()
                    .find(|n| n.kind == NodeKind::Fight && n.archetype == "plaza" && !n.detour)
                    .map(|n| (n.clone(), seed))
            })
            .expect("some run has a plaza");
        let p = Player::new(V3::ZERO, 0, Build::default());
        let mut w = World::new(&c, &node, &[], p, Held::new(0, 1), seed);
        w.player.pos = v3(int(8), 0, int(8));
        let mut ticks = 0;
        while w.outcome == Outcome::Playing && ticks < 60 * 60 {
            let (eye, fwd) = eye_of(&w);
            w.tick(&c, &Controls::default(), eye, fwd);
            ticks += 1;
        }
        assert_eq!(
            w.outcome,
            Outcome::Died,
            "survived {ticks} ticks, {} health",
            w.player.health
        );
    }

    #[test]
    fn aiming_at_an_enemy_and_firing_kills_it() {
        let (c, mut w) = world_for(NodeKind::Fight);
        w.waves_left.clear();
        let d = c.enemy("drifter").unwrap();
        let mut e = Enemy::new(d, &c.enemies[d], w.player.chest().add(v3(0, 0, -int(10))), 1);
        e.state = enemy::State::Rest(10_000);
        w.enemies.push(e);
        let ctl = Controls {
            fire: true,
            ..Controls::default()
        };
        for _ in 0..240 {
            // aim straight at it from behind the shoulder
            let target = w.enemies[0].pos;
            let eye = w.player.chest().add(v3(0, 0, int(4)));
            let fwd = target.sub(eye).norm();
            w.enemies[0].state = enemy::State::Rest(10_000);
            w.enemies[0].pos = target;
            w.tick(&c, &ctl, eye, fwd);
        }
        assert!(!w.enemies[0].alive(), "hp {}", w.enemies[0].hp);
        assert_eq!(w.kills, 1);
        assert!(
            w.cleared,
            "the last enemy dead and no waves left clears the arena"
        );
    }

    #[test]
    fn a_parried_gold_orb_flies_back_and_hurts_its_owner() {
        let (c, mut w) = world_for(NodeKind::Fight);
        w.waves_left.clear();
        // cleared already, so the clearing does not sweep the test's orb away
        w.cleared = true;
        w.player.build.parry = true;
        let d = c.enemy("lantern").unwrap();
        // to the side, in the open (ahead of the entrance stand stairs on
        // some archetypes, which would take the shot)
        let mut e = Enemy::new(d, &c.enemies[d], w.player.chest().add(v3(int(6), int(2), 0)), 1);
        e.state = enemy::State::Rest(10_000);
        w.enemies.push(e);
        let chest = w.player.chest();
        let gold = c.pal.c("gold");
        w.orbs.push(Orb {
            pos: chest.add(v3(0, 0, -int(2))),
            prev: chest,
            dir: v3(0, 0, ONE),
            speed: ONE / 10,
            accel: 0,
            min_speed: 0,
            max_speed: ONE / 10,
            curl: 0,
            radius: ONE * 4 / 10,
            rgb: gold,
            gold: true,
            age: 0,
            owner: 0,
        });
        let before = w.enemies[0].hp;
        let parry = Controls {
            parry_pressed: true,
            ..Controls::default()
        };
        let (eye, fwd) = eye_of(&w);
        w.tick(&c, &parry, eye, fwd);
        for _ in 0..60 {
            w.enemies[0].state = enemy::State::Rest(10_000);
            w.tick(&c, &Controls::default(), eye, fwd);
        }
        assert_eq!(w.player.health, w.player.build.max_health, "the orb did not land");
        assert_eq!(before - w.enemies[0].hp, PARRY_DAMAGE);
    }

    #[test]
    fn without_objection_the_same_orb_hurts() {
        let (c, mut w) = world_for(NodeKind::Fight);
        w.waves_left.clear();
        w.cleared = true;
        let chest = w.player.chest();
        w.orbs.push(Orb {
            pos: chest.add(v3(0, 0, -int(2))),
            prev: chest,
            dir: v3(0, 0, ONE),
            speed: ONE / 10,
            accel: 0,
            min_speed: 0,
            max_speed: ONE / 10,
            curl: 0,
            radius: ONE * 4 / 10,
            rgb: [0; 3],
            gold: true,
            age: 0,
            owner: 0,
        });
        let parry = Controls {
            parry_pressed: true,
            ..Controls::default()
        };
        let (eye, fwd) = eye_of(&w);
        for _ in 0..60 {
            w.tick(&c, &parry, eye, fwd);
        }
        assert_eq!(w.player.health, w.player.build.max_health - ORB_DAMAGE);
    }

    #[test]
    fn a_jump_pickup_adds_a_jump_up_to_four() {
        let (c, mut w) = world_for(NodeKind::Rest);
        w.player.build.jumps = 3;
        for _ in 0..2 {
            w.pickups.push(Pickup {
                pos: w.player.pos,
                kind: PickupKind::Jump,
                taken: false,
                age: 0,
            });
        }
        let (eye, fwd) = eye_of(&w);
        w.tick(&c, &Controls::default(), eye, fwd);
        assert_eq!(w.player.build.jumps, MAX_JUMPS);
        assert!(w.pickups.iter().all(|p| p.taken));
    }

    #[test]
    fn a_gun_on_the_floor_waits_for_the_interact_key_and_swaps() {
        let (c, mut w) = world_for(NodeKind::Rest);
        w.pickups.push(Pickup {
            pos: w.player.pos,
            kind: PickupKind::Gun(2, 3),
            taken: false,
            age: 0,
        });
        let (eye, fwd) = eye_of(&w);
        w.tick(&c, &Controls::default(), eye, fwd);
        assert_eq!(w.held.gun, 0);
        assert_eq!(w.offer, Some(0));
        let take = Controls {
            interact_pressed: true,
            ..Controls::default()
        };
        w.tick(&c, &take, eye, fwd);
        assert_eq!((w.held.gun, w.held.level), (2, 3));
        assert_eq!(
            w.pickups[0].kind,
            PickupKind::Gun(0, 1),
            "the old gun is left behind"
        );
    }

    #[test]
    fn the_bailiff_comes_alone_into_the_court() {
        let (c, mut w) = world_for(NodeKind::Boss);
        let (eye, fwd) = eye_of(&w);
        for _ in 0..90 {
            w.tick(&c, &Controls::default(), eye, fwd);
        }
        assert_eq!(w.enemies.len(), 1);
        assert_eq!(w.boss, Some(0));
        assert_eq!(c.enemies[w.enemies[0].def].id, "bailiff");
    }
}
