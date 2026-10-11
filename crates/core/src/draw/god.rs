//! Zone five's enemies, each drawn as if it came out of another art or
//! another game, and the Idiot God at the Heart.
//!
//! - the Glyph: an eight-bit invader of cubes on a flat grid, two frames;
//! - the Tesseract: a vector display's hypercube, lines of light only,
//!   turning through four dimensions;
//! - the Etching: a shrouded figure in one-bit dither, as an engraving;
//! - the Render: a PlayStation hound of a few big boxes, its corners
//!   snapped to a coarse grid;
//! - the Toon: a rubber-hose cartoon, pie-cut eyes and white gloves, its
//!   ink boiling;
//! - the Reprise: you, in a photograph's negative;
//! - the Recursion: a Sierpinski pyramid in gold.
//!
//! The style passes are the renderer's (`render.rs`): a body's solids go
//! in the pass `Batch::style` names while it is drawn.

use super::rod;
use crate::balance::{BOUNCE_TICKS, PLAYER_CHEST};
use crate::content::{Content, EnemyDef, Rgb};
use crate::enemy::Enemy;
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{
    boxed, inflate_by, shade, Batch, PASS_DITHER, PASS_INK, PASS_NEGATIVE, PASS_PS1, PASS_SOLID,
};

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
        "glyph" => glyph(b, c, e, at, s, paint),
        "tesseract" => tesseract(b, c, e, at, s),
        "etching" => etching(b, c, e, at, s, paint),
        "render" => render(b, c, e, at, s, paint),
        "toon" => toon(b, c, e, at, s, paint),
        "reprise" => reprise(b, c, e, at),
        "recursion" => recursion(b, c, e, at, s, 2, paint),
        "recursion_minor" => recursion(b, c, e, at, s, 1, paint),
        "recursion_least" => recursion(b, c, e, at, s, 0, paint),
        _ => return false,
    }
    b.style = PASS_SOLID;
    true
}

fn u(x: i32) -> i32 {
    ONE * x / 100
}

fn f_of(at: V3, yaw: i32, s: i32) -> Affine {
    Affine::translate(at)
        .then(&Affine::rot_y(yaw))
        .then(&Affine::scale(v3(s, s, s)))
}

// ---- the Glyph ------------------------------------------------------------

/// Two frames of eleven by eight: `#` lit, `o` its eyes.
const GLYPH: [[&str; 8]; 2] = [
    [
        "   #####   ",
        "  #######  ",
        " ##o###o## ",
        " ######### ",
        "  ## # ##  ",
        " #  # #  # ",
        "#  #   #  #",
        "  #     #  ",
    ],
    [
        "   #####   ",
        "  #######  ",
        " ##o###o## ",
        " ######### ",
        "  ## # ##  ",
        "  # # # #  ",
        "  #     #  ",
        " #       # ",
    ],
];

fn glyph(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let lit = paint(c.pal.c("glyph_green"));
    let eye = c.pal.c("glyph_dark");
    let frame = &GLYPH[(e.moves & 1) as usize];
    let cell = s / 9;
    let f = Affine::translate(at).then(&Affine::rot_y(e.yaw));
    for (row, line) in frame.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            let rgb = match ch {
                '#' => lit,
                'o' => eye,
                _ => continue,
            };
            let p = v3(
                (col as i32 * 2 - 10) * cell / 2,
                (7 - row as i32 * 2) * cell / 2,
                0,
            );
            // full bright and flat: a screen's pixels, not a lit thing
            b.inked(Mesh::Cube, &f.then(&boxed(p, v3(cell, cell, cell))), rgb, ONE);
        }
    }
}

// ---- the Tesseract ---------------------------------------------------------

