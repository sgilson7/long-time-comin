//! The moving things, as shapes for the renderer: the Defendant and their
//! gun, every kind of enemy and the Bailiff, orbs with their halos, ground
//! waves, shots, pickups, gates, the hook's line and the sparks. Everything
//! is placed between the last two ticks by `alpha`.

use crate::balance::*;
use crate::content::{Content, Kind, Rgb};
use crate::enemy::{Enemy, State, SPAWN_TICKS};
use crate::fx::{self, deg, int, ONE};
use crate::geom::{facing, right_of, v3, Affine, V3};
use crate::mesh::Mesh;
use crate::player::Player;
use crate::render::{boxed, mix, shade, Batch};
use crate::world::{FxKind, PickupKind, World};

mod beasts;
mod fauna;
mod house;
pub use house::azathoth;

fn dm(x: i32) -> i32 {
    ONE * x / 10
}

fn cm(x: i32) -> i32 {
    ONE * x / 100
}

/// A unit shape stretched from `a` to `b`, `w` across.
fn rod(b: &mut Batch, a: V3, c: V3, w: i32, rgb: Rgb, glow: i32, ink: bool) {
    if let Some(af) = crate::arena::segment(a, c, w) {
        if ink {
            b.inked(Mesh::Cyl, &af, rgb, glow);
        } else {
            b.push(Mesh::Cyl, &af, rgb, glow, crate::render::PASS_SOLID);
        }
    }
}

/// A basis turned to `yaw` at `at`, so a body's parts are placed in its own
/// frame: x right, y up, −z forward.
fn frame(at: V3, yaw: i32) -> Affine {
    Affine::translate(at).then(&Affine::rot_y(yaw))
}

fn part(b: &mut Batch, f: &Affine, mesh: Mesh, at: V3, size: V3, rgb: Rgb) {
    b.inked(mesh, &f.then(&boxed(at, size)), rgb, 0);
}

