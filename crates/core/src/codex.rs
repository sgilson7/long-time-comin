//! The Codex: every mechanical line the game shows, written from the same
//! numbers the simulation runs on, so a description cannot drift from what
//! a gun, an orb or an upgrade does. Story text lives in `data/story.json`;
//! nothing here has a voice.
//!
//! Units shown: metres, seconds, metres a second, degrees, health points.

use crate::balance::*;
use crate::content::{
    Anchor, Content, EnemyDef, Form, Gait, GunLevel, Kind, Launch, Lunge, Role, Shape, Upgrade, Volley,
};
use crate::enemy::PHASE_TICKS;
use crate::fx::{self, ONE};
use serde_json::{json, Value};

/// A Q16 value as a decimal with `places` digits, trailing zeros dropped.
pub fn dec(q: i32, places: u32) -> String {
    let scale = 10i64.pow(places);
    let v = (q as i64 * scale * 2 + if q >= 0 { ONE as i64 } else { -(ONE as i64) }) / (ONE as i64 * 2);
    let (sign, v) = if v < 0 { ("-", -v) } else { ("", v) };
    let (whole, frac) = (v / scale, v % scale);
    if places == 0 || frac == 0 {
        return format!("{sign}{whole}");
    }
    let f = format!("{frac:0width$}", width = places as usize);
    format!("{sign}{whole}.{}", f.trim_end_matches('0'))
}

/// Ticks as seconds.
pub fn s(ticks: i32) -> String {
    dec(fx::div(ticks * ONE, TICKS_PER_SECOND * ONE), 2)
}

/// Q16 metres per tick as metres a second.
pub fn mps(v: i32) -> String {
    dec(v * TICKS_PER_SECOND, 1)
}

/// Q16 metres per tick² as metres a second².
pub fn mps2(a: i32) -> String {
    dec(a * TICKS_PER_SECOND * TICKS_PER_SECOND, 0)
}

/// Q16 metres.
pub fn m(v: i32) -> String {
    dec(v, 2)
}

/// Angle units as degrees.
pub fn deg(a: i32) -> String {
    dec((a as i64 * 360 * ONE as i64 / fx::TURN as i64) as i32, 1)
}

/// Angle units a tick as degrees a second.
pub fn degs(a: i32) -> String {
    deg(a * TICKS_PER_SECOND)
}

/// A standing jump's height: v² / 2g.
pub fn jump_height() -> i32 {
    fx::div(fx::mul(JUMP_SPEED, JUMP_SPEED) / 2, GRAVITY)
}

/// Fills `{key}` placeholders.
pub fn fill(template: &str, pairs: &[(&str, String)]) -> String {
    pairs.iter().fold(template.to_string(), |t, (k, v)| {
        t.replace(&format!("{{{k}}}"), v)
    })
}

/// The numbers the control and rule texts refer to.
fn numbers() -> Vec<(&'static str, String)> {
    vec![
        ("run", mps(RUN_SPEED)),
        ("jump_height", m(jump_height())),
        ("max_jumps", MAX_JUMPS.to_string()),
        ("dash_distance", m(DASH_SPEED * DASH_TICKS)),
        ("dash_time", s(DASH_TICKS)),
        ("dash_iframes", s(DASH_IFRAMES)),
        ("dash_recharge", s(DASH_RECHARGE)),
        ("dash_charges", DASH_CHARGES.to_string()),
        ("grapple_range", m(GRAPPLE_RANGE)),
        ("parry_window", s(PARRY_WINDOW)),
        ("wall_time", s(WALL_TICKS)),
        ("wall_width", m(WALL_HALF_WIDTH * 2)),
        ("pickup_reach", m(PICKUP_REACH)),
        ("pound_speed", mps(POUND_SPEED)),
        ("pound_damage", POUND_DAMAGE.to_string()),
        ("pound_radius", m(POUND_RADIUS)),
        ("throw_damage", THROW_DAMAGE.to_string()),
        ("stop", s(STOP_BASE)),
        ("bench_size", m(BENCH_SIZE)),
        ("bench_time", s(BENCH_TICKS)),
    ]
}

pub fn controls(c: &Content) -> Vec<Value> {
    let n = numbers();
    c.copy_rows("controls")
        .into_iter()
        .map(|(keys, does)| json!([keys, fill(&does, &n)]))
        .collect()
}

/// One line for the HUD: what the gun in hand does.
pub fn gun_summary(l: &GunLevel) -> String {
    let mut parts = vec![if l.pellets > 1 {
        format!("{} pellets × {} damage", l.pellets, l.damage)
    } else {
        format!("{} damage", l.damage)
    }];
    // a spin gun's rate, at full spin
    let per_s = match l.spin {
        Some(sp) => per_second(sp.hot),
        None => fx::div(TICKS_PER_SECOND * ONE, l.rate * ONE),
    };
    parts.push(format!("{}/s", dec(per_s, 1)));
    if l.splash_radius > 0 {
        parts.push(format!("{} burst {} m", l.splash, m(l.splash_radius)));
    }
    if l.pierce > 0 {
        parts.push(format!("pierces {}", l.pierce));
    }
    parts.join(" · ")
}

/// Shots a second, in fixed point, for thousandths of a tick between them.
fn per_second(milliticks: i32) -> i32 {
    (TICKS_PER_SECOND as i64 * 1000 * ONE as i64 / milliticks.max(1) as i64) as i32
}

