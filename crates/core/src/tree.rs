//! The Holding's shop as a passive tree, after Path of Exile 2's: a hub in
//! the middle, a ring round it on which every upgrade that needs nothing
//! stands, and each branch bursting outward from the ring in its own wedge,
//! every upgrade one step further out than the one it needs. Only where
//! things are drawn is decided here; what may be bought is `meta.rs`'s.

use crate::balance::*;
use crate::content::Upgrade;
use crate::fx::{self, TURN};

/// One upgrade's place, in units of a square `TREE_SIZE` across.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub x: i32,
    pub y: i32,
    /// the angle round the hub, clockwise from straight up, in turns
    pub angle: i32,
    /// how many upgrades it stands beyond the ring
    pub ring: i32,
}

/// A branch's wedge: the angles it spans and where its name is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wedge {
    pub branch: String,
    pub from: i32,
    pub to: i32,
    /// how far out it reaches: a little past its outermost upgrade
    pub reach: i32,
    pub label: (i32, i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    /// in the order of the upgrades given
    pub places: Vec<Place>,
    /// (needed, needing), indices into the upgrades
    pub edges: Vec<(usize, usize)>,
    pub wedges: Vec<Wedge>,
}

/// The point `r` out from the centre at `angle` (clockwise from up).
pub fn at(angle: i32, r: i32) -> (i32, i32) {
    let c = TREE_SIZE / 2;
    (
        c + fx::mul(fx::int(r), fx::sin(angle)) / fx::ONE,
        c - fx::mul(fx::int(r), fx::cos(angle)) / fx::ONE,
    )
}

/// Lay out the upgrades, branch by branch in the order given. An upgrade
/// whose branch is not listed is put in the last.
pub fn lay_out(upgrades: &[Upgrade], branches: &[String]) -> Tree {
    let n = upgrades.len();
    let parent: Vec<Option<usize>> = upgrades
        .iter()
        .map(|u| {
            upgrades
                .iter()
                .position(|p| p.id == u.requires && !u.requires.is_empty())
        })
        .collect();
    // how many outermost upgrades hang from each, which sets its share
    fn leaves(i: usize, parent: &[Option<usize>]) -> i32 {
        let kids: Vec<usize> = (0..parent.len()).filter(|&j| parent[j] == Some(i)).collect();
        if kids.is_empty() {
            1
        } else {
            kids.iter().map(|&k| leaves(k, parent)).sum()
        }
    }
    let branch_of = |u: &Upgrade| {
        branches
            .iter()
            .position(|b| *b == u.branch)
            .unwrap_or(branches.len().max(1) - 1)
    };
    let nb = branches.len().max(1);
    let roots: Vec<Vec<usize>> = (0..nb)
        .map(|b| {
            (0..n)
                .filter(|&i| parent[i].is_none() && branch_of(&upgrades[i]) == b)
                .collect()
        })
        .collect();
    let halves: i32 = roots
        .iter()
        .flatten()
        .map(|&r| 2 * leaves(r, &parent))
        .sum::<i32>()
        + 2 * TREE_GAP_HALVES * nb as i32;
    let angle_of = |h: i32| TREE_START + (TURN as i64 * h as i64 / halves.max(1) as i64) as i32;

    let mut places = vec![
        Place {
            x: TREE_SIZE / 2,
            y: TREE_SIZE / 2,
            angle: 0,
            ring: 0
        };
        n
    ];
    let mut wedges = Vec::new();
    let mut cursor = 0;
    // each upgrade takes the middle of its share; its own share is split
    // among the upgrades that need it, in the order the data lists them
    fn place(
        i: usize,
        from: i32,
        ring: i32,
        parent: &[Option<usize>],
        angle_of: &dyn Fn(i32) -> i32,
        places: &mut [Place],
    ) {
        // `from` and the share are in halves of one outermost upgrade's
        let angle = angle_of(from + leaves(i, parent));
        let (x, y) = at(angle, TREE_RING + ring * TREE_STEP);
        places[i] = Place { x, y, angle, ring };
        let mut at_h = from;
        for k in (0..parent.len()).filter(|&j| parent[j] == Some(i)) {
            place(k, at_h, ring + 1, parent, angle_of, places);
            at_h += 2 * leaves(k, parent);
        }
    }
    for (b, list) in roots.iter().enumerate() {
        let start = cursor;
        cursor += TREE_GAP_HALVES;
        for &r in list {
            place(r, cursor, 0, &parent, &angle_of, &mut places);
            cursor += 2 * leaves(r, &parent);
        }
        cursor += TREE_GAP_HALVES;
        let (from, to) = (angle_of(start), angle_of(cursor));
        let outer = (0..n)
            .filter(|&i| branch_of(&upgrades[i]) == b)
            .map(|i| places[i].ring)
            .max()
            .unwrap_or(0);
        wedges.push(Wedge {
            branch: branches.get(b).cloned().unwrap_or_default(),
            from,
            to,
            reach: (TREE_RING + outer * TREE_STEP + TREE_PETAL).min(TREE_EDGE),
            label: at(from + (to - from) / 2, TREE_LABEL),
        });
    }
    let edges = (0..n).filter_map(|j| parent[j].map(|i| (i, j))).collect();
    Tree {
        places,
        edges,
        wedges,
    }
}

