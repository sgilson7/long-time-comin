//! Zone four's creatures: the mountain's own deer, birds, toads, plants,
//! water and bears, turning. Two things are taking them. The ritual that
//! will stop time leaves clockwork in them: gold hands through their
//! hides, rings of hours, and an afterimage that lags behind them where
//! time stutters. The Idiot God, coming nearer, leaves eyes and violet
//! flesh. Both grow with `Enemy::corrupt`, more the higher the arena.

use super::rod;
use crate::content::{Content, EnemyDef, Rgb};
use crate::enemy::Enemy;
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{boxed, mix, shade, Batch, PASS_SOLID};

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
    let spots = match d.id.as_str() {
        "hart" => hart(b, c, e, at, s, paint),
        "rook" => rook(b, c, e, at, s, paint),
        "toad" => toad(b, c, e, at, s, paint),
        "gaper" => gaper(b, c, e, d, at, s, paint),
        "freshet" => freshet(b, c, e, at, s, paint),
        "sporecap" => sporecap(b, c, e, at, s, paint),
        "ursine" => ursine(b, c, e, at, s, paint),
        _ => return false,
    };
    corruption(b, c, e, at, s, &spots);
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

/// What the ritual and the Idiot God have done to it: eyes opening at
/// `spots` (world points on its body), violet tendrils, a gold ring of
/// hours, and a lagging afterimage. None of it at no corruption.
fn corruption(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, spots: &[V3]) {
    let k = e.corrupt;
    if k <= 0 {
        return;
    }
    let eyes = (spots.len() as i32 * k / ONE).max(1) as usize;
    let flesh = c.pal.c("eldritch_flesh");
    let glow = c.pal.c("eldritch_glow");
    let dark = c.pal.c("eldritch_void");
    for (n, p) in spots.iter().take(eyes).enumerate() {
        // each blinks on its own clock
        let open = (e.age + n as i32 * 37) % 150 > 6;
        let r = s / 11;
        b.inked(
            Mesh::Sphere,
            &boxed(*p, v3(r, if open { r } else { r / 5 }, r)),
            glow,
            ONE,
        );
        if open {
            b.push(
                Mesh::Sphere,
                &boxed(*p, v3(r / 2, r / 2, r / 2)),
                dark,
                0,
                PASS_SOLID,
            );
        }
    }
    // tendrils from its back: up, out and curling over, waving, thinner
    // toward their tips
    let tendrils = (k * 6 / ONE - 1).max(0);
    for t in 0..tendrils {
        let a = fx::TURN / tendrils.max(1) * t + deg(25);
        let out = v3(fx::cos(a), 0, fx::sin(a));
        let mut p = at.add(out.scale(s / 5)).add(v3(0, s / 4, 0));
        let mut dir = out.add(v3(0, ONE * 2, 0)).norm();
        for j in 0..5 {
            let sway = fx::sin(e.age * fx::TURN / 70 + j * deg(50) + t * deg(80));
            // each joint bends further out and over
            dir = dir
                .add(out.scale(ONE / 3))
                .add(v3(fx::mul(sway, ONE / 3), -ONE / 5, 0))
                .norm();
            let q = p.add(dir.scale(s / 9));
            rod(b, p, q, s / (16 + j * 6), flesh, ONE / 6, false);
            p = q;
        }
        b.push(
            Mesh::Sphere,
            &boxed(p, v3(s / 30, s / 30, s / 30)),
            glow,
            ONE,
            PASS_SOLID,
        );
    }
    // the ritual: a ring of hours and its hands, turning backward
    let ring = at.add(v3(0, s * 6 / 10, 0));
    let hours = c.pal.c("clock");
    let n = 6 + 6 * k / ONE;
    for h in 0..n {
        let a = fx::TURN / n * h - e.age * deg(2);
        let p = ring.add(v3(
            fx::mul(fx::cos(a), s * 6 / 10),
            0,
            fx::mul(fx::sin(a), s * 6 / 10),
        ));
        b.push(
            Mesh::Cube,
            &boxed(p, v3(s / 30, s / 12, s / 30)),
            hours,
            ONE / 3,
            PASS_SOLID,
        );
    }
    // where time stutters, it is a moment behind itself
    let lag = e.pos.sub(e.prev).scale(-ONE * 5);
    if k > ONE / 2 && lag != V3::ZERO {
        let ghost = at.add(lag);
        b.glow(
            Mesh::Sphere,
            &boxed(ghost, v3(s / 2, s / 2, s / 2)),
            c.pal.c("gold"),
            (k - ONE / 2) / 2,
        );
    }
}