fn gun_level(l: &GunLevel, level: usize, erases: bool) -> Value {
    // a spin gun's rate is the one it reaches at full spin
    let per_s = match l.spin {
        Some(sp) => per_second(sp.hot),
        None => fx::div(TICKS_PER_SECOND * ONE, l.rate * ONE),
    };
    // one target: every pellet on it; a burst's damage goes to the others
    let per_shot = l.damage * l.pellets;
    let mut notes = Vec::new();
    if l.pierce > 0 {
        notes.push(format!(
            "Each shot passes through {} enemy before it stops.",
            l.pierce
        ));
    }
    if l.splash_radius > 0 {
        notes.push(format!(
            "Bursts on hitting an enemy or a surface, or at the end of its range: {} damage to every other enemy within {} m.",
            l.splash,
            m(l.splash_radius)
        ));
    }
    if erases {
        notes.push("The burst also removes every enemy orb inside it.".to_string());
    }
    if let Some(sp) = l.spin {
        notes.push(format!(
            "Spins up: {} shots/s when it starts, {} once the trigger has been held {} s (the rate and damage per second shown); its spread widens to ±{}°. Let go and the spin bleeds away over {} s.",
            dec(per_second(sp.cold), 1),
            dec(per_s, 1),
            s(sp.up),
            deg(sp.hot_spread),
            s(sp.bleed)
        ));
    }
    if l.erase_ward > 0 {
        let which = if l.erase_ward > crate::world::WARDS {
            "every enemy orb but gold".to_string()
        } else if l.erase_ward == 1 {
            "a blue orb (the Injunction's first ward)".to_string()
        } else {
            format!("an orb of the Injunction's first {} wards", l.erase_ward)
        };
        notes.push(format!(
            "Each round erases {which} it meets{}.",
            match l.through {
                0 => ", and is spent".to_string(),
                1 => " and goes on through one more".to_string(),
                n => format!(" and goes on through {n} more"),
            }
        ));
    }
    if l.pellets > 1 {
        notes.push("The first pellet flies along the aim; the rest scatter inside the spread. Damage per second assumes every pellet hits.".to_string());
    }
    json!({
        "level": level,
        "damage": if l.pellets > 1 { format!("{} pellets × {}", l.pellets, l.damage) } else { l.damage.to_string() },
        "rate": match l.spin {
            Some(sp) => format!("{} shots/s at full spin ({} s apart)", dec(per_s, 1), dec((sp.hot as i64 * ONE as i64 / (TICKS_PER_SECOND as i64 * 1000)) as i32, 3)),
            None => format!("{} shots/s ({} s apart)", dec(per_s, 1), s(l.rate)),
        },
        "dps": dec(fx::mul(per_s, per_shot * ONE), 0),
        "speed": format!("{} m/s", mps(l.speed)),
        "range": format!("{} m", dec(l.speed * l.life, 0)),
        "spread": if l.spread > 0 { format!("±{}°", deg(l.spread)) } else { "none".to_string() },
        "notes": notes,
    })
}

/// A volley in words, from its numbers.
pub fn volley(v: &Volley) -> String {
    let size = format!("{} m orbs", m(v.size * 2));
    let speed = if v.accel > 0 {
        format!(
            "starting at {} m/s and speeding up by {} m/s² to {} m/s",
            mps(v.speed),
            mps2(v.accel),
            mps(v.max_speed)
        )
    } else if v.accel < 0 {
        format!(
            "starting at {} m/s and slowing by {} m/s² to {} m/s",
            mps(v.speed),
            mps2(-v.accel),
            mps(v.min_speed)
        )
    } else {
        format!("at {} m/s", mps(v.speed))
    };
    let mut text = match v.shape {
        Shape::Ring => {
            let facing = if v.aim { ", one orb aimed at you" } else { "" };
            format!(
                "Ring of {} {size} spread evenly round the enemy{facing}, the ring tilting up to {}° toward you, {speed}",
                v.count,
                deg(crate::orbs::RING_TILT)
            )
        }
        Shape::Aimed if v.count == 1 => format!("One {} m orb at you, {speed}", m(v.size * 2)),
        Shape::Aimed => format!("Fan of {} {size} across {}°, centred on you, {speed}", v.count, deg(v.spread)),
        Shape::Sphere => format!("Burst of {} rings of up to {} {size} from 50° below level to 50° above, {speed}", v.rings, v.count),
        Shape::Wave => format!(
            "Ground wave: a ring along the floor from under the enemy, spreading at {} m/s to {} m. It is {} m high; above it you are not hit",
            mps(v.speed),
            m(v.size),
            m(WAVE_HEIGHT)
        ),
        Shape::Form => formation(v, &size, &speed),
    };
    if v.curl != 0 {
        let way = if v.alternate {
            ", alternate orbs curving opposite ways"
        } else {
            ""
        };
        text += &format!("; each orb curves {}°/s{way}", degs(v.curl.abs()));
    }
    if v.repeat > 1 {
        text += &format!("; fired {} times, {} s apart", v.repeat, s(v.every));
        if v.spin != 0 {
            text += &format!(", each turned {}° from the last", deg(v.spin.abs()));
        }
    }
    if v.delay > 0 {
        text += &format!("; starts {} s into the attack", s(v.delay));
    }
    if let Some(r) = v.rehold {
        text += &format!(
            "; after {} s of flight each stops, waits {} s, then flies {} at {} m/s",
            s(r.stop),
            s(r.wait),
            launch_words(r.launch),
            mps(r.speed)
        );
    }
    if v.gold {
        text += "; gold: Objection can turn these back";
    }
    match v.ward {
        0 => {}
        1 => text += "; blue: the Injunction breaks these",
        k => text += &format!("; the Injunction breaks these from its ward {k} (a pale rim once yours does)"),
    }
    let damage = v.damage;
    text + &format!(". {damage} damage a hit.")
}

fn launch_words(l: Launch) -> &'static str {
    match l {
        Launch::Keep => "on the way it was going",
        Launch::Out => "outward from the shape's centre",
        Launch::In => "inward to the shape's centre",
        Launch::Gather => "inward to the shape's centre, where it is spent",
        Launch::Aim => "at where you are at that moment",
        Launch::Lead => "at where you will be if you keep moving as you are",
        Launch::Down => "straight down",
        Launch::Along => "along the line from the enemy to where you were",
        Launch::Spin => "round the shape's centre",
        Launch::Close => "straight across toward the lane's middle",
    }
}

