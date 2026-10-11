//! Zone five, the Idiot God ("Outside"): floors in the void inside the god,
//! after Ultrakill's Fraud (rooms that are trick art), Manifold Garden,
//! Antichamber, Escher's Relativity, Hyperbolica and Half-Life's Xen. Every
//! arena wraps: off any edge you fall, and come down again from 64 m over
//! the same floor (`wrap`; the scene is drawn again above and below, so the
//! floor you will land on hangs overhead). Far off, the god's insides: walls
//! of flesh with eyes in them, tendrils, bubbles, swirls of light, its
//! mindless dancers piping on a ring, and its pipes. The square and its
//! gates are zone one's, so the route, the spawn grid and the reach tests
//! apply; between the gates there are gaps to fall through.
//!
//! - the Fold: islands joined by arms, and the Waiting City hanging upside
//!   down overhead
//! - the Penrose Stair: four flights round a well, each climbing; the top
//!   of the fourth is, through a seam, the foot of the first
//! - the Doors: free-standing gold frames, paired as seams, so a doorway
//!   leads to another across the room, turned
//! - the Lattice: Xen; rocks floating at several heights, updrafts and
//!   crystals
//! - the Orrery: rings of floor round a sun, spokes across the gaps
//!
//! and the Gullet (arrival: ribs and teeth), the Still Point (rest: a
//! stopped clock and the trumpet's case) and the Heart (the boss: a disc
//! ringed with bones and eye-stalks, the god over its north side).

use super::{boxed, dm, m, Seam, Side, Vent, B, COURT_RADIUS, GATE_GAP, HALF};
use crate::content::Rgb;
use crate::fx::{self, deg, ONE};
use crate::geom::{facing, v3, Affine, V3};
use crate::level::Solid;
use crate::mesh::Mesh;
use crate::render::{mix, shade};

/// How far the world repeats above and below.
const WRAP: i32 = fx::int(64);
/// A floor slab's depth under its top.
const SLAB: i32 = fx::int(2);
/// A step's rise, and the thickness of a floating step.
const RISE: i32 = ONE * 4 / 10;
const TREAD: i32 = ONE * 8 / 10;
/// The Penrose Stair: the well's half width, the lane's outer half width,
/// the height each flight climbs, and where the seam stands on the west
/// lane (z).
const WELL: i32 = fx::int(6);
const LANE: i32 = fx::int(11);
const FLIGHT: i32 = fx::int(3);
const SEAM_Z: i32 = fx::int(3);
/// A centimetre, raised a little off a floor so it does not flicker.
const fn cm1() -> i32 {
    ONE / 50
}

/// 2π, Q16.
const TWO_PI: i32 = 411_775;
/// An updraft's radius.
const VENT_RADIUS: i32 = ONE * 12 / 10;

