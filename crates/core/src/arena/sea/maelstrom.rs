//! The Maelstrom: the Far Shore's third section, under the water. The
//! fleet's wreckage spirals down and in toward the whirlpool's eye, and you
//! go down with it: from a great hull high in the water, across pieces of
//! wreck turning slowly round the eye and the broken hulls fixed among
//! them, to the last hull on the eye's rim, where the way on stands over
//! the funnel. Gravity is light here and Space strokes you up again; a
//! current draws you toward the eye while you are off your feet. The
//! funnel's bottom drowns, and the Drowned surge between the pieces.
//!
//! Each stop on the way is laid round the eye (`round`): so many degrees
//! from due south, so far out, so high. A piece that turns is laid where
//! it will be when the line reaches it (`adrift`): the tick each is due
//! is the tick the line's body comes down on it, measured with the pieces
//! held still (it crosses with every piece up to 40 ticks early or late).

use super::super::{boxed, dm, m, B};
use crate::balance::per_tick2;
use crate::course::Step;
use crate::fx::{self, deg, ONE};
use crate::geom::{rotate_y, v3, Affine, V3};
use crate::level::Solid;
use crate::mesh::Mesh;

/// the eye of the whirlpool, under the middle of the spiral
const EYE: V3 = v3(0, 0, -120 * ONE);
/// the funnel's bottom: below it you drown (above the fall that returns
/// you to your last footing, 30 m down)
const DROWN: i32 = -24 * ONE;
/// the current toward the eye, while you are off your feet
const PULL: i32 = per_tick2(3, 1);
/// a turning piece goes once round the eye in this many ticks
const TURN_TICKS: i32 = 90 * 60;
/// and bobs this much every this many ticks
const BOB: i32 = ONE * 4 / 10;
const BOB_TICKS: i32 = 4 * 60;

/// A point `a` degrees round the eye from due south, `r` out, at height `y`.
fn round(a: i32, r: i32, y: i32) -> V3 {
    v3(
        EYE.x + fx::mul(fx::sin(deg(a)), r),
        y,
        EYE.z + fx::mul(fx::cos(deg(a)), r),
    )
}

/// How far a turning piece goes round in a tick: against the way down, so
/// a piece you have left draws away behind you instead of passing over the
/// next one while you stand on it.
fn turn() -> i32 {
    -fx::TURN / TURN_TICKS
}

