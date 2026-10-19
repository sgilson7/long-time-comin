//! The Far Shore's Drowned Fleet and its beasts, Dagon and its tentacles,
//! and the Vault's machines. Each is laid out in centimetres in its own
//! frame (x right, y up, −z forward), grown as it spawns. The pirates come
//! from every century the anomaly has taken, and its light is in their
//! eyes, a faint green.

use super::rod;
use crate::content::{Content, EnemyDef, Rgb};
use crate::enemy::Enemy;
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{boxed, mix, shade, Batch, PASS_SOLID};

/// Draws one of these; false for any other, which is drawn elsewhere.
pub fn draw(
    b: &mut Batch,
    c: &Content,
    e: &Enemy,
    d: &EnemyDef,
    at: V3,
    s: i32,
    paint: &dyn Fn(Rgb) -> Rgb,
) -> bool {
    let o = Body::new(at, e, d, s);
    match d.id.as_str() {
        "deckhand" => deckhand(b, c, e, &o.feet(d), paint),
        "gunner" => gunner(b, c, e, &o.feet(d), paint),
        "powder_monkey" => powder_monkey(b, c, e, &o.feet(d), paint),
        "lookout" => lookout(b, c, e, &o.feet(d), paint),
        "bosun" => bosun(b, c, e, &o.feet(d), paint),
        "drowned" => drowned(b, c, e, &o.feet(d), paint),
        "cockatrice" => cockatrice(b, c, e, &o, paint),
        "chimera" => chimera(b, c, e, &o, paint),
        "storm_harpy" => storm_harpy(b, c, e, &o, paint),
        "captured_dragon" => dragon(b, c, e, &o, paint),
        "chain_lock" => chain_lock(b, c, e, &o.feet(d), paint),
        "tentacle" => tentacle(b, c, e, &o.feet(d), paint),
        "dagon" => dagon(b, c, e, &o, paint),
        "custodian" => custodian(b, c, e, &o.feet(d), paint),
        "ward" => ward(b, c, e, &o, paint),
        "excisor" => excisor(b, c, e, &o, paint),
        "archivist" => archivist(b, c, e, &o, paint),
        _ => return false,
    }
    true
}

fn cm(x: i32) -> i32 {
    ONE * x / 100
}

fn at_cm(p: [i32; 3]) -> V3 {
    v3(cm(p[0]), cm(p[1]), cm(p[2]))
}

/// A body's frame in metres, and how far it has grown.
#[derive(Clone)]
struct Body {
    f: Affine,
    k: i32,
    age: i32,
}

impl Body {
    fn new(at: V3, e: &Enemy, d: &EnemyDef, s: i32) -> Body {
        let k = fx::div(s, (d.radius * 2).max(1));
        Body {
            f: Affine::translate(at)
                .then(&Affine::rot_y(e.yaw))
                .then(&Affine::scale(v3(k, k, k))),
            k,
            age: e.age,
        }
    }

    /// The same frame moved down to the floor it stands on.
    fn feet(&self, d: &EnemyDef) -> Body {
        self.moved(v3(0, -d.hover, 0))
    }

    fn moved(&self, by: V3) -> Body {
        Body {
            f: self.f.then(&Affine::translate(by)),
            ..self.clone()
        }
    }

    /// The same frame, `t` times as big.
    fn scaled(&self, t: i32) -> Body {
        Body {
            f: self.f.then(&Affine::scale(v3(t, t, t))),
            k: fx::mul(self.k, t),
            age: self.age,
        }
    }

    /// A point, in the world.
    fn p(&self, p: [i32; 3]) -> V3 {
        self.f.apply(at_cm(p))
    }

    /// A width, in the world.
    fn w(&self, x: i32) -> i32 {
        fx::mul(cm(x), self.k)
    }

    /// A unit shape at `p`, `size` across.
    fn at(&self, p: [i32; 3], size: [i32; 3]) -> Affine {
        self.f.then(&boxed(at_cm(p), at_cm(size)))
    }

    /// A unit shape at `p`, turned by `r`, `size` across.
    fn place(&self, p: [i32; 3], r: &Affine, size: [i32; 3]) -> Affine {
        self.f
            .then(&Affine::translate(at_cm(p)))
            .then(r)
            .then(&Affine::scale(at_cm(size)))
    }
}

/// A rod from `a` to `z`, `w` across, all in centimetres of `o`.
fn bar(b: &mut Batch, o: &Body, a: [i32; 3], z: [i32; 3], w: i32, rgb: Rgb, ink: bool) {
    rod(b, o.p(a), o.p(z), o.w(w), rgb, 0, ink);
}

/// A flat blade from `a` to `z`, `w` across.
fn blade(b: &mut Batch, o: &Body, a: [i32; 3], z: [i32; 3], w: i32, rgb: Rgb) {
    if let Some(s) = crate::arena::segment(o.p(a), o.p(z), o.w(w)) {
        let s = Affine {
            c2: s.c2.scale(ONE / 4),
            ..s
        };
        b.inked(Mesh::Cube, &s, rgb, 0);
    }
}

/// A wing on `side`, its root at `root`; `size` is its span, its chord
/// and how far it is beaten up. A webbed wing has bones along its leading
/// edge. Returns the frame of its outer half, whose root is the elbow.
fn wing(
    b: &mut Batch,
    o: &Body,
    side: i32,
    root: [i32; 3],
    size: [i32; 3],
    rgb: Rgb,
    bone: Option<Rgb>,
) -> Affine {
    let [span, chord, beat] = size;
    let thin = cm(span / 30 + 1);
    let inner =
        o.f.then(&Affine::translate(at_cm(root)))
            .then(&Affine::rot_z(side * beat));
    b.inked(
        Mesh::Wedge,
        &inner.then(&boxed(
            v3(cm(side * span / 4), 0, 0),
            v3(cm(span / 2), thin, cm(chord)),
        )),
        rgb,
        0,
    );
    let outer = inner
        .then(&Affine::translate(v3(cm(side * span / 2), 0, 0)))
        .then(&Affine::rot_z(side * beat / 2));
    b.inked(
        Mesh::Wedge,
        &outer.then(&boxed(
            v3(cm(side * span / 4), 0, cm(chord / 8)),
            v3(cm(span / 2), thin, cm(chord * 3 / 4)),
        )),
        shade(rgb, 90),
        0,
    );
    if let Some(bone) = bone {
        let front = -cm(chord / 2);
        let shoulder = inner.apply(v3(0, 0, front));
        let elbow = outer.apply(v3(0, 0, front));
        let tip = outer.apply(v3(cm(side * span / 2), 0, front / 2));
        rod(b, shoulder, elbow, o.w(span / 20), bone, 0, false);
        rod(b, elbow, tip, o.w(span / 30), bone, 0, false);
    }
    outer
}

/// A serpent's tail from `root`, back along +z: `shape` is how many
/// segments, how long each, how wide at its root and how far each rises.
/// It sways, thinner to its tip; returns the tip.
fn tail(b: &mut Batch, o: &Body, root: [i32; 3], shape: [i32; 4], rgb: Rgb) -> [i32; 3] {
    let [n, len, w, rise] = shape;
    let mut p = root;
    for j in 0..n {
        let sway = fx::sin(o.age * fx::TURN / 50 + j * deg(50));
        let q = [p[0] + fx::mul(sway, len * 6 / 10), p[1] + rise, p[2] + len];
        bar(
            b,
            o,
            p,
            q,
            w * (n + 1 - j) / (n + 1),
            shade(rgb, 100 - 4 * j),
            true,
        );
        p = q;
    }
    p
}

enum Hat {
    Tricorn(Rgb),
    Bandana(Rgb),
    Cap(Rgb),
    /// no hat: a fish's head where theirs was
    Fish,
}

/// A pirate: how tall (per cent of 1.75 m), how broad (per cent), their
/// coat and their hat.
struct Crew {
    tall: i32,
    broad: i32,
    coat: Rgb,
    hat: Hat,
}

