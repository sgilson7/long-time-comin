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
use crate::content::{Content, EnemyDef, Kind, Lunge, On, Rgb, Role};
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
    match d.kind {
        Kind::Tank => executor(b, c, e, d, at, paint),
        Kind::Mech => mech(b, c, e, d, at, target, paint),
        Kind::Double => double(b, c, e, d, at, target),
        _ => agent(b, c, e, d, at, alpha, target, paint),
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
    // the Ascent's assassins wear your own coat, under a clock-face mask
    let you = c.zone_of_enemy(e.def).unwrap_or(0) >= 3;
    let suit = paint(c.pal.c(if you {
        "player_coat"
    } else if chief {
        "agent_suit_dark"
    } else {
        "agent_suit"
    }));
    let dark = paint(c.pal.c(if you { "assassin_dark" } else { "agent_suit_dark" }));
    let shirt = paint(c.pal.c(if you { "player_trim" } else { "agent_shirt" }));
    let skin = paint(c.pal.c(if you { "clock" } else { "agent_skin" }));
    let shades = c.pal.c(if you { "assassin_dark" } else { "agent_shades" });
    let tie = c.pal.c(if you { "player_trim" } else { "agent_tie" });
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
    let sc = t.scale;
    let hf = Affine::translate(at)
        .then(&Affine::rot_y(t.hull))
        .then(&Affine::scale(v3(sc, sc, sc)));
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
    let tf = Affine::translate(at.add(v3(0, fx::mul(tank::TURRET_Y, sc), 0)))
        .then(&Affine::rot_y(t.turret))
        .then(&Affine::scale(v3(sc, sc, sc)));
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
        .add(v3(0, fx::mul(tank::TURRET_Y + ONE / 2, sc), 0))
        .add(facing(t.turret, 0).scale(fx::mul(int(2), sc)));
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

/// The Pioneer: a frame after Iron Vector's, boxy legs that bend back, a
/// core, a head with a lit visor, a rifle on each arm, missile pods on the
/// shoulders, and boosters that burn when it quick-boosts or rises.
fn mech(b: &mut Batch, c: &Content, e: &Enemy, d: &EnemyDef, at: V3, target: V3, paint: &dyn Fn(Rgb) -> Rgb) {
    let k = d.radius;
    let u = |x: i32| fx::mul(k, ONE * x / 100);
    let primary = paint(c.pal.c("mech_primary"));
    let second = paint(c.pal.c("mech_secondary"));
    let dark = c.pal.c("mech_dark");
    let glow = c.pal.c("mech_glow");
    let feet = at.sub(v3(0, d.hover + e.lift, 0)).add(v3(0, e.lift, 0));
    let f = Affine::translate(feet).then(&Affine::rot_y(e.yaw));
    let hip_y = d.hover;
    let step = fx::sin(e.age * fx::TURN / 50);
    for side in [-1, 1] {
        let swing = if e.lift > ONE {
            0
        } else {
            fx::mul(step * side, u(20))
        };
        let hip = v3(side * u(40), hip_y, 0);
        let knee = v3(side * u(48), hip_y / 2, u(30) + swing);
        let foot = v3(side * u(50), u(6), -swing / 2);
        rod(b, f.apply(hip), f.apply(knee), u(30), second, 0, true);
        rod(b, f.apply(knee), f.apply(foot), u(26), primary, 0, true);
        b.inked(
            Mesh::Cube,
            &f.then(&boxed(foot.add(v3(0, u(4), -u(10))), v3(u(36), u(14), u(60)))),
            dark,
            0,
        );
    }
    let core = f.then(&Affine::translate(v3(0, hip_y + u(45), 0)));
    b.inked(
        Mesh::Cube,
        &core.then(&boxed(V3::ZERO, v3(u(110), u(80), u(80)))),
        primary,
        0,
    );
    b.inked(
        Mesh::Wedge,
        &core.then(&boxed(v3(0, u(30), -u(45)), v3(u(90), u(30), u(20)))),
        second,
        0,
    );
    let head = core.then(&Affine::translate(v3(0, u(62), -u(10))));
    b.inked(
        Mesh::Cube,
        &head.then(&boxed(V3::ZERO, v3(u(50), u(34), u(50)))),
        second,
        0,
    );
    b.push(
        Mesh::Cube,
        &head.then(&boxed(v3(0, 0, -u(26)), v3(u(40), u(8), u(4)))),
        glow,
        ONE,
        PASS_SOLID,
    );
    // the arms and their rifles, toward you
    let (aim_yaw, aim_pitch) = crate::geom::yaw_pitch_of(target.sub(at));
    let aim = facing(aim_yaw, aim_pitch);
    for side in [-1, 1] {
        let shoulder = core.apply(v3(side * u(70), u(20), 0));
        let hand = shoulder.add(aim.scale(u(60))).add(v3(0, -u(30), 0));
        rod(b, shoulder, hand, u(24), primary, 0, true);
        rod(b, hand, hand.add(aim.scale(u(110))), u(16), dark, 0, true);
        // the shoulder pods
        let pod = core.apply(v3(side * u(46), u(60), u(30)));
        b.inked(Mesh::Cube, &boxed(pod, v3(u(36), u(30), u(50))), second, 0);
        for j in 0..2 {
            b.push(
                Mesh::Cyl,
                &boxed(
                    pod.add(v3(side * u(6) * (j * 2 - 1), u(8), -u(26))),
                    v3(u(10), u(4), u(10)),
                ),
                glow,
                ONE / 2,
                PASS_SOLID,
            );
        }
    }
    // the boosters, burning when it bursts or rises
    let burn = if e.boost > 0 {
        ONE
    } else if e.lift > ONE / 4 {
        ONE * 2 / 3
    } else {
        0
    };
    for side in [-1, 1] {
        let nozzle = core.apply(v3(side * u(34), -u(10), u(48)));
        b.inked(Mesh::Cyl, &boxed(nozzle, v3(u(26), u(30), u(26))), dark, 0);
        if burn > 0 {
            let flame = Affine::translate(nozzle.add(v3(0, -u(40), u(20))))
                .then(&Affine::rot_x(fx::HALF))
                .then(&Affine::scale(v3(u(30), fx::mul(u(110), burn), u(30))));
            b.glow(Mesh::Cone, &flame, c.pal.c("mech_flame"), burn);
        }
    }
    let charge = e.glow(c);
    if charge > 0 {
        let h = fx::mul(u(120), ONE + charge);
        b.glow(
            Mesh::Sphere,
            &boxed(core.apply(V3::ZERO), v3(h, h, h)),
            glow,
            charge / 3,
        );
    }
}

/// The Defendant: you, older, in your coat, a ring of hours turning round
/// you and your eyes gold; the world goes grey when you stop time.
fn double(b: &mut Batch, c: &Content, e: &Enemy, d: &EnemyDef, at: V3, target: V3) {
    let feet = at.sub(v3(0, d.hover + e.lift, 0)).add(v3(0, e.lift, 0));
    let (yaw, _) = crate::geom::yaw_pitch_of(target.sub(at));
    super::standing(b, c, feet, yaw, c.gun("big_iron").min(c.guns.len() - 1));
    let gold = c.pal.c("gold");
    let head = feet.add(v3(0, cm(178), 0));
    b.push(
        Mesh::Sphere,
        &boxed(head.add(facing(yaw, 0).scale(cm(12))), v3(cm(14), cm(5), cm(5))),
        gold,
        ONE,
        PASS_SOLID,
    );
    for k in 0..12 {
        let a = fx::TURN / 12 * k + e.age * deg(2);
        let p = feet.add(v3(
            fx::mul(fx::cos(a), cm(120)),
            cm(130) + fx::mul(fx::sin(a * 2), cm(10)),
            fx::mul(fx::sin(a), cm(120)),
        ));
        b.inked(
            Mesh::Cube,
            &boxed(p, v3(cm(8), cm(18), cm(4))),
            c.pal.c("clock"),
            ONE / 3,
        );
    }
    // a tell in its windup only, kept close: a glow that hung round it
    // through every cast read as a shield (SECOND-ORDER Q13)
    if let State::Windup(_) = e.state {
        let charge = e.glow(c);
        let h = fx::mul(cm(90), ONE + charge);
        b.glow(
            Mesh::Sphere,
            &boxed(feet.add(v3(0, cm(100), 0)), v3(h, h, h)),
            gold,
            charge / 4,
        );
    }
    let hand = feet.add(v3(0, cm(120), 0));
    match (e.lunge, e.state) {
        // the chain flies out through its windup, then pulls it along
        (Lunge::Hook, State::Windup(_)) => {
            let tip = hand.lerp(e.anchor.add(v3(0, ONE, 0)), e.glow(c));
            rod(b, hand, tip, cm(5), c.pal.c("hook"), ONE / 2, false);
            b.inked(
                Mesh::Octa,
                &boxed(tip, v3(cm(40), cm(40), cm(40))),
                c.pal.c("hook"),
                ONE / 2,
            );
        }
        (Lunge::Hook, State::Casting(_)) if e.anchor.dist(feet) > crate::balance::LUNGE_HOOK_ARRIVE => {
            let tip = e.anchor.add(v3(0, ONE, 0));
            rod(b, hand, tip, cm(5), c.pal.c("hook"), ONE / 2, false);
            b.inked(
                Mesh::Octa,
                &boxed(tip, v3(cm(40), cm(40), cm(40))),
                c.pal.c("hook"),
                ONE / 2,
            );
        }
        // where it will land, marked on the floor as it leaps
        (Lunge::Pound, State::Windup(_)) => {
            let r = crate::balance::LUNGE_POUND_HEIGHT * 2 / 3;
            for k in 0..16 {
                let a = fx::TURN / 16 * k + e.age * deg(3);
                let p = e
                    .anchor
                    .add(v3(fx::mul(fx::cos(a), r), cm(10), fx::mul(fx::sin(a), r)));
                b.glow(
                    Mesh::Cube,
                    &boxed(p, v3(cm(20), cm(6), cm(20))),
                    c.pal.c("pound"),
                    ONE / 2,
                );
            }
            for k in 1..4 {
                let g = feet.add(v3(0, -cm(60) * k, 0));
                b.glow(
                    Mesh::Sphere,
                    &boxed(g, v3(cm(50), cm(90), cm(50))),
                    c.pal.c("pound"),
                    ONE / (k + 1),
                );
            }
        }
        _ => {}
    }
    if e.boost > 0 {
        for k in 1..4 {
            let g = feet.sub(e.vel.scale(ONE * 3 * k)).add(v3(0, cm(90), 0));
            b.glow(
                Mesh::Sphere,
                &boxed(g, v3(cm(60), cm(160), cm(60))),
                gold,
                ONE / (k + 1),
            );
        }
    }
}

/// The Idiot God, bursting into the sky over the summit: a heaving mass of
/// dark bubbles, more eyes than can be counted, arms turning round it, and
/// a maw that opens and draws everything in. `k` is how far it has come.
pub fn azathoth(b: &mut Batch, c: &Content, clock: i32, k: i32) {
    let maw = crate::world::MAW;
    let grow = fx::mul(k.min(ONE / 6) * 6, ONE).min(ONE);
    let size = fx::mul(fx::int(90), grow.max(ONE / 50));
    let flesh = c.pal.c("eldritch_flesh");
    let dark = c.pal.c("eldritch_dark");
    let glow = c.pal.c("eldritch_glow");
    let s = |x: i32| fx::mul(size, ONE * x / 100);
    // the bubbling mass
    for n in 0..40 {
        let a = fx::TURN / 40 * n * 7;
        let r = s(60 + (n * 13) % 50);
        let up = fx::sin(a * 3 + clock * deg(1) * (1 + n % 3));
        let p = maw.add(v3(
            fx::mul(fx::cos(a), r),
            fx::mul(up, s(40)),
            fx::mul(fx::sin(a), r) / 2 - s(30),
        ));
        let bs = s(30 + (n * 7) % 30) + fx::mul(fx::sin(clock * deg(3) + n * deg(40)), s(6));
        b.inked(
            Mesh::Blob,
            &boxed(p, v3(bs, bs, bs)),
            if n % 3 == 0 { dark } else { flesh },
            ONE / 8,
        );
        if n % 2 == 0 {
            eye(
                b,
                c,
                p.add(v3(0, 0, bs / 2)),
                v3(0, 0, ONE),
                bs / 3,
                clock + n * 17,
            );
        }
    }
    // the arms, turning
    for n in 0..9 {
        let a = fx::TURN / 9 * n + clock * deg(1) / 2;
        let mut p = maw;
        for j in 1..7 {
            let q = maw.add(v3(
                fx::mul(fx::cos(a + j * deg(20)), s(40 + 22 * j)),
                fx::mul(fx::sin(a * 2 + j * deg(30) + clock * deg(2)), s(30)),
                fx::mul(fx::sin(a + j * deg(20)), s(20 + 10 * j)),
            ));
            rod(b, p, q, s(14) - s(2) * j, dark, ONE / 10, true);
            p = q;
        }
    }
    // the maw: a ring of teeth round the dark, opening
    let open = s(55);
    for n in 0..24 {
        let a = fx::TURN / 24 * n + clock * deg(1);
        let p = maw.add(v3(fx::mul(fx::cos(a), open), fx::mul(fx::sin(a), open), s(30)));
        let tip = maw.add(v3(
            fx::mul(fx::cos(a), open / 2),
            fx::mul(fx::sin(a), open / 2),
            s(40),
        ));
        rod(b, p, tip, s(6), c.pal.c("clock"), 0, true);
    }
    b.push(
        Mesh::Sphere,
        &boxed(maw.add(v3(0, 0, s(25))), v3(open * 2, open * 2, s(10))),
        c.pal.c("eldritch_void"),
        0,
        PASS_SOLID,
    );
    b.glow(
        Mesh::Sphere,
        &boxed(maw, v3(size * 3, size * 3, size * 2)),
        glow,
        ONE / 6,
    );
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
