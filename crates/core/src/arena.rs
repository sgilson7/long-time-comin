//! Building an arena: the solid boxes the bodies collide with, and every
//! shape that is only looked at. An arena is a square of the Waiting City
//! walled by building facades, with gates in the gaps, an interior laid out
//! by its archetype (plaza, market, rooftops, canal, atrium, and the
//! special crater, antechamber and court), and around it a skyline that
//! goes out to the clouds.
//!
//! The look follows `Sam_Comm_final.png`: salmon and coral walls, rows of
//! lit windows, rooftop clutter (air-conditioning boxes, antennas, water
//! tanks, billboards), cables between buildings, plants everywhere, and
//! one plum ink line round everything (the `ink` flag; the renderer draws it
//! as an inverted hull).

use crate::content::{Palette, Rgb};
use crate::fx::{self, deg, int, ONE};
use crate::geom::{v3, Affine, V3};
use crate::level::{Level, Solid};
use crate::mesh::Mesh;
use crate::rng::Rng;

/// Half the width of an arena's floor.
pub const HALF: i32 = int(28);
/// the gap in the wall a gate stands in
pub const GATE_GAP: i32 = int(8);
/// the boss court's radius
pub const COURT_RADIUS: i32 = int(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    pub mesh: Mesh,
    pub at: Affine,
    pub rgb: Rgb,
    pub glow: i32,
    pub ink: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    South,
    North,
    East,
    West,
}

