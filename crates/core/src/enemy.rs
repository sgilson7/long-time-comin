//! Enemies: how each kind moves, and the attack cycle every kind shares.
//!
//! The cycle is wind up (the body glows, so you can read what is coming),
//! cast (the pattern's volleys fire on their own clock), rest. Attacks are
//! taken in order from the current phase. A boss moves to its next phase
//! when its health falls past the phase's `from_pct`, calls its summons, and
//! holds still for a moment while the arena is cleared of its orbs.

use crate::balance::*;
use crate::content::{Content, EnemyDef, Kind, Pattern};
use crate::fx::{self, int, ONE};
use crate::geom::{v3, yaw_pitch_of, V3};
use crate::level::Level;
use crate::orbs::{self, Salvo};

/// A new enemy takes this long to appear, and cannot be hurt or fire meanwhile.
pub const SPAWN_TICKS: i32 = secs(4, 5);
/// a boss changing phase holds still this long
pub const PHASE_TICKS: i32 = secs(3, 2);
/// how high a walker's step can climb
const WALKER_STEP: i32 = ONE * 6 / 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Spawning(i32),
    Windup(i32),
    Casting(i32),
    Rest(i32),
    Changing(i32),
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
}

/// What an enemy did this tick that the world must act on.
#[derive(Debug, Default)]
pub struct Deeds {
    pub salvo: Salvo,
    /// enemies to summon, by definition index
    pub summons: Vec<usize>,
    pub phase_changed: bool,
    pub burst: bool,
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
        }
    }

    pub fn alive(&self) -> bool {
        self.hp > 0
    }

    pub fn hurtable(&self) -> bool {
        self.alive() && !matches!(self.state, State::Spawning(_) | State::Changing(_))
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
        self.yaw = fx::lerp_angle(self.yaw, aim_yaw, ONE / 8);

        if let State::Spawning(t) = self.state {
            self.state = if t > 1 {
                State::Spawning(t - 1)
            } else {
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

        self.locomote(d, level, target, to);
        if d.kind == Kind::Moth {
            return deeds;
        }

        let Some(attack) = self.current_attack(c).cloned() else {
            return deeds;
        };
        let pattern: &Pattern = &c.patterns[attack.pattern];
        self.state = match self.state {
            State::Rest(t) if t > 1 => State::Rest(t - 1),
            State::Rest(_) => State::Windup(attack.windup.max(1)),
            State::Windup(t) if t > 1 => State::Windup(t - 1),
            State::Windup(_) => State::Casting(0),
            State::Casting(t) => {
                for v in &pattern.volleys {
                    let since = t - v.delay;
                    if since < 0 {
                        continue;
                    }
                    let k = if v.every > 0 { since / v.every } else { since };
                    let on_beat = v.every == 0 || since % v.every == 0;
                    if on_beat && k < v.repeat {
                        let s = orbs::fire(v, self.muzzle(d), target, k, me, &c.pal, level);
                        deeds.salvo.orbs.extend(s.orbs);
                        deeds.salvo.waves.extend(s.waves);
                    }
                }
                if t >= pattern.length() {
                    self.attack += 1;
                    State::Rest(attack.rest)
                } else {
                    State::Casting(t + 1)
                }
            }
            s => s,
        };
        deeds
    }

    /// Where the orbs leave from: the body's centre, or a walker's chest.
    pub fn muzzle(&self, _d: &EnemyDef) -> V3 {
        self.pos
    }

    fn locomote(&mut self, d: &EnemyDef, level: &Level, target: V3, to: V3) {
        let flat = v3(to.x, 0, to.z);
        let dist = flat.len();
        let towards = flat.norm();
        let side = v3(-towards.z, 0, towards.x).scale(self.orbit * ONE);
        match d.kind {
            Kind::Turret => {}
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
                let next = self.pos.add(self.vel);
                if level.sphere_hits(next, d.radius) {
                    self.orbit = -self.orbit;
                    self.vel = self.vel.neg().scale(ONE / 2);
                } else {
                    self.pos = next;
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
        assert_eq!(d.summons.len(), 4);
        assert!(
            !e.hurtable(),
            "it holds still and cannot be hurt while it changes"
        );
    }
}
