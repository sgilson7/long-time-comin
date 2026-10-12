//! Chumbles, and the last mission's other things. He is Sam's cat: a
//! long-haired lynx point, cream with a white ruff and belly, grey-blue
//! points (his mask, ears, legs and a plumed tail), darker ear tips and a
//! darker mask round his eyes, tabby lines on his forehead, big blue eyes
//! and a pink nose. One model draws him, posed by what he is doing: the
//! scream as he grows, the zoomies, the crouch and pounce, sitting tall on
//! the furniture, the 3 AM creep, the treat frenzy, eating, and asleep in a
//! loaf.
//!
//! The model is laid out in centimetres at his full size, from the floor
//! below his middle (x right, y up, −z forward), and scaled as one piece,
//! so his feet stay on the floor at any size.

use crate::balance::CAT_FED_TICKS;
use crate::content::{Content, EnemyDef, Rgb};
use crate::enemy::{Enemy, State};
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{boxed, mix, shade, Batch, PASS_SOLID};

/// Centimetres at his full size.
fn u(x: i32) -> i32 {
    ONE * x / 100
}

/// The radius the model is laid out for; a cat of another radius is
/// scaled to it.
const MODEL_RADIUS: i32 = ONE * 230 / 100;

/// His head, bigger than a real cat's for his body, so his face reads.
const HEAD: i32 = ONE * 13 / 10;

/// His colours, painted (a hit flashes them toward white).
struct Coat {
    cream: Rgb,
    white: Rgb,
    point: Rgb,
    light: Rgb,
    dark: Rgb,
    tabby: Rgb,
    eye: Rgb,
    pupil: Rgb,
    nose: Rgb,
    ear: Rgb,
    mouth: Rgb,
}

impl Coat {
    fn of(c: &Content, paint: &dyn Fn(Rgb) -> Rgb) -> Coat {
        let p = |n: &str| paint(c.pal.c(n));
        Coat {
            cream: p("cat_cream"),
            white: p("cat_white"),
            point: p("cat_point"),
            light: p("cat_point_light"),
            dark: p("cat_point_dark"),
            tabby: p("cat_tabby"),
            eye: c.pal.c("cat_eye"),
            pupil: c.pal.c("cat_pupil"),
            nose: p("cat_nose"),
            ear: p("cat_ear"),
            mouth: p("cat_mouth"),
        }
    }
}

/// How he holds himself: where his hindquarters, chest and head are, his
/// paws, and the rest. Positions are in the model's centimetres.
#[derive(Debug, Clone, Copy)]
struct Pose {
    hip: V3,
    chest: V3,
    head: V3,
    /// the head's pitch (up is positive), turn and tilt
    nod: i32,
    turn: i32,
    tilt: i32,
    /// paws: front left, front right, hind left, hind right
    feet: [V3; 4],
    /// the hind legs' hocks, left and right
    hocks: [V3; 2],
    /// the tail: how far up from straight back it leaves the rump, how
    /// much it bends up at each joint, and round to his right
    tail_up: i32,
    tail_bend: i32,
    tail_wrap: i32,
    /// fur, Q16 of its usual size: up when he bristles; the tail's apart
    fluff: i32,
    tail_fluff: i32,
    /// 0 pricked, ONE flat back
    ears_back: i32,
    /// 0 shut, ONE open
    eyes: i32,
    /// 0 a slit, ONE round and huge
    pupil: i32,
    /// his eyes' glow
    shine: i32,
    /// 0 shut, ONE as wide as it goes
    mouth: i32,
    /// his breath, Q16 added to his body's girth
    breathe: i32,
}

/// On all fours, alert, tail up.
fn stand() -> Pose {
    Pose {
        hip: v3(0, u(178), u(130)),
        chest: v3(0, u(188), -u(100)),
        head: v3(0, u(272), -u(212)),
        nod: 0,
        turn: 0,
        tilt: 0,
        feet: [
            v3(-u(52), 0, -u(130)),
            v3(u(52), 0, -u(130)),
            v3(-u(60), 0, u(150)),
            v3(u(60), 0, u(150)),
        ],
        hocks: [v3(-u(62), u(70), u(200)), v3(u(62), u(70), u(200))],
        tail_up: deg(62),
        tail_bend: deg(9),
        tail_wrap: 0,
        fluff: ONE,
        tail_fluff: ONE,
        ears_back: 0,
        eyes: ONE,
        pupil: ONE / 4,
        shine: 0,
        mouth: 0,
        breathe: 0,
    }
}

/// Sitting up tall, front paws together, tail round his feet.
fn sit() -> Pose {
    Pose {
        hip: v3(0, u(105), u(95)),
        chest: v3(0, u(220), -u(45)),
        head: v3(0, u(326), -u(118)),
        feet: [
            v3(-u(40), 0, -u(120)),
            v3(u(40), 0, -u(120)),
            v3(-u(82), 0, -u(30)),
            v3(u(82), 0, -u(30)),
        ],
        hocks: [v3(-u(86), u(22), u(70)), v3(u(86), u(22), u(70))],
        tail_up: -deg(30),
        tail_bend: 0,
        tail_wrap: deg(17),
        ..stand()
    }
}