/// A pirate standing on `o`'s origin, facing −z, their hands at `hands`
/// (centimetres of a pirate 1.75 m tall). Returns their frame, for what
/// they hold.
fn pirate(
    b: &mut Batch,
    c: &Content,
    e: &Enemy,
    o: &Body,
    crew: &Crew,
    hands: [[i32; 3]; 2],
    paint: &dyn Fn(Rgb) -> Rgb,
) -> Body {
    let o = o.scaled(ONE * crew.tall / 100);
    let skin = paint(mix(c.pal.c("agent_skin"), c.pal.c("dagon_teal"), ONE / 5));
    let dark = paint(c.pal.c("hull_dark"));
    let anomaly = c.pal.c("biolume");
    let bw = |x: i32| x * crew.broad / 100;
    let stride = if e.vel.len_xz() > 0 {
        fx::sin(e.age * fx::TURN / 24)
    } else {
        0
    };
    for side in [-1, 1] {
        let swing = side * fx::mul(stride, 14);
        bar(
            b,
            &o,
            [side * bw(12), 88, 0],
            [side * bw(14), 8, swing],
            15,
            dark,
            true,
        );
        b.push(
            Mesh::Cube,
            &o.at([side * bw(14), 6, swing - 4], [14, 12, 26]),
            shade(dark, 70),
            0,
            PASS_SOLID,
        );
    }
    b.inked(Mesh::Sphere, &o.at([0, 120, 0], [bw(50), 72, 34]), crew.coat, 0);
    b.push(
        Mesh::Sphere,
        &o.at([0, 126, -11], [bw(22), 46, 14]),
        paint(c.pal.c("sail")),
        0,
        PASS_SOLID,
    );
    b.push(Mesh::Cyl, &o.at([0, 92, 0], [bw(44), 9, 30]), dark, 0, PASS_SOLID);
    b.push(
        Mesh::Cube,
        &o.at([0, 92, -15], [8, 7, 3]),
        c.pal.c("brass"),
        ONE / 4,
        PASS_SOLID,
    );
    for (side, h) in [-1, 1].into_iter().zip(hands) {
        bar(b, &o, [side * bw(24), 146, 0], h, 11, crew.coat, true);
        b.push(Mesh::Sphere, &o.at(h, [10, 10, 10]), skin, 0, PASS_SOLID);
    }
    if let Hat::Fish = crew.hat {
        let teal = paint(c.pal.c("dagon_teal"));
        b.inked(Mesh::Sphere, &o.at([0, 158, -6], [28, 30, 42]), teal, 0);
        b.push(
            Mesh::Sphere,
            &o.at([0, 149, -26], [16, 6, 6]),
            c.pal.c("ocean_deep"),
            0,
            PASS_SOLID,
        );
        b.push(
            Mesh::Wedge,
            &o.at([0, 176, 4], [4, 18, 32]),
            shade(teal, 80),
            0,
            PASS_SOLID,
        );
        for side in [-1, 1] {
            b.push(
                Mesh::Sphere,
                &o.at([side * 13, 162, -14], [9, 9, 9]),
                anomaly,
                ONE / 2,
                PASS_SOLID,
            );
        }
        return o;
    }
    b.inked(Mesh::Sphere, &o.at([0, 160, -2], [28, 30, 28]), skin, 0);
    for side in [-1, 1] {
        b.push(
            Mesh::Sphere,
            &o.at([side * 6, 162, -14], [5, 4, 3]),
            anomaly,
            ONE / 3,
            PASS_SOLID,
        );
    }
    match crew.hat {
        Hat::Tricorn(h) => {
            b.inked(Mesh::Cyl, &o.at([0, 174, 0], [48, 4, 42]), h, 0);
            b.push(Mesh::Sphere, &o.at([0, 178, 0], [28, 14, 26]), h, 0, PASS_SOLID);
            b.push(
                Mesh::Ring,
                &o.at([0, 175, 0], [49, 3, 43]),
                c.pal.c("brass"),
                0,
                PASS_SOLID,
            );
        }
        Hat::Bandana(h) => {
            b.inked(Mesh::Sphere, &o.at([0, 168, 1], [31, 20, 31]), h, 0);
            b.push(Mesh::Sphere, &o.at([0, 166, 16], [9, 9, 9]), h, 0, PASS_SOLID);
        }
        Hat::Cap(h) => {
            b.inked(Mesh::Sphere, &o.at([0, 170, 0], [30, 18, 30]), h, 0);
            b.push(
                Mesh::Ring,
                &o.at([0, 166, 0], [31, 5, 31]),
                shade(h, 80),
                0,
                PASS_SOLID,
            );
        }
        Hat::Fish => {}
    }
    o
}

/// A deckhand: a cutlass in one hand, raised through a windup, a pistol
/// in the other.
fn deckhand(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let lift = fx::mul(g, 40);
    let crew = Crew {
        tall: 100,
        broad: 100,
        coat: paint(c.pal.c("pirate_navy")),
        hat: Hat::Bandana(paint(c.pal.c("pirate_red"))),
    };
    let hilt = [32, 118 + lift, -30];
    let p = pirate(b, c, e, o, &crew, [[-30, 108, -22], hilt], paint);
    // the cutlass: forward at rest, back over the shoulder to cut
    let tip = [34, 180 + lift, -58 + fx::mul(g, 76)];
    blade(b, &p, [32, 124 + lift, -32], tip, 7, paint(c.pal.c("chrome")));
    b.push(
        Mesh::Cube,
        &p.at([32, 123 + lift, -31], [14, 3, 6]),
        c.pal.c("brass"),
        0,
        PASS_SOLID,
    );
    // the pistol
    bar(
        b,
        &p,
        [-30, 112, -24],
        [-30, 114, -50],
        5,
        paint(c.pal.c("iron")),
        true,
    );
    b.push(
        Mesh::Cube,
        &p.at([-30, 104, -22], [5, 12, 7]),
        paint(c.pal.c("hull")),
        0,
        PASS_SOLID,
    );
}

/// A gunner behind a deck cannon on its carriage, a pile of shot beside
/// it; the fuse at its breech sputters brighter through a windup.
fn gunner(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let iron = paint(c.pal.c("iron"));
    let wood = paint(c.pal.c("hull"));
    let crew = Crew {
        tall: 100,
        broad: 105,
        coat: wood,
        hat: Hat::Cap(paint(c.pal.c("pirate_red"))),
    };
    pirate(
        b,
        c,
        e,
        &o.moved(v3(0, 0, cm(45))),
        &crew,
        [[-18, 96, -40], [18, 96, -40]],
        paint,
    );
    b.inked(Mesh::Cube, &o.at([0, 32, -46], [58, 34, 92]), wood, 0);
    bar(b, o, [0, 64, 0], [0, 88, -112], 30, iron, true);
    b.push(Mesh::Sphere, &o.at([0, 62, 6], [16, 16, 16]), iron, 0, PASS_SOLID);
    if let Some(m) = crate::arena::segment(o.p([0, 87, -108]), o.p([0, 88, -114]), o.w(36)) {
        b.push(Mesh::Ring, &m, c.pal.c("brass"), 0, PASS_SOLID);
    }
    for side in [-1, 1] {
        b.inked(
            Mesh::Cyl,
            &o.place([side * 34, 24, -50], &Affine::rot_z(fx::QUARTER), [44, 10, 44]),
            paint(c.pal.c("hull_dark")),
            0,
        );
    }
    for p in [[-40, 8, 20], [-24, 8, 24], [-32, 20, 22]] {
        b.push(Mesh::Sphere, &o.at(p, [16, 16, 16]), iron, 0, PASS_SOLID);
    }
    let ember = c.pal.c("ember");
    b.push(
        Mesh::Sphere,
        &o.at([0, 74, 4], [7, 7, 7]),
        ember,
        ONE / 3 + g * 2 / 3,
        PASS_SOLID,
    );
    if g > 0 {
        let h = o.w(20 + fx::mul(g, 40));
        b.glow(Mesh::Sphere, &boxed(o.p([0, 74, 4]), v3(h, h, h)), ember, g / 2);
    }
}

/// A powder monkey: a small one running with a lit keg over their head.
fn powder_monkey(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let crew = Crew {
        tall: 72,
        broad: 85,
        coat: paint(c.pal.c("sail")),
        hat: Hat::Bandana(paint(c.pal.c("pirate_red"))),
    };
    let p = pirate(b, c, e, o, &crew, [[-20, 186, -4], [20, 186, -4]], paint);
    let side = Affine::rot_z(fx::QUARTER);
    b.inked(
        Mesh::Cyl,
        &p.place([0, 202, -4], &side, [38, 46, 38]),
        paint(c.pal.c("hull")),
        0,
    );
    for x in [-14, 14] {
        b.push(
            Mesh::Ring,
            &p.place([x, 202, -4], &side, [40, 5, 40]),
            paint(c.pal.c("iron")),
            0,
            PASS_SOLID,
        );
    }
    bar(
        b,
        &p,
        [0, 220, -4],
        [6, 234, -6],
        3,
        paint(c.pal.c("hull_dark")),
        false,
    );
    let ember = c.pal.c("ember");
    let flick = (e.age / 3) % 2;
    b.push(
        Mesh::Sphere,
        &p.at([6, 236, -6], [8 + 4 * flick, 8 + 4 * flick, 8 + 4 * flick]),
        ember,
        ONE,
        PASS_SOLID,
    );
    let h = p.w(36 + 10 * flick);
    b.glow(
        Mesh::Sphere,
        &boxed(p.p([6, 236, -6]), v3(h, h, h)),
        ember,
        ONE / 4 + g / 2,
    );
}

