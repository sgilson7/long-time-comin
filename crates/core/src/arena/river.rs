//! The Water Gap (2066): the river Calder from under Calder Hill down to
//! Lake Stillwater. Each section is a course (`course.rs`), laid north
//! along −z from its first mooring post, the water at y = 0 under it:
//!
//! - the Spillway (`spillway`): the house's old weir and sluices; Bound
//!   Over, the hook over water, the first switch and its gate, slow curtains
//! - the Narrows (`narrows`): a slot canyon; the Bench
//! - the Falls (`falls`): a cascade down five stages; the ground pound
//! - the Shallows (`shallows`): the lake's flooded edge; all of it at once
//!
//! and the Boathouse (rest) and the Gallows Oak (the boss). The builders lay
//! the line a player is meant to take as they go (`Course::line`), and the
//! tests drive a body along it to prove each section can be crossed.

use super::{boxed, dm, m, segment, Side, B};
use crate::course::{Course, Door, Emitter, Mover, Step, Switch, SwitchKind};
use crate::fx::{self, ONE};
use crate::geom::{v3, Affine, V3};
use crate::level::{Dyn, Solid};
use crate::mesh::Mesh;

mod falls;
mod narrows;
mod oak;
mod shallows;
mod spillway;

/// The water's surface, and where a body drowns (a little under it).
pub(crate) const SURFACE: i32 = 0;
const DROWN_AT: i32 = -ONE / 2;
/// rock goes this far under the water
const BED: i32 = -fx::int(8);

