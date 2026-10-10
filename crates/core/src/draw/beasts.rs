//! The enemies added after zone one's first six: the Clerk of Minutes,
//! zone two's seven, and the Witness. Each is built from the same meshes
//! as the rest, in its zone's colours: zone one's plum shells and magenta
//! eyes, zone two's navy shells, ember eyes and cobalt cores, so they read
//! against white ice.

use super::rod;
use crate::content::{Content, EnemyDef, Rgb};
use crate::enemy::{Enemy, State};
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{boxed, shade, Batch, PASS_SOLID};

/// Draws the enemy if it is one of these; false if it is not.
pub fn draw(
    b: &mut Batch,
    c: &Content,
    e: &Enemy,
    d: &EnemyDef,
    at: V3,
    s: i32,
    paint: &dyn Fn(Rgb) -> Rgb,
) -> bool {
    match d.id.as_str() {
        "clerk" => clerk(b, c, e, at, s, paint),
        "cairn" => cairn(b, c, e, d, at, s, paint),
        "rime_kite" => kite(b, c, e, at, s, paint),
        "hauler" => hauler(b, c, e, at, s, paint),
        "shardling" => shardling(b, c, e, at, s, paint),
        "auger" => auger(b, c, e, at, s, paint),
        "prism" => prism(b, c, e, at, s, paint),
        "aurora" => aurora(b, c, e, at, s, paint),
        "witness" => witness(b, c, e, at, s, paint),
        _ => return false,
    }
    true
}

fn f_of(at: V3, yaw: i32, s: i32) -> Affine {
    Affine::translate(at)
        .then(&Affine::rot_y(yaw))
        .then(&Affine::scale(v3(s, s, s)))
}

fn u(x: i32) -> i32 {
    ONE * x / 100
}

/// The Clerk of Minutes: a hooded figure that hovers over an open ledger,
/// a quill in its hand, a small clock on its back.
fn clerk(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let shell = paint(c.pal.c("enemy_shell"));
    let plate = paint(c.pal.c("enemy_plate"));
    let paper = c.pal.c("ledger");
    let eye = c.pal.c("enemy_eye");
    let f = f_of(at, e.yaw, s);
    b.inked(
        Mesh::Cone,
        &f.then(&boxed(v3(0, -u(5), 0), v3(u(70), u(110), u(70)))),
        shell,
        0,
    );
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, u(48), 0), v3(u(42), u(42), u(42)))),
        plate,
        0,
    );
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, u(46), -u(17)), v3(u(16), u(10), u(8)))),
        eye,
        ONE,
    );
    // the ledger, open, tilted toward you; its pages turn
    let turn = fx::sin(e.age * fx::TURN / 50) / 6;
    for side in [-1, 1] {
        let page = f
            .then(&Affine::translate(v3(side * u(18), u(5), -u(48))))
            .then(&Affine::rot_x(-deg(35)))
            .then(&Affine::rot_z(side * (deg(12) + turn)))
            .then(&Affine::scale(v3(u(34), u(3), u(44))));
        b.inked(Mesh::Cube, &page, paper, ONE / 5);
    }
    // the quill
    let hand = f.apply(v3(u(30), u(18), -u(30)));
    let tip = hand.add(f.apply_dir(v3(0, u(40), 0)));
    rod(b, hand, tip, s / 30, c.pal.c("ledger"), 0, true);
    // the clock on its back, stopped at 21:14
    let disc = f
        .then(&Affine::translate(v3(0, u(30), u(32))))
        .then(&Affine::rot_x(fx::QUARTER))
        .then(&Affine::scale(v3(u(40), u(5), u(40))));
    b.inked(Mesh::Cyl, &disc, c.pal.c("clock"), ONE / 4);
}