pub fn world(
    b: &mut Batch,
    c: &Content,
    w: &World,
    alpha: i32,
    clock: i32,
    body: bool,
    cam: &crate::render::Camera,
) {
    if body {
        player(b, c, &w.player, w.held.gun, alpha);
    }
    // enemies in a heavy black line, so their shapes stand out from the
    // warm city behind them
    b.bold = true;
    let chest = w.player.chest();
    for e in w.enemies.iter().filter(|e| e.alive()) {
        enemy(b, c, e, alpha, chest);
    }
    b.bold = false;
    // the Conservatory's glass, until it breaks
    let glass = c.pal.c("glass");
    for (k, p) in w.arena.panes.iter().enumerate() {
        let broken = w.broken.get(k).is_some_and(|t| *t != 0 && w.tick >= *t);
        if !broken {
            b.glow(Mesh::Cube, &boxed(p.centre, p.size), glass, ONE / 3);
        }
    }
    orbs(b, c, w, alpha, cam);
    if w.ending > 0 {
        house::azathoth(b, c, clock, w.melt());
    }
    // a revolver loading: the cylinder swung out, a shell going in
    let gl = c.guns[w.held.gun].level(w.held.level);
    if body && w.held.reloading(gl.mag) {
        let hand = hand_of(&w.player, alpha);
        let side = right_of(w.player.yaw).scale(-cm(12));
        let cyl = hand.add(side).add(v3(0, cm(4), 0));
        let spin = fx::div(w.held.loading * ONE, gl.shell.max(1) * ONE);
        let f = Affine::translate(cyl)
            .then(&Affine::rot_y(w.player.yaw))
            .then(&Affine::rot_x(fx::QUARTER))
            .then(&Affine::rot_y(fx::mul(spin, fx::TURN / 6)));
        b.inked(
            Mesh::Cyl,
            &f.then(&Affine::scale(v3(cm(14), cm(10), cm(14)))),
            c.pal.c("player_gun"),
            0,
        );
        for k in 0..gl.mag {
            let a = fx::TURN / gl.mag * k;
            let full = k < w.held.rounds;
            let p = f.apply(v3(fx::mul(fx::cos(a), cm(4)), cm(6), fx::mul(fx::sin(a), cm(4))));
            let col = if full { c.pal.c("brass") } else { c.pal.c("shadow") };
            b.push(
                Mesh::Cyl,
                &boxed(p, v3(cm(3), cm(3), cm(3))),
                col,
                ONE / 3,
                crate::render::PASS_SOLID,
            );
        }
        // the next shell, rising into its chamber
        let shell = cyl.add(v3(0, -cm(30) + fx::mul(spin, cm(26)), 0));
        b.inked(
            Mesh::Cyl,
            &boxed(shell, v3(cm(4), cm(8), cm(4))),
            c.pal.c("brass"),
            ONE / 3,
        );
    }
    if let Some(wall) = w.wall() {
        injunction(b, c, &wall, w.player.wall_left);
    }
    for s in &w.shots {
        let at = s.prev.lerp(s.pos, alpha);
        let tail = at.sub(s.vel.scale(ONE * 6 / 10));
        match s.kind {
            crate::weapons::ShotKind::Bullet => {
                rod(b, tail, at, s.radius * 2, s.rgb, ONE, false);
            }
            crate::weapons::ShotKind::Thrown(def) => {
                // the body, tumbling
                let d = &c.enemies[def];
                let mut e = Enemy::new(def, d, at, 1);
                e.state = State::Rest(1);
                e.yaw = w.tick * deg(23);
                e.age = w.tick;
                enemy(b, c, &e, ONE, chest);
                b.glow(
                    Mesh::Sphere,
                    &boxed(at, v3(d.radius * 3, d.radius * 3, d.radius * 3)),
                    s.rgb,
                    ONE / 4,
                );
            }
            crate::weapons::ShotKind::Saw => {
                // a spinning blade, its teeth round its rim
                let spin = w.tick * deg(40);
                let r = s.radius * 2;
                let disc = Affine::translate(at)
                    .then(&Affine::rot_y(spin))
                    .then(&Affine::rot_z(deg(70)));
                b.inked(
                    Mesh::Cyl,
                    &disc.then(&Affine::scale(v3(r, cm(4), r))),
                    s.rgb,
                    ONE / 3,
                );
                for k in 0..8 {
                    let a = fx::TURN / 8 * k;
                    let tooth = disc
                        .then(&Affine::translate(v3(
                            fx::mul(fx::cos(a), r / 2),
                            0,
                            fx::mul(fx::sin(a), r / 2),
                        )))
                        .then(&Affine::rot_y(-a))
                        .then(&Affine::scale(v3(cm(6), cm(4), cm(10))));
                    b.push(Mesh::Wedge, &tooth, s.rgb, ONE / 2, crate::render::PASS_SOLID);
                }
                if s.lodged.is_some() && w.revving > 0 {
                    b.glow(
                        Mesh::Sphere,
                        &boxed(at, v3(r * 2, r * 2, r * 2)),
                        c.pal.c("spark"),
                        ONE / 3,
                    );
                }
            }
            _ => {
                let r = s.radius * 2;
                b.inked(Mesh::Sphere, &boxed(at, v3(r, r, r)), s.rgb, ONE);
                b.glow(
                    Mesh::Sphere,
                    &boxed(at, v3(r * 2, r * 2, r * 2)),
                    s.rgb,
                    ONE * 6 / 10,
                );
            }
        }
    }
    for p in w.pickups.iter().filter(|p| !p.taken) {
        let bob = fx::sin(p.age * fx::TURN / 120) / 5;
        let spin = p.age * fx::TURN / 180;
        let at = p.pos.add(v3(0, ONE + bob, 0));
        pickup_icon(b, c, p.kind, at, spin, ONE);
        let col = pickup_colour(c, p.kind);
        b.push(
            Mesh::Cyl,
            &boxed(p.pos.add(v3(0, cm(5), 0)), v3(dm(14), cm(10), dm(14))),
            shade(col, 70),
            ONE / 2,
            crate::render::PASS_SOLID,
        );
        b.glow(
            Mesh::Cyl,
            &boxed(p.pos.add(v3(0, int(2), 0)), v3(dm(9), int(4), dm(9))),
            col,
            ONE / 4,
        );
    }
    for g in &w.gates {
        gate(b, c, g, w.cleared, clock);
    }
    if let Some(g) = w.player.grapple {
        let hand = hand_of(&w.player, alpha);
        rod(b, hand, g.anchor, cm(5), c.pal.c("hook"), ONE / 2, false);
        b.inked(
            Mesh::Octa,
            &boxed(g.anchor, v3(dm(4), dm(4), dm(4))),
            c.pal.c("hook"),
            ONE / 2,
        );
    }
    for e in &w.effects {
        let k = fx::div(e.age * ONE, e.life.max(1) * ONE);
        match e.kind {
            FxKind::Spark => {
                let s = fx::mul(e.size, ONE - k).max(ONE / 50);
                b.push(
                    Mesh::Cube,
                    &boxed(e.pos, v3(s, s, s)),
                    e.rgb,
                    ONE,
                    crate::render::PASS_SOLID,
                );
            }
            FxKind::Burst => {
                let s = fx::mul(e.size, ONE / 3 + k);
                b.glow(Mesh::Sphere, &boxed(e.pos, v3(s, s, s)), e.rgb, ONE - k);
            }
            FxKind::Flash => {
                let s = fx::mul(e.size, ONE - k / 2);
                b.glow(Mesh::Sphere, &boxed(e.pos, v3(s, s, s)), e.rgb, (ONE - k) / 2);
            }
            FxKind::Ring => {
                let s = fx::mul(e.size, k.max(ONE / 10));
                b.push(
                    Mesh::Ring,
                    &boxed(e.pos, v3(s, dm(3), s)),
                    e.rgb,
                    ONE - k,
                    crate::render::PASS_SOLID,
                );
                b.glow(Mesh::Cyl, &boxed(e.pos, v3(s, dm(6), s)), e.rgb, (ONE - k) / 3);
            }
            FxKind::Shard => {
                // a pane's piece, tumbling as it falls
                let spin = e.age * deg(9) + e.size;
                let a = Affine::translate(e.pos)
                    .then(&Affine::rot_y(spin))
                    .then(&Affine::rot_x(spin / 2))
                    .then(&Affine::scale(v3(e.size, e.size, cm(3))));
                // pale and lit, so it reads against the dark room
                b.push(Mesh::Cube, &a, e.rgb, (ONE - k) / 2, crate::render::PASS_SOLID);
            }
        }
    }
    // a soft shadow under the body, so a jump reads against the floor
    let feet = w.player.prev_pos.lerp(w.player.pos, alpha);
    if let Some(floor) = w.arena.level.floor_below(feet.x, feet.y + dm(1), feet.z) {
        let high = (feet.y - floor).clamp(0, int(8));
        let s = dm(9) - fx::mul(high, ONE / 20);
        b.push(
            Mesh::Cyl,
            &boxed(v3(feet.x, floor + cm(2), feet.z), v3(s, cm(1), s)),
            c.pal.c("shadow"),
            0,
            crate::render::PASS_SOLID,
        );
    }
}

fn hand_of(p: &Player, alpha: i32) -> V3 {
    let feet = p.prev_pos.lerp(p.pos, alpha);
    feet.add(v3(0, PLAYER_CHEST + dm(1), 0))
        .add(right_of(p.yaw).scale(dm(4)))
        .add(facing(p.yaw, p.pitch).scale(dm(6)))
}

