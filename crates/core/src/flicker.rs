//! Flicker: two surfaces the page draws in one plane, facing one way, fight
//! for the depth buffer, and which of them shows changes from frame to frame
//! as the camera moves (z-fighting). This finds them in what the core hands
//! the page: the arena's static shapes and, for a course, the scenery drawn
//! each tick (decks, doors, waves), at a few ticks.
//!
//! A face is one triangle of a flat-sided mesh (box, wedge, panel, facade,
//! cylinder) drawn as a solid, placed. Two faces fight when they face the
//! same way, overlap by more than `MIN_AREA`, and differ in colour, glow or
//! pass (same-coloured faces in one plane fight invisibly), and their
//! planes are within `PLANE_TOL` of each other, or further apart than that
//! but too near for the depth buffer to tell apart from the nearest place a
//! camera comes (`Seen::apart`), short of the fog. And only where it can be
//! seen: no box presses on the fight from in front (a crate's bottom on the
//! floor, a panel's back a hand's breadth off its wall), a face looking down
//! has room under it for a camera and is not under the lowest place a body
//! stands, and some camera over a place a body stands has a line to it that
//! no solid cuts (the outside of a house's walls, the back of a building).
//! The places a body stands are the section's start, middle, gates, spawn
//! points and a course's posts: the walkthrough's camera, which flies over
//! the arenas, may see a little more.
//!
//! Ink and halos are left out: the vertex shader draws every ink hull a few
//! depth steps back and every halo a few forward (`INK_BIAS`, `HALO_BIAS` in
//! `data/shaders/scene.vert`), so a tie with a solid always goes the solid's
//! way, or the halo's.
//!
//! The arithmetic is integer: Q16.16 positions widened to i64, and i128 where
//! two products meet.

use crate::content::{Content, Rgb};
use crate::fx::ONE;
use crate::geom::{v3, Affine, V3};
use crate::mesh::{self, Mesh};
use crate::render::{self, Batch, Camera, DRAW_VALUES, INSTANCE_VALUES};
use crate::route::{Have, Route};
use crate::world::{Exit, World};
use std::collections::BTreeMap;

/// Planes nearer than this are one plane wherever they are seen from (two
/// millimetres: the rounding of shapes placed to lie in one plane); planes
/// further apart fight only where a camera is too far off to tell them
/// apart (`Seen::apart`).
pub const PLANE_TOL: i64 = (ONE / 500) as i64;
/// Overlaps smaller than this (a 10 cm square) are let be.
pub const MIN_AREA: i64 = (ONE as i64) * (ONE as i64) / 100;
/// The faces' normals within about a degree of each other.
const SAME_WAY: i64 = (ONE as i64) - (ONE as i64) / 5000;

#[derive(Debug, Clone)]
pub struct Inst {
    pub mesh: Mesh,
    pub at: Affine,
    pub rgb: Rgb,
    pub glow: i32,
    pub pass: u32,
    /// where it came from: "static" or "tick N"
    pub from: String,
}

/// The instances in a batch's finished values and draws.
pub fn instances(values: &[i32], draws: &[Vec<u32>], scenery_only: bool, from: &str) -> Vec<Inst> {
    let mut out = Vec::new();
    for list in draws {
        for d in list.chunks(DRAW_VALUES) {
            let (mesh, offset, count, pass, see) = (d[1], d[2], d[3], d[4], d[5]);
            if scenery_only && see == 0 {
                continue;
            }
            let first = offset as usize / render::BYTES_PER_VALUE;
            for k in 0..count as usize {
                let v = &values[first + k * INSTANCE_VALUES..first + (k + 1) * INSTANCE_VALUES];
                let at = Affine {
                    c0: v3(v[0], v[1], v[2]),
                    c1: v3(v[4], v[5], v[6]),
                    c2: v3(v[8], v[9], v[10]),
                    t: v3(v[3], v[7], v[11]),
                };
                out.push(Inst {
                    mesh: Mesh::ALL[mesh as usize],
                    at,
                    rgb: [v[12], v[13], v[14]],
                    glow: v[15],
                    pass,
                    from: from.to_string(),
                });
            }
        }
    }
    out
}