/// Low, hindquarters up, ready to spring.
fn crouch() -> Pose {
    Pose {
        hip: v3(0, u(150), u(135)),
        chest: v3(0, u(112), -u(90)),
        head: v3(0, u(175), -u(225)),
        nod: deg(4),
        feet: [
            v3(-u(52), 0, -u(175)),
            v3(u(52), 0, -u(175)),
            v3(-u(64), 0, u(105)),
            v3(u(64), 0, u(105)),
        ],
        hocks: [v3(-u(66), u(40), u(180)), v3(u(66), u(40), u(180))],
        tail_up: -deg(4),
        tail_bend: deg(3),
        pupil: ONE * 7 / 10,
        ..stand()
    }
}

/// Stretched out in the air, front paws reaching, hind legs trailing;
/// paws down to land as he falls.
fn leap(falling: bool) -> Pose {
    let reach = if falling {
        [v3(-u(66), u(40), -u(290)), v3(u(66), u(40), -u(290))]
    } else {
        [v3(-u(40), u(175), -u(330)), v3(u(40), u(160), -u(320))]
    };
    Pose {
        hip: v3(0, u(195), u(150)),
        chest: v3(0, u(215), -u(110)),
        head: v3(0, u(290), -u(235)),
        nod: if falling { -deg(15) } else { deg(5) },
        feet: [
            reach[0],
            reach[1],
            v3(-u(50), u(110), u(350)),
            v3(u(50), u(125), u(340)),
        ],
        hocks: [v3(-u(56), u(150), u(270)), v3(u(56), u(155), u(265))],
        tail_up: deg(8),
        tail_bend: deg(2),
        ears_back: ONE * 6 / 10,
        pupil: ONE,
        ..stand()
    }
}

/// Belly low, head out, every step placed: the 3 AM creep.
fn creep() -> Pose {
    Pose {
        hip: v3(0, u(128), u(140)),
        chest: v3(0, u(112), -u(100)),
        head: v3(0, u(160), -u(250)),
        feet: [
            v3(-u(56), 0, -u(160)),
            v3(u(56), 0, -u(160)),
            v3(-u(64), 0, u(120)),
            v3(u(64), 0, u(120)),
        ],
        hocks: [v3(-u(66), u(52), u(190)), v3(u(66), u(52), u(190))],
        tail_up: -deg(6),
        tail_bend: deg(5),
        ears_back: ONE / 3,
        pupil: ONE,
        shine: ONE * 7 / 10,
        ..stand()
    }
}

/// Flat out, ears back, tail streaming: the zoomies.
fn run() -> Pose {
    Pose {
        hip: v3(0, u(172), u(140)),
        chest: v3(0, u(180), -u(108)),
        head: v3(0, u(250), -u(238)),
        nod: -deg(8),
        tail_up: deg(6),
        tail_bend: deg(1),
        ears_back: ONE,
        pupil: ONE * 6 / 10,
        ..stand()
    }
}

/// Crouched over his bowl, head down in it.
fn eat(age: i32) -> Pose {
    let chew = fx::sin(age * fx::TURN / 14);
    Pose {
        hip: v3(0, u(100), u(95)),
        chest: v3(0, u(172), -u(85)),
        head: v3(0, u(150) + fx::mul(chew, u(6)), -u(225)),
        nod: -deg(50),
        feet: [
            v3(-u(46), 0, -u(140)),
            v3(u(46), 0, -u(140)),
            v3(-u(82), 0, -u(30)),
            v3(u(82), 0, -u(30)),
        ],
        eyes: ONE / 3,
        ..sit()
    }
}

/// Asleep in a loaf: paws tucked, tail round him, eyes shut.
fn loaf(age: i32) -> Pose {
    Pose {
        hip: v3(0, u(88), u(80)),
        chest: v3(0, u(92), -u(60)),
        head: v3(0, u(150), -u(165)),
        nod: -deg(10),
        tilt: deg(8),
        feet: [
            v3(-u(32), 0, -u(150)),
            v3(u(32), 0, -u(150)),
            v3(-u(72), 0, u(40)),
            v3(u(72), 0, u(40)),
        ],
        hocks: [v3(-u(74), u(12), u(110)), v3(u(74), u(12), u(110))],
        tail_up: -deg(40),
        tail_bend: 0,
        tail_wrap: deg(21),
        eyes: 0,
        fluff: ONE * 11 / 10,
        breathe: fx::sin(age * fx::TURN / 200) / 22,
        ..stand()
    }
}

/// His legs through a step: `ph` the gait's angle, each paw `amp` forward
/// and back and `lift` up off the floor, at its own offset round the step.
fn step(p: &mut Pose, ph: i32, amp: i32, lift: i32, offs: [i32; 4]) {
    for (i, o) in offs.iter().enumerate() {
        let a = ph + o;
        let dz = -fx::mul(fx::sin(a), amp);
        let dy = fx::mul(fx::cos(a).max(0), lift);
        p.feet[i] = p.feet[i].add(v3(0, dy, dz));
        if i >= 2 {
            p.hocks[i - 2] = p.hocks[i - 2].add(v3(0, dy, dz * 3 / 4));
        }
    }
}