/// The Defendant: a long coat, a scarf that trails, legs that stride, and
/// the gun held out along the aim.
pub fn player(b: &mut Batch, c: &Content, p: &Player, gun: usize, alpha: i32) {
    let feet = p.prev_pos.lerp(p.pos, alpha);
    let coat = c.pal.c("player_coat");
    let trim = c.pal.c("player_trim");
    let skin = c.pal.c("player_skin");
    let hair = c.pal.c("player_hair");
    let dark = shade(coat, 70);
    let hurt = p.hurt_age < 6;
    let tint = |x: Rgb| if hurt { mix(x, [ONE; 3], ONE / 2) } else { x };
    let f = frame(feet, p.yaw);
    // legs: a stride on the ground, tucked in the air
    let swing = if p.grounded { fx::sin(p.stride) * 2 / 5 } else { 0 };
    let tuck = if p.grounded { 0 } else { dm(2) };
    for (side, phase) in [(-1, swing), (1, -swing)] {
        let hip = v3(side * cm(13), dm(9), 0);
        let knee = hip.add(v3(0, -dm(4) + tuck, -fx::mul(phase, dm(4)) - tuck));
        let foot = knee.add(v3(0, -cm(45) + tuck / 2, fx::mul(phase, dm(1)) + tuck / 2));
        rod(b, f.apply(hip), f.apply(knee), cm(16), tint(dark), 0, true);
        rod(b, f.apply(knee), f.apply(foot), cm(14), tint(dark), 0, true);
        b.inked(
            Mesh::Cube,
            &f.then(&boxed(foot.add(v3(0, cm(4), -cm(5))), v3(cm(15), cm(10), cm(28)))),
            hair,
            0,
        );
    }
    // the coat: a body and a skirt that flares with speed
    let speed = p.vel.len_xz().min(RUN_SPEED * 2);
    let flare = fx::mul(fx::div(speed, RUN_SPEED * 2), dm(3));
    part(
        b,
        &f,
        Mesh::Cube,
        v3(0, dm(13), 0),
        v3(dm(5), dm(6), dm(3)),
        tint(coat),
    );
    let skirt = f
        .then(&Affine::translate(v3(0, dm(8), flare / 2)))
        .then(&Affine::rot_x(-fx::mul(flare, deg(60))))
        .then(&Affine::scale(v3(cm(52), dm(5), cm(32))));
    b.inked(Mesh::Cube, &skirt, tint(coat), 0);
    part(
        b,
        &f,
        Mesh::Cube,
        v3(0, cm(105), 0),
        v3(cm(52), cm(8), cm(32)),
        trim,
    );
    // the scarf, trailing behind the way the body moves
    let back = p.vel.neg().scale(ONE * 12).clamp_len(dm(6));
    let neck = feet.add(v3(0, dm(16), 0));
    let tail = neck
        .add(back)
        .add(v3(0, -dm(3), 0))
        .add(facing(p.yaw, 0).scale(-dm(2)));
    rod(b, neck, tail, cm(12), trim, 0, true);
    // the head, turned up or down with the aim
    let head_at = v3(0, dm(17) + cm(5), 0);
    let hf = f
        .then(&Affine::translate(head_at))
        .then(&Affine::rot_x(p.pitch / 3));
    b.inked(
        Mesh::Sphere,
        &hf.then(&boxed(V3::ZERO, v3(cm(26), cm(28), cm(26)))),
        skin,
        0,
    );
    b.inked(
        Mesh::Sphere,
        &hf.then(&boxed(v3(0, cm(6), cm(4)), v3(cm(29), cm(22), cm(28)))),
        hair,
        0,
    );
    // arms and the gun, along the aim
    let aim = facing(p.yaw, p.pitch);
    let shoulder_r = feet.add(v3(0, dm(15), 0)).add(right_of(p.yaw).scale(dm(3)));
    let shoulder_l = feet.add(v3(0, dm(15), 0)).add(right_of(p.yaw).scale(-dm(3)));
    let hand = hand_of(p, alpha);
    rod(b, shoulder_r, hand, cm(11), tint(coat), 0, true);
    rod(
        b,
        shoulder_l,
        hand.add(aim.scale(dm(2))),
        cm(11),
        tint(coat),
        0,
        true,
    );
    gun_model(b, c, gun, hand, p.yaw, p.pitch, ONE);
    // a dash leaves ghosts
    if p.dashing() {
        for k in 1..4 {
            let g = feet.sub(p.dash_dir.scale(dm(7) * k)).add(v3(0, dm(9), 0));
            b.glow(
                Mesh::Sphere,
                &boxed(g, v3(dm(6), dm(16), dm(6))),
                c.pal.c("cyan"),
                ONE / (k + 1),
            );
        }
    }
    if p.pounding {
        // the drop: a column of light above, the gavel's fall
        for k in 1..4 {
            let g = feet.add(v3(0, dm(9) + dm(8) * k, 0));
            b.glow(
                Mesh::Sphere,
                &boxed(g, v3(dm(7), dm(14), dm(7))),
                c.pal.c("pound"),
                ONE / (k + 1),
            );
        }
    }
    if p.parry_window > 0 {
        let at = feet.add(v3(0, PLAYER_CHEST, 0));
        let r = PARRY_REACH * 2;
        b.glow(Mesh::Sphere, &boxed(at, v3(r, r, r)), c.pal.c("gold"), ONE / 3);
    }
}