/// A Cairn: stones stacked from the floor, an ember eye in the top one,
/// crystals turning round it.
fn cairn(b: &mut Batch, c: &Content, e: &Enemy, d: &EnemyDef, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let shell = paint(c.pal.c("frost_shell"));
    let plate = paint(c.pal.c("frost_plate"));
    let ice = c.pal.c("frost_ice");
    let foot = at.sub(v3(0, d.hover + d.radius / 2, 0));
    // the stack from the floor to the body
    let n = 4;
    for k in 0..n {
        let y = fx::lerp(foot.y, at.y - s / 3, fx::div(k * ONE, n * ONE));
        let w = s * (10 - k) / 12;
        let rgb = if k % 2 == 0 { shell } else { plate };
        let stone = Affine::translate(v3(at.x, y + s / 8, at.z))
            .then(&Affine::rot_y(deg(23) * k))
            .then(&Affine::scale(v3(w, s / 4, w * 9 / 10)));
        b.inked(Mesh::Octa, &stone, rgb, 0);
    }
    let f = f_of(at, e.yaw, s);
    b.inked(
        Mesh::Octa,
        &f.then(&boxed(V3::ZERO, v3(u(80), u(70), u(80)))),
        shell,
        0,
    );
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, u(5), -u(34)), v3(u(26), u(26), u(14)))),
        c.pal.c("frost_eye"),
        ONE,
    );
    for k in 0..5 {
        let a = e.age * deg(2) + fx::TURN / 5 * k;
        let p = at.add(v3(
            fx::mul(fx::cos(a), s * 7 / 10),
            fx::mul(fx::sin(a * 2), s / 6),
            fx::mul(fx::sin(a), s * 7 / 10),
        ));
        b.inked(Mesh::Octa, &boxed(p, v3(s / 8, s / 4, s / 8)), ice, ONE / 3);
    }
}

/// A Rime Kite: a diamond of ice on the wind, wings that flex, a tail.
fn kite(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let shell = paint(c.pal.c("frost_shell"));
    let plate = paint(c.pal.c("frost_plate"));
    let ice = c.pal.c("frost_ice");
    let f = f_of(at, e.yaw, s);
    let body = f
        .then(&Affine::rot_x(fx::QUARTER))
        .then(&boxed(V3::ZERO, v3(u(70), u(110), u(18))));
    b.inked(Mesh::Octa, &body, plate, 0);
    let flex = fx::sin(e.age * fx::TURN / 40) / 5;
    for side in [-1, 1] {
        let wing = f
            .then(&Affine::translate(v3(side * u(55), 0, u(5))))
            .then(&Affine::rot_z(side * flex))
            .then(&Affine::scale(v3(u(70), u(4), u(40))));
        b.inked(Mesh::Wedge, &wing, shell, 0);
    }
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, u(4), -u(26)), v3(u(22), u(22), u(14)))),
        c.pal.c("frost_eye"),
        ONE,
    );
    // the tail: bows of ice on a line, swinging
    let mut p = f.apply(v3(0, 0, u(50)));
    for k in 1..5 {
        let sway = fx::sin(e.age * fx::TURN / 60 + k * deg(50));
        let q = p.add(f.apply_dir(v3(fx::mul(sway, u(12)), -u(10), u(28))));
        rod(b, p, q, s / 40, shade(shell, 120), 0, false);
        b.inked(Mesh::Octa, &boxed(q, v3(s / 10, s / 14, s / 10)), ice, ONE / 4);
        p = q;
    }
}

