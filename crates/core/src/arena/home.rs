//! The last mission's own zone, home: your bedroom (arrival), the landing
//! at the top of the stairs (rest) and downstairs (the boss: Chumbles).
//!
//! The house is drawn larger than life, so a third-person camera has room:
//! upstairs at about one and a half times, downstairs at twice and more,
//! because reality is broken and the house has come back bigger than you
//! remember. Sizes here are centimetres, `[x, y, z]` as a floor plan has
//! them: x east, y up, z south.
//!
//! - Your Bedroom: the bed unmade, a nightstand with the lamp and the alarm
//!   clock, the desk under the east window, the bookshelf, the wardrobe,
//!   the cat's bed; the door north stands open on the hall
//! - The Landing: a runner down the middle, the family's photos, the hall
//!   table, shut doors, and the stairs going down between the banisters;
//!   Chumbles' water fountain stands where zone one keeps its fountain
//! - Downstairs: open plan; the kitchen north (counters, the island, the
//!   fridge, the treat bag, the back door), the dining table east, the
//!   living room west (sofa, TV, coffee table, the divider shelf), the cat
//!   tree, the stairs down the south wall behind you. The middle is open
//!   for his charges and leaps; the high places he sits on are `perches`.

use super::{boxed, Side, B};
use crate::content::{cm, Rgb};
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine, V3};
use crate::level::Solid;
use crate::mesh::Mesh;
use crate::render::shade;

/// Half the bedroom's and the landing's depth, north to south: their gates
/// stand here.
const ROOM: i32 = fx::int(10);
/// The upstairs walls' inner face (cm from the middle, north and south):
/// just past the gate, whose panel stands inside the wall, so only the
/// doorway shows it.
const FACE: i32 = 1035;
/// The upstairs ceiling (cm).
const UP_H: i32 = 700;
/// Half the bedroom door's width, and its height (cm).
const DOOR_W: i32 = 140;
const DOOR_H: i32 = 420;
/// Downstairs: the ceiling, and the walls' inner face either side (cm).
const DOWN_H: i32 = 1400;
const DOWN_W: i32 = 3600;

/// A point or a size in centimetres.
type Cm = [i32; 3];

fn p(a: Cm) -> V3 {
    v3(cm(a[0]), cm(a[1]), cm(a[2]))
}

/// The low and high corners of the box between two opposite corners.
fn span(a: Cm, b: Cm) -> (Cm, Cm) {
    (
        [a[0].min(b[0]), a[1].min(b[1]), a[2].min(b[2])],
        [a[0].max(b[0]), a[1].max(b[1]), a[2].max(b[2])],
    )
}

/// A wall's inner face: across x (an east or west wall, `x_axis`) or across
/// z, `at` cm from the middle, the room on its `inward` side (+1 or −1).
#[derive(Clone, Copy)]
struct Wall {
    x_axis: bool,
    at: i32,
    inward: i32,
}

impl Wall {
    /// `along` the wall (z on an east or west wall, x otherwise), `y` up,
    /// `out` from the face into the room.
    fn pt(self, along: i32, y: i32, out: i32) -> Cm {
        let n = self.at + self.inward * out;
        if self.x_axis {
            [n, y, along]
        } else {
            [along, y, n]
        }
    }

    fn size(self, w: i32, h: i32, t: i32) -> Cm {
        if self.x_axis {
            [t, h, w]
        } else {
            [w, h, t]
        }
    }