/// A gun's model at `hand`, pointing along yaw and pitch. Also the icon
/// for a gun lying on the floor.
pub fn gun_model(b: &mut Batch, c: &Content, gun: usize, hand: V3, yaw: i32, pitch: i32, scale: i32) {
    let f = Affine::translate(hand)
        .then(&Affine::rot_y(yaw))
        .then(&Affine::rot_x(pitch))
        .then(&Affine::scale(v3(scale, scale, scale)));
    let body = c.pal.c("player_gun");
    let glow = c.pal.c(&c.guns[gun].colour);
    match c.guns[gun].id.as_str() {
        "scattergun" => {
            part(b, &f, Mesh::Cube, v3(0, 0, -dm(2)), v3(dm(2), dm(2), dm(6)), body);
            for s in [-1, 1] {
                let tube = f
                    .then(&Affine::translate(v3(s * cm(5), cm(4), -dm(5))))
                    .then(&Affine::rot_x(fx::QUARTER))
                    .then(&Affine::scale(v3(cm(8), dm(5), cm(8))));
                b.inked(Mesh::Cyl, &tube, shade(body, 120), 0);
            }
            b.push(
                Mesh::Cube,
                &f.then(&boxed(v3(0, cm(11), -dm(2)), v3(cm(6), cm(3), dm(3)))),
                glow,
                ONE,
                crate::render::PASS_SOLID,
            );
        }
        "launcher" => {
            let tube = f
                .then(&Affine::translate(v3(0, cm(4), -dm(3))))
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&Affine::scale(v3(dm(2), dm(8), dm(2))));
            b.inked(Mesh::Cyl, &tube, body, 0);
            b.push(
                Mesh::Sphere,
                &f.then(&boxed(v3(0, cm(4), -dm(7)), v3(cm(16), cm(16), cm(16)))),
                glow,
                ONE,
                crate::render::PASS_SOLID,
            );
            b.glow(
                Mesh::Sphere,
                &f.then(&boxed(v3(0, cm(4), -dm(7)), v3(dm(3), dm(3), dm(3)))),
                glow,
                ONE / 2,
            );
            part(b, &f, Mesh::Cube, v3(0, -cm(6), 0), v3(cm(6), dm(2), cm(8)), body);
        }
        "big_iron" => {
            // a long barrel, a heavy cylinder, a hammer, a worn grip
            let barrel = f
                .then(&Affine::translate(v3(0, cm(4), -dm(5))))
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&Affine::scale(v3(cm(6), dm(6), cm(6))));
            b.inked(Mesh::Cyl, &barrel, shade(body, 120), 0);
            let cyl = f
                .then(&Affine::translate(v3(0, cm(4), -cm(5))))
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&Affine::scale(v3(cm(14), cm(12), cm(14))));
            b.inked(Mesh::Cyl, &cyl, body, 0);
            part(
                b,
                &f,
                Mesh::Cube,
                v3(0, cm(9), cm(5)),
                v3(cm(3), cm(5), cm(4)),
                body,
            );
            let grip = f
                .then(&Affine::translate(v3(0, -cm(6), cm(8))))
                .then(&Affine::rot_x(-deg(20)))
                .then(&Affine::scale(v3(cm(6), cm(14), cm(7))));
            b.inked(Mesh::Cube, &grip, c.pal.c("wood"), 0);
            b.push(
                Mesh::Cube,
                &f.then(&boxed(v3(0, cm(8), -dm(5)), v3(cm(2), cm(2), dm(5)))),
                glow,
                ONE,
                crate::render::PASS_SOLID,
            );
        }
        "ripsaw" => {
            // a body, a guard over the front, a blade ready in it
            part(
                b,
                &f,
                Mesh::Cube,
                v3(0, 0, -dm(2)),
                v3(cm(10), cm(14), dm(5)),
                body,
            );
            part(
                b,
                &f,
                Mesh::Cube,
                v3(0, -cm(9), 0),
                v3(cm(6), cm(12), cm(6)),
                body,
            );
            let blade = f
                .then(&Affine::translate(v3(0, cm(6), -dm(5))))
                .then(&Affine::rot_z(fx::QUARTER))
                .then(&Affine::scale(v3(dm(3), cm(2), dm(3))));
            b.inked(Mesh::Cyl, &blade, glow, ONE / 3);
            let guard = f
                .then(&Affine::translate(v3(0, cm(10), -dm(5))))
                .then(&Affine::scale(v3(cm(4), cm(4), dm(3))));
            b.inked(Mesh::Cube, &guard, shade(body, 120), 0);
        }
        _ => {
            part(
                b,
                &f,
                Mesh::Cube,
                v3(0, 0, -dm(3)),
                v3(cm(9), cm(12), dm(8)),
                body,
            );
            let barrel = f
                .then(&Affine::translate(v3(0, cm(2), -dm(8))))
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&Affine::scale(v3(cm(5), dm(4), cm(5))));
            b.inked(Mesh::Cyl, &barrel, shade(body, 120), 0);
            part(
                b,
                &f,
                Mesh::Cube,
                v3(0, -cm(8), -dm(1)),
                v3(cm(6), cm(12), cm(6)),
                body,
            );
            b.push(
                Mesh::Cube,
                &f.then(&boxed(v3(0, cm(7), -dm(3)), v3(cm(3), cm(2), dm(5)))),
                glow,
                ONE,
                crate::render::PASS_SOLID,
            );
        }
    }
}

pub fn pickup_colour(c: &Content, k: PickupKind) -> Rgb {
    c.pal.c(match k {
        PickupKind::Heal(_) => "pickup_heal",
        PickupKind::MaxHealth(_) => "pickup_health",
        PickupKind::Speed(_) => "pickup_speed",
        PickupKind::Jump => "pickup_jump",
        PickupKind::Gun(_, _) => "pickup_weapon",
        PickupKind::Pierce => "pickup_pierce",
        PickupKind::Pulse => "pickup_pulse",
        PickupKind::Mend => "pickup_mend",
        PickupKind::Trigger => "pickup_trigger",
        PickupKind::Plating => "pickup_plating",
    })
}

