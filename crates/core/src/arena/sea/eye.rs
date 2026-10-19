//! The Eye: Dagon's arena. The whirlpool is a great cone of water round
//! you, its walls winding down and in to the eye, where Dagon rises. The
//! fleet's wreckage falls in down four arms of the cone: each piece (a
//! deck, a hull's side, a hatch heaped with crates, a mast lying on its
//! raft of planks) comes over the rim at an arm's head, winds down the wall faster as
//! the cone narrows, and near the eye sinks out from under whoever stands
//! on it; another comes over the rim. So you keep moving: down an arm with
//! the wreckage, a hop up it against the fall, across the cone to another
//! arm a turn below, the hook to a higher piece or to a mast on the rim. On the rim stand
//! four broken hulls at the arms' heads, each with its mooring post, and
//! between them four rocks; the tentacles rise there, over the cone. Fall
//! into the eye and you come back at your post.
//!
//! Nothing leads on from here: when Dagon falls the whirlpool drains, and
//! you sink.

use super::super::{boxed, dm, m, B};
use crate::balance::{EYE_FALL_TICKS, EYE_PIECES, EYE_SINK_TICKS};
use crate::course::{Look, Mover, Spiral, Step, Whirl};
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine, V3};
use crate::level::{Dyn, Solid};
use crate::mesh::Mesh;
use crate::render::mix;

/// the eye, the whirlpool's middle; its water stands at the course's
const EYE: V3 = v3(0, 0, -30 * ONE);
/// the cone's wall, ring by ring from the eye's throat out and up (how far
/// out, how high): steep down to the throat a metre under the wreckage's
/// track, a ledge of slack water at the wreckage's rim where the hulls and
/// rocks stand, and the walls of water towering up round it all to the
/// sea, which stands at the last ring's height
const WALL: [(i32, i32); 15] = [
    (9 * ONE, 4 * ONE),
    (12 * ONE, 6 * ONE),
    (16 * ONE, 8 * ONE),
    (20 * ONE, 10 * ONE),
    (24 * ONE, 13 * ONE),
    (28 * ONE, 15 * ONE),
    (32 * ONE, 17 * ONE),
    (36 * ONE, 20 * ONE),
    (40 * ONE, 22 * ONE),
    (46 * ONE, 23 * ONE),
    (53 * ONE, 23 * ONE),
    (61 * ONE, 29 * ONE),
    (70 * ONE, 37 * ONE),
    (78 * ONE, 44 * ONE),
    (86 * ONE, 49 * ONE),
];
/// a falling piece's track: the middle of its top from here, over the
/// rim, down to here by the eye
const TRACK_FROM: (i32, i32) = (40 * ONE, 23 * ONE);
const TRACK_TO: (i32, i32) = (12 * ONE, 7 * ONE);
/// how far round an arm winds evenly, and how much further it is whirled
/// as it narrows (so the pieces down it stay about as far apart)
const SWEEP: i32 = deg(120);
const WHIRL: i32 = deg(140);
/// the cone's arms, the first due south of the eye
const ARMS: i32 = 4;
/// a sinking piece drops this much further before it comes back
const SINK: i32 = 8 * ONE;
/// the spout of black water Dagon rides in the eye, this high
const SPOUT: i32 = 10 * ONE;
/// the pieces down an arm, across and along, in turn (drawn by their
/// shape: a deck, a hull's side, a hatch, a mast on its raft), and deep
const PIECES: [(i32, i32); 4] = [
    (6 * ONE, 6 * ONE),
    (ONE * 13 / 2, ONE * 11 / 2),
    (ONE * 11 / 2, ONE * 13 / 2),
    (7 * ONE, ONE * 9 / 2),
];
const PIECE_DEEP: i32 = ONE * 12 / 10;
/// the rim's hulls (at the arms' heads) and rocks (between): this far out,
/// their tops this high, this wide
const PERCH: (i32, i32) = (48 * ONE, 24 * ONE);
const PERCH_SIZE: i32 = 6 * ONE;

/// A point `a` (a share of a turn) round the eye from due south, `r` out,
/// at `y`.
fn about(a: i32, r: i32, y: i32) -> V3 {
    v3(EYE.x + fx::mul(fx::sin(a), r), y, EYE.z + fx::mul(fx::cos(a), r))
}

