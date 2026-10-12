//! The last mission's own zone, home: your bedroom (arrival), the landing
//! at the top of the stairs (rest) and downstairs (the boss: Chumbles).
//! A stub, to be built out.

use super::{boxed, m, Side, B, COURT_RADIUS};
use crate::fx::ONE;
use crate::geom::v3;
use crate::level::Solid;
use crate::mesh::Mesh;

/// Half the width of the bedroom and the landing.
const ROOM: i32 = fx_int(10);

const fn fx_int(n: i32) -> i32 {
    n * ONE
}

impl B<'_> {
    pub(super) fn home(&mut self, kind: &str, exits: &[Side]) {
        let _ = exits;
        let floor = self.c("ground");
        if kind == "boss" {
            let r = COURT_RADIUS + m(4);
            self.solids.push(Solid::new(v3(-r, -m(1), -r), v3(r, 0, r)));
            self.shape(
                Mesh::Cube,
                boxed(v3(0, -ONE / 2, 0), v3(r * 2, ONE, r * 2)),
                floor,
                0,
                false,
            );
            for (x, z) in [(-m(12), -m(10)), (m(12), -m(10)), (0, -m(20))] {
                self.solids.push(Solid::new(
                    v3(x - m(2), 0, z - m(1)),
                    v3(x + m(2), m(2), z + m(1)),
                ));
                self.shape(
                    Mesh::Cube,
                    boxed(v3(x, m(1), z), v3(m(4), m(2), m(2))),
                    floor,
                    0,
                    true,
                );
                self.perches.push(v3(x, m(2), z));
            }
            return;
        }
        self.half = ROOM;
        let r = ROOM + m(2);
        self.solids.push(Solid::new(v3(-r, -m(1), -r), v3(r, 0, r)));
        self.shape(
            Mesh::Cube,
            boxed(v3(0, -ONE / 2, 0), v3(r * 2, ONE, r * 2)),
            floor,
            0,
            false,
        );
    }
}