/// A walk: the paws in a lateral sequence, hind left first.
fn walk(p: &mut Pose, ph: i32) {
    let q = fx::TURN / 4;
    step(p, ph, u(48), u(34), [q, 3 * q, 0, 2 * q]);
    let bob = fx::mul(fx::sin(ph * 2), u(5));
    p.hip.y += bob;
    p.chest.y += bob;
    p.head.y += bob;
}

/// A gallop: the front paws together, then the hind, the back flexing.
fn gallop(p: &mut Pose, ph: i32) {
    let t = fx::TURN / 10;
    step(p, ph, u(110), u(70), [0, t, 5 * t, 6 * t]);
    let flex = fx::cos(ph);
    p.chest = p
        .chest
        .add(v3(0, fx::mul(fx::sin(ph), u(18)), -fx::mul(flex, u(28))));
    p.hip = p
        .hip
        .add(v3(0, -fx::mul(fx::sin(ph), u(14)), fx::mul(flex, u(28))));
    p.head.z -= fx::mul(flex, u(20));
}

/// An ellipsoid from `a` to `c`, `w` across and `h` deep.
fn span(a: V3, c: V3, w: i32, h: i32) -> Option<Affine> {
    let s = crate::arena::segment(a, c, ONE)?;
    Some(Affine {
        c0: s.c0.scale(w),
        c1: s.c1,
        c2: s.c2.scale(h),
        t: s.t,
    })
}

/// Draws into the batch through one frame; small parts are outlined only
/// when he is big enough for the line not to swallow them.
struct Pen<'a> {
    b: &'a mut Batch,
    fine: bool,
}

impl Pen<'_> {
    fn blob(&mut self, f: &Affine, at: V3, size: V3, rgb: Rgb) {
        self.b.inked(Mesh::Sphere, &f.then(&boxed(at, size)), rgb, 0);
    }

    fn span(&mut self, f: &Affine, a: V3, c: V3, w: i32, h: i32, rgb: Rgb) {
        if let Some(s) = span(a, c, w, h) {
            self.b.inked(Mesh::Sphere, &f.then(&s), rgb, 0);
        }
    }

    /// A small part: outlined only when fine.
    fn bit(&mut self, mesh: Mesh, a: &Affine, rgb: Rgb, glow: i32) {
        if self.fine {
            self.b.inked(mesh, a, rgb, glow);
        } else {
            self.b.push(mesh, a, rgb, glow, PASS_SOLID);
        }
    }

    /// A part with no outline.
    fn flat(&mut self, mesh: Mesh, a: &Affine, rgb: Rgb, glow: i32) {
        self.b.push(mesh, a, rgb, glow, PASS_SOLID);
    }
}

