//! The Executor: a tracked tank. Its hull turns on the spot to a heading,
//! then drives with weight, slow to start and slow to stop, steering a
//! little as it goes; now and then it backs off, and when you stand in front
//! of it in the open it rams. Its turret turns on its own, slower than you
//! can run round it, to keep its guns on you.
//!
//! Its hull and turret take no damage. Each phase opens weak points (in
//! `enemies.json`, on the turret or the hull, in that part's frame), and
//! breaking every one of them starts the next phase, which grows its
//! cannons. The last phase is outlasted: it rises off the floor and fires
//! from every cannon it has grown until its time runs out.

use crate::balance::*;
use crate::content::{EnemyDef, On, Part, Phase};
use crate::enemy::Enemy;
use crate::fx::{self, int, ONE};
use crate::geom::{facing, right_of, v3, yaw_pitch_of, V3};
use crate::level::Level;

/// The hull's centre stands this high over the floor.
pub const HULL_Y: i32 = ONE * 13 / 10;
/// the turret's ring, over the hull's centre
pub const TURRET_Y: i32 = ONE * 13 / 10;
/// the hull is this long and wide
pub const HULL_LEN: i32 = int(8);
pub const HULL_WIDE: i32 = ONE * 52 / 10;
/// it drives no further than this from the court's middle
const ROAM: i32 = int(22);
/// how long it spends coming in through the glass
pub const ENTER_TICKS: i32 = secs(3, 1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    /// coming in from outside, straight ahead
    Enter,
    /// turning on the spot toward `goal`
    Turn,
    /// driving at `goal`
    Go,
    /// backing away
    Back,
    /// straight at where you were, fast
    Ram,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Treads {
    pub hull: i32,
    /// the turret's heading, in the world
    pub turret: i32,
    /// signed, along the hull's heading
    pub speed: i32,
    pub drive: Drive,
    pub timer: i32,
    pub goal: V3,
    /// the health left in each of this phase's weak points
    pub weak: Vec<i32>,
    /// ticks since the phase began, for the cannons growing in
    pub grown: i32,
    /// how far it has risen (its last phase)
    pub lift: i32,
    /// ticks of its last phase still to outlast
    pub survive: i32,
    /// the tread marks' turn, for drawing
    pub roll: i32,
    /// choices made, for its rhythm
    pub choices: i32,
    pub rammed: i32,
}

impl Treads {
    pub fn new(d: &EnemyDef) -> Treads {
        Treads {
            hull: fx::HALF,
            turret: fx::HALF,
            speed: 0,
            drive: Drive::Enter,
            timer: ENTER_TICKS,
            goal: V3::ZERO,
            weak: d.phases[0].weak.iter().map(|w| w.hp).collect(),
            grown: secs(10, 1),
            lift: 0,
            survive: 0,
            roll: 0,
            choices: 0,
            rammed: 0,
        }
    }

    /// The phase begins: its weak points, its time.
    pub fn begin(&mut self, p: &Phase) {
        self.weak = p.weak.iter().map(|w| w.hp).collect();
        self.grown = 0;
        self.survive = p.survive;
    }
}

/// A part's place in the world: on the hull or the turret, raised by the
/// last phase's lift.
pub fn place(e: &Enemy, t: &Treads, p: &Part) -> V3 {
    let yaw = match p.on {
        On::Hull => t.hull,
        On::Turret => t.turret,
    };
    let base = match p.on {
        On::Hull => e.pos,
        On::Turret => e.pos.add(v3(0, TURRET_Y, 0)),
    };
    base.add(right_of(yaw).scale(p.at[0]))
        .add(v3(0, p.at[1], 0))
        .add(facing(yaw, 0).scale(p.at[2]))
}

/// Every cannon grown by phase `phase`, with the phase each grew in.
pub fn cannons(d: &EnemyDef, phase: usize) -> Vec<(usize, Part)> {
    d.phases
        .iter()
        .enumerate()
        .take(phase + 1)
        .flat_map(|(k, p)| p.cannons.iter().map(move |c| (k, *c)))
        .collect()
}

/// Where each grown cannon's shot leaves from: its muzzle, along its aim.
pub fn muzzles(e: &Enemy, d: &EnemyDef) -> Vec<V3> {
    let Some(t) = e.tank.as_ref() else {
        return Vec::new();
    };
    cannons(d, e.phase)
        .iter()
        .map(|(_, c)| {
            let yaw = match c.on {
                On::Hull => t.hull,
                On::Turret => t.turret,
            } + c.yaw;
            place(e, t, c).add(facing(yaw, c.pitch).scale(int(1)))
        })
        .collect()
}

/// The main gun's muzzle, at the turret's front.
pub fn main_gun(e: &Enemy) -> V3 {
    let Some(t) = e.tank.as_ref() else {
        return e.pos;
    };
    e.pos
        .add(v3(0, TURRET_Y + ONE / 2, 0))
        .add(facing(t.turret, 0).scale(int(6)))
}

/// The armour shots glance off: the hull as three spheres along it, the
/// turret as one.
pub fn armour(e: &Enemy) -> Vec<(V3, i32)> {
    let Some(t) = e.tank.as_ref() else {
        return Vec::new();
    };
    let fwd = facing(t.hull, 0);
    let mut out: Vec<(V3, i32)> = [-int(25) / 10, 0, int(25) / 10]
        .iter()
        .map(|&k| (e.pos.add(fwd.scale(k)), HULL_WIDE / 2))
        .collect();
    out.push((e.pos.add(v3(0, TURRET_Y + ONE / 3, 0)), ONE * 21 / 10));
    out
}

/// This phase's weak points still standing: where, how big, which.
pub fn weak_points(e: &Enemy, d: &EnemyDef) -> Vec<(V3, i32, usize)> {
    let Some(t) = e.tank.as_ref() else {
        return Vec::new();
    };
    let Some(p) = d.phases.get(e.phase) else {
        return Vec::new();
    };
    p.weak
        .iter()
        .enumerate()
        .filter(|(k, _)| t.weak.get(*k).is_some_and(|hp| *hp > 0))
        .map(|(k, w)| (place(e, t, w), w.radius, k))
        .collect()
}

/// Turns `from` toward `to` by at most `rate`.
fn turn(from: i32, to: i32, rate: i32) -> i32 {
    let d = fx::wrap(to - from);
    fx::wrap(from + d.clamp(-rate, rate))
}

/// Would the hull fit with its centre at `at`? Low things (under a metre)
/// it rolls over.
fn fits(level: &Level, at: V3) -> bool {
    let r = HULL_WIDE / 2;
    at.len_xz() < ROAM + int(2) && !level.blocked(v3(at.x - r, ONE, at.z - r), v3(at.x + r, int(3), at.z + r))
}

/// One tick of the tank's driving and its turret. `target` is your chest,
/// `moving` your velocity, `entry` the glass it comes in through.
pub fn drive(e: &mut Enemy, level: &Level, target: V3, top: i32) {
    let Some(mut t) = e.tank.take() else {
        return;
    };
    let me = e.pos;
    let flat_to = v3(target.x - me.x, 0, target.z - me.z);
    let (to_yaw, _) = yaw_pitch_of(flat_to);
    let dist = flat_to.len_xz();
    t.timer -= 1;
    t.grown += 1;
    t.rammed += 1;
    let fwd = facing(t.hull, 0);
    match t.drive {
        Drive::Enter => {
            // through the glass at a ram's speed, until well inside
            t.speed = TANK_RAM_SPEED;
            if me.len_xz() < int(12) {
                t.drive = Drive::Turn;
                t.goal = V3::ZERO;
                t.timer = secs(2, 1);
            }
        }
        Drive::Turn => {
            t.speed = fx::approach(t.speed, 0, TANK_ACCEL * 2);
            let (want, _) = yaw_pitch_of(t.goal.sub(me));
            t.hull = turn(t.hull, want, TANK_TURN);
            if fx::wrap(want - t.hull).abs() < fx::deg(6) || t.timer <= 0 {
                t.drive = Drive::Go;
                t.timer = secs(5, 1);
            }
        }
        Drive::Go => {
            let (want, _) = yaw_pitch_of(t.goal.sub(me));
            t.hull = turn(t.hull, want, TANK_TURN / 3);
            let near = t.goal.sub(me).len_xz() < int(3);
            t.speed = if near {
                fx::approach(t.speed, 0, TANK_ACCEL * 2)
            } else {
                fx::approach(t.speed, top, TANK_ACCEL)
            };
            if (near && t.speed == 0) || t.timer <= 0 {
                choose(&mut t, me, target, to_yaw, dist);
            }
        }
        Drive::Back => {
            t.speed = fx::approach(t.speed, -top * 2 / 3, TANK_ACCEL);
            if t.timer <= 0 {
                choose(&mut t, me, target, to_yaw, dist);
            }
        }
        Drive::Ram => {
            t.speed = fx::approach(t.speed, TANK_RAM_SPEED, TANK_ACCEL * 3);
            if t.timer <= 0 {
                t.drive = Drive::Turn;
                t.timer = secs(2, 1);
                t.goal = V3::ZERO;
            }
        }
    }
    // the last phase rises and hovers; it no longer drives
    if t.survive > 0 || t.lift > 0 {
        t.speed = fx::approach(t.speed, 0, TANK_ACCEL * 4);
        t.lift = fx::approach(t.lift, TANK_RISE, ONE / 40);
    }
    let step = fwd.scale(t.speed);
    let next = v3(me.x + step.x, me.y, me.z + step.z);
    if t.drive == Drive::Enter || fits(level, next) {
        e.pos = next;
    } else if t.speed != 0 {
        // something in the way: stop and pick another heading
        t.speed = 0;
        choose(&mut t, me, target, to_yaw, dist);
        t.drive = Drive::Turn;
    }
    let ground = if t.drive == Drive::Enter {
        0
    } else {
        level.floor_below(e.pos.x, e.pos.y + ONE, e.pos.z).unwrap_or(0)
    };
    e.pos.y = ground + HULL_Y + t.lift;
    e.vel = step;
    t.roll = t.roll.wrapping_add(t.speed);
    // the turret keeps its guns on you, slower than you can circle it
    let (aim, _) = yaw_pitch_of(target.sub(me));
    t.turret = turn(t.turret, aim, TANK_TURRET_TURN);
    e.yaw = t.turret;
    e.tank = Some(t);
}

/// The next move: ram if you stand in front of it in the open, back off if
/// you are close, else drive to a place across from you.
fn choose(t: &mut Treads, me: V3, target: V3, to_yaw: i32, dist: i32) {
    t.choices += 1;
    let ahead = fx::wrap(to_yaw - t.hull).abs() < fx::deg(25);
    if ahead && dist > int(9) && dist < int(26) && t.rammed > secs(8, 1) {
        t.drive = Drive::Ram;
        t.timer = secs(16, 10);
        t.rammed = 0;
        return;
    }
    if dist < int(8) && t.choices % 2 == 0 {
        t.drive = Drive::Back;
        t.timer = secs(18, 10);
        return;
    }
    // a place on the far side of you, turned a little each time
    let a = to_yaw + fx::deg(60) * (t.choices % 5 - 2);
    let mut goal = v3(target.x, 0, target.z).add(facing(a, 0).scale(int(12)));
    if goal.len_xz() > ROAM {
        goal = goal.scale(fx::div(ROAM, goal.len_xz()));
    }
    let _ = me;
    t.goal = goal;
    t.drive = Drive::Turn;
    t.timer = secs(25, 10);
}
