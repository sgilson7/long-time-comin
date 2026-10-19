//! The Hurricane: the Drowned Fleet sailing in line through the storm, six
//! ships in a column along −z, each heaving and swaying on the swell out
//! of step with the ships either side, so the water between them opens and
//! closes. You work forward from the rearmost ship to the flagship's bow:
//! the hook onto a mast as it swings past, Bound Over when two decks line
//! up, gusts that shove you while their rain slants, lightning on the decks
//! and harpies out of the cloud. A rogue wave rolls under the line every
//! fourteen seconds, lifting each ship seven metres in turn, and green
//! washes run across the decks: jump them or go over the side. At the flagship's bow a wave the size of a
//! cliff rises over everything: the way on.

use super::super::{boxed, dm, m, B};
use crate::balance::*;
use crate::course::{Rogue, Step, Wash};
use crate::fx::{deg, ONE};
use crate::geom::{v3, V3};
use crate::level::Solid;
use crate::mesh::Mesh;

/// the swell: every ship heaves and sways once in it, in ticks
const SWELL: i32 = 4 * 60;
/// a deck's top at the bottom of its heave
const LOW: i32 = 2 * ONE;
/// a deck's half-width
const BEAM: i32 = 5 * ONE;

/// One ship of the line: its deck from `stern` forward to `bow` (both z),
/// how far it heaves and sways, where in the swell it starts (degrees),
/// and its masts (z, height).
struct Hull {
    stern: i32,
    bow: i32,
    rise: i32,
    swing: V3,
    phase: i32,
    masts: &'static [(i32, i32)],
}

impl Hull {
    /// its deck's top, halfway up its heave
    fn mean(&self) -> i32 {
        LOW + self.rise / 2
    }

    /// its deck's top at the top of its heave: where its post stands, so
    /// whoever the water returns there drops onto the deck, never into it
    fn crest(&self) -> i32 {
        LOW + self.rise
    }
}

/// on a ship swaying across the line, the walk along its deck keeps this
/// far to starboard, clear of its masts and short of its rail
const WALK: i32 = 5 * ONE / 2;

/// The line, rearmost first. The phases are set against each other: a deck
/// you leave by the hook is falling as you go (one rising under you as the
/// hook lifts you would catch your feet), and a deck you Bound onto is
/// falling, below you and its stern coming back toward you as you land.
const LINE: [Hull; 6] = [
    // the rearmost, where you come aboard from the boat: an easy heave and
    // no sway; it starts at the top of its heave, level with the boat
    Hull {
        stern: 8 * ONE,
        bow: -30 * ONE,
        rise: 2 * ONE,
        swing: V3::ZERO,
        phase: 180,
        masts: &[(-3 * ONE, 14 * ONE), (-16 * ONE, 16 * ONE)],
    },
    // swinging across your bow: hook its foremast as it swings out to
    // starboard and lingers there
    Hull {
        stern: -44 * ONE,
        bow: -80 * ONE,
        rise: 3 * ONE,
        swing: V3 {
            x: 3 * ONE / 2,
            y: 0,
            z: 0,
        },
        phase: 0,
        masts: &[(-49 * ONE, 15 * ONE), (-66 * ONE, 18 * ONE)],
    },
    // pitching fore and aft: its stern comes back toward the second's bow
    // as it falls, as you reach that bow, and you Bound Over then
    Hull {
        stern: -92 * ONE,
        bow: -128 * ONE,
        rise: 3 * ONE,
        swing: V3 {
            x: 0,
            y: 0,
            z: -5 * ONE / 2,
        },
        phase: 27,
        masts: &[(-100 * ONE, 16 * ONE), (-112 * ONE, 15 * ONE)],
    },
    // heaving hard and swinging across the bow of the one before: hook its
    // foremast as it swings out
    Hull {
        stern: -144 * ONE,
        bow: -180 * ONE,
        rise: 4 * ONE,
        swing: V3 {
            x: 3 * ONE / 2,
            y: 0,
            z: 0,
        },
        phase: 292,
        masts: &[(-150 * ONE, 15 * ONE), (-166 * ONE, 18 * ONE)],
    },
    // pitching again, falling as you reach the fourth's bow; a gust at your
    // back over the gap: Bound Over with it
    Hull {
        stern: -192 * ONE,
        bow: -228 * ONE,
        rise: 3 * ONE,
        swing: V3 {
            x: 0,
            y: 0,
            z: -2 * ONE,
        },
        phase: 313,
        masts: &[(-200 * ONE, 15 * ONE), (-212 * ONE, 16 * ONE)],
    },
    // the flagship: hook its foremast as it swings out, run to its bow
    Hull {
        stern: -246 * ONE,
        bow: -300 * ONE,
        rise: 2 * ONE,
        swing: V3 {
            x: 3 * ONE / 2,
            y: 0,
            z: 0,
        },
        phase: 133,
        masts: &[
            (-252 * ONE, 16 * ONE),
            (-276 * ONE, 20 * ONE),
            (-290 * ONE, 15 * ONE),
        ],
    },
];

