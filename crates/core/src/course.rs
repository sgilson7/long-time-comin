//! A course: an arena that is crossed, not cleared (the Water Gap's
//! sections). It has no waves; its way on stands at the far end, open from
//! the start. Under it runs water that drowns: you come back at the last
//! mooring post you reached, a share of your health the poorer. On the way:
//! levers you hook, plates you pound and lamps you shoot, each opening
//! doors for a few seconds or silencing emitters; roots that rise and sink;
//! rotten floors a ground pound breaks; drum-caps that double its bounce;
//! emitters set in the rock that fire their patterns while you are near.
//!
//! The Bench is here too, though it is yours in every zone: a plank laid
//! under you in the air, or where you aim on the ground, that you stand on
//! and shoot through.

use crate::balance::*;
use crate::content::Content;
use crate::fx::{self, int, ONE};
use crate::geom::{v3, V3};
use crate::level::{Dyn, Solid};
use crate::player::{bounce_speed, Event};
use crate::world::{FxKind, World};

mod waves;
pub use waves::{Rogue, Wash, Waves};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchKind {
    /// hook its ring: the pull throws it
    Lever,
    /// land a ground pound on it
    Plate,
    /// shoot it out: the emitter by this index is the lamp
    Lamp(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switch {
    pub kind: SwitchKind,
    /// where it is: the box a hook or a pound must meet
    pub at: Solid,
    /// doors it opens, by index, and emitters it silences
    pub opens: Vec<usize>,
    pub silences: Vec<usize>,
    /// how long, in ticks
    pub ticks: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Door {
    /// its solid among the level's changing ones
    pub dynamic: usize,
    /// ticks it stays open yet (0: shut), and of how many
    pub left: i32,
    pub total: i32,
}

/// Ground that moves: a root that rises and sinks with the tree's
/// breathing, a deck that heaves and sways on the swell, wreckage turning
/// round a whirlpool's eye. Whatever stands on it goes with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mover {
    pub dynamic: usize,
    /// where it stands at its lowest
    pub base: Solid,
    pub rise: i32,
    pub period: i32,
    /// where in its breath it starts, a share of a turn
    pub phase: i32,
    /// how far it sways each way along the level, on the same breath
    pub swing: V3,
    /// turns a tick it goes round `round` (0: it does not)
    pub orbit: i32,
    pub round: V3,
    /// how it is drawn
    pub look: Look,
}

/// How moving ground is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Look {
    /// a root of the Oak
    #[default]
    Root,
    /// a ship's deck, its hull under it and its rail round it
    Deck,
    /// a mast, its yards and furled sails, a crow's nest at the top
    Mast,
    /// wreckage: planks, a broken hull, a crate
    Wreck,
}

impl Mover {
    /// One that rises `rise` and sinks again every `period` ticks.
    pub fn breathing(dynamic: usize, base: Solid, rise: i32, period: i32, phase: i32) -> Mover {
        Mover {
            dynamic,
            base,
            rise,
            period,
            phase,
            swing: V3::ZERO,
            orbit: 0,
            round: V3::ZERO,
            look: Look::Root,
        }
    }

    /// Where its lowest corner is at tick `tick`.
    pub fn corner(&self, tick: i32) -> V3 {
        let t = (fx::TURN as i64 * tick as i64 / self.period.max(1) as i64) as i32 + self.phase;
        let up = self.rise / 2 - fx::mul(self.rise / 2, fx::cos(t));
        let sway = self.swing.scale(fx::sin(t));
        let mut at = self.base.min.add(v3(sway.x, up, sway.z));
        if self.orbit != 0 {
            let c = self.base.centre();
            let r = v3(c.x - self.round.x, 0, c.z - self.round.z);
            let turned = crate::geom::rotate_y(r, self.orbit.wrapping_mul(tick));
            at = at.add(v3(turned.x - r.x, 0, turned.z - r.z));
        }
        at
    }
}

/// Something set in the rock that fires: an enemy that never moves, woken
/// only while you are near.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emitter {
    pub pos: V3,
    /// the enemy it is, by id
    pub enemy: String,
}

/// A ship of a fought fleet and her colours: struck once her crew is down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flag {
    /// her deck and the mast the colours fly from, by mover index
    pub deck: usize,
    pub mast: usize,
    /// her crew, by emitter index
    pub crew: Vec<usize>,
    pub struck: bool,
}

/// The Captured Dragon's chain: while any of its locks holds, the dragon
/// flies only round the flagship's masts and cannot be hurt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    /// the dragon and the locks, by emitter index
    pub dragon: usize,
    pub locks: Vec<usize>,
    /// where the chain is made fast (the mainmast's foot), and the mast
    /// tops it may perch on while chained
    pub mast: V3,
    pub perches: Vec<V3>,
    pub free: bool,
}

/// One step of a course's intended line, as its builder lays it: the line
/// a player is meant to take, and the one the tests drive a body along to
/// prove the course can be crossed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// run to a point
    To(V3),
    /// jump toward a point, and come down
    Jump(V3),
    /// Bound Over toward a point (a dash and a jump together), and come down
    Bound(V3),
    /// hook a point and ride the hook until it lets go
    Hook(V3),
    /// hook a point and let go at once: a lever's ring, thrown
    Tug(V3),
    /// lay a Bench (in the air, under you)
    Bench,
    /// a ground pound, down to whatever is below
    Pound,
    /// stand still this many ticks (a door, a root, a curtain)
    Wait(i32),
    /// in the air, steer toward a point until you land
    Air(V3),
    /// a jump with no run, straight up from where you stand
    Hop,
}

/// A zone of wind that shoves you along `dir` for `on` ticks of every
/// `period` (the Hurricane); its rain slants for `WARN` ticks before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gust {
    pub zone: Solid,
    pub dir: V3,
    pub push: i32,
    pub period: i32,
    pub on: i32,
    pub phase: i32,
}

impl Gust {
    /// Ticks until it blows (0: blowing now).
    pub fn until(&self, tick: i32) -> i32 {
        let t = (tick + self.phase).rem_euclid(self.period.max(1));
        (self.period - self.on - t).max(0)
    }
}

/// A storm over the course: every `every` ticks lightning picks a place
/// near you, marks it, and strikes it `warn` ticks later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Storm {
    pub every: i32,
    pub warn: i32,
    pub radius: i32,
    pub damage: i32,
}

/// Under the water (the Maelstrom): gravity this share of the air's, a
/// stroke on the jump key, and a current pulling toward `eye`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Underwater {
    pub gravity_pct: i32,
    pub eye: V3,
    pub pull: i32,
}

/// The Eraser's charge (the Vault): pylons held in turn, each `need`
/// ticks within `radius` of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Charge {
    pub pylons: Vec<V3>,
    pub radius: i32,
    pub need: i32,
    /// ticks each has been held so far
    pub held: Vec<i32>,
}

impl Charge {
    /// The charge so far, Q16 of whole.
    pub fn level(&self) -> i32 {
        let total = self.need * self.pylons.len() as i32;
        fx::div(self.held.iter().sum::<i32>(), total.max(1))
    }

    /// The pylon being charged now, if any is left.
    pub fn next(&self) -> Option<usize> {
        (0..self.pylons.len()).find(|&k| self.held.get(k).copied().unwrap_or(0) < self.need)
    }
}

/// The Gallows Oak's place in its arena: its trunk's axis and radius, and
/// its crown's height and reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OakSite {
    pub axis: V3,
    pub trunk: i32,
    pub crown_y: i32,
    pub crown_r: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Course {
    /// the water's surface: below it you drown
    pub water: i32,
    /// in the open sea, how far under the surface you may sink before it
    /// takes you (0: at once, as the river does); there you swim
    pub depth: i32,
    /// ticks spent in the sea since you were last out of it
    pub wet: i32,
    /// mooring posts, in the order they are reached; the first is the start
    pub posts: Vec<V3>,
    pub doors: Vec<Door>,
    pub switches: Vec<Switch>,
    pub movers: Vec<Mover>,
    /// rotten floors, among the level's changing solids
    pub rotten: Vec<usize>,
    /// drum-caps: a pound landing on one bounces twice as high
    pub drums: Vec<Solid>,
    pub emitters: Vec<Emitter>,
    /// where a strand's reward waits, off the main line
    pub strands: Vec<V3>,
    /// the way on, at the far end
    pub finish: V3,
    pub finish_yaw: i32,
    /// the intended line, from the first post to the finish
    pub line: Vec<Step>,
    /// fought: its way on opens only when its emitters (here, crews) and
    /// waves are all down
    pub fought: bool,
    /// where its waves come in (near you, as an arena's do)
    pub spawns: Vec<V3>,
    pub gusts: Vec<Gust>,
    pub storm: Option<Storm>,
    /// lightning marked and coming: where, and ticks left
    pub strikes: Vec<(V3, i32)>,
    pub underwater: Option<Underwater>,
    /// where the boss comes up (Dagon, in the eye)
    pub boss_at: Option<V3>,
    pub charge: Option<Charge>,
    /// a fleet's ships and their colours
    pub flags: Vec<Flag>,
    pub chain: Option<Chain>,
    /// the Gallows Oak's arena: where the tree stands
    pub oak: Option<OakSite>,
    /// the sea's rogue waves and green washes (the Hurricane)
    pub waves: Waves,
    /// the highest the climb has reached (the Oak's first bar)
    pub climbed: i32,
    /// the Summons, by enemy index, once the crown is reached
    pub summons: Option<usize>,
    // ---- in play
    /// the furthest post reached
    pub post: usize,
    /// ticks each emitter stays silent yet
    pub silent: Vec<i32>,
    /// each emitter's enemy, by index into the world's enemies
    pub foes: Vec<usize>,
    /// switches thrown, so a lamp shot out throws once
    pub thrown: Vec<bool>,
}

