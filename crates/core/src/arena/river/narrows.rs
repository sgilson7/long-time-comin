//! The Narrows: a slot canyon ten metres across and forty high, the river
//! fast at its foot. It teaches the Bench: laid out over a gap with no
//! anchor, a plank to land on and Bound Over again from; then a chain of
//! hooks from wall to wall, root-eyes in the rock firing lattices across
//! the slot while you stand on a Bench, and a lever on the east wall that
//! opens a gate through a fin of rock on the west for three seconds.

use super::super::{dm, m, B};
use crate::course::Step;
use crate::fx::ONE;
use crate::geom::v3;

/// the first shelf's height over the water, and the rest's, a step lower
const TOP: i32 = 2 * ONE;
const LOW: i32 = ONE;
/// the slot's half-width, wall face to the river's middle
const HALF: i32 = 5 * ONE;

impl B<'_> {
    pub(super) fn narrows(&mut self) {
        let (t, h) = (TOP, LOW);
        let p = |z: i32| v3(0, h, z);

        // 2a: a 24 m gap and nothing to hook. Lay the Bench out over it
        // from the edge, Bound Over onto it, and again to the far side
        self.mooring(0, t, m(2));
        self.rock_shelf(-m(4), m(6), m(4), -m(30), t);
        self.rock_shelf(-m(4), -m(54), m(4), -m(86), h);
        self.mooring(0, h, -m(64));
        self.line(Step::To(v3(0, t, -m(29) - dm(8))));
        self.line(Step::Bench);
        self.line(Step::Bound(v3(0, t, -m(38))));
        self.line(Step::To(v3(0, t, -m(42) - dm(3))));
        self.line(Step::Bound(p(-m(66))));

        // 2b: ledges off alternate walls, each with a ring hung over its
        // near end: the pull carries you over the water to it
        self.line(Step::To(p(-m(82))));
        let mut from_east = true;
        for z in [-m(112), -m(148), -m(184)] {
            let side = if from_east { 1 } else { -1 };
            let x = side * m(3);
            self.rock_shelf(0, z, side * HALF, z - m(18), h);
            self.hook_ring(x, h + m(4), z, Some(m(40)));
            self.line(Step::Hook(v3(x, h + m(4), z)));
            self.line(Step::Air(v3(side * m(2), h, z - m(6))));
            self.line(Step::To(v3(side * m(2), h, z - m(14))));
            from_east = !from_east;
        }
        self.mooring(m(1), h, -m(194));

        // 2c: root-eyes in the walls lattice the slot. Two gaps, each
        // crossed on a Bench laid out from the edge, where you stand
        // while a lattice goes by under you
        self.rock_shelf(-m(4), -m(200), m(4), -m(214), h);
        self.rock_shelf(-m(4), -m(229), m(4), -m(250), h);
        self.rock_shelf(-m(4), -m(265), m(4), -m(288), h);
        self.rock_emitter(-HALF + dm(5), m(9), -m(224), "root_eye");
        self.rock_emitter(HALF - dm(5), m(9), -m(260), "root_eye");
        self.line(Step::To(p(-m(206))));
        for edge in [-m(214), -m(250)] {
            // the Bench goes down twelve metres out from where you stand;
            // Bound Over short of it and the steer back brings you down
            self.line(Step::To(p(edge + dm(4))));
            self.line(Step::Bench);
            self.line(Step::Bound(p(edge - m(8))));
            self.line(Step::Wait(40));
            self.line(Step::To(p(edge - m(11) - dm(8))));
            self.line(Step::Jump(p(edge - m(20))));
            // the Bench's cooldown
            self.line(Step::Wait(60));
        }
        self.mooring(0, h, -m(272));

        // 2d: a fin of rock across the slot, its gate on the west side; a
        // lever on the east wall opens it three seconds while an eye
        // lattices the way through
        let gate = self.sluice_door(v3(-HALF, h, -m(297)), v3(-m(1), h + m(8), -m(296)));
        self.rock_shelf(-m(1), -m(296), HALF, -m(300), m(40));
        self.hook_lever(HALF - dm(8), h + m(7), -m(291), vec![gate], vec![], 3 * 60);
        self.rock_emitter(-HALF + dm(5), m(10), -m(312), "root_eye");
        self.rock_shelf(-m(4), -m(296), m(4), -m(330), h);
        self.line(Step::To(v3(-m(2) - dm(5), h, -m(278))));
        self.line(Step::Tug(v3(HALF - dm(8), h + m(7), -m(291))));
        self.line(Step::To(v3(-m(3), h, -m(286) + dm(8))));
        self.line(Step::Bound(v3(-m(3), h, -m(304))));
        self.line(Step::To(p(-m(328) - dm(4))));

        // the last gap, the strand high on the west wall, and the way on
        self.rock_shelf(-m(4), -m(340), m(4), -m(412), h);
        self.line(Step::Bound(p(-m(348))));
        self.rock_shelf(-HALF, -m(360), -m(2), -m(370), m(9));
        self.hook_ring(-m(3), m(11), -m(360), Some(m(40)));
        self.strand_reward(-m(3) - dm(5), m(9), -m(366));
        self.line(Step::To(p(-m(398))));
        self.way_on(0, h, -m(400));

        // the river, the slot's walls, and its dressing
        self.river_water(-HALF, m(8), HALF, -m(414));
        self.canyon_walls(m(8), -m(414), HALF, m(40));
        for (x0, z0, x1, z1) in [
            (-m(4), m(6), m(4), -m(30)),
            (-m(4), -m(54), m(4), -m(86)),
            (-m(4), -m(229), m(4), -m(250)),
            (-m(4), -m(296), m(4), -m(330)),
            (-m(4), -m(340), m(4), -m(412)),
        ] {
            let top = if z0 > 0 { t } else { h };
            self.river_dress(x0, z0, x1, z1, top, 6);
        }
        // drift timber jammed between the walls, high over the slot
        let wood = self.c("river_wood_dark");
        for (z, y) in [
            (-m(40), m(30)),
            (-m(130), m(24)),
            (-m(240), m(33)),
            (-m(370), m(27)),
        ] {
            self.rod(v3(-HALF, y, z), v3(HALF, y + m(2), z - m(1)), dm(5), wood, true);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_narrows_can_be_crossed_on_its_line_with_the_rivers_kit() {
        let (ticks, _lost) = crate::course::tests::cross("narrows").unwrap();
        assert!(ticks < 60 * 150, "{ticks}");
    }
}
