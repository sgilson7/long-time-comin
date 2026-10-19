//! The map: the timeline as a hologram over the crater, as Iron Vector's
//! star map hangs over its console. Its root, 2031, is at the top; each
//! later zone of the main line is a row below it, down the left of the
//! middle; the River forks after its zone (the Glass House) and runs down
//! the right. Each zone is a small diorama of itself; the page puts its
//! label beside it, where the core says it lands on screen.

use crate::balance::*;
use crate::content::Content;
use crate::geom::{v3, V3};
use crate::render::Camera;

/// A zone on the map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spot {
    pub zone: usize,
    /// "both" for the years both lines share, else "peak" or "river"
    pub line: &'static str,
    pub at: V3,
}

/// Every built zone on the two lines, and the beams between them (by spot).
pub fn spots(c: &Content) -> (Vec<Spot>, Vec<(usize, usize)>) {
    let main = c.built_zones();
    let fork = c
        .zones
        .iter()
        .find(|z| z.built && !z.branch_from.is_empty())
        .and_then(|z| c.zones.iter().position(|f| f.id == z.branch_from));
    let branch: Vec<usize> = (0..c.zones.len())
        .filter(|&k| c.zones[k].built && !c.zones[k].branch_from.is_empty())
        .collect();
    let rows = main.max(fork.map_or(0, |f| f + 1 + branch.len())) as i32;
    let place = |row: i32, side: i32| {
        // the middle row at the centre; rows lower down lean back a little
        let y = ATLAS_ROW * (rows - 1) / 2 - ATLAS_ROW * row;
        v3(side * ATLAS_COL, y, -ATLAS_LEAN * row).add(ATLAS_CENTRE)
    };
    let mut spots = Vec::new();
    let mut beams = Vec::new();
    // with no branch the main line runs down the middle
    let left = if fork.is_some() { -1 } else { 0 };
    for k in 0..main {
        let line = if fork.is_some_and(|f| k <= f) {
            "both"
        } else {
            "peak"
        };
        spots.push(Spot {
            zone: k,
            line,
            at: place(k as i32, left),
        });
        if k > 0 {
            beams.push((k - 1, k));
        }
    }
    if let Some(f) = fork {
        let mut from = f;
        for (n, &k) in branch.iter().enumerate() {
            spots.push(Spot {
                zone: k,
                line: "river",
                at: place((f + 1 + n) as i32, 1),
            });
            beams.push((from, spots.len() - 1));
            from = spots.len() - 1;
        }
    }
    (spots, beams)
}

/// The camera looking at the map, square on, from in front of its middle.
pub fn camera() -> Camera {
    let eye = ATLAS_CENTRE.add(v3(0, 0, ATLAS_BACK));
    Camera {
        eye,
        fwd: ATLAS_CENTRE.sub(eye).norm(),
        fov: FOV,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;
    use crate::fx::ONE;

    #[test]
    fn the_root_is_at_the_top_and_each_later_year_hangs_below_the_one_before() {
        let c = content();
        let (s, beams) = spots(&c);
        let main: Vec<&Spot> = s.iter().filter(|p| p.line != "river").collect();
        assert_eq!(c.zones[main[0].zone].id, "waiting_city");
        for w in main.windows(2) {
            assert!(
                w[1].at.y < w[0].at.y,
                "{} below {}",
                c.zones[w[1].zone].id,
                c.zones[w[0].zone].id
            );
        }
        for &(a, b) in &beams {
            assert!(s[b].at.y < s[a].at.y, "a beam runs down");
        }
    }

    #[test]
    fn the_river_forks_after_the_glass_house_down_its_own_side() {
        let c = content();
        let (s, beams) = spots(&c);
        let house = s
            .iter()
            .position(|p| c.zones[p.zone].id == "glass_house")
            .unwrap();
        let river: Vec<usize> = (0..s.len()).filter(|&k| s[k].line == "river").collect();
        assert_eq!(river.len(), 3);
        assert!(beams.contains(&(house, river[0])), "the fork comes off the house");
        for &k in &river {
            assert!(s[k].at.x > 0, "the River on the right");
        }
        for p in s.iter().filter(|p| p.line != "river") {
            assert!(p.at.x < 0, "the main line on the left");
        }
    }

    #[test]
    fn every_zone_is_on_screen_with_room_either_side_for_its_label() {
        let c = content();
        let (s, _) = spots(&c);
        let cam = camera();
        // a wide window and a narrow one (aspect, Q16)
        for aspect in [ONE * 16 / 9, ONE * 4 / 3] {
            for p in &s {
                let (x, y) = cam.screen(aspect, p.at).expect("in front");
                let (x, y) = (x / ONE, y / ONE);
                assert!((8..=92).contains(&y), "{}: {y}% down", c.zones[p.zone].id);
                assert!((20..=80).contains(&x), "{}: {x}% across", c.zones[p.zone].id);
            }
        }
    }
}
