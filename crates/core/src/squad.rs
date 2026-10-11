//! The house's agents as a squad. Each agent, as its rest begins, asks for
//! a place, and the squad gives it one by its role:
//!
//! - a rifle (the Grey Man) and the Handler take cover: a place you cannot
//!   see from where you stand, beside something tall, with a spot within a
//!   few metres it can step out to and see you from; the squad spreads them
//!   round you, each as far round from the others as it can, so together
//!   they surround you;
//! - a flanker (the Outrider) goes to your back: round past the edge of
//!   what you can see from where you face;
//! - a marksman takes the highest place it can reach that sees you from far;
//! - a grenadier (the Sapper) takes cover and needs no line to you;
//! - a shield (the Warder) and a rusher (the Cleaner) come at you.
//!
//! The squad also fires together. A rifle, a flanker or the Handler, its
//! rest over and stepped out, waits (`Ready`) for a partner: two waiting
//! agents at least `CROSSFIRE_APART` round you from each other wind up
//! together and fire on the same tick (a crossfire; with a flanker at your
//! back, a pincer); while a Handler lives, three or more waiting fire as a
//! ring, and every agent rests `COMMAND_PCT` percent less. An agent left
//! waiting `SYNC_WAIT` fires alone.

use crate::balance::*;
use crate::content::{Content, Kind, Role};
use crate::enemy::{Enemy, Order, State};
use crate::fx::{self, int, ONE};
use crate::geom::{facing, v3, yaw_pitch_of, V3};
use crate::level::Level;
use crate::nav::Nav;

/// The player as the squad sees them.
#[derive(Debug, Clone, Copy)]
pub struct Mark {
    pub feet: V3,
    pub chest: V3,
    /// where they face
    pub yaw: i32,
}

/// Places asked for in one tick, at most: a place costs a search.
const PLACES_PER_TICK: usize = 2;
/// a path search gives up past this many places
const PATH_LIMIT: usize = 24_000;

/// The angle round `m` that `p` stands at.
pub fn bearing(m: &Mark, p: V3) -> i32 {
    yaw_pitch_of(v3(p.x - m.feet.x, 0, p.z - m.feet.z)).0
}

/// Is `p` outside what the player sees from where they face?
pub fn behind(m: &Mark, p: V3) -> bool {
    fx::wrap(bearing(m, p) - m.yaw).abs() > VIEW_HALF
}

fn clear(level: &Level, from: V3, to: V3) -> bool {
    let d = to.sub(from);
    let len = d.len();
    len == 0 || level.ray(from, d.norm(), len).is_none()
}

/// Roles that wait to fire together.
fn syncs(r: Role) -> bool {
    matches!(r, Role::Rifle | Role::Flank | Role::Command)
}

/// Keeps a place inside the room.
fn inside(p: V3) -> V3 {
    let k = crate::arena::HALF - int(3);
    v3(p.x.clamp(-k, k), p.y, p.z.clamp(-k, k))
}