/// The cat, in pose `p`, through `f` (the model's frame in the world).
fn model(b: &mut Batch, k: &Coat, f: &Affine, p: &Pose, fine: bool) {
    let mut pen = Pen { b, fine };
    let fl = |x: i32| fx::mul(u(x), p.fluff);
    // his body's own frame: at his chest, −z along his spine to the front
    let spine = p.chest.sub(p.hip);
    let len = spine.len();
    let fwd = spine.norm();
    let up = V3::UP.sub(fwd.scale(fwd.y)).norm();
    let back = fwd.neg();
    let body = Affine {
        c0: up.cross(back),
        c1: up,
        c2: back,
        t: p.chest,
    };
    let bf = f.then(&body);
    let girth = ONE + p.breathe;

    // the tail: a plume of overlapping tufts, darker to its tip
    let mut at = body.apply(v3(0, u(45), len + u(105)));
    let mut dir = v3(0, fx::sin(p.tail_up), fx::cos(p.tail_up));
    let widths = [62, 76, 88, 96, 100, 98, 90, 72];
    for (j, w) in widths.iter().enumerate() {
        if j > 0 {
            dir = Affine::rot_y(p.tail_wrap)
                .then(&Affine::rot_x(-p.tail_bend))
                .apply_dir(dir);
        }
        let w = fx::mul(u(*w), p.tail_fluff);
        let zig = if j % 2 == 0 { u(7) } else { -u(7) };
        let mut next = at.add(dir.scale(u(40))).add(v3(zig, zig / 2, 0));
        // it lies along the floor, never through it
        next.y = next.y.max(w * 2 / 5);
        let rgb = match j {
            0 => k.light,
            7 => k.dark,
            6 => shade(k.point, 80),
            _ if j % 2 == 1 => shade(k.point, 94),
            _ => k.point,
        };
        pen.span(f, at.sub(dir.scale(u(28))), next.add(dir.scale(u(24))), w, w, rgb);
        at = next;
    }

    // the legs: front straight down from the shoulder; hind through the
    // hock, in fluffy britches
    for side in [-1, 1] {
        let i = (side + 1) as usize / 2;
        let foot = p.feet[i];
        let top = body.apply(v3(side * u(55), -u(30), u(15)));
        let elbow = top.lerp(foot, ONE * 45 / 100).add(v3(0, 0, u(12)));
        pen.span(f, top, elbow, fl(72), fl(76), k.light);
        pen.span(f, elbow, foot.add(v3(0, u(18), 0)), u(52), u(52), k.point);
        pen.blob(f, foot.add(v3(0, u(16), -u(8))), v3(u(62), u(34), u(74)), k.point);

        let foot = p.feet[2 + i];
        let hock = p.hocks[i];
        let top = body.apply(v3(side * u(66), -u(10), len + u(40)));
        pen.span(f, top.add(v3(0, u(30), 0)), hock, fl(104), fl(110), k.cream);
        pen.span(f, hock, foot.add(v3(0, u(16), 0)), u(48), u(48), k.point);
        pen.blob(f, foot.add(v3(0, u(15), -u(8))), v3(u(60), u(32), u(76)), k.point);
    }

    // the body: a long fluffy loaf, its back tufted, a white fringe below
    pen.span(
        f,
        body.apply(v3(0, 0, len + u(95))),
        body.apply(v3(0, 0, -u(70))),
        fx::mul(u(190), girth),
        fx::mul(u(176), girth),
        k.cream,
    );
    pen.blob(
        &bf,
        v3(0, u(4), len + u(30)),
        v3(fl(205), fl(185), u(170)),
        k.cream,
    );
    for (j, z) in [0, 1, 2, 3].iter().map(|j| (j, len * j / 3 - u(10))) {
        let s = 100 + 6 * (j % 2);
        pen.blob(
            &bf,
            v3(0, u(62), z),
            v3(fl(126), fl(84 * s / 100), u(150)),
            k.cream,
        );
        let x = if j % 2 == 0 { u(72) } else { -u(72) };
        pen.blob(&bf, v3(x, -u(8), z + u(30)), v3(fl(84), fl(124), u(140)), k.cream);
        pen.blob(
            &bf,
            v3(-x * 7 / 10, -u(66), z),
            v3(fl(96), fl(70), u(120)),
            k.white,
        );
    }

    // the ruff: white, round his neck and down his chest
    for j in 0..9 {
        let a = deg(160) + deg(25) * j;
        let at = v3(
            fx::mul(fx::cos(a), u(96)),
            u(48) + fx::mul(fx::sin(a), u(76)),
            -u(62) - u(10) * (j % 2),
        );
        pen.blob(&bf, at, v3(fl(96), fl(94), fl(86)), k.white);
    }
    pen.blob(&bf, v3(0, -u(36), -u(98)), v3(fl(156), fl(132), fl(96)), k.white);
    pen.blob(&bf, v3(0, -u(104), -u(82)), v3(fl(100), fl(92), fl(80)), k.white);
    for side in [-1, 1] {
        pen.blob(
            &bf,
            v3(side * u(52), -u(82), -u(74)),
            v3(fl(78), fl(100), fl(78)),
            k.white,
        );
    }

    head(&mut pen, k, f, p);
}