/// A lookout in a crow's nest on its mast, a spyglass to one eye and a
/// long rifle that comes up to aim through a windup.
fn lookout(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let wood = paint(c.pal.c("hull"));
    let dark = paint(c.pal.c("hull_dark"));
    bar(b, o, [0, 0, 0], [0, -260, 0], 24, dark, true);
    let crew = Crew {
        tall: 100,
        broad: 100,
        coat: paint(c.pal.c("pirate_navy")),
        hat: Hat::Tricorn(dark),
    };
    let raise = fx::mul(g, 22);
    let p = pirate(b, c, e, o, &crew, [[-8, 156, -26], [22, 122 + raise, -24]], paint);
    b.inked(Mesh::Cyl, &o.at([0, 52, 0], [112, 104, 112]), wood, 0);
    b.push(Mesh::Ring, &o.at([0, 104, 0], [118, 8, 118]), dark, 0, PASS_SOLID);
    let light = c.pal.c("lightning");
    bar(b, &p, [-7, 162, -16], [-7, 164, -56], 7, c.pal.c("brass"), true);
    b.push(
        Mesh::Sphere,
        &p.at([-7, 164, -57], [8, 8, 3]),
        light,
        ONE / 2 + g / 2,
        PASS_SOLID,
    );
    let tip = [18, 136 + raise, -100];
    bar(b, &p, [22, 128 + raise, 14], tip, 5, paint(c.pal.c("iron")), true);
    b.push(
        Mesh::Wedge,
        &p.at([22, 126 + raise, 10], [7, 12, 26]),
        wood,
        0,
        PASS_SOLID,
    );
    if g > 0 {
        let h = p.w(12 + fx::mul(g, 30));
        b.glow(Mesh::Sphere, &boxed(p.p(tip), v3(h, h, h)), light, g / 2);
    }
}

/// A bosun: big, bearded, a whip trailing from one raised hand that
/// whirls over their head through a windup.
fn bosun(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let dark = paint(c.pal.c("hull_dark"));
    let lift = fx::mul(g, 22);
    let crew = Crew {
        tall: 118,
        broad: 140,
        coat: paint(c.pal.c("pirate_red")),
        hat: Hat::Tricorn(paint(c.pal.c("pirate_navy"))),
    };
    let p = pirate(b, c, e, o, &crew, [[-34, 100, -8], [36, 150 + lift, -14]], paint);
    b.push(
        Mesh::Sphere,
        &p.at([0, 151, -12], [24, 16, 14]),
        dark,
        0,
        PASS_SOLID,
    );
    for side in [-1, 1] {
        b.push(
            Mesh::Sphere,
            &p.at([side * 32, 146, 0], [14, 6, 14]),
            c.pal.c("brass"),
            0,
            PASS_SOLID,
        );
    }
    let mut q = [40, 160 + lift, -26];
    bar(b, &p, [36, 150 + lift, -14], q, 5, paint(c.pal.c("hull")), true);
    for j in 0..7 {
        let a = e.age * fx::TURN / 40 + j * deg(40);
        let step = if g > 0 {
            [fx::mul(fx::cos(a), 22), 4, fx::mul(fx::sin(a), 22)]
        } else {
            [fx::mul(fx::sin(a), 10), -20, 10 + fx::mul(fx::cos(a), 6)]
        };
        let next = [q[0] + step[0], q[1] + step[1], q[2] + step[2]];
        bar(b, &p, q, next, 3, dark, false);
        q = next;
    }
    b.push(
        Mesh::Sphere,
        &p.at(q, [5, 5, 5]),
        paint(c.pal.c("sail")),
        0,
        PASS_SOLID,
    );
}

/// The Drowned: a crewman gone fish-like in the deep, a fish's head and a
/// fin down their back, leaning into their surge with clawed hands out.
fn drowned(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let teal = paint(c.pal.c("dagon_teal"));
    let lean = Body {
        f: o.f.then(&Affine::rot_x(-deg(15))),
        ..o.clone()
    };
    let crew = Crew {
        tall: 95,
        broad: 90,
        coat: paint(mix(c.pal.c("pirate_navy"), c.pal.c("dagon_teal"), ONE / 3)),
        hat: Hat::Fish,
    };
    let reach = fx::mul(fx::sin(e.age * fx::TURN / 30), 6);
    let p = pirate(
        b,
        c,
        e,
        &lean,
        &crew,
        [[-22, 120 + reach, -50], [22, 120 - reach, -50]],
        paint,
    );
    b.inked(Mesh::Wedge, &p.at([0, 140, 16], [4, 30, 28]), teal, 0);
    for side in [-1, 1] {
        b.push(
            Mesh::Wedge,
            &p.at([side * 22, 118, -56], [10, 3, 14]),
            shade(teal, 85),
            0,
            PASS_SOLID,
        );
        b.push(
            Mesh::Cube,
            &p.at([side * 10, 146, -10], [3, 8, 6]),
            paint(c.pal.c("dagon_flesh")),
            0,
            PASS_SOLID,
        );
    }
    b.push(
        Mesh::Wedge,
        &p.at([0, 88, 16], [30, 30, 10]),
        shade(crew.coat, 80),
        0,
        PASS_SOLID,
    );
}

/// A cockatrice: a rooster's head with its comb and wattle on a serpent's
/// tail, on bat's wings, always in the air. Its eyes glow through the
/// windup of its gaze.
fn cockatrice(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let body = paint(c.pal.c("cockatrice"));
    let red = paint(c.pal.c("pirate_red"));
    let beak = paint(c.pal.c("fauna_beak"));
    let flap = fx::mul(fx::sin(e.age * fx::TURN / 16), deg(40));
    b.inked(Mesh::Sphere, &o.at([0, 0, 0], [60, 56, 90]), body, 0);
    b.inked(
        Mesh::Sphere,
        &o.at([0, -6, -26], [46, 44, 40]),
        mix(body, c.pal.c("sail"), ONE / 3),
        0,
    );
    bar(b, o, [0, 10, -34], [0, 42, -52], 22, body, true);
    b.inked(Mesh::Sphere, &o.at([0, 48, -56], [30, 30, 32]), body, 0);
    if let Some(cone) = crate::arena::segment(o.p([0, 46, -68]), o.p([0, 42, -90]), o.w(14)) {
        b.inked(Mesh::Cone, &cone, beak, 0);
    }
    for p in [[0, 64, -50], [0, 68, -58], [0, 63, -66]] {
        b.push(Mesh::Sphere, &o.at(p, [6, 14, 12]), red, 0, PASS_SOLID);
    }
    b.push(Mesh::Sphere, &o.at([0, 34, -66], [8, 14, 8]), red, 0, PASS_SOLID);
    let gaze = c.pal.c("lightning");
    for side in [-1, 1] {
        b.push(
            Mesh::Sphere,
            &o.at([side * 12, 52, -66], [7, 7, 5]),
            gaze,
            ONE / 2 + g / 2,
            PASS_SOLID,
        );
        let membrane = shade(body, 60);
        wing(
            b,
            o,
            side,
            [24, 10, 0],
            [110, 60, flap],
            membrane,
            Some(shade(body, 45)),
        );
        bar(b, o, [side * 10, -20, 0], [side * 12, -44, -6], 6, beak, false);
    }
    tail(b, o, [0, 0, 40], [5, 26, 22, -6], shade(body, 75));
    if g > 0 {
        let h = o.w(40 + fx::mul(g, 60));
        b.glow(Mesh::Sphere, &boxed(o.p([0, 52, -70]), v3(h, h, h)), gaze, g / 2);
    }
}

/// A chimera: a lion, a goat's head rising from its back, a serpent for a
/// tail. Its legs fold under it in a bound.
fn chimera(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let lion = paint(c.pal.c("chimera_lion"));
    let mane = mix(lion, paint(c.pal.c("hull")), ONE / 2);
    let goat = paint(c.pal.c("goat"));
    let serpent = paint(c.pal.c("dragon_scale"));
    let gold = c.pal.c("gold");
    let tuck = if e.leap > 0 { ONE } else { 0 };
    let step = e.age * fx::TURN / 28;
    b.inked(Mesh::Sphere, &o.at([0, 0, 0], [90, 80, 170]), lion, 0);
    for (k, (lx, lz)) in [(-1, -1), (1, -1), (-1, 1), (1, 1)].into_iter().enumerate() {
        let swing = fx::mul(fx::sin(step + fx::HALF * (k as i32 % 2)), 14);
        let foot = [
            lx * 34,
            -106 + fx::mul(tuck, 40),
            lz * 56 + swing + fx::mul(tuck, lz * 20),
        ];
        bar(b, o, [lx * 32, -10, lz * 56], foot, 26, lion, true);
        b.push(
            Mesh::Sphere,
            &o.at(foot, [26, 14, 32]),
            shade(lion, 85),
            0,
            PASS_SOLID,
        );
    }
    b.inked(Mesh::Sphere, &o.at([0, 22, -78], [84, 84, 60]), mane, 0);
    b.inked(Mesh::Sphere, &o.at([0, 24, -98], [56, 56, 56]), lion, 0);
    b.push(
        Mesh::Sphere,
        &o.at([0, 12 - fx::mul(g, 6), -124], [30, 24 + fx::mul(g, 10), 20]),
        mix(lion, c.pal.c("sail"), ONE / 3),
        0,
        PASS_SOLID,
    );
    for side in [-1, 1] {
        b.push(
            Mesh::Sphere,
            &o.at([side * 13, 32, -122], [8, 6, 4]),
            gold,
            ONE / 3 + g / 2,
            PASS_SOLID,
        );
    }
    // the goat, out of its back
    bar(b, o, [0, 36, 10], [0, 82, -4], 22, goat, true);
    b.inked(Mesh::Sphere, &o.at([0, 92, -14], [30, 32, 42]), goat, 0);
    for side in [-1, 1] {
        if let Some(horn) =
            crate::arena::segment(o.p([side * 8, 104, -6]), o.p([side * 18, 124, 24]), o.w(10))
        {
            b.inked(Mesh::Cone, &horn, c.pal.c("fauna_antler"), 0);
        }
        b.push(
            Mesh::Sphere,
            &o.at([side * 13, 96, -28], [5, 5, 5]),
            gold,
            ONE / 3,
            PASS_SOLID,
        );
    }
    b.push(
        Mesh::Cone,
        &o.place([0, 78, -30], &Affine::rot_x(fx::HALF), [8, 14, 8]),
        goat,
        0,
        PASS_SOLID,
    );
    // the serpent, rearing over its back
    let head = tail(b, o, [0, 8, 80], [4, 30, 22, 14], serpent);
    b.inked(Mesh::Sphere, &o.at(head, [26, 22, 34]), serpent, 0);
    b.push(
        Mesh::Sphere,
        &o.at([head[0], head[1] + 6, head[2] + 8], [20, 5, 5]),
        gold,
        ONE / 3 + g / 2,
        PASS_SOLID,
    );
}