/// The floating icon of a pickup (or of a gate's reward): a heart, arrows,
/// stacked chevrons, or the gun itself.
pub fn pickup_icon(b: &mut Batch, c: &Content, k: PickupKind, at: V3, spin: i32, scale: i32) {
    let col = pickup_colour(c, k);
    let f = Affine::translate(at)
        .then(&Affine::rot_y(spin))
        .then(&Affine::scale(v3(scale, scale, scale)));
    match k {
        PickupKind::Heal(_) => {
            b.inked(
                Mesh::Sphere,
                &f.then(&boxed(V3::ZERO, v3(dm(3), dm(3), dm(3)))),
                col,
                ONE / 2,
            );
        }
        PickupKind::MaxHealth(_) => {
            for s in [-1, 1] {
                b.inked(
                    Mesh::Sphere,
                    &f.then(&boxed(v3(s * cm(15), dm(1), 0), v3(dm(4), dm(4), dm(3)))),
                    col,
                    ONE / 2,
                );
            }
            let tip = f
                .then(&Affine::translate(v3(0, -dm(1), 0)))
                .then(&Affine::rot_z(fx::HALF / 4))
                .then(&Affine::scale(v3(cm(45), cm(45), cm(28))));
            b.inked(Mesh::Cube, &tip, col, ONE / 2);
        }
        PickupKind::Speed(_) => {
            for k in 0..2 {
                let a = f
                    .then(&Affine::translate(v3(0, 0, dm(2) - dm(3) * k)))
                    .then(&Affine::rot_x(-fx::QUARTER))
                    .then(&Affine::scale(v3(dm(5), dm(3), dm(1))));
                b.inked(Mesh::Wedge, &a, col, ONE / 2);
            }
        }
        PickupKind::Jump => {
            for k in 0..3 {
                let a = f
                    .then(&Affine::translate(v3(0, -dm(2) + dm(2) * k, 0)))
                    .then(&Affine::scale(v3(dm(5), dm(1), dm(3))));
                b.inked(Mesh::Wedge, &a, col, ONE / 2);
            }
        }
        PickupKind::Pierce => {
            // an arrowhead through a plate
            let tip = f
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&boxed(V3::ZERO, v3(dm(3), dm(6), dm(3))));
            b.inked(Mesh::Cone, &tip, col, ONE / 2);
            b.inked(
                Mesh::Cube,
                &f.then(&boxed(v3(0, 0, dm(1)), v3(dm(5), dm(5), cm(4)))),
                shade(col, 70),
                ONE / 3,
            );
        }
        PickupKind::Pulse => {
            // a ring round a core
            let ring = f
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&Affine::scale(v3(dm(6), cm(8), dm(6))));
            b.push(Mesh::Ring, &ring, col, ONE, crate::render::PASS_SOLID);
            b.inked(
                Mesh::Sphere,
                &f.then(&boxed(V3::ZERO, v3(dm(2), dm(2), dm(2)))),
                col,
                ONE / 2,
            );
        }
        PickupKind::Mend => {
            // a gold orb with a heart's two lobes
            b.inked(
                Mesh::Sphere,
                &f.then(&boxed(V3::ZERO, v3(dm(3), dm(3), dm(3)))),
                c.pal.c("gold"),
                ONE / 2,
            );
            for sx in [-1, 1] {
                b.inked(
                    Mesh::Sphere,
                    &f.then(&boxed(v3(sx * cm(8), dm(3), 0), v3(cm(15), cm(15), cm(12)))),
                    col,
                    ONE / 2,
                );
            }
        }
        PickupKind::Trigger => {
            // a trigger guard round a bolt
            let ring = f
                .then(&Affine::rot_z(fx::QUARTER))
                .then(&Affine::scale(v3(dm(4), cm(6), dm(4))));
            b.push(Mesh::Ring, &ring, col, ONE, crate::render::PASS_SOLID);
            for k in 0..3 {
                let bolt = f
                    .then(&Affine::translate(v3(cm(6) * (k - 1), cm(6) * (1 - k), 0)))
                    .then(&Affine::rot_z(deg(30)))
                    .then(&Affine::scale(v3(cm(7), dm(2), cm(6))));
                b.inked(Mesh::Cube, &bolt, col, ONE / 2);
            }
        }
        PickupKind::Plating => {
            // a shield of three plates
            for k in 0..3 {
                let plate = f
                    .then(&Affine::translate(v3(0, dm(2) - dm(2) * k, cm(3) * (k % 2))))
                    .then(&Affine::scale(v3(dm(5) - cm(8) * k, cm(16), cm(6))));
                b.inked(Mesh::Cube, &plate, shade(col, 100 - 10 * k), ONE / 2);
            }
        }
        PickupKind::Gun(g, level) => {
            gun_model(b, c, g, at, spin, 0, scale * 2);
            for k in 0..level {
                let p = at.add(v3(fx::mul(dm(3) * (k - 1), scale), fx::mul(dm(6), scale), 0));
                b.inked(Mesh::Octa, &boxed(p, v3(cm(15), cm(15), cm(15))), col, ONE);
            }
        }
    }
}

fn gate(b: &mut Batch, c: &Content, g: &crate::world::Gate, cleared: bool, clock: i32) {
    let across_x = g.yaw == 0 || g.yaw == fx::HALF;
    let (w, d) = if across_x {
        (int(5), dm(2))
    } else {
        (dm(2), int(5))
    };
    let out = facing(g.yaw, 0).neg().scale(dm(5));
    let at = g.pos.add(out).add(v3(0, int(3), 0));
    if g.to.is_none() {
        b.inked(Mesh::Cube, &boxed(at, v3(w, int(6), d)), c.pal.c("gate_shut"), 0);
        return;
    }
    let colour = if !cleared {
        c.pal.c("gate_shut")
    } else if g.main {
        c.pal.c("gate_main")
    } else {
        c.pal.c("gate_detour")
    };
    let glow = if cleared { ONE } else { 0 };
    b.push(
        Mesh::Cube,
        &boxed(at, v3(w, int(6), d)),
        colour,
        glow,
        crate::render::PASS_SOLID,
    );
    if cleared {
        let pulse = ONE / 3 + fx::sin(clock * 8) / 6;
        b.glow(
            Mesh::Cube,
            &boxed(at, v3(w + dm(6), int(6) + dm(6), d + dm(6))),
            colour,
            pulse,
        );
    }
    // the reward floats over the gate only once the gate is open
    if let Some(r) = g.reward.filter(|_| cleared) {
        let k = crate::world::reward_pickup(r);
        pickup_icon(b, c, k, at.add(v3(0, int(5), 0)), clock * 2, ONE * 2);
    }
}

