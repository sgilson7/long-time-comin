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
use crate::content::{Content, EnemyDef, Gait, Kind, Lunge, Pattern, Role};
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
    /// a mech's or a double's quick boost (ticks left) and its rise (Q16
    /// metres over its standing height), and their clocks
    pub boost: i32,
    pub lift: i32,
    pub air: i32,
    /// held in your hands, to be thrown
    pub held: bool,
    /// the attacks taken once in so many, how many times they came up
    pub passes: i32,
    /// what its body does in this attack, and where to (on the floor)
    pub lunge: Lunge,
    pub anchor: V3,
    /// ticks a hook has pulled it so far
    pub pulled: i32,
    /// a beast: ticks left in a leap; a bite's reach out from its body,
    /// its way, and its clock
    pub leap: i32,
    pub reach: i32,
    pub bite_dir: V3,
    pub bite_t: i32,
    /// how far corruption has taken it, 0 to ONE: the ritual's clockwork
    /// and the Idiot God's eyes, more the higher up the mountain
    pub corrupt: i32,
    /// a percher's perches (found as it first moves), the one it sits on
    /// or flies to, how long it has sat, how many moves it has made, and
    /// its flight: ticks left, and whether it is the swoop's dive
    pub perches: Vec<V3>,
    pub perch: usize,
    pub perched: i32,
    pub moves: i32,
    pub flying: i32,
    pub diving: bool,
    /// you are looking at it (the world says, each tick): an Etching
    /// stands still
    pub seen: bool,
    /// where you were a few seconds ago (the world says): a Reprise
    /// walks there
    pub trail: Option<V3>,
    /// the world's clock, so a beat can be shared (the Toons)
    pub beat: i32,
    /// it cannot be brought below this (the Idiot God, which is sung to
    /// sleep, not killed; Chumbles, who is fed)
    pub floor_hp: i32,
    /// how big it is, Q16 of its full size: Chumbles screams and grows
    /// (everything else is always ONE)
    pub size: i32,
    /// on the last mission: it fires its patterns' remixes
    pub remix: bool,
    /// its Husk (the Summons' shield) and its most; ticks since it was hit;
    /// ticks of a stagger left (the Husk broken: no attacks, double damage)
    pub husk: i32,
    pub husk_max: i32,
    pub unhit: i32,
    pub stagger: i32,
    pub stagger_len: i32,
    /// never hurt (the Gallows Oak)
    pub shielded: bool,
    /// held within this far of a point (the Captured Dragon on its chain)
    pub leash: Option<(V3, i32)>,
}

