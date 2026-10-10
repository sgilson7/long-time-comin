//! Enemy fire: orbs and ground waves, and the volleys that make them.
//!
//! An orb flies in a straight line unless it curls (turns about the vertical
//! a little every tick) or changes speed (accelerates toward `max_speed`,
//! or slows toward `min_speed`). Those three numbers, with how many orbs a
//! volley holds and how far each repeat is turned, make every pattern in
//! `data/patterns.json`: spirals, flowers, blooms that burst and hang.
//!
//! A ground wave is a ring that grows along the floor from where it was
//! struck. It hits a body standing in it and misses one in the air.

use crate::balance::*;
use crate::content::{Palette, Rgb, Shape, Volley};
use crate::fx::{self, ONE};
use crate::geom::{facing, right_of, v3, yaw_pitch_of, V3};
use crate::level::Level;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Orb {
    pub pos: V3,
    pub prev: V3,
    /// unit direction of travel
    pub dir: V3,
    pub speed: i32,
    pub accel: i32,
    pub min_speed: i32,
    pub max_speed: i32,
    pub curl: i32,
    pub radius: i32,
    pub rgb: Rgb,
    pub gold: bool,
    pub age: i32,
    /// the enemy that fired it, so a parried orb knows where to go
    pub owner: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wave {
    pub centre: V3,
    pub radius: i32,
    pub prev_radius: i32,
    pub speed: i32,
    pub reach: i32,
    pub rgb: Rgb,
    pub hit: bool,
}

/// A ring tilts toward a target above or below it by at most this much.
pub const RING_TILT: i32 = fx::deg(35);

/// Everything one volley fires.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Salvo {
    pub orbs: Vec<Orb>,
    pub waves: Vec<Wave>,
}

/// Fires one repeat of a volley from `origin` at a target. `repeat` counts
/// the repeats already fired, for the spin between them.
pub fn fire(
    v: &Volley,
    origin: V3,
    target: V3,
    repeat: i32,
    owner: usize,
    pal: &Palette,
    level: &Level,
) -> Salvo {
    let mut out = Salvo::default();
    let rgb = pal.c(&v.colour);
    let (aim_yaw, aim_pitch) = yaw_pitch_of(target.sub(origin));
    let spin = v.spin.wrapping_mul(repeat);
    let mut push = |dir: V3, curl: i32| {
        out.orbs.push(Orb {
            pos: origin,
            prev: origin,
            dir,
            speed: v.speed,
            accel: v.accel,
            min_speed: v.min_speed,
            max_speed: v.max_speed.max(v.speed),
            curl,
            radius: v.size,
            rgb,
            gold: v.gold,
            age: 0,
            owner,
        })
    };
    let curl_of = |i: i32| {
        if v.alternate && i % 2 == 1 {
            -v.curl
        } else {
            v.curl
        }
    };
    match v.shape {
        Shape::Ring => {
            // The ring's plane tilts through the target, so a turret on a
            // roof still sweeps the floor below it (SECOND-ORDER G2): each
            // orb leaves at angle θ round the ring from the line to the
            // target, cos θ along it and sin θ across it.
            let tilt = fx::wrap(aim_pitch).clamp(-RING_TILT, RING_TILT) + v.pitch;
            let along = facing(aim_yaw, tilt);
            let across = right_of(aim_yaw).neg();
            let base = if v.aim { 0 } else { -aim_yaw } + spin;
            for i in 0..v.count {
                let theta = base + fx::TURN / v.count * i;
                let dir = along.scale(fx::cos(theta)).add(across.scale(fx::sin(theta)));
                push(dir.norm(), curl_of(i));
            }
        }
        Shape::Aimed => {
            for i in 0..v.count {
                let off = if v.count == 1 {
                    0
                } else {
                    -v.spread / 2 + v.spread * i / (v.count - 1)
                };
                push(facing(aim_yaw + off + spin, aim_pitch), curl_of(i));
            }
        }
        Shape::Sphere => {
            for r in 0..v.rings {
                let pitch = if v.rings == 1 {
                    0
                } else {
                    -fx::deg(50) + fx::deg(100) * r / (v.rings - 1)
                };
                // fewer orbs on the rings near the top, so they space evenly
                let n = (fx::mul(v.count * ONE, fx::cos(pitch)) / ONE).max(3);
                let base = spin + fx::TURN / n / 2 * (r % 2);
                for i in 0..n {
                    push(facing(base + fx::TURN / n * i, pitch), curl_of(i));
                }
            }
        }
        Shape::Wave => {
            let ground = level.floor_below(origin.x, origin.y + ONE, origin.z).unwrap_or(0);
            out.waves.push(Wave {
                centre: v3(origin.x, ground, origin.z),
                radius: 0,
                prev_radius: 0,
                speed: v.speed,
                reach: v.size,
                rgb,
                hit: false,
            });
        }
    }
    out
}