/// A held formation in words.
fn formation(v: &Volley, size: &str, speed: &str) -> String {
    let r = m(v.radius);
    let shape = match v.form {
        Form::Ring => format!("a ring of {} {size} {r} m in radius", v.count),
        Form::Grid => format!("a grid of {} by {} {size}, {r} m apart", v.count, v.rows),
        Form::Curtain => format!(
            "a wall of {} by {} {size}, {r} m apart, with a hole {} across that moves each time",
            v.count, v.rows, v.gap
        ),
        Form::Line => format!("a line of {} {size} {} m long", v.count, m(v.radius * 2)),
        Form::Column => format!("a column of {} {size} {r} m tall", v.count),
        Form::Star => format!("a star of {} arms of {} {size}, {r} m long", v.count, v.rows),
        Form::Spiral => format!("a spiral of {} {size}, {} turns out to {r} m", v.count, v.rows),
        Form::Arc => format!("an arc of {} {size} across {}° at {r} m", v.count, deg(v.spread)),
        Form::Dome => format!(
            "a half sphere of {} rings of {size} round you, {r} m out, the lowest of {}",
            v.rows, v.count
        ),
        Form::Ribbon => format!("a wavy line of {} {size} {} m long", v.count, m(v.radius * 2)),
        Form::Lane => format!(
            "two walls of {} {size}, {} m long, {r} m either side of the line to you",
            v.count,
            m(v.length)
        ),
    };
    let plane = if v.flat {
        "lying flat"
    } else {
        "standing, facing you"
    };
    let place = match v.anchor {
        Anchor::Body => "round the enemy".to_string(),
        Anchor::Front => format!("{} m in front of the enemy, toward you", m(v.at)),
        Anchor::Target => "round you".to_string(),
        Anchor::Above => format!("{} m over you", m(v.at)),
    };
    let release = if v.stagger > 0 {
        format!(
            "Held {} s, then they fly one after another, {} s apart,",
            s(v.hold.max(1)),
            s(v.stagger)
        )
    } else {
        format!("Held {} s, then all fly at once", s(v.hold.max(1)))
    };
    format!(
        "Formation: {shape}, {plane}, {place}. {release} {} {speed}",
        launch_words(v.launch)
    )
}