/// The SVG path of a wedge: the ring of the branch between the hub and
/// its edge, from one angle to the other.
pub fn wedge_path(w: &Wedge) -> String {
    let (a, b) = (at(w.from, w.reach), at(w.to, w.reach));
    let (c, d) = (at(w.to, TREE_HUB), at(w.from, TREE_HUB));
    let large = i32::from(w.to - w.from > TURN / 2);
    format!(
        "M {} {} A {e} {e} 0 {large} 1 {} {} L {} {} A {h} {h} 0 {large} 0 {} {} Z",
        a.0,
        a.1,
        b.0,
        b.1,
        c.0,
        c.1,
        d.0,
        d.1,
        e = w.reach,
        h = TREE_HUB,
    )
}

/// How far a wedge's name is turned, in degrees, to run along the ring,
/// kept upright on the lower half.
pub fn label_turn(w: &Wedge) -> i32 {
    let mid = fx::wrap(w.from + (w.to - w.from) / 2);
    let deg = (mid as i64 * 360 / TURN as i64) as i32;
    let deg = deg.rem_euclid(360);
    if deg > 90 && deg < 270 {
        deg - 180
    } else {
        deg
    }
}

/// A length in tree units as a percentage of the square, to a tenth.
pub fn pct(v: i32) -> String {
    let t = v * 1000 / TREE_SIZE;
    format!("{}.{}%", t / 10, t % 10)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;

    fn tree() -> (crate::content::Content, Vec<String>, Tree) {
        let c = content();
        let branches: Vec<String> = c
            .copy_rows("holding_branches")
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        let t = lay_out(&c.upgrades, &branches);
        (c, branches, t)
    }

    fn out(p: &Place) -> i32 {
        let (dx, dy) = ((p.x - TREE_SIZE / 2) as i64, (p.y - TREE_SIZE / 2) as i64);
        fx::isqrt((dx * dx + dy * dy) as u64) as i32
    }

    #[test]
    fn every_upgrade_that_needs_nothing_stands_on_the_ring_and_the_rest_a_step_out_from_what_they_need() {
        let (c, _, t) = tree();
        for (i, u) in c.upgrades.iter().enumerate() {
            let p = &t.places[i];
            match c.upgrades.iter().position(|x| x.id == u.requires) {
                None => {
                    assert_eq!(p.ring, 0, "{} needs nothing", u.id);
                    assert!(
                        (out(p) - TREE_RING).abs() <= 2,
                        "{} on the ring ({})",
                        u.id,
                        out(p)
                    );
                }
                Some(k) => {
                    assert_eq!(
                        p.ring,
                        t.places[k].ring + 1,
                        "{} one ring past {}",
                        u.id,
                        c.upgrades[k].id
                    );
                    assert!(
                        out(p) > out(&t.places[k]) + TREE_STEP / 2,
                        "{} further out than {}",
                        u.id,
                        c.upgrades[k].id
                    );
                    assert!(
                        t.edges.contains(&(k, i)),
                        "a line from {} to {}",
                        c.upgrades[k].id,
                        u.id
                    );
                }
            }
            assert!(
                out(p) > TREE_HUB && out(p) + TREE_ROOM / 2 < TREE_SIZE / 2,
                "{} inside the square, outside the hub",
                u.id
            );
        }
    }

    #[test]
    fn no_two_upgrades_crowd_each_other() {
        let (c, _, t) = tree();
        for i in 0..t.places.len() {
            for j in i + 1..t.places.len() {
                let (a, b) = (&t.places[i], &t.places[j]);
                let (dx, dy) = ((a.x - b.x) as i64, (a.y - b.y) as i64);
                let d = fx::isqrt((dx * dx + dy * dy) as u64) as i32;
                assert!(
                    d >= TREE_ROOM,
                    "{} and {} are {} apart",
                    c.upgrades[i].id,
                    c.upgrades[j].id,
                    d
                );
            }
        }
    }

    #[test]
    fn each_branch_bursts_out_in_its_own_wedge() {
        let (c, branches, t) = tree();
        assert_eq!(t.wedges.len(), branches.len());
        for (k, w) in t.wedges.iter().enumerate() {
            assert!(w.from < w.to, "{} has width", w.branch);
            if k > 0 {
                assert!(
                    w.from > t.wedges[k - 1].to - 1,
                    "{} starts after {} ends",
                    w.branch,
                    t.wedges[k - 1].branch
                );
            }
        }
        let last = t.wedges.last().unwrap();
        assert!(last.to <= t.wedges[0].from + TURN, "the wedges go once round");
        for (i, u) in c.upgrades.iter().enumerate() {
            let w = t.wedges.iter().find(|w| w.branch == u.branch).unwrap();
            assert!(
                out(&t.places[i]) + TREE_ROOM / 2 <= w.reach,
                "{} inside its petal",
                u.id
            );
        }
        for (i, u) in c.upgrades.iter().enumerate() {
            let w = t
                .wedges
                .iter()
                .find(|w| w.branch == u.branch)
                .expect("its branch has a wedge");
            let a = t.places[i].angle;
            assert!(
                a > w.from && a < w.to,
                "{} at {} inside {}'s {}..{}",
                u.id,
                a,
                w.branch,
                w.from,
                w.to
            );
            // its icon (about TREE_ROOM across) clears the wedge's border lines
            let r = out(&t.places[i]);
            let clear = fx::mul(fx::int(r), fx::sin((a - w.from).min(w.to - a))) / fx::ONE;
            assert!(
                clear >= TREE_ROOM * 3 / 4,
                "{} is {} from {}'s border",
                u.id,
                clear,
                w.branch
            );
        }
    }
}
