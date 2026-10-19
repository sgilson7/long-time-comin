//! The Eye: Dagon's arena. A ring of wreckage round the whirlpool's eye,
//! two metres over the water: four great hulls fixed at the ring's
//! quarters, each with its mooring post, and between them rafts of
//! wreck riding the swell; inside the ring a band of smaller pieces turning
//! slowly round the eye; in the middle the funnel, black and whirling,
//! where Dagon comes up. Its tentacles rise on the hulls' decks.
//!
//! Nothing leads on from here: when Dagon falls the whirlpool drains, and
//! you sink.

use super::super::{boxed, dm, m, B};
use crate::course::Step;
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;

/// the eye, the whirlpool's middle
const EYE: V3 = v3(0, 0, -30 * ONE);
/// the ring's decks stand this far over the water
const FLOOR: i32 = 2 * ONE;
/// the ring: this far out from the eye
const RING: i32 = 30 * ONE;
/// a great hull's deck, square; a raft's
const HULL: i32 = 16 * ONE;
const RAFT: i32 = 8 * ONE;
/// rafts between each pair of hulls
const RAFTS: i32 = 3;
/// a raft rises this far on the swell every this many ticks
const SWELL: i32 = ONE * 6 / 10;
const SWELL_TICKS: i32 = 5 * 60;
/// the band inside the ring: this far out, this many pieces, each this
/// wide, once round in this many ticks
const BAND: i32 = 16 * ONE;
const BAND_PIECES: i32 = 8;
const PIECE: i32 = 7 * ONE;
const BAND_TICKS: i32 = 80 * 60;

/// A point `a` degrees round the eye from due south, `r` out, at `y`.
fn about(a: i32, r: i32, y: i32) -> V3 {
    v3(EYE.x + fx::mul(fx::sin(a), r), y, EYE.z + fx::mul(fx::cos(a), r))
}

/// How far a square deck `size` across reaches from its middle along the
/// flat direction `u`.
fn reach(size: i32, u: V3) -> i32 {
    let most = u.x.abs().max(u.z.abs()).max(ONE / 2);
    fx::div(size / 2, most)
}

