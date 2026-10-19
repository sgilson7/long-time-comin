//! stub
use super::super::{m, B};
use crate::course::Step;
use crate::geom::v3;

impl B<'_> {
    pub(super) fn eraser(&mut self) {
        self.mooring(0, 0, m(2));
        self.board_walk(-m(4), m(6), m(4), -m(40), 0);
        self.line(Step::To(v3(0, 0, -m(36))));
        self.k().boss_at = Some(v3(0, m(1), -m(20)));
        self.k().finish = v3(0, -m(80), 0);
        self.eraser_pylons(
            vec![v3(-m(3), 0, -m(10)), v3(m(3), 0, -m(20)), v3(0, 0, -m(30))],
            m(3),
            20 * 60,
        );
    }
}