fn movement(d: &EnemyDef, c: &Content) -> String {
    match d.kind {
        Kind::Beast => match d.gait {
            Gait::Bound => format!(
                "Stands, then every {} s leaps in a high arc ({} s in the air) to a place {} m from you, a little further round you.",
                s(BOUND_EVERY),
                s(BOUND_TICKS),
                m(d.range)
            ),
            Gait::Hop => format!(
                "Hops {} m every {} s, toward you until within {} m and away when closer.",
                m(HOP_LENGTH),
                s(HOP_EVERY),
                m(d.range)
            ),
            Gait::Wheel => format!(
                "Circles {} m over the ground at about {} m from you, at up to {} m/s; as it winds up it dives to {} m over your chest and closes to {}% of that distance.",
                m(d.hover),
                m(d.range),
                mps(d.speed),
                m(WHEEL_DIVE_OVER),
                WHEEL_DIVE_PCT
            ),
            Gait::Root if d.bite > 0 => format!(
                "Rooted where it grows. Come within {} m and it lunges {} m at you and bites ({} damage), then again every {} s while you stay.",
                m(d.bite + d.radius + BITE_MARGIN),
                m(d.bite),
                d.contact,
                s(BITE_EVERY)
            ),
            Gait::Root => "Rooted where it grows; it does not move.".to_string(),
            Gait::Surge => format!(
                "Rushes along the ground at up to {} m/s, sweeping round a circle {} m from you once every {} s.",
                mps(d.speed),
                m(d.range),
                s(SURGE_PERIOD)
            ),
            Gait::Perch => format!(
                "Sits on {PERCHES} dead snags {} m high round the arena, {} s on each, firing down; then glides to another at {} m/s. One move in {SWOOP_EVERY} it spreads its wings for {} s and swoops instead, at {} m/s, through where you stood and {} m past, striking with its talons ({} damage), and climbs to its next perch. It holds its fire while it flies.",
                m(PERCH_HEIGHT),
                s(PERCH_HOLD),
                mps(PERCH_GLIDE),
                s(SWOOP_TELL),
                mps(SWOOP_SPEED),
                m(SWOOP_PAST),
                d.contact
            ),
            Gait::March => format!(
                "Hangs {} m over the floor and marches: a step of {} m sideways every {} s; every {}th step is toward you (or away, nearer than {} m), and then it marches back the other way.",
                m(d.hover),
                m(MARCH_STEP),
                s(MARCH_EVERY),
                MARCH_ACROSS,
                m(d.range)
            ),
            Gait::Unseen => format!(
                "While you look at it (within {}° of the middle of your view, nothing between) it does not move or fire, and can be shot. While you look away it comes straight at you at {} m/s, and fires.",
                deg(UNSEEN_LOOK),
                mps(d.speed)
            ),
            Gait::Charge => format!(
                "Turns on the spot at {}°/s until it faces you, then charges straight ahead at {} m/s for {} s, whatever you do; it stops at a wall or an edge and stands {} s.",
                deg(CHARGE_TURN * TICKS_PER_SECOND),
                mps(d.speed),
                s(CHARGE_TICKS),
                s(CHARGE_REST)
            ),
            Gait::Bounce => format!(
                "Bounces {} m high on a beat of {} s that every Toon shares, drifting to about {} m from you, and fires as it lands.",
                m(BOUNCE_HEIGHT),
                s(BOUNCE_TICKS),
                m(d.range)
            ),
            Gait::Echo => format!(
                "Walks the path you walked {} s ago, at up to {} m/s: stand still and it reaches you.",
                s(ECHO_DELAY),
                mps(d.speed)
            ),
            Gait::None => String::new(),
        },
        Kind::Oak => "Rooted on the shore and never hurt. Its first bar is your climb up its trunk; its eyes open just above you as you go. At its crown it calls the Summons out of the lake, and its bar follows the Summons' health.".to_string(),
        Kind::God => {
            let bars: Vec<String> = d
                .phases
                .iter()
                .enumerate()
                .map(|(k, p)| format!("bar {} from {}% health", k + 1, p.from_pct))
                .collect();
            let verses = d.lullaby.as_ref().map_or(0, |l| l.verses.len());
            format!(
                "Vast, over the north of the Heart, heaving. Its health is {} bars ({}); at each it calls help, clears every orb and cannot be hurt for {} s. When the last bar empties it does not die: you take out the trumpet, and play it {} verses to sleep while it fires on the beat.",
                d.phases.len(),
                bars.join("; "),
                s(PHASE_TICKS),
                verses
            )
        }
        Kind::Cat => {
            let gaits: Vec<String> = d
                .phases
                .iter()
                .enumerate()
                .map(|(k, p)| {
                    let how = match p.gait {
                        Gait::Charge => "turns on the spot, then charges straight",
                        Gait::Bound => "leaps round you, and pounces on where you stand",
                        Gait::Perch => "sits up on the furniture, leaps between it and swoops at you",
                        Gait::Unseen => "creeps at you only while you are not looking",
                        Gait::Surge => "rushes past you in sweeping curves",
                        Gait::Hop => "hops about you",
                        _ => "moves",
                    };
                    format!("bar {} (from {}% health) {how}", k + 1, p.from_pct)
                })
                .collect();
            format!(
                "Screams for {} s first (to go outside, or for treats) and grows to three times his size, and cannot be hurt meanwhile. His health is {} bars: {}. At each bar he calls help, clears every orb and cannot be hurt for {} s. When the last bar empties he is fed, and that is the end.",
                s(CAT_SCREAM),
                d.phases.len(),
                gaits.join("; "),
                s(PHASE_TICKS)
            )
        }
        Kind::Turret => format!(
            "Does not move. Sits {} m above the floor and turns to face you.",
            m(d.hover)
        ),
        Kind::Floater => format!(
            "Flies about {} m above the floor and circles you at about {} m, at up to {} m/s.",
            m(d.hover),
            m(d.range),
            mps(d.speed)
        ),
        Kind::Walker => format!(
            "Walks toward you at up to {} m/s until within {} m, then circles. Climbs steps up to 0.6 m.",
            mps(d.speed),
            m(d.range)
        ),
        Kind::Moth => format!(
            "Flies straight at you at up to {} m/s, weaving. It bursts on touching you.",
            mps(d.speed)
        ),
        Kind::Blinker => format!(
            "Hovers {} m above the floor. As each rest ends it vanishes and reappears about {} m from you, where it can see you, then winds up.",
            m(d.hover),
            m(d.range)
        ),
        Kind::Burrower => format!(
            "Between attacks it is under the floor and cannot be hurt; a ring of cracks shows where, and turns ember just before it comes up. Under the floor it moves at up to {} m/s to about {} m from you. It surfaces, winds up and fires, and stays up for {} s afterwards.",
            mps(d.speed),
            m(d.range),
            s(crate::enemy::BURROW_LINGER)
        ),
        Kind::Agent => {
            let walk = format!("Walks and runs the floors and stairs at up to {} m/s.", mps(d.speed));
            let part = match d.role {
                Role::Rifle => "Takes cover out of your sight, beside something tall, and steps out to fire. It waits for a partner at least 90° round you from it and fires with it on the same tick.",
                Role::Flank => "Goes round to your back, past the edge of what you can see from where you face, and fires with a partner in front of you when it gets there.",
                Role::Sniper => "Takes the highest place it can walk to that sees you from far off. A red line shows where it aims; the aim stops following you just before it fires.",
                Role::Shield => "Carries a shield and walks at you. Shots that meet it from in front glance off; shoot it from the side, from behind or from well above.",
                Role::Grenadier => "Takes cover and lobs over it: it needs no line to you.",
                Role::Rush => "Runs straight at you and hurts on touch.",
                Role::Command => "Keeps behind the others. While it lives every agent rests less, and three or more agents ready together fire at once from round you.",
                Role::Chief => "Keeps back behind its men and calls more of them.",
                Role::None => "",
            };
            let mut words = format!("{walk} {part}");
            if !d.summon.is_empty() {
                let calls: Vec<String> = d.summon.iter().map(|&i| c.enemies[i].name.clone()).collect();
                words += &format!(
                    " Every {} s it calls {}, while fewer than {} it called are up.",
                    s(d.summon_every),
                    calls.join(", "),
                    d.summon_cap
                );
            }
            if !d.rally.is_empty() {
                words += &format!(" At {}% health it calls {} more at once.", d.rally_pct, d.rally.len());
            }
            if !d.summon.is_empty() {
                words += " When it falls, those it called withdraw; they are worth no Clemency.";
            }
            if d.armour > 0 {
                words += &format!(" Its shield covers {}° in front of it.", deg(d.armour * 2));
            }
            words
        }
        Kind::Mech => format!(
            "A frame after Iron Vector's. It glides round you at about {} m, at up to {} m/s, quick-boosts sideways every few seconds, and every 9 s ascends 7 m and hovers before it comes down.",
            m(d.range),
            mps(d.speed)
        ),
        Kind::Double => {
            let stops: Vec<String> = d
                .phases
                .iter()
                .flat_map(|p| p.attacks.iter())
                .filter(|a| a.stop > 0)
                .map(|a| s(a.stop))
                .collect();
            let rare = d
                .phases
                .iter()
                .flat_map(|p| p.attacks.iter())
                .filter(|a| a.stop > 0)
                .map(|a| a.once_in)
                .max()
                .unwrap_or(1);
            let lunges: Vec<&str> = d
                .phases
                .iter()
                .flat_map(|p| p.attacks.iter())
                .map(|a| match a.moves {
                    Lunge::Hook => "throws a hook to beside you and pulls itself there",
                    Lunge::Pound => "leaps over you and drops in a ground pound",
                    Lunge::None => "",
                })
                .filter(|w| !w.is_empty())
                .fold(Vec::new(), |mut v, w| {
                    if !v.contains(&w) {
                        v.push(w);
                    }
                    v
                });
            let calls: Vec<String> = d
                .phases
                .iter()
                .enumerate()
                .skip(1)
                .map(|(k, p)| format!("phase {} from {}% health", k + 1, p.from_pct))
                .collect();
            format!(
                "Moves as you do: it runs round you at about {} m, dashes, and jumps; in its attacks it {}. Some of its attacks stop time ({} s), taken one time in {} they come up: everything but it holds still, you included, while it steps round you and lays its shots; they fly when time resumes. It has {} phases ({}); at each change it calls help, clears every orb and cannot be hurt for {} s.",
                m(d.range),
                lunges.join(" and "),
                stops.join(", "),
                rare,
                d.phases.len(),
                calls.join("; "),
                s(PHASE_TICKS)
            )
        }
        Kind::Tank => {
            let phases: Vec<String> = d
                .phases
                .iter()
                .enumerate()
                .map(|(k, p)| {
                    if p.survive > 0 {
                        format!(
                            "phase {}: it grows {} more cannons, rises and cannot be hurt; outlast it for {} s",
                            k + 1,
                            p.cannons.len(),
                            s(p.survive)
                        )
                    } else {
                        let hp: Vec<String> = p.weak.iter().map(|w| w.hp.to_string()).collect();
                        format!(
                            "phase {}: {} cannons grown, {} weak points ({} health)",
                            k + 1,
                            p.cannons.len(),
                            p.weak.len(),
                            hp.join(", ")
                        )
                    }
                })
                .collect();
            format!(
                "A tracked tank. It turns on the spot, drives at up to {} m/s, backs off when you are close, and rams at {} m/s when you stand in front of it in the open. Touching it hurts and throws you clear. Its turret turns at {}°/s. Shots glance off its hull and turret: only its glowing weak points take damage, and breaking every weak point of a phase starts the next. {}. At each change it clears every orb and cannot be hurt for {} s.",
                mps(d.speed),
                mps(TANK_RAM_SPEED),
                degs(TANK_TURRET_TURN),
                phases.join("; "),
                s(PHASE_TICKS)
            )
        }
        Kind::Boss => {
            let changes: Vec<String> = d
                .phases
                .iter()
                .skip(1)
                .map(|p| {
                    let calls: Vec<String> = p.summons.iter().map(|&i| c.enemies[i].name.clone()).collect();
                    let calls = if calls.is_empty() {
                        "nothing".to_string()
                    } else {
                        calls.join(", ")
                    };
                    format!("at {}% health it calls {calls}", p.from_pct)
                })
                .collect();
            format!(
                "Floats about {} m up and circles 9 m round the court's centre. It has {} phases: {}. At each change it clears every orb in the court and cannot be hurt for {} s.",
                m(d.hover),
                d.phases.len(),
                changes.join("; "),
                s(PHASE_TICKS)
            )
        }
    }
}