/// A triangle `p0 p1 p2` drawn as a wedge a centimetre thick, its face
/// looking toward `toward`: a wedge's sides are right triangles, and any
/// triangle is a right triangle's image under some affine map.
fn triangle(p0: V3, p1: V3, p2: V3, toward: V3) -> Affine {
    // the wedge's +x side runs (y, z) = (−½, ½), (−½, −½), (½, ½)
    let (p1, p2) = if p1.sub(p0).cross(p2.sub(p0)).dot(toward) < 0 {
        (p2, p1)
    } else {
        (p1, p2)
    };
    let c1 = p2.sub(p0);
    let c2 = p0.sub(p1);
    let face = c1.cross(c2).norm();
    Affine {
        c0: face.scale(ONE / 100),
        c1,
        c2,
        t: p1.add(p2).scale(ONE / 2),
    }
}

/// The `j`th piece down arm `arm`: it is `j` pieces' spacing along its fall
/// at tick 0.
fn track(arm: i32, j: i32) -> Spiral {
    Spiral {
        eye: EYE,
        ticks: EYE_FALL_TICKS,
        start: j * EYE_FALL_TICKS / EYE_PIECES,
        live: EYE_FALL_TICKS - EYE_SINK_TICKS,
        from: TRACK_FROM,
        to: TRACK_TO,
        bearing: fx::TURN / ARMS * arm,
        sweep: SWEEP,
        whirl: WHIRL,
        sink: SINK,
    }
}

/// The piece down arm `arm` that is between `along` and one spacing
/// further along its fall at `tick`.
#[cfg(test)]
fn piece_at(arm: i32, tick: i32, along: i32) -> Spiral {
    let gap = EYE_FALL_TICKS / EYE_PIECES;
    (0..EYE_PIECES)
        .map(|j| track(arm, j))
        .find(|sp| {
            let n = sp.along(tick);
            n >= along && n < along + gap
        })
        .unwrap_or_else(|| track(arm, 0))
}

/// The pilot's hops (the line the tests drive): off the start's hull onto
/// the first arm's head piece as it comes over the rim, down three more
/// pieces, and up one again against the fall: each jump made at this tick,
/// to the piece this many pieces down the arm from the last (the first:
/// the one just over the rim), aimed where it will be as the jump comes
/// down, this many ticks later; and the tick the jump will have come down.
const HOPS: [(i32, i32, i32); 5] = [
    (150, 0, 182),
    (250, 1, 301),
    (350, 1, 401),
    (450, 1, 501),
    (560, -1, 591),
];
const FLIGHT: i32 = 36;

