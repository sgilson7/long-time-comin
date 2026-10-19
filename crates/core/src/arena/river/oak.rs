//! The end of the river: the Boathouse (rest) on the shore of Lake
//! Stillwater, and the Gallows Oak (the boss), 150 m of black trunk climbed
//! ring to ring from its roots: a shelf of bark every 9 m round it, a ring
//! hung over each to hook, three broad ledges (the climb's mooring posts),
//! and at the top its crown, a ring of broken boughs over the lake.

use super::super::{dm, m, B};
use crate::course::{OakSite, Step};
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, V3};
use crate::level::Solid;
use crate::mesh::Mesh;

/// the shore's height over the lake
const SHORE: i32 = 2 * ONE;
/// the trunk's half width (it is solid as a square, drawn round)
const TRUNK: i32 = 6 * ONE;
/// a shelf of bark every this far up, the crown's floor at the top
const RUNG: i32 = 9 * ONE;
const RUNGS: i32 = 16;
const CROWN: i32 = SHORE + 150 * ONE;
const CROWN_R: i32 = 20 * ONE;
/// a shelf of bark this wide each way; each a turn this far round from the
/// last; its ring this far over the next shelf's top
const SHELF: i32 = 5 * ONE;
const ROUND: i32 = 34;
const RING_OVER: i32 = ONE * 15 / 10;
/// and this far out past the shelf under it, so a body rising past it
/// does not come down on it
const RING_OUT: i32 = 2 * ONE;
/// the shelves that go all the way round: the climb's ledges
const LEDGES: [i32; 3] = [4, 9, 13];
/// a ledge is this wide each way
const LEDGE: i32 = 8 * ONE;