fn enemy(d: &EnemyDef, c: &Content) -> Value {
    let phases: Vec<Value> = d
        .phases
        .iter()
        .enumerate()
        .map(|(k, p)| {
            let attacks: Vec<Value> = p
                .attacks
                .iter()
                .map(|a| {
                    let pat = &c.patterns[a.pattern];
                    json!({
                        "windup": if pat.length() == 0 {
                            format!("{} s wind-up (it glows), fires once, then {} s rest", s(a.windup), s(a.rest))
                        } else {
                            format!("{} s wind-up (it glows), then {} s of fire, then {} s rest", s(a.windup), s(pat.length()), s(a.rest))
                        },
                        "volleys": pat.volleys.iter().map(volley).collect::<Vec<_>>(),
                    })
                })
                .collect();
            json!({
                "from": if matches!(d.kind, Kind::Boss | Kind::God | Kind::Cat) { format!("Phase {} (from {}% health)", k + 1, p.from_pct) } else { String::new() },
                "attacks": attacks,
            })
        })
        .collect();
    let zone = c
        .zone_of_enemy(c.enemy(&d.id).unwrap_or(0))
        .map(|z| c.zones[z].name.clone())
        .unwrap_or_default();
    let mut moves = movement(d, c);
    if let Some(p) = d.burst {
        let words: Vec<String> = c.patterns[p].volleys.iter().map(volley).collect();
        moves += &format!(" When it dies it leaves this behind: {}", words.join(" "));
    }
    let remixes: Vec<String> = d
        .phases
        .iter()
        .flat_map(|p| p.attacks.iter())
        .filter_map(|a| c.patterns[a.pattern].remix.map(|k| (a.pattern, k)))
        .map(|(p, k)| format!("{} as {}", c.patterns[p].id, c.patterns[k].id))
        .collect();
    if !remixes.is_empty() {
        moves += &format!(" On the last mission it fires remixes: {}.", remixes.join("; "));
    }
    if let Some(&first) = d.splits.first() {
        moves += &format!(
            " When it dies it splits into {} × {}.",
            d.splits.len(),
            c.enemies[first].name
        );
    }
    if let Some(l) = d.lullaby.as_ref() {
        let notes: usize = l.verses.iter().map(|v| v.notes.len()).sum();
        let beats: Vec<String> = l
            .beats
            .iter()
            .map(|b| {
                let from = if b.from_verse > 0 {
                    format!(", from verse {}", b.from_verse + 1)
                } else {
                    String::new()
                };
                format!("{} every {} beats{from}", c.patterns[b.pattern].id, b.every)
            })
            .collect();
        moves += &format!(
            " The lullaby: {} verses ({notes} notes) at a beat of {} s, each piped by the dancers and then played back by you on five keys (left mouse, Q, E, R, F). A note within {} s of its time is great, within {} s good; {}% of a verse's notes good or better and it sleeps a verse deeper, fewer and it stirs ({}) and the verse comes again. On the beat it fires: {}.",
            l.verses.len(),
            s(l.beat),
            s(l.great),
            s(l.good),
            l.pass_pct,
            c.patterns[l.stir].id,
            beats.join("; ")
        );
    }
    json!({
        "name": d.name,
        "zone": zone,
        "health": d.hp,
        "size": format!("{} m across", m(d.radius * 2)),
        "touch": format!("{} damage", d.contact),
        "clemency": d.clemency,
        "moves": moves,
        "order": match d.kind {
            Kind::Boss => "Attacks in the listed order within its phase, repeating.",
            Kind::Moth => "No ranged attack.",
            _ => "Attacks in the listed order, repeating.",
        },
        "phases": phases,
    })
}

