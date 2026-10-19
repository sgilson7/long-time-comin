//! Zone two, the White Waste (2046): arenas on the ice shelf the
//! expedition crossed. The square and its gates are zone one's, so the
//! route, the spawn grid and the reach tests work unchanged; what stands in
//! it is new. Ice cliffs wall each arena in; out past them are mountains,
//! icebergs and an aurora. Five fights, after Returnal's and Saros's
//! arenas:
//!
//! - the Crevasse: a chasm across the north, crossed by two ice bridges, a
//!   high arch and a ledge along the west cliff; a fall returns you, at a cost
//! - the Stepped Shelf: two plateaus 4.5 m up joined by a bridge over the
//!   valley, a terrace, a spire on the east plateau
//! - the Wreck of the Long Patience: an icebreaker frozen in, broken in the
//!   middle; through the breach, through the stern's hold, along the deck,
//!   up the wheelhouse and the foremast
//! - the Antenna Field: a raised square catwalk, pylons at its corners with
//!   platforms 9 m up, a mast in the middle
//! - the Blue Hall: an ice cave with a roof, a ledge round three walls,
//!   columns and two arches
//!
//! and the Landing (the camp), the Weather Station (the rest) and the
//! Witness Stand (the boss). Updraft vents (`Vent`) throw a body about 6 m
//! up: they reach every height a stair does not.

use super::{boxed, dm, m, Side, Vent, B, COURT_RADIUS, DECK, GATE_GAP, HALF};
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine, V3};
use crate::level::Solid;
use crate::mesh::Mesh;
use crate::render::shade;

/// The Crevasse's chasm runs east to west between these two z values.
pub const CHASM: (i32, i32) = (-fx::int(13), -fx::int(5));
/// a vent's width
const VENT_RADIUS: i32 = ONE * 12 / 10;