fn orbs(b: &mut Batch, c: &Content, w: &World, alpha: i32, cam: &crate::render::Camera) {
    let gold_ring = c.pal.c("gold");
    let blue_rim = c.pal.c("ice_white");
    let chest = w
        .player
        .prev_pos
        .lerp(w.player.pos, alpha)
        .add(v3(0, PLAYER_CHEST, 0));
    for o in &w.orbs {
        let at = o.prev.lerp(o.pos, alpha);
        let r = o.radius * 2;
        // between the camera and the body, or right by the lens: a faint
        // glow you can see through, so it does not blind the view
        let by_lens = at.dist(cam.eye) < ORB_FADE_NEAR;
        let behind = at.sub(chest).dot(cam.fwd) < 0 && at.dist(cam.eye) < ORB_FADE_BEHIND;
        if by_lens || behind {
            b.glow(Mesh::Sphere, &boxed(at, v3(r, r, r)), o.rgb, ONE / 4);
            continue;
        }
        // a held orb hangs smaller and swells to full size as it is about
        // to fly, a ring round it while it waits
        let r = if o.wait > 0 {
            let k = fx::div(o.wait.min(HOLD_TELL) * ONE, HOLD_TELL * ONE);
            fx::mul(r, ONE - k * 4 / 10)
        } else {
            r
        };
        b.inked(Mesh::Sphere, &boxed(at, v3(r, r, r)), o.rgb, ONE);
        b.glow(Mesh::Sphere, &boxed(at, v3(r * 2, r * 2, r * 2)), o.rgb, ONE / 2);
        if o.wait > 0 {
            let ring = Affine::translate(at)
                .then(&Affine::rot_y(o.age * deg(9)))
                .then(&Affine::scale(v3(r * 2, cm(4), r * 2)));
            b.push(Mesh::Ring, &ring, o.rgb, ONE, crate::render::PASS_SOLID);
        }
        if o.ward > 0 && o.ward <= w.player.build.wards().max(1) {
            // a pale rim: the Injunction breaks these
            let rim = Affine::translate(at)
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&Affine::scale(v3(r * 13 / 10, cm(5), r * 13 / 10)));
            b.push(Mesh::Ring, &rim, blue_rim, ONE, crate::render::PASS_SOLID);
        }
        if o.gold {
            let ring = Affine::translate(at)
                .then(&Affine::rot_y(o.age * deg(6)))
                .then(&Affine::rot_x(fx::QUARTER))
                .then(&Affine::scale(v3(r * 16 / 10, cm(8), r * 16 / 10)));
            b.push(Mesh::Ring, &ring, gold_ring, ONE, crate::render::PASS_SOLID);
        }
    }
    for wv in &w.waves {
        let r = fx::lerp(wv.prev_radius, wv.radius, alpha) * 2;
        let at = wv.centre.add(v3(0, WAVE_HEIGHT / 2, 0));
        b.push(
            Mesh::Ring,
            &boxed(at, v3(r, WAVE_HEIGHT, r)),
            wv.rgb,
            ONE,
            crate::render::PASS_SOLID,
        );
        b.glow(Mesh::Ring, &boxed(at, v3(r, WAVE_HEIGHT * 2, r)), wv.rgb, ONE / 2);
        b.push(
            Mesh::Ring,
            &boxed(at.add(v3(0, WAVE_HEIGHT / 2, 0)), v3(r + dm(1), cm(6), r + dm(1))),
            c.pal.c("ink"),
            0,
            crate::render::PASS_SOLID,
        );
    }
}

/// The Injunction's wall: a pane of light in front of you, edged, fading
/// as its half second runs out.
fn injunction(b: &mut Batch, c: &Content, wall: &crate::world::WallPlane, left: i32) {
    let k = fx::div(left * ONE, WALL_TICKS.max(1) * ONE);
    let rgb = c.pal.c("wall_glow");
    let mid = wall.centre;
    let up = WALL_HEIGHT;
    let centre = v3(mid.x, wall.floor + up / 2, mid.z);
    let pane = Affine {
        c0: wall.across.scale(WALL_HALF_WIDTH * 2),
        c1: V3::UP.scale(up),
        c2: wall.normal.scale(cm(6)),
        t: centre,
    };
    b.glow(Mesh::Cube, &pane, rgb, ONE / 3 + k / 3);
    let half = wall.across.scale(WALL_HALF_WIDTH);
    let (lo, hi) = (wall.floor, wall.floor + up);
    let corners = [
        v3(mid.x, lo, mid.z).sub(half),
        v3(mid.x, lo, mid.z).add(half),
        v3(mid.x, hi, mid.z).add(half),
        v3(mid.x, hi, mid.z).sub(half),
    ];
    for i in 0..4 {
        rod(b, corners[i], corners[(i + 1) % 4], cm(6), rgb, ONE, false);
    }
}

