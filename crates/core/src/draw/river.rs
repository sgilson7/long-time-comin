//! The Water Gap's things that move or change: its emitters (the weir-lamp,
//! the root-eye, the spray-eye), the Gallows Oak's eye, and the Summons.
//! The emitters are set in rock and face out of it: in their own frame the
//! stone is behind them (+z) and you are in front (−z).

use super::rod;
use crate::content::{Content, EnemyDef, Rgb};
use crate::enemy::Enemy;
use crate::fx::{self, deg, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{boxed, mix, shade, Batch, PASS_SOLID};

/// Draws a river enemy; false for any other, which is drawn elsewhere.
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
        "weir_lamp" => weir_lamp(b, c, e, at, s, paint),
        "root_eye" => root_eye(b, c, e, at, s, paint, false),
        "spray_eye" => root_eye(b, c, e, at, s, paint, true),
        "gallows_oak" => oak_eye(b, c, e, at, paint),
        "summons" => summons(b, c, e, at, s, paint),
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

/// A weir-lamp: an old sluice lantern bolted to the weir's wall, a brass
/// drum under an iron hood, a caged river-green lens that brightens
/// through its windup.
fn weir_lamp(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let brass = paint(c.pal.c("brass"));
    let iron = paint(c.pal.c("iron"));
    let g = e.glow(c);
    let f = f_of(at, e.yaw, s);
    // the plate it is bolted to, and its rivets
    b.inked(
        Mesh::Cube,
        &f.then(&boxed(v3(0, 0, u(36)), v3(u(96), u(116), u(14)))),
        shade(iron, 85),
        0,
    );
    for (x, y) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
        b.push(
            Mesh::Sphere,
            &f.then(&boxed(v3(x * u(38), y * u(48), u(28)), v3(u(10), u(10), u(10)))),
            brass,
            0,
            PASS_SOLID,
        );
    }
    // the drum, lying out of the wall, banded in iron
    let drum = Affine::rot_x(fx::QUARTER).then(&Affine::scale(v3(u(70), u(60), u(70))));
    b.inked(Mesh::Cyl, &f.then(&drum), brass, 0);
    for z in [-u(18), u(12)] {
        let band = Affine::translate(v3(0, 0, z))
            .then(&Affine::rot_x(fx::QUARTER))
            .then(&Affine::scale(v3(u(76), u(8), u(76))));
        b.push(Mesh::Ring, &f.then(&band), iron, 0, PASS_SOLID);
    }
    b.inked(
        Mesh::Wedge,
        &f.then(&boxed(v3(0, u(40), -u(6)), v3(u(86), u(18), u(76)))),
        iron,
        0,
    );
    // the sluice's hand-wheel on its side
    let wheel = Affine::translate(v3(u(40), 0, u(6)))
        .then(&Affine::rot_z(fx::QUARTER))
        .then(&Affine::scale(v3(u(34), u(8), u(34))));
    b.inked(Mesh::Cyl, &f.then(&wheel), shade(brass, 85), 0);
    // the lens, and its cage
    let river = c.pal.c("river");
    let lens = mix(paint(river), c.pal.c("river_foam"), g / 2);
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, 0, -u(30)), v3(u(54), u(54), u(18)))),
        lens,
        ONE / 3 + g * 2 / 3,
    );
    for size in [v3(u(6), u(60), u(4)), v3(u(60), u(6), u(4))] {
        b.push(
            Mesh::Cube,
            &f.then(&boxed(v3(0, 0, -u(40)), size)),
            iron,
            0,
            PASS_SOLID,
        );
    }
    let h = fx::mul(s, ONE + g);
    b.glow(
        Mesh::Sphere,
        &boxed(f.apply(v3(0, 0, -u(36))), v3(h, h, h)),
        river,
        ONE / 8 + g / 2,
    );
}

