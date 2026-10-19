//! The Falls: the Calder drops a hundred and twenty metres down a cascade
//! of five stages, with the pool at the foot. It teaches the ground pound:
//! a rotten timber floor broken from a ring above it, the bounce off the
//! stage below carrying you on over the gap; a pulley gate whose lever you
//! hook as you fall past it; a drum-cap whose doubled bounce lifts you over
//! a rib of rock to a ring beyond; a stretch out in the spray where the only
//! standing place is a Bench you lay; and a last rotten floor over the pool.
//! Spray-eyes in the walls drop curtains of the river's orbs on all of it.
//! Every miss falls the whole way to the water.

use super::super::{boxed, dm, m, B};
use super::SURFACE;
use crate::course::Step;
use crate::fx::{int, ONE};
use crate::geom::v3;
use crate::level::Solid;
use crate::mesh::Mesh;

/// the stages' tops, top to bottom, and the ledge by the pool
const S1: i32 = int(120);
const S2: i32 = int(96);
const S3: i32 = int(72);
const S4: i32 = int(48);
const S5: i32 = int(24);
const FOOT: i32 = int(2);
/// the canyon's half width: every stage runs wall to wall
const W: i32 = int(10);
/// where the chains of the hung rings meet their beams, over each stage
const HANG: i32 = int(20);
/// the canyon's rim
const TOP: i32 = S1 + HANG + int(6);

