//! Enemy fire: orbs and ground waves, and the volleys that make them.
//!
//! An orb flies in a straight line unless it curls (turns about the vertical
//! a little every tick) or changes speed (accelerates toward `max_speed`,
//! or slows toward `min_speed`). Those three numbers, with how many orbs a
//! volley holds and how far each repeat is turned, make every pattern in
//! `data/patterns.json`: spirals, flowers, blooms that burst and hang.
//!
//! A held formation (zone two) places its orbs in a shape round the enemy,
//! in front of it, round you or over you, and holds them still; each sets
//! off when its hold ends, outward, inward, at you, down, along or round.
//! Any orb may stop again after a while in flight, wait, and set off a
//! second time (a refraction, a boomerang).
//!
//! A ground wave is a ring that grows along the floor from where it was
//! struck. It hits a body standing in it and misses one in the air.

use crate::balance::*;
use crate::content::{Anchor, Form, Launch, Palette, Rehold, Rgb, Shape, Volley};
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
    pub damage: i32,
    /// the Injunction breaks it
    pub blue: bool,
    /// ticks it still hangs still before it sets off; 0 once moving
    pub wait: i32,
    /// the hold it was given, for the drawn tell
    pub held: i32,
    pub launch: Launch,
    /// its formation's centre, and the formation's plane's normal
    pub hub: V3,
    pub axis: V3,
    /// the speed it sets off at
    pub go: i32,
    pub rehold: Option<Rehold>,
    /// ticks flown since it last set off
    pub flown: i32,
}

impl Orb {
    /// A plain orb flying along `dir` at `speed`.
    pub fn new(pos: V3, dir: V3, speed: i32, radius: i32, rgb: Rgb, damage: i32) -> Orb {
        Orb {
            pos,
            prev: pos,
            dir,
            speed,
            accel: 0,
            min_speed: 0,
            max_speed: speed,
            curl: 0,
            radius,
            rgb,
            gold: false,
            age: 0,
            owner: 0,
            damage,
            blue: false,
            wait: 0,
            held: 0,
            launch: Launch::Keep,
            hub: pos,
            axis: V3::UP,
            go: speed,
            rehold: None,
            flown: 0,
        }
    }

    /// Hanging still, waiting to set off.
    pub fn holding(&self) -> bool {
        self.wait > 0 || self.speed == 0
    }
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
    pub damage: i32,
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
            damage: v.damage,
            blue: v.blue,
            wait: 0,
            held: 0,
            launch: Launch::Keep,
            hub: origin,
            axis: V3::UP,
            go: v.speed,
            rehold: v.rehold,
            flown: 0,
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
        Shape::Form => {
            out.orbs = formation(v, origin, target, repeat, owner, rgb);
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
                damage: v.damage,
            });
        }
    }
    out
}

