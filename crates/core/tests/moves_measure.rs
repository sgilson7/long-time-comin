//! Prints how far each move carries the body on flat ground, from a run at
//! top speed: the numbers the Water Gap's gaps are laid from.
//! `cargo test -p ltc_core --test moves_measure -- --ignored --nocapture`

use ltc_core::balance::*;
use ltc_core::fx::{int, ONE};
use ltc_core::geom::v3;
use ltc_core::level::{Level, Solid};
use ltc_core::player::{Build, Controls, Player};

fn floor() -> Level {
    // a runway ending at z = -40; past it nothing (a long way down)
    Level::new(vec![Solid::new(
        v3(-int(5), -ONE, -int(40)),
        v3(int(5), 0, int(40)),
    )])
}

fn fwd() -> Controls {
    Controls {
        move_z: ONE,
        ..Default::default()
    }
}

/// Runs to the edge, does `at_edge` on the tick the feet leave the edge
/// zone, then holds forward; how far past the edge it lands and how high it went.
fn leap(build: Build, at_edge: &[Controls]) -> (i32, i32) {
    let l = floor();
    let mut p = Player::new(v3(0, 0, int(30)), 0, build);
    while p.pos.z > -int(39) {
        p.step(&fwd(), &l);
    }
    for c in at_edge {
        p.step(c, &l);
    }
    let mut top = p.pos.y;
    for _ in 0..600 {
        p.step(&fwd(), &l);
        top = top.max(p.pos.y);
        if p.pos.y < -int(1) {
            break;
        }
    }
    // distance past the edge when the feet pass the edge's height going down
    let mut q = p.clone();
    let _ = &mut q;
    (-int(40) - p.pos.z, top)
}

fn m(v: i32) -> String {
    format!("{:.2}", v as f64 / ONE as f64)
}

#[test]
#[ignore]
fn print_the_moves() {
    let b = Build {
        jumps: 2,
        ..Build::default()
    };
    let jump = Controls {
        move_z: ONE,
        jump_pressed: true,
        ..Default::default()
    };
    let dash = Controls {
        move_z: ONE,
        dash_pressed: true,
        ..Default::default()
    };
    let both = Controls {
        move_z: ONE,
        jump_pressed: true,
        dash_pressed: true,
        ..Default::default()
    };
    for (name, seq) in [
        ("jump", vec![jump]),
        ("dash then jump next tick (Bound Over)", vec![dash, jump]),
        ("dash and jump same tick", vec![both]),
        ("dash, 4 ticks, jump", vec![dash, fwd(), fwd(), fwd(), jump]),
        ("jump then dash next tick", vec![jump, dash]),
    ] {
        let (d, top) = leap(b, &seq);
        println!(
            "{name}: lands {} m past the edge (crossing y=-1), top {} m",
            m(d),
            m(top)
        );
    }
    // double jump at the peak
    let l = floor();
    let mut p = Player::new(v3(0, 0, int(30)), 0, b);
    while p.pos.z > -int(39) {
        p.step(&fwd(), &l);
    }
    p.step(&jump, &l);
    let mut t = 0;
    while p.vel.y > 0 {
        p.step(&fwd(), &l);
        t += 1;
    }
    p.step(&jump, &l);
    let mut top = p.pos.y;
    for _ in 0..600 {
        p.step(&fwd(), &l);
        top = top.max(p.pos.y);
        if p.pos.y < -int(1) {
            break;
        }
    }
    println!(
        "jump + air jump at peak ({t} ticks up): {} m past, top {}",
        m(-int(40) - p.pos.z),
        m(top)
    );
    // Bound Over then air jump at peak
    let mut p = Player::new(v3(0, 0, int(30)), 0, b);
    while p.pos.z > -int(39) {
        p.step(&fwd(), &l);
    }
    p.step(&dash, &l);
    p.step(&jump, &l);
    while p.vel.y > 0 {
        p.step(&fwd(), &l);
    }
    p.step(&jump, &l);
    let mut top = p.pos.y;
    for _ in 0..600 {
        p.step(&fwd(), &l);
        top = top.max(p.pos.y);
        if p.pos.y < -int(1) {
            break;
        }
    }
    println!(
        "Bound Over + air jump at peak: {} m past, top {}",
        m(-int(40) - p.pos.z),
        m(top)
    );
    println!(
        "run speed {} m/tick, dash {} m/tick x {}, grapple range {}",
        m(RUN_SPEED),
        m(DASH_SPEED),
        DASH_TICKS,
        m(GRAPPLE_RANGE)
    );
}
