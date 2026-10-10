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
    Pierce,
    Pulse,
    Mend,
}

/// The pickup a reward becomes.
pub fn reward_pickup(r: Reward) -> PickupKind {
    match r {
        Reward::Gun(g, l) => PickupKind::Gun(g, l),
        Reward::Jump => PickupKind::Jump,
        Reward::Health => PickupKind::MaxHealth(HEALTH_PICKUP),
        Reward::Speed => PickupKind::Speed(SPEED_PICKUP_PCT),
        Reward::Pierce => PickupKind::Pierce,
        Reward::Pulse => PickupKind::Pulse,
        Reward::Mend => PickupKind::Mend,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pickup {
    pub pos: V3,
    pub kind: PickupKind,
    pub taken: bool,
    pub age: i32,
    /// pickups sharing a group are a choice: taking one takes the rest
    /// away, and each waits for the interact key
    pub group: u8,
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
pub const HEAL_DROP_PCT: i32 = 18;
pub const HEAL_DROP: i32 = 8;

#[derive(Debug, Clone)]
pub struct World {
    pub arena: Arena,
    /// the zone it is in, by index
    pub zone: usize,
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
    /// where the last hit came from, and how long ago, for the HUD's arc
    pub hit_from: V3,
    pub hit_age: i32,
    pub pulse_cooldown: i32,
    /// the tick the arena was cleared on
    pub cleared_at: i32,
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
        let z = &c.zones[node.zone];
        let arena = arena::build(&z.id, &node.archetype, node.seed, &sides, &c.pal);
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
            .map(|e| z.encounters[e].waves.clone())
            .unwrap_or_default();
        let fight = matches!(node.kind, NodeKind::Fight | NodeKind::Boss);
        let mut w = World {
            arena,
            zone: node.zone,
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
            hit_from: V3::ZERO,
            hit_age: 1000,
            pulse_cooldown: 0,
            cleared_at: 0,
            offer: None,
        };
        if node.kind == NodeKind::Boss {
            let boss = c.enemy(&c.zones[node.zone].boss).unwrap_or(0);
            w.waves_left = vec![vec![boss]];
        }
        if node.kind == NodeKind::Arrival && !node.cache.is_empty() {
            // the expedition's cache: take one
            for (k, r) in node.cache.iter().enumerate() {
                let x = if k == 0 { -int(3) } else { int(3) };
                let at = w.arena.start.add(v3(x, 0, -int(6)));
                w.pickups.push(Pickup {
                    pos: at,
                    kind: reward_pickup(*r),
                    taken: false,
                    age: 0,
                    group: 2,
                });
            }
        }
        if node.kind == NodeKind::Rest {
            // the Antechamber's choice: half your health back from the
            // fountain, or its reward; taking one loses the other
            let heal = w.player.build.max_health * REST_HEAL_PCT / 100;
            let mut offer = vec![(v3(-int(4), 0, int(5)), PickupKind::Heal(heal))];
            if let Some(r) = node.reward {
                offer.push((v3(int(4), 0, int(5)), reward_pickup(r)));
            }
            for (pos, kind) in offer {
                w.pickups.push(Pickup {
                    pos,
                    kind,
                    taken: false,
                    age: 0,
                    group: 1,
                });
            }
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
            if !e.alive() || e.buried {
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
        let air = flies(d.kind);
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
            at.y += standing_lift(d);
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
                Some(p) => {
                    let anchor = if self.aim_enemy.is_some() {
                        p
                    } else {
                        self.mantle(eye, fwd, p)
                    };
                    self.player.start_grapple(anchor, self.aim_enemy)
                }
                None => self.note("hook_miss", c.say("callout_hook_miss")),
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
                Event::HookSnag => "hook_miss",
                Event::Wall => "wall",
                Event::Vent => "vent",
            };
            let text = match ev {
                Event::Recess => c.say("callout_recess"),
                Event::Fell => c.say("callout_fell"),
                Event::HookSnag => c.say("callout_hook_snag"),
                _ => String::new(),
            };
            self.note(sound, text);
            if ev == Event::Dash && self.player.build.pulse && self.pulse_cooldown == 0 {
                // Dash Pulse: the orbs round you are gone
                self.pulse_cooldown = PULSE_COOLDOWN;
                let at = self.player.chest();
                let (gone, kept): (Vec<Orb>, Vec<Orb>) = std::mem::take(&mut self.orbs)
                    .into_iter()
                    .partition(|o| orbs::within(o.pos, at, PULSE_RADIUS));
                self.orbs = kept;
                for o in gone {
                    self.effect(FxKind::Burst, o.pos, V3::ZERO, 10, o.rgb, o.radius * 2);
                }
                self.effect(FxKind::Burst, at, V3::ZERO, 16, c.pal.c("cyan"), PULSE_RADIUS * 2);
            }
        }
        self.pulse_cooldown = (self.pulse_cooldown - 1).max(0);
        self.hit_age = (self.hit_age + 1).min(1000);

        // an updraft under the feet throws you up
        let feet = self.player.pos;
        if self.arena.vents.iter().any(|v| {
            feet.sub(v.pos).len_xz() < v.radius
                && feet.y >= v.pos.y - ONE / 5
                && feet.y < v.pos.y + ONE * 3 / 2
        }) {
            self.player.launch_up(VENT_SPEED);
            if self.player.events.contains(&Event::Vent) {
                self.note("vent", String::new());
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
            self.player.build.pierce,
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

        // an enemy that has not seen you for a long while comes back where
        // it can: no fight can stall behind a wall (SECOND-ORDER P7)
        for i in 0..self.enemies.len() {
            let e = &self.enemies[i];
            if e.alive() && e.hurtable() && e.unseen > LOST_TICKS && Some(i) != self.boss {
                self.relocate(c, i, chest);
            }
        }

        // the enemies and what they fire
        for i in 0..self.enemies.len() {
            if !self.enemies[i].alive() {
                continue;
            }
            let deeds = self.enemies[i].step(c, &self.arena.level, chest, i);
            let d = &c.enemies[self.enemies[i].def];
            if let Some((from, to)) = deeds.blink {
                let rgb = c.pal.c("enemy_core");
                self.effect(FxKind::Flash, from, V3::ZERO, 14, rgb, d.radius * 3);
                self.effect(FxKind::Flash, to, V3::ZERO, 18, rgb, d.radius * 3);
                self.note("blink", String::new());
            }
            if deeds.surfaced || deeds.dove {
                let at = self.enemies[i].pos;
                let ground = self
                    .arena
                    .level
                    .floor_below(at.x, chest.y + int(2), at.z)
                    .unwrap_or(0);
                let rgb = c.pal.c("spark");
                self.sparks(v3(at.x, ground, at.z), rgb, 18, ONE / 5);
                self.note(if deeds.surfaced { "surface" } else { "dive" }, String::new());
            }
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
                    self.burst(c, i);
                }
                let from = self.enemies[i].pos;
                self.hurt_from(dmg, from);
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
            let ready = alive as i32 <= WAVE_REMAINDER || self.enemies.is_empty();
            if ready {
                self.wave_timer -= 1;
                if self.wave_timer <= 0 {
                    let wave = self.waves_left.remove(0);
                    for def in wave {
                        self.spawn(c, def);
                    }
                    self.wave_timer = WAVE_GAP;
                    if self.kind == NodeKind::Boss {
                        let name = c
                            .story
                            .bosses
                            .get(&c.zones[self.zone].boss)
                            .map(|b| b.name.clone())
                            .unwrap_or_default();
                        self.note("boss_intro", name);
                    } else {
                        self.note("wave", String::new());
                    }
                }
            }
        } else if alive == 0 && self.kind != NodeKind::Boss {
            self.cleared = true;
            self.cleared_at = self.tick;
            self.orbs.clear();
            self.waves.clear();
            let heal = HEAL_ON_CLEAR;
            self.player.heal(heal);
            if let Some(r) = self.reward {
                let kind = reward_pickup(r);
                // on the open floor nearest the spot ahead of the body: the
                // spot itself could be inside a wall
                let ahead = self.player.pos.add(facing(self.player.yaw, 0).scale(int(4)));
                let at = self
                    .arena
                    .ground_spawns
                    .iter()
                    .copied()
                    .min_by_key(|p| p.dist(ahead))
                    .unwrap_or(ahead);
                self.pickups.push(Pickup {
                    pos: at,
                    kind,
                    taken: false,
                    age: 0,
                    group: 0,
                });
            }
            self.note("cleared", c.say("callout_cleared"));
        }
    }

    fn orbs_step(&mut self, c: &Content, chest: V3) {
        let level = &self.arena.level;
        let mut kept = Vec::with_capacity(self.orbs.len());
        let mut hurt: Option<(i32, V3)> = None;
        let mut parried: Vec<Orb> = Vec::new();
        let mut spent: Vec<Orb> = Vec::new();
        let parrying = self.player.parry_window > 0;
        let moving = self.player.pos.sub(self.player.prev_pos);
        let wall = self.wall();
        let mut broken = 0;
        for mut o in std::mem::take(&mut self.orbs) {
            if !o.step(level, chest, moving) {
                spent.push(o);
                continue;
            }
            if o.blue && wall.is_some_and(|w| w.breaks(&o)) {
                broken += 1;
                spent.push(o);
                continue;
            }
            if parrying && o.gold && orbs::within(o.pos, chest, PARRY_REACH) {
                parried.push(o);
                continue;
            }
            if hurt.is_none() && o.touches(self.player.pos) && !self.player.invulnerable() {
                hurt = Some((o.damage, o.pos.sub(o.dir.scale(ONE))));
                spent.push(o);
                continue;
            }
            kept.push(o);
        }
        self.orbs = kept;
        if broken > 0 {
            self.note("shatter", String::new());
        }
        if let Some((damage, from)) = hurt {
            self.hurt_from(damage, from);
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
            self.effect(FxKind::Flash, chest, V3::ZERO, 12, reflect, PARRY_REACH * 2);
            self.note("reflect", c.say("callout_objection"));
            if self.player.build.mend {
                self.player.heal(MEND_HEAL);
            }
        }
        let mut wave_hit: Option<(i32, V3)> = None;
        let feet = self.player.pos;
        let inv = self.player.invulnerable();
        self.waves.retain_mut(|w| {
            let alive = w.step();
            if !w.hit && !inv && w.touches(feet) {
                w.hit = true;
                wave_hit = Some((w.damage, w.centre));
            }
            alive
        });
        if let Some((damage, from)) = wave_hit {
            self.hurt_from(damage, from);
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
            self.burst(c, i);
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
                    group: 0,
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
            if p.group != 0 {
                // a choice: it waits for the interact key, and taking it
                // takes the others in its group away
                self.offer = Some(k);
                if !ctl.interact_pressed {
                    continue;
                }
                for q in self.pickups.iter_mut() {
                    if q.group == p.group {
                        q.taken = true;
                    }
                }
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
                    let text = if p.group != 0 {
                        c.say("callout_fountain")
                    } else {
                        String::new()
                    };
                    self.note("heal", text);
                }
                PickupKind::Pierce => {
                    self.player.build.pierce += PIERCE_BONUS;
                    self.note("pickup", c.say("callout_pierce"));
                }
                PickupKind::Pulse => {
                    self.player.build.pulse = true;
                    self.note("pickup", c.say("callout_pulse"));
                }
                PickupKind::Mend => {
                    self.player.build.mend = true;
                    self.note("pickup", c.say("callout_mend"));
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

    /// Brings enemy `i` back at a spawn point with a clear line to `chest`.
    fn relocate(&mut self, c: &Content, i: usize, chest: V3) {
        let d = &c.enemies[self.enemies[i].def];
        let air = flies(d.kind);
        let pool = if air {
            &self.arena.air_spawns
        } else {
            &self.arena.ground_spawns
        };
        let lift = if air { 0 } else { standing_lift(d) };
        let level = &self.arena.level;
        let fits: Vec<V3> = pool
            .iter()
            .map(|p| p.add(v3(0, lift, 0)))
            .filter(|p| {
                let to = chest.sub(*p);
                let dist = to.len_xz();
                dist > int(10) && dist < int(32) && level.ray(*p, to.norm(), to.len()).is_none()
            })
            .collect();
        let e = &mut self.enemies[i];
        e.unseen = 0;
        if fits.is_empty() {
            return;
        }
        let at = fits[self.rng.range(0, fits.len() as i32 - 1) as usize];
        e.pos = at;
        e.prev = at;
        e.vel = V3::ZERO;
        e.state = enemy::State::Spawning(enemy::SPAWN_TICKS);
        self.effect(
            FxKind::Flash,
            at,
            V3::ZERO,
            enemy::SPAWN_TICKS,
            c.pal.c("enemy_core"),
            d.radius * 3,
        );
    }

    /// The Injunction's wall while it stands.
    pub fn wall(&self) -> Option<WallPlane> {
        if self.player.wall_left == 0 {
            return None;
        }
        let n = facing(self.player.yaw, 0);
        Some(WallPlane {
            centre: self.player.chest().add(n.scale(WALL_AHEAD)),
            normal: n,
            across: right_of(self.player.yaw),
            floor: self.player.pos.y - ONE / 5,
        })
    }

    /// An enemy that bursts where it dies fires its pattern once.
    fn burst(&mut self, c: &Content, i: usize) {
        let d = &c.enemies[self.enemies[i].def];
        let Some(p) = d.burst else {
            return;
        };
        let at = self.enemies[i].pos;
        let chest = self.player.chest();
        for v in &c.patterns[p].volleys {
            let s = orbs::fire(v, at, chest, 0, i, &c.pal, &self.arena.level);
            let room = MAX_ORBS.saturating_sub(self.orbs.len());
            self.orbs.extend(s.orbs.into_iter().take(room));
            self.waves.extend(s.waves);
        }
    }

    /// A hit, and where it came from.
    fn hurt_from(&mut self, damage: i32, from: V3) {
        if self.player.hurt(damage) {
            self.hit_from = from;
            self.hit_age = 0;
        }
    }

    /// A hook aimed at a wall just under a ledge catches above the ledge
    /// instead, so it lifts you onto the roof (SECOND-ORDER P5).
    fn mantle(&self, eye: V3, fwd: V3, p: V3) -> V3 {
        let skip = eye.dist(self.player.chest()) + ONE / 2;
        let start = eye.add(fwd.scale(skip));
        let Some(hit) = self.arena.level.ray(start, fwd, GRAPPLE_RANGE + skip) else {
            return p;
        };
        let s = &self.arena.level.solids[hit.solid];
        let below_top = s.max.y - hit.point.y;
        if below_top > ONE / 20 && below_top <= MANTLE_REACH {
            let flat = v3(fwd.x, 0, fwd.z).norm();
            let over = hit.point.add(flat.scale(ONE * 6 / 10));
            return v3(over.x, s.max.y + ONE * 13 / 10, over.z);
        }
        p
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

/// The Injunction's wall: a standing rectangle facing the way you face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WallPlane {
    pub centre: V3,
    pub normal: V3,
    pub across: V3,
    /// its foot; it stands `WALL_HEIGHT` over it
    pub floor: i32,
}

impl WallPlane {
    /// Does an orb meet the wall this tick? Its path from `prev` to `pos`
    /// reaches the wall's plane, within the wall's width and height.
    pub fn breaks(&self, o: &Orb) -> bool {
        let r = o.radius;
        let d0 = o.prev.sub(self.centre).dot(self.normal);
        let d1 = o.pos.sub(self.centre).dot(self.normal);
        if d0.min(d1) > r || d0.max(d1) < -r {
            return false;
        }
        let side = o.pos.sub(self.centre).dot(self.across).abs();
        side <= WALL_HALF_WIDTH + r && o.pos.y >= self.floor - r && o.pos.y <= self.floor + WALL_HEIGHT + r
    }
}

/// Kinds that spawn in the air.
fn flies(k: crate::content::Kind) -> bool {
    use crate::content::Kind;
    matches!(k, Kind::Floater | Kind::Moth | Kind::Blinker)
}

/// How far over the floor a ground kind's centre stands.
fn standing_lift(d: &crate::content::EnemyDef) -> i32 {
    use crate::content::Kind;
    match d.kind {
        Kind::Walker | Kind::Burrower => d.radius,
        _ => d.hover,
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
    use crate::level::{Level, Solid};
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
            gold: true,
            ..Orb::new(
                chest.add(v3(0, 0, -int(2))),
                v3(0, 0, ONE),
                ONE / 10,
                ONE * 4 / 10,
                gold,
                ORB_DAMAGE,
            )
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
            gold: true,
            ..Orb::new(
                chest.add(v3(0, 0, -int(2))),
                v3(0, 0, ONE),
                ONE / 10,
                ONE * 4 / 10,
                [0; 3],
                ORB_DAMAGE,
            )
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
        // without the Antechamber's own choice, which waits for E
        w.pickups.clear();
        w.player.build.jumps = 3;
        for _ in 0..2 {
            w.pickups.push(Pickup {
                pos: w.player.pos,
                kind: PickupKind::Jump,
                taken: false,
                age: 0,
                group: 0,
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
        w.pickups.clear();
        w.pickups.push(Pickup {
            pos: w.player.pos,
            kind: PickupKind::Gun(2, 3),
            taken: false,
            age: 0,
            group: 0,
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

    fn orb_at(at: V3, gold: bool) -> Orb {
        Orb {
            gold,
            ..Orb::new(at, v3(0, 0, ONE), ONE / 20, ONE * 4 / 10, [0; 3], ORB_DAMAGE)
        }
    }

    #[test]
    fn a_dash_pulse_clears_the_orbs_round_you_and_only_those() {
        let (c, mut w) = world_for(NodeKind::Fight);
        w.waves_left.clear();
        w.cleared = true;
        w.player.build.pulse = true;
        let chest = w.player.chest();
        w.orbs.push(orb_at(chest.add(v3(int(2), 0, 0)), false));
        w.orbs.push(orb_at(chest.add(v3(-int(10), 0, 0)), false));
        let (eye, fwd) = eye_of(&w);
        let dash = Controls {
            dash_pressed: true,
            ..Controls::default()
        };
        w.tick(&c, &dash, eye, fwd);
        // PULSE_RADIUS is 2.5 m: the orb 2 m off is gone, the one 10 m off stays
        assert_eq!(w.orbs.len(), 1);
        assert!(w.orbs[0].pos.x < 0);
        assert_eq!(w.pulse_cooldown, PULSE_COOLDOWN - 1);
    }

    #[test]
    fn a_mending_parry_heals_when_it_turns_an_orb() {
        let (c, mut w) = world_for(NodeKind::Fight);
        w.waves_left.clear();
        w.cleared = true;
        w.player.build.parry = true;
        w.player.build.mend = true;
        w.player.health = 50;
        let chest = w.player.chest();
        w.orbs.push(orb_at(chest.add(v3(0, 0, -int(2))), true));
        let (eye, fwd) = eye_of(&w);
        let parry = Controls {
            parry_pressed: true,
            ..Controls::default()
        };
        w.tick(&c, &parry, eye, fwd);
        assert_eq!(w.player.health, 50 + MEND_HEAL);
    }

    #[test]
    fn the_antechamber_offers_health_or_its_reward_and_taking_one_loses_the_other() {
        let (c, mut w) = world_for(NodeKind::Rest);
        assert_eq!(w.pickups.len(), 2, "the fountain's health and the reward");
        assert!(w.pickups.iter().all(|p| p.group == 1));
        // walk onto the health pedestal: nothing until E
        w.player.pos = w.pickups[0].pos;
        w.player.health = 10;
        let (eye, fwd) = eye_of(&w);
        w.tick(&c, &Controls::default(), eye, fwd);
        assert_eq!(w.player.health, 10);
        let take = Controls {
            interact_pressed: true,
            ..Controls::default()
        };
        w.tick(&c, &take, eye, fwd);
        // REST_HEAL_PCT is 50: half of 100
        assert_eq!(w.player.health, 10 + 50);
        assert!(w.pickups.iter().all(|p| p.taken), "the other choice is gone");
    }

    #[test]
    fn a_hook_aimed_just_under_a_ledge_catches_above_it() {
        let (_, w) = world_for(NodeKind::Fight);
        let mut l = vec![Solid::new(v3(-int(50), -ONE, -int(50)), v3(int(50), 0, int(50)))];
        // a wall 10 m ahead with its top at 4 m
        l.push(Solid::new(v3(-int(5), 0, -int(12)), v3(int(5), int(4), -int(10))));
        let mut w = w;
        w.arena.level = Level::new(l);
        w.player.pos = v3(0, 0, 0);
        let eye = v3(0, int(3), int(1));
        // aimed at the wall's face at about 3 m up: within MANTLE_REACH of the top
        let fwd = v3(0, 0, -ONE);
        let anchor = w.mantle(eye, fwd, v3(0, int(3), -int(10)));
        assert!(anchor.y > int(4), "{anchor:?}");
        assert!(anchor.z < -int(10), "over the top, not at the face: {anchor:?}");
        // aimed low on the wall, it catches where it hits
        let low = w.mantle(v3(0, ONE, int(1)), fwd, v3(0, ONE, -int(10)));
        assert_eq!(low, v3(0, ONE, -int(10)));
    }

    #[test]
    fn an_enemy_lost_behind_cover_comes_back_where_it_can_see_you() {
        let (c, mut w) = world_for(NodeKind::Fight);
        w.waves_left.clear();
        let d = c.enemy("sentinel").unwrap();
        let mut e = Enemy::new(d, &c.enemies[d], v3(int(80), int(2), int(80)), 1);
        e.state = enemy::State::Rest(10_000);
        e.unseen = LOST_TICKS + 1;
        w.enemies.push(e);
        let (eye, fwd) = eye_of(&w);
        w.tick(&c, &Controls::default(), eye, fwd);
        let e = &w.enemies[0];
        assert!(e.pos.x.abs() < int(40), "moved back into the arena: {:?}", e.pos);
        assert!(
            e.sees(&w.arena.level, w.player.chest()),
            "with a clear line to you"
        );
        assert!(
            matches!(e.state, enemy::State::Spawning(_)),
            "and appears as a new enemy does"
        );
    }

    fn ice_world(kind: NodeKind) -> (Content, World) {
        let c = content();
        let r = Route::build_for(&c, 1, 11, 1);
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

    /// An orb flying at the chest from `ahead` metres in front.
    fn incoming(w: &World, ahead: i32, side: i32, blue: bool) -> Orb {
        let chest = w.player.chest();
        let n = facing(w.player.yaw, 0);
        let at = chest
            .add(n.scale(int(ahead)))
            .add(right_of(w.player.yaw).scale(int(side)));
        Orb {
            blue,
            ..Orb::new(at, n.neg(), ONE / 4, ONE * 3 / 10, [0; 3], ORB_DAMAGE)
        }
    }

    #[test]
    fn the_injunction_breaks_blue_orbs_in_front_of_you_and_lets_the_rest_through() {
        let (c, mut w) = world_for(NodeKind::Fight);
        w.waves_left.clear();
        w.cleared = true;
        w.player.build.wall = true;
        w.orbs.push(incoming(&w, 5, 0, true));
        w.orbs.push(incoming(&w, 5, 0, false));
        // blue, but wide of the wall
        w.orbs.push(incoming(&w, 5, 6, true));
        let (eye, fwd) = eye_of(&w);
        let wall = Controls {
            wall_pressed: true,
            ..Controls::default()
        };
        w.tick(&c, &wall, eye, fwd);
        assert!(w.wall().is_some());
        // 15 m/s from 5 m out: the blue orb meets the wall, 1.8 m out, a
        // few ticks before either orb could reach you
        for _ in 0..12 {
            w.tick(&c, &Controls::default(), eye, fwd);
        }
        let right = right_of(w.player.yaw);
        let chest = w.player.chest();
        let in_front = |o: &Orb| o.pos.sub(chest).dot(right).abs() < ONE;
        assert!(
            !w.orbs.iter().any(|o| o.blue && in_front(o)),
            "the blue orb in front broke at the wall"
        );
        assert!(w.orbs.iter().any(|o| !o.blue), "the other colour still flies");
        assert_eq!(
            w.player.health, w.player.build.max_health,
            "nothing has reached you yet"
        );
        for _ in 0..20 {
            w.tick(&c, &Controls::default(), eye, fwd);
        }
        assert!(
            w.orbs.iter().any(|o| o.blue),
            "the blue orb wide of the wall did not break"
        );
        // the other colour came through and hit
        assert_eq!(w.player.health, w.player.build.max_health - ORB_DAMAGE);
    }

    #[test]
    fn the_wall_stands_half_a_second_and_then_waits_its_cooldown() {
        let (c, mut w) = world_for(NodeKind::Fight);
        w.waves_left.clear();
        w.cleared = true;
        w.player.build.wall = true;
        let (eye, fwd) = eye_of(&w);
        let wall = Controls {
            wall_pressed: true,
            ..Controls::default()
        };
        w.tick(&c, &wall, eye, fwd);
        let mut up = 0;
        while w.wall().is_some() {
            up += 1;
            w.tick(&c, &wall, eye, fwd);
            assert!(up < 100);
        }
        assert_eq!(up, WALL_TICKS);
        // pressed again at once: nothing until the cooldown is over
        w.tick(&c, &wall, eye, fwd);
        assert!(w.wall().is_none());
        // without the upgrade there is no wall at all
        let (c2, mut w2) = world_for(NodeKind::Fight);
        w2.tick(&c2, &wall, eye, fwd);
        assert!(w2.wall().is_none());
    }

    #[test]
    fn an_updraft_vent_throws_you_up_about_six_metres() {
        let (c, mut w) = ice_world(NodeKind::Fight);
        w.waves_left.clear();
        w.cleared = true;
        let v = *w.arena.vents.first().expect("a fight on the ice has vents");
        w.player.pos = v.pos;
        w.player.prev_pos = v.pos;
        let (eye, fwd) = eye_of(&w);
        let mut top = v.pos.y;
        for _ in 0..120 {
            w.tick(&c, &Controls::default(), eye, fwd);
            top = top.max(w.player.pos.y);
            // the first rise only: falling back into the vent throws you again
            if w.player.vel.y < 0 {
                break;
            }
        }
        let rise = top - v.pos.y;
        assert!(rise > int(5) && rise < int(7), "{rise}");
    }

    #[test]
    fn a_shardling_leaves_a_held_ring_where_it_dies() {
        let (c, mut w) = ice_world(NodeKind::Fight);
        w.waves_left.clear();
        let d = c.enemy("shardling").unwrap();
        let at = w.player.chest().add(v3(0, 0, -int(8)));
        let mut e = Enemy::new(d, &c.enemies[d], at, 1);
        e.state = enemy::State::Rest(1000);
        w.enemies.push(e);
        w.enemies[0].hp = 0;
        w.burst(&c, 0);
        let ring = c.patterns.iter().find(|p| p.id == "shatter").unwrap().volleys[0].count as usize;
        assert_eq!(w.orbs.len(), ring);
        assert!(w.orbs.iter().all(|o| o.wait > 0 && o.blue && o.hub == at));
    }

    #[test]
    fn an_auger_cannot_be_hurt_under_the_floor_and_comes_up_to_attack() {
        let (c, mut w) = ice_world(NodeKind::Fight);
        w.waves_left.clear();
        let d = c.enemy("auger").unwrap();
        let at = w.player.pos.add(v3(int(4), c.enemies[d].radius, -int(10)));
        w.enemies.push(Enemy::new(d, &c.enemies[d], at, 1));
        let (eye, fwd) = eye_of(&w);
        let mut was_buried = false;
        let mut came_up = false;
        let mut fired = false;
        for _ in 0..60 * 8 {
            w.player.iframes = 1000;
            w.tick(&c, &Controls::default(), eye, fwd);
            let e = &w.enemies[0];
            if e.buried {
                was_buried = true;
                assert!(!e.hurtable(), "under the floor it cannot be hurt");
                assert!(e.pos.y < 0, "and is under the floor: {:?}", e.pos);
            } else if was_buried {
                came_up = true;
            }
            fired |= !w.orbs.is_empty();
        }
        assert!(was_buried && came_up && fired);
    }

    #[test]
    fn a_clerk_reappears_elsewhere_in_sight_of_you() {
        let (c, mut w) = world_for(NodeKind::Fight);
        w.waves_left.clear();
        let d = c.enemy("clerk").unwrap();
        let start = w.player.chest().add(v3(int(3), int(1), -int(12)));
        let mut e = Enemy::new(d, &c.enemies[d], start, 1);
        e.state = enemy::State::Rest(2);
        w.enemies.push(e);
        let (eye, fwd) = eye_of(&w);
        let mut moved_to = None;
        for _ in 0..4 {
            w.tick(&c, &Controls::default(), eye, fwd);
            if w.enemies[0].pos.dist(start) > int(3) {
                moved_to = Some(w.enemies[0].pos);
                break;
            }
        }
        let to = moved_to.expect("it blinked");
        assert!(
            w.enemies[0].sees(&w.arena.level, w.player.chest()),
            "where it can see you"
        );
        assert!(
            matches!(w.enemies[0].state, enemy::State::Windup(_)),
            "and winds up there"
        );
        assert!(to.x.abs() <= int(24) && to.z.abs() <= int(24));
    }

    #[test]
    fn the_landing_offers_a_cache_of_two_and_taking_one_loses_the_other() {
        let (c, mut w) = ice_world(NodeKind::Arrival);
        assert_eq!(w.pickups.len(), 2);
        assert!(w.pickups.iter().all(|p| p.group == 2));
        assert_ne!(w.pickups[0].kind, w.pickups[1].kind);
        w.player.pos = w.pickups[1].pos;
        let (eye, fwd) = eye_of(&w);
        let take = Controls {
            interact_pressed: true,
            ..Controls::default()
        };
        w.tick(&c, &take, eye, fwd);
        assert!(w.pickups.iter().all(|p| p.taken));
        // zone one's crater has no cache
        let (_, crater) = world_for(NodeKind::Arrival);
        assert!(crater.pickups.is_empty());
    }
}