/// A held formation's orbs: placed, still, each with its hold.
fn formation(v: &Volley, origin: V3, target: V3, repeat: i32, owner: usize, rgb: Rgb) -> Vec<Orb> {
    let to = target.sub(origin);
    let (aim_yaw, _) = yaw_pitch_of(to);
    // across the line to you, along it (flat), and up
    let toward = facing(aim_yaw, 0);
    let across = right_of(aim_yaw);
    let hub = match v.anchor {
        Anchor::Body => origin,
        // on the line from the enemy to you, `at` along it
        Anchor::Front => origin.add(to.norm().scale(v.at.min(to.len()))),
        Anchor::Target => target,
        Anchor::Above => target.add(v3(0, v.at, 0)),
    };
    // A flat formation placed by the enemy lies in the plane through you
    // (tilted as zone one's rings are), so what flies outward reaches you;
    // one placed round or over you lies level.
    let (_, aim_pitch) = yaw_pitch_of(target.sub(hub));
    let level = matches!(v.anchor, Anchor::Target | Anchor::Above);
    let tilt = if level {
        0
    } else {
        fx::wrap(aim_pitch).clamp(-RING_TILT, RING_TILT)
    };
    let (e1, e2, axis) = if v.flat {
        let along = facing(aim_yaw, tilt);
        (across, along, across.cross(along).norm())
    } else {
        (across, V3::UP, toward)
    };
    let r = v.radius;
    let n = v.count.max(1);
    let base = v.spin.wrapping_mul(repeat);
    let at = |a: i32, rad: i32| {
        hub.add(e1.scale(fx::mul(fx::cos(a), rad)))
            .add(e2.scale(fx::mul(fx::sin(a), rad)))
    };
    // a position k of n spread evenly over [−r, r]
    let span = |k: i32, n: i32| if n <= 1 { 0 } else { -r + 2 * r / (n - 1) * k };
    let mut spots = Vec::new();
    match v.form {
        Form::Ring => {
            for i in 0..n {
                spots.push(at(base + fx::TURN / n * i, r));
            }
        }
        Form::Grid => {
            // `radius` apart, centred on the hub
            let off = |k: i32, n: i32| r * (2 * k - (n - 1)) / 2;
            for j in 0..v.rows {
                for i in 0..n {
                    spots.push(hub.add(e1.scale(off(i, n))).add(e2.scale(off(j, v.rows))));
                }
            }
        }
        Form::Line => {
            for i in 0..n {
                spots.push(hub.add(e1.scale(span(i, n))));
            }
        }
        Form::Ribbon => {
            for i in 0..n {
                // `rows` half-turns of a sine across the line
                let rise = fx::sin(fx::TURN / 2 * v.rows * i / (n - 1).max(1));
                spots.push(
                    hub.add(e1.scale(span(i, n)))
                        .add(V3::UP.scale(fx::mul(rise, r / 4))),
                );
            }
        }
        Form::Column => {
            for i in 0..n {
                let up = if n <= 1 { 0 } else { r / (n - 1) * i };
                spots.push(hub.add(v3(0, up, 0)));
            }
        }
        Form::Star => {
            for j in 0..v.rows {
                for i in 0..n {
                    spots.push(at(base + fx::TURN / n * i, r / v.rows * (j + 1)));
                }
            }
        }
        Form::Spiral => {
            for i in 0..n {
                let f = fx::div((i + 1) * ONE, n * ONE);
                spots.push(at(base + fx::mul(fx::TURN * v.rows, f), fx::mul(r, f)));
            }
        }
        Form::Arc => {
            for i in 0..n {
                let off = if n <= 1 {
                    0
                } else {
                    -v.spread / 2 + v.spread / (n - 1) * i
                };
                spots.push(hub.add(facing(aim_yaw + off + base, 0).scale(r)));
            }
        }
    }
    let along = to.norm();
    spots
        .into_iter()
        .enumerate()
        .map(|(k, pos)| {
            let wait = (v.hold + v.stagger * k as i32).max(1);
            let curl = if v.alternate && k % 2 == 1 {
                -v.curl
            } else {
                v.curl
            };
            Orb {
                pos,
                prev: pos,
                dir: along,
                speed: 0,
                accel: v.accel,
                min_speed: v.min_speed,
                max_speed: v.max_speed.max(v.speed),
                curl,
                radius: v.size,
                rgb,
                gold: v.gold,
                age: 0,
                owner,
                damage: v.damage,
                blue: v.blue,
                wait,
                held: wait,
                launch: v.launch,
                hub,
                axis,
                go: v.speed,
                rehold: v.rehold,
                flown: 0,
            }
        })
        .collect()
}

impl Orb {
    /// Its hold is over: it sets off, the way its launch says. `moving`
    /// is the target's velocity, for a lead.
    fn release(&mut self, target: V3, moving: V3) {
        let from_hub = self.pos.sub(self.hub).norm();
        let d = match self.launch {
            Launch::Keep | Launch::Along => self.dir,
            Launch::Out => from_hub,
            Launch::In => from_hub.neg(),
            Launch::Aim => target.sub(self.pos).norm(),
            Launch::Lead => {
                // where the target will be after the flight to where it is
                let ticks = fx::div(target.dist(self.pos), self.go.max(1)) / ONE;
                target
                    .add(moving.scale(ticks.min(240) * ONE))
                    .sub(self.pos)
                    .norm()
            }
            Launch::Down => v3(0, -ONE, 0),
            Launch::Spin => self.axis.cross(self.pos.sub(self.hub)).norm(),
        };
        if d != V3::ZERO {
            self.dir = d;
        }
        self.speed = self.go;
        self.flown = 0;
    }