impl B<'_> {
    pub(super) fn maelstrom(&mut self) {
        self.k().water = DROWN;
        self.k().fought = true;
        self.under_water(EYE, PULL, crate::balance::SWIM_GRAVITY_PCT);

        // 3a: the start, high on a great hull at the spiral's outer edge
        let s0 = round(0, m(110), m(80));
        self.hulk(s0, m(16), m(26), &[(m(4), m(6), m(10)), (-m(4), -m(6), m(8))]);
        let start = v3(s0.x, s0.y, s0.z + m(10));
        self.mooring(start.x, start.y, start.z);
        self.line(Step::To(v3(s0.x + m(5), s0.y, s0.z - m(10))));

        // 3b: down across the turning pieces to the first fixed hull
        let d1 = round(9, m(104), m(75));
        self.adrift(d1, m(7), 260);
        self.line(Step::Jump(d1));
        let d2 = round(17, m(98), m(70));
        self.adrift(d2, m(7), 394);
        self.line(Step::Jump(d2));
        let w1 = round(30, m(92), m(64));
        self.hulk(w1, m(14), m(14), &[(m(4), -m(4), m(9))]);
        self.line(Step::Bound(w1));
        self.mooring(w1.x, w1.y, w1.z);
        self.line(Step::To(w1));

        // 3c: the hook across to a hull lower down, its mast beyond where
        // you come down; a stroke steers you onto its deck
        let w2 = round(50, m(84), m(56));
        let mast2 = v3(w2.x + m(3), w2.y, w2.z - m(4));
        self.hulk(w2, m(16), m(16), &[(m(3), -m(4), m(10))]);
        let toward = w2.sub(w1);
        let lip = w1.add(v3(toward.x, 0, toward.z).norm().scale(m(6)));
        self.line(Step::To(lip));
        self.line(Step::Hook(mast2.add(v3(0, m(3), 0))));
        self.line(Step::Jump(w2));
        self.line(Step::To(w2));

        // across two more turning pieces to the third hull
        let d3 = round(60, m(78), m(51));
        self.adrift(d3, m(6), 853);
        self.line(Step::Jump(d3));
        let d4 = round(70, m(73), m(46));
        self.adrift(d4, m(6), 983);
        self.line(Step::Jump(d4));
        let w3 = round(84, m(68), m(40));
        self.hulk(w3, m(14), m(14), &[(-m(4), m(4), m(8))]);
        self.line(Step::Bound(w3));
        self.mooring(w3.x, w3.y, w3.z);
        self.line(Step::To(w3));
        // a strand's reward on a broken stern off the line, outward
        let aside = round(84, m(84), m(42));
        self.hulk(aside, m(8), m(8), &[]);
        self.strand_reward(aside.x, aside.y, aside.z);

        // 3d: the hook again, to the fourth hull's mast
        let w4 = round(110, m(62), m(33));
        let mast4 = v3(w4.x - m(4), w4.y, w4.z - m(3));
        self.hulk(w4, m(16), m(16), &[(-m(4), -m(3), m(10))]);
        let toward = w4.sub(w3);
        let lip = w3.add(v3(toward.x, 0, toward.z).norm().scale(m(6)));
        self.line(Step::To(lip));
        self.line(Step::Hook(mast4.add(v3(0, m(3), 0))));
        self.line(Step::Jump(w4));
        self.line(Step::To(w4));

        let d5 = round(122, m(56), m(28));
        self.adrift(d5, m(5), 1444);
        self.line(Step::Jump(d5));
        let d6 = round(134, m(51), m(23));
        self.adrift(d6, m(5), 1576);
        self.line(Step::Jump(d6));
        let w5 = round(150, m(46), m(17));
        self.hulk(w5, m(14), m(14), &[(m(4), m(4), m(8))]);
        self.line(Step::Jump(w5));
        self.mooring(w5.x, w5.y, w5.z);
        self.line(Step::To(w5));

        // 3e: the last pieces, and the hull on the eye's rim
        let d7 = round(165, m(41), m(12));
        self.adrift(d7, m(5), 1839);
        self.line(Step::Jump(d7));
        let d8 = round(180, m(37), m(8));
        self.adrift(d8, m(5), 1967);
        self.line(Step::Jump(d8));
        let w6 = round(200, m(33), m(3));
        self.hulk(w6, m(18), m(18), &[(-m(6), m(6), m(12))]);
        self.line(Step::Jump(w6));
        self.mooring(w6.x, w6.y, w6.z);
        self.line(Step::To(w6));
        // the way on at its inner side, over the funnel
        let gate = round(200, m(27), m(3));
        self.way_on(gate.x, gate.y, gate.z);
        self.line(Step::To(round(200, m(29), m(3))));

        // where the Drowned come in: the fixed hulls' decks
        for (at, spread) in [
            (s0, m(5)),
            (w1, m(4)),
            (w2, m(5)),
            (w3, m(4)),
            (w4, m(5)),
            (w5, m(4)),
            (w6, m(6)),
        ] {
            for (dx, dz) in [(spread, 0), (-spread, 0), (0, spread), (0, -spread)] {
                self.k().spawns.push(v3(at.x + dx, at.y, at.z + dz));
            }
        }

        self.maelstrom_scene();
    }

    /// A broken hull fixed in the current: its deck `w` by `d`, its top at
    /// `at`, solid three metres down, and its masts (across, along, high
    /// over the deck), solid too, so the hook catches them.
    pub(super) fn hulk(&mut self, at: V3, w: i32, d: i32, masts: &[(i32, i32, i32)]) {
        let (x0, z0) = (at.x - w / 2, at.z - d / 2);
        let (x1, z1) = (at.x + w / 2, at.z + d / 2);
        self.solids
            .push(Solid::new(v3(x0, at.y - m(3), z0), v3(x1, at.y, z1)));
        let hull = self.c("hull");
        let dark = self.c("hull_dark");
        let sail = self.c("sail");
        let red = self.c("pirate_red");
        self.deco(Mesh::Cube, v3(at.x, at.y - dm(15), at.z), v3(w, m(3), d), hull);
        // the hull's belly under the deck, and its keel
        self.deco(
            Mesh::Blob,
            v3(at.x, at.y - m(3), at.z),
            v3(w + m(1), m(5), d + m(1)),
            dark,
        );
        // a broken rail at the edge
        for k in 0..3 {
            let x = x0 + (x1 - x0) * (k * 2 + 1) / 6;
            self.deco(
                Mesh::Cube,
                v3(x, at.y + dm(4), z0 + dm(2)),
                v3(m(2), dm(8), dm(2)),
                dark,
            );
        }
        for &(dx, dz, h) in masts {
            let (x, z) = (at.x + dx, at.z + dz);
            self.solids.push(Solid::new(
                v3(x - dm(3), at.y, z - dm(3)),
                v3(x + dm(3), at.y + h, z + dm(3)),
            ));
            self.deco(Mesh::Cyl, v3(x, at.y + h / 2, z), v3(dm(6), h, dm(6)), dark);
            // a yard and a rag of sail on it
            self.deco(
                Mesh::Cube,
                v3(x, at.y + h - m(2), z),
                v3(m(5), dm(3), dm(3)),
                dark,
            );
            self.deco(
                Mesh::Cube,
                v3(x + dm(5), at.y + h - m(4), z),
                v3(m(3), m(3), dm(1)),
                sail,
            );
        }
        // crates and a cask about the deck
        let n = self.rng.range(1, 3);
        for _ in 0..n {
            let x = at.x + self.metres(1, (w / ONE / 2).max(2)) - w / 4;
            let z = at.z - d / 2 + dm(15);
            let s = self.metres(1, 2);
            self.deco(Mesh::Cube, v3(x, at.y + s / 2, z), v3(s, s, s), hull);
        }
        self.deco(
            Mesh::Cyl,
            v3(at.x - w / 2 + m(1), at.y + dm(5), at.z + d / 2 - m(1)),
            v3(m(1), m(1), m(1)),
            red,
        );
    }

    /// A piece of wreck turning round the eye: `half` across each way, its
    /// top's middle at `at` when the world is `due` ticks old.
    fn adrift(&mut self, at: V3, half: i32, due: i32) {
        let turn = turn();
        // back round the eye to where it starts, and down by its bob then
        let r = v3(at.x - EYE.x, 0, at.z - EYE.z);
        let back = rotate_y(r, turn.wrapping_mul(due).wrapping_neg());
        let phase = deg(self.rng.range(0, 359));
        let t = (fx::TURN as i64 * due as i64 / BOB_TICKS as i64) as i32 + phase;
        let up = BOB / 2 - fx::mul(BOB / 2, fx::cos(t));
        let c = v3(EYE.x + back.x, at.y - up, EYE.z + back.z);
        self.wreckage(
            v3(c.x - half, c.y - dm(12), c.z - half),
            v3(c.x + half, c.y, c.z + half),
            EYE,
            turn,
            BOB,
            BOB_TICKS,
            phase,
        );
    }

    /// The water all round: dark overhead, the drowned fleet's wreckage
    /// hanging in it, bubbles going up, and the funnel at the eye.
    fn maelstrom_scene(&mut self) {
        let deep = self.c("ocean_deep");
        let sea = self.c("ocean");
        let foam = self.c("river_foam");
        let hull = self.c("hull");
        let dark = self.c("hull_dark");
        let navy = self.c("pirate_navy");
        let gloom = self.c("storm_cloud");
        // the surface far overhead, the light through it
        self.deco(
            Mesh::Cube,
            v3(EYE.x, m(130), EYE.z),
            v3(m(700), m(2), m(700)),
            deep,
        );
        self.halo(
            Mesh::Blob,
            boxed(v3(EYE.x, m(125), EYE.z + m(60)), v3(m(200), m(4), m(200))),
            sea,
            ONE / 5,
        );
        // the wreckage of the fleet, hanging and turning in the dark,
        // drawn only
        for _ in 0..70 {
            let a = self.rng.range(0, 359);
            let r = self.metres(30, 170);
            let y = self.metres(-20, 120);
            let at = round(a, r, y);
            match self.rng.range(0, 3) {
                0 => {
                    let s = self.metres(1, 3);
                    self.deco(Mesh::Cube, at, v3(s, s, s), hull);
                }
                1 => {
                    let l = self.metres(6, 18);
                    self.deco(Mesh::Blob, at, v3(l / 3, l / 4, l), dark);
                }
                2 => {
                    let l = self.metres(5, 14);
                    let tip = at.add(v3(self.metres(-4, 4), l, self.metres(-4, 4)));
                    self.rod(at, tip, dm(4), dark, true);
                }
                _ => {
                    let l = self.metres(3, 8);
                    self.deco(Mesh::Cube, at, v3(l, dm(3), dm(8)), hull);
                }
            }
        }
        // bubbles going up
        for _ in 0..90 {
            let a = self.rng.range(0, 359);
            let r = self.metres(10, 140);
            let y = self.metres(-10, 110);
            let s = dm(self.rng.range(2, 8));
            self.halo(Mesh::Sphere, boxed(round(a, r, y), v3(s, s, s)), foam, ONE / 3);
        }
        // the funnel: rings of dark water narrowing down into the eye, a
        // black throat, and foam streaks spiralling in
        for k in 0..12 {
            let r = m(24) - m(2) * k;
            let y = -m(2) * k;
            let rgb = if k % 2 == 0 { deep } else { navy };
            self.shape(
                Mesh::Ring,
                Affine::translate(v3(EYE.x, y, EYE.z))
                    .then(&Affine::rot_y(deg(17 * k)))
                    .then(&Affine::scale(v3(r * 2, m(2), r * 2))),
                rgb,
                0,
                false,
            );
        }
        // the throat: a cone turned point down
        self.shape(
            Mesh::Cone,
            Affine::translate(v3(EYE.x, -m(12), EYE.z))
                .then(&Affine::rot_x(fx::HALF))
                .then(&Affine::scale(v3(m(10), m(24), m(10)))),
            gloom,
            0,
            true,
        );
        self.deco(
            Mesh::Cyl,
            v3(EYE.x, DROWN - m(2), EYE.z),
            v3(m(60), m(2), m(60)),
            gloom,
        );
        for k in 0..36 {
            let a = 23 * k;
            let r = m(26) - dm(6) * k;
            let at = round(a, r, -dm(5) * k + m(1));
            self.shape(
                Mesh::Panel,
                Affine::translate(at)
                    .then(&Affine::rot_y(deg(a + 90)))
                    .then(&Affine::scale(v3(m(5), ONE, dm(3)))),
                foam,
                ONE / 3,
                false,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_maelstrom_can_be_crossed_on_its_line() {
        let (ticks, _) = crate::course::tests::cross_in("far_shore", "maelstrom", true).unwrap();
        assert!(ticks < 60 * 200, "{ticks}");
    }
}