/// A hart: a stag on long legs, its antlers branching into the hands of
/// clocks. Its legs fold under it in a bound.
fn hart(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) -> Vec<V3> {
    let hide = paint(c.pal.c("fauna_hide"));
    let pale = paint(c.pal.c("fauna_hide_pale"));
    let bone = c.pal.c("fauna_antler");
    let gold = c.pal.c("gold");
    let f = f_of(at, e.yaw, s);
    let tuck = if e.leap > 0 { ONE } else { 0 };
    let step = e.age * fx::TURN / 30;
    for (k, (lx, lz)) in [(-1, -1), (1, -1), (-1, 1), (1, 1)].iter().enumerate() {
        let swing = fx::mul(fx::sin(step + fx::HALF * (k as i32 % 2)), u(6));
        let hip = f.apply(v3(lx * u(16), -u(6), lz * u(30)));
        let knee = f.apply(v3(
            lx * u(17),
            -u(30) + fx::mul(tuck, u(16)),
            lz * u(30) + swing + fx::mul(tuck, lz * u(10)),
        ));
        let foot = f.apply(v3(lx * u(17), -u(62) + fx::mul(tuck, u(40)), lz * u(30) + swing));
        rod(b, hip, knee, s / 16, hide, 0, true);
        rod(b, knee, foot, s / 22, shade(hide, 80), 0, true);
    }
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(V3::ZERO, v3(u(42), u(36), u(80)))),
        hide,
        0,
    );
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, -u(6), 0), v3(u(32), u(24), u(64)))),
        pale,
        0,
    );
    let neck = f.apply(v3(0, u(10), -u(34)));
    let head = f.apply(v3(0, u(42), -u(48)));
    rod(b, neck, head, s / 9, hide, 0, true);
    b.inked(
        Mesh::Cone,
        &f.then(
            &Affine::translate(v3(0, u(42), -u(58)))
                .then(&Affine::rot_x(-fx::QUARTER))
                .then(&Affine::scale(v3(u(16), u(30), u(16)))),
        ),
        hide,
        0,
    );
    // antlers: tines, and the last of each a clock's hand
    for side in [-1, 1] {
        let mut p = f.apply(v3(side * u(6), u(50), -u(48)));
        for j in 1..4 {
            let q = p.add(f.apply_dir(v3(side * u(12), u(14), u(4) * j)));
            rod(b, p, q, s / 40, bone, 0, true);
            let tine = q.add(f.apply_dir(v3(side * u(4), u(12), -u(6))));
            rod(
                b,
                q,
                tine,
                s / 55,
                if j == 3 { gold } else { bone },
                if j == 3 { ONE / 2 } else { 0 },
                true,
            );
            p = q;
        }
    }
    vec![
        f.apply(v3(-u(8), u(46), -u(60))),
        f.apply(v3(u(8), u(46), -u(60))),
        f.apply(v3(-u(20), u(10), 0)),
        f.apply(v3(u(20), u(10), u(10))),
        f.apply(v3(0, u(18), u(30))),
        f.apply(v3(-u(12), -u(8), u(24))),
    ]
}

