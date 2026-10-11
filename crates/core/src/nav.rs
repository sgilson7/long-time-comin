//! Where a person can walk in an arena, for the house's agents: a grid of
//! half-metre columns, each holding the heights a body can stand at in it
//! (the floor, a gallery over it, a table), joined to its neighbours where a
//! body can step up (at most `AGENT_STEP`) or drop (at most `AGENT_DROP`,
//! and never through a floor between). A path is the shortest walk on it.
//!
//! It also knows which places are next to something tall enough to hide
//! behind: an agent's cover.

use crate::balance::{AGENT_DROP, AGENT_STEP, PLAYER_HEIGHT, PLAYER_RADIUS};
use crate::fx::{int, ONE};
use crate::geom::{v3, V3};
use crate::level::Level;
use std::collections::VecDeque;

/// The grid's spacing.
pub const CELL: i32 = ONE / 2;
/// heights kept in one column, lowest first
const LEVELS: usize = 3;
/// something this much taller than a place, beside it, hides a body there
const COVER_HEIGHT: i32 = ONE * 15 / 10;
const NONE: u32 = u32::MAX;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nav {
    /// columns along x and z, from `-half` in steps of `CELL`
    pub side: i32,
    pub half: i32,
    /// for each column, up to `LEVELS` standing heights (`i32::MIN` for none)
    heights: Vec<[i32; LEVELS]>,
    /// nodes beside cover
    pub cover: Vec<u32>,
}

impl Nav {
    /// The grid over the square `half` either side of the middle.
    pub fn build(level: &Level, half: i32) -> Nav {
        let side = half * 2 / CELL + 1;
        let mut heights = vec![[i32::MIN; LEVELS]; (side * side) as usize];
        let r = PLAYER_RADIUS * 3 / 4;
        for j in 0..side {
            for i in 0..side {
                let (x, z) = (-half + i * CELL, -half + j * CELL);
                let mut tops: Vec<i32> = Vec::new();
                level.each_near(v3(x - r, -int(12), z - r), v3(x + r, int(40), z + r), |s| {
                    let over = s.min.x <= x + r && s.max.x >= x - r && s.min.z <= z + r && s.max.z >= z - r;
                    if over && s.max.y < int(30) && !tops.contains(&s.max.y) {
                        tops.push(s.max.y);
                    }
                });
                tops.sort();
                let mut k = 0;
                for y in tops {
                    if k == LEVELS {
                        break;
                    }
                    let free = !level.blocked(
                        v3(x - r, y + ONE / 20, z - r),
                        v3(x + r, y + PLAYER_HEIGHT, z + r),
                    );
                    if free {
                        heights[(j * side + i) as usize][k] = y;
                        k += 1;
                    }
                }
            }
        }
        let mut nav = Nav {
            side,
            half,
            heights,
            cover: Vec::new(),
        };
        for n in 0..(side * side) as u32 * LEVELS as u32 {
            let Some(p) = nav.pos(n) else {
                continue;
            };
            let near = ONE * 8 / 10;
            let tall = level.blocked(
                v3(p.x - near, p.y + ONE / 2, p.z - near),
                v3(p.x + near, p.y + COVER_HEIGHT, p.z + near),
            );
            if tall {
                nav.cover.push(n);
            }
        }
        nav
    }

    fn column(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        let i = (x + self.half + CELL / 2).div_euclid(CELL);
        let j = (z + self.half + CELL / 2).div_euclid(CELL);
        (i >= 0 && j >= 0 && i < self.side && j < self.side).then_some((i, j))
    }

    /// The place node `n` stands for: a column's centre at a height.
    pub fn pos(&self, n: u32) -> Option<V3> {
        if n == NONE {
            return None;
        }
        let c = (n / LEVELS as u32) as i32;
        let k = (n % LEVELS as u32) as usize;
        let y = *self.heights.get(c as usize)?.get(k)?;
        if y == i32::MIN {
            return None;
        }
        let (i, j) = (c % self.side, c / self.side);
        Some(v3(-self.half + i * CELL, y, -self.half + j * CELL))
    }

    /// The node nearest a body's feet: in its column (or one beside it), the
    /// highest standing height not above the feet by more than a step.
    pub fn nearest(&self, feet: V3) -> Option<u32> {
        let (ci, cj) = self.column(feet.x, feet.z)?;
        for ring in 0..4i32 {
            let mut best: Option<(i32, u32)> = None;
            for dj in -ring..=ring {
                for di in -ring..=ring {
                    if di.abs() != ring && dj.abs() != ring {
                        continue;
                    }
                    let (i, j) = (ci + di, cj + dj);
                    if i < 0 || j < 0 || i >= self.side || j >= self.side {
                        continue;
                    }
                    let c = j * self.side + i;
                    for (k, &y) in self.heights[c as usize].iter().enumerate() {
                        if y == i32::MIN || y > feet.y + AGENT_STEP {
                            continue;
                        }
                        let gap = feet.y - y;
                        if best.is_none_or(|(b, _)| gap < b) {
                            best = Some((gap, c as u32 * LEVELS as u32 + k as u32));
                        }
                    }
                }
            }
            if let Some((_, n)) = best {
                return Some(n);
            }
        }
        None
    }