/// A root-eye: a knot of the Gallows Oak's black roots broken out of the
/// canyon's stone, a gold eye in it whose lids part wider through its
/// windup. A spray-eye is one behind the falls, wrapped in their spray.
fn root_eye(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb, spray: bool) {
    let root = paint(c.pal.c("root"));
    let bark = paint(c.pal.c("oak_bark"));
    let glint = c.pal.c("root_glow");
    let g = e.glow(c);
    let f = f_of(at, e.yaw, s);
    // the broken stone round it
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, 0, u(24)), v3(u(100), u(90), u(40)))),
        c.pal.c(if spray { "sandstone" } else { "sandstone_dark" }),
        0,
    );
    // roots out of the knot, kinked, and back into the stone; the crack
    // each goes into glows
    let roots = if spray { 4 } else { 5 };
    for k in 0..roots {
        let a = fx::TURN / roots * k + deg(20);
        let (x, y) = (fx::cos(a), fx::sin(a));
        let kink = if k % 2 == 0 { u(8) } else { -u(8) };
        let p0 = f.apply(v3(fx::mul(x, u(22)), fx::mul(y, u(22)), -u(6)));
        let p1 = f.apply(v3(
            fx::mul(x, u(52)) - fx::mul(y, kink),
            fx::mul(y, u(48)) + fx::mul(x, kink),
            -u(14),
        ));
        let p2 = f.apply(v3(fx::mul(x, u(80)), fx::mul(y, u(72)), u(16)));
        let tone = if k % 2 == 0 { root } else { shade(root, 85) };
        rod(b, p0, p1, s / 7, tone, 0, true);
        rod(b, p1, p2, s / 12, tone, 0, true);
        b.push(
            Mesh::Sphere,
            &boxed(p2, v3(s / 14, s / 14, s / 14)),
            glint,
            ONE / 2,
            PASS_SOLID,
        );
    }
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, 0, -u(6)), v3(u(58), u(58), u(30)))),
        root,
        0,
    );
    eye(b, c, &f, g, bark, u(40));
    if g > 0 {
        let h = fx::mul(s, ONE + g / 2);
        b.glow(
            Mesh::Sphere,
            &boxed(f.apply(v3(0, 0, -u(20))), v3(h, h, h)),
            glint,
            g / 2,
        );
    }
    if spray {
        // the falls' spray clinging round it, boiling
        let foam = c.pal.c("river_foam");
        for k in 0..6 {
            let a = fx::TURN / 6 * k + e.age * deg(2);
            let r = u(20) + fx::mul(fx::sin(e.age * deg(7) + k * deg(70)).abs(), u(8));
            let p = v3(fx::mul(fx::cos(a), u(56)), fx::mul(fx::sin(a), u(50)), -u(4));
            b.push(
                Mesh::Sphere,
                &f.then(&boxed(p, v3(r, r, r))),
                foam,
                ONE / 4,
                PASS_SOLID,
            );
        }
        b.glow(
            Mesh::Sphere,
            &boxed(at, v3(s * 3 / 2, s * 3 / 2, s * 3 / 2)),
            foam,
            ONE / 6,
        );
    }
}

/// A gold eye `w` across in frame `f`, under two bark lids that part as
/// `g` rises: a slit at rest, wide at the end of a windup.
fn eye(b: &mut Batch, c: &Content, f: &Affine, g: i32, lid: Rgb, w: i32) {
    let at = |x: i32| x * w / 40;
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, 0, -at(20)), v3(at(40), at(40), at(20)))),
        c.pal.c("oak_eye"),
        ONE / 3 + g * 2 / 3,
    );
    // its slit pupil, taller as it opens
    b.push(
        Mesh::Sphere,
        &f.then(&boxed(
            v3(0, 0, -at(29)),
            v3(at(6), at(8) + fx::mul(g, at(14)), at(4)),
        )),
        c.pal.c("river_ink"),
        0,
        PASS_SOLID,
    );
    let part = at(17) + fx::mul(g, at(10));
    for side in [-1, 1] {
        b.inked(
            Mesh::Sphere,
            &f.then(&boxed(v3(0, side * part, -at(24)), v3(at(54), at(26), at(24)))),
            lid,
            0,
        );
    }
}

