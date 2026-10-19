//! The Shallows: Lake Stillwater's flooded wood, where the Calder ends.
//! Drowned trees stand in open water and the Gallows Oak's roots run out
//! under it, rising and sinking with the tree's breath. Everything the river
//! taught, at once: roots you cross while they are up, a lever that opens
//! the door through a hollow trunk with a lamp inside that puts out the
//! lattice beyond, sixty metres of open water crossed on the Bench and the
//! hook, and a last run over rising roots under the root-eyes' fire, the
//! Oak growing ahead.

use super::super::{dm, m, B};
use super::BED;
use crate::course::Step;
use crate::fx::{self, deg, ONE};
use crate::geom::v3;
use crate::level::Solid;
use crate::mesh::Mesh;

/// the ground's height over the lake
const TOP: i32 = ONE;
/// the Oak's breath, in ticks: every root rises and sinks once in it
const BREATH: i32 = 6 * 60;
/// a root at its lowest stands this far under the surface, and rises this far
const ROOT_LOW: i32 = -ONE;
const ROOT_RISE: i32 = 3 * ONE;

impl B<'_> {
    pub(super) fn shallows(&mut self) {
        let t = TOP;
        let p = |z: i32| v3(0, t, z);

        // 4a: the first roots, out from the shore, each a breath behind the
        // last so the one ahead is rising as you reach it
        self.mooring(0, t, m(2));
        self.root_bank(-m(3), m(6), m(3), -m(14), t);
        self.line(Step::To(p(-m(9))));
        self.line(Step::Wait(100));
        let end = self.root_run(-m(14), 4, -87);
        self.line(Step::Jump(p(end - m(5))));

        // 4b: a lever on a drowned stake opens the door into the hollow
        // trunk for two and a half seconds; the lamp inside, shot out, puts
        // the root-eye's lattice beyond it out for two
        self.root_bank(-m(4), end - dm(25), m(4), -m(150), t);
        self.mooring(0, t, -m(62));
        self.drowned_tree(-m(9), -m(103), m(9), dm(6));
        let door = self.sluice_door(v3(-m(2), t, -m(112)), v3(m(2), t + m(4), -m(111)));
        self.hook_lever(-m(7), t + m(4), -m(103), vec![door], vec![], 150);
        self.rope_line(v3(-m(9), t + m(5), -m(103)), v3(-m(7), t + m(5), -m(103)));
        self.hollow_trunk(-m(112), -m(128), t);
        let eye = self.rock_emitter(-m(6), t + dm(15), -m(136), "root_eye");
        self.lamp_switch(dm(15), t + m(3), -m(120), "weir_lamp", vec![], vec![eye], 120);
        let brass = self.c("brass");
        self.deco(
            Mesh::Cube,
            v3(dm(19), t + m(3), -m(120)),
            v3(dm(2), dm(8), dm(8)),
            brass,
        );
        self.stump(-m(6), -m(136), t);
        self.line(Step::To(p(-m(100))));
        self.line(Step::Tug(v3(-m(7), t + m(4), -m(103))));
        self.line(Step::To(p(-m(134))));
        self.mooring(0, t, -m(142));

        // the strand: a broken bough high on a drowned tree east of the
        // post, a ring hung over it to reach it by
        self.drowned_tree(m(12), -m(140), m(16), dm(8));
        self.bark_ledge(m(7), -m(137), m(11), -m(143), m(7));
        self.hook_ring(m(5), m(13), -m(140), Some(m(16)));
        let wood = self.c("river_wood_dark");
        self.rod(
            v3(m(12), m(15), -m(140)),
            v3(m(5), m(16), -m(140)),
            dm(2),
            wood,
            true,
        );
        self.strand_reward(m(9), m(7), -m(140));

        // 4c: sixty metres of open water to the far roots. A Bench laid
        // out over the water, Bound Over onto it, hook the ring on the
        // drowned tree's branch, a second Bench in the air, Bound Over
        self.line(Step::To(v3(0, t, -m(149) - dm(4))));
        self.line(Step::Bench);
        self.line(Step::To(p(-m(146) - dm(4))));
        self.line(Step::Bound(p(-m(157))));
        self.line(Step::To(p(-m(161))));
        self.line(Step::Wait(100));
        let branch = v3(0, t + m(7), -m(190));
        self.drowned_tree(-m(8), branch.z, m(18), dm(9));
        self.hook_ring(branch.x, branch.y, branch.z, Some(m(12)));
        self.rope_line(v3(-m(8), m(12), branch.z), v3(0, m(12), branch.z));
        self.line(Step::Hook(branch));
        self.line(Step::Wait(30));
        self.line(Step::Bench);
        self.line(Step::Wait(3));
        self.line(Step::Bound(p(-m(216))));
        self.line(Step::To(p(-m(218))));

        // 4d: the last run: roots rising in the Oak's breath, root-eyes on
        // the stumps either side, a Bound over the last of the open water
        self.root_bank(-m(4), -m(210), m(4), -m(236), t);
        self.mooring(0, t, -m(218));
        self.line(Step::To(p(-m(231))));
        self.line(Step::Wait(60));
        let end = self.root_run(-m(236), 6, -120);
        self.line(Step::Jump(p(end - m(5))));
        for (x, z) in [
            (m(9), -m(262)),
            (-m(9), -m(282)),
            (m(9), -m(312)),
            (-m(9), -m(345)),
        ] {
            self.stump(x, z, t);
            self.rock_emitter(x, t + dm(15), z, "root_eye");
        }
        self.root_bank(-m(4), end - dm(25), m(4), -m(318), t);
        self.root_bank(-m(5), -m(329), m(5), -m(392), t);
        self.line(Step::To(v3(0, t, -m(317) - dm(4))));
        self.line(Step::Bound(p(-m(334))));
        self.line(Step::To(p(-m(384))));
        self.way_on(0, t, -m(386));

        // the lake, the drowned wood, and the Oak ahead
        self.river_water(-m(70), m(12), m(70), -m(400));
        for (x, z, h) in [
            (-m(14), -m(10), m(14)),
            (m(16), -m(30), m(20)),
            (-m(20), -m(52), m(12)),
            (m(13), -m(78), m(17)),
            (-m(22), -m(92), m(22)),
            (m(26), -m(118), m(15)),
            (-m(28), -m(150), m(19)),
            (m(22), -m(172), m(24)),
            (-m(24), -m(204), m(16)),
            (m(18), -m(226), m(13)),
            (-m(18), -m(250), m(21)),
            (m(24), -m(286), m(18)),
            (-m(26), -m(322), m(15)),
            (m(20), -m(360), m(22)),
            (-m(16), -m(378), m(12)),
        ] {
            self.drowned_tree(x, z, h, dm(5) + h / 40);
        }
        for (x0, z0, x1, z1) in [
            (-m(3), m(6), m(3), -m(14)),
            (-m(4), -m(56), m(4), -m(110)),
            (-m(4), -m(210), m(4), -m(236)),
            (-m(5), -m(329), m(5), -m(392)),
        ] {
            self.river_dress(x0, z0, x1, z1, t, 6);
        }
        self.oak_far(v3(0, 0, -m(700)));
    }

    /// Ground of old root and silt from under the water to `top`.
    fn root_bank(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, top: i32) {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        self.solids.push(Solid::new(v3(x0, BED, z0), v3(x1, top, z1)));
        let root = self.c("root");
        let moss = self.c("moss");
        // the body stops inside the moss: two tops in one plane flicker
        let h = top - dm(1) - BED;
        self.deco(
            Mesh::Cube,
            v3((x0 + x1) / 2, BED + h / 2, (z0 + z1) / 2),
            v3(x1 - x0, h, z1 - z0),
            root,
        );
        self.deco(
            Mesh::Cube,
            v3((x0 + x1) / 2, top - dm(1), (z0 + z1) / 2),
            v3(x1 - x0 + dm(2), dm(2), z1 - z0 + dm(2)),
            moss,
        );
    }

    /// A run of `n` roots out from a bank's edge at `edge`, each a fifth
    /// of a breath behind the one before, so the next is rising as you
    /// reach it; `first` is the first one's phase, in degrees. Lays the
    /// line across them; where the last ends.
    fn root_run(&mut self, edge: i32, n: i32, first: i32) -> i32 {
        let t = TOP;
        let mut z = edge;
        self.line(Step::To(v3(0, t, edge + dm(6))));
        for k in 0..n {
            let (z0, z1) = (z - dm(25), z - dm(95));
            self.root(z0, z1, deg(first - 60 * k));
            self.line(Step::Jump(v3(0, t, (z0 + z1) / 2)));
            self.line(Step::To(v3(0, t, z1 + dm(6))));
            z = z1;
        }
        z
    }

    /// A root across the way from z0 to z1 that breathes with the Oak: at
    /// its lowest a metre under the water, at its highest two over it.
    /// `phase` is where in its breath it starts.
    fn root(&mut self, z0: i32, z1: i32, phase: i32) {
        let half = dm(25);
        self.breathing(-half, z0, half, z1, ROOT_LOW, ROOT_RISE, BREATH, phase);
        // the root's arch under the water, either side, drawn only
        let root = self.c("root");
        for x in [-half - dm(5), half + dm(5)] {
            self.deco(
                Mesh::Cyl,
                v3(x, (BED + ROOT_LOW) / 2, (z0 + z1) / 2),
                v3(dm(8), ROOT_LOW - BED, dm(8)),
                root,
            );
        }
    }

    /// A drowned tree: a bare trunk `r` across from the lake bed to `high`,
    /// its dead branches reaching out. Solid, so the hook can strike it.
    fn drowned_tree(&mut self, x: i32, z: i32, high: i32, r: i32) {
        self.solids
            .push(Solid::new(v3(x - r, BED, z - r), v3(x + r, high, z + r)));
        let bark = self.c("oak_bark");
        let wood = self.c("river_wood_dark");
        self.deco(
            Mesh::Cyl,
            v3(x, (BED + high) / 2, z),
            v3(r * 2, high - BED, r * 2),
            bark,
        );
        let n = self.rng.range(2, 4);
        for k in 0..n {
            let a = deg(self.rng.range(0, 359)) + fx::TURN / 4 * k;
            let y = high - self.metres(1, 6).min(high - m(3));
            let len = self.metres(3, 7);
            let tip = v3(
                x + fx::mul(fx::cos(a), len),
                y + self.metres(1, 3),
                z + fx::mul(fx::sin(a), len),
            );
            self.rod(v3(x, y, z), tip, dm(2), wood, true);
        }
    }

    /// A drowned stump standing just out of the water at `top`.
    fn stump(&mut self, x: i32, z: i32, top: i32) {
        let bark = self.c("oak_bark");
        self.solids.push(Solid::new(
            v3(x - m(1), BED, z - m(1)),
            v3(x + m(1), top, z + m(1)),
        ));
        self.deco(
            Mesh::Cyl,
            v3(x, (BED + top) / 2, z),
            v3(m(2), top - BED, m(2)),
            bark,
        );
    }

    /// A broken bough to stand on, its top at `top`.
    fn bark_ledge(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, top: i32) {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        self.solids
            .push(Solid::new(v3(x0, top - m(1), z0), v3(x1, top, z1)));
        let bark = self.c("oak_bark");
        self.deco(
            Mesh::Cube,
            v3((x0 + x1) / 2, top - dm(5), (z0 + z1) / 2),
            v3(x1 - x0, m(1), z1 - z0),
            bark,
        );
    }

    /// The hollow foot of a great drowned trunk across the way, from z0 to
    /// z1: a passage four metres wide and high through it, the trunk too
    /// broad and too tall to go round or over.
    fn hollow_trunk(&mut self, z0: i32, z1: i32, top: i32) {
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        let w = m(2);
        let high = m(40);
        for (x0, x1) in [(-m(16), -w), (w, m(16))] {
            self.solids.push(Solid::new(v3(x0, BED, z0), v3(x1, high, z1)));
        }
        self.solids
            .push(Solid::new(v3(-w, top + m(4), z0), v3(w, high, z1)));
        let bark = self.c("oak_bark");
        let wood = self.c("river_wood_dark");
        let zc = (z0 + z1) / 2;
        for (x, sx) in [(-m(9), m(14)), (m(9), m(14))] {
            self.deco(
                Mesh::Cube,
                v3(x, (BED + high) / 2, zc),
                v3(sx, high - BED, z1 - z0),
                bark,
            );
        }
        self.deco(
            Mesh::Cube,
            v3(0, (top + m(4) + high) / 2, zc),
            v3(w * 2, high - top - m(4), z1 - z0),
            bark,
        );
        self.deco(
            Mesh::Cyl,
            v3(0, high + m(2), zc),
            v3(m(30), m(4), z1 - z0 + m(4)),
            bark,
        );
        // its broken limbs, high over the lake
        for k in 0..5 {
            let a = fx::TURN / 5 * k + deg(20);
            let tip = v3(
                fx::mul(fx::cos(a), m(18)),
                high + m(8),
                zc + fx::mul(fx::sin(a), m(18)),
            );
            self.rod(v3(0, high, zc), tip, dm(6), wood, true);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_shallows_can_be_crossed_on_their_line_with_the_rivers_kit() {
        let (ticks, _lost) = crate::course::tests::cross("shallows").unwrap();
        assert!(ticks < 60 * 150, "{ticks}");
    }
}