impl Side {
    /// Where the gate on this side stands, and the way it faces into the arena.
    pub fn place(self, half: i32) -> (V3, i32) {
        match self {
            Side::South => (v3(0, 0, half), 0),
            Side::North => (v3(0, 0, -half), fx::HALF),
            Side::East => (v3(half, 0, 0), fx::QUARTER),
            Side::West => (v3(-half, 0, 0), -fx::QUARTER),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arena {
    pub level: Level,
    pub shapes: Vec<Shape>,
    pub start: V3,
    pub start_yaw: i32,
    pub ground_spawns: Vec<V3>,
    pub air_spawns: Vec<V3>,
    /// gate positions on the floor, and the way each faces into the arena
    pub gates: Vec<(V3, i32)>,
    pub centre: V3,
    /// where a rest node's fountain heals
    pub fountain: Option<V3>,
}

// The builder's helpers take a box's six numbers and a colour, as the TikZ
// art's do; a struct per call would hide the sizes at the call sites.
#[allow(clippy::too_many_arguments)]
struct B<'a> {
    solids: Vec<Solid>,
    shapes: Vec<Shape>,
    rng: Rng,
    pal: &'a Palette,
    canal: bool,
}

/// The dry canal's channel runs north to south between these two x values.
const CANAL: (i32, i32) = (-int(17), -int(7));
/// shallow enough to jump out of (a jump clears 1.5 m): falling in is a
/// detour, never a trap (SECOND-ORDER G5)
const CANAL_DEPTH: i32 = ONE * 12 / 10;
/// a walkway slab's thickness
const DECK: i32 = ONE * 4 / 10;

fn m(x: i32) -> i32 {
    int(x)
}

/// tenths of a metre
fn dm(x: i32) -> i32 {
    ONE * x / 10
}

#[allow(clippy::too_many_arguments)]
impl<'a> B<'a> {
    fn shape(&mut self, mesh: Mesh, at: Affine, rgb: Rgb, glow: i32, ink: bool) {
        self.shapes.push(Shape {
            mesh,
            at,
            rgb,
            glow,
            ink,
        });
    }

    /// A shape from its centre and full size.
    fn deco(&mut self, mesh: Mesh, c: V3, size: V3, rgb: Rgb) {
        self.shape(mesh, boxed(c, size), rgb, 0, true);
    }

    /// A facade panel facing `n`, `w` along the face and `h` high, its
    /// centre `at` (just proud of the wall); the shader draws its windows.
    /// `seed`, in the glow channel, picks which are lit.
    fn facade(&mut self, at: V3, n: V3, w: i32, h: i32, lit: Rgb, seed: i32) {
        // the panel's +y turns to the normal; x runs up the wall and z
        // along it, so the basis stays right-handed (x × y = z)
        let along = V3::UP.cross(n);
        let af = Affine {
            c0: V3::UP.scale(h),
            c1: n.scale(ONE / 64),
            c2: along.scale(w),
            t: at.add(n.scale(dm(1) / 2)),
        };
        self.shape(Mesh::Facade, af, lit, seed, false);
    }

    fn glow(&mut self, mesh: Mesh, c: V3, size: V3, rgb: Rgb) {
        self.shape(mesh, boxed(c, size), rgb, ONE, false);
    }

    /// A solid box standing on `y`, drawn.
    fn block(&mut self, x: i32, y: i32, z: i32, w: i32, h: i32, d: i32, rgb: Rgb) {
        self.solids.push(Solid::stand(x, y, z, w, h, d));
        self.deco(Mesh::Cube, v3(x, y + h / 2, z), v3(w, h, d), rgb);
    }

    /// A solid nobody sees: the arena's outer bound.
    fn wall(&mut self, min: V3, max: V3) {
        self.solids.push(Solid::new(min, max));
    }

    fn rod(&mut self, a: V3, b: V3, w: i32, rgb: Rgb, ink: bool) {
        if let Some(af) = segment(a, b, w) {
            self.shape(Mesh::Cyl, af, rgb, 0, ink);
        }
    }

    fn pick(&mut self, list: &[Rgb]) -> Rgb {
        list[self.rng.range(0, list.len() as i32 - 1) as usize]
    }

    fn c(&self, name: &str) -> Rgb {
        self.pal.c(name)
    }

    /// Between `lo` and `hi` metres, in tenths.
    fn metres(&mut self, lo: i32, hi: i32) -> i32 {
        dm(self.rng.range(lo * 10, hi * 10))
    }

    // ---- the city's furniture ------------------------------------------

    fn tree(&mut self, x: i32, y: i32, z: i32, scale: i32) {
        let h = fx::mul(m(4), scale);
        let trunk = self.c("trunk");
        self.deco(Mesh::Cyl, v3(x, y + h / 2, z), v3(dm(3), h, dm(3)), trunk);
        let crowns = self.rng.range(2, 3);
        for k in 0..crowns {
            let g = self.pick(&self.pal.greens.clone());
            let r = fx::mul(m(2) + dm(self.rng.range(0, 8)), scale);
            let off = v3(
                dm(self.rng.range(-9, 9)),
                h + dm(k * 7),
                dm(self.rng.range(-9, 9)),
            );
            self.deco(
                Mesh::Blob,
                v3(x, y, z).add(off),
                v3(r * 2, r * 2 * 4 / 5, r * 2),
                g,
            );
        }
    }

    fn planter(&mut self, x: i32, y: i32, z: i32) {
        let stone = self.c("stone");
        self.block(x, y, z, dm(16), dm(6), dm(16), stone);
        self.tree(x, y + dm(6), z, ONE * 3 / 4);
    }

    fn bush(&mut self, x: i32, y: i32, z: i32) {
        let g = self.pick(&self.pal.greens.clone());
        let r = dm(self.rng.range(6, 12));
        self.deco(Mesh::Blob, v3(x, y + r / 2, z), v3(r * 2, r, r * 2), g);
    }

    fn lamp(&mut self, x: i32, y: i32, z: i32) {
        let metal = self.c("metal_dark");
        let lamp = self.c("lamp");
        self.deco(Mesh::Cyl, v3(x, y + m(2), z), v3(dm(2), m(4), dm(2)), metal);
        self.deco(Mesh::Cube, v3(x, y + m(4), z), v3(dm(6), dm(1), dm(6)), metal);
        self.glow(Mesh::Blob, v3(x, y + dm(37), z), v3(dm(5), dm(5), dm(5)), lamp);
    }

    fn crate_stack(&mut self, x: i32, y: i32, z: i32) {
        let wood = self.c("wood");
        let n = self.rng.range(1, 3);
        let mut h = y;
        for k in 0..n {
            let s = dm(self.rng.range(8, 13));
            let jitter = dm(self.rng.range(-2, 2));
            self.block(x + jitter * k, h, z - jitter * k, s, s, s, wood);
            h += s;
        }
    }

    fn barrel(&mut self, x: i32, y: i32, z: i32) {
        let c = self.pick(&[self.c("metal"), self.c("cloth_2"), self.c("trim")]);
        self.solids.push(Solid::stand(x, y, z, dm(8), dm(11), dm(8)));
        self.deco(Mesh::Cyl, v3(x, y + dm(55) / 10, z), v3(dm(8), dm(11), dm(8)), c);
    }

    fn bench(&mut self, x: i32, y: i32, z: i32, along_x: bool) {
        let wood = self.c("wood");
        let metal = self.c("metal_dark");
        let (w, d) = if along_x { (m(2), dm(5)) } else { (dm(5), m(2)) };
        self.block(x, y + dm(4), z, w, dm(1), d, wood);
        self.deco(
            Mesh::Cube,
            v3(x, y + dm(2), z),
            v3(w * 9 / 10, dm(4), d / 3),
            metal,
        );
    }

    fn sign(&mut self, at: V3, w: i32, h: i32, normal_x: bool) {
        let col = self.pick(&self.pal.signs.clone());
        let ink = self.c("ink");
        let (sx, sz) = if normal_x { (dm(2), w) } else { (w, dm(2)) };
        self.deco(Mesh::Cube, at, v3(sx, h, sz), ink);
        let (gx, gz) = if normal_x {
            (dm(3), w * 8 / 10)
        } else {
            (w * 8 / 10, dm(3))
        };
        self.glow(Mesh::Cube, at, v3(gx, h * 7 / 10, gz), col);
    }

    fn car(&mut self, x: i32, y: i32, z: i32, yaw: i32) {
        let body = self.pick(&self.pal.signs.clone());
        let body = crate::render::shade(body, 75);
        let ink = self.c("ink");
        let glass = self.c("window_dark");
        let rot = Affine::translate(v3(x, y, z)).then(&Affine::rot_y(yaw));
        self.solids.push(Solid::stand(x, y, z, dm(42), dm(14), dm(42)));
        let parts = [
            (v3(0, dm(6), 0), v3(dm(18), dm(7), dm(42)), body),
            (v3(0, dm(12), dm(2)), v3(dm(16), dm(6), dm(22)), glass),
        ];
        for (c, s, rgb) in parts {
            self.shape(
                Mesh::Cube,
                rot.then(&Affine::translate(c)).then(&Affine::scale(s)),
                rgb,
                0,
                true,
            );
        }
        for (wx, wz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
            let at = v3(dm(9) * wx, dm(3), dm(13) * wz);
            let wheel = rot
                .then(&Affine::translate(at))
                .then(&Affine::rot_z(fx::QUARTER))
                .then(&Affine::scale(v3(dm(6), dm(3), dm(6))));
            self.shape(Mesh::Cyl, wheel, ink, 0, false);
        }
    }

    /// A stair of steps from `(x, y, z)` climbing `rise` along the unit
    /// direction `(dx, dz)`, `w` wide; each step is walkable without a jump.
    fn stairs(&mut self, x: i32, y: i32, z: i32, dx: i32, dz: i32, rise: i32, w: i32) {
        let step_h = dm(4);
        let run = dm(6);
        let steps = (rise + step_h - 1) / step_h;
        let stone = self.c("stone");
        let dark = self.c("paving_dark");
        for k in 0..steps {
            let top = ((k + 1) * step_h).min(rise);
            let cx = x + fx::mul(dx, run * k + run / 2);
            let cz = z + fx::mul(dz, run * k + run / 2);
            let (sw, sd) = if dx != 0 { (run, w) } else { (w, run) };
            let rgb = if k % 2 == 0 { stone } else { dark };
            self.block(cx, y, cz, sw, top, sd, rgb);
        }
    }

    /// A raised platform with a rail of posts and a stair down one side.
    fn platform(&mut self, x: i32, z: i32, w: i32, d: i32, h: i32, stair_dir: (i32, i32)) {
        let stone = self.c("stone");
        let trim = self.c("trim");
        self.block(x, 0, z, w, h, d, stone);
        self.deco(
            Mesh::Cube,
            v3(x, h + dm(1) / 2, z),
            v3(w + dm(2), dm(1), d + dm(2)),
            trim,
        );
        let (sx, sz) = stair_dir;
        // the stair leaves from the middle of the side it faces
        let run = dm(6) * ((h + dm(4) - 1) / dm(4));
        let (ox, oz) = (x + fx::mul(sx, w / 2 + run), z + fx::mul(sz, d / 2 + run));
        self.stairs(ox, 0, oz, -sx, -sz, h, m(3));
        // posts at the corners, a bush or a crate on top
        for (px, pz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
            let at = v3(x + px * (w / 2 - dm(3)), h, z + pz * (d / 2 - dm(3)));
            self.deco(Mesh::Cyl, at.add(v3(0, dm(5), 0)), v3(dm(2), m(1), dm(2)), trim);
        }
        if self.rng.chance(50) {
            self.bush(x + w / 4, h, z - d / 4);
        } else {
            self.crate_stack(x - w / 4, h, z + d / 4);
        }
    }

    /// A walkway or bridge: a slab whose top is at `h`, over the rectangle
    /// from `(x0, z0)` to `(x1, z1)`, with rails along both long sides and a
    /// solid post at each `post`. Under it is clear to walk through when
    /// `h` is above a body's height (SECOND-ORDER V1).
    fn deck(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, h: i32, posts: &[(i32, i32)]) {
        let stone = self.c("stone");
        let trim = self.c("trim");
        let (cx, cz) = ((x0 + x1) / 2, (z0 + z1) / 2);
        let (w, d) = ((x1 - x0).abs(), (z1 - z0).abs());
        self.block(cx, h - DECK, cz, w, DECK, d, stone);
        self.deco(
            Mesh::Cube,
            v3(cx, h - DECK - dm(1), cz),
            v3(w - dm(4), dm(2), d - dm(4)),
            crate::render::shade(trim, 85),
        );
        // rails: a top bar and posts along the two long sides
        let along_x = w >= d;
        for side in [-1, 1] {
            let (a, b) = if along_x {
                (
                    v3(x0.min(x1), h + dm(9), cz + side * (d / 2 - dm(1))),
                    v3(x0.max(x1), h + dm(9), cz + side * (d / 2 - dm(1))),
                )
            } else {
                (
                    v3(cx + side * (w / 2 - dm(1)), h + dm(9), z0.min(z1)),
                    v3(cx + side * (w / 2 - dm(1)), h + dm(9), z0.max(z1)),
                )
            };
            self.rod(a, b, dm(1), trim, true);
            let n = (a.dist(b) / m(2)).max(1);
            for k in 0..=n {
                let p = a.lerp(b, fx::div(k * ONE, n * ONE));
                self.rod(p.add(v3(0, -dm(9), 0)), p, dm(1) / 2, trim, false);
            }
        }
        for &(px, pz) in posts {
            self.block(px, 0, pz, dm(7), h - DECK, dm(7), crate::render::shade(stone, 85));
        }
    }

    /// Cover to hide behind: `n` broken walls a little taller than a body,
    /// set where they block nothing the arena needs (gate approaches, the
    /// entrance, stairs and decks).
    fn cover_walls(&mut self, half: i32, n: i32) {
        let mut placed = 0;
        let mut tries = 0;
        while placed < n && tries < 200 {
            tries += 1;
            let x = dm(self.rng.range(-(half / dm(1)) + 40, half / dm(1) - 40));
            let z = dm(self.rng.range(-(half / dm(1)) + 40, half / dm(1) - 100));
            if x.abs() < m(4) || z.abs() < m(4) || (self.canal && x < CANAL.1 + m(3)) {
                continue;
            }
            let along_x = self.rng.chance(50);
            let (w, d) = if along_x { (m(4), dm(6)) } else { (dm(6), m(4)) };
            let probe = Level::new(self.solids.clone());
            // room round it to walk, and nothing over it
            if probe.blocked(
                v3(x - w / 2 - m(2), -dm(5), z - d / 2 - m(2)),
                v3(x + w / 2 + m(2), m(8), z + d / 2 + m(2)),
            ) {
                continue;
            }
            let wall = self.pick(&self.pal.walls.clone());
            let h = dm(21);
            self.block(x, 0, z, w, h, d, wall);
            // a broken top: a notch and fallen bricks
            let notch = if along_x {
                v3(x + w / 4, h, z)
            } else {
                v3(x, h, z + d / 4)
            };
            self.deco(
                Mesh::Cube,
                notch.add(v3(0, dm(2), 0)),
                v3(
                    if along_x { m(1) } else { d },
                    dm(4),
                    if along_x { d } else { m(1) },
                ),
                crate::render::shade(wall, 85),
            );
            let brick = self.c("paving_dark");
            for k in 0..3 {
                let off = if along_x {
                    v3(dm(8) * (k - 1), 0, d)
                } else {
                    v3(d, 0, dm(8) * (k - 1))
                };
                self.deco(
                    Mesh::Octa,
                    v3(x, dm(1), z).add(off),
                    v3(dm(4), dm(3), dm(4)),
                    brick,
                );
            }
            placed += 1;
        }
    }

    /// A building, its footprint centred on `(x, z)`, with windows on the
    /// faces in `faces` (+x, −x, +z, −z), trim, and rooftop clutter. Solid
    /// when `solid`.
    fn building(
        &mut self,
        x: i32,
        z: i32,
        w: i32,
        d: i32,
        h: i32,
        faces: [bool; 4],
        solid: bool,
        detail: bool,
    ) {
        let wall = self.pick(&self.pal.walls.clone());
        let trim = crate::render::shade(wall, 80);
        if solid {
            self.block(x, 0, z, w, h, d, wall);
        } else {
            self.deco(Mesh::Cube, v3(x, h / 2, z), v3(w, h, d), wall);
        }
        // a cornice and a band at each storey
        self.deco(
            Mesh::Cube,
            v3(x, h + dm(2), z),
            v3(w + dm(4), dm(4), d + dm(4)),
            trim,
        );
        let storey = dm(32);
        let floors = (h - m(2)) / storey;
        let lit = self.c("window");
        for (fi, &on) in faces.iter().enumerate() {
            if !on {
                continue;
            }
            // the face's centre, its outward normal, and its width
            let (n, span) = match fi {
                0 => (v3(ONE, 0, 0), d),
                1 => (v3(-ONE, 0, 0), d),
                2 => (v3(0, 0, ONE), w),
                _ => (v3(0, 0, -ONE), w),
            };
            let half_depth = if fi < 2 { w / 2 } else { d / 2 };
            let col_w = dm(24);
            let cols = (span - m(1)) / col_w;
            if cols <= 0 {
                continue;
            }
            let shop = detail && self.rng.chance(60);
            if shop {
                // a shopfront: a glowing window, an awning and a sign
                let along = if fi < 2 {
                    v3(0, 0, span * 7 / 10)
                } else {
                    v3(span * 7 / 10, 0, 0)
                };
                let at = v3(x, dm(14), z).add(n.scale(half_depth + dm(1)));
                let size = along.add(n.scale(dm(1))).add(v3(0, dm(18), 0));
                let sz = v3(size.x.abs().max(dm(1)), size.y, size.z.abs().max(dm(1)));
                self.glow(Mesh::Cube, at, sz, lit);
                let cloth = self.pick(&[self.c("cloth"), self.c("cloth_2")]);
                let aw_at = v3(x, dm(26), z).add(n.scale(half_depth + dm(6)));
                let aw = along.add(n.scale(dm(12))).add(v3(0, dm(2), 0));
                let aws = v3(aw.x.abs().max(dm(2)), aw.y, aw.z.abs().max(dm(2)));
                self.deco(Mesh::Cube, aw_at, aws, cloth);
                let sign_at = v3(x, dm(31), z).add(n.scale(half_depth + dm(2)));
                self.sign(sign_at, span / 3, dm(7), fi < 2);
            }
            // The windows: one facade panel over the face, its grid of
            // windows drawn by the shader (a lit or dark pane in each cell
            // of `col_w` by `storey`, chosen by a hash of the cell and the
            // panel's seed). One instance where there were seventy: a
            // software renderer's cost is per instance (SECOND-ORDER G7).
            let first = if shop { 1 } else { 0 };
            let rows = floors - first;
            if rows > 0 {
                let along_mid = -span / 2 + m(1) / 2 + col_w * cols / 2;
                let along = if fi < 2 {
                    v3(0, 0, along_mid)
                } else {
                    v3(along_mid, 0, 0)
                };
                let y_mid = m(2) + storey * first + storey * rows / 2;
                let at = v3(x, y_mid, z).add(along).add(n.scale(half_depth));
                let seed = self.rng.range(0, 999) * (ONE / 1000);
                self.facade(at, n, col_w * cols, storey * rows, lit, seed);
            }
            if detail {
                for f in first..floors {
                    let y = m(2) + storey * f + storey / 2;
                    for k in 0..cols {
                        if !self.rng.chance(8) {
                            continue;
                        }
                        // an air-conditioning box under a window
                        let off = -span / 2 + col_w / 2 + m(1) / 2 + col_w * k;
                        let along = if fi < 2 { v3(0, 0, off) } else { v3(off, 0, 0) };
                        let ac = self.c("metal");
                        let below = v3(x, y - dm(11), z).add(along).add(n.scale(half_depth + dm(3)));
                        self.deco(Mesh::Cube, below, v3(dm(7), dm(5), dm(7)), ac);
                    }
                    if f > 0 && self.rng.chance(12) {
                        // a balcony slab
                        let at = v3(x, y - storey / 2 + dm(1), z).add(n.scale(half_depth + dm(5)));
                        let size = if fi < 2 {
                            v3(dm(10), dm(2), span / 3)
                        } else {
                            v3(span / 3, dm(2), dm(10))
                        };
                        self.deco(Mesh::Cube, at, size, trim);
                    }
                }
            }
            if detail && self.rng.chance(50) {
                // a drainpipe down the face
                let off = span / 2 - dm(4);
                let along = if fi < 2 { v3(0, 0, off) } else { v3(off, 0, 0) };
                let base = v3(x, 0, z).add(along).add(n.scale(half_depth + dm(2)));
                let metal = self.c("metal_dark");
                self.rod(base, base.add(v3(0, h, 0)), dm(2), metal, false);
            }
            if detail && self.rng.chance(35) {
                // ivy hanging from the roof
                let g = self.pick(&self.pal.greens.clone());
                let off = dm(self.rng.range(-30, 30)).clamp(-span / 3, span / 3);
                let len = self.metres(3, 9).min(h - m(1));
                let along = if fi < 2 { v3(0, 0, off) } else { v3(off, 0, 0) };
                let at = v3(x, h - len / 2, z).add(along).add(n.scale(half_depth + dm(1)));
                let size = if fi < 2 {
                    v3(dm(2), len, m(2))
                } else {
                    v3(m(2), len, dm(2))
                };
                self.deco(Mesh::Cube, at, size, g);
            }
        }
        self.roof_clutter(x, z, w, d, h);
    }

    fn roof_clutter(&mut self, x: i32, z: i32, w: i32, d: i32, h: i32) {
        let metal = self.c("metal");
        let dark = self.c("metal_dark");
        let wood = self.c("wood");
        let things = self.rng.range(1, 4);
        for _ in 0..things {
            let px = x + dm(self.rng.range(-4, 4)) * (w / m(1)).max(1) / 10;
            let pz = z + dm(self.rng.range(-4, 4)) * (d / m(1)).max(1) / 10;
            match self.rng.range(0, 5) {
                0 | 1 => {
                    // an air-conditioning unit with its fan
                    self.deco(
                        Mesh::Cube,
                        v3(px, h + dm(6), pz),
                        v3(dm(16), dm(12), dm(12)),
                        metal,
                    );
                    self.deco(Mesh::Cyl, v3(px, h + dm(12), pz), v3(dm(9), dm(1), dm(9)), dark);
                }
                2 => {
                    // a water tank on legs with a cone roof
                    for (lx, lz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                        let a = v3(px + lx * dm(8), h, pz + lz * dm(8));
                        self.rod(a, a.add(v3(0, m(2), 0)), dm(1), dark, false);
                    }
                    self.deco(Mesh::Cyl, v3(px, h + m(3), pz), v3(m(2), m(2), m(2)), wood);
                    self.deco(
                        Mesh::Cone,
                        v3(px, h + dm(45), pz),
                        v3(dm(22), dm(9), dm(22)),
                        crate::render::shade(wood, 80),
                    );
                }
                3 => {
                    // an antenna mast with cross-bars
                    let top = h + self.metres(3, 7);
                    self.rod(v3(px, h, pz), v3(px, top, pz), dm(1), dark, false);
                    for k in 1..4 {
                        let y = h + (top - h) * k / 4;
                        self.rod(
                            v3(px - dm(6), y, pz),
                            v3(px + dm(6), y, pz),
                            dm(1) / 2,
                            dark,
                            false,
                        );
                    }
                }
                4 => {
                    // a billboard on posts
                    let col = self.pick(&self.pal.signs.clone());
                    self.rod(
                        v3(px - m(1), h, pz),
                        v3(px - m(1), h + m(3), pz),
                        dm(2),
                        dark,
                        false,
                    );
                    self.rod(
                        v3(px + m(1), h, pz),
                        v3(px + m(1), h + m(3), pz),
                        dm(2),
                        dark,
                        false,
                    );
                    self.deco(
                        Mesh::Cube,
                        v3(px, h + dm(38), pz),
                        v3(m(4), m(2), dm(3)),
                        self.c("ink"),
                    );
                    self.glow(Mesh::Cube, v3(px, h + dm(38), pz), v3(dm(36), dm(16), dm(4)), col);
                }
                _ => {
                    // a little shed
                    let c = self.pick(&self.pal.walls.clone());
                    self.deco(Mesh::Cube, v3(px, h + m(1), pz), v3(m(2), m(2), m(2)), c);
                    self.deco(
                        Mesh::Wedge,
                        v3(px, h + dm(23), pz),
                        v3(dm(22), dm(6), dm(22)),
                        self.c("roof"),
                    );
                }
            }
        }
    }

    /// The facades that wall the arena in, leaving a gap at each side in
    /// `gaps`, and an invisible bound behind them.
    fn perimeter(&mut self, half: i32, gaps: [bool; 4]) {
        let sides = [Side::South, Side::North, Side::East, Side::West];
        for (si, side) in sides.iter().enumerate() {
            let mut along = -half - m(6);
            while along < half + m(6) {
                let w = self.metres(8, 16);
                let mid = along + w / 2;
                along += w;
                if gaps[si] && mid.abs() < GATE_GAP / 2 + w / 2 {
                    continue;
                }
                let depth = self.metres(8, 14);
                let h = self.metres(12, 30);
                let out = half + depth / 2 + dm(5);
                let (x, z, bw, bd, faces) = match side {
                    Side::South => (mid, out, w, depth, [false, false, false, true]),
                    Side::North => (mid, -out, w, depth, [false, false, true, false]),
                    Side::East => (out, mid, depth, w, [false, true, false, false]),
                    Side::West => (-out, mid, depth, w, [true, false, false, false]),
                };
                self.building(x, z, bw, bd, h, faces, true, true);
            }
            // the gate's frame, or a barricade where no gate stands
            let (pos, _) = side.place(half);
            if gaps[si] {
                self.gate_frame(pos, *side);
            } else {
                self.barricade(pos, *side);
            }
        }
        // the bound: past the facades nobody can leave, even over a roof
        let o = half + m(16);
        let t = m(1);
        let top = m(80);
        self.wall(v3(-o - t, -m(2), -o - t), v3(o + t, top, -o));
        self.wall(v3(-o - t, -m(2), o), v3(o + t, top, o + t));
        self.wall(v3(-o - t, -m(2), -o), v3(-o, top, o));
        self.wall(v3(o, -m(2), -o), v3(o + t, top, o));
    }

    fn gate_frame(&mut self, pos: V3, side: Side) {
        let stone = self.c("stone");
        let trim = self.c("gold_trim");
        let across_x = matches!(side, Side::North | Side::South);
        let out = match side {
            Side::South => v3(0, 0, ONE),
            Side::North => v3(0, 0, -ONE),
            Side::East => v3(ONE, 0, 0),
            Side::West => v3(-ONE, 0, 0),
        };
        let at = pos.add(out.scale(dm(5)));
        for s in [-1, 1] {
            let off = if across_x {
                v3(s * m(3), 0, 0)
            } else {
                v3(0, 0, s * m(3))
            };
            let p = at.add(off);
            self.block(p.x, 0, p.z, dm(12), m(6), dm(12), stone);
            self.deco(
                Mesh::Blob,
                v3(p.x, m(6) + dm(5), p.z),
                v3(dm(9), dm(9), dm(9)),
                trim,
            );
        }
        let (lw, ld) = if across_x {
            (m(7) + dm(2), dm(12))
        } else {
            (dm(12), m(7) + dm(2))
        };
        self.deco(Mesh::Cube, v3(at.x, m(6) + dm(2), at.z), v3(lw, dm(8), ld), stone);
        self.deco(
            Mesh::Cube,
            v3(at.x, m(6) - dm(3), at.z),
            v3(lw * 9 / 10, dm(2), ld + dm(1)),
            trim,
        );
        // behind the gate the corridor is closed
        let back = pos.add(out.scale(m(3)));
        let (bw, bd) = if across_x {
            (GATE_GAP + m(2), dm(5))
        } else {
            (dm(5), GATE_GAP + m(2))
        };
        self.solids.push(Solid::stand(back.x, 0, back.z, bw, m(20), bd));
        let wall = self.c("court_dark");
        self.deco(Mesh::Cube, v3(back.x, m(6), back.z), v3(bw, m(12), bd), wall);
    }

    fn barricade(&mut self, pos: V3, side: Side) {
        let across_x = matches!(side, Side::North | Side::South);
        let wall = self.pick(&self.pal.walls.clone());
        let (bw, bd) = if across_x {
            (GATE_GAP + m(4), m(4))
        } else {
            (m(4), GATE_GAP + m(4))
        };
        let out = match side {
            Side::South => v3(0, 0, m(2)),
            Side::North => v3(0, 0, -m(2)),
            Side::East => v3(m(2), 0, 0),
            Side::West => v3(-m(2), 0, 0),
        };
        let at = pos.add(out);
        let h = self.metres(9, 16);
        self.building(at.x, at.z, bw, bd, h, [true, true, true, true], true, false);
        let _ = wall;
        // rubble at its foot
        for _ in 0..5 {
            let r = dm(self.rng.range(4, 10));
            let off = if across_x {
                v3(dm(self.rng.range(-40, 40)), 0, -out.z.signum() * m(1))
            } else {
                v3(-out.x.signum() * m(1), 0, dm(self.rng.range(-40, 40)))
            };
            let c = self.c("paving_dark");
            self.deco(
                Mesh::Octa,
                pos.add(off).add(v3(0, r / 3, 0)),
                v3(r, r * 2 / 3, r),
                c,
            );
        }
    }

    /// The far city: buildings in rings out to the fog, a clock tower
    /// stopped at 21:14, cables, and outlined clouds.
    fn skyline(&mut self, half: i32) {
        let rings = [
            (half + m(22), half + m(40), 26),
            (half + m(45), half + m(85), 34),
            (half + m(90), half + m(170), 30),
        ];
        for (lo, hi, n) in rings {
            for k in 0..n {
                let a = fx::TURN / n * k + deg(self.rng.range(-4, 4));
                let r = lo + dm(self.rng.range(0, (hi - lo) / dm(1)));
                let (x, z) = (fx::mul(fx::cos(a), r), fx::mul(fx::sin(a), r));
                let w = self.metres(10, 22);
                let d = self.metres(10, 22);
                let h = self.metres(18, 46) + (r - lo) / 3;
                let face = [x < 0, x > 0, z < 0, z > 0];
                self.building(x, z, w, d, h, face, false, lo < half + m(30));
                if self.rng.chance(30) {
                    // a setback tower on top
                    let extra = self.metres(8, 24);
                    self.building(x, z, w * 6 / 10, d * 6 / 10, h + extra, face, false, false);
                }
            }
        }
        // cables strung between the nearer roofs
        let ink = self.c("ink");
        for _ in 0..14 {
            let a = deg(self.rng.range(0, 359));
            let b = a + deg(self.rng.range(10, 30));
            let ra = half + self.metres(22, 40);
            let rb = half + self.metres(22, 40);
            let ya = self.metres(10, 20);
            let pa = v3(fx::mul(fx::cos(a), ra), ya, fx::mul(fx::sin(a), ra));
            let pb = v3(fx::mul(fx::cos(b), rb), ya - m(2), fx::mul(fx::sin(b), rb));
            let sag = pa.lerp(pb, ONE / 2).add(v3(0, -m(3), 0));
            self.rod(pa, sag, dm(1), ink, false);
            self.rod(sag, pb, dm(1), ink, false);
        }
        // the clock tower, stopped at 21:14
        let (tx, tz) = (-m(40), -half - m(70));
        let stone = self.c("stone");
        let clock = self.c("clock");
        self.deco(
            Mesh::Cube,
            v3(tx, m(40), tz),
            v3(m(14), m(80), m(14)),
            crate::render::shade(stone, 90),
        );
        self.deco(Mesh::Cube, v3(tx, m(86), tz), v3(m(16), m(12), m(16)), stone);
        self.deco(
            Mesh::Cone,
            v3(tx, m(102), tz),
            v3(m(18), m(20), m(18)),
            self.c("roof"),
        );
        let face = v3(tx, m(86), tz + m(8) + dm(2));
        let disc = Affine::translate(face)
            .then(&Affine::rot_x(fx::QUARTER))
            .then(&Affine::scale(v3(m(10), dm(4), m(10))));
        self.shape(Mesh::Cyl, disc, clock, ONE / 2, true);
        // 21:14: the hour hand a little past nine, the minute hand near three
        for (angle, len, w) in [(-(deg(270) + deg(7)), m(3), dm(5)), (-deg(84), dm(45), dm(3))] {
            let tip = face.add(v3(
                fx::mul(fx::sin(-angle), len),
                fx::mul(fx::cos(-angle), len),
                dm(3),
            ));
            self.rod(face.add(v3(0, 0, dm(3))), tip, w, ink, false);
        }
        // clouds: flattened spheres, outlined
        let cloud = self.c("sky_low");
        for _ in 0..22 {
            let a = deg(self.rng.range(0, 359));
            let r = self.metres(220, 420);
            let c = v3(
                fx::mul(fx::cos(a), r),
                self.metres(70, 130),
                fx::mul(fx::sin(a), r),
            );
            let puffs = self.rng.range(3, 6);
            for p in 0..puffs {
                let s = self.metres(14, 30);
                let off = v3(
                    s * (p - puffs / 2) * 2 / 3,
                    dm(self.rng.range(-40, 40)),
                    dm(self.rng.range(-60, 60)),
                );
                self.shape(
                    Mesh::Blob,
                    boxed(c.add(off), v3(s * 2, s, s * 2)),
                    cloud,
                    ONE / 3,
                    true,
                );
            }
        }
    }

    /// The floor: one slab, paving tiles, puddles and cracks.
    fn floor(&mut self, half: i32, channel: bool) {
        let paving = self.c("paving");
        let ground = self.c("ground");
        let o = half + m(20);
        if channel {
            let (x0, x1) = CANAL;
            let quay = self.c("paving_dark");
            self.solids.push(Solid::new(v3(-o, -m(1), -o), v3(x0, 0, o)));
            self.solids.push(Solid::new(v3(x1, -m(1), -o), v3(o, 0, o)));
            self.solids.push(Solid::new(
                v3(x0, -CANAL_DEPTH - m(1), -o),
                v3(x1, -CANAL_DEPTH, o),
            ));
            self.shape(
                Mesh::Cube,
                boxed(v3((-o + x0) / 2, -dm(5), 0), v3(x0 + o, m(1), o * 2)),
                paving,
                0,
                false,
            );
            self.shape(
                Mesh::Cube,
                boxed(v3((o + x1) / 2, -dm(5), 0), v3(o - x1, m(1), o * 2)),
                paving,
                0,
                false,
            );
            self.shape(
                Mesh::Cube,
                boxed(
                    v3((x0 + x1) / 2, -CANAL_DEPTH - dm(5), 0),
                    v3(x1 - x0, m(1), o * 2),
                ),
                quay,
                0,
                false,
            );
            // the channel's walls
            for x in [x0, x1] {
                self.shape(
                    Mesh::Cube,
                    boxed(v3(x, -CANAL_DEPTH / 2, 0), v3(dm(2), CANAL_DEPTH, o * 2)),
                    crate::render::shade(quay, 85),
                    0,
                    true,
                );
            }
        } else {
            self.solids.push(Solid::new(v3(-o, -m(1), -o), v3(o, 0, o)));
            self.shape(
                Mesh::Cube,
                boxed(v3(0, -dm(5), 0), v3(o * 2, m(1), o * 2)),
                paving,
                0,
                false,
            );
        }
        // a wide ground plane for the city beyond
        self.shape(
            Mesh::Ground,
            boxed(v3(0, -dm(2), 0), v3(m(1200), ONE, m(1200))),
            ground,
            0,
            false,
        );
        let dark = self.c("paving_dark");
        let tile = m(4);
        let n = half * 2 / tile;
        for i in 0..n {
            for j in 0..n {
                let x = -half + tile * i + tile / 2;
                let z = -half + tile * j + tile / 2;
                let in_channel = channel && x > CANAL.0 - tile && x < CANAL.1 + tile;
                if !in_channel && self.rng.chance(22) {
                    let c = if self.rng.chance(50) {
                        dark
                    } else {
                        crate::render::shade(paving, 94)
                    };
                    self.shape(
                        Mesh::Cube,
                        boxed(v3(x, dm(1) / 10, z), v3(tile - dm(2), dm(1) / 5, tile - dm(2))),
                        c,
                        0,
                        false,
                    );
                }
            }
        }
        let water = self.c("water");
        for _ in 0..6 {
            let x = dm(self.rng.range(-(half / dm(1)) + 30, half / dm(1) - 30));
            if channel && x < CANAL.1 + m(4) {
                continue;
            }
            let z = dm(self.rng.range(-(half / dm(1)) + 30, half / dm(1) - 30));
            let r = self.metres(1, 3);
            self.shape(
                Mesh::Cyl,
                boxed(v3(x, dm(1) / 5, z), v3(r * 2, dm(1) / 10, r * 3 / 2)),
                water,
                ONE / 5,
                false,
            );
        }
    }

    fn scatter_props(&mut self, half: i32, count: i32) {
        for _ in 0..count {
            let x = dm(self.rng.range(-(half / dm(1)) + 15, half / dm(1) - 15));
            let z = dm(self.rng.range(-(half / dm(1)) + 15, half / dm(1) - 15));
            // keep the gates' approaches, the middle and any channel clear
            if x.abs() < m(5) || z.abs() < m(5) || (self.canal && x < CANAL.1 + m(2) && x > CANAL.0 - m(2)) {
                continue;
            }
            let probe = Level::new(self.solids.clone());
            if probe.blocked(v3(x - m(2), dm(1), z - m(2)), v3(x + m(2), m(3), z + m(2))) {
                continue;
            }
            match self.rng.range(0, 9) {
                0 | 1 => self.planter(x, 0, z),
                2 => self.lamp(x, 0, z),
                3 => self.crate_stack(x, 0, z),
                4 => self.barrel(x, 0, z),
                5 => {
                    let along = self.rng.chance(50);
                    self.bench(x, 0, z, along);
                }
                6 => {
                    let yaw = deg(self.rng.range(0, 359));
                    self.car(x, 0, z, yaw);
                }
                7 => self.bush(x, 0, z),
                _ => {
                    let c = self.c("paving_dark");
                    let r = dm(self.rng.range(5, 12));
                    self.deco(Mesh::Octa, v3(x, r / 3, z), v3(r, r * 2 / 3, r), c);
                }
            }
        }
    }

    // ---- interiors ------------------------------------------------------

    fn plaza(&mut self) {
        // the fountain in the middle: a basin, a column, water
        let stone = self.c("stone");
        let water = self.c("water");
        let trim = self.c("gold_trim");
        self.solids.push(Solid::stand(0, 0, 0, m(7), dm(8), m(7)));
        self.deco(Mesh::Cyl, v3(0, dm(4), 0), v3(m(8), dm(8), m(8)), stone);
        self.glow(Mesh::Cyl, v3(0, dm(8), 0), v3(m(7), dm(1), m(7)), water);
        self.block(0, 0, 0, dm(14), m(4), dm(14), stone);
        self.deco(Mesh::Blob, v3(0, dm(43), 0), v3(dm(12), dm(12), dm(12)), trim);
        // cover blocks in a broken ring
        for k in 0..6 {
            let a = fx::TURN / 6 * k + deg(30);
            let r = m(14);
            let (x, z) = (fx::mul(fx::cos(a), r), fx::mul(fx::sin(a), r));
            let wall = self.pick(&self.pal.walls.clone());
            self.block(x, 0, z, m(3), dm(16), dm(8), wall);
        }
        // two towers on the north side joined by a bridge you can walk under,
        // and a low terrace in the south-east
        self.platform(-m(19), -m(12), m(8), m(8), m(4), (0, ONE));
        self.platform(m(19), -m(12), m(8), m(8), m(4), (0, ONE));
        self.deck(
            -m(15),
            -m(13) - dm(5),
            m(15),
            -m(10) - dm(5),
            m(4),
            &[(-m(5), -m(12)), (m(5), -m(12))],
        );
        self.platform(m(17), m(12), m(6), m(6), m(2), (-ONE, 0));
        for (x, z) in [(-m(8), -m(20)), (m(8), m(20)), (m(22), -m(2)), (-m(22), m(4))] {
            self.planter(x, 0, z);
        }
    }

    fn market(&mut self) {
        // rows of stalls under awnings
        for row in [-1, 1] {
            for k in -2..=2 {
                if k == 0 {
                    continue;
                }
                let x = k * m(8);
                let z = row * m(9);
                let wood = self.c("wood");
                let cloth = if (k + row) % 2 == 0 {
                    self.c("cloth")
                } else {
                    self.c("cloth_2")
                };
                self.block(x, 0, z, m(4), dm(11), m(2), wood);
                for (px, pz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
                    let a = v3(x + px * dm(19), 0, z + pz * dm(9));
                    self.rod(a, a.add(v3(0, dm(28), 0)), dm(1), wood, false);
                }
                self.deco(Mesh::Wedge, v3(x, dm(30), z), v3(dm(46), dm(5), dm(26)), cloth);
                let n = self.rng.range(1, 3);
                for g in 0..n {
                    let c = self.pick(&self.pal.signs.clone());
                    self.deco(
                        Mesh::Blob,
                        v3(x - dm(10) + dm(10) * g, dm(13), z),
                        v3(dm(4), dm(4), dm(4)),
                        c,
                    );
                }
            }
        }
        // the footbridge across the middle at five metres, stairs at each end
        let stone = self.c("stone");
        let trim = self.c("trim");
        self.block(0, m(5) - dm(5), 0, m(20), dm(5), m(4), stone);
        self.deco(
            Mesh::Cube,
            v3(0, m(5) + dm(5), m(2)),
            v3(m(20), dm(1), dm(1)),
            trim,
        );
        self.deco(
            Mesh::Cube,
            v3(0, m(5) + dm(5), -m(2)),
            v3(m(20), dm(1), dm(1)),
            trim,
        );
        for s in [-1, 1] {
            self.block(
                s * m(9),
                0,
                0,
                m(2),
                m(5) - dm(5),
                m(4),
                crate::render::shade(stone, 85),
            );
            self.stairs(s * (m(10) + dm(78)), 0, 0, -s * ONE, 0, m(5), m(3));
        }
        // a gallery at five metres along the north side, reached from the
        // footbridge by two side walks; the gate path runs under it
        self.deck(
            -m(20),
            -m(17) - dm(5),
            m(20),
            -m(14) - dm(5),
            m(5),
            &[(-m(19), -m(16)), (-m(6), -m(16)), (m(6), -m(16)), (m(19), -m(16))],
        );
        for sx in [-1, 1] {
            self.deck(
                sx * m(8),
                -m(14) - dm(5),
                sx * m(11),
                -m(2),
                m(5),
                &[(sx * m(10), -m(12))],
            );
        }
        // lanterns strung over the stalls
        let lamp = self.c("lamp");
        let ink = self.c("ink");
        for row in [-1, 1] {
            let a = v3(-m(20), m(6), row * m(13));
            let b = v3(m(20), m(6), row * m(13));
            self.rod(a, b, dm(1) / 2, ink, false);
            for k in 0..10 {
                let p = a.lerp(b, ONE / 10 * k + ONE / 20);
                let c = if k % 2 == 0 {
                    lamp
                } else {
                    self.pick(&self.pal.signs.clone())
                };
                self.glow(Mesh::Blob, p.add(v3(0, -dm(4), 0)), v3(dm(5), dm(6), dm(5)), c);
            }
        }
    }

    fn rooftops(&mut self) {
        // the floor is broken into roofs at different heights, stairs between
        let blocks = [
            (-m(15), -m(14), m(14), m(12), m(4)),
            (m(15), -m(14), m(14), m(12), m(4)),
            (-m(15), m(15), m(14), m(10), m(4)),
            (m(15), m(15), m(14), m(10), m(2)),
            (0, 0, m(8), m(8), m(6)),
        ];
        // the north roofs climb from their south sides, clear of the bridge
        let dirs = [(0, ONE), (0, ONE), (ONE, 0), (-ONE, 0), (0, ONE)];
        for (k, (x, z, w, d, h)) in blocks.iter().enumerate() {
            let wall = self.pick(&self.pal.walls.clone());
            self.block(*x, 0, *z, *w, *h, *d, wall);
            let trim = crate::render::shade(wall, 80);
            self.deco(
                Mesh::Cube,
                v3(*x, h + dm(1), *z),
                v3(w + dm(2), dm(2), d + dm(2)),
                trim,
            );
            self.roof_clutter(*x, *z, w / 2, d / 2, *h);
            let (sx, sz) = dirs[k];
            let run = dm(6) * ((h + dm(4) - 1) / dm(4));
            let (ox, oz) = (x + fx::mul(sx, w / 2 + run), z + fx::mul(sz, d / 2 + run));
            self.stairs(ox, 0, oz, -sx, -sz, *h, m(3));
        }
        // a bridge between the two north roofs, over the way to the gate,
        // and a catwalk off the south-west roof that looks over the tower's stair
        self.deck(-m(8), -m(15) - dm(5), m(8), -m(12) - dm(5), m(4), &[]);
        self.deck(-m(8), m(10), -m(2), m(13), m(4), &[(-m(3), m(11) + dm(5))]);
        // parapets on the high roofs to duck behind
        for (x, z) in [(-m(15), -m(9)), (m(15), -m(9))] {
            let wall = self.pick(&self.pal.walls.clone());
            self.block(x, m(4), z, m(6), dm(12), dm(5), wall);
        }
        // skylights and vents on the low roofs
        let glass = self.c("window");
        self.glow(
            Mesh::Cube,
            v3(-m(15), m(2) + dm(1), -m(14)),
            v3(m(3), dm(2), m(2)),
            glass,
        );
        self.glow(
            Mesh::Cube,
            v3(m(15), m(2) + dm(1), m(15)),
            v3(m(2), dm(2), m(3)),
            glass,
        );
    }

    fn canal(&mut self) {
        // the dry canal: a channel 1.2 m down along the west
        // side, bridges over it, stairs into it (the floor is laid in pieces
        // round it by `floor`)
        let water = self.c("water");
        let stone = self.c("stone");
        let (x0, x1) = CANAL;
        let mid = (x0 + x1) / 2;
        let depth = CANAL_DEPTH;
        self.glow(
            Mesh::Cube,
            v3(mid, -depth + dm(2), 0),
            v3(x1 - x0, dm(2), HALF * 2 + m(30)),
            water,
        );
        for z in [-m(13), m(13)] {
            self.block(mid, -dm(4), z, x1 - x0 + m(1), dm(4), m(4), stone);
            for s in [-1, 1] {
                let a = v3(x0, 0, z + s * m(2));
                self.rod(
                    a.add(v3(0, dm(9), 0)),
                    a.add(v3(x1 - x0, dm(9), 0)),
                    dm(1),
                    stone,
                    true,
                );
            }
        }
        // stairs out of the channel on both sides, every so often
        let steps = (depth + dm(4) - 1) / dm(4);
        for z in [-m(20), 0, m(20)] {
            self.stairs(x1 - dm(6) * steps, -depth, z, ONE, 0, depth, m(3));
            self.stairs(x0 + dm(6) * steps, -depth, z + m(10), -ONE, 0, depth, m(3));
        }
        // the quay's edge, boats aground, lamp posts
        self.deco(
            Mesh::Cube,
            v3(x1 + dm(1), dm(1), 0),
            v3(dm(3), dm(2), HALF * 2),
            stone,
        );
        let wood = self.c("wood");
        self.block(mid, -depth, -m(4), dm(18), dm(9), m(5), wood);
        self.block(
            mid + dm(5),
            -depth,
            m(20),
            dm(18),
            dm(9),
            m(5),
            crate::render::shade(wood, 80),
        );
        for k in -2..=2 {
            self.lamp(x1 + m(2), 0, k * m(10) + m(5));
        }
        self.platform(m(16), -m(12), m(8), m(8), m(3), (-ONE, 0));
        // an aqueduct at five metres from the west wall over the channel to
        // a tower in the east; walk under it, or climb it
        self.platform(m(16), m(7), m(6), m(6), m(5), (0, ONE));
        self.deck(
            -m(26),
            m(5) + dm(5),
            m(13),
            m(8) + dm(5),
            m(5),
            &[(-m(20), m(7)), (-m(4), m(7)), (m(5), m(7))],
        );
        self.stairs(-m(24) - dm(5), 0, m(8) + dm(5) + dm(78), 0, -ONE, m(5), m(3));
    }

    fn atrium(&mut self) {
        // a ring of columns round a raised dais; a balcony on two walls
        let stone = self.c("stone");
        let trim = self.c("gold_trim");
        self.block(0, 0, 0, m(10), m(1), m(10), stone);
        self.stairs(0, 0, m(5) + dm(5), 0, -ONE, m(1), m(4));
        self.stairs(0, 0, -m(5) - dm(5), 0, ONE, m(1), m(4));
        for k in 0..10 {
            let a = fx::TURN / 10 * k;
            let r = m(13);
            let (x, z) = (fx::mul(fx::cos(a), r), fx::mul(fx::sin(a), r));
            self.solids.push(Solid::stand(x, 0, z, dm(15), m(9), dm(15)));
            self.deco(Mesh::Cyl, v3(x, dm(45), z), v3(dm(16), m(9), dm(16)), stone);
            self.deco(
                Mesh::Cube,
                v3(x, m(9) + dm(2), z),
                v3(dm(22), dm(4), dm(22)),
                trim,
            );
        }
        {
            let s = -1;
            let x = s * (HALF - m(3));
            self.block(x, m(5), -m(3) + dm(5), m(6), dm(5), m(35), stone);
            self.deco(
                Mesh::Cube,
                v3(x - s * m(3), m(6), -m(3) + dm(5)),
                v3(dm(2), m(1), m(35)),
                trim,
            );
            self.stairs(x, 0, m(15) + dm(78), 0, -ONE, m(5), m(3));
            // a high walk across the north, past the ring of columns and over
            // the way to the north gate, to a tower in the north-east
            self.deck(
                x + m(3),
                -m(19) - dm(5),
                m(19),
                -m(16) - dm(5),
                m(5) + dm(5),
                &[(-m(8), -m(18)), (m(8), -m(18))],
            );
            self.platform(m(22), -m(18), m(6), m(6), m(5) + dm(5), (0, ONE));
            // banners hanging from the balcony
            for k in -2..=2 {
                let c = if k % 2 == 0 {
                    self.c("cloth")
                } else {
                    self.c("court")
                };
                self.deco(
                    Mesh::Cube,
                    v3(x - s * m(3), m(3) + dm(5), k * m(5)),
                    v3(dm(1), m(4), m(2)),
                    c,
                );
            }
        }
    }

    fn crater(&mut self) {
        // where the Defendant landed: a scorched bowl, the pod's wreck, signs
        let crater = self.c("crater");
        let dark = self.c("paving_dark");
        self.shape(
            Mesh::Cyl,
            boxed(v3(m(5), dm(1) / 5, m(8)), v3(m(12), dm(1) / 5, m(12))),
            crater,
            0,
            false,
        );
        for k in 0..12 {
            let a = fx::TURN / 12 * k;
            let r = m(6) + dm(self.rng.range(-5, 5));
            let (x, z) = (m(5) + fx::mul(fx::cos(a), r), m(8) + fx::mul(fx::sin(a), r));
            let s = dm(self.rng.range(6, 12));
            self.deco(Mesh::Octa, v3(x, s / 3, z), v3(s, s * 2 / 3, s), dark);
        }
        let metal = self.c("metal");
        let glow = self.c("shot_launcher");
        // the pod lies off to the side of the walk north
        self.block(m(7), 0, m(9), m(3), dm(14), m(2), metal);
        let tilt = Affine::translate(v3(m(7), dm(20), m(9)))
            .then(&Affine::rot_z(deg(25)))
            .then(&Affine::scale(v3(m(2), m(3), m(2))));
        self.shape(Mesh::Cyl, tilt, metal, 0, true);
        self.glow(Mesh::Blob, v3(m(7), dm(22), m(9)), v3(dm(9), dm(9), dm(9)), glow);
        for (x, z) in [(-m(16), -m(10)), (m(16), -m(14)), (-m(18), m(14)), (m(15), m(16))] {
            self.planter(x, 0, z);
        }
        self.platform(-m(18), -m(2), m(6), m(10), m(3), (ONE, 0));
        // the end of a broken overpass, its stair at the south end
        self.deck(
            m(11),
            -m(12),
            m(14),
            m(2),
            m(4),
            &[(m(12) + dm(5), -m(8)), (m(12) + dm(5), -m(2))],
        );
        self.stairs(m(12) + dm(5), 0, m(2) + dm(60), 0, -ONE, m(4), m(3));
    }

    fn antechamber(&mut self) {
        // a quiet room: the fountain that heals, benches, lamps
        let stone = self.c("stone");
        let water = self.c("water");
        self.solids.push(Solid::stand(0, 0, 0, m(5), dm(7), m(5)));
        self.deco(Mesh::Cyl, v3(0, dm(35) / 10, 0), v3(m(6), dm(7), m(6)), stone);
        self.glow(Mesh::Cyl, v3(0, dm(7), 0), v3(m(5), dm(1), m(5)), water);
        self.glow(
            Mesh::Blob,
            v3(0, m(2), 0),
            v3(dm(8), dm(8), dm(8)),
            self.c("pickup_heal"),
        );
        for k in 0..4 {
            let a = fx::TURN / 4 * k + deg(45);
            let (x, z) = (fx::mul(fx::cos(a), m(8)), fx::mul(fx::sin(a), m(8)));
            self.bench(x, 0, z, k % 2 == 0);
            self.lamp(fx::mul(fx::cos(a), m(12)), 0, fx::mul(fx::sin(a), m(12)));
        }
        for (x, z) in [(-m(18), -m(18)), (m(18), -m(18)), (-m(18), m(18)), (m(18), m(18))] {
            self.planter(x, 0, z);
        }
    }

    /// The Court of Hours: a round floor ringed by jury boxes, pillars to
    /// hide behind, the judge's bench, and a vast stopped clock.
    fn court(&mut self) {
        let court = self.c("court");
        let dark = self.c("court_dark");
        let trim = self.c("gold_trim");
        let clock = self.c("clock");
        let ink = self.c("ink");
        let r = COURT_RADIUS;
        // the round wall, as a ring of tall blocks
        for k in 0..32 {
            let a = fx::TURN / 32 * k;
            let (x, z) = (fx::mul(fx::cos(a), r + m(2)), fx::mul(fx::sin(a), r + m(2)));
            let rot = Affine::translate(v3(x, m(9), z)).then(&Affine::rot_y(fx::QUARTER - a));
            self.solids.push(Solid::stand(x, 0, z, m(5), m(30), m(5)));
            self.shape(
                Mesh::Cube,
                rot.then(&Affine::scale(v3(m(6) + dm(2), m(18), m(3)))),
                if k % 2 == 0 { court } else { dark },
                0,
                true,
            );
            self.shape(
                Mesh::Cube,
                rot.then(&Affine::translate(v3(0, m(9), 0)))
                    .then(&Affine::scale(v3(m(6) + dm(4), dm(5), m(3) + dm(4)))),
                trim,
                0,
                true,
            );
            // tiered jury seats on the inside
            let inner = v3(fx::mul(fx::cos(a), r - m(1)), 0, fx::mul(fx::sin(a), r - m(1)));
            if k % 4 != 0 {
                let rot2 =
                    Affine::translate(inner.add(v3(0, dm(6), 0))).then(&Affine::rot_y(fx::QUARTER - a));
                self.shape(
                    Mesh::Cube,
                    rot2.then(&Affine::scale(v3(m(5), dm(12), m(1)))),
                    crate::render::shade(dark, 90),
                    0,
                    true,
                );
                // a juror: a hooded figure that does not move
                if self.rng.chance(60) {
                    let hood = self.pick(&[ink, dark, court]);
                    let fig = inner.add(v3(0, dm(16), 0));
                    self.deco(Mesh::Cone, fig, v3(dm(8), dm(14), dm(8)), hood);
                    self.glow(
                        Mesh::Blob,
                        fig.add(v3(0, dm(3), 0)),
                        v3(dm(2), dm(1), dm(2)),
                        self.c("enemy_eye"),
                    );
                }
            }
        }
        // the floor: a dark disc with gold rings
        self.shape(
            Mesh::Cyl,
            boxed(v3(0, dm(1) / 5, 0), v3(r * 2, dm(1) / 5, r * 2)),
            dark,
            0,
            false,
        );
        for rr in [m(10), m(20)] {
            self.shape(
                Mesh::Ring,
                boxed(v3(0, dm(1) / 2, 0), v3(rr * 2, dm(1) / 2, rr * 2)),
                trim,
                ONE / 2,
                false,
            );
        }
        // four raised stands between the pillars: a ground wave cannot reach
        // a body standing on one (SECOND-ORDER V2)
        for (sx, sz) in [(1, 1), (-1, 1), (1, -1), (-1, -1)] {
            let (x, z) = (sx * m(16) + sx * dm(3), sz * m(16) + sz * dm(3));
            // the stair comes off the side facing the middle along z, clear
            // of the bridge
            self.platform(x, z, m(5), m(5), m(3), (0, -sz * ONE));
        }
        // the two southern stands are joined by a bridge you can run under
        let zb = m(16) + dm(3);
        self.deck(
            -m(14),
            zb - m(1) - dm(5),
            m(14),
            zb + m(1) + dm(5),
            m(3),
            &[(-m(5), zb), (m(5), zb)],
        );
        // eight pillars for cover
        for k in 0..8 {
            let a = fx::TURN / 8 * k + deg(22);
            // the two by the southern bridge stand further in, clear of it
            // and of the Bailiff's circle
            let r = if k == 1 || k == 2 { m(11) } else { m(17) };
            let (x, z) = (fx::mul(fx::cos(a), r), fx::mul(fx::sin(a), r));
            self.solids.push(Solid::stand(x, 0, z, dm(18), m(6), dm(18)));
            self.deco(Mesh::Cyl, v3(x, m(3), z), v3(m(2), m(6), m(2)), court);
            self.deco(
                Mesh::Cube,
                v3(x, m(6) + dm(2), z),
                v3(dm(26), dm(4), dm(26)),
                trim,
            );
        }
        // the judge's bench, north
        self.block(0, 0, -r + m(4), m(12), m(3), m(3), dark);
        self.deco(
            Mesh::Cube,
            v3(0, m(3) + dm(2), -r + m(4)),
            v3(m(12) + dm(4), dm(4), m(3) + dm(4)),
            trim,
        );
        // the clock over the bench, stopped at 21:14
        let face = v3(0, m(27), -r);
        let disc = Affine::translate(face)
            .then(&Affine::rot_x(fx::QUARTER))
            .then(&Affine::scale(v3(m(18), dm(5), m(18))));
        self.shape(Mesh::Cyl, disc, clock, ONE / 2, true);
        for k in 0..12 {
            let a = fx::TURN / 12 * k;
            let p = face.add(v3(fx::mul(fx::sin(a), m(8)), fx::mul(fx::cos(a), m(8)), dm(3)));
            self.deco(Mesh::Cube, p, v3(dm(6), dm(6), dm(2)), ink);
        }
        for (angle, len, w) in [(deg(277), m(5), dm(7)), (deg(84), m(7), dm(4))] {
            let tip = face.add(v3(
                fx::mul(fx::sin(angle), len),
                fx::mul(fx::cos(angle), len),
                dm(4),
            ));
            self.rod(face.add(v3(0, 0, dm(4))), tip, w, ink, false);
        }
        // the bound
        self.wall(v3(-r - m(10), -m(2), -r - m(10)), v3(r + m(10), m(80), -r - m(4)));
        self.wall(v3(-r - m(10), -m(2), r + m(4)), v3(r + m(10), m(80), r + m(10)));
        self.wall(v3(-r - m(10), -m(2), -r - m(10)), v3(-r - m(4), m(80), r + m(10)));
        self.wall(v3(r + m(4), -m(2), -r - m(10)), v3(r + m(10), m(80), r + m(10)));
    }
}

pub fn boxed(at: V3, size: V3) -> Affine {
    Affine::translate(at).then(&Affine::scale(size))
}

/// A unit shape stretched from `a` to `b`, `w` across.
pub fn segment(a: V3, b: V3, w: i32) -> Option<Affine> {
    let dir = b.sub(a);
    let len = dir.len();
    if len == 0 {
        return None;
    }
    let y = dir.scale(fx::div(ONE, len));
    let helper = if y.y.abs() > ONE - ONE / 8 {
        v3(ONE, 0, 0)
    } else {
        V3::UP
    };
    let x = helper.cross(y).norm();
    let z = x.cross(y);
    Some(Affine {
        c0: x.scale(w),
        c1: y.scale(len),
        c2: z.scale(w),
        t: a.lerp(b, ONE / 2),
    })
}

/// Builds the arena for an archetype. `exits` are the sides with a way on;
/// the player enters from the south.
pub fn build(kind: &str, seed: u64, exits: &[Side], pal: &Palette) -> Arena {
    let mut b = B {
        solids: Vec::new(),
        shapes: Vec::new(),
        rng: Rng::new(seed),
        pal,
        canal: kind == "canal",
    };
    let is_court = kind == "boss";
    if is_court {
        b.court();
        b.shape(
            Mesh::Ground,
            boxed(v3(0, -dm(2), 0), v3(m(1200), ONE, m(1200))),
            pal.c("ground"),
            0,
            false,
        );
        b.solids
            .push(Solid::new(v3(-m(60), -m(1), -m(60)), v3(m(60), 0, m(60))));
        b.skyline(COURT_RADIUS + m(6));
    } else {
        b.floor(HALF, kind == "canal");
        let gaps = [
            true,
            exits.contains(&Side::North),
            exits.contains(&Side::East),
            exits.contains(&Side::West),
        ];
        b.perimeter(HALF, gaps);
        match kind {
            "arrival" => b.crater(),
            "rest" => b.antechamber(),
            "market" => b.market(),
            "rooftops" => b.rooftops(),
            "canal" => b.canal(),
            "atrium" => b.atrium(),
            _ => b.plaza(),
        }
        if matches!(kind, "plaza" | "market" | "rooftops" | "canal" | "atrium") {
            b.cover_walls(HALF, 5);
        }
        let props = if matches!(kind, "rest" | "arrival") {
            10
        } else {
            24
        };
        b.scatter_props(HALF, props);
        b.skyline(HALF);
    }
    let level = Level::new(b.solids.clone());
    // spawn points: a grid over the floor, kept where a body fits
    let mut ground_spawns = Vec::new();
    let mut air_spawns = Vec::new();
    let reach = if is_court {
        COURT_RADIUS - m(4)
    } else {
        HALF - m(4)
    };
    let mut z = -reach;
    while z <= reach {
        let mut x = -reach;
        while x <= reach {
            let near_entry = z > reach - m(10) && x.abs() < m(10);
            if !near_entry && (!is_court || v3(x, 0, z).len_xz() < reach) {
                let top = level.floor_below(x, m(12), z).unwrap_or(0);
                let p = v3(x, top, z);
                if !level.blocked(
                    v3(x - m(1), top + dm(1), z - m(1)),
                    v3(x + m(1), top + m(3), z + m(1)),
                ) {
                    // nothing stands on a walkway's narrow slab: a turret
                    // there could only be reached by climbing (SECOND-ORDER V3)
                    let on_deck = level.solids.iter().any(|s| {
                        // a walkway or balcony: a thin slab held off the floor
                        s.max.y == top
                            && s.max.y - s.min.y <= ONE * 6 / 10
                            && s.min.y > int(2)
                            && x >= s.min.x
                            && x <= s.max.x
                            && z >= s.min.z
                            && z <= s.max.z
                    });
                    if !on_deck {
                        ground_spawns.push(p);
                    }
                    // a floater spawned inside a bridge would be stuck there
                    let air = p.add(v3(0, m(4), 0));
                    if !level.sphere_hits(air, m(2)) {
                        air_spawns.push(air);
                    }
                }
            }
            x += m(6);
        }
        z += m(6);
    }
    let (start, gates) = if is_court {
        (v3(0, 0, COURT_RADIUS - m(8)), Vec::new())
    } else {
        let mut gates = vec![Side::South.place(HALF)];
        for s in exits {
            gates.push(s.place(HALF));
        }
        (v3(0, 0, HALF - m(5)), gates)
    };
    Arena {
        level,
        shapes: b.shapes,
        start,
        start_yaw: 0,
        ground_spawns,
        air_spawns,
        gates,
        centre: if is_court {
            v3(0, m(7), -m(4))
        } else {
            v3(0, m(4), 0)
        },
        fountain: if kind == "rest" { Some(v3(0, 0, 0)) } else { None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;

    #[test]
    fn every_archetype_builds_with_room_to_spawn_and_a_clear_start() {
        let c = content();
        for kind in [
            "arrival", "rest", "plaza", "market", "rooftops", "canal", "atrium", "boss",
        ] {
            let a = build(kind, 7, &[Side::North, Side::East], &c.pal);
            assert!(
                a.ground_spawns.len() > 30,
                "{kind}: {} spawns",
                a.ground_spawns.len()
            );
            let s = a.start;
            assert!(
                !a.level.blocked(
                    v3(s.x - dm(4), s.y + dm(1), s.z - dm(4)),
                    v3(s.x + dm(4), s.y + m(2), s.z + dm(4))
                ),
                "{kind}: the start is inside a solid"
            );
            // thousands of shapes: the city is detailed
            assert!(a.shapes.len() > 1500, "{kind}: {} shapes", a.shapes.len());
        }
    }

    #[test]
    fn the_same_seed_builds_the_same_arena_and_another_seed_does_not() {
        let c = content();
        let a = build("plaza", 3, &[Side::North], &c.pal);
        let b = build("plaza", 3, &[Side::North], &c.pal);
        let d = build("plaza", 4, &[Side::North], &c.pal);
        assert_eq!(a, b);
        assert_ne!(a.shapes, d.shapes);
    }

    #[test]
    fn a_gate_on_each_exit_side_and_one_behind_the_player() {
        let c = content();
        let a = build("market", 1, &[Side::North, Side::East], &c.pal);
        assert_eq!(a.gates.len(), 3);
        let b = build("market", 1, &[Side::North], &c.pal);
        assert_eq!(b.gates.len(), 2);
        // the gate's floor is open to walk to
        for (g, _) in &a.gates {
            let inside = g.scale(ONE * 9 / 10);
            assert!(
                !a.level.blocked(
                    v3(inside.x - dm(4), dm(1), inside.z - dm(4)),
                    v3(inside.x + dm(4), m(2), inside.z + dm(4))
                ),
                "{g:?}"
            );
        }
    }
}

#[cfg(test)]
mod size {
    use super::*;
    use crate::content::tests::content;

    #[test]
    fn an_arena_is_drawn_in_a_few_tens_of_thousands_of_instances() {
        // what the page uploads once a scene; a GPU of a school laptop draws
        // this many boxes, each with its ink, comfortably
        let c = content();
        for kind in ["plaza", "market", "boss"] {
            let a = build(kind, 9, &[Side::North, Side::East], &c.pal);
            let mut b = crate::render::Batch::new([0; 3]);
            for s in &a.shapes {
                b.shape(s);
            }
            let n = b.count();
            assert!(n < 16_000, "{kind}: {n}");
        }
    }
}

#[cfg(test)]
mod reach {
    use super::*;
    use crate::balance::{PLAYER_HEIGHT, PLAYER_RADIUS, STEP_UP};
    use crate::content::tests::content;
    use std::collections::BTreeSet;

    const GRID: i32 = ONE / 2;

    /// Can a body stand with its feet at `p`?
    fn fits(l: &Level, p: V3) -> bool {
        let r = PLAYER_RADIUS;
        !l.blocked(
            v3(p.x - r, p.y + ONE / 20, p.z - r),
            v3(p.x + r, p.y + PLAYER_HEIGHT, p.z + r),
        )
    }

    /// Every place a body can walk to from the start without jumping: a
    /// step onto anything up to `STEP_UP` higher, a drop onto anything
    /// lower. Returns the feet positions reached.
    fn walk(a: &Arena) -> Vec<V3> {
        let l = &a.level;
        let snap = |v: i32| (v / GRID) * GRID;
        let start = v3(snap(a.start.x), a.start.y, snap(a.start.z));
        let mut seen: BTreeSet<(i32, i32, i32)> = BTreeSet::new();
        let mut todo = vec![start];
        let mut out = Vec::new();
        seen.insert((start.x, start.y, start.z));
        while let Some(p) = todo.pop() {
            out.push(p);
            for (dx, dz) in [(GRID, 0), (-GRID, 0), (0, GRID), (0, -GRID)] {
                let (nx, nz) = (p.x + dx, p.z + dz);
                if nx.abs() > int(40) || nz.abs() > int(40) {
                    continue;
                }
                // as the game does: the body stands on the highest top under
                // its footprint that it can step up to
                let r = PLAYER_RADIUS;
                let (lo, hi) = (v3(nx - r, -int(10), nz - r), v3(nx + r, p.y + STEP_UP, nz + r));
                let mut top: Option<i32> = None;
                l.each_near(lo, hi, |s| {
                    let over = s.min.x < hi.x && s.max.x > lo.x && s.min.z < hi.z && s.max.z > lo.z;
                    if over && s.max.y <= p.y + STEP_UP {
                        top = Some(top.map_or(s.max.y, |t: i32| t.max(s.max.y)));
                    }
                });
                let Some(y) = top else {
                    continue;
                };
                let n = v3(nx, y, nz);
                if y < p.y - int(6) || !fits(l, n) {
                    continue;
                }
                if seen.insert((n.x, n.y, n.z)) {
                    todo.push(n);
                }
            }
        }
        out
    }

    #[test]
    fn every_gate_can_be_walked_to_and_every_fight_has_heights_and_shelter() {
        let c = content();
        for kind in [
            "arrival", "rest", "plaza", "market", "rooftops", "canal", "atrium", "boss",
        ] {
            // cover walls are placed at random, so many seeds
            for seed in 1..=20 {
                let a = build(kind, seed, &[Side::North, Side::East], &c.pal);
                let reached = walk(&a);
                for (g, _) in a.gates.iter().skip(1) {
                    let near = reached
                        .iter()
                        .any(|p| p.sub(*g).len_xz() < int(2) && p.y.abs() < ONE);
                    assert!(near, "{kind} seed {seed}: the gate at {g:?} cannot be walked to");
                }
                if matches!(kind, "arrival" | "rest") {
                    continue;
                }
                let high = reached.iter().map(|p| p.y).max().unwrap_or(0);
                assert!(
                    high >= int(3),
                    "{kind} seed {seed}: nothing 3 m up can be climbed to ({high})"
                );
                // a place on the ground with a solid overhead within 6 m
                let under = reached.iter().any(|p| {
                    p.y <= ONE
                        && a.level
                            .ray(p.add(v3(0, PLAYER_HEIGHT + ONE / 10, 0)), V3::UP, int(5))
                            .is_some()
                });
                assert!(under, "{kind} seed {seed}: nowhere to walk under");
            }
        }
    }

    #[test]
    fn nothing_solid_stands_through_a_walkway() {
        // a pillar or a stair through a deck would block the walk along it
        // or the climb to it; the reachability walk cannot see that when
        // another way up exists
        let c = content();
        for kind in [
            "arrival", "plaza", "market", "rooftops", "canal", "atrium", "boss",
        ] {
            let a = build(kind, 2, &[Side::North, Side::East], &c.pal);
            let decks: Vec<&Solid> = a
                .level
                .solids
                .iter()
                .filter(|s| s.max.y - s.min.y == DECK && s.min.y > int(2))
                .collect();
            assert!(!decks.is_empty(), "{kind}: no decks");
            for d in &decks {
                for o in &a.level.solids {
                    if std::ptr::eq(*d, o) {
                        continue;
                    }
                    let horizontal =
                        o.min.x < d.max.x && o.max.x > d.min.x && o.min.z < d.max.z && o.max.z > d.min.z;
                    // something from below that rises above the walking
                    // surface (a stair or post that ends flush with it is
                    // how a deck is reached or held up)
                    let pierces = o.min.y < d.min.y && o.max.y > d.max.y + ONE / 20;
                    // the arena's invisible bound is tall and far out: skip it
                    let bound = o.max.y >= int(60);
                    assert!(
                        !(horizontal && pierces && !bound),
                        "{kind}: {o:?} rises through the deck {d:?}"
                    );
                }
            }
        }
    }
}
