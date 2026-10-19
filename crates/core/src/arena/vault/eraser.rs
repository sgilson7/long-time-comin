//! The Eraser: a round chamber forty metres across, black glass walled and
//! ribbed in chrome, under a dome of the dark. At its heart, on a white
//! dais, the Eraser: a ring machine taller than a house, standing on its
//! edge, shut. Its keeper, the Archivist, comes in through the anomaly's
//! light on the far side and flies the chamber on its jet pack. Three
//! chrome pylons stand round the dais with low walls behind them, cover
//! from it. When it falls the Eraser opens, the pylons feed it, and a step
//! leads up onto the dais and into its ring.

use super::super::{boxed, dm, m, B};
use crate::balance::ERASER_HOLD;
use crate::course::{Eraser, Step};
use crate::fx::{self, ONE};
use crate::geom::{v3, Affine, V3};
use crate::level::Solid;
use crate::mesh::Mesh;

/// the chamber's radius, its wall's height, and the dais's
const R: i32 = 20 * ONE;
const TALL: i32 = 24 * ONE;
const DAIS: i32 = 5 * ONE;
const DAIS_TOP: i32 = ONE * 8 / 10;
/// the Eraser's ring: its radius, standing on the dais
const RING: i32 = 6 * ONE;
/// how near its middle the open ring takes you
const REACH: i32 = ONE * 3 / 2;

impl B<'_> {
    pub(super) fn eraser(&mut self) {
        // the floor, the walls, and the start on the south side
        self.vault_slab(-R - m(2), R + m(2), R + m(2), -R - m(2), 0);
        self.shaft_walls(R, -m(2), TALL);
        self.vault_post(0, 0, m(16));

        // the pylons round the dais: west, east, and the far one, north-east
        let pylons = vec![v3(-m(12), 0, m(2)), v3(m(12), 0, m(2)), v3(m(10), 0, -m(10))];
        for &at in &pylons {
            self.pylon(at);
        }
        // a low wall behind each, toward the far side
        for &at in &pylons {
            let toward = v3(-at.x, 0, -R - at.z).norm();
            let c = at.add(toward.scale(m(4)));
            self.cover_wall(c, toward);
        }
        // and two tall blocks of glass beyond the dais
        let glass = self.c("black_glass");
        for x in [-m(7), m(3)] {
            self.block(x, 0, -m(11), m(2), m(4), m(2), glass);
        }

        // the Eraser on its dais, shut until its keeper falls
        self.the_eraser();
        self.k().eraser = Some(Eraser {
            at: v3(0, DAIS_TOP, 0),
            radius: REACH,
            ring: RING,
            need: ERASER_HOLD,
            pylons: pylons.iter().map(|p| p.add(v3(0, m(10), 0))).collect(),
            open: false,
            opened: 0,
            held: 0,
            erased: 0,
        });

        // the anomaly's light through the far wall: the Archivist comes in
        // under it, high
        let violet = self.c("vault_violet");
        let cyan = self.c("vault_cyan");
        self.halo(
            Mesh::Sphere,
            boxed(v3(0, m(6), -R + m(1)), v3(m(10), m(12), m(4))),
            violet,
            ONE / 2,
        );
        self.glow(
            Mesh::Cube,
            v3(0, m(6), -R + dm(5)),
            v3(m(3), m(10), dm(2)),
            violet,
        );
        self.k().boss_at = Some(v3(0, m(8), -m(14)));

        // the dome of the dark over it all, banded in light
        let sky = self.c("vault_sky");
        self.deco(
            Mesh::Cyl,
            v3(0, TALL + m(1), 0),
            v3(R * 2 + m(4), m(2), R * 2 + m(4)),
            sky,
        );
        self.shape(
            Mesh::Ring,
            boxed(v3(0, TALL - m(1), 0), v3(R * 2 - m(2), dm(4), R * 2 - m(2))),
            cyan,
            ONE,
            false,
        );

        // the line: up the step and into the ring
        self.line(Step::To(v3(0, DAIS_TOP, DAIS + m(1))));
        self.line(Step::To(v3(0, DAIS_TOP, 0)));
        // nothing leads on from the Eraser: what comes after is read
        self.k().finish = v3(0, -m(80), 0);
    }

    /// A pylon: a tall chrome column, its head lit cyan.
    fn pylon(&mut self, at: V3) {
        let chrome = self.c("chrome");
        let cyan = self.c("vault_cyan");
        let white = self.c("vault_white");
        self.solids
            .push(Solid::stand(at.x, at.y, at.z, dm(12), m(10), dm(12)));
        self.deco(
            Mesh::Cyl,
            at.add(v3(0, m(5), 0)),
            v3(dm(12), m(10), dm(12)),
            chrome,
        );
        self.deco(
            Mesh::Cyl,
            at.add(v3(0, dm(3), 0)),
            v3(dm(24), dm(6), dm(24)),
            white,
        );
        self.glow(
            Mesh::Sphere,
            at.add(v3(0, m(10), 0)),
            v3(dm(9), dm(9), dm(9)),
            cyan,
        );
        for y in [m(3), m(6), m(9)] {
            self.shape(
                Mesh::Ring,
                boxed(at.add(v3(0, y, 0)), v3(dm(18), dm(2), dm(18))),
                cyan,
                ONE,
                false,
            );
        }
    }

    /// A wall of black glass, chest high and four metres long, capped in
    /// chrome, standing across `facing` at `c`.
    fn cover_wall(&mut self, c: V3, facing: V3) {
        let glass = self.c("black_glass");
        let chrome = self.c("chrome");
        let h = ONE * 12 / 10;
        // solid as the box round it, drawn turned to face the far side
        let along = v3(-facing.z, 0, facing.x);
        let half = along.scale(m(2));
        let (a, b) = (c.add(half), c.sub(half));
        let r = dm(4);
        self.solids.push(Solid::new(
            v3(a.x.min(b.x) - r, 0, a.z.min(b.z) - r),
            v3(a.x.max(b.x) + r, h, a.z.max(b.z) + r),
        ));
        let yaw = fx::atan2(facing.x, facing.z);
        self.shape(
            Mesh::Cube,
            Affine::translate(c.add(v3(0, h / 2, 0)))
                .then(&Affine::rot_y(yaw))
                .then(&Affine::scale(v3(m(4), h, dm(8)))),
            glass,
            0,
            true,
        );
        // the cap overhangs the glass a little at each end, so its ends do
        // not lie in the wall's
        let over = along.scale(ONE * 3 / 100);
        self.rod(
            a.add(over).add(v3(0, h, 0)),
            b.sub(over).add(v3(0, h, 0)),
            dm(2),
            chrome,
            false,
        );
    }

    /// The Eraser: a white dais with a step up from the south, two chrome
    /// feet, and the ring standing on edge between them, its foot on the
    /// dais, facing the start. What fills it (shut, open, taking you) is
    /// drawn by the world.
    fn the_eraser(&mut self) {
        let white = self.c("vault_white");
        let chrome = self.c("chrome");
        let cyan = self.c("vault_cyan");
        self.solids
            .push(Solid::new(v3(-DAIS, 0, -DAIS), v3(DAIS, DAIS_TOP, DAIS)));
        self.deco(
            Mesh::Cyl,
            v3(0, DAIS_TOP / 2, 0),
            v3(DAIS * 2 + m(1), DAIS_TOP, DAIS * 2 + m(1)),
            white,
        );
        // the step, half the dais's height, out of the south side
        self.block(0, 0, DAIS + dm(6), m(4), DAIS_TOP / 2, dm(16), white);
        // the feet, solid, either side, up to the ring's waist
        for x in [-RING - dm(5), RING + dm(5)] {
            self.block(x, 0, 0, m(1), DAIS_TOP + RING, m(2), chrome);
        }
        // the ring, on edge, facing south toward the start
        let ring = |r: i32, w: i32| {
            Affine::translate(v3(0, DAIS_TOP + RING, 0))
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&Affine::scale(v3(r * 2, w, r * 2)))
        };
        self.shape(Mesh::Ring, ring(RING, m(2)), chrome, 0, true);
        self.shape(Mesh::Ring, ring(RING - dm(9), dm(5)), cyan, ONE, false);
    }
}