    /// The nodes a body at `n` can walk to in one step.
    fn next(&self, n: u32, out: &mut Vec<u32>) {
        out.clear();
        let c = (n / LEVELS as u32) as i32;
        let y = self.heights[c as usize][(n % LEVELS as u32) as usize];
        let (i, j) = (c % self.side, c / self.side);
        for (di, dj) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ] {
            let (ni, nj) = (i + di, j + dj);
            if ni < 0 || nj < 0 || ni >= self.side || nj >= self.side {
                continue;
            }
            let pick = |ii: i32, jj: i32| -> Option<usize> {
                let col = &self.heights[(jj * self.side + ii) as usize];
                // the highest height reachable: a step up, or a drop with
                // nothing between
                let mut best = None;
                for (k, &h) in col.iter().enumerate() {
                    if h == i32::MIN || h > y + AGENT_STEP || h < y - AGENT_DROP {
                        continue;
                    }
                    best = Some(k);
                }
                let k = best?;
                let h = col[k];
                let floor_between = col.iter().any(|&o| o != i32::MIN && o > h && o <= y + AGENT_STEP);
                (!floor_between).then_some(k)
            };
            let Some(k) = pick(ni, nj) else {
                continue;
            };
            if di != 0 && dj != 0 && (pick(i + di, j).is_none() || pick(i, j + dj).is_none()) {
                continue;
            }
            out.push((nj * self.side + ni) as u32 * LEVELS as u32 + k as u32);
        }
    }

    /// The shortest walk from `from` to `to`, as places, `to` last; empty
    /// if there is none (or `from` is `to`). Gives up beyond `limit` nodes.
    pub fn path(&self, from: u32, to: u32, limit: usize) -> Vec<V3> {
        if from == to {
            return Vec::new();
        }
        let total = self.heights.len() * LEVELS;
        let mut parent = vec![NONE; total];
        let mut q = VecDeque::new();
        parent[from as usize] = from;
        q.push_back(from);
        let mut seen = 0;
        let mut buf = Vec::with_capacity(8);
        while let Some(n) = q.pop_front() {
            if n == to {
                break;
            }
            seen += 1;
            if seen > limit {
                return Vec::new();
            }
            self.next(n, &mut buf);
            for &m in &buf {
                if parent[m as usize] == NONE {
                    parent[m as usize] = n;
                    q.push_back(m);
                }
            }
        }
        if parent[to as usize] == NONE {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut n = to;
        while n != from {
            if let Some(p) = self.pos(n) {
                out.push(p);
            }
            n = parent[n as usize];
        }
        out.reverse();
        out
    }

    /// Every node a walk from `from` reaches, for tests and the builder.
    pub fn reach(&self, from: u32) -> Vec<u32> {
        let total = self.heights.len() * LEVELS;
        let mut seen = vec![false; total];
        let mut q = VecDeque::new();
        seen[from as usize] = true;
        q.push_back(from);
        let mut out = Vec::new();
        let mut buf = Vec::with_capacity(8);
        while let Some(n) = q.pop_front() {
            out.push(n);
            self.next(n, &mut buf);
            for &m in &buf {
                if !seen[m as usize] {
                    seen[m as usize] = true;
                    q.push_back(m);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::Solid;

    fn room() -> Level {
        Level::new(vec![
            Solid::new(v3(-int(20), -ONE, -int(20)), v3(int(20), 0, int(20))),
            // a wall across the middle with a gap at its east end
            Solid::new(v3(-int(20), 0, -ONE / 4), v3(int(10), int(4), ONE / 4)),
            // a gallery 5 m up along the west, over a walkable floor
            Solid::new(
                v3(-int(12), int(5) - ONE * 4 / 10, int(4)),
                v3(-int(6), int(5), int(12)),
            ),
        ])
    }

    #[test]
    fn a_path_goes_round_a_wall_and_not_through_it() {
        let l = room();
        let n = Nav::build(&l, int(16));
        let a = n.nearest(v3(0, 0, -int(6))).unwrap();
        let b = n.nearest(v3(0, 0, int(6))).unwrap();
        let p = n.path(a, b, 100_000);
        assert!(!p.is_empty(), "a way round");
        // it goes through the gap, east of x = 10
        assert!(p.iter().any(|q| q.x > int(10) && q.z.abs() < ONE), "{p:?}");
        assert!(p
            .iter()
            .all(|q| l.floor_below(q.x, q.y + ONE / 10, q.z).is_some()));
    }

    #[test]
    fn a_gallery_with_no_stair_cannot_be_walked_onto_but_can_be_dropped_from() {
        let l = room();
        let n = Nav::build(&l, int(16));
        let floor = n.nearest(v3(-int(9), 0, int(8))).unwrap();
        let top = n.nearest(v3(-int(9), int(5), int(8))).unwrap();
        assert_eq!(n.pos(top).unwrap().y, int(5));
        assert_eq!(n.pos(floor).unwrap().y, 0, "under the gallery is a place too");
        assert!(n.path(floor, top, 100_000).is_empty(), "no stair up");
        assert!(!n.path(top, floor, 100_000).is_empty(), "a drop off its edge");
    }

    #[test]
    fn places_beside_a_wall_are_cover() {
        let l = room();
        let n = Nav::build(&l, int(16));
        let beside = n.nearest(v3(0, 0, -ONE)).unwrap();
        let open = n.nearest(v3(0, 0, -int(8))).unwrap();
        assert!(n.cover.contains(&beside));
        assert!(!n.cover.contains(&open));
    }
}
