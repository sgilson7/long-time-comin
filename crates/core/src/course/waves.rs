//! The sea's waves over a course (the Hurricane). Two kinds:
//!
//! - a rogue wave: a swell `height` high rolling across the fleet on a long
//!   cycle, rising out of the sea first as a dark line (its warning). As it
//!   passes under a ship it lifts it, deck, masts and whoever stands there,
//!   so for a few seconds one deck stands metres over the next. In the
//!   water it shoves a swimmer along its way.
//! - a green wash: a low wall of green water sweeping across a deck, its
//!   foam line showing first where it will come aboard. Feet under its
//!   height over the floor beneath them, inside it, are shoved along its way
//!   (over the side, likely) and hurt, once a wash. Jump it.

use super::{Course, Look, Mover};
use crate::balance::*;
use crate::content::Content;
use crate::fx::{self, ONE};
use crate::geom::{v3, V3};
use crate::world::World;

/// A rogue wave. Each cycle of `period` ticks its crest starts at `from`,
/// rises there through `ROGUE_WARN`, and rolls along `dir` at `speed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rogue {
    pub from: V3,
    /// a flat unit vector
    pub dir: V3,
    /// metres a tick
    pub speed: i32,
    pub period: i32,
    pub phase: i32,
    pub height: i32,
    /// from its crest to its foot, either way along `dir`
    pub half: i32,
    /// how far its crest runs either way of its middle
    pub reach: i32,
}

impl Rogue {
    /// Ticks into its cycle.
    pub fn age(&self, tick: i32) -> i32 {
        (tick + self.phase).rem_euclid(self.period.max(1))
    }

    /// Its height now: rising out of the sea through its warning.
    pub fn tall(&self, tick: i32) -> i32 {
        let a = self.age(tick).min(ROGUE_WARN);
        (self.height as i64 * a as i64 / ROGUE_WARN.max(1) as i64) as i32
    }

    /// The middle of its crest now.
    pub fn crest(&self, tick: i32) -> V3 {
        let run = (self.speed as i64 * self.age(tick) as i64) as i32;
        self.from.add(self.dir.scale(run))
    }

    /// How far `at` stands ahead of its crest, and how far along it.
    fn place(&self, at: V3, tick: i32) -> (i32, i32) {
        let d = at.sub(self.crest(tick));
        let d = v3(d.x, 0, d.z);
        let ahead = d.dot(self.dir);
        let along = d.dot(v3(-self.dir.z, 0, self.dir.x));
        (ahead, along)
    }

    /// Its share of its height over `at` (Q16), smooth from its foot up to
    /// its crest and down again.
    pub fn share(&self, at: V3, tick: i32) -> i32 {
        let (ahead, along) = self.place(at, tick);
        if ahead.abs() >= self.half || along.abs() > self.reach {
            return 0;
        }
        let a = (fx::HALF as i64 * ahead as i64 / self.half.max(1) as i64) as i32;
        (ONE + fx::cos(a)) / 2
    }

    /// How far it lifts what floats at `at`.
    pub fn lift(&self, at: V3, tick: i32) -> i32 {
        fx::mul(self.tall(tick), self.share(at, tick))
    }
}

/// A green wash. Each cycle of `period` ticks its foam line shows at
/// `from` through `WASH_WARN`, then it runs along `dir` at `speed` for
/// `run` ticks, a wall `WASH_HEIGHT` over the floor and `WASH_THICK` deep,
/// its front `reach` either way of its middle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wash {
    pub from: V3,
    pub dir: V3,
    pub speed: i32,
    pub run: i32,
    pub period: i32,
    pub phase: i32,
    pub reach: i32,
}

impl Wash {
    pub fn age(&self, tick: i32) -> i32 {
        (tick + self.phase).rem_euclid(self.period.max(1))
    }

    /// Which wash of its cycles this is.
    pub fn cycle(&self, tick: i32) -> i32 {
        (tick + self.phase).div_euclid(self.period.max(1))
    }

    /// Ticks until it runs (0: running), or none once it has run.
    pub fn until(&self, tick: i32) -> Option<i32> {
        let a = self.age(tick);
        if a < WASH_WARN {
            Some(WASH_WARN - a)
        } else if a < WASH_WARN + self.run {
            Some(0)
        } else {
            None
        }
    }