/// A storm harpy: a woman's head and breast on a bird of the storm, cloud
/// for hair; lightning crackles between its wings through a windup.
fn storm_harpy(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let feather = paint(c.pal.c("storm_cloud"));
    let skin = paint(mix(c.pal.c("agent_skin"), c.pal.c("storm_cloud"), ONE / 3));
    let light = c.pal.c("lightning");
    let flap = fx::mul(fx::sin(e.age * fx::TURN / 14), deg(45));
    b.inked(Mesh::Sphere, &o.at([0, -14, 8], [40, 36, 52]), feather, 0);
    b.inked(Mesh::Sphere, &o.at([0, 16, -8], [30, 40, 24]), skin, 0);
    b.inked(Mesh::Sphere, &o.at([0, 46, -12], [24, 26, 24]), skin, 0);
    for (p, r) in [([0, 52, 0], 26), ([0, 46, 14], 22), ([0, 38, 24], 18)] {
        b.push(
            Mesh::Sphere,
            &o.at(p, [r, r, r]),
            shade(feather, 80),
            0,
            PASS_SOLID,
        );
    }
    let mut tips = [V3::ZERO; 2];
    for (n, side) in [-1, 1].into_iter().enumerate() {
        b.push(
            Mesh::Sphere,
            &o.at([side * 6, 48, -23], [5, 4, 3]),
            light,
            ONE / 2 + g / 2,
            PASS_SOLID,
        );
        let outer = wing(b, o, side, [16, 24, -4], [120, 50, flap], feather, None);
        tips[n] = outer.apply(v3(cm(side * 60), 0, 0));
        bar(
            b,
            o,
            [side * 8, -28, 6],
            [side * 10, -50, -6],
            6,
            paint(c.pal.c("fauna_beak")),
            false,
        );
    }
    b.inked(
        Mesh::Wedge,
        &o.at([0, -18, 40], [36, 4, 40]),
        shade(feather, 90),
        0,
    );
    if g > 0 {
        // a jagged arc from wingtip to wingtip, through its breast
        let mid = o.p([0, 16, -20]);
        let kink = o.p([fx::mul(fx::sin(e.age * deg(50)), 30), 30, -10]);
        for (a, z) in [(tips[0], kink), (kink, mid), (mid, tips[1])] {
            rod(b, a, z, o.w(3), light, ONE, false);
        }
        let h = o.w(60 + fx::mul(g, 80));
        b.glow(Mesh::Sphere, &boxed(mid, v3(h, h, h)), light, g / 2);
    }
}

/// A lock on the dragon's chain, made fast round the mainmast's foot: a
/// brass padlock as big as a chest, its shackle through an iron ring, and
/// the chain's links going up the mast. It shakes as it is shot.
fn chain_lock(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let brass = paint(c.pal.c("brass"));
    let iron = paint(c.pal.c("iron"));
    // it shakes for a moment after each hit
    let rattle = if e.flash < 12 {
        fx::mul(fx::sin(e.age * fx::TURN / 9), 4)
    } else {
        0
    };
    b.inked(Mesh::Cube, &o.at([rattle, 40, 0], [80, 60, 36]), brass, 0);
    b.push(
        Mesh::Sphere,
        &o.at([rattle, 34, -19], [14, 18, 6]),
        shade(brass, 40),
        0,
        PASS_SOLID,
    );
    let up = Affine::rot_x(deg(90));
    b.inked(Mesh::Ring, &o.place([rattle, 84, 0], &up, [56, 56, 14]), brass, 0);
    for k in 0..3 {
        let turn = if k % 2 == 0 {
            Affine::rot_x(deg(90))
        } else {
            Affine::rot_y(deg(90)).then(&Affine::rot_x(deg(90)))
        };
        b.inked(
            Mesh::Ring,
            &o.place([0, 124 + k * 34, 0], &turn, [34, 44, 10]),
            iron,
            0,
        );
    }
}

/// The Captured Dragon: a great wyvern of the fleet, an iron collar on
/// its neck with three brass locks and a chain trailing down from it to
/// the mast. Freed, the locks are gone and a stub of chain swings. Its
/// wings fold on a perch and beat in flight; fire gathers in its jaws
/// through a windup.
fn dragon(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let scale = paint(c.pal.c("dragon_scale"));
    let belly = paint(c.pal.c("dragon_belly"));
    let iron = paint(c.pal.c("iron"));
    let bone = paint(c.pal.c("fauna_antler"));
    let ember = c.pal.c("ember");
    let flying = e.flying > 0;
    b.inked(Mesh::Sphere, &o.at([0, 0, 0], [150, 130, 260]), scale, 0);
    b.inked(Mesh::Sphere, &o.at([0, -22, -20], [120, 90, 200]), belly, 0);
    for j in 0..6 {
        b.push(
            Mesh::Cone,
            &o.at([0, 62 - (j - 2) * (j - 2) * 3, -90 + j * 40], [14, 28, 22]),
            shade(scale, 80),
            0,
            PASS_SOLID,
        );
    }
    // the neck, an S up and forward
    let sway = fx::mul(fx::sin(e.age * fx::TURN / 80), 12);
    let neck = [
        [0, 30, -110],
        [sway / 2, 80, -150],
        [sway, 130, -160],
        [sway, 160, -190],
        [sway, 170, -230],
    ];
    for j in 0..4 {
        bar(b, o, neck[j], neck[j + 1], 60 - 6 * j as i32, scale, true);
    }
    let h = o.moved(at_cm([sway, 0, 0]));
    b.inked(Mesh::Sphere, &h.at([0, 176, -250], [84, 70, 96]), scale, 0);
    b.inked(Mesh::Wedge, &h.at([0, 164, -310], [60, 36, 90]), scale, 0);
    let open = deg(8) + fx::mul(g, deg(22));
    b.inked(
        Mesh::Wedge,
        &h.place([0, 150, -262], &Affine::rot_x(open), [42, 14, 64])
            .then(&Affine::translate(v3(0, 0, -ONE / 3))),
        belly,
        0,
    );
    for side in [-1, 1] {
        if let Some(horn) =
            crate::arena::segment(h.p([side * 20, 196, -236]), h.p([side * 34, 214, -180]), h.w(16))
        {
            b.inked(Mesh::Cone, &horn, bone, 0);
        }
        b.push(
            Mesh::Sphere,
            &h.at([side * 30, 190, -286], [14, 9, 9]),
            ember,
            ONE / 2,
            PASS_SOLID,
        );
    }
    if g > 0 {
        let fire = 20 + fx::mul(g, 40);
        b.push(
            Mesh::Sphere,
            &h.at([0, 158, -372], [fire, fire, fire]),
            ember,
            ONE,
            PASS_SOLID,
        );
        let r = h.w(fire * 3);
        b.glow(
            Mesh::Sphere,
            &boxed(h.p([0, 158, -372]), v3(r, r, r)),
            ember,
            g / 2,
        );
    }
    // the collar, its locks, and the chain
    let tilt = Affine::rot_x(-deg(40));
    b.inked(Mesh::Ring, &o.place([0, 56, -130], &tilt, [86, 22, 86]), iron, 0);
    let chained = e.phase == 0;
    if chained {
        for k in 0..3 {
            let a = fx::TURN / 3 * k + deg(90);
            let lock = Affine::rot_y(a).then(&Affine::translate(v3(0, 0, cm(44))));
            b.push(
                Mesh::Cube,
                &o.place([0, 56, -130], &tilt.then(&lock), [16, 18, 10]),
                c.pal.c("brass"),
                ONE / 4,
                PASS_SOLID,
            );
        }
    }
    let links = if chained { 8 } else { 2 };
    for j in 0..links {
        let swing = fx::mul(fx::sin(e.age * fx::TURN / 60 + j * deg(20)), 4 * j);
        let p = [swing, 20 - 30 * j, -136 + 6 * j];
        let turn = Affine::rot_y(fx::QUARTER * (j % 2)).then(&Affine::rot_x(fx::QUARTER));
        b.push(Mesh::Ring, &o.place(p, &turn, [20, 6, 34]), iron, 0, PASS_SOLID);
    }
    // wings: folded on a perch, beating in flight
    let beat = if flying {
        fx::mul(fx::sin(e.age * fx::TURN / 22), deg(35))
    } else {
        deg(40)
    };
    let membrane = mix(scale, belly, ONE / 4);
    for side in [-1, 1] {
        let outer = wing(
            b,
            o,
            side,
            [60, 50, -30],
            [460, 200, beat],
            membrane,
            Some(shade(scale, 70)),
        );
        let elbow = outer.apply(v3(0, 0, -cm(100)));
        for t in 1..4 {
            let end = outer.apply(v3(cm(side * 230 * t / 4), 0, cm(80)));
            rod(b, elbow, end, o.w(8), shade(scale, 70), 0, false);
        }
    }
    // legs, tucked in flight
    let tuck = if flying { 40 } else { 0 };
    for (lx, lz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
        let foot = [lx * 62, -130 + tuck, lz * 84 + tuck * lz / 2];
        bar(b, o, [lx * 56, -30, lz * 80], foot, 34, scale, true);
        b.push(Mesh::Sphere, &o.at(foot, [30, 12, 40]), bone, 0, PASS_SOLID);
    }
    let tip = tail(b, o, [0, 10, 130], [6, 50, 50, -8], scale);
    b.inked(
        Mesh::Wedge,
        &o.at([tip[0], tip[1], tip[2] + 16], [44, 6, 40]),
        shade(scale, 80),
        0,
    );
}

