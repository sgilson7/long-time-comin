//! The Vault (under Stillwater): the alien building under the lake bed,
//! older than the Oak. White halls, black glass, chrome, light with nothing
//! to cast it.
//!
//! - the Shaft (`shaft`): down a vast shaft of rings and gantries
//! - the Halls (`halls`): long halls, fought, against the Vault's machines
//!
//! and the Eraser (the boss): its chamber, where the Vault's keeper, the
//! Archivist, flies on its jet pack; when it falls the Eraser opens, and
//! you step into it.

use super::{m, B};
use crate::course::Course;
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
            "rest" => self.the_lock(),
            _ => self.shaft(),
        }
    }

    /// The Lock: a still white room between the Halls and the Eraser, where
    /// the rest's two pedestals stand; black glass walls, a cyan rule.
    fn the_lock(&mut self) {
        let (w, near, far, h) = (m(10), m(14), -m(18), m(8));
        self.vault_slab(-w, near, w, far, 0);
        self.vault_post(0, 0, m(10));
        let glass = self.c("black_glass");
        let cyan = self.c("vault_cyan");
        let white = self.c("vault_white");
        for x in [-w - m(1), w + m(1)] {
            self.block(x, 0, (near + far) / 2, m(2), h, near - far, glass);
            self.glow(
                crate::mesh::Mesh::Cube,
                v3(x.signum() * w, m(3), (near + far) / 2),
                v3(crate::arena::dm(2), crate::arena::dm(2), near - far),
                cyan,
            );
        }
        self.deco(
            crate::mesh::Mesh::Cube,
            v3(0, h + crate::arena::dm(5), (near + far) / 2),
            v3(w * 2 + m(4), m(1), near - far),
            white,
        );
        self.vault_door(0, 0, far + m(2));
        self.line(crate::course::Step::To(v3(0, 0, far + m(4))));
    }
}

#[cfg(test)]
mod lock {
    use crate::course::tests::{course_world_in, cross_in};

    #[test]
    fn the_vaults_rest_is_the_lock_a_small_room_not_the_shaft_again() {
        let (_, w) = course_world_in("vault", "rest");
        let k = w.course.as_ref().unwrap();
        assert_eq!(k.posts.len(), 1, "one post: {:?}", k.posts);
        cross_in("vault", "rest", false).unwrap();
    }
}
