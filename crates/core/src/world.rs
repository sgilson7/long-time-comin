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
use crate::content::Kind;
use crate::rng::Rng;
use crate::route::{Node, NodeKind, Reward};
use crate::tank;
use crate::weapons::{self, Held, Lodge, Pull, Shot, ShotKind};

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
    Trigger,
    Plating,
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
        Reward::Trigger => PickupKind::Trigger,
        Reward::Plating => PickupKind::Plating,
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
    /// a ring along the floor, growing to `size` (a ground pound)
    Ring,
    /// a piece of glass, tumbling down
    Shard,
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
/// the level of the Ripsaw the Lien leaves at an arrival
pub const LIEN_LEVEL: i32 = 2;
/// a pane of glass this far from where the Executor breaks through breaks
/// this many ticks later
const GLASS_RIPPLE: i32 = int(3);
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
    /// a mini-boss in the arena, for its bar
    pub elite: Option<usize>,
    /// ticks the camera still shakes
    pub shake: i32,
    /// for each pane of glass, the tick it breaks on (0: whole)
    pub broken: Vec<i32>,
    /// ticks the trigger has been held, for the saw's rev
    pub revving: i32,
    /// the enemy whose orb or touch last hurt you, for the playtest harness
    pub hit_by: Option<usize>,
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
            elite: None,
            shake: 0,
            broken: Vec::new(),
            revving: 0,
            hit_by: None,
        };
        w.broken = vec![0; w.arena.panes.len()];
        if let Some(def) = node.elite {
            // a mini-boss, alone
            w.waves_left = vec![vec![def]];
        }
        if node.kind == NodeKind::Arrival && w.player.build.lien {
            // the Lien: a Ripsaw waits where you arrive
            let saw = c.gun("ripsaw");
            if w.held.gun != saw && c.guns.get(saw).is_some_and(|g| g.id == "ripsaw") {
                let at = w.arena.start.add(v3(int(2), 0, -int(3)));
                w.pickups.push(Pickup {
                    pos: at,
                    kind: PickupKind::Gun(saw, LIEN_LEVEL),
                    taken: false,
                    age: 0,
                    group: 0,
                });
            }
        }
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
        let reach = skip + self.player.build.grapple_range();
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
            for (at, r) in body_spheres(c, e) {
                if let Some(t) = ray_sphere(o, d, at, r) {
                    if t >= skip && t < best {
                        best = t;
                        who = Some(i);
                    }
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
        let mut at = if d.kind == Kind::Boss || list.is_empty() {
            self.arena.centre
        } else {
            list[self.rng.range(0, list.len() as i32 - 1) as usize]
        };
        if !air && d.kind != Kind::Boss {
            at.y += standing_lift(d);
        }
        if d.kind == Kind::Tank {
            // outside, beyond the glass, coming in
            at = v3(0, tank::HULL_Y, -crate::arena::COURT_RADIUS - int(14));
        }
        let orbit = if self.rng.chance(50) { 1 } else { -1 };
        let mut e = Enemy::new(def, d, at, orbit);
        e.home = self.arena.centre;
        if d.kind == Kind::Tank {
            e.state = enemy::State::Rest(tank::ENTER_TICKS + secs(1, 1));
        }
        if matches!(d.kind, Kind::Boss | Kind::Tank) {
            self.boss = Some(self.enemies.len());
        }
        if d.elite {
            self.elite = Some(self.enemies.len());
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
                Event::Pound => "pound",
            };
            let text = match ev {
                Event::Recess => c.say("callout_recess"),
                Event::Fell => c.say("callout_fell"),
                Event::HookSnag => c.say("callout_hook_snag"),
                _ => String::new(),
            };
            self.note(sound, text);
            if ev == Event::Pound {
                self.pound(c);
            }
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
        let pull = Pull {
            held: ctl.fire,
            pressed: ctl.fire_pressed,
            pierce: self.player.build.pierce,
            trigger: self.player.build.trigger,
        };
        let shots = weapons::trigger(&mut self.held, gun, rgb, pull, muzzle, self.aim_point, &mut self.rng);
        self.revving = if ctl.fire { self.revving + 1 } else { 0 };
        if !shots.is_empty() {
            let sound = match gun.id.as_str() {
                "carbine" => "fire_carbine",
                "scattergun" => "fire_scatter",
                "ripsaw" => "saw_throw",
                _ => "fire_launcher",
            };
            self.note(sound, String::new());
            self.effect(FxKind::Flash, muzzle, V3::ZERO, 3, rgb, ONE / 3);
        }
        self.shots.extend(shots);

        // the waves of enemies, and those a caller calls
        self.waves_step(c);
        self.calls_step(c);

        // the house's agents work as a squad
        if let Some(nav) = self.arena.nav.as_ref() {
            let mark = crate::squad::Mark {
                feet: self.player.pos,
                chest,
                yaw: self.player.yaw,
            };
            crate::squad::direct(&mut self.enemies, c, nav, &self.arena.level, &mark, self.tick);
        }

        // an enemy that has not seen you for a long while comes back where
        // it can: no fight can stall behind a wall (SECOND-ORDER P7)
        for i in 0..self.enemies.len() {
            let e = &self.enemies[i];
            let walks = matches!(c.enemies[e.def].kind, Kind::Agent | Kind::Tank);
            if e.alive() && e.hurtable() && e.unseen > LOST_TICKS && Some(i) != self.boss && !walks {
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
        self.glass_step(c);
        self.shake = (self.shake - 1).max(0);

        self.orbs_step(c, chest);
        self.shots_step(c);
        self.saws_step(c);

        // touching an enemy hurts; a moth bursts on you
        let feet = self.player.pos;
        for i in 0..self.enemies.len() {
            let d = &c.enemies[self.enemies[i].def];
            if self.enemies[i].alive() && enemy::contact(&self.enemies[i], d, feet) {
                let dmg = d.contact;
                if d.kind == Kind::Tank && !self.player.invulnerable() {
                    // run down: thrown clear of the treads
                    let away = v3(feet.x - self.enemies[i].pos.x, 0, feet.z - self.enemies[i].pos.z).norm();
                    let push = away.scale(TANK_PUSH);
                    self.player.vel = v3(push.x, TANK_PUSH / 2, push.z);
                    self.player.grounded = false;
                    self.shake = self.shake.max(SHAKE_TICKS / 3);
                }
                if d.kind == crate::content::Kind::Moth {
                    self.enemies[i].hp = 0;
                    let at = self.enemies[i].pos;
                    self.sparks(at, c.pal.c("enemy_eye"), 14, ONE / 5);
                    self.note("boom", String::new());
                    self.burst(c, i);
                }
                let from = self.enemies[i].pos;
                if !self.player.invulnerable() {
                    self.hit_by = Some(i);
                }
                self.hurt_from(dmg, from);
            }
        }

        self.pickups_step(c, ctl);
        self.gates_step();
        self.effects_step();

        if let Some(b) = self.boss {
            if !self.enemies[b].alive() {
                if self.win_timer == 0 {
                    let name = c
                        .story
                        .bosses
                        .get(&c.enemies[self.enemies[b].def].id)
                        .map(|w| w.name.clone())
                        .unwrap_or_default();
                    self.note("boss_down", c.say("callout_boss_down").replace("{name}", &name));
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
            let heal = HEAL_ON_CLEAR + self.player.build.bail;
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
                self.hit_by = Some(o.owner);
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
            // a lobbed canister plants its pattern where it lands
            if let (Some(p), true) = (o.plant, o.landed) {
                let at = o.prev.add(v3(0, ONE / 2, 0));
                for v in &c.patterns[p].volleys {
                    let s = orbs::fire(v, at, chest, 0, o.owner, &c.pal, &self.arena.level);
                    let room = MAX_ORBS.saturating_sub(self.orbs.len());
                    self.orbs.extend(s.orbs.into_iter().take(room));
                    self.waves.extend(s.waves);
                }
                self.note("plant", String::new());
            }
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
                lodged: None,
                teeth: 0,
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
        let mut glanced: Vec<V3> = Vec::new();
        let mut weak_hits: Vec<(usize, usize, i32, V3)> = Vec::new();
        for mut s in std::mem::take(&mut self.shots) {
            s.prev = s.pos;
            if s.lodged.is_some() {
                // a lodged blade: the saws' own step moves and spends it
                kept.push(s);
                continue;
            }
            s.life -= 1;
            let len = s.vel.len();
            let dir = s.vel.norm();
            let wall = self.arena.level.ray(s.pos, dir, len).map(|h| h.t);
            let first = self.first_hit(c, s.pos, dir, s.radius, len, &s.passed);
            let enemy_first = match (first, wall) {
                (Some((te, _)), Some(tw)) => te <= tw,
                (Some(_), None) => true,
                _ => false,
            };
            if enemy_first {
                let (t, target) = first.unwrap_or((0, Target::Armour(0)));
                let at = s.pos.add(dir.scale(t));
                match target {
                    Target::Armour(_) => {
                        // it glances off: armour, or a shield it met in front
                        glanced.push(at);
                        if s.kind == ShotKind::Saw {
                            self.held.recall_blade(saw_recall(c, &self.held));
                        }
                        if s.kind == ShotKind::Bomb {
                            blasts.push((at, s.splash_radius, s.splash, s.erases, s.rgb, None));
                        }
                        continue;
                    }
                    Target::Weak(i, k) => {
                        weak_hits.push((i, k, s.damage, at));
                        hits.push((at, s.rgb));
                        if s.kind == ShotKind::Saw {
                            let e = &self.enemies[i];
                            s.lodged = Some(Lodge::Enemy {
                                enemy: i,
                                weak: Some(k),
                                offset: at.sub(e.pos),
                            });
                            s.pos = at;
                            kept.push(s);
                            continue;
                        }
                        if s.kind == ShotKind::Bomb {
                            blasts.push((at, s.splash_radius, s.splash, s.erases, s.rgb, None));
                        }
                        continue;
                    }
                    Target::Body(i) => {
                        if self.enemies[i].hurt(s.damage) {
                            killed.push(i);
                        }
                        hits.push((at, s.rgb));
                        if s.kind == ShotKind::Saw {
                            if self.enemies[i].alive() {
                                s.lodged = Some(Lodge::Enemy {
                                    enemy: i,
                                    weak: None,
                                    offset: at.sub(self.enemies[i].pos).scale(ONE * 8 / 10),
                                });
                                s.pos = at;
                                kept.push(s);
                            } else {
                                self.held.recall_blade(saw_recall(c, &self.held));
                            }
                            continue;
                        }
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
                    }
                }
            } else if let Some(t) = wall {
                let at = s.pos.add(dir.scale(t));
                if s.kind == ShotKind::Bomb {
                    blasts.push((at, s.splash_radius, s.splash, s.erases, s.rgb, None));
                } else if s.kind == ShotKind::Saw {
                    // it bites into the wall for a moment
                    s.pos = at.sub(dir.scale(s.radius));
                    s.lodged = Some(Lodge::Wall(SAW_IN_WALL));
                    kept.push(s);
                    continue;
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
                if s.kind == ShotKind::Saw {
                    self.held.recall_blade(saw_recall(c, &self.held));
                }
                continue;
            }
            kept.push(s);
        }
        for at in glanced {
            self.sparks(at, c.pal.c("spark"), 6, ONE / 6);
            self.note("clank", String::new());
        }
        for (i, k, dmg, at) in weak_hits {
            self.hurt_weak(c, i, k, dmg, at);
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
                for (wp, wr, k) in tank::weak_points(&self.enemies[i], &c.enemies[self.enemies[i].def]) {
                    if orbs::within(wp, at, radius + wr) {
                        self.hurt_weak(c, i, k, damage, wp);
                    }
                }
            }
            if erases {
                self.orbs.retain(|o| !orbs::within(o.pos, at, radius));
            }
        }
        self.fell(c, killed);
    }

    /// Enemies killed: their Clemency, their bursts, their heals.
    fn fell(&mut self, c: &Content, mut killed: Vec<usize>) {
        killed.sort();
        killed.dedup();
        for i in killed {
            self.kills += 1;
            self.burst(c, i);
            let e = &self.enemies[i];
            let (at, def) = (e.pos, e.def);
            let d = &c.enemies[def];
            // what a caller called is worth nothing: a fight with a caller
            // in it would never run dry
            if e.summoned_by.is_none() {
                self.earned += d.clemency;
            }
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
                PickupKind::Trigger => {
                    self.player.build.trigger += 1;
                    self.note("pickup", c.say("callout_trigger"));
                }
                PickupKind::Plating => {
                    if self.player.build.plating < PLATING_MAX {
                        self.player.build.plating += 1;
                    } else {
                        let max = self.player.build.max_health;
                        self.player.heal(max);
                    }
                    self.note("pickup", c.say("callout_plating"));
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
            if matches!(e.kind, FxKind::Spark | FxKind::Shard) {
                e.pos = e.pos.add(e.vel);
                e.vel.y -= g;
                if e.kind == FxKind::Shard && e.pos.y < ONE / 20 {
                    // glass skitters along the floor
                    e.pos.y = ONE / 20;
                    e.vel = v3(e.vel.x / 2, (-e.vel.y) / 4, e.vel.z / 2);
                }
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

/// What a shot meets first on an enemy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    /// its body, which it hurts
    Body(usize),
    /// a tank's weak point
    Weak(usize, usize),
    /// armour, or a shield met from in front: it glances off
    Armour(usize),
}

/// a blade that misses sticks in the wall this long
const SAW_IN_WALL: i32 = secs(1, 2);
/// the saws' rev rattles this often
const REV_NOTE: i32 = 6;

/// The ticks a spent blade of the held gun takes to come back.
fn saw_recall(c: &Content, h: &Held) -> i32 {
    c.guns[h.gun].level(h.level).saw.map_or(1, |s| s.recall)
}

/// The spheres a shot or the aim can meet on an enemy: a tank's armour and
/// weak points, or the body.
fn body_spheres(c: &Content, e: &Enemy) -> Vec<(V3, i32)> {
    let d = &c.enemies[e.def];
    if d.kind == Kind::Tank {
        let mut v = tank::armour(e);
        v.extend(tank::weak_points(e, d).into_iter().map(|(p, r, _)| (p, r)));
        v
    } else {
        vec![(e.pos, d.radius)]
    }
}

impl World {
    /// The first enemy part along a ray, within `len`.
    fn first_hit(&self, c: &Content, o: V3, dir: V3, radius: i32, len: i32, passed: &[usize]) -> Option<(i32, Target)> {
        let mut first: Option<(i32, Target)> = None;
        let mut take = |t: i32, what: Target| {
            if t <= len && first.is_none_or(|(ft, _)| t < ft) {
                first = Some((t, what));
            }
        };
        for (i, e) in self.enemies.iter().enumerate() {
            if !e.hurtable() || passed.contains(&i) {
                continue;
            }
            let d = &c.enemies[e.def];
            if d.kind == Kind::Tank {
                for (at, r) in tank::armour(e) {
                    if let Some(t) = ray_sphere(o, dir, at, r + radius) {
                        take(t, Target::Armour(i));
                    }
                }
                for (at, r, k) in tank::weak_points(e, d) {
                    if let Some(t) = ray_sphere(o, dir, at, r + radius) {
                        // a weak point stands proud of the armour: a touch
                        // nearer wins
                        take(t - ONE / 20, Target::Weak(i, k));
                    }
                }
                continue;
            }
            if let Some(t) = ray_sphere(o, dir, e.pos, d.radius + radius) {
                // a shield stops what comes at it from in front, unless
                // it comes from well above
                let from = dir.neg();
                let shielded = d.armour > 0 && from.dot(facing(e.yaw, 0)) > fx::cos(d.armour) && from.y < ONE / 2;
                take(t, if shielded { Target::Armour(i) } else { Target::Body(i) });
            }
        }
        first
    }

    /// Damage to a tank's weak point; breaking the phase's last one starts
    /// the next phase.
    fn hurt_weak(&mut self, c: &Content, i: usize, k: usize, damage: i32, at: V3) {
        let d = &c.enemies[self.enemies[i].def];
        if !self.enemies[i].hurtable() {
            return;
        }
        let e = &mut self.enemies[i];
        let Some(t) = e.tank.as_mut() else {
            return;
        };
        let Some(hp) = t.weak.get_mut(k) else {
            return;
        };
        if *hp <= 0 {
            return;
        }
        *hp -= damage;
        let broke = *hp <= 0;
        let left = t.weak.iter().filter(|h| **h > 0).count();
        e.flash = 0;
        e.hp = (e.hp - damage).max(1);
        if !broke {
            return;
        }
        self.effect(FxKind::Burst, at, V3::ZERO, 30, c.pal.c("tank_weak"), int(3));
        self.sparks(at, c.pal.c("spark"), 30, ONE / 3);
        self.note("weak_break", c.say("callout_weak"));
        self.shake = self.shake.max(SHAKE_TICKS / 2);
        if left > 0 {
            return;
        }
        // every weak point of the phase broken: the next phase grows
        let next = self.enemies[i].phase + 1;
        let Some(phase) = d.phases.get(next) else {
            return;
        };
        let e = &mut self.enemies[i];
        e.phase = next;
        e.attack = 0;
        e.state = enemy::State::Changing(enemy::PHASE_TICKS);
        if let Some(t) = e.tank.as_mut() {
            t.begin(phase);
        }
        for o in std::mem::take(&mut self.orbs) {
            self.effect(FxKind::Burst, o.pos, V3::ZERO, 14, o.rgb, o.radius * 2);
        }
        self.waves.clear();
        let text = if phase.survive > 0 {
            c.say("callout_survive")
        } else {
            c.say("callout_phase")
        };
        self.note("boss_phase", text);
    }

    /// The Ripsaw's lodged blades: they ride what they are in, rev while
    /// the trigger is held, and come back when their teeth are spent or
    /// what they were in is gone.
    fn saws_step(&mut self, c: &Content) {
        let gun = &c.guns[self.held.gun];
        let lvl = gun.level(self.held.level);
        let rev = lvl.saw.filter(|_| self.revving > 0);
        let every = lvl
            .saw
            .map_or(1, |s| weapons::rate(s.rev_every, self.player.build.trigger));
        let on_beat = self.revving > 0 && self.revving % every == 0;
        let mut spent = 0;
        let mut damage: Vec<(usize, Option<usize>, i32, V3)> = Vec::new();
        for s in self.shots.iter_mut() {
            if s.kind != ShotKind::Saw {
                continue;
            }
            match s.lodged {
                Some(Lodge::Wall(t)) => {
                    s.lodged = if t > 1 { Some(Lodge::Wall(t - 1)) } else { None };
                    if t <= 1 {
                        s.life = 0;
                        spent += 1;
                    }
                }
                Some(Lodge::Enemy { enemy, weak, offset }) => {
                    let e = &self.enemies[enemy];
                    let d = &c.enemies[e.def];
                    let at = match weak {
                        Some(k) => tank::weak_points(e, d)
                            .into_iter()
                            .find(|w| w.2 == k)
                            .map(|w| w.0),
                        None => e.alive().then(|| e.pos.add(offset)),
                    };
                    let Some(at) = at else {
                        s.life = 0;
                        s.lodged = None;
                        spent += 1;
                        continue;
                    };
                    s.prev = s.pos;
                    s.pos = at;
                    if let (Some(r), true) = (rev, on_beat) {
                        let dmg = r.rev_damage.min(s.teeth);
                        s.teeth -= dmg;
                        damage.push((enemy, weak, dmg, at));
                        if s.teeth <= 0 {
                            s.life = 0;
                            s.lodged = None;
                            spent += 1;
                        }
                    }
                }
                None => {}
            }
        }
        // a spent blade, lodged no more, flies no more
        self.shots.retain(|s| !(s.kind == ShotKind::Saw && s.lodged.is_none() && s.life <= 0));
        for _ in 0..spent {
            self.held.recall_blade(saw_recall(c, &self.held));
        }
        let mut killed = Vec::new();
        let any = !damage.is_empty();
        let rgb = c.pal.c(&gun.colour);
        for (i, weak, dmg, at) in damage {
            match weak {
                Some(k) => self.hurt_weak(c, i, k, dmg, at),
                None => {
                    if self.enemies[i].hurt(dmg) {
                        killed.push(i);
                    }
                }
            }
            self.sparks(at, rgb, 3, ONE / 6);
        }
        if any && self.revving % REV_NOTE == 0 {
            self.note("rev", String::new());
        }
        self.fell(c, killed);
    }

    /// A ground pound lands: a shockwave along the floor.
    fn pound(&mut self, c: &Content) {
        let feet = self.player.pos;
        let (radius, damage) = if self.player.build.pound >= 2 {
            (POUND_PLUS_RADIUS, POUND_PLUS_DAMAGE)
        } else {
            (POUND_RADIUS, POUND_DAMAGE)
        };
        let mut killed = Vec::new();
        for i in 0..self.enemies.len() {
            let e = &self.enemies[i];
            if !e.hurtable() {
                continue;
            }
            let d = &c.enemies[e.def];
            let near = e.pos.sub(feet).len_xz() < radius + d.radius && (e.pos.y - feet.y).abs() < int(4);
            if near && self.enemies[i].hurt(damage) {
                killed.push(i);
            }
            for (wp, wr, k) in tank::weak_points(&self.enemies[i], d) {
                if wp.sub(feet).len_xz() < radius + wr && (wp.y - feet.y).abs() < int(6) {
                    self.hurt_weak(c, i, k, damage, wp);
                }
            }
        }
        let rgb = c.pal.c("pound");
        self.effect(FxKind::Ring, feet.add(v3(0, ONE / 10, 0)), V3::ZERO, 20, rgb, radius * 2);
        self.sparks(feet, rgb, 24, ONE / 4);
        self.shake = self.shake.max(SHAKE_TICKS / 4);
        self.fell(c, killed);
    }

    /// A caller calls its batch while it lives (and once more, a larger
    /// one, when it is hurt enough); when it falls, what it called withdraws.
    fn calls_step(&mut self, c: &Content) {
        for i in 0..self.enemies.len() {
            let e = &self.enemies[i];
            let d = &c.enemies[e.def];
            if let Some(k) = e.summoned_by {
                if !self.enemies[k].alive() && e.alive() {
                    let at = e.pos;
                    self.enemies[i].hp = 0;
                    self.effect(FxKind::Flash, at, V3::ZERO, 18, c.pal.c("enemy_core"), int(2));
                    self.note("withdraw", String::new());
                }
                continue;
            }
            if d.summon.is_empty() || !e.hurtable() {
                continue;
            }
            let mine = self
                .enemies
                .iter()
                .filter(|o| o.alive() && o.summoned_by == Some(i))
                .count() as i32;
            let mut call: Vec<usize> = Vec::new();
            let e = &mut self.enemies[i];
            e.summon_timer += 1;
            if e.summon_timer >= d.summon_every && mine < d.summon_cap {
                e.summon_timer = 0;
                call.extend(d.summon.iter().take((d.summon_cap - mine).max(0) as usize));
            }
            if !e.rallied && d.rally_pct > 0 && e.health_pct() <= d.rally_pct {
                e.rallied = true;
                call.extend(d.rally.iter());
            }
            if call.is_empty() {
                continue;
            }
            self.note("call", c.say("callout_call"));
            for def in call {
                self.spawn(c, def);
                if let Some(n) = self.enemies.last_mut() {
                    n.summoned_by = Some(i);
                }
            }
        }
    }

    /// The Executor comes through the glass: each pane breaks a moment
    /// after the one nearer where it broke through, in falling shards.
    fn glass_step(&mut self, c: &Content) {
        if self.arena.panes.is_empty() {
            return;
        }
        let Some(b) = self.boss else {
            return;
        };
        let front = self.enemies[b].pos;
        let line = self.arena.panes[0].centre.z;
        let started = self.broken.iter().any(|t| *t != 0);
        if !started && front.z + tank::HULL_LEN / 2 > line {
            for (k, p) in self.arena.panes.iter().enumerate() {
                let far = p.centre.sub(v3(front.x, p.centre.y, line)).len() / GLASS_RIPPLE;
                self.broken[k] = self.tick + 1 + far;
            }
            self.shake = SHAKE_TICKS;
            self.note("crash", c.say("callout_crash"));
        }
        let glass = c.pal.c("glass");
        for k in 0..self.arena.panes.len() {
            if self.broken[k] != self.tick {
                continue;
            }
            let p = self.arena.panes[k];
            for n in 0..6 {
                let off = v3(
                    fx::mul(p.size.x, ONE * (n % 3 - 1) / 3),
                    fx::mul(p.size.y, ONE * (n / 3 * 2 - 1) / 4),
                    0,
                );
                let vel = v3(
                    int(self.rng.range(-3, 3)) / 60,
                    int(self.rng.range(0, 4)) / 60,
                    int(self.rng.range(4, 12)) / 60,
                );
                let life = self.rng.range(70, 130);
                self.effect(FxKind::Shard, p.centre.add(off), vel, life, glass, p.size.x / 3);
            }
        }
    }
}

/// Kinds that spawn in the air.
fn flies(k: Kind) -> bool {
    matches!(k, Kind::Floater | Kind::Moth | Kind::Blinker)
}

/// How far over the floor a ground kind's centre stands.
fn standing_lift(d: &crate::content::EnemyDef) -> i32 {
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

#[cfg(test)]
mod house {
    use super::*;
    use crate::content::tests::content;
    use crate::enemy::State;
    use crate::player::Build;
    use crate::route::Route;

    /// An arena of the house, its waves cleared so a test adds its own.
    fn house_world(kind: NodeKind, elite: bool) -> (Content, World) {
        let c = content();
        let r = Route::build_for(&c, 2, 11, 1);
        let i = r
            .nodes
            .iter()
            .position(|n| n.kind == kind && n.elite.is_some() == elite && (kind != NodeKind::Fight || n.archetype == "hall" || elite))
            .or_else(|| r.nodes.iter().position(|n| n.kind == kind && n.elite.is_some() == elite))
            .unwrap();
        let p = Player::new(V3::ZERO, 0, Build::default());
        let mut w = World::new(&c, &r.nodes[i], &[], p, Held::new(0, 1), 1);
        if kind == NodeKind::Fight && !elite {
            w.waves_left.clear();
        }
        (c, w)
    }

    fn put(c: &Content, w: &mut World, id: &str, at: V3) -> usize {
        let d = c.enemy(id).unwrap();
        let mut e = Enemy::new(d, &c.enemies[d], at.add(v3(0, c.enemies[d].hover, 0)), 1);
        e.state = State::Rest(1);
        w.enemies.push(e);
        w.enemies.len() - 1
    }

    fn still() -> Controls {
        Controls::default()
    }

    fn step(c: &Content, w: &mut World, ctl: &Controls) {
        let f = facing(w.player.yaw, w.player.pitch);
        let eye = w.player.chest().sub(f.scale(int(4)));
        w.tick(c, ctl, eye, f);
    }

    #[test]
    fn an_agent_takes_cover_out_of_your_sight_and_steps_out_to_fire() {
        let (c, mut w) = house_world(NodeKind::Fight, false);
        w.player.iframes = 1_000_000;
        let spot = w
            .arena
            .ground_spawns
            .iter()
            .copied()
            .find(|p| p.sub(w.player.pos).len_xz() > int(16))
            .unwrap();
        let i = put(&c, &mut w, "greyman", spot);
        let mut hidden_in_cover = false;
        let mut fired_seen = false;
        for _ in 0..60 * 20 {
            step(&c, &mut w, &still());
            let e = &w.enemies[i];
            let head = e.pos;
            let chest = w.player.chest();
            let to = head.sub(chest);
            let blocked = w.arena.level.ray(chest, to.norm(), to.len()).is_some();
            if e.order == crate::enemy::Order::Cover && e.path.is_empty() && matches!(e.state, State::Rest(_)) && blocked {
                hidden_in_cover = true;
            }
            if hidden_in_cover && matches!(e.state, State::Casting(0)) {
                fired_seen = !blocked;
                break;
            }
        }
        assert!(hidden_in_cover, "it reached cover you cannot see into");
        assert!(fired_seen, "it fires from where it can see you");
    }

    #[test]
    fn two_rifles_round_you_from_each_other_fire_on_the_same_tick() {
        let (c, mut w) = house_world(NodeKind::Fight, false);
        w.player.iframes = 1_000_000;
        let me = w.player.pos;
        let spawns: Vec<V3> = w.arena.ground_spawns.clone();
        let east = spawns.iter().copied().filter(|p| p.x > me.x + int(10)).min_by_key(|p| (p.z - me.z).abs()).unwrap();
        let west = spawns.iter().copied().filter(|p| p.x < me.x - int(10)).min_by_key(|p| (p.z - me.z).abs()).unwrap();
        let a = put(&c, &mut w, "greyman", east);
        let b = put(&c, &mut w, "greyman", west);
        let mut fired: Vec<(usize, i32, i32)> = Vec::new();
        for _ in 0..60 * 30 {
            step(&c, &mut w, &still());
            for i in [a, b] {
                if matches!(w.enemies[i].state, State::Casting(0)) {
                    let m = crate::squad::Mark {
                        feet: w.player.pos,
                        chest: w.player.chest(),
                        yaw: w.player.yaw,
                    };
                    fired.push((i, w.tick, crate::squad::bearing(&m, w.enemies[i].pos)));
                }
            }
            if fired.len() >= 6 {
                break;
            }
        }
        let together = fired.iter().any(|(i, t, ang)| {
            fired
                .iter()
                .any(|(j, u, bng)| i != j && t == u && fx::wrap(ang - bng).abs() >= CROSSFIRE_APART)
        });
        assert!(together, "a crossfire: {fired:?}");
    }

    #[test]
    fn an_outrider_goes_round_behind_where_you_face() {
        let (c, mut w) = house_world(NodeKind::Fight, false);
        w.player.iframes = 1_000_000;
        // you face north; it starts north of you, in front
        let me = w.player.pos;
        let ahead = w
            .arena
            .ground_spawns
            .iter()
            .copied()
            .filter(|p| p.z < me.z - int(14) && p.x.abs() < int(8))
            .min_by_key(|p| p.x.abs())
            .unwrap();
        let i = put(&c, &mut w, "outrider", ahead);
        let m = |w: &World| crate::squad::Mark {
            feet: w.player.pos,
            chest: w.player.chest(),
            yaw: w.player.yaw,
        };
        assert!(!crate::squad::behind(&m(&w), w.enemies[i].pos));
        let mut got_behind = false;
        for _ in 0..60 * 12 {
            step(&c, &mut w, &still());
            if crate::squad::behind(&m(&w), w.enemies[i].pos) && w.enemies[i].path.is_empty() {
                got_behind = true;
                break;
            }
        }
        assert!(got_behind, "it is at {:?}, you at {:?}", w.enemies[i].pos, w.player.pos);
    }

    #[test]
    fn a_warders_shield_turns_shots_from_in_front_and_not_from_behind() {
        let (c, mut w) = house_world(NodeKind::Fight, false);
        let at = w.player.pos.add(v3(0, 0, -int(8)));
        let i = put(&c, &mut w, "warder", at);
        let e = w.enemies[i].pos;
        // it faces you: south (+z) is yaw HALF
        w.enemies[i].yaw = fx::HALF;
        w.enemies[i].state = State::Rest(10_000);
        let shot = |from: V3| Shot {
            pos: from,
            prev: from,
            vel: e.sub(from).norm().scale(ONE),
            damage: 10,
            radius: ONE / 10,
            life: 100,
            splash_radius: 0,
            splash: 0,
            pierce: 0,
            passed: Vec::new(),
            rgb: [0; 3],
            kind: ShotKind::Bullet,
            erases: false,
            lodged: None,
            teeth: 0,
        };
        let hp = w.enemies[i].hp;
        w.shots.push(shot(e.add(v3(0, 0, int(4)))));
        for _ in 0..10 {
            w.enemies[i].yaw = fx::HALF;
            w.shots_step(&c);
        }
        assert_eq!(w.enemies[i].hp, hp, "the shield took it");
        w.shots.push(shot(e.add(v3(0, 0, -int(4)))));
        for _ in 0..10 {
            w.enemies[i].yaw = fx::HALF;
            w.shots_step(&c);
        }
        assert_eq!(w.enemies[i].hp, hp - 10, "from behind it lands");
    }

    #[test]
    fn a_marksmans_aim_locks_before_it_fires() {
        let (c, mut w) = house_world(NodeKind::Fight, false);
        let at = w.player.pos.add(v3(int(12), 0, -int(12)));
        let i = put(&c, &mut w, "marksman", at);
        let lock = c.enemies[w.enemies[i].def].phases[0].attacks[0].lock;
        w.enemies[i].state = State::Windup(lock + 2);
        w.enemies[i].path.clear();
        let level = w.arena.level.clone();
        let chest = w.player.chest();
        for _ in 0..3 {
            w.enemies[i].step(&c, &level, chest, i);
        }
        let locked = w.enemies[i].locked.expect("the aim locked");
        // you move a long way; its shot still goes where you were
        let moved = chest.add(v3(int(6), 0, 0));
        let mut first = None;
        for _ in 0..(lock + 4) {
            let d = w.enemies[i].step(&c, &level, moved, i);
            if let Some(o) = d.salvo.orbs.first() {
                first = Some(o.dir);
                break;
            }
        }
        let dir = first.expect("it fired");
        let want = locked.sub(w.enemies[i].muzzle(&c.enemies[w.enemies[i].def])).norm();
        assert!(dir.dot(want) > ONE * 99 / 100, "{dir:?} {want:?}");
    }

    fn tank_world() -> (Content, World, usize) {
        let (c, mut w) = house_world(NodeKind::Boss, false);
        w.player.iframes = 1_000_000;
        for _ in 0..80 {
            step(&c, &mut w, &still());
        }
        let b = w.boss.expect("the Executor came");
        (c, w, b)
    }

    #[test]
    fn the_executor_comes_through_the_glass() {
        let (c, mut w, b) = tank_world();
        assert!(w.broken.iter().all(|t| *t == 0), "whole while it is outside");
        for _ in 0..60 * 5 {
            step(&c, &mut w, &still());
        }
        assert!(w.enemies[b].pos.z > -int(24), "it drove in: {:?}", w.enemies[b].pos);
        let gone = w.broken.iter().filter(|t| **t != 0 && w.tick >= **t).count();
        assert_eq!(gone, w.arena.panes.len(), "every pane broke");
        assert!(w.notes.iter().any(|n| n.sound == "crash"));
    }

    #[test]
    fn the_executor_is_hurt_only_in_its_weak_points_and_each_phase_grows_cannons() {
        let (c, mut w, b) = tank_world();
        for _ in 0..60 * 5 {
            step(&c, &mut w, &still());
        }
        w.enemies[b].state = State::Rest(10_000);
        let d = &c.enemies[w.enemies[b].def];
        // a shot at the hull's middle from in front glances off
        let hull = w.enemies[b].pos;
        let from = hull.add(facing(w.enemies[b].tank.as_ref().unwrap().hull, 0).scale(int(10)));
        let weak_before: i32 = w.enemies[b].tank.as_ref().unwrap().weak.iter().sum();
        w.shots.push(Shot {
            pos: from,
            prev: from,
            vel: hull.sub(from).norm().scale(ONE),
            damage: 50,
            radius: ONE / 10,
            life: 100,
            splash_radius: 0,
            splash: 0,
            pierce: 0,
            passed: Vec::new(),
            rgb: [0; 3],
            kind: ShotKind::Bullet,
            erases: false,
            lodged: None,
            teeth: 0,
        });
        for _ in 0..20 {
            w.shots_step(&c);
        }
        let weak_after: i32 = w.enemies[b].tank.as_ref().unwrap().weak.iter().sum();
        assert_eq!(weak_before, weak_after, "the armour turned it");
        let guns = crate::tank::muzzles(&w.enemies[b], d).len();
        // break every weak point of the first phase
        for (at, _, k) in crate::tank::weak_points(&w.enemies[b], d) {
            w.hurt_weak(&c, b, k, 10_000, at);
        }
        assert_eq!(w.enemies[b].phase, 1);
        assert!(matches!(w.enemies[b].state, State::Changing(_)));
        assert!(crate::tank::muzzles(&w.enemies[b], d).len() > guns, "it grew cannons");
        assert_eq!(crate::tank::weak_points(&w.enemies[b], d).len(), d.phases[1].weak.len());
    }

    #[test]
    fn the_executors_last_phase_is_outlasted() {
        let (c, mut w, b) = tank_world();
        let d = c.enemies[w.enemies[b].def].clone();
        for k in 0..d.phases.len() - 1 {
            w.enemies[b].state = State::Rest(10);
            for (at, _, j) in crate::tank::weak_points(&w.enemies[b], &d) {
                w.hurt_weak(&c, b, j, 10_000, at);
            }
            assert_eq!(w.enemies[b].phase, k + 1);
        }
        let t = w.enemies[b].tank.as_ref().unwrap();
        assert_eq!(t.survive, d.phases.last().unwrap().survive);
        assert!(w.enemies[b].alive());
        // it cannot be hurt in it
        w.enemies[b].state = State::Rest(10);
        assert!(crate::tank::weak_points(&w.enemies[b], &d).is_empty());
        w.enemies[b].tank.as_mut().unwrap().survive = 30;
        for _ in 0..30 + WIN_TICKS + 5 {
            step(&c, &mut w, &still());
        }
        assert_eq!(w.outcome, Outcome::Won);
    }

    #[test]
    fn a_ground_pound_drops_straight_down_hurts_what_is_near_and_bounces_over_where_it_began() {
        let (c, mut w) = house_world(NodeKind::Fight, false);
        w.player.build.pound = 1;
        w.player.iframes = 0;
        let ground = w.player.pos;
        let i = put(&c, &mut w, "greyman", ground.add(v3(int(3), 0, 0)));
        w.enemies[i].state = State::Rest(10_000);
        let start = ground.y + int(4);
        w.player.pos = v3(ground.x, start, ground.z);
        w.player.prev_pos = w.player.pos;
        w.player.grounded = false;
        w.player.vel = v3(ONE / 10, 0, 0);
        let hp = w.enemies[i].hp;
        step(
            &c,
            &mut w,
            &Controls {
                pound_pressed: true,
                ..still()
            },
        );
        assert!(w.player.pounding);
        assert_eq!((w.player.vel.x, w.player.vel.z), (0, 0), "straight down");
        let mut landed = false;
        let mut peak = 0;
        for _ in 0..200 {
            w.enemies[i].state = State::Rest(10_000);
            step(&c, &mut w, &still());
            if !w.player.pounding {
                landed = true;
                peak = peak.max(w.player.pos.y);
            }
        }
        assert!(landed);
        assert_eq!(hp - w.enemies[i].hp, POUND_DAMAGE, "the shockwave");
        assert!(peak >= start + POUND_OVER - ONE / 5, "bounced to {peak}, began at {start}");
    }

    #[test]
    fn a_ripsaw_blade_lodges_and_revs_while_the_trigger_is_held() {
        let (c, mut w) = house_world(NodeKind::Fight, false);
        let saw = c.gun("ripsaw");
        w.held = Held::new(saw, 1);
        let target = w.player.chest().add(v3(0, 0, -int(8)));
        let d = c.enemy("warder").unwrap();
        let mut e = Enemy::new(d, &c.enemies[d], target, 1);
        // its back to you: the shield faces away
        e.yaw = 0;
        e.state = State::Rest(10_000);
        w.enemies.push(e);
        let aim = |w: &World| {
            let eye = w.player.chest().add(v3(0, 0, int(3)));
            (eye, w.enemies[0].pos.sub(eye).norm())
        };
        let hp = w.enemies[0].hp;
        let (eye, fwd) = aim(&w);
        w.tick(
            &c,
            &Controls {
                fire: true,
                fire_pressed: true,
                ..still()
            },
            eye,
            fwd,
        );
        for _ in 0..30 {
            w.enemies[0].state = State::Rest(10_000);
            let (eye, fwd) = aim(&w);
            w.tick(&c, &still(), eye, fwd);
        }
        let after_hit = w.enemies[0].hp;
        assert!(after_hit < hp, "the throw hit");
        assert!(w.shots.iter().any(|s| s.kind == ShotKind::Saw && s.lodged.is_some()), "it lodged");
        for _ in 0..30 {
            w.enemies[0].state = State::Rest(10_000);
            let (eye, fwd) = aim(&w);
            w.tick(&c, &still(), eye, fwd);
        }
        assert_eq!(w.enemies[0].hp, after_hit, "no rev without the trigger");
        for _ in 0..60 {
            w.enemies[0].state = State::Rest(10_000);
            let (eye, fwd) = aim(&w);
            w.tick(&c, &Controls { fire: true, ..still() }, eye, fwd);
        }
        assert!(w.enemies[0].hp < after_hit - 40, "the rev bites: {} to {}", after_hit, w.enemies[0].hp);
    }

    #[test]
    fn the_section_chief_calls_agents_up_to_his_cap_and_they_withdraw_when_he_falls() {
        let (c, mut w) = house_world(NodeKind::Fight, true);
        w.player.iframes = 1_000_000;
        for _ in 0..80 {
            step(&c, &mut w, &still());
        }
        let k = w.elite.expect("the chief came");
        let d = c.enemies[w.enemies[k].def].clone();
        for _ in 0..(d.summon_every * 4) {
            w.player.iframes = 1_000_000;
            step(&c, &mut w, &still());
        }
        let called = w.enemies.iter().filter(|e| e.alive() && e.summoned_by == Some(k)).count() as i32;
        assert!(called > 0 && called <= d.summon_cap, "{called}");
        let earned = w.earned;
        w.enemies[k].hp = 1;
        w.enemies[k].state = State::Rest(100);
        w.enemies[k].hurt(5);
        step(&c, &mut w, &still());
        step(&c, &mut w, &still());
        assert!(w.enemies.iter().all(|e| e.summoned_by != Some(k) || !e.alive()), "his men withdrew");
        let _ = earned;
    }
}
