//! Enemies: how each kind moves, and the attack cycle every kind shares.
//!
//! The cycle is wind up (the body glows, so you can read what is coming),
//! cast (the pattern's volleys fire on their own clock), rest. Attacks are
//! taken in order from the current phase. A boss moves to its next phase
//! when its health falls past the phase's `from_pct`, calls its summons, and
//! holds still for a moment while the arena is cleared of its orbs.
//!
//! Two kinds move by their own rules. A blinker (the Clerk of Minutes)
//! hovers, and as each rest ends it vanishes and reappears somewhere else
//! in sight of you. A burrower (the Auger) spends its rests under the
//! floor, where it cannot be hurt, moving to a place at its range from you,
//! and surfaces to attack.

use crate::balance::*;
use crate::content::{Content, EnemyDef, Kind, Pattern, Role};
use crate::fx::{self, int, ONE};
use crate::geom::{facing, v3, yaw_pitch_of, V3};
use crate::level::Level;
use crate::orbs::{self, Salvo};
use crate::tank::{self, Treads};

/// A new enemy takes this long to appear, and cannot be hurt or fire meanwhile.
pub const SPAWN_TICKS: i32 = secs(4, 5);
/// a boss changing phase holds still this long
pub const PHASE_TICKS: i32 = secs(3, 2);
/// how high a walker's step can climb
const WALKER_STEP: i32 = ONE * 6 / 10;
/// a blinker reappears at least this far from where it was, and inside
/// this square round the arena's middle
const BLINK_MIN: i32 = int(4);
const BLINK_BOUND: i32 = int(24);
/// how many places a blinker tries before it stays where it is
const BLINK_TRIES: i32 = 12;
/// a burrower stays up this long after its attack, open to fire
pub const BURROW_LINGER: i32 = secs(4, 5);
/// and moves this far under the floor
const BURROW_DEPTH: i32 = int(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Spawning(i32),
    Windup(i32),
    Casting(i32),
    Rest(i32),
    Changing(i32),
    /// an agent out of its rest, at its place, waiting (ticks so far) for
    /// its squad to fire with it
    Ready(i32),
}

/// What the squad has told an agent to do with its rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// nothing yet: ask the squad
    None,
    /// hide at a place out of your sight; step out to fire
    Cover,
    /// go round to your back
    Flank,
    /// a high place, far off
    High,
    /// at you
    Rush,
    /// stand where it is
    Hold,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enemy {
    pub def: usize,
    /// the body's centre
    pub pos: V3,
    pub prev: V3,
    pub vel: V3,
    pub yaw: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub phase: usize,
    pub attack: usize,
    pub state: State,
    /// ticks since it was last hurt, for the white flash
    pub flash: i32,
    pub orbit: i32,
    pub age: i32,
    /// where a boss circles
    pub home: V3,
    /// it held its last attack for want of a line of sight
    pub held_fire: bool,
    /// ticks since it last had a clear line to its target
    pub unseen: i32,
    /// a burrower under the floor
    pub buried: bool,
    /// ticks a surfaced burrower stays up after its attack
    pub linger: i32,
    // an agent: the places it is walking through, its order, where it
    // steps out to fire from, and how long it has been walking
    pub path: Vec<V3>,
    pub order: Order,
    pub peek: Option<V3>,
    pub walked: i32,
    /// it waits in `Ready` for partners before it winds up
    pub sync: bool,
    /// percent off its rests (a Handler's command)
    pub haste: i32,
    /// the aim, frozen before a locked attack fires
    pub locked: Option<V3>,
    /// the walk cycle, for drawing legs
    pub stride: i32,
    /// the enemy that called it, if one did, and a caller's clock
    pub summoned_by: Option<usize>,
    pub summon_timer: i32,
    pub rallied: bool,
    /// the Executor's treads, turret and weak points
    pub tank: Option<Box<Treads>>,
}

/// What an enemy did this tick that the world must act on.
#[derive(Debug, Default)]
pub struct Deeds {
    pub salvo: Salvo,
    /// enemies to summon, by definition index
    pub summons: Vec<usize>,
    pub phase_changed: bool,
    pub burst: bool,
    /// a blinker moved: from, to
    pub blink: Option<(V3, V3)>,
    /// a burrower came up or went down this tick
    pub surfaced: bool,
    pub dove: bool,
}