impl Course {
    /// Is emitter enemy `i` asleep: too far from you, or silenced?
    pub fn asleep(&self, i: usize, at: V3) -> bool {
        match self.foes.iter().position(|&f| f == i) {
            Some(k) => {
                self.silent.get(k).is_some_and(|&t| t > 0) || self.emitters[k].pos.dist(at) > EMITTER_WAKE
            }
            None => false,
        }
    }
}

impl World {
    /// The course's emitters into the world, as enemies, and its strands'
    /// rewards where they wait. Called once the arena is in place.
    pub(crate) fn course_begin(&mut self, c: &Content) {
        let Some(k) = self.arena.course.clone() else {
            return;
        };
        // a course is crossed, not cleared; a fought one, and a boss's, is
        // cleared as an arena is
        self.cleared = !k.fought && self.kind != crate::route::NodeKind::Boss;
        let mut foes = Vec::new();
        for e in &k.emitters {
            let def = c.enemy(&e.enemy).unwrap_or(0);
            let d = &c.enemies[def];
            let mut foe = crate::enemy::Enemy::new(def, d, e.pos, 1);
            foe.home = e.pos;
            foes.push(self.enemies.len());
            self.enemies.push(foe);
        }
        let mut course = k;
        course.silent = vec![0; course.emitters.len()];
        course.thrown = vec![false; course.switches.len()];
        course.foes = foes;
        if let (Some(r), Some(&at)) = (self.reward, course.strands.first()) {
            self.pickups.push(crate::world::Pickup {
                pos: at,
                kind: crate::world::reward_pickup(r, self.held.level),
                taken: false,
                age: 0,
                group: 0,
            });
            self.reward = None;
        }
        self.course = Some(course);
    }