impl Orb {
    /// One tick of flight; false when the orb is spent (hit a solid, flew
    /// out of its life or out of the arena).
    pub fn step(&mut self, level: &Level) -> bool {
        self.prev = self.pos;
        self.age += 1;
        if self.curl != 0 {
            let (s, c) = (fx::sin(self.curl), fx::cos(self.curl));
            let d = self.dir;
            self.dir = v3(
                fx::mul(d.x, c) + fx::mul(d.z, s),
                d.y,
                fx::mul(d.z, c) - fx::mul(d.x, s),
            );
            if self.age % 16 == 0 {
                self.dir = self.dir.norm();
            }
        }
        if self.accel != 0 {
            self.speed = (self.speed + self.accel).clamp(self.min_speed, self.max_speed);
        }
        self.pos = self.pos.add(self.dir.scale(self.speed));
        self.age < ORB_LIFE
            && self.pos.y > -fx::int(4)
            && self.pos.len_xz() < fx::int(160)
            && !level.sphere_hits(self.pos, self.radius / 2)
    }

    /// Does the orb touch a body standing at `feet`? The body is a capsule:
    /// a vertical segment with the body's radius round it.
    pub fn touches(&self, feet: V3) -> bool {
        let r = self.radius * ORB_HIT_PCT / 100 + PLAYER_RADIUS;
        let lo = feet.y + PLAYER_RADIUS;
        let hi = feet.y + PLAYER_HEIGHT - PLAYER_RADIUS;
        let nearest = v3(feet.x, self.pos.y.clamp(lo, hi), feet.z);
        within(self.pos, nearest, r)
    }
}

/// `|a − b| < r`, without a square root.
pub fn within(a: V3, b: V3, r: i32) -> bool {
    let d = a.sub(b);
    let d2 = d.x as i64 * d.x as i64 + d.y as i64 * d.y as i64 + d.z as i64 * d.z as i64;
    d2 < r as i64 * r as i64
}

impl Wave {
    pub fn step(&mut self) -> bool {
        self.prev_radius = self.radius;
        self.radius += self.speed;
        self.radius < self.reach
    }