/// A four-dimensional cube, turned in the x–w and z–w planes and in x–y,
/// seen in perspective down w: its sixteen corners and thirty-two edges,
/// in light only.
fn tesseract(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32) {
    let cyan = c.pal.c("vector_cyan");
    let pink = c.pal.c("vector_pink");
    let a1 = e.age * deg(1);
    let a2 = e.age * deg(2) / 3;
    let a3 = e.age * deg(1) / 2;
    let turn = |x: i32, y: i32, a: i32| {
        let (sn, cs) = (fx::sin(a), fx::cos(a));
        (fx::mul(x, cs) - fx::mul(y, sn), fx::mul(x, sn) + fx::mul(y, cs))
    };
    let corner = |k: usize| -> V3 {
        let sign = |bit: usize| if k & (1 << bit) != 0 { ONE } else { -ONE };
        let (x, y, z, w) = (sign(0), sign(1), sign(2), sign(3));
        let (x, w) = turn(x, w, a1);
        let (z, w) = turn(z, w, a2);
        let (x, y) = turn(x, y, a3);
        // nearer in w is larger
        let k = fx::div(ONE * 3, ONE * 3 - w);
        let r = s * 3 / 10;
        at.add(v3(
            fx::mul(fx::mul(x, k), r),
            fx::mul(fx::mul(y, k), r),
            fx::mul(fx::mul(z, k), r),
        ))
    };
    let pts: Vec<V3> = (0..16).map(corner).collect();
    for i in 0..16 {
        for bit in 0..4 {
            let j = i ^ (1 << bit);
            if j > i {
                if let Some(af) = crate::arena::segment(pts[i], pts[j], u(4)) {
                    let rgb = if bit == 3 { pink } else { cyan };
                    b.glow(Mesh::Cyl, &af, rgb, ONE);
                }
            }
        }
        b.glow(
            Mesh::Sphere,
            &boxed(pts[i], v3(u(14), u(14), u(14))),
            cyan,
            ONE / 2,
        );
    }
}

// ---- the Etching ----------------------------------------------------------

/// A gaunt figure in a hooded robe with long hands, engraved: one-bit
/// dither. Looked at, it is a statue; unwatched, it leans at you.
fn etching(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    b.style = PASS_DITHER;
    let paper = paint(c.pal.c("etching_paper"));
    let hollow = shade(paper, 15);
    let lean = if e.seen {
        0
    } else {
        deg(12) + fx::sin(e.age * deg(4)) / 40
    };
    let h = s * 22 / 10;
    let feet = at.sub(v3(0, s, 0));
    let f = Affine::translate(feet)
        .then(&Affine::rot_y(e.yaw))
        .then(&Affine::rot_x(-lean));
    // the robe, falling to the floor
    b.inked(
        Mesh::Cone,
        &f.then(&boxed(
            v3(0, h * 4 / 10, 0),
            v3(s * 9 / 10, h * 8 / 10, s * 8 / 10),
        )),
        paper,
        0,
    );
    b.inked(
        Mesh::Cyl,
        &f.then(&boxed(
            v3(0, h * 6 / 10, 0),
            v3(s * 5 / 10, h * 4 / 10, s * 4 / 10),
        )),
        paper,
        0,
    );
    // the hood, and the dark in it
    let head = v3(0, h * 85 / 100, 0);
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(head, v3(s * 5 / 10, s * 6 / 10, s * 5 / 10))),
        paper,
        0,
    );
    b.push(
        Mesh::Sphere,
        &f.then(&boxed(
            head.add(v3(0, -u(4), -s / 6)),
            v3(s * 3 / 10, s * 4 / 10, s / 5),
        )),
        hollow,
        0,
        PASS_SOLID,
    );
    // long arms, and longer fingers, hanging
    let sway = if e.seen { 0 } else { fx::sin(e.age * deg(3)) / 8 };
    for side in [-1, 1] {
        let shoulder = f.apply(v3(side * s / 4, h * 7 / 10, 0));
        let hand = f.apply(v3(side * s * 4 / 10, h * 25 / 100, -s / 4 - fx::mul(sway, s)));
        rod(b, shoulder, hand, s / 8, paper, 0, true);
        for k in -1..=1 {
            let tip = hand.add(v3(side * s / 10 + k * s / 20, -s * 4 / 10, -s / 10));
            rod(b, hand, tip, s / 30, paper, 0, true);
        }
    }
}

// ---- the Render -------------------------------------------------------------