/// A rook: a black bird of the crags, its wings ragged, too many eyes
/// along them.
fn rook(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) -> Vec<V3> {
    let feather = paint(c.pal.c("fauna_feather"));
    let beak = c.pal.c("fauna_beak");
    let f = f_of(at, e.yaw, s);
    let flap = fx::sin(e.age * fx::TURN / 14) / 2;
    b.inked(
        Mesh::Octa,
        &f.then(&boxed(V3::ZERO, v3(u(36), u(32), u(80)))),
        feather,
        0,
    );
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, u(10), -u(40)), v3(u(30), u(28), u(30)))),
        feather,
        0,
    );
    b.inked(
        Mesh::Cone,
        &f.then(
            &Affine::translate(v3(0, u(8), -u(60)))
                .then(&Affine::rot_x(-fx::QUARTER))
                .then(&Affine::scale(v3(u(10), u(26), u(10)))),
        ),
        beak,
        0,
    );
    for side in [-1, 1] {
        for j in 0..2 {
            let wing = f
                .then(&Affine::translate(v3(side * (u(30) + u(36) * j), u(6) * j, u(4))))
                .then(&Affine::rot_z(side * fx::mul(flap, ONE + ONE * j / 2)))
                .then(&Affine::scale(v3(u(70), u(4), u(46) - u(10) * j)));
            b.inked(Mesh::Wedge, &wing, shade(feather, 90 + 10 * j), 0);
        }
    }
    b.inked(
        Mesh::Wedge,
        &f.then(&boxed(v3(0, 0, u(48)), v3(u(34), u(4), u(36)))),
        feather,
        0,
    );
    vec![
        f.apply(v3(-u(10), u(16), -u(52))),
        f.apply(v3(u(10), u(16), -u(52))),
        f.apply(v3(-u(50), u(4), 0)),
        f.apply(v3(u(50), u(4), 0)),
        f.apply(v3(-u(80), u(8), u(6))),
        f.apply(v3(u(80), u(8), u(6))),
        f.apply(v3(0, u(20), u(10))),
    ]
}

/// A toad: squat and mottled, its throat swelling with what it spits.
fn toad(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) -> Vec<V3> {
    let skin = paint(c.pal.c("fauna_toad"));
    let belly = paint(c.pal.c("fauna_toad_belly"));
    let f = f_of(at, e.yaw, s);
    let crouch = if e.leap > 0 { u(10) } else { 0 };
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, crouch / 2, u(6)), v3(u(80), u(46), u(84)))),
        skin,
        0,
    );
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, -u(10), -u(30)), v3(u(64), u(30), u(40)))),
        belly,
        0,
    );
    // the throat: it swells through a windup
    let swell = ONE + e.glow(c) / 2 + fx::sin(e.age * fx::TURN / 40) / 10;
    let t = fx::mul(u(34), swell);
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, -u(16), -u(42)), v3(t, t, t))),
        mix(belly, c.pal.c("eldritch_glow"), e.corrupt / 2),
        ONE / 4,
    );
    for side in [-1, 1] {
        b.inked(
            Mesh::Sphere,
            &f.then(&boxed(v3(side * u(24), u(26), -u(30)), v3(u(20), u(20), u(20)))),
            skin,
            0,
        );
        b.push(
            Mesh::Sphere,
            &f.then(&boxed(v3(side * u(26), u(30), -u(38)), v3(u(10), u(10), u(6)))),
            c.pal.c("gold"),
            ONE,
            PASS_SOLID,
        );
        let leg = f
            .then(&Affine::translate(v3(side * u(40), -u(20) - crouch, u(20))))
            .then(&Affine::rot_y(side * deg(30)))
            .then(&Affine::scale(v3(u(30), u(20), u(56))));
        b.inked(Mesh::Wedge, &leg, shade(skin, 85), 0);
    }
    vec![
        f.apply(v3(-u(20), u(30), u(10))),
        f.apply(v3(u(22), u(28), u(20))),
        f.apply(v3(0, u(24), u(36))),
        f.apply(v3(-u(36), u(10), u(30))),
        f.apply(v3(u(36), u(12), -u(4))),
    ]
}

