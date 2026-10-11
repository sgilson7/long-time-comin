//! The gun in hand and the shots it makes. One gun is carried at a time, as
//! in Returnal and Tower of Guns; a pickup on a side path swaps it for a
//! different or better one.
//!
//! The Ripsaw is thrown, not fired: each press of the trigger throws a
//! blade, if one is in hand, and a blade lodges in what it hits. Holding
//! the trigger revs every lodged blade (the world deals that damage); a
//! blade whose teeth are spent comes back after its recall.

use crate::content::{Gun, Rgb};
use crate::fx;
use crate::geom::{facing, yaw_pitch_of, V3};
use crate::rng::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Held {
    pub gun: usize,
    pub level: i32,
    pub cooldown: i32,
    /// a saw gun's blades out of hand, and the recall clocks of those spent
    pub saws_out: i32,
    pub recall: [i32; 4],
}

impl Held {
    pub fn new(gun: usize, level: i32) -> Held {
        Held {
            gun,
            level,
            cooldown: 0,
            saws_out: 0,
            recall: [0; 4],
        }
    }

    /// A spent blade starts back to hand.
    pub fn recall_blade(&mut self, ticks: i32) {
        if let Some(slot) = self.recall.iter_mut().find(|r| **r == 0) {
            *slot = ticks.max(1);
        } else {
            self.saws_out = (self.saws_out - 1).max(0);
        }
    }

    /// One tick of the blades coming back.
    pub fn recall_step(&mut self) {
        for r in self.recall.iter_mut() {
            if *r > 0 {
                *r -= 1;
                if *r == 0 {
                    self.saws_out = (self.saws_out - 1).max(0);
                }
            }
        }
    }
}