/// A Hauler: a heavy beast of the ice on six legs, a plough on its brow.
fn hauler(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let shell = paint(c.pal.c("frost_shell"));
    let plate = paint(c.pal.c("frost_plate"));
    let ice = c.pal.c("frost_ice");
    let f = f_of(at, e.yaw, s);
    let step = e.age * fx::TURN / 36;
    for k in 0..6 {
        let side = if k % 2 == 0 { -1 } else { 1 };
        let z = u(-35) + u(35) * (k / 2);
        let lift = fx::sin(step + fx::HALF * (k % 2) + deg(60) * (k / 2)).max(0) / 5;
        let hip = f.apply(v3(side * u(32), -u(12), z));
        let foot = f.apply(v3(side * u(52), -u(50) + lift, z));
        rod(b, hip, foot, s / 8, shade(shell, 85), 0, true);
    }
    b.inked(
        Mesh::Cube,
        &f.then(&boxed(V3::ZERO, v3(u(78), u(52), u(100)))),
        shell,
        0,
    );
    // plates down its back, frost on them
    for k in 0..3 {
        let plate_at = f
            .then(&Affine::translate(v3(0, u(30), u(-25) + u(25) * k)))
            .then(&Affine::rot_x(-deg(20)))
            .then(&Affine::scale(v3(u(70), u(14), u(26))));
        b.inked(Mesh::Wedge, &plate_at, plate, 0);
        b.push(
            Mesh::Cube,
            &f.then(&boxed(v3(0, u(38), u(-25) + u(25) * k), v3(u(50), u(3), u(14)))),
            ice,
            ONE / 4,
            PASS_SOLID,
        );
    }
    // the plough
    let blade = f.then(
        &Affine::translate(v3(0, -u(8), -u(58)))
            .then(&Affine::rot_y(fx::HALF))
            .then(&Affine::scale(v3(u(96), u(50), u(26)))),
    );
    b.inked(Mesh::Wedge, &blade, plate, 0);
    b.inked(
        Mesh::Cube,
        &f.then(&boxed(v3(0, u(14), -u(52)), v3(u(56), u(10), u(6)))),
        c.pal.c("frost_eye"),
        ONE,
    );
}

/// A Shardling: a splinter of ice with an ember heart, tumbling.
fn shardling(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let ice = paint(c.pal.c("frost_ice"));
    let spin = e.age * deg(9);
    let body = Affine::translate(at)
        .then(&Affine::rot_y(spin))
        .then(&Affine::rot_z(deg(30)))
        .then(&Affine::scale(v3(s * 6 / 10, s * 12 / 10, s * 6 / 10)));
    b.inked(Mesh::Octa, &body, ice, ONE / 6);
    b.push(
        Mesh::Sphere,
        &boxed(at, v3(s / 3, s / 3, s / 3)),
        c.pal.c("frost_eye"),
        ONE,
        PASS_SOLID,
    );
    for k in 0..3 {
        let a = spin * 2 + fx::TURN / 3 * k;
        let p = at.add(v3(
            fx::mul(fx::cos(a), s * 7 / 10),
            0,
            fx::mul(fx::sin(a), s * 7 / 10),
        ));
        b.inked(Mesh::Octa, &boxed(p, v3(s / 6, s / 3, s / 6)), shade(ice, 90), 0);
    }
}

/// An Auger: under the ice a ring of cracks that follows you; above it a
/// drill-headed worm, its ember eyes in a ring.
fn auger(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let shell = paint(c.pal.c("frost_shell"));
    let plate = paint(c.pal.c("frost_plate"));
    let eye = c.pal.c("frost_eye");
    if e.buried {
        // the ice over it heaves; brighter as it is about to come up
        let ground = at.y + crate::fx::int(3);
        let soon = matches!(e.state, State::Rest(t) if t < 30);
        let pulse = ONE / 3 + fx::sin(e.age * deg(12)) / 6;
        let ring = boxed(v3(at.x, ground + u(5), at.z), v3(s * 3 / 2, u(10), s * 3 / 2));
        b.push(
            Mesh::Ring,
            &ring,
            if soon { eye } else { c.pal.c("frost_ice") },
            if soon { ONE } else { pulse },
            PASS_SOLID,
        );
        for k in 0..5 {
            let a = fx::TURN / 5 * k + e.age * deg(3);
            let p = v3(
                at.x + fx::mul(fx::cos(a), s * 6 / 10),
                ground + u(10),
                at.z + fx::mul(fx::sin(a), s * 6 / 10),
            );
            b.inked(
                Mesh::Octa,
                &boxed(p, v3(s / 7, s / 4, s / 7)),
                c.pal.c("frost_ice"),
                0,
            );
        }
        return;
    }
    let spin = e.age * deg(14);
    for k in 0..3 {
        let seg = at.sub(v3(0, s * k / 3, 0));
        let w = s * (9 - k) / 12;
        b.inked(
            Mesh::Cyl,
            &boxed(seg, v3(w, s / 3, w)),
            if k % 2 == 0 { shell } else { plate },
            0,
        );
    }
    let drill = Affine::translate(at.add(v3(0, s * 4 / 10, 0)))
        .then(&Affine::rot_y(spin))
        .then(&Affine::scale(v3(s * 6 / 10, s * 7 / 10, s * 6 / 10)));
    b.inked(Mesh::Cone, &drill, plate, 0);
    for k in 0..4 {
        let a = fx::TURN / 4 * k + e.yaw;
        let p = at.add(v3(
            fx::mul(fx::cos(a), s * 4 / 10),
            s / 10,
            fx::mul(fx::sin(a), s * 4 / 10),
        ));
        b.push(
            Mesh::Sphere,
            &boxed(p, v3(s / 8, s / 8, s / 8)),
            eye,
            ONE,
            PASS_SOLID,
        );
    }
}