impl B<'_> {
    pub(super) fn eye(&mut self) {
        // depth 0: the eye's water takes you at once
        {
            let k = self.k();
            k.depth = 0;
            k.whirl = Some(Whirl {
                eye: EYE,
                wall: WALL.to_vec(),
            });
        }
        // the hulls on the rim at the arms' heads, the south one first: you
        // come in on it; rocks between
        for arm in 0..ARMS {
            let a = fx::TURN / ARMS * arm;
            let at = about(a, PERCH.0, PERCH.1);
            self.perch(at, a);
            // its post by its inner edge, where you look down into the cone
            let post = about(a, PERCH.0 - PERCH_SIZE / 4, PERCH.1);
            self.mooring(post.x, post.y, post.z);
            let q = PERCH_SIZE / 4;
            for (dx, dz) in [(q, q), (-q, -q)] {
                self.k().spawns.push(v3(at.x + dx, at.y, at.z + dz));
            }
            let rock = about(a + fx::TURN / ARMS / 2, PERCH.0, PERCH.1);
            self.rim_rock(rock);
            for (dx, dz) in [(q, -q), (-q, q)] {
                self.k().spawns.push(v3(rock.x + dx, rock.y, rock.z + dz));
            }
        }
        // you come in looking down into the cone, at the eye
        let start = self.k().posts[0];
        self.start_yaw = crate::geom::yaw_pitch_of(v3(EYE.x - start.x, 0, EYE.z - start.z)).0;

        // the wreckage falling in down each arm
        for arm in 0..ARMS {
            for j in 0..EYE_PIECES {
                let sp = track(arm, j);
                let (w, d) = PIECES[(j % 4) as usize];
                self.falling(sp, w, d);
            }
        }

        // the line: off the start's hull onto the first arm, down it from
        // piece to piece, and up one again against the fall
        let lip = about(0, PERCH.0 - PERCH_SIZE / 2 + dm(6), PERCH.1);
        self.line(Step::To(lip));
        let gap = EYE_FALL_TICKS / EYE_PIECES;
        let (mut j, mut ready) = (0, 12);
        for (n, &(tick, down, landed)) in HOPS.iter().enumerate() {
            let land = tick + FLIGHT;
            j = if n == 0 {
                (0..EYE_PIECES)
                    .find(|&j| track(0, j).along(land) < gap)
                    .unwrap_or(0)
            } else {
                (j + down).rem_euclid(EYE_PIECES)
            };
            if tick > ready {
                self.line(Step::Wait(tick - ready));
            }
            let to = track(0, j).place(land);
            // off the rim a jump; across the arm's wider gaps a Bound Over
            self.line(if n == 0 { Step::Jump(to) } else { Step::Bound(to) });
            ready = landed;
        }

        // Dagon comes up in the eye on a spout of black water, its body
        // level with the wreckage's lower turns; nothing leads on
        let spout = EYE.add(v3(0, SPOUT, 0));
        self.solids.push(Solid::new(
            v3(spout.x - m(1), -m(2), spout.z - m(1)),
            v3(spout.x + m(1), spout.y, spout.z + m(1)),
        ));
        let gloom = self.c("storm_cloud");
        let foam = self.c("river_foam");
        self.deco(
            Mesh::Cyl,
            spout.sub(v3(0, SPOUT / 2, 0)),
            v3(m(4), SPOUT, m(4)),
            gloom,
        );
        for k in 0..3 {
            let at = v3(spout.x, SPOUT * (k + 1) / 4, spout.z);
            let r = m(5) - m(1) * k;
            self.shape(
                Mesh::Ring,
                Affine::translate(at)
                    .then(&Affine::rot_y(deg(40 * k)))
                    .then(&Affine::scale(v3(r, dm(4), r))),
                foam,
                ONE / 4,
                false,
            );
        }
        // its body five metres over the spout, held there by it
        self.k().boss_at = Some(spout.add(v3(0, m(5), 0)));
        self.k().finish = v3(0, -m(60), 0);
        self.eye_scene();
    }

    /// A piece of wreckage falling down the cone along `sp`, `w` across and
    /// `d` along.
    fn falling(&mut self, sp: Spiral, w: i32, d: i32) {
        let top = sp.place(0);
        let piece = Solid::new(
            v3(top.x - w / 2, top.y - PIECE_DEEP, top.z - d / 2),
            v3(top.x + w / 2, top.y, top.z + d / 2),
        );
        self.mover(piece, sp, Look::Wreck);
    }

    fn mover(&mut self, base: Solid, sp: Spiral, look: Look) {
        self.dynamic.push(Dyn {
            solid: base,
            on: sp.on(0),
            one_way: false,
        });
        let dynamic = self.dynamic.len() - 1;
        let mut mv = Mover::breathing(dynamic, base, 0, 1, 0);
        mv.spiral = Some(sp);
        mv.round = EYE;
        mv.look = look;
        self.k().movers.push(mv);
    }

    /// A broken hull caught on the rim, its deck's middle at `at`, facing
    /// the eye from bearing `a`: its mast to your left as you face the eye,
    /// clear of the camera over your right shoulder.
    fn perch(&mut self, at: V3, a: i32) {
        let h = PERCH_SIZE / 2;
        self.solids.push(Solid::new(
            v3(at.x - h, at.y - m(4), at.z - h),
            v3(at.x + h, at.y, at.z + h),
        ));
        let hull = self.c("hull");
        let dark = self.c("hull_dark");
        let sail = self.c("sail");
        let lamp = self.c("biolume");
        self.deco(
            Mesh::Cube,
            v3(at.x, at.y - dm(15), at.z),
            v3(PERCH_SIZE, m(3), PERCH_SIZE),
            hull,
        );
        self.deco(
            Mesh::Blob,
            v3(at.x, at.y - m(4), at.z),
            v3(PERCH_SIZE + m(2), m(6), PERCH_SIZE + m(2)),
            dark,
        );
        let foot = about(a, PERCH.0 - PERCH_SIZE / 4, at.y).add(about(a - fx::QUARTER, dm(26), 0).sub(EYE));
        let top = foot.add(v3(0, m(11), 0));
        self.solids.push(Solid::new(
            v3(foot.x - dm(3), foot.y, foot.z - dm(3)),
            v3(foot.x + dm(3), top.y, foot.z + dm(3)),
        ));
        self.rod(foot, top, dm(6), dark, true);
        let yard = about(a + fx::QUARTER, m(3), 0).sub(EYE);
        let y = top.add(v3(0, -m(2), 0));
        self.rod(y.sub(yard), y.add(yard), dm(3), dark, true);
        // hung a little off the mast: its edge in the mast's side flickers
        let rag = about(a + fx::QUARTER, dm(5) + ONE / 25, 0).sub(EYE);
        self.deco(
            Mesh::Cube,
            y.add(rag).add(v3(0, -dm(15), 0)),
            v3(dm(16), m(3), dm(16)),
            sail,
        );
        self.halo(Mesh::Sphere, boxed(top, v3(dm(5), dm(5), dm(5))), lamp, ONE / 2);
    }

    /// A rock jutting from the rim, its top at `at`.
    fn rim_rock(&mut self, at: V3) {
        let s = PERCH_SIZE * 5 / 6;
        self.solids.push(Solid::new(
            v3(at.x - s / 2, at.y - m(4), at.z - s / 2),
            v3(at.x + s / 2, at.y, at.z + s / 2),
        ));
        let rock = self.c("ice_rock_dark");
        let dark = self.c("hull_dark");
        self.deco(Mesh::Cube, v3(at.x, at.y - m(2), at.z), v3(s, m(4), s), rock);
        self.deco(
            Mesh::Blob,
            v3(at.x, at.y - m(3), at.z),
            v3(s + m(2), m(5), s + m(2)),
            dark,
        );
    }

    /// The whirlpool: the sea round the rim, the cone of water winding down
    /// and in from it (bands of water turned a little further each band
    /// down, foam streaming down its arms), the throat black and whirling,
    /// the deep's lights in it.
    fn eye_scene(&mut self) {
        let sea = self.c("ocean");
        let deep = self.c("ocean_deep");
        let navy = self.c("pirate_navy");
        let gloom = self.c("storm_cloud");
        let foam = self.c("river_foam");
        let teal = self.c("dagon_teal");
        let light = self.c("biolume");
        // the sea round the whirlpool, level with its top, in four strips
        // round the cone's square
        let sea_at = WALL[WALL.len() - 1];
        let (hole, far, y) = (sea_at.0, m(160), sea_at.1);
        for (x0, z0, x1, z1) in [
            (-far, hole, far, far),
            (-far, -far, far, -hole),
            (-far, -hole, -hole, hole),
            (hole, -hole, far, hole),
        ] {
            let (cx, cz) = (EYE.x + (x0 + x1) / 2, EYE.z + (z0 + z1) / 2);
            self.shape(
                Mesh::Ground,
                boxed(v3(cx, y, cz), v3(x1 - x0, ONE, z1 - z0)),
                sea,
                0,
                false,
            );
        }
        // the cone's wall, darker the deeper, in bands turned a little
        // further each one down so its stripes wind in; foam down its arms;
        // and the sea's flat from its top out past the square's corners
        let mut rings = WALL.to_vec();
        rings.push((hole * 3 / 2, y - dm(1) / 2));
        const ROUND: i32 = 24;
        let bands = rings.len() as i32 - 1;
        for band in 0..bands {
            let (inner, outer) = (rings[band as usize], rings[band as usize + 1]);
            let twist = deg(11) * band;
            let lit = mix(gloom, sea, ONE * band.min(bands - 2) / (bands - 2));
            let shades = [lit, mix(lit, navy, ONE / 3), lit, mix(lit, deep, ONE / 2)];
            for j in 0..ROUND {
                let a = fx::TURN * j / ROUND + twist;
                let rgb = if band == bands - 1 {
                    sea
                } else {
                    shades[(j % 4) as usize]
                };
                self.wall_tile(a, inner, outer, fx::TURN / ROUND, rgb);
            }
            if band < bands - 1 {
                // the tiles are chords: in as far as a chord's middle comes,
                // so the foam lies over the water and not in it
                let k = fx::cos(fx::TURN / ROUND / 2);
                let (fi, fo) = ((fx::mul(inner.0, k), inner.1), (fx::mul(outer.0, k), outer.1));
                for arm in 0..ARMS {
                    let a = fx::TURN / ARMS * arm + twist * 2 + deg(7);
                    self.wall_panel(a, fi, fo, fx::TURN / ROUND / 3, foam, ONE / 3, dm(1));
                }
            }
        }
        // the throat: a black whirl down from the wall's foot to the eye's
        // water, and the water there dark
        self.shape(
            Mesh::Ring,
            Affine::translate(v3(EYE.x, WALL[0].1 / 2, EYE.z)).then(&Affine::scale(v3(
                WALL[0].0 * 2,
                WALL[0].1 + dm(4),
                WALL[0].0 * 2,
            ))),
            navy,
            0,
            false,
        );
        self.shape(
            Mesh::Cyl,
            Affine::translate(v3(EYE.x, -dm(8), EYE.z)).then(&Affine::scale(v3(
                WALL[0].0 * 2,
                m(1),
                WALL[0].0 * 2,
            ))),
            gloom,
            0,
            false,
        );
        for k in 0..5 {
            let r = WALL[0].0 - m(1) - m(1) * k;
            self.shape(
                Mesh::Ring,
                Affine::translate(v3(EYE.x, dm(1) + dm(1) * k, EYE.z))
                    .then(&Affine::rot_y(deg(31 * k)))
                    .then(&Affine::scale(v3(r * 2, dm(3), r * 2))),
                if k % 2 == 0 { foam } else { deep },
                if k % 2 == 0 { ONE / 4 } else { 0 },
                false,
            );
        }
        // the deep's lights round the eye, and the teal of what waits
        for k in 0..10 {
            let a = deg(37 * k);
            let at = about(a, m(4) + dm(4) * k, dm(5) + dm(3) * k);
            self.halo(Mesh::Sphere, boxed(at, v3(dm(6), dm(6), dm(6))), light, ONE / 2);
        }
        self.deco(Mesh::Blob, v3(EYE.x, -m(3), EYE.z), v3(m(14), m(3), m(14)), teal);
        // wreckage drifting on the sea round the rim, drawn only
        let dark = self.c("hull_dark");
        let hull = self.c("hull");
        for _ in 0..30 {
            let a = deg(self.rng.range(0, 359));
            let r = self.metres(90, 150);
            let at = about(a, r, y);
            if self.rng.chance(50) {
                let l = self.metres(4, 12);
                self.deco(Mesh::Blob, at, v3(l / 3, dm(8), l), dark);
            } else {
                let s = self.metres(1, 2);
                self.deco(Mesh::Cube, at, v3(s, s, s), hull);
            }
        }
    }

    /// One tile of the cone's wall at bearing `a`, `wide` of a turn, from
    /// the ring `inner` to the ring `outer` (out, up): the four corners
    /// exactly, as two triangles, so a tile meets its neighbours edge to
    /// edge. Panels laid a little over each other to close the gaps lie in
    /// one plane where they overlap (the flat band at the rim wholly), and
    /// which shows there flickers as the camera moves.
    fn wall_tile(&mut self, a: i32, inner: (i32, i32), outer: (i32, i32), wide: i32, rgb: [i32; 3]) {
        let (l, r) = (a - wide / 2, a + wide / 2);
        let (il, ir) = (about(l, inner.0, inner.1), about(r, inner.0, inner.1));
        let (ol, or) = (about(l, outer.0, outer.1), about(r, outer.0, outer.1));
        // the way the panels of old faced: into the cone
        let (s, c) = (fx::sin(a), fx::cos(a));
        let slant = v3(
            fx::mul(s, outer.0 - inner.0),
            outer.1 - inner.1,
            fx::mul(c, outer.0 - inner.0),
        );
        let into = slant.norm().cross(v3(c, 0, -s)).norm();
        for (p0, p1, p2) in [(il, ir, or), (il, or, ol)] {
            self.shape(Mesh::Wedge, triangle(p0, p1, p2, into), rgb, 0, false);
        }
    }

    /// One panel of the cone's wall at bearing `a`, `wide` of a turn, from
    /// the ring `inner` to the ring `outer` (out, up), raised `lift` off it.
    #[allow(clippy::too_many_arguments)]
    fn wall_panel(
        &mut self,
        a: i32,
        inner: (i32, i32),
        outer: (i32, i32),
        wide: i32,
        rgb: [i32; 3],
        glow: i32,
        lift: i32,
    ) {
        let (s, c) = (fx::sin(a), fx::cos(a));
        let across = v3(c, 0, -s);
        let slant = v3(
            fx::mul(s, outer.0 - inner.0),
            outer.1 - inner.1,
            fx::mul(c, outer.0 - inner.0),
        );
        let normal = slant.norm().cross(across).norm();
        let mid_r = (inner.0 + outer.0) / 2;
        // the arc at the middle, and a little over so the panels meet
        let width = fx::mul(mid_r, fx::mul(fx::ratio(44, 7), wide)) * 11 / 10;
        let mid = about(a, mid_r, (inner.1 + outer.1) / 2).add(normal.scale(lift));
        self.shape(
            Mesh::Panel,
            Affine {
                c0: across.scale(width),
                c1: normal,
                c2: slant,
                t: mid,
            },
            rgb,
            glow,
            false,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;

    #[test]
    fn the_eye_is_a_cone_its_wreckage_at_many_heights_falling_down_and_in_toward_the_eye() {
        let c = content();
        let a = crate::arena::build_at("far_shore", "boss", 1, &[], &c.pal, 0);
        let k = a.course.as_ref().expect("a course");
        let pieces: Vec<&Mover> = k.movers.iter().filter(|mv| mv.look == Look::Wreck).collect();
        assert!(pieces.len() >= 40, "{} pieces", pieces.len());
        // at any tick, the pieces bearing weight stand at many heights,
        // lower the nearer the eye
        for tick in [0, 777, 1500, 2900] {
            let mut tops: Vec<(i32, i32)> = pieces
                .iter()
                .filter(|mv| mv.on(tick))
                .map(|mv| {
                    let p = mv.spiral.unwrap().place(tick);
                    (p.sub(EYE).len_xz(), p.y)
                })
                .collect();
            tops.sort();
            let low = tops.iter().map(|t| t.1).min().unwrap();
            let high = tops.iter().map(|t| t.1).max().unwrap();
            assert!(high - low > m(12), "tick {tick}: tops from {low} to {high}");
            let storeys: std::collections::BTreeSet<i32> = tops.iter().map(|t| t.1 / (2 * ONE)).collect();
            assert!(storeys.len() >= 6, "tick {tick}: {} storeys", storeys.len());
            for w in tops.windows(2) {
                assert!(
                    w[0].1 <= w[1].1 + ONE / 8,
                    "tick {tick}: higher nearer the eye: {w:?}"
                );
            }
        }
        // and each falls: lower and nearer the eye as it goes, till it sinks
        for mv in &pieces {
            let sp = mv.spiral.unwrap();
            let t0 = sp.ticks - sp.along(0);
            let (mut r, mut y) = (i32::MAX, i32::MAX);
            for t in (t0..t0 + sp.live).step_by(60) {
                let p = sp.place(t);
                assert!(p.y < y && p.sub(EYE).len_xz() < r, "rising or out at {t}: {p:?}");
                (r, y) = (p.sub(EYE).len_xz(), p.y);
            }
            assert!(y < m(9) && r < m(15), "it falls to the eye: {r} out, {y} up");
        }
    }

    #[test]
    fn dagon_rises_in_the_eye_in_sight_of_the_wreckage_and_the_tentacles_stand_on_the_rim() {
        let c = content();
        let a = crate::arena::build_at("far_shore", "boss", 1, &[], &c.pal, 0);
        let k = a.course.as_ref().expect("a course");
        let at = k.boss_at.expect("Dagon's place");
        assert!(at.sub(EYE).len_xz() < ONE, "in the eye: {at:?}");
        assert!(k.spawns.len() >= 8, "{} spawns", k.spawns.len());
        for p in k.spawns.iter().chain(std::iter::once(&a.start)) {
            let floor = a.level.floor_below(p.x, p.y, p.z);
            assert!(
                floor.is_some_and(|f| p.y - f >= 0 && p.y - f <= ONE / 2),
                "{p:?} stands on {floor:?}"
            );
            assert!(p.sub(EYE).len_xz() > TRACK_FROM.0 + m(4), "{p:?} on the rim");
        }
        assert_eq!(k.posts.len(), 4, "a post on each arm's head");
        // from where you stand on the wreckage, Dagon's body is in sight
        let mut level = a.level.clone();
        let body = at;
        let (mut seen, mut all) = (0, 0);
        for tick in [0, 400, 1100] {
            for mv in &k.movers {
                let s = mv.corner(tick);
                let d = &mut level.dynamic[mv.dynamic];
                d.solid = Solid::new(s, s.add(mv.base.size()));
                d.on = mv.on(tick);
            }
            for mv in k.movers.iter().filter(|mv| mv.look == Look::Wreck && mv.on(tick)) {
                // looking out over its edge: itself out of the way
                level.dynamic[mv.dynamic].on = false;
                let top = mv.spiral.unwrap().place(tick);
                let eye = top.add(v3(0, m(1) + dm(5), 0));
                let to = body.sub(eye);
                let clear = level
                    .ray(eye, to.norm(), to.len())
                    .is_none_or(|h| h.t > to.len() - m(6));
                level.dynamic[mv.dynamic].on = true;
                all += 1;
                seen += i32::from(clear);
            }
        }
        assert!(seen * 10 >= all * 9, "Dagon in sight from {seen} of {all} pieces");
    }

    /// The Eye's world with Dagon kept down: the cone and its wreckage.
    fn the_cone() -> (crate::content::Content, crate::world::World) {
        let (c, mut w) = crate::course::tests::course_world_in("far_shore", "boss");
        w.waves_left.clear();
        (c, w)
    }

    fn stand(c: &crate::content::Content, w: &mut crate::world::World) {
        let eye = w.player.chest();
        let fwd = crate::geom::facing(w.player.yaw, 0);
        w.tick(c, &crate::player::Controls::default(), eye, fwd);
    }

    /// The falling piece down arm `arm` that is `along` ticks or a little
    /// more into its fall now; you stood in the middle of its top.
    fn aboard(w: &mut crate::world::World, arm: i32, along: i32) -> Mover {
        let k = w.course.as_ref().unwrap();
        let sp = piece_at(arm, w.tick, along);
        let mv = k
            .movers
            .iter()
            .find(|mv| mv.spiral == Some(sp) && mv.look == Look::Wreck)
            .unwrap()
            .clone();
        let s = w.arena.level.dynamic[mv.dynamic].solid;
        w.player.pos = v3(s.centre().x, s.max.y, s.centre().z);
        w.player.prev_pos = w.player.pos;
        w.player.grounded = true;
        mv
    }

    #[test]
    fn standing_on_falling_wreckage_you_go_down_and_round_with_it() {
        let (c, mut w) = the_cone();
        let mv = aboard(&mut w, 2, 200);
        let from = w.player.pos;
        for _ in 0..60 * 10 {
            stand(&c, &mut w);
        }
        let p = w.player.pos;
        let s = w.arena.level.dynamic[mv.dynamic].solid;
        assert!(w.player.grounded, "on its feet at {p:?}");
        assert!(
            (p.y - s.max.y).abs() < ONE / 20 && p.sub(s.centre()).len_xz() < ONE / 2,
            "still in the middle of its top: {p:?} on {s:?}"
        );
        assert!(from.y - p.y > m(3), "carried down: {from:?} to {p:?}");
        assert!(
            p.sub(EYE).len_xz() + m(5) < from.sub(EYE).len_xz(),
            "carried in: {from:?} to {p:?}"
        );
        let turned = fx::atan2(p.x - EYE.x, p.z - EYE.z) - fx::atan2(from.x - EYE.x, from.z - EYE.z);
        assert!(turned.rem_euclid(fx::TURN) > deg(30), "carried round: {turned}");
    }

    #[test]
    fn a_piece_sinks_out_from_under_you_and_comes_back_over_the_rim_empty() {
        let (c, mut w) = the_cone();
        let mv = aboard(&mut w, 2, EYE_FALL_TICKS - EYE_SINK_TICKS - 120);
        let sp = mv.spiral.unwrap();
        let lowest = sp.place(w.tick + 120);
        let mut fell = false;
        for _ in 0..EYE_SINK_TICKS + 300 {
            stand(&c, &mut w);
            let p = w.player.pos;
            fell |= w.notes.iter().any(|n| n.sound == "drown") || p.y < lowest.y - m(2);
            let s = w.arena.level.dynamic[mv.dynamic].solid;
            if sp.along(w.tick) >= sp.live + 10 {
                // sinking: you are soon no longer stood on it
                let on_it = w.player.grounded
                    && (p.y - s.max.y).abs() < ONE / 20
                    && p.sub(s.centre()).len_xz() < s.size().x.max(s.size().z) / 2;
                assert!(!on_it, "carried down under with it at {p:?}");
            }
            if sp.along(w.tick) < 300 {
                assert!(
                    p.sub(s.centre()).len_xz() > s.size().x.max(s.size().z),
                    "carried back up to the rim with it: {p:?} by {s:?}"
                );
            }
        }
        assert!(fell, "it sank from under you; you fell in");
    }

    #[test]
    fn you_can_hop_down_the_falling_wreckage_from_the_rim_and_up_it_again() {
        use crate::course::Pilot;
        let (c, mut w) = the_cone();
        let line = w.course.as_ref().unwrap().line.clone();
        let mut pilot = Pilot::default();
        let mut stood: Vec<(usize, i32, i32)> = Vec::new();
        for tick in 0..60 * 30 {
            let Some(d) = pilot.drive(&line, &w.player) else {
                break;
            };
            w.player.yaw = d.yaw;
            let eye = w.player.chest();
            w.tick(&c, &d.controls, eye, d.aim);
            assert!(
                !w.notes.iter().any(|n| n.sound == "drown"),
                "fell in on step {} ({:?}) at tick {tick}, from {:?}; stood on {stood:?}",
                pilot.step,
                line[pilot.step.min(line.len() - 1)],
                w.player.prev_pos
            );
            let p = w.player.pos;
            let k = w.course.as_ref().unwrap();
            if w.player.grounded {
                let under = k.movers.iter().position(|mv| {
                    let s = w.arena.level.dynamic[mv.dynamic].solid;
                    mv.look == Look::Wreck
                        && (p.y - s.max.y).abs() < ONE / 20
                        && p.x >= s.min.x - dm(4)
                        && p.x <= s.max.x + dm(4)
                        && p.z >= s.min.z - dm(4)
                        && p.z <= s.max.z + dm(4)
                });
                if let Some(mv) = under.filter(|&mv| stood.last().is_none_or(|l| l.0 != mv)) {
                    stood.push((mv, p.y, w.tick));
                }
            }
            assert!(
                pilot.t < 60 * 6,
                "stuck on step {} ({:?}) at {p:?}, tick {tick}",
                pilot.step,
                line[pilot.step]
            );
        }
        assert_eq!(pilot.step, line.len(), "the line done; stood on {stood:?}");
        let heights: std::collections::BTreeSet<i32> = stood.iter().map(|s| s.1 / (ONE / 2)).collect();
        assert!(stood.len() >= 5 && heights.len() >= 4, "stood on {stood:?}");
        // and up one again: the last stood on higher than the one before
        let n = stood.len();
        assert!(stood[n - 1].1 > stood[n - 2].1, "up again: {stood:?}");
    }

    #[test]
    fn you_come_in_facing_the_eye() {
        let c = crate::content::tests::content();
        let a = crate::arena::build("far_shore", "boss", 3, &[], &c.pal);
        let look = crate::geom::facing(a.start_yaw, 0);
        let to = v3(EYE.x - a.start.x, 0, EYE.z - a.start.z).norm();
        assert!(look.dot(to) > ONE * 9 / 10, "facing {look:?}, the eye {to:?}");
    }
}
