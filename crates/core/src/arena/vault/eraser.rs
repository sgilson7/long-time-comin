//! The Eraser: a round chamber forty metres across, black glass walled and
//! ribbed in chrome, under a dome of the dark. At its heart, on a white
//! dais, the Eraser: a ring machine taller than a house, standing on its
//! edge. Three pylons stand round it, tall chrome columns each with a cyan
//! ring on the floor at its foot: stand in the ring and the pylon charges,
//! one after another, while your older self comes through the anomaly's
//! light on the far side to stop you. Low walls stand behind each pylon,
//! toward the far side, so a pylon can be held under fire.

use super::super::{boxed, dm, m, B};
use crate::course::Step;
use crate::fx::{self, ONE};
use crate::geom::{v3, Affine, V3};
use crate::level::Solid;
use crate::mesh::Mesh;

/// the chamber's radius, its wall's height, and the dais's
const R: i32 = 20 * ONE;
const TALL: i32 = 24 * ONE;
const DAIS: i32 = 5 * ONE;
const DAIS_TOP: i32 = ONE * 8 / 10;
/// the Eraser's ring: its radius and how high its middle stands
const RING: i32 = 6 * ONE;
const RING_Y: i32 = 9 * ONE;

impl B<'_> {
    pub(super) fn eraser(&mut self) {
        // the floor, the walls, and the start on the south side
        self.vault_slab(-R - m(2), R + m(2), R + m(2), -R - m(2), 0);
        self.shaft_walls(R, -m(2), TALL);
        self.vault_post(0, 0, m(16));

        // the pylons, held in turn: west, east, and the far one, north-east
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
        // and two tall blocks of glass beyond the dais, off the lines from
        // the start to the pylons
        let glass = self.c("black_glass");
        for x in [-m(7), m(3)] {
            self.block(x, 0, -m(11), m(2), m(4), m(2), glass);
        }
        self.eraser_pylons(pylons.clone(), m(3), 20 * 60);

        // the Eraser on its dais
        self.the_eraser();

        // the anomaly's light through the far wall: the Defendant comes in
        // under it
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
        self.k().boss_at = Some(v3(0, m(1), -m(16)));

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

        self.line(Step::To(pylons[0]));
        // nothing leads on from the Eraser: what comes after is read
        self.k().finish = v3(0, -m(80), 0);
    }

    /// A pylon: a tall chrome column, its head lit cyan, and the ring at its
    /// foot you stand in to charge it.
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
        // the ring on the floor: the charge's reach
        self.shape(
            Mesh::Ring,
            boxed(at.add(v3(0, dm(1), 0)), v3(m(6), dm(1), m(6))),
            cyan,
            ONE,
            false,
        );
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
        self.rod(a.add(v3(0, h, 0)), b.add(v3(0, h, 0)), dm(2), chrome, false);
    }

    /// The Eraser: a white dais, two chrome feet, and the ring standing on
    /// edge between them, a cyan ring inside it and violet at its heart.
    fn the_eraser(&mut self) {
        let white = self.c("vault_white");
        let chrome = self.c("chrome");
        let cyan = self.c("vault_cyan");
        let violet = self.c("vault_violet");
        self.solids
            .push(Solid::new(v3(-DAIS, 0, -DAIS), v3(DAIS, DAIS_TOP, DAIS)));
        self.deco(
            Mesh::Cyl,
            v3(0, DAIS_TOP / 2, 0),
            v3(DAIS * 2 + m(1), DAIS_TOP, DAIS * 2 + m(1)),
            white,
        );
        // the feet, solid, either side
        for x in [-RING + m(1), RING - m(1)] {
            self.block(x, DAIS_TOP, 0, m(1), m(4), m(2), chrome);
        }
        // the ring, on edge, facing south toward the start
        let ring = |r: i32, w: i32| {
            Affine::translate(v3(0, RING_Y, 0))
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&Affine::scale(v3(r * 2, w, r * 2)))
        };
        self.shape(Mesh::Ring, ring(RING, m(2)), chrome, 0, true);
        self.shape(Mesh::Ring, ring(RING - m(1), dm(5)), cyan, ONE, false);
        self.halo(
            Mesh::Sphere,
            boxed(v3(0, RING_Y, 0), v3(m(4), m(4), m(4))),
            violet,
            ONE / 2,
        );
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
    fn the_erasers_three_pylons_stand_on_the_floor_and_its_boss_comes_in() {
        let a = arena();
        let k = a.course.as_ref().unwrap();
        let ch = k.charge.as_ref().expect("the charge");
        assert_eq!(ch.pylons.len(), 3);
        assert_eq!(ch.held, vec![0; 3]);
        for p in &ch.pylons {
            let f = a.level.floor_below(p.x, p.y, p.z).expect("floor under the pylon");
            assert!(p.y - f <= ONE / 2 && p.y >= f, "{p:?} over {f}");
        }
        let boss = k.boss_at.expect("where the boss comes in");
        // on the far side from the start
        assert!(boss.z < a.start.z - 20 * ONE, "{boss:?}");
        assert!(k.finish.y < -60 * ONE);
    }

    #[test]
    fn the_start_can_walk_to_each_pylon_with_nothing_in_the_way() {
        let a = arena();
        let ch = a.course.as_ref().unwrap().charge.clone().unwrap();
        let chest = |p: crate::geom::V3| p.add(v3(0, crate::balance::PLAYER_CHEST, 0));
        let from = chest(a.start);
        let walked = crate::arena::walk(&a.level, a.start);
        for p in &ch.pylons {
            // a clear line to the pylon's ring (its column is solid)
            let to = chest(*p).sub(from);
            let reach = to.len() - ch.radius;
            assert!(
                a.level.ray(from, to.norm(), reach).is_none(),
                "something stands between the start and the pylon at {p:?}"
            );
            // and a way there on foot, into its ring
            assert!(
                walked
                    .iter()
                    .any(|w| w.sub(*p).len_xz() < ch.radius && (w.y - p.y).abs() < ONE / 2),
                "no walk into the ring of the pylon at {p:?}"
            );
        }
    }
}
