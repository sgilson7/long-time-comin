//! The Halls: three hundred metres of white hall, twenty high and
//! twenty-four across, far bigger than anything that walks in it needs.
//! Pillars of black glass hold up a ceiling ruled in cyan light; low
//! chrome-capped blocks stand about the floor for cover. Twice the floor
//! gives out: a cut across the hall you Bound over, and a trench either
//! side of a raised walkway that ends short of the far floor, crossed by a
//! ring hung from the ceiling over it. It is fought: the Vault's machines
//! (the Custodian, the Ward, the Excisor) come in waves near you all the
//! way along, and the way on opens when they are down.

use super::super::{boxed, dm, m, B};
use crate::course::Step;
use crate::fx::ONE;
use crate::geom::v3;
use crate::level::Solid;
use crate::mesh::Mesh;

/// the hall's half width, its ceiling, and how far down its trenches go
const W: i32 = 12 * ONE;
const H: i32 = 20 * ONE;
const DEEP: i32 = -40 * ONE;
/// the walkway and the galleries over the trench
const RAISED: i32 = 3 * ONE;
/// the hall's two ends
const NEAR: i32 = 10 * ONE;
const FAR: i32 = -310 * ONE;

impl B<'_> {
    pub(super) fn halls(&mut self) {
        let p = |y: i32, z: i32| v3(0, y, m(z));
        // tenths of a metre, for the stops short of an edge
        let q = |y: i32, z: i32| v3(0, y, dm(z));

        // the first hall: pillars, cover, and a cut across the floor
        self.hall_floor(NEAR, -m(60));
        self.vault_post(0, 0, m(4));
        self.hall_pillars(-m(4), -m(56));
        self.hall_cover(-m(10), -m(54));
        self.line(Step::To(q(0, -586)));
        self.line(Step::Bound(p(0, -76)));

        // the second: up a stair onto a walkway with the floor gone either
        // side of it, galleries along the walls, pillars in the trench
        self.hall_floor(-m(70), -m(100));
        self.vault_post(0, 0, -m(80));
        self.hall_cover(-m(84), -m(92));
        let white = self.c("vault_white");
        for k in 0..6 {
            let top = RAISED * (k + 1) / 6;
            let z = -m(94) - m(1) * k;
            self.block(0, top - dm(5), z - dm(5), m(6), dm(5), m(1), white);
        }
        self.vault_slab(-m(3), -m(100), m(3), -m(170), RAISED);
        for side in [-1, 1] {
            self.vault_slab(side * m(8), -m(104), side * W, -m(166), RAISED);
        }
        let glass = self.c("black_glass");
        let mut z = -m(110);
        while z > -m(166) {
            for side in [-1, 1] {
                let x = side * (m(5) + dm(5));
                self.solids.push(Solid::new(
                    v3(x - dm(7), DEEP, z - dm(7)),
                    v3(x + dm(7), H, z + dm(7)),
                ));
                self.deco(
                    Mesh::Cube,
                    v3(x, (DEEP + H) / 2, z),
                    v3(dm(14), H - DEEP, dm(14)),
                    glass,
                );
            }
            z -= m(14);
        }
        self.line(Step::To(p(0, -93)));
        self.line(Step::To(p(RAISED, -104)));
        self.vault_post(0, RAISED, -m(120));
        self.line(Step::To(q(RAISED, -1686)));

        // the walkway ends over the trench; a ring hung from the ceiling
        // over the far floor's edge pulls you across to it
        self.vault_ring(p(m(5), -196), p(H, -196));
        self.line(Step::Hook(p(m(5), -196)));
        self.line(Step::Air(p(0, -204)));

        // the last hall, to the way on
        self.hall_floor(-m(194), FAR);
        self.vault_post(0, 0, -m(206));
        self.hall_pillars(-m(210), -m(296));
        self.hall_cover(-m(214), -m(290));
        self.line(Step::To(p(0, -290)));
        self.vault_door(0, 0, -m(300));

        // the hall's shell: walls either side and at the ends, solid, and
        // drawn white with black glass let into them; the ceiling's light
        self.hall_shell();

        // the machines come in near you all the way along: on the floor,
        // and on the galleries over the trench
        let mut spawns = Vec::new();
        let mut z = -m(10);
        let mut side = 1;
        while z > -m(296) {
            let raised = z < -m(104) && z > -m(166);
            let gap = (z <= -m(60) && z >= -m(70)) || (z <= -m(166) && z >= -m(194));
            if raised {
                spawns.push(v3(side * m(10), RAISED, z));
            } else if !gap {
                spawns.push(v3(side * m(8), 0, z));
            }
            side = -side;
            z -= m(12);
        }
        // and either side of the ring's gap
        spawns.push(v3(-m(10), RAISED, -m(164)));
        spawns.push(v3(m(8), 0, -m(198)));
        let k = self.k();
        k.fought = true;
        k.spawns = spawns;
    }

    /// The hall's white floor across its width from z0 to z1, its top at 0,
    /// a cyan line where it meets each wall.
    fn hall_floor(&mut self, z0: i32, z1: i32) {
        self.vault_slab(-W, z0, W, z1, 0);
        let cyan = self.c("vault_cyan");
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        for side in [-1, 1] {
            self.glow(
                Mesh::Cube,
                v3(side * (W - dm(2)), dm(1), (z0 + z1) / 2),
                v3(dm(3), dm(2), z1 - z0),
                cyan,
            );
        }
    }

    /// Pillars of black glass, floor to ceiling, two rows down the hall.
    fn hall_pillars(&mut self, z0: i32, z1: i32) {
        let glass = self.c("black_glass");
        let chrome = self.c("chrome");
        let mut z = z0;
        while z > z1 {
            for side in [-1, 1] {
                let x = side * m(7);
                self.block(x, 0, z, m(2), H, m(2), glass);
                self.deco(Mesh::Cube, v3(x, dm(2), z), v3(dm(22), dm(4), dm(22)), chrome);
                self.deco(Mesh::Cube, v3(x, H - dm(2), z), v3(dm(22), dm(4), dm(22)), chrome);
            }
            z -= m(14);
        }
    }

    /// Low white blocks capped in chrome, chest high, off the line either
    /// side: cover from the Custodians' rings.
    fn hall_cover(&mut self, z0: i32, z1: i32) {
        let white = self.c("vault_white");
        let chrome = self.c("chrome");
        let mut z = z0;
        let mut side = -1;
        while z > z1 {
            let x = side * (m(3) + dm(5));
            let h = ONE * 12 / 10;
            self.block(x, 0, z, dm(24), h, dm(8), white);
            self.deco(Mesh::Cube, v3(x, h, z), v3(dm(26), dm(1), dm(10)), chrome);
            side = -side;
            z -= m(9);
        }
    }

    /// The walls, the ends, the ceiling and its light.
    fn hall_shell(&mut self) {
        let white = self.c("vault_white");
        let glass = self.c("black_glass");
        let cyan = self.c("vault_cyan");
        let violet = self.c("vault_violet");
        let len = NEAR - FAR;
        let mid = (NEAR + FAR) / 2;
        for side in [-1, 1] {
            let x = side * (W + m(1));
            self.solids
                .push(Solid::new(v3(x - m(1), DEEP, FAR), v3(x + m(1), H, NEAR)));
            self.deco(
                Mesh::Cube,
                v3(x, (DEEP + H) / 2, mid),
                v3(m(2), H - DEEP, len),
                white,
            );
            // black glass let into the wall, a violet line under it; its
            // face stands a little proud of the wall's (one plane flickers)
            let mut z = NEAR - m(8);
            while z > FAR + m(8) {
                self.deco(
                    Mesh::Cube,
                    v3(side * (W + dm(1) - ONE / 20), m(10), z),
                    v3(dm(2), m(12), m(10)),
                    glass,
                );
                self.glow(
                    Mesh::Cube,
                    v3(side * W, m(3) + dm(5), z),
                    v3(dm(2), dm(2), m(10)),
                    violet,
                );
                z -= m(16);
            }
        }
        for z in [NEAR + m(1), FAR - m(1)] {
            self.solids
                .push(Solid::new(v3(-W, DEEP, z - m(1)), v3(W, H, z + m(1))));
            self.deco(
                Mesh::Cube,
                v3(0, (DEEP + H) / 2, z),
                v3(W * 2, H - DEEP, m(2)),
                white,
            );
        }
        // the ceiling, ruled across in cyan
        self.deco(
            Mesh::Cube,
            v3(0, H + dm(5), mid),
            v3(W * 2 + m(4), m(1), len),
            white,
        );
        let mut z = NEAR - m(4);
        while z > FAR {
            self.glow(Mesh::Cube, v3(0, H - dm(1), z), v3(W * 2, dm(2), dm(4)), cyan);
            z -= m(8);
        }
        for x in [-m(4), m(4)] {
            self.glow(Mesh::Cube, v3(x, H - dm(1), mid), v3(dm(3), dm(2), len), cyan);
        }
        // the trench's dark, and the light far down in it
        self.halo(
            Mesh::Sphere,
            boxed(v3(0, -m(30), -m(150)), v3(W * 2, m(20), m(90))),
            violet,
            ONE / 4,
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_halls_can_be_fought_through_on_their_line_with_the_rivers_kit() {
        let (ticks, _lost) = crate::course::tests::cross_in("vault", "halls", true).unwrap();
        assert!(ticks < 60 * 45, "{ticks}");
    }

    #[test]
    fn the_halls_are_fought_and_their_machines_come_in_along_the_whole_way() {
        let c = crate::content::tests::content();
        let a = crate::arena::build_at("vault", "halls", 1, &[], &c.pal, 0);
        let k = a.course.as_ref().unwrap();
        assert!(k.fought);
        let one = crate::fx::ONE;
        // no stretch of the line goes forty metres without a place to come in
        let mut z = 0;
        while z > k.finish.z {
            assert!(
                k.spawns.iter().any(|s| (s.z - z).abs() < 20 * one),
                "nothing comes in near z {}",
                z / one
            );
            z -= 10 * one;
        }
        // and each stands on something
        for s in &k.spawns {
            let f = a.level.floor_below(s.x, s.y + one / 2, s.z);
            assert!(f.is_some_and(|y| (s.y - y).abs() <= one / 2), "{s:?}");
        }
        assert!(k.finish.z < -290 * one);
    }
}
