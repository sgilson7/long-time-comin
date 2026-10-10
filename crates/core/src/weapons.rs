//! The gun in hand and the shots it makes. One gun is carried at a time, as
//! in Returnal and Tower of Guns; a pickup on a side path swaps it for a
//! different or better one.

use crate::content::{Gun, Rgb};
use crate::fx;
use crate::geom::{facing, yaw_pitch_of, V3};
use crate::rng::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Held {
    pub gun: usize,
    pub level: i32,
    pub cooldown: i32,
}

impl Held {
    pub fn new(gun: usize, level: i32) -> Held {
        Held {
            gun,
            level,
            cooldown: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotKind {
    Bullet,
    Bomb,
    /// a gold orb turned back by a parry
    Reflect,
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
}

/// Counts the gun's cooldown down, and fires if the trigger is held and the
/// gun is ready: the shots leave `muzzle` toward `aim_point`, each strayed
/// inside the level's spread by the seeded generator.
pub fn trigger(
    held: &mut Held,
    gun: &Gun,
    rgb: Rgb,
    firing: bool,
    muzzle: V3,
    aim_point: V3,
    rng: &mut Rng,
) -> Vec<Shot> {
    held.cooldown = (held.cooldown - 1).max(0);
    if !firing || held.cooldown > 0 {
        return Vec::new();
    }
    let l = gun.level(held.level);
    held.cooldown = l.rate;
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
                pierce: l.pierce,
                passed: Vec::new(),
                rgb,
                kind,
                erases: held.level >= 3 && kind == ShotKind::Bomb,
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

    #[test]
    fn a_held_trigger_fires_at_the_guns_rate() {
        let c = content();
        let g = &c.guns[c.gun("carbine")];
        let mut h = Held::new(c.gun("carbine"), 1);
        let mut rng = Rng::new(1);
        let mut shots = 0;
        for _ in 0..60 {
            shots += trigger(&mut h, g, [0; 3], true, V3::ZERO, v3(0, 0, -int(10)), &mut rng).len();
        }
        // every 6 ticks (105 ms rounded) for 60 ticks: 10 shots
        assert_eq!(shots, 10);
    }

    #[test]
    fn the_scattergun_fires_its_pellets_in_a_cone_round_the_aim() {
        let c = content();
        let g = &c.guns[c.gun("scattergun")];
        let mut h = Held::new(c.gun("scattergun"), 2);
        let mut rng = Rng::new(9);
        let shots = trigger(&mut h, g, [0; 3], true, V3::ZERO, v3(0, 0, -int(10)), &mut rng);
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
            let s = trigger(&mut h, g, [0; 3], true, V3::ZERO, v3(0, 0, -int(10)), &mut rng);
            assert_eq!(s[0].kind, ShotKind::Bomb);
            assert_eq!(s[0].erases, erases, "level {level}");
        }
    }
}