/// A tentacle of Dagon's, rising six metres out of the water, swaying,
/// curling over at its tip, suckers down its inner side that light
/// through a windup.
fn tentacle(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let flesh = paint(c.pal.c("dagon_flesh"));
    let sucker = mix(flesh, c.pal.c("biolume"), g / 2 + ONE / 6);
    let (mut ax, mut az) = (0, 0);
    let mut p = [0, 0, 0];
    for j in 0..8 {
        ax += fx::mul(fx::sin(e.age * fx::TURN / 90 + j * deg(30)), deg(9));
        az += if j >= 5 { deg(28) } else { deg(3) } - fx::mul(g, deg(4));
        let len = 96 - 4 * j;
        let w = 70 - 7 * j;
        let q = [
            p[0] + fx::mul(fx::sin(ax), len),
            p[1] + fx::mul(fx::mul(fx::cos(ax), fx::cos(az)), len),
            p[2] - fx::mul(fx::sin(az), len),
        ];
        bar(b, o, p, q, w, shade(flesh, 100 - 3 * j), true);
        if j < 6 {
            // the inner side, toward the curl
            let (ny, nz) = (-fx::sin(az), -fx::cos(az));
            for x in [-1, 1] {
                let m = [
                    (p[0] + q[0]) / 2 + x * w / 5,
                    (p[1] + q[1]) / 2 + fx::mul(ny, w * 9 / 20),
                    (p[2] + q[2]) / 2 + fx::mul(nz, w * 9 / 20),
                ];
                let r = w * 3 / 10;
                b.push(Mesh::Sphere, &o.at(m, [r, r, r]), sucker, g / 2, PASS_SOLID);
            }
        }
        p = q;
    }
    b.push(Mesh::Sphere, &o.at(p, [12, 12, 12]), flesh, 0, PASS_SOLID);
    let turn = Affine::rot_y(e.age * deg(3));
    b.push(
        Mesh::Ring,
        &o.place([0, 0, 0], &turn, [160, 10, 160]),
        c.pal.c("river_foam"),
        ONE / 4,
        PASS_SOLID,
    );
    if g > 0 {
        let h = o.w(80 + fx::mul(g, 120));
        b.glow(
            Mesh::Sphere,
            &boxed(o.p(p), v3(h, h, h)),
            c.pal.c("biolume"),
            g / 2,
        );
    }
}

/// Dagon: a vast octopus-fish rising out of the whirlpool's eye. A fish's
/// head with a mouth of teeth that gapes through a windup, a lantern of
/// living light hung on a stalk from its brow, rows of eyes down its
/// flanks blinking each on its own clock, fins and gills, and a crown of
/// tentacles round its shoulders curling out and over.
fn dagon(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let teal = paint(c.pal.c("dagon_teal"));
    let flesh = paint(c.pal.c("dagon_flesh"));
    let light = c.pal.c("biolume");
    let deep = c.pal.c("ocean_deep");
    let teeth = c.pal.c("fauna_teeth");
    let pulse = fx::sin(e.age * fx::TURN / 50);
    // the body out of the water, its mantle, its belly
    b.inked(Mesh::Cyl, &o.at([0, -300, 40], [640, 440, 560]), teal, 0);
    b.inked(Mesh::Sphere, &o.at([0, 40, 60], [760, 720, 640]), teal, 0);
    b.inked(Mesh::Sphere, &o.at([0, -60, -160], [520, 560, 320]), flesh, 0);
    // the head, its brow and its jaw
    b.inked(Mesh::Sphere, &o.at([0, 300, -200], [560, 380, 520]), teal, 0);
    b.inked(
        Mesh::Wedge,
        &o.at([0, 420, -260], [480, 90, 420]),
        shade(teal, 85),
        0,
    );
    let drop = fx::mul(g, 60);
    b.inked(
        Mesh::Wedge,
        &o.at([0, 170 - drop, -330], [500, 110, 420]),
        shade(teal, 75),
        0,
    );
    b.push(
        Mesh::Sphere,
        &o.at([0, 225 - drop / 2, -430], [400, 90 + drop, 120]),
        deep,
        0,
        PASS_SOLID,
    );
    let down = Affine::rot_x(fx::HALF);
    for x in [-150, -90, -30, 30, 90, 150] {
        b.push(
            Mesh::Cone,
            &o.place([x, 244, -452], &down, [24, 64, 24]),
            teeth,
            0,
            PASS_SOLID,
        );
    }
    for x in [-120, -60, 0, 60, 120] {
        b.push(
            Mesh::Cone,
            &o.at([x, 206 - drop, -440], [24, 56, 24]),
            teeth,
            0,
            PASS_SOLID,
        );
    }
    for side in [-1, 1] {
        b.inked(
            Mesh::Sphere,
            &o.at([side * 250, 320, -300], [90, 90, 70]),
            light,
            ONE / 2,
        );
        b.push(
            Mesh::Sphere,
            &o.at([side * 270, 320, -326], [30, 50, 20]),
            deep,
            0,
            PASS_SOLID,
        );
    }
    // the lantern, swinging on its stalk
    let swing = fx::mul(fx::sin(e.age * fx::TURN / 140), 40);
    let stalk = [
        [0, 470, -220],
        [swing / 3, 620, -260],
        [swing * 2 / 3, 700, -380],
        [swing, 680, -520],
        [swing, 620, -566],
    ];
    for j in 0..4 {
        bar(b, o, stalk[j], stalk[j + 1], 30 - 4 * j as i32, flesh, true);
    }
    let lamp = [swing, 570, -572];
    let r = 90 + fx::mul(pulse, 10);
    b.inked(Mesh::Sphere, &o.at(lamp, [r, r, r]), light, ONE * 3 / 4);
    b.push(
        Mesh::Sphere,
        &o.at(lamp, [50, 50, 50]),
        c.pal.c("sail"),
        ONE,
        PASS_SOLID,
    );
    let h = o.w(320 + fx::mul(pulse, 40) + fx::mul(g, 200));
    b.glow(
        Mesh::Sphere,
        &boxed(o.p(lamp), v3(h, h, h)),
        light,
        ONE / 4 + pulse / 10 + g / 2,
    );
    // rows of eyes down its flanks, on the mantle
    let (rx, ry, rz) = (380_i64, 360_i64, 320_i64);
    let mut n = 0;
    for dy in [80, -60] {
        for dz in [-200, -70, 60, 190] {
            let (y, z) = (dy as i64, dz as i64);
            let x2 = rx * rx - rx * rx * z * z / (rz * rz) - rx * rx * y * y / (ry * ry);
            let x = fx::isqrt(x2.max(0) as u64) as i32 * 95 / 100;
            for side in [-1, 1] {
                let open = (e.age + n * 29) % 170 > 8;
                let p = [side * x, 40 + dy, 60 + dz];
                b.inked(
                    Mesh::Sphere,
                    &o.at(p, [46, if open { 46 } else { 8 }, 46]),
                    light,
                    ONE / 2 + g / 3,
                );
                if open {
                    b.push(
                        Mesh::Sphere,
                        &o.at([p[0] + side * 20, p[1], p[2]], [12, 30, 16]),
                        deep,
                        0,
                        PASS_SOLID,
                    );
                }
                n += 1;
            }
        }
    }
    // fins and gills
    for side in [-1, 1] {
        b.inked(
            Mesh::Wedge,
            &o.place(
                [side * 390, -40, 40],
                &Affine::rot_z(-side * deg(30)),
                [260, 16, 200],
            ),
            shade(teal, 80),
            0,
        );
        for j in 0..3 {
            b.push(
                Mesh::Cube,
                &o.at([side * 280, 220, -60 + j * 40], [10, 120, 14]),
                shade(flesh, 70),
                0,
                PASS_SOLID,
            );
        }
    }
    for (j, p) in [[0, 400, 100], [0, 360, 240], [0, 280, 340]]
        .into_iter()
        .enumerate()
    {
        let j = j as i32;
        b.inked(Mesh::Wedge, &o.at(p, [16, 180 - j * 30, 200]), flesh, 0);
    }
    // the crown of tentacles, out of its shoulders, up and curling over
    for t in 0..8 {
        let a = fx::TURN / 8 * t + deg(22);
        let out = v3(fx::cos(a), 0, fx::sin(a));
        let across = v3(-out.z, 0, out.x);
        let mut p = v3(fx::mul(out.x, cm(340)), cm(100), fx::mul(out.z, cm(340)) + cm(60));
        let mut dir = v3(out.x / 4, ONE, out.z / 4).norm();
        for j in 0..4 {
            let sway = fx::sin(e.age * fx::TURN / 100 + t * deg(45) + j * deg(40));
            dir = dir
                .add(out.scale(ONE / 4))
                .add(v3(0, -ONE * j / 4, 0))
                .add(across.scale(sway / 3))
                .norm();
            let q = p.add(dir.scale(cm(200 - 30 * j)));
            rod(
                b,
                o.f.apply(p),
                o.f.apply(q),
                o.w(100 - 18 * j),
                shade(flesh, 100 - 5 * j),
                0,
                true,
            );
            p = q;
        }
        b.push(
            Mesh::Sphere,
            &o.f.then(&boxed(p, v3(cm(30), cm(30), cm(30)))),
            mix(flesh, light, ONE / 3),
            ONE / 4,
            PASS_SOLID,
        );
    }
    // the water churning round it
    for i in 0..2 {
        let turn = Affine::rot_y(e.age * deg(2) * (i + 1));
        let r = 780 + i * 260;
        b.push(
            Mesh::Ring,
            &o.place([0, -500, 40], &turn, [r, 20, r]),
            c.pal.c("river_foam"),
            ONE / 4,
            PASS_SOLID,
        );
    }
}

