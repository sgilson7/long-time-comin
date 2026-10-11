//! Zone four, the Ascent (the evening of 14 March 2081, always 21:14): the
//! climb up Mount Meridian to the Observatory on its summit. Each arena is a
//! terrace cut into the mountain, the cliff rising behind it to the north
//! and the drop open on the other three sides, and each stands `rise`
//! higher than the arena before it: what is below (the mountain's flanks,
//! a layer of cloud, the city's lights on the plain) is drawn that far
//! down, so every arena is visibly higher than the last. The square and
//! its gates are zone one's, so the route, the spawn grid and the reach
//! tests apply. Off an edge is a fall, which returns you at a cost.
//!
//! - the Switchback: terraces stepping up toward the cliff, stairs between
//! - the Span: a gap across the terrace, a stone bridge low and a rope
//!   bridge high
//! - the Spire: a pillar of rock with platforms climbing round it
//! - the Ruined Chapel: columns, broken walls, an apse up a stair
//! - the Ledge: half of it under an overhang, a cave mouth, boulders
//!
//! and the Trailhead (arrival), the Hermitage (rest) and the Circle (the
//! boss): a round summit, trilithons round it, the Long Door standing in it.

use super::{boxed, dm, m, Side, B, COURT_RADIUS, GATE_GAP, HALF};
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine};
use crate::level::Solid;
use crate::mesh::Mesh;
use crate::render::shade;

/// The summit, for the Circle: higher than any step of the map reaches.
const SUMMIT: i32 = fx::int(420);
/// the Span's gap runs east to west between these two z values
const GAP: (i32, i32) = (-fx::int(7), fx::int(1));