/// His head: a grey mask over cream cheeks, a white muzzle, blue eyes in
/// a darker mask, tabby lines, pink nose, tufted ears.
fn head(pen: &mut Pen, k: &Coat, f: &Affine, p: &Pose) {
    let hf = f.then(
        &Affine::translate(p.head)
            .then(&Affine::rot_y(p.turn))
            .then(&Affine::rot_x(p.nod))
            .then(&Affine::rot_z(p.tilt))
            .then(&Affine::scale(v3(HEAD, HEAD, HEAD))),
    );
    let fl = |x: i32| fx::mul(u(x), p.fluff);
    pen.blob(&hf, V3::ZERO, v3(u(150), u(128), u(138)), k.light);
    for side in [-1, 1] {
        // fluffy cheeks, wider than his skull
        pen.blob(
            &hf,
            v3(side * u(50), -u(24), -u(14)),
            v3(fl(96), fl(90), fl(96)),
            k.cream,
        );
        pen.blob(
            &hf,
            v3(side * u(80), -u(40), u(4)),
            v3(fl(56), fl(64), fl(64)),
            mix(k.cream, k.white, ONE / 2),
        );
    }
    pen.blob(&hf, v3(0, u(4), -u(36)), v3(u(116), u(100), u(72)), k.point);

    // the ears: grey, pink inside, a dark tip and a tuft; flat back when
    // he is cross or running
    for side in [-1, 1] {
        let ear = hf
            .then(&Affine::translate(v3(side * u(42), u(42), -u(4))))
            .then(&Affine::rot_z(-side * (deg(12) + fx::mul(p.ears_back, deg(58)))))
            .then(&Affine::rot_x(fx::mul(p.ears_back, deg(40))))
            .then(&Affine::scale(v3(
                ONE * 9 / 10,
                fx::mul(ONE * 9 / 10, ONE - p.ears_back * 3 / 10),
                ONE * 9 / 10,
            )));
        pen.b.inked(
            Mesh::Cone,
            &ear.then(&boxed(v3(0, u(36), 0), v3(u(64), u(80), u(36)))),
            k.point,
            0,
        );
        pen.flat(
            Mesh::Cone,
            &ear.then(&boxed(v3(0, u(32), -u(10)), v3(u(42), u(60), u(14)))),
            k.ear,
            0,
        );
        pen.flat(
            Mesh::Cone,
            &ear.then(&boxed(v3(0, u(28), -u(16)), v3(u(16), u(52), u(8)))),
            k.white,
            0,
        );
        pen.bit(
            Mesh::Cone,
            &ear.then(&boxed(v3(0, u(66), 0), v3(u(28), u(28), u(20)))),
            k.dark,
            0,
        );
        pen.bit(
            Mesh::Cone,
            &ear.then(&boxed(v3(0, u(82), 0), v3(u(7), u(16), u(7)))),
            k.dark,
            0,
        );
    }

    // tabby lines up his forehead
    for x in [-1, 0, 1] {
        let line = hf
            .then(&Affine::translate(v3(x * u(15), u(36), -u(56))))
            .then(&Affine::rot_x(-deg(38)))
            .then(&Affine::rot_z(x * deg(12)));
        pen.flat(
            Mesh::Sphere,
            &line.then(&Affine::scale(v3(u(8), u(34), u(8)))),
            k.tabby,
            0,
        );
    }

    // the mask, darker round the eyes, and the eyes
    for side in [-1, 1] {
        let patch = hf
            .then(&Affine::translate(v3(side * u(34), u(6), -u(60))))
            .then(&Affine::rot_z(side * deg(12)));
        pen.flat(
            Mesh::Sphere,
            &patch.then(&Affine::scale(v3(u(72), u(54), u(24)))),
            k.dark,
            0,
        );
        let eye = v3(side * u(32), u(9), -u(71));
        if p.eyes > ONE / 5 {
            let h = fx::mul(u(38), p.eyes);
            pen.flat(
                Mesh::Sphere,
                &hf.then(&boxed(eye, v3(u(44), h, u(16)))),
                k.eye,
                p.shine,
            );
            let wide = u(6) + fx::mul(u(28), p.pupil);
            pen.flat(
                Mesh::Sphere,
                &hf.then(&boxed(eye.add(v3(0, 0, -u(5))), v3(wide, h * 85 / 100, u(6)))),
                k.pupil,
                0,
            );
            if p.eyes > ONE / 2 {
                pen.flat(
                    Mesh::Sphere,
                    &hf.then(&boxed(
                        eye.add(v3(-side * u(5), u(7), -u(8))),
                        v3(u(7), u(7), u(4)),
                    )),
                    k.white,
                    ONE / 2,
                );
            }
            if p.shine > 0 {
                pen.b.glow(
                    Mesh::Sphere,
                    &hf.then(&boxed(eye, v3(u(80), u(70), u(50)))),
                    k.eye,
                    p.shine / 2,
                );
            }
        } else {
            // shut: a curve down at each end, contented
            let mid = eye.add(v3(0, -u(4), -u(4)));
            for end in [-1, 1] {
                if let Some(s) = span(mid, eye.add(v3(end * u(17), u(3), 0)), u(6), u(6)) {
                    pen.flat(Mesh::Cyl, &hf.then(&s), k.pupil, 0);
                }
            }
        }
    }

    // the muzzle: white, a pink nose; open in a scream
    let open = fx::mul(p.mouth, u(40));
    if p.mouth > 0 {
        pen.blob(
            &hf,
            v3(0, -u(30) - open / 2, -u(58)),
            v3(u(54), u(26) + open, u(34)),
            k.mouth,
        );
        pen.flat(
            Mesh::Sphere,
            &hf.then(&boxed(
                v3(0, -u(36) - open * 3 / 4, -u(64)),
                v3(u(34), u(14), u(22)),
            )),
            k.nose,
            0,
        );
        for side in [-1, 1] {
            pen.flat(
                Mesh::Cone,
                &hf.then(
                    &Affine::translate(v3(side * u(15), -u(30), -u(70)))
                        .then(&Affine::rot_x(fx::HALF))
                        .then(&Affine::scale(v3(u(7), u(16), u(7)))),
                ),
                k.white,
                0,
            );
        }
    }
    for side in [-1, 1] {
        pen.blob(
            &hf,
            v3(side * u(15), -u(24), -u(64)),
            v3(u(44), u(34), u(36)),
            k.white,
        );
    }
    pen.blob(
        &hf,
        v3(0, -u(42) - open, -u(50)),
        v3(u(46), u(28), u(36)),
        k.white,
    );
    pen.bit(
        Mesh::Sphere,
        &hf.then(&boxed(v3(0, -u(6), -u(80)), v3(u(22), u(14), u(12)))),
        k.nose,
        0,
    );
    // whiskers
    for side in [-1, 1] {
        for j in 0..3 {
            let a = hf.apply(v3(side * u(26), -u(22), -u(74)));
            let c = hf.apply(v3(side * u(118), -u(10) - u(14) * j, -u(58) + u(8) * j));
            if let Some(s) = crate::arena::segment(a, c, fx::mul(u(2), hf.c0.len())) {
                pen.flat(Mesh::Cyl, &s, k.white, 0);
            }
        }
    }
}