/// the rogue wave's way across the line (for each metre to starboard, this
/// far along it), and where in its cycle it starts
const ROGUE_SLANT: i32 = -ONE / 5;
const ROGUE_PHASE: i32 = 0;

/// The green washes: the ship (by its place in the line), the middle of
/// the stretch of its deck it comes over (z) and half that stretch's length,
/// which way it runs (+1 to starboard), how often, and where in its cycle it
/// starts. One over each deck, each timed to come aboard just ahead of the
/// line or just behind it (jump it, if you are caught).
const WASHES: [(usize, i32, i32, i32, i32, i32); 6] = [
    (0, -11 * ONE, 8 * ONE, 1, 6 * 60, 100),
    (1, -71 * ONE, 8 * ONE, -1, 6 * 60, 80),
    (2, -119 * ONE, 8 * ONE, 1, 6 * 60, 180),
    (3, -162 * ONE, 8 * ONE, -1, 6 * 60, 80),
    (4, -210 * ONE, 8 * ONE, 1, 6 * 60, 60),
    (5, -273 * ONE, 8 * ONE, -1, 7 * 60, 80),
];

impl B<'_> {
    pub(super) fn hurricane(&mut self) {
        self.k().fought = true;
        for h in &LINE {
            let masts: Vec<(i32, i32, i32)> = h.masts.iter().map(|&(z, ht)| (0, z, ht)).collect();
            self.ship(
                -BEAM,
                h.stern,
                BEAM,
                h.bow,
                LOW,
                &masts,
                h.rise,
                h.swing,
                SWELL,
                deg(h.phase),
            );
            // where the waves come aboard: along the deck, over it
            let mut z = h.stern - m(6);
            while z > h.bow + m(4) {
                self.k().spawns.push(v3(0, h.mean() + m(1), z));
                z -= m(12);
            }
        }
        let [a, b, c, d, e, f] = &LINE;

        // 2a: the rearmost ship, come aboard from the boat; the next swings
        // across its bow, and the hook takes its foremast as it swings out
        self.jolly_boat(a.stern, a.crest());
        self.mooring(0, a.crest(), a.stern + m(5));
        self.line(Step::To(v3(0, a.mean(), a.stern - m(4))));
        self.hook_across(a, b, 1, 40);

        // 2b: lightning on the deck; then Bound Over to the next as its
        // stern pitches back toward you
        self.bound_across(b, c);
        self.crew_member(v3(0, c.mean() + m(1), c.bow + m(6)), "deckhand");

        // 2c: the hook again, onto a ship heaving four metres
        self.hook_across(c, d, 1, 17);

        // 2d: a gust at your back over the gap: Bound Over with it
        self.bound_across(d, e);
        self.crew_member(v3(0, e.mean() + m(1), e.bow + m(8)), "deckhand");

        // 2e: the flagship: its foremast, then its bow
        self.hook_across(e, f, 1, 21);
        self.line(Step::To(v3(WALK, f.mean(), f.masts[2].0 - m(4))));
        self.line(Step::To(v3(0, f.mean(), f.bow + m(4))));

        // 2f: the way on at the flagship's bow; a fixed gate, so it stands
        // on the deck's mean
        self.way_on(0, f.mean(), f.bow + m(2));

        // the gusts: across the second ship's deck, at your back over the
        // fourth gap, and across the flagship
        self.wind_gust(
            v3(-m(12), 0, b.masts[1].0 - m(4)),
            v3(m(12), m(30), b.masts[0].0 - m(9)),
            v3(ONE, 0, 0),
            ONE * 4 / 100,
            3 * 60,
            60,
            0,
        );
        self.wind_gust(
            v3(-m(12), 0, e.stern - m(4)),
            v3(m(12), m(30), d.bow - m(1)),
            v3(0, 0, -ONE),
            ONE * 5 / 100,
            4 * 60,
            90,
            30,
        );
        self.wind_gust(
            v3(-m(12), 0, f.masts[2].0 + m(4)),
            v3(m(12), m(30), f.masts[0].0 - m(9)),
            v3(-ONE, 0, 0),
            ONE * 3 / 100,
            3 * 60,
            60,
            90,
        );
        self.thunder_storm(3 * 60, 60, dm(25), 15);

        // the waves: a rogue wave rolling across the line from port, and
        // green washes over the decks
        self.rogue_wave(v3(ONE, 0, ROGUE_SLANT), ROGUE_PHASE);
        for &(n, z, reach, side, period, phase) in &WASHES {
            self.green_wash(&LINE[n], z, reach, side, period, phase);
        }
        let k = self.k();
        let movers = k.movers.clone();
        k.waves.anchor(&movers);

        // the strand: a capsized hulk off the third ship's beam, still in
        // the water; Bound Over to it and back
        let hulk = v3(m(17), LOW, (c.stern + c.bow) / 2);
        self.solids.push(Solid::new(
            v3(hulk.x - m(4), -m(6), hulk.z - m(8)),
            v3(hulk.x + m(4), hulk.y, hulk.z + m(8)),
        ));
        let keel = self.c("hull_dark");
        self.deco(
            Mesh::Blob,
            v3(hulk.x, hulk.y - m(2), hulk.z),
            v3(m(9), m(5), m(17)),
            keel,
        );
        self.strand_reward(hulk.x, hulk.y, hulk.z);

        self.storm_overhead();
        self.cliff_wave(f.bow - m(30));
        self.sea_surface(-m(120), m(40), m(120), -m(380));
    }

    /// From the bow of `from` (after `wait` ticks), the hook onto `to`'s
    /// foremast as it swings out to `side` (+1 starboard, −1 port) and
    /// lingers there, down onto its deck beyond the mast, and its post. Its
    /// mainmast stands out of the hook's line, so nothing else is caught.
    fn hook_across(&mut self, from: &Hull, to: &Hull, side: i32, wait: i32) {
        let mast = to.masts[0].0;
        let out = to.swing.x * side;
        self.hold(from, wait);
        self.line(Step::To(v3(0, from.mean(), from.bow + m(7))));
        self.line(Step::Hook(v3(out, to.mean() + m(7), mast)));
        self.line(Step::Air(v3(out / 2, to.mean(), mast - m(3))));
        self.mooring(WALK, to.crest(), mast - m(6));
        self.line(Step::To(v3(WALK, to.mean(), mast - m(6))));
    }

    /// Stand in the lee of `h`'s mast nearest its bow for `wait` ticks,
    /// until the moment comes, then step round it. (Come up behind the mast:
    /// whatever way is left on you carries you into it, not over the side.)
    fn hold(&mut self, h: &Hull, wait: i32) {
        let mast = h.masts[h.masts.len() - 1].0;
        self.line(Step::To(v3(m(2), h.mean(), mast + m(5))));
        self.line(Step::To(v3(dm(4), h.mean(), mast + dm(25))));
        self.line(Step::To(v3(dm(4), h.mean(), mast + dm(8))));
        self.line(Step::Wait(wait));
        self.line(Step::To(v3(m(2), h.mean(), mast - m(1))));
    }

    /// Along `from`'s deck to its bow and Bound Over onto `to`'s stern, and
    /// its post. No waiting on a deck that sways across the line: `to`'s
    /// swell is set so that it comes right as you reach the bow.
    fn bound_across(&mut self, from: &Hull, to: &Hull) {
        self.line(Step::To(v3(WALK, from.mean(), from.bow + m(4))));
        self.line(Step::To(v3(0, from.mean(), from.bow + dm(6))));
        self.line(Step::Bound(v3(0, to.mean(), to.stern - m(6))));
        self.mooring(m(3), to.crest(), to.stern - m(10));
        self.line(Step::To(v3(m(3), to.mean(), to.stern - m(10))));
    }

    /// A rogue wave rolling along `dir` across the whole line, and on past
    /// it before it comes again: so it lifts no ship as its cycle turns.
    fn rogue_wave(&mut self, dir: V3, phase: i32) {
        let dir = dir.norm();
        let mut lo = i32::MAX;
        for h in &LINE {
            let (sx, sz) = (BEAM + h.swing.x.abs(), h.swing.z.abs());
            for (x, z) in [
                (-sx, h.stern + sz),
                (sx, h.stern + sz),
                (-sx, h.bow - sz),
                (sx, h.bow - sz),
            ] {
                let d = v3(x, 0, z).dot(dir);
                lo = lo.min(d);
            }
        }
        let speed = ROGUE_SPEED;
        let start = lo - ROGUE_HALF - speed * ROGUE_WARN;
        let mid = v3(0, 0, (LINE[0].stern + LINE[5].bow) / 2);
        let from = mid.add(dir.scale(start - mid.dot(dir)));
        self.k().waves.rogues.push(Rogue {
            from,
            dir,
            speed,
            period: ROGUE_PERIOD,
            phase,
            height: ROGUE_HEIGHT,
            half: ROGUE_HALF,
            reach: m(260),
        });
    }

    /// A green wash over `h`'s deck, the stretch `reach` either way of `z`,
    /// running across it to `side` (+1 starboard) every `period` ticks.
    fn green_wash(&mut self, h: &Hull, z: i32, reach: i32, side: i32, period: i32, phase: i32) {
        let edge = BEAM + h.swing.x.abs() + m(1);
        let run = 2 * edge / WASH_SPEED;
        self.k().waves.washes.push(Wash {
            from: v3(-edge * side, h.mean(), z),
            dir: v3(ONE * side, 0, 0),
            speed: WASH_SPEED,
            run,
            period,
            phase,
            reach,
        });
    }

    /// The boat you come aboard from, astern of the rearmost ship: still
    /// on the swell but lifted with the rest when a rogue wave comes
    /// through, its top at `top`, its bow at `z`.
    fn jolly_boat(&mut self, z: i32, top: i32) {
        self.ship_deck(
            v3(-m(2), top - m(1), z),
            v3(m(2), top, z + m(8)),
            0,
            V3::ZERO,
            300,
            0,
        );
    }

    /// The storm's dark: heavy cloud low over the line.
    fn storm_overhead(&mut self) {
        let cloud = self.c("storm_cloud");
        let sky = self.c("storm_sky");
        let mut z = m(30);
        let mut k = 0;
        while z > -m(380) {
            let x = self.metres(-40, 40);
            let y = self.metres(30, 42);
            let w = self.metres(40, 70);
            let d = self.metres(30, 50);
            let rgb = if k % 3 == 0 { sky } else { cloud };
            self.shape(Mesh::Blob, boxed(v3(x, y, z), v3(w, m(10), d)), rgb, 0, false);
            z -= m(22);
            k += 1;
        }
    }

    /// The wave the size of a cliff beyond the flagship's bow, at `z`.
    fn cliff_wave(&mut self, z: i32) {
        let water = self.c("water");
        let foam = self.c("river_foam");
        let deep = self.c("ocean");
        self.shape(
            Mesh::Blob,
            boxed(v3(0, m(30), z), v3(m(260), m(70), m(40))),
            water,
            0,
            true,
        );
        self.shape(
            Mesh::Blob,
            boxed(v3(0, m(8), z + m(14)), v3(m(240), m(20), m(30))),
            deep,
            0,
            false,
        );
        // its crest, curling over toward you
        self.shape(
            Mesh::Blob,
            boxed(v3(0, m(62), z + m(16)), v3(m(250), m(10), m(26))),
            foam,
            0,
            true,
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::course::tests::course_world_in;
    use crate::course::{Look, Pilot};
    use crate::fx::ONE;
    use crate::world::{Outcome, World};

    /// Drives a body along the Hurricane's line as `cross_in` does,
    /// fighting, after `set` has had the world, showing `watch` the world
    /// each tick: the ticks it took and the health it lost, or where it went
    /// wrong.
    pub(crate) fn cross_watching(
        mut set: impl FnMut(&mut World),
        mut watch: impl FnMut(&World, usize),
    ) -> Result<(i32, i32), String> {
        let (c, mut w) = course_world_in("far_shore", "hurricane");
        set(&mut w);
        let line = w.course.as_ref().unwrap().line.clone();
        let mut pilot = Pilot::default();
        let start = w.player.health;
        for tick in 0..60 * 60 * 4 {
            let at = w.player.pos;
            let reach = if pilot.step >= line.len() { 90 } else { 30 } * ONE;
            for e in w
                .enemies
                .iter_mut()
                .filter(|e| e.alive() && e.pos.dist(at) < reach)
            {
                e.hp = 0;
            }
            let d = match pilot.drive(&line, &w.player) {
                Some(d) => d,
                None => {
                    // the line done: on to the way on, once it opens
                    let to = w.course.as_ref().unwrap().finish.sub(w.player.pos);
                    let yaw = crate::geom::yaw_pitch_of(to).0;
                    crate::course::Drive {
                        controls: crate::player::Controls {
                            move_z: ONE,
                            ..Default::default()
                        },
                        yaw,
                        aim: crate::geom::facing(yaw, 0),
                    }
                }
            };
            w.player.yaw = d.yaw;
            let eye = w.player.chest();
            let before = w.notes.iter().filter(|n| n.sound == "drown").count();
            w.tick(&c, &d.controls, eye, d.aim);
            watch(&w, pilot.step);
            if w.notes.iter().filter(|n| n.sound == "drown").count() > before {
                return Err(format!(
                    "drowned on step {} ({:?}) at tick {tick}, from {:?}",
                    pilot.step,
                    line.get(pilot.step),
                    w.player.prev_pos
                ));
            }
            if pilot.t > 60 * 20 {
                return Err(format!(
                    "stuck on step {} ({:?}) at {:?}",
                    pilot.step,
                    line.get(pilot.step),
                    w.player.pos
                ));
            }
            if matches!(w.outcome, Outcome::Through(_)) {
                return Ok((tick, start - w.player.health));
            }
        }
        Err("out of time".into())
    }

    #[test]
    fn the_hurricane_can_be_crossed_on_its_line() {
        let (ticks, _) = crate::course::tests::cross_in("far_shore", "hurricane", true).unwrap();
        assert!(ticks < 60 * 200, "{ticks}");
    }

    #[test]
    fn the_hurricane_is_crossed_through_its_waves_a_rogue_lifting_the_ships_and_washes_over_the_decks() {
        use crate::balance::*;
        let mut lifted = 0;
        let mut carried = 0;
        let mut ran = Vec::new();
        let (ticks, lost) = cross_watching(
            |_| {},
            |w, _| {
                let k = w.course.as_ref().unwrap();
                let decks = (0..k.movers.len()).filter(|&i| k.movers[i].look == Look::Deck);
                lifted = decks
                    .map(|i| k.waves.lift_of(i, w.tick))
                    .max()
                    .unwrap_or(0)
                    .max(lifted);
                if w.player.grounded {
                    let p = w.player.pos;
                    carried = carried.max(k.waves.lift_under(&k.movers, p, w.tick));
                }
                ran.resize(k.waves.washes.len(), false);
                for (n, s) in k.waves.washes.iter().enumerate() {
                    ran[n] |= s.until(w.tick) == Some(0);
                }
            },
        )
        .unwrap();
        assert!(
            lifted > ROGUE_HEIGHT - ONE / 10,
            "a ship lifted the rogue's height: {lifted}"
        );
        assert!(carried > 2 * ONE, "the line itself lifted by it: {carried}");
        assert!(
            !ran.is_empty() && ran.iter().all(|&r| r),
            "every wash came aboard: {ran:?}"
        );
        assert!(ticks < 60 * 200, "{ticks}");
        let _ = lost;
    }

    #[test]
    fn a_rogue_wave_lifts_no_ship_as_its_cycle_turns() {
        let (_, w) = course_world_in("far_shore", "hurricane");
        let k = w.course.as_ref().unwrap();
        let r = k.waves.rogues[0];
        assert!(
            k.waves.anchors.iter().all(|a| a.is_some()),
            "every mover tied to its ship"
        );
        for (i, _) in k.movers.iter().enumerate() {
            for tick in [r.period - r.phase - 1, r.period - r.phase] {
                assert_eq!(k.waves.lift_of(i, tick), 0, "mover {i} at {tick}");
            }
        }
    }

    #[test]
    fn the_boat_you_start_on_rides_the_rogue_wave_with_the_ships() {
        let (_, w) = course_world_in("far_shore", "hurricane");
        let k = w.course.as_ref().unwrap();
        let start = k.posts[0];
        let (i, boat) = k
            .movers
            .iter()
            .enumerate()
            .find(|(_, m)| {
                let b = &m.base;
                m.look == Look::Deck
                    && b.min.x <= start.x
                    && start.x <= b.max.x
                    && b.min.z <= start.z
                    && start.z <= b.max.z
            })
            .expect("the boat under the start is a deck on the sea");
        let most = (0..crate::balance::ROGUE_PERIOD)
            .map(|t| k.waves.corner(i, boat, t).y - boat.corner(t).y)
            .max()
            .unwrap_or(0);
        assert!(most > crate::balance::ROGUE_HEIGHT / 2, "the boat lifted {most}");
    }
}