    /// One tick of the course, after the body has moved: the Bench, the
    /// switches, the doors, the roots, the posts and the water.
    pub(crate) fn course_step(&mut self, c: &Content, events: &[Event]) {
        self.benches_step(c, events);
        let Some(mut k) = self.course.take() else {
            return;
        };
        let feet = self.player.pos;
        let mut said: Vec<(&'static str, String)> = Vec::new();
        let mut burst: Option<V3> = None;
        let level = &mut self.arena.level;

        // what was thrown this tick
        let mut thrown = Vec::new();
        for (s, sw) in k.switches.iter().enumerate() {
            let hit = match sw.kind {
                SwitchKind::Lever => {
                    // where the hook struck, before it is lifted over a ledge
                    self.hooked.is_some_and(|t| near_box(&sw.at, t, ONE / 2))
                }
                SwitchKind::Plate => events.contains(&Event::Pound) && on_top(&sw.at, feet),
                SwitchKind::Lamp(e) => {
                    !k.thrown[s] && k.foes.get(e).is_some_and(|&f| !self.enemies[f].alive())
                }
            };
            if hit {
                thrown.push(s);
            }
        }
        for s in thrown {
            k.thrown[s] = true;
            let sw = k.switches[s].clone();
            for &d in &sw.opens {
                let door = &mut k.doors[d];
                door.left = sw.ticks;
                door.total = sw.ticks;
                level.dynamic[door.dynamic].on = false;
            }
            for &e in &sw.silences {
                k.silent[e] = sw.ticks;
            }
            said.push(("switch", c.say("callout_switch")));
        }

        // doors shut when their time is up, but never on you
        let (bmin, bmax) = body(feet);
        for door in k.doors.iter_mut() {
            if door.left > 0 {
                door.left -= 1;
                let d = &mut level.dynamic[door.dynamic];
                if door.left == 0 {
                    if d.solid.overlaps(bmin, bmax) {
                        door.left = 1;
                    } else {
                        d.on = true;
                    }
                }
            }
        }
        for t in k.silent.iter_mut() {
            *t = (*t - 1).max(0);
        }

        // moving ground, carrying whoever stands on it: you, and enemies
        for (i, m) in k.movers.iter().enumerate() {
            let d = &mut level.dynamic[m.dynamic];
            let delta = k.waves.corner(i, m, self.tick).sub(d.solid.min);
            // what stands on it goes with it, down as well as up, and so
            // keeps its footing (a fall would leave it hanging each tick)
            let riding = self.player.grounded
                && (feet.y - d.solid.max.y).abs() <= ONE / 20
                && feet.x >= d.solid.min.x - PLAYER_RADIUS
                && feet.x <= d.solid.max.x + PLAYER_RADIUS
                && feet.z >= d.solid.min.z - PLAYER_RADIUS
                && feet.z <= d.solid.max.z + PLAYER_RADIUS;
            let top = d.solid.max.y;
            let (lo, hi) = (d.solid.min, d.solid.max);
            d.solid.min = d.solid.min.add(delta);
            d.solid.max = d.solid.max.add(delta);
            if riding {
                self.player.pos = self.player.pos.add(delta);
            } else if delta.y > 0 {
                // rising into a body coming down on it: the body lands on
                // its top, not thrown out of its side
                let b = &self.player;
                let (bmin, bmax) = body(b.pos);
                let over = d.solid.min.x < bmax.x
                    && d.solid.max.x > bmin.x
                    && d.solid.min.z < bmax.z
                    && d.solid.max.z > bmin.z;
                if over && b.pos.y >= top - ONE / 4 && b.pos.y < d.solid.max.y {
                    self.player.pos.y = d.solid.max.y;
                    self.player.vel.y = 0;
                    self.player.grounded = true;
                }
            }
            if delta != V3::ZERO {
                for e in self.enemies.iter_mut().filter(|e| e.alive()) {
                    let p = e.pos;
                    if p.x >= lo.x
                        && p.x <= hi.x
                        && p.z >= lo.z
                        && p.z <= hi.z
                        && p.y >= top
                        && p.y <= top + ENEMY_RIDE
                    {
                        e.pos = e.pos.add(delta);
                        e.prev = e.prev.add(delta);
                        e.home = e.home.add(delta);
                    }
                }
            }
        }

        // a pound: through a rotten floor, or off a drum-cap twice as high
        if events.contains(&Event::Pound) {
            let p = self.player.pos;
            if let Some(&r) = k.rotten.iter().find(|&&r| {
                let s = level.dynamic[r].solid;
                level.dynamic[r].on && on_top(&s, p)
            }) {
                level.dynamic[r].on = false;
                // on down, the drop still a pound
                self.player.vel.y = -POUND_SPEED;
                self.player.pounding = true;
                self.player.pound_cooldown = 0;
                burst = Some(level.dynamic[r].solid.centre());
                said.push(("timber", String::new()));
            } else if k.drums.iter().any(|s| on_top(s, p)) {
                let h = (self.player.pound_from - p.y + POUND_OVER).max(POUND_MIN_BOUNCE);
                self.player.vel.y = bounce_speed(h * 2);
                said.push(("drum_cap", String::new()));
            }
        }

        // a mooring post reached is where the water returns you
        let p = self.player.pos;
        if let Some(n) = k
            .posts
            .iter()
            .enumerate()
            .skip(k.post + 1)
            .find(|(_, q)| q.sub(p).len_xz() < POST_REACH && (q.y - p.y).abs() < int(3))
            .map(|(n, _)| n)
        {
            k.post = n;
            said.push(("post", c.say("callout_post")));
        }

        // a ship whose crew is all down strikes her colours
        let fleet = k.flags.len();
        for n in 0..fleet {
            let down = |i: &usize| k.foes.get(*i).is_none_or(|&f| !self.enemies[f].alive());
            if !k.flags[n].struck && k.flags[n].crew.iter().all(down) {
                k.flags[n].struck = true;
                let struck = k.flags.iter().filter(|f| f.struck).count();
                let words = c
                    .say("callout_struck")
                    .replace("{n}", &struck.to_string())
                    .replace("{of}", &fleet.to_string());
                said.push(("struck", words));
            }
        }
        // the dragon on its chain: kept to the flagship's masts and never
        // hurt until its three locks are shot off
        if let Some(ch) = k.chain.as_mut().filter(|ch| !ch.free) {
            let foe = |i: usize| k.foes.get(i).copied();
            let held = ch
                .locks
                .iter()
                .any(|&l| foe(l).is_some_and(|f| self.enemies[f].alive()));
            if let Some(e) = foe(ch.dragon).map(|f| &mut self.enemies[f]).filter(|e| e.alive()) {
                if held {
                    e.shielded = true;
                    e.leash = Some((ch.mast, CHAIN_LEASH));
                    if e.perches != ch.perches {
                        e.perches = ch.perches.clone();
                        e.perch = 0;
                    }
                } else {
                    ch.free = true;
                    e.shielded = false;
                    e.leash = None;
                    e.perches.clear();
                    e.phase = 1;
                    e.attack = 0;
                    said.push(("roar", c.say("callout_dragon_free")));
                }
            }
        }

        // a fought course is cleared when its crews and its waves are down;
        // whoever goes over the side drowns
        for e in self.enemies.iter_mut().filter(|e| e.alive()) {
            if e.pos.y < k.water - ENEMY_DROWN {
                e.hp = 0;
            }
        }
        if k.fought && !self.cleared && self.waves_left.is_empty() && self.enemies.iter().all(|e| !e.alive())
        {
            self.cleared = true;
            self.cleared_at = self.tick;
            said.push(("cleared", c.say("callout_course_cleared")));
        }

        // the wind and the current shove you on top of how you move
        let mut shove = V3::ZERO;
        // the wind: a gust shoves you while it blows
        for g in &k.gusts {
            let inside =
                p.x >= g.zone.min.x && p.x <= g.zone.max.x && p.z >= g.zone.min.z && p.z <= g.zone.max.z;
            if inside && g.until(self.tick) == 0 && !self.player.dashing() {
                let push = g
                    .dir
                    .scale(if self.player.grounded { g.push / 2 } else { g.push });
                shove = shove.add(v3(push.x, 0, push.z));
            }
        }
        // lightning: marked, then struck
        if let Some(st) = k.storm {
            if self.tick % st.every.max(1) == 0 {
                let off = v3(int(self.rng.range(-6, 6)), 0, int(self.rng.range(-6, 6)));
                let at = p.add(off);
                let ground = level.floor_below(at.x, p.y + int(3), at.z).unwrap_or(p.y);
                k.strikes.push((v3(at.x, ground, at.z), st.warn));
            }
            for s in k.strikes.iter_mut() {
                s.1 -= 1;
            }
            for &(at, left) in k.strikes.iter().filter(|s| s.1 == 0) {
                said.push(("thunder", String::new()));
                if v3(p.x - at.x, 0, p.z - at.z).len_xz() <= st.radius && (p.y - at.y).abs() < int(3) {
                    self.player.hurt(st.damage);
                }
                let _ = left;
            }
            k.strikes.retain(|s| s.1 > 0);
        }
        // under the water: lighter, a stroke to swim, a current to the eye
        let in_sea = k.depth > 0 && p.y < k.water;
        k.wet = if in_sea { k.wet + 1 } else { 0 };
        self.player.swimming = k.underwater.is_some() || in_sea;
        self.player.afloat = in_sea;
        if let Some(u) = k.underwater {
            if !self.player.grounded {
                shove = shove.add(v3(u.eye.x - p.x, 0, u.eye.z - p.z).norm().scale(u.pull));
            }
        }
        self.player.shove = shove;
        self.sea_step(c, &mut k, &mut said);
        // the Eraser: held pylon by pylon; the boss's bar is what is left
        if let Some(ch) = k.charge.as_mut() {
            if let Some(n) = ch.next() {
                if p.sub(ch.pylons[n]).len_xz() <= ch.radius {
                    ch.held[n] += 1;
                    if ch.held[n] == ch.need {
                        said.push(("switch", c.say("callout_pylon")));
                    }
                }
            }
            let level_now = ch.level();
            if let Some(b) = self.boss {
                // your older self cannot be hurt here: only the Eraser ends it
                let e = &mut self.enemies[b];
                e.shielded = true;
                if e.alive() {
                    e.hp = if ch.next().is_none() {
                        0
                    } else {
                        (e.max_hp - fx::mul(e.max_hp, level_now)).max(1)
                    };
                }
            }
        }

        // the water: at once in the river; in the sea, too deep or too long
        let drowned = if k.depth > 0 {
            p.y < k.water - k.depth || k.wet > SEA_BREATH
        } else {
            p.y < k.water
        };
        if drowned {
            k.wet = 0;
            let back = k.posts.get(k.post).copied().unwrap_or(self.arena.start);
            let back = back.add(v3(0, k.waves.lift_under(&k.movers, back, self.tick), 0));
            let pl = &mut self.player;
            pl.release_grapple();
            pl.pounding = false;
            pl.dash_ticks = 0;
            pl.pos = back.add(v3(0, ONE / 2, 0));
            pl.prev_pos = pl.pos;
            pl.vel = V3::ZERO;
            pl.last_ground = back;
            pl.iframes = 0;
            let cost = (pl.build.max_health * DROWN_PCT / 100).max(1);
            pl.hurt(cost);
            pl.iframes = HURT_IFRAMES;
            said.push(("drown", c.say("callout_drown")));
        }
        self.course = Some(k);
        for (sound, text) in said {
            self.note(sound, text);
        }
        if let Some(at) = burst {
            let rgb = c.pal.c("river_wood");
            self.sparks_at(at, rgb, 24);
        }
    }

    /// The Gallows Oak: its eye opens on the trunk just above you while you
    /// climb, and its bar is the climb; at the crown it calls the Summons
    /// and its bar follows the Summons' health until the Summons falls.
    pub(crate) fn oak_step(&mut self, c: &Content) {
        let (Some(b), Some(site)) = (self.boss, self.course.as_ref().and_then(|k| k.oak)) else {
            return;
        };
        if c.enemies[self.enemies[b].def].kind != crate::content::Kind::Oak || !self.enemies[b].alive() {
            return;
        }
        let p = self.player.pos;
        let summons = self.course.as_ref().and_then(|k| k.summons);
        let max = self.enemies[b].max_hp;
        let climb_line = max * OAK_CLIMB_BAR_PCT / 100;
        match summons {
            None => {
                // the eye: on the trunk toward you, a little above you
                let toward = v3(p.x - site.axis.x, 0, p.z - site.axis.z).norm();
                let y = (p.y + OAK_EYE_OVER).min(site.crown_y - int(2));
                let eye = v3(site.axis.x, y, site.axis.z).add(toward.scale(site.trunk + ONE));
                let o = &mut self.enemies[b];
                o.prev = o.pos;
                o.pos = eye;
                o.home = eye;
                let k = self.course.as_mut().unwrap();
                k.climbed = k.climbed.max(p.y - site.axis.y);
                let span = (site.crown_y - site.axis.y).max(ONE);
                let left = span - k.climbed.min(span);
                // the bar falls from full to the climb's line as you rise
                o.hp = climb_line + ((max - climb_line) as i64 * left as i64 / span as i64) as i32;
                let on_crown = p.y >= site.crown_y - ONE
                    && v3(p.x - site.axis.x, 0, p.z - site.axis.z).len_xz() <= site.crown_r;
                if on_crown && self.player.grounded {
                    o.hp = climb_line;
                    let def = c.enemy(OAK_SUMMONS).unwrap_or(0);
                    let at = v3(site.axis.x, site.crown_y, site.axis.z - site.crown_r / 2);
                    let mut s = crate::enemy::Enemy::new(def, &c.enemies[def], at.add(v3(0, int(6), 0)), 1);
                    s.home = v3(site.axis.x, site.crown_y, site.axis.z);
                    let i = self.enemies.len();
                    self.enemies.push(s);
                    self.elite = Some(i);
                    self.course.as_mut().unwrap().summons = Some(i);
                    // the eye closes; the tree sits at its crown's heart
                    self.enemies[b].pos = v3(site.axis.x, site.crown_y - int(4), site.axis.z);
                    self.shake = SHAKE_TICKS * 2;
                    self.note("eldritch", c.say("callout_summons"));
                }
            }
            Some(s) => {
                let sm = &self.enemies[s];
                let left = if sm.alive() { sm.hp.max(1) } else { 0 };
                let hp = (climb_line as i64 * left as i64 / sm.max_hp.max(1) as i64) as i32;
                self.enemies[b].hp = if left > 0 { hp.max(1) } else { 0 };
            }
        }
    }

    /// The Bench: laid on the key, standing its time, the oldest taken up
    /// when one more would stand than the build allows.
    fn benches_step(&mut self, c: &Content, events: &[Event]) {
        for b in self.benches.iter_mut() {
            if b.1 > 0 {
                b.1 -= 1;
                if b.1 == 0 {
                    self.arena.level.dynamic[b.0].on = false;
                }
            }
        }
        if !events.contains(&Event::Bench) {
            return;
        }
        let (most, ticks) = self.player.build.benches();
        if most == 0 {
            return;
        }
        let p = self.player.pos;
        // under you in the air; where you aim, not past a wall, on the ground
        let at = if self.player.grounded {
            let chest = self.player.chest();
            let to = self.aim_point.sub(chest);
            let reach = (to.len() - ONE).clamp(int(2), BENCH_REACH);
            let at = chest.add(to.norm().scale(reach));
            v3(at.x, at.y - ONE, at.z)
        } else {
            p
        };
        let half = BENCH_SIZE / 2;
        let solid = Solid::new(
            v3(at.x - half, at.y - BENCH_THICK, at.z - half),
            v3(at.x + half, at.y, at.z + half),
        );
        let standing: Vec<usize> = (0..self.benches.len())
            .filter(|&k| self.benches[k].1 > 0)
            .collect();
        if standing.len() >= most {
            // the oldest goes
            if let Some(&k) = standing.iter().min_by_key(|&&k| self.benches[k].1) {
                self.arena.level.dynamic[self.benches[k].0].on = false;
                self.benches[k].1 = 0;
            }
        }
        let d = Dyn {
            solid,
            on: true,
            one_way: true,
        };
        match self.benches.iter().position(|b| b.1 == 0) {
            Some(k) => {
                self.arena.level.dynamic[self.benches[k].0] = d;
                self.benches[k].1 = ticks;
            }
            None => {
                let i = self.arena.level.add_dynamic(d);
                self.benches.push((i, ticks));
            }
        }
        let rgb = c.pal.c("bench");
        self.sparks_at(v3(at.x, at.y, at.z), rgb, 10);
    }

    fn sparks_at(&mut self, at: V3, rgb: [i32; 3], n: i32) {
        for k in 0..n {
            let a = fx::TURN / n.max(1) * k;
            let v = v3(
                fx::mul(fx::cos(a), ONE / 6),
                ONE / 6,
                fx::mul(fx::sin(a), ONE / 6),
            );
            self.effects.push(crate::world::Effect {
                kind: FxKind::Spark,
                pos: at,
                vel: v,
                age: 0,
                life: 30,
                rgb,
                size: ONE / 5,
            });
        }
    }
}

/// The body's box, standing at `feet`.
fn body(feet: V3) -> (V3, V3) {
    (
        v3(feet.x - PLAYER_RADIUS, feet.y, feet.z - PLAYER_RADIUS),
        v3(
            feet.x + PLAYER_RADIUS,
            feet.y + PLAYER_HEIGHT,
            feet.z + PLAYER_RADIUS,
        ),
    )
}

/// Is `p` within `slack` of the box?
fn near_box(s: &Solid, p: V3, slack: i32) -> bool {
    p.x >= s.min.x - slack
        && p.x <= s.max.x + slack
        && p.y >= s.min.y - slack
        && p.y <= s.max.y + slack
        && p.z >= s.min.z - slack
        && p.z <= s.max.z + slack
}

/// Are feet at `p` standing on the box's top?
fn on_top(s: &Solid, p: V3) -> bool {
    p.x >= s.min.x - PLAYER_RADIUS
        && p.x <= s.max.x + PLAYER_RADIUS
        && p.z >= s.min.z - PLAYER_RADIUS
        && p.z <= s.max.z + PLAYER_RADIUS
        && (p.y - s.max.y).abs() <= ONE / 4
}

/// A driver for a course's line: each tick, the controls a body following
/// it would give, where it would face and where it would aim. The tests
/// use it to prove a course can be crossed; the playtest harness too.
#[derive(Debug, Clone, Default)]
pub struct Pilot {
    /// the step it is on, and ticks into it
    pub step: usize,
    pub t: i32,
    /// ticks it has gone without finishing a step, all told
    pub stuck: i32,
    /// the hook of this step has caught (it presses again until it does,
    /// as a player would while the hook's cooldown runs)
    pub caught: bool,
}

/// What the pilot asks for on one tick.
#[derive(Debug, Clone, Copy)]
pub struct Drive {
    pub controls: crate::player::Controls,
    pub yaw: i32,
    pub aim: V3,
}

impl Pilot {
    /// The next tick's drive along `line`, or nothing once the line is done.
    pub fn drive(&mut self, line: &[Step], p: &crate::player::Player) -> Option<Drive> {
        use crate::geom::{facing, yaw_pitch_of};
        use crate::player::Controls;
        let step = *line.get(self.step)?;
        let yaw_to = |q: V3| {
            let d = v3(q.x - p.pos.x, 0, q.z - p.pos.z);
            if d.len_xz() < ONE / 20 {
                p.yaw
            } else {
                yaw_pitch_of(d).0
            }
        };
        let fwd = Controls {
            move_z: ONE,
            ..Controls::default()
        };
        let t = self.t;
        let landed = p.grounded && t > 3;
        let (controls, yaw, aim, done) = match step {
            Step::To(q) => {
                let near = v3(q.x - p.pos.x, 0, q.z - p.pos.z).len_xz() < ONE / 2;
                (fwd, yaw_to(q), V3::ZERO, near && p.grounded)
            }
            Step::Jump(q) => (
                Controls {
                    jump_pressed: t == 0,
                    ..fwd
                },
                yaw_to(q),
                V3::ZERO,
                landed,
            ),
            Step::Bound(q) => (
                Controls {
                    jump_pressed: t == 0,
                    dash_pressed: t == 0,
                    ..fwd
                },
                yaw_to(q),
                V3::ZERO,
                landed,
            ),
            Step::Hop => (
                Controls {
                    jump_pressed: t == 0,
                    ..Controls::default()
                },
                p.yaw,
                V3::ZERO,
                landed,
            ),
            Step::Hook(a) => {
                let hooked = p.grapple.is_some();
                self.caught |= hooked;
                (
                    Controls {
                        grapple_pressed: !self.caught,
                        grapple_held: true,
                        ..fwd
                    },
                    yaw_to(a),
                    a,
                    self.caught && !hooked,
                )
            }
            Step::Tug(a) => (
                Controls {
                    grapple_pressed: t == 0,
                    grapple_held: t == 0,
                    ..Controls::default()
                },
                yaw_to(a),
                a,
                t >= 2,
            ),
            Step::Bench => (
                Controls {
                    bench_pressed: t == 0,
                    ..Controls::default()
                },
                p.yaw,
                V3::ZERO,
                t >= 1,
            ),
            Step::Pound => (
                Controls {
                    pound_pressed: t == 0,
                    ..Controls::default()
                },
                p.yaw,
                V3::ZERO,
                t > 1 && !p.pounding,
            ),
            Step::Wait(n) => (Controls::default(), p.yaw, V3::ZERO, t >= n),
            Step::Air(q) => (fwd, yaw_to(q), V3::ZERO, landed),
        };
        let aim = if aim == V3::ZERO {
            facing(yaw, 0)
        } else {
            aim.sub(p.chest()).norm()
        };
        if done {
            self.step += 1;
            self.t = 0;
            self.caught = false;
            return self.drive(line, p);
        }
        self.t += 1;
        self.stuck += 1;
        Some(Drive { controls, yaw, aim })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::content::tests::content;
    use crate::player::{Build, Player};
    use crate::route::{Have, NodeKind, Route};
    use crate::weapons::Held;
    use crate::world::{Exit, Outcome};

    /// The river's kit and no more: one jump, the hook, Gavel Drop, the Bench.
    /// how near an enemy comes before the fighting Pilot fells it
    const FELLED: i32 = 30 * ONE;

    pub(crate) fn river_kit() -> Build {
        Build {
            max_health: 100_000,
            grapple: true,
            pound: 1,
            bench: 1,
            ..Build::default()
        }
    }

    pub(crate) fn course_world(arch: &str) -> (Content, World) {
        course_world_in("water_gap", arch)
    }

    /// A course of zone `zone`'s, with the river's kit.
    pub(crate) fn course_world_in(zone: &str, arch: &str) -> (Content, World) {
        let c = content();
        let zone = c.zones.iter().position(|z| z.id == zone).expect("the zone");
        let r = Route::build_for(&c, zone, 7, &Have::level(1));
        let node = r
            .nodes
            .iter()
            .find(|n| n.archetype == arch)
            .expect("the section")
            .clone();
        let exit = Exit {
            to: 1,
            main: true,
            reward: None,
            sealed: false,
        };
        let p = Player::new(V3::ZERO, 0, river_kit());
        let w = World::new(&c, &node, &[exit], p, Held::new(0, 1), 3);
        let _ = NodeKind::Fight;
        (c, w)
    }

    /// Drives a body along the course's line; the ticks it took and the
    /// health it lost, or where it stuck.
    pub(crate) fn cross(arch: &str) -> Result<(i32, i32), String> {
        cross_in("water_gap", arch, false)
    }

    /// Drives a body along a course of zone `zone`'s; when `fight`, every
    /// enemy that comes within `FELLED` of it falls (as if shot: the Pilot
    /// cannot aim), and at the line's end it waits for the way on to open.
    pub(crate) fn cross_in(zone: &str, arch: &str, fight: bool) -> Result<(i32, i32), String> {
        let (c, mut w) = course_world_in(zone, arch);
        let line = w.course.as_ref().unwrap().line.clone();
        let mut pilot = Pilot::default();
        let start = w.player.health;
        for tick in 0..60 * 60 * 4 {
            if fight {
                let at = w.player.pos;
                for e in w
                    .enemies
                    .iter_mut()
                    .filter(|e| e.alive() && e.pos.dist(at) < FELLED)
                {
                    e.hp = 0;
                }
            }
            let Some(d) = pilot.drive(&line, &w.player) else {
                // the line done: a few steps on to the way on (once it opens)
                for _ in 0..if fight { 60 * 30 } else { 60 } {
                    if fight {
                        let at = w.player.pos;
                        for e in w
                            .enemies
                            .iter_mut()
                            .filter(|e| e.alive() && e.pos.dist(at) < FELLED * 3)
                        {
                            e.hp = 0;
                        }
                    }
                    let fin = w.course.as_ref().unwrap().finish;
                    let to = fin.sub(w.player.pos);
                    w.player.yaw = crate::geom::yaw_pitch_of(to).0;
                    let ctl = crate::player::Controls {
                        move_z: ONE,
                        ..Default::default()
                    };
                    let eye = w.player.chest();
                    w.tick(&c, &ctl, eye, crate::geom::facing(w.player.yaw, 0));
                    if matches!(w.outcome, Outcome::Through(_)) {
                        return Ok((tick, start - w.player.health));
                    }
                }
                return Err(format!(
                    "{arch}: the line ends at {:?}, short of the way on at {:?}",
                    w.player.pos,
                    w.course.as_ref().unwrap().finish
                ));
            };
            w.player.yaw = d.yaw;
            let eye = w.player.chest();
            let before = w.notes.iter().filter(|n| n.sound == "drown").count();
            w.tick(&c, &d.controls, eye, d.aim);
            if w.notes.iter().filter(|n| n.sound == "drown").count() > before {
                return Err(format!(
                    "{arch}: drowned on step {} ({:?}) at tick {tick}, from {:?}",
                    pilot.step, line[pilot.step], w.player.prev_pos
                ));
            }
            if pilot.t > 60 * 20 {
                return Err(format!(
                    "{arch}: stuck on step {} ({:?}) at {:?}",
                    pilot.step, line[pilot.step], w.player.pos
                ));
            }
            if matches!(w.outcome, Outcome::Through(_)) {
                return Ok((tick, start - w.player.health));
            }
        }
        Err(format!("{arch}: out of time"))
    }

    #[test]
    fn the_spillway_can_be_crossed_on_its_line_with_the_rivers_kit() {
        let (ticks, lost) = cross("spillway").unwrap();
        assert!(ticks < 60 * 120, "{ticks} ticks");
        let _ = lost;
    }
    fn tick(c: &Content, w: &mut World, ctl: crate::player::Controls) {
        let eye = w.player.chest();
        let fwd = crate::geom::facing(w.player.yaw, 0);
        w.tick(c, &ctl, eye, fwd);
    }

    fn idle() -> crate::player::Controls {
        crate::player::Controls::default()
    }

    #[test]
    fn a_bench_laid_in_the_air_is_stood_on_and_shots_pass_through_it() {
        let (c, mut w) = course_world("spillway");
        // over open water, falling
        w.player.pos = v3(0, int(6), -int(44));
        w.player.grounded = false;
        tick(
            &c,
            &mut w,
            crate::player::Controls {
                bench_pressed: true,
                ..idle()
            },
        );
        let (i, _) = w.benches[0];
        let top = w.arena.level.dynamic[i].solid.max.y;
        for _ in 0..30 {
            tick(&c, &mut w, idle());
        }
        assert!(w.player.grounded, "standing on the Bench");
        assert!((w.player.pos.y - top).abs() <= ONE / 20, "on its top");
        // a ray down through it meets nothing until the water's bed
        let hit = w
            .arena
            .level
            .ray(v3(0, top + ONE, -int(44)), v3(0, -ONE, 0), int(3));
        assert!(hit.is_none(), "shots and orbs pass through a Bench");
        // from below, a body rises through it
        assert!(!w.arena.level.blocked(
            v3(-ONE / 4, top - ONE / 2, -int(44) - ONE / 4),
            v3(ONE / 4, top + ONE, -int(44) + ONE / 4)
        ));
        // and it is gone once its time is up
        for _ in 0..BENCH_TICKS {
            tick(&c, &mut w, idle());
        }
        assert!(!w.arena.level.dynamic[i].on, "taken up after its time");
    }

    #[test]
    fn the_water_returns_you_to_the_last_post_you_reached_a_tenth_of_your_health_the_poorer() {
        let (c, mut w) = course_world("spillway");
        w.player.build.max_health = 200;
        w.player.health = 200;
        let posts = w.course.as_ref().unwrap().posts.clone();
        // reach the second post
        w.player.pos = posts[1];
        tick(&c, &mut w, idle());
        assert_eq!(w.course.as_ref().unwrap().post, 1);
        // and fall in
        w.player.pos = v3(0, -ONE, -int(140));
        tick(&c, &mut w, idle());
        assert_eq!(w.player.health, 200 - 200 * DROWN_PCT / 100);
        assert!(w.player.pos.sub(posts[1]).len_xz() < ONE, "back at the post");
        assert!(w.notes.iter().any(|n| n.sound == "drown"));
    }

    #[test]
    fn a_lever_hooked_opens_its_gate_for_its_time_and_the_gate_shuts_but_never_on_you() {
        let (c, mut w) = course_world("spillway");
        let k = w.course.as_ref().unwrap().clone();
        let lever = k
            .switches
            .iter()
            .find(|s| s.kind == SwitchKind::Lever)
            .unwrap()
            .clone();
        let door = k.doors[lever.opens[0]].dynamic;
        assert!(w.arena.level.dynamic[door].on, "shut to begin with");
        // the hook strikes the lever's ring
        w.player.pos = v3(0, int(2), -int(168));
        let ring = lever.at.centre();
        w.player.yaw = crate::geom::yaw_pitch_of(ring.sub(w.player.pos)).0;
        let eye = w.player.chest();
        w.tick(
            &c,
            &crate::player::Controls {
                grapple_pressed: true,
                grapple_held: true,
                ..idle()
            },
            eye,
            ring.sub(eye).norm(),
        );
        assert!(!w.arena.level.dynamic[door].on, "open");
        // standing in the gateway when its time is up, it waits
        let gate = w.arena.level.dynamic[door].solid.centre();
        w.player.release_grapple();
        for _ in 0..lever.ticks + 30 {
            w.player.pos = v3(gate.x, gate.y - int(2), gate.z);
            w.player.vel = V3::ZERO;
            tick(&c, &mut w, idle());
        }
        assert!(!w.arena.level.dynamic[door].on, "never shuts on you");
        w.player.pos = v3(0, int(2), -int(170));
        tick(&c, &mut w, idle());
        tick(&c, &mut w, idle());
        assert!(w.arena.level.dynamic[door].on, "shut once you are clear");
    }

    /// A world with a rotten floor over a drum-cap, a root, and a plain floor.
    fn ground_world() -> (Content, World) {
        let (c, mut w) = course_world("spillway");
        let level = &mut w.arena.level;
        let rotten = level.add_dynamic(Dyn {
            solid: Solid::new(v3(-int(2), int(9), -int(42)), v3(int(2), int(10), -int(38))),
            on: true,
            one_way: false,
        });
        let drum = Solid::new(v3(-int(2), int(3), -int(42)), v3(int(2), int(4), -int(38)));
        level.solids.push(drum);
        *level = {
            let mut l = crate::level::Level::new(level.solids.clone());
            for d in level.dynamic.clone() {
                l.add_dynamic(d);
            }
            l
        };
        let root = w.arena.level.add_dynamic(Dyn {
            solid: Solid::new(v3(-int(2), 0, -int(50)), v3(int(2), int(2), -int(46))),
            on: true,
            one_way: false,
        });
        let k = w.course.as_mut().unwrap();
        k.rotten.push(rotten);
        k.drums.push(drum);
        k.movers.push(Mover::breathing(
            root,
            Solid::new(v3(-int(2), 0, -int(50)), v3(int(2), int(2), -int(46))),
            int(4),
            240,
            0,
        ));
        (c, w)
    }

    #[test]
    fn a_pound_breaks_a_rotten_floor_and_drops_on_through_and_off_a_drum_cap_bounces_twice_as_high() {
        let (c, mut w) = ground_world();
        w.player.pos = v3(0, int(14), -int(40));
        w.player.grounded = false;
        tick(
            &c,
            &mut w,
            crate::player::Controls {
                pound_pressed: true,
                ..idle()
            },
        );
        let mut top = 0;
        let mut broke = false;
        let mut bounced = false;
        for _ in 0..240 {
            tick(&c, &mut w, idle());
            let k = w.course.as_ref().unwrap();
            broke |= !w.arena.level.dynamic[k.rotten[0]].on;
            if broke && w.player.vel.y > 0 {
                bounced = true;
            }
            if bounced {
                top = top.max(w.player.pos.y);
            }
            if bounced && w.player.vel.y < 0 {
                break;
            }
        }
        assert!(broke, "the rotten floor broke");
        // pounded from 14 m onto the drum at 4 m: a plain bounce rises a
        // little over 14 m; the drum's, twice the drop and more
        assert!(top > int(4) + int(10) * 2, "bounced to {}", top / ONE);
    }

    #[test]
    fn a_root_rises_and_sinks_and_carries_whoever_stands_on_it() {
        let (c, mut w) = ground_world();
        w.player.pos = v3(0, int(2), -int(48));
        w.player.grounded = true;
        let mut low = i32::MAX;
        let mut high = i32::MIN;
        for k in 0..240 {
            tick(&c, &mut w, idle());
            low = low.min(w.player.pos.y);
            high = high.max(w.player.pos.y);
            assert!(
                w.player.grounded || k < 2,
                "footing kept as it rises and sinks (tick {k})"
            );
        }
        assert!(high - low > int(3), "carried {} m", (high - low) / ONE);
        assert!(w.player.grounded);
    }

    #[test]
    fn a_curtain_leaves_a_hole_that_moves_along_it() {
        let c = content();
        let p = c.patterns.iter().find(|p| p.id == "weir_curtain").unwrap();
        let v = &p.volleys[0];
        assert_eq!(v.form, crate::content::Form::Curtain);
        let from = v3(0, int(2), -int(20));
        let target = v3(0, int(2), 0);
        let mut holes = Vec::new();
        for repeat in 0..3 {
            let orbs = crate::orbs::formation_for_test(v, from, target, repeat, &c.pal);
            assert_eq!(
                orbs.len() as i32,
                v.count * v.rows - v.gap * v.gap,
                "a hole of {} by {}",
                v.gap,
                v.gap
            );
            // where the hole is: the mean across of what is missing
            let xs: i32 = orbs.iter().map(|o| o.pos.x).sum::<i32>() / orbs.len() as i32;
            holes.push(xs);
        }
        assert!(
            holes[0] != holes[1] && holes[1] != holes[2],
            "the hole moves: {holes:?}"
        );
    }

    #[test]
    fn the_oak_can_be_climbed_on_its_line_its_bar_falling_as_you_rise_and_at_its_crown_it_calls_the_summons()
    {
        let c = content();
        let zone = c.zones.iter().position(|z| z.id == "water_gap").unwrap();
        let r = Route::build_for(&c, zone, 7, &Have::level(1));
        let node = r.nodes.iter().find(|n| n.kind == NodeKind::Boss).unwrap().clone();
        let p = Player::new(V3::ZERO, 0, river_kit());
        let mut w = World::new(&c, &node, &[], p, Held::new(0, 1), 3);
        assert!(!w.cleared, "the Oak's arena is fought");
        let line = w.course.as_ref().unwrap().line.clone();
        let mut pilot = Pilot::default();
        let mut first_bar = None;
        let mut halfway = None;
        for tick in 0..60 * 60 * 3 {
            let Some(d) = pilot.drive(&line, &w.player) else {
                break;
            };
            w.player.yaw = d.yaw;
            let eye = w.player.chest();
            w.tick(&c, &d.controls, eye, d.aim);
            if let Some(b) = w.boss {
                first_bar.get_or_insert(w.enemies[b].hp);
                // halfway up, the bar is part way down
                let site = w.course.as_ref().unwrap().oak.unwrap();
                if w.player.pos.y > (site.crown_y + site.axis.y) / 2 && halfway.is_none() {
                    halfway = Some(w.enemies[b].hp);
                }
            }
            assert!(
                !w.notes.iter().any(|n| n.sound == "drown"),
                "fell on step {} ({:?}) at tick {tick}",
                pilot.step,
                line[pilot.step.min(line.len() - 1)]
            );
            assert!(
                pilot.t < 60 * 20,
                "stuck on step {} ({:?}) at {:?}",
                pilot.step,
                line[pilot.step],
                w.player.pos
            );
            if w.course.as_ref().unwrap().summons.is_some() {
                break;
            }
        }
        let k = w.course.as_ref().unwrap();
        let s = k.summons.expect("the Summons came at the crown");
        let b = w.boss.expect("the Oak");
        assert_eq!(c.enemies[w.enemies[s].def].id, "summons");
        assert_eq!(w.elite, Some(s), "its own bar");
        let line_hp = w.enemies[b].max_hp * OAK_CLIMB_BAR_PCT / 100;
        assert!(first_bar.unwrap() > line_hp, "the bar began full");
        let mid = halfway.expect("the climb passed halfway");
        assert!(
            mid < first_bar.unwrap() && mid > line_hp,
            "halfway up, the bar halfway down: {mid}"
        );
        assert!(
            w.enemies[b].hp <= line_hp,
            "and fell to the climb's line as you rose"
        );
        // the Summons' health is now the Oak's bar
        w.enemies[s].hp = w.enemies[s].max_hp / 2;
        tick(&c, &mut w, idle());
        assert!(w.enemies[b].hp <= line_hp / 2 + 1);
        // and its fall is the Oak's
        w.enemies[s].hp = 0;
        tick(&c, &mut w, idle());
        assert!(!w.enemies[b].alive());
    }

    #[test]
    fn the_summons_husk_takes_every_blow_first_breaks_to_a_stagger_at_double_damage_and_grows_back() {
        let c = content();
        let def = c.enemy("summons").unwrap();
        let d = &c.enemies[def];
        let mut e = crate::enemy::Enemy::new(def, d, v3(0, int(10), 0), 1);
        e.state = crate::enemy::State::Rest(100);
        let hp = e.hp;
        assert!(e.husk > 0);
        e.hurt(50);
        assert_eq!(e.hp, hp, "the Husk takes it");
        e.hurt(d.husk);
        assert_eq!(e.husk, 0);
        assert!(e.stagger > 0, "broken, it staggers");
        e.hurt(50);
        assert_eq!(e.hp, hp - 100, "double damage while staggered");
        let level = crate::level::Level::new(vec![Solid::new(
            v3(-int(30), -ONE, -int(30)),
            v3(int(30), 0, int(30)),
        )]);
        for _ in 0..d.stagger + 2 {
            e.step(&c, &level, v3(0, int(2), int(14)), 0);
        }
        assert_eq!(e.stagger, 0);
        assert_eq!(e.husk, e.husk_max, "whole again after the stagger");
        // chipped and then left alone, it grows back
        e.hurt(100);
        assert!(e.husk < e.husk_max);
        for _ in 0..d.husk_regen + HUSK_REGROW_TICKS + 2 {
            e.step(&c, &level, v3(0, int(2), int(14)), 0);
        }
        assert_eq!(e.husk, e.husk_max);
    }
    #[test]
    fn a_gust_shoves_you_only_while_it_blows_and_its_rain_slants_first() {
        let (c, mut w) = course_world("spillway");
        w.player.pos = v3(0, int(2), -int(10));
        let zone = Solid::new(v3(-int(20), -int(5), -int(30)), v3(int(20), int(20), int(10)));
        let g = Gust {
            zone,
            dir: v3(ONE, 0, 0),
            push: ONE / 20,
            period: 300,
            on: 60,
            phase: 0,
        };
        w.course.as_mut().unwrap().gusts.push(g);
        // still: nothing moves you until it blows
        let mut blown_at = None;
        for k in 0..300 {
            let x = w.player.pos.x;
            tick(&c, &mut w, idle());
            if w.player.pos.x > x + ONE / 100 && blown_at.is_none() {
                blown_at = Some(k);
            }
        }
        let at = blown_at.expect("blown sideways");
        assert!(at >= 300 - 60 - 2, "not before it blows ({at})");
        assert!(g.until(239) > 0 && g.until(240) == 0, "a warning, then the gust");
    }

    #[test]
    fn lightning_is_marked_where_it_will_strike_and_hurts_only_there() {
        let (c, mut w) = course_world("spillway");
        w.player.pos = v3(0, int(2), -int(20));
        w.player.build.max_health = 1000;
        w.player.health = 1000;
        w.course.as_mut().unwrap().storm = Some(Storm {
            every: 30,
            warn: 40,
            radius: int(3),
            damage: 25,
        });
        tick(&c, &mut w, idle());
        for _ in 0..29 {
            tick(&c, &mut w, idle());
        }
        let strikes = w.course.as_ref().unwrap().strikes.clone();
        assert!(!strikes.is_empty(), "a strike marked");
        // stand on the first mark until it strikes
        let (at, left) = strikes[0];
        let before = w.player.health;
        for _ in 0..left + 1 {
            w.player.pos = at;
            w.player.iframes = 0;
            tick(&c, &mut w, idle());
        }
        assert!(w.player.health < before, "struck where it was marked");
    }

    #[test]
    fn under_the_water_you_fall_slower_stroke_up_again_and_are_drawn_toward_the_eye() {
        let (c, mut w) = course_world("spillway");
        w.course.as_mut().unwrap().underwater = Some(Underwater {
            gravity_pct: SWIM_GRAVITY_PCT,
            eye: v3(int(30), 0, -int(20)),
            pull: ONE / 400,
        });
        w.course.as_mut().unwrap().water = -int(100);
        w.player.pos = v3(0, int(40), -int(20));
        w.player.grounded = false;
        w.player.jumps_used = w.player.build.jumps;
        tick(&c, &mut w, idle());
        for _ in 0..30 {
            tick(&c, &mut w, idle());
        }
        let slow = w.player.vel.y;
        assert!(slow > -GRAVITY * 30 * 6 / 10, "falls slower: {}", slow);
        assert!(w.player.pos.x > 0, "drawn toward the eye");
        tick(
            &c,
            &mut w,
            crate::player::Controls {
                jump_pressed: true,
                ..idle()
            },
        );
        assert!(w.player.vel.y > 0, "a stroke up with no jumps left");
    }

    #[test]
    fn wreckage_turns_round_its_eye_and_carries_whoever_stands_on_it() {
        let (c, mut w) = course_world("spillway");
        // a plank in the open air over the race, turning round a point
        let base = Solid::new(v3(int(4), int(18), -int(46)), v3(int(8), int(20), -int(42)));
        let i = w.arena.level.add_dynamic(Dyn {
            solid: base,
            on: true,
            one_way: false,
        });
        let mut mv = Mover::breathing(i, base, 0, 600, 0);
        mv.orbit = fx::TURN / 600;
        mv.round = v3(0, 0, -int(44));
        w.course.as_mut().unwrap().movers.push(mv);
        w.player.pos = v3(int(6), int(20), -int(44));
        w.player.grounded = true;
        // a crewman on it too, asleep where he stands
        let def = c.enemy("gunner").unwrap();
        let mut e = crate::enemy::Enemy::new(def, &c.enemies[def], v3(int(7), int(21), -int(45)), 1);
        e.state = crate::enemy::State::Rest(100_000);
        w.enemies.push(e);
        let rider = w.enemies.len() - 1;
        for _ in 0..150 {
            tick(&c, &mut w, idle());
        }
        let q = w.enemies[rider].pos;
        assert!(
            q.x.abs() < int(4) && (q.z + int(44)).abs() > int(4),
            "the crewman rode it too: {q:?}"
        );
        // a quarter of a turn: from the east of the point to its north or south
        let p = w.player.pos;
        assert!(
            p.x.abs() < int(3) && (p.z + int(44)).abs() > int(4),
            "carried round: {p:?}"
        );
        assert!(w.player.grounded);
    }

    #[test]
    fn a_fought_course_opens_only_when_its_crews_are_down_and_one_overboard_drowns() {
        let (c, mut w) = course_world_in("far_shore", "fleet");
        assert!(w.course.as_ref().unwrap().fought);
        assert!(!w.cleared, "a fought course begins shut");
        // only this crew: the ships' own crews put down
        for e in w.enemies.iter_mut() {
            e.hp = 0;
        }
        let def = c.enemy("deckhand").unwrap();
        let mut e = crate::enemy::Enemy::new(def, &c.enemies[def], v3(0, int(3), -int(60)), 1);
        e.home = e.pos;
        w.enemies.push(e);
        for _ in 0..10 {
            tick(&c, &mut w, idle());
        }
        assert!(!w.cleared, "shut while a crew stands");
        // over the side
        let last = w.enemies.len() - 1;
        w.enemies[last].pos.y = -int(5);
        for _ in 0..3 {
            tick(&c, &mut w, idle());
        }
        assert!(!w.enemies[last].alive(), "drowned");
        assert!(w.cleared, "open once all are down");
    }

    #[test]
    fn nothing_leafy_stands_over_the_oaks_crown_to_hide_the_summons() {
        let (c, w) = course_world_in("water_gap", "boss");
        let site = w.course.as_ref().unwrap().oak.unwrap();
        let leaf = c.pal.c("oak_leaf");
        let crown = site.axis.y + site.crown_y;
        for s in w.arena.shapes.iter().filter(|s| s.rgb == leaf) {
            let at = s.at.t;
            let near = v3(at.x - site.axis.x, 0, at.z - site.axis.z).len_xz() < site.crown_r + int(10);
            assert!(
                !near || at.y < crown,
                "leaves over the crown at {at:?} (its floor at {crown})"
            );
        }
    }

    #[test]
    fn overboard_in_the_open_sea_you_sink_slowly_and_can_hook_yourself_back_aboard() {
        let (c, mut w) = course_world_in("far_shore", "fleet");
        for e in w.enemies.iter_mut() {
            e.hp = 0;
        }
        let k = w.course.clone().unwrap();
        assert!(k.depth > 0, "the sea lets you swim");
        // the first ship's tallest mast, and the sea off her side
        let flag = &k.flags[0];
        let mast = k.movers[flag.mast].base;
        let deck = k.movers[flag.deck].base;
        let foot = v3(mast.centre().x, deck.max.y, mast.centre().z);
        w.player.pos = v3(deck.min.x - int(6), k.water + int(1), foot.z);
        w.player.prev_pos = w.player.pos;
        let health = w.player.health;
        let mut fastest = 0;
        for _ in 0..90 {
            tick(&c, &mut w, idle());
            // once in, the sea slows the fall
            if w.player.pos.y < k.water - ONE {
                fastest = fastest.max(-w.player.vel.y);
            }
        }
        assert!(!w.notes.iter().any(|n| n.sound == "drown"), "not drowned yet");
        assert!(
            w.player.pos.y < k.water && w.player.pos.y > k.water - k.depth,
            "in the sea: {}",
            w.player.pos.y
        );
        assert!(fastest <= SEA_SINK, "sank at {fastest}");
        assert_eq!(w.player.health, health);
        // the hook, at her mast, pulls you out and up over her deck
        let line = [
            Step::Hook(foot.add(v3(0, int(4), 0))),
            Step::Air(foot.add(v3(int(1), 0, 0))),
        ];
        let mut pilot = Pilot::default();
        for _ in 0..60 * 8 {
            let Some(d) = pilot.drive(&line, &w.player) else {
                break;
            };
            w.player.yaw = d.yaw;
            let eye = w.player.chest();
            w.tick(&c, &d.controls, eye, d.aim);
        }
        assert!(!w.notes.iter().any(|n| n.sound == "drown"), "drowned on the way");
        assert!(
            w.player.pos.y >= deck.max.y - ONE,
            "back aboard: {:?}",
            w.player.pos
        );
    }

    #[test]
    fn the_sea_takes_you_if_you_stay_in_it_too_long_or_sink_too_deep() {
        let (c, mut w) = course_world_in("far_shore", "fleet");
        for e in w.enemies.iter_mut() {
            e.hp = 0;
        }
        let k = w.course.clone().unwrap();
        let off = v3(-int(60), k.water - int(1), int(40));
        let mut drowned_at = None;
        for t in 0..SEA_BREATH + 30 {
            w.player.pos = off;
            w.player.vel = V3::ZERO;
            tick(&c, &mut w, idle());
            if w.notes.iter().any(|n| n.sound == "drown") {
                drowned_at = Some(t);
                break;
            }
        }
        let t = drowned_at.expect("treading water forever");
        assert!(t >= SEA_BREATH - 2, "drowned after {t} ticks");
        // and below the depth, at once
        let (c, mut w) = course_world_in("far_shore", "fleet");
        w.player.pos = v3(off.x, k.water - k.depth - ONE, off.z);
        tick(&c, &mut w, idle());
        assert!(w.notes.iter().any(|n| n.sound == "drown"), "too deep");
    }

    #[test]
    fn the_fleets_decks_and_masts_are_cut_away_between_the_camera_and_you() {
        use crate::render::{Batch, Camera, DRAW_VALUES, DYNAMIC_BUFFER};
        let (c, w) = course_world_in("far_shore", "fleet");
        let cam = Camera {
            eye: w.player.pos.add(v3(0, int(3), int(6))),
            fwd: v3(0, 0, -ONE),
            fov: FOV,
        };
        let mut b = Batch::new([0; 3]);
        crate::draw::world(&mut b, &c, &w, ONE, 0, true, &cam);
        let (_, draws) = b.finish(DYNAMIC_BUFFER);
        let cut: u32 = draws
            .iter()
            .flat_map(|d| d.chunks(DRAW_VALUES))
            .filter(|v| v[5] == 1)
            .map(|v| v[3])
            .sum();
        let movers = w.course.as_ref().unwrap().movers.len() as u32;
        assert!(
            cut >= movers,
            "{cut} instances cut away, for {movers} decks and masts"
        );
        assert!(
            !b_is_scenery_after(&c, &w, &cam),
            "and the batch is left as it was"
        );
    }

    /// Is a batch still marking scenery once the world is drawn?
    fn b_is_scenery_after(c: &Content, w: &World, cam: &crate::render::Camera) -> bool {
        let mut b = crate::render::Batch::new([0; 3]);
        crate::draw::world(&mut b, c, w, ONE, 0, true, cam);
        b.scenery
    }

    #[test]
    fn each_ship_of_the_fleet_strikes_her_colours_when_her_crew_is_down() {
        let (c, mut w) = course_world_in("far_shore", "fleet");
        let k = w.course.clone().unwrap();
        assert_eq!(k.flags.len(), 7, "seven ships flying colours");
        let mut crewed: Vec<usize> = k.flags.iter().flat_map(|f| f.crew.clone()).collect();
        crewed.sort();
        crewed.dedup();
        assert_eq!(crewed.len(), k.emitters.len(), "every hand on one ship");
        for &i in &k.flags[2].crew[1..] {
            w.enemies[k.foes[i]].hp = 0;
        }
        tick(&c, &mut w, idle());
        assert!(
            !w.course.as_ref().unwrap().flags[2].struck,
            "one hand still standing"
        );
        w.enemies[k.foes[k.flags[2].crew[0]]].hp = 0;
        tick(&c, &mut w, idle());
        let flags = &w.course.as_ref().unwrap().flags;
        assert!(flags[2].struck, "her colours struck");
        assert_eq!(flags.iter().filter(|f| f.struck).count(), 1, "and only hers");
    }

    #[test]
    fn the_captured_dragon_keeps_to_its_chain_unhurt_until_its_three_locks_are_shot_off() {
        let (c, mut w) = course_world_in("far_shore", "fleet");
        tick(&c, &mut w, idle());
        let k = w.course.clone().unwrap();
        let ch = k.chain.clone().expect("the dragon's chain");
        assert_eq!(ch.locks.len(), 3);
        assert!(
            ch.perches.len() >= 2,
            "the flagship's mast tops: {:?}",
            ch.perches
        );
        let dragon = k.foes[ch.dragon];
        let e = &w.enemies[dragon];
        assert!(e.shielded && e.leash.is_some(), "on its chain");
        assert_eq!(e.perches, ch.perches, "perching on her masts");
        for (n, &l) in ch.locks.iter().enumerate() {
            w.enemies[k.foes[l]].hp = 0;
            tick(&c, &mut w, idle());
            let e = &w.enemies[dragon];
            if n < 2 {
                assert!(e.shielded, "{} locks left and it is still held", 2 - n);
            } else {
                assert!(!e.shielded && e.leash.is_none(), "free");
                assert_eq!(e.phase, 1, "and fighting all-out");
                assert!(w.course.as_ref().unwrap().chain.as_ref().unwrap().free);
            }
        }
    }

    #[test]
    fn on_its_chain_the_dragon_swoops_no_further_than_the_chain_reaches() {
        let (c, mut w) = course_world_in("far_shore", "fleet");
        let ch = w.course.as_ref().unwrap().chain.clone().unwrap();
        tick(&c, &mut w, idle());
        let dragon = w.course.as_ref().unwrap().foes[ch.dragon];
        // you stand on the flagship's bow, beyond the chain's reach
        let bow = v3(ch.mast.x - int(12), ch.mast.y, ch.mast.z + int(22));
        let mut furthest = 0;
        let mut swooped = false;
        for _ in 0..60 * 60 {
            w.player.pos = bow;
            w.player.vel = V3::ZERO;
            w.player.iframes = 1000;
            tick(&c, &mut w, idle());
            let e = &w.enemies[dragon];
            swooped |= e.diving;
            furthest = furthest.max(e.pos.sub(ch.mast).len_xz());
        }
        assert!(swooped, "it never swooped at you");
        assert!(furthest <= CHAIN_LEASH + ONE, "it flew {furthest} from the mast");
    }

    #[test]
    fn dagon_and_your_older_self_come_where_their_course_puts_them_away_from_you() {
        let c = content();
        for zone in ["far_shore", "vault"] {
            let z = c.zones.iter().position(|q| q.id == zone).unwrap();
            let r = Route::build_for(&c, z, 7, &Have::level(1));
            let node = r.nodes.iter().find(|n| n.kind == NodeKind::Boss).unwrap().clone();
            let mut w = World::new(
                &c,
                &node,
                &[],
                Player::new(V3::ZERO, 0, river_kit()),
                Held::new(0, 1),
                3,
            );
            let start = w.player.pos;
            let at = w.course.as_ref().unwrap().boss_at.expect("the boss's place");
            for _ in 0..90 {
                w.player.pos = start;
                tick(&c, &mut w, idle());
            }
            let b = w.boss.expect("the boss came");
            let e = &w.enemies[b];
            assert!(
                e.pos.sub(at).len_xz() < int(12),
                "{zone}: the boss at {:?}, not by {at:?}",
                e.pos
            );
            assert!(
                e.pos.sub(start).len_xz() > int(15),
                "{zone}: the boss on top of you at {:?}",
                e.pos
            );
        }
    }

    #[test]
    fn the_erasers_charge_fills_only_while_you_hold_its_lit_pylon_and_its_end_is_the_bosss() {
        let c = content();
        let zone = c.zones.iter().position(|z| z.id == "vault").unwrap();
        let r = Route::build_for(&c, zone, 7, &Have::level(1));
        let node = r.nodes.iter().find(|n| n.kind == NodeKind::Boss).unwrap().clone();
        let mut w = World::new(
            &c,
            &node,
            &[],
            Player::new(V3::ZERO, 0, river_kit()),
            Held::new(0, 1),
            3,
        );
        for _ in 0..90 {
            tick(&c, &mut w, idle());
        }
        let b = w.boss.expect("your older self came");
        let ch = w.course.as_ref().unwrap().charge.clone().unwrap();
        assert_eq!(ch.next(), Some(0));
        // away from the pylon, nothing
        let away = ch.pylons[0].add(v3(ch.radius * 3, 0, 0));
        for _ in 0..60 {
            w.player.pos = away;
            w.player.iframes = 1000;
            tick(&c, &mut w, idle());
        }
        assert_eq!(w.course.as_ref().unwrap().charge.as_ref().unwrap().held[0], 0);
        // in its ring, it charges, and the bar falls with it
        for _ in 0..ch.need / 2 {
            w.player.pos = ch.pylons[0];
            w.player.iframes = 1000;
            tick(&c, &mut w, idle());
        }
        assert!(w.course.as_ref().unwrap().charge.as_ref().unwrap().held[0] > 0);
        assert!(w.enemies[b].hp < w.enemies[b].max_hp, "the bar is the charge");
        assert!(w.enemies[b].shielded, "your older self cannot be hurt here");
        // all three held: the boss is gone
        // (a pylon holds no charge while time is stopped, so a few ticks over)
        for k in 0..3 {
            for _ in 0..ch.need * 3 / 2 {
                if w.course.as_ref().unwrap().charge.as_ref().unwrap().held[k] >= ch.need {
                    break;
                }
                w.player.pos = ch.pylons[k];
                w.player.iframes = 1000;
                tick(&c, &mut w, idle());
            }
        }
        assert!(
            !w.enemies[b].alive(),
            "erased: {:?}",
            w.course.as_ref().unwrap().charge.as_ref().unwrap().held
        );
    }
    #[test]
    fn a_cockatrices_gaze_slows_what_it_hits() {
        let (c, mut w) = course_world("spillway");
        let full = w.player.top_speed();
        let p = c.patterns.iter().position(|p| p.id == "gaze_wall").unwrap();
        assert!(c.patterns[p].volleys[0].slow);
        let chest = w.player.chest();
        let mut o = crate::orbs::formation_for_test(
            &c.patterns[p].volleys[0],
            chest.add(v3(0, 0, -int(6))),
            chest,
            0,
            &c.pal,
        )
        .remove(0);
        o.pos = chest;
        o.prev = chest;
        o.wait = 0;
        w.orbs.push(o);
        tick(&c, &mut w, idle());
        assert!(w.player.slowed > 0, "slowed");
        assert!(w.player.top_speed() < full, "and slower");
    }

    #[test]
    fn a_deck_rising_into_a_body_coming_down_on_it_lands_it_on_top_not_off_the_side() {
        let (c, mut w) = course_world("spillway");
        let base = Solid::new(v3(-int(4), int(16), -int(48)), v3(int(4), int(18), -int(40)));
        let i = w.arena.level.add_dynamic(Dyn {
            solid: base,
            on: true,
            one_way: false,
        });
        // rising fast at the start of its breath
        w.course
            .as_mut()
            .unwrap()
            .movers
            .push(Mover::breathing(i, base, int(6), 120, fx::TURN / 4));
        let mut landed = false;
        // just over its top at the top of a jump, as it comes up fast
        tick(&c, &mut w, idle());
        let top = w.arena.level.dynamic[i].solid.max.y;
        w.player.pos = v3(0, top + ONE / 50, -int(42));
        // and coming in fast along it, as off a Bound Over
        w.player.vel = v3(0, ONE / 100, -ONE * 3 / 10);
        w.player.grounded = false;
        for _ in 0..90 {
            tick(&c, &mut w, idle());
            let top = w.arena.level.dynamic[i].solid.max.y;
            if w.player.grounded && (w.player.pos.y - top).abs() <= ONE / 20 {
                landed = true;
            }
            let p = w.player.pos;
            assert!(p.z < -int(42) + ONE / 2, "never thrown back off its side: {p:?}");
            if landed {
                break;
            }
        }
        assert!(landed);
    }
}

#[cfg(test)]
mod drum_cap {
    use super::tests::*;
    use super::*;