/// A gaper: a pitcher of a plant on a thick stalk, its head two jaws of
/// teeth; it lunges on the stalk to bite.
fn gaper(
    b: &mut Batch,
    c: &Content,
    e: &Enemy,
    d: &EnemyDef,
    at: V3,
    s: i32,
    paint: &dyn Fn(Rgb) -> Rgb,
) -> Vec<V3> {
    let leaf = paint(c.pal.c("fauna_leaf"));
    let dark = paint(c.pal.c("fauna_leaf_dark"));
    let petal = paint(c.pal.c("fauna_petal"));
    let teeth = c.pal.c("fauna_teeth");
    let base = at.sub(v3(0, d.hover, 0));
    // leaves round its root
    for k in 0..6 {
        let a = fx::TURN / 6 * k + deg(15);
        let lf =
            Affine::translate(base.add(v3(fx::mul(fx::cos(a), s / 3), s / 20, fx::mul(fx::sin(a), s / 3))))
                .then(&Affine::rot_y(-a))
                .then(&Affine::rot_z(deg(20)))
                .then(&Affine::scale(v3(s * 6 / 10, s / 30, s / 4)));
        b.inked(Mesh::Wedge, &lf, if k % 2 == 0 { leaf } else { dark }, 0);
    }
    let head = at.add(e.head().sub(e.pos));
    let sway = fx::sin(e.age * fx::TURN / 90);
    let mid = base.lerp(head, ONE / 2).add(v3(fx::mul(sway, s / 6), 0, 0));
    rod(b, base, mid, s / 7, dark, 0, true);
    rod(b, mid, head, s / 8, leaf, 0, true);
    // the jaws open as it reaches
    let open = deg(10) + fx::div(e.reach * deg(40), d.bite.max(1));
    let f = f_of(head, e.yaw, s);
    for side in [-1, 1] {
        let jaw = f
            .then(&Affine::rot_x(side * open))
            .then(&Affine::translate(v3(0, side * u(10), -u(16))))
            .then(&Affine::scale(v3(u(52), u(14), u(54))));
        b.inked(Mesh::Sphere, &jaw, petal, 0);
        for t in 0..5 {
            let x = u(-20) + u(10) * t;
            let tooth = f
                .then(&Affine::rot_x(side * open))
                .then(&Affine::translate(v3(x, side * u(4), -u(36))))
                .then(&Affine::rot_x(if side < 0 { fx::HALF } else { 0 }))
                .then(&Affine::scale(v3(u(5), u(12), u(5))));
            b.push(Mesh::Cone, &tooth, teeth, 0, PASS_SOLID);
        }
    }
    vec![
        f.apply(v3(-u(16), u(24), -u(10))),
        f.apply(v3(u(16), u(24), -u(10))),
        mid,
        base.lerp(mid, ONE / 2),
        f.apply(v3(0, u(30), u(10))),
    ]
}

/// A freshet: a mountain stream stood up, a column of water turning on
/// itself, white at its foot, two pale lights for a face.
fn freshet(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) -> Vec<V3> {
    let water = paint(c.pal.c("fauna_water"));
    let deep = paint(c.pal.c("fauna_water_deep"));
    let foam = c.pal.c("fauna_foam");
    let mut spots = Vec::new();
    for k in 0..6 {
        let a = e.age * deg(6) + k * deg(60);
        let r = s / 3 - s / 30 * k;
        let p = at.add(v3(
            fx::mul(fx::cos(a), s / 10),
            s / 6 * k - s / 3,
            fx::mul(fx::sin(a), s / 10),
        ));
        b.inked(
            Mesh::Sphere,
            &boxed(p, v3(r, s / 4, r)),
            if k % 2 == 0 { water } else { deep },
            ONE / 6,
        );
        spots.push(p.add(v3(r / 2, 0, -r / 2)));
    }
    let foot = at.sub(v3(0, s / 2, 0));
    let ring = Affine::translate(foot)
        .then(&Affine::rot_y(e.age * deg(4)))
        .then(&Affine::scale(v3(s * 7 / 10, s / 10, s * 7 / 10)));
    b.push(Mesh::Ring, &ring, foam, ONE / 2, PASS_SOLID);
    let f = f_of(at, e.yaw, s);
    for side in [-1, 1] {
        b.push(
            Mesh::Sphere,
            &f.then(&boxed(v3(side * u(12), u(40), -u(24)), v3(u(8), u(12), u(6)))),
            foam,
            ONE,
            PASS_SOLID,
        );
    }
    b.glow(Mesh::Sphere, &boxed(at, v3(s, s, s)), water, ONE / 5);
    spots
}