/// A Custodian: a turret of nested rings turning round a black glass core
/// with a cyan eye, on a white pedestal. The rings spin faster through a
/// windup.
fn custodian(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let white = paint(c.pal.c("vault_white"));
    let chrome = paint(c.pal.c("chrome"));
    let cyan = c.pal.c("vault_cyan");
    b.inked(Mesh::Cyl, &o.at([0, 20, 0], [120, 40, 120]), white, 0);
    b.push(
        Mesh::Ring,
        &o.at([0, 42, 0], [110, 8, 110]),
        chrome,
        0,
        PASS_SOLID,
    );
    b.inked(Mesh::Cyl, &o.at([0, 70, 0], [40, 60, 40]), chrome, 0);
    b.inked(
        Mesh::Sphere,
        &o.at([0, 130, 0], [56, 56, 56]),
        paint(c.pal.c("black_glass")),
        0,
    );
    b.push(
        Mesh::Sphere,
        &o.at([0, 130, -26], [24, 24, 8]),
        cyan,
        ONE / 2 + g / 2,
        PASS_SOLID,
    );
    let spin = e.age * deg(2) + fx::mul(g, deg(180));
    for k in 0..3 {
        let size = 90 + 40 * k;
        let sign = if k % 2 == 0 { 1 } else { -1 };
        let turn = Affine::rot_y(sign * spin * (k + 1)).then(&Affine::rot_x(deg(30) * k + spin));
        let ring = o.place([0, 130, 0], &turn, [size, 10, size]);
        b.inked(Mesh::Ring, &ring, if k % 2 == 0 { white } else { chrome }, 0);
        for x in [-1, 1] {
            b.push(
                Mesh::Sphere,
                &ring.then(&boxed(v3(x * ONE / 2, 0, 0), v3(ONE / 10, ONE, ONE / 10))),
                cyan,
                ONE / 2 + g / 2,
                PASS_SOLID,
            );
        }
    }
    let h = o.w(140 + fx::mul(g, 100));
    b.glow(
        Mesh::Sphere,
        &boxed(o.p([0, 130, 0]), v3(h, h, h)),
        c.pal.c("vault_violet"),
        ONE / 8 + g / 2,
    );
}

/// A Ward: a floating lens of black glass in a chrome rim, a cyan iris
/// that widens through a windup, a violet halo turning behind it.
fn ward(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let chrome = paint(c.pal.c("chrome"));
    let cyan = c.pal.c("vault_cyan");
    let violet = c.pal.c("vault_violet");
    let o = o.moved(v3(0, fx::mul(fx::sin(e.age * fx::TURN / 80), cm(8)), 0));
    let face = Affine::rot_x(fx::QUARTER);
    b.inked(
        Mesh::Sphere,
        &o.at([0, 0, 0], [100, 100, 30]),
        paint(c.pal.c("black_glass")),
        0,
    );
    b.inked(Mesh::Ring, &o.place([0, 0, 0], &face, [108, 14, 108]), chrome, 0);
    let iris = 40 + fx::mul(g, 24);
    b.push(
        Mesh::Sphere,
        &o.at([0, 0, -14], [iris, iris, 8]),
        cyan,
        ONE / 2 + g / 2,
        PASS_SOLID,
    );
    b.push(
        Mesh::Sphere,
        &o.at([0, 0, -18], [12, 12, 4]),
        paint(c.pal.c("black_glass")),
        0,
        PASS_SOLID,
    );
    let halo = face.then(&Affine::rot_y(e.age * deg(3)));
    b.push(
        Mesh::Ring,
        &o.place([0, 0, 20], &halo, [150, 4, 150]),
        violet,
        ONE / 2,
        PASS_SOLID,
    );
    for k in 0..4 {
        let at = halo
            .then(&Affine::rot_y(fx::QUARTER * k))
            .then(&Affine::translate(v3(0, 0, cm(75))));
        b.push(
            Mesh::Cube,
            &o.place([0, 0, 20], &at, [8, 8, 8]),
            cyan,
            ONE / 2,
            PASS_SOLID,
        );
    }
    for k in 0..3 {
        let fin = Affine::rot_z(fx::TURN / 3 * k).then(&Affine::translate(v3(0, cm(60), 0)));
        b.push(
            Mesh::Wedge,
            &o.place([0, 0, 0], &fin, [6, 24, 20]),
            chrome,
            0,
            PASS_SOLID,
        );
    }
    let h = o.w(160 + fx::mul(g, 80));
    b.glow(
        Mesh::Sphere,
        &boxed(o.p([0, 0, 0]), v3(h, h, h)),
        violet,
        ONE / 6 + g / 2,
    );
}

/// The Archivist, the Vault's keeper: a grey-violet alien, a great head
/// with five black eyes, a thin body and long limbs hanging, on a chrome
/// jet pack whose two cyan flames flare when it boosts (and burn brighter
/// as it grows); it leans into a boost and its legs trail. Drawn as big as
/// it has grown (`o` is scaled by the world).
fn archivist(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let skin = paint(c.pal.c("alien_skin"));
    let dark = paint(c.pal.c("alien_dark"));
    let chrome = paint(c.pal.c("chrome"));
    let eye = c.pal.c("alien_eye");
    let cyan = c.pal.c("vault_cyan");
    let violet = c.pal.c("vault_violet");
    let boosting = e.flying > 0;
    let lean = if boosting { -deg(28) } else { -deg(6) };
    let o = &Body {
        f: o.f.then(&Affine::rot_x(lean)),
        ..o.clone()
    };
    let sway = fx::sin(e.age * fx::TURN / 70);
    let trail = if boosting { 40 } else { 0 };
    // the body, thin, and its belly
    b.inked(Mesh::Sphere, &o.at([0, -10, 0], [70, 100, 50]), skin, 0);
    b.inked(Mesh::Sphere, &o.at([0, -40, -8], [50, 50, 36]), dark, 0);
    bar(b, o, [0, 30, 0], [0, 60, -6], 18, skin, true);
    // the head: great, a cranium swelling back, a small chin
    b.inked(Mesh::Sphere, &o.at([0, 100, -6], [120, 100, 110]), skin, 0);
    b.inked(Mesh::Sphere, &o.at([0, 120, 30], [100, 80, 90]), skin, 0);
    b.inked(Mesh::Sphere, &o.at([0, 56, -40], [34, 26, 28]), dark, 0);
    // five eyes, black, lit cyan as it winds up
    for (x, y, w) in [
        (-30, 104, 30),
        (30, 104, 30),
        (0, 128, 24),
        (-16, 80, 18),
        (16, 80, 18),
    ] {
        b.inked(Mesh::Sphere, &o.at([x, y, -54], [w, w * 3 / 4, 12]), eye, 0);
        b.push(
            Mesh::Sphere,
            &o.at([x + w / 6, y + w / 6, -60], [w / 3, w / 3, 4]),
            cyan,
            ONE / 3 + g,
            PASS_SOLID,
        );
    }
    // the arms, long, three fingers to a hand
    for side in [-1, 1] {
        let reach = fx::mul(sway, 20) * side;
        let shoulder = [side * 34, 26, 0];
        let elbow = [side * 74, -20 + reach / 2, -16];
        let hand = [side * 84, -86 + reach, -40 - trail / 2];
        bar(b, o, shoulder, elbow, 16, skin, true);
        bar(b, o, elbow, hand, 13, skin, true);
        for f in [-1, 0, 1] {
            let tip = [hand[0] + side * 6 + f * 8, hand[1] - 30, hand[2] - 6 + f * 6];
            bar(b, o, hand, tip, 6, dark, false);
        }
    }
    // the legs, long, hanging, trailing in a boost
    for side in [-1, 1] {
        let swing = fx::mul(sway, 16) * side;
        let hip = [side * 18, -54, 0];
        let knee = [side * 26, -112, -14 + swing + trail];
        let foot = [side * 22, -172, 4 + swing + trail * 2];
        bar(b, o, hip, knee, 18, skin, true);
        bar(b, o, knee, foot, 14, skin, true);
        b.inked(
            Mesh::Sphere,
            &o.at([foot[0], foot[1], foot[2] - 10], [18, 10, 30]),
            dark,
            0,
        );
    }
    // the jet pack: a chrome case on its back, two tanks, two nozzles
    b.inked(Mesh::Cube, &o.at([0, 10, 40], [76, 84, 34]), chrome, 0);
    b.push(
        Mesh::Cube,
        &o.at([0, 24, 58], [44, 10, 4]),
        cyan,
        ONE / 2,
        PASS_SOLID,
    );
    // flames: short and flickering while it hovers, long when it boosts,
    // brighter each phase
    let flicker = fx::sin(e.age * fx::TURN / 6);
    let long = if boosting { 150 } else { 50 } + fx::mul(flicker, 12) + 30 * e.phase as i32;
    let bright = ONE / 2 + ONE / 4 * e.phase.min(2) as i32;
    let down = Affine::rot_x(fx::HALF);
    for side in [-1, 1] {
        let x = side * 26;
        b.inked(Mesh::Cyl, &o.at([x, 0, 60], [26, 96, 26]), chrome, 0);
        b.inked(Mesh::Cone, &o.at([x, -56, 60], [30, 22, 30]), dark, 0);
        b.push(
            Mesh::Cone,
            &o.place([x, -66 - long / 2, 60], &down, [28, long, 28]),
            cyan,
            bright,
            PASS_SOLID,
        );
        b.push(
            Mesh::Cone,
            &o.place([x, -66 - long / 4, 60], &down, [14, long / 2, 14]),
            c.pal.c("vault_white"),
            ONE,
            PASS_SOLID,
        );
    }
    let h = o.w(80 + long);
    b.glow(
        Mesh::Sphere,
        &boxed(o.p([0, -80 - long / 2, 60]), v3(h / 2, h, h / 2)),
        cyan,
        bright / 3,
    );
    let h = o.w(110 + fx::mul(g, 120));
    b.glow(
        Mesh::Sphere,
        &boxed(o.p([0, 100, 0]), v3(h, h, h)),
        violet,
        ONE / 16 + g / 2,
    );
}