/// What an upgrade does, in numbers.
pub fn upgrade_effect(u: &Upgrade, pal: &crate::content::Palette) -> String {
    let n = u.amount;
    match u.effect.as_str() {
        "max_health" => format!("+{n} maximum health (base {BASE_HEALTH})."),
        "speed" => format!("+{n}% run speed (base {} m/s).", mps(RUN_SPEED)),
        "jumps" => format!("Every run starts with {} jumps instead of {BASE_JUMPS}.", BASE_JUMPS + n),
        "dash" => format!(
            "+{n} dash charge ({} in all). Each charge recharges in {} s.",
            DASH_CHARGES + n,
            s(DASH_RECHARGE)
        ),
        "armory" => format!("The gun you carry into a run starts at level {n} instead of level 1."),
        "exhibit" => "A run may carry in this gun instead of the Carbine: choose it under Carry in. It starts at the level the Standing Armory gives.".to_string(),
        "recess" => format!("Once a run, a hit that would kill you leaves you at {RECESS_HEALTH_PCT}% of maximum health instead."),
        "grapple" => format!(
            "Hold F: a hook flies along your aim up to {} m from you and catches on any surface or enemy. It pulls you at {} m/s², up to {} m/s, and lets go when you release F, come within {} m, or after {} s. You keep your speed. Steering still works while pulled. Aimed at a wall up to {} m below a ledge, it lifts you over the ledge. If it stops moving you for {} s it lets go. {} s cooldown. Jumping or dashing lets go.",
            m(GRAPPLE_RANGE),
            mps2(GRAPPLE_PULL),
            mps(GRAPPLE_MAX_SPEED),
            m(GRAPPLE_ARRIVE),
            s(GRAPPLE_MAX_TICKS),
            m(MANTLE_REACH),
            s(GRAPPLE_STALL_TICKS),
            s(GRAPPLE_COOLDOWN)
        ),
        "parry" => format!(
            "Q: a parry lasting {} s. A gold orb that comes within {} m of your chest during it flies back at the enemy that fired it (or where you aim, if that enemy is gone) at {} m/s for {PARRY_DAMAGE} damage. Orbs of other colours still hit you. {} s cooldown.",
            s(PARRY_WINDOW),
            m(PARRY_REACH),
            mps(PARRY_SPEED),
            s(PARRY_COOLDOWN)
        ),
        "wall" => format!(
            "R: a wall of light {} m wide and {} m tall stands {} m in front of you for {} s, turning as you turn. Every blue orb (cyan in the Waiting City, cobalt on the ice; each has a pale rim) that meets it breaks; a Standing Order and a Permanent Injunction add further colours. Other orbs, ground waves and enemies pass through it. {} s cooldown.",
            m(WALL_HALF_WIDTH * 2),
            m(WALL_HEIGHT),
            m(WALL_AHEAD),
            s(WALL_TICKS),
            s(WALL_COOLDOWN)
        ),
        "pound" => format!(
            "C in the air: drop straight down at {} m/s. Landing sends a shockwave along the floor that deals {POUND_DAMAGE} damage to every enemy and weak point within {} m, then bounces you back up to {} m over the height you dropped from (at least {} m over the floor), with your air jumps back. You cannot be hit while you drop, nor for {} s after landing. {} s cooldown.",
            mps(POUND_SPEED),
            m(POUND_RADIUS),
            m(POUND_OVER),
            m(POUND_MIN_BOUNCE),
            s(POUND_IFRAMES),
            s(POUND_COOLDOWN)
        ),
        "pound_plus" => format!("The ground pound's shockwave deals {POUND_PLUS_DAMAGE} damage within {} m.", m(POUND_PLUS_RADIUS)),
        "grapple_plus" => format!(
            "The hook reaches {} m further ({} m) and is ready again {} s after it lets go.",
            m(GRAPPLE_PLUS_RANGE),
            m(GRAPPLE_RANGE + GRAPPLE_PLUS_RANGE),
            s(GRAPPLE_PLUS_COOLDOWN)
        ),
        "parry_plus" => format!(
            "The parry lasts {} s and is ready again after {} s.",
            s(PARRY_PLUS_WINDOW),
            s(PARRY_PLUS_COOLDOWN)
        ),
        "wall_plus" => format!(
            "The Injunction's wall stands {} s and is ready again after {} s.",
            s(WALL_PLUS_TICKS),
            s(WALL_PLUS_COOLDOWN)
        ),
        "wall_ward" => {
            // its amount is the ward it opens
            let colours = pal
                .wards
                .get((u.amount - 1).max(0) as usize)
                .map(|w| w.join(", "))
                .unwrap_or_default();
            format!(
                "The Injunction's wall breaks one more family of orbs: {colours}, besides every colour it broke before. Those it breaks have a pale rim. Gold orbs are the parry's, and no wall breaks them."
            )
        }
        "appeal" => "Recess turns a second killing blow in the same run.".to_string(),
        "stop_plus" => format!(
            "Your time stop lasts {} s more (it is {} s once the Defendant is beaten, {} s with every Stopped Clock). {} s cooldown.",
            s(STOP_PLUS * n),
            s(STOP_BASE),
            s(STOP_BASE + STOP_PLUS * 3),
            s(STOP_COOLDOWN)
        ),
        "bench" => match n {
            1 => format!(
                "V lays a Bench: a {} m plank, under you in the air or where you aim, that stands {} s. You stand on it; shots and orbs pass through. {} s cooldown. The river's timeline needs it.",
                m(BENCH_SIZE),
                s(BENCH_TICKS),
                s(BENCH_COOLDOWN)
            ),
            2 => "Two Benches may stand at once.".to_string(),
            _ => format!("A Bench stands {} s.", s(BENCH_LONG_TICKS)),
        },
        "precedent" => format!("Every run starts with Piercing Rounds: shots pass through {n} more enemy."),
        "conduct" => format!("+{n}% to all Clemency earned."),
        "bail" => format!("Clearing an arena restores {n} more health ({} in all).", HEAL_ON_CLEAR + n),
        "lien" => format!(
            "A Ripsaw {} waits where every run arrives, and where every later zone begins. Press E by it to take it in place of your gun.",
            crate::world::roman(crate::world::LIEN_LEVEL)
        ),
        _ => String::new(),
    }
}

/// A gun's second fire in words.
pub fn alt_words(a: &crate::content::Alt) -> String {
    use crate::content::AltKind;
    let what = match a.kind {
        AltKind::Burst => format!("{} rounds of {} damage, one after another, each passing through {} enemy", a.count, a.damage, a.pierce),
        AltKind::Slug => format!("one slug of {} damage that passes through {} enemies", a.damage, a.pierce),
        AltKind::Cluster => format!(
            "{} bombs in a fan, {}° apart, each {} damage and bursting for {} within {} m",
            a.count,
            deg(a.spread),
            a.damage,
            a.splash,
            m(a.splash_radius)
        ),
        AltKind::Fan => format!(
            "fans the hammer: every round left in the cylinder, one every {} s, {} damage each, a little wild",
            s(crate::weapons::FAN_EVERY),
            a.damage
        ),
        AltKind::Detonate => format!(
            "every blade lodged in an enemy bursts for {} damage and half the damage it had left to deal, and {} to others within {} m; the blades come back (nothing happens while none is lodged)",
            a.damage,
            a.splash,
            m(a.splash_radius)
        ),
        AltKind::Cloture => format!(
            "one lance of {} damage, plus up to {} more as the gun is spun up, passing through {} enemies; it spends the spin",
            a.damage,
            a.spin_damage,
            a.pierce
        ),
        AltKind::Redaction => format!(
            "a flat bar {} m wide slides forward {} m, erasing every enemy orb it crosses but gold and dealing {} damage to each enemy it passes through",
            m(a.half_w * 2),
            m(a.speed * a.life),
            a.damage
        ),
    };
    format!(
        "Right mouse, {}: {what}. {} s cooldown. Each level above I adds {}% to its damage.",
        a.name,
        s(a.cooldown),
        crate::content::ALT_LEVEL_PCT
    )
}