/// A sporecap: a great toadstool, spotted, its gills under the cap
/// breathing out a haze.
fn sporecap(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) -> Vec<V3> {
    let cap = paint(c.pal.c("fauna_cap"));
    let gill = paint(c.pal.c("fauna_gill"));
    let stem = paint(c.pal.c("fauna_hide_pale"));
    let breathe = ONE + fx::sin(e.age * fx::TURN / 120) / 12 + e.glow(c) / 4;
    let foot = at.sub(v3(0, s / 2, 0));
    b.inked(
        Mesh::Cyl,
        &boxed(
            foot.add(v3(0, s * 3 / 10, 0)),
            v3(s * 3 / 10, s * 6 / 10, s * 3 / 10),
        ),
        stem,
        0,
    );
    let top = at.add(v3(0, s / 4, 0));
    let w = fx::mul(s * 12 / 10, breathe);
    b.inked(Mesh::Sphere, &boxed(top, v3(w, s / 2, w)), cap, 0);
    b.push(
        Mesh::Cyl,
        &boxed(top.sub(v3(0, s / 10, 0)), v3(w * 95 / 100, s / 40, w * 95 / 100)),
        gill,
        0,
        PASS_SOLID,
    );
    let mut spots = Vec::new();
    for k in 0..7 {
        let a = fx::TURN / 7 * k;
        let p = top.add(v3(
            fx::mul(fx::cos(a), w * 3 / 10),
            s / 5,
            fx::mul(fx::sin(a), w * 3 / 10),
        ));
        b.push(
            Mesh::Sphere,
            &boxed(p, v3(s / 12, s / 30, s / 12)),
            shade(stem, 115),
            0,
            PASS_SOLID,
        );
        spots.push(p);
    }
    b.glow(
        Mesh::Sphere,
        &boxed(top.sub(v3(0, s / 4, 0)), v3(w, s / 3, w)),
        gill,
        ONE / 6,
    );
    spots
}

/// An ursine: a bear of the high slopes, a hump of fur, a jaw that has
/// split; it walks on all fours.
fn ursine(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) -> Vec<V3> {
    let fur = paint(c.pal.c("fauna_fur"));
    let teeth = c.pal.c("fauna_teeth");
    let f = f_of(at, e.yaw, s);
    let step = e.age * fx::TURN / 44;
    for (k, (lx, lz)) in [(-1, -1), (1, -1), (-1, 1), (1, 1)].iter().enumerate() {
        let lift = fx::sin(step + fx::HALF * (k as i32 % 2)).max(0) / 6;
        let hip = f.apply(v3(lx * u(26), -u(10), lz * u(28)));
        let foot = f.apply(v3(lx * u(28), -u(48) + lift, lz * u(30)));
        rod(b, hip, foot, s / 6, shade(fur, 85), 0, true);
    }
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, 0, u(6)), v3(u(66), u(56), u(92)))),
        fur,
        0,
    );
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, u(26), -u(14)), v3(u(54), u(36), u(40)))),
        shade(fur, 110),
        0,
    );
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, u(14), -u(50)), v3(u(36), u(32), u(34)))),
        fur,
        0,
    );
    let gape = deg(12) + e.glow(c) * 25 / 100;
    for side in [-1, 1] {
        let jaw = f
            .then(&Affine::translate(v3(0, u(8), -u(60))))
            .then(&Affine::rot_x(side * gape))
            .then(&Affine::scale(v3(u(22), u(8), u(26))));
        b.inked(Mesh::Wedge, &jaw, shade(fur, 70), 0);
        b.push(
            Mesh::Cone,
            &f.then(&boxed(
                v3(side * u(8), u(8) + side * u(4), -u(70)),
                v3(u(4), u(10), u(4)),
            )),
            teeth,
            0,
            PASS_SOLID,
        );
        b.inked(
            Mesh::Sphere,
            &f.then(&boxed(v3(side * u(18), u(40), -u(40)), v3(u(10), u(12), u(8)))),
            fur,
            0,
        );
    }
    vec![
        f.apply(v3(-u(10), u(22), -u(64))),
        f.apply(v3(u(10), u(22), -u(64))),
        f.apply(v3(0, u(46), -u(10))),
        f.apply(v3(-u(30), u(20), u(20))),
        f.apply(v3(u(30), u(24), u(30))),
        f.apply(v3(-u(20), u(36), u(40))),
        f.apply(v3(u(14), u(10), u(50))),
    ]
}