#[allow(clippy::too_many_arguments)]
impl B<'_> {
    pub(super) fn god(&mut self, kind: &str, exits: &[Side]) {
        self.wrap = WRAP;
        if kind == "boss" {
            self.heart();
            self.repeat = self.shapes.len();
            self.dreams();
            self.insides(COURT_RADIUS);
            return;
        }
        let gaps = [
            true,
            exits.contains(&Side::North),
            exits.contains(&Side::East),
            exits.contains(&Side::West),
        ];
        match kind {
            "arrival" => self.gullet(),
            "rest" => self.still_point(),
            "stair" => self.penrose(),
            "doors" => self.doors(),
            "lattice" => self.lattice(),
            "orrery" => self.orrery(),
            _ => self.fold(),
        }
        self.gate_arches(gaps);
        self.repeat = self.shapes.len();
        self.dreams();
        self.insides(HALF);
    }

    // ---- pieces ---------------------------------------------------------

    /// A floor slab from `(x0, z0)` to `(x1, z1)`, its top at `top`, with a
    /// ragged underside of flesh.
    fn slab(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, top: i32, rgb: Rgb) {
        let (lx, hx) = (x0.min(x1), x0.max(x1));
        let (lz, hz) = (z0.min(z1), z0.max(z1));
        self.solids
            .push(Solid::new(v3(lx, top - SLAB, lz), v3(hx, top, hz)));
        let c = v3((lx + hx) / 2, top - SLAB / 2, (lz + hz) / 2);
        self.deco(Mesh::Cube, c, v3(hx - lx, SLAB, hz - lz), rgb);
        // the underside: a hanging lump, as if torn from somewhere
        let under = shade(self.c("god_flesh_dark"), 90);
        let (w, d) = (hx - lx, hz - lz);
        self.shape(
            Mesh::Cone,
            Affine::translate(v3(c.x, top - SLAB - w.min(d) / 4, c.z))
                .then(&Affine::rot_x(fx::HALF))
                .then(&Affine::scale(v3(w * 9 / 10, w.min(d) / 2, d * 9 / 10))),
            under,
            0,
            false,
        );
    }

    /// Steps from `(x, z)` along the axis `(dx, dz)` (one of them ±ONE),
    /// `len` long and `w` across, climbing from `from` to `to`. Each is a
    /// floating tread; the low ones stand on the floor below.
    fn climb(&mut self, x: i32, z: i32, dx: i32, dz: i32, len: i32, w: i32, from: i32, to: i32) {
        let n = ((to - from).abs() + RISE - 1) / RISE;
        let run = len / n.max(1);
        let stone = self.c("god_stone");
        let vein = self.c("god_vein");
        for k in 0..n {
            let top = from + (to - from) * (k + 1) / n;
            let along = run * k + run / 2;
            let cx = x + fx::mul(dx, along);
            let cz = z + fx::mul(dz, along);
            let (sw, sd) = if dx != 0 { (run, w) } else { (w, run) };
            let bottom = (top - TREAD).min(from);
            let bottom = if top - TREAD < from + RISE {
                bottom
            } else {
                top - TREAD
            };
            self.solids.push(Solid::new(
                v3(cx - sw / 2, bottom, cz - sd / 2),
                v3(cx + sw / 2, top, cz + sd / 2),
            ));
            let rgb = if k % 2 == 0 { stone } else { shade(stone, 85) };
            self.deco(
                Mesh::Cube,
                v3(cx, (top + bottom) / 2, cz),
                v3(sw, top - bottom, sd),
                rgb,
            );
            if k % 3 == 0 {
                // a vein along the riser
                self.deco(
                    Mesh::Cube,
                    v3(cx, top - RISE / 2, cz),
                    v3(sw + dm(1), dm(1), sd + dm(1)),
                    vein,
                );
            }
        }
    }

    /// A solid post standing on `y`.
    fn post(&mut self, x: i32, y: i32, z: i32, w: i32, h: i32, rgb: Rgb) {
        self.block(x, y, z, w, h, w, rgb);
    }

    /// An arch across `(x, z)`: two posts `span` apart and a lintel at
    /// `h`, its span along x or along z. A place to stand under.
    fn arch(&mut self, x: i32, z: i32, along_x: bool, span: i32, h: i32, rgb: Rgb) {
        let (ax, az) = if along_x { (span / 2, 0) } else { (0, span / 2) };
        for s in [-1, 1] {
            self.post(x + s * ax, 0, z + s * az, dm(8), h, rgb);
        }
        let (lw, ld) = if along_x {
            (span + dm(8), dm(8))
        } else {
            (dm(8), span + dm(8))
        };
        self.solids.push(Solid::new(
            v3(x - lw / 2, h, z - ld / 2),
            v3(x + lw / 2, h + dm(6), z + ld / 2),
        ));
        self.deco(Mesh::Cube, v3(x, h + dm(3), z), v3(lw, dm(6), ld), rgb);
        // an eye in the keystone
        let eye = self.c("god_eye");
        self.deco(Mesh::Sphere, v3(x, h + dm(3), z), v3(dm(5), dm(5), dm(5)), eye);
    }

    /// A gold doorframe whose opening is a seam: posts, a lintel, and a
    /// faint film of light in the opening. Returns the seam.
    fn frame(&mut self, at: V3, yaw: i32, half: i32, height: i32, film: Rgb, to: usize) -> Seam {
        let gold = self.c("god_seam");
        let n = facing(yaw, 0);
        let across = v3(-n.z, 0, n.x);
        for s in [-1, 1] {
            let p = at.add(across.scale(s * (half + dm(3))));
            self.block(p.x, at.y, p.z, dm(5), height + dm(5), dm(5), gold);
        }
        let top = at.add(v3(0, height + dm(5), 0));
        let (lw, ld) = if across.x.abs() > ONE / 2 {
            (half * 2 + m(1), dm(5))
        } else {
            (dm(5), half * 2 + m(1))
        };
        self.solids.push(Solid::new(
            v3(top.x - lw / 2, top.y - dm(5), top.z - ld / 2),
            v3(top.x + lw / 2, top.y, top.z + ld / 2),
        ));
        self.deco(Mesh::Cube, top.sub(v3(0, dm(2), 0)), v3(lw, dm(5), ld), gold);
        let film_at = Affine::translate(at.add(v3(0, height / 2, 0)))
            .then(&Affine::rot_y(yaw))
            .then(&Affine::scale(v3(half * 2, height, dm(1))));
        self.halo(Mesh::Cube, film_at, film, ONE / 3);
        Seam {
            at,
            yaw,
            half,
            height,
            to,
        }
    }

    /// A pair of seams, each leading to the other.
    fn seam_pair(&mut self, a: (V3, i32), b: (V3, i32), half: i32, height: i32, film: Rgb) {
        let k = self.seams.len();
        let sa = self.frame(a.0, a.1, half, height, film, k + 1);
        let sb = self.frame(b.0, b.1, half, height, film, k);
        self.seams.push(sa);
        self.seams.push(sb);
    }

    /// An updraft from the floor at `(x, y, z)`: a ring of crystal and a
    /// column of glowing air.
    fn updraft(&mut self, x: i32, y: i32, z: i32) {
        let crystal = self.c("god_crystal");
        self.glow(
            Mesh::Cyl,
            v3(x, y + dm(1) / 2, z),
            v3(VENT_RADIUS * 2, dm(1), VENT_RADIUS * 2),
            crystal,
        );
        for k in 0..3 {
            let r = VENT_RADIUS * (6 - k) / 6;
            let ring = boxed(v3(x, y + m(1) + m(k * 2), z), v3(r * 2, dm(1) / 2, r * 2));
            self.shape(Mesh::Ring, ring, crystal, ONE * (3 - k) / 4, false);
        }
        self.vents.push(Vent {
            pos: v3(x, y, z),
            radius: VENT_RADIUS,
        });
    }

    /// A cluster of glowing crystals on `(x, y, z)`.
    fn crystals(&mut self, x: i32, y: i32, z: i32) {
        let crystal = self.c("god_crystal");
        let iris = self.c("god_iris");
        for k in 0..4 {
            let a = fx::TURN / 4 * k + deg(self.rng.range(0, 60));
            let r = dm(self.rng.range(2, 8));
            let h = dm(self.rng.range(8, 24));
            let p = v3(x + fx::mul(fx::cos(a), r), y + h / 2, z + fx::mul(fx::sin(a), r));
            let tilt = Affine::translate(p)
                .then(&Affine::rot_y(a))
                .then(&Affine::rot_x(deg(self.rng.range(-20, 20))))
                .then(&Affine::scale(v3(h / 3, h, h / 3)));
            let c = if k % 2 == 0 { crystal } else { iris };
            self.shape(Mesh::Octa, tilt, c, ONE * 7 / 10, true);
        }
    }

    /// A stalk with an eye on top, standing on `(x, y, z)`; only looked at.
    fn eye_stalk(&mut self, x: i32, y: i32, z: i32, h: i32) {
        let flesh = self.c("god_flesh_pale");
        let eye = self.c("god_eye");
        let iris = self.c("god_iris");
        let lean = v3(dm(self.rng.range(-10, 10)), 0, dm(self.rng.range(-10, 10)));
        let top = v3(x, y + h, z).add(lean);
        self.rod(v3(x, y, z), top, dm(3), flesh, true);
        let r = dm(6);
        self.deco(Mesh::Sphere, top, v3(r * 2, r * 2, r * 2), eye);
        // the iris looks in, toward the middle of the floor
        let look = v3(-top.x, 0, -top.z).norm().scale(r * 8 / 10);
        self.shape(Mesh::Sphere, boxed(top.add(look), v3(r, r, r)), iris, ONE, false);
    }

    /// Fallen cover: a tilted block of dream-stone, solid at its footprint.
    fn rubble(&mut self, x: i32, z: i32) {
        let w = self.metres(2, 3);
        let h = self.metres(1, 2);
        let d = self.metres(2, 3);
        let stone = self.c("god_stone_dark");
        self.solids.push(Solid::stand(x, 0, z, w, h, d));
        let a = Affine::translate(v3(x, h / 2, z))
            .then(&Affine::rot_y(deg(self.rng.range(0, 89))))
            .then(&Affine::scale(v3(w, h, d)));
        self.shape(Mesh::Cube, a, stone, 0, true);
    }

    /// A ring of floor tiles from radius `r0` to `r1` round `(0, 0)`, its
    /// top at `top`: solid in rows a metre deep, drawn as `segments`
    /// pieces round the circle.
    fn annulus(&mut self, r0: i32, r1: i32, top: i32, rgb: Rgb, segments: i32) {
        let cell = m(1);
        let mut z = -r1;
        while z < r1 {
            let zc = z + cell / 2;
            let w = |r: i32| -> i32 {
                let s = r as i64 * r as i64 - zc as i64 * zc as i64;
                if s <= 0 {
                    0
                } else {
                    fx::isqrt(s as u64) as i32
                }
            };
            let (xo, xi) = (w(r1), w(r0));
            if xo > 0 {
                let spans = if xi > 0 {
                    vec![(-xo, -xi), (xi, xo)]
                } else {
                    vec![(-xo, xo)]
                };
                for (a, b) in spans {
                    if b - a > dm(2) {
                        self.solids
                            .push(Solid::new(v3(a, top - SLAB, z), v3(b, top, z + cell)));
                    }
                }
            }
            z += cell;
        }
        if r0 <= 0 {
            self.deco(Mesh::Cyl, v3(0, top - SLAB / 2, 0), v3(r1 * 2, SLAB, r1 * 2), rgb);
            return;
        }
        let mid = (r0 + r1) / 2;
        // a segment's length round the ring: 2πr over the segments
        let arc = fx::mul(mid, TWO_PI) / segments.max(1);
        for k in 0..segments {
            let a = fx::TURN / segments * k;
            let at = v3(fx::mul(fx::sin(a), mid), top - SLAB / 2, fx::mul(fx::cos(a), mid));
            let seg = Affine::translate(at)
                .then(&Affine::rot_y(a))
                .then(&Affine::scale(v3(arc + dm(4), SLAB, r1 - r0)));
            let c = if k % 2 == 0 { rgb } else { shade(rgb, 88) };
            self.shape(Mesh::Cube, seg, c, 0, true);
        }
    }

    /// An arch of flesh over each gate, and a lip of floor past it.
    fn gate_arches(&mut self, gaps: [bool; 4]) {
        let flesh = self.c("god_flesh");
        let teeth = self.c("god_teeth");
        for (k, side) in [Side::South, Side::North, Side::East, Side::West]
            .iter()
            .enumerate()
        {
            if !gaps[k] {
                continue;
            }
            let (g, _) = side.place(HALF);
            let along = if g.x == 0 { v3(ONE, 0, 0) } else { v3(0, 0, ONE) };
            for s in [-1, 1] {
                let p = g.add(along.scale(s * (GATE_GAP / 2 + m(1))));
                self.block(p.x, 0, p.z, m(1) + dm(4), m(6), m(1) + dm(4), flesh);
                // teeth down the posts
                for t in 0..3 {
                    self.deco(
                        Mesh::Cone,
                        p.add(along.scale(-s * dm(9))).add(v3(0, m(2) * t + m(1), 0)),
                        v3(dm(4), dm(8), dm(4)),
                        teeth,
                    );
                }
            }
            let top = g.add(v3(0, m(6) + dm(5), 0));
            let size = if g.x == 0 {
                v3(GATE_GAP + m(4), m(1), m(1) + dm(4))
            } else {
                v3(m(1) + dm(4), m(1), GATE_GAP + m(4))
            };
            self.deco(Mesh::Blob, top, size, flesh);
        }
    }

    // ---- the god's insides ----------------------------------------------

    /// What drifts in the air round the floor, only looked at: impossible
    /// triangles, stairs that turn back on themselves, doorframes with
    /// nothing round them, a chessboard of tiles coming apart, and eyes
    /// set in the dark like windows.
    fn dreams(&mut self) {
        let gold = self.c("god_seam");
        let bone = self.c("god_bone");
        let stone = self.c("god_stone");
        let dark = self.c("god_stone_dark");
        let eye = self.c("god_eye");
        let iris = self.c("god_iris");
        // Penrose triangles: three bars that meet where they cannot
        for k in 0..5 {
            let a = fx::TURN / 5 * k + deg(self.rng.range(0, 40));
            let r = self.metres(40, 70);
            let c = v3(fx::mul(fx::sin(a), r), self.metres(8, 40), fx::mul(fx::cos(a), r));
            let size = self.metres(6, 12);
            let spin = deg(self.rng.range(0, 359));
            let tilt = deg(self.rng.range(-40, 40));
            let frame = Affine::translate(c)
                .then(&Affine::rot_y(spin))
                .then(&Affine::rot_x(tilt));
            let corners = [
                v3(0, size, 0),
                v3(size * 87 / 100, -size / 2, 0),
                v3(-size * 87 / 100, -size / 2, 0),
            ];
            for j in 0..3 {
                let (p, q) = (corners[j], corners[(j + 1) % 3]);
                // each bar runs on past its corner and out of the plane, so
                // the three never quite meet as they seem to
                let past = q
                    .sub(p)
                    .norm()
                    .scale(size / 5)
                    .add(v3(0, 0, size / 4 * (j as i32 - 1)));
                if let Some(af) = super::segment(frame.apply(p), frame.apply(q.add(past)), size / 6) {
                    let c = [gold, bone, stone][j];
                    self.shape(Mesh::Cube, af, c, 0, true);
                }
            }
        }
        // stairs in a loop, out in the dark
        for k in 0..3 {
            let a = fx::TURN / 3 * k + deg(50);
            let at = v3(
                fx::mul(fx::sin(a), m(48)),
                self.metres(-20, 30),
                fx::mul(fx::cos(a), m(48)),
            );
            let spin = deg(self.rng.range(0, 359));
            for side in 0..4 {
                for st in 0..6 {
                    let f = Affine::translate(at)
                        .then(&Affine::rot_y(spin + fx::QUARTER * side))
                        .then(&Affine::translate(v3(
                            m(5),
                            RISE * (st + side * 6),
                            -m(5) + dm(17) * st,
                        )))
                        .then(&Affine::scale(v3(m(2), RISE, dm(17))));
                    self.shape(Mesh::Cube, f, if st % 2 == 0 { stone } else { dark }, 0, true);
                }
            }
        }
        // doorframes standing in nothing
        for k in 0..6 {
            let a = fx::TURN / 6 * k + deg(15);
            let r = self.metres(36, 60);
            let at = v3(
                fx::mul(fx::sin(a), r),
                self.metres(-6, 26),
                fx::mul(fx::cos(a), r),
            );
            let f = Affine::translate(at)
                .then(&Affine::rot_y(a))
                .then(&Affine::rot_z(deg(self.rng.range(-30, 30))));
            for (p, size) in [
                (v3(-m(2), m(2), 0), v3(dm(5), m(4), dm(5))),
                (v3(m(2), m(2), 0), v3(dm(5), m(4), dm(5))),
                (v3(0, m(4), 0), v3(m(4) + dm(5), dm(5), dm(5))),
            ] {
                self.shape(Mesh::Cube, f.then(&boxed(p, size)), gold, 0, true);
            }
            self.halo(
                Mesh::Cube,
                f.then(&boxed(v3(0, m(2), 0), v3(m(4), m(4), dm(1)))),
                iris,
                ONE / 4,
            );
        }
        // a chessboard coming apart
        let cx = self.metres(-30, 30);
        for i in 0..6 {
            for j in 0..6 {
                if self.rng.chance(30) {
                    continue;
                }
                let lift = self.metres(0, 12);
                let at = v3(cx + m(3) * i - m(9), m(34) + lift, -m(50) + m(3) * j);
                let t = Affine::translate(at)
                    .then(&Affine::rot_y(deg(lift / ONE * 9)))
                    .then(&Affine::rot_x(deg(lift / ONE * 7)))
                    .then(&Affine::scale(v3(m(3), dm(3), m(3))));
                let c = if (i + j) % 2 == 0 {
                    bone
                } else {
                    self.c("eldritch_void")
                };
                self.shape(Mesh::Cube, t, c, 0, true);
            }
        }
        // eyes in the dark below the floor, looking up
        for _ in 0..12 {
            let at = v3(self.metres(-40, 40), -self.metres(14, 30), self.metres(-40, 40));
            let r = self.metres(2, 5);
            self.shape(
                Mesh::Sphere,
                boxed(at, v3(r * 2, r * 2, r * 2)),
                eye,
                ONE / 5,
                false,
            );
            self.shape(
                Mesh::Sphere,
                boxed(at.add(v3(0, r * 8 / 10, 0)), v3(r, r / 2, r)),
                iris,
                ONE,
                false,
            );
        }
    }

    /// Veins and lidded eyes across a floor from `(x0, z0)` to `(x1, z1)`
    /// at `top`: only looked at.
    fn floor_life(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, top: i32, n: i32) {
        let vein = self.c("god_vein");
        let eye = self.c("god_eye");
        let iris = self.c("god_iris");
        let lid = self.c("god_flesh_pale");
        for k in 0..n {
            let a = v3(
                self.rng.range(x0 / ONE, x1 / ONE) * ONE,
                top + cm1(),
                self.rng.range(z0 / ONE, z1 / ONE) * ONE,
            );
            if k % 4 == 0 {
                // a lidded eye in the floor, half open
                let r = self.metres(1, 2);
                self.deco(Mesh::Blob, a, v3(r * 2 + dm(4), dm(2), r + dm(4)), lid);
                self.shape(Mesh::Sphere, boxed(a, v3(r * 2, dm(2), r)), eye, ONE / 5, false);
                self.shape(
                    Mesh::Sphere,
                    boxed(a.add(v3(0, cm1(), 0)), v3(r * 2 / 3, dm(2), r * 2 / 3)),
                    iris,
                    ONE,
                    false,
                );
            } else {
                let b = v3(
                    (a.x + self.metres(-4, 4)).clamp(x0, x1),
                    a.y,
                    (a.z + self.metres(-4, 4)).clamp(z0, z1),
                );
                self.rod(a, b, dm(2), vein, false);
            }
        }
    }

    /// What is round every arena, far off: walls of flesh curving away,
    /// eyes in them, tendrils reaching in, bubbles, swirls of light, the
    /// dancers piping on their ring, and the god's pipes.
    fn insides(&mut self, half: i32) {
        let flesh = self.c("god_flesh");
        let dark = self.c("god_flesh_dark");
        let eldritch = self.c("eldritch_flesh");
        let pale = self.c("god_flesh_pale");
        let eye = self.c("god_eye");
        let iris = self.c("god_iris");
        let void = self.c("eldritch_void");
        // the walls: great lumps on every side, above and below
        for k in 0..22 {
            let a = fx::TURN / 22 * k + deg(self.rng.range(-6, 6));
            let r = half + self.metres(130, 210);
            let y = self.metres(-90, 120);
            let s = self.metres(60, 130);
            let p = v3(fx::mul(fx::sin(a), r), y, fx::mul(fx::cos(a), r));
            let c = [flesh, dark, eldritch][k as usize % 3];
            let tall = s * 3 / 4 + self.metres(0, 30);
            self.shape(Mesh::Sphere, boxed(p, v3(s, tall, s)), c, 0, false);
            if k % 2 == 0 {
                // an eye, opening in it, looking in
                let er = s / 4;
                let inward = p.neg().norm();
                let at = p.add(inward.scale(s / 2 - er / 3));
                self.shape(
                    Mesh::Sphere,
                    boxed(at, v3(er * 2, er * 2, er * 2)),
                    eye,
                    ONE / 4,
                    false,
                );
                let iat = at.add(inward.scale(er * 7 / 10));
                self.shape(Mesh::Sphere, boxed(iat, v3(er, er, er)), iris, ONE, false);
                self.shape(
                    Mesh::Sphere,
                    boxed(iat.add(inward.scale(er / 3)), v3(er / 2, er / 2, er / 2)),
                    void,
                    0,
                    false,
                );
            }
        }
        // tendrils reaching in from the walls, in curling chains
        for k in 0..10 {
            let a = fx::TURN / 10 * k + deg(self.rng.range(0, 30));
            let mut p = v3(
                fx::mul(fx::sin(a), half + m(150)),
                self.metres(-40, 60),
                fx::mul(fx::cos(a), half + m(150)),
            );
            let mut w = self.metres(6, 10);
            for j in 0..6 {
                let b = a + fx::HALF + deg(30) * (j % 3 - 1) + deg(self.rng.range(-20, 20));
                let step = v3(
                    fx::mul(fx::sin(b), m(22)),
                    self.metres(-8, 8),
                    fx::mul(fx::cos(b), m(22)),
                );
                let q = p.add(step);
                self.rod(p, q, w, if j % 2 == 0 { pale } else { flesh }, false);
                p = q;
                w = w * 8 / 10;
            }
        }
        // bubbles of light rising through it
        let bubble = mix(self.c("god_crystal"), self.c("god_light"), ONE / 2);
        for _ in 0..26 {
            let a = deg(self.rng.range(0, 359));
            let r = half + self.metres(30, 110);
            let p = v3(
                fx::mul(fx::sin(a), r),
                self.metres(-30, 60),
                fx::mul(fx::cos(a), r),
            );
            let s = self.metres(3, 12);
            self.halo(Mesh::Sphere, boxed(p, v3(s, s, s)), bubble, ONE / 3);
        }
        // swirls: the nuclear chaos at the centre of everything
        let swirl = [self.c("eldritch_glow"), self.c("god_crystal"), self.c("god_sun")];
        for k in 0..9 {
            let r = half + self.metres(60, 160);
            let at = v3(0, self.metres(30, 90), 0);
            let ring = Affine::translate(at)
                .then(&Affine::rot_y(deg(40 * k)))
                .then(&Affine::rot_x(deg(self.rng.range(-50, 50))))
                .then(&Affine::scale(v3(r * 2, m(3), r * 2)));
            self.halo(Mesh::Ring, ring, swirl[k as usize % 3], ONE / 4);
        }
        // the dancers, piping on their ring
        let dancer = self.c("god_dancer");
        let flute = self.c("god_bone");
        for k in 0..16 {
            let a = fx::TURN / 16 * k;
            let r = half + m(90);
            let y = m(30) + fx::mul(fx::sin(a * 3), m(8));
            let p = v3(fx::mul(fx::sin(a), r), y, fx::mul(fx::cos(a), r));
            self.shape(
                Mesh::Blob,
                boxed(p, v3(m(5), m(16), m(5))),
                dancer,
                ONE / 5,
                false,
            );
            let lean = v3(fx::mul(fx::cos(a), m(5)), m(6), -fx::mul(fx::sin(a), m(5)));
            self.rod(p.add(v3(0, m(5), 0)), p.add(lean), dm(6), flute, false);
        }
        // the pipes: vast flutes standing out in the dark
        let pipe = self.c("god_bone");
        for k in 0..7 {
            let a = fx::TURN / 7 * k + deg(20);
            let r = half + m(170);
            let h = self.metres(90, 200);
            let p = v3(fx::mul(fx::sin(a), r), h / 2 - m(60), fx::mul(fx::cos(a), r));
            self.shape(Mesh::Cyl, boxed(p, v3(m(9), h, m(9))), pipe, 0, false);
            for j in 1..5 {
                let hole = p.add(v3(0, h / 2 - h * j / 6, 0)).add(p.neg().norm().scale(m(4)));
                self.shape(Mesh::Sphere, boxed(hole, v3(m(3), m(3), m(3))), void, 0, false);
            }
        }
    }

    // ---- the arenas ---------------------------------------------------------

    /// The Gullet: where you came down, ribbed and wet, teeth by the way on.
    fn gullet(&mut self) {
        let flesh = self.c("god_flesh");
        let pale = self.c("god_flesh_pale");
        let bone = self.c("god_bone");
        let teeth = self.c("god_teeth");
        self.slab(-m(31), -m(31), m(31), m(31), 0, flesh);
        self.floor_life(-m(28), -m(28), m(28), m(28), 0, 70);
        // folds in the floor
        for k in 0..12 {
            let z = -m(26) + m(5) * k;
            self.deco(Mesh::Blob, v3(0, dm(1), z), v3(m(40), dm(3), m(2)), pale);
        }
        // the ribs: arcs from side to side, overhead
        for k in 0..7 {
            let z = -m(22) + m(7) * k;
            let mut prev = v3(-m(27), 0, z);
            for j in 1..=8 {
                let a = fx::HALF * j / 8;
                let p = v3(-fx::mul(fx::cos(a), m(27)), fx::mul(fx::sin(a), m(18)), z);
                self.rod(prev, p, dm(9), bone, true);
                prev = p;
            }
            for s in [-1, 1] {
                self.block(s * m(27), 0, z, m(1), m(3), m(1), bone);
            }
        }
        // teeth either side of the way on
        for s in [-1, 1] {
            for k in 0..5 {
                let x = s * (GATE_GAP / 2 + m(3) + m(2) * k);
                self.deco(
                    Mesh::Cone,
                    v3(x, m(1), -m(25)),
                    v3(m(1), m(2) + dm(5) * k, m(1)),
                    teeth,
                );
            }
        }
        self.eye_stalk(-m(14), 0, -m(8), m(4));
        self.eye_stalk(m(15), 0, m(6), m(5));
    }

    /// The Still Point: the one calm place; a stopped clock, a basin, and
    /// on a pedestal the trumpet's case, open.
    fn still_point(&mut self) {
        let stone = self.c("god_stone");
        let gold = self.c("god_seam");
        let brass = self.c("god_sun");
        self.slab(-m(31), -m(31), m(31), m(31), 0, stone);
        // the basin (the fountain)
        self.solids.push(Solid::stand(0, 0, 0, m(2), dm(8), m(2)));
        self.deco(
            Mesh::Cyl,
            v3(0, dm(4), 0),
            v3(m(2) + dm(4), dm(8), m(2) + dm(4)),
            stone,
        );
        self.glow(
            Mesh::Cyl,
            v3(0, dm(8), 0),
            v3(m(2), dm(1), m(2)),
            self.c("god_crystal"),
        );
        // the clock, vast, stopped, standing in the dark to the north
        let face = Affine::translate(v3(0, m(22), -m(48)))
            .then(&Affine::rot_x(fx::QUARTER))
            .then(&Affine::scale(v3(m(36), m(2), m(36))));
        self.shape(Mesh::Cyl, face, self.c("god_bone"), ONE / 6, true);
        for k in 0..12 {
            let a = fx::TURN / 12 * k;
            let p = v3(
                fx::mul(fx::sin(a), m(15)),
                m(22) + fx::mul(fx::cos(a), m(15)),
                -m(46),
            );
            self.deco(Mesh::Cube, p, v3(m(1), m(2), dm(5)), gold);
        }
        // its hands at 21:14, and still
        for (a, len) in [(deg(-60), m(10)), (deg(84), m(13))] {
            let tip = v3(fx::mul(fx::sin(a), len), m(22) + fx::mul(fx::cos(a), len), -m(45));
            self.rod(v3(0, m(22), -m(45)), tip, dm(9), gold, true);
        }
        // the pedestal and the trumpet's case
        self.block(0, 0, -m(10), m(2), m(1), m(1) + dm(4), stone);
        let case = self.c("god_flesh_dark");
        self.deco(
            Mesh::Cube,
            v3(0, m(1) + dm(1), -m(10)),
            v3(m(1) + dm(6), dm(2), dm(9)),
            case,
        );
        self.deco(
            Mesh::Cube,
            v3(0, m(1) + dm(5), -m(10) - dm(5)),
            v3(m(1) + dm(6), dm(8), dm(1)),
            case,
        );
        // the bell's shape in the velvet, and the brass glinting
        self.glow(
            Mesh::Cone,
            v3(dm(4), m(1) + dm(3), -m(10)),
            v3(dm(4), dm(3), dm(4)),
            brass,
        );
        self.rod(
            v3(-dm(6), m(1) + dm(3), -m(10)),
            v3(dm(3), m(1) + dm(3), -m(10)),
            dm(1),
            brass,
            false,
        );
        self.bench_stone(-m(8), m(6));
        self.bench_stone(m(8), m(6));
    }

    fn bench_stone(&mut self, x: i32, z: i32) {
        let dark = self.c("god_stone_dark");
        self.block(x, 0, z, m(3), dm(5), dm(8), dark);
    }

    /// The Fold: islands in the void joined by arms; a corner island high
    /// up either way; and overhead, the Waiting City upside down.
    fn fold(&mut self) {
        let stone = self.c("god_stone");
        let dark = self.c("god_stone_dark");
        let flesh = self.c("god_flesh");
        // the middle, and its arms out to every gate
        self.slab(-m(12), -m(12), m(12), m(12), 0, stone);
        self.floor_life(-m(12), -m(12), m(12), m(12), 0, 40);
        self.slab(-m(7), m(12), m(7), m(31), 0, dark);
        self.slab(-m(7), -m(31), m(7), -m(12), 0, dark);
        self.slab(m(12), -m(7), m(31), m(7), 0, dark);
        self.slab(-m(31), -m(7), -m(12), m(7), 0, dark);
        // the corner islands: one level by a bridge, two up steps, one a
        // jump away
        self.slab(m(15), m(15), m(26), m(26), 0, flesh);
        self.slab(m(7), m(18), m(15), m(22), 0, dark);
        self.slab(m(15), -m(26), m(26), -m(15), m(4), flesh);
        self.climb(m(20), -m(7), 0, -ONE, m(8), m(4), 0, m(4));
        self.slab(-m(26), -m(26), -m(15), -m(15), m(6), flesh);
        self.climb(-m(7), -m(20), -ONE, 0, m(8), m(4), 0, m(6));
        self.slab(-m(26), m(9), -m(15), m(18), 0, flesh);
        // arches to stand under, and cover
        let bone = self.c("god_bone");
        self.arch(0, -m(6), true, m(6), m(4), bone);
        self.arch(-m(6), m(4), false, m(5), m(3) + dm(5), bone);
        for (x, z) in [
            (m(7), m(7)),
            (-m(8), -m(8)),
            (m(8), -m(7)),
            (m(20), m(20)),
            (-m(20), m(12)),
        ] {
            self.rubble(x, z);
        }
        self.crystals(m(22), m(4), -m(22));
        self.eye_stalk(-m(22), m(6), -m(22), m(4));
        // an arch hanging upside down from nothing, over the middle
        let hang = m(16);
        for s in [-1, 1] {
            self.deco(
                Mesh::Cube,
                v3(s * m(4), hang + m(3), 0),
                v3(dm(8), m(6), dm(8)),
                bone,
            );
        }
        self.deco(Mesh::Cube, v3(0, hang, 0), v3(m(9), dm(6), dm(8)), bone);
        self.hanging_city();
    }

    /// The Waiting City overhead, upside down: blocks of its buildings
    /// hanging from a street, windows lit, 30 to 55 m up.
    fn hanging_city(&mut self) {
        let walls = self.pal.walls.clone();
        let street = self.c("paving");
        let lit = self.c("window");
        let ceiling = m(56);
        self.deco(
            Mesh::Cube,
            v3(0, ceiling + dm(5), 0),
            v3(m(110), m(1), m(110)),
            street,
        );
        for k in 0..14 {
            let x = self.metres(-48, 48);
            let z = self.metres(-48, 48);
            let w = self.metres(8, 14);
            let d = self.metres(8, 14);
            let h = self.metres(10, 24);
            let wall = walls[k as usize % walls.len()];
            let c = v3(x, ceiling - h / 2, z);
            self.shape(Mesh::Cube, boxed(c, v3(w, h, d)), wall, 0, true);
            // its roof, now at the bottom, and a water tank under it
            self.deco(
                Mesh::Cube,
                v3(x, ceiling - h - dm(2), z),
                v3(w + dm(4), dm(4), d + dm(4)),
                shade(wall, 80),
            );
            if k % 3 == 0 {
                self.deco(
                    Mesh::Cyl,
                    v3(x, ceiling - h - m(2), z),
                    v3(m(3), m(3), m(3)),
                    self.c("metal"),
                );
            }
            let seed = self.rng.range(0, 999) * (ONE / 1000);
            for (n, half) in [(v3(0, 0, ONE), d / 2), (v3(ONE, 0, 0), w / 2)] {
                let span = if n.x != 0 { d } else { w };
                self.facade(c.add(n.scale(half)), n, span - m(1), h - m(2), lit, seed);
            }
        }
    }

    /// The Penrose Stair: four flights round a well, each climbing 3 m;
    /// the top of the fourth meets, through a seam, the foot of the first.
    fn penrose(&mut self) {
        let stone = self.c("god_stone");
        let dark = self.c("god_stone_dark");
        let gold = self.c("god_seam");
        // the floor round it all, out to the gates
        self.slab(-m(31), LANE, m(31), m(31), 0, dark);
        self.slab(-m(31), -m(31), m(31), -LANE, 0, dark);
        self.slab(LANE, -LANE, m(31), LANE, 0, dark);
        self.slab(-m(31), -LANE, -LANE, LANE, 0, dark);
        self.floor_life(-m(30), LANE, m(30), m(30), 0, 30);
        self.floor_life(-m(30), -m(30), m(30), -LANE, 0, 30);
        let w = LANE - WELL;
        let mid = (LANE + WELL) / 2;
        // the south-west corner, at the floor: where flight one begins and
        // the stub the seam lets out onto
        self.slab(-LANE, SEAM_Z, -WELL, LANE, 0, stone);
        // flight one, east along the south lane, 0 to 3 m
        self.climb(-WELL, mid, ONE, 0, WELL * 2, w, 0, FLIGHT);
        self.slab(WELL, WELL, LANE, LANE, FLIGHT, stone);
        // flight two, north along the east lane, 3 to 6 m
        self.climb(mid, WELL, 0, -ONE, WELL * 2, w, FLIGHT, FLIGHT * 2);
        self.slab(WELL, -LANE, LANE, -WELL, FLIGHT * 2, stone);
        // flight three, west along the north lane, 6 to 9 m
        self.climb(WELL, -mid, -ONE, 0, WELL * 2, w, FLIGHT * 2, FLIGHT * 3);
        self.slab(-LANE, -LANE, -WELL, -WELL, FLIGHT * 3, stone);
        // flight four, south along the west lane, 9 to 12 m, ending at the
        // seam; past it a landing hangs over the stub
        self.climb(-mid, -WELL, 0, ONE, WELL + SEAM_Z, w, FLIGHT * 3, FLIGHT * 4);
        self.slab(-LANE, SEAM_Z, -WELL, WELL, FLIGHT * 4, stone);
        // the seam: the top, facing back up the flight, and the stub
        // below it facing the other way, so one is the other 12 m down
        let k = self.seams.len();
        let film = self.c("god_crystal");
        let top = self.frame(v3(-mid, FLIGHT * 4, SEAM_Z), 0, w / 2, m(3), film, k + 1);
        let foot = self.frame(v3(-mid, 0, SEAM_Z), fx::HALF, w / 2, m(3), film, k);
        self.seams.push(top);
        self.seams.push(foot);
        // the well: gold lines down its rim, and an eye far below
        for (x, z) in [(-WELL, -WELL), (WELL, -WELL), (WELL, WELL), (-WELL, WELL)] {
            self.rod(v3(x, -m(30), z), v3(x, FLIGHT * 4 + m(4), z), dm(2), gold, false);
        }
        self.shape(
            Mesh::Sphere,
            boxed(v3(0, -m(40), 0), v3(m(9), m(9), m(9))),
            self.c("god_iris"),
            ONE,
            false,
        );
        // Relativity: a stair on its side and one upside down, out in the
        // dark, going nowhere
        for (at, roll) in [
            (v3(-m(26), m(20), -m(20)), fx::QUARTER),
            (v3(m(24), m(26), m(18)), fx::HALF),
        ] {
            for s in 0..8 {
                let p = Affine::translate(at)
                    .then(&Affine::rot_z(roll))
                    .then(&Affine::translate(v3(0, RISE * s, dm(9) * s)))
                    .then(&Affine::scale(v3(m(4), RISE, dm(9))));
                self.shape(Mesh::Cube, p, if s % 2 == 0 { stone } else { dark }, 0, true);
            }
        }
        // arches on the outer floor to stand under, and cover
        let bone = self.c("god_bone");
        self.arch(-m(18), m(20), true, m(5), m(4), bone);
        self.arch(m(19), -m(18), false, m(5), m(4), bone);
        self.arch(m(20), m(20), true, m(5), m(3) + dm(5), bone);
        for (x, z) in [
            (m(16), m(14)),
            (-m(15), -m(17)),
            (m(22), -m(5)),
            (-m(22), m(4)),
            (m(5), m(17)),
        ] {
            self.rubble(x, z);
        }
    }

    /// The Doors: a floor broken by holes, three pairs of gold frames each
    /// leading to the other across the room, and doorways that lead
    /// nowhere at all.
    fn doors(&mut self) {
        let stone = self.c("god_stone");
        let dark = self.c("god_stone_dark");
        // tiles six metres across; a few left out, never on the ways
        // between the gates
        let holes = [(-3, -3), (2, -3), (-3, 2), (2, 2), (-5, 1), (4, -2)];
        for i in -5..5 {
            for j in -5..5 {
                if holes.contains(&(i, j)) {
                    continue;
                }
                let (x0, z0) = (m(6) * i + m(1), m(6) * j + m(1));
                let c = if (i + j) % 2 == 0 { stone } else { dark };
                self.slab(x0, z0, x0 + m(6), z0 + m(6), 0, c);
            }
        }
        self.floor_life(-m(29), -m(29), m(29), m(29), 0, 60);
        // a dais in the north-east, up steps
        self.slab(m(13), -m(23), m(25), -m(13), m(3) + dm(2), stone);
        self.climb(m(19), -m(5), 0, -ONE, m(8), m(4), 0, m(3) + dm(2));
        // the pairs
        let films = [self.c("god_crystal"), self.c("god_iris"), self.c("god_dancer")];
        let (half, h) = (m(1) + dm(5), m(3) + dm(2));
        self.seam_pair(
            (v3(-m(14), 0, m(10)), 0),
            (v3(m(14), 0, -m(9)), fx::QUARTER),
            half,
            h,
            films[0],
        );
        self.seam_pair(
            (v3(-m(20), 0, -m(10)), -fx::QUARTER),
            (v3(m(20), 0, m(14)), fx::HALF),
            half,
            h,
            films[1],
        );
        self.seam_pair(
            (v3(m(12), 0, m(22)), -fx::QUARTER),
            (v3(-m(12), 0, -m(22)), 0),
            half,
            h,
            films[2],
        );
        // doorways to nowhere: walls with a door shape in them, nothing in it
        let bone = self.c("god_bone");
        self.arch(-m(22), m(22), true, m(4), m(4), bone);
        self.arch(m(24), m(12), false, m(4), m(4), bone);
        self.arch(-m(8), -m(14), true, m(4), m(3) + dm(5), bone);
        for (x, z) in [
            (m(8), m(9)),
            (-m(9), m(16)),
            (m(9), -m(17)),
            (-m(17), m(3)),
            (m(17), m(4)),
        ] {
            self.rubble(x, z);
        }
        self.eye_stalk(m(19), m(3) + dm(2), -m(19), m(3));
    }

    /// The Lattice: Xen. Rocks hang in the void at several heights; one is
    /// reached by a stair grown up a root, the rest by updrafts and jumps.
    fn lattice(&mut self) {
        let stone = self.c("god_stone");
        let dark = self.c("god_stone_dark");
        let flesh = self.c("god_flesh");
        self.slab(-m(16), -m(16), m(16), m(16), 0, stone);
        self.floor_life(-m(16), -m(16), m(16), m(16), 0, 40);
        self.slab(-m(7), m(16), m(7), m(31), 0, dark);
        self.slab(-m(7), -m(31), m(7), -m(16), 0, dark);
        self.slab(m(16), -m(7), m(31), m(7), 0, dark);
        self.slab(-m(31), -m(7), -m(16), m(7), 0, dark);
        // the rocks
        let rocks = [
            (m(23), m(23), m(6), m(4)),
            (-m(23), m(22), m(5), m(6)),
            (-m(23), -m(23), m(6), m(5)),
            (m(23), -m(22), m(5), m(7)),
            (0, 0, m(5), m(8)),
            (m(12), -m(25), m(4), m(3)),
        ];
        for (x, z, r, top) in rocks {
            self.slab(x - r, z - r, x + r, z + r, top, flesh);
            self.crystals(x + r / 2, top, z - r / 2);
        }
        // the root stair up to the south-east rock
        self.climb(m(7), m(20), ONE, 0, m(10), m(3), 0, m(4));
        // the hanging rock over the middle: stand under it
        for (x, z) in [(-m(4), -m(4)), (m(4), -m(4)), (-m(4), m(4)), (m(4), m(4))] {
            self.post(x, 0, z, dm(9), m(6), dark);
        }
        // updrafts beside the high rocks
        for (x, z) in [(-m(14), m(14)), (-m(14), -m(14)), (m(14), -m(14)), (m(10), m(9))] {
            self.updraft(x, 0, z);
        }
        for (x, z) in [(m(9), m(1)), (-m(9), m(2)), (m(6), m(10)), (-m(6), -m(10))] {
            self.rubble(x, z);
        }
        // light-stalks and crystals in the dark between
        for k in 0..10 {
            let a = fx::TURN / 10 * k;
            let p = v3(
                fx::mul(fx::sin(a), m(38)),
                self.metres(-10, 20),
                fx::mul(fx::cos(a), m(38)),
            );
            self.deco(Mesh::Octa, p, v3(m(3), m(5), m(3)), dark);
            self.crystals(p.x, p.y + m(2), p.z);
        }
    }

    /// The Orrery: rings of floor round a sun, spokes across the gaps, the
    /// inner ring raised; planets hanging.
    fn orrery(&mut self) {
        let stone = self.c("god_stone");
        let dark = self.c("god_stone_dark");
        let flesh = self.c("god_flesh");
        let sun = self.c("god_sun");
        let inner = m(3) + dm(2);
        self.annulus(m(23), m(32), 0, dark, 40);
        self.annulus(m(15), m(20), 0, stone, 28);
        self.annulus(m(7), m(11), inner, flesh, 20);
        // the spokes: flat across the outer gap, climbing across the inner
        for (dx, dz) in [(0, ONE), (0, -ONE), (ONE, 0), (-ONE, 0)] {
            let (ax, az) = (fx::mul(dx, m(1)), fx::mul(dz, m(1)));
            let across = if dx == 0 { (m(2), 0) } else { (0, m(2)) };
            let (a0, a1) = (v3(ax * 19, 0, az * 19), v3(ax * 24, 0, az * 24));
            self.slab(
                a0.x.min(a1.x) - across.0,
                a0.z.min(a1.z) - across.1,
                a0.x.max(a1.x) + across.0,
                a0.z.max(a1.z) + across.1,
                0,
                stone,
            );
            self.climb(ax * 16, az * 16, -dx, -dz, m(6), m(3), 0, inner);
        }
        // the sun: a pillar to stand on, burning
        self.block(0, 0, 0, m(4), m(4), m(4), sun);
        self.halo(
            Mesh::Sphere,
            boxed(v3(0, m(9), 0), v3(m(9), m(9), m(9))),
            sun,
            ONE / 2,
        );
        self.glow(Mesh::Sphere, v3(0, m(9), 0), v3(m(4), m(4), m(4)), sun);
        // planets on their rings of light, hanging
        let planets = [
            self.c("god_crystal"),
            self.c("god_iris"),
            self.c("god_dancer"),
            self.c("god_flesh_pale"),
        ];
        for k in 0..4 {
            let r = m(10 + 6 * k);
            let a = deg(70 * k + 20);
            let p = v3(fx::mul(fx::sin(a), r), m(8) + m(2) * k, fx::mul(fx::cos(a), r));
            let s = m(2) + dm(5) * k;
            self.deco(Mesh::Sphere, p, v3(s, s, s), planets[k as usize]);
            self.halo(
                Mesh::Ring,
                Affine::translate(v3(0, p.y, 0)).then(&Affine::scale(v3(r * 2, dm(2), r * 2))),
                planets[k as usize],
                ONE / 5,
            );
        }
        // arches on the outer ring to stand under, and cover
        let bone = self.c("god_bone");
        for (x, z, along) in [
            (m(19), m(19), true),
            (-m(19), -m(19), false),
            (-m(19), m(20), true),
            (m(20), -m(19), false),
        ] {
            self.arch(x, z, along, m(4), m(4), bone);
        }
        for (x, z) in [(m(12), m(13)), (-m(12), m(13)), (m(12), -m(13)), (-m(13), -m(12))] {
            self.rubble(x, z);
        }
    }

    /// The Heart: a disc in the void before the god, ringed with standing
    /// bones and eye-stalks; open to the north, where the god heaves.
    fn heart(&mut self) {
        let r = COURT_RADIUS;
        let flesh = self.c("god_flesh");
        let pale = self.c("god_flesh_pale");
        let bone = self.c("god_bone");
        let vein = self.c("god_vein");
        self.annulus(0, r, 0, flesh, 1);
        self.floor_life(-m(20), -m(20), m(20), m(20), 0, 50);
        // veins across it, toward the god
        for k in 0..9 {
            let a = deg(-60 + 15 * k);
            let tip = v3(
                fx::mul(fx::sin(a), r - m(1)),
                dm(1),
                -fx::mul(fx::cos(a), r - m(1)),
            );
            self.rod(v3(0, dm(1), m(4)), tip, dm(4), vein, false);
        }
        self.deco(
            Mesh::Ring,
            v3(0, dm(1), 0),
            v3(r * 2 - m(2), dm(2), r * 2 - m(2)),
            pale,
        );
        // standing bones and eye-stalks round the rim, none to the north
        for k in 0..18 {
            let a = fx::TURN / 18 * k + deg(10);
            let north = fx::wrap(a - fx::HALF).abs() < deg(50);
            if north {
                continue;
            }
            let (s, c) = (fx::sin(a), fx::cos(a));
            let p = v3(fx::mul(s, r - m(3)), 0, fx::mul(c, r - m(3)));
            if k % 2 == 0 {
                let h = self.metres(4, 7);
                self.post(p.x, 0, p.z, dm(9), h, bone);
                self.deco(
                    Mesh::Sphere,
                    v3(p.x, h, p.z),
                    v3(m(1) + dm(3), m(1), m(1) + dm(3)),
                    bone,
                );
            } else {
                let h = self.metres(3, 6);
                self.eye_stalk(p.x, 0, p.z, h);
            }
        }
        // a raised bone terrace either side, up a stair, and an arch to
        // stand under
        let high = m(3) + dm(6);
        for s in [-1, 1] {
            self.climb(s * m(19), m(10), 0, -ONE, m(12), m(3), 0, high);
            self.slab(s * m(15), -m(2), s * m(24), -m(12), high, pale);
        }
        self.arch(0, m(12), true, m(6), m(4), bone);
        for (x, z) in [(-m(7), m(2)), (m(7), m(3)), (0, -m(6))] {
            self.rubble(x, z);
        }
    }
}
