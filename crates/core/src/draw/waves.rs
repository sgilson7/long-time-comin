//! The sea's waves over a course (the Hurricane): a rogue wave, a towering
//! green ridge with white foam along its crest, rising out of the sea as a
//! dark line first; and the green washes, a foam line where each will come
//! aboard, then a low wall of green water running across the deck.

use crate::balance::*;
use crate::content::Content;
use crate::course::{Rogue, Wash};
use crate::fx::{self, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::Batch;
use crate::world::World;

/// a rogue wave's crest is drawn in pieces this long
const PIECE: i32 = 40 * ONE;
/// a wash's front, in pieces this long, each standing on its own floor
const WASH_PIECE: i32 = 2 * ONE;

/// Every wave over the course.
pub fn waves(b: &mut Batch, c: &Content, w: &World) {
    let Some(k) = &w.course else { return };
    for r in &k.waves.rogues {
        rogue(b, c, w, r, k.water);
    }
    for s in &k.waves.washes {
        wash(b, c, w, s);
    }
}

/// A box laid along `dir` (its length across it, its depth along it).
fn laid(at: V3, dir: V3, size: V3) -> Affine {
    Affine::translate(at)
        .then(&Affine::looking(dir))
        .then(&Affine::scale(size))
}

fn rogue(b: &mut Batch, c: &Content, w: &World, r: &Rogue, water: i32) {
    let tall = r.tall(w.tick);
    if tall <= 0 {
        return;
    }
    let green = c.pal.c("rogue_wave");
    let foam = c.pal.c("river_foam");
    let dark = c.pal.c("ocean_deep");
    let crest = r.crest(w.tick);
    let across = v3(-r.dir.z, 0, r.dir.x);
    let warning = r.age(w.tick) < ROGUE_WARN;
    let pieces = r.reach / PIECE;
    for n in -pieces..=pieces {
        let at = crest.add(across.scale(PIECE * n));
        let sea = v3(at.x, water, at.z);
        // its body: a swell from foot to foot, `tall` over the sea
        b.inked(
            Mesh::Blob,
            &laid(sea, r.dir, v3(PIECE + PIECE / 3, tall * 2, r.half * 2)),
            green,
            0,
        );
        // the foam along its crest, and its lip curling over ahead
        let top = sea.add(v3(0, tall - tall / 10, 0));
        b.glow(
            Mesh::Blob,
            &laid(top, r.dir, v3(PIECE + PIECE / 4, ONE + tall / 6, r.half / 3)),
            foam,
            ONE / 4,
        );
        b.glow(
            Mesh::Blob,
            &laid(
                top.add(r.dir.scale(r.half / 5)).add(v3(0, -tall / 8, 0)),
                r.dir,
                v3(PIECE, ONE, r.half / 6),
            ),
            foam,
            ONE / 4,
        );
        // rising: a dark line on the sea ahead of it
        if warning {
            b.inked(
                Mesh::Cube,
                &laid(
                    sea.add(r.dir.scale(r.half)).add(v3(0, ONE / 4, 0)),
                    r.dir,
                    v3(PIECE, ONE / 4, 3 * ONE),
                ),
                dark,
                0,
            );
        }
    }
}

fn wash(b: &mut Batch, c: &Content, w: &World, s: &Wash) {
    let Some(until) = s.until(w.tick) else { return };
    let green = c.pal.c("wash");
    let foam = c.pal.c("river_foam");
    let front = s.front(w.tick);
    let across = v3(-s.dir.z, 0, s.dir.x);
    let pieces = s.reach / WASH_PIECE;
    let lift = w.course.as_ref().map_or(0, |k| k.waves.lift(front, w.tick));
    for n in -pieces..=pieces {
        let at = front.add(across.scale(WASH_PIECE * n));
        // over whatever floor is under it (a deck heaving, or the sea)
        let floor = w
            .arena
            .level
            .floor_below(at.x, s.from.y + 5 * ONE + lift, at.z)
            .unwrap_or(w.course.as_ref().map_or(0, |k| k.water));
        let on = v3(at.x, floor, at.z);
        if until > 0 {
            // its foam line where it will come aboard, brightening
            let g = ONE - fx::div(until * ONE, WASH_WARN.max(1) * ONE) / 2;
            b.glow(
                Mesh::Cube,
                &laid(
                    on.add(v3(0, ONE / 20, 0)),
                    s.dir,
                    v3(WASH_PIECE, ONE / 10, ONE / 2),
                ),
                foam,
                g,
            );
            continue;
        }
        // the wall of green water, and its foam
        b.inked(
            Mesh::Blob,
            &laid(
                on.add(v3(0, WASH_HEIGHT / 2, 0)),
                s.dir,
                v3(WASH_PIECE + ONE / 2, WASH_HEIGHT + ONE / 4, WASH_THICK * 2),
            ),
            green,
            0,
        );
        b.glow(
            Mesh::Blob,
            &laid(
                on.add(v3(0, WASH_HEIGHT, 0)).add(s.dir.scale(WASH_THICK / 2)),
                s.dir,
                v3(WASH_PIECE + ONE / 2, ONE / 3, WASH_THICK),
            ),
            foam,
            ONE / 3,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::tests::course_world_in;

    #[test]
    fn the_hurricanes_waves_draw_within_their_budget_and_show() {
        let (c, mut w) = course_world_in("far_shore", "hurricane");
        let k = w.course.as_ref().unwrap();
        assert!(!k.waves.rogues.is_empty() && !k.waves.washes.is_empty());
        let mut most = 0;
        let mut drew = false;
        for t in 0..ROGUE_PERIOD {
            w.tick = t;
            let mut b = Batch::new([0; 3]);
            waves(&mut b, &c, &w);
            most = most.max(b.count());
            drew |= b.count() > 0;
        }
        assert!(drew, "no wave was drawn");
        assert!(most <= 400, "the waves drew {most} instances");
    }
}
