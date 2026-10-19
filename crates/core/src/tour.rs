//! A zone's walkthrough, opened from the map: the camera flies through the
//! zone's own arenas one after another, along the main path from its arrival
//! to its boss, as a fishing game shows a spot before you cast there. A
//! course is flown along the line a player is meant to take; any other arena
//! is circled. The arenas run as they would in play (the decks heave, the
//! crews and bosses move and fire) round a body parked at the start that
//! nothing can hurt and nobody sees.

use crate::balance::*;
use crate::content::Content;
use crate::course::Step;
use crate::fx::{self, int, ONE};
use crate::geom::{v3, V3};
use crate::player::{Controls, Player};
use crate::render::Camera;
use crate::route::{Have, Route};
use crate::weapons::Held;
use crate::world::{Exit, World};

/// The walkthrough of one zone.
pub struct Tour {
    pub zone: usize,
    /// the line a run started from here follows (the Peak or the River)
    pub line: String,
    pub route: Route,
    /// the main path, by node index
    pub stops: Vec<usize>,
    /// the stop shown now, and ticks into it
    pub at: usize,
    pub t: i32,
    pub world: World,
    /// the camera's way through this stop: a course's line, or empty for a
    /// circle round the arena
    path: Vec<V3>,
    /// ticks this stop is shown
    pub len: i32,
    eye: (V3, V3),
    prev_eye: (V3, V3),
}

/// The main path from the arrival to the boss.
fn main_path(route: &Route) -> Vec<usize> {
    let mut stops = vec![route.start()];
    let mut at = route.start();
    while let Some(&to) = route.nodes[at].next.first() {
        if stops.contains(&to) {
            break;
        }
        stops.push(to);
        at = to;
    }
    stops
}

/// Where a step of a course's line goes, if it goes anywhere.
fn step_to(s: &Step) -> Option<V3> {
    match *s {
        Step::To(p) | Step::Jump(p) | Step::Bound(p) | Step::Hook(p) | Step::Tug(p) | Step::Air(p) => Some(p),
        _ => None,
    }
}

/// The point `s` metres along the polyline.
fn along(path: &[V3], s: i32) -> V3 {
    let mut left = s.max(0);
    for w in path.windows(2) {
        let d = w[1].sub(w[0]).len();
        if left <= d {
            return w[0].lerp(w[1], fx::div(left, d.max(1)));
        }
        left -= d;
    }
    path.last().copied().unwrap_or(V3::ZERO)
}

fn length(path: &[V3]) -> i32 {
    path.windows(2).map(|w| w[1].sub(w[0]).len()).sum()
}

impl Tour {
    pub fn new(c: &Content, zone: usize, line: &str) -> Tour {
        let route = Route::build_for(c, zone, TOUR_SEED, &Have::level(1));
        let stops = main_path(&route);
        let world = Tour::world_at(c, &route, stops[0]);
        let mut t = Tour {
            zone,
            line: line.to_string(),
            route,
            stops,
            at: 0,
            t: 0,
            world,
            path: Vec::new(),
            len: 0,
            eye: (V3::ZERO, V3::ZERO),
            prev_eye: (V3::ZERO, V3::ZERO),
        };
        t.enter(c, 0);
        t
    }

    fn world_at(c: &Content, route: &Route, at: usize) -> World {
        let node = &route.nodes[at];
        let exits: Vec<Exit> = node
            .next
            .iter()
            .enumerate()
            .map(|(k, &to)| Exit {
                to,
                main: k == 0,
                reward: route.nodes[to].reward,
                sealed: route.nodes[to].sealed,
            })
            .collect();
        let player = Player::new(V3::ZERO, 0, Default::default());
        World::new(c, node, &exits, player, Held::new(0, 1), TOUR_SEED ^ at as u64)
    }

    /// Shows stop `k` from its beginning.
    pub fn enter(&mut self, c: &Content, k: usize) {
        self.at = k.min(self.stops.len() - 1);
        self.t = 0;
        self.world = Tour::world_at(c, &self.route, self.stops[self.at]);
        let w = &self.world;
        self.path = match &w.course {
            Some(k) if !k.line.is_empty() => {
                let mut p = vec![w.arena.start];
                p.extend(k.line.iter().filter_map(step_to));
                p.push(k.finish);
                p
            }
            _ => Vec::new(),
        };
        self.len = if self.path.is_empty() {
            TOUR_CIRCLE
        } else {
            (length(&self.path) / TOUR_SPEED).clamp(TOUR_MIN, TOUR_MAX)
        };
        self.eye = self.aim();
        self.prev_eye = self.eye;
    }

    /// The stop's archetype (its section).
    pub fn archetype<'a>(&'a self, _c: &'a Content) -> &'a str {
        &self.route.nodes[self.stops[self.at]].archetype
    }

