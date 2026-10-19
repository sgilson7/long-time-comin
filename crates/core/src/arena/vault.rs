//! The Vault (under Stillwater): the alien building under the lake bed,
//! older than the Oak. White halls, black glass, chrome, light with nothing
//! to cast it.
//!
//! - the Shaft (`shaft`): down a vast shaft of rings and gantries
//! - the Halls (`halls`): long halls, fought, against the Vault's machines
//!
//! and the Eraser (the boss): its chamber, three pylons round it held in
//! turn while your older self, who cannot be hurt here, tries to stop it.

use super::{m, B};
use crate::course::{Charge, Course};
use crate::fx::ONE;
use crate::geom::v3;

mod eraser;
mod halls;
mod shaft;

impl B<'_> {
    pub(super) fn vault(&mut self, kind: &str) {
        self.course = Some(Course {
            // the Vault has no water: a long fall returns you to the post
            water: -m(20),
            ..Course::default()
        });
        self.start_yaw = 0;
        match kind {
            "halls" => self.halls(),
            "boss" => self.eraser(),
            _ => self.shaft(),
        }
    }

    /// The Eraser's pylons, held in turn, each `need` ticks within `radius`.
    pub(super) fn eraser_pylons(&mut self, at: Vec<crate::geom::V3>, radius: i32, need: i32) {
        let n = at.len();
        self.k().charge = Some(Charge {
            pylons: at,
            radius,
            need,
            held: vec![0; n],
        });
        let _ = ONE;
        let _ = v3(0, 0, 0);
    }
}