/// The rules not tied to one gun or enemy.
fn rules(c: &Content) -> Vec<String> {
    let clemency: Vec<String> = c
        .zones
        .iter()
        .filter(|z| z.built)
        .map(|z| {
            format!(
                "in {}, {} for a main fight cleared, {} for a side fight, {} for the boss",
                z.name, z.route.clemency_main, z.route.clemency_detour, z.route.clemency_boss
            )
        })
        .collect();
    vec![
        format!("You start each run with {BASE_HEALTH} health, {BASE_JUMPS} jump and {DASH_CHARGES} dash charges, before upgrades."),
        format!("Running speed: {} m/s. A standing jump rises about {} m; each further jump in the air is a little lower. At most {MAX_JUMPS} jumps.", mps(RUN_SPEED), m(jump_height())),
        "A jump pushed in a direction while in the air turns you that way at full running speed.".to_string(),
        format!("Dash: {} m in {} s along the way you are moving (or facing). You take no damage for the first {} s. {DASH_CHARGES} charges, each back in {} s.", m(DASH_SPEED * DASH_TICKS), s(DASH_TICKS), s(DASH_IFRAMES), s(DASH_RECHARGE)),
        format!("Orbs deal {ORB_DAMAGE} damage. Their hit area is {ORB_HIT_PCT}% of their drawn size. Ground waves deal {WAVE_DAMAGE}. Touching an enemy deals its touch damage."),
        format!("After taking damage you cannot be hit again for {} s.", s(HURT_IFRAMES)),
        format!("Falling out of the arena returns you to the last floor you stood on, for {FALL_DAMAGE} damage."),
        format!(
            "Clearing an arena restores {HEAL_ON_CLEAR} health. A defeated enemy other than the boss leaves a heal of {} health {}% of the time; walk over it to take it.",
            crate::world::HEAL_DROP,
            crate::world::HEAL_DROP_PCT
        ),
        "An arena's gates open only when every enemy in it is defeated. Blue gates continue the main path; orange gates start a side path.".to_string(),
        "Side paths are one or two fights a tier harder than the main fight they leave. The last fight holds the reward shown on the map and over the gate, and the path rejoins the next main arena.".to_string(),
        format!("Rewards: a gun one level above the gun in your hand when you take it (up to level {}), +{HEALTH_PICKUP} maximum health, +{SPEED_PICKUP_PCT}% run speed, one more jump (up to {MAX_JUMPS}), a Purse of Clemency ({PURSE_TIMES} times the zone's side-path share, kept when you die), or one of four that last the run: Piercing Rounds (every shot passes through {PIERCE_BONUS} more enemy), Dash Pulse (a dash removes every enemy orb within {} m of you, at most once every {} s), Mending Parry (each orb you turn back heals {MEND_HEAL}), Contempt (an enemy you kill bursts for {CONTEMPT_PCT}% of its own maximum health on every other enemy within {} m, and what that kills bursts too; each further Contempt adds {} m, up to {CONTEMPT_MAX}; a boss never bursts).", crate::world::roman(MAX_GUN_LEVEL), m(PULSE_RADIUS), s(PULSE_COOLDOWN), m(CONTEMPT_RADIUS), m(CONTEMPT_WIDEN)),
        format!("A side path's reward is rolled from the zone's table: no reward twice in a zone, and nothing you could not use. A path two fights long holds a choice of two: press E at one; the other is lost. About {} side paths in a hundred are sealed: the map does not show what one holds, and it holds more: a gun a level higher, or two rewards that are both yours.", c.zones[0].route.sealed_pct),
        format!("The gun you carry into a run is chosen in the Holding: the Carbine, or any gun whose Exhibit you have bought. The Standing Armory starts it at level {} at most; side paths take it up to {}.", c.upgrades.iter().filter(|u| u.effect == "armory").map(|u| u.amount).max().unwrap_or(1), crate::world::roman(MAX_GUN_LEVEL)),
        format!("The Antechamber offers a choice: {REST_HEAL_PCT}% of your maximum health back, or one reward. Press E at one; the other is lost."),
        format!("A wave of enemies comes {} s after the last one is defeated.", s(WAVE_GAP)),
        format!("An enemy that cannot see you when it finishes winding up holds that attack and tries again sooner; the next one it fires regardless. An enemy (not the boss) that has not seen you for {} s reappears where it can.", s(LOST_TICKS)),
        format!("A gun on the floor is taken with E within {} m. Your old gun is left in its place.", m(PICKUP_REACH)),
        format!("Clemency: {}; plus each enemy's own value. You keep it all when you die. Dying mid-fight credits that fight's share for each of its enemies you killed. Your first death adds a grant of {FIRST_DEATH_GRANT}. Each 10 years of sentence add {SENTENCE_BONUS_PCT}% to all Clemency earned.", clemency.join("; ")),
        "Beating a zone's boss does not end the run: after an interlude the run goes on into the next zone, with your health restored and everything you carry kept. Once a zone's boss has been beaten, a run can also start at the next zone, from the Holding.".to_string(),
        "Zone two's and zone three's arrivals hold a cache of two rewards: press E at one to take it; the other is lost.".to_string(),
        format!(
            "In the house, people take cover: they hide where you cannot see them and step out to fire. Two who are ready wait for each other and fire on the same tick when they stand at least {}° apart round you; with a Handler alive, three or more fire at once from round you, and all of them rest {COMMAND_PCT}% less. One waits at most {} s.",
            deg(CROSSFIRE_APART),
            s(SYNC_WAIT)
        ),
        format!(
            "G grabs an enemy at {GRAB_PCT}% of its health or less (not a boss, a mini-boss, a tank or a mech) within {} m in front of you; G again, or {} s later, throws it along your aim. It dies, and what it hits takes {THROW_DAMAGE}, and others within {} m {THROW_SPLASH}.",
            m(GRAB_REACH),
            s(GRAB_HOLD),
            m(THROW_RADIUS)
        ),
        "On the Ascent every arena stands higher up the mountain than the last, and off its edges is a fall. Its creatures are corrupted by the ritual and by what is coming: the higher the arena, the more clockwork and eyes they carry.".to_string(),
        "Beating the game opens the boss rush in the Holding: each zone's boss in turn, from the Bailiff on, your health restored between them, with what the Holding gives you to start a run.".to_string(),
        format!("From the house on, a deep side path's gun stands two levels above yours. Rewards found from the house on: Hair Trigger (guns fire {TRIGGER_PCT}% faster; they stack) and Plating (hits do {PLATING_PCT}% less, up to {PLATING_MAX})."),
        format!("Updraft vents (glowing grates on the ice) throw you about {} m up when you stand on them. You keep your air jumps.", m(fx::div(fx::mul(VENT_SPEED, VENT_SPEED) / 2, GRAVITY))),
        "Some attacks place their orbs in a shape and hold them still before they fly: a held orb is smaller, with a ring round it, and swells to full size just before it moves. Held orbs hurt to touch.".to_string(),
        format!("Each death reads the next count of the indictment and adds years to the sentence: 1 for the first death, 2 for the second, and so on. The first run to reach a tier {FAR_TIER} fight, and the first win, each read a count too."),
    ]
}