/// One tick of the squad: places for agents that need one, partners
/// started together, the Handler's haste.
pub fn direct(enemies: &mut [Enemy], c: &Content, nav: &Nav, level: &Level, m: &Mark, tick: i32) {
    let agents: Vec<usize> = (0..enemies.len())
        .filter(|&i| enemies[i].alive() && c.enemies[enemies[i].def].kind == Kind::Agent)
        .collect();
    if agents.is_empty() {
        return;
    }
    let command = agents
        .iter()
        .any(|&i| c.enemies[enemies[i].def].role == Role::Command && enemies[i].hurtable());
    // an agent waits for partners only when it has one
    let partners = agents
        .iter()
        .filter(|&&i| syncs(c.enemies[enemies[i].def].role))
        .count();
    for &i in &agents {
        let role = c.enemies[enemies[i].def].role;
        let e = &mut enemies[i];
        e.haste = if command { COMMAND_PCT } else { 0 };
        e.sync = syncs(role) && partners >= 2;
    }

    // places
    let mut asked = 0;
    for &i in &agents {
        if asked == PLACES_PER_TICK {
            break;
        }
        let e = &enemies[i];
        let role = c.enemies[e.def].role;
        let resting = matches!(e.state, State::Rest(_));
        let chase = matches!(role, Role::Rush | Role::Shield) && resting && (tick + i as i32 * 7) % 40 == 0;
        if !(chase || resting && e.order == Order::None) {
            continue;
        }
        if matches!(e.state, State::Spawning(_)) {
            continue;
        }
        asked += 1;
        place(enemies, i, c, nav, level, m, tick);
    }

    // partners
    let ready: Vec<usize> = agents
        .iter()
        .copied()
        .filter(|&i| matches!(enemies[i].state, State::Ready(_)))
        .collect();
    let windup = |e: &Enemy| {
        let d = &c.enemies[e.def];
        let atk = &d.phases[0].attacks;
        atk.get(e.attack % atk.len().max(1))
            .map_or(30, |a| a.windup.max(1))
    };
    let start = |enemies: &mut [Enemy], group: &[usize]| {
        let w = group.iter().map(|&i| windup(&enemies[i])).max().unwrap_or(30);
        for &i in group {
            enemies[i].state = State::Windup(w);
        }
    };
    if command && ready.len() >= 3 {
        start(enemies, &ready);
        return;
    }
    let mut pair = None;
    'outer: for (a, &i) in ready.iter().enumerate() {
        for &j in &ready[a + 1..] {
            let apart = fx::wrap(bearing(m, enemies[i].pos) - bearing(m, enemies[j].pos)).abs();
            if apart >= CROSSFIRE_APART {
                pair = Some([i, j]);
                break 'outer;
            }
        }
    }
    if let Some(p) = pair {
        start(enemies, &p);
    }
}

/// Finds agent `i` a place by its role, and the path there.
fn place(enemies: &mut [Enemy], i: usize, c: &Content, nav: &Nav, level: &Level, m: &Mark, tick: i32) {
    let d = &c.enemies[enemies[i].def];
    let role = d.role;
    let me = enemies[i].pos;
    let feet = enemies[i].feet(d);
    let others: Vec<i32> = enemies
        .iter()
        .enumerate()
        .filter(|(k, o)| *k != i && o.alive() && c.enemies[o.def].kind == Kind::Agent)
        .map(|(_, o)| bearing(m, o.path.last().copied().unwrap_or(o.pos)))
        .collect();
    // the bearing round you furthest from every other agent's, the
    // nearer to `from` on a tie; `from` itself when it is alone
    let spread = |from: i32, span: i32| {
        if others.is_empty() {
            return from;
        }
        let mut best = (i32::MIN, from);
        for k in 0..12 {
            let a = from - span / 2 + span / 11 * k;
            let gap = others
                .iter()
                .map(|&o| fx::wrap(a - o).abs())
                .min()
                .unwrap_or(fx::HALF)
                .min(fx::QUARTER * 3 / 2);
            let score = gap * 4 - fx::wrap(a - from).abs();
            if score > best.0 {
                best = (score, a);
            }
        }
        best.1
    };
    let range = d.range;
    let (order, goal, peek) = match role {
        Role::Rush | Role::Shield => {
            // at you, stopping at its range
            let to = v3(m.feet.x - feet.x, 0, m.feet.z - feet.z);
            let stop = to.len_xz() - range;
            let goal = if stop > 0 {
                feet.add(to.norm().scale(stop))
            } else {
                feet
            };
            (Order::Rush, Some(goal), None)
        }
        Role::Flank => {
            let a = spread(m.yaw + fx::HALF, fx::deg(90));
            let want = inside(m.feet.add(facing(a, 0).scale(range)));
            let spot = open_near(nav, level, m, want, int(8), |p| behind(m, p));
            (Order::Flank, spot, None)
        }
        Role::Sniper => {
            let spot = high(nav, level, m, me, range);
            (Order::High, spot, None)
        }
        _ => {
            let a = match role {
                Role::Command | Role::Chief => {
                    // behind its men, from you
                    let mean = others.first().copied().unwrap_or(bearing(m, me));
                    mean
                }
                _ => spread(bearing(m, me), fx::TURN),
            };
            let far = if matches!(role, Role::Command | Role::Chief) {
                range + int(5)
            } else {
                range
            };
            let want = inside(m.feet.add(facing(a, 0).scale(far)));
            // where it already hides, if that still works, saves a walk
            let here = (feet.sub(m.feet).len_xz() - far).abs() < int(6)
                && nav
                    .cover
                    .iter()
                    .any(|&n| nav.pos(n).is_some_and(|p| p.dist(feet) < ONE));
            let want = if here { feet } else { want };
            match cover(nav, level, m, want, role != Role::Grenadier) {
                Some((c, p)) => (Order::Cover, Some(c), p),
                None => {
                    let spot = open_near(nav, level, m, want, int(8), |_| true);
                    (Order::Hold, spot, None)
                }
            }
        }
    };
    let e = &mut enemies[i];
    e.order = order;
    e.peek = peek;
    e.walked = 0;
    e.path = match (goal, nav.nearest(feet)) {
        (Some(g), Some(from)) => match nav.nearest(g) {
            Some(to) => nav.path(from, to, PATH_LIMIT),
            None => Vec::new(),
        },
        _ => Vec::new(),
    };
    let _ = tick;
}