    /// Turns a disc's axis (y) to face out of the wall.
    fn turn(self) -> Affine {
        if self.x_axis {
            Affine::rot_z(fx::QUARTER)
        } else {
            Affine::rot_x(fx::QUARTER)
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl B<'_> {
    pub(super) fn home(&mut self, kind: &str, exits: &[Side]) {
        // the bedroom and the landing go on north only; downstairs is the end
        let _ = exits;
        match kind {
            "boss" => self.home_downstairs(),
            "arrival" => {
                self.half = ROOM;
                self.home_bedroom();
            }
            _ => {
                self.half = ROOM;
                self.home_landing();
            }
        }
    }

    // ---- the builder's hands, in centimetres ------------------------------

    fn h_shape(&mut self, mesh: Mesh, a: Cm, b: Cm, rgb: Rgb, glow: i32, ink: bool) {
        let (lo, hi) = span(a, b);
        let (lo, hi) = (p(lo), p(hi));
        self.shape(mesh, boxed(lo.lerp(hi, ONE / 2), hi.sub(lo)), rgb, glow, ink);
    }

    /// A box between two corners, drawn and inked.
    fn h_box(&mut self, a: Cm, b: Cm, rgb: Rgb) {
        self.h_shape(Mesh::Cube, a, b, rgb, 0, true);
    }

    /// A box with no ink: floor boards, rugs, the ceiling.
    fn h_flat(&mut self, a: Cm, b: Cm, rgb: Rgb) {
        self.h_shape(Mesh::Cube, a, b, rgb, 0, false);
    }

    fn h_lit(&mut self, a: Cm, b: Cm, rgb: Rgb, glow: i32) {
        self.h_shape(Mesh::Cube, a, b, rgb, glow, false);
    }

    fn h_solid(&mut self, a: Cm, b: Cm) {
        let (lo, hi) = span(a, b);
        self.solids.push(Solid::new(p(lo), p(hi)));
    }

    /// A solid box, drawn.
    fn h_block(&mut self, a: Cm, b: Cm, rgb: Rgb) {
        self.h_solid(a, b);
        self.h_box(a, b, rgb);
    }

    /// A shape from its centre and size, inked.
    fn h_at(&mut self, mesh: Mesh, c: Cm, s: Cm, rgb: Rgb) {
        self.shape(mesh, boxed(p(c), p(s)), rgb, 0, true);
    }

    fn h_flat_at(&mut self, mesh: Mesh, c: Cm, s: Cm, rgb: Rgb) {
        self.shape(mesh, boxed(p(c), p(s)), rgb, 0, false);
    }

    fn h_glow_at(&mut self, mesh: Mesh, c: Cm, s: Cm, rgb: Rgb, glow: i32) {
        self.shape(mesh, boxed(p(c), p(s)), rgb, glow, false);
    }

    fn h_rod(&mut self, a: Cm, b: Cm, w: i32, rgb: Rgb) {
        self.rod(p(a), p(b), cm(w), rgb, true);
    }

    /// A shape in a turned frame `f` (centre and size in it, cm).
    fn h_part(&mut self, f: &Affine, mesh: Mesh, c: Cm, s: Cm, rgb: Rgb) {
        self.shape(mesh, f.then(&boxed(p(c), p(s))), rgb, 0, true);
    }

    /// A box on a wall: `w` along it, `h` high round `y`, its front `out`
    /// from the face and `t` deep behind that.
    fn h_on(
        &mut self,
        wall: Wall,
        along: i32,
        y: i32,
        w: i32,
        h: i32,
        out: i32,
        t: i32,
        rgb: Rgb,
        glow: i32,
    ) {
        let c = wall.pt(along, y, out - t / 2);
        self.shape(
            Mesh::Cube,
            boxed(p(c), p(wall.size(w, h, t))),
            rgb,
            glow,
            glow == 0,
        );
    }

    /// A disc on a wall, facing out of it: a clock, a plate, a painted sun.
    fn h_disc(&mut self, wall: Wall, along: i32, y: i32, d: i32, out: i32, rgb: Rgb, glow: i32) {
        let f = Affine::translate(p(wall.pt(along, y, out))).then(&wall.turn());
        self.shape(
            Mesh::Cyl,
            f.then(&Affine::scale(p([d, 3, d]))),
            rgb,
            glow,
            glow == 0,
        );
    }

    /// Sunlight falling from `from` to `to`, `w` wide along `across`.
    fn h_beam(&mut self, from: Cm, to: Cm, across: V3, w: i32, thick: i32) {
        let (a, b) = (p(from), p(to));
        let dir = b.sub(a);
        let af = Affine {
            c0: across.cross(dir).norm().scale(cm(thick)),
            c1: dir,
            c2: across.scale(cm(w)),
            t: a.lerp(b, ONE / 2),
        };
        let sun = self.c("home_window");
        self.halo(Mesh::Cube, af, sun, ONE / 7);
    }

    // ---- the house's furniture --------------------------------------------

    /// A wood floor, its top at 0, over x0..x1 and z0..z1, boards `lane`
    /// wide running north to south and a joint across each about every
    /// `joint`; `hole` (x0, z0, x1, z1) left open.
    fn h_boards(
        &mut self,
        x0: i32,
        z0: i32,
        x1: i32,
        z1: i32,
        lane: i32,
        joint: i32,
        hole: Option<[i32; 4]>,
    ) {
        let wood = self.c("home_floor");
        let alt = shade(wood, 93);
        let seam = self.c("home_floor_dark");
        let rects: Vec<[i32; 4]> = match hole {
            None => vec![[x0, z0, x1, z1]],
            Some([a, b, c, d]) => vec![[x0, z0, x1, b], [x0, d, x1, z1], [x0, b, a, d], [c, b, x1, d]],
        };
        let rects: Vec<[i32; 4]> = rects.into_iter().filter(|r| r[2] > r[0] && r[3] > r[1]).collect();
        for r in &rects {
            self.h_solid([r[0], -100, r[1]], [r[2], 0, r[3]]);
            self.h_flat([r[0], -100, r[1]], [r[2], 0, r[3]], wood);
        }
        let mut x = x0;
        let mut k = 0;
        while x < x1 {
            let xe = (x + lane).min(x1);
            for r in &rects {
                let (a, b) = (x.max(r[0]), xe.min(r[2]));
                if b <= a || r[3] <= r[1] {
                    continue;
                }
                if k % 2 == 1 {
                    // a centimetre in from the lane's ends and sides: a
                    // board's side in a wall's face flickers
                    self.h_flat([a + 1, 0, r[1] + 1], [b - 1, 1, r[3] - 1], alt);
                }
                let mut z = r[1] + self.rng.range(joint / 4, joint);
                while z < r[3] - 30 {
                    self.h_flat([a + 2, 0, z], [b - 2, 2, z + 3], seam);
                    z += self.rng.range(joint * 3 / 4, joint * 5 / 4);
                }
            }
            x = xe;
            k += 1;
        }
    }

    /// An oval rug: `rings` bands from the edge in.
    fn h_rug(&mut self, x: i32, z: i32, w: i32, d: i32, rings: &[Rgb], oval: bool) {
        let mesh = if oval { Mesh::Cyl } else { Mesh::Cube };
        for (k, &rgb) in rings.iter().enumerate() {
            let k = k as i32;
            let inset = k * 40;
            self.h_flat_at(
                mesh,
                [x, 2 + k, z],
                [w - inset * 2, 2 + k * 2, d - inset * 2],
                rgb,
            );
        }
    }

    /// A window on a wall: sunlit panes in a frame, `cols` by `rows`, a
    /// sill, and curtains when `curtain` is given.
    fn h_window(
        &mut self,
        wall: Wall,
        along: i32,
        y: i32,
        w: i32,
        h: i32,
        cols: i32,
        rows: i32,
        curtain: Option<Rgb>,
    ) {
        let trim = self.c("home_trim");
        let sun = self.c("home_window");
        self.h_on(wall, along, y, w + 24, h + 24, 4, 4, trim, 0);
        self.h_on(wall, along, y, w, h, 6, 2, sun, ONE);
        for c in 1..cols {
            let a = along - w / 2 + w * c / cols;
            self.h_on(wall, a, y, 8, h, 10, 6, trim, 0);
        }
        for r in 1..rows {
            let yy = y - h / 2 + h * r / rows;
            self.h_on(wall, along, yy, w, 8, 10, 6, trim, 0);
        }
        for s in [-1, 1] {
            self.h_on(wall, along + s * (w / 2 + 6), y, 12, h + 24, 12, 8, trim, 0);
            self.h_on(wall, along, y + s * (h / 2 + 6), w + 24, 12, 12, 8, trim, 0);
        }
        self.h_on(wall, along, y - h / 2 - 16, w + 50, 10, 22, 22, trim, 0);
        if let Some(cloth) = curtain {
            let top = y + h / 2 + 50;
            let dark = self.c("home_wood_dark");
            self.h_on(wall, along, top, w + 200, 6, 26, 6, dark, 0);
            for s in [-1, 1] {
                let len = h + 120;
                self.h_on(
                    wall,
                    along + s * (w / 2 + 50),
                    top - len / 2,
                    80,
                    len,
                    30,
                    14,
                    cloth,
                    0,
                );
                // the tieback's gather
                self.h_on(
                    wall,
                    along + s * (w / 2 + 50),
                    top - len * 6 / 10,
                    96,
                    14,
                    34,
                    22,
                    shade(cloth, 85),
                    0,
                );
            }
        }
    }

    /// A framed picture: a canvas in a frame, and on it a sun over a hill
    /// (0), stripes (1), or the family (2, a photograph).
    fn h_picture(&mut self, wall: Wall, along: i32, y: i32, w: i32, h: i32, art: i32) {
        let frame = if art == 2 {
            self.c("home_wood_dark")
        } else {
            self.c("home_trim")
        };
        let cols = [
            self.c("home_sofa"),
            self.c("home_rug"),
            self.c("home_rug_2"),
            self.c("home_bed"),
            self.c("home_plant"),
            self.c("home_cabinet"),
        ];
        let ground = cols[self.rng.range(0, 5) as usize];
        self.h_on(wall, along, y, w, h, 5, 5, frame, 0);
        let (iw, ih) = (w - w / 6, h - h / 6);
        match art {
            0 => {
                self.h_on(wall, along, y, iw, ih, 7, 2, self.c("home_sheet"), 0);
                self.h_on(wall, along, y - ih / 4, iw, ih / 2, 8, 1, ground, 0);
                let sun = self.c("home_rug_2");
                self.h_disc(wall, along + iw / 5, y + ih / 6, ih / 3, 8, sun, 0);
            }
            1 => {
                self.h_on(wall, along, y, iw, ih, 7, 2, ground, 0);
                for k in 0..3 {
                    let c = cols[((k + 2) % 6) as usize];
                    self.h_on(
                        wall,
                        along - iw / 3 + iw * k / 3,
                        y,
                        iw / 8,
                        ih * 8 / 10,
                        8,
                        1,
                        c,
                        0,
                    );
                }
            }
            _ => {
                self.h_on(wall, along, y, iw, ih, 7, 2, self.c("home_sky"), 0);
                let skin = self.c("agent_skin");
                let n = self.rng.range(1, 3);
                for k in 0..n {
                    let a = along - iw / 4 * (n - 1) + iw / 2 * k;
                    let shirt = cols[self.rng.range(0, 5) as usize];
                    self.h_on(wall, a, y - ih / 4, iw / 4, ih / 2, 8, 1, shirt, 0);
                    self.h_disc(wall, a, y + ih / 8, ih / 4, 9, skin, 0);
                }
            }
        }
    }

    /// A shut door on a wall: its frame, two panels and a knob.
    fn h_door(&mut self, wall: Wall, along: i32, w: i32, h: i32, rgb: Rgb) {
        let trim = self.c("home_trim");
        let brass = self.c("house_brass");
        for s in [-1, 1] {
            self.h_on(
                wall,
                along + s * (w / 2 + 8),
                (h + 12) / 2,
                16,
                h + 12,
                6,
                6,
                trim,
                0,
            );
        }
        self.h_on(wall, along, h + 6, w + 32, 12, 6, 6, trim, 0);
        self.h_on(wall, along, h / 2, w, h, 3, 3, rgb, 0);
        let panel = shade(rgb, 92);
        self.h_on(wall, along, h * 3 / 4, w * 7 / 10, h * 4 / 10, 5, 2, panel, 0);
        self.h_on(wall, along, h / 4 + 5, w * 7 / 10, h * 4 / 10, 5, 2, panel, 0);
        let knob = wall.pt(along + w / 2 - 22, h * 4 / 10, 10);
        self.h_at(Mesh::Sphere, knob, [11, 11, 11], brass);
    }

    /// Books standing on a shelf whose front is `wall`, from `from` to `to`
    /// along it, on `y`, up to `h` tall, `d` deep; `scale` widens them.
    fn h_books(&mut self, wall: Wall, from: i32, to: i32, y: i32, h: i32, d: i32, scale: i32) {
        let cols = [
            self.c("home_bed"),
            self.c("home_rug"),
            self.c("home_sofa"),
            self.c("house_book"),
            self.c("house_book_2"),
            self.c("house_book_3"),
            self.c("home_rug_2"),
            self.c("home_plant"),
            self.c("home_sheet"),
        ];
        let mut a = from + self.rng.range(0, 15);
        while a < to - 20 {
            let w = (self.rng.range(15, 45) * scale / 100).min(to - a);
            let bh = h * self.rng.range(65, 95) / 100;
            let c = cols[self.rng.range(0, cols.len() as i32 - 1) as usize];
            self.h_on(wall, a + w / 2, y + bh / 2, w - 2, bh, -3, d, c, 0);
            a += w;
            if self.rng.chance(12) {
                a += self.rng.range(15, 40) * scale / 100;
            }
        }
    }

    /// A pot plant, `s` percent of a metre-high one.
    fn h_plant(&mut self, x: i32, y: i32, z: i32, s: i32) {
        let pot = self.c("home_pot");
        let leaf = self.c("home_plant");
        let k = |v: i32| v * s / 100;
        self.h_at(Mesh::Cyl, [x, y + k(20), z], [k(40), k(40), k(40)], pot);
        self.h_at(Mesh::Cyl, [x, y + k(41), z], [k(44), k(4), k(44)], shade(pot, 85));
        for i in 0..5 {
            let a = fx::TURN / 5 * i + self.rng.range(0, 3000);
            let r = k(18);
            let (dx, dz) = (
                fx::mul(fx::cos(a), r * ONE) / ONE,
                fx::mul(fx::sin(a), r * ONE) / ONE,
            );
            let c = if i % 2 == 0 { leaf } else { shade(leaf, 82) };
            let h = k(50 + self.rng.range(0, 40));
            self.h_at(
                Mesh::Blob,
                [x + dx, y + k(40) + h / 2, z + dz],
                [k(34), h, k(34)],
                c,
            );
        }
    }

    /// A mug on `y`.
    fn h_mug(&mut self, x: i32, y: i32, z: i32, rgb: Rgb, s: i32) {
        let k = |v: i32| v * s / 100;
        self.h_at(Mesh::Cyl, [x, y + k(6), z], [k(9), k(12), k(9)], rgb);
        self.h_box(
            [x + k(4), y + k(3), z - k(1)],
            [x + k(7), y + k(9), z + k(1)],
            rgb,
        );
    }

    /// A cat toy mouse on the floor.
    fn h_mouse(&mut self, x: i32, z: i32, s: i32) {
        let grey = self.c("home_steel");
        let pink = self.c("heart");
        let k = |v: i32| v * s / 100;
        self.h_at(Mesh::Blob, [x, k(5), z], [k(22), k(12), k(12)], grey);
        self.h_at(Mesh::Sphere, [x + k(8), k(9), z], [k(5), k(5), k(5)], pink);
        self.h_rod([x - k(11), k(2), z], [x - k(28), 1, z + k(8)], 1, pink);
    }

    /// A yarn ball on the floor and a loose strand.
    fn h_yarn(&mut self, x: i32, z: i32, d: i32) {
        let yarn = self.c("yarn");
        self.h_at(Mesh::Sphere, [x, d / 2, z], [d, d, d], yarn);
        self.h_at(Mesh::Ring, [x, d / 2, z], [d + 2, d / 5, d + 2], shade(yarn, 80));
        self.h_rod([x + d / 2, 2, z], [x + d * 3, 1, z + d * 2], 1, yarn);
    }

    /// A ceiling light: a glowing dome.
    fn h_ceiling_light(&mut self, x: i32, top: i32, z: i32, d: i32) {
        let trim = self.c("home_trim");
        let lamp = self.c("lamp");
        self.h_at(Mesh::Cyl, [x, top - 4, z], [d + 20, 8, d + 20], trim);
        self.h_glow_at(Mesh::Blob, [x, top - 8, z], [d, d / 3, d], lamp, ONE);
    }

    /// A pendant lamp hanging from `top` to `y`: a cord and a glowing shade.
    fn h_pendant(&mut self, x: i32, top: i32, y: i32, z: i32, d: i32, shade_rgb: Rgb) {
        let dark = self.c("home_ink");
        let lamp = self.c("lamp");
        self.h_rod([x, top, z], [x, y + d / 2, z], 2, dark);
        self.shape(
            Mesh::Cone,
            boxed(p([x, y + d / 4, z]), p([d, d / 2, d])),
            shade_rgb,
            ONE / 3,
            true,
        );
        self.h_glow_at(Mesh::Blob, [x, y, z], [d / 3, d / 4, d / 3], lamp, ONE);
    }

    // ---- upstairs -------------------------------------------------------------

    /// An upstairs room: the floor, four walls `hx` east and west and at
    /// `FACE` north and south, the ceiling. The north wall has an opening
    /// `door` either side of the middle from `low` up to `top`, and the
    /// walls go down to `deep`; `hole` is left out of the floor.
    fn h_upstairs(
        &mut self,
        hx: i32,
        paper: Rgb,
        accent: Rgb,
        door: i32,
        low: i32,
        top: i32,
        deep: i32,
        hole: Option<[i32; 4]>,
    ) {
        let o = hx + 100;
        let zo = FACE + 100;
        self.h_boards(-o, -zo, o, zo, 80, 380, hole);
        let t = 100;
        let n = -FACE;
        let mut north = vec![
            ([-o, deep, n - t], [-door, UP_H, n]),
            ([door, deep, n - t], [o, UP_H, n]),
            ([-door, top, n - t], [door, UP_H, n]),
        ];
        if low > deep {
            north.push(([-door, deep, n - t], [door, low, n]));
        }
        for (a, b) in north {
            self.h_block(a, b, paper);
        }
        self.h_block([-o, deep, FACE], [o, UP_H, FACE + t], paper);
        self.h_block([-hx - t, deep, -zo], [-hx, UP_H, zo], accent);
        self.h_block([hx, deep, -zo], [hx + t, UP_H, zo], paper);
        // the ceiling
        let trim = self.c("home_trim");
        self.h_solid([-o, UP_H, -zo], [o, UP_H + 100, zo]);
        self.h_lit([-o, UP_H, -zo], [o, UP_H + 100, zo], shade(trim, 97), ONE / 3);
        // skirting and cornice round the walls
        let walls = [
            Wall {
                x_axis: false,
                at: -FACE,
                inward: 1,
            },
            Wall {
                x_axis: false,
                at: FACE,
                inward: -1,
            },
            Wall {
                x_axis: true,
                at: -hx,
                inward: 1,
            },
            Wall {
                x_axis: true,
                at: hx,
                inward: -1,
            },
        ];
        for (k, w) in walls.iter().enumerate() {
            let len = if w.x_axis { FACE * 2 } else { hx * 2 };
            if k == 0 {
                let side = (hx - door) / 2;
                for s in [-1, 1] {
                    self.h_on(*w, s * (door + side), 12, side * 2, 24, 4, 4, trim, 0);
                }
            } else {
                self.h_on(*w, 0, 12, len, 24, 4, 4, trim, 0);
            }
            self.h_on(*w, 0, UP_H - 8, len, 16, 10, 10, trim, 0);
        }
    }

    /// Your bedroom on a Saturday morning: the sun through the east window,
    /// the bed unmade, and the door open on the hall.
    fn home_bedroom(&mut self) {
        let hx = 900;
        let paper = self.c("home_wall");
        let accent = self.c("home_wall_2");
        self.h_upstairs(hx, paper, accent, DOOR_W, 0, DOOR_H, 0, None);
        let north = Wall {
            x_axis: false,
            at: -FACE,
            inward: 1,
        };
        let south = Wall {
            x_axis: false,
            at: FACE,
            inward: -1,
        };
        let west = Wall {
            x_axis: true,
            at: -hx,
            inward: 1,
        };
        let east = Wall {
            x_axis: true,
            at: hx,
            inward: -1,
        };
        let wood = self.c("home_wood");
        let dark = self.c("home_wood_dark");
        let trim = self.c("home_trim");
        let sheet = self.c("home_sheet");
        let duvet = self.c("home_bed");
        let rug = self.c("home_rug");
        let rug2 = self.c("home_rug_2");
        let brass = self.c("house_brass");
        let screen = self.c("home_screen");
        let steel = self.c("home_steel");
        let lamp = self.c("lamp");

        // the door: its frame, and the door swung open into the room
        // a centimetre clear of the opening's sides and top: in their
        // planes, they flickered
        for s in [-1, 1] {
            self.h_on(
                north,
                s * (DOOR_W + 10),
                (DOOR_H + 14) / 2,
                18,
                DOOR_H + 14,
                6,
                8,
                trim,
                0,
            );
        }
        self.h_on(north, 0, DOOR_H + 6, DOOR_W * 2 + 38, 14, 6, 8, trim, 0);
        let leaf = DOOR_W * 2 - 10;
        self.h_block([DOOR_W, 0, -FACE], [DOOR_W + 10, DOOR_H - 4, -FACE + leaf], trim);
        for x in [DOOR_W - 2, DOOR_W + 12] {
            for (y0, y1) in [(30, 190), (230, 390)] {
                self.h_box(
                    [x - 1, y0, -FACE + 25],
                    [x + 1, y1, -FACE + leaf - 25],
                    shade(trim, 92),
                );
            }
            self.h_at(
                Mesh::Sphere,
                [x + (x - DOOR_W - 5) / 2, 170, -FACE + leaf - 22],
                [10, 10, 10],
                brass,
            );
        }
        // a light switch by it
        self.h_on(north, -DOOR_W - 50, 160, 12, 18, 2, 2, trim, 0);
        // the hall beyond: its floor, the far wall and a picture
        self.h_block([-300, -100, -1600], [300, 0, -FACE - 100], self.c("home_floor"));
        self.h_flat([-90, 0, -1600], [90, 2, -FACE - 100], rug);
        self.h_block([-300, 0, -1700], [300, UP_H, -1600], accent);
        let hall = Wall {
            x_axis: false,
            at: -1600,
            inward: 1,
        };
        self.h_picture(hall, 0, 260, 140, 100, 0);

        // the bed, its head to the west wall: unmade
        let (bx0, bx1, bz0, bz1) = (-hx, -560, -340, -60);
        self.h_solid([bx0, 0, bz0], [bx1, 100, bz1]);
        self.h_box([bx0 + 30, 15, bz0], [bx1, 55, bz1], wood);
        for z in [bz0 + 12, bz1 - 12] {
            self.h_box([bx1 - 22, 0, z - 10], [bx1 - 2, 15, z + 10], dark);
        }
        self.h_flat([bx0 + 30, 55, bz0 + 6], [bx1 - 6, 95, bz1 - 6], sheet);
        self.h_block([bx0, 0, bz0 - 15], [bx0 + 30, 210, bz1 + 15], dark);
        let top = Affine::translate(p([bx0 + 15, 210, (bz0 + bz1) / 2])).then(&Affine::rot_x(fx::QUARTER));
        self.h_part(&top, Mesh::Cyl, [0, 0, 0], [30, bz1 - bz0 + 30, 30], dark);
        // the duvet thrown back toward the foot, hanging down the sides
        self.h_box([-770, 95, bz0 - 6], [bx1 + 12, 116, bz1 + 6], duvet);
        self.h_box([-770, 48, bz0 - 9], [bx1 + 12, 116, bz0 - 3], duvet);
        self.h_box([-770, 48, bz1 + 3], [bx1 + 12, 116, bz1 + 9], duvet);
        self.h_box([bx1 + 6, 40, bz0 - 6], [bx1 + 14, 116, bz1 + 6], duvet);
        self.h_at(Mesh::Blob, [-760, 122, -195], [70, 46, 300], duvet);
        self.h_at(Mesh::Blob, [-660, 120, -125], [120, 34, 100], shade(duvet, 90));
        // a white sheet where you slept, the pillows, a cushion knocked over
        self.h_at(Mesh::Blob, [-838, 112, -268], [66, 34, 104], sheet);
        self.h_at(Mesh::Blob, [-832, 110, -134], [66, 32, 104], shade(sheet, 96));
        self.h_at(Mesh::Blob, [-800, 104, -60], [40, 40, 40], rug2);
        // a throw at the foot, and the phone left on the bed
        self.h_box([-640, 116, bz0 - 4], [-585, 123, bz1 + 4], rug2);
        self.h_box([-640, 123, -260], [-585, 124, -240], rug);
        self.h_box([-720, 96, -110], [-706, 99, -82], screen);
        // the nightstands either side
        for (z0, z1) in [(-485, -375), (-35, 75)] {
            self.h_block([-hx, 0, z0], [-hx + 100, 105, z1], wood);
            self.h_box([-hx + 100, 58, z0 + 8], [-hx + 103, 96, z1 - 8], shade(wood, 88));
            self.h_at(Mesh::Sphere, [-hx + 106, 77, (z0 + z1) / 2], [7, 7, 7], brass);
        }
        // the lamp and the alarm clock on the north one
        self.h_at(Mesh::Cyl, [-850, 116, -440], [26, 22, 26], brass);
        self.h_rod([-850, 125, -440], [-850, 160, -440], 3, brass);
        self.shape(
            Mesh::Cone,
            boxed(p([-850, 180, -440]), p([56, 44, 56])),
            rug2,
            ONE * 2 / 3,
            true,
        );
        self.h_glow_at(Mesh::Blob, [-850, 168, -440], [80, 50, 80], lamp, ONE / 3);
        self.h_box([-842, 105, -408], [-822, 125, -384], screen);
        self.h_lit([-821, 110, -404], [-819, 121, -388], self.c("yarn"), ONE);
        // a glass of water, a book and the phone's charger on the other
        self.h_glow_at(Mesh::Cyl, [-860, 117, 40], [10, 24, 10], self.c("water"), ONE / 3);
        self.h_box([-870, 105, -20], [-820, 110, 25], self.c("house_book_2"));
        self.h_box([-866, 110, -16], [-826, 114, 20], self.c("house_book"));
        self.h_rod([-830, 105, 50], [-hx + 2, 60, 60], 1, steel);
        // slippers by the bed
        for z in [-30, 2] {
            self.h_at(Mesh::Blob, [-500, 5, z], [36, 10, 16], rug2);
        }
        // a sock on the floor
        self.h_at(Mesh::Blob, [-330, 3, 220], [30, 6, 12], self.c("home_sofa"));

        // the rug
        self.h_rug(-150, -150, 720, 500, &[rug, rug2, rug], true);

        // the desk under the window: the monitor, the laptop, a mug
        let (dx0, dz0, dz1) = (760, -620, -220);
        self.h_solid([dx0, 0, dz0], [hx, 125, dz1]);
        self.h_box([dx0, 115, dz0], [hx, 125, dz1], wood);
        self.h_box([dx0 + 10, 0, dz0 + 6], [hx, 115, dz0 + 120], dark);
        for k in 0..3 {
            let y = 18 + k * 32;
            self.h_box(
                [dx0 + 8, y, dz0 + 16],
                [dx0 + 10, y + 26, dz0 + 110],
                shade(dark, 120),
            );
        }
        self.h_box([dx0 + 10, 0, dz1 - 12], [dx0 + 22, 115, dz1], dark);
        self.h_box([hx - 14, 0, dz1 - 12], [hx, 115, dz1], dark);
        // the monitor
        self.h_box([850, 125, -500], [880, 128, -440], steel);
        self.h_box([862, 128, -476], [870, 175, -464], steel);
        self.h_box([848, 165, -555], [858, 250, -385], screen);
        self.h_lit([846, 170, -548], [847, 245, -392], self.c("home_sofa"), ONE / 3);
        // the laptop open, its lid tipped back
        self.h_box([790, 125, -340], [850, 128, -260], steel);
        let lid = Affine::translate(p([852, 150, -300])).then(&Affine::rot_z(-deg(15)));
        self.h_part(&lid, Mesh::Cube, [0, 0, 0], [3, 48, 80], steel);
        self.shape(
            Mesh::Cube,
            lid.then(&boxed(p([-2, 0, 0]), p([1, 40, 70]))),
            self.c("home_bed"),
            ONE / 2,
            false,
        );
        self.h_mug(800, 125, -580, rug, 100);
        // a pen pot and papers
        self.h_at(Mesh::Cyl, [880, 135, -230], [12, 20, 12], self.c("home_cabinet"));
        let papers = Affine::translate(p([800, 126, -420])).then(&Affine::rot_y(deg(12)));
        self.h_part(&papers, Mesh::Cube, [0, 0, 0], [40, 1, 30], sheet);
        // the chair, a hoodie over its back and jeans on the seat
        let chair = self.c("home_sofa_dark");
        self.h_solid([620, 0, -450], [720, 95, -350]);
        self.h_box([660, 0, -455], [680, 6, -345], dark);
        self.h_box([615, 0, -410], [725, 6, -390], dark);
        self.h_rod([670, 6, -400], [670, 80, -400], 6, steel);
        self.h_at(Mesh::Cyl, [670, 88, -400], [100, 14, 100], chair);
        self.h_block([614, 95, -445], [628, 200, -355], chair);
        self.h_box([606, 130, -452], [636, 205, -348], rug);
        self.h_box([606, 70, -440], [612, 140, -420], rug);
        self.h_box([640, 95, -430], [700, 101, -370], duvet);
        self.h_rod([700, 98, -420], [735, 20, -430], 8, duvet);

        // the window over the desk, the sun coming in
        self.h_window(east, -420, 340, 480, 300, 2, 2, Some(rug2));
        self.h_beam([880, 340, -420], [350, 0, -380], v3(0, 0, ONE), 460, 30);
        // the sun's patch over the boards' seams (2 cm), not level with them
        self.h_lit([300, 0, -620], [620, 4, -200], self.c("home_window"), ONE / 4);
        // and a window north, by the door, a radiator under it
        self.h_window(north, -560, 360, 320, 280, 2, 2, Some(self.c("home_bed")));
        self.h_on(north, -560, 90, 300, 110, 14, 10, trim, 0);
        for k in 0..8 {
            self.h_on(north, -700 + k * 40, 90, 6, 100, 18, 4, shade(trim, 92), 0);
        }
        self.h_plant(-800, 0, -900, 130);

        // the dresser east of the door: a mirror over it, its odds and ends
        self.h_block([360, 0, -FACE], [760, 160, -915], wood);
        for k in 0..3 {
            let y = 25 + k * 45;
            self.h_box([380, y, -915], [740, y + 38, -913], shade(wood, 90));
            for x in [480, 640] {
                self.h_at(Mesh::Sphere, [x, y + 19, -910], [7, 7, 7], brass);
            }
        }
        self.h_on(north, 560, 300, 200, 200, 4, 4, dark, 0);
        self.h_on(north, 560, 300, 170, 170, 5, 1, steel, ONE / 5);
        self.h_at(Mesh::Cyl, [420, 166, -970], [30, 12, 30], self.c("home_counter"));
        self.h_glow_at(
            Mesh::Cyl,
            [460, 172, -975],
            [10, 24, 10],
            self.c("heart"),
            ONE / 4,
        );
        self.h_box([620, 160, -985], [680, 210, -975], dark);
        self.h_box([626, 166, -976], [674, 204, -974], self.c("home_sky"));
        self.h_plant(720, 160, -960, 40);

        // the bookshelf, books and a cat figure on it
        let (sx0, sz0, sz1) = (810, 180, 560);
        self.h_solid([sx0, 0, sz0], [hx, 360, sz1]);
        self.h_box([sx0, 0, sz0], [hx, 8, sz1], dark);
        self.h_box([sx0, 352, sz0], [hx, 360, sz1], dark);
        for z in [sz0, sz1 - 6] {
            self.h_box([sx0, 0, z], [hx, 360, z + 6], dark);
        }
        // the back a centimetre inside the case all round: its edges in
        // the sides' and top's faces flickered
        self.h_box([hx - 4, 1, sz0 + 1], [hx, 359, sz1 - 1], shade(dark, 120));
        let front = Wall {
            x_axis: true,
            at: sx0,
            inward: -1,
        };
        for k in 0..4 {
            let y = 8 + k * 86;
            if k > 0 {
                self.h_box([sx0, y - 6, sz0], [hx, y, sz1], dark);
            }
            self.h_books(front, sz0 + 10, sz1 - 10, y, 70, 60, 100);
        }
        self.h_plant(850, 360, 240, 50);
        self.h_at(Mesh::Blob, [850, 375, 480], [26, 30, 22], self.c("cat_cream"));

        // the wardrobe in the south-west corner, boxes on top
        self.h_block([-880, 0, 910], [-460, 440, FACE], wood);
        self.h_box([-672, 10, 908], [-668, 430, 910], shade(wood, 80));
        for x in [-690, -650] {
            self.h_box([x - 3, 180, 902], [x + 3, 260, 910], brass);
        }
        self.h_box([-860, 440, 930], [-700, 520, 1020], self.c("treat"));
        self.h_box([-680, 440, 950], [-560, 490, 1020], self.c("home_rug_2"));
        // the laundry basket, the clothes heaped in it
        self.h_at(Mesh::Cyl, [-320, 40, 920], [100, 80, 100], self.c("home_cattree"));
        for (dx, dz, c) in [(-20, 0, rug), (20, 10, duvet), (0, -20, sheet)] {
            self.h_at(Mesh::Blob, [-320 + dx, 85, 920 + dz], [60, 30, 50], c);
        }
        // a poster on the south wall, frames over the bed
        self.h_picture(south, 380, 330, 220, 300, 1);
        self.h_picture(west, -200, 360, 260, 160, 0);
        self.h_picture(east, 0, 300, 90, 120, 2);
        self.h_picture(east, -10, 160, 60, 50, 2);
        // the cat's bed by the bookshelf, and his toys
        self.h_at(Mesh::Ring, [600, 18, 750], [150, 36, 150], rug);
        self.h_at(Mesh::Cyl, [600, 8, 750], [140, 16, 140], self.c("home_rug_2"));
        self.h_at(Mesh::Blob, [600, 18, 750], [100, 12, 100], sheet);
        self.h_yarn(430, 620, 30);
        self.h_mouse(180, -620, 100);
        self.h_mouse(-420, 500, 100);
        // by the way in: a long mirror, hooks with a backpack and a scarf,
        // the guitar leant on the wall
        self.h_on(south, -150, 220, 110, 380, 5, 5, wood, 0);
        self.h_on(south, -150, 220, 90, 360, 6, 1, steel, ONE / 5);
        self.h_on(south, 160, 300, 160, 12, 8, 8, dark, 0);
        self.h_at(
            Mesh::Blob,
            [130, 240, FACE - 25],
            [60, 90, 40],
            self.c("home_cabinet"),
        );
        self.h_box([180, 200, FACE - 14], [200, 300, FACE - 8], rug);
        let guitar = Affine::translate(p([300, 0, FACE - 55])).then(&Affine::rot_x(deg(12)));
        self.h_part(
            &guitar,
            Mesh::Sphere,
            [0, 45, 0],
            [60, 70, 20],
            self.c("home_pot"),
        );
        self.h_part(
            &guitar,
            Mesh::Sphere,
            [0, 92, 0],
            [48, 50, 20],
            self.c("home_pot"),
        );
        self.h_part(&guitar, Mesh::Cyl, [0, 70, 0], [16, 22, 22], dark);
        self.h_part(&guitar, Mesh::Cube, [0, 160, 0], [8, 110, 5], dark);
        // a shelf over the cat's bed: plants and a little speaker
        self.h_box([440, 260, FACE - 40], [760, 270, FACE], wood);
        self.h_plant(500, 270, FACE - 20, 35);
        self.h_plant(700, 270, FACE - 20, 30);
        self.h_box([580, 270, FACE - 30], [640, 310, FACE - 5], screen);
        // fairy lights strung over the bed
        let lamp_rgb = self.c("lamp");
        let n = 14;
        let mut prev: Option<Cm> = None;
        for k in 0..=n {
            let z = -520 + 640 * k / n;
            let t = k * 2 - n;
            let y = 300 - (n * n - t * t) * 40 / (n * n);
            let at = [-hx + 6, y, z];
            if let Some(a) = prev {
                self.h_rod(a, at, 1, dark);
            }
            self.h_glow_at(
                Mesh::Blob,
                [at[0] + 4, at[1] - 6, at[2]],
                [10, 12, 10],
                lamp_rgb,
                ONE,
            );
            prev = Some(at);
        }
        // a corkboard with notes and a calendar by the bed
        self.h_on(west, 420, 260, 220, 150, 4, 4, self.c("home_cattree"), 0);
        for (dz, dy, c) in [
            (-70, 30, "home_sheet"),
            (-10, -20, "home_rug_2"),
            (60, 25, "heart"),
            (40, -40, "home_bed"),
            (-60, -40, "home_sheet"),
        ] {
            self.h_on(west, 420 + dz, 260 + dy, 44, 36, 6, 1, self.c(c), 0);
        }
        self.h_on(west, 700, 250, 80, 110, 3, 3, sheet, 0);
        self.h_on(west, 700, 285, 80, 30, 4, 1, rug, 0);
        // on the desk: a lamp, headphones, books, a plant, sticky notes
        self.h_at(Mesh::Cyl, [870, 128, -260], [24, 6, 24], dark);
        self.h_rod([870, 128, -260], [830, 200, -260], 3, dark);
        self.h_rod([830, 200, -260], [800, 190, -270], 3, dark);
        self.shape(
            Mesh::Cone,
            boxed(p([795, 182, -272]), p([24, 18, 24])),
            rug2,
            ONE / 2,
            true,
        );
        self.h_at(Mesh::Ring, [820, 132, -560], [30, 10, 36], self.c("home_ink"));
        self.h_box([860, 125, -620], [895, 140, -560], self.c("house_book"));
        self.h_box([862, 140, -615], [893, 150, -565], self.c("home_plant"));
        self.h_plant(880, 125, -200, 22);
        for (dz, dy, c) in [
            (-540, 230, "home_rug_2"),
            (-520, 200, "heart"),
            (-400, 240, "home_rug_2"),
        ] {
            // a centimetre in front of the screen's picture, not in it
            self.h_box([845, dy, dz], [846, dy + 10, dz + 10], self.c(c));
        }
        // a beanbag, a t-shirt on the floor, a box under the bed
        self.h_at(
            Mesh::Blob,
            [-150, 40, 860],
            [150, 80, 140],
            self.c("home_cabinet"),
        );
        self.h_at(
            Mesh::Blob,
            [-150, 70, 880],
            [110, 40, 90],
            shade(self.c("home_cabinet"), 90),
        );
        let tee = Affine::translate(p([-380, 1, -620])).then(&Affine::rot_y(deg(25)));
        self.h_part(&tee, Mesh::Cube, [0, 1, 0], [60, 2, 50], self.c("home_rug_2"));
        self.h_part(&tee, Mesh::Cube, [0, 1, -32], [90, 2, 20], self.c("home_rug_2"));
        self.h_box([-610, 0, -330], [-530, 30, -220], self.c("treat"));
        // a smoke alarm and the ceiling light
        self.h_at(Mesh::Cyl, [300, UP_H - 3, 500], [24, 6, 24], trim);
        self.h_ceiling_light(0, UP_H, -100, 120);
    }

    /// The landing at the top of the stairs: the runner, the photos, the
    /// shut doors, and the stairs going down north past the banisters.
    fn home_landing(&mut self) {
        let hx = 700;
        let paper = self.c("home_wall_2");
        let accent = self.c("home_wall_2");
        // the stairwell: open in the floor from here north, under the wall
        let well = 200;
        let lip = -700;
        self.h_upstairs(
            hx,
            paper,
            accent,
            well,
            -700,
            460,
            -700,
            // the floor's edge two centimetres back from the north wall's
            // end and the well's walls (one plane flickers)
            Some([-well - 2, -FACE - 100, well + 2, lip]),
        );
        let north = Wall {
            x_axis: false,
            at: -FACE,
            inward: 1,
        };
        let south = Wall {
            x_axis: false,
            at: FACE,
            inward: -1,
        };
        let west = Wall {
            x_axis: true,
            at: -hx,
            inward: 1,
        };
        let east = Wall {
            x_axis: true,
            at: hx,
            inward: -1,
        };
        let wood = self.c("home_wood");
        let dark = self.c("home_wood_dark");
        let trim = self.c("home_trim");
        let rug = self.c("home_rug");
        let rug2 = self.c("home_rug_2");
        let carpet = self.c("home_carpet");
        let brass = self.c("house_brass");
        let wall = self.c("home_wall");

        // the stairs down: twelve steps, then the half-landing at the foot
        let steps = 12;
        for k in 0..steps {
            let (z0, z1) = (lip - 80 * (k + 1), lip - 80 * k);
            let top = -40 * (k + 1);
            self.h_solid([-well, -800, z0], [well, top, z1]);
            self.h_box([-well, top - 40, z0], [well, top, z1], trim);
            self.h_flat([-well + 40, top, z0], [well - 40, top + 2, z1 + 2], carpet);
        }
        let foot = lip - 80 * steps;
        let low = -40 * steps;
        self.h_block([-300, -800, foot - 300], [300, low, foot], self.c("home_floor"));
        // the stairwell's walls, its ceiling, and the light from downstairs
        for s in [-1, 1] {
            // a centimetre back from the opening in the floor and the north
            // wall: their ends lay in this wall's face and flickered
            let (x0, x1) = if s < 0 { (-300, -well - 1) } else { (well + 1, 300) };
            self.h_block([x0, -800, foot - 400], [x1, UP_H, -FACE - 1], wall);
            let x = s * (well - 6);
            self.h_rod([x, 90, lip], [x, low + 90, foot], 5, wood);
        }
        self.h_block([-300, -800, foot - 400], [300, UP_H, foot - 300], wall);
        self.h_flat(
            [-300, 461, foot - 400],
            [300, UP_H - 1, -FACE - 1],
            shade(trim, 97),
        );
        let far = Wall {
            x_axis: false,
            at: foot - 300,
            inward: 1,
        };
        self.h_window(far, 0, low + 260, 200, 260, 2, 2, None);
        // the banisters round the top of the stairs
        for s in [-1, 1] {
            let x = s * (well + 5);
            self.h_solid([x - 5, 0, -FACE], [x + 5, 110, lip]);
            self.h_rod([x, 105, lip], [x, 105, -FACE], 8, wood);
            self.h_box([x - 10, 0, lip - 10], [x + 10, 130, lip + 10], dark);
            self.h_at(Mesh::Sphere, [x, 138, lip], [20, 20, 20], dark);
            let mut z = lip - 35;
            while z > -FACE {
                self.h_rod([x, 0, z], [x, 100, z], 4, trim);
                z -= 35;
            }
        }
        // the runner down the middle, and Chumbles' fountain on its mat
        self.h_rug(0, 100, 220, 1500, &[rug2, rug], false);
        self.h_flat([-110, 0, -600], [110, 2, -560], rug2);
        self.h_flat_at(Mesh::Cyl, [0, 5, 0], [180, 2, 130], self.c("home_bed"));
        self.h_at(Mesh::Cyl, [0, 16, 0], [80, 26, 80], trim);
        self.h_glow_at(Mesh::Cyl, [0, 29, 0], [64, 2, 64], self.c("water"), ONE / 2);
        self.h_at(Mesh::Cyl, [0, 40, 0], [16, 24, 16], self.c("home_steel"));
        self.h_glow_at(Mesh::Blob, [0, 55, 0], [22, 16, 22], self.c("water"), ONE / 2);
        self.h_at(Mesh::Cyl, [62, 6, 20], [36, 12, 36], self.c("home_rug"));
        self.h_at(Mesh::Blob, [62, 12, 20], [26, 6, 26], self.c("treat"));

        // the doors: the bathroom west, the spare room east, the airing
        // cupboard south
        self.h_door(west, -250, 160, 400, trim);
        self.h_door(east, -320, 160, 400, trim);
        self.h_door(south, -420, 140, 380, wood);
        // the photos of the family up the west wall
        for (k, (z, y, w, h)) in [
            (250, 300, 100, 80),
            (380, 340, 70, 90),
            (500, 280, 120, 90),
            (300, 180, 60, 70),
            (430, 200, 90, 70),
            (620, 360, 70, 70),
            (700, 220, 80, 100),
        ]
        .into_iter()
        .enumerate()
        {
            self.h_picture(west, z, y, w, h, if k == 3 { 0 } else { 2 });
        }
        for (x, y) in [(-560, 330), (-380, 260), (440, 260)] {
            self.h_picture(north, x, y, 110, 90, 2);
        }
        // the hall table, a mirror over it, the keys, a lamp and flowers
        self.h_block([610, 0, 180], [hx, 120, 520], wood);
        self.h_box([620, 30, 190], [hx, 36, 510], shade(wood, 85));
        self.h_on(east, 350, 280, 200, 160, 4, 4, brass, 0);
        self.h_on(east, 350, 280, 180, 140, 5, 1, self.c("home_steel"), ONE / 5);
        self.h_at(Mesh::Cyl, [650, 130, 240], [36, 20, 36], self.c("home_counter"));
        self.h_box([640, 140, 238], [660, 143, 244], brass);
        self.h_at(Mesh::Cyl, [655, 134, 460], [24, 28, 24], self.c("home_pot"));
        self.h_rod([655, 148, 460], [655, 190, 460], 3, brass);
        self.shape(
            Mesh::Cone,
            boxed(p([655, 205, 460]), p([46, 34, 46])),
            rug2,
            ONE * 2 / 3,
            true,
        );
        self.h_at(Mesh::Cyl, [655, 140, 350], [18, 40, 18], self.c("water"));
        for (dx, dz, c) in [
            (0, 0, self.c("heart")),
            (8, 10, self.c("home_rug_2")),
            (-8, 6, self.c("yarn")),
        ] {
            self.h_at(Mesh::Sphere, [655 + dx, 172, 350 + dz], [16, 16, 16], c);
        }
        // a basket of letters under the table
        self.h_at(Mesh::Cyl, [655, 22, 350], [60, 44, 60], self.c("home_cattree"));
        // the window at the south end, the sun on the floor, a radiator
        self.h_window(south, 220, 360, 280, 320, 2, 2, Some(self.c("home_sofa")));
        self.h_on(south, 220, 90, 260, 100, 14, 10, trim, 0);
        self.h_lit([60, 0, 560], [380, 4, 900], self.c("home_window"), ONE / 4);
        self.h_beam([220, 360, FACE - 10], [220, 0, 600], v3(ONE, 0, 0), 260, 30);
        self.h_plant(-600, 0, 900, 140);
        self.h_plant(600, 0, -900, 120);
        // a laundry hamper, the cat's scratching post and a toy
        self.h_at(Mesh::Cyl, [-600, 45, 650], [70, 90, 70], self.c("home_cattree"));
        self.h_at(
            Mesh::Cyl,
            [-600, 92, 650],
            [76, 8, 76],
            shade(self.c("home_cattree"), 80),
        );
        let rope = self.c("home_cattree");
        self.h_block([420, 0, -880], [520, 8, -780], carpet);
        self.h_at(Mesh::Cyl, [470, 100, -830], [26, 190, 26], rope);
        self.h_at(Mesh::Cube, [470, 196, -830], [50, 6, 50], carpet);
        self.h_rod([490, 190, -830], [500, 120, -830], 1, brass);
        self.h_at(Mesh::Sphere, [500, 116, -830], [10, 10, 10], self.c("heart"));
        self.h_yarn(-300, -400, 28);
        self.h_mouse(260, 200, 100);
        // a bench by the stairs with cushions, shoes by the bathroom door
        self.h_block([-660, 0, -FACE], [-300, 55, -FACE + 90], wood);
        self.h_at(
            Mesh::Blob,
            [-560, 70, -FACE + 45],
            [70, 30, 60],
            self.c("home_bed"),
        );
        self.h_at(Mesh::Blob, [-420, 70, -FACE + 45], [70, 30, 60], rug2);
        for (z, c) in [(-130, "home_rug"), (-95, "home_sofa_dark")] {
            self.h_at(Mesh::Blob, [-hx + 60, 8, z], [44, 16, 22], self.c(c));
        }
        // a drawing on the spare room's door, more photos east
        self.h_on(east, -280, 250, 60, 80, 6, 1, self.c("home_sheet"), 0);
        self.h_on(east, -295, 260, 26, 20, 7, 1, self.c("cat_cream"), 0);
        self.h_on(east, -270, 230, 30, 10, 7, 1, self.c("heart"), 0);
        for (z, y) in [(700, 300), (850, 230), (620, 190)] {
            self.h_picture(east, z, y, 80, 70, 2);
        }
        self.h_plant(600, 0, -100, 120);
        // the loft hatch in the ceiling, a smoke alarm, light switches
        self.h_box([200, UP_H - 4, 150], [380, UP_H, 330], trim);
        self.h_box([215, UP_H - 6, 165], [365, UP_H - 4, 315], shade(trim, 92));
        self.h_at(Mesh::Cyl, [-200, UP_H - 3, 300], [24, 6, 24], trim);
        self.h_on(west, -80, 150, 12, 18, 2, 2, trim, 0);
        self.h_on(east, -160, 150, 12, 18, 2, 2, trim, 0);
        // the stair rods on the carpet down the stairs
        for k in 0..steps {
            let z = lip - 80 * k - 4;
            let y = -40 * (k + 1);
            self.h_rod([-well + 36, y + 2, z], [well - 36, y + 2, z], 2, brass);
        }
        // panelling below the dado rail, bare at the doors and the stairs
        self.h_wainscot(north, -hx, hx, 140, 120, &[(-230, 230)]);
        self.h_wainscot(south, -hx, hx, 140, 120, &[(-520, -320)]);
        self.h_wainscot(west, -FACE, FACE, 140, 120, &[(-350, -150)]);
        self.h_wainscot(east, -FACE, FACE, 140, 120, &[(-420, -220)]);
        // the lights
        self.h_ceiling_light(0, UP_H, -300, 100);
        self.h_ceiling_light(0, UP_H, 500, 100);
    }

    // ---- downstairs ---------------------------------------------------------------

    /// Downstairs, larger than it was: the open-plan ground floor, with
    /// the kitchen north, the dining table east, the living room west and
    /// the stairs coming down along the south wall behind you.
    fn home_downstairs(&mut self) {
        let w = DOWN_W;
        let top = DOWN_H;
        let paper = self.c("home_wall");
        let trim = self.c("home_trim");
        let wood = self.c("home_wood");
        // the floor, boards over the house and tiles in the kitchen
        self.h_boards(-w - 200, -w - 200, w + 200, w + 200, 120, 800, None);
        let tile = self.c("home_counter");
        let kz = -2400;
        // over the boards' seams (2 cm), not level with them
        self.h_flat([-w, 0, -w], [w, 6, kz], shade(tile, 92));
        let mut i = 0;
        let mut x = -w;
        while x < w {
            let mut j = 0;
            let mut z = -w;
            while z < kz {
                if (i + j) % 2 == 0 {
                    self.h_flat([x + 4, 6, z + 4], [x + 196, 7, z + 196], self.c("home_sheet"));
                }
                z += 200;
                j += 1;
            }
            x += 200;
            i += 1;
        }
        // the walls, to the ceiling; the ceiling with its beams
        let t = 200;
        self.h_block([-w - t, 0, -w - t], [w + t, top, -w], self.c("home_wall_2"));
        self.h_block([-w - t, 0, w], [w + t, top, w + t], paper);
        self.h_block([-w - t, 0, -w], [-w, top, w], paper);
        self.h_block([w, 0, -w], [w + t, top, w], paper);
        // nothing leaves over them
        self.h_solid([-w - t, top, -w - t], [w + t, top + 600, -w]);
        self.h_solid([-w - t, top, w], [w + t, top + 600, w + t]);
        self.h_solid([-w - t, top, -w], [-w, top + 600, w]);
        self.h_solid([w, top, -w], [w + t, top + 600, w]);
        // the ceiling, open over the top of the stairs
        let hole_x1 = -2500;
        let hole_z0 = 3200;
        self.h_solid([-w, top, -w], [w, top + 100, hole_z0]);
        self.h_solid([hole_x1, top, hole_z0], [w, top + 100, w]);
        self.h_lit([-w, top, -w], [w, top + 100, hole_z0], shade(trim, 97), ONE / 3);
        self.h_lit(
            [hole_x1, top, hole_z0],
            [w, top + 100, w],
            shade(trim, 97),
            ONE / 3,
        );
        let mut a = -w + 600;
        while a < w {
            self.h_box([a - 25, top - 50, -w], [a + 25, top, hole_z0], shade(wood, 110));
            a += 900;
        }
        let walls = [
            Wall {
                x_axis: false,
                at: -w,
                inward: 1,
            },
            Wall {
                x_axis: false,
                at: w,
                inward: -1,
            },
            Wall {
                x_axis: true,
                at: -w,
                inward: 1,
            },
            Wall {
                x_axis: true,
                at: w,
                inward: -1,
            },
        ];
        for wl in walls {
            self.h_on(wl, 0, 18, w * 2, 36, 5, 5, trim, 0);
            // a centimetre under the ceiling: the stair's top tread lies
            // level with it there
            self.h_on(wl, 0, top - 16, w * 2, 30, 14, 14, trim, 0);
        }
        let [north, south, west, east] = walls;
        // a big round rug over the middle, worn thin by the cat
        let (c1, c2, c3) = (self.c("home_rug"), self.c("home_rug_2"), self.c("home_carpet"));
        self.h_rug(0, 200, 1800, 1800, &[c1, c2, c3], true);
        self.h_flat_at(Mesh::Cyl, [0, 7, 200], [700, 2, 700], shade(c3, 90));
        self.h_flat_at(Mesh::Cyl, [0, 8, 200], [300, 2, 300], c1);
        self.home_kitchen(north);
        self.home_dining(east);
        self.home_living(west);
        self.home_stairs_down(south);
        self.home_entry(south);
        self.home_sunlight(north, south, west, east);
        self.home_corners(west);
    }

    /// The kitchen along the north wall: counters either side of the back
    /// door, the sink under a window, the stove and its hood, the fridge,
    /// the pantry, the island, and the treat bag.
    fn home_kitchen(&mut self, north: Wall) {
        let w = DOWN_W;
        let cab = self.c("home_cabinet");
        let counter = self.c("home_counter");
        let steel = self.c("home_steel");
        let trim = self.c("home_trim");
        let screen = self.c("home_screen");
        let brass = self.c("house_brass");
        let n = -w;
        let depth = 140;
        let ct = 180;
        // the counters, cupboard doors and handles, the worktop
        for (x0, x1) in [(-2460, -320), (320, 2200)] {
            self.h_solid([x0, 0, n], [x1, ct, n + depth]);
            self.h_box([x0, 12, n], [x1, ct - 14, n + depth - 4], cab);
            self.h_box([x0 + 6, 0, n], [x1 - 6, 12, n + depth - 12], shade(cab, 60));
            self.h_box([x0 - 6, ct - 14, n], [x1 + 6, ct, n + depth + 6], counter);
            let mut x = x0 + 120;
            while x < x1 - 40 {
                self.h_box(
                    [x - 2, 20, n + depth - 4],
                    [x + 2, ct - 22, n + depth - 2],
                    shade(cab, 80),
                );
                self.h_box(
                    [x - 30, ct - 50, n + depth - 4],
                    [x - 10, ct - 44, n + depth + 2],
                    steel,
                );
                x += 120;
            }
            // the tiles behind
            self.h_box([x0, ct, n], [x1, ct + 160, n + 3], trim);
        }
        // the sink under the window, its tap, the dish rack, herbs
        self.h_window(north, -1200, 470, 700, 340, 3, 2, None);
        self.h_box([-1400, ct - 4, n + 30], [-1000, ct + 1, n + 110], steel);
        self.h_box(
            [-1380, ct - 2, n + 40],
            [-1020, ct + 2, n + 100],
            shade(steel, 70),
        );
        self.h_rod([-1200, ct, n + 20], [-1200, ct + 60, n + 20], 6, steel);
        self.h_rod([-1200, ct + 60, n + 20], [-1200, ct + 55, n + 55], 5, steel);
        self.h_box([-900, ct, n + 20], [-760, ct + 8, n + 110], steel);
        for k in 0..5 {
            let x = -890 + k * 28;
            self.h_at(Mesh::Cyl, [x, ct + 30, n + 65], [4, 50, 50], self.c("home_sheet"));
        }
        for k in 0..3 {
            self.h_plant(-1440 + k * 120, 300, n + 20, 40);
        }
        // the stove, its knobs and the oven, the hood up to the ceiling
        let sx = 1200;
        self.h_box([sx - 140, ct - 14, n], [sx + 140, ct + 2, n + depth + 4], screen);
        for (dx, dz) in [(-70, 40), (70, 40), (-70, 100), (70, 100)] {
            self.h_at(
                Mesh::Ring,
                [sx + dx, ct + 4, n + dz],
                [50, 4, 50],
                shade(steel, 60),
            );
        }
        self.h_glow_at(
            Mesh::Ring,
            [sx + 70, ct + 5, n + 100],
            [44, 3, 44],
            self.c("yarn"),
            ONE / 2,
        );
        self.h_box(
            [sx - 130, 20, n + depth - 2],
            [sx + 130, ct - 30, n + depth + 2],
            steel,
        );
        self.h_box(
            [sx - 100, 40, n + depth],
            [sx + 100, ct - 70, n + depth + 3],
            shade(screen, 160),
        );
        self.h_box(
            [sx - 110, ct - 52, n + depth + 2],
            [sx + 110, ct - 46, n + depth + 10],
            steel,
        );
        let hood = Affine::translate(p([sx, 540, n + 60]));
        self.h_part(&hood, Mesh::Wedge, [0, 0, 0], [300, 80, 120], steel);
        self.h_box([sx - 60, 580, n], [sx + 60, DOWN_H, n + 60], steel);
        // a pan on the stove, a kettle, the toaster, the knife block
        self.h_at(
            Mesh::Cyl,
            [sx - 70, ct + 12, n + 40],
            [70, 18, 70],
            shade(steel, 50),
        );
        self.h_rod([sx - 70, ct + 18, n + 40], [sx - 70, ct + 22, n + 140], 5, screen);
        self.h_at(
            Mesh::Cyl,
            [-2000, ct + 30, n + 70],
            [44, 60, 44],
            self.c("home_rug"),
        );
        self.h_box([1700, ct, n + 40], [1800, ct + 50, n + 100], steel);
        self.h_box(
            [1900, ct, n + 50],
            [1950, ct + 60, n + 100],
            self.c("home_wood_dark"),
        );
        self.h_box([-600, ct, n + 30], [-480, ct + 90, n + 110], screen);
        self.h_mug(-560, ct, n + 120, self.c("home_rug_2"), 200);
        // the treat bag, open, a few spilt beside it
        let treat = self.c("treat");
        self.h_box([560, ct, n + 40], [660, ct + 120, n + 100], treat);
        self.h_box([555, ct + 120, n + 45], [665, ct + 136, n + 95], shade(treat, 80));
        self.h_on(north, 610, ct + 70, 50, 40, 101, 1, self.c("heart"), 0);
        for (dx, dz) in [(30, 120), (60, 110), (10, 130)] {
            self.h_at(
                Mesh::Blob,
                [660 + dx, ct + 5, n + dz],
                [12, 8, 10],
                shade(treat, 80),
            );
        }
        // the cupboards on the wall, apart from the window and the fridge
        for (x0, x1) in [(-2200, -1650), (-700, -350), (350, 900), (1500, 2200)] {
            self.h_box([x0, 400, n], [x1, 600, n + 80], cab);
            let mut x = x0 + 100;
            while x < x1 {
                self.h_box([x - 1, 410, n + 80], [x + 1, 590, n + 82], shade(cab, 80));
                x += 100;
            }
        }
        // the back door: glazed above, the cat flap below
        let door = Wall {
            x_axis: false,
            at: n,
            inward: 1,
        };
        self.h_door(door, 0, 360, 480, self.c("home_sofa"));
        self.h_on(door, 0, 360, 260, 160, 7, 2, self.c("home_window"), ONE);
        self.h_on(door, 0, 50, 70, 60, 8, 3, shade(screen, 150), 0);
        self.h_on(door, 0, 82, 80, 8, 9, 3, trim, 0);
        self.h_rug(0, n + 80, 300, 140, &[self.c("home_rug_2")], false);
        // the clock over it
        self.h_disc(north, 0, 620, 90, 6, trim, 0);
        self.h_rod([0, 620, n + 10], [0, 650, n + 10], 3, screen);
        self.h_rod([0, 620, n + 10], [18, 620, n + 10], 3, screen);
        // the fridge, its magnets and a drawing of the cat
        let (fx0, fx1, fz) = (-2740, -2460, n + 280);
        self.h_block([fx0, 0, n], [fx1, 360, fz], steel);
        self.h_box(
            [(fx0 + fx1) / 2 - 2, 130, fz],
            [(fx0 + fx1) / 2 + 2, 355, fz + 2],
            shade(steel, 70),
        );
        self.h_box([fx0 + 5, 125, fz], [fx1 - 5, 130, fz + 2], shade(steel, 70));
        for s in [-1, 1] {
            let x = (fx0 + fx1) / 2 + s * 20;
            self.h_box([x - 4, 160, fz], [x + 4, 320, fz + 10], shade(steel, 80));
        }
        self.h_box(
            [fx0 + 30, 230, fz],
            [fx0 + 110, 330, fz + 2],
            self.c("home_sheet"),
        );
        self.h_at(
            Mesh::Blob,
            [fx0 + 70, 270, fz + 3],
            [44, 30, 2],
            self.c("cat_cream"),
        );
        for (x, y, c) in [
            (fx1 - 40, 300, "yarn"),
            (fx1 - 80, 250, "home_plant"),
            (fx1 - 50, 200, "home_rug_2"),
        ] {
            self.h_box([x - 8, y - 8, fz], [x + 8, y + 8, fz + 4], self.c(c));
        }
        // the pantry in the corner
        self.h_block([-w, 0, n], [-3400, 520, -2900], self.c("home_cabinet"));
        self.h_box([-3398, 20, -3240], [-3396, 500, -3236], shade(cab, 80));
        for z in [-3260, -3220] {
            self.h_box([-3396, 230, z - 3], [-3390, 300, z + 3], brass);
        }
        // the cat's bowls by the fridge, on a mat
        self.h_flat([-2400, 0, -3200], [-2160, 2, -3060], self.c("home_rug_2"));
        self.h_at(Mesh::Cyl, [-2340, 8, -3130], [60, 16, 60], self.c("home_rug"));
        self.h_at(Mesh::Cyl, [-2220, 8, -3130], [60, 16, 60], self.c("home_bed"));
        self.h_glow_at(
            Mesh::Cyl,
            [-2220, 16, -3130],
            [44, 2, 44],
            self.c("water"),
            ONE / 3,
        );
        self.h_at(Mesh::Blob, [-2340, 16, -3130], [40, 8, 40], self.c("treat"));
        // the island, its stools on the south side, the pendants over it
        let (ix0, ix1, iz0, iz1) = (-1350, -450, -2660, -2300);
        self.h_solid([ix0, 0, iz0], [ix1, 180, iz1]);
        self.h_box(
            [ix0 + 10, 10, iz0 + 10],
            [ix1 - 10, 166, iz1 - 50],
            shade(cab, 90),
        );
        self.h_box([ix0, 166, iz0], [ix1, 180, iz1], counter);
        let mut x = ix0 + 120;
        while x < ix1 - 60 {
            self.h_box([x - 2, 20, iz0 + 6], [x + 2, 160, iz0 + 10], shade(cab, 70));
            x += 120;
        }
        for x in [-1150, -900, -650] {
            self.h_at(Mesh::Cyl, [x, 126, -2210], [70, 10, 70], self.c("home_wood"));
            for (dx, dz) in [(-22, -22), (22, -22), (-22, 22), (22, 22)] {
                self.h_rod(
                    [x + dx, 0, -2210 + dz],
                    [x + dx * 7 / 10, 122, -2210 + dz * 7 / 10],
                    4,
                    self.c("home_wood_dark"),
                );
            }
            self.h_pendant(x, DOWN_H, 760, -2480, 70, self.c("home_cabinet"));
        }
        // a fruit bowl, a mug and the paper on the island
        self.h_at(Mesh::Cyl, [-1180, 190, -2500], [70, 20, 70], trim);
        for (dx, dz, c) in [(-12, 0, "home_pot"), (14, 8, "yarn"), (2, -14, "home_plant")] {
            self.h_at(
                Mesh::Sphere,
                [-1180 + dx, 205, -2500 + dz],
                [24, 24, 24],
                self.c(c),
            );
        }
        self.h_at(
            Mesh::Blob,
            [-1150, 212, -2490],
            [50, 12, 16],
            self.c("home_rug_2"),
        );
        self.h_mug(-600, 180, -2420, self.c("home_bed"), 200);
        let paper = Affine::translate(p([-750, 181, -2540])).then(&Affine::rot_y(deg(20)));
        self.h_part(&paper, Mesh::Cube, [0, 0, 0], [80, 2, 56], self.c("home_sheet"));
        // his perches here: the fridge top and the island
        self.perches.push(p([(fx0 + fx1) / 2, 360, fz - 30]));
        self.perches.push(p([(ix0 + ix1) / 2, 180, (iz0 + iz1) / 2]));
    }

    /// The dining table east, its chairs, the sideboard and a lamp over it.
    fn home_dining(&mut self, east: Wall) {
        let wood = self.c("home_wood");
        let dark = self.c("home_wood_dark");
        let (cx, cz) = (2000, -1200);
        self.h_rug(
            cx,
            cz,
            1400,
            1100,
            &[self.c("home_carpet"), shade(self.c("home_carpet"), 90)],
            false,
        );
        // the table: top, apron and legs
        let (x0, x1, z0, z1) = (cx - 400, cx + 400, cz - 200, cz + 200);
        self.h_solid([x0, 0, z0], [x1, 150, z1]);
        self.h_box([x0 - 10, 138, z0 - 10], [x1 + 10, 150, z1 + 10], wood);
        self.h_box([x0 + 20, 118, z0 + 20], [x1 - 20, 138, z1 - 20], dark);
        for (x, z) in [
            (x0 + 30, z0 + 30),
            (x1 - 30, z0 + 30),
            (x0 + 30, z1 - 30),
            (x1 - 30, z1 - 30),
        ] {
            self.h_box([x - 14, 0, z - 14], [x + 14, 138, z + 14], dark);
        }
        // the chairs: seats solid, backs only looked at
        let mut chairs = Vec::new();
        for x in [cx - 270, cx, cx + 270] {
            chairs.push((x, z0 - 110, 1));
            chairs.push((x, z1 + 110, -1));
        }
        for (k, (x, z, s)) in chairs.into_iter().enumerate() {
            let pulled = if k == 3 { 60 * s } else { 0 };
            let z = z - pulled;
            self.h_solid([x - 50, 0, z - 50], [x + 50, 95, z + 50]);
            self.h_box([x - 52, 85, z - 52], [x + 52, 97, z + 52], wood);
            for (dx, dz) in [(-42, -42), (42, -42), (-42, 42), (42, 42)] {
                self.h_box([x + dx - 5, 0, z + dz - 5], [x + dx + 5, 85, z + dz + 5], dark);
            }
            let back = z - s * 46;
            self.h_box([x - 50, 97, back - 5], [x + 50, 210, back + 5], wood);
            self.h_box([x - 40, 140, back - 6], [x + 40, 170, back + 6], shade(wood, 80));
            self.h_box([x - 30, 97, z - 30], [x + 30, 102, z + 30], self.c("home_rug"));
        }
        for (x, s) in [(x0 - 110, 1), (x1 + 110, -1)] {
            self.h_solid([x - 50, 0, cz - 50], [x + 50, 95, cz + 50]);
            self.h_box([x - 52, 85, cz - 52], [x + 52, 97, cz + 52], wood);
            let back = x - s * 46;
            self.h_box([back - 5, 97, cz - 50], [back + 5, 210, cz + 50], wood);
            for (dx, dz) in [(-42, -42), (42, -42), (-42, 42), (42, 42)] {
                self.h_box([x + dx - 5, 0, cz + dz - 5], [x + dx + 5, 85, cz + dz + 5], dark);
            }
        }
        // on it: flowers, placemats and plates, candles, the paper
        let sheet = self.c("home_sheet");
        self.h_at(Mesh::Cyl, [cx, 175, cz], [30, 50, 30], self.c("home_bed"));
        for (dx, dz, c) in [
            (0, 0, "heart"),
            (14, 10, "home_rug_2"),
            (-12, 8, "yarn"),
            (6, -14, "home_window"),
        ] {
            self.h_at(Mesh::Sphere, [cx + dx, 215, cz + dz], [24, 22, 24], self.c(c));
        }
        self.h_at(Mesh::Blob, [cx, 200, cz], [60, 20, 60], self.c("home_plant"));
        for x in [cx - 270, cx, cx + 270] {
            for z in [z0 + 50, z1 - 50] {
                self.h_flat([x - 40, 150, z - 30], [x + 40, 151, z + 30], self.c("home_rug_2"));
                self.h_at(Mesh::Cyl, [x, 153, z], [50, 4, 50], sheet);
            }
        }
        for dx in [-120, 120] {
            self.h_at(Mesh::Cyl, [cx + dx, 160, cz], [14, 20, 14], self.c("house_brass"));
            self.h_at(Mesh::Cyl, [cx + dx, 190, cz], [6, 40, 6], sheet);
        }
        // the lamp over the table
        self.h_pendant(cx, DOWN_H, 800, cz, 160, self.c("home_rug_2"));
        // the sideboard against the east wall, a mirror, plates, a lamp
        let (sz0, sz1) = (-1700, -700);
        self.h_block([DOWN_W - 140, 0, sz0], [DOWN_W, 200, sz1], dark);
        for k in 0..4 {
            let z = sz0 + 125 + k * 250;
            self.h_box(
                [DOWN_W - 142, 20, z - 115],
                [DOWN_W - 140, 180, z + 115],
                shade(dark, 120),
            );
            self.h_at(
                Mesh::Sphere,
                [DOWN_W - 146, 120, z + 90],
                [8, 8, 8],
                self.c("house_brass"),
            );
        }
        self.h_on(east, -1200, 420, 300, 260, 5, 5, self.c("house_brass"), 0);
        self.h_on(east, -1200, 420, 270, 230, 6, 1, self.c("home_steel"), ONE / 5);
        self.h_at(
            Mesh::Cyl,
            [DOWN_W - 70, 215, -1550],
            [36, 30, 36],
            self.c("home_pot"),
        );
        self.shape(
            Mesh::Cone,
            boxed(p([DOWN_W - 70, 270, -1550]), p([60, 50, 60])),
            self.c("home_rug_2"),
            ONE * 2 / 3,
            true,
        );
        self.h_plant(DOWN_W - 70, 200, -850, 60);
        for k in 0..3 {
            let z = -1350 + k * 60;
            self.h_at(
                Mesh::Cyl,
                [DOWN_W - 70, 202 + k * 4, z],
                [60, 4, 60],
                self.c("home_sheet"),
            );
        }
        // a big plant in the corner
        self.h_plant(DOWN_W - 150, 0, -2100, 220);
        self.perches.push(p([cx, 150, cz]));
    }

    /// The living room west: the TV on its stand, the sofa facing it, the
    /// coffee table, an armchair, the divider shelf, lamps, plants, and
    /// the cat tree across the room by the east windows.
    fn home_living(&mut self, west: Wall) {
        let w = DOWN_W;
        let sofa = self.c("home_sofa");
        let sofa_dark = self.c("home_sofa_dark");
        let wood = self.c("home_wood");
        let dark = self.c("home_wood_dark");
        let screen = self.c("home_screen");
        let rug = self.c("home_rug");
        let rug2 = self.c("home_rug_2");
        let brass = self.c("house_brass");
        let carpet = self.c("home_carpet");
        self.h_rug(-2050, 850, 1900, 1500, &[rug, rug2, rug], false);
        // the TV on its stand, a console and the speakers
        self.h_block([-w, 0, 250], [-w + 180, 120, 1450], dark);
        for k in 0..3 {
            let z = 250 + 200 + k * 400;
            self.h_box(
                [-w + 180, 15, z - 190],
                [-w + 182, 105, z + 190],
                shade(dark, 120),
            );
        }
        self.h_box([-w + 30, 120, 760], [-w + 70, 140, 940], dark);
        self.h_box([-w + 40, 140, 830], [-w + 60, 165, 870], dark);
        self.h_box([-w + 25, 165, 380], [-w + 55, 540, 1320], screen);
        self.h_lit(
            [-w + 56, 180, 400],
            [-w + 57, 525, 1300],
            self.c("home_sofa_dark"),
            ONE / 4,
        );
        self.h_lit(
            [-w + 57, 420, 1050],
            [-w + 58, 500, 1260],
            self.c("home_window"),
            ONE / 6,
        );
        self.h_box([-w + 100, 120, 1100], [-w + 140, 128, 1160], screen);
        for z in [120, 1580] {
            self.h_block([-w, 0, z - 50], [-w + 100, 260, z + 50], screen);
            self.h_at(Mesh::Cyl, [-w + 101, 200, z], [2, 40, 40], shade(screen, 160));
        }
        // the shelf over the TV: books and a plant
        self.h_box([-w, 700, 400], [-w + 60, 712, 1300], wood);
        let shelf = Wall {
            x_axis: true,
            at: -w + 60,
            inward: 1,
        };
        self.h_books(shelf, 450, 950, 712, 70, 50, 150);
        self.h_plant(-w + 30, 712, 1150, 60);
        self.h_picture(west, -800, 500, 500, 340, 0);
        // the sofa, facing the TV
        let (sx0, sx1, sz0, sz1) = (-1640, -1360, 350, 1250);
        self.h_block([sx0, 0, sz0], [sx1, 100, sz1], sofa);
        self.h_block([sx1 - 70, 100, sz0], [sx1, 200, sz1], sofa);
        for (z0, z1) in [(sz0, sz0 + 70), (sz1 - 70, sz1)] {
            self.h_block([sx0, 100, z0], [sx1 - 70, 150, z1], sofa_dark);
        }
        for k in 0..3 {
            let z = sz0 + 70 + 127 + k * 254;
            self.h_box(
                [sx0 + 5, 100, z - 120],
                [sx1 - 75, 122, z + 120],
                shade(sofa, 108),
            );
            self.h_at(Mesh::Blob, [sx1 - 90, 150, z], [50, 90, 220], shade(sofa, 104));
        }
        for (z, c) in [
            (sz0 + 140, rug2),
            (sz1 - 150, rug),
            (sz1 - 230, self.c("home_bed")),
        ] {
            let f = Affine::translate(p([sx1 - 110, 150, z])).then(&Affine::rot_z(deg(20)));
            self.h_part(&f, Mesh::Cube, [0, 0, 0], [24, 60, 60], c);
        }
        // a blanket over the arm, the arm scratched to threads
        self.h_box(
            [sx0 - 4, 60, sz1 - 74],
            [sx1 - 66, 154, sz1 - 40],
            self.c("home_bed"),
        );
        for k in 0..4 {
            let y = 30 + k * 16;
            self.h_box(
                [sx0 - 3, y, sz0 + 10 + k * 8],
                [sx0 - 1, y + 8, sz0 + 60],
                sofa_dark,
            );
        }
        for (x, z) in [
            (sx0 + 20, sz0 + 20),
            (sx1 - 20, sz0 + 20),
            (sx0 + 20, sz1 - 20),
            (sx1 - 20, sz1 - 20),
        ] {
            self.h_box([x - 8, -1, z - 8], [x + 8, 4, z + 8], dark);
        }
        self.perches.push(p([sx1 - 35, 200, (sz0 + sz1) / 2]));
        // the coffee table: mugs, the remote, magazines underneath
        let (tx0, tx1, tz0, tz1) = (-2600, -2200, 550, 1050);
        self.h_solid([tx0, 0, tz0], [tx1, 90, tz1]);
        self.h_box([tx0, 80, tz0], [tx1, 90, tz1], wood);
        self.h_box([tx0 + 20, 25, tz0 + 20], [tx1 - 20, 30, tz1 - 20], wood);
        for (x, z) in [
            (tx0 + 15, tz0 + 15),
            (tx1 - 15, tz0 + 15),
            (tx0 + 15, tz1 - 15),
            (tx1 - 15, tz1 - 15),
        ] {
            self.h_box([x - 8, 0, z - 8], [x + 8, 80, z + 8], dark);
        }
        self.h_box(
            [tx0 + 60, 30, tz0 + 60],
            [tx0 + 200, 40, tz0 + 260],
            self.c("home_rug"),
        );
        self.h_box(
            [tx0 + 80, 40, tz0 + 70],
            [tx0 + 210, 46, tz0 + 250],
            self.c("home_bed"),
        );
        self.h_mug(-2280, 90, 640, rug2, 200);
        self.h_mug(-2300, 90, 960, self.c("home_cabinet"), 200);
        self.h_box([-2520, 90, 850], [-2490, 96, 940], screen);
        self.h_box([-2560, 90, 620], [-2420, 96, 720], self.c("house_book_2"));
        self.h_plant(-2450, 90, 1000, 40);
        self.perches.push(p([(tx0 + tx1) / 2, 90, (tz0 + tz1) / 2]));
        // the armchair by the window, a footstool, a reading lamp
        let (ax0, ax1, az0, az1) = (-2640, -2360, 1660, 1940);
        self.h_block([ax0, 0, az0], [ax1, 100, az1], sofa_dark);
        self.h_block([ax1 - 70, 100, az0], [ax1, 220, az1], sofa_dark);
        for (z0, z1) in [(az0, az0 + 50), (az1 - 50, az1)] {
            self.h_block([ax0, 100, z0], [ax1 - 70, 150, z1], shade(sofa_dark, 120));
        }
        self.h_box(
            [ax0 + 5, 100, az0 + 50],
            [ax1 - 75, 120, az1 - 50],
            shade(sofa_dark, 115),
        );
        self.h_block([ax0 - 200, 0, 1700], [ax0 - 80, 60, 1900], sofa_dark);
        self.perches.push(p([ax1 - 35, 220, (az0 + az1) / 2]));
        for (x, z) in [(-2200, 2100), (-1250, 1400)] {
            self.h_at(Mesh::Cyl, [x, 3, z], [50, 6, 50], dark);
            self.h_rod([x, 0, z], [x, 300, z], 4, brass);
            self.shape(
                Mesh::Cone,
                boxed(p([x, 320, z]), p([70, 50, 70])),
                rug2,
                ONE * 2 / 3,
                true,
            );
        }
        // the shelf that divides the room from the kitchen
        let (bx0, bx1, bz0, bz1) = (-2600, -1800, -1060, -940);
        self.h_solid([bx0, 0, bz0], [bx1, 320, bz1]);
        for x in [bx0, bx1 - 8] {
            self.h_box([x, 0, bz0], [x + 8, 320, bz1], dark);
        }
        self.h_box([bx0 + 1, 1, bz0 + 56], [bx1 - 1, 319, bz0 + 64], shade(dark, 120));
        for k in 0..5 {
            let y = k * 79;
            self.h_box([bx0, y, bz0], [bx1, y + 8, bz1], dark);
            if k < 4 {
                for (at, inward) in [(bz1, 1), (bz0, -1)] {
                    let front = Wall {
                        x_axis: false,
                        at,
                        inward,
                    };
                    self.h_books(front, bx0 + 20, bx1 - 20, y + 8, 64, 50, 160);
                }
            }
        }
        self.h_plant(bx0 + 60, 320, -1000, 60);
        self.h_at(
            Mesh::Cyl,
            [bx1 - 80, 340, -1000],
            [40, 40, 40],
            self.c("home_rug"),
        );
        self.perches.push(p([(bx0 + bx1) / 2, 320, (bz0 + bz1) / 2]));
        // plants by the TV, a ceiling fan over the sofa
        self.h_plant(-w + 120, 0, -150, 240);
        self.h_plant(-w + 120, 0, 2000, 200);
        let fan_y = DOWN_H - 160;
        self.h_rod([-2000, DOWN_H, 800], [-2000, fan_y, 800], 6, dark);
        self.h_at(Mesh::Cyl, [-2000, fan_y, 800], [50, 30, 50], wood);
        for k in 0..4 {
            let f = Affine::translate(p([-2000, fan_y, 800])).then(&Affine::rot_y(fx::QUARTER * k + deg(20)));
            self.h_part(&f, Mesh::Cube, [130, 0, 0], [220, 4, 40], wood);
        }
        // the cat tree across the room, by the east windows: a cubby at
        // its foot, two platforms on their own posts, a bed on top
        let (cx, cz) = (2500, 1400);
        let rope = self.c("home_cattree");
        let fur = shade(carpet, 92);
        self.h_block([cx - 180, 0, cz - 180], [cx + 180, 30, cz + 180], carpet);
        self.h_solid([cx - 45, 0, cz - 45], [cx + 45, 320, cz + 45]);
        self.h_at(Mesh::Cyl, [cx, 175, cz], [90, 290, 90], rope);
        for k in 0..4 {
            self.h_at(Mesh::Ring, [cx, 60 + k * 70, cz], [94, 4, 94], shade(rope, 85));
        }
        self.h_block([cx + 50, 30, cz - 175], [cx + 175, 150, cz - 50], fur);
        let cubby = Wall {
            x_axis: false,
            at: cz - 175,
            inward: -1,
        };
        self.h_disc(cubby, cx + 112, 85, 70, 1, self.c("home_ink"), 0);
        for (dx, dz, y) in [(115, 115, 130), (-115, -115, 230)] {
            let (px, pz) = (cx + dx, cz + dz);
            self.h_block([px - 95, y, pz - 95], [px + 95, y + 20, pz + 95], fur);
            self.h_at(Mesh::Cyl, [px, 30 + (y - 30) / 2, pz], [40, y - 30, 40], rope);
            self.h_solid([px - 20, 0, pz - 20], [px + 20, y, pz + 20]);
            self.h_rod([px + 90, y, pz + 90], [px + 90, y - 70, pz + 90], 1, rope);
            self.h_at(
                Mesh::Sphere,
                [px + 90, y - 75, pz + 90],
                [16, 16, 16],
                self.c("heart"),
            );
        }
        self.h_block([cx - 140, 320, cz - 140], [cx + 140, 340, cz + 140], fur);
        self.h_at(Mesh::Ring, [cx, 358, cz], [250, 36, 250], shade(carpet, 80));
        self.h_at(Mesh::Cyl, [cx, 343, cz], [220, 6, 220], self.c("home_sheet"));
        self.perches.push(p([cx, 340, cz]));
        // the crinkle tunnel and toys about the floor
        let tunnel = Affine::translate(p([2900, 35, 2300])).then(&Affine::rot_x(fx::QUARTER));
        self.h_part(&tunnel, Mesh::Ring, [0, 0, 0], [70, 200, 70], self.c("home_bed"));
        self.h_yarn(1800, 400, 40);
        self.h_mouse(-600, -500, 200);
        self.h_mouse(900, 1600, 200);
    }

    /// The stairs you came down, along the south wall, rising west into
    /// the ceiling: walkable, a banister on the open side.
    fn home_stairs_down(&mut self, south: Wall) {
        let w = DOWN_W;
        let wood = self.c("home_wood");
        let trim = self.c("home_trim");
        let wall = self.c("home_wall");
        let carpet = self.c("home_rug");
        let (z0, z1) = (3240, w);
        let foot = -500;
        let steps = 35;
        for k in 0..steps {
            let (x0, x1) = (foot - 80 * (k + 1), foot - 80 * k);
            let y = 40 * (k + 1);
            self.h_solid([x0, 0, z0], [x1, y, z1]);
            self.h_box([x0, 0, z0], [x1, y - 6, z0 + 10], wall);
            self.h_box([x0, y - 6, z0], [x1 + 4, y, z1], wood);
            self.h_flat([x0, y, z0 + 70], [x1, y + 2, z1 - 70], carpet);
            // the banister: a solid rail along each step, balusters
            self.h_solid([x0, y, z0 - 4], [x1, y + 110, z0 + 10]);
            if k % 2 == 0 {
                self.h_rod([x1 - 40, y, z0 + 4], [x1 - 40, y + 100, z0 + 4], 4, trim);
            }
        }
        let top_x = foot - 80 * steps;
        let top_y = 40 * steps;
        self.h_rod([foot, 100, z0 + 4], [top_x, top_y + 100, z0 + 4], 8, wood);
        self.h_box(
            [foot - 12, 0, z0 - 8],
            [foot + 12, 140, z0 + 16],
            self.c("home_wood_dark"),
        );
        self.h_at(
            Mesh::Sphere,
            [foot, 150, z0 + 4],
            [24, 24, 24],
            self.c("home_wood_dark"),
        );
        // under the stairs: the stringer, a cupboard door
        let side = Wall {
            x_axis: false,
            at: z0,
            inward: -1,
        };
        self.h_door(side, -1400, 160, 260, trim);
        // the landing at the top, past the ceiling, and its walls
        self.h_block([-w, 0, z0], [top_x, top_y, z1], wall);
        self.h_flat([-w, top_y, z0], [top_x, top_y + 2, z1], self.c("home_floor"));
        // drawn from a centimetre over the ceiling's underside (their
        // bottoms in its plane flickered)
        self.h_solid([-w, top_y, 2800], [-2500, top_y + 600, 3200]);
        // and a centimetre back from the ceiling's edge at the stairs
        self.h_box([-w, top_y + 1, 2800], [-2500, top_y + 600, 3199], wall);
        self.h_solid([-2500, top_y, 2800], [-2400, top_y + 600, z1]);
        self.h_box([-2500, top_y + 1, 2800], [-2400, top_y + 600, z1], wall);
        self.h_solid([-w, top_y + 600, 2800], [-2400, top_y + 700, z1]);
        self.h_flat([-w, top_y + 600, 2800], [-2400, top_y + 700, z1], shade(trim, 90));
        // photos climbing the wall beside the stairs
        for k in 0..6 {
            let x = foot - 300 - k * 400;
            let y = (-500 - x) / 2 + 230 + (k % 2) * 50;
            self.h_picture(south, x, y, 120, 100 + (k % 3) * 30, 2);
        }
    }

    /// The front door in the south wall, the hall's coats and shoes.
    fn home_entry(&mut self, south: Wall) {
        let w = DOWN_W;
        let dark = self.c("home_wood_dark");
        self.h_door(south, 1200, 360, 520, self.c("home_rug"));
        for s in [-1, 1] {
            self.h_window(south, 1200 + s * 300, 300, 80, 400, 1, 3, None);
        }
        self.h_rug(1200, w - 100, 360, 160, &[self.c("home_cattree")], false);
        // the shoe rack, shoes on it
        self.h_block([1700, 0, w - 120], [2100, 80, w], dark);
        for k in 0..5 {
            let x = 1730 + k * 75;
            let c = if k % 2 == 0 {
                self.c("home_sofa_dark")
            } else {
                self.c("home_rug")
            };
            self.h_at(Mesh::Blob, [x, 92, w - 60], [50, 24, 90], c);
        }
        // the coat stand
        let x = 700;
        let z = w - 100;
        self.h_at(Mesh::Cyl, [x, 4, z], [70, 8, 70], dark);
        self.h_rod([x, 0, z], [x, 420, z], 8, dark);
        for (dx, dz, c) in [
            (30, 0, "home_rug"),
            (-26, 16, "home_cabinet"),
            (0, -30, "home_sofa"),
        ] {
            self.h_at(Mesh::Blob, [x + dx, 300, z + dz], [70, 200, 60], self.c(c));
        }
        // post on the mat
        for (dx, dz, a) in [(-40, -60, 10), (20, -90, -25)] {
            let f = Affine::translate(p([1200 + dx, 3, w - 140 + dz])).then(&Affine::rot_y(deg(a)));
            self.h_part(&f, Mesh::Cube, [0, 0, 0], [50, 2, 34], self.c("home_sheet"));
        }
    }

    /// The windows round the room, the morning sun through the east ones.
    fn home_sunlight(&mut self, north: Wall, south: Wall, west: Wall, east: Wall) {
        let curtain = self.c("home_rug_2");
        let blue = self.c("home_bed");
        for z in [-2600, 800, 2500] {
            self.h_window(east, z, 760, 800, 900, 3, 3, Some(curtain));
            self.h_beam(
                [DOWN_W - 20, 900, z],
                [DOWN_W - 1300, 0, z + 150],
                v3(0, 0, ONE),
                760,
                40,
            );
            self.h_lit(
                [DOWN_W - 1500, 3, z - 380],
                [DOWN_W - 500, 4, z + 560],
                self.c("home_window"),
                ONE / 4,
            );
        }
        for z in [-2400, 2300] {
            self.h_window(west, z, 760, 700, 900, 3, 3, Some(blue));
        }
        for x in [2300, 3150] {
            self.h_window(south, x, 700, 600, 800, 2, 3, Some(curtain));
        }
        self.h_window(north, 2900, 760, 800, 900, 4, 3, Some(blue));
        // pictures between the windows
        self.h_picture(east, -1200, 900, 300, 200, 1);
        self.h_picture(west, 300, 900, 260, 180, 2);
        // ceiling lights
        for (x, z) in [(0, -1000), (0, 1000), (1600, 1200), (-1200, -1800), (1600, -2700)] {
            self.h_ceiling_light(x, DOWN_H, z, 160);
        }
        // panelling round the walls, bare where a door or the stairs are
        self.h_wainscot(south, -500, DOWN_W, 240, 300, &[(780, 1620)]);
        self.h_wainscot(east, -DOWN_W, DOWN_W, 240, 300, &[]);
        self.h_wainscot(west, -2900, DOWN_W, 240, 300, &[(-1870, -1130)]);
    }

    /// Panelling on a wall to `h`, from `from` to `to` along it, a rail on
    /// top and a stile every `step`, bare along each of `gaps`.
    fn h_wainscot(&mut self, wall: Wall, from: i32, to: i32, h: i32, step: i32, gaps: &[(i32, i32)]) {
        let trim = self.c("home_trim");
        let mut spans = vec![(from, to)];
        for &(g0, g1) in gaps {
            let mut out = Vec::new();
            for (a, b) in spans {
                if g1 <= a || g0 >= b {
                    out.push((a, b));
                    continue;
                }
                if g0 > a {
                    out.push((a, g0));
                }
                if g1 < b {
                    out.push((g1, b));
                }
            }
            spans = out;
        }
        for (a, b) in spans {
            self.h_on(wall, (a + b) / 2, h / 2, b - a, h, 3, 3, shade(trim, 96), 0);
            self.h_on(wall, (a + b) / 2, h, b - a, 10, 7, 7, trim, 0);
            let mut x = a + step;
            while x < b - step / 3 {
                self.h_on(wall, x, h / 2, 8, h - 20, 5, 2, trim, 0);
                x += step;
            }
        }
    }

    /// The rest of the house round the edges: the fireplace, the piano,
    /// the reading corner, a desk, the breakfast table, plants, the
    /// litter tray.
    fn home_corners(&mut self, west: Wall) {
        let w = DOWN_W;
        let paper = self.c("home_wall");
        let trim = self.c("home_trim");
        let wood = self.c("home_wood");
        let dark = self.c("home_wood_dark");
        let screen = self.c("home_screen");
        let sheet = self.c("home_sheet");
        // the fireplace on the west wall, its chimney breast to the ceiling
        let (z0, z1) = (-1850, -1150);
        self.h_block([-w, 0, z0], [-w + 160, DOWN_H, z1], shade(paper, 95));
        self.h_block(
            [-w + 160, 0, z0 + 20],
            [-w + 300, 12, z1 - 20],
            shade(self.c("home_counter"), 80),
        );
        self.h_box([-w + 160, 0, z0 + 150], [-w + 162, 200, z1 - 150], screen);
        for z in [z0 + 110, z1 - 110] {
            self.h_box([-w + 160, 0, z - 40], [-w + 180, 260, z + 40], trim);
        }
        self.h_box([-w + 160, 200, z0 + 70], [-w + 180, 260, z1 - 70], trim);
        self.h_box([-w + 150, 260, z0 + 40], [-w + 230, 280, z1 - 40], wood);
        for k in 0..3 {
            let f = Affine::translate(p([-w + 230, 20 + k * 22, -1500 + (k - 1) * 30]))
                .then(&Affine::rot_x(fx::QUARTER));
            self.h_part(&f, Mesh::Cyl, [0, 0, 0], [24, 220, 24], dark);
        }
        self.h_glow_at(
            Mesh::Blob,
            [-w + 230, 12, -1500],
            [160, 12, 50],
            self.c("yarn"),
            ONE / 3,
        );
        // on the mantel: a clock, candles, photos; a painting over it
        self.h_at(Mesh::Cyl, [-w + 190, 310, -1500], [30, 60, 60], wood);
        self.h_disc(west, -1500, 315, 44, 191, sheet, 0);
        for z in [-1700, -1640, -1330] {
            self.h_at(Mesh::Cyl, [-w + 190, 310, z], [12, 60, 12], sheet);
            self.h_glow_at(Mesh::Blob, [-w + 190, 346, z], [8, 12, 8], self.c("lamp"), ONE);
        }
        self.h_box([-w + 175, 280, -1420], [-w + 185, 340, -1380], dark);
        self.h_picture(west, -1500, 600, 360, 260, 0);
        // the basket of logs beside it
        self.h_at(
            Mesh::Cyl,
            [-w + 260, 35, -1990],
            [90, 70, 90],
            self.c("home_cattree"),
        );
        // the piano against the south wall, its stool, music on it
        let (px0, px1) = (200, 700);
        self.h_block([px0, 0, w - 180], [px1, 260, w], dark);
        self.h_solid([px0, 0, w - 240], [px1, 150, w - 180]);
        self.h_box([px0 + 10, 130, w - 240], [px1 - 10, 150, w - 180], sheet);
        let mut x = px0 + 30;
        while x < px1 - 20 {
            self.h_box([x - 4, 150, w - 236], [x + 4, 156, w - 205], screen);
            x += 26;
        }
        self.h_box([px0, 80, w - 240], [px0 + 12, 130, w - 180], dark);
        self.h_box([px1 - 12, 80, w - 240], [px1, 130, w - 180], dark);
        self.h_box([px0 + 120, 160, w - 186], [px1 - 120, 230, w - 182], sheet);
        self.h_block([px0 + 130, 0, w - 380], [px1 - 130, 100, w - 300], dark);
        self.h_box(
            [px0 + 125, 100, w - 385],
            [px1 - 125, 110, w - 295],
            self.c("home_rug"),
        );
        self.h_plant(px1 - 60, 260, w - 90, 50);
        self.h_box([px0 + 40, 260, w - 120], [px0 + 100, 330, w - 110], dark);
        self.h_box(
            [px0 + 45, 265, w - 121],
            [px0 + 95, 325, w - 120],
            self.c("home_sky"),
        );
        // a shelf over it: books and a plant
        self.h_box([px0 - 50, 520, w - 60], [px1 + 50, 532, w], wood);
        let shelf = Wall {
            x_axis: false,
            at: w - 60,
            inward: -1,
        };
        self.h_books(shelf, px0, px0 + 300, 532, 70, 50, 150);
        self.h_plant(px1, 532, w - 30, 60);
        // the reading corner: a tall bookcase, a beanbag, a lamp
        // clear of the window's frame at 2650 (its side lay in the case's)
        let (bz0, bz1) = (2652, 3150);
        self.h_block([-w, 0, bz0], [-w + 150, 420, bz1], dark);
        let face = Wall {
            x_axis: true,
            at: -w + 150,
            inward: 1,
        };
        for k in 0..5 {
            let y = 10 + k * 82;
            self.h_box([-w + 150, y - 8, bz0], [-w + 152, y, bz1], shade(dark, 120));
            self.h_books(face, bz0 + 15, bz1 - 15, y, 66, 30, 120);
        }
        self.h_at(
            Mesh::Blob,
            [-w + 420, 45, 2850],
            [180, 90, 180],
            self.c("home_cabinet"),
        );
        // the desk east, under the windows: a computer, a chair, papers
        let (dz0, dz1) = (1600, 2050);
        self.h_block([w - 220, 0, dz0], [w, 150, dz1], wood);
        self.h_box([w - 150, 150, 1760], [w - 140, 290, 1900], screen);
        self.h_lit(
            [w - 152, 160, 1770],
            [w - 151, 280, 1890],
            self.c("home_bed"),
            ONE / 3,
        );
        self.h_box([w - 200, 150, 1720], [w - 170, 154, 1940], self.c("home_steel"));
        self.h_mug(w - 180, 150, 1660, self.c("home_rug"), 200);
        self.h_block([w - 420, 0, 1760], [w - 300, 95, 1880], self.c("home_sofa_dark"));
        self.h_box(
            [w - 430, 95, 1760],
            [w - 410, 220, 1880],
            self.c("home_sofa_dark"),
        );
        // the breakfast table in the kitchen's east corner
        let (tx, tz) = (2900, -2950);
        self.h_solid([tx - 110, 0, tz - 110], [tx + 110, 150, tz + 110]);
        self.h_at(Mesh::Cyl, [tx, 144, tz], [230, 12, 230], wood);
        self.h_at(Mesh::Cyl, [tx, 70, tz], [30, 140, 30], dark);
        self.h_at(Mesh::Cyl, [tx, 4, tz], [100, 8, 100], dark);
        for s in [-1, 1] {
            let x = tx + s * 210;
            self.h_block([x - 50, 0, tz - 50], [x + 50, 95, tz + 50], wood);
            self.h_box(
                [x + s * 45 - 5, 95, tz - 50],
                [x + s * 45 + 5, 210, tz + 50],
                wood,
            );
        }
        self.h_plant(tx, 150, tz, 40);
        self.h_mug(tx + 50, 150, tz + 40, self.c("home_rug_2"), 200);
        // the litter tray in the corner by the pantry
        self.h_box([-3380, 0, -2800], [-3120, 36, -2560], self.c("home_bed"));
        self.h_flat([-3360, 36, -2780], [-3140, 38, -2580], self.c("home_rug_2"));
        // plants in the corners
        self.h_plant(3350, 0, 3350, 260);
        self.h_plant(-200, 0, 3380, 200);
        self.h_plant(3400, 0, -400, 220);
    }
}

#[cfg(test)]
mod tests {
    use super::super::{build, dm, walk, Arena, Side, COURT_RADIUS};
    use super::*;
    use crate::balance::{PLAYER_HEIGHT, PLAYER_RADIUS};
    use crate::content::tests::content;
    use crate::fx::int;

    fn home(kind: &str, seed: u64) -> Arena {
        let c = content();
        build("home", kind, seed, &[Side::North], &c.pal)
    }

    /// No solid in a body's box with its feet at `at`.
    fn clear(a: &Arena, at: V3) -> bool {
        let r = PLAYER_RADIUS;
        !a.level.blocked(
            v3(at.x - r, at.y + ONE / 20, at.z - r),
            v3(at.x + r, at.y + PLAYER_HEIGHT, at.z + r),
        )
    }

    #[test]
    fn every_home_room_builds_with_a_clear_start_and_the_door_north_walked_to() {
        for kind in ["arrival", "rest", "boss"] {
            for seed in 1..=4 {
                let a = home(kind, seed);
                assert!(
                    clear(&a, a.start),
                    "{kind} seed {seed}: the start is inside a solid"
                );
                assert!(a.shapes.len() > 300, "{kind}: only {} shapes", a.shapes.len());
                if kind == "boss" {
                    assert_eq!(a.start, v3(0, 0, COURT_RADIUS - int(8)));
                    continue;
                }
                assert_eq!(a.gates.len(), 2, "{kind}: the way in and the door on");
                assert_eq!(a.gates[0].0, v3(0, 0, ROOM), "{kind}: the way in");
                assert_eq!(a.gates[1].0, v3(0, 0, -ROOM), "{kind}: the door north");
                let reached = walk(&a.level, a.start);
                let door = a.gates[1].0;
                let near = reached
                    .iter()
                    .any(|p| p.sub(door).len_xz() < int(2) && p.y.abs() < ONE);
                assert!(near, "{kind} seed {seed}: the door north cannot be walked to");
            }
        }
    }

    #[test]
    fn the_cat_walks_from_beside_you_to_the_bedroom_door_on_clear_floor() {
        let a = home("arrival", 1);
        let from = a.start.add(v3(int(3), 0, -int(4)));
        let to = a.gates[1].0;
        for k in 0..=20 {
            let at = from.lerp(to, ONE * k / 20);
            assert_eq!(
                a.level.floor_below(at.x, ONE, at.z),
                Some(0),
                "no floor at {at:?}"
            );
            assert!(
                !a.level.blocked(
                    v3(at.x - dm(3), dm(1), at.z - dm(3)),
                    v3(at.x + dm(3), ONE, at.z + dm(3))
                ),
                "the cat's way is blocked at {at:?}"
            );
        }
    }

    #[test]
    fn the_landing_keeps_its_fountain_and_both_pickups_on_clear_floor() {
        let a = home("rest", 1);
        assert_eq!(a.fountain, Some(v3(0, 0, 0)));
        let reached = walk(&a.level, a.start);
        for at in [v3(0, 0, 0), v3(-int(4), 0, int(5)), v3(int(4), 0, int(5))] {
            assert_eq!(
                a.level.floor_below(at.x, ONE, at.z),
                Some(0),
                "{at:?} is not on the floor"
            );
            assert!(clear(&a, at), "{at:?} has a solid on it");
            assert!(
                reached.iter().any(|p| p.sub(at).len_xz() < ONE && p.y == 0),
                "{at:?} cannot be walked to"
            );
        }
    }

    /// Does a sphere touch a solid that stands higher than `above`?
    fn sphere_meets_above(a: &Arena, c: V3, r: i32, above: i32) -> Option<V3> {
        a.level
            .solids
            .iter()
            .filter(|s| s.max.y > above)
            .find(|s| {
                let d = s.nearest(c).sub(c);
                let d2 = d.x as i64 * d.x as i64 + d.y as i64 * d.y as i64 + d.z as i64 * d.z as i64;
                d2 < r as i64 * r as i64
            })
            .map(|s| s.centre())
    }

    #[test]
    fn downstairs_has_room_for_chumbles_at_home_on_his_perches_and_for_his_yarn() {
        let c = content();
        let d = &c.enemies[c.enemy("chumbles").unwrap()];
        for seed in 1..=4 {
            let a = home("boss", seed);
            // where he sits at first, full size: clear 4 m round and 6 m up
            let h = crate::world::CAT_HOME;
            assert!(
                !a.level.blocked(
                    v3(h.x - int(4), h.y + ONE / 20, h.z - int(4)),
                    v3(h.x + int(4), h.y + int(6), h.z + int(4))
                ),
                "seed {seed}: something stands where Chumbles sits"
            );
            assert_eq!(a.level.floor_below(h.x, int(3), h.z), Some(0));
            // the floor reaches 34 m round the middle
            for k in 0..16 {
                let ang = fx::TURN / 16 * k;
                let (x, z) = (fx::mul(fx::cos(ang), int(34)), fx::mul(fx::sin(ang), int(34)));
                assert!(
                    a.level.floor_below(x, int(20), z).is_some(),
                    "no floor at ({x}, {z})"
                );
            }
            // his perches: on top of a solid, his body clear over each
            assert!(a.perches.len() >= 6, "seed {seed}: {} perches", a.perches.len());
            for p in &a.perches {
                assert_eq!(
                    a.level.floor_below(p.x, p.y + ONE / 10, p.z),
                    Some(p.y),
                    "seed {seed}: the perch at {p:?} stands on nothing"
                );
                let body = p.add(v3(0, d.hover, 0));
                if let Some(s) = sphere_meets_above(&a, body, d.radius, p.y) {
                    panic!("seed {seed}: at the perch {p:?} his body meets the solid at {s:?}");
                }
            }
            // somewhere for the yarn balls he calls
            assert!(
                a.ground_spawns.len() >= 30,
                "seed {seed}: {} spawns",
                a.ground_spawns.len()
            );
        }
    }

    #[test]
    fn downstairs_leaves_long_lanes_for_a_charge() {
        // from where he sits, south past you, and across the room
        let a = home("boss", 1);
        let r = int(2);
        for (from, to) in [
            (v3(0, 0, -int(20)), v3(0, 0, int(30))),
            (v3(-int(30), 0, -int(18)), v3(int(30), 0, -int(18))),
            (v3(-int(10), 0, int(26)), v3(int(10), 0, -int(20))),
        ] {
            for k in 0..=40 {
                let at = from.lerp(to, ONE * k / 40).add(v3(0, ONE * 21 / 10, 0));
                assert!(!a.level.sphere_hits(at, r), "the lane is blocked at {at:?}");
            }
        }
    }
}