/// The model's frame: at `floor` below his middle, facing `yaw`, `scale`
/// of his full size.
fn frame_of(floor: V3, yaw: i32, scale: i32) -> Affine {
    Affine::translate(floor)
        .then(&Affine::rot_y(yaw))
        .then(&Affine::scale(v3(scale, scale, scale)))
}

/// Chumbles in the fight, at `at` (his body's centre), as big as `e.size`
/// says; `fed` counts up once his last bar is empty (0 before).
pub fn chumbles(b: &mut Batch, c: &Content, e: &Enemy, d: &EnemyDef, at: V3, fed: i32) {
    let flash = e.flash < 4;
    let paint = |x: Rgb| if flash { mix(x, [ONE; 3], ONE * 7 / 10) } else { x };
    let k = Coat::of(c, &paint);
    let scale = fx::mul(e.size, fx::div(d.radius, MODEL_RADIUS));
    let floor = v3(at.x, at.y - fx::mul(d.hover, e.size), at.z);
    let f = frame_of(floor, e.yaw, scale);
    let p = pose(c, e, fed);
    // small, his outline is the thin one the cat in the bedroom has, so
    // the black line does not swallow him
    let bold = b.bold;
    b.bold = bold && scale > ONE / 2;
    model(b, &k, &f, &p, scale > ONE / 2);
    if fed > 0 {
        bowl(b, c, &f, fed);
    }
    if !e.alive() {
        dreams(b, c, &f, &p, e.age);
    }
    let glow = e.glow(c);
    if glow > 0 && e.alive() {
        let h = fx::mul(d.radius * 2, e.size);
        b.glow(
            Mesh::Sphere,
            &boxed(at, v3(h, h, h)),
            c.pal.c("cat_eye"),
            glow / 5,
        );
    }
    b.bold = bold;
}

/// How he holds himself now, by what he is doing.
fn pose(c: &Content, e: &Enemy, fed: i32) -> Pose {
    let age = e.age;
    if !e.alive() {
        return loaf(age);
    }
    if fed > 0 {
        return eat(age);
    }
    let changing = matches!(e.state, State::Changing(_));
    if changing && e.size < ONE && e.hp == e.max_hp {
        // the scream: head up, mouth wide, fur on end
        let shake = fx::sin(age * fx::TURN / 6);
        return Pose {
            head: v3(0, u(362), -u(92)),
            nod: deg(34),
            tilt: fx::mul(shake, deg(4)),
            mouth: ONE * 8 / 10 + shake / 5,
            eyes: ONE / 4,
            ears_back: ONE,
            fluff: ONE * 12 / 10,
            tail_fluff: ONE * 16 / 10,
            tail_up: deg(82),
            tail_bend: deg(3),
            tail_wrap: 0,
            ..sit()
        };
    }
    let moving = e.pos.sub(e.prev).len_xz() > ONE / 200;
    let sway = fx::mul(fx::sin(age * fx::TURN / 90), deg(10));
    let mut p = if changing {
        // between phases: sat up, cross, tail lashing
        Pose {
            ears_back: ONE / 2,
            tail_wrap: deg(17) + sway * 2,
            ..sit()
        }
    } else {
        match e.phase {
            0 | 4 if e.leap > 0 => {
                let mut p = run();
                gallop(&mut p, age * fx::TURN / 16);
                p
            }
            1 if e.leap > 0 || e.lift > 0 => leap(e.vel.y < 0),
            1 => {
                // the rump wiggles before he springs
                let mut p = crouch();
                p.hip.x += fx::mul(fx::sin(age * fx::TURN / 9), u(14));
                p.tail_wrap = fx::sin(age * fx::TURN / 11) / 12;
                p
            }
            2 if e.flying > 0 || e.diving => {
                let mut p = leap(e.diving || e.vel.y < 0);
                if e.diving {
                    p.ears_back = ONE;
                }
                p
            }
            2 => sit(),
            3 => {
                let mut p = creep();
                if moving {
                    walk(&mut p, age * fx::TURN / 36);
                }
                p.tail_wrap = sway / 6;
                p
            }
            _ => {
                let mut p = stand();
                if moving {
                    walk(&mut p, age * fx::TURN / 24);
                }
                p.tail_wrap = sway / 5;
                p
            }
        }
    };
    if e.phase == 4 {
        // the treats: frenzied, every hair on end, the tail a bottle-brush
        p.fluff = ONE * 12 / 10;
        p.tail_fluff = ONE * 17 / 10;
        p.ears_back = ONE;
        p.pupil = ONE;
    }
    if e.phase == 3 {
        p.pupil = ONE;
        p.shine = p.shine.max(ONE * 7 / 10);
    }
    // before he strikes: eyes flare, fur bristles
    let glow = e.glow(c);
    p.shine = p.shine.max(glow);
    p.fluff += glow / 8;
    p.tail_fluff += glow / 4;
    // breathing, a blink, an ear's twitch
    p.breathe = fx::sin(age * fx::TURN / 120) / 40;
    if (age + 90) % 190 < 6 && p.eyes == ONE && p.shine == 0 {
        p.eyes = 0;
    }
    if age % 233 < 9 {
        p.ears_back = p.ears_back.max(ONE / 3);
    }
    p
}