/// A Prism: a tall crystal with a cobalt core and three shards round it.
fn prism(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let ice = paint(c.pal.c("frost_ice"));
    let plate = paint(c.pal.c("frost_plate"));
    let spin = e.age * deg(2);
    let body = Affine::translate(at)
        .then(&Affine::rot_y(spin))
        .then(&Affine::scale(v3(s * 6 / 10, s * 14 / 10, s * 6 / 10)));
    b.inked(Mesh::Octa, &body, ice, ONE / 8);
    b.push(
        Mesh::Sphere,
        &boxed(at, v3(s / 3, s / 2, s / 3)),
        c.pal.c("frost_core"),
        ONE,
        PASS_SOLID,
    );
    b.inked(
        Mesh::Sphere,
        &f_of(at, e.yaw, s).then(&boxed(v3(0, 0, -u(26)), v3(u(16), u(16), u(10)))),
        c.pal.c("frost_eye"),
        ONE,
    );
    for k in 0..3 {
        let a = -spin * 3 + fx::TURN / 3 * k;
        let p = at.add(v3(
            fx::mul(fx::cos(a), s * 8 / 10),
            fx::mul(fx::sin(a), s / 4),
            fx::mul(fx::sin(a), s * 8 / 10),
        ));
        b.inked(Mesh::Octa, &boxed(p, v3(s / 6, s * 4 / 10, s / 6)), plate, 0);
    }
}

/// An Aurora: a bell of light high up, its long tendrils swaying in green
/// and pink.
fn aurora(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let shell = paint(c.pal.c("frost_shell"));
    let green = c.pal.c("aurora_green");
    let pink = c.pal.c("aurora_pink");
    let f = f_of(at, e.yaw, s);
    b.inked(
        Mesh::Blob,
        &f.then(&boxed(v3(0, u(10), 0), v3(u(100), u(55), u(100)))),
        shell,
        0,
    );
    b.push(
        Mesh::Ring,
        &f.then(&boxed(v3(0, -u(12), 0), v3(u(96), u(8), u(96)))),
        green,
        ONE,
        PASS_SOLID,
    );
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, u(5), -u(40)), v3(u(20), u(20), u(12)))),
        c.pal.c("frost_eye"),
        ONE,
    );
    for k in 0..6 {
        let a = fx::TURN / 6 * k;
        let mut p = at.add(v3(
            fx::mul(fx::cos(a), s * 4 / 10),
            -s / 8,
            fx::mul(fx::sin(a), s * 4 / 10),
        ));
        let rgb = if k % 2 == 0 { green } else { pink };
        for j in 1..4 {
            let sway = fx::sin(e.age * fx::TURN / 70 + k * deg(60) + j * deg(40));
            let q = p.add(v3(fx::mul(sway, s / 6), -s * 4 / 10, fx::mul(fx::cos(a), s / 12)));
            rod(b, p, q, s / 22, rgb, ONE, false);
            p = q;
        }
    }
}