/// A hound from the first years of 3D: a few big boxes and wedges, flat
/// colours, its corners snapped to the screen's coarse grid. It faces
/// where its tank controls turned it, and its legs move only in a charge.
fn render(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    b.style = PASS_PS1;
    let skin = paint(c.pal.c("render_skin"));
    let dark = paint(c.pal.c("render_dark"));
    let eye = c.pal.c("render_eye");
    let f = f_of(at, e.yaw, s);
    // body, chest, haunch
    b.inked(
        Mesh::Cube,
        &f.then(&boxed(v3(0, 0, 0), v3(u(55), u(45), u(100)))),
        skin,
        0,
    );
    b.inked(
        Mesh::Cube,
        &f.then(&boxed(v3(0, u(10), -u(45)), v3(u(62), u(55), u(30)))),
        skin,
        0,
    );
    // head and snout, ears
    let head = v3(0, u(35), -u(70));
    b.inked(
        Mesh::Cube,
        &f.then(&boxed(head, v3(u(40), u(36), u(36)))),
        skin,
        0,
    );
    b.inked(
        Mesh::Wedge,
        &f.then(&boxed(head.add(v3(0, -u(8), -u(30))), v3(u(28), u(20), u(30)))),
        dark,
        0,
    );
    for side in [-1, 1] {
        b.inked(
            Mesh::Wedge,
            &f.then(&boxed(
                head.add(v3(side * u(14), u(26), u(4))),
                v3(u(10), u(22), u(12)),
            )),
            dark,
            0,
        );
        b.push(
            Mesh::Cube,
            &f.then(&boxed(
                head.add(v3(side * u(12), u(6), -u(19))),
                v3(u(9), u(7), u(3)),
            )),
            eye,
            ONE,
            PASS_SOLID,
        );
    }
    // legs: still unless it charges
    let swing = if e.leap > 0 {
        fx::sin(e.age * deg(40)) / 3
    } else {
        0
    };
    for (k, (lx, lz)) in [(-1, -1), (1, -1), (-1, 1), (1, 1)].iter().enumerate() {
        let ph = if k % 2 == 0 { swing } else { -swing };
        let leg = f
            .then(&Affine::translate(v3(lx * u(20), -u(20), lz * u(38))))
            .then(&Affine::rot_x(ph))
            .then(&boxed(v3(0, -u(25), 0), v3(u(14), u(50), u(14))));
        b.inked(Mesh::Cube, &leg, dark, 0);
    }
    // a stub of a tail
    b.inked(
        Mesh::Wedge,
        &f.then(&boxed(v3(0, u(25), u(55)), v3(u(10), u(16), u(22)))),
        dark,
        0,
    );
}

// ---- the Toon ---------------------------------------------------------------

/// A shape pushed solid, with an outline whose thickness shifts every few
/// ticks: the boiling line of drawn animation.
fn boil(b: &mut Batch, mesh: Mesh, a: &Affine, rgb: Rgb, beat: i32, k: i32) {
    b.push(mesh, a, rgb, ONE * 7 / 10, PASS_SOLID);
    let t = u(5) + u(2) * ((beat / 5 + k * 7).rem_euclid(3));
    b.push(mesh, &inflate_by(a, t), [0; 3], 0, PASS_INK);
}