/// An enemy, by kind. It grows in as it spawns, flashes white when hit,
/// and its core glows brighter through a windup.
pub fn enemy(b: &mut Batch, c: &Content, e: &Enemy, alpha: i32, target: V3) {
    let d = &c.enemies[e.def];
    let at = e.prev.lerp(e.pos, alpha);
    let grow = match e.state {
        State::Spawning(t) => ONE - fx::div(t * ONE, SPAWN_TICKS * ONE),
        _ => ONE,
    };
    let s = fx::mul(d.radius * 2, grow.max(ONE / 20));
    let flash = e.flash < 4;
    let paint = |x: Rgb| if flash { mix(x, [ONE; 3], ONE * 7 / 10) } else { x };
    let shell = paint(c.pal.c("enemy_shell"));
    let plate = paint(c.pal.c("enemy_plate"));
    let eye = c.pal.c("enemy_eye");
    let core = c.pal.c("enemy_core");
    let charge = e.glow(c);
    // zone two's enemies wind up in cobalt, zone one's in violet
    let core = if c.zone_of_enemy(e.def).unwrap_or(0) > 0 {
        c.pal.c("frost_core")
    } else {
        core
    };
    if matches!(d.kind, Kind::Agent | Kind::Tank | Kind::Mech | Kind::Double) {
        house::draw(b, c, e, d, at, alpha, target, &paint);
        return;
    }
    if fauna::draw(b, c, e, d, at, s, &paint) || beasts::draw(b, c, e, d, at, s, &paint) {
        if charge > 0 && d.kind != Kind::Boss && !e.buried {
            let h = fx::mul(s, ONE + charge);
            b.glow(Mesh::Sphere, &boxed(at, v3(h, h, h)), core, charge / 2);
        }
        return;
    }
    let f = Affine::translate(at)
        .then(&Affine::rot_y(e.yaw))
        .then(&Affine::scale(v3(s, s, s)));
    let u = |x: i32| ONE * x / 100;
    let spin = e.age * deg(3);
    match d.kind {
        Kind::Turret => {
            // a pedestal down to the floor, a shell, plates that turn, an eye
            let ped = at.sub(v3(0, d.hover / 2 + d.radius / 2, 0));
            b.inked(
                Mesh::Cyl,
                &boxed(ped, v3(s / 2, d.hover, s / 2)),
                shade(shell, 80),
                0,
            );
            b.inked(
                Mesh::Sphere,
                &f.then(&boxed(V3::ZERO, v3(u(80), u(80), u(80)))),
                shell,
                0,
            );
            for k in 0..4 {
                let pf = f
                    .then(&Affine::rot_y(spin + fx::QUARTER * k))
                    .then(&boxed(v3(0, 0, -u(42)), v3(u(40), u(60), u(12))));
                b.inked(Mesh::Cube, &pf, plate, 0);
            }
            b.inked(
                Mesh::Sphere,
                &f.then(&boxed(v3(0, 0, -u(38)), v3(u(26), u(26), u(14)))),
                eye,
                ONE,
            );
        }
        Kind::Floater | Kind::Blinker | Kind::Burrower => {
            let lantern = d.id == "lantern";
            if lantern {
                // a lantern: a cone cap, a cage, a burning core, a chain
                b.inked(
                    Mesh::Cone,
                    &f.then(&boxed(v3(0, u(55), 0), v3(u(90), u(40), u(90)))),
                    shell,
                    0,
                );
                for k in 0..6 {
                    let a = fx::TURN / 6 * k + spin / 2;
                    let p = v3(fx::mul(fx::cos(a), u(40)), 0, fx::mul(fx::sin(a), u(40)));
                    let bar = f.then(&boxed(p, v3(u(6), u(90), u(6))));
                    b.inked(Mesh::Cube, &bar, plate, 0);
                }
                b.inked(
                    Mesh::Cyl,
                    &f.then(&boxed(v3(0, -u(48), 0), v3(u(80), u(10), u(80)))),
                    shell,
                    0,
                );
                b.push(
                    Mesh::Sphere,
                    &f.then(&boxed(V3::ZERO, v3(u(50), u(60), u(50)))),
                    c.pal.c("gold"),
                    ONE,
                    crate::render::PASS_SOLID,
                );
                rod(
                    b,
                    at.add(v3(0, s * 3 / 4, 0)),
                    at.add(v3(0, s * 3, 0)),
                    cm(4),
                    shade(shell, 70),
                    0,
                    false,
                );
            } else {
                // a drifter: a spinning octahedron with fins and an eye
                let body = f
                    .then(&Affine::rot_y(spin))
                    .then(&boxed(V3::ZERO, v3(u(90), u(100), u(90))));
                b.inked(Mesh::Octa, &body, shell, 0);
                for side in [-1, 1] {
                    let fin = f
                        .then(&Affine::translate(v3(side * u(50), 0, 0)))
                        .then(&Affine::rot_z(side * deg(20)))
                        .then(&Affine::scale(v3(u(40), u(8), u(50))));
                    b.inked(Mesh::Wedge, &fin, plate, 0);
                }
                b.inked(
                    Mesh::Sphere,
                    &f.then(&boxed(v3(0, 0, -u(30)), v3(u(28), u(28), u(20)))),
                    eye,
                    ONE,
                );
            }
        }
        Kind::Walker => {
            // a warden: a squat body on four legs, shoulder plates, a visor
            let step = e.age * fx::TURN / 40;
            for (k, (lx, lz)) in [(-1, -1), (1, -1), (-1, 1), (1, 1)].iter().enumerate() {
                let lift = fx::sin(step + fx::HALF * (k as i32 % 2)).max(0) / 4;
                let hip = f.apply(v3(lx * u(35), -u(10), lz * u(30)));
                let foot = f.apply(v3(lx * u(55), -u(50) + lift, lz * u(50)));
                rod(b, hip, foot, s / 7, shade(shell, 85), 0, true);
            }
            b.inked(
                Mesh::Cube,
                &f.then(&boxed(v3(0, 0, 0), v3(u(80), u(55), u(80)))),
                shell,
                0,
            );
            for side in [-1, 1] {
                b.inked(
                    Mesh::Wedge,
                    &f.then(&boxed(v3(side * u(48), u(22), 0), v3(u(30), u(25), u(70)))),
                    plate,
                    0,
                );
            }
            b.inked(
                Mesh::Cube,
                &f.then(&boxed(v3(0, u(10), -u(40)), v3(u(50), u(12), u(6)))),
                eye,
                ONE,
            );
        }
        Kind::Moth => {
            let flap = fx::sin(e.age * fx::TURN / 8) / 2;
            b.inked(
                Mesh::Octa,
                &f.then(&boxed(V3::ZERO, v3(u(50), u(50), u(90)))),
                shell,
                0,
            );
            for side in [-1, 1] {
                let wing = f
                    .then(&Affine::translate(v3(side * u(30), 0, 0)))
                    .then(&Affine::rot_z(side * flap))
                    .then(&Affine::scale(v3(u(90), u(5), u(60))));
                b.inked(Mesh::Cube, &wing, plate, 0);
            }
            b.inked(
                Mesh::Sphere,
                &f.then(&boxed(v3(0, 0, -u(40)), v3(u(22), u(22), u(22)))),
                eye,
                ONE,
            );
        }
        Kind::Boss => bailiff(b, c, e, at, s, paint),
        Kind::Agent | Kind::Tank | Kind::Mech | Kind::Double | Kind::Beast => {}
    }
    // the windup: a halo that grows as the attack comes
    if charge > 0 && d.kind != Kind::Boss {
        let h = fx::mul(s, ONE + charge);
        b.glow(Mesh::Sphere, &boxed(at, v3(h, h, h)), core, charge / 2);
    }
}

