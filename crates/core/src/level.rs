//! The solid part of an arena: axis-aligned boxes, a grid over them so a
//! query looks only at the boxes near it, and the three questions the rest
//! of the core asks: what does this box overlap, where does this ray first
//! hit, and does this sphere touch anything.

use crate::fx::{self, int, ONE};
use crate::geom::{v3, V3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Solid {
    pub min: V3,
    pub max: V3,
}

impl Solid {
    pub fn new(min: V3, max: V3) -> Solid {
        Solid { min, max }
    }

    /// A box standing on `y` with its footprint centred at `(x, z)`.
    pub fn stand(x: i32, y: i32, z: i32, w: i32, h: i32, d: i32) -> Solid {
        Solid {
            min: v3(x - w / 2, y, z - d / 2),
            max: v3(x + w / 2, y + h, z + d / 2),
        }
    }

    pub fn overlaps(&self, min: V3, max: V3) -> bool {
        self.min.x < max.x
            && self.max.x > min.x
            && self.min.y < max.y
            && self.max.y > min.y
            && self.min.z < max.z
            && self.max.z > min.z
    }

    pub fn centre(&self) -> V3 {
        self.min.lerp(self.max, ONE / 2)
    }

    pub fn size(&self) -> V3 {
        self.max.sub(self.min)
    }

    /// The point of the box nearest `p`.
    pub fn nearest(&self, p: V3) -> V3 {
        v3(
            p.x.clamp(self.min.x, self.max.x),
            p.y.clamp(self.min.y, self.max.y),
            p.z.clamp(self.min.z, self.max.z),
        )
    }

    /// The distance along a unit ray from `o` to where it enters the box, if
    /// it does within `max_t`. A ray starting inside reports zero.
    pub fn ray(&self, o: V3, d: V3, max_t: i32) -> Option<i32> {
        let mut t0: i64 = 0;
        let mut t1: i64 = max_t as i64;
        for (o, d, lo, hi) in [
            (o.x, d.x, self.min.x, self.max.x),
            (o.y, d.y, self.min.y, self.max.y),
            (o.z, d.z, self.min.z, self.max.z),
        ] {
            if d == 0 {
                if o < lo || o > hi {
                    return None;
                }
                continue;
            }
            let a = (((lo - o) as i64) << fx::FRAC_BITS) / d as i64;
            let b = (((hi - o) as i64) << fx::FRAC_BITS) / d as i64;
            let (near, far) = if a < b { (a, b) } else { (b, a) };
            t0 = t0.max(near);
            t1 = t1.min(far);
            if t0 > t1 {
                return None;
            }
        }
        Some(t0 as i32)
    }
}

/// A solid that can change while the arena is played: a gate that opens,
/// a rotten floor that breaks, a root that rises and sinks, a Bench laid in
/// the air. Few at once, so they are asked one by one, not through the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dyn {
    pub solid: Solid,
    /// there now (a gate shut, a floor whole, a Bench laid)
    pub on: bool,
    /// stood on from above only: a body rises through it, shots and orbs
    /// pass through it, and only a body's fall stops on its top (the Bench)
    pub one_way: bool,
}

/// Grid cells are this wide.
const CELL: i32 = int(8);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Level {
    pub solids: Vec<Solid>,
    origin: (i32, i32),
    dims: (i32, i32),
    cells: Vec<Vec<u16>>,
    /// each solid's first cell, so a query that spans several cells calls
    /// back once per solid without keeping a list of what it has seen
    first: Vec<(i32, i32)>,
    /// the solids that change, asked after the grid's; their indices follow
    /// the fixed solids'
    pub dynamic: Vec<Dyn>,
}

/// What a ray met first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub t: i32,
    pub point: V3,
    /// the solid it met, by index
    pub solid: usize,
}

impl Level {
    pub fn new(solids: Vec<Solid>) -> Level {
        let (mut lo_x, mut lo_z, mut hi_x, mut hi_z) = (0, 0, CELL, CELL);
        for s in &solids {
            lo_x = lo_x.min(s.min.x);
            lo_z = lo_z.min(s.min.z);
            hi_x = hi_x.max(s.max.x);
            hi_z = hi_z.max(s.max.z);
        }
        let origin = (lo_x - CELL, lo_z - CELL);
        let dims = ((hi_x - origin.0) / CELL + 2, (hi_z - origin.1) / CELL + 2);
        let mut cells = vec![Vec::new(); (dims.0 * dims.1) as usize];
        let mut first = Vec::with_capacity(solids.len());
        for (i, s) in solids.iter().enumerate() {
            let (x0, z0) = Level::cell_of(origin, dims, s.min.x, s.min.z);
            let (x1, z1) = Level::cell_of(origin, dims, s.max.x, s.max.z);
            first.push((x0, z0));
            for cz in z0..=z1 {
                for cx in x0..=x1 {
                    cells[(cz * dims.0 + cx) as usize].push(i as u16);
                }
            }
        }
        Level {
            solids,
            origin,
            dims,
            cells,
            first,
            dynamic: Vec::new(),
        }
    }