    /// One tick: the arena runs on round the parked body, the camera moves
    /// on, and at the stop's end the next is shown (round to the first after
    /// the boss). True when the stop changed, so the scene is rebuilt.
    pub fn step(&mut self, c: &Content) -> bool {
        let start = self.world.arena.start;
        let p = &mut self.world.player;
        p.pos = start;
        p.prev_pos = start;
        p.vel = V3::ZERO;
        p.iframes = TOUR_SHIELD;
        p.health = p.build.max_health;
        let (eye, fwd) = self.eye;
        self.world.tick(c, &Controls::default(), eye, fwd);
        self.t += 1;
        if self.t >= self.len {
            let next = (self.at + 1) % self.stops.len();
            self.enter(c, next);
            return true;
        }
        self.prev_eye = self.eye;
        self.eye = self.aim();
        false
    }

    /// Where the camera is and where it looks, now.
    fn aim(&self) -> (V3, V3) {
        let f = fx::div(self.t, self.len.max(1));
        if self.path.is_empty() {
            // round the arena's middle (its boss's place, if it has one)
            let w = &self.world;
            let centre = w
                .course
                .as_ref()
                .and_then(|k| k.boss_at)
                .unwrap_or(w.arena.centre);
            let reach = w
                .arena
                .gates
                .iter()
                .map(|(g, _)| g.sub(centre).len_xz())
                .max()
                .unwrap_or(int(20))
                .clamp(TOUR_ORBIT_MIN, TOUR_ORBIT_MAX);
            let a = TOUR_ORBIT_FROM + fx::mul(TOUR_ORBIT_SWEEP, f);
            let eye = centre.add(v3(
                fx::mul(fx::sin(a), reach),
                TOUR_ORBIT_UP,
                fx::mul(fx::cos(a), reach),
            ));
            let look = centre.add(v3(0, int(1), 0));
            (eye, look.sub(eye).norm())
        } else {
            let total = length(&self.path);
            let s = fx::mul(total, f);
            let here = along(&self.path, s);
            let ahead = along(&self.path, s + TOUR_AHEAD);
            let way = v3(ahead.x - here.x, 0, ahead.z - here.z);
            let back = if way.len() > ONE / 10 {
                way.norm()
            } else {
                v3(0, 0, -ONE)
            };
            let eye = here.sub(back.scale(TOUR_BEHIND)).add(v3(0, TOUR_ABOVE, 0));
            (eye, ahead.sub(eye).norm())
        }
    }

    /// The camera, between the last tick and this one.
    pub fn camera(&self, alpha: i32) -> Camera {
        let eye = self.prev_eye.0.lerp(self.eye.0, alpha);
        let fwd = self.prev_eye.1.lerp(self.eye.1, alpha).norm();
        Camera { eye, fwd, fov: FOV }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;

    #[test]
    fn a_walkthrough_runs_the_main_path_from_the_arrival_to_the_boss() {
        let c = content();
        for (z, zone) in c.zones.iter().enumerate().filter(|(_, z)| z.built) {
            let t = Tour::new(&c, z, "peak");
            let first = &t.route.nodes[t.stops[0]];
            let last = &t.route.nodes[*t.stops.last().unwrap()];
            assert_eq!(first.kind, crate::route::NodeKind::Arrival, "{}", zone.id);
            assert_eq!(last.kind, crate::route::NodeKind::Boss, "{}", zone.id);
            assert!(
                t.stops.iter().all(|&s| !t.route.nodes[s].detour),
                "{}: a detour",
                zone.id
            );
        }
    }

    #[test]
    fn a_course_is_flown_along_its_line_and_the_camera_moves_on_through_it() {
        let c = content();
        let z = c.zones.iter().position(|z| z.id == "water_gap").unwrap();
        let mut t = Tour::new(&c, z, "river");
        assert!(t.path.len() > 4, "flown along the Spillway's line");
        let from = t.eye.0;
        for _ in 0..t.len / 2 {
            t.step(&c);
        }
        let on = t.eye.0;
        assert!(
            on.sub(from).len() > int(40),
            "the camera went {} m",
            on.sub(from).len() / ONE
        );
        // and the body nobody sees is still where it was parked, unhurt
        assert_eq!(t.world.player.pos, t.world.arena.start);
        assert_eq!(t.world.player.health, t.world.player.build.max_health);
    }

    #[test]
    fn each_stop_gives_way_to_the_next_and_after_the_boss_to_the_first() {
        let c = content();
        let z = c.zones.iter().position(|z| z.id == "far_shore").unwrap();
        let mut t = Tour::new(&c, z, "river");
        let n = t.stops.len();
        let mut seen = vec![t.at];
        let mut changed = 0;
        for _ in 0..TOUR_MAX * (n as i32 + 1) {
            if t.step(&c) {
                changed += 1;
                seen.push(t.at);
            }
        }
        assert!(changed >= n, "changed {changed} times over {n} stops");
        assert_eq!(seen[n], 0, "round again after the boss: {seen:?}");
    }
}
