//! The house's people and the Executor.
//!
//! The Continuance's agents are all one man: a charcoal suit, a white
//! shirt, a black tie, dark glasses, the same face. Each role carries what
//! tells it apart at a distance: the Warder's riot shield, the Marksman's
//! long rifle and its red sight line, the Sapper's canisters, the Cleaner's
//! baton and his sprint, the Handler's red armband and radio, the Section
//! Chief's long coat, hat and badge. They walk with a stride, crouch in
//! cover, and turn to aim.
//!
//! The Executor is a gunmetal tank on two treads, a turret with a long gun,
//! its weak points burning ember; each phase its new cannons grow out of
//! it. In its last phase it is gold and lifted off the floor, wheels within
//! wheels turning round it, every surface opening eyes.

use super::rod;
use crate::content::{Content, EnemyDef, Kind, On, Rgb, Role};
use crate::enemy::{Enemy, Order, State, PHASE_TICKS, SPAWN_TICKS};
use crate::fx::{self, deg, int, ONE};
use crate::geom::{facing, right_of, v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::{boxed, mix, shade, Batch, PASS_SOLID};
use crate::tank;

fn cm(x: i32) -> i32 {
    ONE * x / 100
}

/// Draws an agent or the Executor. `target` is your chest, for a sight line.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    b: &mut Batch,
    c: &Content,
    e: &Enemy,
    d: &EnemyDef,
    at: V3,
    alpha: i32,
    target: V3,
    paint: &dyn Fn(Rgb) -> Rgb,
) {
    if d.kind == Kind::Tank {
        executor(b, c, e, d, at, paint);
    } else {
        agent(b, c, e, d, at, alpha, target, paint);
    }
}