impl Enemy {
    pub fn new(def: usize, d: &EnemyDef, pos: V3, orbit: i32) -> Enemy {
        Enemy {
            def,
            pos,
            prev: pos,
            vel: V3::ZERO,
            yaw: 0,
            hp: d.hp,
            max_hp: d.hp,
            phase: 0,
            attack: 0,
            state: State::Spawning(SPAWN_TICKS),
            flash: 100,
            orbit,
            age: 0,
            home: pos,
            held_fire: false,
            unseen: 0,
            buried: false,
            linger: 0,
            path: Vec::new(),
            order: Order::None,
            peek: None,
            walked: 0,
            sync: false,
            haste: 0,
            locked: None,
            stride: 0,
            summoned_by: None,
            summon_timer: 0,
            rallied: false,
            tank: (d.kind == Kind::Tank).then(|| Box::new(Treads::new(d))),
        }
    }

    pub fn alive(&self) -> bool {
        self.hp > 0
    }

    pub fn hurtable(&self) -> bool {
        self.alive() && !self.buried && !matches!(self.state, State::Spawning(_) | State::Changing(_))
    }

    /// An agent's feet, from its centre.
    pub fn feet(&self, d: &EnemyDef) -> V3 {
        self.pos.sub(v3(0, d.hover, 0))
    }

    /// Takes damage; true if this blow killed it.
    pub fn hurt(&mut self, damage: i32) -> bool {
        if !self.hurtable() {
            return false;
        }
        self.hp -= damage;
        self.flash = 0;
        self.hp <= 0
    }

    pub fn health_pct(&self) -> i32 {
        self.hp.max(0) * 100 / self.max_hp.max(1)
    }

    /// How brightly the body glows: up through a windup, so an attack is
    /// readable before it comes. Q16, 0 to ONE.
    pub fn glow(&self, c: &Content) -> i32 {
        match self.state {
            State::Windup(t) => {
                let total = self.current_attack(c).map_or(1, |a| a.windup.max(1));
                ONE - fx::div(t * ONE, total * ONE)
            }
            State::Casting(_) => ONE,
            _ => 0,
        }
    }