#[allow(clippy::too_many_arguments)]
impl B<'_> {
    pub(super) fn ice(&mut self, kind: &str, exits: &[Side]) {
        if kind == "boss" {
            self.witness_stand();
            self.ice_skyline(COURT_RADIUS + m(6));
            return;
        }
        let chasm = kind == "crevasse";
        self.ice_floor(HALF, chasm);
        let gaps = [
            true,
            exits.contains(&Side::North),
            exits.contains(&Side::East),
            exits.contains(&Side::West),
        ];
        self.ice_perimeter(HALF, gaps, kind == "arrival");
        match kind {
            "arrival" => self.landing(),
            "rest" => self.station(),
            "crevasse" => self.crevasse(),
            "terraces" => self.terraces(),
            "wreck" => self.wreck(),
            "pylons" => self.pylons(),
            _ => self.hall(),
        }
        if !matches!(kind, "arrival" | "rest") {
            self.cover_walls(HALF, 4);
        }
        let props = if matches!(kind, "arrival" | "rest") {
            12
        } else {
            22
        };
        self.ice_props(HALF, props, chasm);
        self.ice_skyline(HALF);
    }

    // ---- pieces ---------------------------------------------------------

    /// An updraft vent on the surface at `(x, y, z)`: a grate and a column
    /// of glowing air.
    fn vent(&mut self, x: i32, y: i32, z: i32) {
        let metal = self.c("ice_metal_dark");
        let air = self.c("ice_vent");
        self.deco(
            Mesh::Cyl,
            v3(x, y + dm(1) / 2, z),
            v3(VENT_RADIUS * 2 + dm(4), dm(1), VENT_RADIUS * 2 + dm(4)),
            metal,
        );
        self.glow(
            Mesh::Cyl,
            v3(x, y + dm(1), z),
            v3(VENT_RADIUS * 2, dm(1) / 2, VENT_RADIUS * 2),
            air,
        );
        for k in 0..3 {
            let r = VENT_RADIUS * (6 - k) / 6;
            let ring = boxed(v3(x, y + m(1) + m(k * 2), z), v3(r * 2, dm(1) / 2, r * 2));
            self.shape(Mesh::Ring, ring, air, ONE * (3 - k) / 4, false);
        }
        self.vents.push(Vent {
            pos: v3(x, y, z),
            radius: VENT_RADIUS,
        });
    }

    /// A snow drift: a low mound, only looked at.
    pub(super) fn drift(&mut self, x: i32, y: i32, z: i32) {
        let snow = self.c("ice_snow");
        let r = dm(self.rng.range(8, 16));
        self.deco(Mesh::Blob, v3(x, y + r / 4, z), v3(r * 2, r / 2, r * 3 / 2), snow);
    }

    /// An ice shard pointing up from `(x, y, z)`, solid at its foot.
    fn shard(&mut self, x: i32, y: i32, z: i32, h: i32) {
        let c = self.pick(&[self.c("ice_blue"), self.c("ice_white"), self.c("ice_deep")]);
        let w = h / 3;
        self.solids
            .push(Solid::stand(x, y, z, w * 2 / 3, h * 2 / 3, w * 2 / 3));
        let tilt = Affine::translate(v3(x, y + h / 2, z))
            .then(&Affine::rot_y(deg(self.rng.range(0, 89))))
            .then(&Affine::rot_z(deg(self.rng.range(-8, 8))))
            .then(&Affine::scale(v3(w, h, w)));
        self.shape(Mesh::Octa, tilt, c, 0, true);
    }

    /// An icicle hanging from `(x, y, z)`, `len` long.
    fn icicle(&mut self, x: i32, y: i32, z: i32, len: i32) {
        let c = self.c("ice_white");
        let down = Affine::translate(v3(x, y - len / 2, z))
            .then(&Affine::rot_x(fx::HALF))
            .then(&Affine::scale(v3(len / 5, len, len / 5)));
        self.shape(Mesh::Cone, down, c, ONE / 6, true);
    }

    /// A cliff of ice with a rock foot, a snow cap and shards on top;
    /// `k`th along its row.
    #[allow(clippy::too_many_arguments)]
    fn cliff(&mut self, x: i32, z: i32, w: i32, d: i32, h: i32, inward: V3, k: i32) {
        let ice = self.pick(&[self.c("ice_blue"), self.c("ice_deep"), self.c("ice_white")]);
        let rock = self.pick(&[self.c("ice_rock"), self.c("ice_rock_dark")]);
        let snow = self.c("ice_snow");
        self.block(x, 0, z, w, h, d, ice);
        // under the cap, never level with it
        let foot = self.metres(2, 5).min(h - dm(5));
        // the feet of neighbours overlap: every other one stands out two
        // centimetres further and higher, so no two fronts or tops lie in
        // one plane (that flickers); and its foot goes a little under the
        // cliff's, which may hang over a chasm
        let jog = dm(1) / 5 * (k % 2) * 2;
        let sunk = dm(1) / 5;
        self.deco(
            Mesh::Cube,
            v3(x, (foot + jog - sunk) / 2, z),
            v3(w + dm(3) + jog, foot + jog + sunk, d + dm(3) + jog),
            rock,
        );
        // a quarter metre over the sides all round: the sides stand on a
        // tenth-metre grid, and a cap's side level with a neighbour's
        // face flickers
        // and its top off the grid too (a wall's top may lie there)
        self.deco(
            Mesh::Cube,
            v3(x, h + dm(3) - ONE / 40, z),
            v3(w + dm(5), dm(6), d + dm(5)),
            snow,
        );
        // great broken chunks along the top, so the line of cliffs is
        // jagged against the sky and never a row of roofs
        for _ in 0..self.rng.range(2, 3) {
            let cx = x + fx::mul(dm(self.rng.range(-35, 35)), fx::div(w, m(8)));
            let cz = z + fx::mul(dm(self.rng.range(-35, 35)), fx::div(d, m(8)));
            let ch = h / 2 + self.metres(1, 5);
            let c = self.pick(&[self.c("ice_white"), self.c("ice_blue"), self.c("ice_deep")]);
            let chunk = Affine::translate(v3(cx, h - ch / 4, cz))
                .then(&Affine::rot_y(deg(self.rng.range(0, 89))))
                .then(&Affine::rot_z(deg(self.rng.range(-14, 14))))
                .then(&Affine::scale(v3(w * 7 / 10, ch, d * 7 / 10)));
            self.shape(Mesh::Octa, chunk, c, 0, true);
        }
        for _ in 0..self.rng.range(1, 3) {
            let sx = x + fx::mul(dm(self.rng.range(-30, 30)), fx::div(w, m(8)));
            let sz = z + fx::mul(dm(self.rng.range(-30, 30)), fx::div(d, m(8)));
            let sh = self.metres(2, 6);
            let c = self.pick(&[self.c("ice_white"), self.c("ice_blue")]);
            let tilt = Affine::translate(v3(sx, h + sh / 2, sz))
                .then(&Affine::rot_y(deg(self.rng.range(0, 89))))
                .then(&Affine::scale(v3(sh / 2, sh, sh / 2)));
            self.shape(Mesh::Octa, tilt, c, 0, true);
        }
        // the inner face: a streak of glowing ice and icicles under the cap
        // clear of the foot's front (as much as 17 cm out)
        let face = v3(x, 0, z).add(v3(
            fx::mul(inward.x, w / 2 + dm(1) + dm(1) * 3 / 10),
            0,
            fx::mul(inward.z, d / 2 + dm(1) + dm(1) * 3 / 10),
        ));
        let along_x = inward.x == 0;
        let glow = self.c("ice_glow");
        if self.rng.chance(60) {
            let off = dm(self.rng.range(-20, 20));
            let p = face.add(if along_x { v3(off, 0, 0) } else { v3(0, 0, off) });
            let len = h * 2 / 3;
            // its ends and foot off the tenth-metre grid the cliff's sides
            // and the feet stand on, never level with them (that flickers)
            let wide = dm(6) - dm(1) / 5;
            let size = if along_x {
                v3(wide, len, dm(1))
            } else {
                v3(dm(1), len, wide)
            };
            self.shape(
                Mesh::Cube,
                boxed(p.add(v3(0, h - len / 2 - dm(5) + dm(1) * 15 / 100, 0)), size),
                glow,
                ONE / 3,
                false,
            );
        }
        for _ in 0..self.rng.range(1, 4) {
            let off = fx::mul(dm(self.rng.range(-35, 35)), fx::div(w.max(d), m(10)));
            let p = face.add(if along_x { v3(off, 0, 0) } else { v3(0, 0, off) });
            let len = self.metres(1, 3);
            self.icicle(p.x, h, p.z, len);
        }
    }

    // ---- the floor, the walls, the far ice --------------------------------

    /// The ice sheet: one slab (two round the Crevasse's chasm), drifts,
    /// cracks and patches of clear blue ice; and ground out to the
    /// mountains, laid round the arena so a chasm stays open.
    fn ice_floor(&mut self, half: i32, chasm: bool) {
        let snow = self.c("ice_snow");
        let o = half + m(20);
        if chasm {
            let (z0, z1) = CHASM;
            self.solids.push(Solid::new(v3(-o, -m(1), z1), v3(o, 0, o)));
            self.solids.push(Solid::new(v3(-o, -m(1), -o), v3(o, 0, z0)));
            self.shape(
                Mesh::Cube,
                boxed(v3(0, -dm(5), (o + z1) / 2), v3(o * 2, m(1), o - z1)),
                snow,
                0,
                false,
            );
            self.shape(
                Mesh::Cube,
                boxed(v3(0, -dm(5), (z0 - o) / 2), v3(o * 2, m(1), z0 + o)),
                snow,
                0,
                false,
            );
            // the chasm's walls going down into the blue, and mist at the bottom
            let deep = self.c("ice_deep");
            let rock = self.c("ice_rock_dark");
            for z in [z0, z1] {
                // the upper band's top a little under the floor's: a bridge
                // lies over the lip (one plane flickers)
                let lip = dm(1) / 5;
                for (top, h, c) in [(-lip, m(8) - lip, deep), (-m(8), m(16), rock)] {
                    self.shape(
                        Mesh::Cube,
                        boxed(v3(0, top - h / 2, z), v3(o * 2, h, dm(4))),
                        c,
                        0,
                        true,
                    );
                }
            }
            let mist = self.c("ice_glow");
            self.shape(
                Mesh::Cube,
                boxed(
                    v3(0, -m(18), (z0 + z1) / 2),
                    v3(o * 2 - dm(1) / 5, dm(2), z1 - z0),
                ),
                mist,
                ONE / 3,
                false,
            );
            for _ in 0..10 {
                let x = dm(self.rng.range(-250, 250));
                for z in [z0, z1] {
                    let len = self.metres(1, 4);
                    self.icicle(x, 0, z + if z == z0 { dm(2) } else { -dm(2) }, len);
                }
            }
        } else {
            self.solids.push(Solid::new(v3(-o, -m(1), -o), v3(o, 0, o)));
            self.shape(
                Mesh::Cube,
                boxed(v3(0, -dm(5), 0), v3(o * 2, m(1), o * 2)),
                snow,
                0,
                false,
            );
        }
        // the ground beyond, in four strips round the arena's slab
        let far = m(600);
        let ground = self.c("ice_snow_dark");
        for (cx, cz, w, d) in [
            (0, -(o + far) / 2 - o / 2, far * 2, far - o),
            (0, (o + far) / 2 + o / 2, far * 2, far - o),
            (-(o + far) / 2 - o / 2, 0, far - o, o * 2),
            ((o + far) / 2 + o / 2, 0, far - o, o * 2),
        ] {
            self.shape(
                Mesh::Ground,
                boxed(v3(cx, -dm(2), cz), v3(w, ONE, d)),
                ground,
                0,
                false,
            );
        }
        // patches of clear ice, cracks, drifts
        let blue = self.c("ice_blue");
        for _ in 0..9 {
            let x = dm(self.rng.range(-240, 240));
            let z = dm(self.rng.range(-240, 240));
            if chasm && z > CHASM.0 - m(4) && z < CHASM.1 + m(4) {
                continue;
            }
            let r = self.metres(2, 5);
            self.shape(
                Mesh::Cyl,
                boxed(v3(x, dm(1) / 5, z), v3(r * 2, dm(1) / 10, r * 3 / 2)),
                blue,
                ONE / 6,
                false,
            );
        }
        let ink = self.c("ice_ink");
        for _ in 0..12 {
            let a = v3(
                dm(self.rng.range(-250, 250)),
                dm(1) / 4,
                dm(self.rng.range(-250, 250)),
            );
            if chasm && a.z > CHASM.0 - m(3) && a.z < CHASM.1 + m(3) {
                continue;
            }
            let mut p = a;
            for _ in 0..3 {
                let q = p.add(v3(dm(self.rng.range(-25, 25)), 0, dm(self.rng.range(-25, 25))));
                self.rod(p, q, dm(1) / 3, shade(ink, 120), false);
                p = q;
            }
        }
        for _ in 0..14 {
            let x = dm(self.rng.range(-250, 250));
            let z = dm(self.rng.range(-250, 250));
            if chasm && z > CHASM.0 - m(3) && z < CHASM.1 + m(3) {
                continue;
            }
            if x.abs() < m(5) || z.abs() < m(5) {
                continue;
            }
            self.drift(x, 0, z);
        }
    }

    /// Cliffs round the arena, a gap at each side in `gaps`, and the
    /// invisible bound. The Landing's south side is a low shelf edge, so the
    /// sea and the ship show over it.
    fn ice_perimeter(&mut self, half: i32, gaps: [bool; 4], low_south: bool) {
        let sides = [Side::South, Side::North, Side::East, Side::West];
        for (si, side) in sides.iter().enumerate() {
            let mut along = -half - m(6);
            let mut k = 0;
            while along < half + m(6) {
                k += 1;
                let w = self.metres(7, 14);
                let mid = along + w / 2;
                along += w;
                if gaps[si] && mid.abs() < GATE_GAP / 2 + w / 2 {
                    continue;
                }
                let depth = self.metres(6, 12);
                let h = if low_south && *side == Side::South {
                    self.metres(2, 4)
                } else {
                    self.metres(10, 24)
                };
                let out = half + depth / 2 + dm(5);
                let (x, z, bw, bd, inward) = match side {
                    Side::South => (mid, out, w, depth, v3(0, 0, -ONE)),
                    Side::North => (mid, -out, w, depth, v3(0, 0, ONE)),
                    Side::East => (out, mid, depth, w, v3(-ONE, 0, 0)),
                    Side::West => (-out, mid, depth, w, v3(ONE, 0, 0)),
                };
                self.cliff(x, z, bw, bd, h, inward, k);
            }
            let (pos, _) = side.place(half);
            if gaps[si] {
                self.beacon_gate(pos, *side);
            } else {
                self.serac_wall(pos, *side);
            }
        }
        let o = half + m(16);
        let t = m(1);
        let top = m(80);
        self.wall(v3(-o - t, -m(40), -o - t), v3(o + t, top, -o));
        self.wall(v3(-o - t, -m(40), o), v3(o + t, top, o + t));
        self.wall(v3(-o - t, -m(40), -o), v3(-o, top, o));
        self.wall(v3(o, -m(40), -o), v3(o + t, top, o));
    }

    /// A gate on the ice: two survey beacons with lamps and flags, a beam
    /// between them, and the corridor behind shut by ice.
    fn beacon_gate(&mut self, pos: V3, side: Side) {
        let metal = self.c("ice_metal_dark");
        let red = self.c("ice_red");
        let lamp = self.c("ice_lamp");
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
            self.block(p.x, 0, p.z, dm(8), m(6), dm(8), metal);
            for k in 0..3 {
                let band = boxed(v3(p.x, m(1) + m(k * 2), p.z), v3(dm(9), dm(3), dm(9)));
                self.shape(Mesh::Cube, band, red, 0, true);
            }
            self.glow(
                Mesh::Blob,
                v3(p.x, m(6) + dm(4), p.z),
                v3(dm(7), dm(7), dm(7)),
                lamp,
            );
            // a flag on a pole
            let pole = p.add(v3(0, m(6), 0));
            self.rod(pole, pole.add(v3(0, m(2), 0)), dm(1), metal, false);
            let flag = if across_x {
                v3(dm(9), dm(5), dm(1) / 2)
            } else {
                v3(dm(1) / 2, dm(5), dm(9))
            };
            let fo = if across_x {
                v3(s * dm(5), m(2) - dm(3), 0)
            } else {
                v3(0, m(2) - dm(3), s * dm(5))
            };
            self.deco(Mesh::Cube, p.add(v3(0, m(6), 0)).add(fo), flag, red);
        }
        let (lw, ld) = if across_x { (m(7), dm(5)) } else { (dm(5), m(7)) };
        self.deco(Mesh::Cube, v3(at.x, m(5) + dm(5), at.z), v3(lw, dm(4), ld), metal);
        let back = pos.add(out.scale(m(3)));
        let (bw, bd) = if across_x {
            (GATE_GAP + m(2), dm(5))
        } else {
            (dm(5), GATE_GAP + m(2))
        };
        self.solids.push(Solid::stand(back.x, 0, back.z, bw, m(20), bd));
        let deep = self.c("ice_deep");
        self.deco(Mesh::Cube, v3(back.x, m(6), back.z), v3(bw, m(12), bd), deep);
    }

    /// Where no gate stands: a wall of seracs.
    fn serac_wall(&mut self, pos: V3, side: Side) {
        let across_x = matches!(side, Side::North | Side::South);
        let out = match side {
            Side::South => v3(0, 0, m(2)),
            Side::North => v3(0, 0, -m(2)),
            Side::East => v3(m(2), 0, 0),
            Side::West => v3(-m(2), 0, 0),
        };
        let at = pos.add(out);
        let (bw, bd) = if across_x {
            (GATE_GAP + m(4), m(4))
        } else {
            (m(4), GATE_GAP + m(4))
        };
        let inward = v3(-out.x.signum() * ONE, 0, -out.z.signum() * ONE);
        let h = self.metres(9, 15);
        self.cliff(at.x, at.z, bw, bd, h, inward, 0);
        for k in -2..=2 {
            let off = if across_x {
                v3(m(2) * k, 0, 0)
            } else {
                v3(0, 0, m(2) * k)
            };
            let p = pos.add(off).add(inward.scale(dm(5)));
            let sh = self.metres(2, 4);
            self.shard(p.x, 0, p.z, sh);
        }
    }

    /// Out past the cliffs: three rings of mountains with snow on their
    /// peaks, icebergs on the frozen sea, low cloud, and the aurora.
    fn ice_skyline(&mut self, half: i32) {
        let rock = self.c("ice_rock");
        let rock_dark = self.c("ice_rock_dark");
        let snow = self.c("ice_white");
        let rings = [
            (half + m(45), half + m(80), 18),
            (half + m(90), half + m(160), 22),
            (half + m(180), half + m(300), 16),
        ];
        for (lo, hi, n) in rings {
            for k in 0..n {
                let a = fx::TURN / n * k + deg(self.rng.range(-6, 6));
                let r = lo + dm(self.rng.range(0, (hi - lo) / dm(1)));
                let (x, z) = (fx::mul(fx::cos(a), r), fx::mul(fx::sin(a), r));
                let w = self.metres(30, 70) + (r - lo) / 3;
                let h = self.metres(25, 60) + (r - lo) / 3;
                let c = if self.rng.chance(50) { rock } else { rock_dark };
                self.shape(Mesh::Cone, boxed(v3(x, h / 2, z), v3(w, h, w)), c, 0, true);
                // the peak's snow: a smaller cone over the top third
                let cap = h * 4 / 10;
                self.shape(
                    Mesh::Cone,
                    boxed(v3(x, h - cap / 2 + dm(5), z), v3(w * 42 / 100, cap, w * 42 / 100)),
                    snow,
                    ONE / 8,
                    true,
                );
            }
        }
        // icebergs and glacier walls nearer in
        let blue = self.c("ice_blue");
        let white = self.c("ice_white");
        for k in 0..20 {
            let a = fx::TURN / 20 * k + deg(self.rng.range(-7, 7));
            let r = half + self.metres(22, 42);
            let (x, z) = (fx::mul(fx::cos(a), r), fx::mul(fx::sin(a), r));
            let s = self.metres(6, 16);
            let c = if k % 2 == 0 { blue } else { white };
            let tilt = Affine::translate(v3(x, s / 3, z))
                .then(&Affine::rot_y(deg(self.rng.range(0, 89))))
                .then(&Affine::scale(v3(s * 2, s, s * 3 / 2)));
            self.shape(Mesh::Octa, tilt, c, 0, true);
        }
        // low cloud
        let cloud = self.c("ice_white");
        for _ in 0..16 {
            let a = deg(self.rng.range(0, 359));
            let r = self.metres(160, 320);
            let c = v3(
                fx::mul(fx::cos(a), r),
                self.metres(40, 90),
                fx::mul(fx::sin(a), r),
            );
            for p in 0..self.rng.range(3, 5) {
                let s = self.metres(12, 26);
                let off = v3(
                    s * (p - 2) * 2 / 3,
                    dm(self.rng.range(-30, 30)),
                    dm(self.rng.range(-50, 50)),
                );
                self.shape(
                    Mesh::Blob,
                    boxed(c.add(off), v3(s * 2, s * 2 / 3, s * 2)),
                    cloud,
                    ONE / 4,
                    true,
                );
            }
        }
        self.aurora(half);
    }

    /// The aurora: curtains of light across the northern sky, each a row of
    /// tall glowing panes that rise and fall.
    fn aurora(&mut self, half: i32) {
        let colours = [
            self.c("aurora_green"),
            self.c("aurora_pink"),
            self.c("aurora_green"),
            self.c("aurora_green"),
        ];
        for (band, rgb) in colours.iter().enumerate() {
            let band = band as i32;
            let r = half + m(140) + m(band * 25);
            let a0 = deg(195 + band * 20);
            let panes = 34;
            let base = m(60) + m(band * 14);
            for k in 0..panes {
                let a = a0 + deg(4) * k;
                let wave = fx::sin(k * fx::TURN / 17 + band * fx::QUARTER);
                let (x, z) = (fx::mul(fx::cos(a), r), fx::mul(fx::sin(a), r));
                let h = m(36) + fx::mul(wave.abs(), m(34));
                let y = base + fx::mul(wave, m(14));
                // a curtain of light: tall, thin, facing the arena
                let pane = Affine::translate(v3(x, y + h / 2, z))
                    .then(&Affine::rot_y(fx::QUARTER - a))
                    .then(&Affine::scale(v3(m(14), h, dm(5))));
                let strength = ONE * (4 + (k % 3)) / 10;
                self.halo(Mesh::Cube, pane, *rgb, strength);
            }
        }
    }

    /// The camp's things, the survey's flags, drifts and shards, scattered
    /// where they block nothing the arena needs.
    fn ice_props(&mut self, half: i32, count: i32, chasm: bool) {
        for _ in 0..count {
            let x = dm(self.rng.range(-(half / dm(1)) + 15, half / dm(1) - 15));
            let z = dm(self.rng.range(-(half / dm(1)) + 15, half / dm(1) - 15));
            if x.abs() < m(5) || z.abs() < m(5) {
                continue;
            }
            if chasm && z > CHASM.0 - m(3) && z < CHASM.1 + m(3) {
                continue;
            }
            let probe = crate::level::Level::new(self.solids.clone());
            if probe.blocked(v3(x - m(2), dm(1), z - m(2)), v3(x + m(2), m(3), z + m(2))) {
                continue;
            }
            match self.rng.range(0, 7) {
                0 | 1 => self.drift(x, 0, z),
                2 => {
                    let h = self.metres(2, 4);
                    self.shard(x, 0, z, h);
                }
                3 => self.crate_stack(x, 0, z),
                4 => self.drum(x, 0, z),
                5 => self.survey_flag(x, 0, z),
                6 => self.sledge(x, 0, z),
                _ => self.camp_lamp(x, 0, z),
            }
        }
    }

    fn drum(&mut self, x: i32, y: i32, z: i32) {
        let c = self.pick(&[self.c("ice_red"), self.c("ice_orange"), self.c("ice_metal")]);
        self.solids.push(Solid::stand(x, y, z, dm(8), dm(11), dm(8)));
        self.deco(Mesh::Cyl, v3(x, y + dm(55) / 10, z), v3(dm(8), dm(11), dm(8)), c);
        self.deco(
            Mesh::Cyl,
            v3(x, y + dm(11), z),
            v3(dm(8), dm(1), dm(8)),
            self.c("ice_snow"),
        );
    }

    fn survey_flag(&mut self, x: i32, y: i32, z: i32) {
        let metal = self.c("ice_metal_dark");
        let red = self.c("ice_red");
        self.rod(v3(x, y, z), v3(x, y + m(2), z), dm(1) / 2, metal, false);
        self.deco(
            Mesh::Cube,
            v3(x + dm(3), y + m(2) - dm(2), z),
            v3(dm(6), dm(3), dm(1) / 3),
            red,
        );
    }

    fn sledge(&mut self, x: i32, y: i32, z: i32) {
        let wood = self.c("wood");
        let canvas = self.c("ice_canvas");
        let metal = self.c("ice_metal_dark");
        self.block(x, y + dm(2), z, dm(9), dm(4), m(2), wood);
        self.deco(Mesh::Cube, v3(x, y + dm(8), z), v3(dm(8), dm(3), dm(16)), canvas);
        for s in [-1, 1] {
            let a = v3(x + s * dm(4), y + dm(1), z - m(1));
            self.rod(a, a.add(v3(0, 0, m(2))), dm(1) / 2, metal, false);
        }
    }

    fn camp_lamp(&mut self, x: i32, y: i32, z: i32) {
        let metal = self.c("ice_metal_dark");
        let lamp = self.c("ice_lamp");
        self.deco(
            Mesh::Cyl,
            v3(x, y + m(1) + dm(5), z),
            v3(dm(2), m(3), dm(2)),
            metal,
        );
        self.deco(Mesh::Cube, v3(x, y + m(3), z), v3(dm(5), dm(1), dm(5)), metal);
        self.glow(Mesh::Blob, v3(x, y + dm(27), z), v3(dm(5), dm(5), dm(5)), lamp);
    }

    /// A tent: a canvas wedge with its door lit.
    fn tent(&mut self, x: i32, z: i32, along_x: bool) {
        let canvas = self.pick(&[self.c("ice_canvas"), self.c("ice_orange"), self.c("ice_red")]);
        let (w, d) = if along_x { (m(4), m(3)) } else { (m(3), m(4)) };
        self.solids.push(Solid::stand(x, 0, z, w, dm(22), d));
        let yaw = if along_x { 0 } else { fx::QUARTER };
        let body = Affine::translate(v3(x, dm(12), z))
            .then(&Affine::rot_y(yaw))
            .then(&Affine::scale(v3(m(3), dm(24), m(4))));
        self.shape(Mesh::Wedge, body, canvas, 0, true);
        let lamp = self.c("ice_lamp");
        let door = if along_x {
            v3(x, dm(7), z + d / 2)
        } else {
            v3(x + w / 2, dm(7), z)
        };
        let size = if along_x {
            v3(dm(8), dm(12), dm(1))
        } else {
            v3(dm(1), dm(12), dm(8))
        };
        self.glow(Mesh::Cube, door, size, lamp);
    }

    /// A shipping container, `h` metres of them stacked.
    fn container(&mut self, x: i32, z: i32, along_x: bool, stack: i32) {
        let (w, d) = if along_x { (m(6), dm(24)) } else { (dm(24), m(6)) };
        for k in 0..stack {
            let c = self.pick(&[
                self.c("ice_red"),
                self.c("ice_orange"),
                self.c("ice_hull_dark"),
                self.c("ice_metal"),
            ]);
            let y = dm(26) * k;
            self.block(x, y, z, w, dm(26), d, c);
            self.deco(
                Mesh::Cube,
                v3(x, y + dm(26) + dm(1), z),
                v3(w + dm(1), dm(2), d + dm(1)),
                self.c("ice_snow"),
            );
        }
    }

    // ---- the arenas -----------------------------------------------------

    fn crevasse(&mut self) {
        let (z0, z1) = CHASM;
        let ice = self.c("ice_blue");
        let white = self.c("ice_white");
        // two bridges of ice at the floor's level
        for (x0, x1) in [(-m(18), -m(13)), (m(9), m(13))] {
            self.solids.push(Solid::new(v3(x0, -m(1), z0), v3(x1, 0, z1)));
            self.deco(
                Mesh::Cube,
                v3((x0 + x1) / 2, -dm(6), (z0 + z1) / 2),
                v3(x1 - x0, dm(12), z1 - z0),
                ice,
            );
            // its underside hangs down into the chasm
            let belly = Affine::translate(v3((x0 + x1) / 2, -m(2), (z0 + z1) / 2))
                .then(&Affine::rot_x(fx::HALF))
                // a little in from the deck's sides (one plane flickers)
                .then(&Affine::scale(v3(x1 - x0 - dm(1) / 5, m(3), z1 - z0 - dm(1) / 5)));
            self.shape(Mesh::Wedge, belly, shade(ice, 85), 0, true);
            for s in [x0, x1] {
                for k in 0..3 {
                    let z = z0 + (z1 - z0) / 4 * (k + 1);
                    self.deco(Mesh::Octa, v3(s, dm(3), z), v3(dm(4), dm(7), dm(4)), white);
                }
            }
        }
        // the high arch: two towers and a bridge 4.5 m up over the chasm
        let h = m(4) + dm(5);
        self.platform(0, -m(3), m(8), m(4), h, (0, ONE));
        self.platform(0, -m(15), m(8), m(4), h, (ONE, 0));
        self.deck(-m(2), z0, m(2), z1, h, &[]);
        // a ledge along the west cliff, 3.5 m up, over the chasm too
        let lh = m(3) + dm(5);
        self.deck(
            -m(27),
            -m(22),
            -m(23),
            m(10),
            lh,
            &[(-m(25), -m(20)), (-m(25), m(5)), (-m(25), -m(2))],
        );
        let steps = (lh + dm(4) - 1) / dm(4);
        self.stairs(-m(25), 0, m(10) + dm(6) * steps, 0, -ONE, lh, m(3));
        // an arch of ice over the south-east to walk under
        self.deck(m(12), m(8), m(24), m(12), m(5), &[(m(13), m(10)), (m(23), m(10))]);
        // vents: up to the arch over the chasm, and to the west ledge from the north
        self.vent(m(6), 0, -m(2));
        self.vent(-m(20), 0, -m(18));
        self.vent(m(18), 0, m(4));
    }

    fn terraces(&mut self) {
        let h = m(4) + dm(5);
        let ice = self.c("ice_blue");
        let snow = self.c("ice_snow");
        let deep = self.c("ice_deep");
        // the west plateau, the east plateau, and a terrace below the west one
        for (x, z, w, d, top) in [
            (-m(18), -m(6), m(16), m(20), h),
            (m(18), -m(14), m(16), m(16), h),
            (-m(20), m(10), m(12), m(12), m(2)),
        ] {
            self.block(x, 0, z, w, top, d, ice);
            // its snow a little under the 0.4 m steps that climb onto it,
            // not level with the first (one plane flickers)
            let cap = dm(4) - ONE / 40;
            self.deco(
                Mesh::Cube,
                v3(x, top + cap / 2, z),
                v3(w + dm(4), cap, d + dm(4)),
                snow,
            );
            // banded faces: older ice lower down
            self.deco(
                Mesh::Cube,
                v3(x, top / 4, z),
                v3(w + dm(2), top / 2, d + dm(2)),
                deep,
            );
        }
        let steps = |rise: i32| (rise + dm(4) - 1) / dm(4);
        self.stairs(-m(10) + dm(6) * steps(h), 0, -m(8), -ONE, 0, h, m(3));
        self.stairs(m(10) - dm(6) * steps(h), 0, -m(8), ONE, 0, h, m(3));
        self.stairs(-m(14) + dm(6) * steps(m(2)), 0, m(10), -ONE, 0, m(2), m(3));
        // from the terrace up onto the west plateau
        self.stairs(
            -m(20),
            m(2),
            m(4) + dm(6) * steps(h - m(2)),
            0,
            -ONE,
            h - m(2),
            m(3),
        );
        // the bridge between the plateaus, over the valley
        self.deck(
            -m(10),
            -m(13),
            m(10),
            -m(10),
            h,
            &[(-m(5), -m(11) - dm(5)), (m(5), -m(11) - dm(5))],
        );
        // a dais before the north gate
        self.block(0, 0, -m(19) - dm(5), m(12), dm(16), m(5), ice);
        self.stairs(0, 0, -m(17) + dm(6) * steps(dm(16)), 0, -ONE, dm(16), m(3));
        // a spire on the east plateau
        self.block(m(20), h, -m(18), m(4), m(4) + dm(5), m(4), shade(ice, 90));
        self.deco(
            Mesh::Cube,
            v3(m(20), h * 2 + dm(2), -m(18)),
            v3(m(4) + dm(4), dm(4), m(4) + dm(4)),
            snow,
        );
        // a frozen fall down the west plateau's face
        let glow = self.c("ice_glow");
        self.shape(
            Mesh::Cube,
            // proud of the face it runs down, not level with it
            boxed(v3(-m(10) + ONE / 50, h / 2, -m(14)), v3(dm(2), h, m(3))),
            glow,
            ONE / 3,
            false,
        );
        self.vent(-m(8), 0, m(6));
        self.vent(m(12), 0, -m(3));
        self.vent(m(15), h, -m(14));
    }

    fn wreck(&mut self) {
        let hull = self.c("ice_hull");
        let hull_dark = self.c("ice_hull_dark");
        let deck_c = self.c("wood");
        let metal = self.c("ice_metal");
        let snow = self.c("ice_snow");
        let (zn, zs) = (-m(11), -m(2));
        let zc = (zn + zs) / 2;
        let beam = zs - zn;
        let h = m(4) + dm(5);
        // the bow: solid, a forecastle on it, a pointed stem
        self.block(-m(11), 0, zc, m(16), h, beam, hull);
        self.block(-m(16), h, zc, m(6), m(2), beam, hull_dark);
        let stem = Affine::translate(v3(-m(20), h / 2, zc))
            .then(&Affine::rot_y(fx::QUARTER))
            .then(&Affine::scale(v3(beam, h, m(4))));
        self.shape(Mesh::Wedge, stem, hull, 0, true);
        // the breach: torn plates, the deck carried across it
        self.block(0, h - DECK, zc, m(6), DECK, beam, deck_c);
        for s in [-1, 1] {
            for k in 0..4 {
                let p = v3(s * (m(3) + dm(2)), dm(5) + m(1) * k, zn + beam / 4 * (k % 3 + 1));
                let tear = Affine::translate(p)
                    .then(&Affine::rot_z(deg(15) * s))
                    .then(&Affine::scale(v3(dm(3), m(1) + dm(4), m(2))));
                self.shape(Mesh::Wedge, tear, hull_dark, 0, true);
            }
        }
        // the stern: a hold you can walk through, a torn hole in its south side
        let wall_h = h - dm(4);
        for (x0, x1) in [(m(3), m(8)), (m(11), m(15))] {
            self.block((x0 + x1) / 2, 0, zs - dm(3), x1 - x0, wall_h, dm(6), hull);
        }
        self.block(m(9), 0, zn + dm(3), m(12), wall_h, dm(6), hull);
        self.block(m(9), wall_h, zc, m(12), dm(4), beam, deck_c);
        self.block(m(17), 0, zc, m(4), h, beam, hull);
        let lamp = self.c("ice_lamp");
        for x in [m(5), m(9), m(13)] {
            self.glow(
                Mesh::Blob,
                v3(x, wall_h - dm(4), zc),
                v3(dm(4), dm(4), dm(4)),
                lamp,
            );
        }
        self.crate_stack(m(6), 0, zc - m(2));
        self.crate_stack(m(13), 0, zc + m(2));
        // a waterline stripe and portholes along both sides
        for z in [zn - dm(1), zs + dm(1)] {
            self.deco(Mesh::Cube, v3(0, dm(9), z), v3(m(38), dm(4), dm(1)), hull_dark);
            let n = if z < zc { v3(0, 0, -ONE) } else { v3(0, 0, ONE) };
            self.facade(v3(-m(11), m(2) + dm(8), z), n, m(14), m(3), lamp, ONE / 3);
        }
        // the deck's rail, snow on the deck
        for z in [zn, zs] {
            self.rod(
                v3(-m(19), h + dm(9), z),
                v3(m(19), h + dm(9), z),
                dm(1),
                metal,
                true,
            );
        }
        self.deco(
            Mesh::Cube,
            v3(-m(11), h + dm(1), zc),
            v3(m(15), dm(2), beam - dm(4)),
            snow,
        );
        // the wheelhouse, its windows lit
        self.block(m(10) + dm(5), h, zc, m(5), m(3) + dm(5), m(5), shade(metal, 110));
        self.deco(
            Mesh::Cube,
            v3(m(10) + dm(5), h + m(3) + dm(7), zc),
            v3(m(5) + dm(6), dm(4), m(5) + dm(6)),
            hull_dark,
        );
        self.facade(
            v3(m(10) + dm(5), h + m(2), zc + m(2) + dm(5)),
            v3(0, 0, ONE),
            m(4),
            m(2),
            lamp,
            ONE / 5,
        );
        // the foremast and its crow's nest, rigging to the rails
        let mast = v3(-m(8), h, zc);
        let nest = h + m(5) + dm(5);
        self.solids
            .push(Solid::stand(mast.x, h, mast.z, dm(5), nest - DECK - h, dm(5)));
        self.rod(mast, mast.add(v3(0, m(12), 0)), dm(4), shade(deck_c, 80), true);
        self.rod(
            mast.add(v3(-m(4), m(8), 0)),
            mast.add(v3(m(4), m(8), 0)),
            dm(2),
            shade(deck_c, 80),
            true,
        );
        self.deck(
            mast.x - m(1) - dm(5),
            mast.z - m(1) - dm(5),
            mast.x + m(1) + dm(5),
            mast.z + m(1) + dm(5),
            nest,
            &[],
        );
        let ink = self.c("ice_ink");
        for (dx, z) in [(-m(9), zn), (m(9), zn), (-m(9), zs), (m(9), zs)] {
            self.rod(
                mast.add(v3(0, m(11), 0)),
                v3(mast.x + dx, h, z),
                dm(1) / 3,
                ink,
                false,
            );
        }
        // up onto the deck from the south and the north
        let steps = (h + dm(4) - 1) / dm(4);
        self.stairs(-m(9), 0, zs + dm(6) * steps, 0, -ONE, h, m(3));
        self.stairs(m(6), 0, zn - dm(6) * steps, 0, ONE, h, m(3));
        // the cargo, spilled on the ice
        self.container(-m(17), m(11), true, 2);
        self.container(m(17), m(12), false, 1);
        self.container(m(21), -m(19), true, 2);
        self.container(-m(20), -m(19), false, 1);
        // vents: onto the wheelhouse, and up the mast to the nest
        self.vent(m(14) + dm(5), h, zc);
        self.vent(-m(5), h, zc);
        self.vent(-m(14), 0, m(4));
    }

    fn pylons(&mut self) {
        let h = m(4);
        let metal = self.c("ice_metal");
        let dark = self.c("ice_metal_dark");
        let red = self.c("ice_red");
        let lamp = self.c("ice_lamp");
        let s = m(14);
        let w = m(1) + dm(5);
        let inner = s - w;
        // the catwalk: four sides of a square 4 m up, meeting the corner towers
        self.deck(-inner, -s - w, inner, -s + w, h, &[(-m(6), -s), (m(6), -s)]);
        self.deck(-inner, s - w, inner, s + w, h, &[(-m(6), s), (m(6), s)]);
        self.deck(-s - w, -inner, -s + w, inner, h, &[(-s, -m(7)), (-s, m(7))]);
        self.deck(s - w, -inner, s + w, inner, h, &[(s, -m(7)), (s, m(7))]);
        for (sx, sz) in [(1, 1), (-1, 1), (1, -1), (-1, -1)] {
            let (cx, cz) = (sx * s, sz * s);
            // the corner tower: a base the catwalk meets, a mast, a platform 9 m up
            self.block(cx, 0, cz, w * 2 + dm(2), h, w * 2 + dm(2), dark);
            let (mx, mz) = (cx + sx * dm(9), cz + sz * dm(9));
            self.solids.push(Solid::stand(mx, h, mz, dm(6), m(5), dm(6)));
            self.rod(v3(mx, h, mz), v3(mx, h + m(12), mz), dm(5), metal, true);
            for k in 1..6 {
                let y = h + m(2) * k;
                self.rod(v3(mx - dm(6), y, mz), v3(mx + dm(6), y, mz), dm(1), dark, false);
            }
            self.glow(
                Mesh::Blob,
                v3(mx, h + m(12) + dm(3), mz),
                v3(dm(5), dm(5), dm(5)),
                red,
            );
            let top = m(9);
            self.deck(
                mx - m(1) - dm(5),
                mz - m(1) - dm(5),
                mx + m(1) + dm(5),
                mz + m(1) + dm(5),
                top,
                &[],
            );
            self.vent(cx - sx * m(3), h, cz);
        }
        // stairs up to the catwalk, one a side, clear of the gate paths
        let steps = (h + dm(4) - 1) / dm(4);
        let run = dm(6) * steps;
        self.stairs(-m(7), 0, -s - w - run, 0, ONE, h, m(3));
        self.stairs(m(7), 0, s - w - run, 0, ONE, h, m(3));
        self.stairs(-s - w - run, 0, -m(6), ONE, 0, h, m(3));
        self.stairs(s + w + run, 0, m(7), -ONE, 0, h, m(3));
        // the mast in the middle, guyed to the catwalk's corners
        self.block(0, 0, -m(2), m(2), m(1), m(2), dark);
        self.solids
            .push(Solid::stand(0, m(1), -m(2), dm(8), m(22), dm(8)));
        self.rod(v3(0, m(1), -m(2)), v3(0, m(24), -m(2)), dm(7), metal, true);
        for k in 0..4 {
            let a = fx::TURN / 4 * k + fx::QUARTER / 2;
            let tip = v3(fx::mul(fx::cos(a), s), h, fx::mul(fx::sin(a), s) - m(2));
            self.rod(v3(0, m(20), -m(2)), tip, dm(1) / 3, dark, false);
        }
        for k in 0..3 {
            let y = m(8) + m(5) * k;
            let dish = Affine::translate(v3(0, y, -m(2) + dm(8)))
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&Affine::scale(v3(m(2), dm(3), m(2))));
            self.shape(Mesh::Cone, dish, shade(metal, 115), 0, true);
        }
        self.glow(
            Mesh::Blob,
            v3(0, m(24) + dm(4), -m(2)),
            v3(dm(8), dm(8), dm(8)),
            lamp,
        );
        // two inner pylons with platforms 6 m up, each with a vent beside it
        for (px, pz) in [(-m(7), m(7)), (m(7), -m(8))] {
            self.deck(
                px - m(1) - dm(5),
                pz - m(1) - dm(5),
                px + m(1) + dm(5),
                pz + m(1) + dm(5),
                m(6),
                &[(px, pz)],
            );
            self.vent(px + m(2) + dm(5), 0, pz);
        }
    }

    fn hall(&mut self) {
        let ice = self.c("ice_blue");
        let deep = self.c("ice_deep");
        let white = self.c("ice_white");
        let glow = self.c("ice_glow");
        let roof_y = m(14);
        // the roof, open over the middle, icicles under it
        for (x0, z0, x1, z1) in [
            (-HALF - m(6), -HALF - m(6), HALF + m(6), -m(8)),
            (-HALF - m(6), m(8), HALF + m(6), HALF + m(6)),
            (-HALF - m(6), -m(8), -m(8), m(8)),
            (m(8), -m(8), HALF + m(6), m(8)),
        ] {
            self.solids
                .push(Solid::new(v3(x0, roof_y, z0), v3(x1, roof_y + m(2), z1)));
            self.deco(
                Mesh::Cube,
                v3((x0 + x1) / 2, roof_y + m(1), (z0 + z1) / 2),
                v3(x1 - x0, m(2), z1 - z0),
                deep,
            );
        }
        for _ in 0..30 {
            let x = dm(self.rng.range(-260, 260));
            let z = dm(self.rng.range(-260, 260));
            if x.abs() < m(8) && z.abs() < m(8) {
                continue;
            }
            let len = self.metres(1, 4);
            self.icicle(x, roof_y, z, len);
        }
        // columns from the floor to the roof
        for (x, z) in [
            (-m(10), -m(10)),
            (m(10), -m(10)),
            (-m(10), m(10)),
            (m(10), m(10)),
            (-m(17), m(3)),
            (m(17), m(5)),
        ] {
            self.solids.push(Solid::stand(x, 0, z, dm(24), roof_y, dm(24)));
            self.deco(Mesh::Cyl, v3(x, roof_y / 2, z), v3(dm(26), roof_y, dm(26)), ice);
            self.deco(Mesh::Cone, v3(x, roof_y - m(1), z), v3(m(5), m(2), m(5)), white);
            self.deco(Mesh::Cyl, v3(x, m(1), z), v3(dm(34), m(2), dm(34)), deep);
        }
        // arches between the column pairs, 5 m up, reached by vent
        self.deck(-m(8) - dm(8), -m(11), m(8) + dm(8), -m(9), m(5), &[]);
        self.deck(-m(8) - dm(8), m(9), m(8) + dm(8), m(11), m(5), &[]);
        self.vent(0, 0, -m(7));
        self.vent(0, 0, m(7) - dm(5));
        // a ledge round three walls, 5 m up, stairs at its south ends
        let lh = m(5);
        self.deck(
            -m(27),
            -m(22),
            -m(22),
            m(18),
            lh,
            &[(-m(24) - dm(5), -m(10)), (-m(24) - dm(5), m(8))],
        );
        self.deck(
            -m(27),
            -m(27),
            m(27),
            -m(22),
            lh,
            &[(-m(12), -m(24) - dm(5)), (m(12), -m(24) - dm(5))],
        );
        self.deck(
            m(22),
            -m(22),
            m(27),
            m(18),
            lh,
            &[(m(24) + dm(5), -m(12)), (m(24) + dm(5), m(10))],
        );
        let steps = (lh + dm(4) - 1) / dm(4);
        self.stairs(-m(24) - dm(5), 0, m(18) + dm(6) * steps, 0, -ONE, lh, m(3));
        self.stairs(m(24) + dm(5), 0, m(18) + dm(6) * steps, 0, -ONE, lh, m(3));
        // the frozen pool under the opening, crystals that glow
        self.shape(
            Mesh::Cyl,
            boxed(v3(0, dm(1) / 4, 0), v3(m(12), dm(1) / 4, m(12))),
            glow,
            ONE / 4,
            false,
        );
        self.shape(
            Mesh::Ring,
            boxed(v3(0, dm(1) / 2, 0), v3(m(13), dm(1) / 2, m(13))),
            white,
            ONE / 2,
            false,
        );
        for k in 0..8 {
            let a = fx::TURN / 8 * k + deg(10);
            let r = m(20);
            let (x, z) = (fx::mul(fx::cos(a), r), fx::mul(fx::sin(a), r));
            if x.abs() < m(5) || z.abs() < m(5) {
                continue;
            }
            for j in 0..3 {
                let p = v3(x + dm(6) * (j - 1), 0, z + dm(4) * (j % 2));
                let s = dm(6) + dm(4) * j;
                let tilt = Affine::translate(p.add(v3(0, s, 0)))
                    .then(&Affine::rot_z(deg(12) * (j - 1)))
                    .then(&Affine::scale(v3(s / 2, s * 2, s / 2)));
                self.shape(Mesh::Octa, tilt, glow, ONE * 2 / 3, true);
            }
        }
    }

    fn landing(&mut self) {
        // the expedition's camp on the shelf: tents, crates, the drill, the mast
        for (x, z, along) in [
            (-m(12), m(8), true),
            (-m(20), m(4), false),
            (m(13), m(6), true),
            (m(19), -m(6), false),
            (-m(10), -m(14), true),
        ] {
            self.tent(x, z, along);
        }
        for (x, z) in [(-m(7), m(14)), (m(8), m(15)), (-m(16), m(14)), (m(16), -m(16))] {
            self.crate_stack(x, 0, z);
            self.drum(x + m(2), 0, z);
        }
        // the drill rig: a frame tower over a bore hole
        let metal = self.c("ice_metal");
        let dark = self.c("ice_metal_dark");
        let red = self.c("ice_red");
        let (dx, dz) = (m(10), -m(12));
        self.block(dx, 0, dz, m(4), dm(6), m(4), dark);
        for (lx, lz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
            let foot = v3(dx + lx * m(2), dm(6), dz + lz * m(2));
            self.rod(foot, v3(dx, m(14), dz), dm(2), red, true);
        }
        for k in 1..5 {
            let y = m(3) * k;
            let r = m(2) * (14 * 10 - y * 10 / ONE) / 140;
            for (ax, az, bx, bz) in [(-1, -1, 1, -1), (1, -1, 1, 1), (1, 1, -1, 1), (-1, 1, -1, -1)] {
                self.rod(
                    v3(dx + ax * r, y, dz + az * r),
                    v3(dx + bx * r, y, dz + bz * r),
                    dm(1),
                    metal,
                    false,
                );
            }
        }
        self.glow(
            Mesh::Blob,
            v3(dx, m(14) + dm(3), dz),
            v3(dm(6), dm(6), dm(6)),
            self.c("ice_lamp"),
        );
        // a loading gantry with a stair, over the crates
        self.deck(
            -m(22),
            -m(12),
            -m(14),
            -m(6),
            m(4),
            &[(-m(21), -m(9)), (-m(15), -m(9))],
        );
        self.stairs(-m(18), 0, -m(6) + dm(6) * 10, 0, -ONE, m(4), m(3));
        for k in 0..4 {
            self.camp_lamp(if k % 2 == 0 { -m(6) } else { m(6) }, 0, m(18) - m(9) * k);
        }
        // the expedition's ship, moored out past the shelf's edge to the south
        let hull = self.c("ice_hull");
        let hull_dark = self.c("ice_hull_dark");
        let (sx, sz) = (m(14), HALF + m(26));
        self.deco(Mesh::Cube, v3(sx, m(3), sz), v3(m(40), m(7), m(9)), hull);
        self.deco(
            Mesh::Cube,
            v3(sx, m(1), sz),
            v3(m(40) + dm(2), dm(5), m(9) + dm(2)),
            hull_dark,
        );
        let bow = Affine::translate(v3(sx - m(23), m(3), sz))
            .then(&Affine::rot_y(fx::QUARTER))
            .then(&Affine::scale(v3(m(9), m(7), m(6))));
        self.shape(Mesh::Wedge, bow, hull, 0, true);
        self.deco(
            Mesh::Cube,
            v3(sx + m(8), m(8), sz),
            v3(m(10), m(4), m(7)),
            self.c("ice_white"),
        );
        self.facade(
            v3(sx + m(8), m(8), sz - m(3) - dm(5)),
            v3(0, 0, -ONE),
            m(8),
            m(2),
            self.c("ice_lamp"),
            ONE / 2,
        );
        for (mx, top) in [(sx - m(6), m(22)), (sx + m(4), m(18))] {
            self.rod(v3(mx, m(6), sz), v3(mx, top, sz), dm(4), dark, false);
        }
        self.deco(Mesh::Cyl, v3(sx + m(14), m(11), sz), v3(m(3), m(4), m(3)), red);
        let water = self.c("ice_water");
        self.shape(
            Mesh::Cube,
            // its top over the ground beyond (at −0.2 m), not level with it
            boxed(v3(0, -dm(2), sz), v3(m(200), dm(2), m(30))),
            water,
            ONE / 6,
            false,
        );
    }

    fn station(&mut self) {
        // the weather station: a hut, its stove (the heal), its instruments
        let wood = self.c("wood");
        let red = self.c("ice_red");
        let metal = self.c("ice_metal");
        let dark = self.c("ice_metal_dark");
        let lamp = self.c("ice_lamp");
        let white = self.c("ice_white");
        self.solids
            .push(Solid::stand(0, 0, 0, m(2) + dm(4), m(1), m(2) + dm(4)));
        self.deco(
            Mesh::Cube,
            v3(0, dm(5), 0),
            v3(m(2) + dm(4), m(1), m(2) + dm(4)),
            dark,
        );
        self.glow(
            Mesh::Cube,
            v3(0, m(1) + dm(1), 0),
            v3(m(2), dm(1), m(2)),
            self.c("ice_orange"),
        );
        self.glow(
            Mesh::Blob,
            v3(0, m(2), 0),
            v3(dm(8), dm(8), dm(8)),
            self.c("pickup_heal"),
        );
        self.rod(v3(0, m(1), 0), v3(0, m(4), 0), dm(3), dark, false);
        // the hut to the north, windows lit
        self.block(0, 0, -m(14), m(10), m(4), m(6), red);
        let roof =
            Affine::translate(v3(0, m(4) + dm(8), -m(14))).then(&Affine::scale(v3(m(11), dm(16), m(7))));
        self.shape(Mesh::Wedge, roof, white, 0, true);
        self.facade(v3(0, m(2), -m(11)), v3(0, 0, ONE), m(7), m(2), lamp, ONE / 4);
        // the anemometer's mast, a screen box on legs, a tethered balloon
        self.rod(v3(m(8), 0, -m(8)), v3(m(8), m(8), -m(8)), dm(2), metal, true);
        for k in 0..3 {
            let a = fx::TURN / 3 * k;
            let tip = v3(
                m(8) + fx::mul(fx::cos(a), dm(8)),
                m(8),
                -m(8) + fx::mul(fx::sin(a), dm(8)),
            );
            self.rod(v3(m(8), m(8), -m(8)), tip, dm(1) / 2, dark, false);
            self.deco(Mesh::Sphere, tip, v3(dm(2), dm(2), dm(2)), white);
        }
        self.block(-m(8), m(1), -m(8), dm(10), dm(8), dm(8), white);
        for (lx, lz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
            let a = v3(-m(8) + lx * dm(4), 0, -m(8) + lz * dm(3));
            self.rod(a, a.add(v3(0, m(1), 0)), dm(1) / 2, wood, false);
        }
        self.rod(v3(m(12), 0, m(8)), v3(m(14), m(16), m(4)), dm(1) / 4, dark, false);
        self.deco(
            Mesh::Sphere,
            v3(m(14), m(17) + dm(5), m(4)),
            v3(m(3), m(3), m(3)),
            white,
        );
        for k in 0..4 {
            let a = fx::TURN / 4 * k + deg(45);
            let (x, z) = (fx::mul(fx::cos(a), m(8)), fx::mul(fx::sin(a), m(8)));
            self.bench(x, 0, z, k % 2 == 0);
            self.camp_lamp(fx::mul(fx::cos(a), m(12)), 0, fx::mul(fx::sin(a), m(12)));
        }
    }

    /// The Witness Stand: a hollow in the ice round a frozen lake, figures
    /// held in its walls, pillars, four raised stands (two joined by a
    /// bridge), vents beside them, and the spire the Witness waited in.
    fn witness_stand(&mut self) {
        let r = COURT_RADIUS;
        let ice = self.c("ice_blue");
        let deep = self.c("ice_deep");
        let white = self.c("ice_white");
        let rock = self.c("ice_rock_dark");
        let water = self.c("ice_water");
        let glow = self.c("ice_glow");
        let ink = self.c("ice_ink");
        for k in 0..32 {
            let a = fx::TURN / 32 * k;
            let (x, z) = (fx::mul(fx::cos(a), r + m(2)), fx::mul(fx::sin(a), r + m(2)));
            let h = m(16) + dm(self.rng.range(0, 80));
            let rot = Affine::translate(v3(x, h / 2, z)).then(&Affine::rot_y(fx::QUARTER - a));
            self.solids.push(Solid::stand(x, 0, z, m(5), m(30), m(5)));
            self.shape(
                Mesh::Cube,
                rot.then(&Affine::scale(v3(m(6) + dm(2), h, m(3)))),
                if k % 2 == 0 { ice } else { deep },
                0,
                true,
            );
            self.shape(
                Mesh::Cube,
                rot.then(&Affine::translate(v3(0, -h / 2 + m(2), 0)))
                    .then(&Affine::scale(v3(m(6) + dm(4), m(4), m(3) + dm(4)))),
                rock,
                0,
                true,
            );
            // someone the ice kept: a dark figure behind a pane of light
            let inner = v3(fx::mul(fx::cos(a), r), 0, fx::mul(fx::sin(a), r));
            if k % 3 == 1 {
                let fig = inner.add(v3(0, m(2), 0));
                self.deco(Mesh::Cone, fig, v3(dm(9), m(2), dm(9)), ink);
                self.deco(
                    Mesh::Sphere,
                    fig.add(v3(0, m(1) + dm(3), 0)),
                    v3(dm(5), dm(5), dm(5)),
                    ink,
                );
                let pane = Affine::translate(inner.add(v3(0, m(2), 0)).sub(v3(
                    fx::mul(fx::cos(a), dm(5)),
                    0,
                    fx::mul(fx::sin(a), dm(5)),
                )))
                .then(&Affine::rot_y(fx::QUARTER - a))
                .then(&Affine::scale(v3(m(2), m(4), dm(1))));
                self.shape(Mesh::Cube, pane, glow, ONE / 4, false);
            }
        }
        // the frozen lake: dark ice, cracks out from the middle, rings
        self.shape(
            Mesh::Cyl,
            boxed(v3(0, dm(1) / 5, 0), v3(r * 2, dm(1) / 5, r * 2)),
            water,
            ONE / 8,
            false,
        );
        for k in 0..12 {
            let a = fx::TURN / 12 * k + deg(self.rng.range(-8, 8));
            let mut p = v3(0, dm(1) / 3, 0);
            for j in 1..5 {
                let q = v3(
                    fx::mul(fx::cos(a + deg(self.rng.range(-12, 12))), m(6) * j),
                    dm(1) / 3,
                    fx::mul(fx::sin(a + deg(self.rng.range(-12, 12))), m(6) * j),
                );
                self.rod(p, q, dm(1) / 2, white, false);
                p = q;
            }
        }
        for rr in [m(10), m(20)] {
            self.shape(
                Mesh::Ring,
                boxed(v3(0, dm(1) / 2, 0), v3(rr * 2, dm(1) / 2, rr * 2)),
                glow,
                ONE / 2,
                false,
            );
        }
        // four stands 3 m up; the two southern ones joined by a bridge
        for (sx, sz) in [(1, 1), (-1, 1), (1, -1), (-1, -1)] {
            let (x, z) = (sx * m(16) + sx * dm(3), sz * m(16) + sz * dm(3));
            self.platform(x, z, m(5), m(5), m(3), (0, -sz * ONE));
            self.vent(x - sx * m(4), 0, z - sz * m(4));
        }
        let zb = m(16) + dm(3);
        self.deck(
            -m(14),
            zb - m(1) - dm(5),
            m(14),
            zb + m(1) + dm(5),
            m(3),
            &[(-m(5), zb), (m(5), zb)],
        );
        // pillars of ice to hide behind
        for k in 0..8 {
            let a = fx::TURN / 8 * k + deg(22);
            let pr = if k == 1 || k == 2 { m(11) } else { m(17) };
            let (x, z) = (fx::mul(fx::cos(a), pr), fx::mul(fx::sin(a), pr));
            self.solids.push(Solid::stand(x, 0, z, dm(18), m(6), dm(18)));
            let tilt = Affine::translate(v3(x, m(3), z))
                .then(&Affine::rot_y(a))
                .then(&Affine::scale(v3(m(2) + dm(4), m(7), m(2) + dm(4))));
            self.shape(Mesh::Octa, tilt, ice, 0, true);
            self.deco(
                Mesh::Cyl,
                v3(x, m(1), z),
                v3(m(2) + dm(2), m(2), m(2) + dm(2)),
                deep,
            );
        }
        // the spire it waited in, broken open, north
        for k in 0..7 {
            let a = fx::TURN / 7 * k;
            let p = v3(
                fx::mul(fx::cos(a), m(3)),
                0,
                -r + m(5) + fx::mul(fx::sin(a), m(3)),
            );
            let hgt = m(10) + m(k % 3) * 4;
            let tilt = Affine::translate(p.add(v3(0, hgt / 2, 0)))
                .then(&Affine::rot_y(a))
                .then(&Affine::rot_z(deg(10 + 4 * (k % 3))))
                .then(&Affine::scale(v3(m(2), hgt, m(2))));
            self.shape(
                Mesh::Octa,
                tilt,
                if k % 2 == 0 { white } else { ice },
                ONE / 10,
                true,
            );
        }
        self.solids.push(Solid::stand(0, 0, -r + m(5), m(6), m(10), m(5)));
        // the ground out to the mountains, and the bound
        self.shape(
            Mesh::Ground,
            boxed(v3(0, -dm(2), 0), v3(m(1200), ONE, m(1200))),
            self.c("ice_snow_dark"),
            0,
            false,
        );
        self.solids
            .push(Solid::new(v3(-m(60), -m(1), -m(60)), v3(m(60), 0, m(60))));
        self.wall(v3(-r - m(10), -m(2), -r - m(10)), v3(r + m(10), m(80), -r - m(4)));
        self.wall(v3(-r - m(10), -m(2), r + m(4)), v3(r + m(10), m(80), r + m(10)));
        self.wall(v3(-r - m(10), -m(2), -r - m(10)), v3(-r - m(4), m(80), r + m(10)));
        self.wall(v3(r + m(4), -m(2), -r - m(10)), v3(r + m(10), m(80), r + m(10)));
    }
}