/// The Bailiff of Hours: a robed figure whose face is a clock, two great
/// hands holding gavels, a crown, and a ring of hours turning round it.
fn bailiff(b: &mut Batch, c: &Content, e: &Enemy, at: V3, s: i32, paint: impl Fn(Rgb) -> Rgb) {
    let robe = paint(c.pal.c("boss_robe"));
    let trim = c.pal.c("boss_trim");
    let face = paint(c.pal.c("boss_face"));
    let ink = c.pal.c("ink");
    let f = Affine::translate(at).then(&Affine::rot_y(e.yaw));
    let u = |x: i32| fx::mul(s, ONE * x / 100);
    // the robe: a cone that hangs below the body
    b.inked(
        Mesh::Cone,
        &f.then(&boxed(v3(0, -u(20), 0), v3(u(85), u(110), u(85)))),
        robe,
        0,
    );
    b.inked(
        Mesh::Cube,
        &f.then(&boxed(v3(0, u(25), 0), v3(u(60), u(14), u(40)))),
        trim,
        0,
    );
    // the face, a clock, turned to you, its hands running backwards
    let head = f.then(&Affine::translate(v3(0, u(62), 0)));
    let disc = head
        .then(&Affine::rot_x(fx::QUARTER))
        .then(&Affine::scale(v3(u(60), u(10), u(60))));
    b.inked(Mesh::Cyl, &disc, face, ONE / 3);
    let t = e.age;
    for (rate, len, w) in [(-deg(2), u(26), u(5)), (-deg(24), u(20), u(7))] {
        let a = t * rate;
        let tip = v3(fx::mul(fx::sin(a), len), fx::mul(fx::cos(a), len), -u(7));
        let from = head.apply(v3(0, 0, -u(7)));
        rod(b, from, head.apply(tip), w, ink, 0, false);
    }
    let charge = e.glow(c);
    b.push(
        Mesh::Sphere,
        &head.then(&boxed(v3(0, 0, -u(8)), v3(u(8), u(8), u(4)))),
        c.pal.c("enemy_eye"),
        ONE,
        crate::render::PASS_SOLID,
    );
    // the crown
    b.inked(
        Mesh::Cyl,
        &head.then(&boxed(v3(0, u(36), 0), v3(u(40), u(12), u(40)))),
        trim,
        0,
    );
    for k in 0..5 {
        let a = fx::TURN / 5 * k;
        let p = v3(fx::mul(fx::cos(a), u(18)), u(46), fx::mul(fx::sin(a), u(18)));
        b.inked(
            Mesh::Octa,
            &head.then(&boxed(p, v3(u(8), u(14), u(8)))),
            trim,
            ONE / 3,
        );
    }
    // the hands with their gavels, rising and falling
    for side in [-1, 1] {
        let lift = fx::sin(t * deg(4) + side * fx::QUARTER) / 4;
        let hand = f.apply(v3(side * u(75), u(10) + fx::mul(lift, u(40)), -u(20)));
        b.inked(Mesh::Sphere, &boxed(hand, v3(u(22), u(22), u(22))), face, 0);
        let head_of_gavel = hand.add(v3(0, u(30), 0));
        rod(b, hand, head_of_gavel, u(5), shade(robe, 120), 0, true);
        let gav = Affine::translate(head_of_gavel)
            .then(&Affine::rot_y(e.yaw))
            .then(&Affine::rot_z(fx::QUARTER))
            .then(&Affine::scale(v3(u(14), u(30), u(14))));
        b.inked(Mesh::Cyl, &gav, trim, 0);
    }
    // the ring of hours
    let ring = Affine::translate(at.add(v3(0, u(20), 0)))
        .then(&Affine::rot_y(t * deg(1)))
        .then(&Affine::rot_x(deg(12)))
        .then(&Affine::scale(v3(u(230), u(4), u(230))));
    b.push(Mesh::Ring, &ring, trim, ONE, crate::render::PASS_SOLID);
    for k in 0..12 {
        let a = fx::TURN / 12 * k + t * deg(1);
        let p = at.add(v3(
            fx::mul(fx::cos(a), u(115)),
            u(20) + fx::mul(fx::sin(a * 2), u(10)),
            fx::mul(fx::sin(a), u(115)),
        ));
        b.inked(Mesh::Octa, &boxed(p, v3(u(10), u(16), u(10))), trim, ONE / 2);
    }
    if charge > 0 {
        let h = fx::mul(s * 2, ONE + charge / 2);
        b.glow(
            Mesh::Sphere,
            &boxed(at.add(v3(0, u(40), 0)), v3(h, h, h)),
            c.pal.c("violet"),
            charge / 3,
        );
    }
}

/// The Holding's figure: the Defendant standing at rest, for the menus.
pub fn standing(b: &mut Batch, c: &Content, at: V3, yaw: i32, gun: usize) {
    let mut p = Player::new(at, yaw, crate::player::Build::default());
    p.pitch = -deg(10);
    player(b, c, &p, gun, ONE);
}