pub fn codex_json(c: &Content) -> String {
    let guns: Vec<Value> = c
        .guns
        .iter()
        .map(|g| {
            let mut levels: Vec<Value> = g
                .levels
                .iter()
                .enumerate()
                .map(|(k, l)| gun_level(l, k + 1, k + 1 >= 3 && l.splash_radius > 0))
                .collect();
            // a note every level shares is said once, for the gun
            let first: Vec<Value> = levels[0]["notes"].as_array().cloned().unwrap_or_default();
            let shared: Vec<Value> = first
                .into_iter()
                .filter(|n| {
                    levels
                        .iter()
                        .all(|l| l["notes"].as_array().is_some_and(|a| a.contains(n)))
                })
                .collect();
            for l in levels.iter_mut() {
                if let Some(a) = l["notes"].as_array_mut() {
                    a.retain(|n| !shared.contains(n));
                }
            }
            let mut notes = shared;
            notes.push(json!(alt_words(&g.alt)));
            if g.level(1).mag > 0 {
                notes.push(json!(format!(
                    "{} rounds in the cylinder. Once you let the trigger be for {} s it loads one shell every {} s; emptied, it loads all {} before it fires again.",
                    g.level(1).mag,
                    s(RELOAD_WAIT),
                    s(g.level(1).shell),
                    g.level(1).mag
                )));
            }
            if let Some(w) = g.level(1).saw {
                notes.push(json!(format!(
                    "Each click throws a blade, while one is in hand ({} at level I, one more each level). A blade lodges in what it hits; holding the trigger revs every lodged blade for {} damage every {} s at level I, until it has dealt {} in all; then it comes back after {} s.",
                    w.saws,
                    w.rev_damage,
                    s(w.rev_every),
                    w.teeth,
                    s(w.recall)
                )));
            }
            json!({ "id": g.id, "name": g.name, "role": g.blurb, "notes": notes, "levels": levels })
        })
        .collect();
    let enemies: Vec<Value> = c.enemies.iter().map(|d| enemy(d, c)).collect();
    let upgrades: Vec<Value> = c
        .upgrades
        .iter()
        .map(|u| {
            let needs = c.upgrades.iter().find(|o| o.id == u.requires).map(|o| o.name.clone()).unwrap_or_default();
            json!({ "name": u.name, "cost": u.cost, "needs": needs, "effect": upgrade_effect(u, &c.pal), "icon": u.icon })
        })
        .collect();
    json!({
        "controls": controls(c),
        "guns": guns,
        "enemies": enemies,
        "upgrades": upgrades,
        "rules": rules(c),
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;

    #[test]
    fn decimals_round_and_drop_trailing_zeros() {
        assert_eq!(dec(ONE * 3 / 2, 2), "1.5");
        assert_eq!(dec(ONE, 2), "1");
        assert_eq!(dec(ONE / 3, 2), "0.33");
        assert_eq!(dec(-ONE / 2, 1), "-0.5");
        assert_eq!(s(15), "0.25");
        assert_eq!(mps(RUN_SPEED), "9.5");
    }

    #[test]
    fn the_carbine_entry_matches_its_data() {
        // weapons.json: 8 damage every 105 ms, which the game rounds to 6
        // ticks: 10 shots a second, 80 damage a second; 90 m/s for 1.6 s
        let c = content();
        let v: Value = serde_json::from_str(&codex_json(&c)).unwrap();
        let l1 = &v["guns"][0]["levels"][0];
        assert_eq!(l1["damage"], "8");
        assert_eq!(l1["rate"], "10 shots/s (0.1 s apart)");
        assert_eq!(l1["dps"], "80");
        assert_eq!(l1["speed"], "90 m/s");
        assert_eq!(l1["range"], "144 m");
    }

    #[test]
    fn a_spiral_is_described_from_its_numbers() {
        let c = content();
        let p = c.patterns.iter().find(|p| p.id == "spiral_quad").unwrap();
        let t = volley(&p.volleys[0]);
        // patterns.json: 4 orbs of 34 cm radius at 8.2 m/s, 28 times, 75 ms
        // apart (rounded to 5 ticks, 0.08 s), turned 9° each time
        assert!(t.starts_with("Ring of 4 0.68 m orbs"), "{t}");
        assert!(t.contains("at 8.2 m/s"), "{t}");
        assert!(
            t.contains("fired 28 times, 0.08 s apart, each turned 9° from the last"),
            "{t}"
        );
        assert!(t.ends_with(&format!("{ORB_DAMAGE} damage a hit.")), "{t}");
    }

    #[test]
    fn every_upgrade_and_enemy_has_a_description() {
        let c = content();
        for u in &c.upgrades {
            assert!(!upgrade_effect(u, &c.pal).is_empty(), "{}", u.id);
        }
        let v: Value = serde_json::from_str(&codex_json(&c)).unwrap();
        assert_eq!(v["enemies"].as_array().unwrap().len(), c.enemies.len());
        let controls = v["controls"].as_array().unwrap();
        assert!(!controls.is_empty());
        for row in controls {
            assert!(
                !row[1].as_str().unwrap().contains('{'),
                "an unfilled placeholder: {row}"
            );
        }
    }
}
