//! Zone five, inside the Idiot God (placeholder: a floor in the void).

use super::{boxed, m, Side, B, COURT_RADIUS, HALF};
use crate::balance::WRAP_BELOW;
use crate::geom::v3;
use crate::level::Solid;
use crate::mesh::Mesh;

impl B<'_> {
    pub(super) fn god(&mut self, kind: &str, exits: &[Side]) {
        let _ = exits;
        self.wrap = m(64);
        let _ = WRAP_BELOW;
        let half = if kind == "boss" { COURT_RADIUS } else { HALF };
        let flesh = self.c("god_flesh");
        self.solids.push(Solid::new(
            v3(-half - m(2), -m(3), -half - m(2)),
            v3(half + m(2), 0, half + m(2)),
        ));
        self.shape(
            Mesh::Cube,
            boxed(v3(0, -m(3) / 2, 0), v3(half * 2 + m(4), m(3), half * 2 + m(4))),
            flesh,
            0,
            true,
        );
    }
}