fn toon(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let black = paint(c.pal.c("toon_black"));
    let white = paint(c.pal.c("toon_white"));
    let red = paint(c.pal.c("toon_red"));
    let t = e.beat.rem_euclid(BOUNCE_TICKS);
    // squashed at the landing, stretched in the air
    let air = fx::sin(t * fx::HALF / BOUNCE_TICKS.max(1));
    let sy = u(78) + fx::mul(air, u(40));
    let sx = fx::div(ONE, sy.max(ONE / 2)).min(ONE * 13 / 10);
    let foot_y = -fx::mul(s / 2, sy);
    let f = Affine::translate(at)
        .then(&Affine::rot_y(e.yaw))
        .then(&Affine::scale(v3(fx::mul(s, sx), fx::mul(s, sy), fx::mul(s, sx))));
    let n = e.beat;
    boil(
        b,
        Mesh::Sphere,
        &f.then(&boxed(V3::ZERO, v3(ONE, ONE, ONE))),
        black,
        n,
        0,
    );
    // pie-cut eyes: white ovals, black pupils with a wedge cut from them
    for (k, side) in [-1, 1].into_iter().enumerate() {
        let eye = v3(side * u(17), u(18), -u(38));
        boil(
            b,
            Mesh::Sphere,
            &f.then(&boxed(eye, v3(u(22), u(36), u(12)))),
            white,
            n,
            1 + k as i32,
        );
        b.push(
            Mesh::Sphere,
            &f.then(&boxed(eye.add(v3(0, -u(4), -u(5))), v3(u(10), u(18), u(4)))),
            black,
            0,
            PASS_SOLID,
        );
        b.push(
            Mesh::Wedge,
            &f.then(&Affine::translate(eye.add(v3(u(3), -u(1), -u(7)))))
                .then(&Affine::rot_z(deg(30)))
                .then(&Affine::scale(v3(u(5), u(6), u(3)))),
            white,
            ONE,
            PASS_SOLID,
        );
    }
    // the grin, and a tongue
    boil(
        b,
        Mesh::Sphere,
        &f.then(&boxed(v3(0, -u(16), -u(40)), v3(u(44), u(16), u(10)))),
        white,
        n,
        3,
    );
    b.push(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, -u(20), -u(44)), v3(u(16), u(7), u(4)))),
        red,
        ONE / 2,
        PASS_SOLID,
    );
    // noodle arms, white gloves
    let world = |p: V3| f.apply(p);
    for (k, side) in [-1, 1].into_iter().enumerate() {
        let wave = fx::sin(n * deg(9) + side * deg(90)) / 3;
        let mut prev = world(v3(side * u(45), 0, 0));
        for j in 1..=3 {
            let p = world(v3(
                side * (u(45) + u(16) * j),
                fx::mul(wave, u(20) * j) + u(6) * j,
                -u(8) * j,
            ));
            rod(b, prev, p, s / 14, black, ONE / 2, false);
            prev = p;
        }
        let glove = boxed(prev, v3(s / 4, s / 4, s / 4));
        boil(b, Mesh::Sphere, &glove, white, n, 4 + k as i32);
        b.push(
            Mesh::Cyl,
            &boxed(prev.add(v3(-side * s / 10, 0, 0)), v3(s / 6, s / 14, s / 6)),
            white,
            ONE,
            PASS_SOLID,
        );
    }
    // legs, and big shoes
    for side in [-1, 1] {
        let hip = world(v3(side * u(18), -u(40), 0));
        let foot = at
            .add(v3(0, foot_y - s / 6, 0))
            .add(crate::geom::right_of(e.yaw).scale(side * s / 5));
        rod(b, hip, foot, s / 14, black, ONE / 2, false);
        boil(
            b,
            Mesh::Sphere,
            &boxed(
                foot.add(crate::geom::facing(e.yaw, 0).scale(s / 10)),
                v3(s / 4, s / 7, s * 2 / 5),
            ),
            red,
            n,
            6,
        );
    }
}

// ---- the Reprise ------------------------------------------------------------

/// You, a few seconds ago, in negative.
fn reprise(b: &mut Batch, c: &Content, e: &Enemy, at: V3) {
    b.style = PASS_NEGATIVE;
    let feet = at.sub(v3(0, PLAYER_CHEST, 0));
    super::standing(b, c, feet, e.yaw, c.gun("carbine"));
}

// ---- the Recursion ----------------------------------------------------------