/// An Excisor: a small chrome drone, four arms each spinning a blade of
/// light.
fn excisor(b: &mut Batch, c: &Content, e: &Enemy, o: &Body, paint: &dyn Fn(Rgb) -> Rgb) {
    let g = e.glow(c);
    let chrome = paint(c.pal.c("chrome"));
    let cyan = c.pal.c("vault_cyan");
    b.inked(Mesh::Octa, &o.at([0, 0, 0], [50, 36, 60]), chrome, 0);
    b.push(
        Mesh::Sphere,
        &o.at([0, 0, -28], [18, 14, 8]),
        cyan,
        ONE / 2 + g / 2,
        PASS_SOLID,
    );
    b.push(
        Mesh::Cube,
        &o.at([0, -14, 0], [36, 6, 40]),
        paint(c.pal.c("black_glass")),
        0,
        PASS_SOLID,
    );
    for (k, (x, z)) in [(-1, -1), (1, -1), (-1, 1), (1, 1)].into_iter().enumerate() {
        let end = [x * 50, 4, z * 40];
        bar(
            b,
            o,
            [x * 14, 0, z * 12],
            end,
            6,
            paint(c.pal.c("vault_white")),
            false,
        );
        let spin = Affine::rot_y(e.age * deg(40) + k as i32 * deg(45));
        b.push(
            Mesh::Cube,
            &o.place(end, &spin, [60, 3, 6]),
            cyan,
            ONE,
            PASS_SOLID,
        );
    }
    let h = o.w(90 + fx::mul(g, 60));
    b.glow(
        Mesh::Sphere,
        &boxed(o.p([0, 0, 0]), v3(h, h, h)),
        cyan,
        ONE / 6 + g / 2,
    );
}

/// A piece of wreckage, drawn by its shape: a long one a mast lying on its
/// raft of planks, a wide one a hull's side, a deep one a hatch heaped with
/// crates, a square one a deck on its broken hull. Falling into a
/// whirlpool, foam boils round it as it is about to sink; once under the
/// eye's water it is not drawn.
pub fn wreck(
    b: &mut Batch,
    c: &Content,
    m: &crate::course::Mover,
    s: &crate::level::Solid,
    tick: i32,
    water: i32,
) {
    if s.max.y < water - ONE {
        return;
    }
    let (mid, size) = (s.centre(), s.size());
    let plank = c.pal.c("river_wood");
    let hull = c.pal.c("hull");
    let dark = c.pal.c("hull_dark");
    let top = v3(mid.x, s.max.y, mid.z);
    let (w, d) = (size.x, size.z);
    b.inked(Mesh::Cube, &boxed(mid, size), plank, 0);
    if w * 3 > d * 4 || d * 3 > w * 4 {
        // a raft of planks, the mast lying along it and over its ends, its
        // yard across
        let along = if w > d {
            v3(w * 13 / 10, 0, 0)
        } else {
            v3(0, 0, d * 13 / 10)
        };
        let a = top.add(v3(0, dm(4), 0)).sub(along.scale(ONE / 2));
        rod(b, a, a.add(along), dm(8), dark, 0, true);
        let across = if w > d { v3(0, 0, d) } else { v3(w, 0, 0) };
        let y = top.add(v3(0, dm(3), 0)).add(along.scale(ONE / 5));
        rod(
            b,
            y.sub(across.scale(ONE / 2)),
            y.add(across.scale(ONE / 2)),
            dm(3),
            dark,
            0,
            true,
        );
    } else if w != d {
        // a hull's side or a hatch: its belly, its ribs, crates on a hatch
        b.inked(
            Mesh::Blob,
            &boxed(v3(mid.x, s.min.y - dm(4), mid.z), v3(w, dm(14), d)),
            hull,
            0,
        );
        for k in [-1, 1] {
            let (off, rib) = if w > d {
                (v3(w * k / 3, dm(1), 0), v3(dm(3), dm(3), d))
            } else {
                (v3(0, dm(1), d * k / 3), v3(w, dm(3), dm(3)))
            };
            b.inked(Mesh::Cube, &boxed(top.add(off), rib), dark, 0);
        }
        if d > w {
            for (dx, dz, e) in [(-1, -1, 9), (1, -1, 7), (-1, 1, 6)] {
                let at = top.add(v3(w * dx / 3, ONE * e / 20, d * dz / 3));
                b.inked(Mesh::Cube, &boxed(at, v3(dm(e), dm(e), dm(e))), hull, 0);
            }
        }
    } else {
        b.inked(
            Mesh::Cube,
            &boxed(v3(mid.x, s.min.y - dm(3), mid.z), v3(w * 3 / 4, dm(6), d * 3 / 4)),
            hull,
            0,
        );
    }
    // about to sink: foam boiling up round it
    if let Some(sp) = m.spiral {
        let left = sp.live - sp.along(tick);
        if left > 0 && left < crate::balance::EYE_WARN_TICKS {
            let r = w.max(d) + ONE;
            let swell = fx::sin(tick * fx::TURN / 20);
            b.glow(
                Mesh::Ring,
                &Affine::translate(v3(mid.x, s.min.y, mid.z))
                    .then(&Affine::rot_y(tick * fx::TURN / 90))
                    .then(&Affine::scale(v3(r, dm(6) + fx::mul(dm(2), swell), r))),
                c.pal.c("river_foam"),
                ONE / 2,
            );
        }
    }
}

/// A whirlpool's water turning: foam streaming down the cone's wall in
/// arms, faster nearer the eye.
pub fn whirl(b: &mut Batch, c: &Content, wh: &crate::course::Whirl, tick: i32) {
    let foam = c.pal.c("river_foam");
    let spin = (tick as i64 * fx::TURN as i64 / crate::balance::EYE_WHIRL_TICKS as i64) as i32;
    const ARMS: i32 = 6;
    let bands = wh.wall.len() as i32 - 1;
    for band in 0..bands {
        let (inner, outer) = (wh.wall[band as usize], wh.wall[band as usize + 1]);
        // the inner bands go round faster
        let turned = spin * (bands + 2 - band) / bands;
        for arm in 0..ARMS {
            let a = fx::TURN / ARMS * arm - deg(14) * band + turned;
            let (s, co) = (fx::sin(a), fx::cos(a));
            let across = v3(co, 0, -s);
            let slant = v3(
                fx::mul(s, outer.0 - inner.0),
                outer.1 - inner.1,
                fx::mul(co, outer.0 - inner.0),
            );
            let normal = slant.norm().cross(across).norm();
            let r = (inner.0 + outer.0) / 2;
            let mid = v3(
                wh.eye.x + fx::mul(s, r),
                (inner.1 + outer.1) / 2,
                wh.eye.z + fx::mul(co, r),
            )
            .add(normal.scale(dm(2)));
            b.glow(
                Mesh::Panel,
                &Affine {
                    c0: across.scale(r / 3),
                    c1: normal,
                    c2: slant.scale(ONE * 3 / 10),
                    t: mid,
                },
                foam,
                ONE / 3,
            );
        }
    }
}