/// The Gallows Oak's eye, opening in its bark wherever you are on the
/// climb; the trunk is the arena's. When the Summons is up the eye has
/// closed into the bark, and there is nothing to draw.
fn oak_eye(b: &mut Batch, c: &Content, e: &Enemy, at: V3, paint: &dyn Fn(Rgb) -> Rgb) {
    if e.phase > 0 {
        return;
    }
    let bark = paint(c.pal.c("oak_bark"));
    let g = e.glow(c);
    // in metres: the eye is two across whatever the def's radius
    let f = f_of(at, e.yaw, ONE);
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, 0, u(40)), v3(u(320), u(260), u(140)))),
        shade(bark, 85),
        0,
    );
    // the bark rucked up round it
    for k in 0..6 {
        let a = fx::TURN / 6 * k + deg(30);
        let (x, y) = (fx::cos(a), fx::sin(a));
        let p0 = f.apply(v3(fx::mul(x, u(140)), fx::mul(y, u(110)), -u(10)));
        let p1 = f.apply(v3(fx::mul(x, u(220)), fx::mul(y, u(180)), u(60)));
        rod(b, p0, p1, u(40), bark, 0, true);
    }
    eye(b, c, &f, g, bark, u(200));
    b.glow(
        Mesh::Sphere,
        &boxed(f.apply(v3(0, 0, -u(60))), v3(u(300), u(300), u(300))),
        c.pal.c("oak_eye"),
        ONE / 8 + g / 2,
    );
}