/// A cover place near `want`: hidden from you, and (if `peek`) with a spot
/// within `PEEK_REACH` that sees you. The nearest such to `want`.
fn cover(nav: &Nav, level: &Level, m: &Mark, want: V3, peek: bool) -> Option<(V3, Option<V3>)> {
    let mut near: Vec<(i32, V3)> = nav
        .cover
        .iter()
        .filter_map(|&n| nav.pos(n))
        .map(|p| (p.dist(want), p))
        .filter(|(dd, p)| *dd < int(9) && p.sub(m.feet).len_xz() > int(6) && inside(*p) == *p)
        .collect();
    near.sort_by_key(|(dd, _)| *dd);
    let head = |p: V3| p.add(v3(0, PLAYER_CHEST, 0));
    for (_, cpos) in near.into_iter().take(40) {
        if clear(level, m.chest, head(cpos)) {
            continue;
        }
        if !peek {
            return Some((cpos, None));
        }
        // step out sideways to see past it
        let to = m.feet.sub(cpos);
        let side = v3(-to.z, 0, to.x).norm();
        for k in [3, -3, 5, -5] {
            let at = cpos.add(side.scale(PEEK_REACH * k / 5));
            let Some(n) = nav.nearest(at.add(v3(0, ONE / 2, 0))) else {
                continue;
            };
            let Some(p) = nav.pos(n) else {
                continue;
            };
            if (p.y - cpos.y).abs() <= AGENT_STEP
                && p.dist(cpos) <= PEEK_REACH + ONE
                && clear(level, head(p), m.chest)
            {
                return Some((cpos, Some(p)));
            }
        }
    }
    None
}

/// An open place near `want` that sees you and passes `ok`.
fn open_near(
    nav: &Nav,
    level: &Level,
    m: &Mark,
    want: V3,
    reach: i32,
    ok: impl Fn(V3) -> bool,
) -> Option<V3> {
    let mut best: Option<(i32, V3)> = None;
    let step = int(2);
    let n = reach / step;
    for j in -n..=n {
        for i in -n..=n {
            let at = want.add(v3(i * step, ONE, j * step));
            let Some(k) = nav.nearest(at) else {
                continue;
            };
            let Some(p) = nav.pos(k) else {
                continue;
            };
            if p.sub(m.feet).len_xz() < int(5) || !ok(p) {
                continue;
            }
            if !clear(level, p.add(v3(0, PLAYER_CHEST, 0)), m.chest) {
                continue;
            }
            let dd = p.dist(want);
            if best.is_none_or(|(b, _)| dd < b) {
                best = Some((dd, p));
            }
        }
    }
    best.map(|(_, p)| p)
}

/// The highest place a walk from `me` reaches that sees you from at least
/// half its range off.
fn high(nav: &Nav, level: &Level, m: &Mark, me: V3, range: i32) -> Option<V3> {
    let from = nav.nearest(me.sub(v3(0, ONE, 0)))?;
    let mut best: Option<(i32, V3)> = None;
    for (k, n) in nav.reach(from).into_iter().enumerate() {
        if k % 7 != 0 {
            continue;
        }
        let Some(p) = nav.pos(n) else {
            continue;
        };
        let dist = p.sub(m.feet).len_xz();
        if dist < range / 2 || dist > range + int(10) {
            continue;
        }
        let score = p.y * 4 - (dist - range).abs();
        if best.is_some_and(|(b, _)| score <= b) {
            continue;
        }
        if clear(level, p.add(v3(0, PLAYER_CHEST, 0)), m.chest) {
            best = Some((score, p));
        }
    }
    best.map(|(_, p)| p)
}