/// The Witness: a hooded figure grown into a pillar of ice, one great eye,
/// a crown of shards, and the pages of its testimony turning round it.
fn witness(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let robe = paint(c.pal.c("witness_robe"));
    let ice = paint(c.pal.c("witness_ice"));
    let trim = c.pal.c("witness_trim");
    let eye = c.pal.c("witness_eye");
    let f = Affine::translate(at).then(&Affine::rot_y(e.yaw));
    let w = |x: i32| fx::mul(s, ONE * x / 100);
    b.inked(
        Mesh::Cone,
        &f.then(&boxed(v3(0, -w(25), 0), v3(w(80), w(120), w(80)))),
        robe,
        0,
    );
    // the ice it is grown into, in shards up its sides
    for k in 0..7 {
        let a = fx::TURN / 7 * k;
        let p = v3(
            fx::mul(fx::cos(a), w(38)),
            -w(40) + w(9) * (k % 3),
            fx::mul(fx::sin(a), w(38)),
        );
        let shard = f
            .then(&Affine::translate(p))
            .then(&Affine::rot_y(a))
            .then(&Affine::rot_z(deg(12) * (k % 3 - 1)))
            .then(&Affine::scale(v3(w(20), w(70), w(20))));
        b.inked(Mesh::Octa, &shard, ice, ONE / 8);
    }
    // the hood and the eye
    let head = f.then(&Affine::translate(v3(0, w(55), 0)));
    b.inked(
        Mesh::Cone,
        &head.then(&boxed(v3(0, w(8), 0), v3(w(55), w(60), w(55)))),
        robe,
        0,
    );
    let charge = e.glow(c);
    let open = ONE / 2 + charge / 2;
    b.push(
        Mesh::Sphere,
        &head.then(&boxed(
            v3(0, -w(4), -w(20)),
            v3(w(26), fx::mul(w(22), open), w(12)),
        )),
        eye,
        ONE,
        PASS_SOLID,
    );
    // the crown of shards
    for k in 0..6 {
        let a = fx::TURN / 6 * k;
        let p = v3(fx::mul(fx::cos(a), w(20)), w(42), fx::mul(fx::sin(a), w(20)));
        b.inked(
            Mesh::Octa,
            &head.then(&boxed(p, v3(w(8), w(22), w(8)))),
            trim,
            ONE / 3,
        );
    }
    // two long arms of ice, open as if giving evidence
    for side in [-1, 1] {
        let lift = fx::sin(e.age * deg(3) + side * fx::QUARTER) / 5;
        let shoulder = f.apply(v3(side * w(30), w(30), 0));
        let hand = f.apply(v3(side * w(85), w(10) + fx::mul(lift, w(30)), -w(25)));
        rod(b, shoulder, hand, w(10), ice, 0, true);
        b.inked(Mesh::Octa, &boxed(hand, v3(w(14), w(22), w(14))), trim, ONE / 3);
    }
    // the pages, turning round it
    for k in 0..10 {
        let a = fx::TURN / 10 * k + e.age * deg(1);
        let p = at.add(v3(
            fx::mul(fx::cos(a), w(120)),
            w(20) + fx::mul(fx::sin(a * 3), w(12)),
            fx::mul(fx::sin(a), w(120)),
        ));
        let page = Affine::translate(p)
            .then(&Affine::rot_y(-a))
            .then(&Affine::rot_x(deg(20)))
            .then(&Affine::scale(v3(w(16), w(2), w(22))));
        b.inked(Mesh::Cube, &page, c.pal.c("ledger"), ONE / 3);
    }
    if charge > 0 {
        let h = fx::mul(s * 2, ONE + charge / 2);
        b.glow(
            Mesh::Sphere,
            &boxed(at.add(v3(0, w(30), 0)), v3(h, h, h)),
            c.pal.c("frost_core"),
            charge / 3,
        );
    }
}