    /// The middle of its front now (at `from` until it runs).
    pub fn front(&self, tick: i32) -> V3 {
        let a = (self.age(tick) - WASH_WARN).clamp(0, self.run);
        let run = (self.speed as i64 * a as i64) as i32;
        self.from.add(self.dir.scale(run))
    }

    /// Is the column over `at` inside it now?
    pub fn over(&self, at: V3, tick: i32) -> bool {
        if self.until(tick) != Some(0) {
            return false;
        }
        let d = at.sub(self.front(tick));
        let d = v3(d.x, 0, d.z);
        let ahead = d.dot(self.dir);
        let along = d.dot(v3(-self.dir.z, 0, self.dir.x));
        ahead.abs() <= WASH_THICK / 2 + PLAYER_RADIUS && along.abs() <= self.reach
    }
}

/// A course's waves, and the point each mover's lift is read at.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Waves {
    pub rogues: Vec<Rogue>,
    pub washes: Vec<Wash>,
    /// by mover: the middle of its ship's deck, so a ship and its masts
    /// rise together (none: the waves do not lift it)
    pub anchors: Vec<Option<V3>>,
    /// by wash: the cycle it last struck you in
    pub struck: Vec<i32>,
}

impl Waves {
    /// How far the rogue waves lift what floats at `at`.
    pub fn lift(&self, at: V3, tick: i32) -> i32 {
        self.rogues.iter().map(|r| r.lift(at, tick)).max().unwrap_or(0)
    }

    /// How far they lift mover `i`.
    pub fn lift_of(&self, i: usize, tick: i32) -> i32 {
        match self.anchors.get(i) {
            Some(Some(a)) => self.lift(*a, tick),
            _ => 0,
        }
    }

    /// Where mover `i` stands at `tick`: its own heave and sway, and the
    /// rogue waves' lift.
    pub fn corner(&self, i: usize, m: &Mover, tick: i32) -> V3 {
        m.corner(tick).add(v3(0, self.lift_of(i, tick), 0))
    }

    /// Ties each mover to the ship it belongs to: a deck to itself, a mast
    /// to the deck it stands on. Called once its movers are all laid.
    pub fn anchor(&mut self, movers: &[Mover]) {
        let decks: Vec<_> = movers
            .iter()
            .filter(|m| m.look == Look::Deck)
            .map(|m| m.base)
            .collect();
        self.anchors = movers
            .iter()
            .map(|m| {
                let c = m.base.centre();
                decks
                    .iter()
                    .find(|d| c.x >= d.min.x && c.x <= d.max.x && c.z >= d.min.z && c.z <= d.max.z)
                    .map(|d| d.centre())
            })
            .collect();
    }

    /// How far they lift the deck under `at` (where the water returns you).
    pub fn lift_under(&self, movers: &[Mover], at: V3, tick: i32) -> i32 {
        movers
            .iter()
            .enumerate()
            .filter(|(_, m)| m.look == Look::Deck)
            .filter(|(_, m)| {
                let (lo, hi, s) = (m.base.min, m.base.max, m.swing);
                at.x >= lo.x - s.x.abs()
                    && at.x <= hi.x + s.x.abs()
                    && at.z >= lo.z - s.z.abs()
                    && at.z <= hi.z + s.z.abs()
            })
            .map(|(i, _)| self.lift_of(i, tick))
            .max()
            .unwrap_or(0)
    }
}