impl B<'_> {
    /// The Boathouse: a floor of boards over the shallows, its roof, and
    /// the way on along the shore. The rest's pickups stand on its floor.
    pub(super) fn boathouse(&mut self) {
        let t = 0;
        self.mooring(0, t, m(10));
        self.board_walk(-m(12), m(14), m(12), -m(16), t);
        let wood = self.c("river_wood_dark");
        let roof = self.c("sandstone_dark");
        for (x, z) in [(-m(11), m(12)), (m(11), m(12)), (-m(11), -m(12)), (m(11), -m(12))] {
            self.deco(Mesh::Cyl, v3(x, m(6), z), v3(dm(5), m(12), dm(5)), wood);
        }
        // the roof high over the boards, so the camera stays under it
        self.deco(Mesh::Cube, v3(0, m(13), 0), v3(m(26), dm(6), m(28)), roof);
        // the old boats, upturned
        let hull = self.c("river_wood");
        for (x, z) in [(-m(8), -m(4)), (m(8), m(2))] {
            self.deco(Mesh::Blob, v3(x, dm(4), z), v3(m(2), dm(8), m(5)), hull);
        }
        self.way_on(0, t, -m(14));
        self.line(Step::To(v3(0, t, -m(12))));
        self.river_water(-m(80), m(60), m(80), -m(80));
        // the Oak across the water, ahead
        self.oak_far(v3(0, 0, -m(160)));
    }

    /// The Gallows Oak's arena.
    pub(super) fn gallows_oak(&mut self) {
        let axis = v3(0, SHORE, 0);
        self.k().oak = Some(OakSite {
            axis,
            trunk: TRUNK,
            crown_y: CROWN,
            crown_r: CROWN_R,
        });
        // the shore and the roots, south of the trunk
        self.mooring(0, SHORE, m(26));
        self.rock_shelf(-m(14), m(32), m(14), m(8), SHORE);
        self.rock_shelf(-m(10), m(8), m(10), -m(10), SHORE);
        self.line(Step::To(v3(0, SHORE, m(12))));

        // the trunk
        self.solids
            .push(Solid::new(v3(-TRUNK, -m(8), -TRUNK), v3(TRUNK, CROWN, TRUNK)));
        let bark = self.c("oak_bark");
        let eye = self.c("oak_eye");
        let root = self.c("root");
        self.deco(
            Mesh::Cyl,
            v3(0, (CROWN - m(8)) / 2, 0),
            v3(TRUNK * 2 + m(2), CROWN + m(8), TRUNK * 2 + m(2)),
            bark,
        );
        // the trunk's gnarls and its sleeping eyes
        for k in 0..24 {
            let a = deg(self.rng.range(0, 359));
            let y = SHORE + self.metres(4, 146);
            let at = v3(
                fx::mul(fx::cos(a), TRUNK + dm(5)),
                y,
                fx::mul(fx::sin(a), TRUNK + dm(5)),
            );
            if k % 3 == 0 {
                self.glow(Mesh::Sphere, at, v3(dm(9), dm(5), dm(9)), eye);
            } else {
                self.deco(Mesh::Blob, at, v3(m(3), m(4), m(3)), bark);
            }
        }
        // roots running out into the lake
        for k in 0..9 {
            let a = fx::TURN / 9 * k + deg(self.rng.range(-10, 10));
            let out = v3(fx::mul(fx::cos(a), m(30)), -m(1), fx::mul(fx::sin(a), m(30)));
            self.rope_line(v3(0, SHORE, 0), out);
            if let Some(af) = super::super::segment(v3(0, SHORE + m(2), 0), out, m(2)) {
                self.shape(Mesh::Cyl, af, root, 0, true);
            }
        }

        // the climb: a shelf of bark every rung, round and up, and over each
        // shelf a ring hung on a chain from the next: the hook pulls you
        // straight up past the next shelf, and you come down onto it; the
        // ledges go all the way round
        let mut prev = v3(0, SHORE, m(9));
        for k in 1..=RUNGS {
            let y = SHORE + RUNG * k;
            let a = fx::QUARTER + deg(ROUND) * k;
            let ledge = LEDGES.contains(&k);
            let size = if ledge { LEDGE } else { SHELF };
            let c = v3(
                fx::mul(fx::cos(a), TRUNK + size / 2),
                y,
                fx::mul(fx::sin(a), TRUNK + size / 2),
            );
            if ledge {
                // a broad ledge, a mooring post on it
                self.bark_slab(
                    c.x - LEDGE / 2,
                    c.z - LEDGE / 2,
                    c.x + LEDGE / 2,
                    c.z + LEDGE / 2,
                    y,
                );
                self.mooring(c.x, y, c.z);
            } else {
                self.bark_slab(
                    c.x - SHELF / 2,
                    c.z - SHELF / 2,
                    c.x + SHELF / 2,
                    c.z + SHELF / 2,
                    y,
                );
            }
            // the ring over where you stand now, out past the shelf's edge
            let out = v3(prev.x, 0, prev.z);
            let at = out.norm().scale(out.len_xz() + RING_OUT);
            let ring = v3(at.x, y + RING_OVER, at.z);
            self.hook_ring(ring.x, ring.y, ring.z, Some(y + RUNG));
            self.line(Step::Hook(ring));
            self.line(Step::Air(c));
            self.line(Step::To(c));
            prev = c;
        }
        // the crown: the trunk's top, and a ring of boughs with gaps over
        // the lake; its post is where a fall off it returns you
        let top = CROWN;
        self.bark_slab(-TRUNK - m(2), -TRUNK - m(2), TRUNK + m(2), TRUNK + m(2), top);
        let bough = self.c("oak_bark");
        let leaf = self.c("oak_leaf");
        // one of the gaps is over the last shelf: the last pull goes up
        // through it, from a ring hung in it
        let last = fx::QUARTER + deg(ROUND) * RUNGS;
        let turn = last - fx::TURN / 12 * 5 / 2;
        let lip = v3(
            fx::mul(fx::cos(last), TRUNK + SHELF / 2 + RING_OUT),
            top + RING_OVER,
            fx::mul(fx::sin(last), TRUNK + SHELF / 2 + RING_OUT),
        );
        self.hook_ring(lip.x, lip.y, lip.z, Some(top + m(10)));
        self.line(Step::Hook(lip));
        let onto = v3(fx::mul(fx::cos(last), m(4)), top, fx::mul(fx::sin(last), m(4)));
        self.line(Step::Air(onto));
        for k in 0..12 {
            if k % 3 == 2 {
                continue; // a gap: the lake far below
            }
            let a0 = turn + fx::TURN / 12 * k;
            let a1 = a0 + fx::TURN / 12;
            for r in [m(10), m(16)] {
                let mid = (a0 + a1) / 2;
                let at = v3(fx::mul(fx::cos(mid), r), top, fx::mul(fx::sin(mid), r));
                let w = m(7);
                self.solids.push(Solid::new(
                    v3(at.x - w / 2, top - m(2), at.z - w / 2),
                    v3(at.x + w / 2, top, at.z + w / 2),
                ));
                self.deco(Mesh::Cube, v3(at.x, top - m(1), at.z), v3(w, m(2), w), bough);
                // the leaves hang under the boughs, clear of the fight above
                self.deco(
                    Mesh::Blob,
                    v3(at.x, top - m(4), at.z),
                    v3(w + m(2), m(4), w + m(2)),
                    leaf,
                );
            }
        }
        // the spokes joining trunk and ring
        for k in 0..4 {
            let a = turn + fx::TURN / 4 * k + fx::TURN / 24;
            let at = v3(fx::mul(fx::cos(a), m(9)), top, fx::mul(fx::sin(a), m(9)));
            self.solids.push(Solid::new(
                v3(at.x - m(2), top - m(2), at.z - m(2)),
                v3(at.x + m(2), top, at.z + m(2)),
            ));
            self.deco(
                Mesh::Cube,
                v3(at.x, top - m(1), at.z),
                v3(m(4), m(2), m(4)),
                bough,
            );
        }
        self.mooring(0, top, TRUNK);
        self.line(Step::To(v3(0, top, 0)));
        // nothing leads on from the Oak's arena: its way on is under the lake
        self.k().finish = v3(0, -m(60), 0);
        self.river_water(-m(160), m(120), m(160), -m(160));
        self.oak_far(v3(0, 0, -m(4000)));
    }

    /// A slab of bark a metre thick, its top at `y`, standing off the trunk.
    fn bark_slab(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, y: i32) {
        self.solids.push(Solid::new(v3(x0, y - m(1), z0), v3(x1, y, z1)));
        let bark = self.c("oak_bark");
        self.deco(
            Mesh::Cube,
            v3((x0 + x1) / 2, y - dm(5), (z0 + z1) / 2),
            v3(x1 - x0, m(1), z1 - z0),
            bark,
        );
    }

    /// The Oak seen from far off: a black trunk and a dark crown.
    pub(super) fn oak_far(&mut self, at: V3) {
        let bark = self.c("oak_bark");
        let leaf = self.c("oak_leaf");
        self.deco(Mesh::Cyl, at.add(v3(0, m(75), 0)), v3(m(14), m(150), m(14)), bark);
        for k in 0..7 {
            let a = fx::TURN / 7 * k;
            let c = at.add(v3(
                fx::mul(fx::cos(a), m(22)),
                m(150) + m(4) * (k % 3),
                fx::mul(fx::sin(a), m(22)),
            ));
            self.deco(Mesh::Blob, c, v3(m(26), m(14), m(26)), leaf);
        }
    }
}