    fn current_attack<'a>(&self, c: &'a Content) -> Option<&'a crate::content::Attack> {
        let d = &c.enemies[self.def];
        let attacks = &d.phases[self.phase.min(d.phases.len() - 1)].attacks;
        attacks.get(self.attack % attacks.len().max(1))
    }

    /// One tick: move, then advance the attack cycle. `target` is the
    /// player's chest.
    pub fn step(&mut self, c: &Content, level: &Level, target: V3, me: usize) -> Deeds {
        let d = &c.enemies[self.def];
        let mut deeds = Deeds::default();
        self.prev = self.pos;
        self.age += 1;
        self.flash = (self.flash + 1).min(100);
        let to = target.sub(self.pos);
        let (aim_yaw, _) = yaw_pitch_of(to);
        if !matches!(d.kind, Kind::Agent | Kind::Tank) {
            self.yaw = fx::lerp_angle(self.yaw, aim_yaw, ONE / 8);
        }

        if let State::Spawning(t) = self.state {
            self.state = if t > 1 {
                State::Spawning(t - 1)
            } else {
                // a burrower goes straight under
                if d.kind == Kind::Burrower {
                    self.dive(level, target);
                    deeds.dove = true;
                }
                State::Rest(secs(1, 2) + (me as i32 % 4) * secs(1, 5))
            };
            return deeds;
        }
        if let State::Changing(t) = self.state {
            self.state = if t > 1 {
                State::Changing(t - 1)
            } else {
                State::Rest(secs(1, 3))
            };
            return deeds;
        }

        if d.kind == Kind::Tank {
            tank::drive(self, level, target, d.speed);
            if let Some(t) = self.tank.as_mut() {
                // its last phase is outlasted
                if t.survive > 0 {
                    t.survive -= 1;
                    if t.survive == 0 {
                        self.hp = 0;
                        return deeds;
                    }
                }
            }
        }
        // a boss changes phase the moment its health crosses the line
        if d.kind == Kind::Boss {
            let next = self.phase + 1;
            if next < d.phases.len() && self.health_pct() <= d.phases[next].from_pct {
                self.phase = next;
                self.attack = 0;
                self.state = State::Changing(PHASE_TICKS);
                deeds.summons = d.phases[next].summons.clone();
                deeds.phase_changed = true;
                return deeds;
            }
        }

        if d.kind != Kind::Tank {
            self.locomote(d, level, target, to);
        }
        self.unseen = if self.sees(level, target) {
            0
        } else {
            self.unseen + 1
        };
        if d.kind == Kind::Moth {
            return deeds;
        }

        let Some(attack) = self.current_attack(c).cloned() else {
            return deeds;
        };
        let pattern: &Pattern = &c.patterns[attack.pattern];
        if d.kind == Kind::Agent {
            return self.agent_cycle(c, d, level, target, me, &attack, deeds);
        }
        if d.kind == Kind::Burrower && !self.buried && matches!(self.state, State::Rest(_)) {
            // up after its attack for a moment, then under again
            self.linger -= 1;
            if self.linger <= 0 {
                self.dive(level, target);
                deeds.dove = true;
            }
        }
        self.state = match self.state {
            State::Rest(t) if t > 1 => State::Rest(t - 1),
            State::Rest(_) => {
                if d.kind == Kind::Blinker {
                    let from = self.pos;
                    if let Some(to) = self.blink_spot(d, level, target, me) {
                        self.pos = to;
                        self.prev = to;
                        self.vel = V3::ZERO;
                        deeds.blink = Some((from, to));
                    }
                }
                if d.kind == Kind::Burrower && self.buried {
                    self.buried = false;
                    deeds.surfaced = true;
                    let ground = level
                        .floor_below(self.pos.x, target.y + int(2), self.pos.z)
                        .unwrap_or(0);
                    self.pos.y = ground + d.radius;
                    self.prev = self.pos;
                }
                State::Windup(attack.windup.max(1))
            }
            State::Windup(t) if t > 1 => State::Windup(t - 1),
            // an enemy that cannot see you holds its fire and tries again
            // sooner, so walls do not fill with orbs (SECOND-ORDER P4)
            // (at most every other attack, so cover helps without making
            // an enemy idle)
            State::Windup(_) if !self.held_fire && !self.sees(level, target) => {
                self.held_fire = true;
                State::Rest((attack.rest / 2).max(secs(1, 4)))
            }
            State::Windup(_) => {
                self.held_fire = false;
                State::Casting(0)
            }
            State::Casting(t) => {
                let from = if attack.cannons {
                    tank::muzzles(self, d)
                } else if d.kind == Kind::Tank {
                    vec![tank::main_gun(self)]
                } else {
                    vec![self.muzzle(d)]
                };
                self.cast(c, pattern, t, &from, target, me, level, &mut deeds);
                if t >= pattern.length() {
                    self.attack += 1;
                    self.linger = BURROW_LINGER;
                    State::Rest(attack.rest)
                } else {
                    State::Casting(t + 1)
                }
            }
            s => s,
        };
        deeds
    }

    /// The volleys of a pattern that fire on tick `t` of its cast, from
    /// each of `from`.
    #[allow(clippy::too_many_arguments)]
    fn cast(&self, c: &Content, pattern: &Pattern, t: i32, from: &[V3], target: V3, me: usize, level: &Level, deeds: &mut Deeds) {
        for v in &pattern.volleys {
            let since = t - v.delay;
            if since < 0 {
                continue;
            }
            let k = if v.every > 0 { since / v.every } else { since };
            let on_beat = v.every == 0 || since % v.every == 0;
            if on_beat && k < v.repeat {
                for &at in from {
                    let s = orbs::fire(v, at, target, k, me, &c.pal, level);
                    deeds.salvo.orbs.extend(s.orbs);
                    deeds.salvo.waves.extend(s.waves);
                }
            }
        }
    }

    /// An agent's attack cycle. Its rest is spent walking where the squad
    /// sent it; as the rest ends it steps out to where it can see you, and
    /// either winds up or (`sync`) waits for its partners, whom the squad
    /// starts together. A locked attack stops following you before it fires.
    #[allow(clippy::too_many_arguments)]
    fn agent_cycle(
        &mut self,
        c: &Content,
        d: &EnemyDef,
        level: &Level,
        target: V3,
        me: usize,
        attack: &crate::content::Attack,
        mut deeds: Deeds,
    ) -> Deeds {
        let pattern = &c.patterns[attack.pattern];
        let walking = !self.path.is_empty() && self.walked < AGENT_GIVE_UP;
        self.state = match self.state {
            State::Rest(t) if t > 1 => State::Rest(t - 1),
            // still on its way: the rest waits for it
            State::Rest(_) if walking => State::Rest(1),
            State::Rest(_) => {
                let sees = self.sees(level, target);
                let feet = self.feet(d);
                match self.peek.take() {
                    // step out to where it can see you first (if it got to
                    // its cover: a peek from elsewhere is no use)
                    Some(p) if !sees && !attack.indirect && p.dist(feet) <= PEEK_REACH * 2 => {
                        self.path = vec![p];
                        self.walked = 0;
                        State::Rest(1)
                    }
                    _ if self.sync => State::Ready(0),
                    _ => State::Windup(attack.windup.max(1)),
                }
            }
            State::Ready(t) if t >= SYNC_WAIT => State::Windup(attack.windup.max(1)),
            State::Ready(t) => State::Ready(t + 1),
            State::Windup(t) if t > 1 => {
                if t == attack.lock {
                    self.locked = Some(target);
                }
                State::Windup(t - 1)
            }
            State::Windup(_) if !attack.indirect && !self.held_fire && !self.sees(level, target) => {
                self.held_fire = true;
                self.order = Order::None;
                State::Rest((attack.rest / 2).max(secs(1, 4)))
            }
            State::Windup(_) => {
                self.held_fire = false;
                if attack.lock > 0 && self.locked.is_none() {
                    self.locked = Some(target);
                }
                State::Casting(0)
            }
            State::Casting(t) => {
                let aim = self.locked.unwrap_or(target);
                let from = [self.muzzle(d)];
                self.cast(c, pattern, t, &from, aim, me, level, &mut deeds);
                if t >= pattern.length() {
                    self.attack += 1;
                    self.locked = None;
                    self.order = Order::None;
                    State::Rest((attack.rest * (100 - self.haste) / 100).max(1))
                } else {
                    State::Casting(t + 1)
                }
            }
            s => s,
        };
        deeds
    }

    /// A burrower goes under the floor where it stands.
    fn dive(&mut self, level: &Level, target: V3) {
        self.buried = true;
        let ground = level
            .floor_below(self.pos.x, target.y + int(2), self.pos.z)
            .unwrap_or(0);
        self.pos.y = ground - BURROW_DEPTH;
        self.prev = self.pos;
    }

    /// Somewhere else in sight of `target`, about the enemy's range from
    /// it, at its hover height; chosen from the enemy's age and index, so
    /// the same fight blinks the same way.
    fn blink_spot(&self, d: &EnemyDef, level: &Level, target: V3, me: usize) -> Option<V3> {
        let seed = (self.age as u32)
            .wrapping_mul(2_654_435_761)
            .wrapping_add(me as u32 * 40_503);
        for k in 0..BLINK_TRIES {
            // twelve directions round you, five twelfths of a turn apart
            let a = (seed % fx::TURN as u32) as i32 + fx::TURN / 12 * 5 * k;
            let dist = d.range + int(k % 3) * 2 - int(2);
            let flat = target.add(facing(a, 0).scale(dist));
            if flat.x.abs() > BLINK_BOUND || flat.z.abs() > BLINK_BOUND {
                continue;
            }
            let Some(ground) = level.floor_below(flat.x, target.y + int(3), flat.z) else {
                continue;
            };
            let at = v3(flat.x, ground + d.hover, flat.z);
            if at.dist(self.pos) < BLINK_MIN || level.sphere_hits(at, ONE) {
                continue;
            }
            let to = target.sub(at);
            if level.ray(at, to.norm(), to.len()).is_none() {
                return Some(at);
            }
        }
        None
    }

    /// The colour of the orbs its current attack fires, for the HUD.
    pub fn next_colour<'a>(&self, c: &'a Content) -> Option<&'a str> {
        let a = self.current_attack(c)?;
        c.patterns[a.pattern].volleys.first().map(|v| v.colour.as_str())
    }

    /// A clear line from the body to `target`.
    pub fn sees(&self, level: &Level, target: V3) -> bool {
        let to = target.sub(self.pos);
        let len = to.len();
        len == 0 || level.ray(self.pos, to.norm(), len).is_none()
    }

    /// Where the orbs leave from: the body's centre, or an agent's gun.
    pub fn muzzle(&self, d: &EnemyDef) -> V3 {
        if d.kind == Kind::Agent {
            return self
                .pos
                .add(v3(0, d.radius / 2, 0))
                .add(facing(self.yaw, 0).scale(d.radius));
        }
        self.pos
    }

    /// An agent walks its path: toward the next place, one axis at a time
    /// as a body does, climbing steps and dropping off edges; it faces
    /// where it walks, or you while it aims.
    fn walk(&mut self, d: &EnemyDef, level: &Level, target: V3) {
        let feet = self.feet(d);
        let aiming = matches!(self.state, State::Windup(_) | State::Casting(_) | State::Ready(_));
        let speed = if d.role == Role::Rush && !aiming {
            d.speed * 3 / 2
        } else if aiming {
            0
        } else {
            d.speed
        };
        while let Some(&next) = self.path.first() {
            if v3(next.x - feet.x, 0, next.z - feet.z).len_xz() < ONE / 4 {
                self.path.remove(0);
            } else {
                break;
            }
        }
        let mut wish = V3::ZERO;
        if let Some(&next) = self.path.first() {
            wish = v3(next.x - feet.x, 0, next.z - feet.z).clamp_len(speed);
            self.walked += 1;
        } else if d.role == Role::Rush && !aiming {
            // in the open, straight at you
            wish = v3(target.x - feet.x, 0, target.z - feet.z).clamp_len(speed);
        }
        self.vel = v3(wish.x, self.vel.y, wish.z);
        // as narrow as the walk grid assumes, so it goes where its path does
        let r = PLAYER_RADIUS * 3 / 4;
        for axis in 0..2 {
            let mut next = self.pos;
            if axis == 0 {
                next.x += self.vel.x;
            } else {
                next.z += self.vel.z;
            }
            let f = next.sub(v3(0, d.hover, 0));
            let free = !level.blocked(
                v3(next.x - r, f.y + AGENT_STEP, next.z - r),
                v3(next.x + r, f.y + d.hover + d.radius, next.z + r),
            );
            if free {
                self.pos = next;
            }
        }
        // stand on the floor under the feet, or fall to it
        let f = self.feet(d);
        let ground = level.floor_below(f.x, f.y + AGENT_STEP, f.z).unwrap_or(f.y);
        if ground >= f.y - ONE / 20 {
            self.pos.y = fx::approach(self.pos.y, ground + d.hover, ONE / 5);
            self.vel.y = 0;
        } else {
            self.vel.y = (self.vel.y - GRAVITY).max(-MAX_FALL);
            self.pos.y = (self.pos.y + self.vel.y).max(ground + d.hover);
        }
        let moved = v3(self.vel.x, 0, self.vel.z);
        self.stride = (self.stride + fx::mul(moved.len_xz(), fx::TURN / 3)) & (fx::TURN - 1);
        let look = if aiming || moved == V3::ZERO {
            target.sub(self.pos)
        } else {
            moved
        };
        let (want, _) = yaw_pitch_of(look);
        let turn = fx::wrap(want - self.yaw).clamp(-AGENT_TURN, AGENT_TURN);
        self.yaw = fx::wrap(self.yaw + turn);
    }

    fn locomote(&mut self, d: &EnemyDef, level: &Level, target: V3, to: V3) {
        let flat = v3(to.x, 0, to.z);
        let dist = flat.len();
        let towards = flat.norm();
        let side = v3(-towards.z, 0, towards.x).scale(self.orbit * ONE);
        match d.kind {
            Kind::Turret | Kind::Tank => {}
            Kind::Agent => self.walk(d, level, target),
            Kind::Blinker => {
                // a slow bob where it stands; it moves by blinking
                let ground = level.floor_below(self.pos.x, self.pos.y, self.pos.z).unwrap_or(0);
                let bob = fx::sin(self.age * fx::TURN / 90) / 4;
                self.pos.y = fx::approach(self.pos.y, ground + d.hover + bob, ONE / 30);
            }
            Kind::Burrower => {
                if self.buried {
                    // under the floor, toward a place at its range from you
                    let away = v3(-towards.x, 0, -towards.z);
                    let mut goal = target.add(away.scale(d.range));
                    goal.x = goal.x.clamp(-BLINK_BOUND, BLINK_BOUND);
                    goal.z = goal.z.clamp(-BLINK_BOUND, BLINK_BOUND);
                    let step = v3(goal.x - self.pos.x, 0, goal.z - self.pos.z).clamp_len(d.speed);
                    let ground = level
                        .floor_below(self.pos.x, target.y + int(2), self.pos.z)
                        .unwrap_or(0);
                    self.pos = v3(self.pos.x + step.x, ground - BURROW_DEPTH, self.pos.z + step.z);
                    self.vel = step;
                } else {
                    self.vel = V3::ZERO;
                }
                let _ = dist;
            }
            Kind::Moth => {
                // straight at the chest, weaving
                let weave = side.scale(fx::sin(self.age * fx::TURN / 50) / 2);
                let dir = to.norm().add(weave).norm();
                self.vel = self.vel.lerp(dir.scale(d.speed), ONE / 10);
                self.pos = self.pos.add(self.vel);
            }
            Kind::Floater | Kind::Boss => {
                let home = if d.kind == Kind::Boss {
                    // circle the arena's centre, facing the player
                    let a = self.age * fx::TURN / secs(26, 1) * self.orbit;
                    self.home
                        .add(v3(fx::mul(fx::cos(a), int(9)), 0, fx::mul(fx::sin(a), int(9))))
                } else {
                    self.pos
                };
                let radial = if d.kind == Kind::Boss {
                    V3::ZERO
                } else if dist > d.range + int(3) {
                    towards
                } else if dist < d.range - int(3) {
                    towards.neg()
                } else {
                    V3::ZERO
                };
                let ground = level.floor_below(self.pos.x, self.pos.y, self.pos.z).unwrap_or(0);
                let bob = fx::sin(self.age * fx::TURN / 150) / 3;
                let want_y = ground + d.hover + bob;
                let mut wish = radial.add(side).norm().scale(d.speed);
                if d.kind == Kind::Boss {
                    wish = home.sub(self.pos).clamp_len(d.speed);
                }
                wish.y = (want_y - self.pos.y).clamp(-d.speed.max(ONE / 30), d.speed.max(ONE / 30));
                self.vel = self.vel.lerp(wish, ONE / 12);
                // blocked: slide along one axis, or rise over it (it flies);
                // turn back only when every way is shut (SECOND-ORDER P7)
                let v = self.vel;
                let tries = [
                    v,
                    v3(v.x, v.y, 0),
                    v3(0, v.y, v.z),
                    v3(v.x / 2, d.speed.max(ONE / 30), v.z / 2),
                ];
                match tries
                    .iter()
                    .find(|t| !level.sphere_hits(self.pos.add(**t), d.radius))
                {
                    Some(t) => self.pos = self.pos.add(*t),
                    None => {
                        self.orbit = -self.orbit;
                        self.vel = self.vel.neg().scale(ONE / 2);
                    }
                }
            }
            Kind::Walker => {
                let wish = if dist > d.range {
                    towards.scale(d.speed)
                } else {
                    side.scale(d.speed / 2)
                };
                self.vel = self.vel.lerp(wish, ONE / 8);
                for axis in 0..2 {
                    let mut next = self.pos;
                    if axis == 0 {
                        next.x += self.vel.x;
                    } else {
                        next.z += self.vel.z;
                    }
                    let feet = v3(next.x, next.y - d.radius, next.z);
                    let floor = level.floor_below(feet.x, feet.y + WALKER_STEP, feet.z);
                    let r = d.radius * 3 / 4;
                    let free = !level.blocked(
                        v3(next.x - r, feet.y + WALKER_STEP, next.z - r),
                        v3(next.x + r, feet.y + d.radius * 2, next.z + r),
                    );
                    if free && floor.is_some() {
                        self.pos = next;
                    } else {
                        self.orbit = -self.orbit;
                    }
                }
                // settle on the floor under the feet
                if let Some(f) =
                    level.floor_below(self.pos.x, self.pos.y - d.radius + WALKER_STEP, self.pos.z)
                {
                    self.pos.y = fx::approach(self.pos.y, f + d.radius, ONE / 6);
                }
                let _ = target;
            }
        }
    }
}

