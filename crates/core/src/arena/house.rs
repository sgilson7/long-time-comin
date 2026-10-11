//! Zone three, the Glass House (2056): the rooms of the house on Calder
//! Hill, where the Defendant spent ten years and a fortune on laboratories.
//! The square and its gates are zone one's, so the route, the spawn grid
//! and the reach tests work unchanged; the rooms are walled and roofed
//! (15 m up, so the camera has room), after Resident Evil's mansion, and
//! the laboratories after Ultrakill's.
//!
//! - the Great Hall: twin stairs to a gallery round three walls, a statue,
//!   a chandelier, pillars
//! - the Dining Room: a long table to stand on, sideboards, two fireplaces,
//!   a minstrels' gallery
//! - the Library: aisles of tall shelves (lanes), a mezzanine on two sides
//! - the East Laboratory: a floor broken through into the basement lab,
//!   specimen tanks below, a bridge across, catwalks above
//! - the Long Gallery: display cases, a dais down the middle, an upper walk
//!
//! and the Vestibule (arrival), the Safe Room (the rest, a typewriter), the
//! Ballroom (the Section Chief) and the Conservatory (the Executor), whose
//! north wall is glass (`Pane`s, drawn apart so they can shatter) with the
//! lawn beyond.

use super::{boxed, dm, m, Pane, Side, B, COURT_RADIUS, GATE_GAP, HALF};
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine, V3};
use crate::level::Solid;
use crate::mesh::Mesh;
use crate::render::shade;

/// The rooms' ceiling.
pub const CEILING: i32 = fx::int(15);
/// The walls' inner face stands this far out from the square's edge, so a
/// gate stands in its doorway.
const WALL_OUT: i32 = fx::int(1);