#[allow(clippy::too_many_arguments)]
impl B<'_> {
    pub(super) fn ascent(&mut self, kind: &str, exits: &[Side], rise: i32) {
        if kind == "boss" {
            self.circle();
            self.below(COURT_RADIUS, SUMMIT);
            return;
        }
        self.terrace(HALF, kind == "span");
        let gaps = [
            true,
            exits.contains(&Side::North),
            exits.contains(&Side::East),
            exits.contains(&Side::West),
        ];
        self.mountain_edges(HALF, gaps);
        match kind {
            "arrival" => self.trailhead(),
            "rest" => self.hermitage(),
            "span" => self.span(),
            "spire" => self.spire(),
            "chapel" => self.chapel(),
            "ledge" => self.ledge(),
            _ => self.switchback(),
        }
        if matches!(kind, "switchback" | "spire" | "chapel" | "ledge") {
            self.cover_walls(HALF, 4);
        }
        self.below(HALF, rise);
    }

    // ---- the mountain -----------------------------------------------------

    /// The terrace's floor: stone with moss and snow in patches; `gap`
    /// leaves the Span's chasm.
    fn terrace(&mut self, half: i32, gap: bool) {
        let o = half + m(2);
        let stone = self.c("ascent_stone");
        let pieces: Vec<(i32, i32)> = if gap {
            vec![(-o, GAP.0), (GAP.1, o)]
        } else {
            vec![(-o, o)]
        };
        for (z0, z1) in pieces {
            self.solids.push(Solid::new(v3(-o, -m(3), z0), v3(o, 0, z1)));
            self.shape(
                Mesh::Cube,
                boxed(v3(0, -dm(15), (z0 + z1) / 2), v3(o * 2, m(3), z1 - z0)),
                stone,
                0,
                true,
            );
        }
        let moss = self.c("ascent_moss");
        let snow = self.c("ascent_snow");
        for _ in 0..40 {
            let x = dm(self.rng.range(-(half / dm(1)) + 10, half / dm(1) - 10));
            let z = dm(self.rng.range(-(half / dm(1)) + 10, half / dm(1) - 10));
            if gap && z > GAP.0 - m(2) && z < GAP.1 + m(2) {
                continue;
            }
            let r = self.metres(1, 3);
            let c = if self.rng.chance(60) { moss } else { snow };
            self.shape(
                Mesh::Blob,
                boxed(v3(x, dm(1) / 4, z), v3(r * 2, dm(1) / 2, r * 3 / 2)),
                c,
                0,
                false,
            );
        }
    }

    /// The cliff to the north, rising out of sight, and the drop's edge on
    /// the other sides: a low parapet of stones, an arch where a gate is,
    /// and a path on past it into the rock.
    fn mountain_edges(&mut self, half: i32, gaps: [bool; 4]) {
        let rock = self.c("ascent_rock");
        let dark = self.c("ascent_stone_dark");
        let snow = self.c("ascent_snow");
        // the cliff: blocks of rock, ragged, up to 60 m
        let mut x = -half - m(8);
        while x < half + m(8) {
            let w = self.metres(6, 12);
            let h = self.metres(30, 60);
            let d = self.metres(8, 14);
            let z = -half - m(2) - d / 2 - dm(self.rng.range(0, 20));
            let gate = gaps[1] && (x + w / 2).abs() < GATE_GAP / 2 + w / 2;
            if !gate {
                self.solids.push(Solid::stand(x + w / 2, 0, z, w, h, d));
            } else {
                // the north gate's way on: a cut in the cliff, a stair up it
                self.solids.push(Solid::stand(x + w / 2, 0, z - m(4), w, h, d));
                self.stairs(0, 0, -half - m(1), 0, -ONE, m(3), GATE_GAP);
            }
            let rot =
                Affine::translate(v3(x + w / 2, h / 2, z)).then(&Affine::rot_y(deg(self.rng.range(-6, 6))));
            self.shape(
                Mesh::Cube,
                rot.then(&Affine::scale(v3(w + dm(4), h, d))),
                rock,
                0,
                true,
            );
            self.deco(Mesh::Wedge, v3(x + w / 2, h + m(1), z), v3(w, m(2), d), snow);
            x += w - dm(5);
        }
        // the other three edges: parapets, arches, the ways on
        for (si, side) in [Side::South, Side::East, Side::West].iter().enumerate() {
            let gi = [0, 2, 3][si];
            let (n, along) = match side {
                Side::South => (v3(0, 0, ONE), v3(ONE, 0, 0)),
                Side::East => (v3(ONE, 0, 0), v3(0, 0, ONE)),
                _ => (v3(-ONE, 0, 0), v3(0, 0, ONE)),
            };
            let edge = half + m(1);
            // out past the corners, so none is open
            let mut a = -half - m(2);
            while a <= half + m(2) {
                let in_gate = gaps[gi] && a.abs() < GATE_GAP / 2 + m(1);
                if !in_gate {
                    let p = n.scale(edge).add(along.scale(a));
                    let s = self.metres(1, 2);

                    self.deco(
                        Mesh::Octa,
                        p.add(v3(0, dm(4), 0)),
                        v3(s + dm(3), dm(9), s + dm(3)),
                        dark,
                    );
                }
                a += m(2) + dm(5);
            }
            // the parapet itself, unbroken but for the gate: knee-high, to
            // be jumped, never walked through
            let spans: Vec<(i32, i32)> = if gaps[gi] {
                vec![
                    (-half - m(3), -(GATE_GAP / 2 + m(1))),
                    (GATE_GAP / 2 + m(1), half + m(3)),
                ]
            } else {
                vec![(-half - m(3), half + m(3))]
            };
            for (a0, a1) in spans {
                let c = n.scale(edge).add(along.scale((a0 + a1) / 2));
                let len = a1 - a0;
                let (w, d) = if along.x != 0 { (len, dm(8)) } else { (dm(8), len) };
                self.solids.push(Solid::stand(c.x, 0, c.z, w, dm(7), d));
            }
            if gaps[gi] {
                // an arch of two stones and a lintel over the gate
                let g = n.scale(half);
                for s in [-1, 1] {
                    let p = g.add(along.scale(s * (GATE_GAP / 2 + m(1))));
                    self.block(p.x, 0, p.z, m(1) + dm(4), m(6), m(1) + dm(4), dark);
                }
                let lw = if along.x != 0 {
                    v3(GATE_GAP + m(4), m(1), m(1) + dm(4))
                } else {
                    v3(m(1) + dm(4), m(1), GATE_GAP + m(4))
                };
                self.deco(Mesh::Cube, g.add(v3(0, m(6) + dm(5), 0)), lw, rock);
                // the path on, round the mountain, and its end
                let back = g.add(n.scale(m(5)));
                let (bw, bd) = if along.x != 0 {
                    (GATE_GAP + m(2), m(1))
                } else {
                    (m(1), GATE_GAP + m(2))
                };
                self.solids.push(Solid::stand(back.x, 0, back.z, bw, m(8), bd));
                let path = g.add(n.scale(m(3)));
                let (pw, pd) = if along.x != 0 {
                    (GATE_GAP, m(5))
                } else {
                    (m(5), GATE_GAP)
                };
                self.solids.push(Solid::new(
                    v3(path.x - pw / 2, -m(3), path.z - pd / 2),
                    v3(path.x + pw / 2, 0, path.z + pd / 2),
                ));
                self.deco(
                    Mesh::Cube,
                    path.add(v3(0, -dm(15), 0)),
                    v3(pw, m(3), pd),
                    self.c("ascent_stone"),
                );
                self.deco(
                    Mesh::Octa,
                    back.add(v3(0, m(4), 0)),
                    v3(bw + m(1), m(8), bd + m(1)),
                    rock,
                );
            }
        }
    }

    /// Everything below the terrace, `rise` down: the mountain's flanks
    /// falling away, a layer of cloud, the plain and the city's lights.
    fn below(&mut self, half: i32, rise: i32) {
        let rock = self.c("ascent_rock");
        let dark = self.c("ascent_stone_dark");
        let snow = self.c("ascent_snow");
        let depth = rise + m(80);
        // the flanks: rock shelving down and out
        for k in 0..18 {
            let a = fx::TURN / 18 * k + deg(self.rng.range(-6, 6));
            let r = half + m(4) + self.metres(0, 10);
            let (x, z) = (fx::mul(fx::cos(a), r), fx::mul(fx::sin(a), r));
            if z < -half {
                continue;
            }
            let top = -m(3);
            let h = depth;
            let w = self.metres(16, 30);
            let flank = Affine::translate(v3(x, top - h / 2, z))
                .then(&Affine::rot_y(fx::QUARTER - a))
                .then(&Affine::scale(v3(w, h, w * 3 / 2)));
            self.shape(
                Mesh::Cone,
                flank.then(&Affine::rot_x(fx::HALF)),
                if k % 2 == 0 { rock } else { dark },
                0,
                true,
            );
        }
        // the mountain behind and above, its snowy shoulders (none over the
        // summit itself)
        for k in 0..if rise >= SUMMIT { 0 } else { 5 } {
            let x = -m(80) + m(40) * k;
            let h = m(140) + self.metres(0, 60);
            self.deco(
                Mesh::Cone,
                v3(x, h / 2 - m(20), -half - m(60)),
                v3(m(70), h, m(50)),
                rock,
            );
            self.deco(
                Mesh::Cone,
                v3(x, h - m(30), -half - m(60)),
                v3(m(30), m(30), m(22)),
                snow,
            );
        }
        // cloud, partway down, in drifts
        let cloud = self.c("ascent_cloud");
        let cy = -rise / 2 - m(20);
        for _ in 0..26 {
            let a = deg(self.rng.range(0, 359));
            let r = self.metres(60, 260);
            let p = v3(
                fx::mul(fx::cos(a), r),
                cy + self.metres(-8, 8),
                fx::mul(fx::sin(a), r),
            );
            if p.z < -half - m(20) {
                continue;
            }
            let s = self.metres(20, 50);
            self.shape(
                Mesh::Blob,
                boxed(p, v3(s * 2, s / 3, s * 3 / 2)),
                cloud,
                ONE / 4,
                true,
            );
        }
        // the plain, far down, and Meridian's lights on it
        let plain = self.c("ascent_plain");
        self.shape(
            Mesh::Ground,
            boxed(v3(0, -depth, 0), v3(m(1600), ONE, m(1600))),
            plain,
            0,
            false,
        );
        let lamp = self.c("lamp");
        for _ in 0..140 {
            let a = deg(self.rng.range(0, 359));
            let r = self.metres(120, 600);
            let p = v3(fx::mul(fx::cos(a), r), -depth + m(2), fx::mul(fx::sin(a), r));
            if p.z < -half - m(40) {
                continue;
            }
            let s = self.metres(4, 12);
            self.glow(Mesh::Cube, p, v3(s, s / 3, s), lamp);
        }
        // the city's streets, lit lines across the plain
        for _ in 0..12 {
            let a = deg(self.rng.range(0, 179));
            let c = v3(self.metres(-300, 300), -depth + m(1), self.metres(80, 500));
            let d = v3(fx::mul(fx::cos(a), m(200)), 0, fx::mul(fx::sin(a), m(200)));
            self.rod(c.sub(d), c.add(d), m(2), lamp, false);
        }
        // the evening sun, low in the west, never setting
        self.halo(
            Mesh::Sphere,
            boxed(v3(-m(500), m(40), -m(200)), v3(m(80), m(80), m(80))),
            self.c("ascent_sun"),
            ONE / 2,
        );
    }

    // ---- the terraces -------------------------------------------------------

    fn switchback(&mut self) {
        // three terraces, each higher, toward the cliff; stairs alternate
        let h1 = m(3) + dm(5);
        let h2 = m(7);
        // in two wings, the path to the north gate open between them
        for s in [-1, 1] {
            self.block(s * m(17), 0, -m(10), m(22), h1, m(10), self.c("ascent_stone"));
            self.block(s * m(17), 0, -m(22), m(22), h2, m(8), self.c("ascent_stone_dark"));
        }
        let steps1 = (h1 + dm(4) - 1) / dm(4);
        self.stairs(m(18), 0, -m(5) + dm(6) * steps1, 0, -ONE, h1, m(4));
        let steps2 = (h2 - h1 + dm(4) - 1) / dm(4);
        self.stairs(-m(18), h1, -m(18) + dm(6) * steps2, 0, -ONE, h2 - h1, m(4));
        // a wooden walkway out over the lower terrace from the middle one
        self.deck(-m(6), -m(5), m(6), m(4), h1, &[(-m(5), m(3)), (m(5), m(3))]);
        self.prayer_flags(-m(24), h2, -m(22), m(24), h2, -m(22));
        self.prayer_flags(-m(26), h1 + m(2), -m(10), m(26), h1 + m(2), -m(10));
        for (x, z) in [(-m(14), m(14)), (m(12), m(12)), (0, m(20))] {
            self.boulder(x, z);
        }
    }

    fn span(&mut self) {
        let (z0, z1) = GAP;
        // the low stone bridge, the high rope bridge with stairs up to it
        self.deck(-m(3), z0 - dm(5), m(3), z1 + dm(5), 0, &[]);
        let hb = m(5);
        self.deck(
            m(14),
            z0 - m(2),
            m(18),
            z1 + m(2),
            hb,
            &[(m(16), z0 - m(1)), (m(16), z1 + m(1))],
        );
        let steps = (hb + dm(4) - 1) / dm(4);
        self.stairs(m(16), 0, z1 + m(2) + dm(6) * steps, 0, -ONE, hb, m(3));
        let rope = self.c("ascent_rope");
        for s in [-1, 1] {
            self.rod(
                v3(m(16) + s * m(2), hb + m(1), z0 - m(2)),
                v3(m(16) + s * m(2), hb + m(1), z1 + m(2)),
                dm(1),
                rope,
                false,
            );
        }
        // the crags either side of the gap
        for (x, z) in [(-m(16), z0 - m(5)), (-m(16), z1 + m(6)), (m(4), z1 + m(10))] {
            self.platform(x, z, m(6), m(5), m(3), (ONE, 0));
        }
        self.prayer_flags(-m(26), m(3), z0, m(26), m(3), z1);
    }

    fn spire(&mut self) {
        // the pillar, and platforms climbing round it
        let rock = self.c("ascent_rock");
        self.solids.push(Solid::stand(0, 0, -m(4), m(6), m(24), m(6)));
        self.deco(Mesh::Cone, v3(0, m(12), -m(4)), v3(m(8), m(26), m(8)), rock);
        let stone = self.c("ascent_stone");
        for (k, (x, z)) in [(m(8), -m(4)), (0, -m(13)), (-m(8), -m(4)), (0, m(5))]
            .iter()
            .enumerate()
        {
            let h = m(3) + m(3) * k as i32 / 2;
            self.block(*x, 0, *z, m(5), h, m(5), stone);
        }
        let hb = m(4) + dm(5);
        self.deck(
            -m(14),
            m(10),
            m(14),
            m(14),
            hb,
            &[(-m(10), m(12)), (m(10), m(12))],
        );
        let steps = (hb + dm(4) - 1) / dm(4);
        self.stairs(-m(12), 0, m(14) + dm(6) * steps, 0, -ONE, hb, m(3));
        self.stairs(m(8), 0, -m(1) - dm(5) + dm(6) * 8, 0, -ONE, m(3), m(3));
        self.prayer_flags(0, m(24), -m(4), -m(24), m(6), m(20));
        self.prayer_flags(0, m(24), -m(4), m(24), m(6), m(20));
    }

    fn chapel(&mut self) {
        let stone = self.c("ascent_stone");
        let dark = self.c("ascent_stone_dark");
        // two rows of columns, some broken
        for s in [-1, 1] {
            for k in 0..5 {
                let (x, z) = (s * m(9), -m(16) + m(7) * k);
                let h = if (k + s) % 3 == 0 { m(3) } else { m(9) };
                self.block(x, 0, z, dm(16), h, dm(16), stone);
                self.deco(Mesh::Cube, v3(x, h + dm(2), z), v3(m(2), dm(4), m(2)), dark);
            }
        }
        // the apse up a stair, a gallery over the nave's south end
        let ha = m(3);
        self.platform(0, -m(22), m(12), m(6), ha, (0, ONE));
        let hg = m(5);
        self.deck(-m(10), m(16), m(10), m(20), hg, &[(-m(6), m(18)), (m(6), m(18))]);
        let steps = (hg + dm(4) - 1) / dm(4);
        self.stairs(m(8), 0, m(16) - dm(6) * steps, 0, ONE, hg, m(3));
        // the fallen bell
        let bell = self.c("brass");
        let tilt = Affine::translate(v3(m(4), dm(9), -m(4)))
            .then(&Affine::rot_z(deg(70)))
            .then(&Affine::scale(v3(m(2), m(2), m(2))));
        self.shape(Mesh::Cone, tilt, bell, ONE / 10, true);
        self.solids
            .push(Solid::stand(m(4), 0, -m(4), m(2), m(1) + dm(5), m(2)));
    }

    fn ledge(&mut self) {
        // the overhang: a roof of rock over the north half, a cave mouth
        let rock = self.c("ascent_rock");
        let roof = m(7);
        self.solids.push(Solid::new(
            v3(-HALF - m(2), roof, -HALF - m(2)),
            v3(HALF + m(2), roof + m(4), -m(6)),
        ));
        self.deco(
            Mesh::Cube,
            v3(0, roof + m(2), -m(17)),
            v3(HALF * 2 + m(4), m(4), m(22)),
            rock,
        );
        for k in 0..8 {
            let x = -m(24) + m(7) * k;
            let len = self.metres(1, 3);
            self.deco(
                Mesh::Cone,
                v3(x, roof - len / 2, -m(7)),
                v3(m(1), len, m(1)),
                rock,
            );
        }
        // a shelf up to it, reached by stairs
        let hs = m(3) + dm(5);
        self.block(m(18), 0, -m(16), m(14), hs, m(14), self.c("ascent_stone"));
        let steps = (hs + dm(4) - 1) / dm(4);
        self.stairs(m(18), 0, -m(9) + dm(6) * steps, 0, -ONE, hs, m(3));
        for (x, z) in [(-m(14), m(8)), (-m(4), m(16)), (m(10), m(6)), (-m(18), -m(14))] {
            self.boulder(x, z);
        }
        let lamp = self.c("lamp");
        self.glow(Mesh::Blob, v3(-m(6), m(1), -m(24)), v3(m(1), m(1), m(1)), lamp);
        // a climbers' scaffold by the cave mouth
        let hd = m(4);
        self.deck(-m(22), -m(2), -m(12), m(2), hd, &[(-m(20), 0), (-m(14), 0)]);
        let steps = (hd + dm(4) - 1) / dm(4);
        self.stairs(-m(17), 0, m(2) + dm(6) * steps, 0, -ONE, hd, m(3));
    }

    fn trailhead(&mut self) {
        // the path's start: a wooden gate, a signpost, lanterns, a shrine
        let wood = self.c("wood");
        for s in [-1, 1] {
            self.block(s * m(3), 0, m(14), dm(5), m(4), dm(5), wood);
        }
        self.deco(Mesh::Cube, v3(0, m(4), m(14)), v3(m(7), dm(4), dm(5)), wood);
        self.block(-m(10), 0, -m(6), dm(3), m(2), dm(3), wood);
        self.deco(Mesh::Cube, v3(-m(9), m(2), -m(6)), v3(m(2), dm(5), dm(1)), wood);
        let lamp = self.c("lamp");
        for k in 0..6 {
            let z = m(20) - m(8) * k;
            for s in [-1, 1] {
                self.rod(v3(s * m(6), 0, z), v3(s * m(6), m(2), z), dm(1), wood, true);
                self.glow(
                    Mesh::Blob,
                    v3(s * m(6), m(2) + dm(2), z),
                    v3(dm(4), dm(5), dm(4)),
                    lamp,
                );
            }
        }
        let hl = m(4);
        self.platform(m(18), -m(16), m(8), m(6), hl, (0, ONE));
        self.prayer_flags(-m(24), m(4), -m(24), m(24), m(4), -m(24));
    }

    fn hermitage(&mut self) {
        // the hermit's shrine: a roofed alcove, a basin (the fountain), a bench
        let stone = self.c("ascent_stone");
        self.solids.push(Solid::stand(0, 0, 0, m(2), dm(8), m(2)));
        self.deco(
            Mesh::Cyl,
            v3(0, dm(4), 0),
            v3(m(2) + dm(4), dm(8), m(2) + dm(4)),
            stone,
        );
        self.glow(Mesh::Cyl, v3(0, dm(8), 0), v3(m(2), dm(1), m(2)), self.c("water"));
        self.block(0, 0, -m(14), m(8), m(4), m(4), stone);
        self.deco(
            Mesh::Wedge,
            v3(0, m(4) + dm(8), -m(14)),
            v3(m(9), dm(16), m(5)),
            self.c("roof"),
        );
        self.glow(
            Mesh::Blob,
            v3(0, m(2), -m(11) - dm(8)),
            v3(dm(6), dm(6), dm(6)),
            self.c("lamp"),
        );
        self.block(-m(8), 0, m(6), m(3), dm(5), dm(8), self.c("wood"));
        self.prayer_flags(-m(20), m(5), -m(10), m(20), m(5), -m(10));
    }

    /// The Circle at the summit: a round plateau over the drop, a ring of
    /// trilithons, bluestones nearer in, an altar, and the Long Door.
    fn circle(&mut self) {
        let r = COURT_RADIUS;
        let stone = self.c("ascent_stone");
        let dark = self.c("ascent_stone_dark");
        let moss = self.c("ascent_moss");
        self.solids.push(Solid::new(
            v3(-r - m(2), -m(3), -r - m(2)),
            v3(r + m(2), 0, r + m(2)),
        ));
        self.shape(
            Mesh::Cyl,
            boxed(v3(0, -dm(15), 0), v3(r * 2 + m(4), m(3), r * 2 + m(4))),
            stone,
            0,
            true,
        );
        self.shape(
            Mesh::Ring,
            boxed(v3(0, dm(1) / 3, 0), v3(m(30), dm(1) / 3, m(30))),
            moss,
            0,
            false,
        );
        // a ring of low stones round the edge, unbroken
        for k in 0..32 {
            let a = fx::TURN / 32 * k;
            let (x, z) = (fx::mul(fx::cos(a), r + m(1)), fx::mul(fx::sin(a), r + m(1)));
            let w = m(7);
            let (sw, sd) = (
                fx::mul(w, fx::sin(a).abs()).max(dm(8)),
                fx::mul(w, fx::cos(a).abs()).max(dm(8)),
            );
            self.solids.push(Solid::stand(x, 0, z, sw, dm(7), sd));
            self.deco(Mesh::Octa, v3(x, dm(4), z), v3(m(2), dm(9), m(2)), dark);
        }
        // the trilithons
        for k in 0..10 {
            let a = fx::TURN / 10 * k + deg(18);
            let (x, z) = (fx::mul(fx::cos(a), m(20)), fx::mul(fx::sin(a), m(20)));
            let rot = Affine::rot_y(fx::QUARTER - a);
            for s in [-1, 1] {
                let p = v3(x, 0, z).add(rot.apply_dir(v3(s * m(1) + s * dm(5), 0, 0)));
                self.solids
                    .push(Solid::stand(p.x, 0, p.z, m(1) + dm(4), m(6), m(1) + dm(4)));
                let up = Affine::translate(p.add(v3(0, m(3), 0)))
                    .then(&rot)
                    .then(&Affine::scale(v3(m(1) + dm(4), m(6), m(1) + dm(2))));
                self.shape(Mesh::Cube, up, if s < 0 { stone } else { dark }, 0, true);
            }
            let lintel = Affine::translate(v3(x, m(6) + dm(5), z))
                .then(&rot)
                .then(&Affine::scale(v3(m(4) + dm(4), m(1), m(1) + dm(4))));
            self.shape(Mesh::Cube, lintel, stone, 0, true);
        }
        // the bluestones, the altar
        for k in 0..6 {
            let a = fx::TURN / 6 * k;
            let (x, z) = (fx::mul(fx::cos(a), m(11)), fx::mul(fx::sin(a), m(11)));
            self.block(x, 0, z, m(1), m(2) + dm(5), dm(8), self.c("ascent_blue"));
        }
        self.block(0, 0, m(2), m(4), dm(9), m(2), dark);
        // the Long Door: a frame of stone, light standing in it
        let z = -m(6);
        for s in [-1, 1] {
            self.block(s * m(2), 0, z, dm(8), m(5), dm(8), stone);
        }
        self.deco(Mesh::Cube, v3(0, m(5) + dm(4), z), v3(m(5), dm(8), dm(8)), stone);
        self.halo(
            Mesh::Cube,
            boxed(v3(0, m(2) + dm(5), z), v3(m(3) + dm(4), m(5), dm(2))),
            self.c("ascent_door"),
            ONE / 2,
        );
        // a raised walk along the east, to stand on and duck under
        let hs = m(3) + dm(5);
        self.deck(
            m(22),
            -m(12),
            m(27),
            m(12),
            hs,
            &[(m(24) + dm(5), -m(6)), (m(24) + dm(5), m(6))],
        );
        let steps = (hs + dm(4) - 1) / dm(4);
        self.stairs(m(24) + dm(5), 0, m(12) + dm(6) * steps, 0, -ONE, hs, m(3));
        self.prayer_flags(-m(20), m(6), -m(10), m(20), m(6), -m(10));
        let _ = GATE_GAP;
    }

    // ---- pieces -------------------------------------------------------------

    /// A boulder: solid, cover.
    fn boulder(&mut self, x: i32, z: i32) {
        let s = self.metres(2, 4);
        self.solids
            .push(Solid::stand(x, 0, z, s * 3 / 4, s * 3 / 4, s * 3 / 4));
        let rot = Affine::translate(v3(x, s / 3, z)).then(&Affine::rot_y(deg(self.rng.range(0, 89))));
        self.shape(
            Mesh::Octa,
            rot.then(&Affine::scale(v3(s, s * 3 / 4, s))),
            self.c("ascent_rock"),
            0,
            true,
        );
    }

    /// A line of prayer flags from one point to another, sagging.
    fn prayer_flags(&mut self, x0: i32, y0: i32, z0: i32, x1: i32, y1: i32, z1: i32) {
        let a = v3(x0, y0, z0);
        let b = v3(x1, y1, z1);
        let mid = a.lerp(b, ONE / 2).add(v3(0, -m(1), 0));
        self.rod(a, mid, dm(1) / 3, self.c("ascent_rope"), false);
        self.rod(mid, b, dm(1) / 3, self.c("ascent_rope"), false);
        let colours = [
            self.c("cloth"),
            self.c("gold"),
            self.c("cyan"),
            self.c("ascent_moss"),
            self.c("clock"),
        ];
        for k in 0..12 {
            let t = ONE * (k + 1) / 13;
            let p = if t < ONE / 2 {
                a.lerp(mid, t * 2)
            } else {
                mid.lerp(b, t * 2 - ONE)
            };
            self.deco(
                Mesh::Cube,
                p.add(v3(0, -dm(3), 0)),
                v3(dm(5), dm(5), dm(1)),
                shade(colours[k as usize % 5], 100),
            );
        }
    }
}
