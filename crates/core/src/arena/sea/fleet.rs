//! stub
use super::super::{m, B};
use crate::course::Step;
use crate::geom::v3;

impl B<'_> {
    pub(super) fn fleet(&mut self) {
        self.mooring(0, 0, m(2));
        self.board_walk(-m(4), m(6), m(4), -m(40), 0);
        self.line(Step::To(v3(0, 0, -m(36))));
        self.way_on(0, 0, -m(38));
        self.k().fought = true;
        self.crew_member(v3(0, m(1), -m(20)), "deckhand");
        self.sea_surface(-m(60), m(20), m(60), -m(60));
    }
}
