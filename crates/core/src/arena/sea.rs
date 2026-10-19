//! The Far Shore (inside the anomaly): an ocean under a strange sky, and
//! the Drowned Fleet on it. Each section is a course, fought: its way on
//! opens when its crews and waves are down. The water drowns, as the
//! river's does.
//!
//! - the Fleet (`fleet`): seven ships at anchor; their crews on them
//! - the Hurricane (`hurricane`): the fleet in line through the storm, the
//!   decks heaving and swaying, gusts and lightning
//! - the Lee (`rest`): a capsized hull's dry pocket
//! - the Maelstrom (`maelstrom`): under the water, the wreckage turning down
//!   and in toward the eye
//!
//! and the Eye (the boss): a ring of wreckage turning round the whirlpool,
//! Dagon rising in the middle.

use super::{dm, m, B};
use crate::course::{Chain, Course, Flag, Gust, Look, Mover, Storm, Underwater};
use crate::fx::{self, ONE};
use crate::geom::{v3, V3};
use crate::level::{Dyn, Solid};
use crate::mesh::Mesh;

mod eye;
mod fleet;
mod hurricane;
mod maelstrom;

#[allow(clippy::too_many_arguments)]
impl B<'_> {
    pub(super) fn sea(&mut self, kind: &str) {
        self.course = Some(Course {
            water: -ONE / 2,
            ..Course::default()
        });
        self.start_yaw = 0;
        match kind {
            "hurricane" => self.hurricane(),
            "maelstrom" => self.maelstrom(),
            "rest" => self.the_lee(),
            "boss" => self.eye(),
            _ => self.fleet(),
        }
    }

    /// The ocean's surface over x0..x1 by z0..z1.
    pub(super) fn sea_surface(&mut self, x0: i32, z0: i32, x1: i32, z1: i32) {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        let sea = self.c("ocean");
        let deep = self.c("ocean_deep");
        let foam = self.c("river_foam");
        let c = v3((x0 + x1) / 2, -dm(1), (z0 + z1) / 2);
        self.shape(
            Mesh::Ground,
            super::boxed(c, v3(x1 - x0, ONE, z1 - z0)),
            sea,
            0,
            false,
        );
        self.shape(
            Mesh::Ground,
            super::boxed(c.add(v3(0, -m(4), 0)), v3(x1 - x0, ONE, z1 - z0)),
            deep,
            0,
            false,
        );
        for _ in 0..60 {
            let x = x0 + self.metres(0, ((x1 - x0) / ONE).max(1));
            let z = z0 + self.metres(0, ((z1 - z0) / ONE).max(1));
            let len = self.metres(2, 7);
            self.shape(
                Mesh::Panel,
                super::boxed(v3(x, dm(1) / 2, z), v3(len, ONE, dm(3))),
                foam,
                ONE / 3,
                false,
            );
        }
    }

    /// A ship's deck that heaves `rise` and sways `swing` on the swell every
    /// `period` ticks: a moving solid, drawn each tick by `draw.rs`, with
    /// its hull, masts and furniture drawn on it there too. Its index among
    /// the course's movers.
    pub(super) fn ship_deck(
        &mut self,
        min: V3,
        max: V3,
        rise: i32,
        swing: V3,
        period: i32,
        phase: i32,
    ) -> usize {
        let base = Solid::new(min, max);
        self.dynamic.push(Dyn {
            solid: base,
            on: true,
            one_way: false,
        });
        let dynamic = self.dynamic.len() - 1;
        let k = self.k();
        let mut mv = Mover::breathing(dynamic, base, rise, period, phase);
        mv.swing = swing;
        mv.look = Look::Deck;
        k.movers.push(mv);
        k.movers.len() - 1
    }

    /// A ship: a deck from x0..x1 by z0..z1 with its top at `top`, and a
    /// mast at each of `masts` (x, z, height over the deck), all heaving and
    /// swaying together. The masts are solid, so the hook catches them. The
    /// deck's index among the movers.
    pub(super) fn ship(
        &mut self,
        x0: i32,
        z0: i32,
        x1: i32,
        z1: i32,
        top: i32,
        masts: &[(i32, i32, i32)],
        rise: i32,
        swing: V3,
        period: i32,
        phase: i32,
    ) -> usize {
        let (x0, x1) = (x0.min(x1), x0.max(x1));
        let (z0, z1) = (z0.min(z1), z0.max(z1));
        let deck = self.ship_deck(
            v3(x0, top - m(2), z0),
            v3(x1, top, z1),
            rise,
            swing,
            period,
            phase,
        );
        for &(x, z, h) in masts {
            let i = self.ship_deck(
                v3(x - dm(3), top, z - dm(3)),
                v3(x + dm(3), top + h, z + dm(3)),
                rise,
                swing,
                period,
                phase,
            );
            self.k().movers[i].look = Look::Mast;
        }
        deck
    }

    /// Wreckage turning round `round` at `turn` a tick (the Maelstrom, the
    /// Eye): a moving solid from `min` to `max`. Its index among the movers.
    pub(super) fn wreckage(
        &mut self,
        min: V3,
        max: V3,
        round: V3,
        turn: i32,
        bob: i32,
        period: i32,
        phase: i32,
    ) -> usize {
        let base = Solid::new(min, max);
        self.dynamic.push(Dyn {
            solid: base,
            on: true,
            one_way: false,
        });
        let dynamic = self.dynamic.len() - 1;
        let k = self.k();
        let mut mv = Mover::breathing(dynamic, base, bob, period, phase);
        mv.orbit = turn;
        mv.round = round;
        mv.look = Look::Wreck;
        k.movers.push(mv);
        k.movers.len() - 1
    }

    /// A crew member, on board from the start, waking as you come near.
    /// Each crewed ship's colours on her tallest mast, her crew those
    /// standing over her deck (a lookout in her rigging, the dragon on her
    /// mast). Called once the fleet and its crews are laid.
    pub(super) fn fly_colours(&mut self) {
        let k = self.k();
        let over = |s: &Solid, p: V3| {
            p.x >= s.min.x - ONE && p.x <= s.max.x + ONE && p.z >= s.min.z - ONE && p.z <= s.max.z + ONE
        };
        let mut flags = Vec::new();
        for (deck, d) in k.movers.iter().enumerate().filter(|(_, d)| d.look == Look::Deck) {
            let mast = k
                .movers
                .iter()
                .enumerate()
                .filter(|(_, m)| m.look == Look::Mast && over(&d.base, m.base.centre()))
                .max_by_key(|(_, m)| m.base.max.y)
                .map(|(i, _)| i);
            let crew: Vec<usize> = (0..k.emitters.len())
                .filter(|&i| over(&d.base, k.emitters[i].pos))
                .collect();
            if let (Some(mast), false) = (mast, crew.is_empty()) {
                flags.push(Flag {
                    deck,
                    mast,
                    crew,
                    struck: false,
                });
            }
        }
        k.flags = flags;
    }

    /// The Captured Dragon chained to the mainmast of the ship it sits
    /// over: three locks round the mast's foot, its perches her mast tops.
    pub(super) fn chain_dragon(&mut self, dragon: &str, lock: &str) {
        let k = self.k();
        let Some(d) = k.emitters.iter().position(|e| e.enemy == dragon) else {
            return;
        };
        let Some(f) = k.flags.iter().position(|f| f.crew.contains(&d)) else {
            return;
        };
        let (deck, main) = (k.flags[f].deck, k.flags[f].mast);
        let foot = {
            let b = &k.movers[main].base;
            v3(b.centre().x, b.min.y, b.centre().z)
        };
        let perches: Vec<V3> = k
            .movers
            .iter()
            .filter(|m| {
                m.look == Look::Mast
                    && k.movers[deck].base.min.x <= m.base.centre().x
                    && m.base.centre().x <= k.movers[deck].base.max.x
                    && k.movers[deck].base.min.z <= m.base.centre().z
                    && m.base.centre().z <= k.movers[deck].base.max.z
            })
            .map(|m| v3(m.base.centre().x, m.base.max.y + ONE, m.base.centre().z))
            .collect();
        let mut locks = Vec::new();
        for a in 0..3 {
            let at = foot.add(crate::geom::facing(fx::TURN / 3 * a, 0).scale(ONE * 3 / 2));
            locks.push(self.k().emitters.len());
            self.crew_member(v3(at.x, foot.y, at.z), lock);
        }
        let k = self.k();
        k.flags[f].crew.extend(locks.iter().copied());
        k.chain = Some(Chain {
            dragon: d,
            locks,
            mast: foot,
            perches,
            free: false,
        });
    }

    pub(super) fn crew_member(&mut self, at: V3, enemy: &str) {
        self.rock_emitter(at.x, at.y, at.z, enemy);
    }

    /// A gust over the box min..max, shoving along `dir`.
    pub(super) fn wind_gust(
        &mut self,
        min: V3,
        max: V3,
        dir: V3,
        push: i32,
        period: i32,
        on: i32,
        phase: i32,
    ) {
        self.k().gusts.push(Gust {
            zone: Solid::new(min, max),
            dir: dir.norm(),
            push,
            period,
            on,
            phase,
        });
    }

    pub(super) fn thunder_storm(&mut self, every: i32, warn: i32, radius: i32, damage: i32) {
        self.k().storm = Some(Storm {
            every,
            warn,
            radius,
            damage,
        });
    }

    pub(super) fn under_water(&mut self, eye: V3, pull: i32, gravity_pct: i32) {
        self.k().underwater = Some(Underwater {
            gravity_pct,
            eye,
            pull,
        });
    }

    /// The Lee: a capsized hull's dry pocket, its keel overhead; the rest's
    /// pickups on its boards.
    pub(super) fn the_lee(&mut self) {
        let t = 0;
        self.mooring(0, t, m(10));
        self.board_walk(-m(12), m(14), m(12), -m(16), t);
        let hull = self.c("hull_dark");
        self.deco(Mesh::Blob, v3(0, m(12), 0), v3(m(30), m(8), m(36)), hull);
        self.way_on(0, t, -m(14));
        self.line(crate::course::Step::To(v3(0, t, -m(12))));
        self.sea_surface(-m(80), m(60), m(80), -m(80));
        let _ = fx::ONE;
    }
}