    /// One tick of flight; false when the orb is spent (hit a solid, flew
    /// out of its life or out of the arena). `target` is the chest a held
    /// orb aims at when it sets off, and `moving` that chest's velocity.
    pub fn step(&mut self, level: &Level, target: V3, moving: V3) -> bool {
        self.prev = self.pos;
        self.age += 1;
        if self.wait > 0 {
            self.wait -= 1;
            if self.wait > 0 {
                return self.age < ORB_LIFE && !level.sphere_hits(self.pos, self.radius / 2);
            }
            self.release(target, moving);
        }
        if let Some(r) = self.rehold {
            self.flown += 1;
            if self.flown > r.stop {
                // it stops, and will set off again
                self.speed = 0;
                self.wait = r.wait.max(1);
                self.held = self.wait;
                self.launch = r.launch;
                self.go = r.speed;
                self.rehold = None;
                return self.age < ORB_LIFE;
            }
        }
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
            s.orbs[0].step(&l, V3::ZERO, V3::ZERO);
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
            o.step(&l, V3::ZERO, V3::ZERO);
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
        while o.step(&l, V3::ZERO, V3::ZERO) {
            ticks += 1;
            assert!(ticks < 60, "still flying at {:?}", o.pos);
        }
        // 18 m/s (patterns.json snipe): the wall face 5.5 m away, less the
        // orb's half radius, is reached in about 18 ticks
        assert!((16..=20).contains(&ticks), "{ticks}");
    }