/// The bowl he eats from, emptying as he eats.
fn bowl(b: &mut Batch, c: &Content, f: &Affine, fed: i32) {
    let blue = c.pal.c("cat_bowl");
    let treat = c.pal.c("treat");
    let at = v3(0, u(24), -u(330));
    b.inked(Mesh::Cyl, &f.then(&boxed(at, v3(u(200), u(48), u(200)))), blue, 0);
    b.push(
        Mesh::Cyl,
        &f.then(&boxed(at.add(v3(0, u(24), 0)), v3(u(160), u(4), u(160)))),
        shade(blue, 70),
        0,
        PASS_SOLID,
    );
    let left = 9 - 9 * fed.min(CAT_FED_TICKS) / CAT_FED_TICKS;
    for j in 0..left {
        let a = deg(137) * j;
        let r = u(12) + u(6) * j;
        let p = at.add(v3(
            fx::mul(fx::cos(a), r),
            u(26) + u(4) * (j % 2),
            fx::mul(fx::sin(a), r),
        ));
        let t = f
            .then(&Affine::translate(p))
            .then(&Affine::rot_y(a))
            .then(&Affine::scale(v3(u(24), u(12), u(16))));
        b.inked(Mesh::Cube, &t, treat, 0);
    }
}

/// Asleep: a Z or two rising off him.
fn dreams(b: &mut Batch, c: &Content, f: &Affine, p: &Pose, age: i32) {
    let white = c.pal.c("cat_white");
    for j in 0..3 {
        let t = (age + j * 70) % 210;
        let s = u(30) + u(30) * t / 210;
        let at = p.head.add(v3(u(40) + u(30) * t / 210, u(110) + u(2) * t, u(0)));
        let z = f.then(&Affine::translate(at));
        let bar = |y: i32| boxed(v3(0, y, 0), v3(s, s / 6, s / 6));
        b.inked(Mesh::Cube, &z.then(&bar(s / 2)), white, 0);
        b.inked(Mesh::Cube, &z.then(&bar(-s / 2)), white, 0);
        let slash = Affine::rot_z(deg(45)).then(&Affine::scale(v3(s * 14 / 10, s / 6, s / 6)));
        b.inked(Mesh::Cube, &z.then(&slash), white, 0);
    }
}

/// The cat in the bedroom, walking: at its feet `at`, facing `yaw`, its
/// legs `stride` round their step, `size` as for `chumbles`.
pub fn guide(b: &mut Batch, c: &Content, at: V3, yaw: i32, stride: i32, size: i32, radius: i32) {
    let k = Coat::of(c, &|x| x);
    let scale = fx::mul(size, fx::div(radius, MODEL_RADIUS));
    let f = frame_of(at, yaw, scale);
    // one step for every 2.4 m of his own, at his size
    let cycle = fx::mul(u(240), scale).max(1);
    let ph = fx::mul(fx::div(stride, cycle), fx::TURN);
    let mut p = stand();
    walk(&mut p, ph);
    p.tail_up = deg(80);
    p.tail_bend = deg(6);
    p.tail_wrap = fx::mul(fx::sin(ph / 2), deg(5));
    model(b, &k, &f, &p, scale > ONE / 2);
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
    match d.id.as_str() {
        "yarn" => yarn(b, c, e, d, at, s, paint),
        _ => return false,
    }
    true
}

