//! The Shaft: the way into the Vault, straight down a shaft a hundred and
//! forty metres across and a hundred deep, black glass walled, ribbed in
//! chrome, a band of cyan light round it every fifteen metres. Nothing
//! stands on its floor: gantries and white slabs float on struts from the
//! walls, and the way down goes round the shaft by drops, a jump, Bound
//! Over between gantries, rings hung from the beams and set in the walls,
//! and two crossings where the only standing place is a Bench you lay. A
//! miss falls past the last light into the dark, and back to the post.

use super::super::{boxed, dm, m, B};
use crate::course::Step;
use crate::fx::{self, ONE};
use crate::geom::{v3, Affine, V3};
use crate::level::Solid;
use crate::mesh::Mesh;

/// the shaft's radius, its rim, and the dark under its last floor
const R: i32 = 72 * ONE;
const RIM: i32 = 130 * ONE;
const DEEP: i32 = -40 * ONE;
/// a band of light round the shaft this often, up from the bottom
const BAND: i32 = 15 * ONE;
/// the floor at the foot, where the way on is
const FOOT: i32 = 2 * ONE;

impl B<'_> {
    pub(super) fn shaft(&mut self) {
        let p = |x: i32, y: i32, z: i32| v3(m(x), m(y), m(z));
        // half metres, for the stops short of an edge
        let q = |x: i32, y: i32, z: i32| v3(dm(x), m(y), dm(z));

        // 1: north across the shaft from the ledge by the south wall. A jump
        // down to a slab, Bound Over from it to a gantry, and from the
        // gantry's end a ring hung from a beam over the next slab
        self.vault_slab(-m(6), m(56), m(6), m(36), m(100));
        self.vault_post(0, m(100), m(46));
        self.line(Step::To(q(0, 100, 374)));
        self.vault_slab(-m(5), m(30), m(5), m(20), m(94));
        self.line(Step::Jump(p(0, 94, 25)));
        self.line(Step::To(q(0, 94, 214)));
        self.vault_gantry(-m(2), m(10), m(2), -m(8), m(90));
        self.line(Step::Bound(p(0, 90, 3)));
        self.vault_post(0, m(90), 0);
        self.line(Step::To(q(0, 90, -66)));
        self.vault_slab(-m(10), -m(16), m(10), -m(40), m(84));
        self.hung_light(p(0, 88, -22), m(108));
        self.line(Step::Hook(p(0, 88, -22)));
        self.line(Step::Air(p(0, 84, -28)));
        self.line(Step::To(p(4, 84, -28)));

        // 2: east, the first Bench. A ring on the face of a black glass
        // pillar pulls you off the slab's edge and into the pillar; you lay
        // a Bench under you at the top of the pull, and Bound from it to
        // the gantry running south: too far, and too high, to reach falling
        self.line(Step::To(q(86, 84, -280)));
        self.vault_pillar(m(26), -m(31), m(32), -m(25));
        self.vault_ring(p(25, 88, -28), p(26, 88, -28));
        self.line(Step::Hook(p(25, 88, -28)));
        self.line(Step::Wait(12));
        self.line(Step::Bench);
        self.line(Step::Wait(30));
        self.vault_gantry(m(22), -m(16), m(26), m(4), m(86));
        self.line(Step::Bound(p(24, 86, -10)));
        self.vault_post(m(24), m(86), -m(6));
        self.line(Step::To(p(24, 86, 2)));

        // 3: south. A long drop off the gantry's end to the slab under it,
        // and a ring set in the east wall over the corner slab
        self.vault_slab(m(22), m(6), m(34), m(20), m(66));
        self.line(Step::To(p(28, 66, 14)));
        self.line(Step::To(q(280, 66, 186)));
        self.vault_slab(m(20), m(22), m(36), m(42), m(60));
        self.wall_light(p(28, 64, 30));
        self.line(Step::Hook(p(28, 64, 30)));
        self.line(Step::Air(p(28, 60, 30)));
        self.line(Step::To(p(26, 60, 28)));

        // the strand: a white ledge high against the east wall, and a ring
        // to reach it by
        self.vault_slab(m(44), m(26), m(52), m(34), m(66));
        self.wall_light(p(42, 70, 30));
        self.strand_reward(m(48), m(66), m(30));

        // 4: west. Bound Over to a long gantry, a jump down to a slab at its
        // end, and a drop to the corner slab and its post
        self.line(Step::To(q(214, 60, 280)));
        self.vault_gantry(m(10), m(26), -m(10), m(30), m(56));
        self.line(Step::Bound(p(2, 56, 28)));
        self.line(Step::To(q(-86, 56, 280)));
        self.vault_slab(-m(14), m(22), -m(22), m(34), m(50));
        self.line(Step::Jump(p(-18, 50, 28)));
        self.line(Step::To(q(-206, 50, 280)));
        self.vault_slab(-m(24), m(20), -m(36), m(36), m(42));
        self.line(Step::To(p(-28, 42, 28)));
        self.vault_post(-m(28), m(42), m(28));
        self.line(Step::To(q(-280, 42, 214)));

        // 5: north, down to the foot. A ring on a second pillar's face over
        // the gap; a Bench as you slide down it; Bound round the pillar to a
        // slab, a drop to the slab past it, and the long drop off that to
        // the floor
        self.vault_pillar(-m(31), -m(4), -m(25), m(2));
        self.vault_ring(v3(-m(28), m(46), dm(26)), p(-28, 46, 2));
        self.line(Step::Hook(v3(-m(28), m(46), dm(26))));
        self.line(Step::Wait(12));
        self.line(Step::Bench);
        self.line(Step::Wait(30));
        self.vault_slab(-m(22), -m(2), -m(10), -m(14), m(44));
        self.line(Step::Bound(p(-16, 44, -8)));
        self.line(Step::To(q(-160, 44, -126)));
        self.vault_slab(-m(22), -m(18), -m(10), -m(24), m(36));
        self.line(Step::To(p(-16, 36, -21)));
        self.line(Step::To(q(-160, 36, -226)));
        self.vault_slab(-m(32), -m(28), -m(12), -m(46), FOOT);
        self.line(Step::To(p(-20, 2, -40)));
        self.vault_door(-m(22), FOOT, -m(44));

        // each slab and gantry held on a chrome strut out of the wall
        let chrome = self.c("chrome");
        let floats: Vec<Solid> = self
            .solids
            .iter()
            .filter(|s| s.size().y <= m(1) && s.size().x > m(2) && s.max.y > FOOT)
            .copied()
            .collect();
        for s in floats {
            let c = s.centre();
            let out = v3(c.x, 0, c.z);
            if out.len_xz() > 0 {
                let w = out.norm().scale(R);
                self.rod(
                    v3(c.x, s.min.y, c.z),
                    v3(w.x, s.min.y - m(8), w.z),
                    dm(3),
                    chrome,
                    true,
                );
            }
        }

        self.shaft_walls(R, DEEP, RIM);
        // the beam the hung ring hangs from, across the shaft
        let beam = -m(22);
        self.rod(v3(-R, m(108), beam), v3(R, m(108), beam), dm(5), chrome, true);
        // the dark under the foot, and the light that comes up out of it
        let violet = self.c("vault_violet");
        self.halo(
            Mesh::Sphere,
            boxed(v3(0, DEEP, 0), v3(R, m(30), R)),
            violet,
            ONE / 3,
        );
    }

    // ---- the Vault's furniture ------------------------------------------

    /// A white slab a metre thick, its top at `top`, over x0..x1 by z0..z1:
    /// standing on nothing, its underside rimmed in cyan light.
    pub(super) fn vault_slab(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, top: i32) {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        self.solids
            .push(Solid::new(v3(x0, top - m(1), z0), v3(x1, top, z1)));
        let white = self.c("vault_white");
        let cyan = self.c("vault_cyan");
        let (cx, cz) = ((x0 + x1) / 2, (z0 + z1) / 2);
        self.deco(
            Mesh::Cube,
            v3(cx, top - dm(5), cz),
            v3(x1 - x0, m(1), z1 - z0),
            white,
        );
        self.glow(
            Mesh::Cube,
            v3(cx, top - m(1) - dm(1), cz),
            v3(x1 - x0 - dm(4), dm(2), z1 - z0 - dm(4)),
            cyan,
        );
    }

    /// A chrome gantry half a metre thick, railed along its length (the
    /// rails drawn only), violet light under it.
    pub(super) fn vault_gantry(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, top: i32) {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        self.solids
            .push(Solid::new(v3(x0, top - dm(5), z0), v3(x1, top, z1)));
        let chrome = self.c("chrome");
        let violet = self.c("vault_violet");
        let (cx, cz) = ((x0 + x1) / 2, (z0 + z1) / 2);
        self.deco(
            Mesh::Cube,
            v3(cx, top - dm(25), cz),
            v3(x1 - x0, dm(5), z1 - z0),
            chrome,
        );
        self.glow(
            Mesh::Cube,
            v3(cx, top - dm(6), cz),
            v3(x1 - x0 - dm(6), dm(1), z1 - z0 - dm(6)),
            violet,
        );
        let rail = top + m(1);
        if x1 - x0 > z1 - z0 {
            for z in [z0 + dm(1), z1 - dm(1)] {
                self.rod(v3(x0, rail, z), v3(x1, rail, z), dm(1), chrome, false);
            }
        } else {
            for x in [x0 + dm(1), x1 - dm(1)] {
                self.rod(v3(x, rail, z0), v3(x, rail, z1), dm(1), chrome, false);
            }
        }
    }

    /// A ring the hook catches: a chrome hoop round a cyan light, held on
    /// a strut from `from`.
    pub(super) fn vault_ring(&mut self, at: V3, from: V3) {
        self.solids.push(Solid::new(
            at.sub(v3(dm(5), dm(5), dm(5))),
            at.add(v3(dm(5), dm(5), dm(5))),
        ));
        let chrome = self.c("chrome");
        let cyan = self.c("vault_cyan");
        self.shape(
            Mesh::Ring,
            Affine::translate(at)
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&Affine::scale(v3(dm(16), dm(3), dm(16)))),
            chrome,
            0,
            true,
        );
        self.glow(Mesh::Sphere, at, v3(dm(6), dm(6), dm(6)), cyan);
        self.rod(from, at, dm(2), chrome, true);
    }

    /// A ring hung on a rod from a beam at `beam` above it.
    fn hung_light(&mut self, at: V3, beam: i32) {
        self.vault_ring(at, v3(at.x, beam, at.z));
    }

    /// A ring on a strut out of the shaft's wall, straight out from its axis.
    fn wall_light(&mut self, at: V3) {
        let out = v3(at.x, 0, at.z);
        let w = out.norm().scale(R);
        self.vault_ring(at, v3(w.x, at.y, w.z));
    }

    /// A pillar of black glass from the dark to the rim, chrome at its
    /// corners, a cyan band round it with each of the shaft's.
    pub(super) fn vault_pillar(&mut self, x0: i32, z0: i32, x1: i32, z1: i32) {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        self.solids.push(Solid::new(v3(x0, DEEP, z0), v3(x1, RIM, z1)));
        let glass = self.c("black_glass");
        let chrome = self.c("chrome");
        let cyan = self.c("vault_cyan");
        let (cx, cz) = ((x0 + x1) / 2, (z0 + z1) / 2);
        self.deco(
            Mesh::Cube,
            v3(cx, (DEEP + RIM) / 2, cz),
            v3(x1 - x0, RIM - DEEP, z1 - z0),
            glass,
        );
        for (x, z) in [(x0, z0), (x1, z0), (x0, z1), (x1, z1)] {
            self.rod(v3(x, DEEP, z), v3(x, RIM, z), dm(3), chrome, true);
        }
        let mut y = FOOT + BAND;
        while y < RIM {
            self.glow(
                Mesh::Cube,
                v3(cx, y, cz),
                v3(x1 - x0 + dm(2), dm(4), z1 - z0 + dm(2)),
                cyan,
            );
            y += BAND;
        }
    }

    /// A post the fall returns you to: a chrome pillar to the left of it,
    /// lit cyan at its head.
    pub(super) fn vault_post(&mut self, x: i32, y: i32, z: i32) {
        self.k().posts.push(v3(x, y, z));
        let chrome = self.c("chrome");
        let cyan = self.c("vault_cyan");
        let px = x - m(3);
        self.deco(Mesh::Cyl, v3(px, y + dm(9), z), v3(dm(3), dm(18), dm(3)), chrome);
        self.glow(Mesh::Sphere, v3(px, y + dm(19), z), v3(dm(3), dm(3), dm(3)), cyan);
    }

    /// The way on: a doorway of chrome and black glass, open, lit violet.
    pub(super) fn vault_door(&mut self, x: i32, y: i32, z: i32) {
        {
            let k = self.k();
            k.finish = v3(x, y, z);
            k.finish_yaw = fx::HALF;
        }
        let chrome = self.c("chrome");
        let glass = self.c("black_glass");
        let violet = self.c("vault_violet");
        for dx in [-m(4), m(4)] {
            self.deco(
                Mesh::Cube,
                v3(x + dx, y + m(4), z),
                v3(dm(8), m(8), dm(8)),
                chrome,
            );
        }
        self.deco(Mesh::Cube, v3(x, y + m(8), z), v3(m(9), dm(8), dm(8)), chrome);
        self.deco(
            Mesh::Cube,
            v3(x, y + m(4), z - m(2)),
            v3(m(8), m(8), dm(2)),
            glass,
        );
        self.glow(Mesh::Cube, v3(x, y + dm(75), z), v3(m(7), dm(2), dm(2)), violet);
    }

    /// A round shaft's walls, radius `r`, from `y0` to `y1`: solid blocks
    /// round it (nothing is lost sideways), drawn as black glass panes
    /// ribbed in chrome, a cyan band of light round it every `BAND`.
    pub(super) fn shaft_walls(&mut self, r: i32, y0: i32, y1: i32) {
        let n = 36;
        let step = fx::TURN / n;
        // a pane's width: the circle's share, and a little over
        let w = fx::mul(r, fx::ratio(6283, 1000)) / n + m(1);
        let glass = self.c("black_glass");
        let chrome = self.c("chrome");
        let cyan = self.c("vault_cyan");
        for k in 0..n {
            let a = step * k;
            let (c, s) = (fx::cos(a), fx::sin(a));
            let at = v3(fx::mul(c, r + w / 2), 0, fx::mul(s, r + w / 2));
            self.solids.push(Solid::new(
                v3(at.x - w / 2, y0, at.z - w / 2),
                v3(at.x + w / 2, y1, at.z + w / 2),
            ));
            let face = v3(fx::mul(c, r + dm(5)), (y0 + y1) / 2, fx::mul(s, r + dm(5)));
            self.shape(
                Mesh::Cube,
                Affine::translate(face)
                    .then(&Affine::rot_y(fx::QUARTER - a))
                    .then(&Affine::scale(v3(w, y1 - y0, dm(4)))),
                glass,
                0,
                false,
            );
            let rib = a + step / 2;
            let foot = v3(fx::mul(fx::cos(rib), r), y0, fx::mul(fx::sin(rib), r));
            self.rod(foot, v3(foot.x, y1, foot.z), dm(4), chrome, true);
        }
        let mut y = FOOT + BAND;
        while y < y1 {
            self.shape(
                Mesh::Ring,
                boxed(v3(0, y, 0), v3(r * 2 - m(1), dm(6), r * 2 - m(1))),
                cyan,
                ONE,
                false,
            );
            y += BAND;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_shaft_can_be_crossed_down_its_line_with_the_rivers_kit() {
        let (ticks, _lost) = crate::course::tests::cross_in("vault", "shaft", false).unwrap();
        assert!(ticks < 60 * 40, "{ticks}");
    }

    #[test]
    fn the_shaft_goes_down_from_its_rim_to_its_foot_by_four_posts_and_a_strand() {
        let c = crate::content::tests::content();
        let a = crate::arena::build_at("vault", "shaft", 1, &[], &c.pal, 0);
        let k = a.course.as_ref().unwrap();
        assert_eq!(k.posts.len(), 4);
        assert_eq!(k.strands.len(), 1);
        assert!(k.posts[0].y >= 95 * crate::fx::ONE, "{:?}", k.posts[0]);
        assert!(k.finish.y <= 3 * crate::fx::ONE, "{:?}", k.finish);
    }
}