    #[test]
    fn an_orb_touches_a_body_it_passes_through_and_misses_one_it_clears() {
        let o = Orb::new(v3(0, int(1), 0), V3::ZERO, 0, ONE * 4 / 10, [0; 3], ORB_DAMAGE);
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

    fn pattern(id: &str) -> Vec<Volley> {
        let c = content();
        c.patterns.iter().find(|p| p.id == id).unwrap().volleys.clone()
    }

    #[test]
    fn a_halo_hangs_still_then_its_outer_ring_flies_and_its_inner_ring_follows_at_twice_the_speed() {
        let c = content();
        let l = open_floor();
        let (from, target) = (v3(0, int(2), 0), v3(0, int(1), -int(12)));
        let vs = pattern("halo");
        let mut outer = fire(&vs[0], from, target, 0, 0, &c.pal, &l).orbs;
        let mut inner = fire(&vs[1], from, target, 0, 0, &c.pal, &l).orbs;
        assert_eq!(
            (outer.len() as i32, inner.len() as i32),
            (vs[0].count, vs[1].count)
        );
        // placed in mid-air in front of the enemy, toward you, and still
        let hub = outer[0].hub;
        assert!(hub.z < -int(3) && hub.z > -int(4), "{hub:?}");
        let start: Vec<V3> = outer.iter().map(|o| o.pos).collect();
        for _ in 0..vs[0].hold - 1 {
            for o in outer.iter_mut().chain(inner.iter_mut()) {
                o.step(&l, target, V3::ZERO);
            }
        }
        assert!(
            outer.iter().zip(&start).all(|(o, s)| o.pos == *s),
            "the outer ring has not moved"
        );
        // the outer ring goes first, outward from the middle
        for o in outer.iter_mut().chain(inner.iter_mut()) {
            o.step(&l, target, V3::ZERO);
        }
        assert!(outer.iter().all(|o| o.speed == vs[0].speed && o.wait == 0));
        assert!(
            inner.iter().all(|o| o.speed == 0 && o.wait > 0),
            "the inner ring still waits"
        );
        let out = outer[0].pos.sub(outer[0].hub).norm();
        assert!(outer[0].dir.dot(out) > ONE - ONE / 50);
        for _ in 0..vs[1].hold - vs[0].hold {
            for o in outer.iter_mut().chain(inner.iter_mut()) {
                o.step(&l, target, V3::ZERO);
            }
        }
        // then the inner, at twice the outer's speed
        assert!(inner.iter().all(|o| o.wait == 0));
        // (cm/s to metres a tick rounds each speed on its own)
        assert!(
            (inner[0].speed - outer[0].speed * 2).abs() <= 1,
            "{} {}",
            inner[0].speed,
            outer[0].speed
        );
    }

    #[test]
    fn a_cage_round_you_closes_on_where_you_were() {
        let c = content();
        let l = open_floor();
        let target = v3(int(5), int(1), int(5));
        let v = &pattern("cage")[0];
        let mut s = fire(v, v3(0, int(8), 0), target, 0, 0, &c.pal, &l).orbs;
        let d0 = s[0].pos.dist(target);
        // in a level ring 5.5 m round you, at your chest
        assert!((d0 - v.radius).abs() < ONE / 20, "{d0}");
        assert!(s.iter().all(|o| (o.pos.y - target.y).abs() < ONE / 50));
        for _ in 0..v.hold + 30 {
            for o in s.iter_mut() {
                o.step(&l, v3(-int(20), int(1), 0), V3::ZERO);
            }
        }
        // it closes on where you stood, not where you went
        assert!(s.iter().all(|o| o.pos.dist(target) < d0 - int(2)));
    }

    #[test]
    fn a_staggered_formation_sets_off_one_orb_after_another() {
        let c = content();
        let l = open_floor();
        let target = v3(0, int(1), -int(14));
        let v = &pattern("crown")[0];
        let mut s = fire(v, v3(0, int(2), 0), target, 0, 0, &c.pal, &l).orbs;
        assert_eq!(s.len() as i32, v.count * v.rows);
        let mut moving = Vec::new();
        for _ in 0..v.hold + v.stagger * (s.len() as i32) {
            for o in s.iter_mut() {
                o.step(&l, target, V3::ZERO);
            }
            moving.push(s.iter().filter(|o| o.wait == 0).count());
        }
        // the count rises by one every stagger, never all at once
        assert!(moving.windows(2).all(|w| w[1] - w[0] <= 1), "{moving:?}");
        assert_eq!(*moving.last().unwrap(), s.len());
    }

    #[test]
    fn a_refracted_shot_stops_waits_and_comes_on_faster_at_where_you_are_then() {
        let c = content();
        let l = open_floor();
        let v = &pattern("refract")[0];
        let r = v.rehold.unwrap();
        let mut o = fire(v, v3(0, int(2), 0), v3(0, int(2), -int(30)), 0, 0, &c.pal, &l).orbs[2].clone();
        for _ in 0..=r.stop {
            o.step(&l, V3::ZERO, V3::ZERO);
        }
        assert_eq!(o.speed, 0, "it stops");
        let there = o.pos;
        // you moved: it turns to where you are when it sets off
        let now = there.add(v3(int(10), 0, 0));
        for _ in 0..r.wait - 1 {
            o.step(&l, now, V3::ZERO);
        }
        assert_eq!(o.pos, there, "and waits");
        o.step(&l, now, V3::ZERO);
        assert!(o.dir.x > ONE * 9 / 10, "{:?}", o.dir);
        assert!(o.speed > v.speed, "{} vs {}", o.speed, v.speed);
    }

    #[test]
    fn blue_orbs_are_the_cyan_and_cobalt_ones() {
        let c = content();
        let blue: Vec<bool> = c
            .patterns
            .iter()
            .flat_map(|p| p.volleys.iter().map(|v| v.blue))
            .collect();
        assert!(blue.iter().any(|b| *b) && blue.iter().any(|b| !*b));
        for v in c.patterns.iter().flat_map(|p| &p.volleys) {
            assert_eq!(v.blue, v.colour == "cyan" || v.colour == "blue", "{}", v.colour);
        }
    }
}