/// The damage a body takes from touching an enemy, if it does.
pub fn contact(e: &Enemy, d: &EnemyDef, feet: V3) -> bool {
    let chest = feet.add(v3(0, PLAYER_CHEST, 0));
    e.hurtable() && orbs::within(e.pos, chest, d.radius + PLAYER_RADIUS + ONE / 4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;
    use crate::level::Solid;

    fn floor() -> Level {
        Level::new(vec![Solid::new(
            v3(-int(60), -ONE, -int(60)),
            v3(int(60), 0, int(60)),
        )])
    }

    fn make(c: &Content, id: &str, pos: V3) -> Enemy {
        let i = c.enemy(id).unwrap();
        Enemy::new(i, &c.enemies[i], pos, 1)
    }

    #[test]
    fn a_sentinel_appears_winds_up_and_then_fires_its_spiral() {
        let c = content();
        let l = floor();
        let mut e = make(&c, "sentinel", v3(0, int(2), 0));
        let target = v3(0, int(1), -int(12));
        let mut first_fire = None;
        let mut orbs = 0;
        for t in 0..600 {
            let d = e.step(&c, &l, target, 0);
            if !d.salvo.orbs.is_empty() && first_fire.is_none() {
                first_fire = Some(t);
            }
            orbs += d.salvo.orbs.len();
        }
        // spawning (48 ticks) then a rest, then the 500 ms windup (30 ticks)
        let t = first_fire.expect("it fired");
        assert!(t > SPAWN_TICKS + 30, "fired at {t}");
        // the spiral is 4 orbs, 36 times
        assert!(orbs >= 144, "{orbs}");
    }

    #[test]
    fn an_enemy_cannot_be_hurt_while_it_appears() {
        let c = content();
        let mut e = make(&c, "drifter", v3(0, int(3), 0));
        assert!(!e.hurt(50));
        assert_eq!(e.hp, e.max_hp);
        e.state = State::Rest(10);
        assert!(!e.hurt(10));
        assert!(e.hurt(1000));
    }

    #[test]
    fn a_drifter_keeps_its_range() {
        let c = content();
        let l = floor();
        let mut e = make(&c, "drifter", v3(0, int(3), 0));
        let target = v3(0, int(1), -int(40));
        for _ in 0..900 {
            e.step(&c, &l, target, 0);
        }
        let d = target.sub(e.pos).len_xz();
        let range = c.enemies[e.def].range;
        assert!((d - range).abs() < int(6), "{d} vs {range}");
        // and floats near its hover height
        assert!((e.pos.y - c.enemies[e.def].hover).abs() < ONE, "{}", e.pos.y);
    }

    #[test]
    fn a_moth_flies_at_you() {
        let c = content();
        let l = floor();
        let mut e = make(&c, "moth", v3(0, int(2), 0));
        let target = v3(int(20), int(1), 0);
        let mut touched = false;
        for _ in 0..400 {
            e.step(&c, &l, target, 0);
            touched |= contact(&e, &c.enemies[e.def], target.sub(v3(0, PLAYER_CHEST, 0)));
        }
        assert!(touched);
    }

    #[test]
    fn the_boss_changes_phase_as_its_health_falls_and_calls_its_summons() {
        let c = content();
        let l = floor();
        let mut e = make(&c, "bailiff", v3(0, int(6), 0));
        e.state = State::Rest(5);
        e.hp = e.max_hp * 60 / 100;
        let d = e.step(&c, &l, v3(0, int(1), -int(15)), 0);
        assert!(d.phase_changed);
        assert_eq!(e.phase, 1);
        // enemies.json: phase 2 calls two moths and a lantern
        assert_eq!(d.summons.len(), 3);
        assert!(
            !e.hurtable(),
            "it holds still and cannot be hurt while it changes"
        );
    }

    #[test]
    fn an_enemy_that_cannot_see_you_holds_its_fire() {
        let c = content();
        // a wall between the sentinel and the target
        let l = Level::new(vec![
            Solid::new(v3(-int(60), -ONE, -int(60)), v3(int(60), 0, int(60))),
            Solid::new(v3(-int(10), 0, -int(6)), v3(int(10), int(10), -int(5))),
        ]);
        let mut e = make(&c, "sentinel", v3(0, int(2), 0));
        let behind = v3(0, int(1), -int(12));
        let mut fired = 0;
        for _ in 0..600 {
            fired += e.step(&c, &l, behind, 0).salvo.orbs.len();
        }
        // it holds one attack, then fires the next into the wall
        assert!(fired > 0, "it does not wait forever");
        let mut held = 0;
        let mut e2 = make(&c, "sentinel", v3(0, int(2), 0));
        for _ in 0..200 {
            let before = e2.held_fire;
            e2.step(&c, &l, behind, 0);
            held += (!before && e2.held_fire) as i32;
        }
        assert_eq!(held, 1, "the first attack is held");
        // in the open it fires
        let open_target = v3(0, int(1), int(12));
        for _ in 0..600 {
            fired += e.step(&c, &l, open_target, 0).salvo.orbs.len();
        }
        assert!(fired > 0);
    }
}