/// Where a lodged blade sits: in an enemy (or one of its weak points), off
/// its centre; or in a wall, for a moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lodge {
    Enemy {
        enemy: usize,
        weak: Option<usize>,
        offset: V3,
    },
    Wall(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotKind {
    Bullet,
    Bomb,
    /// a gold orb turned back by a parry
    Reflect,
    /// a Ripsaw's blade
    Saw,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shot {
    pub pos: V3,
    pub prev: V3,
    pub vel: V3,
    pub damage: i32,
    pub radius: i32,
    pub life: i32,
    pub splash_radius: i32,
    pub splash: i32,
    pub pierce: i32,
    /// enemies this shot has already passed through
    pub passed: Vec<usize>,
    pub rgb: Rgb,
    pub kind: ShotKind,
    /// a level 3 launcher's burst erases enemy orbs inside it
    pub erases: bool,
    /// a blade: where it is lodged, and the damage it has left to deal
    pub lodged: Option<Lodge>,
    pub teeth: i32,
}

/// What the trigger is doing this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Pull {
    pub held: bool,
    pub pressed: bool,
    /// Pierce from the run's rewards, and Hair Triggers taken
    pub pierce: i32,
    pub trigger: i32,
}

/// The ticks between shots, quickened by Hair Triggers.
pub fn rate(base: i32, triggers: i32) -> i32 {
    (base * 100 / (100 + crate::balance::TRIGGER_PCT * triggers.max(0))).max(1)
}

/// Counts the gun's cooldown down, and fires if the trigger is held and the
/// gun is ready: the shots leave `muzzle` toward `aim_point`, each strayed
/// inside the level's spread by the seeded generator. A saw gun throws a
/// blade on a press, while it has one in hand.
pub fn trigger(
    held: &mut Held,
    gun: &Gun,
    rgb: Rgb,
    pull: Pull,
    muzzle: V3,
    aim_point: V3,
    rng: &mut Rng,
) -> Vec<Shot> {
    held.cooldown = (held.cooldown - 1).max(0);
    held.recall_step();
    let l = gun.level(held.level);
    let pierce_bonus = pull.pierce;
    if let Some(saw) = l.saw {
        if !pull.pressed || held.cooldown > 0 || held.saws_out >= saw.saws {
            return Vec::new();
        }
        held.cooldown = rate(l.rate, pull.trigger);
        held.saws_out += 1;
        let dir = aim_point.sub(muzzle).norm();
        return vec![Shot {
            pos: muzzle,
            prev: muzzle,
            vel: dir.scale(l.speed),
            damage: l.damage,
            radius: l.size,
            life: l.life,
            splash_radius: 0,
            splash: 0,
            pierce: 0,
            passed: Vec::new(),
            rgb,
            kind: ShotKind::Saw,
            erases: false,
            lodged: None,
            teeth: saw.teeth,
        }];
    }
    if !pull.held || held.cooldown > 0 {
        return Vec::new();
    }
    held.cooldown = rate(l.rate, pull.trigger);
    let (yaw, pitch) = yaw_pitch_of(aim_point.sub(muzzle));
    let kind = if l.splash_radius > 0 {
        ShotKind::Bomb
    } else {
        ShotKind::Bullet
    };
    (0..l.pellets)
        .map(|k| {
            // the first pellet of a spread flies true, so a close shot lands
            let (dy, dp) = if k == 0 && l.pellets > 1 {
                (0, 0)
            } else {
                (rng.range(-l.spread, l.spread), rng.range(-l.spread, l.spread))
            };
            let dir = facing(yaw + dy, fx::wrap(pitch + dp));
            Shot {
                pos: muzzle,
                prev: muzzle,
                vel: dir.scale(l.speed),
                damage: l.damage,
                radius: l.size,
                life: l.life,
                splash_radius: l.splash_radius,
                splash: l.splash,
                pierce: l.pierce + pierce_bonus,
                passed: Vec::new(),
                rgb,
                kind,
                erases: held.level >= 3 && kind == ShotKind::Bomb,
                lodged: None,
                teeth: 0,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;
    use crate::fx::{int, ONE};
    use crate::geom::v3;

    fn firing() -> Pull {
        Pull {
            held: true,
            pressed: true,
            ..Pull::default()
        }
    }

    #[test]
    fn a_held_trigger_fires_at_the_guns_rate() {
        let c = content();
        let g = &c.guns[c.gun("carbine")];
        let mut h = Held::new(c.gun("carbine"), 1);
        let mut rng = Rng::new(1);
        let mut shots = 0;
        for _ in 0..60 {
            shots += trigger(
                &mut h,
                g,
                [0; 3],
                firing(),
                V3::ZERO,
                v3(0, 0, -int(10)),
                &mut rng,
            )
            .len();
        }
        // every 6 ticks (105 ms rounded) for 60 ticks: 10 shots
        assert_eq!(shots, 10);
    }

    #[test]
    fn hair_triggers_quicken_the_rate() {
        assert_eq!(rate(6, 0), 6);
        // 15% faster: 6 × 100 / 115 = 5.2, so 5
        assert_eq!(rate(6, 1), 5);
        assert!(rate(60, 2) < rate(60, 1));
    }

    #[test]
    fn a_ripsaw_throws_one_blade_a_press_while_it_has_one_and_gets_it_back() {
        let c = content();
        let gun = c.gun("ripsaw");
        let g = &c.guns[gun];
        let saws = g.level(1).saw.expect("the ripsaw is a saw gun").saws;
        let mut h = Held::new(gun, 1);
        let mut rng = Rng::new(1);
        let held_only = Pull {
            held: true,
            ..Pull::default()
        };
        let mut thrown = 0;
        for _ in 0..60 {
            thrown += trigger(
                &mut h,
                g,
                [0; 3],
                held_only,
                V3::ZERO,
                v3(0, 0, -int(10)),
                &mut rng,
            )
            .len();
        }
        assert_eq!(thrown, 0, "holding the trigger throws nothing");
        for _ in 0..120 {
            thrown += trigger(
                &mut h,
                g,
                [0; 3],
                firing(),
                V3::ZERO,
                v3(0, 0, -int(10)),
                &mut rng,
            )
            .len();
        }
        assert_eq!(thrown as i32, saws, "no more than the blades in hand");
        h.recall_blade(10);
        for _ in 0..11 {
            h.recall_step();
        }
        assert_eq!(h.saws_out, saws - 1, "a spent blade comes back");
    }

    #[test]
    fn the_scattergun_fires_its_pellets_in_a_cone_round_the_aim() {
        let c = content();
        let g = &c.guns[c.gun("scattergun")];
        let mut h = Held::new(c.gun("scattergun"), 2);
        let mut rng = Rng::new(9);
        let shots = trigger(
            &mut h,
            g,
            [0; 3],
            firing(),
            V3::ZERO,
            v3(0, 0, -int(10)),
            &mut rng,
        );
        assert_eq!(shots.len(), 11);
        let cos_spread = fx::cos(g.level(2).spread * 3 / 2);
        for s in &shots {
            let d = s.vel.norm();
            assert!(d.dot(v3(0, 0, -ONE)) >= cos_spread, "{d:?}");
        }
        // the first flies true
        assert!(shots[0].vel.norm().z < -ONE + 8);
    }

    #[test]
    fn only_a_level_three_launcher_erases_orbs() {
        let c = content();
        let g = &c.guns[c.gun("launcher")];
        let mut rng = Rng::new(3);
        for (level, erases) in [(1, false), (2, false), (3, true)] {
            let mut h = Held::new(c.gun("launcher"), level);
            let s = trigger(
                &mut h,
                g,
                [0; 3],
                firing(),
                V3::ZERO,
                v3(0, 0, -int(10)),
                &mut rng,
            );
            assert_eq!(s[0].kind, ShotKind::Bomb);
            assert_eq!(s[0].erases, erases, "level {level}");
        }
    }
}
