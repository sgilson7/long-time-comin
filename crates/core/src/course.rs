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

/// A root that rises and sinks with the tree's breathing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mover {
    pub dynamic: usize,
    /// where it stands at its lowest
    pub base: Solid,
    pub rise: i32,
    pub period: i32,
    /// where in its breath it starts, a share of a turn
    pub phase: i32,
}

/// Something set in the rock that fires: an enemy that never moves, woken
/// only while you are near.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emitter {
    pub pos: V3,
    /// the enemy it is, by id
    pub enemy: String,
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
    /// the Gallows Oak's arena: where the tree stands
    pub oak: Option<OakSite>,
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
        // a course is crossed, not cleared; the Oak's is fought
        self.cleared = self.kind != crate::route::NodeKind::Boss;
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

        // roots rise and sink, carrying whoever stands on them
        for m in &k.movers {
            let t = (fx::TURN as i64 * self.tick as i64 / m.period.max(1) as i64) as i32 + m.phase;
            let off = m.rise / 2 - fx::mul(m.rise / 2, fx::cos(t));
            let d = &mut level.dynamic[m.dynamic];
            let dy = m.base.min.y + off - d.solid.min.y;
            let riding = self.player.grounded
                && (feet.y - d.solid.max.y).abs() <= ONE / 20
                && feet.x >= d.solid.min.x - PLAYER_RADIUS
                && feet.x <= d.solid.max.x + PLAYER_RADIUS
                && feet.z >= d.solid.min.z - PLAYER_RADIUS
                && feet.z <= d.solid.max.z + PLAYER_RADIUS;
            d.solid.min.y += dy;
            d.solid.max.y += dy;
            if riding {
                self.player.pos.y += dy;
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

        // the water
        if p.y < k.water {
            let back = k.posts.get(k.post).copied().unwrap_or(self.arena.start);
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
        let c = content();
        let zone = c
            .zones
            .iter()
            .position(|z| z.id == "water_gap")
            .expect("the Water Gap");
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
        let (c, mut w) = course_world(arch);
        let line = w.course.as_ref().unwrap().line.clone();
        let mut pilot = Pilot::default();
        let start = w.player.health;
        for tick in 0..60 * 60 * 4 {
            let Some(d) = pilot.drive(&line, &w.player) else {
                // the line done: a few steps on to the way on
                for _ in 0..60 {
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
            if std::env::var("LTC_TRACE").is_ok() && tick % 4 == 0 {
                let q = w.player.pos;
                let f = |v: i32| v as f64 / ONE as f64;
                eprintln!(
                    "t{tick} s{} {:?} pos ({:.1},{:.1},{:.1}) vel ({:.2},{:.2},{:.2}) g{} hook {:?} target {:?}",
                    pilot.step, line[pilot.step.min(line.len() - 1)], f(q.x), f(q.y), f(q.z), f(w.player.vel.x), f(w.player.vel.y), f(w.player.vel.z),
                    w.player.grounded as i32, w.player.grapple.map(|g| (f(g.anchor.y), f(g.anchor.z))), w.hook_target.map(|h| (f(h.y), f(h.z)))
                );
            }
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
        k.movers.push(Mover {
            dynamic: root,
            base: Solid::new(v3(-int(2), 0, -int(50)), v3(int(2), int(2), -int(46))),
            rise: int(4),
            period: 240,
            phase: 0,
        });
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
        for _ in 0..240 {
            tick(&c, &mut w, idle());
            low = low.min(w.player.pos.y);
            high = high.max(w.player.pos.y);
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
        for tick in 0..60 * 60 * 3 {
            let Some(d) = pilot.drive(&line, &w.player) else {
                break;
            };
            w.player.yaw = d.yaw;
            let eye = w.player.chest();
            w.tick(&c, &d.controls, eye, d.aim);
            if std::env::var("LTC_TRACE").is_ok() && tick % 3 == 0 {
                let f = |v: i32| v as f64 / ONE as f64;
                let q = w.player.pos;
                eprintln!(
                    "t{tick} s{} pos ({:.1},{:.1},{:.1}) v ({:.2},{:.2},{:.2}) g{} hook {:?} tgt {:?}",
                    pilot.step,
                    f(q.x),
                    f(q.y),
                    f(q.z),
                    f(w.player.vel.x),
                    f(w.player.vel.y),
                    f(w.player.vel.z),
                    w.player.grounded as i32,
                    w.player
                        .grapple
                        .map(|g| (f(g.anchor.x), f(g.anchor.y), f(g.anchor.z))),
                    w.hook_target.map(|h| (f(h.x), f(h.y), f(h.z)))
                );
            }
            if let Some(b) = w.boss {
                first_bar.get_or_insert(w.enemies[b].hp);
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
}