    /// Hooked toward the Falls' drum-cap, a pound thrown from a third of a
    /// second into the pull onward lands on the cap and bounces twice as
    /// high.
    #[test]
    fn a_pound_thrown_early_in_the_pull_still_lands_on_the_drum_cap() {
        let mut report = Vec::new();
        for delay in [20, 25, 30, 40, 50, 60] {
            let (c, mut w) = course_world_in("water_gap", "falls");
            let k = w.course.clone().unwrap();
            let drum = k.drums[0];
            // stand where the line hooks from
            w.player.pos = v3(0, drum.max.y - ONE, drum.centre().z + int(11));
            w.player.prev_pos = w.player.pos;
            for _ in 0..10 {
                let eye = w.player.chest();
                w.tick(
                    &c,
                    &crate::player::Controls::default(),
                    eye,
                    crate::geom::facing(w.player.yaw, 0),
                );
            }
            let ring = v3(0, drum.max.y + int(4), drum.centre().z - ONE);
            let line = [Step::Hook(ring)];
            let mut pilot = Pilot::default();
            let mut bounced = false;
            let mut t0 = None;
            for t in 0..200 {
                let mut d = pilot.drive(&line, &w.player);
                if w.player.grapple.is_some() && t0.is_none() {
                    t0 = Some(t);
                }
                let mut ctl = d.map(|d| d.controls).unwrap_or_default();
                if let Some(d) = d.as_mut() {
                    w.player.yaw = d.yaw;
                }
                if t0.is_some_and(|s| t == s + delay) {
                    ctl.pound_pressed = true;
                }
                let eye = w.player.chest();
                let aim = d.map(|d| d.aim).unwrap_or(crate::geom::facing(w.player.yaw, 0));
                w.tick(&c, &ctl, eye, aim);
                bounced |= w.notes.iter().any(|n| n.sound == "drum_cap");
            }
            report.push((delay, bounced));
        }
        assert!(report.iter().all(|r| r.1), "missed the cap: {report:?}");
    }
}