/// The Summons: a knot of green vines round a coral seed-pod with one
/// lantern eye, thorn guns on its flanks, seed-missile pods on its back,
/// a vine blade at each side and tendrils trailing under it. Its Husk is
/// a violet shell of plates, fewer and thinner as it is broken; with the
/// Husk gone it staggers and its core flashes.
fn summons(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: &dyn Fn(Rgb) -> Rgb) {
    let vine = paint(c.pal.c("summons_vine"));
    let lantern = c.pal.c("summons_eye");
    let g = e.glow(c);
    let f = f_of(at, e.yaw, s);
    let staggered = e.stagger > 0;
    // the core: flashing while it staggers
    let flash = staggered && (e.age / 4) % 2 == 0;
    let core = paint(c.pal.c("summons_core"));
    let core = if flash {
        mix(core, lantern, ONE * 6 / 10)
    } else {
        core
    };
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(V3::ZERO, v3(u(56), u(66), u(52)))),
        core,
        if staggered { ONE / 2 } else { 0 },
    );
    if staggered {
        b.glow(
            Mesh::Sphere,
            &boxed(at, v3(s, s, s)),
            c.pal.c("summons_core"),
            if flash { ONE / 2 } else { ONE / 5 },
        );
    }
    // the lantern eye, its iris swelling through a windup
    b.inked(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, 0, -u(24)), v3(u(32), u(32), u(14)))),
        mix(lantern, [ONE; 3], ONE / 2),
        ONE / 2,
    );
    let iris = fx::mul(u(23), ONE + g / 4);
    b.push(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, 0, -u(29)), v3(iris, iris, u(10)))),
        c.pal.c("gold"),
        ONE / 2 + g / 2,
        PASS_SOLID,
    );
    b.push(
        Mesh::Sphere,
        &f.then(&boxed(v3(0, 0, -u(34)), v3(u(5), u(14), u(4)))),
        c.pal.c("river_ink"),
        0,
        PASS_SOLID,
    );
    if g > 0 {
        let h = fx::mul(u(60), ONE + g);
        b.glow(
            Mesh::Sphere,
            &f.then(&boxed(v3(0, 0, -u(30)), v3(h, h, h))),
            lantern,
            g / 2,
        );
    }
    // the knot: three vine bands tilted round it, writhing a little
    let writhe = fx::sin(e.age * fx::TURN / 160) / 20;
    for k in 0..3 {
        let band = Affine::rot_z(deg(60) * k + writhe * (k + 1))
            .then(&Affine::rot_x(deg(35)))
            .then(&Affine::scale(v3(u(120), u(18), u(120))));
        b.inked(Mesh::Ring, &f.then(&band), shade(vine, 90 + 10 * k), 0);
    }
    for side in [-1, 1] {
        // thorn guns, three on each flank, pointing out
        for (n, y) in [-u(12), 0, u(12)].into_iter().enumerate() {
            let gun = Affine::translate(v3(side * u(62), y, -u(4) + u(4) * n as i32))
                .then(&Affine::rot_z(-side * fx::QUARTER))
                .then(&Affine::scale(v3(u(8), u(26), u(8))));
            b.inked(Mesh::Cone, &f.then(&gun), c.pal.c("river_foam"), 0);
        }
        // a vine blade: an arm out of the knot, and the blade down from it
        let swing = fx::mul(fx::sin(e.age * fx::TURN / 90 + side * deg(40)), u(10));
        let shoulder = f.apply(v3(side * u(50), -u(15), 0));
        let elbow = f.apply(v3(side * u(80), -u(42), -u(4)));
        let tip = f.apply(v3(side * u(104), -u(104) + swing, -u(12) + swing));
        rod(b, shoulder, elbow, s / 12, vine, 0, true);
        if let Some(blade) = crate::arena::segment(elbow, tip, s / 7) {
            let blade = Affine {
                c2: blade.c2.scale(ONE / 4),
                ..blade
            };
            b.inked(Mesh::Cone, &blade, shade(vine, 120), 0);
        }
    }
    // seed-missile pods on its back, lit as a volley winds up
    for dx in [-1, 0, 1] {
        let pod = Affine::translate(v3(dx * u(22), u(46) - dx.abs() * u(6), u(12)))
            .then(&Affine::rot_z(-dx * deg(25)))
            .then(&Affine::scale(v3(u(17), u(25), u(17))));
        b.inked(Mesh::Sphere, &f.then(&pod), c.pal.c("lime"), ONE / 4 + g / 4);
    }
    // tendrils trailing under it, swaying, thinner to their tips
    for t in 0..5 {
        let mut p = f.apply(v3(u(15) * (t - 2), -u(34), u(6) * (t % 2)));
        for j in 0..4 {
            let sway = fx::sin(e.age * fx::TURN / 60 + j * deg(45) + t * deg(72));
            let q = p.add(f.apply_dir(v3(fx::mul(sway, u(8)), -u(22), fx::mul(sway, u(4)))));
            rod(b, p, q, s * (8 - j) / 100, shade(vine, 100 - 6 * j), 0, true);
            p = q;
        }
    }
    // the Husk: plates round the core, fewer and thinner as it breaks
    if e.husk > 0 && !staggered {
        let k = fx::div(e.husk, e.husk_max.max(1)).min(ONE);
        let husk = paint(c.pal.c("husk"));
        let shown = (8 * k + ONE - 1) / ONE;
        let w = fx::mul(u(40), ONE / 2 + k / 2);
        let thick = fx::mul(u(6), ONE / 3 + k * 2 / 3);
        for n in 0..shown {
            let (ring, i) = (n % 2, n / 2);
            let a = fx::QUARTER * i + deg(45) * ring + e.age * deg(1) / 2;
            let el = if ring == 0 { deg(25) } else { -deg(25) };
            let plate = Affine::rot_y(a)
                .then(&Affine::rot_x(el))
                .then(&boxed(v3(0, 0, -u(72)), v3(w, w, thick)));
            b.inked(Mesh::Cube, &f.then(&plate), husk, ONE / 4 + k / 4);
        }
        let h = u(156);
        b.glow(Mesh::Sphere, &f.then(&boxed(V3::ZERO, v3(h, h, h))), husk, k / 5);
    }
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

    #[test]
    fn each_river_enemy_draws_within_its_budget() {
        for (id, most) in [
            ("weir_lamp", 40),
            ("root_eye", 40),
            ("spray_eye", 40),
            ("gallows_oak", 40),
            ("summons", 120),
        ] {
            let (mine, n) = drawn(id, |_| {});
            assert!(mine, "{id} is not drawn here");
            assert!(n > 0 && n <= most, "{id} drew {n} instances");
        }
        // a windup's glow adds to it but stays in budget
        let (_, n) = drawn("summons", |e| e.state = State::Windup(1));
        assert!(n <= 120, "the Summons winding up drew {n}");
    }

    #[test]
    fn others_are_not_drawn_here() {
        let (mine, n) = drawn("hart", |_| {});
        assert!(!mine && n == 0, "a hart was drawn as a river enemy");
    }

    #[test]
    fn the_oaks_eye_is_gone_once_the_summons_is_up() {
        let (mine, n) = drawn("gallows_oak", |e| e.phase = 1);
        assert!(mine && n == 0, "the Oak's eye drew {n} with the Summons up");
    }

    #[test]
    fn the_summons_husk_thins_and_is_gone_in_a_stagger() {
        let (_, whole) = drawn("summons", |_| {});
        let (_, cracked) = drawn("summons", |e| e.husk = e.husk_max / 4);
        let (_, broken) = drawn("summons", |e| {
            e.husk = 0;
            e.stagger = 10;
        });
        assert!(
            whole > cracked,
            "a cracked Husk drew {cracked}, a whole one {whole}"
        );
        assert!(
            cracked > broken,
            "a broken Husk drew {broken}, a cracked one {cracked}"
        );
    }
}
