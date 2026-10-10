//! The Codex: every mechanical line the game shows, written from the same
//! numbers the simulation runs on, so a description cannot drift from what
//! a gun, an orb or an upgrade does. Story text lives in `data/story.json`;
//! nothing here has a voice.
//!
//! Units shown: metres, seconds, metres a second, degrees, health points.

use crate::balance::*;
use crate::content::{Anchor, Content, EnemyDef, Form, GunLevel, Kind, Launch, Shape, Upgrade, Volley};
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
    parts.push(format!(
        "{}/s",
        dec(fx::div(TICKS_PER_SECOND * ONE, l.rate * ONE), 1)
    ));
    if l.splash_radius > 0 {
        parts.push(format!("{} burst {} m", l.splash, m(l.splash_radius)));
    }
    if l.pierce > 0 {
        parts.push(format!("pierces {}", l.pierce));
    }
    parts.join(" · ")
}

fn gun_level(l: &GunLevel, level: usize, erases: bool) -> Value {
    let per_s = fx::div(TICKS_PER_SECOND * ONE, l.rate * ONE);
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
    if l.pellets > 1 {
        notes.push("The first pellet flies along the aim; the rest scatter inside the spread. Damage per second assumes every pellet hits.".to_string());
    }
    json!({
        "level": level,
        "damage": if l.pellets > 1 { format!("{} pellets × {}", l.pellets, l.damage) } else { l.damage.to_string() },
        "rate": format!("{} shots/s ({} s apart)", dec(per_s, 1), s(l.rate)),
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
    if v.blue {
        text += "; blue: the Injunction breaks these";
    }
    let damage = v.damage;
    text + &format!(". {damage} damage a hit.")
}

fn launch_words(l: Launch) -> &'static str {
    match l {
        Launch::Keep => "on the way it was going",
        Launch::Out => "outward from the shape's centre",
        Launch::In => "inward to the shape's centre",
        Launch::Aim => "at where you are at that moment",
        Launch::Lead => "at where you will be if you keep moving as you are",
        Launch::Down => "straight down",
        Launch::Along => "along the line from the enemy to where you were",
        Launch::Spin => "round the shape's centre",
    }
}

/// A held formation in words.
fn formation(v: &Volley, size: &str, speed: &str) -> String {
    let r = m(v.radius);
    let shape = match v.form {
        Form::Ring => format!("a ring of {} {size} {r} m in radius", v.count),
        Form::Grid => format!("a grid of {} by {} {size}, {r} m apart", v.count, v.rows),
        Form::Line => format!("a line of {} {size} {} m long", v.count, m(v.radius * 2)),
        Form::Column => format!("a column of {} {size} {r} m tall", v.count),
        Form::Star => format!("a star of {} arms of {} {size}, {r} m long", v.count, v.rows),
        Form::Spiral => format!("a spiral of {} {size}, {} turns out to {r} m", v.count, v.rows),
        Form::Arc => format!("an arc of {} {size} across {}° at {r} m", v.count, deg(v.spread)),
        Form::Ribbon => format!("a wavy line of {} {size} {} m long", v.count, m(v.radius * 2)),
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
                "from": if d.kind == Kind::Boss { format!("Phase {} (from {}% health)", k + 1, p.from_pct) } else { String::new() },
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
pub fn upgrade_effect(u: &Upgrade) -> String {
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
        "armory" => format!("Every run starts with a level {n} Carbine instead of level 1."),
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
            "Right mouse or Q: a parry lasting {} s. A gold orb that comes within {} m of your chest during it flies back at the enemy that fired it (or where you aim, if that enemy is gone) at {} m/s for {PARRY_DAMAGE} damage. Orbs of other colours still hit you. {} s cooldown.",
            s(PARRY_WINDOW),
            m(PARRY_REACH),
            mps(PARRY_SPEED),
            s(PARRY_COOLDOWN)
        ),
        "wall" => format!(
            "R: a wall of light {} m wide and {} m tall stands {} m in front of you for {} s, turning as you turn. Every blue orb (cyan in the Waiting City, cobalt on the ice; each has a pale rim) that meets it breaks. Orbs of other colours, ground waves and enemies pass through it. {} s cooldown.",
            m(WALL_HALF_WIDTH * 2),
            m(WALL_HEIGHT),
            m(WALL_AHEAD),
            s(WALL_TICKS),
            s(WALL_COOLDOWN)
        ),
        _ => String::new(),
    }
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
        format!("Rewards: a gun of level 2 or 3 (never a Carbine no better than the one you start with), +{HEALTH_PICKUP} maximum health, +{SPEED_PICKUP_PCT}% run speed, one more jump (up to {MAX_JUMPS}), or one of three that last the run: Piercing Rounds (every shot passes through {PIERCE_BONUS} more enemy), Dash Pulse (a dash removes every enemy orb within {} m of you, at most once every {} s), Mending Parry (each orb you turn back heals {MEND_HEAL}).", m(PULSE_RADIUS), s(PULSE_COOLDOWN)),
        format!("The Antechamber offers a choice: {REST_HEAL_PCT}% of your maximum health back, or one reward. Press E at one; the other is lost."),
        format!("A wave of enemies comes {} s after the last one is defeated.", s(WAVE_GAP)),
        format!("An enemy that cannot see you when it finishes winding up holds that attack and tries again sooner; the next one it fires regardless. An enemy (not the boss) that has not seen you for {} s reappears where it can.", s(LOST_TICKS)),
        format!("A gun on the floor is taken with E within {} m. Your old gun is left in its place.", m(PICKUP_REACH)),
        format!("Clemency: {}; plus each enemy's own value. You keep it all when you die. Dying mid-fight credits that fight's share for each of its enemies you killed. Your first death adds a grant of {FIRST_DEATH_GRANT}. Each 10 years of sentence add {SENTENCE_BONUS_PCT}% to all Clemency earned.", clemency.join("; ")),
        "Beating a zone's boss does not end the run: after an interlude the run goes on into the next zone, with your health restored and everything you carry kept. Once a zone's boss has been beaten, a run can also start at the next zone, from the Holding.".to_string(),
        "Zone two's arrival holds a cache of two rewards: press E at one to take it; the other is lost.".to_string(),
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
            json!({ "id": g.id, "name": g.name, "role": g.blurb, "notes": shared, "levels": levels })
        })
        .collect();
    let enemies: Vec<Value> = c.enemies.iter().map(|d| enemy(d, c)).collect();
    let upgrades: Vec<Value> = c
        .upgrades
        .iter()
        .map(|u| {
            let needs = c.upgrades.iter().find(|o| o.id == u.requires).map(|o| o.name.clone()).unwrap_or_default();
            json!({ "name": u.name, "cost": u.cost, "needs": needs, "effect": upgrade_effect(u), "icon": u.icon })
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
            assert!(!upgrade_effect(u).is_empty(), "{}", u.id);
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