#[allow(clippy::too_many_arguments)]
impl B<'_> {
    pub(super) fn house(&mut self, kind: &str, exits: &[Side]) {
        if kind == "boss" {
            self.conservatory();
            return;
        }
        let pattern = match kind {
            "dining" | "rest" => 1,
            "library" | "gallery" => 2,
            "lab" => 3,
            _ => 0,
        };
        self.house_floor(HALF, pattern, kind == "lab");
        let gaps = [
            true,
            exits.contains(&Side::North),
            exits.contains(&Side::East),
            exits.contains(&Side::West),
        ];
        let paper = match kind {
            "library" | "dining" => self.c("house_wallpaper_2"),
            "lab" => self.c("lab_white"),
            _ => self.c("house_wallpaper"),
        };
        self.house_walls(HALF, gaps, paper, kind == "lab");
        self.ceiling(HALF, kind == "lab");
        match kind {
            "arrival" => self.vestibule(),
            "rest" => self.safe_room(),
            "ballroom" => self.ballroom(),
            "dining" => self.dining(),
            "library" => self.library(),
            "lab" => self.lab(),
            "gallery" => self.gallery(),
            _ => self.great_hall(),
        }
        if matches!(kind, "hall" | "dining" | "gallery" | "ballroom") {
            self.cover_walls(HALF, 4);
        }
        // the bound: nobody leaves through a wall, or over one
        let o = HALF + m(8);
        let t = m(1);
        self.wall(v3(-o - t, -m(8), -o - t), v3(o + t, m(40), -o));
        self.wall(v3(-o - t, -m(8), o), v3(o + t, m(40), o + t));
        self.wall(v3(-o - t, -m(8), -o), v3(-o, m(40), o));
        self.wall(v3(o, -m(8), -o), v3(o + t, m(40), o));
    }

    // ---- the shell of a room ---------------------------------------------

    /// The floor: marble in a checker (0), parquet (1), carpet over boards
    /// (2), or white tile (3). `pit` leaves the lab's hole.
    fn house_floor(&mut self, half: i32, pattern: i32, pit: bool) {
        // out under the doorways' passages too
        let o = half + m(7);
        let (base, alt) = match pattern {
            1 => (self.c("house_wood"), shade(self.c("house_wood"), 80)),
            2 => (self.c("house_wood_dark"), self.c("house_carpet")),
            3 => (self.c("lab_white"), self.c("lab_steel")),
            _ => (self.c("house_marble"), self.c("house_marble_dark")),
        };
        if pit {
            // the floor round the hole, and the basement under it
            let (x0, x1, z0, z1) = LAB_PIT;
            // solid down past the basement's floor, so the hole has walls
            let deep = -LAB_DEPTH - m(1);
            for (a, b) in [
                (v3(-o, deep, -o), v3(o, 0, z0)),
                (v3(-o, deep, z1), v3(o, 0, o)),
                (v3(-o, deep, z0), v3(x0, 0, z1)),
                (v3(x1, deep, z0), v3(o, 0, z1)),
            ] {
                self.solids.push(crate::level::Solid::new(a, b));
                let (a, b) = (v3(a.x, -m(1), a.z), b);
                let c = a.lerp(b, ONE / 2);
                self.shape(Mesh::Cube, boxed(c, b.sub(a)), base, 0, false);
            }
            let low = -LAB_DEPTH;
            self.solids
                .push(crate::level::Solid::new(v3(x0, low - m(1), z0), v3(x1, low, z1)));
            let dark = self.c("lab_steel_dark");
            self.shape(
                Mesh::Cube,
                boxed(v3((x0 + x1) / 2, low - dm(5), (z0 + z1) / 2), v3(x1 - x0, m(1), z1 - z0)),
                dark,
                0,
                false,
            );
            // the hole's walls, down to the basement
            for (cx, cz, w, d) in [
                ((x0 + x1) / 2, z0, x1 - x0, dm(2)),
                ((x0 + x1) / 2, z1, x1 - x0, dm(2)),
                (x0, (z0 + z1) / 2, dm(2), z1 - z0),
                (x1, (z0 + z1) / 2, dm(2), z1 - z0),
            ] {
                self.shape(Mesh::Cube, boxed(v3(cx, -LAB_DEPTH / 2, cz), v3(w, LAB_DEPTH, d)), shade(dark, 120), 0, true);
            }
            // the broken edge: torn boards and hazard tape
            let tape = self.c("lab_hazard");
            for (a, b) in [
                (v3(x0, 0, z0), v3(x1, 0, z0)),
                (v3(x0, 0, z1), v3(x1, 0, z1)),
                (v3(x0, 0, z0), v3(x0, 0, z1)),
                (v3(x1, 0, z0), v3(x1, 0, z1)),
            ] {
                self.rod(a.add(v3(0, dm(1), 0)), b.add(v3(0, dm(1), 0)), dm(2), tape, true);
                self.rod(a.add(v3(0, -LAB_DEPTH / 2, 0)), b.add(v3(0, -LAB_DEPTH / 2, 0)), dm(3), dark, false);
            }
            for k in 0..10 {
                let x = x0 + (x1 - x0) / 10 * k + dm(5);
                let len = self.metres(1, 2);
                let board = Affine::translate(v3(x, -len / 3, z0 + dm(3)))
                    .then(&Affine::rot_x(deg(25 + 5 * (k % 3))))
                    .then(&Affine::scale(v3(dm(4), dm(1), len)));
                self.shape(Mesh::Cube, board, alt, 0, true);
            }
        } else {
            self.solids
                .push(crate::level::Solid::new(v3(-o, -m(1), -o), v3(o, 0, o)));
            self.shape(Mesh::Cube, boxed(v3(0, -dm(5), 0), v3(o * 2, m(1), o * 2)), base, 0, false);
        }
        // the pattern laid over it
        let tile = if pattern == 1 { m(1) } else { m(2) };
        let n = half * 2 / tile;
        for i in 0..n {
            for j in 0..n {
                let x = -half + tile * i + tile / 2;
                let z = -half + tile * j + tile / 2;
                if pit && in_pit(x, z, tile) {
                    continue;
                }
                let lay = match pattern {
                    0 | 3 => (i + j) % 2 == 0,
                    1 => (i / 2 + j) % 2 == 0 && j % 3 == 0,
                    _ => x.abs() < m(5) && z.abs() < half - m(3),
                };
                if !lay {
                    continue;
                }
                let size = if pattern == 2 {
                    v3(tile, dm(1) / 5, tile)
                } else {
                    v3(tile - dm(1) / 2, dm(1) / 5, tile - dm(1) / 2)
                };
                self.shape(Mesh::Cube, boxed(v3(x, dm(1) / 10, z), size), alt, 0, false);
            }
        }
    }

    /// The room's walls at `half + WALL_OUT`, with a doorway where each gap
    /// is: panelled to 1.4 m, papered above, pilasters, tall moonlit
    /// windows, portraits and sconces, a cornice.
    fn house_walls(&mut self, half: i32, gaps: [bool; 4], paper: Rgb, lab: bool) {
        let sides = [Side::South, Side::North, Side::East, Side::West];
        let face = half + WALL_OUT;
        let panel = if lab {
            self.c("lab_steel")
        } else {
            self.c("house_panel")
        };
        let trim = if lab {
            self.c("lab_steel_dark")
        } else {
            self.c("house_wood_dark")
        };
        let brass = self.c("house_brass");
        let moon = self.c("house_moon");
        for (si, side) in sides.iter().enumerate() {
            // the inward normal, and the way along the wall
            let (n, along) = match side {
                Side::South => (v3(0, 0, -ONE), v3(ONE, 0, 0)),
                Side::North => (v3(0, 0, ONE), v3(ONE, 0, 0)),
                Side::East => (v3(-ONE, 0, 0), v3(0, 0, ONE)),
                Side::West => (v3(ONE, 0, 0), v3(0, 0, ONE)),
            };
            let at = |a: i32, y: i32, out: i32| -> V3 { n.scale(-(face + out)).add(along.scale(a)).add(v3(0, y, 0)) };
            let size = |len: i32, h: i32, thick: i32| -> V3 {
                if along.x != 0 {
                    v3(len, h, thick)
                } else {
                    v3(thick, h, len)
                }
            };
            let gap = gaps[si];
            let gw = GATE_GAP / 2 + dm(5);
            // the wall in pieces either side of a doorway
            let spans: Vec<(i32, i32)> = if gap {
                vec![(-face - m(1), -gw), (gw, face + m(1))]
            } else {
                vec![(-face - m(1), face + m(1))]
            };
            for (a0, a1) in spans {
                let mid = (a0 + a1) / 2;
                let len = a1 - a0;
                let c = at(mid, 0, m(1) / 2);
                let sz = size(len, CEILING, m(1));
                self.solids.push(Solid::stand(c.x, 0, c.z, sz.x, sz.y, sz.z));
                self.deco(Mesh::Cube, c.add(v3(0, CEILING / 2, 0)), sz, paper);
                // the wainscot and its rail
                self.deco(Mesh::Cube, at(mid, dm(7), -dm(1)), size(len, dm(14), dm(2)), panel);
                self.deco(Mesh::Cube, at(mid, dm(14), -dm(2)), size(len, dm(1), dm(2)), trim);
            }
            // over the doorway, the wall goes on above the door
            if gap {
                let c = at(0, 0, m(1) / 2);
                let sz = size(gw * 2, CEILING - m(7), m(1));
                self.solids
                    .push(Solid::stand(c.x, m(7), c.z, sz.x, sz.y, sz.z));
                self.deco(Mesh::Cube, c.add(v3(0, m(7) + sz.y / 2, 0)), sz, paper);
                self.doorway(at(0, 0, 0), n, along, size(1, 1, 1), lab);
            }
            // the cornice
            self.deco(Mesh::Cube, at(0, CEILING - dm(4), -dm(2)), size(face * 2, dm(8), dm(4)), trim);
            // pilasters, windows between them, portraits and sconces low
            let step = m(7);
            let mut a = -face + step / 2;
            let mut k = 0;
            while a < face {
                let in_door = gap && a.abs() < gw + m(2);
                if !in_door {
                    self.deco(Mesh::Cube, at(a, CEILING / 2, -dm(2)), size(dm(8), CEILING, dm(4)), trim);
                }
                let mid = a + step / 2;
                if mid < face - m(2) && !(gap && mid.abs() < gw + m(3)) {
                    if k % 2 == 0 && !lab {
                        // a tall window: moonlight in a frame, mullions
                        self.glow(Mesh::Cube, at(mid, m(8), -dm(1)), size(m(3), m(7), dm(1)), moon);
                        self.deco(Mesh::Cube, at(mid, m(8), -dm(2)), size(dm(2), m(7), dm(2)), trim);
                        self.deco(Mesh::Cube, at(mid, m(8), -dm(2)), size(m(3), dm(2), dm(2)), trim);
                        self.deco(Mesh::Cube, at(mid, m(4) + dm(4), -dm(4)), size(m(3) + dm(6), dm(3), dm(6)), trim);
                        // its arch
                        let arch = Affine::translate(at(mid, m(11) + dm(5), -dm(1)))
                            .then(&Affine::rot_x(if along.x != 0 { fx::QUARTER } else { 0 }))
                            .then(&Affine::rot_z(if along.x != 0 { 0 } else { fx::QUARTER }))
                            .then(&Affine::scale(v3(m(3), dm(1), m(3))));
                        self.shape(Mesh::Cyl, arch, moon, ONE * 3 / 4, false);
                    } else if lab {
                        // a lab's observation window and its sign
                        let fluid = self.c("lab_fluid");
                        self.glow(Mesh::Cube, at(mid, m(4), -dm(1)), size(m(4), m(2), dm(1)), shade(fluid, 60));
                        self.deco(Mesh::Cube, at(mid, m(4), -dm(2)), size(m(4) + dm(4), dm(2), dm(3)), trim);
                        self.deco(Mesh::Cube, at(mid, m(6) + dm(5), -dm(2)), size(m(2), dm(5), dm(1)), self.c("lab_hazard"));
                    } else {
                        // a portrait over the panelling
                        let frame = self.c("house_brass");
                        let canvas = self.pick(&[self.c("house_wallpaper_2"), self.c("house_book"), self.c("house_book_2")]);
                        self.deco(Mesh::Cube, at(mid, m(4), -dm(1)), size(m(2) + dm(4), m(3) + dm(4), dm(2)), frame);
                        self.deco(Mesh::Cube, at(mid, m(4), -dm(2)), size(m(2), m(3), dm(1)), canvas);
                        self.deco(Mesh::Sphere, at(mid, m(4) + dm(5), -dm(3)), size(dm(8), dm(9), dm(1)), self.c("agent_skin"));
                    }
                    // sconces either side
                    for s in [-1, 1] {
                        let p = at(mid + s * m(2), m(3), -dm(3));
                        self.deco(Mesh::Cube, p, size(dm(2), dm(4), dm(2)), brass);
                        self.glow(Mesh::Blob, p.add(v3(0, dm(4), 0)), v3(dm(2), dm(3), dm(2)), self.c("house_candle"));
                    }
                }
                a += step;
                k += 1;
            }
        }
    }

    /// A doorway at `at` (on the wall's inner face), its doors swung open
    /// on a dark passage that ends in a shut door.
    fn doorway(&mut self, at: V3, n: V3, along: V3, _unit: V3, lab: bool) {
        let trim = if lab {
            self.c("lab_steel_dark")
        } else {
            self.c("house_wood_dark")
        };
        let wood = if lab {
            self.c("lab_steel")
        } else {
            self.c("house_wood")
        };
        let brass = self.c("house_brass");
        let w = GATE_GAP / 2 + dm(5);
        let sz = |len: i32, h: i32, thick: i32| -> V3 {
            if along.x != 0 {
                v3(len, h, thick)
            } else {
                v3(thick, h, len)
            }
        };
        // the frame: two pilasters and a pediment
        for s in [-1, 1] {
            let p = at.add(along.scale(s * w)).add(n.scale(dm(2)));
            self.deco(Mesh::Cube, p.add(v3(0, m(7) / 2, 0)), sz(dm(8), m(7), dm(6)), trim);
            self.deco(Mesh::Blob, p.add(v3(0, m(7) + dm(3), 0)), v3(dm(6), dm(6), dm(6)), brass);
        }
        self.deco(Mesh::Cube, at.add(v3(0, m(7) + dm(3), 0)).add(n.scale(dm(2))), sz(w * 2 + dm(8), dm(6), dm(6)), trim);
        let ped = Affine::translate(at.add(v3(0, m(8), 0)).add(n.scale(dm(2))))
            .then(&Affine::rot_y(if along.x != 0 { 0 } else { fx::QUARTER }))
            .then(&Affine::scale(v3(w * 2, m(1), dm(5))));
        self.shape(Mesh::Wedge, ped, trim, 0, true);
        // the doors, swung back against the passage's sides
        for s in [-1, 1] {
            let p = at.add(along.scale(s * (w - dm(2)))).add(n.scale(-m(1)));
            self.deco(Mesh::Cube, p.add(v3(0, m(3), 0)), sz(dm(1), m(6), m(2)), wood);
        }
        // the passage: dark sides and a shut door at its end
        let dark = self.c("house_wood_dark");
        let back = at.add(n.scale(-m(4)));
        self.solids.push(Solid::stand(back.x, 0, back.z, sz(w * 2, 1, 1).x.max(dm(5)), m(7), sz(w * 2, 1, 1).z.max(dm(5))));
        self.deco(Mesh::Cube, back.add(v3(0, m(3) + dm(5), 0)), sz(w * 2, m(7), dm(5)), shade(dark, 70));
        for s in [-1, 1] {
            let p = at.add(along.scale(s * (w + dm(3)))).add(n.scale(-m(2)));
            self.solids.push(Solid::stand(p.x, 0, p.z, sz(1, 1, m(4)).x.max(dm(5)), m(7), sz(1, 1, m(4)).z.max(dm(5))));
        }
    }

    /// The ceiling 15 m up: beams in a grid, and a lab's lamps.
    fn ceiling(&mut self, half: i32, lab: bool) {
        let o = half + m(3);
        self.solids
            .push(Solid::new(v3(-o, CEILING, -o), v3(o, CEILING + m(1), o)));
        let wood = if lab {
            self.c("lab_white")
        } else {
            self.c("house_wood_dark")
        };
        self.shape(
            Mesh::Cube,
            boxed(v3(0, CEILING + dm(5), 0), v3(o * 2, m(1), o * 2)),
            shade(wood, 80),
            0,
            false,
        );
        let step = m(8);
        let mut a = -half;
        while a <= half {
            self.deco(Mesh::Cube, v3(a, CEILING - dm(3), 0), v3(dm(5), dm(6), o * 2), wood);
            self.deco(Mesh::Cube, v3(0, CEILING - dm(3), a), v3(o * 2, dm(6), dm(5)), wood);
            a += step;
        }
        if lab {
            let lamp = self.c("lab_white");
            for k in 0..6 {
                for j in 0..2 {
                    let p = v3(-m(20) + m(8) * k, CEILING - dm(8), -m(8) + m(16) * j);
                    self.glow(Mesh::Cube, p, v3(m(4), dm(2), dm(6)), lamp);
                }
            }
        }
    }

    /// A chandelier hanging from the ceiling over `(x, z)`, its lowest
    /// candles at `y`.
    fn chandelier(&mut self, x: i32, y: i32, z: i32, r: i32) {
        let brass = self.c("house_brass");
        let candle = self.c("house_candle");
        self.rod(v3(x, CEILING, z), v3(x, y + m(1), z), dm(1), brass, false);
        for (k, rr) in [(0, r), (1, r * 6 / 10)] {
            let ring = boxed(v3(x, y + m(1) * k, z), v3(rr * 2, dm(2), rr * 2));
            self.shape(Mesh::Ring, ring, brass, ONE / 4, true);
            let n = 8 - 3 * k;
            for i in 0..n {
                let a = fx::TURN / n * i;
                let p = v3(x + fx::mul(fx::cos(a), rr), y + m(1) * k + dm(2), z + fx::mul(fx::sin(a), rr));
                self.glow(Mesh::Blob, p, v3(dm(2), dm(3), dm(2)), candle);
            }
        }
        self.halo(
            Mesh::Sphere,
            boxed(v3(x, y + dm(5), z), v3(r * 3, r * 2, r * 3)),
            candle,
            ONE / 8,
        );
    }

    /// A pillar from the floor to the ceiling, solid, with a base and a
    /// capital.
    fn pillar(&mut self, x: i32, z: i32, w: i32) {
        let marble = self.c("house_marble");
        let trim = self.c("house_wood_dark");
        self.solids.push(Solid::stand(x, 0, z, w, CEILING, w));
        self.deco(Mesh::Cyl, v3(x, CEILING / 2, z), v3(w, CEILING, w), marble);
        self.deco(Mesh::Cube, v3(x, dm(4), z), v3(w + dm(4), dm(8), w + dm(4)), trim);
        self.deco(Mesh::Cube, v3(x, CEILING - m(1), z), v3(w + dm(6), dm(8), w + dm(6)), trim);
    }

    /// A statue on a plinth: solid, cover.
    fn statue(&mut self, x: i32, z: i32, yaw: i32) {
        let marble = self.c("house_marble");
        let dark = self.c("house_marble_dark");
        self.block(x, 0, z, m(2), m(2), m(2), dark);
        let f = Affine::translate(v3(x, m(2), z)).then(&Affine::rot_y(yaw));
        self.shape(Mesh::Cone, f.then(&boxed(v3(0, dm(9), 0), v3(dm(9), dm(18), dm(9)))), marble, 0, true);
        self.shape(Mesh::Sphere, f.then(&boxed(v3(0, m(2), 0), v3(dm(4), dm(5), dm(4)))), marble, 0, true);
        // an arm raised, holding up an hourglass
        self.shape(
            Mesh::Cube,
            f.then(&Affine::translate(v3(dm(4), dm(17), 0)))
                .then(&Affine::rot_z(-deg(30)))
                .then(&Affine::scale(v3(dm(2), dm(8), dm(2)))),
            marble,
            0,
            true,
        );
        for s in [-1, 1] {
            self.shape(Mesh::Cone, f.then(&boxed(v3(dm(7), dm(26) + s * dm(1), 0), v3(dm(3), dm(2), dm(3)))), self.c("house_brass"), 0, true);
        }
    }

    /// A grandfather clock against a wall, stopped at 21:14.
    fn clock(&mut self, x: i32, z: i32, facing_z: bool) {
        let wood = self.c("house_wood");
        let face = self.c("house_cream");
        let ink = self.c("house_ink");
        self.block(x, 0, z, m(1), m(3), dm(6), wood);
        let n = if facing_z { v3(0, 0, ONE) } else { v3(ONE, 0, 0) };
        let c = v3(x, m(2) + dm(4), z).add(n.scale(dm(4)));
        self.shape(
            Mesh::Cyl,
            Affine::translate(c)
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&Affine::scale(v3(dm(7), dm(1), dm(7)))),
            face,
            ONE / 3,
            true,
        );
        for (angle, len) in [(-(deg(270) + deg(7)), dm(2)), (-deg(84), dm(3))] {
            let tip = c.add(v3(fx::mul(fx::sin(-angle), len), fx::mul(fx::cos(-angle), len), dm(1)));
            self.rod(c.add(n.scale(dm(1))), tip, dm(1) / 3, ink, false);
        }
    }

    /// An armchair, only looked at.
    fn armchair(&mut self, x: i32, z: i32, yaw: i32) {
        let cloth = self.c("house_carpet");
        let wood = self.c("house_wood_dark");
        let f = Affine::translate(v3(x, 0, z)).then(&Affine::rot_y(yaw));
        self.shape(Mesh::Cube, f.then(&boxed(v3(0, dm(3), 0), v3(dm(9), dm(4), dm(8)))), cloth, 0, true);
        self.shape(Mesh::Cube, f.then(&boxed(v3(0, dm(8), dm(4)), v3(dm(9), dm(10), dm(2)))), cloth, 0, true);
        for s in [-1, 1] {
            self.shape(Mesh::Cube, f.then(&boxed(v3(s * dm(5), dm(5), 0), v3(dm(2), dm(4), dm(8)))), wood, 0, true);
        }
    }

    /// A bookcase, solid, its shelves of books in three colours.
    fn bookcase(&mut self, x: i32, z: i32, len: i32, h: i32, along_x: bool) {
        let wood = self.c("house_wood_dark");
        let (w, d) = if along_x { (len, dm(10)) } else { (dm(10), len) };
        self.block(x, 0, z, w, h, d, wood);
        let books = [self.c("house_book"), self.c("house_book_2"), self.c("house_book_3")];
        let shelves = h / m(1);
        for s in 0..shelves {
            let y = m(1) * s + dm(5);
            for side in [-1, 1] {
                let n = (len / m(2)).max(1);
                for k in 0..n {
                    let a = -len / 2 + m(2) * k + m(1);
                    let c = books[((s + k + side.max(0)) % 3) as usize];
                    let (p, size) = if along_x {
                        (v3(x + a, y, z + side * (d / 2 + dm(1) / 4)), v3(m(2) - dm(2), dm(7), dm(1) / 2))
                    } else {
                        (v3(x + side * (w / 2 + dm(1) / 4), y, z + a), v3(dm(1) / 2, dm(7), m(2) - dm(2)))
                    };
                    self.shape(Mesh::Cube, boxed(p, size), c, 0, false);
                }
            }
        }
    }

    /// A lab bench, solid, with glassware and a lamp.
    fn bench_lab(&mut self, x: i32, z: i32, along_x: bool) {
        let steel = self.c("lab_steel");
        let white = self.c("lab_white");
        let (w, d) = if along_x { (m(4), dm(12)) } else { (dm(12), m(4)) };
        self.block(x, 0, z, w, dm(10), d, steel);
        self.deco(Mesh::Cube, v3(x, dm(10) + dm(1) / 2, z), v3(w + dm(1), dm(1), d + dm(1)), white);
        let fluid = self.c("lab_fluid");
        for k in [-1, 0, 1] {
            let (px, pz) = if along_x { (x + k * m(1), z) } else { (x, z + k * m(1)) };
            let r = dm(1) + dm(1) * (k + 1) / 2;
            self.glow(Mesh::Cyl, v3(px, dm(12) + r, pz), v3(r, r * 2, r), shade(fluid, 80 + 10 * k));
        }
    }

    /// A specimen tank: a glass cylinder of green fluid, something dark
    /// asleep in it, solid.
    fn tank_specimen(&mut self, x: i32, y: i32, z: i32) {
        let steel = self.c("lab_steel_dark");
        let fluid = self.c("lab_fluid");
        let ink = self.c("house_ink");
        self.solids.push(Solid::stand(x, y, z, dm(16), m(4), dm(16)));
        self.deco(Mesh::Cyl, v3(x, y + dm(3), z), v3(dm(22), dm(6), dm(22)), steel);
        self.deco(Mesh::Cyl, v3(x, y + m(4) - dm(2), z), v3(dm(22), dm(4), dm(22)), steel);
        self.glow(Mesh::Cyl, v3(x, y + m(2), z), v3(dm(18), m(3), dm(18)), shade(fluid, 70));
        self.shape(Mesh::Cone, boxed(v3(x, y + m(2), z), v3(dm(5), dm(14), dm(5))), ink, 0, false);
        self.shape(Mesh::Sphere, boxed(v3(x, y + m(3), z), v3(dm(4), dm(4), dm(4))), ink, 0, false);
        self.rod(v3(x, y + m(4), z), v3(x, CEILING, z), dm(1), steel, false);
    }

    // ---- the rooms ----------------------------------------------------------

    fn great_hall(&mut self) {
        // the gallery round three walls, 6 m up
        let gh = m(6);
        self.deck(-m(28), -m(28), m(28), -m(22), gh, &[(-m(14), -m(25)), (m(14), -m(25))]);
        self.deck(-m(28), -m(22), -m(22), m(12), gh, &[(-m(25), -m(8)), (-m(25), m(6))]);
        self.deck(m(22), -m(22), m(28), m(12), gh, &[(m(25), -m(8)), (m(25), m(6))]);
        // the twin stairs, up from the floor to the gallery's front edge
        let steps = (gh + dm(4) - 1) / dm(4);
        for sx in [-1, 1] {
            self.stairs(sx * m(8), 0, -m(22) + dm(6) * steps, 0, -ONE, gh, m(3));
        }
        // the statue between them, the chandelier over the floor
        self.statue(0, -m(14), 0);
        self.chandelier(0, m(9), 0, m(3));
        for (x, z) in [(-m(12), -m(4)), (m(12), -m(4)), (-m(12), m(10)), (m(12), m(10))] {
            self.pillar(x, z, dm(18));
        }
        self.clock(-m(20), m(22), false);
        for (x, z, yaw) in [(m(18), m(18), deg(200)), (m(20), m(15), deg(250)), (-m(17), m(17), deg(120))] {
            self.armchair(x, z, yaw);
        }
        // a carpet runner from the door to the stairs
        let carpet = self.c("house_carpet");
        self.shape(Mesh::Cube, boxed(v3(0, dm(1) / 5, m(4)), v3(m(4), dm(1) / 5, m(40))), carpet, 0, false);
    }

    fn dining(&mut self) {
        // the long table: a jump up to stand on, cover to crouch at
        let wood = self.c("house_wood");
        let cream = self.c("house_cream");
        self.block(0, 0, -m(2), m(4), dm(9), m(30), wood);
        self.deco(Mesh::Cube, v3(0, dm(9) + dm(1) / 4, -m(2)), v3(m(3), dm(1) / 2, m(29)), cream);
        for k in 0..7 {
            let z = -m(15) + m(4) * k + m(1);
            self.glow(Mesh::Cyl, v3(0, m(1) + dm(2), z), v3(dm(1), dm(4), dm(1)), self.c("house_candle"));
            for s in [-1, 1] {
                // a chair pushed back
                self.deco(Mesh::Cube, v3(s * m(3), dm(5), z), v3(dm(6), dm(1), dm(6)), self.c("house_wood_dark"));
                self.deco(Mesh::Cube, v3(s * (m(3) + dm(3)), dm(10), z), v3(dm(1), dm(10), dm(6)), self.c("house_wood_dark"));
            }
        }
        // sideboards and china cabinets along the walls
        for s in [-1, 1] {
            for k in 0..3 {
                let z = -m(16) + m(12) * k;
                self.block(s * m(26), 0, z, dm(10), dm(12), m(4), self.c("house_wood_dark"));
                self.block(s * m(26), 0, z + m(4), dm(10), dm(22), m(2), self.c("house_wood"));
            }
            // a fireplace, its chimney breast to the ceiling
            let x = s * m(27);
            self.block(x, 0, m(10), m(2), m(15), m(5), self.c("house_marble"));
            self.glow(Mesh::Cube, v3(x - s * m(1), dm(6), m(10)), v3(dm(2), dm(10), m(2)), self.c("house_candle"));
        }
        // the minstrels' gallery at the north, stairs down both ends
        let gh = m(5);
        self.deck(-m(20), -m(28), m(20), -m(21), gh, &[(-m(8), -m(24)), (m(8), -m(24))]);
        let steps = (gh + dm(4) - 1) / dm(4);
        for s in [-1, 1] {
            self.stairs(s * m(18), 0, -m(21) + dm(6) * steps, 0, -ONE, gh, m(3));
        }
        for (x, z) in [(-m(10), -m(18)), (m(10), -m(18)), (-m(10), m(14)), (m(10), m(14))] {
            self.pillar(x, z, dm(16));
        }
        self.chandelier(0, m(8), -m(9), m(2) + dm(5));
        self.chandelier(0, m(8), m(5), m(2) + dm(5));
    }

    fn library(&mut self) {
        // aisles of shelves running east to west, a lane down the middle
        for row in 0..5 {
            let z = -m(16) + m(7) * row;
            for s in [-1, 1] {
                self.bookcase(s * m(12), z, m(12), dm(45), true);
            }
        }
        // the mezzanine on both sides, 5 m up, with stairs at its south ends
        let mh = m(5);
        self.deck(-m(28), -m(26), -m(21), m(14), mh, &[(-m(24), -m(10)), (-m(24), m(4))]);
        self.deck(m(21), -m(26), m(28), m(14), mh, &[(m(24), -m(10)), (m(24), m(4))]);
        let steps = (mh + dm(4) - 1) / dm(4);
        for s in [-1, 1] {
            self.stairs(s * m(24), 0, m(14) + dm(6) * steps, 0, -ONE, mh, m(3));
        }
        // shelves on the mezzanines' walls
        for s in [-1, 1] {
            for k in 0..3 {
                self.bookcase(s * m(28), -m(20) + m(12) * k, m(10), m(4), false);
            }
        }
        // a reading desk with a lamp and a globe at the north
        self.block(0, 0, -m(24), m(5), dm(9), m(2), self.c("house_wood"));
        self.glow(Mesh::Blob, v3(m(1), m(1) + dm(3), -m(24)), v3(dm(4), dm(4), dm(4)), self.c("house_candle"));
        self.deco(Mesh::Sphere, v3(-m(5), m(1) + dm(4), -m(23)), v3(m(1), m(1), m(1)), self.c("house_book_2"));
        self.chandelier(0, m(9), -m(2), m(2));
    }

    fn lab(&mut self) {
        // below: specimen tanks and the basement's instruments
        let (x0, x1, z0, _) = LAB_PIT;
        let low = -LAB_DEPTH;
        for (x, z) in [(-m(6), -m(9)), (-m(2), -m(9)), (m(2), -m(9)), (m(6), -m(9)), (-m(7), m(4)), (m(7), m(4))] {
            self.tank_specimen(x, low, z);
        }
        self.solids.push(Solid::stand(0, low, m(4), m(4), dm(12), m(2)));
        self.deco(Mesh::Cube, v3(0, low + dm(6), m(4)), v3(m(4), dm(12), m(2)), self.c("lab_steel"));
        // ramps of steps down on the east and west
        let steps = (LAB_DEPTH + dm(4) - 1) / dm(4);
        for s in [-1, 1] {
            let x = if s < 0 { x0 + m(2) } else { x1 - m(2) };
            self.stairs(x, low, z0 + dm(6) * steps, 0, -ONE, LAB_DEPTH, m(3));
        }
        // a bridge across the hole at floor height
        self.deck(x0 - m(1), -m(3) - dm(5), x1 + m(1), -m(1) + dm(5), 0, &[]);
        // catwalks along the north and both sides, 5 m up
        let ch = m(5);
        self.deck(-m(28), -m(28), m(28), -m(23), ch, &[(-m(18), -m(25)), (m(18), -m(25))]);
        self.deck(-m(28), -m(23), -m(23), m(10), ch, &[(-m(25), -m(5))]);
        self.deck(m(23), -m(23), m(28), m(10), ch, &[(m(25), -m(5))]);
        let st = (ch + dm(4) - 1) / dm(4);
        for s in [-1, 1] {
            self.stairs(s * m(25) + s * dm(5), 0, m(10) + dm(6) * st, 0, -ONE, ch, m(3));
        }
        // benches on the floor round the hole
        for (x, z, along) in [
            (-m(18), m(16), true),
            (m(18), m(16), true),
            (-m(18), -m(17), false),
            (m(18), -m(17), false),
            (0, m(18), true),
        ] {
            self.bench_lab(x, z, along);
        }
        // pipes along the ceiling, and a hazard sign
        let steel = self.c("lab_steel_dark");
        for k in 0..4 {
            let z = -m(18) + m(12) * k;
            self.rod(v3(-HALF, CEILING - m(2), z), v3(HALF, CEILING - m(2), z), dm(3), steel, false);
        }
    }

    fn gallery(&mut self) {
        // the dais down the middle, with steps at both ends
        let dh = dm(15);
        self.platform(0, -m(4), m(8), m(24), dh, (0, ONE));
        let steps = (dh + dm(4) - 1) / dm(4);
        self.stairs(0, 0, -m(16) - dm(6) * steps, 0, ONE, dh, m(3));
        // display cases either side
        let glass = self.c("glass");
        for s in [-1, 1] {
            for k in 0..4 {
                let (x, z) = (s * m(11), -m(14) + m(8) * k);
                self.block(x, 0, z, m(2), dm(10), m(2), self.c("house_wood_dark"));
                self.solids.push(Solid::stand(x, dm(10), z, m(2), dm(8), m(2)));
                self.halo(Mesh::Cube, boxed(v3(x, dm(14), z), v3(m(2), dm(8), m(2))), glass, ONE / 5);
                let c = self.pick(&[self.c("house_brass"), self.c("lab_fluid"), self.c("house_book_2")]);
                self.glow(Mesh::Octa, v3(x, dm(14), z), v3(dm(4), dm(5), dm(4)), c);
            }
            self.statue(s * m(20), -m(18), if s < 0 { deg(90) } else { -deg(90) });
            self.statue(s * m(20), m(8), if s < 0 { deg(90) } else { -deg(90) });
        }
        // the upper walk along east and west, 5.5 m up
        let wh = m(5) + dm(5);
        self.deck(-m(28), -m(26), -m(23), m(16), wh, &[(-m(25), -m(6)), (-m(25), m(6))]);
        self.deck(m(23), -m(26), m(28), m(16), wh, &[(m(25), -m(6)), (m(25), m(6))]);
        let st = (wh + dm(4) - 1) / dm(4);
        for s in [-1, 1] {
            self.stairs(s * m(25) + s * dm(5), 0, m(16) + dm(6) * st, 0, -ONE, wh, m(3));
        }
        for k in 0..3 {
            self.chandelier(0, m(9), -m(16) + m(12) * k, m(2));
        }
    }

    fn vestibule(&mut self) {
        // the way in: a stair to a landing, coats, the clock, the cache
        let lh = m(4);
        self.deck(-m(28), -m(28), -m(14), -m(20), lh, &[(-m(21), -m(24))]);
        let steps = (lh + dm(4) - 1) / dm(4);
        self.stairs(-m(16), 0, -m(20) + dm(6) * steps, 0, -ONE, lh, m(3));
        self.clock(m(26), -m(6), false);
        for (x, z) in [(-m(9), -m(10)), (m(9), -m(10))] {
            self.pillar(x, z, dm(16));
        }
        let brass = self.c("house_brass");
        for k in 0..3 {
            let x = m(18) + m(2) * k;
            self.rod(v3(x, 0, m(20)), v3(x, m(2), m(20)), dm(1), brass, true);
            self.deco(Mesh::Cone, v3(x, m(1) + dm(3), m(20)), v3(dm(6), m(1), dm(6)), self.c("agent_suit"));
        }
        // a lab coat dropped on the floor, a clipboard beside it
        self.deco(Mesh::Cube, v3(-m(6), dm(1) / 2, m(6)), v3(m(1), dm(1) / 2, m(2)), self.c("lab_white"));
        self.deco(Mesh::Cube, v3(-m(5), dm(1) / 2, m(8)), v3(dm(4), dm(1) / 2, dm(5)), self.c("house_cream"));
        self.chandelier(0, m(8), -m(2), m(2));
    }

    fn safe_room(&mut self) {
        // the typewriter on its desk at the middle, a chest, a lamp, quiet
        let wood = self.c("house_wood");
        self.solids.push(Solid::stand(0, 0, 0, m(2), dm(9), m(1)));
        self.deco(Mesh::Cube, v3(0, dm(4), 0), v3(m(2), dm(9), m(1)), wood);
        self.deco(Mesh::Cube, v3(0, m(1), 0), v3(dm(6), dm(2), dm(4)), self.c("agent_suit_dark"));
        self.glow(Mesh::Cube, v3(0, m(1) + dm(2), -dm(1)), v3(dm(5), dm(1), dm(1)), self.c("house_cream"));
        self.glow(Mesh::Blob, v3(m(1), m(1) + dm(4), 0), v3(dm(5), dm(5), dm(5)), self.c("house_candle"));
        self.block(-m(8), 0, -m(10), m(3), m(1), m(2), self.c("house_wood_dark"));
        self.deco(Mesh::Cube, v3(-m(8), m(1), -m(10)), v3(m(3) + dm(2), dm(2), m(2) + dm(2)), self.c("house_brass"));
        for (x, z, yaw) in [(m(10), -m(8), deg(200)), (-m(12), m(10), deg(40))] {
            self.armchair(x, z, yaw);
        }
        self.bookcase(m(20), -m(20), m(8), m(4), true);
        self.bookcase(-m(20), -m(20), m(8), m(4), true);
        self.clock(m(26), m(4), false);
        self.chandelier(0, m(8), 0, m(2));
    }

    fn ballroom(&mut self) {
        // the dance floor, mirrors, a ring of pillars, the musicians' balcony
        for k in 0..10 {
            let a = fx::TURN / 10 * k + deg(18);
            let (x, z) = (fx::mul(fx::cos(a), m(19)), fx::mul(fx::sin(a), m(19)));
            if z > m(14) && x.abs() < m(8) {
                continue;
            }
            self.pillar(x, z, dm(16));
        }
        let gh = m(5);
        self.deck(-m(16), -m(28), m(16), -m(22), gh, &[(-m(6), -m(25)), (m(6), -m(25))]);
        let steps = (gh + dm(4) - 1) / dm(4);
        for s in [-1, 1] {
            self.stairs(s * m(14), 0, -m(22) + dm(6) * steps, 0, -ONE, gh, m(3));
        }
        // the piano on the balcony
        self.deco(Mesh::Cube, v3(m(6), gh + dm(5), -m(25)), v3(m(2), m(1), m(3)), self.c("agent_suit_dark"));
        let moon = self.c("house_moon");
        for s in [-1, 1] {
            for k in 0..3 {
                // mirrors catch the moonlight
                let z = -m(14) + m(12) * k;
                self.halo(Mesh::Cube, boxed(v3(s * (HALF + dm(8)), m(5), z), v3(dm(1), m(6), m(3))), moon, ONE / 4);
            }
        }
        for (x, z) in [(-m(8), 0), (m(8), 0), (0, -m(8))] {
            self.chandelier(x, m(9), z, m(2) + dm(5));
        }
        // a ring inlaid in the floor
        let brass = self.c("house_brass");
        self.shape(Mesh::Ring, boxed(v3(0, dm(1) / 4, 0), v3(m(24), dm(1) / 4, m(24))), brass, ONE / 6, false);
    }

    /// The Conservatory: the Executor's room. Walls of panelling and glass
    /// to the south, east and west, and to the north one wall of glass on
    /// the lawn, which the Executor comes through.
    fn conservatory(&mut self) {
        let r = COURT_RADIUS;
        let face = r + m(1);
        let wood = self.c("house_wood_dark");
        let paper = self.c("house_wallpaper_2");
        let glass = self.c("glass");
        let moon = self.c("house_moon");
        // the floor, tiled
        self.solids
            .push(Solid::new(v3(-m(60), -m(1), -m(60)), v3(m(60), 0, m(60))));
        self.shape(Mesh::Cube, boxed(v3(0, -dm(5), 0), v3(face * 2, m(1), face * 2)), self.c("house_marble"), 0, false);
        let dark = self.c("house_marble_dark");
        let tile = m(2);
        let n = face * 2 / tile;
        for i in 0..n {
            for j in 0..n {
                if (i + j) % 2 == 0 {
                    let (x, z) = (-face + tile * i + tile / 2, -face + tile * j + tile / 2);
                    self.shape(Mesh::Cube, boxed(v3(x, dm(1) / 10, z), v3(tile - dm(1), dm(1) / 5, tile - dm(1))), dark, 0, false);
                }
            }
        }
        // the lawn outside, hedges and trees, the night
        self.shape(Mesh::Ground, boxed(v3(0, -dm(2), 0), v3(m(1200), ONE, m(1200))), self.c("grass_night"), 0, false);
        let hedge = self.c("hedge");
        for k in 0..12 {
            let x = -m(66) + m(12) * k;
            self.deco(Mesh::Cube, v3(x, m(1), -r - m(22)), v3(m(9), m(2), m(2)), hedge);
        }
        for k in 0..9 {
            let x = -m(60) + m(15) * k + self.metres(-3, 3);
            let z = -r - m(34) - self.metres(0, 30);
            let h = self.metres(8, 16);
            self.rod(v3(x, 0, z), v3(x, h / 2, z), dm(6), self.c("house_wood_dark"), true);
            self.deco(Mesh::Blob, v3(x, h * 3 / 4, z), v3(m(6), h / 2, m(6)), hedge);
        }
        // the sides: panelled low, glass above, a glass roof
        for (si, (n, along)) in [
            (v3(0, 0, -ONE), v3(ONE, 0, 0)),
            (v3(-ONE, 0, 0), v3(0, 0, ONE)),
            (v3(ONE, 0, 0), v3(0, 0, ONE)),
        ]
        .iter()
        .enumerate()
        {
            let at = |a: i32, y: i32, out: i32| -> V3 { n.scale(-(face + out)).add(along.scale(a)).add(v3(0, y, 0)) };
            let size = |len: i32, h: i32, t: i32| if along.x != 0 { v3(len, h, t) } else { v3(t, h, len) };
            let c = at(0, 0, m(1) / 2);
            let sz = size(face * 2 + m(2), m(20), m(1));
            self.solids.push(Solid::stand(c.x, 0, c.z, sz.x, sz.y, sz.z));
            self.deco(Mesh::Cube, at(0, m(2), m(1) / 2), size(face * 2 + m(2), m(4), m(1)), paper);
            self.deco(Mesh::Cube, at(0, m(4), -dm(2)), size(face * 2, dm(3), dm(4)), wood);
            self.halo(Mesh::Cube, boxed(at(0, m(10), m(1) / 2), size(face * 2, m(12), dm(2))), moon, ONE / 10);
            let mut a = -face;
            while a <= face {
                self.deco(Mesh::Cube, at(a, m(10), 0), size(dm(3), m(12), dm(3)), wood);
                a += m(5);
            }
            let _ = si;
        }
        // the glass roof's ribs
        let mut a = -face;
        while a <= face {
            self.deco(Mesh::Cube, v3(a, m(16), 0), v3(dm(3), dm(4), face * 2), wood);
            a += m(6);
        }
        self.halo(Mesh::Cube, boxed(v3(0, m(16) + dm(2), 0), v3(face * 2, dm(1), face * 2)), moon, ONE / 12);
        // the north: one wall of glass, pane by pane, on iron mullions
        let z = -r - dm(5);
        let (cols, rows) = (15, 3);
        let pw = face * 2 / cols;
        let ph = m(4);
        for row in 0..rows {
            for col in 0..cols {
                let x = -face + pw * col + pw / 2;
                let y = ph * row + ph / 2;
                self.panes.push(Pane {
                    centre: v3(x, y, z),
                    size: v3(pw - dm(2), ph - dm(2), dm(1)),
                });
            }
        }
        let iron = self.c("tank_tread");
        for col in 0..=cols {
            let x = -face + pw * col;
            self.deco(Mesh::Cube, v3(x, ph * rows / 2, z), v3(dm(2), ph * rows, dm(3)), iron);
        }
        for row in 0..=rows {
            self.deco(Mesh::Cube, v3(0, ph * row, z), v3(face * 2, dm(2), dm(3)), iron);
        }
        let _ = glass;
        // the bound: nobody leaves, the Executor comes in from the north
        self.wall(v3(-r - m(10), -m(2), -r - m(2)), v3(r + m(10), m(40), -r - m(1)));
        self.wall(v3(-r - m(10), -m(2), r + m(4)), v3(r + m(10), m(40), r + m(10)));
        self.wall(v3(-r - m(10), -m(2), -r - m(10)), v3(-r - m(4), m(40), r + m(10)));
        self.wall(v3(r + m(4), -m(2), -r - m(10)), v3(r + m(10), m(40), r + m(10)));
        self.solids.push(Solid::new(v3(-face - m(2), m(17), -face - m(2)), v3(face + m(2), m(18), face + m(2))));
        // inside: palms in great pots round the edge (cover it cannot
        // reach), stone urns nearer in (it turns at them), a gallery south
        for k in 0..12 {
            let a = fx::TURN / 12 * k + deg(15);
            let (x, zz) = (fx::mul(fx::cos(a), m(25)), fx::mul(fx::sin(a), m(25)));
            if zz < -m(20) {
                continue;
            }
            self.block(x, 0, zz, m(2), dm(14), m(2), self.c("house_marble_dark"));
            self.rod(v3(x, dm(14), zz), v3(x, m(6), zz), dm(3), self.c("house_wood"), true);
            for f in 0..5 {
                let fa = fx::TURN / 5 * f;
                let tip = v3(x + fx::mul(fx::cos(fa), m(2)), m(5), zz + fx::mul(fx::sin(fa), m(2)));
                self.rod(v3(x, m(6), zz), tip, dm(3), self.c("hedge"), true);
            }
        }
        for k in 0..6 {
            let a = fx::TURN / 6 * k;
            let (x, zz) = (fx::mul(fx::cos(a), m(14)), fx::mul(fx::sin(a), m(14)));
            self.block(x, 0, zz, dm(14), m(2), dm(14), self.c("house_marble"));
            self.deco(Mesh::Sphere, v3(x, m(2) + dm(4), zz), v3(dm(16), dm(10), dm(16)), self.c("house_marble"));
        }
        let gh = m(5);
        self.deck(-m(22), m(24), m(22), m(30), gh, &[(-m(10), m(27)), (m(10), m(27))]);
        let steps = (gh + dm(4) - 1) / dm(4);
        for s in [-1, 1] {
            self.stairs(s * m(18), 0, m(24) - dm(6) * steps, 0, ONE, gh, m(3));
        }
    }
}

/// The lab's hole into the basement: x from, x to, z from, z to.
const LAB_PIT: (i32, i32, i32, i32) = (-fx::int(12), fx::int(12), -fx::int(13), fx::int(8));
/// and how far down the basement's floor is
const LAB_DEPTH: i32 = fx::int(5);

fn in_pit(x: i32, z: i32, tile: i32) -> bool {
    let (x0, x1, z0, z1) = LAB_PIT;
    x > x0 - tile / 2 && x < x1 + tile / 2 && z > z0 - tile / 2 && z < z1 + tile / 2
}

use crate::content::Rgb;