#[allow(clippy::too_many_arguments)]
fn agent(
    b: &mut Batch,
    c: &Content,
    e: &Enemy,
    d: &EnemyDef,
    at: V3,
    alpha: i32,
    target: V3,
    paint: &dyn Fn(Rgb) -> Rgb,
) {
    let _ = alpha;
    let grow = match e.state {
        State::Spawning(t) => ONE - fx::div(t * ONE, SPAWN_TICKS * ONE),
        _ => ONE,
    };
    let chief = d.role == Role::Chief;
    let k = fx::mul(if chief { ONE * 13 / 10 } else { ONE }, grow.max(ONE / 20));
    let u = |x: i32| fx::mul(cm(x), k);
    let feet = at.sub(v3(0, d.hover, 0));
    let f = Affine::translate(feet).then(&Affine::rot_y(e.yaw));
    let suit = paint(c.pal.c(if chief { "agent_suit_dark" } else { "agent_suit" }));
    let dark = paint(c.pal.c("agent_suit_dark"));
    let shirt = paint(c.pal.c("agent_shirt"));
    let skin = paint(c.pal.c("agent_skin"));
    let shades = c.pal.c("agent_shades");
    let tie = c.pal.c("agent_tie");
    let aiming = matches!(e.state, State::Windup(_) | State::Casting(_) | State::Ready(_));
    // crouched behind cover while it rests there
    let crouch = e.order == Order::Cover && e.path.is_empty() && matches!(e.state, State::Rest(_));
    let drop = if crouch { u(45) } else { 0 };
    let running = d.role == Role::Rush && !aiming;
    let swing = fx::sin(e.stride) * 2 / 5;
    let moving = !e.path.is_empty() || running;
    let swing = if moving { swing } else { 0 };
    for (side, phase) in [(-1, swing), (1, -swing)] {
        let hip = v3(side * u(13), u(92) - drop, 0);
        let knee = if crouch {
            hip.add(v3(0, -u(10), -u(40)))
        } else {
            hip.add(v3(0, -u(44), -fx::mul(phase, u(40))))
        };
        let foot = if crouch {
            v3(side * u(15), u(4), u(10))
        } else {
            knee.add(v3(0, -u(44), fx::mul(phase, u(10))))
        };
        rod(b, f.apply(hip), f.apply(knee), u(16), dark, 0, true);
        rod(b, f.apply(knee), f.apply(foot), u(14), dark, 0, true);
        b.inked(
            Mesh::Cube,
            &f.then(&boxed(foot.add(v3(0, u(4), -u(5))), v3(u(14), u(9), u(28)))),
            shades,
            0,
        );
    }
    // a lean forward when it runs
    let lean = if running { deg(16) } else { 0 };
    let body = f
        .then(&Affine::translate(v3(0, u(92) - drop, 0)))
        .then(&Affine::rot_x(-lean));
    let coat_len = if chief || d.role == Role::Flank {
        u(95)
    } else {
        u(62)
    };
    b.inked(
        Mesh::Cube,
        &body.then(&boxed(
            v3(0, u(62) - coat_len / 2 + u(10), 0),
            v3(u(50), coat_len, u(30)),
        )),
        suit,
        0,
    );
    b.inked(
        Mesh::Wedge,
        &body.then(&boxed(v3(0, u(56), -u(15)), v3(u(16), u(20), u(2)))),
        shirt,
        0,
    );
    b.inked(
        Mesh::Cube,
        &body.then(&boxed(v3(0, u(50), -u(17)), v3(u(5), u(26), u(2)))),
        tie,
        0,
    );
    // the head: the same face on every one, and the glasses
    let head = body.then(&Affine::translate(v3(0, u(86), 0)));
    b.inked(
        Mesh::Sphere,
        &head.then(&boxed(V3::ZERO, v3(u(24), u(28), u(25)))),
        skin,
        0,
    );
    b.inked(
        Mesh::Sphere,
        &head.then(&boxed(v3(0, u(7), u(3)), v3(u(26), u(16), u(26)))),
        shades,
        0,
    );
    b.push(
        Mesh::Cube,
        &head.then(&boxed(v3(0, u(1), -u(12)), v3(u(22), u(5), u(3)))),
        shades,
        ONE / 6,
        PASS_SOLID,
    );
    if chief {
        // a hat, and the badge
        b.inked(
            Mesh::Cyl,
            &head.then(&boxed(v3(0, u(16), 0), v3(u(44), u(3), u(44)))),
            dark,
            0,
        );
        b.inked(
            Mesh::Cyl,
            &head.then(&boxed(v3(0, u(24), 0), v3(u(26), u(14), u(26)))),
            dark,
            0,
        );
        b.inked(
            Mesh::Octa,
            &body.then(&boxed(v3(u(14), u(54), -u(17)), v3(u(7), u(7), u(2)))),
            c.pal.c("agent_badge"),
            ONE / 2,
        );
    }
    if d.role == Role::Command {
        b.inked(
            Mesh::Cube,
            &body.then(&boxed(v3(-u(27), u(44), 0), v3(u(4), u(10), u(26)))),
            c.pal.c("lab_red"),
            0,
        );
        rod(
            b,
            head.apply(v3(u(12), 0, 0)),
            head.apply(v3(u(12), -u(22), -u(6))),
            u(2),
            shades,
            0,
            false,
        );
    }
    // the arms: out to the gun along the aim, or swinging
    let aim_to = target.sub(at);
    let (aim_yaw, aim_pitch) = crate::geom::yaw_pitch_of(aim_to);
    let aim = if aiming {
        facing(aim_yaw, aim_pitch)
    } else {
        facing(e.yaw, -deg(20))
    };
    let shoulder = |s: i32| body.apply(v3(s * u(27), u(56), 0));
    let hand = shoulder(1)
        .add(aim.scale(u(45)))
        .add(right_of(e.yaw).scale(-u(10)));
    match d.role {
        Role::Rush => {
            // a baton, and the off arm pumping
            rod(b, shoulder(1), hand, u(11), suit, 0, true);
            rod(
                b,
                hand,
                hand.add(aim.scale(u(60))).add(v3(0, u(20), 0)),
                u(6),
                shades,
                ONE / 8,
                true,
            );
            rod(
                b,
                shoulder(-1),
                shoulder(-1).add(v3(0, -u(40), fx::mul(swing, u(30)))),
                u(11),
                suit,
                0,
                true,
            );
        }
        _ => {
            rod(b, shoulder(1), hand, u(11), suit, 0, true);
            rod(b, shoulder(-1), hand.add(aim.scale(u(20))), u(11), suit, 0, true);
            let long = if d.role == Role::Sniper { u(140) } else { u(70) };
            rod(
                b,
                hand.sub(aim.scale(u(15))),
                hand.add(aim.scale(long)),
                u(8),
                c.pal.c("tank_gun"),
                0,
                true,
            );
        }
    }
    if d.role == Role::Shield {
        // the riot shield, held in front, its window strip
        let fwd = facing(e.yaw, 0);
        let centre = feet.add(v3(0, u(90) - drop, 0)).add(fwd.scale(u(45)));
        let shield = Affine::translate(centre)
            .then(&Affine::rot_y(e.yaw))
            .then(&Affine::scale(v3(u(90), u(150), u(5))));
        b.inked(Mesh::Cube, &shield, paint(c.pal.c("agent_shield")), ONE / 10);
        b.push(
            Mesh::Cube,
            &Affine::translate(centre.add(v3(0, u(40), 0)).add(fwd.scale(u(3))))
                .then(&Affine::rot_y(e.yaw))
                .then(&Affine::scale(v3(u(60), u(12), u(2)))),
            c.pal.c("glass"),
            ONE / 3,
            PASS_SOLID,
        );
    }
    if d.role == Role::Grenadier {
        for s in 0..3 {
            let p = body.apply(v3(-u(18) + u(18) * s, u(30), u(18)));
            b.inked(
                Mesh::Cyl,
                &boxed(p, v3(u(10), u(20), u(10))),
                c.pal.c("lime"),
                ONE / 4,
            );
        }
    }
    // the windup: the gun's mouth burns, and a marksman's sight line
    let charge = e.glow(c);
    let muzzle = e.muzzle(d);
    if charge > 0 {
        let h = fx::mul(u(40), ONE + charge);
        b.glow(
            Mesh::Sphere,
            &boxed(muzzle, v3(h, h, h)),
            c.pal.c("scarlet"),
            charge / 2,
        );
    }
    if d.role == Role::Sniper && matches!(e.state, State::Windup(_) | State::Ready(_)) {
        let to = e.locked.unwrap_or(target);
        let strength = if e.locked.is_some() { ONE } else { ONE / 3 };
        rod(b, muzzle, to, cm(3), c.pal.c("sight"), strength, false);
    }
}