impl B<'_> {
    pub(super) fn eye(&mut self) {
        // the four hulls, the south one first: you come in on it
        let quarter = fx::QUARTER;
        let hulls: Vec<V3> = (0..4).map(|k| about(quarter * k, RING, FLOOR)).collect();
        let masts: [&[(i32, i32, i32)]; 4] = [
            &[(m(5), m(5), m(9))],
            &[(m(5), -m(5), m(11)), (-m(4), m(5), m(7))],
            &[(-m(5), -m(5), m(10))],
            &[(-m(5), m(5), m(8))],
        ];
        for (k, &at) in hulls.iter().enumerate() {
            self.hulk(at, HULL, HULL, masts[k]);
            self.mooring(at.x, at.y, at.z);
            // where its tentacles rise: the deck's four quarters
            let q = HULL / 4;
            for (dx, dz) in [(q, q), (-q, q), (q, -q), (-q, -q)] {
                self.k().spawns.push(v3(at.x + dx, at.y, at.z + dz));
            }
        }

        // the rafts between, riding the swell; the line goes round on them,
        // hull to raft to raft to hull, and back to where it began
        let step = quarter / (RAFTS + 1);
        let mut from = hulls[0];
        let mut from_size = HULL;
        for k in 0..4 * (RAFTS + 1) {
            let a = step * (k + 1);
            let (to, to_size) = if (k + 1) % (RAFTS + 1) == 0 {
                (hulls[(((k + 1) / (RAFTS + 1)) % 4) as usize], HULL)
            } else {
                let at = about(a, RING, FLOOR);
                let phase = deg(self.rng.range(0, 359));
                self.wreckage(
                    v3(at.x - RAFT / 2, FLOOR - SWELL / 2 - m(1), at.z - RAFT / 2),
                    v3(at.x + RAFT / 2, FLOOR - SWELL / 2, at.z + RAFT / 2),
                    EYE,
                    0,
                    SWELL,
                    SWELL_TICKS,
                    phase,
                );
                (at, RAFT)
            };
            let u = v3(to.x - from.x, 0, to.z - from.z).norm();
            let lip = from.add(u.scale(reach(from_size, u) - dm(6)));
            self.line(Step::To(lip));
            self.line(Step::Jump(to));
            self.line(Step::To(to));
            from = to;
            from_size = to_size;
        }

        // the band inside the ring, turning
        let turn = fx::TURN / BAND_TICKS;
        for k in 0..BAND_PIECES {
            let at = about(fx::TURN / BAND_PIECES * k + fx::TURN / 16, BAND, FLOOR);
            let phase = deg(self.rng.range(0, 359));
            self.wreckage(
                v3(at.x - PIECE / 2, FLOOR - SWELL / 2 - m(1), at.z - PIECE / 2),
                v3(at.x + PIECE / 2, FLOOR - SWELL / 2, at.z + PIECE / 2),
                EYE,
                turn,
                SWELL,
                SWELL_TICKS,
                phase,
            );
        }

        // Dagon comes up in the eye; nothing leads on
        self.k().boss_at = Some(EYE.add(v3(0, m(1), 0)));
        self.k().finish = v3(0, -m(60), 0);
        self.eye_scene();
    }

    /// The sea round the ring, open over the eye; the funnel whirling down
    /// into the dark, foam spiralling in, the lantern-light of the deep.
    fn eye_scene(&mut self) {
        let hole = m(13);
        let far = m(110);
        // the sea in four strips round the hole over the eye
        self.sea_surface(EYE.x - far, EYE.z + hole, EYE.x + far, EYE.z + far);
        self.sea_surface(EYE.x - far, EYE.z - far, EYE.x + far, EYE.z - hole);
        self.sea_surface(EYE.x - far, EYE.z - hole, EYE.x - hole, EYE.z + hole);
        self.sea_surface(EYE.x + hole, EYE.z - hole, EYE.x + far, EYE.z + hole);
        let sea = self.c("ocean");
        let deep = self.c("ocean_deep");
        let navy = self.c("pirate_navy");
        let gloom = self.c("storm_cloud");
        let foam = self.c("river_foam");
        let teal = self.c("dagon_teal");
        let light = self.c("biolume");
        // the funnel: rings of water narrowing down, turned each a little
        // further, round a black throat
        for k in 0..14 {
            let r = hole - dm(8) * k;
            let y = -dm(15) - m(1) * k;
            let rgb = match k % 3 {
                0 => sea,
                1 => deep,
                _ => navy,
            };
            self.shape(
                Mesh::Ring,
                Affine::translate(v3(EYE.x, y, EYE.z))
                    .then(&Affine::rot_y(deg(23 * k)))
                    .then(&Affine::scale(v3(r * 2, m(1), r * 2))),
                rgb,
                0,
                false,
            );
        }
        self.shape(
            Mesh::Cone,
            Affine::translate(v3(EYE.x, -m(9), EYE.z))
                .then(&Affine::rot_x(fx::HALF))
                .then(&Affine::scale(v3(m(14), m(16), m(14)))),
            gloom,
            0,
            true,
        );
        // foam in spirals, three arms winding in to the throat
        for arm in 0..3 {
            for k in 0..22 {
                let a = deg(120 * arm + 19 * k);
                let r = m(40) - m(14) * k / 11;
                let y = if r > hole { dm(1) } else { -dm(4) * (k - 11) };
                let at = about(a, r, y);
                self.shape(
                    Mesh::Panel,
                    Affine::translate(at)
                        .then(&Affine::rot_y(a + fx::QUARTER))
                        .then(&Affine::scale(v3(m(4), ONE, dm(4)))),
                    foam,
                    ONE / 3,
                    false,
                );
            }
        }
        // the deep's lights down the throat, and the teal of what waits
        for k in 0..10 {
            let a = deg(37 * k);
            let at = about(a, m(3) + dm(5) * k, -m(2) - m(1) * k);
            self.halo(Mesh::Sphere, boxed(at, v3(dm(6), dm(6), dm(6))), light, ONE / 2);
        }
        self.deco(Mesh::Blob, v3(EYE.x, -m(15), EYE.z), v3(m(10), m(4), m(10)), teal);
        // wreckage drifting in the sea outside the ring, drawn only
        let dark = self.c("hull_dark");
        let hull = self.c("hull");
        for _ in 0..30 {
            let a = deg(self.rng.range(0, 359));
            let r = self.metres(42, 100);
            let at = about(a, r, 0);
            if self.rng.chance(50) {
                let l = self.metres(4, 12);
                self.deco(Mesh::Blob, at, v3(l / 3, dm(8), l), dark);
            } else {
                let s = self.metres(1, 2);
                self.deco(Mesh::Cube, at, v3(s, s, s), hull);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;

    #[test]
    fn the_eye_raises_dagon_in_the_middle_and_its_tentacles_and_your_start_stand_on_the_wrecks() {
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
            assert!(p.sub(EYE).len_xz() > m(20), "{p:?} out on the ring");
        }
        assert_eq!(k.posts.len(), 4, "a post on each great hull");
    }

    #[test]
    fn the_ring_can_be_walked_round_post_to_post_and_back_to_the_start() {
        use crate::course::Pilot;
        let c = content();
        let zone = c.zones.iter().position(|z| z.id == "far_shore").unwrap();
        let r = crate::route::Route::build_for(&c, zone, 7, &crate::route::Have::level(1));
        let node = r
            .nodes
            .iter()
            .find(|n| n.kind == crate::route::NodeKind::Boss)
            .unwrap()
            .clone();
        let p = crate::player::Player::new(V3::ZERO, 0, crate::course::tests::river_kit());
        let mut w = crate::world::World::new(&c, &node, &[], p, crate::weapons::Held::new(0, 1), 3);
        // the ring alone: Dagon kept down
        w.waves_left.clear();
        let line = w.course.as_ref().unwrap().line.clone();
        let start = w.player.pos;
        let mut pilot = Pilot::default();
        for tick in 0..60 * 120 {
            let Some(d) = pilot.drive(&line, &w.player) else {
                break;
            };
            w.player.yaw = d.yaw;
            let eye = w.player.chest();
            w.tick(&c, &d.controls, eye, d.aim);
            assert!(
                !w.notes.iter().any(|n| n.sound == "drown"),
                "fell in on step {} ({:?}) at tick {tick}",
                pilot.step,
                line[pilot.step.min(line.len() - 1)]
            );
            assert!(
                pilot.t < 60 * 10,
                "stuck on step {} ({:?}) at {:?}",
                pilot.step,
                line[pilot.step],
                w.player.pos
            );
        }
        assert_eq!(pilot.step, line.len(), "the line walked to its end");
        assert_eq!(w.course.as_ref().unwrap().post, 3, "every post reached");
        assert!(
            w.player.pos.sub(start).len_xz() < m(2),
            "back at the start: {:?}",
            w.player.pos
        );
    }
}