#[cfg(test)]
mod tests {
    use crate::fx::ONE;
    use crate::geom::v3;

    fn arena() -> crate::arena::Arena {
        let c = crate::content::tests::content();
        crate::arena::build_at("vault", "boss", 1, &[], &c.pal, 0)
    }

    #[test]
    fn the_eraser_stands_shut_on_its_dais_and_its_keeper_comes_in_on_the_far_side() {
        let a = arena();
        let k = a.course.as_ref().unwrap();
        let er = k.eraser.as_ref().expect("the Eraser");
        assert!(!er.open && er.held == 0 && er.erased == 0);
        assert_eq!(er.pylons.len(), 3);
        let f = a
            .level
            .floor_below(er.at.x, er.at.y + ONE, er.at.z)
            .expect("the dais");
        assert_eq!(f, er.at.y, "its ring's floor is the dais");
        let boss = k.boss_at.expect("where the boss comes in");
        // on the far side from the start, and up in the air
        assert!(boss.z < a.start.z - 20 * ONE, "{boss:?}");
        assert!(boss.y > 4 * ONE, "{boss:?}");
        assert!(k.finish.y < -60 * ONE);
    }

    #[test]
    fn the_start_can_walk_up_onto_the_dais_and_into_the_erasers_ring() {
        let a = arena();
        let er = a.course.as_ref().unwrap().eraser.clone().unwrap();
        let walked = crate::arena::walk(&a.level, a.start);
        assert!(
            walked
                .iter()
                .any(|w| w.sub(er.at).len_xz() < er.radius && (w.y - er.at.y).abs() < ONE / 2),
            "no walk into the ring at {:?}",
            er.at
        );
        // and nothing stands between the start and the ring
        let chest = |p: crate::geom::V3| p.add(v3(0, crate::balance::PLAYER_CHEST, 0));
        let to = chest(er.at).sub(chest(a.start));
        assert!(a
            .level
            .ray(chest(a.start), to.norm(), to.len() - er.radius)
            .is_none());
    }
}