fn executor(b: &mut Batch, c: &Content, e: &Enemy, d: &EnemyDef, at: V3, paint: &dyn Fn(Rgb) -> Rgb) {
    let Some(t) = e.tank.as_ref() else {
        return;
    };
    let last = t.survive > 0 || t.lift > 0;
    let gold = c.pal.c("tank_halo");
    let tone = |x: Rgb| {
        let x = paint(x);
        if last {
            mix(x, gold, ONE / 2)
        } else {
            x
        }
    };
    let hull_c = tone(c.pal.c("tank_hull"));
    let plate = tone(c.pal.c("tank_plate"));
    let tread = c.pal.c("tank_tread");
    let gun = c.pal.c("tank_gun");
    let hf = Affine::translate(at).then(&Affine::rot_y(t.hull));
    let (len, wide) = (tank::HULL_LEN, tank::HULL_WIDE);
    // the hull: a body, a sloped glacis at the front (−z is forward), an
    // engine deck behind with grilles
    b.inked(
        Mesh::Cube,
        &hf.then(&boxed(v3(0, 0, 0), v3(wide - int(1), int(15) / 10, len))),
        hull_c,
        0,
    );
    let glacis = hf
        .then(&Affine::translate(v3(0, ONE / 3, -len / 2 - ONE / 2)))
        .then(&Affine::rot_y(fx::HALF))
        .then(&Affine::scale(v3(wide - int(1), ONE, int(1))));
    b.inked(Mesh::Wedge, &glacis, plate, 0);
    for k in 0..4 {
        b.inked(
            Mesh::Cube,
            &hf.then(&boxed(
                v3(-ONE + cm(65) * k, cm(78), len / 2 - int(1)),
                v3(cm(40), cm(6), int(1)),
            )),
            tread,
            0,
        );
    }
    // the treads, their road wheels turning, the links running
    for s in [-1, 1] {
        let x = s * (wide / 2 - cm(40));
        b.inked(
            Mesh::Cube,
            &hf.then(&boxed(v3(x, -cm(30), 0), v3(cm(90), cm(130), len + cm(60)))),
            tread,
            0,
        );
        for w in 0..6 {
            let z = -len / 2 + cm(60) + (len - cm(120)) / 5 * w;
            let wheel = hf
                .then(&Affine::translate(v3(x + s * cm(48), -cm(40), z)))
                .then(&Affine::rot_z(fx::QUARTER))
                .then(&Affine::rot_y(t.roll * 8))
                .then(&Affine::scale(v3(cm(80), cm(10), cm(80))));
            b.inked(Mesh::Cyl, &wheel, plate, 0);
        }
        let off = (t.roll * 4).rem_euclid(cm(50));
        for l in 0..16 {
            let z = -len / 2 + off + cm(50) * l;
            if z > len / 2 {
                continue;
            }
            b.push(
                Mesh::Cube,
                &hf.then(&boxed(v3(x, cm(36), z), v3(cm(92), cm(6), cm(12)))),
                plate,
                0,
                PASS_SOLID,
            );
        }
    }
    // the turret, the long gun with its muzzle brake
    let tf = Affine::translate(at.add(v3(0, tank::TURRET_Y, 0))).then(&Affine::rot_y(t.turret));
    b.inked(
        Mesh::Cyl,
        &tf.then(&boxed(v3(0, cm(20), 0), v3(int(4), cm(120), int(4)))),
        hull_c,
        0,
    );
    b.inked(
        Mesh::Cube,
        &tf.then(&boxed(v3(0, cm(40), cm(80)), v3(cm(300), cm(90), cm(200)))),
        plate,
        0,
    );
    b.inked(
        Mesh::Cyl,
        &tf.then(&boxed(v3(cm(80), cm(100), cm(60)), v3(cm(60), cm(30), cm(60)))),
        hull_c,
        0,
    );
    let root = at
        .add(v3(0, tank::TURRET_Y + ONE / 2, 0))
        .add(facing(t.turret, 0).scale(int(2)));
    let mouth = tank::main_gun(e);
    rod(b, root, mouth, cm(32), gun, 0, true);
    // the muzzle brake, along the barrel
    let along = mouth.sub(root).norm();
    rod(
        b,
        mouth.sub(along.scale(cm(40))),
        mouth.add(along.scale(cm(10))),
        cm(55),
        gun,
        0,
        true,
    );
    // the cannons it has grown, the newest growing in
    for (k, p) in tank::cannons(d, e.phase) {
        let grow = if k == e.phase && e.phase > 0 {
            fx::div(t.grown.min(PHASE_TICKS) * ONE, PHASE_TICKS * ONE)
        } else {
            ONE
        };
        let place = tank::place(e, t, &p);
        let yaw = match p.on {
            On::Hull => t.hull,
            On::Turret => t.turret,
        } + p.yaw;
        let dir = facing(yaw, p.pitch);
        let s = fx::mul(cm(45), grow.max(ONE / 20));
        b.inked(Mesh::Sphere, &boxed(place, v3(s, s, s)), plate, 0);
        rod(
            b,
            place,
            place.add(dir.scale(fx::mul(int(1) + cm(40), grow))),
            fx::mul(cm(22), grow),
            gun,
            0,
            true,
        );
        if last {
            eye(
                b,
                c,
                place.add(dir.neg().scale(cm(30))).add(v3(0, cm(40), 0)),
                dir,
                fx::mul(cm(40), grow),
                e.age + k as i32 * 17,
            );
        }
    }
    // the weak points: embers in a ring, pulsing; the windup lights them
    let pulse = ONE * 3 / 4 + fx::sin(e.age * deg(8)) / 4;
    for (p, r, _) in tank::weak_points(e, d) {
        b.push(
            Mesh::Sphere,
            &boxed(p, v3(r * 2, r * 2, r * 2)),
            c.pal.c("tank_weak"),
            pulse,
            PASS_SOLID,
        );
        b.glow(
            Mesh::Sphere,
            &boxed(p, v3(r * 4, r * 4, r * 4)),
            c.pal.c("tank_weak"),
            pulse / 3,
        );
        b.inked(Mesh::Ring, &boxed(p, v3(r * 3, cm(10), r * 3)), plate, 0);
    }
    let charge = e.glow(c);
    if charge > 0 {
        for m in tank::muzzles(e, d).into_iter().chain(std::iter::once(mouth)) {
            let h = fx::mul(cm(60), ONE + charge);
            b.glow(
                Mesh::Sphere,
                &boxed(m, v3(h, h, h)),
                c.pal.c("scarlet"),
                charge / 2,
            );
        }
    }
    if last {
        // wheels within wheels, rimmed with eyes, and a crown of light
        let centre = at.add(v3(0, int(2), 0));
        for (w, axis_tilt) in [(0, deg(90)), (1, deg(0)), (2, deg(45))] {
            let r = int(6) + int(1) * w;
            let turn = e.age * deg(1) * (w + 1) * if w % 2 == 0 { 1 } else { -1 };
            let wheel = Affine::translate(centre)
                .then(&Affine::rot_y(turn))
                .then(&Affine::rot_x(axis_tilt));
            b.inked(
                Mesh::Ring,
                &wheel.then(&Affine::scale(v3(r * 2, cm(30), r * 2))),
                gold,
                ONE / 3,
            );
            for k in 0..10 {
                let a = fx::TURN / 10 * k;
                let p = wheel.apply(v3(fx::mul(fx::cos(a), r), 0, fx::mul(fx::sin(a), r)));
                let out = p.sub(centre).norm();
                eye(b, c, p, out, cm(55), e.age + k * 31 + w * 7);
            }
        }
        b.glow(
            Mesh::Ring,
            &boxed(at.add(v3(0, int(6), 0)), v3(int(5), cm(20), int(5))),
            gold,
            ONE,
        );
        b.glow(
            Mesh::Sphere,
            &boxed(centre, v3(int(14), int(10), int(14))),
            gold,
            ONE / 10,
        );
        // the hull itself opens eyes
        for k in 0..8 {
            let p = hf.apply(v3(
                if k % 2 == 0 { -int(2) } else { int(2) },
                cm(80),
                -int(3) + cm(85) * k,
            ));
            eye(b, c, p, V3::UP, cm(45), e.age + k * 13);
        }
    }
}

/// An eye at `at` looking along `dir`: white, an iris, a pupil, and a
/// blink now and then.
fn eye(b: &mut Batch, c: &Content, at: V3, dir: V3, s: i32, clock: i32) {
    let blink = if clock % 180 < 8 { ONE / 6 } else { ONE };
    let f = Affine::translate(at)
        .then(&Affine::looking(dir))
        .then(&Affine::scale(v3(s, fx::mul(s, blink), s)));
    b.inked(Mesh::Sphere, &f, c.pal.c("tank_eye"), ONE / 3);
    let iris = at.add(dir.scale(s * 4 / 10));
    let k = s / 2;
    b.push(
        Mesh::Sphere,
        &boxed(iris, v3(k, fx::mul(k, blink), k)),
        c.pal.c("tank_iris"),
        ONE / 2,
        PASS_SOLID,
    );
    let pupil = at.add(dir.scale(s / 2));
    let p = s / 4;
    b.push(
        Mesh::Sphere,
        &boxed(pupil, v3(p, fx::mul(p, blink), p)),
        c.pal.c("house_ink"),
        0,
        PASS_SOLID,
    );
    let _ = shade;
}