/// A Sierpinski pyramid `depth` deep: four smaller pyramids at the
/// corners of each, down to a solid at the last.
fn recursion(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, depth: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let gold = paint(c.pal.c("recursion_gold"));
    let dark = paint(c.pal.c("recursion_dark"));
    let f = Affine::translate(at)
        .then(&Affine::rot_y(e.age * deg(1)))
        .then(&Affine::rot_x(deg(20)));
    let corners = [
        v3(ONE, ONE, ONE),
        v3(ONE, -ONE, -ONE),
        v3(-ONE, ONE, -ONE),
        v3(-ONE, -ONE, ONE),
    ];
    let mut cells = vec![(V3::ZERO, s * 6 / 10)];
    for _ in 0..depth {
        let mut next = Vec::new();
        for (p, r) in cells {
            for k in corners {
                next.push((p.add(k.scale(r / 4)), r / 2));
            }
        }
        cells = next;
    }
    for (k, (p, r)) in cells.iter().enumerate() {
        let rgb = if k % 3 == 0 { dark } else { gold };
        let leaf = f
            .then(&Affine::translate(*p))
            .then(&Affine::rot_x(deg(35)))
            .then(&Affine::rot_y(deg(45)))
            .then(&Affine::scale(v3(*r * 7 / 10, *r * 7 / 10, *r * 7 / 10)));
        b.inked(Mesh::Octa, &leaf, rgb, ONE / 4);
    }
    b.glow(
        Mesh::Sphere,
        &boxed(at, v3(s / 2, s / 2, s / 2)),
        c.pal.c("god_sun"),
        ONE / 3,
    );
}

// ---- the Idiot God ----------------------------------------------------------

/// A number from `k` that looks unordered, in [0, ONE).
fn scatter(k: i32, salt: i32) -> i32 {
    let h = (k.wrapping_mul(2654435761u32 as i32) ^ salt.wrapping_mul(40503)).wrapping_mul(1103515245);
    (h >> 8).rem_euclid(ONE)
}

/// A point on the unit sphere for `k` of `n`, spread evenly (a spiral).
fn on_sphere(k: i32, n: i32) -> V3 {
    let y = ONE - 2 * (k * 2 + 1) * ONE / (2 * n.max(1));
    let r = fx::isqrt((ONE as i64 * ONE as i64 - y as i64 * y as i64).max(0) as u64) as i32;
    let a = k * deg(137) + deg(30);
    v3(fx::mul(fx::cos(a), r), y, fx::mul(fx::sin(a), r))
}

/// How many eyes the god has.
pub const EYES: i32 = 26;
const BUBBLES: i32 = 46;
const DANCERS: i32 = 8;