    /// A solid by its index: a fixed one, or a changing one after them.
    pub fn solid(&self, i: usize) -> &Solid {
        match i.checked_sub(self.solids.len()) {
            Some(k) => &self.dynamic[k].solid,
            None => &self.solids[i],
        }
    }

    /// Adds a changing solid; its index among `dynamic`.
    pub fn add_dynamic(&mut self, d: Dyn) -> usize {
        self.dynamic.push(d);
        self.dynamic.len() - 1
    }

    /// The tops of the one-way solids there now under the box's footprint
    /// that a body falling from `from_y` to the box's bottom passes down
    /// through: the highest, if any.
    pub fn one_way_catch(&self, min: V3, max: V3, from_y: i32) -> Option<i32> {
        self.dynamic
            .iter()
            .filter(|d| d.on && d.one_way)
            .filter(|d| {
                let s = d.solid;
                s.min.x < max.x && s.max.x > min.x && s.min.z < max.z && s.max.z > min.z
            })
            .map(|d| d.solid.max.y)
            .filter(|&top| from_y >= top && min.y < top)
            .max()
    }

    fn cell_of(origin: (i32, i32), dims: (i32, i32), x: i32, z: i32) -> (i32, i32) {
        (
            ((x - origin.0) / CELL).clamp(0, dims.0 - 1),
            ((z - origin.1) / CELL).clamp(0, dims.1 - 1),
        )
    }

    /// Calls `f` once for each solid whose cells touch the box `min..max`.
    pub fn each_near(&self, min: V3, max: V3, mut f: impl FnMut(&Solid)) {
        self.each_near_indexed(min, max, |_, s| f(s));
    }

    /// As `each_near`, with each solid's index.
    pub fn each_near_indexed(&self, min: V3, max: V3, mut f: impl FnMut(usize, &Solid)) {
        let (x0, z0) = Level::cell_of(self.origin, self.dims, min.x, min.z);
        let (x1, z1) = Level::cell_of(self.origin, self.dims, max.x, max.z);
        for cz in z0..=z1 {
            for cx in x0..=x1 {
                for &i in &self.cells[(cz * self.dims.0 + cx) as usize] {
                    // the first cell this query shares with the solid
                    let (fx0, fz0) = self.first[i as usize];
                    if (fx0.max(x0), fz0.max(z0)) == (cx, cz) {
                        f(i as usize, &self.solids[i as usize]);
                    }
                }
            }
        }
        let n = self.solids.len();
        for (k, d) in self.dynamic.iter().enumerate() {
            let s = &d.solid;
            let near = s.min.x <= max.x + CELL
                && s.max.x >= min.x - CELL
                && s.min.z <= max.z + CELL
                && s.max.z >= min.z - CELL;
            if d.on && !d.one_way && near {
                f(n + k, s);
            }
        }
    }

    pub fn blocked(&self, min: V3, max: V3) -> bool {
        let mut hit = false;
        self.each_near(min, max, |s| hit |= s.overlaps(min, max));
        hit
    }

    /// Does a sphere touch a solid?
    pub fn sphere_hits(&self, c: V3, r: i32) -> bool {
        let rr = v3(r, r, r);
        let mut hit = false;
        let r2 = r as i64 * r as i64;
        self.each_near(c.sub(rr), c.add(rr), |s| {
            if hit {
                return;
            }
            let d = s.nearest(c).sub(c);
            let d2 = d.x as i64 * d.x as i64 + d.y as i64 * d.y as i64 + d.z as i64 * d.z as i64;
            hit = d2 < r2;
        });
        hit
    }