/// A ball of red yarn, rolling the way it goes, wound round with strands,
/// a loose end trailing behind it.
fn yarn(b: &mut Batch, c: &Content, e: &Enemy, d: &EnemyDef, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let red = paint(c.pal.c("yarn"));
    let dark = shade(red, 72);
    let moved = e.pos.sub(e.prev);
    let heading = if moved.len_xz() > ONE / 200 {
        crate::geom::yaw_pitch_of(moved).0
    } else {
        e.yaw
    };
    let roll = e.age * deg(12);
    let f = Affine::translate(at)
        .then(&Affine::rot_y(heading))
        .then(&Affine::rot_x(-roll));
    b.inked(Mesh::Sphere, &f.then(&Affine::scale(v3(s, s, s))), red, 0);
    // the strands wound round it, each a band on a great circle
    let r = s * 102 / 100;
    for j in 0..6 {
        let band = f
            .then(&Affine::rot_y(deg(31) * j))
            .then(&Affine::rot_x(deg(62) + deg(9) * j))
            .then(&Affine::scale(v3(r, s / 14, r)));
        b.push(
            Mesh::Ring,
            &band,
            if j % 2 == 0 { dark } else { shade(red, 115) },
            0,
            PASS_SOLID,
        );
    }
    // the loose end, wriggling along the floor behind
    let floor = at.y - fx::mul(d.hover, fx::div(s, d.radius * 2));
    let behind = crate::geom::facing(heading, 0).neg();
    let across = v3(-behind.z, 0, behind.x);
    let mut p = at.add(behind.scale(s / 2));
    p.y = floor + s / 6;
    for j in 1..6 {
        let wig = fx::sin(e.age * fx::TURN / 20 + j * deg(70));
        let q = v3(at.x, floor + ONE / 30, at.z)
            .add(behind.scale(s / 2 + s * j * 3 / 10))
            .add(across.scale(fx::mul(wig, s / 6)));
        super::rod(b, p, q, s / 22, red, 0, false);
        p = q;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::balance::CAT_SMALL;

    fn cat() -> (Content, Enemy) {
        let c = crate::content::tests::content();
        let k = c.enemy("chumbles").unwrap();
        let d = &c.enemies[k];
        let mut e = Enemy::new(k, d, v3(0, d.hover, 0), 1);
        e.size = ONE;
        e.state = State::Rest(10);
        e.age = 50;
        (c, e)
    }

    /// Every instance's centre, and how many there were.
    fn drawn(f: impl FnOnce(&mut Batch)) -> (Vec<V3>, usize) {
        let mut b = Batch::new([0; 3]);
        f(&mut b);
        let n = b.count();
        let (values, _) = b.finish(crate::render::DYNAMIC_BUFFER);
        let at = values
            .chunks(crate::render::INSTANCE_VALUES)
            .map(|v| v3(v[3], v[7], v[11]))
            .collect();
        (at, n)
    }

    #[test]
    fn chumbles_fills_the_sphere_he_is_hit_by_and_stands_on_the_floor() {
        let (c, e) = cat();
        let d = &c.enemies[e.def];
        let (at, _) = drawn(|b| chumbles(b, &c, &e, d, e.pos, 0));
        let lo = at.iter().map(|p| p.y).min().unwrap();
        let hi = at.iter().map(|p| p.y).max().unwrap();
        let near = at.iter().map(|p| p.z).min().unwrap();
        let far = at.iter().map(|p| p.z).max().unwrap();
        // his paws on the floor, his ears near the top of the sphere, and
        // nose to tail across its width
        assert!(lo.abs() < ONE / 4, "lowest part at {lo}");
        assert!(hi > d.hover + d.radius * 6 / 10, "highest part at {hi}");
        assert!(far - near > d.radius * 2, "{near}..{far}");
    }

    #[test]
    fn small_he_is_a_third_of_his_size_with_his_feet_still_on_the_floor() {
        let (c, mut e) = cat();
        let d = &c.enemies[e.def];
        e.size = CAT_SMALL;
        e.pos.y = fx::mul(d.hover, CAT_SMALL);
        let (at, _) = drawn(|b| chumbles(b, &c, &e, d, e.pos, 0));
        let lo = at.iter().map(|p| p.y).min().unwrap();
        let hi = at.iter().map(|p| p.y).max().unwrap();
        assert!(lo.abs() < ONE / 10, "lowest part at {lo}");
        assert!(hi < (d.hover + d.radius) / 2, "highest part at {hi}");
        // and the cat in the bedroom is the same cat
        let (g, _) = drawn(|b| guide(b, &c, V3::ZERO, e.yaw, 0, CAT_SMALL, d.radius));
        let ghi = g.iter().map(|p| p.y).max().unwrap();
        assert!((ghi - hi).abs() < ONE / 2, "{ghi} {hi}");
    }

    #[test]
    fn every_pose_keeps_to_its_part_budget() {
        let (c, e) = cat();
        let d = &c.enemies[e.def];
        let poses: Vec<(&str, Enemy, i32)> = vec![
            ("stand", e.clone(), 0),
            (
                "scream",
                Enemy {
                    size: ONE / 2,
                    state: State::Changing(30),
                    ..e.clone()
                },
                0,
            ),
            ("run", Enemy { leap: 5, ..e.clone() }, 0),
            (
                "crouch",
                Enemy {
                    phase: 1,
                    ..e.clone()
                },
                0,
            ),
            (
                "pounce",
                Enemy {
                    phase: 1,
                    leap: 5,
                    ..e.clone()
                },
                0,
            ),
            (
                "perched",
                Enemy {
                    phase: 2,
                    ..e.clone()
                },
                0,
            ),
            (
                "creep",
                Enemy {
                    phase: 3,
                    ..e.clone()
                },
                0,
            ),
            (
                "frenzy",
                Enemy {
                    phase: 4,
                    leap: 5,
                    ..e.clone()
                },
                0,
            ),
            ("eating", e.clone(), 30),
            ("asleep", Enemy { hp: 0, ..e.clone() }, CAT_FED_TICKS),
        ];
        for (name, e, fed) in poses {
            let (_, n) = drawn(|b| chumbles(b, &c, &e, d, e.pos, fed));
            assert!((100..=300).contains(&n), "{name}: {n} instances");
        }
    }

    #[test]
    fn asleep_his_eyes_are_shut_and_eating_he_has_a_bowl() {
        let (c, e) = cat();
        let shut = pose(&c, &Enemy { hp: 0, ..e.clone() }, CAT_FED_TICKS);
        assert_eq!(shut.eyes, 0);
        let eating = pose(&c, &e, 30);
        assert!(eating.nod < 0 && eating.head.y < stand().head.y * 2 / 3);
        let screaming = pose(
            &c,
            &Enemy {
                size: ONE / 2,
                state: State::Changing(30),
                ..e.clone()
            },
            0,
        );
        assert!(screaming.mouth > ONE / 2 && screaming.nod > 0);
    }
}