/// What fills the Eraser's ring: shut, an iris of chrome blades over black
/// glass, dim violet behind; opening, the blades draw back into the ring,
/// the pylons' beams reach it, and its heart turns cyan and violet, a ring
/// on the dais pulsing where it will take you; taking you, its light goes
/// grey.
pub fn eraser(b: &mut Batch, c: &Content, er: &crate::course::Eraser, tick: i32) {
    let chrome = c.pal.c("chrome");
    let cyan = c.pal.c("vault_cyan");
    let violet = c.pal.c("vault_violet");
    let grey = c.pal.c("erased_grey");
    let mid = er.at.add(v3(0, er.ring, 0));
    // how far it has opened, 0 to ONE, over its first second and a half
    let open = (er.opened * (ONE / crate::balance::ERASER_OPEN_TICKS)).clamp(0, ONE);
    let inner = er.ring - dm(9);
    let face = Affine::translate(mid).then(&Affine::rot_x(fx::QUARTER));
    // the iris: six blades, drawn back as it opens
    for k in 0..6 {
        let a = fx::TURN / 6 * k + fx::mul(open, deg(40));
        let out = inner / 2 + fx::mul(open, inner * 6 / 10);
        let size = fx::mul(ONE - open * 3 / 4, inner);
        let blade = Affine::translate(mid)
            .then(&Affine::rot_z(a))
            .then(&boxed(v3(0, out, 0), v3(size, size, dm(2))));
        b.inked(Mesh::Wedge, &blade, chrome, 0);
    }
    if open == 0 {
        b.push(
            Mesh::Cyl,
            &face.then(&Affine::scale(v3(inner * 2, dm(1), inner * 2))),
            c.pal.c("black_glass"),
            0,
            PASS_SOLID,
        );
        b.glow(Mesh::Sphere, &boxed(mid, v3(m(4), m(4), m(4))), violet, ONE / 4);
        return;
    }
    let taking = er.grey();
    let tint = |x: Rgb| mix(x, grey, taking);
    // its heart, swirling
    let pulse = fx::sin(tick * fx::TURN / 50);
    b.push(
        Mesh::Cyl,
        &face.then(&Affine::scale(v3(
            fx::mul(inner * 2, open),
            dm(1),
            fx::mul(inner * 2, open),
        ))),
        tint(violet),
        ONE / 2 + pulse / 8,
        PASS_SOLID,
    );
    for k in 1..4 {
        let r = fx::mul(inner * 2 * k / 4, open);
        let spin = Affine::rot_y(tick * deg(3) * if k % 2 == 0 { 1 } else { -1 });
        b.glow(
            Mesh::Ring,
            &face.then(&spin).then(&Affine::scale(v3(r, dm(3), r))),
            tint(cyan),
            ONE / 2,
        );
    }
    // the pylons feed it
    for &p in &er.pylons {
        rod(b, p, mid, fx::mul(dm(3), open), tint(cyan), ONE, false);
    }
    // and on the dais, where it takes you
    let r = er.radius * 2 + fx::mul(dm(3), pulse);
    b.glow(
        Mesh::Ring,
        &boxed(er.at.add(v3(0, dm(1), 0)), v3(r, dm(2), r)),
        tint(cyan),
        ONE * 3 / 4,
    );
    let h = fx::mul(m(10), open);
    b.glow(Mesh::Sphere, &boxed(mid, v3(h, h, h / 3)), tint(violet), ONE / 3);
}

fn m(n: i32) -> i32 {
    n * ONE
}

fn dm(n: i32) -> i32 {
    n * ONE / 10
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;
    use crate::enemy::State;

    /// How many instances `id` draws, posed by `pose`, and whether it was
    /// one of these.
    fn drawn(id: &str, pose: impl FnOnce(&mut Enemy)) -> (bool, usize) {
        let c = content();
        let def = c.enemy(id).unwrap();
        let d = &c.enemies[def];
        let mut e = Enemy::new(def, d, v3(0, ONE * 2, 0), 1);
        e.state = State::Rest(10);
        e.age = 40;
        pose(&mut e);
        let mut b = Batch::new([0; 3]);
        let mine = draw(&mut b, &c, &e, d, e.pos, d.radius * 2, &|x| x);
        (mine, b.count())
    }

    const BUDGETS: [(&str, usize); 17] = [
        ("deckhand", 50),
        ("gunner", 50),
        ("powder_monkey", 50),
        ("lookout", 50),
        ("bosun", 50),
        ("cockatrice", 50),
        ("chimera", 50),
        ("storm_harpy", 50),
        ("drowned", 50),
        ("captured_dragon", 120),
        ("chain_lock", 20),
        ("tentacle", 40),
        ("dagon", 220),
        ("custodian", 50),
        ("ward", 50),
        ("excisor", 50),
        ("archivist", 80),
    ];

    #[test]
    fn each_sea_and_vault_enemy_draws_within_its_budget() {
        for (id, most) in BUDGETS {
            for windup in [false, true] {
                let (mine, n) = drawn(id, |e| {
                    if windup {
                        e.state = State::Windup(1);
                    }
                });
                assert!(mine, "{id} is not drawn here");
                assert!(n > 0 && n <= most, "{id} drew {n} instances (windup {windup})");
            }
        }
        // the dragon in flight, and freed of its chain
        let (_, n) = drawn("captured_dragon", |e| {
            e.flying = 10;
            e.phase = 1;
        });
        assert!(n <= 120, "the dragon flying free drew {n}");
    }

    /// The Archivist as the world draws it, posed: how far its parts reach
    /// from its middle, and the longest part (its flames, in a boost).
    fn archivist_reach(pose: impl FnOnce(&mut Enemy)) -> (i32, i32) {
        let c = content();
        let def = c.enemy("archivist").unwrap();
        let d = &c.enemies[def];
        let mut e = Enemy::new(def, d, v3(0, ONE * 8, 0), 1);
        e.state = State::Rest(10);
        e.age = 40;
        pose(&mut e);
        let mut b = Batch::new([0; 3]);
        crate::draw::enemy(&mut b, &c, &e, ONE, v3(0, ONE, ONE * 10));
        let (values, _) = b.finish(0);
        let at = |k: usize| v3(values[k + 3], values[k + 7], values[k + 11]);
        let ks = (0..values.len()).step_by(16);
        let far = ks.clone().map(|k| at(k).sub(e.pos).len()).max().unwrap();
        let long = ks
            .map(|k| v3(values[k + 4], values[k + 5], values[k + 6]).len())
            .max()
            .unwrap();
        (far, long)
    }

    #[test]
    fn the_archivist_is_drawn_as_big_as_it_has_grown_and_its_flames_flare_in_a_boost() {
        let (small, _) = archivist_reach(|_| {});
        let (big, _) = archivist_reach(|e| e.scale = ONE * 5 / 2);
        assert!(big > small * 2, "grown {big}, small {small}");
        let (_, hover) = archivist_reach(|_| {});
        let (_, boost) = archivist_reach(|e| e.flying = 10);
        assert!(
            boost > hover + ONE / 4,
            "its flames do not flare: {boost} vs {hover}"
        );
    }

    #[test]
    fn the_erasers_ring_is_shut_until_it_opens_and_then_lit() {
        let c = content();
        let a = crate::arena::build_at("vault", "boss", 1, &[], &c.pal, 0);
        let mut er = a.course.unwrap().eraser.unwrap();
        let count = |er: &crate::course::Eraser| {
            let mut b = Batch::new([0; 3]);
            eraser(&mut b, &c, er, 30);
            b.count()
        };
        let shut = count(&er);
        er.open = true;
        er.opened = 200;
        let open = count(&er);
        assert!(open > shut, "open {open}, shut {shut}");
        assert!(open <= 40, "the open Eraser drew {open}");
    }

    #[test]
    fn others_are_not_drawn_here() {
        let (mine, n) = drawn("hart", |_| {});
        assert!(!mine && n == 0, "a hart was drawn as a sea enemy");
    }

    #[test]
    fn the_dragons_chain_and_locks_are_gone_once_it_is_free() {
        let (_, chained) = drawn("captured_dragon", |_| {});
        let (_, free) = drawn("captured_dragon", |e| e.phase = 1);
        assert_eq!(chained, free + 9, "chained {chained}, free {free}");
    }

    #[test]
    fn the_tentacle_sways() {
        let c = content();
        let def = c.enemy("tentacle").unwrap();
        let d = &c.enemies[def];
        let at = |age: i32| {
            let mut e = Enemy::new(def, d, v3(0, ONE * 2, 0), 1);
            e.age = age;
            let mut b = Batch::new([0; 3]);
            draw(&mut b, &c, &e, d, e.pos, d.radius * 2, &|x| x);
            b.finish(0).0
        };
        assert_ne!(at(0), at(45), "the tentacle stood still");
    }
}