    /// The first solid a unit ray meets within `max_t`. The ray is walked in
    /// grid-sized steps so a long ray asks only the cells it passes.
    pub fn ray(&self, o: V3, d: V3, max_t: i32) -> Option<Hit> {
        let mut best: Option<(i32, usize)> = None;
        let mut start = 0;
        while start < max_t {
            let end = (start + CELL).min(max_t);
            let a = o.add(d.scale(start));
            let b = o.add(d.scale(end));
            let min = v3(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z));
            let max = v3(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z));
            self.each_near_indexed(min, max, |i, s| {
                if let Some(t) = s.ray(o, d, max_t) {
                    if best.is_none_or(|(b, _)| t < b) {
                        best = Some((t, i));
                    }
                }
            });
            if let Some((t, _)) = best {
                if t <= end {
                    break;
                }
            }
            start = end;
        }
        best.map(|(t, solid)| Hit {
            t,
            point: o.add(d.scale(t)),
            solid,
        })
    }

    /// The top of the highest solid under `(x, z)` at or below `y`, or `None`.
    pub fn floor_below(&self, x: i32, y: i32, z: i32) -> Option<i32> {
        let mut best: Option<i32> = None;
        let p = v3(x, y, z);
        self.each_near(p, p, |s| {
            if x >= s.min.x && x <= s.max.x && z >= s.min.z && z <= s.max.z && s.max.y <= y {
                best = Some(best.map_or(s.max.y, |b| b.max(s.max.y)));
            }
        });
        // a Bench is floor too, seen from above
        for d in self.dynamic.iter().filter(|d| d.on && d.one_way) {
            let s = d.solid;
            if x >= s.min.x && x <= s.max.x && z >= s.min.z && z <= s.max.z && s.max.y <= y {
                best = Some(best.map_or(s.max.y, |b| b.max(s.max.y)));
            }
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor_and_pillar() -> Level {
        Level::new(vec![
            Solid::new(v3(-int(20), -ONE, -int(20)), v3(int(20), 0, int(20))),
            Solid::stand(int(5), 0, 0, int(2), int(4), int(2)),
        ])
    }

    #[test]
    fn a_ray_down_meets_the_floor_and_a_ray_along_it_meets_the_pillar() {
        let l = floor_and_pillar();
        let down = l.ray(v3(0, int(3), 0), v3(0, -ONE, 0), int(10)).unwrap();
        assert_eq!(down.t, int(3));
        // the pillar's near face is at x = 4: from x = 0 the ray travels 4 m
        let along = l.ray(v3(0, int(1), 0), v3(ONE, 0, 0), int(30)).unwrap();
        assert!((along.t - int(4)).abs() < 4, "{}", along.t);
        // over the pillar the ray finds nothing within range
        assert!(l.ray(v3(0, int(5), 0), v3(ONE, 0, 0), int(30)).is_none());
    }

    #[test]
    fn a_long_ray_finds_the_nearest_hit_not_the_first_box_listed() {
        let l = Level::new(vec![
            Solid::stand(int(40), 0, 0, int(2), int(4), int(2)),
            Solid::stand(int(12), 0, 0, int(2), int(4), int(2)),
        ]);
        let h = l.ray(v3(0, int(1), 0), v3(ONE, 0, 0), int(100)).unwrap();
        assert!((h.t - int(11)).abs() < 4, "{}", h.t);
    }

    #[test]
    fn a_sphere_touching_a_face_hits_and_one_clear_of_it_does_not() {
        let l = floor_and_pillar();
        // pillar face at x = 4; a 0.5 m sphere centred at 3.6 touches it
        assert!(l.sphere_hits(v3(ONE * 36 / 10, int(2), 0), ONE / 2));
        assert!(!l.sphere_hits(v3(ONE * 34 / 10, int(2), 0), ONE / 2));
    }

    #[test]
    fn the_floor_below_a_point_is_the_highest_top_under_it() {
        let l = floor_and_pillar();
        assert_eq!(l.floor_below(int(5), int(10), 0), Some(int(4)));
        assert_eq!(l.floor_below(0, int(10), 0), Some(0));
        assert_eq!(l.floor_below(int(5), int(3), 0), Some(0));
    }

    #[test]
    fn a_solid_spanning_many_cells_is_visited_once_per_query() {
        let l = floor_and_pillar();
        let mut n = 0;
        l.each_near(v3(-int(15), 0, -int(15)), v3(int(15), ONE, int(15)), |_| n += 1);
        assert_eq!(n, 2);
        let mut m = 0;
        l.each_near(v3(int(9), 0, int(9)), v3(int(15), ONE, int(15)), |_| m += 1);
        assert_eq!(m, 1, "only the floor reaches that corner");
    }
}