/// What an enemy did this tick that the world must act on.
#[derive(Debug, Default)]
pub struct Deeds {
    pub salvo: Salvo,
    /// enemies to summon, by definition index
    pub summons: Vec<usize>,
    pub phase_changed: bool,
    /// it stops time this long, for everything but itself
    pub stop: i32,
    pub burst: bool,
    /// a blinker moved: from, to
    pub blink: Option<(V3, V3)>,
    /// a burrower came up or went down this tick
    pub surfaced: bool,
    pub dove: bool,
    /// a cat began its scream this tick, or finished growing
    pub scream: bool,
    pub grown: bool,
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
            boost: 0,
            lift: 0,
            air: 0,
            held: false,
            passes: 0,
            lunge: Lunge::None,
            anchor: pos,
            pulled: 0,
            leap: 0,
            reach: 0,
            bite_dir: V3::ZERO,
            bite_t: 0,
            corrupt: 0,
            perches: Vec::new(),
            perch: 0,
            perched: 0,
            moves: 0,
            flying: 0,
            diving: false,
            seen: false,
            trail: None,
            beat: 0,
            floor_hp: if matches!(d.kind, Kind::God | Kind::Cat) {
                1
            } else {
                0
            },
            size: if d.kind == Kind::Cat { CAT_SMALL } else { ONE },
            remix: false,
            husk: d.husk,
            husk_max: d.husk,
            unhit: 0,
            stagger: 0,
            stagger_len: d.stagger.max(1),
            shielded: d.kind == Kind::Oak,
            leash: None,
        }
    }

    pub fn alive(&self) -> bool {
        self.hp > 0
    }

    pub fn hurtable(&self) -> bool {
        self.alive()
            && !self.buried
            && !self.shielded
            && !matches!(self.state, State::Spawning(_) | State::Changing(_))
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
        self.flash = 0;
        self.unhit = 0;
        if self.husk > 0 {
            // the Husk takes it first; broken, it staggers
            self.husk -= damage;
            if self.husk <= 0 {
                self.husk = 0;
                self.stagger = self.stagger_len;
            }
            return false;
        }
        let damage = if self.stagger > 0 { damage * 2 } else { damage };
        self.hp = (self.hp - damage).max(self.floor_hp);
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

    /// How it moves now: a cat by its phase's gait, a beast by its own.
    pub fn gait(&self, d: &EnemyDef) -> Gait {
        if d.kind == Kind::Cat {
            d.phases[self.phase.min(d.phases.len() - 1)].gait
        } else {
            d.gait
        }
    }

    /// The pattern it fires for an attack's: on the last mission, the
    /// pattern's remix when it has one.
    pub fn pattern_of(&self, c: &Content, pattern: usize) -> usize {
        match (self.remix, c.patterns[pattern].remix) {
            (true, Some(k)) => k,
            _ => pattern,
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
        if !matches!(d.kind, Kind::Agent | Kind::Tank) && self.gait(d) != Gait::Charge {
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
                // a cat goes straight into its scream
                if d.kind == Kind::Cat && self.size < ONE {
                    deeds.scream = true;
                    State::Changing(CAT_SCREAM + 1)
                } else {
                    State::Rest(secs(1, 2) + (me as i32 % 4) * secs(1, 5))
                }
            };
            return deeds;
        }
        if d.kind == Kind::Cat && self.size < ONE && self.hp == self.max_hp {
            // the scream: it grows to its full size, and cannot be hurt
            // or fire meanwhile; its feet stay on the floor
            let t = match self.state {
                State::Changing(t) => t - 1,
                _ => {
                    deeds.scream = true;
                    CAT_SCREAM
                }
            };
            let feet = self.pos.y - fx::mul(d.hover, self.size);
            self.size = CAT_SMALL + (ONE - CAT_SMALL) / CAT_SCREAM * (CAT_SCREAM - t);
            self.state = State::Changing(t.max(1));
            if t <= 0 {
                self.size = ONE;
                self.state = State::Rest(secs(1, 2));
                deeds.grown = true;
            }
            self.pos.y = feet + fx::mul(d.hover, self.size);
            self.prev = self.pos;
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
        if self.husk_max > 0 {
            if self.stagger > 0 {
                // staggered, the Husk broken: it sinks a little, fires
                // nothing, and takes double; then the Husk is whole again
                self.stagger -= 1;
                self.lunge = Lunge::None;
                self.state = State::Rest(secs(1, 2));
                self.pos.y = fx::approach(self.pos.y, self.home.y, ONE / 20);
                if self.stagger == 0 {
                    self.husk = self.husk_max;
                }
                return deeds;
            }
            self.unhit += 1;
            if self.unhit > d.husk_regen && self.husk < self.husk_max {
                self.husk = (self.husk + self.husk_max / HUSK_REGROW_TICKS).min(self.husk_max);
            }
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
        if matches!(
            d.kind,
            Kind::Boss | Kind::Double | Kind::God | Kind::Cat | Kind::Oak
        ) || d.phases.len() > 1
        {
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
        // an Etching looked at is a statue: it neither moves nor fires
        // (a cat in the dark only stops moving)
        if d.kind == Kind::Beast && d.gait == Gait::Unseen && self.seen {
            return deeds;
        }
        // a Toon waits for the beat, so it fires as it lands
        if d.gait == Gait::Bounce {
            if let (State::Rest(t), Some(a)) = (self.state, self.current_attack(c)) {
                if t <= 1 && (self.beat + a.windup) % BOUNCE_TICKS != 0 {
                    self.state = State::Rest(1);
                    return deeds;
                }
            }
        }

        // a percher holds its fire while it flies
        if self.gait(d) == Gait::Perch && self.flying > 0 {
            if let State::Rest(t) = self.state {
                self.state = State::Rest(t.max(2));
            }
        }
        if d.kind != Kind::Agent && matches!(self.state, State::Rest(t) if t <= 1) {
            self.pass_over(c);
        }
        let Some(attack) = self.current_attack(c).cloned() else {
            return deeds;
        };
        let pattern: &Pattern = &c.patterns[self.pattern_of(c, attack.pattern)];
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
                self.lunge = attack.moves;
                self.pulled = 0;
                if attack.moves != Lunge::None {
                    self.anchor = self.lunge_spot(level, target, attack.moves);
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
                deeds.stop = attack.stop;
                State::Casting(0)
            }
            // a hook's cast waits until the pull has brought it beside you
            State::Casting(0) if self.hooking() => {
                self.pulled += 1;
                State::Casting(0)
            }
            State::Casting(t) => {
                // in a stopped moment it moves round you between volleys
                if attack.stop > 0
                    && t > 0
                    && pattern.volleys.iter().any(|v| {
                        t >= v.delay
                            && v.every > 0
                            && (t - v.delay) % v.every == 0
                            && (t - v.delay) / v.every < v.repeat
                    })
                {
                    self.reposition(level, target, t);
                }
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
                    self.lunge = Lunge::None;
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
    fn cast(
        &self,
        c: &Content,
        pattern: &Pattern,
        t: i32,
        from: &[V3],
        target: V3,
        me: usize,
        level: &Level,
        deeds: &mut Deeds,
    ) {
        cast_volleys(c, pattern, t, from, target, me, level, &mut deeds.salvo);
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
        let pattern = &c.patterns[self.pattern_of(c, attack.pattern)];
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

    /// Somewhere else round `target` at this enemy's range, on the floor
    /// there: where a time stopper steps to between volleys.
    fn reposition(&mut self, level: &Level, target: V3, t: i32) {
        let d_range = int(9);
        for k in 0..6 {
            let a = (self.age * 37 + t * 211 + k * 97) % 360;
            let flat = target.add(facing(fx::deg(a), 0).scale(d_range));
            let Some(ground) = level.floor_below(flat.x, target.y + int(3), flat.z) else {
                continue;
            };
            let at = v3(flat.x, ground + self.pos.y - self.floor(level), flat.z);
            if !level.sphere_hits(at, ONE / 2) {
                self.pos = at;
                self.prev = at;
                self.vel = V3::ZERO;
                return;
            }
        }
    }

    /// Passes over the attacks taken once in so many that are not due:
    /// each is taken the first time it comes up, then every `once_in`th.
    fn pass_over(&mut self, c: &Content) {
        let d = &c.enemies[self.def];
        let n = d.phases[self.phase.min(d.phases.len() - 1)].attacks.len();
        for _ in 0..n {
            let Some(a) = self.current_attack(c) else {
                return;
            };
            if a.once_in <= 1 {
                return;
            }
            let due = self.passes % a.once_in == 0;
            self.passes += 1;
            if due {
                return;
            }
            self.attack += 1;
        }
    }

    /// Where a lunge goes, on the floor: a hook beside you, a pound where
    /// you stand.
    fn lunge_spot(&self, level: &Level, target: V3, lunge: Lunge) -> V3 {
        let flat = v3(target.x - self.pos.x, 0, target.z - self.pos.z).norm();
        let side = v3(-flat.z, 0, flat.x).scale(self.orbit * ONE);
        let tries = match lunge {
            Lunge::Hook => vec![
                target.add(side.scale(LUNGE_HOOK_BESIDE)),
                target.sub(side.scale(LUNGE_HOOK_BESIDE)),
                target.sub(flat.scale(LUNGE_HOOK_BESIDE)),
            ],
            _ => vec![target],
        };
        tries
            .into_iter()
            .find_map(|p| {
                let ground = level.floor_below(p.x, target.y + int(2), p.z)?;
                let at = v3(p.x, ground, p.z);
                (!level.sphere_hits(at.add(v3(0, ONE, 0)), ONE / 2)).then_some(at)
            })
            .unwrap_or(self.pos)
    }

    /// Where its bite is: its body, or its head when it lunges.
    pub fn head(&self) -> V3 {
        self.pos.add(self.bite_dir.scale(self.reach))
    }

    /// A beast moves by its gait.
    fn beast(&mut self, d: &EnemyDef, level: &Level, target: V3) {
        let flat = v3(target.x - self.pos.x, 0, target.z - self.pos.z);
        let dist = flat.len();
        let ground = |p: V3| level.floor_below(p.x, p.y + ONE, p.z);
        // what it collides with: its body, kept clear of the floor it sits on
        let body = d.radius.min(d.hover * 9 / 10).max(ONE / 10);
        let gait = self.gait(d);
        match gait {
            Gait::Bound | Gait::Hop => {
                let hop = gait == Gait::Hop;
                if self.leap > 0 {
                    // in the air: on along its arc
                    self.leap -= 1;
                    self.vel.y -= BEAST_GRAVITY;
                    let next = self.pos.add(self.vel);
                    if level.sphere_hits(next, body) {
                        self.vel = v3(0, self.vel.y.min(0), 0);
                    } else {
                        self.pos = next;
                    }
                    let floor = ground(self.pos).unwrap_or(self.pos.y - d.hover);
                    if self.leap == 0 || (self.vel.y < 0 && self.pos.y <= floor + d.hover) {
                        self.leap = 0;
                        self.pos.y = floor + d.hover;
                        self.vel = V3::ZERO;
                    }
                    return;
                }
                let (every, ticks) = if hop {
                    (HOP_EVERY, HOP_TICKS)
                } else {
                    (BOUND_EVERY, BOUND_TICKS)
                };
                if (self.age + self.def as i32 * 7) % every == 0 {
                    // where to: a hop goes a short way, toward its range;
                    // a bound goes round you to its range
                    let spot = if hop {
                        let way = if dist > d.range {
                            flat.norm()
                        } else {
                            flat.norm().neg()
                        };
                        let side = v3(-way.z, 0, way.x).scale(self.orbit * ONE / 2);
                        self.pos.add(way.add(side).norm().scale(HOP_LENGTH))
                    } else {
                        let (yaw, _) = yaw_pitch_of(self.pos.sub(target));
                        let a = yaw + fx::deg(BOUND_TURN_DEG) * self.orbit;
                        target.add(facing(a, 0).scale(d.range))
                    };
                    let fits = ground(v3(spot.x, self.pos.y + int(3), spot.z))
                        .map(|f| v3(spot.x, f + d.hover, spot.z))
                        .filter(|p| !level.sphere_hits(*p, body));
                    match fits {
                        Some(to) => {
                            let t = ticks * ONE;
                            self.vel = v3(
                                fx::div(to.x - self.pos.x, t),
                                BEAST_GRAVITY * ticks / 2 + fx::div(to.y - self.pos.y, t),
                                fx::div(to.z - self.pos.z, t),
                            );
                            self.leap = ticks;
                        }
                        None => self.orbit = -self.orbit,
                    }
                }
                if let Some(f) = ground(self.pos) {
                    self.pos.y = fx::approach(self.pos.y, f + d.hover, ONE / 6);
                }
            }
            Gait::Wheel => {
                // round you at its range and height; closer and low as it
                // winds up, a dive
                let diving = matches!(self.state, State::Windup(_) | State::Casting(_));
                let r = if diving {
                    d.range * WHEEL_DIVE_PCT / 100
                } else {
                    d.range
                };
                let (yaw, _) = yaw_pitch_of(self.pos.sub(target));
                let step = fx::div(d.speed, r.max(ONE)) * self.orbit;
                let round = target.add(facing(yaw + step, 0).scale(r));
                let floor = ground(round).unwrap_or(target.y - PLAYER_CHEST);
                let y = if diving {
                    target.y + WHEEL_DIVE_OVER
                } else {
                    floor.max(target.y - PLAYER_CHEST) + d.hover
                };
                let want = v3(round.x, y, round.z);
                let wish = want.sub(self.pos).clamp_len(d.speed * 2);
                self.vel = self.vel.lerp(wish, ONE / 8);
                let next = self.pos.add(self.vel);
                if level.sphere_hits(next, body) {
                    self.orbit = -self.orbit;
                    self.pos.y += d.speed;
                } else {
                    self.pos = next;
                }
            }
            Gait::Root => {
                self.vel = V3::ZERO;
                if d.bite <= 0 {
                    return;
                }
                if self.bite_t > 0 {
                    self.bite_t -= 1;
                }
                let since = BITE_EVERY - self.bite_t;
                // out over BITE_OUT ticks, held as long, and back
                self.reach = if since < BITE_OUT {
                    d.bite * since / BITE_OUT
                } else if since < BITE_OUT * 2 {
                    d.bite
                } else if since < BITE_OUT * 3 {
                    d.bite * (BITE_OUT * 3 - since) / BITE_OUT
                } else {
                    0
                };
                let chest = target.sub(self.pos);
                if self.bite_t == 0 && chest.len() < d.bite + d.radius + BITE_MARGIN {
                    self.bite_t = BITE_EVERY;
                    self.bite_dir = chest.norm();
                }
            }
            Gait::Surge => {
                let a = self.age * fx::TURN / SURGE_PERIOD * self.orbit;
                let round = target.add(v3(fx::mul(fx::cos(a), d.range), 0, fx::mul(fx::sin(a), d.range)));
                let wish = v3(round.x - self.pos.x, 0, round.z - self.pos.z).clamp_len(d.speed);
                self.vel = self.vel.lerp(wish, SURGE_TURN);
                let v = self.vel;
                let tries = [v, v3(v.x, 0, 0), v3(0, 0, v.z)];
                match tries.iter().map(|t| self.pos.add(*t)).find(|p| {
                    !level.sphere_hits(*p, body) && ground(*p).is_some_and(|f| f > self.pos.y - d.hover - ONE)
                }) {
                    Some(p) => self.pos = p,
                    None => self.orbit = -self.orbit,
                }
                if let Some(f) = ground(self.pos) {
                    self.pos.y = fx::approach(self.pos.y, f + d.hover, ONE / 6);
                }
            }
            Gait::Perch => self.perching(level, target),
            Gait::March => {
                // in the air at its height over you, a step at a time
                let floor = ground(self.pos).unwrap_or(target.y - PLAYER_CHEST);
                let y = floor.max(target.y - PLAYER_CHEST) + d.hover;
                if self.leap > 0 {
                    self.leap -= 1;
                    let next = self.pos.add(self.vel);
                    if level.sphere_hits(next, body) {
                        self.leap = 0;
                        self.orbit = -self.orbit;
                    } else {
                        self.pos = next;
                    }
                } else if (self.age + self.def as i32 * 5) % MARCH_EVERY == 0 {
                    self.moves += 1;
                    let way = flat.norm();
                    let step = if self.moves % MARCH_ACROSS == 0 {
                        // a step nearer (or back, too near), and the other way
                        self.orbit = -self.orbit;
                        if dist > d.range {
                            way
                        } else {
                            way.neg()
                        }
                    } else {
                        v3(-way.z * self.orbit, 0, way.x * self.orbit)
                    };
                    self.vel = step.scale(MARCH_STEP / MARCH_GLIDE);
                    self.leap = MARCH_GLIDE;
                }
                self.pos.y = fx::approach(self.pos.y, y, ONE / 10);
            }
            Gait::Unseen => {
                // still while you look; while you do not, straight at you
                self.vel = V3::ZERO;
                if self.seen || dist < d.radius {
                    return;
                }
                let wish = flat.norm().scale(d.speed);
                let tries = [wish, v3(wish.x, 0, 0), v3(0, 0, wish.z)];
                if let Some(p) = tries
                    .iter()
                    .map(|t| self.pos.add(*t))
                    .find(|p| !level.sphere_hits(*p, body))
                {
                    self.vel = p.sub(self.pos);
                    self.pos = p;
                }
                if let Some(f) = ground(self.pos) {
                    self.pos.y = fx::approach(self.pos.y, f + d.hover, ONE / 6);
                }
            }
            Gait::Charge => {
                // tank controls: turn on the spot, then straight on
                if self.leap > 0 {
                    self.leap -= 1;
                    let next = self.pos.add(self.bite_dir.scale(d.speed));
                    let floor_ahead = ground(next).is_some_and(|f| f > self.pos.y - d.hover - int(2));
                    if level.sphere_hits(next, body) || !floor_ahead {
                        self.leap = 0;
                    } else {
                        self.pos = next;
                    }
                    if self.leap == 0 {
                        self.perched = CHARGE_REST;
                    }
                } else if self.perched > 0 {
                    self.perched -= 1;
                } else {
                    let (want, _) = yaw_pitch_of(flat);
                    let off = fx::wrap(want - self.yaw);
                    if off.abs() <= CHARGE_AIM {
                        self.leap = CHARGE_TICKS;
                        self.bite_dir = facing(self.yaw, 0);
                    } else {
                        self.yaw = fx::wrap(self.yaw + off.clamp(-CHARGE_TURN, CHARGE_TURN));
                    }
                }
                if let Some(f) = ground(self.pos) {
                    self.pos.y = fx::approach(self.pos.y, f + d.hover, ONE / 6);
                }
            }
            Gait::Bounce => {
                // up and down on the shared beat; across only in the air,
                // toward its range from you
                let t = self.beat % BOUNCE_TICKS;
                let up = (4 * t as i64 * (BOUNCE_TICKS - t) as i64 * BOUNCE_HEIGHT as i64
                    / (BOUNCE_TICKS as i64 * BOUNCE_TICKS as i64)) as i32;
                let floor =
                    ground(v3(self.pos.x, self.pos.y + int(2), self.pos.z)).unwrap_or(self.pos.y - d.hover);
                if t > 0 {
                    let way = if dist > d.range {
                        flat.norm()
                    } else {
                        flat.norm().neg()
                    };
                    let side = v3(-way.z, 0, way.x).scale(self.orbit * ONE / 2);
                    let wish = way.add(side).norm().scale(d.speed);
                    let next = v3(self.pos.x + wish.x, self.pos.y, self.pos.z + wish.z);
                    if !level.sphere_hits(next, body) && ground(next).is_some() {
                        self.pos = next;
                    } else {
                        self.orbit = -self.orbit;
                    }
                }
                self.pos.y = floor + d.hover + up;
            }
            Gait::Echo => {
                // where you were: it walks the very path
                if let Some(to) = self.trail {
                    let step = to.sub(self.pos).clamp_len(d.speed);
                    self.vel = step;
                    self.pos = self.pos.add(step);
                }
            }
            Gait::None => {}
        }
    }

    /// Its wings are spread for a swoop that is coming.
    pub fn swoop_tell(&self) -> bool {
        self.flying == 0 && (self.moves + 1) % SWOOP_EVERY == 0 && self.perched > PERCH_HOLD - SWOOP_TELL
    }

    /// A percher: on a perch, then off to another, now and then by way of
    /// a swoop through where you stand.
    fn perching(&mut self, level: &Level, target: V3) {
        if self.perches.is_empty() {
            self.perches = perches(self.home, level);
            let near = (0..self.perches.len())
                .min_by_key(|&k| self.perches[k].dist(self.pos))
                .unwrap_or(0);
            self.perch = near;
            self.anchor = self.perches.get(near).copied().unwrap_or(self.pos);
            self.fly_to(self.anchor, PERCH_GLIDE);
        }
        if self.flying > 0 {
            self.flying -= 1;
            self.pos = self.pos.add(self.vel);
            if self.flying == 0 {
                if self.diving {
                    // up out of the swoop, to its next perch
                    self.diving = false;
                    self.fly_to(self.perches[self.perch], PERCH_GLIDE);
                } else {
                    self.pos = self.anchor;
                    self.vel = V3::ZERO;
                    self.perched = 0;
                }
            }
            return;
        }
        self.perched += 1;
        if self.perched < PERCH_HOLD {
            return;
        }
        self.moves += 1;
        let n = self.perches.len().max(1);
        self.perch = (self.perch + 1 + (self.age as usize / 7) % (n - 1).max(1)) % n;
        if self.moves % SWOOP_EVERY == 0 {
            // through where you stand, and on past
            let way = target.sub(self.pos).norm();
            let mut to = target.add(way.scale(SWOOP_PAST));
            if let Some((at, reach)) = self.leash {
                // no further than its chain lets it
                let off = v3(to.x - at.x, 0, to.z - at.z).clamp_len(reach);
                to = v3(at.x + off.x, to.y, at.z + off.z);
            }
            self.diving = true;
            self.fly_to(to, SWOOP_SPEED);
        } else {
            self.fly_to(self.perches[self.perch], PERCH_GLIDE);
        }
    }

    fn fly_to(&mut self, to: V3, speed: i32) {
        let way = to.sub(self.pos);
        let ticks = (fx::div(way.len(), speed.max(1)) / ONE).max(1);
        self.anchor = to;
        self.flying = ticks;
        self.vel = v3(way.x / ticks, way.y / ticks, way.z / ticks);
    }

    /// Still pulled along its hook toward the anchor.
    fn hooking(&self) -> bool {
        let to = v3(self.anchor.x - self.pos.x, 0, self.anchor.z - self.pos.z);
        self.lunge == Lunge::Hook && to.len() > LUNGE_HOOK_ARRIVE && self.pulled < LUNGE_HOOK_MAX
    }

    /// A step of a lunge: true while the lunge moves the body, so its
    /// skirmish waits.
    fn lunge_step(&mut self, d: &EnemyDef, level: &Level) -> bool {
        let to = v3(self.anchor.x - self.pos.x, 0, self.anchor.z - self.pos.z);
        let (step, height, fall) = match (self.lunge, self.state) {
            (Lunge::Hook, State::Windup(_)) => (V3::ZERO, 0, ONE / 4),
            (Lunge::Hook, State::Casting(_)) if self.hooking() => {
                (to.clamp_len(LUNGE_HOOK_SPEED), 0, ONE / 4)
            }
            // over where you were by the end of its windup
            (Lunge::Pound, State::Windup(t)) => (
                v3(to.x / t.max(1), 0, to.z / t.max(1)),
                LUNGE_POUND_HEIGHT,
                ONE / 4,
            ),
            (Lunge::Pound, State::Casting(_)) => (V3::ZERO, 0, LUNGE_POUND_DROP),
            _ => return false,
        };
        // over what is in the way, never off the edge
        let next = self.pos.add(step);
        let clear = (0..8)
            .map(|k| next.add(v3(0, ONE / 2 * k, 0)))
            .find(|p| !level.sphere_hits(*p, d.radius));
        if let Some(p) = clear.filter(|p| level.floor_below(p.x, p.y + ONE, p.z).is_some()) {
            self.pos = p;
        }
        self.vel = step;
        self.boost = 0;
        self.lift = height;
        let ground = self.floor(level);
        self.pos.y = fx::approach(self.pos.y, ground + d.hover + height, fall);
        true
    }

    fn floor(&self, level: &Level) -> i32 {
        level.floor_below(self.pos.x, self.pos.y, self.pos.z).unwrap_or(0)
    }

    /// A mech or a double skirmishes: it circles you at its range on its
    /// legs, bursts sideways now and then (a quick boost, a dash), and now
    /// and then rises (a mech's ascent, a double's jump) and comes down.
    fn skirmish(&mut self, d: &EnemyDef, level: &Level, target: V3) {
        let to = v3(target.x - self.pos.x, 0, target.z - self.pos.z);
        let dist = to.len_xz();
        let towards = to.norm();
        let side = v3(-towards.z, 0, towards.x).scale(self.orbit * ONE);
        if self.lunge_step(d, level) {
            return;
        }
        let mech = d.kind == Kind::Mech;
        let (boost_every, boost_len, boost_speed) = if mech {
            (secs(3, 1), secs(1, 2), per_tick(22, 1))
        } else {
            (secs(22, 10), DASH_TICKS, DASH_SPEED)
        };
        let (rise_every, rise, rise_ticks) = if mech {
            (secs(9, 1), int(7), secs(4, 1))
        } else {
            (secs(4, 1), ONE * 15 / 10, secs(7, 10))
        };
        self.air += 1;
        if self.boost > 0 {
            self.boost -= 1;
        } else if (self.age + self.def as i32 * 13) % boost_every == 0 {
            self.boost = boost_len;
            self.orbit = -self.orbit;
        }
        let radial = if dist > d.range + int(3) {
            towards
        } else if dist < d.range - int(3) {
            towards.neg()
        } else {
            V3::ZERO
        };
        let wish = if self.boost > 0 {
            side.norm().scale(boost_speed)
        } else {
            radial.add(side).norm().scale(d.speed)
        };
        self.vel = if self.boost > 0 {
            wish
        } else {
            self.vel.lerp(wish, ONE / 10)
        };
        // the rise: up, a hold, and down again
        let phase = self.air % rise_every;
        let want_lift = if phase < rise_ticks {
            fx::mul(
                rise,
                fx::sin(fx::mul(fx::div(phase * ONE, rise_ticks * ONE), fx::HALF)).max(0),
            )
        } else {
            0
        };
        self.lift = fx::approach(self.lift, want_lift, rise / 20);
        let v = self.vel;
        let tries = [v, v3(v.x, 0, 0), v3(0, 0, v.z)];
        if let Some(t) = tries
            .iter()
            .find(|t| !level.sphere_hits(self.pos.add(**t), d.radius))
        {
            self.pos = self.pos.add(*t);
        } else {
            self.orbit = -self.orbit;
            self.boost = 0;
        }
        let ground = if d.aloft {
            // over the crown and its gaps alike, kept near its home
            let off = v3(self.pos.x - self.home.x, 0, self.pos.z - self.home.z);
            if off.len_xz() > ALOFT_LEASH {
                let back = off.norm().scale(ALOFT_LEASH);
                self.pos.x = self.home.x + back.x;
                self.pos.z = self.home.z + back.z;
            }
            self.home.y
        } else {
            level
                .floor_below(self.pos.x, self.pos.y + ONE, self.pos.z)
                .unwrap_or(0)
        };
        self.pos.y = fx::approach(self.pos.y, ground + d.hover + self.lift, ONE / 4);
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
        let rate = if d.role == Role::Shield {
            SHIELD_TURN
        } else {
            AGENT_TURN
        };
        let turn = fx::wrap(want - self.yaw).clamp(-rate, rate);
        self.yaw = fx::wrap(self.yaw + turn);
    }

    fn locomote(&mut self, d: &EnemyDef, level: &Level, target: V3, to: V3) {
        let flat = v3(to.x, 0, to.z);
        let dist = flat.len();
        let towards = flat.norm();
        let side = v3(-towards.z, 0, towards.x).scale(self.orbit * ONE);
        match d.kind {
            Kind::Turret | Kind::Tank | Kind::Oak => {}
            Kind::God => {
                // vast and slow, over its home, heaving
                let bob = v3(
                    fx::sin(self.age * fx::TURN / secs(11, 1)) * 2,
                    fx::sin(self.age * fx::TURN / secs(7, 1)) * 2,
                    0,
                );
                let want = self.home.add(bob);
                self.pos = self.pos.add(want.sub(self.pos).clamp_len(ONE / 30));
                let _ = (dist, side);
            }
            Kind::Mech | Kind::Double => self.skirmish(d, level, target),
            Kind::Cat => {
                // a pounce carries it over you; otherwise its phase's gait
                if !self.lunge_step(d, level) {
                    self.beast(d, level, target);
                }
                let _ = (dist, side);
            }
            Kind::Agent => self.walk(d, level, target),
            Kind::Beast => self.beast(d, level, target),
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

/// The volleys of a pattern that fire on tick `t` of its cast, from each of
/// `from`, at `target`, owned by enemy `me`.
#[allow(clippy::too_many_arguments)]
pub fn cast_volleys(
    c: &Content,
    pattern: &Pattern,
    t: i32,
    from: &[V3],
    target: V3,
    me: usize,
    level: &Level,
    salvo: &mut Salvo,
) {
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
                salvo.orbs.extend(s.orbs);
                salvo.waves.extend(s.waves);
            }
        }
    }
}

/// A percher's perches: round the arena's centre, over floor, high up.
pub fn perches(home: V3, level: &Level) -> Vec<V3> {
    (0..PERCHES)
        .filter_map(|k| {
            let a = fx::TURN / PERCHES * k + fx::TURN / PERCHES / 2;
            [PERCH_RING, PERCH_RING * 3 / 4, PERCH_RING / 2]
                .iter()
                .find_map(|&r| {
                    let p = home.add(facing(a, 0).scale(r));
                    let f = level.floor_below(p.x, home.y + int(4), p.z)?;
                    Some(v3(p.x, f + PERCH_HEIGHT, p.z))
                })
        })
        .collect()
}

/// The damage a body takes from touching an enemy, if it does.
pub fn contact(e: &Enemy, d: &EnemyDef, feet: V3) -> bool {
    let chest = feet.add(v3(0, PLAYER_CHEST, 0));
    let reach = d.radius + PLAYER_RADIUS + ONE / 4;
    e.hurtable()
        && (orbs::within(e.pos, chest, reach) || (e.reach > 0 && orbs::within(e.head(), chest, reach)))
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

    /// A beast of `id` on the open floor, `range` metres from `you`, past
    /// its spawning; `you` is the chest it moves round.
    fn beast_at(c: &Content, id: &str, you: V3) -> Enemy {
        let i = c.enemy(id).unwrap();
        let d = &c.enemies[i];
        let mut e = Enemy::new(i, d, v3(you.x + d.range, d.hover, you.z), 1);
        e.state = State::Rest(1_000_000);
        e
    }

    #[test]
    fn a_hart_bounds_in_a_high_arc_round_you_and_lands_at_its_range() {
        let c = content();
        let l = floor();
        let you = v3(0, int(1), 0);
        let mut e = beast_at(&c, "hart", you);
        let d = &c.enemies[e.def];
        let (start, _) = yaw_pitch_of(e.pos.sub(you));
        let mut top = 0;
        let mut leapt = false;
        for _ in 0..BOUND_EVERY + BOUND_TICKS + 5 {
            e.step(&c, &l, you, 0);
            leapt |= e.leap > 0;
            top = top.max(e.pos.y - d.hover);
        }
        assert!(leapt && e.leap == 0, "it bounded and landed");
        assert!(top > ONE * 3 / 2, "high: {top}");
        let (now, _) = yaw_pitch_of(e.pos.sub(you));
        assert!(
            fx::wrap(now - start).abs() > fx::deg(BOUND_TURN_DEG) / 2,
            "further round you"
        );
        let r = v3(e.pos.x - you.x, 0, e.pos.z - you.z).len();
        assert!((r - d.range).abs() < ONE, "at its range: {r}");
        assert!((e.pos.y - d.hover).abs() < ONE / 10, "on the floor");
    }

    #[test]
    fn a_toad_hops_a_short_way_toward_its_range() {
        let c = content();
        let l = floor();
        let you = v3(0, int(1), 0);
        let mut e = beast_at(&c, "toad", you);
        let d = &c.enemies[e.def];
        e.pos.x = you.x + d.range + int(8);
        let far = e.pos.x;
        let mut hops = 0;
        let mut top = 0;
        let mut from = e.pos;
        for _ in 0..HOP_EVERY * 3 {
            let before = e.leap;
            let at = e.pos;
            e.step(&c, &l, you, 0);
            if before == 0 && e.leap > 0 {
                hops += 1;
                from = at;
            }
            if before > 0 && e.leap == 0 {
                let len = v3(e.pos.x - from.x, 0, e.pos.z - from.z).len();
                assert!((len - HOP_LENGTH).abs() < ONE / 2, "each hop {HOP_LENGTH}: {len}");
            }
            top = top.max(e.pos.y - d.hover);
        }
        assert_eq!(hops, 3, "a hop a second");
        assert!(top < ONE, "low hops: {top}");
        let moved = far - e.pos.x;
        assert!(
            moved > HOP_LENGTH * 2 && moved < HOP_LENGTH * 4,
            "toward you, a short way each: {moved}"
        );
    }

    #[test]
    fn a_rook_circles_high_over_you_and_dives_as_it_winds_up() {
        let c = content();
        let l = floor();
        let you = v3(0, int(1), 0);
        let mut e = beast_at(&c, "rook", you);
        let d = &c.enemies[e.def];
        let (start, _) = yaw_pitch_of(e.pos.sub(you));
        for _ in 0..secs(3, 1) {
            e.step(&c, &l, you, 0);
        }
        let (now, _) = yaw_pitch_of(e.pos.sub(you));
        assert!(fx::wrap(now - start).abs() > fx::deg(20), "it wheels round you");
        assert!(e.pos.y > d.hover - ONE, "high: {}", e.pos.y);
        e.state = State::Windup(1_000_000);
        for _ in 0..secs(2, 1) {
            e.step(&c, &l, you, 0);
        }
        assert!(e.pos.y < you.y + WHEEL_DIVE_OVER + ONE, "it dives: {}", e.pos.y);
        let r = v3(e.pos.x, 0, e.pos.z).len();
        assert!(r < d.range * (WHEEL_DIVE_PCT + 15) / 100, "and closes: {r}");
    }

    #[test]
    fn a_gaper_stays_rooted_and_bites_you_only_within_its_reach() {
        let c = content();
        let l = floor();
        let i = c.enemy("gaper").unwrap();
        let d = &c.enemies[i];
        let mut e = Enemy::new(i, d, v3(0, d.hover, 0), 1);
        e.state = State::Rest(1_000_000);
        let root = e.pos;
        // out of reach: nothing
        let far = v3(d.bite + d.radius + int(3), 0, 0);
        for _ in 0..secs(2, 1) {
            e.step(&c, &l, far.add(v3(0, PLAYER_CHEST, 0)), 0);
            assert!(!contact(&e, d, far));
        }
        assert_eq!(e.reach, 0);
        // just within: it lunges and reaches you past its body
        let near = v3(d.bite + d.radius, 0, 0);
        let mut bit = false;
        for _ in 0..BITE_OUT * 3 {
            e.step(&c, &l, near.add(v3(0, PLAYER_CHEST, 0)), 0);
            bit |= contact(&e, d, near);
        }
        assert!(bit, "it bit");
        assert_eq!(e.pos, root, "and never moved");
    }

    #[test]
    fn a_freshet_rushes_round_you_along_the_ground() {
        let c = content();
        let l = floor();
        let you = v3(0, int(1), 0);
        let mut e = beast_at(&c, "freshet", you);
        let d = &c.enemies[e.def];
        let mut path = 0;
        let mut angles = Vec::new();
        for t in 0..SURGE_PERIOD {
            let before = e.pos;
            e.step(&c, &l, you, 0);
            path += e.pos.sub(before).len();
            assert!((e.pos.y - d.hover).abs() < ONE / 4, "on the ground");
            if t % (SURGE_PERIOD / 4) == 0 {
                angles.push(yaw_pitch_of(e.pos.sub(you)).0);
            }
        }
        assert!(path > d.range * 3, "it rushes: {path}");
        let quarters = angles
            .windows(2)
            .filter(|w| fx::wrap(w[1] - w[0]).abs() > fx::deg(30))
            .count();
        assert!(quarters >= 2, "round you: {angles:?}");
    }

    #[test]
    fn the_great_horned_perches_high_flies_between_perches_and_swoops_through_you() {
        let c = content();
        let l = floor();
        let you = v3(0, PLAYER_CHEST, 0);
        let i = c.enemy("great_horned").unwrap();
        let d = &c.enemies[i];
        let mut e = Enemy::new(i, d, v3(int(10), int(6), 0), 1);
        e.home = V3::ZERO;
        e.state = State::Rest(1_000_000);
        let mut sat_on = Vec::new();
        let mut closest = i32::MAX;
        let mut swoops = 0;
        for _ in 0..PERCH_HOLD * 8 {
            let was = e.flying;
            let diving = e.diving;
            e.step(&c, &l, you, 0);
            if was > 0 && e.flying == 0 && !e.diving {
                sat_on.push(e.perch);
                assert!(e.pos.y >= PERCH_HEIGHT - ONE / 10, "perched high: {}", e.pos.y);
            }
            if diving {
                closest = closest.min(e.pos.dist(you));
            }
            swoops += i32::from(!diving && e.diving);
        }
        assert_eq!(e.perches.len() as i32, PERCHES);
        sat_on.dedup();
        assert!(sat_on.len() >= 4, "from perch to perch: {sat_on:?}");
        assert!(swoops >= 2, "it swoops: {swoops}");
        assert!(closest < d.radius, "through where you stand: {closest}");
    }

    #[test]
    fn the_great_horned_holds_its_fire_while_it_flies() {
        let c = content();
        let l = floor();
        let you = v3(0, PLAYER_CHEST, 0);
        let i = c.enemy("great_horned").unwrap();
        let mut e = Enemy::new(i, &c.enemies[i], v3(int(10), int(6), 0), 1);
        e.home = V3::ZERO;
        e.state = State::Rest(1);
        let mut began_flying = 0;
        for _ in 0..PERCH_HOLD * 6 {
            let before = e.state;
            e.step(&c, &l, you, 0);
            if e.flying > 0 && matches!(before, State::Rest(_)) && matches!(e.state, State::Windup(_)) {
                began_flying += 1;
            }
        }
        assert_eq!(began_flying, 0, "no attack begins in flight");
    }

    #[test]
    fn a_glyph_marches_sideways_a_step_at_a_time_then_steps_toward_you_and_turns_back() {
        let c = content();
        let l = floor();
        let you = v3(0, int(1), 0);
        let i = c.enemy("glyph").unwrap();
        let d = &c.enemies[i];
        // far off, so its step toward you is a step nearer
        let mut e = Enemy::new(i, d, v3(d.range + int(8), d.hover, 0), 1);
        e.state = State::Rest(1_000_000);
        let mut moves = Vec::new();
        let mut still = 0;
        for _ in 0..MARCH_EVERY * (MARCH_ACROSS * 2 + 1) {
            let before = e.pos;
            e.step(&c, &l, you, 0);
            let m = e.pos.sub(before);
            if m.len_xz() < ONE / 100 {
                still += 1;
            } else {
                moves.push(m);
            }
        }
        assert!(still > moves.len() as i32, "it holds still between steps");
        // steps across (along z, its side of you), one toward you (along
        // x), then back across the other way
        let across: Vec<i32> = moves.iter().map(|m| m.z).collect();
        assert!(across.iter().any(|z| *z > ONE / 20), "{across:?}");
        assert!(across.iter().any(|z| *z < -ONE / 20), "and back: {across:?}");
        assert!(
            moves.iter().any(|m| m.x < -ONE / 20 && m.z.abs() < ONE / 50),
            "a step toward you"
        );
    }

    #[test]
    fn an_etching_moves_and_fires_only_while_you_are_not_looking_at_it() {
        let c = content();
        let l = floor();
        let you = v3(0, int(1), 0);
        let i = c.enemy("etching").unwrap();
        let d = &c.enemies[i];
        let mut e = Enemy::new(i, d, v3(int(15), d.hover, 0), 1);
        e.state = State::Rest(1);
        e.seen = true;
        let start = e.pos;
        let mut fired = 0;
        for _ in 0..secs(3, 1) {
            fired += e.step(&c, &l, you, 0).salvo.orbs.len();
        }
        assert_eq!(e.pos, start, "a statue while you look");
        assert_eq!(fired, 0, "and silent");
        e.seen = false;
        for _ in 0..secs(1, 1) {
            fired += e.step(&c, &l, you, 0).salvo.orbs.len();
        }
        assert!(e.pos.x < start.x - int(5), "looking away, it comes: {:?}", e.pos);
        for _ in 0..secs(3, 1) {
            fired += e.step(&c, &l, you, 0).salvo.orbs.len();
        }
        assert!(fired > 0, "and fires");
    }

    #[test]
    fn a_render_turns_on_the_spot_and_then_charges_straight_whatever_you_do() {
        let c = content();
        let l = floor();
        let i = c.enemy("render").unwrap();
        let d = &c.enemies[i];
        let mut e = Enemy::new(i, d, v3(0, d.hover, 0), 1);
        e.state = State::Rest(1_000_000);
        // you are behind it: it turns first, standing where it is
        let you = v3(0, int(1), int(20));
        let start = e.pos;
        let mut turned = 0;
        while e.leap == 0 && turned < secs(3, 1) {
            e.step(&c, &l, you, 0);
            turned += 1;
            if e.leap == 0 {
                assert!(e.pos.sub(start).len_xz() < ONE / 50, "it turns on the spot");
            }
        }
        assert!(turned > secs(1, 2), "a half turn takes a while: {turned}");
        // charging, you step aside: it goes straight on
        let dir = e.bite_dir;
        let aside = v3(int(10), int(1), int(10));
        let from = e.pos;
        for _ in 0..secs(1, 2) {
            e.step(&c, &l, aside, 0);
        }
        let went = e.pos.sub(from);
        assert!(went.len_xz() > int(5), "it charges: {went:?}");
        let off = went.norm().dot(dir);
        assert!(off > ONE * 99 / 100, "straight on, not after you");
    }

    #[test]
    fn every_toon_bounces_on_the_one_shared_beat() {
        let c = content();
        let l = floor();
        let you = v3(0, int(1), 0);
        let i = c.enemy("toon").unwrap();
        let d = &c.enemies[i];
        let mut a = Enemy::new(i, d, v3(int(10), d.hover, 0), 1);
        let mut b = Enemy::new(i, d, v3(-int(10), d.hover, int(4)), -1);
        a.age = 37;
        for e in [&mut a, &mut b] {
            e.state = State::Rest(1_000_000);
        }
        let mut lowest = Vec::new();
        for t in 1..BOUNCE_TICKS * 3 {
            for e in [&mut a, &mut b] {
                e.beat = t;
                e.step(&c, &l, you, 0);
            }
            assert_eq!(a.pos.y, b.pos.y, "together, whatever their own clocks");
            if a.pos.y <= d.hover + ONE / 50 {
                lowest.push(t % BOUNCE_TICKS);
            }
        }
        assert!(
            !lowest.is_empty() && lowest.iter().all(|t| *t == 0),
            "they land on the beat: {lowest:?}"
        );
        let high = (0..BOUNCE_TICKS)
            .map(|t| {
                let mut e = Enemy::new(i, d, v3(0, d.hover, int(9)), 1);
                e.state = State::Rest(1_000_000);
                e.beat = t;
                e.step(&c, &l, you, 0);
                e.pos.y
            })
            .max()
            .unwrap_or(0);
        assert!(high > d.hover + BOUNCE_HEIGHT * 9 / 10, "and bounce high: {high}");
    }
}