/// The Idiot God at its heart: a heaving mass of bubbles and eyes, a dark
/// maw toward you, tendrils. Phase by phase more of it wakes: its eyes
/// open, its dancers come round it piping, and at last part of it turns
/// to negative. `sleep` (Q16, 0 to ONE) is how far the lullaby has put it
/// down: its eyes close one by one, its heaving and its dancers still.
pub fn azathoth_at_heart(b: &mut Batch, c: &Content, e: &Enemy, alpha: i32, target: V3, sleep: i32) {
    let d = &c.enemies[e.def];
    let at = e.prev.lerp(e.pos, alpha);
    let r = d.radius;
    let awake = ONE - sleep.clamp(0, ONE);
    let t = e.age;
    let flash = e.flash < 4;
    let paint = |x: Rgb| {
        if flash {
            crate::render::mix(x, [ONE; 3], ONE * 6 / 10)
        } else {
            x
        }
    };
    let flesh = paint(c.pal.c("god_flesh"));
    let deep = paint(c.pal.c("eldritch_flesh"));
    let pale = paint(c.pal.c("god_flesh_pale"));
    let to = target.sub(at).norm();
    // the bubbles: heaving, more slowly the deeper it sleeps
    for k in 0..BUBBLES {
        let dir = on_sphere(k, BUBBLES);
        let heave = fx::mul(fx::sin(t * deg(2) + k * deg(53)), fx::mul(awake, r / 10));
        let size = r * (5 + scatter(k, 1) * 5 / ONE) / 10 + heave;
        let p = at.add(dir.scale(r * 7 / 10 + heave / 2));
        let rgb = match k % 3 {
            0 => flesh,
            1 => deep,
            _ => pale,
        };
        // the dreamer: part of it in negative
        b.style = if e.phase >= 2 && k % 2 == 0 {
            PASS_NEGATIVE
        } else {
            PASS_SOLID
        };
        b.inked(Mesh::Sphere, &boxed(p, v3(size, size, size)), rgb, 0);
    }
    b.style = PASS_SOLID;
    // the eyes: a share open by phase, and fewer as it sleeps
    let base = match e.phase {
        0 => EYES / 3,
        1 => EYES * 2 / 3,
        _ => EYES,
    };
    let open = fx::mul(base * ONE, awake) / ONE;
    let white = c.pal.c("god_eye");
    let iris = c.pal.c("god_iris");
    let void = c.pal.c("eldritch_void");
    for k in 0..EYES {
        let dir = on_sphere(k * 3 + 1, EYES * 3);
        // the side toward you has the most of them
        let dir = dir.add(to.scale(ONE / 2)).norm();
        let p = at.add(dir.scale(r * 11 / 10));
        let size = r * (14 + scatter(k, 2) * 10 / ONE) / 100;
        if k < open {
            b.inked(Mesh::Sphere, &boxed(p, v3(size, size, size)), white, ONE / 4);
            let look = target.sub(p).norm();
            let ip = p.add(look.scale(size * 4 / 10));
            b.push(
                Mesh::Sphere,
                &boxed(ip, v3(size / 2, size / 2, size / 2)),
                iris,
                ONE,
                PASS_SOLID,
            );
            b.push(
                Mesh::Sphere,
                &boxed(ip.add(look.scale(size / 5)), v3(size / 4, size / 4, size / 4)),
                void,
                0,
                PASS_SOLID,
            );
        } else {
            // a lid, shut
            b.inked(
                Mesh::Sphere,
                &boxed(p, v3(size * 11 / 10, size * 5 / 10, size * 11 / 10)),
                deep,
                0,
            );
        }
    }
    // the maw, toward you: a ring of teeth round the dark
    let teeth = c.pal.c("god_teeth");
    let maw = at.add(to.scale(r * 9 / 10)).add(v3(0, -r / 4, 0));
    let mr = r * 35 / 100 + fx::mul(awake, fx::mul(fx::sin(t * deg(3)), r / 12));
    let side = to.cross(V3::UP).norm();
    let up = side.cross(to).norm();
    for k in 0..14 {
        let a = fx::TURN / 14 * k;
        let rim = maw
            .add(side.scale(fx::mul(fx::cos(a), mr)))
            .add(up.scale(fx::mul(fx::sin(a), mr)));
        let tip = rim.add(maw.sub(rim).scale(ONE * 4 / 10)).add(to.scale(r / 12));
        if let Some(af) = crate::arena::segment(rim, tip, r / 16) {
            b.inked(Mesh::Cone, &af, teeth, 0);
        }
    }
    b.push(
        Mesh::Sphere,
        &boxed(maw, v3(mr * 2, mr * 2, mr * 2)),
        void,
        0,
        PASS_SOLID,
    );
    // tendrils, hanging and swaying
    for k in 0..10 {
        let a = fx::TURN / 10 * k + deg(9);
        let mut prev = at.add(v3(
            fx::mul(fx::cos(a), r * 7 / 10),
            -r / 2,
            fx::mul(fx::sin(a), r * 7 / 10),
        ));
        for j in 1..=5 {
            let sway = fx::mul(awake, fx::sin(t * deg(2) + k * deg(40) + j * deg(25)));
            let p = prev.add(v3(fx::mul(sway, r / 8), -r / 4, fx::mul(fx::cos(a), r / 14)));
            rod(b, prev, p, r / 10 - j * r / 70, deep, 0, true);
            prev = p;
        }
    }
    // the dancers: round it, piping
    if e.phase >= 1 {
        let dancer = paint(c.pal.c("god_dancer"));
        let bone = c.pal.c("god_bone");
        for k in 0..DANCERS {
            let a = fx::TURN / DANCERS * k + fx::mul(t * deg(1) / 2, awake);
            let bob = fx::mul(awake, fx::mul(fx::sin(t * deg(5) + k * deg(70)), r / 6));
            let ring = r * 17 / 10;
            let p = at.add(v3(
                fx::mul(fx::cos(a), ring),
                -r / 3 + bob,
                fx::mul(fx::sin(a), ring),
            ));
            let tall = r * 4 / 10;
            b.inked(
                Mesh::Blob,
                &boxed(p, v3(tall / 2, tall, tall / 2)),
                dancer,
                ONE / 5,
            );
            let head = p.add(v3(0, tall * 6 / 10, 0));
            b.inked(
                Mesh::Sphere,
                &boxed(head, v3(tall / 3, tall / 3, tall / 3)),
                dancer,
                ONE / 5,
            );
            // its flute, held out to the god
            let inward = at.sub(p).norm();
            let mouth = head.add(inward.scale(tall / 6));
            let end = mouth.add(inward.scale(tall * 7 / 10)).add(v3(0, -tall / 4, 0));
            rod(b, mouth, end, tall / 18, bone, 0, true);
        }
    }
    // the windup: a sick glow
    let charge = e.glow(c);
    if charge > 0 {
        let h = fx::mul(r * 3, ONE + charge / 2);
        b.glow(
            Mesh::Sphere,
            &boxed(at, v3(h, h, h)),
            c.pal.c("eldritch_glow"),
            charge / 3,
        );
    }
}

