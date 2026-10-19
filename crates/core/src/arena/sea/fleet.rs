//! The Fleet: open sea under the anomaly's sky, and seven ships of the
//! Drowned Fleet at anchor in a loose ring, each heaving gently on the
//! swell, each with its crew on board. You fight across them in turn: a
//! gangplank from the first to the second, a rope bridge to the third, a
//! Bench over open water to the fourth, the hook to the fifth's foremast,
//! Bound Over to the sixth, and the hook again to the flagship's, where
//! the Enslaved Dragon sits high on the mainmast. The way on is at the
//! flagship's bow, and opens when every crew is down.

use super::super::{dm, m, B};
use crate::course::Step;
use crate::fx::TURN;
use crate::geom::{v3, V3};
use crate::mesh::Mesh;

/// a deck's height over the water, and the flagship's, higher
const TOP: i32 = 5 * crate::fx::ONE / 2;
const HIGH: i32 = 3 * crate::fx::ONE;
/// the swell: how far a deck heaves, and the ships the planks join less,
/// so a deck never stands higher than a step over the plank
const RISE: i32 = 6 * crate::fx::ONE / 10;
const LASHED: i32 = 4 * crate::fx::ONE / 10;

impl B<'_> {
    pub(super) fn fleet(&mut self) {
        let t = TOP;
        let still = V3::ZERO;
        let on = |x: i32, z: i32| v3(x, t + m(1), z);

        // 1a, 1b: the first ship, and a gangplank from her port side to
        // the second, lying broadside to her
        self.ship(
            -m(4),
            m(10),
            m(4),
            -m(14),
            t,
            &[(m(2), m(5), m(14)), (m(2), -m(8), m(12))],
            LASHED,
            still,
            300,
            0,
        );
        self.mooring(0, t, -m(2));
        self.crew_member(on(-m(2), -m(10)), "deckhand");
        self.crew_member(on(-m(2), m(7)), "gunner");
        self.ship(
            -m(34),
            -m(6),
            -m(14),
            -m(14),
            t,
            &[(-m(30), -m(7), m(13)), (-m(19), -m(7), m(11))],
            LASHED,
            still,
            300,
            0,
        );
        self.board_walk(-m(15), -m(9), -m(3), -m(11), t);
        self.line(Step::To(v3(-m(3), t, -m(10))));
        self.line(Step::To(v3(-m(16), t, -m(10))));
        self.mooring(-m(24), t, -m(10));
        self.line(Step::To(v3(-m(24), t, -m(10))));
        self.crew_member(on(-m(22), -m(12)), "deckhand");
        self.crew_member(on(-m(31), -m(11)), "powder_monkey");
        self.crew_member(v3(-m(30), t + m(14), -m(7)), "lookout");

        // 1c: a rope bridge from her bow to the third, ahead
        self.ship(
            -m(30),
            -m(26),
            -m(22),
            -m(50),
            t,
            &[(-m(23), -m(32), m(14)), (-m(23), -m(44), m(12))],
            LASHED,
            still,
            300,
            0,
        );
        self.board_walk(-m(27), -m(13), -m(25), -m(27), t);
        self.line(Step::To(v3(-m(26), t, -m(13))));
        self.line(Step::To(v3(-m(26), t, -m(28))));
        self.mooring(-m(26), t, -m(38));
        self.line(Step::To(v3(-m(26), t, -m(38))));
        self.crew_member(on(-m(28), -m(30)), "bosun");
        self.crew_member(on(-m(28), -m(46)), "deckhand");
        self.crew_member(on(-m(25), -m(40)), "gunner");

        // 1d: twenty-two metres of open water to the fourth. Lay the Bench
        // out from her bow, Bound Over onto it, and again to the next deck
        self.ship(
            -m(32),
            -m(72),
            -m(18),
            -m(98),
            t,
            &[(-m(29), -m(78), m(15)), (-m(29), -m(92), m(13))],
            RISE,
            still,
            270,
            TURN / 2,
        );
        self.line(Step::To(v3(-m(26), t, -m(49) - dm(2))));
        self.line(Step::Bench);
        self.line(Step::Bound(v3(-m(26), t, -m(58))));
        self.line(Step::Wait(40));
        self.line(Step::To(v3(-m(26), t, -m(61) - dm(8))));
        self.line(Step::Bound(v3(-m(26), t, -m(78))));
        self.mooring(-m(25), t, -m(85));
        self.line(Step::To(v3(-m(25), t, -m(85))));
        self.crew_member(on(-m(23), -m(76)), "deckhand");
        self.crew_member(on(-m(23), -m(94)), "deckhand");
        self.crew_member(v3(-m(29), t + m(14), -m(92)), "lookout");
        self.crew_member(v3(-m(26), t + m(6), -m(88)), "cockatrice");

        // the strand: a jolly boat off her port side, Bound Over to it
        self.ship(
            -m(50),
            -m(80),
            -m(44),
            -m(90),
            dm(15),
            &[],
            RISE,
            still,
            330,
            TURN / 2,
        );
        self.strand_reward(-m(47), dm(15), -m(85));

        // 1e: the fifth lies twenty metres off her bow; hook her foremast,
        // which stands near her stern straight ahead, and the pull brings
        // you up against it, over her deck
        self.ship(
            -m(27),
            -m(118),
            -m(15),
            -m(142),
            t,
            &[(-m(21), -m(122), m(14)), (-m(18), -m(136), m(12))],
            RISE,
            still,
            330,
            TURN / 2,
        );
        self.line(Step::To(v3(-m(21), t, -m(96))));
        self.line(Step::Hook(v3(-m(21), t + m(4), -m(122))));
        self.line(Step::Air(v3(-m(22), t, -m(126))));
        self.mooring(-m(21), t, -m(130));
        self.line(Step::To(v3(-m(21), t, -m(130))));
        self.crew_member(on(-m(24), -m(126)), "gunner");
        self.crew_member(on(-m(24), -m(139)), "powder_monkey");
        self.crew_member(on(-m(18), -m(130)), "deckhand");

        // 1f: the sixth, eight metres on and to starboard: Bound Over, deck
        // to deck
        self.ship(
            -m(18),
            -m(150),
            -m(6),
            -m(174),
            t,
            &[(-m(8), -m(156), m(13)), (-m(8), -m(168), m(11))],
            RISE,
            still,
            300,
            TURN * 3 / 4,
        );
        self.line(Step::To(v3(-m(17), t, -m(141) - dm(2))));
        self.line(Step::Bound(v3(-m(14), t, -m(154))));
        self.mooring(-m(12), t, -m(162));
        self.line(Step::To(v3(-m(12), t, -m(162))));
        self.crew_member(on(-m(15), -m(158)), "bosun");
        self.crew_member(on(-m(15), -m(171)), "deckhand");
        self.crew_member(v3(-m(8), t + m(12), -m(168)), "lookout");

        // 1g: the flagship, a galleon twenty metres on; hook her foremast
        // and come down on her deck. The dragon sits on her mainmast
        let h = HIGH;
        self.ship(
            -m(16),
            -m(194),
            m(8),
            -m(244),
            h,
            &[
                (-m(12), -m(199), m(16)),
                (m(2), -m(218), m(22)),
                (m(2), -m(235), m(16)),
            ],
            RISE,
            still,
            360,
            0,
        );
        self.line(Step::To(v3(-m(12), t, -m(173))));
        self.line(Step::Hook(v3(-m(12), h + m(5), -m(199))));
        self.line(Step::Air(v3(-m(11), h, -m(203))));
        self.mooring(-m(6), h, -m(219));
        self.line(Step::To(v3(-m(6), h, -m(219))));
        self.crew_member(v3(-m(10), h + m(1), -m(210)), "chimera");
        self.crew_member(v3(-m(10), h + m(1), -m(228)), "bosun");
        self.crew_member(v3(m(4), h + m(1), -m(240)), "gunner");
        self.crew_member(v3(m(2), h + m(23), -m(218)), "enslaved_dragon");
        self.line(Step::To(v3(-m(6), h, -m(238))));
        self.way_on(-m(4), h, -m(242));
        self.k().fought = true;
        // a wave, if the section has one, comes in on the decks
        for (x, z) in [
            (-m(28), -m(10)),
            (-m(26), -m(34)),
            (-m(26), -m(80)),
            (-m(24), -m(126)),
            (-m(15), -m(166)),
        ] {
            self.k().spawns.push(v3(x, t, z));
        }
        self.k().spawns.push(v3(-m(10), h, -m(224)));

        // the sea, and the island on the horizon the fleet runs for
        self.sea_surface(-m(100), m(60), m(80), -m(320));
        let sand = self.c("sandstone_pale");
        let green = self.c("moss");
        self.deco(Mesh::Blob, v3(m(20), 0, -m(420)), v3(m(90), m(14), m(40)), sand);
        self.deco(
            Mesh::Blob,
            v3(m(24), m(6), -m(424)),
            v3(m(50), m(16), m(26)),
            green,
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_fleet_can_be_crossed_and_cleared_on_its_line() {
        let (ticks, _) = crate::course::tests::cross_in("far_shore", "fleet", true).unwrap();
        assert!(ticks < 60 * 200, "{ticks}");
    }
}