type P3 = [i64; 3];

#[derive(Debug, Clone)]
struct Face {
    inst: usize,
    /// the way it is drawn to, a unit vector in Q16
    n: P3,
    /// its plane's distance from the origin along `n`, Q16
    d: i64,
    pts: [P3; 3],
}

fn isqrt(v: u128) -> u128 {
    if v < 2 {
        return v;
    }
    let mut x = v;
    let mut y = x.div_ceil(2);
    while y < x {
        x = y;
        y = (x + v / x) / 2;
    }
    x
}

fn place(a: &Affine, p: (i32, i32, i32)) -> P3 {
    let m = |c: i32, k: i32| (c as i64 * k as i64) >> 16;
    [
        m(a.c0.x, p.0) + m(a.c1.x, p.1) + m(a.c2.x, p.2) + a.t.x as i64,
        m(a.c0.y, p.0) + m(a.c1.y, p.1) + m(a.c2.y, p.2) + a.t.y as i64,
        m(a.c0.z, p.0) + m(a.c1.z, p.1) + m(a.c2.z, p.2) + a.t.z as i64,
    ]
}

fn sub(a: P3, b: P3) -> P3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: P3, b: P3) -> [i128; 3] {
    let (a, b) = (a.map(|x| x as i128), b.map(|x| x as i128));
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Flat-sided meshes: their faces can lie in one plane with another's.
fn flat(m: Mesh) -> bool {
    matches!(
        m,
        Mesh::Cube | Mesh::Wedge | Mesh::Ground | Mesh::Panel | Mesh::Facade | Mesh::Cyl
    )
}

fn faces(insts: &[Inst]) -> Vec<Face> {
    let mut meshes: BTreeMap<Mesh, Vec<i32>> = BTreeMap::new();
    let mut out = Vec::new();
    for (i, s) in insts.iter().enumerate() {
        if !flat(s.mesh) || !solid(s) {
            continue;
        }
        let vs = meshes.entry(s.mesh).or_insert_with(|| mesh::vertices(s.mesh));
        let stride = mesh::VERTEX_VALUES;
        for t in vs.chunks(stride * 3) {
            let p = |k: usize| place(&s.at, (t[k * stride], t[k * stride + 1], t[k * stride + 2]));
            let pts = [p(0), p(1), p(2)];
            let c = cross(sub(pts[1], pts[0]), sub(pts[2], pts[0]));
            let len = isqrt((c[0] * c[0] + c[1] * c[1] + c[2] * c[2]) as u128) as i128;
            // a sliver (a shape flat along one axis shows its edge): nothing
            // to see, and no area to clip by
            if len < SLIVER {
                continue;
            }
            let n = c.map(|x| (x * ONE as i128 / len) as i64);
            let d = (n[0] as i128 * pts[0][0] as i128
                + n[1] as i128 * pts[0][1] as i128
                + n[2] as i128 * pts[0][2] as i128)
                >> 16;
            out.push(Face {
                inst: i,
                n,
                d: d as i64,
                pts,
            });
        }
    }
    out
}

type P2 = [i64; 2];

/// A triangle smaller than this (twice its area, Q32: half a square
/// centimetre) is a sliver.
const SLIVER: i128 = (ONE as i128) * (ONE as i128) / 10_000;

/// Steps of the depth buffer two planes need between them to be told apart.
const DEPTH_STEPS: i128 = 4;

/// How far in front of a fight to look for a box it is pressed against.
const PROBE: i64 = (ONE / 200) as i64;

/// A face looking into a box this near is in a crevice no camera sees into
/// (a panel's back, a hand's breadth off the wall it hangs on).
const CREVICE: i64 = (ONE * 6 / 100) as i64;

/// A face looking down this near a floor has no room under it for a camera.
const UNDER: i64 = (ONE * 3 / 10) as i64;

/// Drawn as a solid: not an ink hull, not a halo.
fn solid(s: &Inst) -> bool {
    s.pass != render::PASS_INK && s.pass != render::PASS_GLOW
}

/// A closed solid that hides what is pressed against it.
fn closed(s: &Inst) -> bool {
    solid(s) && matches!(s.mesh, Mesh::Cube | Mesh::Cyl | Mesh::Sphere | Mesh::Blob)
}

/// Is `p` inside the shape `s` (its columns at right angles)?
fn inside(s: &Inst, p: P3) -> bool {
    let a = &s.at;
    let rel = [p[0] - a.t.x as i64, p[1] - a.t.y as i64, p[2] - a.t.z as i64];
    // the point in the unit mesh's frame, Q16
    let mut local = [0i128; 3];
    for (k, c) in [a.c0, a.c1, a.c2].iter().enumerate() {
        let c = [c.x as i128, c.y as i128, c.z as i128];
        let len2 = c[0] * c[0] + c[1] * c[1] + c[2] * c[2];
        if len2 == 0 {
            return false;
        }
        let dot = c[0] * rel[0] as i128 + c[1] * rel[1] as i128 + c[2] * rel[2] as i128;
        local[k] = dot * ONE as i128 / len2;
    }
    let half = (ONE / 2) as i128;
    let quarter = half * half;
    match s.mesh {
        Mesh::Cube => local.iter().all(|l| l.abs() < half),
        Mesh::Cyl => local[1].abs() < half && local[0] * local[0] + local[2] * local[2] < quarter,
        _ => local.iter().map(|l| l * l).sum::<i128>() < quarter,
    }
}

/// The triangle in the plane's 2-D projection (the normal's largest axis
/// dropped), turned counter-clockwise.
fn flatten(f: &Face, drop: usize) -> Vec<P2> {
    let (u, v) = match drop {
        0 => (1, 2),
        1 => (2, 0),
        _ => (0, 1),
    };
    let mut p: Vec<P2> = f.pts.iter().map(|q| [q[u], q[v]]).collect();
    if area2(&p) < 0 {
        p.reverse();
    }
    p
}

fn area2(p: &[P2]) -> i128 {
    let mut s = 0i128;
    for k in 0..p.len() {
        let (a, b) = (p[k], p[(k + 1) % p.len()]);
        s += a[0] as i128 * b[1] as i128 - b[0] as i128 * a[1] as i128;
    }
    s
}

/// Clip the convex polygon `subject` by the convex, counter-clockwise `clip`.
fn clip(subject: Vec<P2>, clip: &[P2]) -> Vec<P2> {
    let mut out = subject;
    for k in 0..clip.len() {
        if out.is_empty() {
            break;
        }
        let (a, b) = (clip[k], clip[(k + 1) % clip.len()]);
        let side = |p: P2| -> i128 {
            (b[0] - a[0]) as i128 * (p[1] - a[1]) as i128 - (b[1] - a[1]) as i128 * (p[0] - a[0]) as i128
        };
        let input = std::mem::take(&mut out);
        for i in 0..input.len() {
            let (p, q) = (input[i], input[(i + 1) % input.len()]);
            let (sp, sq) = (side(p), side(q));
            if sp >= 0 {
                out.push(p);
            }
            if (sp >= 0) != (sq >= 0) {
                let t = |x: i64, y: i64| x + ((y - x) as i128 * sp / (sp - sq)) as i64;
                out.push([t(p[0], q[0]), t(p[1], q[1])]);
            }
        }
    }
    out
}

/// The area each pair of instances shares in one plane, its largest piece,
/// where that is and the way it faces.
type Shared = BTreeMap<(usize, usize), (i64, i64, V3, [i64; 3])>;

/// One fight: two instances, the area they share in one plane (Q32 m²).
#[derive(Debug, Clone)]
pub struct Fight {
    /// the way the two faces look, Q16
    pub n: [i64; 3],
    pub a: usize,
    pub b: usize,
    pub area: i64,
    /// a point in the shared area, Q16
    pub at: V3,
}

/// Where a section is seen from, for what a camera there can tell apart.
#[derive(Debug, Clone)]
pub struct Seen {
    /// the world's lowest floor: nothing looks up from under it
    pub bottom: i32,
    /// where a body can stand (its camera is `CAMERA_BACK` off); none: the
    /// planes are judged as if from near by
    pub eyes: Vec<V3>,
    /// past this the fog has everything
    pub fog_far: i32,
    /// the solids a camera cannot see through, if known: a fight none of
    /// the eyes has a clear line to (the outside of a house's walls, the
    /// far side of a block of buildings) is never seen
    pub level: Option<crate::level::Level>,
}

impl Seen {
    /// From near by, with nothing under the world.
    pub fn near() -> Seen {
        Seen {
            bottom: i32::MIN,
            eyes: Vec::new(),
            fog_far: 0,
            level: None,
        }
    }

    /// Can a camera over any eye see `p` (a point just in front of a face)?
    fn sees(&self, p: P3) -> bool {
        let Some(level) = &self.level else {
            return true;
        };
        let p = v3(p[0] as i32, p[1] as i32, p[2] as i32);
        // a solid only hides what lies well behind it: a face may sit inside
        // a collision box that is never drawn (a pylon's, a bound's)
        let behind = ONE / 2;
        self.eyes.iter().any(|e| {
            let o = e.add(v3(0, crate::balance::CAMERA_UP, 0));
            let to = p.sub(o);
            let len = to.len();
            len <= behind || level.ray(o, to.norm(), len - behind).is_none()
        })
    }

    /// How far apart two parallel planes must be to be told apart at `d`
    /// from the lens: `DEPTH_STEPS` steps of a 24-bit depth buffer there.
    fn apart(d: i64) -> i64 {
        let d = d as i128;
        (DEPTH_STEPS * d * d / (render::NEAR as i128 * (1i128 << 24))) as i64
    }

    /// The nearest a camera comes to `p`, Q16.
    fn reach(&self, p: P3) -> i64 {
        let near = self
            .eyes
            .iter()
            .map(|e| {
                let d = [p[0] - e.x as i64, p[1] - e.y as i64, p[2] - e.z as i64];
                isqrt(
                    (d[0] as i128 * d[0] as i128 + d[1] as i128 * d[1] as i128 + d[2] as i128 * d[2] as i128)
                        as u128,
                ) as i64
            })
            .min()
            .unwrap_or(0);
        (near - crate::balance::CAMERA_BACK as i64).max(0)
    }
}

/// Every pair of instances that fight, with the area they share.
pub fn fights(insts: &[Inst], seen: &Seen) -> Vec<Fight> {
    // nothing looks up from under the lowest floor, nor from under the
    // lowest place a body stands
    let bottom = seen
        .eyes
        .iter()
        .map(|e| e.y)
        .min()
        .map_or(seen.bottom, |low| low.max(seen.bottom));
    // the widest gap the depth buffer cannot tell apart, short of the fog
    let window = if seen.eyes.is_empty() {
        PLANE_TOL
    } else {
        PLANE_TOL.max(Seen::apart(seen.fog_far as i64))
    };
    let all = faces(insts);
    let boxes: Vec<usize> = (0..insts.len()).filter(|&k| closed(&insts[k])).collect();
    let grounds: Vec<usize> = (0..insts.len())
        .filter(|&k| insts[k].mesh == Mesh::Ground && insts[k].at.c1.y > 0)
        .collect();
    let q = |x: i64| (x * 64 + (ONE as i64) / 2).div_euclid(ONE as i64);
    let mut buckets: BTreeMap<(i64, i64, i64), Vec<usize>> = BTreeMap::new();
    for (k, f) in all.iter().enumerate() {
        buckets
            .entry((q(f.n[0]), q(f.n[1]), q(f.n[2])))
            .or_default()
            .push(k);
    }
    let mut shared: Shared = BTreeMap::new();
    for list in buckets.values_mut() {
        list.sort_by_key(|&k| all[k].d);
        for x in 0..list.len() {
            let f = &all[list[x]];
            for &g_at in &list[x + 1..] {
                let g = &all[g_at];
                let gap = g.d - f.d;
                if gap > window {
                    break;
                }
                if f.inst == g.inst {
                    continue;
                }
                let (si, sj) = (&insts[f.inst], &insts[g.inst]);
                if si.rgb == sj.rgb && si.glow == sj.glow && si.pass == sj.pass {
                    continue;
                }
                let dot = (f.n[0] as i128 * g.n[0] as i128
                    + f.n[1] as i128 * g.n[1] as i128
                    + f.n[2] as i128 * g.n[2] as i128)
                    >> 16;
                if (dot as i64) < SAME_WAY {
                    continue;
                }
                let drop = (0..3).max_by_key(|&a| f.n[a].abs()).unwrap_or(1);
                let (pa, pb) = (flatten(f, drop), flatten(g, drop));
                let lo = |p: &[P2], a: usize| p.iter().map(|q| q[a]).min().unwrap_or(0);
                let hi = |p: &[P2], a: usize| p.iter().map(|q| q[a]).max().unwrap_or(0);
                if (0..2).any(|a| lo(&pa, a) >= hi(&pb, a) || lo(&pb, a) >= hi(&pa, a)) {
                    continue;
                }
                let common = clip(pa, &pb);
                if common.len() < 3 {
                    continue;
                }
                // the projected area, back to the plane's own
                let a2 = area2(&common);
                let area = (a2 * ONE as i128 / (2 * f.n[drop].abs().max(1) as i128)) as i64;
                if area <= 0 {
                    continue;
                }
                let c = common.len() as i64;
                let mid2 = [
                    common.iter().map(|p| p[0]).sum::<i64>() / c,
                    common.iter().map(|p| p[1]).sum::<i64>() / c,
                ];
                // back to 3-D: the dropped axis from the plane
                let mut p = [0i64; 3];
                let (u, v) = match drop {
                    0 => (1, 2),
                    1 => (2, 0),
                    _ => (0, 1),
                };
                p[u] = mid2[0];
                p[v] = mid2[1];
                let rest = (f.n[u] as i128 * p[u] as i128 + f.n[v] as i128 * p[v] as i128) >> 16;
                p[drop] = ((f.d as i128 - rest) * ONE as i128 / f.n[drop] as i128) as i64;
                // planes apart: fighting only where a camera is too far off
                // to tell them apart, and short of the fog
                if gap > PLANE_TOL {
                    let d = seen.reach(p);
                    if d >= seen.fog_far as i64 || gap >= Seen::apart(d) {
                        continue;
                    }
                }
                // a fight pressed against a box, in front of both faces, is
                // never seen (two shapes' bottoms on the floor)
                let front = [0, 1, 2].map(|a| p[a] + f.n[a] * (gap + PROBE) / ONE as i64);
                let deep = [0, 1, 2].map(|a| p[a] + f.n[a] * (gap + CREVICE) / ONE as i64);
                if boxes
                    .iter()
                    .any(|&k| inside(&insts[k], front) || inside(&insts[k], deep))
                {
                    continue;
                }
                if !seen.sees(front) {
                    continue;
                }
                // a fight facing down within a body's breadth of a floor
                // under it is seen by no camera
                let down = f.n[1] < -(ONE as i64) * 9 / 10;
                if down && p[1] <= bottom as i64 + PLANE_TOL {
                    continue;
                }
                if down {
                    let under = [p[0], p[1] - UNDER, p[2]];
                    let floored = boxes.iter().any(|&k| inside(&insts[k], under))
                        || grounds.iter().any(|&k| {
                            let a = &insts[k].at;
                            let y = a.t.y as i64;
                            y >= under[1]
                                && y <= p[1]
                                && 2 * (p[0] - a.t.x as i64).abs() < a.c0.x.abs() as i64
                                && 2 * (p[2] - a.t.z as i64).abs() < a.c2.z.abs() as i64
                        });
                    if floored {
                        continue;
                    }
                }
                let key = (f.inst.min(g.inst), f.inst.max(g.inst));
                let e = shared.entry(key).or_insert((0, 0, V3::ZERO, f.n));
                e.0 += area;
                // told by its largest piece
                if area > e.1 {
                    e.1 = area;
                    e.2 = v3(p[0] as i32, p[1] as i32, p[2] as i32);
                    e.3 = f.n;
                }
            }
        }
    }
    shared
        .into_iter()
        .filter(|(_, (area, _, _, _))| *area > MIN_AREA)
        .map(|((a, b), (area, _, at, n))| Fight { n, a, b, area, at })
        .collect()
}

/// A section as the page draws it: the static shapes, and for a course
/// the scenery drawn each tick at `ticks` (bodies, orbs and pickups left
/// out: they move, and pass through scenery as they please).
pub fn section(c: &Content, w: &World, ticks: &[i32]) -> Vec<Vec<Inst>> {
    let mut b = Batch::new([0; 3]);
    for s in &w.arena.shapes {
        b.shape(s);
    }
    let (values, draws) = b.finish(render::STATIC_BUFFER);
    let fixed = instances(&values, &draws, false, "static");
    let mut w = w.clone();
    let mut out = Vec::new();
    let eye = w.player.chest();
    let fwd = crate::geom::facing(w.player.yaw, 0);
    let mut at = 0;
    for &t in ticks {
        while at < t {
            w.tick(c, &crate::player::Controls::default(), eye, fwd);
            at += 1;
        }
        let mut v = w.clone();
        v.enemies.clear();
        v.orbs.clear();
        v.shots.clear();
        v.pickups.clear();
        v.effects.clear();
        let mut b = Batch::new([0; 3]);
        let cam = Camera {
            eye,
            fwd,
            fov: crate::fx::deg(70),
        };
        crate::draw::world(&mut b, c, &v, ONE, at, false, &cam);
        let (values, draws) = b.finish(render::DYNAMIC_BUFFER);
        let mut all = fixed.clone();
        all.extend(instances(&values, &draws, false, &format!("tick {at}")));
        out.push(all);
    }
    out
}

/// Every section of every built zone, as the routes lay them: the zone, the
/// section's archetype, and its World.
pub fn sections(c: &Content) -> Vec<(String, String, World)> {
    let mut out = Vec::new();
    for (zi, z) in c.zones.iter().enumerate() {
        if !z.built {
            continue;
        }
        // every archetype, from a few routes; the River's zones (the newest,
        // and the ones that flickered) from more, each route's own
        let river = !z.branch_from.is_empty();
        let seeds: &[u64] = if river {
            &[7, 11, 23, 101, 202, 303, 404, 505, 606]
        } else {
            &[7, 11, 23]
        };
        let mut known: Vec<String> = Vec::new();
        for &seed in seeds {
            if river {
                known.clear();
            }
            let r = Route::build_for(c, zi, seed, &Have::level(1));
            for node in &r.nodes {
                if known.contains(&node.archetype) {
                    continue;
                }
                known.push(node.archetype.clone());
                let exits: Vec<Exit> = node
                    .next
                    .iter()
                    .map(|&to| Exit {
                        to,
                        main: true,
                        reward: None,
                        sealed: false,
                    })
                    .collect();
                let p = crate::player::Player::new(
                    V3::ZERO,
                    0,
                    crate::player::Build {
                        max_health: 1_000_000,
                        ..crate::course::tests::river_kit()
                    },
                );
                let w = World::new(c, node, &exits, p, crate::weapons::Held::new(0, 1), 3);
                out.push((z.id.clone(), node.archetype.clone(), w));
            }
        }
    }
    out
}

/// Where a section is seen from: where a body can stand in it (the start,
/// the middle, the gates, the spawn points, a course's posts), the fog its
/// scene is lit with, and its lowest solid.
pub fn seen(c: &Content, w: &World, zone: &str, arch: &str) -> Seen {
    let a = &w.arena;
    let mut eyes = vec![a.start, a.centre];
    eyes.extend(a.gates.iter().map(|g| g.0));
    eyes.extend(a.ground_spawns.iter().copied());
    if let Some(k) = &w.course {
        eyes.extend(k.posts.iter().copied());
        eyes.push(k.finish);
    }
    let zone = c.zones.iter().find(|z| z.id == zone).expect("the zone");
    let scene = zone.scenes.get(arch).unwrap_or(&zone.scene);
    Seen {
        bottom: a.level.solids.iter().map(|s| s.min.y).min().unwrap_or(0),
        eyes,
        fog_far: crate::fx::int(scene.fog_far_m),
        level: Some(a.level.clone()),
    }
}

/// One line per fight: what fights what, where, and how much.
pub fn describe(insts: &[Inst], f: &Fight) -> String {
    let cm = |x: i32| x as i64 * 100 / ONE as i64;
    let one = |s: &Inst| {
        let a = &s.at;
        format!(
            "{:?} #{} at ({}, {}, {}) size ({}, {}, {}) [{}]",
            s.mesh,
            crate::content::hex(s.rgb).trim_start_matches('#'),
            cm(a.t.x),
            cm(a.t.y),
            cm(a.t.z),
            cm(a.c0.len()),
            cm(a.c1.len()),
            cm(a.c2.len()),
            s.from
        )
    };
    format!(
        "{} vs {}: {} cm² at ({}, {}, {}) facing {:?}",
        one(&insts[f.a]),
        one(&insts[f.b]),
        (f.area as i128 * 10_000) >> 32,
        cm(f.at.x),
        cm(f.at.y),
        cm(f.at.z),
        f.n.map(|x| x * 100 / ONE as i64),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;
    use crate::fx::int;

    fn cube(at: V3, size: V3, rgb: Rgb, pass: u32) -> Inst {
        Inst {
            mesh: Mesh::Cube,
            at: render::boxed(at, size),
            rgb,
            glow: 0,
            pass,
            from: "test".into(),
        }
    }

    #[test]
    fn two_boxes_whose_tops_meet_in_one_plane_fight_and_a_lifted_one_does_not() {
        let floor = cube(v3(0, -ONE / 2, 0), v3(int(4), ONE, int(4)), [ONE, 0, 0], 0);
        // a rug flush with the floor's top
        let rug = cube(v3(0, -ONE / 20, 0), v3(int(2), ONE / 10, int(2)), [0, ONE, 0], 0);
        let f = fights(&[floor.clone(), rug], &Seen::near());
        assert_eq!(f.len(), 1, "a rug flush with the floor fights it: {f:?}");
        // the overlap is the rug's top, 2 m by 2 m
        assert!(
            (f[0].area - int(4) as i64 * ONE as i64).abs() < ONE as i64,
            "{f:?}"
        );
        let lifted = cube(
            v3(0, -ONE / 20 + ONE / 50, 0),
            v3(int(2), ONE / 10, int(2)),
            [0, ONE, 0],
            0,
        );
        assert!(
            fights(&[floor, lifted], &Seen::near()).is_empty(),
            "two centimetres up, it does not"
        );
    }

    #[test]
    fn a_side_in_a_walls_side_fights_and_one_colour_or_a_buried_face_does_not() {
        let wall = cube(v3(0, 0, 0), v3(int(2), int(2), int(2)), [ONE, 0, 0], 0);
        // a band round the wall, its ends in the wall's sides
        let band = cube(v3(0, 0, 0), v3(int(2), ONE / 5, int(3)), [0, 0, ONE], 0);
        let f = fights(&[wall.clone(), band.clone()], &Seen::near());
        assert_eq!(f.len(), 1, "the band's ends lie in the wall's sides: {f:?}");
        // a halo there is the shader's to settle (drawn a few steps forward)
        let halo = Inst {
            pass: render::PASS_GLOW,
            ..band
        };
        assert!(
            fights(&[wall.clone(), halo], &Seen::near()).is_empty(),
            "a halo in a wall's face is the shader's to settle"
        );
        let twin = cube(v3(0, ONE / 2, 0), v3(int(2), int(1), int(2)), [ONE, 0, 0], 0);
        assert!(
            fights(&[wall.clone(), twin], &Seen::near()).is_empty(),
            "the same colour in one plane shows no fight"
        );
        // two crates' backs against a wall, in one plane, face into it
        let back = cube(v3(ONE / 2, int(2), 0), v3(ONE, int(4), int(9)), [0, ONE, 0], 0);
        let a = cube(v3(-ONE, ONE, 0), v3(int(2), int(2), int(2)), [ONE, ONE, 0], 0);
        let b = cube(v3(-ONE / 2, ONE * 3 / 4, 0), v3(ONE, ONE, ONE), [0, ONE, ONE], 0);
        assert!(
            !fights(&[a.clone(), b.clone()], &Seen::near()).is_empty(),
            "alone, their backs fight"
        );
        assert!(
            fights(&[back, a, b], &Seen::near()).is_empty(),
            "backs pressed on a wall are never seen"
        );
    }

    #[test]
    fn the_depth_buffer_tells_planes_three_centimetres_apart_two_hundred_metres_off() {
        // the near plane sets it: at 0.2 m it took 5 cm
        let d = Seen::apart(int(200) as i64);
        assert!(d < (ONE * 3 / 100) as i64, "{} cm", d * 100 / ONE as i64);
    }

    #[test]
    fn the_shader_draws_ink_back_and_halos_forward_of_a_tie() {
        let vert = include_str!("../../../data/shaders/scene.vert");
        assert!(vert.contains("if (uPass == INK) gl_Position.z += INK_BIAS * gl_Position.w;"));
        assert!(vert.contains("if (uPass == GLOW) gl_Position.z -= HALO_BIAS * gl_Position.w;"));
        assert_eq!(render::PASS_INK, 2, "the shader's INK");
        assert_eq!(render::PASS_GLOW, 3, "the shader's GLOW");
    }

    /// Sections left out, each with why. The Eraser's court (the Vault's
    /// boss, `arena/vault/eraser.rs`) is being rebuilt elsewhere; when this
    /// was written one fight stood there, a pylon's chrome post with a side
    /// in the plane of its black glass base's (151 cm², facing south-west).
    const LEFT_OUT: &[(&str, &str)] = &[("vault", "boss")];

    /// Every section of every built zone: what fights, counted.
    #[test]
    fn no_section_of_any_zone_draws_two_surfaces_in_one_plane() {
        let c = content();
        let mut report = Vec::new();
        let mut total = 0;
        for (zone, arch, w) in sections(&c) {
            if LEFT_OUT.contains(&(zone.as_str(), arch.as_str())) {
                continue;
            }
            let ticks: &[i32] = if w.course.is_some() { &[0, 45, 120] } else { &[0] };
            let seen = seen(&c, &w, &zone, &arch);
            let mut known: Vec<(String, String)> = Vec::new();
            let mut n = 0;
            for insts in section(&c, &w, ticks) {
                for f in fights(&insts, &seen) {
                    let d = describe(&insts, &f);
                    let key = (format!("{:?}", insts[f.a].at), format!("{:?}", insts[f.b].at));
                    if known.contains(&key) {
                        continue;
                    }
                    known.push(key);
                    n += 1;
                    if n <= 6 {
                        report.push(format!("  {zone}/{arch}: {d}"));
                    }
                }
            }
            total += n;
            report.push(format!("{zone}/{arch}: {n} pairs"));
        }
        assert!(total == 0, "{total} pairs fight:\n{}", report.join("\n"));
    }
}