impl World {
    /// One tick of the sea's waves, after the wind and the current: a rogue
    /// wave's warning, its shove on a swimmer, and a wash's on whoever it
    /// catches standing.
    pub(crate) fn sea_step(&mut self, c: &Content, k: &mut Course, said: &mut Vec<(&'static str, String)>) {
        let tick = self.tick;
        let p = self.player.pos;
        for r in &k.waves.rogues {
            if r.age(tick) == 0 {
                said.push(("rogue_wave", c.say("callout_rogue_wave")));
            }
            if self.player.swimming && !self.player.grounded {
                let s = fx::mul(r.share(p, tick), fx::div(r.tall(tick), r.height.max(1)));
                if s > 0 {
                    let push = r.dir.scale(fx::mul(ROGUE_SWIM_PUSH, s));
                    self.player.shove = self.player.shove.add(v3(push.x, 0, push.z));
                }
            }
        }
        let n = k.waves.washes.len();
        k.waves.struck.resize(n, i32::MIN);
        for (n, w) in k.waves.washes.iter().enumerate() {
            if !w.over(p, tick) {
                continue;
            }
            // the floor beneath you, if any: the wash runs over it
            let Some(floor) = self.arena.level.floor_below(p.x, p.y + ONE / 20, p.z) else {
                continue;
            };
            if p.y - floor >= WASH_HEIGHT {
                continue;
            }
            let push = w.dir.scale(WASH_PUSH);
            self.player.shove = self.player.shove.add(v3(push.x, 0, push.z));
            let cycle = w.cycle(tick);
            if k.waves.struck[n] != cycle {
                k.waves.struck[n] = cycle;
                // taken off your feet and thrown along its way
                let pl = &mut self.player;
                pl.release_grapple();
                let knock = w.dir.scale(WASH_KNOCK);
                pl.vel = v3(pl.vel.x + knock.x, pl.vel.y.max(WASH_HEAVE), pl.vel.z + knock.z);
                pl.grounded = false;
                pl.hurt(WASH_DAMAGE);
                said.push(("wash", String::new()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::tests::course_world;
    use crate::course::Underwater;
    use crate::fx::int;
    use crate::level::{Dyn, Solid};
    use crate::player::Controls;

    fn tick(c: &Content, w: &mut World, ctl: Controls) {
        let eye = w.player.chest();
        let fwd = crate::geom::facing(w.player.yaw, 0);
        w.tick(c, &ctl, eye, fwd);
    }

    /// A plank in the open air over the spillway's race, its top at 18 m
    /// over x −10..10, z −50..−38: its index among the level's changing
    /// solids.
    fn plank(w: &mut World) -> (usize, crate::level::Solid) {
        let base = Solid::new(v3(-int(10), int(16), -int(50)), v3(int(10), int(18), -int(38)));
        let i = w.arena.level.add_dynamic(Dyn {
            solid: base,
            on: true,
            one_way: false,
        });
        (i, base)
    }

    fn stand(w: &mut World, at: V3) {
        w.player.pos = at;
        w.player.prev_pos = at;
        w.player.vel = V3::ZERO;
        w.player.grounded = true;
    }

    #[test]
    fn a_rogue_wave_lifts_a_deck_by_its_height_as_it_passes_and_carries_who_stands_on_it() {
        let (c, mut w) = course_world("spillway");
        let (i, base) = plank(&mut w);
        let k = w.course.as_mut().unwrap();
        let mut deck = Mover::breathing(i, base, 0, 600, 0);
        deck.look = Look::Deck;
        k.movers.push(deck);
        // rolling along +x, its crest 60 m short of the deck as it rises
        k.waves.rogues.push(Rogue {
            from: v3(-int(60), 0, -int(44)),
            dir: v3(ONE, 0, 0),
            speed: ROGUE_SPEED,
            period: 60 * 60,
            phase: -w.tick,
            height: ROGUE_HEIGHT,
            half: ROGUE_HALF,
            reach: int(50),
        });
        let movers = k.movers.clone();
        k.waves.anchor(&movers);
        let feet = v3(0, base.max.y, -int(44));
        stand(&mut w, feet);
        let (mut top_most, mut feet_most) = (0, 0);
        for n in 0..60 * 12 {
            tick(&c, &mut w, Controls::default());
            let top = w.arena.level.dynamic[i].solid.max.y;
            let p = w.player.pos;
            assert!(
                (p.y - top).abs() <= ONE / 20 && p.sub(feet).len_xz() < ONE / 4,
                "tick {n}: carried, not left behind: feet {p:?}, the deck's top {top}"
            );
            if n < 60 {
                assert!(top - base.max.y < ONE / 10, "not lifted before it comes");
            }
            top_most = top_most.max(top - base.max.y);
            feet_most = feet_most.max(p.y - feet.y);
        }
        let rose = w.arena.level.dynamic[i].solid.max.y - base.max.y;
        assert!(top_most > ROGUE_HEIGHT - ONE / 10, "lifted {top_most}");
        assert!(feet_most > ROGUE_HEIGHT - ONE / 5, "carried up {feet_most}");
        assert!(rose < ONE / 10, "and set down again once it has passed: {rose}");
    }

    /// A wash across the plank at a body standing on it (jumping it as its
    /// front comes, when `jump`): where the body ends, how high it was thrown
    /// off the plank's top, the health it lost, and the crashes heard.
    fn washed(jump: bool) -> (V3, i32, i32, usize) {
        let (c, mut w) = course_world("spillway");
        let (_, base) = plank(&mut w);
        let feet = v3(0, base.max.y, -int(44));
        stand(&mut w, feet);
        w.player.build.max_health = 1000;
        w.player.health = 1000;
        let wash = Wash {
            from: v3(-int(9), base.max.y, -int(44)),
            dir: v3(ONE, 0, 0),
            speed: WASH_SPEED,
            run: int(18) / WASH_SPEED,
            period: 60 * 60,
            phase: -w.tick,
            reach: int(6),
        };
        w.course.as_mut().unwrap().waves.washes.push(wash);
        let mut jumped = false;
        let mut thrown = 0;
        for _ in 0..60 * 4 {
            // jump as the front comes near: over it at the top
            let ahead = w.player.pos.x - wash.front(w.tick).x;
            let go = jump && !jumped && wash.until(w.tick) == Some(0) && ahead < int(2) + ONE / 2;
            jumped |= go;
            tick(
                &c,
                &mut w,
                Controls {
                    jump_pressed: go,
                    ..Controls::default()
                },
            );
            if !jump {
                thrown = thrown.max(w.player.pos.y - base.max.y);
            }
        }
        let n = w.notes.iter().filter(|n| n.sound == "wash").count();
        (w.player.pos, thrown, 1000 - w.player.health, n)
    }

    #[test]
    fn a_wash_shoves_and_hurts_a_body_standing_in_it_once_but_not_one_jumping_it() {
        let (p, thrown, lost, n) = washed(false);
        assert_eq!((lost, n), (WASH_DAMAGE, 1), "hurt once, one crash");
        assert!(thrown > ONE / 5, "taken off its feet: {thrown}");
        assert!(p.x > int(10) && p.y < int(16), "over the side: {p:?}");

        let (p, _, lost, n) = washed(true);
        assert_eq!((lost, n), (0, 0), "jumped it");
        assert!(
            p.x.abs() < ONE && p.y >= int(18) - ONE / 20,
            "still on the plank: {p:?}"
        );
    }

    #[test]
    fn a_rogue_wave_shoves_a_swimmer_along_its_way() {
        let swum = |rogue: bool| {
            let (c, mut w) = course_world("spillway");
            let k = w.course.as_mut().unwrap();
            k.underwater = Some(Underwater {
                gravity_pct: SWIM_GRAVITY_PCT,
                eye: v3(0, 0, -int(20)),
                pull: 0,
            });
            k.water = -int(100);
            let tick0 = w.tick;
            if rogue {
                // risen, its crest just short of the swimmer, rolling on
                let dir = v3(ONE, 0, -ONE).norm();
                k.waves.rogues.push(Rogue {
                    from: v3(-int(4), 0, -int(16)).sub(dir.scale(ROGUE_SPEED * ROGUE_WARN)),
                    dir,
                    speed: ROGUE_SPEED,
                    period: 60 * 60,
                    phase: ROGUE_WARN - tick0,
                    height: ROGUE_HEIGHT,
                    half: ROGUE_HALF,
                    reach: int(50),
                });
            }
            w.player.pos = v3(0, int(40), -int(20));
            w.player.grounded = false;
            w.player.jumps_used = w.player.build.jumps;
            for _ in 0..60 {
                tick(&c, &mut w, Controls::default());
            }
            w.player.pos
        };
        let moved = swum(true).sub(swum(false));
        assert!(
            moved.x > int(3) && moved.z < -int(3),
            "shoved along its way: {moved:?}"
        );
    }
}
