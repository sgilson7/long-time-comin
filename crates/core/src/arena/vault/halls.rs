//! stub
use super::super::{m, B};
use crate::course::Step;
use crate::geom::v3;

impl B<'_> {
    pub(super) fn halls(&mut self) {
        self.mooring(0, 0, m(2));
        self.board_walk(-m(4), m(6), m(4), -m(40), 0);
        self.line(Step::To(v3(0, 0, -m(36))));
        self.way_on(0, 0, -m(38));
        self.k().fought = true;
        self.k().spawns = vec![v3(-m(2), 0, -m(20)), v3(m(2), 0, -m(25)), v3(0, 0, -m(30))];
    }
}