// ---- the trumpet --------------------------------------------------------------

/// A big brass trumpet held to the mouth at `mouth`, along `aim`; where
/// the hands go (the valves).
pub fn trumpet(b: &mut Batch, c: &Content, mouth: V3, aim: V3) -> V3 {
    let brass = c.pal.c("trumpet_brass");
    let dark = c.pal.c("trumpet_dark");
    let along = |m: i32| mouth.add(aim.scale(m));
    // mouthpiece and lead pipe
    rod(b, mouth, along(u(12)), u(4), dark, 0, true);
    rod(b, along(u(12)), along(u(80)), u(5), brass, ONE / 5, true);
    // the second loop of tubing, below
    let below = v3(0, -u(9), 0);
    rod(
        b,
        along(u(25)).add(below),
        along(u(70)).add(below),
        u(5),
        brass,
        ONE / 5,
        true,
    );
    // three valves
    let valves = along(u(42));
    for k in -1..=1 {
        let p = valves.add(aim.scale(k * u(7))).add(v3(0, -u(3), 0));
        b.inked(Mesh::Cyl, &boxed(p, v3(u(6), u(18), u(6))), brass, ONE / 5);
        b.inked(
            Mesh::Cyl,
            &boxed(p.add(v3(0, u(11), 0)), v3(u(7), u(3), u(7))),
            dark,
            0,
        );
    }
    // the bell, opening away
    if let Some(af) = crate::arena::segment(along(u(118)), along(u(78)), u(36)) {
        b.inked(Mesh::Cone, &af, brass, ONE / 4);
    }
    valves
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;
    use crate::enemy::State;
    use crate::render::{PASSES, PASS_GLOW};

    fn drawn(id: &str) -> Vec<Vec<u32>> {
        let c = content();
        let def = c.enemy(id).unwrap();
        let mut e = Enemy::new(def, &c.enemies[def], v3(0, ONE * 2, 0), 1);
        e.state = State::Rest(10);
        e.age = 40;
        e.beat = 13;
        let mut b = Batch::new([0; 3]);
        super::super::enemy(&mut b, &c, &e, ONE, v3(0, ONE, ONE * 10));
        b.finish(crate::render::STATIC_BUFFER).1
    }

    fn count(draws: &[Vec<u32>], pass: u32) -> u32 {
        draws[pass as usize]
            .chunks(crate::render::DRAW_VALUES)
            .map(|d| d[3])
            .sum()
    }

    #[test]
    fn each_of_zone_fives_enemies_is_drawn_in_its_own_art() {
        for (id, pass) in [
            ("etching", PASS_DITHER),
            ("render", PASS_PS1),
            ("reprise", PASS_NEGATIVE),
            ("glyph", PASS_SOLID),
            ("toon", PASS_SOLID),
            ("recursion", PASS_SOLID),
        ] {
            let d = drawn(id);
            assert!(count(&d, pass) > 10, "{id} in pass {pass}: {}", count(&d, pass));
            // and nothing of it leaks into another style
            for other in [PASS_DITHER, PASS_PS1, PASS_NEGATIVE] {
                if other != pass {
                    assert_eq!(count(&d, other), 0, "{id} in pass {other}");
                }
            }
        }
        // a vector display's lines are light and nothing else
        let d = drawn("tesseract");
        assert_eq!(count(&d, PASS_GLOW), 32 + 16);
        for pass in 0..PASSES as u32 {
            if pass != PASS_GLOW {
                assert_eq!(count(&d, pass), 0, "tesseract in pass {pass}");
            }
        }
        // the Recursion's orders are smaller pyramids, fewer pieces
        let solid = |id| count(&drawn(id), PASS_SOLID);
        assert!(solid("recursion") > solid("recursion_minor"));
        assert!(solid("recursion_minor") > solid("recursion_least"));
    }

    #[test]
    fn the_glyph_has_two_frames_and_the_etching_holds_still_while_seen() {
        let c = content();
        let def = c.enemy("glyph").unwrap();
        let mut e = Enemy::new(def, &c.enemies[def], v3(0, ONE * 3, 0), 1);
        e.state = State::Rest(10);
        let frame = |e: &Enemy| {
            let mut b = Batch::new([0; 3]);
            super::super::enemy(&mut b, &c, e, ONE, v3(0, ONE, ONE * 10));
            b.finish(crate::render::STATIC_BUFFER).0
        };
        let a = frame(&e);
        e.moves = 1;
        assert_ne!(a, frame(&e), "a march step swaps the frame");
        let def = c.enemy("etching").unwrap();
        let mut e = Enemy::new(def, &c.enemies[def], v3(0, ONE, 0), 1);
        e.state = State::Rest(10);
        e.seen = true;
        e.age = 10;
        let a = frame(&e);
        e.age = 50;
        assert_eq!(a, frame(&e), "looked at, it does not stir");
        e.seen = false;
        assert_ne!(a, frame(&e), "unwatched, it leans");
    }

    /// The god's open irises, counted by colour.
    fn irises(phase: usize, sleep: i32) -> usize {
        let c = content();
        let def = c.enemy("azathoth").unwrap();
        let mut e = Enemy::new(def, &c.enemies[def], v3(0, ONE * 14, -ONE * 40), 1);
        e.state = State::Rest(10);
        e.phase = phase;
        let mut b = Batch::new([0; 3]);
        azathoth_at_heart(&mut b, &c, &e, ONE, v3(0, ONE, 0), sleep);
        let iris = c.pal.c("god_iris");
        let (values, _) = b.finish(crate::render::STATIC_BUFFER);
        values
            .chunks(crate::render::INSTANCE_VALUES)
            .filter(|v| v[12..15] == iris)
            .count()
    }

    #[test]
    fn the_gods_eyes_open_phase_by_phase_and_close_as_it_sleeps() {
        assert!(irises(0, 0) < irises(1, 0));
        assert!(irises(1, 0) < irises(2, 0));
        assert_eq!(irises(2, 0), EYES as usize);
        let half = irises(2, ONE / 2);
        assert!(half < irises(2, 0) && half > 0, "{half}");
        assert_eq!(irises(2, ONE), 0, "asleep, every eye is shut");
    }

    #[test]
    fn the_trumpet_is_brass_with_a_bell_and_three_valves() {
        let c = content();
        let mut b = Batch::new([0; 3]);
        let mouth = v3(0, ONE * 2, 0);
        let hands = trumpet(&mut b, &c, mouth, v3(0, 0, -ONE));
        assert!(hands.z < mouth.z, "the valves are out along the aim");
        let brass = c.pal.c("trumpet_brass");
        let (values, _) = b.finish(crate::render::STATIC_BUFFER);
        let pieces = values
            .chunks(crate::render::INSTANCE_VALUES)
            .filter(|v| v[12..15] == brass)
            .count();
        assert!(pieces >= 6, "{pieces}");
    }
}
