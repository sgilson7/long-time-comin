//! Chumbles, and the last mission's other things. A stub, to be drawn out.

use crate::content::{Content, EnemyDef, Rgb};
use crate::enemy::Enemy;
use crate::fx::{self, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{boxed, Batch};

/// Chumbles in the fight, at `at` (his body's centre at full size), as big
/// as `e.size` says; `fed` counts up once his last bar is empty (0 before).
pub fn chumbles(b: &mut Batch, c: &Content, e: &Enemy, d: &EnemyDef, at: V3, fed: i32) {
    let _ = fed;
    let s = fx::mul(d.radius * 2, e.size);
    let f = Affine::translate(at).then(&Affine::rot_y(e.yaw));
    let cream = c.pal.c("ground");
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(V3::ZERO, v3(s / 2, s / 3, s))),
        cream,
        0,
    );
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, s / 4, -s / 2), v3(s / 3, s / 3, s / 3))),
        cream,
        0,
    );
}

/// The cat in the bedroom, walking: at its feet `at`, facing `yaw`, its
/// legs `stride` round their step, `size` as for `chumbles`.
pub fn guide(b: &mut Batch, c: &Content, at: V3, yaw: i32, stride: i32, size: i32, radius: i32) {
    let _ = stride;
    let s = fx::mul(radius * 2, size);
    let f = Affine::translate(at.add(v3(0, s / 3, 0))).then(&Affine::rot_y(yaw));
    let cream = c.pal.c("ground");
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(V3::ZERO, v3(s / 2, s / 3, s))),
        cream,
        0,
    );
}

/// The last mission's lesser things (what Chumbles calls), by id; false
/// for anything else.
pub fn draw(
    b: &mut Batch,
    c: &Content,
    e: &Enemy,
    d: &EnemyDef,
    at: V3,
    s: i32,
    paint: &dyn Fn(Rgb) -> Rgb,
) -> bool {
    let _ = (e, s);
    if d.id != "yarn" {
        return false;
    }
    let red = paint(c.pal.c("ground"));
    b.inked(
        Mesh::Sphere,
        &boxed(at, v3(d.radius * 2, d.radius * 2, d.radius * 2)),
        red,
        0,
    );
    let _ = ONE;
    true
}