impl B<'_> {
    pub(super) fn falls(&mut self) {
        let at = |y: i32, z: i32| v3(0, y, z);

        // 3a: the top stage ends against a wall of rock, its last stretch a
        // rotten timber floor over the second; a ring hung over it lifts
        // you off the boards, and the pound from there breaks them and drops
        // you to the stage below, which runs on under the wall
        self.mooring(0, S1, 0);
        self.rock_shelf(-W, m(6), W, -m(20), S1);
        self.rotten_floor(-W, -m(20), W, -m(32), S1);
        self.timber_joists(-m(20), -m(32), S1);
        self.hung_ring(S1 + m(4), -m(27), S1 + HANG);
        self.hanging_rock(v3(-W, S2 + m(5), -m(36)), v3(W, TOP, -m(32)));
        self.line(Step::To(at(S1, -m(17))));
        self.line(Step::Hook(at(S1 + m(4), -m(27))));
        self.line(Step::Pound);
        self.line(Step::Air(at(S2, -m(28))));
        self.line(Step::To(at(S2, -m(56))));

        // 3d: the pulley gate. The second stage ends over a timber gate
        // across the falls; a ring over it pulls you out above it, and the
        // counterweight's lever on the west wall, hooked as you fall past,
        // hauls it open for three seconds
        self.rock_shelf(-W, -m(20), W, -m(62), S2);
        self.mooring(0, S2, -m(50));
        let gate_y = S3 + m(8);
        let gate = self.sluice_door(v3(-W, gate_y - dm(5), -m(78)), v3(W, gate_y, -m(62)));
        let lever = v3(-m(8) - dm(5), gate_y + m(8), -m(74));
        self.hook_lever(lever.x, lever.y, lever.z, vec![gate], vec![], 3 * 60);
        self.rope_line(lever, v3(lever.x, S2 + HANG, lever.z));
        self.rope_line(v3(lever.x, S2 + HANG, lever.z), v3(0, gate_y, -m(70)));
        self.hung_ring(S2 + m(4), -m(69), S2 + HANG);
        self.line(Step::Hook(at(S2 + m(4), -m(69))));
        // the hook comes back to you before the gate is reached
        self.line(Step::Wait(60));
        self.line(Step::Tug(lever));
        self.line(Step::Air(at(S3, -m(74))));

        // 3b: the third stage, under the gate and a roof of rock walled to
        // the canyon's top, ends at a rib higher than a jump or a pound's
        // bounce; a drum-cap at its foot doubles the bounce of a pound from
        // the ring over it, and from the top of that bounce the ring over
        // the fourth stage shows above the rib
        self.rock_shelf(-W, -m(62), W, -m(99), S3);
        self.hanging_rock(v3(-W, S3 + m(6), -m(88)), v3(W, S3 + m(11), -m(78)));
        self.hanging_rock(v3(-W, S3 + m(6), -m(90)), v3(W, TOP, -m(88)));
        self.mooring(0, S3, -m(74));
        // broad, and reaching back toward where you hook from, so a pound
        // thrown early in the pull still lands on it
        self.drum_cap(0, S3 + m(1), -m(94), m(4));
        self.hung_ring(S3 + m(5), -m(97), S3 + HANG);
        self.rock_shelf(-W, -m(99), W, -m(101), S3 + m(5));
        self.line(Step::To(at(S3, -m(86))));
        self.line(Step::Hook(at(S3 + m(5), -m(97))));
        self.line(Step::Pound);
        // up the bounce, the hook back by its top
        self.line(Step::Wait(58));
        self.hung_ring(S4 + m(4), -m(122), S4 + HANG + m(4));
        self.line(Step::Hook(at(S4 + m(4), -m(122))));
        self.line(Step::Air(at(S4, -m(126))));

        // 3c: spray-eyes over the swing down and the stretch in the spray
        self.rock_emitter(W - dm(6), S3 + m(10), -m(110), "spray_eye");
        self.rock_emitter(-W + dm(6), S4 + m(10), -m(156), "spray_eye");
        self.rock_emitter(W - dm(6), S4 + m(8), -m(176), "spray_eye");

        // the fourth stage, and the strand: an alcove high in the east wall
        // and a ring to reach it by
        self.rock_shelf(-W, -m(116), W, -m(140), S4);
        self.mooring(0, S4, -m(126));
        self.hanging_rock(v3(m(5), S4 + m(7), -m(134)), v3(W, S4 + m(9), -m(126)));
        self.hook_ring(m(2), S4 + m(12), -m(130), Some(S4 + HANG + m(4)));
        self.strand_reward(m(8), S4 + m(9), -m(130));
        self.line(Step::To(at(S4, -m(137))));

        // 3e: the Bench in the spray. Out over the fourth fall nothing
        // stands but the rings, and a ring is only a pull: you lay a Bench
        // where the first one stops you, wait out the curtains on it, pull
        // to the second ring as it rots, lay another where that one stops
        // you, and Bound from it to the fifth stage
        self.hung_ring(S4 + m(4), -m(152), S4 + HANG + m(4));
        self.hung_ring(S4 + m(7), -m(172), S4 + HANG + m(4));
        self.line(Step::Hook(at(S4 + m(4), -m(152))));
        self.line(Step::Wait(20));
        self.line(Step::Bench);
        self.line(Step::Wait(165));
        self.line(Step::Hook(at(S4 + m(7), -m(172))));
        // the Bench's cooldown out, and down clear of the ring
        self.line(Step::Wait(42));
        self.line(Step::Bench);
        self.line(Step::Wait(100));
        self.line(Step::Bound(at(S5, -m(204))));

        // 3a again: the fifth stage ends in a rotten floor over the ledge by
        // the pool, against a wall; the way on is under the wall
        self.rock_shelf(-W, -m(194), W, -m(226), S5);
        self.mooring(0, S5, -m(202));
        self.rotten_floor(-W, -m(226), W, -m(240), S5);
        self.timber_joists(-m(226), -m(240), S5);
        self.hung_ring(S5 + m(4), -m(233), S5 + HANG);
        self.hanging_rock(v3(-W, FOOT + m(5), -m(244)), v3(W, TOP, -m(240)));
        self.rock_shelf(-W, -m(226), W, -m(290), FOOT);
        self.line(Step::To(at(S5, -m(223))));
        self.line(Step::Hook(at(S5 + m(4), -m(233))));
        self.line(Step::Pound);
        self.line(Step::Air(at(FOOT, -m(236))));
        self.line(Step::To(at(FOOT, -m(262))));
        self.way_on(0, FOOT, -m(266));

        // the falls themselves, the pool, the walls, and the stages' dressing
        for (z, top, bottom) in [
            (-m(36) - dm(5), S1, S2 + m(5)),
            (-m(62) - dm(5), S2, gate_y),
            (-m(101) - dm(5), S3 + m(5), SURFACE),
            (-m(140) - dm(5), S4, SURFACE),
            (-m(244) - dm(5), S5, FOOT + m(5)),
        ] {
            self.cascade(z, top, bottom);
        }
        self.river_water(-W, m(10), W, -m(300));
        self.canyon_walls(m(10), -m(300), W, TOP);
        for (z0, z1, top) in [
            (m(6), -m(20), S1),
            (-m(36), -m(62), S2),
            (-m(62), -m(78), S3),
            (-m(116), -m(140), S4),
            (-m(194), -m(226), S5),
            (-m(240), -m(290), FOOT),
        ] {
            self.river_dress(-W + m(1), z0, W - m(1), z1, top, 8);
        }
    }

    /// A ring hung on a chain from a timber beam across the canyon.
    fn hung_ring(&mut self, y: i32, z: i32, top: i32) {
        self.hook_ring(0, y, z, Some(top));
        let wood = self.c("river_wood_dark");
        self.deco(Mesh::Cube, v3(0, top, z), v3(W * 2, dm(8), dm(8)), wood);
    }

    /// Rock that hangs from the walls (a roof, a ledge in the wall), not
    /// standing on the bed as a shelf does.
    fn hanging_rock(&mut self, min: crate::geom::V3, max: crate::geom::V3) {
        let s = Solid::new(min, max);
        self.solids.push(s);
        let body = self.c("sandstone_dark");
        let lip = self.c("sandstone_pale");
        // the body stops inside the lip: two tops in one plane flicker
        let under = v3(max.x, max.y - dm(2), max.z);
        self.deco(Mesh::Cube, min.add(under).scale(ONE / 2), under.sub(min), body);
        self.deco(
            Mesh::Cube,
            v3(s.centre().x, max.y - dm(2), s.centre().z),
            v3(max.x - min.x + dm(2), dm(4), max.z - min.z + dm(2)),
            lip,
        );
    }

    /// The beams a rotten floor was laid on, which stay when it breaks.
    fn timber_joists(&mut self, z0: i32, z1: i32, top: i32) {
        let wood = self.c("river_wood_dark");
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        let mut z = z0 + m(1);
        while z < z1 {
            self.deco(Mesh::Cube, v3(0, top - dm(9), z), v3(W * 2, dm(4), dm(4)), wood);
            z += m(4);
        }
    }

    /// A fall of water over a lip at `z`, from `top` to `bottom`: tall pale
    /// strips across the canyon with gaps between, drawn only.
    fn cascade(&mut self, z: i32, top: i32, bottom: i32) {
        let blue = self.c("river_water");
        let foam = self.c("river_foam");
        let h = top - bottom;
        // off the tenth-metre grid the canyon's strata stand on, so no
        // strip's end lies in a stratum's face
        let mut x = -W + dm(5) + ONE / 20;
        let mut k = 0;
        while x < W - m(1) {
            let w = self.metres(1, 3);
            let rgb = if k % 2 == 0 { foam } else { blue };
            let back = dm(self.rng.range(0, 6));
            self.shape(
                Mesh::Cube,
                boxed(v3(x + w / 2, bottom + h / 2, z - back), v3(w, h, dm(4))),
                rgb,
                ONE / 4,
                false,
            );
            x += w + self.metres(0, 2);
            k += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_falls_can_be_crossed_on_their_line_with_the_rivers_kit() {
        let (ticks, _lost) = crate::course::tests::cross("falls").unwrap();
        assert!(ticks < 60 * 150, "{ticks}");
    }
}
