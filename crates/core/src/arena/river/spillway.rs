//! The Spillway: under Calder Hill, the house's old weir and its sluices.
//! It teaches Bound Over on the broken weir wall (gaps of 8, 10 and 11 m),
//! the hook over a pool to a ring on a pole, a lever on the bank that opens
//! a sluice gate across the race for three seconds, and slow curtains of the
//! river's orbs from weir-lamps at the end, each with a hole to go through.

use super::super::{dm, m, B};
use crate::course::Step;
use crate::fx::ONE;
use crate::geom::v3;

/// the walk's height over the water
const TOP: i32 = 2 * ONE;

impl B<'_> {
    pub(super) fn spillway(&mut self) {
        let t = TOP;
        let p = |z: i32| v3(0, t, z);

        // 1a: the weir wall, broken three times
        self.mooring(0, t, m(2));
        self.rock_shelf(-m(2), m(6), m(2), -m(40), t);
        self.rock_shelf(-m(2), -m(48), m(2), -m(64), t);
        self.rock_shelf(-m(2), -m(74), m(2), -m(88), t);
        self.rock_shelf(-m(3), -m(99), m(3), -m(120), t);
        self.mooring(0, t, -m(104));
        self.line(Step::To(v3(0, t, -m(38) - dm(6))));
        self.line(Step::Bound(p(-m(56))));
        self.line(Step::To(v3(0, t, -m(62) - dm(6))));
        self.line(Step::Bound(p(-m(80))));
        self.line(Step::To(v3(0, t, -m(86) - dm(6))));
        self.line(Step::Bound(p(-m(104))));

        // the strand: a ledge high in the east wall, a ring to reach it by
        self.rock_shelf(m(7), -m(106), m(11), -m(116), m(7));
        self.hook_ring(m(4), m(11), -m(111), Some(m(30)));
        self.strand_reward(m(9), m(7), -m(111));

        // 1b: the pool, and a ring on a pole at its far side: the hook pulls
        // you over the water and lets go above the landing
        self.hook_ring(0, m(6), -m(146), None);
        self.rock_shelf(-m(3), -m(146), m(3), -m(174), t);
        self.mooring(0, t, -m(152));
        self.line(Step::To(p(-m(118))));
        self.line(Step::Hook(v3(0, m(6), -m(146))));
        self.line(Step::Air(p(-m(152))));
        self.line(Step::To(p(-m(160))));

        // 1c: a lever on the west bank opens the sluice gate on the far
        // side of the race, three seconds
        self.rock_shelf(-m(12), -m(150), -m(7), -m(176), m(5));
        let gate = self.sluice_door(v3(-m(3), t, -m(185)), v3(m(3), t + m(6), -m(184)));
        self.hook_lever(-m(9), m(7), -m(166), vec![gate], vec![], 3 * 60);
        self.rock_shelf(-m(3), -m(184), m(3), -m(226), t);
        self.mooring(0, t, -m(192));
        self.line(Step::To(p(-m(170))));
        self.line(Step::Tug(v3(-m(9), m(7), -m(166))));
        self.line(Step::To(v3(0, t, -m(173) - dm(6))));
        self.line(Step::Bound(p(-m(192))));

        // 1d: weir-lamps on the walls fire curtains up the race
        self.rock_emitter(-m(9), m(6), -m(242), "weir_lamp");
        self.rock_emitter(m(9), m(6), -m(250), "weir_lamp");
        self.rock_shelf(-m(3), -m(230), m(3), -m(264), t);
        self.line(Step::To(v3(0, t, -m(225) - dm(5))));
        self.line(Step::Jump(p(-m(234))));
        self.line(Step::To(p(-m(258))));
        self.way_on(0, t, -m(260));

        // the race, its walls, and the weir's dressing
        self.river_water(-m(12), m(8), m(12), -m(272));
        self.canyon_walls(m(8), -m(272), m(12), m(26));
        for (x0, z0, x1, z1) in [
            (-m(2), m(6), m(2), -m(40)),
            (-m(3), -m(99), m(3), -m(120)),
            (-m(3), -m(146), m(3), -m(174)),
            (-m(3), -m(184), m(3), -m(226)),
        ] {
            self.river_dress(x0, z0, x1, z1, t, 6);
        }
        // the old sluice frames across the race, overhead
        let wood = self.c("river_wood_dark");
        for z in [-m(44), -m(94), -m(140), -m(228)] {
            self.deco(
                crate::mesh::Mesh::Cube,
                v3(0, m(14), z),
                v3(m(24), dm(8), dm(8)),
                wood,
            );
            for x in [-m(11), m(11)] {
                self.deco(
                    crate::mesh::Mesh::Cube,
                    v3(x, m(7), z),
                    v3(dm(8), m(14), dm(8)),
                    wood,
                );
            }
        }
    }
}