#[allow(clippy::too_many_arguments)]
impl B<'_> {
    pub(super) fn river(&mut self, kind: &str, _exits: &[Side]) {
        self.course = Some(Course {
            water: DROWN_AT,
            ..Course::default()
        });
        self.start_yaw = 0;
        match kind {
            "narrows" => self.narrows(),
            "falls" => self.falls(),
            "shallows" => self.shallows(),
            "rest" => self.boathouse(),
            "boss" => self.gallows_oak(),
            _ => self.spillway(),
        }
    }

    pub(super) fn k(&mut self) -> &mut Course {
        self.course.get_or_insert_with(Course::default)
    }

    pub(super) fn line(&mut self, s: Step) {
        self.k().line.push(s);
    }

    // ---- ground ------------------------------------------------------------

    /// A shelf of sandstone from under the water to `top`, over x0..x1 by
    /// z0..z1, its top lip paler.
    pub(super) fn rock_shelf(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, top: i32) {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        self.solids.push(Solid::new(v3(x0, BED, z0), v3(x1, top, z1)));
        let body = self.c("sandstone");
        let lip = self.c("sandstone_pale");
        // the body stops inside the lip: two tops in one plane flicker
        let h = top - dm(2) - BED;
        self.deco(
            Mesh::Cube,
            v3((x0 + x1) / 2, BED + h / 2, (z0 + z1) / 2),
            v3(x1 - x0, h, z1 - z0),
            body,
        );
        self.deco(
            Mesh::Cube,
            v3((x0 + x1) / 2, top - dm(2), (z0 + z1) / 2),
            v3(x1 - x0 + dm(2), dm(4), z1 - z0 + dm(2)),
            lip,
        );
    }

    /// A walk of planks on posts from the water to `top`.
    pub(super) fn board_walk(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, top: i32) {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        self.solids
            .push(Solid::new(v3(x0, top - dm(4), z0), v3(x1, top, z1)));
        let wood = self.c("river_wood");
        let dark = self.c("river_wood_dark");
        self.deco(
            Mesh::Cube,
            v3((x0 + x1) / 2, top - dm(2), (z0 + z1) / 2),
            v3(x1 - x0, dm(4), z1 - z0),
            wood,
        );
        let mut z = z0 + m(1);
        while z < z1 {
            for x in [x0 + dm(3), x1 - dm(3)] {
                // up into the planks, not to their top
                let post = top - dm(2);
                self.deco(
                    Mesh::Cyl,
                    v3(x, (post + BED) / 2, z),
                    v3(dm(3), post - BED, dm(3)),
                    dark,
                );
            }
            z += m(5);
        }
    }

    /// Canyon walls either side of the river from z0 to z1: `half` out from
    /// x = 0 and `high` above the water. Solid, so nothing is lost sideways.
    pub(super) fn canyon_walls(&mut self, z0: i32, z1: i32, half: i32, high: i32) {
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        let colours = [
            self.c("sandstone"),
            self.c("sandstone_dark"),
            self.c("sandstone_pale"),
        ];
        for side in [-1, 1] {
            let x = side * half;
            self.solids.push(Solid::new(
                v3(x.min(x + side * m(6)), BED, z0),
                v3(x.max(x + side * m(6)), high, z1),
            ));
            // the face in strata, block by block
            let mut z = z0;
            let mut k = 0;
            while z < z1 {
                // its joins off the tenth-metre grid too (a shelf's lip ends
                // on it)
                let first = if k == 0 { ONE / 40 } else { 0 };
                let mut len = (self.metres(10, 22) + first).min(z1 - z);
                if z1 - z - len < m(1) {
                    // no sliver of a block at the end: its face would lie in
                    // the one before it
                    len = z1 - z;
                }
                let jut = self.metres(0, 2);
                let mut y = BED;
                let mut band = 0;
                // the top band stops a little short: a shelf's lip may
                // lie at `high` (two tops in one plane flicker)
                let top = high - dm(1);
                // the bands' joins and faces stand a quarter of a tenth off
                // the tenth-metre grid the ledges, beams and posts are laid
                // on, so none of them lies in a stratum's plane
                let off = ONE / 40;
                while y < top {
                    let mut t = (self.metres(3, 7) + if band == 0 { off } else { 0 }).min(top - y);
                    if top - y - t < m(1) {
                        // no sliver of a band at the top
                        t = top - y;
                    }
                    let rgb = colours[(k + band) % 3];
                    let face = x + side * (m(3) - jut - dm(self.rng.range(0, 8)) + off);
                    self.deco(
                        Mesh::Cube,
                        v3(face, y + t / 2, z + len / 2),
                        v3(m(6), t, len),
                        rgb,
                    );
                    y += t;
                    band += 1;
                }
                z += len;
                k += 1;
            }
        }
    }

    /// The river's surface over x0..x1 by z0..z1, and foam streaks on it.
    pub(super) fn river_water(&mut self, x0: i32, z0: i32, x1: i32, z1: i32) {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        let blue = self.c("river_water");
        let deep = self.c("river_deep");
        let foam = self.c("river_foam");
        self.shape(
            Mesh::Ground,
            boxed(
                v3((x0 + x1) / 2, SURFACE - dm(1), (z0 + z1) / 2),
                v3(x1 - x0, ONE, z1 - z0),
            ),
            blue,
            0,
            false,
        );
        self.shape(
            Mesh::Ground,
            boxed(
                v3((x0 + x1) / 2, SURFACE - m(3), (z0 + z1) / 2),
                v3(x1 - x0, ONE, z1 - z0),
            ),
            deep,
            0,
            false,
        );
        let n = ((z1 - z0) / m(6)).clamp(1, 80);
        for _ in 0..n {
            let x = x0 + self.metres(1, ((x1 - x0) / ONE).max(2) - 1);
            let z = z0 + self.metres(1, ((z1 - z0) / ONE).max(2) - 1);
            let len = self.metres(2, 6);
            self.shape(
                Mesh::Panel,
                boxed(v3(x, SURFACE + dm(1) / 2, z), v3(dm(3), ONE, len)),
                foam,
                ONE / 3,
                false,
            );
        }
    }

    // ---- the course's furniture -----------------------------------------

    /// A mooring post: where the water returns you once you have reached it.
    pub(super) fn mooring(&mut self, x: i32, y: i32, z: i32) {
        self.k().posts.push(v3(x, y, z));
        let wood = self.c("river_wood_dark");
        let rope = self.c("ascent_rope");
        let flag = self.c("brass");
        // off to the left, clear of the camera over your right shoulder
        let px = x - m(3);
        self.deco(Mesh::Cyl, v3(px, y + dm(9), z), v3(dm(3), dm(18), dm(3)), wood);
        self.deco(Mesh::Ring, v3(px, y + dm(12), z), v3(dm(5), dm(1), dm(5)), rope);
        self.glow(
            Mesh::Cube,
            v3(px + dm(2), y + dm(16), z),
            v3(dm(3), dm(2), dm(1)),
            flag,
        );
    }

    /// An iron ring the hook catches: on a pole from the water, or hung on
    /// a chain from `hang` above.
    pub(super) fn hook_ring(&mut self, x: i32, y: i32, z: i32, hang: Option<i32>) {
        self.solids.push(Solid::new(
            v3(x - dm(5), y - dm(5), z - dm(5)),
            v3(x + dm(5), y + dm(5), z + dm(5)),
        ));
        let iron = self.c("iron");
        let brass = self.c("brass");
        self.shape(
            Mesh::Ring,
            Affine::translate(v3(x, y, z))
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&Affine::scale(v3(dm(14), dm(2), dm(14)))),
            brass,
            ONE / 3,
            true,
        );
        match hang {
            Some(top) => self.rod(v3(x, y + dm(7), z), v3(x, top, z), dm(1), iron, false),
            None => self.rod(v3(x, BED, z), v3(x, y - dm(6), z), dm(4), iron, true),
        }
    }

    /// A lever with a ring at its head: hook the ring and it throws,
    /// opening `opens` (doors) and silencing `silences` (emitters) for
    /// `ticks`. Its switch, by index.
    pub(super) fn hook_lever(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        opens: Vec<usize>,
        silences: Vec<usize>,
        ticks: i32,
    ) -> usize {
        let at = Solid::new(
            v3(x - dm(6), y - dm(6), z - dm(6)),
            v3(x + dm(6), y + dm(6), z + dm(6)),
        );
        self.solids.push(at);
        let iron = self.c("iron");
        let brass = self.c("brass");
        self.rod(v3(x, y - m(2), z), v3(x, y, z), dm(2), iron, true);
        self.deco(Mesh::Cube, v3(x, y - m(2), z), v3(dm(8), dm(4), dm(8)), iron);
        self.shape(
            Mesh::Ring,
            Affine::translate(v3(x, y, z))
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&Affine::scale(v3(dm(10), dm(2), dm(10)))),
            brass,
            ONE / 2,
            true,
        );
        let k = self.k();
        k.switches.push(Switch {
            kind: SwitchKind::Lever,
            at,
            opens,
            silences,
            ticks,
        });
        k.switches.len() - 1
    }

    /// A plate in the floor a ground pound throws.
    pub(super) fn pound_plate(
        &mut self,
        x: i32,
        top: i32,
        z: i32,
        opens: Vec<usize>,
        silences: Vec<usize>,
        ticks: i32,
    ) -> usize {
        let at = Solid::new(v3(x - m(1), top - dm(1), z - m(1)), v3(x + m(1), top, z + m(1)));
        let brass = self.c("brass");
        self.glow(Mesh::Cyl, v3(x, top + dm(1) / 2, z), v3(m(2), dm(1), m(2)), brass);
        let k = self.k();
        k.switches.push(Switch {
            kind: SwitchKind::Plate,
            at,
            opens,
            silences,
            ticks,
        });
        k.switches.len() - 1
    }

    /// A gate across x0..x1, y0..y1, z0..z1 that a switch opens for a while:
    /// drawn as it stands, each tick (`draw.rs`). Its index.
    pub(super) fn sluice_door(&mut self, min: V3, max: V3) -> usize {
        self.dynamic.push(Dyn {
            solid: Solid::new(min, max),
            on: true,
            one_way: false,
        });
        let dynamic = self.dynamic.len() - 1;
        let iron = self.c("iron");
        // its frame stands whatever the gate does, a hair clear of the
        // gate's sides (the rock the gate is let into may lie in them)
        let gap = ONE / 50;
        for x in [min.x - dm(3) - gap / 2, max.x + dm(3) + gap / 2] {
            self.deco(
                Mesh::Cube,
                v3(x, (min.y + max.y) / 2 + dm(3), (min.z + max.z) / 2),
                v3(dm(6) - gap, max.y - min.y + dm(6), max.z - min.z + dm(4)),
                iron,
            );
        }
        let k = self.k();
        k.doors.push(Door {
            dynamic,
            left: 0,
            total: 0,
        });
        k.doors.len() - 1
    }

    /// Something set in the rock that fires `enemy`'s patterns while you
    /// are near. Its index among the emitters.
    pub(super) fn rock_emitter(&mut self, x: i32, y: i32, z: i32, enemy: &str) -> usize {
        let k = self.k();
        k.emitters.push(Emitter {
            pos: v3(x, y, z),
            enemy: enemy.to_string(),
        });
        k.emitters.len() - 1
    }

    /// A lamp: an emitter you can shoot out, which throws a switch as it goes.
    pub(super) fn lamp_switch(
        &mut self,
        x: i32,
        y: i32,
        z: i32,
        enemy: &str,
        opens: Vec<usize>,
        silences: Vec<usize>,
        ticks: i32,
    ) -> usize {
        let e = self.rock_emitter(x, y, z, enemy);
        let at = Solid::new(
            v3(x - dm(5), y - dm(5), z - dm(5)),
            v3(x + dm(5), y + dm(5), z + dm(5)),
        );
        let k = self.k();
        k.switches.push(Switch {
            kind: SwitchKind::Lamp(e),
            at,
            opens,
            silences,
            ticks,
        });
        e
    }

    /// A root (or anything) that rises `rise` and sinks again every
    /// `period` ticks, carrying what stands on it.
    pub(super) fn breathing(
        &mut self,
        x0: i32,
        z0: i32,
        x1: i32,
        z1: i32,
        low_top: i32,
        rise: i32,
        period: i32,
        phase: i32,
    ) {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        let base = Solid::new(v3(x0, low_top - m(2), z0), v3(x1, low_top, z1));
        self.dynamic.push(Dyn {
            solid: base,
            on: true,
            one_way: false,
        });
        let dynamic = self.dynamic.len() - 1;
        self.k()
            .movers
            .push(Mover::breathing(dynamic, base, rise, period, phase));
    }

    /// A rotten timber floor a ground pound breaks.
    pub(super) fn rotten_floor(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, top: i32) {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        self.dynamic.push(Dyn {
            solid: Solid::new(v3(x0, top - dm(5), z0), v3(x1, top, z1)),
            on: true,
            one_way: false,
        });
        let i = self.dynamic.len() - 1;
        self.k().rotten.push(i);
    }

    /// A drum-cap fungus on the rock: a ground pound on it bounces twice as
    /// high. `r` across, its top at `top`.
    pub(super) fn drum_cap(&mut self, x: i32, top: i32, z: i32, r: i32) {
        let s = Solid::new(v3(x - r, top - m(1), z - r), v3(x + r, top, z + r));
        self.solids.push(s);
        self.k().drums.push(s);
        let cap = self.c("drum_cap");
        let spots = self.c("drum_spots");
        let stem = self.c("drum_spots");
        self.deco(Mesh::Cyl, v3(x, top - m(2), z), v3(r, m(2), r), stem);
        self.shape(
            Mesh::Blob,
            boxed(
                v3(x, top - dm(5), z),
                v3(r * 2 + dm(4), m(1) + dm(4), r * 2 + dm(4)),
            ),
            cap,
            0,
            true,
        );
        for k in 0..5 {
            let a = fx::TURN / 5 * k;
            let at = v3(
                x + fx::mul(fx::cos(a), r / 2),
                top + dm(1),
                z + fx::mul(fx::sin(a), r / 2),
            );
            self.deco(Mesh::Sphere, at, v3(dm(3), dm(1), dm(3)), spots);
        }
    }

    /// Where a strand's reward waits.
    pub(super) fn strand_reward(&mut self, x: i32, y: i32, z: i32) {
        self.k().strands.push(v3(x, y, z));
        let brass = self.c("brass");
        self.halo(
            Mesh::Sphere,
            boxed(v3(x, y + m(1), z), v3(m(3), m(3), m(3))),
            brass,
            ONE / 4,
        );
    }

    /// The way on: a gate at the far end, open from the start.
    pub(super) fn way_on(&mut self, x: i32, y: i32, z: i32) {
        {
            let k = self.k();
            k.finish = v3(x, y, z);
            k.finish_yaw = fx::HALF;
        }
        let wood = self.c("river_wood_dark");
        let brass = self.c("brass");
        for dx in [-m(3), m(3)] {
            self.deco(Mesh::Cube, v3(x + dx, y + m(3), z), v3(dm(6), m(6), dm(6)), wood);
        }
        self.deco(Mesh::Cube, v3(x, y + m(6), z), v3(m(7), dm(6), dm(6)), wood);
        self.glow(
            Mesh::Cube,
            v3(x, y + dm(55), z + dm(4)),
            v3(m(3), dm(4), dm(1)),
            brass,
        );
    }

    /// Reeds, boulders and moss along a rock edge, for the eye only.
    pub(super) fn river_dress(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, top: i32, n: i32) {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        let reed = self.c("reed");
        let moss = self.c("moss");
        let stone = self.c("sandstone_dark");
        for _ in 0..n {
            let x = x0 + dm(self.rng.range(0, ((x1 - x0) / (ONE / 10)).max(1)));
            let z = z0 + dm(self.rng.range(0, ((z1 - z0) / (ONE / 10)).max(1)));
            match self.rng.range(0, 2) {
                0 => {
                    let h = self.metres(1, 2);
                    self.deco(Mesh::Cone, v3(x, top + h / 2, z), v3(dm(3), h, dm(3)), reed);
                }
                1 => self.deco(Mesh::Blob, v3(x, top, z), v3(dm(14), dm(5), dm(14)), moss),
                _ => self.deco(Mesh::Octa, v3(x, top + dm(3), z), v3(dm(8), dm(6), dm(8)), stone),
            }
        }
    }

    /// A rope or chain between two points, drawn only.
    pub(super) fn rope_line(&mut self, a: V3, b: V3) {
        let rope = self.c("ascent_rope");
        if let Some(af) = segment(a, b, dm(1)) {
            self.shape(Mesh::Cyl, af, rope, 0, false);
        }
    }
}