    /// A body standing in the ring's path as it passes is hit; one whose
    /// feet are above the wave's top is not.
    pub fn touches(&self, feet: V3) -> bool {
        if feet.y >= self.centre.y + WAVE_HEIGHT || feet.y < self.centre.y - ONE {
            return false;
        }
        let d = feet.sub(self.centre).len_xz();
        let lo = self.prev_radius - WAVE_WIDTH / 2 - PLAYER_RADIUS;
        let hi = self.radius + WAVE_WIDTH / 2 + PLAYER_RADIUS;
        d >= lo && d <= hi
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;
    use crate::fx::{deg, int};
    use crate::level::Solid;

    fn open_floor() -> Level {
        Level::new(vec![Solid::new(
            v3(-int(80), -ONE, -int(80)),
            v3(int(80), 0, int(80)),
        )])
    }

    fn volley(id: &str) -> Volley {
        let c = content();
        c.patterns.iter().find(|p| p.id == id).unwrap().volleys[0].clone()
    }

    #[test]
    fn a_ring_fires_its_count_evenly_round_the_compass() {
        let c = content();
        let v = volley("ring_16");
        let s = fire(
            &v,
            v3(0, int(2), 0),
            v3(0, int(2), -int(10)),
            0,
            0,
            &c.pal,
            &open_floor(),
        );
        assert_eq!(s.orbs.len(), 16);
        // the directions sum to nothing: they are spread evenly
        let sum = s.orbs.iter().fold(V3::ZERO, |a, o| a.add(o.dir));
        assert!(sum.len() < 64, "{sum:?}");
        // aim: true turns the ring so one orb flies straight at the target (−z)
        assert!(s.orbs.iter().any(|o| o.dir.z < -ONE + 16));
    }

    #[test]
    fn a_ring_fired_from_a_roof_tilts_down_through_the_target() {
        let c = content();
        let v = volley("ring_16");
        // from 6 m up at a target 10 m away on the floor: 31° down
        let s = fire(
            &v,
            v3(0, int(7), 0),
            v3(0, int(1), -int(10)),
            0,
            0,
            &c.pal,
            &open_floor(),
        );
        let toward = s.orbs.iter().max_by_key(|o| -o.dir.z).unwrap();
        let (_, pitch) = yaw_pitch_of(toward.dir);
        assert!((pitch + deg(31)).abs() < deg(2), "{pitch}");
        // and the orbs flying across still fly level
        let side = s.orbs.iter().max_by_key(|o| o.dir.x).unwrap();
        assert!(side.dir.y.abs() < ONE / 20, "{:?}", side.dir);
    }

    #[test]
    fn an_aimed_fan_is_centred_on_the_target() {
        let c = content();
        let v = volley("fan_5");
        let target = v3(int(10), int(2), 0);
        let s = fire(&v, v3(0, int(2), 0), target, 0, 0, &c.pal, &open_floor());
        assert_eq!(s.orbs.len(), 5);
        // the middle orb of five flies at the target (+x)
        assert!(s.orbs[2].dir.x > ONE - 16, "{:?}", s.orbs[2].dir);
        // the outer two are 22° either side: cos 22° = 0.927
        assert!((s.orbs[0].dir.x - 60_760).abs() < 300, "{:?}", s.orbs[0].dir);
        assert!(s.orbs[0].dir.z.signum() == -s.orbs[4].dir.z.signum());
    }

    #[test]
    fn spin_turns_each_repeat_of_a_spiral() {
        let c = content();
        let v = volley("spiral_quad");
        let l = open_floor();
        let level_target = v3(0, int(2), -int(10));
        let a = fire(&v, v3(0, int(2), 0), level_target, 0, 0, &c.pal, &l);
        let b = fire(&v, v3(0, int(2), 0), level_target, 1, 0, &c.pal, &l);
        let (ya, _) = yaw_pitch_of(a.orbs[0].dir);
        let (yb, _) = yaw_pitch_of(b.orbs[0].dir);
        assert!((fx::wrap(yb - ya) - deg(9)).abs() < deg(1));
    }

    #[test]
    fn a_curling_orb_turns_and_a_flower_turns_both_ways() {
        let c = content();
        let v = volley("flower");
        let l = open_floor();
        let mut s = fire(&v, v3(0, int(2), 0), v3(int(10), int(2), 0), 0, 0, &c.pal, &l);
        assert_eq!(s.orbs[0].curl, -s.orbs[1].curl);
        let start = s.orbs[0].dir;
        for _ in 0..60 {
            s.orbs[0].step(&l);
        }
        // 48°/s for one second: the direction has turned about 48°
        let cos = start.dot(s.orbs[0].dir);
        assert!((cos - fx::cos(deg(48))).abs() < ONE / 40, "cos {cos}");
        // and flown about 6.2 m
        let d = s.orbs[0].pos.sub(v3(0, int(2), 0)).len();
        assert!(d > int(5) && d < int(7), "{d}");
    }

    #[test]
    fn a_bloom_bursts_out_then_slows_to_its_floor() {
        let c = content();
        let v = volley("bloom");
        let l = open_floor();
        let mut s = fire(&v, v3(0, int(2), 0), v3(int(10), int(2), 0), 0, 0, &c.pal, &l);
        let o = &mut s.orbs[0];
        for _ in 0..120 {
            o.step(&l);
        }
        assert_eq!(o.speed, v.min_speed);
    }

    #[test]
    fn an_orb_dies_on_a_wall() {
        let c = content();
        let l = Level::new(vec![Solid::stand(0, 0, -int(6), int(10), int(10), ONE)]);
        let v = volley("snipe");
        let mut s = fire(&v, v3(0, int(2), 0), v3(0, int(2), -int(20)), 0, 0, &c.pal, &l);
        let o = &mut s.orbs[0];
        let mut ticks = 0;
        while o.step(&l) {
            ticks += 1;
            assert!(ticks < 60, "still flying at {:?}", o.pos);
        }
        // 23 m/s: the wall face 5.5 m away is reached in about 14 ticks
        assert!((12..=16).contains(&ticks), "{ticks}");
    }

    #[test]
    fn an_orb_touches_a_body_it_passes_through_and_misses_one_it_clears() {
        let o = Orb {
            pos: v3(0, int(1), 0),
            prev: V3::ZERO,
            dir: V3::ZERO,
            speed: 0,
            accel: 0,
            min_speed: 0,
            max_speed: 0,
            curl: 0,
            radius: ONE * 4 / 10,
            rgb: [0; 3],
            gold: false,
            age: 0,
            owner: 0,
        };
        assert!(o.touches(V3::ZERO));
        // a body jumped 1.6 m clear: its feet are over the orb's top
        assert!(!o.touches(v3(0, ONE * 16 / 10, 0)));
        // to the side by more than both radii
        assert!(!o.touches(v3(ONE, 0, 0)));
    }

    #[test]
    fn a_wave_hits_a_standing_body_and_misses_a_jumping_one() {
        let c = content();
        let v = volley("ground_wave");
        let l = open_floor();
        let s = fire(&v, v3(0, int(1), 0), V3::ZERO, 0, 0, &c.pal, &l);
        let mut w = s.waves[0].clone();
        let body = v3(int(6), 0, 0);
        let air = v3(int(6), ONE, 0);
        let (mut hit_ground, mut hit_air) = (false, false);
        while w.step() {
            hit_ground |= w.touches(body);
            hit_air |= w.touches(air);
        }
        assert!(hit_ground && !hit_air);
    }
}
