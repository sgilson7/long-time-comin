//! The door the shim calls through: build the game from the data files, then
//! each display frame hand it the clock, the held keys and the mouse
//! movement, and read back what to draw and what the screens say.
//!
//! Modes: the title, the sentencing (the story's opening), the Holding (the
//! hub between runs), play, the paused and map overlays, the interlude
//! between one zone's boss and the next zone, the indictment read on a
//! death, and the verdict after the last built zone's boss. Only play runs the
//! simulation, at a fixed 60 ticks a second; `advance` runs the ticks the
//! elapsed time owes, and drawing is placed between the last two. Mouse look
//! is applied every frame, before any tick, so the view never waits.

use crate::arena::{self, Arena};
use crate::balance::*;
use crate::content::{hex, Content, Files};
use crate::draw;
use crate::fx::{self, int, ONE};
use crate::geom::{facing, right_of, v3, V3};
use crate::mesh::{self, Mesh};
use crate::meta::{Meta, Reading, Refusal};
use crate::player::{Controls, Player};
use crate::render::{self, Batch, Camera};
use crate::route::{Have, Node, NodeKind, Reward, Route};
use crate::weapons::Held;
use crate::world::{roman, Exit, Outcome, PickupKind, World};
use serde::Serialize;
use serde_json::{json, Value};

/// The actions a key or button can hold, and the bit each sets.
pub const ACTIONS: &[(&str, u32)] = &[
    ("forward", 1 << 0),
    ("back", 1 << 1),
    ("left", 1 << 2),
    ("right", 1 << 3),
    ("jump", 1 << 4),
    ("dash", 1 << 5),
    ("fire", 1 << 6),
    ("parry", 1 << 7),
    ("grapple", 1 << 8),
    ("interact", 1 << 9),
    // set while the pointer is captured; without it the simulation holds still
    ("focus", 1 << 10),
    ("wall", 1 << 11),
    ("pound", 1 << 12),
    ("alt", 1 << 13),
    ("grab", 1 << 14),
    ("stop", 1 << 15),
    ("bench", 1 << 16),
];

fn bit(name: &str) -> u32 {
    ACTIONS.iter().find(|(n, _)| *n == name).map_or(0, |(_, b)| *b)
}

/// One tick, in microseconds × ticks per second, so the sum stays exact.
const TICK_UNITS: i64 = 1_000_000;
pub const MAX_TICKS_PER_FRAME: u32 = 6;
const MAX_FRAME_US: i64 = 250_000;
const MAX_PIXELS: i64 = 2560 * 1440;
const HUNDREDTHS: i32 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Title,
    Intro,
    Holding,
    Play,
    Paused,
    Map,
    Interlude,
    Dead,
    Won,
}

struct Run {
    /// the zone the run is in, by index
    zone: usize,
    route: Route,
    at: usize,
    visited: Vec<usize>,
    world: World,
    /// Clemency banked from arenas already left
    earned: i32,
    cleared: i32,
    counted: bool,
    /// a boss rush: each zone's boss in turn, nothing between
    rush: bool,
    /// the lullaby alone, for testing: the god put to its song as it
    /// comes, and nothing kept on the record
    practice: bool,
    /// the last mission: zones one to four in one go, harder (new game
    /// plus), then home; and whether its brief has been read
    mission: bool,
    briefed: bool,
    /// on the river's timeline: past the house, the Water Gap
    river: bool,
}

impl Run {
    fn total(&self) -> i32 {
        self.earned + self.world.earned
    }
}

pub struct Game {
    c: Content,
    meta: Meta,
    mode: Mode,
    run: Option<Run>,
    reading: Option<Reading>,
    last: (i32, i32),
    /// a line of the indictment read during play, for the HUD's banner
    story: Option<(u32, String)>,
    story_seq: u32,
    /// the count a first win read, for the verdict
    win_count: Option<String>,
    /// the count a first crossing read, for the interlude
    crossing_count: Option<String>,
    hub: Arena,
    hub_seed: u64,
    menu_us: i64,
    scene_version: u32,
    prev_bits: u32,
    latched: u32,
    last_us: Option<i64>,
    acc: i64,
    alpha: i32,
    css: (i32, i32),
    pixels: (i32, i32),
    camera: Camera,
    static_values: Vec<i32>,
    static_draws: Vec<Vec<u32>>,
    values: Vec<i32>,
    draws: Vec<u32>,
    view: Vec<i32>,
}

impl Game {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        weapons: &str,
        patterns: &str,
        enemies: &str,
        zone: &str,
        upgrades: &str,
        palette: &str,
        story: &str,
        copy: &str,
        saved: &str,
    ) -> Result<Game, String> {
        let c = Content::parse(&Files {
            weapons,
            patterns,
            enemies,
            zone,
            upgrades,
            palette,
            story,
            copy,
        })?;
        let meta = Meta::load(saved, &c);
        let hub_seed = 2031;
        let hub = arena::build(&c.zones[0].id, "arrival", hub_seed, &[], &c.pal);
        let mut g = Game {
            c,
            meta,
            mode: Mode::Title,
            run: None,
            reading: None,
            last: (0, 0),
            story: None,
            story_seq: 0,
            win_count: None,
            crossing_count: None,
            hub,
            hub_seed,
            menu_us: 0,
            scene_version: 0,
            prev_bits: 0,
            latched: 0,
            last_us: None,
            acc: 0,
            alpha: ONE,
            css: (1, 1),
            pixels: (1, 1),
            camera: Camera {
                eye: v3(0, int(6), int(30)),
                fwd: v3(0, 0, -ONE),
                fov: FOV,
            },
            static_values: Vec::new(),
            static_draws: vec![Vec::new(); render::PASSES],
            values: Vec::new(),
            draws: Vec::new(),
            view: Vec::new(),
        };
        g.build_scene();
        g.draw();
        Ok(g)
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    // ---- for tools that drive the game without a page (the playtest
    // harness): read and steer the arena in play, and the record ----------

    pub fn world(&self) -> Option<&World> {
        self.run.as_ref().map(|r| &r.world)
    }

    pub fn world_mut(&mut self) -> Option<&mut World> {
        self.run.as_mut().map(|r| &mut r.world)
    }

    /// The run's map and where in it the player is.
    pub fn route(&self) -> Option<(&Route, usize)> {
        self.run.as_ref().map(|r| (&r.route, r.at))
    }

    pub fn meta(&self) -> &Meta {
        &self.meta
    }

    pub fn content(&self) -> &Content {
        &self.c
    }

    /// Straight into node `to` of the run's map, for tools that drive the
    /// game (the playtest harness, the screenshot script).
    pub fn go_to(&mut self, to: usize) {
        if self.mode == Mode::Play && self.run.as_ref().is_some_and(|r| to < r.route.nodes.len()) {
            self.go(to);
        }
    }

    /// Breaks every weak point the boss has open now, for tools that drive
    /// the game (to see or measure a later phase).
    pub fn break_weak_points(&mut self) {
        let Some(run) = self.run.as_mut() else {
            return;
        };
        let w = &mut run.world;
        let Some(b) = w.boss else {
            return;
        };
        w.enemies[b].state = crate::enemy::State::Rest(1);
        let d = self.c.enemies[w.enemies[b].def].clone();
        for (at, _, k) in crate::tank::weak_points(&w.enemies[b], &d) {
            w.hurt_weak(&self.c, b, k, i32::MAX / 2, at);
        }
    }

    /// Brings the boss in the arena down at once, for tools that drive the
    /// game (to see or measure what follows its fall).
    pub fn fell_boss(&mut self) {
        let Some(run) = self.run.as_mut() else {
            return;
        };
        let w = &mut run.world;
        if let Some(b) = w.boss {
            let e = &mut w.enemies[b];
            e.state = crate::enemy::State::Rest(10);
            if let Some(t) = e.tank.as_mut() {
                t.survive = 1;
            } else {
                e.floor_hp = 0;
                e.hp = 1;
                e.hurt(10);
                if e.shielded {
                    // the Oak is never hurt: it simply falls
                    e.hp = 0;
                }
            }
        }
    }

    /// Empties the god's last bar at once, so the lullaby begins, for
    /// tools that drive the game (to see or measure the song).
    pub fn lull(&mut self) {
        let Some(run) = self.run.as_mut() else {
            return;
        };
        let w = &mut run.world;
        if let Some(b) = w.boss {
            let d = &self.c.enemies[w.enemies[b].def];
            let last = d.phases.len().max(1) - 1;
            let sings = d.lullaby.is_some();
            let e = &mut w.enemies[b];
            if sings && e.floor_hp > 0 && w.song.is_none() {
                e.phase = last;
                e.state = crate::enemy::State::Rest(10);
                e.hp = e.floor_hp;
            }
        }
    }

    /// The name the HUD gives a node.
    pub fn name_of(&self, n: &crate::route::Node) -> String {
        self.node_name(n)
    }

    // ---- what the page asks for ----------------------------------------

    /// From the title: the sentencing on a first visit, else the Holding.
    pub fn begin(&mut self) {
        self.mode = if self.meta.seen_intro {
            Mode::Holding
        } else {
            Mode::Intro
        };
    }

    pub fn finish_intro(&mut self) {
        self.meta.seen_intro = true;
        self.mode = Mode::Holding;
    }

    /// The sentencing again, from the Holding.
    pub fn replay_intro(&mut self) {
        self.mode = Mode::Intro;
    }

    pub fn buy(&mut self, id: &str) -> bool {
        self.mode == Mode::Holding && self.meta.buy(&self.c, id).is_ok()
    }

    /// Chooses the gun runs carry in, in the Holding.
    pub fn carry(&mut self, id: &str) -> bool {
        self.mode == Mode::Holding && self.meta.choose_carry(&self.c, id)
    }

    pub fn start_run(&mut self) {
        self.start_run_at(0);
    }

    /// A run from the start of zone `zone`; a zone past the furthest one
    /// unlocked starts at zone one.
    pub fn start_run_at(&mut self, zone: usize) {
        if !matches!(self.mode, Mode::Holding | Mode::Dead | Mode::Won) {
            return;
        }
        let river = self.meta.on_river(&self.c);
        let branch = self.c.zones.get(zone).is_some_and(|z| !z.branch_from.is_empty());
        let zone = if (zone as i32 <= self.meta.unlocked && !branch)
            || (branch && river && self.meta.branch_open(&self.c, zone))
        {
            zone
        } else {
            0
        };
        let (build, gun_level) = self.meta.build(&self.c);
        let route = Route::build_for(&self.c, zone, self.meta.run_seed(), &Have::of(&build, gun_level));
        let player = Player::new(V3::ZERO, 0, build);
        let held = Held::new(self.meta.carried(&self.c), gun_level);
        let start = route.start();
        let world = self.world_for(&route, start, player, held, 0, false);
        self.run = Some(Run {
            zone,
            route,
            at: start,
            visited: vec![start],
            world,
            earned: 0,
            cleared: 0,
            counted: false,
            rush: false,
            practice: false,
            mission: false,
            briefed: true,
            river: false,
        });
        if let Some(run) = self.run.as_mut() {
            run.river = river;
        }
        self.reading = None;
        self.mode = Mode::Play;
        self.enter();
    }

    /// Chooses a timeline on the Holding's timeline and starts a run on it
    /// from zone `zone`: the Peak (`peak`) or the River (`river`), the river
    /// only once it is open.
    pub fn start_on(&mut self, line: &str, zone: usize) {
        if self.mode != Mode::Holding {
            return;
        }
        if line == "river" && self.meta.river_shut(&self.c).is_some() {
            return;
        }
        self.meta.timeline = if line == "river" {
            "river".into()
        } else {
            String::new()
        };
        self.start_run_at(zone);
    }

    /// Chooses the timeline runs follow past the house, in the Holding:
    /// the Peak (`peak`) or, once open, the River (`river`).
    pub fn choose_timeline(&mut self, line: &str) -> bool {
        if self.mode != Mode::Holding || (line == "river" && self.meta.river_shut(&self.c).is_some()) {
            return false;
        }
        self.meta.timeline = if line == "river" {
            "river".into()
        } else {
            String::new()
        };
        true
    }

    /// The Holding's timeline: the main line of years, and each zone off it
    /// forking from the zone it follows; where each stands on the panel (in
    /// percent), whether a run may start there, and why not.
    fn timeline_json(&self) -> Value {
        let c = &self.c;
        let m = &self.meta;
        let main = c.built_zones();
        let shut = m.river_shut(c);
        let river_on = m.on_river(c);
        let why = |r: &Refusal| match r {
            Refusal::Locked(boss) => c.say("why_locked").replace(
                "{boss}",
                &c.story.bosses.get(boss).map_or(boss.clone(), |b| b.name.clone()),
            ),
            Refusal::Requires(id) => c.say("why_requires").replace(
                "{name}",
                &c.upgrades
                    .iter()
                    .find(|u| &u.id == id)
                    .map_or(id.clone(), |u| u.name.clone()),
            ),
            _ => String::new(),
        };
        let x_of = |k: usize| TIMELINE_LEFT + TIMELINE_SPAN * k as i32 / main.saturating_sub(1).max(1) as i32;
        let step = TIMELINE_SPAN / main.saturating_sub(1).max(1) as i32;
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        // the branch's zones fork after the zone they follow
        let fork = c
            .zones
            .iter()
            .find(|z| z.built && !z.branch_from.is_empty())
            .and_then(|z| c.zones.iter().position(|f| f.id == z.branch_from));
        for k in 0..main {
            let z = &c.zones[k];
            let line = if fork.is_some_and(|f| k <= f) {
                "both"
            } else {
                "peak"
            };
            let start_ok = k as i32 <= m.unlocked;
            nodes.push(json!({
                "zone": k, "line": line, "year": z.year, "name": z.name,
                "boss": c.story.bosses.get(&z.boss).map(|b| b.name.clone()).unwrap_or_default(),
                "x": x_of(k), "y": TIMELINE_MAIN_Y,
                "open": start_ok, "beaten": m.beaten.contains(&z.boss), "sealed": false,
                "why": if start_ok { String::new() } else { c.say("why_not_reached") },
            }));
            if k > 0 {
                edges.push(json!({ "x1": x_of(k - 1), "y1": TIMELINE_MAIN_Y, "x2": x_of(k), "y2": TIMELINE_MAIN_Y, "line": line }));
            }
        }
        if let Some(f) = fork {
            let mut from = (x_of(f), TIMELINE_MAIN_Y);
            let mut n = 0;
            for (k, z) in c
                .zones
                .iter()
                .enumerate()
                .filter(|(_, z)| !z.branch_from.is_empty())
            {
                n += 1;
                let at = (x_of(f) + step * n, TIMELINE_RIVER_Y);
                let open = m.branch_open(c, k);
                nodes.push(json!({
                    "zone": k, "line": "river", "year": z.year, "name": z.name,
                    "boss": c.story.bosses.get(&z.boss).map(|b| b.name.clone()).unwrap_or_default(),
                    "x": at.0, "y": at.1,
                    "open": open, "beaten": m.beaten.contains(&z.boss), "sealed": !z.built,
                    "why": if !z.built {
                        c.say("sealed")
                    } else if let Some(r) = shut.as_ref() {
                        why(r)
                    } else if !open {
                        c.say("why_not_reached")
                    } else {
                        String::new()
                    },
                }));
                edges.push(json!({ "x1": from.0, "y1": from.1, "x2": at.0, "y2": at.1, "line": "river" }));
                from = at;
            }
        }
        let branch_words = c.zones.iter().find_map(|z| c.story.branches.get(&z.id));
        json!({
            "lines": [
                { "id": "peak", "name": branch_words.map(|b| b.main_timeline.clone()).unwrap_or_default(), "chosen": !river_on, "open": true, "why": "" },
                { "id": "river", "name": branch_words.map(|b| b.timeline.clone()).unwrap_or_default(), "chosen": river_on, "open": shut.is_none(), "why": shut.as_ref().map(why).unwrap_or_default() },
            ],
            "nodes": nodes,
            "edges": edges,
        })
    }

    /// A boss rush, once the game has been beaten: the first zone's boss,
    /// then each later zone's, the body healed between.
    pub fn start_rush(&mut self) {
        if !matches!(self.mode, Mode::Holding | Mode::Dead | Mode::Won) || !self.meta.rush_open() {
            return;
        }
        let (build, gun_level) = self.meta.build(&self.c);
        let route = Route::rush(0, self.meta.run_seed());
        let player = Player::new(V3::ZERO, 0, build);
        let held = Held::new(self.meta.carried(&self.c), gun_level);
        let world = self.world_for(&route, 0, player, held, 0, false);
        self.run = Some(Run {
            zone: 0,
            route,
            at: 0,
            visited: vec![0],
            world,
            earned: 0,
            cleared: 0,
            counted: false,
            rush: true,
            practice: false,
            mission: false,
            briefed: true,
            river: false,
        });
        self.reading = None;
        self.mode = Mode::Play;
        self.enter();
    }

    /// The lullaby alone, for testing: the last zone's boss arena, the god
    /// brought to its song as soon as it arrives, with the record's own
    /// body. Winning, dying or abandoning returns to the Holding and
    /// changes nothing on the record.
    pub fn practice_lullaby(&mut self) {
        let (build, gun_level) = self.meta.build(&self.c);
        self.practice_at(self.c.built_zones() - 1, build, gun_level);
    }

    /// Chumbles alone, for testing: downstairs, with everything bought
    /// (the test save's body and Carbine), from his bar `bar` (0 is the
    /// first, with his scream). Winning, dying or abandoning returns to
    /// the Holding and changes nothing on the record.
    pub fn practice_chumbles(&mut self, bar: usize) {
        let Some(home) = self.c.mission_zone() else {
            return;
        };
        let (build, gun_level) = Meta::everything(&self.c).build(&self.c);
        self.practice_at(home, build, gun_level);
        if let Some(run) = self.run.as_mut().filter(|r| r.practice && r.zone == home) {
            run.world.from_bar = bar;
        }
    }

    /// A practice: zone `zone`'s boss alone, with this body.
    fn practice_at(&mut self, zone: usize, build: crate::player::Build, gun_level: i32) {
        if !matches!(self.mode, Mode::Holding | Mode::Dead | Mode::Won) {
            return;
        }
        let route = Route::rush(zone, self.meta.run_seed());
        let player = Player::new(V3::ZERO, 0, build);
        let held = Held::new(self.meta.carried(&self.c), gun_level);
        let world = self.world_for(&route, 0, player, held, 0, false);
        self.run = Some(Run {
            zone,
            route,
            at: 0,
            visited: vec![0],
            world,
            earned: 0,
            cleared: 0,
            counted: false,
            rush: false,
            practice: true,
            mission: false,
            briefed: true,
            river: false,
        });
        self.reading = None;
        self.mode = Mode::Play;
        self.enter();
    }

    /// The last mission, once the Idiot God sleeps: the brief first, then
    /// a run from the crater through the fourth zone's boss in one go, its
    /// fights larger and remixed (new game plus), and then home.
    pub fn start_mission(&mut self) {
        if !matches!(self.mode, Mode::Holding | Mode::Dead | Mode::Won) || !self.meta.mission_open(&self.c) {
            return;
        }
        self.start_run_at(0);
        let Some(run) = self.run.as_mut() else {
            return;
        };
        run.mission = true;
        run.briefed = false;
        let (route, start) = (run.route.clone(), run.at);
        let player = run.world.player.clone();
        let held = run.world.held;
        let w = self.world_for(&route, start, player, held, 0, true);
        let run = self.run.as_mut().unwrap();
        run.world = w;
        self.mode = Mode::Interlude;
        self.enter();
    }

    /// The last mission, straight at home (its bedroom), for tools that
    /// drive the game (to see or measure Chumbles' house and fight).
    pub fn mission_home(&mut self) {
        let Some(home) = self.c.mission_zone() else {
            return;
        };
        self.start_mission();
        let Some(run) = self.run.as_mut() else {
            return;
        };
        run.briefed = true;
        run.zone = self.c.mission_break();
        self.mode = Mode::Interlude;
        self.continue_run();
        debug_assert_eq!(self.run.as_ref().map(|r| r.zone), Some(home));
    }

    /// The zone a run goes on to after zone `zone`'s boss: the next, but on
    /// the last mission home after the fourth.
    fn next_zone(&self, zone: usize, mission: bool, river: bool) -> usize {
        if river && !mission {
            if let Some(b) = self.c.branch_after(zone) {
                return b;
            }
        }
        if !self.c.zones[zone].branch_from.is_empty() {
            // the end of its timeline, for now
            return usize::MAX;
        }
        match self.c.mission_zone() {
            Some(home) if mission && zone == self.c.mission_break() => home,
            _ => zone + 1,
        }
    }

    /// Does zone `next` exist to be crossed into (not the last mission's)?
    fn leads_on(&self, next: usize) -> bool {
        self.c.zones.get(next).is_some_and(|z| z.built && !z.mission)
    }

    /// The boss rush's next boss after zone `zone`'s: the next zone's on
    /// the main line, then the Oak's once it has burnt.
    fn rush_next(&self, zone: usize) -> usize {
        if zone + 1 < self.c.built_zones() {
            return zone + 1;
        }
        self.c
            .zones
            .iter()
            .position(|z| z.built && !z.branch_from.is_empty() && self.meta.beaten.contains(&z.boss))
            .filter(|&b| b != zone && zone + 1 == self.c.built_zones())
            .unwrap_or(usize::MAX)
    }

    /// The end of a practice: back to the Holding, the record untouched.
    fn end_practice(&mut self) {
        self.run = None;
        self.reading = None;
        self.mode = Mode::Holding;
        self.build_scene();
    }

    /// The test save: a record with everything, in the Holding.
    pub fn test_save(&mut self) {
        if matches!(self.mode, Mode::Title | Mode::Intro | Mode::Holding) {
            self.meta = Meta::everything(&self.c);
            self.run = None;
            self.mode = Mode::Holding;
            self.build_scene();
        }
    }

    fn world_for(
        &self,
        route: &Route,
        at: usize,
        player: Player,
        held: Held,
        seq: u32,
        mission: bool,
    ) -> World {
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
        let mut w = World::new(
            &self.c,
            node,
            &exits,
            player,
            held,
            self.meta.run_seed() ^ at as u64,
        );
        w.seq = seq;
        if mission {
            w.mission = true;
            w.remix();
        }
        w
    }

    pub fn pause(&mut self) {
        if self.mode == Mode::Play {
            self.mode = Mode::Paused;
        }
    }

    pub fn resume(&mut self) {
        if matches!(self.mode, Mode::Paused | Mode::Map) {
            self.mode = Mode::Play;
            self.last_us = None;
            self.latched = 0;
        }
    }

    pub fn toggle_map(&mut self) {
        match self.mode {
            Mode::Play | Mode::Paused => self.mode = Mode::Map,
            Mode::Map => self.resume(),
            _ => {}
        }
    }

    /// Gives up the run: the court counts it as a death.
    pub fn abandon(&mut self) {
        if matches!(self.mode, Mode::Paused | Mode::Map) {
            if self.run.as_ref().is_some_and(|r| r.practice) {
                self.end_practice();
            } else {
                self.die();
            }
        }
    }

    /// From the interlude into the next zone: the body, its gun and its
    /// rewards carried, its health restored.
    pub fn continue_run(&mut self) {
        if self.mode != Mode::Interlude {
            return;
        }
        if self.run.as_ref().is_some_and(|r| !r.briefed) {
            // the last mission's brief read: into the crater
            let run = self.run.as_ref().unwrap();
            let name = self.node_name(&run.route.nodes[run.at]);
            let run = self.run.as_mut().unwrap();
            run.briefed = true;
            run.world.note("enter", name);
            self.mode = Mode::Play;
            self.enter();
            return;
        }
        let Some(mut run) = self.run.take() else {
            return;
        };
        run.earned += run.world.earned;
        let zone = self.next_zone(run.zone, run.mission, run.river);
        let have = Have::of(&run.world.player.build, run.world.held.level);
        let route = Route::build_for(&self.c, zone, self.meta.run_seed(), &have);
        let mut player = run.world.player.clone();
        player.health = player.build.max_health;
        let held = run.world.held;
        let seq = run.world.seq;
        let start = route.start();
        run.world = self.world_for(&route, start, player, held, seq, run.mission);
        run.route = route;
        run.zone = zone;
        run.at = start;
        run.visited = vec![start];
        run.counted = false;
        let name = self.node_name(&run.route.nodes[start]);
        run.world.note("enter", name);
        self.run = Some(run);
        self.crossing_count = None;
        self.mode = Mode::Play;
        self.enter();
    }

    pub fn to_holding(&mut self) {
        if matches!(self.mode, Mode::Dead | Mode::Won) {
            self.run = None;
            self.mode = Mode::Holding;
            self.build_scene();
        }
    }

    /// Wipes the record: a fresh defendant.
    pub fn reset(&mut self) {
        self.meta = Meta::default();
        self.run = None;
        self.mode = Mode::Title;
        self.build_scene();
    }

    pub fn saved(&self) -> String {
        self.meta.save()
    }

    /// A record with Clemency to spare, for trying the upgrades.
    pub fn grant(&mut self, clemency: i32) {
        self.meta.clemency += clemency;
    }

    // ---- the frame -----------------------------------------------------

    pub fn resize(&mut self, css_w: i32, css_h: i32, dpr_pct: i32) -> [i32; 2] {
        let (cw, ch) = (css_w.max(1), css_h.max(1));
        let mut w = cw as i64 * dpr_pct.clamp(25, 300) as i64 / 100;
        let mut h = ch as i64 * dpr_pct.clamp(25, 300) as i64 / 100;
        if w * h > MAX_PIXELS {
            let k = fx::isqrt((MAX_PIXELS * 1_000_000 / (w * h)) as u64) as i64;
            w = w * k / 1000;
            h = h * k / 1000;
        }
        self.css = (cw, ch);
        self.pixels = (w.max(1) as i32, h.max(1) as i32);
        self.draw();
        [self.pixels.0, self.pixels.1]
    }

    fn aspect(&self) -> i32 {
        fx::div(self.css.0 * ONE, self.css.1.max(1) * ONE)
    }

    /// One display frame: the page's clock in microseconds, the held
    /// actions, and the mouse movement in hundredths of a CSS pixel.
    pub fn advance(&mut self, now_us: i64, bits: u32, dx: i32, dy: i32) {
        let dt = match self.last_us {
            Some(last) => (now_us - last).clamp(0, MAX_FRAME_US),
            None => 0,
        };
        self.last_us = Some(now_us);
        self.menu_us += dt;
        if self.mode != Mode::Play {
            self.draw();
            return;
        }
        let focused = bits & bit("focus") != 0;
        self.latched |= bits & !self.prev_bits;
        self.prev_bits = bits;
        if !focused {
            self.latched = 0;
            self.draw();
            return;
        }
        let Some(run) = self.run.as_mut() else {
            return;
        };
        run.world.player.look(
            -dx * LOOK_PER_PIXEL / HUNDREDTHS,
            -dy * LOOK_PER_PIXEL / HUNDREDTHS,
        );
        self.acc += dt * TICKS_PER_SECOND as i64;
        let mut ran = 0;
        while self.acc >= TICK_UNITS && ran < MAX_TICKS_PER_FRAME {
            let ctl = controls(bits, self.latched);
            self.latched = 0;
            self.tick(&ctl);
            self.acc -= TICK_UNITS;
            ran += 1;
            if self.mode != Mode::Play {
                break;
            }
        }
        if ran == MAX_TICKS_PER_FRAME {
            self.acc = self.acc.min(TICK_UNITS - 1);
        }
        self.alpha = (self.acc * ONE as i64 / TICK_UNITS) as i32;
        self.draw();
    }

    /// One tick of play, and what follows from how it ended.
    pub fn tick(&mut self, ctl: &Controls) {
        let Some(run) = self.run.as_mut() else {
            return;
        };
        let practice = run.practice;
        let cam = play_camera(&run.world, ONE);
        run.world.tick(&self.c, ctl, cam.eye, cam.fwd);
        if practice {
            match run.world.outcome {
                Outcome::Playing => self.lull(),
                _ => self.end_practice(),
            }
            return;
        }
        let Some(run) = self.run.as_mut() else {
            return;
        };
        let node = &run.route.nodes[run.at];
        let rules = &self.c.zones[run.zone].route;
        if run.world.cleared && !run.counted && node.kind == NodeKind::Fight {
            run.counted = true;
            run.cleared += 1;
            run.earned += if node.detour {
                rules.clemency_detour
            } else {
                rules.clemency_main
            };
        }
        match run.world.outcome {
            Outcome::Playing => {}
            Outcome::Through(to) => self.go(to),
            Outcome::Died => self.die(),
            Outcome::Won => {
                let run = self.run.as_mut().unwrap();
                let boss = self.c.zones[run.zone].boss.clone();
                self.meta.beat(&boss);
                run.earned += self.c.zones[run.zone].route.clemency_boss;
                run.cleared += 1;
                let (mission, zone, river, rush) = (run.mission, run.zone, run.river, run.rush);
                let next = if rush {
                    self.rush_next(zone)
                } else {
                    self.next_zone(zone, mission, river)
                };
                let leads_on = self.leads_on(next);
                let run = self.run.as_mut().unwrap();
                if mission {
                    if Some(run.zone) == self.c.mission_zone() {
                        // Chumbles is fed: the last mission is won
                        let (total, cleared) = (run.total(), run.cleared);
                        let banked = self.meta.record_mission(&self.c, total, cleared);
                        self.last = (banked, cleared);
                        self.win_count = None;
                        self.mode = Mode::Won;
                        return;
                    }
                    // on, through the interlude, with nothing unlocked
                    self.meta.bosses_beaten += 1;
                    run.world.outcome = Outcome::Playing;
                    self.mode = Mode::Interlude;
                    return;
                }
                if run.rush {
                    if leads_on {
                        // straight on to the next boss, healed
                        run.earned += run.world.earned;
                        let mut player = run.world.player.clone();
                        player.health = player.build.max_health;
                        let held = run.world.held;
                        let seq = run.world.seq;
                        let route = Route::rush(next, self.meta.run_seed() ^ next as u64);
                        let w = self.world_for(&route, 0, player, held, seq, false);
                        let run = self.run.as_mut().unwrap();
                        run.world = w;
                        run.route = route;
                        run.zone = next;
                        run.visited = vec![0];
                        self.enter();
                        return;
                    }
                    let (total, cleared) = (run.total(), run.cleared);
                    let banked = self.meta.record_rush(&self.c, total, cleared);
                    self.last = (banked, cleared);
                    self.win_count = None;
                    self.mode = Mode::Won;
                    return;
                }
                if leads_on {
                    // the next zone: the interlude first
                    let year = self.c.zones[next].year.clone();
                    self.crossing_count = self.meta.record_crossing(&self.c, next, &year);
                    run.world.outcome = Outcome::Playing;
                    self.mode = Mode::Interlude;
                    return;
                }
                let (total, cleared) = (run.total(), run.cleared);
                if !self.c.zones[run.zone].branch_from.is_empty() {
                    // the end of the river's timeline, for now
                    let banked = self.meta.record_river(&self.c, total, cleared);
                    self.last = (banked, cleared);
                    self.win_count = None;
                    self.mode = Mode::Won;
                    return;
                }
                let year = self.c.zones[run.zone].year.clone();
                let (banked, newest) = self.meta.record_win(&self.c, total, cleared, &year);
                self.last = (banked, cleared);
                self.win_count = newest;
                self.mode = Mode::Won;
            }
        }
    }

    fn go(&mut self, to: usize) {
        let Some(mut run) = self.run.take() else {
            return;
        };
        run.earned += run.world.earned;
        let player = run.world.player.clone();
        let held = run.world.held;
        let seq = run.world.seq;
        run.world = self.world_for(&run.route, to, player, held, seq, run.mission);
        let name = self.node_name(&run.route.nodes[to]);
        run.world.note("enter", name);
        // the first run to reach the zone's hardest fights hears a count
        if run.route.nodes[to].tier >= FAR_TIER && !self.meta.reached_far {
            self.meta.reached_far = true;
            let year = self.c.zones[run.zone].year.clone();
            let line = self.meta.read_next(&self.c, &year);
            self.story_seq += 1;
            self.story = Some((self.story_seq, line));
        }
        run.at = to;
        run.visited.push(to);
        run.counted = false;
        self.run = Some(run);
        self.enter();
    }

    fn die(&mut self) {
        let Some(run) = self.run.as_ref() else {
            return;
        };
        // a fight left unfinished still counts for the enemies it cost
        let node = &run.route.nodes[run.at];
        let z = &self.c.zones[run.zone];
        let partial = match node.encounter {
            Some(e) if node.kind == NodeKind::Fight && !run.counted => {
                let total: i32 = z.encounters[e].waves.iter().map(|w| w.len() as i32).sum();
                let per = if node.detour {
                    z.route.clemency_detour
                } else {
                    z.route.clemency_main
                };
                per * run.world.kills.min(total) / total.max(1)
            }
            _ => 0,
        };
        let (total, cleared) = (run.total() + partial, run.cleared);
        let year = z.year.clone();
        let reading = self.meta.record_death(&self.c, total, cleared, &year);
        self.last = (reading.banked, cleared);
        self.reading = Some(reading);
        self.mode = Mode::Dead;
    }

    /// A new arena: rebuild the static scene, restart the clock.
    fn enter(&mut self) {
        self.build_scene();
        self.last_us = None;
        self.acc = 0;
        self.latched = 0;
        self.alpha = ONE;
        self.draw();
    }

    /// The zone the scene is in: the run's, or zone one's for the menus.
    fn scene_zone(&self) -> &crate::content::Zone {
        &self.c.zones[self.run.as_ref().map_or(0, |r| r.zone)]
    }

    /// The scene the run is lit by: its section's own, else its zone's.
    fn scene(&self) -> &crate::content::Scene {
        let z = self.scene_zone();
        self.run
            .as_ref()
            .and_then(|r| r.route.nodes.get(r.at))
            .and_then(|n| z.scenes.get(&n.archetype))
            .unwrap_or(&z.scene)
    }

    fn build_scene(&mut self) {
        let ink = self.c.pal.c(&self.scene().ink);
        let arena = match &self.run {
            Some(r) => &r.world.arena,
            None => &self.hub,
        };
        let mut b = Batch::new(ink);
        for s in &arena.shapes {
            b.shape(s);
        }
        let (values, draws) = b.finish(render::STATIC_BUFFER);
        self.static_values = values;
        self.static_draws = draws;
        self.scene_version += 1;
    }

    fn draw(&mut self) {
        let mut b = Batch::new(self.c.pal.c(&self.scene().ink));
        let clock = (self.menu_us * TICKS_PER_SECOND as i64 / 1_000_000) as i32;
        let in_run = matches!(
            self.mode,
            Mode::Play | Mode::Paused | Mode::Map | Mode::Interlude | Mode::Dead | Mode::Won
        );
        // the head the scenery is cut away in front of, and the floor under it
        let mut focus = None;
        match (&self.run, in_run) {
            (Some(run), true) => {
                self.camera = play_camera(&run.world, self.alpha);
                let p = &run.world.player;
                let feet = p.prev_pos.lerp(p.pos, self.alpha);
                let head = feet.add(v3(0, CAMERA_UP, 0));
                let body = self.camera.eye.dist(head) > CAMERA_HIDE_BODY;
                if matches!(self.mode, Mode::Play | Mode::Paused | Mode::Map) {
                    focus = Some((feet.add(v3(0, PLAYER_HEIGHT * 9 / 10, 0)), feet.y));
                }
                let cam = self.camera;
                draw::world(&mut b, &self.c, &run.world, self.alpha, clock, body, &cam);
                if matches!(self.mode, Mode::Dead | Mode::Won | Mode::Interlude) {
                    // the camera drifts up and away while the court reads
                    let up = fx::mul(int(1), (clock % 600) * ONE / 120);
                    self.camera.eye = self.camera.eye.add(v3(0, up, 0));
                }
            }
            _ => {
                // the menus: the crater at dusk, the Defendant standing in it,
                // the camera circling slowly
                let a = clock * fx::TURN / (60 * 70);
                let centre = v3(0, int(1), int(8));
                draw::standing(
                    &mut b,
                    &self.c,
                    v3(0, 0, int(8)),
                    a + fx::HALF,
                    self.c.gun("carbine"),
                );
                let eye = centre.add(v3(
                    fx::mul(fx::sin(a), int(9)),
                    int(3),
                    fx::mul(fx::cos(a), int(9)),
                ));
                let look = centre.add(v3(0, int(1), 0)).sub(eye).norm();
                self.camera = Camera {
                    eye,
                    fwd: look,
                    fov: FOV,
                };
            }
        }
        let (values, dyn_draws) = b.finish(render::DYNAMIC_BUFFER);
        let mut draws = Vec::new();
        for pass in render::PASS_ORDER {
            draws.extend_from_slice(&self.static_draws[pass as usize]);
            draws.extend_from_slice(&dyn_draws[pass as usize]);
        }
        self.values = values;
        self.draws = draws;
        self.view = self.camera.values(self.aspect());
        // the see-through focus: the head and the floor; at the eye itself
        // (nothing is cut) in the menus
        let (at, floor) = focus.unwrap_or((self.camera.eye, self.camera.eye.y));
        self.view.extend([at.x, at.y, at.z, floor]);
        // the summit's melt, stopped time, how far space is bent and the
        // clock (seconds, wrapped), and a bubble of stopped time, for the
        // shader
        let (melt, stopped, bubble) = match (&self.run, in_run) {
            (Some(r), true) => (
                r.world.melt(),
                if r.world.stop.is_some() { ONE } else { 0 },
                r.world.bubble(),
            ),
            _ => (0, 0, [0; 4]),
        };
        let warp = self.scene().warp_pct * ONE / 100;
        let seconds = ((self.menu_us % 1_000_000_000) * ONE as i64 / 1_000_000) as i32;
        self.view.extend([melt, stopped, warp, seconds]);
        self.view.extend(bubble);
        let _ = self.hub_seed;
    }

    pub fn view(&self) -> Vec<i32> {
        self.view.clone()
    }

    pub fn instances(&self) -> Vec<i32> {
        self.values.clone()
    }

    pub fn static_instances(&self) -> Vec<i32> {
        self.static_values.clone()
    }

    pub fn draws(&self) -> Vec<u32> {
        self.draws.clone()
    }

    pub fn scene_version(&self) -> u32 {
        self.scene_version
    }

    pub fn scene_uniforms(&self) -> Vec<i32> {
        let s = self.scene();
        let pal = &self.c.pal;
        let l = v3(-ONE * 4 / 10, ONE * 8 / 10, ONE * 45 / 100).norm();
        let mut u = vec![l.x, l.y, l.z];
        u.extend(pal.c(&s.sky));
        u.extend(pal.c(&s.fog));
        u.extend([int(s.fog_near_m), int(s.fog_far_m)]);
        u.extend(pal.c(&s.light));
        u.extend(pal.c(&s.window_dark));
        u.extend(pal.c(&s.ink));
        u.extend([SEE_NEAR, SEE_WIDE_LENS, SEE_WIDE_HEAD, SEE_SHORT]);
        u.push(SEE_SOLID_PCT * ONE / 100);
        u
    }

    pub fn clear_color(&self) -> Vec<i32> {
        self.c.pal.c(&self.scene().fog).to_vec()
    }

    // ---- the HUD ---------------------------------------------------------

    /// The Codex: controls, guns, enemies, upgrades and rules, in numbers.
    pub fn codex_json(&self) -> String {
        crate::codex::codex_json(&self.c)
    }

    pub fn hud_json(&self) -> String {
        let mut h = json!({ "mode": self.mode });
        let Some(run) = &self.run else {
            return h.to_string();
        };
        let w = &run.world;
        let p = &w.player;
        let c = &self.c;
        let node = &run.route.nodes[run.at];
        let gun = &c.guns[w.held.gun];
        let gl = gun.level(w.held.level);
        let aspect = self.aspect();
        let pct = |a: i32, b: i32| a.max(0) * 100 / b.max(1);
        // the boss's bar, or a mini-boss's: a tank's shows what is left of
        // its phase's weak points, or of the time to outlast it
        let boss = w
            .boss
            .or(w.elite.filter(|&k| w.enemies[k].alive()))
            .filter(|_| w.ending == 0)
            .map(|b| {
                let e = &w.enemies[b];
                let d = &c.enemies[e.def];
                let words = c.story.bosses.get(&d.id);
                // a boss with a bar for each phase: its phase's share of
                // its health; the lullaby's bar is the sleep
                if let Some(w) = words.filter(|w| !w.bars.is_empty()) {
                    let n = d.phases.len();
                    let (bar, pct, title) = match (&run.world.song, d.lullaby.as_ref()) {
                        (Some(song), Some(l)) => (
                            n,
                            song.sleep(l) * 100 / ONE,
                            c.say("song_verse")
                                .replace("{n}", &(song.verse + 1).min(l.verses.len()).to_string())
                                .replace("{max}", &l.verses.len().to_string()),
                        ),
                        _ => {
                            let k = e.phase.min(n - 1);
                            let top = d.phases[k].from_pct;
                            let floor = d.phases.get(k + 1).map_or(0, |p| p.from_pct);
                            let pct = (e.health_pct() - floor).max(0) * 100 / (top - floor).max(1);
                            (k, pct, w.title.clone())
                        }
                    };
                    let bars = n + d.lullaby.is_some() as usize;
                    // a Husk over the health: the boss's own, or the Summons'
                    // the Oak has called
                    let shield = run
                        .world
                        .course
                        .as_ref()
                        .and_then(|k| k.summons)
                        .map(|s| &run.world.enemies[s])
                        .filter(|s| s.alive())
                        .or(Some(e))
                        .filter(|s| s.husk_max > 0);
                    let husk = shield.map_or(-1, |s| s.husk * 100 / s.husk_max.max(1));
                    let staggered = shield.is_some_and(|s| s.stagger > 0);
                    return json!({
                        "husk": husk,
                        "staggered": staggered,
                        "name": w.name,
                        "title": title,
                        "pct": pct,
                        "phase": bar + 1,
                        "timer": w.bars.get(bar).cloned().unwrap_or_default(),
                        "bars": bars,
                        "bars_left": bars - bar,
                        "sleep": run.world.song.is_some(),
                    });
                }
                let (pct, timer) = match e.tank.as_ref() {
                    Some(t) if t.survive > 0 => {
                        let total = d.phases[e.phase].survive.max(1);
                        let secs = (t.survive + TICKS_PER_SECOND - 1) / TICKS_PER_SECOND;
                        (
                            t.survive * 100 / total,
                            c.say("hud_survive").replace("{n}", &secs.to_string()),
                        )
                    }
                    Some(t) => {
                        let total: i32 = d.phases[e.phase].weak.iter().map(|w| w.hp).sum();
                        let left: i32 = t.weak.iter().map(|h| (*h).max(0)).sum();
                        (left * 100 / total.max(1), String::new())
                    }
                    None => (e.health_pct(), String::new()),
                };
                json!({
                    "name": words.map(|x| x.name.clone()).unwrap_or_default(),
                    "title": words.map(|x| x.title.clone()).unwrap_or_default(),
                    "pct": pct,
                    "phase": e.phase + 1,
                    "timer": timer,
                    "bars": 0,
                    "bars_left": 0,
                    "sleep": false,
                })
            });
        let gates: Vec<Value> = w
            .gates
            .iter()
            // a gate says where it leads only once the arena is clear
            .filter(|_| w.cleared)
            .filter_map(|g| {
                let to = g.to?;
                let at = g.pos.add(v3(0, int(9), 0));
                let (x, y) = self.camera.screen(aspect, at)?;
                if !(0..=100 * ONE).contains(&x) || !(0..=100 * ONE).contains(&y) {
                    return None;
                }
                let dest = &run.route.nodes[to];
                Some(json!({
                    "x": x / ONE,
                    "y": y / ONE,
                    "main": g.main,
                    "open": w.cleared,
                    "label": self.node_name(dest),
                    "reward": self.prize_label(dest),
                    "path": if g.main { c.say("gate_main") } else { c.say("gate_side") },
                }))
            })
            .collect();
        let prompt = w.offer.and_then(|k| match w.pickups[k].kind {
            _ if w.pickups[k].group != 0 => Some(
                c.say("prompt_choose")
                    .replace("{thing}", &self.pickup_label(w.pickups[k].kind)),
            ),
            PickupKind::Gun(g, l) => Some(
                c.say("prompt_take")
                    .replace("{gun}", &format!("{} {}", c.guns[g].name, roman(l)))
                    .replace("{held}", &format!("{} {}", gun.name, roman(w.held.level))),
            ),
            _ => None,
        });
        // hints show for a few seconds, then get out of the way
        let fresh = |since: i32| w.tick - since < HINT_TICKS;
        let house = c.zones[run.zone].id == "glass_house";
        let god = c.zones[run.zone].id == "idiot_god";
        let learning = w.song.as_ref().is_some_and(|s| {
            s.asleep == 0 && matches!(s.part, crate::song::Part::Intro | crate::song::Part::Listen)
        });
        let home = c.zones[run.zone].mission;
        let first = run.mission && run.zone == 0 && run.at == run.route.start();
        let course = w.course.is_some();
        // on or by rotten timber: how it is broken
        let rotten = w.course.as_ref().is_some_and(|k| {
            k.rotten.iter().any(|&r| {
                let d = &w.arena.level.dynamic[r];
                let near = v3(p.pos.x - d.solid.centre().x, 0, p.pos.z - d.solid.centre().z).len_xz();
                d.on && near < d.solid.size().x.max(d.solid.size().z) / 2 + int(4)
                    && (p.pos.y - d.solid.max.y).abs() < int(6)
            })
        });
        // by the flagship while the dragon is on its chain: how it is freed
        let chained = w
            .course
            .as_ref()
            .and_then(|k| k.chain.as_ref())
            .is_some_and(|ch| !ch.free && p.pos.sub(ch.mast).len_xz() < DRAGON_HINT_NEAR);
        let hint = match node.kind {
            NodeKind::Arrival | NodeKind::Fight if rotten => c.say("hint_rotten"),
            NodeKind::Boss if learning => c.say("hint_song"),
            NodeKind::Boss if course && fresh(0) && w.course.as_ref().is_some_and(|k| k.charge.is_some()) => {
                c.say("hint_eraser")
            }
            NodeKind::Boss if course && fresh(0) && w.course.as_ref().is_some_and(|k| k.oak.is_some()) => {
                c.say("hint_oak")
            }
            NodeKind::Arrival | NodeKind::Fight if chained => c.say("hint_dragon"),
            NodeKind::Arrival | NodeKind::Fight if course && fresh(0) && node.archetype == "fleet" => {
                c.say("hint_fleet")
            }
            NodeKind::Arrival | NodeKind::Fight if course && fresh(0) && node.archetype == "maelstrom" => {
                c.say("hint_maelstrom")
            }
            NodeKind::Arrival | NodeKind::Fight if course && fresh(0) && node.archetype == "hurricane" => {
                c.say("hint_hurricane")
            }
            NodeKind::Arrival | NodeKind::Fight if course && fresh(0) && node.archetype == "halls" => {
                c.say("hint_halls")
            }
            NodeKind::Arrival | NodeKind::Fight if course && fresh(0) => c.say("hint_course"),
            NodeKind::Arrival | NodeKind::Fight if course => String::new(),
            NodeKind::Arrival if first && fresh(0) => c.say("hint_mission"),
            NodeKind::Arrival if home => c.say("hint_cat"),
            NodeKind::Arrival if god && fresh(0) => c.say("hint_gullet"),
            NodeKind::Arrival if house => c.say("hint_vestibule"),
            NodeKind::Boss if house && w.tick > secs(4, 1) && fresh(secs(4, 1)) => c.say("hint_tank"),
            NodeKind::Arrival if !node.cache.is_empty() => c.say("hint_landing"),
            NodeKind::Arrival => c.say("hint_arrival"),
            NodeKind::Rest => c.say("hint_rest"),
            NodeKind::Fight if w.cleared && fresh(w.cleared_at) => c.say("hint_cleared"),
            NodeKind::Fight if !w.cleared && self.meta.runs < 2 && fresh(0) => c.say("hint_fight"),
            _ => String::new(),
        };
        let reticle = if w.aim_enemy.is_some() {
            "enemy"
        } else if p.build.grapple && w.hook_target.is_some() && p.grapple_cooldown == 0 {
            "hook"
        } else {
            ""
        };
        let notes: Vec<Value> = w
            .notes
            .iter()
            .map(|n| json!({ "seq": n.seq, "sound": n.sound, "text": n.text }))
            .collect();
        let enemies = w.alive_enemies() + w.waves_left.iter().map(|v| v.len()).sum::<usize>();
        // the boss has its own bar; every other enemy gets a marker, and
        // so does each of a tank's weak points
        let mut foes: Vec<Value> = w
            .enemies
            .iter()
            .enumerate()
            .filter(|(i, e)| e.hurtable() && Some(*i) != w.boss)
            .map(|(_, e)| {
                let threat = matches!(
                    e.state,
                    crate::enemy::State::Windup(_) | crate::enemy::State::Casting(_)
                );
                let mut m = self.marker(e.pos, c.enemies[e.def].radius, e.health_pct());
                m["threat"] = json!(threat);
                m["colour"] = json!(hex(e.next_colour(c).map_or(c.pal.c("magenta"), |n| c.pal.c(n))));
                m
            })
            .collect();
        for b in w.boss.into_iter().chain(w.elite) {
            let e = &w.enemies[b];
            let d = &c.enemies[e.def];
            if let Some(t) = e.tank.as_ref().filter(|_| e.hurtable()) {
                for (at, r, k) in crate::tank::weak_points(e, d) {
                    let total = d.phases[e.phase].weak[k].hp.max(1);
                    let mut m = self.marker(at, r, t.weak[k].max(0) * 100 / total);
                    m["threat"] = json!(false);
                    m["weak"] = json!(true);
                    m["colour"] = json!(hex(c.pal.c("tank_weak")));
                    foes.push(m);
                }
            }
        }
        let gl_saw = gl.saw;
        // the arc toward the last hit's source, fading
        let hit = (w.hit_age < HIT_ARC_TICKS).then(|| {
            let m = self.marker(w.hit_from, 0, 100);
            json!({ "angle": m["angle_any"], "fade": 100 - w.hit_age * 100 / HIT_ARC_TICKS })
        });
        // a ground wave about to reach a body standing in its path
        let jump_cue = w.waves.iter().any(|wv| {
            let d = p.pos.sub(wv.centre).len_xz();
            d > wv.radius && d - wv.radius <= wv.speed * JUMP_CUE_TICKS && p.pos.y < wv.centre.y + WAVE_HEIGHT
        });
        let mut perks = Vec::new();
        if p.build.pierce > 0 {
            perks.push(format!("{} +{}", c.say("reward_pierce"), p.build.pierce));
        }
        if p.build.pulse {
            perks.push(c.say("reward_pulse"));
        }
        if p.build.mend {
            perks.push(c.say("reward_mend"));
        }
        if p.build.trigger > 0 {
            perks.push(format!("{} ×{}", c.say("reward_trigger"), p.build.trigger));
        }
        if p.build.plating > 0 {
            perks.push(format!(
                "{} ×{}",
                c.say("reward_plating"),
                p.build.plating.min(PLATING_MAX)
            ));
        }
        if p.build.contempt > 0 {
            perks.push(format!("{} ×{}", c.say("reward_contempt"), p.build.contempt));
        }
        let story = self
            .story
            .as_ref()
            .map(|(seq, text)| json!({ "seq": seq, "text": text, "heading": c.say("story_heading") }));
        h = json!({
            "mode": self.mode,
            "health": p.health,
            "max_health": p.build.max_health,
            "health_pct": pct(p.health, p.build.max_health),
            "low": p.health * 100 < p.build.max_health * 30,
            "hurt": (100 - p.hurt_age * 100 / 30).max(0),
            "jumps": p.build.jumps,
            "jumps_left": (p.build.jumps - p.jumps_used).max(0),
            "dashes": p.build.dash_charges,
            "dash_left": p.dash_left,
            "dash_pct": if p.dash_left < p.build.dash_charges { pct(p.dash_timer, DASH_RECHARGE) } else { 100 },
            "speed_pct": p.build.speed_pct,
            "gun": gun.name,
            "gun_id": gun.id,
            "gun_level": roman(w.held.level),
            "gun_trait": crate::codex::gun_summary(gl),
            "gun_colour": hex(c.pal.c(&gun.colour)),
            "alt": c.say("hud_alt").replace("{name}", &gun.alt.name),
            "grabbing": w.grabbed.is_some(),
            "stop": p.build.stop_ticks > 0,
            "stop_ready": p.build.stop_ticks > 0 && p.stop_cooldown == 0,
            "stopped": w.stop.map(|s| c.say("hud_stopped").replace("{n}", &((s.left + TICKS_PER_SECOND - 1) / TICKS_PER_SECOND).to_string())).unwrap_or_default(),
            "rounds": if gl.mag > 0 {
                let mut t = c.say("hud_rounds").replace("{n}", &w.held.rounds.min(gl.mag).to_string()).replace("{max}", &gl.mag.to_string());
                if w.held.reloading(gl.mag) {
                    t = format!("{t} · {}", c.say("hud_loading"));
                }
                t
            } else {
                String::new()
            },
            "alt_ready": w.held.alt_cooldown == 0,
            "grapple": p.build.grapple,
            "grapple_ready": p.build.grapple && p.grapple_cooldown == 0,
            "parry": p.build.parry,
            "parry_ready": p.build.parry && p.parry_cooldown == 0,
            "parrying": p.parry_window > 0,
            "recess": p.recess_left > 0,
            "pound": p.build.pound > 0,
            "pound_ready": p.build.pound > 0 && p.pound_cooldown == 0 && !p.pounding,
            "pounding": p.pounding,
            // the gun's own trait: blades in hand, the spin, the orbs struck
            "saws": if let Some(s) = gl_saw {
                c.say("hud_saws").replace("{n}", &(s.saws - w.held.saws_out).max(0).to_string()).replace("{max}", &s.saws.to_string())
            } else if gl.spin.is_some() {
                c.say("hud_spin").replace("{n}", &(w.held.spin * 100 / ONE).to_string())
            } else if gl.erase_ward > 0 {
                c.say("hud_struck").replace("{n}", &w.struck.to_string())
            } else {
                String::new()
            },
            "boss": boss,
            "zone": c.zones[run.zone].name,
            "year": c.zones[run.zone].year,
            "wall": p.build.wall,
            "wall_ready": p.build.wall && p.wall_cooldown == 0,
            "walling": p.wall_left > 0,
            "node": self.node_name(node),
            "enemies": enemies,
            "cleared": w.cleared,
            "clemency": run.total(),
            "gates": gates,
            "prompt": prompt,
            "hint": hint,
            "reticle": reticle,
            "notes": notes,
            "foes": foes,
            "hit": hit,
            "jump_cue": jump_cue,
            "jump_text": c.say("jump_cue"),
            "perks": perks,
            "pulse_ready": p.build.pulse && w.pulse_cooldown == 0,
            "story": story,
            "song": self.song_hud(w),
        });
        h.to_string()
    }

    /// The lullaby's staff: the verse's notes scrolling toward the line,
    /// each with its key and how it was played, and what part the song is in.
    fn song_hud(&self, w: &World) -> Value {
        let c = &self.c;
        let (Some(song), Some(b)) = (&w.song, w.boss) else {
            return Value::Null;
        };
        let Some(l) = c.enemies[w.enemies[b].def].lullaby.as_ref() else {
            return Value::Null;
        };
        use crate::song::{Judge, Part};
        let part = match song.part {
            Part::Intro => "intro",
            Part::Listen => "listen",
            Part::Play => "play",
            Part::Finale => "finale",
            Part::Asleep => "asleep",
        };
        let label = c.say(&format!("song_{part}"));
        let verse = &l.verses[song.verse.min(l.verses.len() - 1)];
        let first = crate::song::Song::start(l);
        let span = 100 - STAFF_LINE_PCT - STAFF_END_PCT;
        let notes: Vec<Value> = if matches!(song.part, Part::Listen | Part::Play) {
            verse
                .notes
                .iter()
                .enumerate()
                .filter_map(|(k, &(at, key))| {
                    let ahead = first + at - song.clock;
                    // gone well past the line, or not yet on the staff
                    if ahead < -l.good * 2 || ahead > STAFF_AHEAD {
                        return None;
                    }
                    let state = if song.part == Part::Listen {
                        "ghost"
                    } else {
                        match song.judged.get(k) {
                            Some(Judge::Great) => "great",
                            Some(Judge::Good) => "good",
                            Some(Judge::Miss) => "miss",
                            _ => "wait",
                        }
                    };
                    Some(json!({
                        "x": STAFF_LINE_PCT + ahead * span / STAFF_AHEAD,
                        "lane": key,
                        "key": c.say(&format!("song_key_{key}")),
                        "state": state,
                    }))
                })
                .collect()
        } else if song.part == Part::Finale {
            // the last verse once more, played for you
            let mut out = Vec::new();
            if let Some(v) = l.verses.last() {
                for &(n, key) in &v.notes {
                    let ahead = n - (song.clock - 1);
                    if (-l.good * 2..=STAFF_AHEAD).contains(&ahead) {
                        out.push(json!({
                            "x": STAFF_LINE_PCT + ahead * span / STAFF_AHEAD,
                            "lane": key,
                            "key": c.say(&format!("song_key_{key}")),
                            "state": if ahead <= 0 { "great" } else { "ghost" },
                        }));
                    }
                }
            }
            out
        } else {
            Vec::new()
        };
        let keys: Vec<String> = (0..crate::content::NOTE_KEYS.len())
            .map(|k| c.say(&format!("song_key_{k}")))
            .collect();
        json!({
            "part": part,
            "label": label,
            "line": STAFF_LINE_PCT,
            "notes": notes,
            "keys": keys,
            "words": { "great": c.say("song_great"), "good": c.say("song_good"), "miss": c.say("song_miss") },
        })
    }

    /// Where to mark an enemy on screen: over it, or at the screen's edge
    /// pointing the way to it, with the angle the arrow turns (degrees,
    /// clockwise from pointing right).
    fn marker(&self, at: V3, radius: i32, pct: i32) -> Value {
        let cam = &self.camera;
        let right = cam.fwd.cross(V3::UP).norm();
        let up = right.cross(cam.fwd);
        let rel = at.sub(cam.eye);
        // the way to it across the screen, even when it is on screen: from
        // its side and depth, so something behind points down the screen
        let (sx, sy) = (rel.dot(right), -rel.dot(up).max(0) + (-rel.dot(cam.fwd)).max(0));
        let a = fx::atan2(sy, sx);
        let deg = (a as i64 * 360 / fx::TURN as i64) as i32;
        let over = at.add(v3(0, radius + ONE / 2, 0));
        let screen = cam.screen(self.aspect(), over);
        if let Some((x, y)) = screen {
            if (0..=100 * ONE).contains(&x) && (0..=100 * ONE).contains(&y) {
                return json!({ "x": x / ONE, "y": y / ONE, "edge": false, "pct": pct, "hurt": pct < 100, "angle": 0, "angle_any": deg });
            }
        }
        json!({
            "x": 50 + fx::mul(fx::cos(a), int(44)) / ONE,
            "y": 50 + fx::mul(fx::sin(a), int(40)) / ONE,
            "edge": true,
            "pct": pct,
            "hurt": pct < 100,
            "angle": deg,
            "angle_any": deg,
        })
    }

    fn node_name(&self, n: &crate::route::Node) -> String {
        let key = match n.kind {
            NodeKind::Arrival => "arrival",
            NodeKind::Rest => "rest",
            NodeKind::Boss => "boss",
            NodeKind::Fight => n.archetype.as_str(),
        };
        self.c.zones[n.zone].place_name(key)
    }

    /// The level of the gun in hand, which a gun offered stands above.
    fn held_level(&self) -> i32 {
        self.run.as_ref().map_or(1, |r| r.world.held.level)
    }

    /// What a node holds, in words: its reward, a choice of two, or sealed.
    fn prize_label(&self, n: &Node) -> String {
        let Some(r) = n.reward else {
            return String::new();
        };
        if n.sealed {
            return self.c.say("reward_sealed");
        }
        match n.extra.first() {
            Some(&x) if n.choose => self
                .c
                .say("reward_choice")
                .replace("{a}", &self.reward_label(r))
                .replace("{b}", &self.reward_label(x)),
            _ => self.reward_label(r),
        }
    }

    fn prize_icon(n: &Node) -> &'static str {
        if n.sealed {
            return "sealed";
        }
        n.reward.map(Game::reward_icon).unwrap_or("")
    }

    pub fn reward_label(&self, r: Reward) -> String {
        match r {
            Reward::Gun(g, step) => format!(
                "{} {}",
                self.c.guns[g].name,
                roman((self.held_level() + step).min(crate::balance::MAX_GUN_LEVEL))
            ),
            Reward::Contempt => self.c.say("reward_contempt"),
            Reward::Purse(n) => self.c.say("reward_purse").replace("{n}", &n.to_string()),
            Reward::Jump => self.c.say("reward_jump"),
            Reward::Health => self.c.say("reward_health"),
            Reward::Speed => self.c.say("reward_speed"),
            Reward::Pierce => self.c.say("reward_pierce"),
            Reward::Pulse => self.c.say("reward_pulse"),
            Reward::Mend => self.c.say("reward_mend"),
            Reward::Trigger => self.c.say("reward_trigger"),
            Reward::Plating => self.c.say("reward_plating"),
        }
    }

    /// What a pickup is, in words, for the prompt.
    fn pickup_label(&self, k: PickupKind) -> String {
        let c = &self.c;
        match k {
            PickupKind::Heal(n) => c.say("prompt_heal").replace("{n}", &n.to_string()),
            PickupKind::MaxHealth(_) => c.say("reward_health"),
            PickupKind::Speed(_) => c.say("reward_speed"),
            PickupKind::Jump => c.say("reward_jump"),
            PickupKind::Gun(g, l) => format!("{} {}", c.guns[g].name, roman(l)),
            PickupKind::Pierce => c.say("reward_pierce"),
            PickupKind::Pulse => c.say("reward_pulse"),
            PickupKind::Mend => c.say("reward_mend"),
            PickupKind::Trigger => c.say("reward_trigger"),
            PickupKind::Plating => c.say("reward_plating"),
            PickupKind::Contempt => c.say("reward_contempt"),
            PickupKind::Purse(n) => c.say("reward_purse").replace("{n}", &n.to_string()),
        }
    }

    /// "1 year", "3 years": a number and its word from copy.json.
    fn count_of(&self, n: i32, one: &str, many: &str) -> String {
        format!("{n} {}", self.c.say(if n == 1 { one } else { many }))
    }

    fn reward_icon(r: Reward) -> &'static str {
        match r {
            Reward::Gun(_, _) => "gun",
            Reward::Jump => "jump",
            Reward::Health => "heart",
            Reward::Speed => "speed",
            Reward::Pierce => "pierce",
            Reward::Pulse => "pulse",
            Reward::Mend => "mend",
            Reward::Trigger => "trigger",
            Reward::Plating => "plating",
            Reward::Contempt => "contempt",
            Reward::Purse(_) => "purse",
        }
    }

    // ---- the screens ---------------------------------------------------

    /// Everything the menus show, for the mode the game is in.
    pub fn screen_json(&self) -> String {
        let c = &self.c;
        let m = &self.meta;
        let branch_ids: Vec<String> = c
            .copy_rows("holding_branches")
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        let tree = crate::tree::lay_out(&c.upgrades, &branch_ids);
        let upgrades: Vec<Value> = c
            .upgrades
            .iter()
            .zip(&tree.places)
            .map(|(u, at)| {
                let why = match m.check(c, &u.id) {
                    Ok(()) => String::new(),
                    Err(Refusal::Owned) => c.say("why_owned"),
                    Err(Refusal::Requires(r)) => {
                        let name = c
                            .upgrades
                            .iter()
                            .find(|x| x.id == r)
                            .map_or(r.clone(), |x| x.name.clone());
                        c.say("why_requires").replace("{name}", &name)
                    }
                    Err(Refusal::Cost(n)) => c.say("why_cost").replace("{n}", &n.to_string()),
                    Err(Refusal::Locked(boss)) => c.say("why_locked").replace(
                        "{boss}",
                        &c.story.bosses.get(&boss).map_or(boss.clone(), |b| b.name.clone()),
                    ),
                    Err(Refusal::Unknown) => String::new(),
                };
                // how many upgrades stand above it in its branch
                let mut depth = 0;
                let mut need = u.requires.clone();
                while let Some(up) = c.upgrades.iter().find(|x| x.id == need) {
                    depth += 1;
                    need = up.requires.clone();
                    if depth > 8 {
                        break;
                    }
                }
                json!({
                    "id": u.id,
                    "name": u.name,
                    "branch": u.branch,
                    "depth": depth,
                    "needs": c.upgrades.iter().find(|x| x.id == u.requires).map(|x| x.name.clone()).unwrap_or_default(),
                    "text": crate::codex::upgrade_effect(u, &c.pal),
                    "cost": u.cost,
                    "icon": u.icon,
                    "owned": m.owns(&u.id),
                    "can": m.check(c, &u.id).is_ok(),
                    "why": why,
                    // its place on the tree, as percentages of the square
                    "x": crate::tree::pct(at.x),
                    "y": crate::tree::pct(at.y),
                    "ring": at.ring,
                })
            })
            .collect();
        let (build, gun_level) = m.build(c);
        let mut s = json!({
            "mode": self.mode,
            "case": c.story.case,
            "sentencing": c.story.sentencing,
            "clemency": m.clemency,
            "deaths": m.deaths,
            "years": m.years,
            "runs": m.runs,
            "wins": m.wins,
            "counts_read": m.counts_read,
            "counts_total": c.story.counts.len(),
            "counts": c.story.counts[..m.counts_read as usize],
            "upgrades": upgrades,
            "build_lines": [
                c.say("build_health").replace("{n}", &build.max_health.to_string()),
                self.count_of(build.jumps, "jump", "jumps"),
                self.count_of(build.dash_charges, "dash", "dashes"),
                c.say("build_speed").replace("{n}", &build.speed_pct.to_string()),
                format!("{} {}", c.guns[m.carried(c)].name, roman(gun_level)),
            ],
            // the guns a run may carry in, the one chosen marked
            "carry": m.carry_choices(c).into_iter().map(|g| json!({
                "id": c.guns[g].id,
                "name": c.guns[g].name,
                "chosen": g == m.carried(c),
            })).collect::<Vec<_>>(),
            "sentence_bonus": if m.sentence_bonus_pct() > 0 { c.say("sentence_bonus").replace("{n}", &m.sentence_bonus_pct().to_string()) } else { String::new() },
            "build": {
                "health": build.max_health,
                "jumps": build.jumps,
                "dashes": build.dash_charges,
                "speed": build.speed_pct,
                "grapple": build.grapple,
                "parry": build.parry,
                "recess": build.recess,
                "gun": format!("{} {}", c.guns[m.carried(c)].name, roman(gun_level)),
                "wall": build.wall,
                "pound": build.pound > 0,
            },
            "branches": c.copy_rows("holding_branches").into_iter().map(|(id, name)| json!({ "id": id, "name": name })).collect::<Vec<_>>(),
            // the shop drawn as a passive tree: a wedge for each branch, a
            // line from each upgrade to the one it needs, the ring between
            "tree": {
                "view": format!("0 0 {s} {s}", s = crate::balance::TREE_SIZE),
                "centre": crate::balance::TREE_SIZE / 2,
                "ring": crate::balance::TREE_RING,
                "hub": crate::balance::TREE_HUB,
                "wedges": tree.wedges.iter().map(|w| json!({
                    "branch": w.branch,
                    "name": c.copy_rows("holding_branches").into_iter().find(|(id, _)| *id == w.branch).map(|(_, n)| n).unwrap_or_default(),
                    "d": crate::tree::wedge_path(w),
                    "x": crate::tree::pct(w.label.0),
                    "y": crate::tree::pct(w.label.1),
                    "turn": format!("{}deg", crate::tree::label_turn(w)),
                })).collect::<Vec<_>>(),
                "edges": tree.edges.iter().map(|&(a, b)| {
                    let (pa, pb) = (&tree.places[a], &tree.places[b]);
                    let (ua, ub) = (&c.upgrades[a], &c.upgrades[b]);
                    json!({
                        "x1": pa.x, "y1": pa.y, "x2": pb.x, "y2": pb.y,
                        "branch": ub.branch,
                        // walked: both ends owned; open: the next can be bought
                        "lit": m.owns(&ua.id) && m.owns(&ub.id),
                        "open": m.owns(&ua.id) && !m.owns(&ub.id),
                    })
                }).collect::<Vec<_>>(),
            },
            "timeline": self.timeline_json(),
            "zones": c.zones.iter().filter(|z| !z.mission && z.branch_from.is_empty()).map(|z| json!({
                "name": z.name,
                "year": z.year,
                "built": z.built,
                "boss": c.story.bosses.get(&z.boss).map(|b| b.name.clone()).unwrap_or_default(),
            })).collect::<Vec<_>>(),
            // where a run may start: zone one, and each zone past a boss beaten
            "starts": (1..=m.unlocked.max(0) as usize).filter(|&z| z < c.built_zones()).map(|z| json!({
                "zone": z,
                "label": c.say("start_later").replace("{year}", &c.zones[z].year).replace("{zone}", &c.zones[z].name),
            })).collect::<Vec<_>>(),
            "guns": c.guns.iter().map(|g| json!({ "id": g.id, "name": g.name, "blurb": g.blurb })).collect::<Vec<_>>(),
            "rush": m.rush_open(),
            "mission": m.mission_open(c),
            "mission_wins": m.mission_wins,
            // the bars the Chumbles practice may begin from
            "chumbles_bars": c.mission_zone().and_then(|z| c.story.bosses.get(&c.zones[z].boss)).map(|b| b.bars.clone()).unwrap_or_default(),
        });
        if let Some(run) = &self.run {
            let r = &run.route;
            let nodes: Vec<Value> = r
                .nodes
                .iter()
                .enumerate()
                .map(|(i, n)| {
                    json!({
                        "x": 4 + 92 * n.step / r.steps.max(1),
                        // side nodes alternate rows so their names do not collide
                        "y": if n.lane == 0 { 34 } else if n.step % 2 == 0 { 66 } else { 82 },
                        "tier": if n.kind == NodeKind::Fight { "●".repeat(n.tier as usize) } else { String::new() },
                        "kind": match n.kind { NodeKind::Arrival => "arrival", NodeKind::Fight => "fight", NodeKind::Rest => "rest", NodeKind::Boss => "boss" },
                        "name": self.node_name(n),
                        "detour": n.detour,
                        "reward": self.prize_label(n),
                        "icon": Game::prize_icon(n),
                        "here": i == run.at,
                        "visited": run.visited.contains(&i),
                        "next": run.route.nodes[run.at].next.contains(&i),
                    })
                })
                .collect();
            let edges: Vec<Value> = r
                .nodes
                .iter()
                .enumerate()
                .flat_map(|(i, n)| n.next.iter().map(move |&j| json!([i, j])))
                .collect();
            let z = &c.zones[run.zone];
            s["map"] = json!({
                "nodes": nodes,
                "edges": edges,
                "here": self.node_name(&r.nodes[run.at]),
                "heading": format!("{}, {}", z.name, z.year),
            });
            s["run"] = json!({ "clemency": run.total(), "cleared": run.cleared });
        }
        if self.mode == Mode::Interlude {
            let z = self.run.as_ref().map_or(0, |r| r.zone);
            let mission = self.run.as_ref().is_some_and(|r| r.mission);
            let briefing = self.run.as_ref().is_some_and(|r| !r.briefed);
            // the last mission's own words: its brief, then its interludes
            let river = self.run.as_ref().is_some_and(|r| r.river);
            let next = if briefing {
                z
            } else {
                self.next_zone(z, mission, river)
            };
            // crossing onto another timeline is read in its own words
            let branch = c
                .zones
                .get(next)
                .and_then(|n| c.story.branches.get(&n.id))
                .filter(|_| !briefing);
            let words = match (&c.story.mission, briefing) {
                _ if branch.is_some() => branch.map(|b| &b.interlude),
                (Some(ms), true) => Some(&ms.brief),
                (Some(ms), false) if mission => ms.interludes.get(&c.zones[z].id),
                _ => c.story.interludes.get(&c.zones[z].id),
            };
            s["interlude"] = json!({
                "title": words.map(|w| w.title.clone()).unwrap_or_default(),
                "lines": words.map(|w| w.lines.clone()).unwrap_or_default(),
                "newest": self.crossing_count.clone().unwrap_or_default(),
                "next": c.zones.get(next).map(|n| format!("{}, {}", n.name, n.year)).unwrap_or_default(),
                "art": words.map(|w| w.art.clone()).unwrap_or_default(),
                "go": words.map(|w| w.go.clone()).filter(|g| !g.is_empty()).unwrap_or_else(|| c.say("interlude_next")),
            });
        }
        if let Some(rd) = &self.reading {
            let zone = self.run.as_ref().map_or(0, |r| r.zone);
            let title = c
                .story
                .death_titles
                .get(&c.zones[zone].id)
                .or_else(|| c.story.death_titles.values().next())
                .cloned()
                .unwrap_or_default();
            s["reading"] = json!({
                "title": title,
                "counts": rd.counts,
                "newest": rd.newest,
                "time_served": c.story.time_served
                    .replace("{years}", &self.count_of(rd.years_added, "year", "years"))
                    .replace("{total}", &self.count_of(rd.total_years, "year", "years")),
                "clemency": self.last.0,
                "cleared": self.last.1,
                "grant": if rd.grant > 0 { c.say("first_grant").replace("{n}", &rd.grant.to_string()) } else { String::new() },
                "bonus": if rd.bonus_pct > 0 { c.say("sentence_bonus_applied").replace("{n}", &rd.bonus_pct.to_string()) } else { String::new() },
            });
        }
        let rushed = self.run.as_ref().is_some_and(|r| r.rush);
        let mission = self.run.as_ref().is_some_and(|r| r.mission);
        if let (Mode::Won, true, Some(ms)) = (self.mode, mission, &c.story.mission) {
            s["verdict"] = json!({
                "art": ms.verdict_art,
                "title": ms.victory_title,
                "lines": ms.victory,
                "newest": "",
                "clemency": self.last.0,
                "cleared": self.last.1,
            });
        } else if let (Mode::Won, Some(b)) = (
            self.mode,
            self.run
                .as_ref()
                .filter(|r| !r.rush)
                .and_then(|r| c.story.branches.get(&c.zones[r.zone].id)),
        ) {
            s["verdict"] = json!({
                "art": b.verdict_art,
                "title": b.victory_title,
                "lines": b.victory,
                "newest": "",
                "clemency": self.last.0,
                "cleared": self.last.1,
            });
        } else if self.mode == Mode::Won && rushed {
            s["verdict"] = json!({
                "art": c.story.verdict_art,
                "title": c.say("rush_title"),
                "lines": [c.say("rush_line").replace("{n}", &self.meta.rush_wins.to_string())],
                "newest": "",
                "clemency": self.last.0,
                "cleared": self.last.1,
            });
        } else if self.mode == Mode::Won {
            s["verdict"] = json!({
                "art": c.story.verdict_art,
                "title": c.story.victory_title,
                "lines": c.story.victory,
                "newest": self.win_count.clone().unwrap_or_default(),
                "clemency": self.last.0,
                "cleared": self.last.1,
            });
        }
        s.to_string()
    }
}

/// The held bits and the bits pressed since the last tick, as controls.
pub fn controls(held: u32, pressed: u32) -> Controls {
    let on = |n: &str| held & bit(n) != 0;
    let hit = |n: &str| pressed & bit(n) != 0;
    let axis = |pos: &str, neg: &str| (on(pos) as i32 - on(neg) as i32) * ONE;
    Controls {
        move_x: axis("right", "left"),
        move_z: axis("forward", "back"),
        jump_pressed: hit("jump"),
        dash_pressed: hit("dash"),
        fire: on("fire") || pressed & bit("fire") != 0,
        grapple_held: on("grapple"),
        grapple_pressed: hit("grapple"),
        parry_pressed: hit("parry"),
        interact_pressed: hit("interact"),
        wall_pressed: hit("wall"),
        pound_pressed: hit("pound"),
        fire_pressed: hit("fire"),
        alt_pressed: hit("alt"),
        grab_pressed: hit("grab"),
        stop_pressed: hit("stop"),
        bench_pressed: hit("bench"),
    }
}

/// The over-the-shoulder camera: behind and right of the head, pulled in
/// in front of any wall it would back into.
pub fn play_camera(w: &World, alpha: i32) -> Camera {
    let p = &w.player;
    let feet = p.prev_pos.lerp(p.pos, alpha);
    let head = feet.add(v3(0, CAMERA_UP, 0));
    let right = right_of(p.yaw);
    let level = &w.arena.level;
    let side = level
        .ray(head, right, CAMERA_RIGHT + CAMERA_MARGIN)
        .map_or(CAMERA_RIGHT, |h| (h.t - CAMERA_MARGIN).max(0));
    let pivot = head.add(right.scale(side));
    let fwd = facing(p.yaw, p.pitch);
    let back = fwd.neg();
    let dist = level
        .ray(pivot, back, CAMERA_BACK + CAMERA_MARGIN)
        .map_or(CAMERA_BACK, |h| (h.t - CAMERA_MARGIN).max(ONE * 3 / 10));
    // the Executor's arrival and its blows shake the view
    let shake = if w.shake > 0 {
        let k = fx::mul(SHAKE_SIZE, fx::div(w.shake * ONE, SHAKE_TICKS * ONE));
        v3(
            fx::mul(fx::sin(w.tick * fx::deg(97)), k),
            fx::mul(fx::sin(w.tick * fx::deg(131) + fx::QUARTER), k),
            fx::mul(fx::sin(w.tick * fx::deg(73)), k),
        )
    } else {
        V3::ZERO
    };
    Camera {
        eye: pivot.add(back.scale(dist)).add(shake),
        fwd,
        fov: FOV,
    }
}

pub fn numbers() -> String {
    let vertex_bytes = mesh::VERTEX_VALUES * render::BYTES_PER_VALUE;
    let instance_bytes = render::INSTANCE_VALUES * render::BYTES_PER_VALUE;
    let vec4 = 4 * render::BYTES_PER_VALUE;
    json!({
        "frac_bits": fx::FRAC_BITS,
        "ticks_per_second": TICKS_PER_SECOND,
        "actions": ACTIONS.iter().map(|(n, b)| (n.to_string(), json!(b))).collect::<serde_json::Map<_, _>>(),
        "meshes": Mesh::ALL.iter().map(|m| json!({
            "id": *m as u32,
            "vertices": mesh::vertices(*m).len() / mesh::VERTEX_VALUES,
        })).collect::<Vec<_>>(),
        "vertex": {
            "stride": vertex_bytes,
            "attributes": [["aPos", 3, 0], ["aNormal", 3, 3 * render::BYTES_PER_VALUE]],
        },
        "instance": {
            "stride": instance_bytes,
            "attributes": [["iA", 4, 0], ["iB", 4, vec4], ["iC", 4, 2 * vec4], ["iD", 4, 3 * vec4]],
        },
        "draw_values": render::DRAW_VALUES,
        "buffers": { "static": render::STATIC_BUFFER, "dynamic": render::DYNAMIC_BUFFER },
        "passes": { "solid": render::PASS_SOLID, "facade": render::PASS_FACADE, "ink": render::PASS_INK, "glow": render::PASS_GLOW },
        "uniforms": [["uLightDir", 3], ["uSky", 3], ["uFog", 3], ["uFogRange", 2], ["uLight", 3], ["uWindowDark", 3], ["uInk", 3], ["uSee", 4], ["uSeeSolid", 1]],
        // the view values: the matrix, the eye, then the see-through focus
        // (the head, and the floor under it)
        "view": { "matrix": [0, 16], "eye": [16, 19], "focus": [19, 23], "fx": [23, 27], "bubble": [27, 31] },
        "percent": 100,
    })
    .to_string()
}

pub fn mesh_vertices(id: u32) -> Vec<i32> {
    Mesh::ALL
        .iter()
        .find(|m| **m as u32 == id)
        .map(|m| mesh::vertices(*m))
        .unwrap_or_default()
}

/// The core's white noise for the page's sound: `n` samples in [−1, 1] as
/// Q16, from a seed.
pub fn noise(n: u32, seed: u32) -> Vec<i32> {
    let mut r = crate::rng::Rng::new(seed as u64);
    (0..n.min(1 << 20)).map(|_| r.range(-ONE, ONE)).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::content::tests::files;

    pub fn game_with(saved: &str) -> Game {
        let f = files();
        Game::new(
            f.weapons, f.patterns, f.enemies, f.zone, f.upgrades, f.palette, f.story, f.copy, saved,
        )
        .unwrap()
    }

    const FRAME_US: i64 = 16_667;

    fn frames(g: &mut Game, n: i32, bits: u32) {
        for _ in 0..n {
            let now = g.last_us.unwrap_or(0) + FRAME_US;
            g.advance(now, bits | bit("focus"), 0, 0);
        }
    }

    #[test]
    fn a_first_visit_opens_on_the_title_then_the_sentencing_then_the_holding() {
        let mut g = game_with("");
        assert_eq!(g.mode, Mode::Title);
        g.begin();
        assert_eq!(g.mode, Mode::Intro);
        g.finish_intro();
        assert_eq!(g.mode, Mode::Holding);
        let mut back = game_with(&g.saved());
        back.begin();
        assert_eq!(back.mode, Mode::Holding, "the sentencing is not read twice");
    }

    #[test]
    fn a_run_carries_in_the_gun_chosen_in_the_holding_at_the_armorys_level() {
        let mut g = game_with(r#"{"seen_intro": true, "clemency": 500, "owned": ["exhibit_b", "armory"]}"#);
        g.begin();
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        let names: Vec<&str> = s["carry"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["id"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["carbine", "scattergun"]);
        assert!(g.carry("scattergun"));
        assert!(!g.carry("redactor"), "no Exhibit for it");
        g.start_run();
        let held = g.run.as_ref().unwrap().world.held;
        assert_eq!(held.gun, g.c.gun("scattergun"));
        assert_eq!(held.level, 2, "the Standing Armory's level");
        assert!(!g.carry("carbine"), "chosen only in the Holding");
    }

    #[test]
    fn a_run_starts_in_the_crater_and_its_gate_leads_on_to_the_first_fight() {
        let mut g = game_with(r#"{"seen_intro": true}"#);
        g.begin();
        g.start_run();
        assert_eq!(g.mode, Mode::Play);
        assert_eq!(g.run.as_ref().unwrap().at, 0);
        // run north until through the gate
        for _ in 0..60 * 20 {
            frames(&mut g, 1, bit("forward"));
            if g.run.as_ref().unwrap().at != 0 {
                break;
            }
        }
        let run = g.run.as_ref().unwrap();
        assert_eq!(run.at, 1);
        assert_eq!(run.route.nodes[1].kind, NodeKind::Fight);
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(h["mode"], "play");
        assert_eq!(h["health"], 100);
    }

    #[test]
    fn dying_reads_the_first_count_and_banks_the_runs_clemency() {
        let mut g = game_with(r#"{"seen_intro": true}"#);
        g.begin();
        g.start_run();
        g.run.as_mut().unwrap().earned = 7;
        g.run.as_mut().unwrap().world.player.health = 0;
        frames(&mut g, 2, 0);
        assert_eq!(g.mode, Mode::Dead);
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["reading"]["newest"], g.c.story.counts[0].as_str());
        // 7 earned, and the clerk's grant for a first death
        assert_eq!(s["clemency"], 7 + FIRST_DEATH_GRANT);
        g.to_holding();
        assert_eq!(g.mode, Mode::Holding);
        // 15 Clemency: Fleet I (10) leaves 5, short of Constitution I (8)
        assert!(g.buy("fleet_1"));
        assert!(!g.buy("constitution_1"));
        g.grant(3);
        assert!(g.buy("constitution_1"));
    }

    /// Straight to the run's boss, its last phase, one blow from down.
    fn fell_the_boss(g: &mut Game) {
        let boss = g.run.as_ref().unwrap().route.boss();
        g.go(boss);
        assert_eq!(g.run.as_ref().unwrap().world.kind, NodeKind::Boss);
        frames(g, 120, 0);
        let run = g.run.as_mut().unwrap();
        let b = run.world.boss.expect("the boss came");
        run.world.enemies[b].phase = 2;
        run.world.enemies[b].hp = 1;
        run.world.enemies[b].state = crate::enemy::State::Rest(1000);
        run.world.player.iframes = 100_000;
        run.world.enemies[b].hurt(5);
        frames(g, 200, 0);
    }

    #[test]
    fn felling_each_boss_crosses_to_the_next_zone_and_felling_the_executor_reads_the_verdict() {
        let mut g = game_with(r#"{"seen_intro": true}"#);
        g.begin();
        g.start_run();
        g.run.as_mut().unwrap().world.held = Held::new(2, 3);
        fell_the_boss(&mut g);
        assert_eq!(g.mode, Mode::Interlude, "the Bailiff does not end the run");
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["interlude"]["title"], "THE YEARS BETWEEN");
        // the first crossing reads a count
        assert_eq!(s["interlude"]["newest"], g.c.story.counts[0].as_str());
        assert_eq!(g.meta.unlocked, 1);
        g.run.as_mut().unwrap().world.player.health = 5;
        g.continue_run();
        assert_eq!(g.mode, Mode::Play);
        let run = g.run.as_ref().unwrap();
        assert_eq!(run.zone, 1);
        assert_eq!(run.world.zone, 1);
        assert_eq!(run.world.kind, NodeKind::Arrival);
        // the body comes along healed, with what it carried
        assert_eq!(run.world.player.health, run.world.player.build.max_health);
        assert_eq!((run.world.held.gun, run.world.held.level), (2, 3));
        // the Bailiff's Clemency is the run's, not yet banked
        assert!(run.earned >= g.c.zones[0].route.clemency_boss);
        assert_eq!(g.meta.clemency, 0);
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(h["zone"], "The White Waste");
        assert_eq!(h["node"], "The Landing");
        // the Witness: it says what it saw, and the run goes home
        fell_the_boss(&mut g);
        assert_eq!(g.mode, Mode::Interlude, "the Witness does not end the run");
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["interlude"]["title"], "WHAT THE WITNESS SAW");
        assert_eq!(s["interlude"]["go"], "Go home");
        assert_eq!(g.meta.unlocked, 2);
        g.continue_run();
        let run = g.run.as_ref().unwrap();
        assert_eq!(run.zone, 2);
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(h["zone"], "The Glass House");
        assert_eq!(h["node"], "The Vestibule");
        // the Executor: its last phase outlasted
        let boss = g.run.as_ref().unwrap().route.boss();
        g.go(boss);
        frames(&mut g, 120, 0);
        let run = g.run.as_mut().unwrap();
        let b = run.world.boss.expect("the Executor came");
        assert_eq!(g.c.enemies[run.world.enemies[b].def].id, "executor");
        let last = g.c.enemies[run.world.enemies[b].def].phases.len() - 1;
        run.world.enemies[b].phase = last;
        run.world.enemies[b].state = crate::enemy::State::Rest(1000);
        run.world.enemies[b].tank.as_mut().unwrap().survive = 2;
        run.world.player.iframes = 100_000;
        frames(&mut g, 400, 0);
        // the Executor's orders, and the climb
        assert_eq!(g.mode, Mode::Interlude, "the Executor does not end the run");
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["interlude"]["title"], "THE ORDERS");
        g.continue_run();
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(h["zone"], "The Ascent");
        assert_eq!(h["node"], "The Trailhead");
        // the Defendant, and what comes after it
        let boss = g.run.as_ref().unwrap().route.boss();
        g.go(boss);
        frames(&mut g, 120, 0);
        let run = g.run.as_mut().unwrap();
        let b = run.world.boss.expect("the Defendant came");
        assert_eq!(g.c.enemies[run.world.enemies[b].def].id, "defendant");
        run.world.player.iframes = 100_000;
        let e = &mut run.world.enemies[b];
        e.phase = 2;
        e.state = crate::enemy::State::Rest(1000);
        e.hp = 1;
        e.hurt(5);
        frames(&mut g, 60, 0);
        assert_eq!(g.mode, Mode::Play, "the Idiot God comes first");
        assert!(g.run.as_ref().unwrap().world.melt() > 0);
        frames(&mut g, ENDING_TICKS + 60, 0);
        // the revelation, and the fall into it
        assert_eq!(g.mode, Mode::Interlude, "the Defendant does not end the run");
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["interlude"]["title"], "THE DOOR");
        g.continue_run();
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(h["zone"], "The Idiot God");
        assert_eq!(h["node"], "The Gullet");
        // the god, sung to sleep (here, at once)
        let boss = g.run.as_ref().unwrap().route.boss();
        g.go(boss);
        frames(&mut g, 120, 0);
        let run = g.run.as_mut().unwrap();
        let b = run.world.boss.expect("the god came");
        assert_eq!(g.c.enemies[run.world.enemies[b].def].id, "azathoth");
        run.world.player.iframes = 100_000;
        let e = &mut run.world.enemies[b];
        e.state = crate::enemy::State::Rest(1000);
        e.floor_hp = 0;
        e.hp = 1;
        e.hurt(5);
        frames(&mut g, 400, 0);
        assert_eq!(g.mode, Mode::Won);
        assert_eq!(g.meta.wins, 1);
        // all five bosses' Clemency banked: 25, 40, 55, 80 and 150 in zone.json
        assert!(g.meta.clemency >= 350, "{}", g.meta.clemency);
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["verdict"]["title"], g.c.story.victory_title.as_str());
        g.to_holding();
        assert_eq!(g.mode, Mode::Holding);
        // the time stop is yours now
        assert_eq!(g.meta.build(&g.c).0.stop_ticks, STOP_BASE);
        // and a run may now start on the ice, in the house, on the
        // mountain or in the god
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["starts"][0]["zone"], 1);
        assert_eq!(s["starts"][1]["zone"], 2);
        assert_eq!(s["starts"][2]["zone"], 3);
        assert_eq!(s["starts"][3]["zone"], 4);
        g.start_run_at(4);
        assert_eq!(g.run.as_ref().unwrap().zone, 4);
    }

    #[test]
    fn a_boss_rush_runs_each_zones_boss_in_turn_and_only_once_the_game_is_beaten() {
        let mut g = game_with(r#"{"seen_intro": true}"#);
        g.begin();
        g.start_rush();
        assert_eq!(g.mode, Mode::Holding, "not before the game is beaten");
        g.test_save();
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["rush"], true);
        g.start_rush();
        assert_eq!(g.mode, Mode::Play);
        let mut bosses = Vec::new();
        for zone in 0..g.c.built_zones() {
            let run = g.run.as_ref().unwrap();
            assert_eq!(run.zone, zone);
            assert_eq!(run.world.kind, NodeKind::Boss, "a boss's arena from the start");
            bosses.push(run_down_the_boss(&mut g));
            if zone + 1 < g.c.built_zones() {
                assert_eq!(g.mode, Mode::Play, "no interlude between");
                let w = &g.run.as_ref().unwrap().world;
                assert_eq!(w.player.health, w.player.build.max_health, "healed");
            }
        }
        // the Oak, once it has burnt, after the god
        assert_eq!(g.mode, Mode::Play, "on to the river's boss");
        assert_eq!(g.c.zones[g.run.as_ref().unwrap().zone].id, "water_gap");
        bosses.push(run_down_the_boss(&mut g));
        assert_eq!(
            bosses,
            [
                "bailiff",
                "witness",
                "executor",
                "defendant",
                "azathoth",
                "gallows_oak"
            ]
        );
        assert_eq!(g.mode, Mode::Won);
        assert_eq!(g.meta.rush_wins, 1);
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["verdict"]["title"], g.c.say("rush_title").as_str());
    }

    /// The boss in the arena, brought down at once; its id.
    fn run_down_the_boss(g: &mut Game) -> String {
        frames(g, 120, 0);
        let run = g.run.as_mut().unwrap();
        let b = run.world.boss.expect("the boss came");
        let id = g.c.enemies[run.world.enemies[b].def].id.clone();
        let last = g.c.enemies[run.world.enemies[b].def].phases.len() - 1;
        run.world.player.iframes = 100_000;
        let e = &mut run.world.enemies[b];
        e.phase = last;
        e.state = crate::enemy::State::Rest(1000);
        if let Some(t) = e.tank.as_mut() {
            t.survive = 2;
        } else {
            // the god is put to sleep at once, its song skipped; the Oak,
            // never hurt, simply falls
            e.floor_hp = 0;
            e.hp = 1;
            e.hurt(5);
            if e.shielded {
                e.hp = 0;
            }
        }
        let zone = g.run.as_ref().unwrap().zone;
        for _ in 0..ENDING_TICKS + 200 {
            frames(g, 1, 0);
            if g.mode != Mode::Play || g.run.as_ref().is_some_and(|r| r.zone != zone) {
                break;
            }
        }
        id
    }

    #[test]
    fn the_god_has_a_bar_for_each_phase_and_then_the_lullabys_staff() {
        let mut g = game_with(r#"{"seen_intro": true}"#);
        g.begin();
        g.test_save();
        g.start_run_at(4);
        let boss = g.run.as_ref().unwrap().route.boss();
        g.go(boss);
        frames(&mut g, 120, 0);
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(h["boss"]["bars"], 4, "three phases and the lullaby");
        assert_eq!(h["boss"]["bars_left"], 4);
        assert_eq!(h["boss"]["timer"], "THE BLIND MASS");
        assert_eq!(h["boss"]["pct"], 100);
        assert!(h["song"].is_null());
        // half way through the second bar
        let run = g.run.as_mut().unwrap();
        let b = run.world.boss.unwrap();
        run.world.player.iframes = 1_000_000;
        let e = &mut run.world.enemies[b];
        e.phase = 1;
        e.hp = e.max_hp / 2;
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(h["boss"]["bars_left"], 3);
        assert_eq!(h["boss"]["timer"], "THE DANCERS");
        let pct = h["boss"]["pct"].as_i64().unwrap();
        assert!((45..=55).contains(&pct), "{pct}");
        // its last bar empty: the staff
        g.lull();
        frames(&mut g, 2, 0);
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(h["boss"]["sleep"], true);
        assert_eq!(h["boss"]["timer"], "THE LULLABY");
        assert_eq!(h["song"]["keys"].as_array().unwrap().len(), 5);
        frames(&mut g, 300, 0);
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(h["song"]["part"], "listen");
        assert!(
            !h["song"]["notes"].as_array().unwrap().is_empty(),
            "notes on the staff"
        );
    }

    #[test]
    fn the_lullaby_practice_starts_at_the_song_and_leaves_the_record_alone() {
        let mut g = game_with(r#"{"seen_intro": true}"#);
        g.begin();
        let before = (g.meta.deaths, g.meta.clemency, g.meta.years, g.meta.rush_wins);
        g.practice_lullaby();
        assert_eq!(g.mode, Mode::Play);
        assert_eq!(g.run.as_ref().unwrap().zone, g.c.built_zones() - 1);
        frames(&mut g, 120, 0);
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(h["boss"]["timer"], "THE LULLABY", "straight to the song");
        assert!(h["song"].is_object());
        // dying in it: back to the Holding, no count read
        g.run.as_mut().unwrap().world.player.health = 0;
        frames(&mut g, 2, 0);
        assert_eq!(g.mode, Mode::Holding);
        assert!(g.run.is_none());
        // winning it: the same
        g.practice_lullaby();
        frames(&mut g, 120, 0);
        let run = g.run.as_mut().unwrap();
        run.world.player.iframes = 1_000_000;
        // the song played through to its end
        let song = run.world.song.as_mut().expect("the song began");
        song.part = crate::song::Part::Asleep;
        song.clock = 0;
        for _ in 0..ENDING_TICKS * 2 {
            frames(&mut g, 1, 0);
            if g.mode != Mode::Play {
                break;
            }
        }
        assert_eq!(g.mode, Mode::Holding);
        // abandoning it: the same
        g.practice_lullaby();
        g.pause();
        g.abandon();
        assert_eq!(g.mode, Mode::Holding);
        let after = (g.meta.deaths, g.meta.clemency, g.meta.years, g.meta.rush_wins);
        assert_eq!(before, after, "the record untouched");
    }

    #[test]
    fn the_last_mission_opens_once_the_god_sleeps_and_runs_one_to_four_then_home() {
        let mut g = game_with(r#"{"seen_intro": true, "wins": 1, "beaten": ["bailiff", "witness"]}"#);
        g.begin();
        g.start_mission();
        assert_eq!(g.mode, Mode::Holding, "not before the Idiot God sleeps");
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["mission"], false);
        g.meta.beat("azathoth");
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["mission"], true);
        let unlocked = g.meta.unlocked;
        // the brief first, on the interlude's screen
        g.start_mission();
        assert_eq!(g.mode, Mode::Interlude);
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["interlude"]["title"], "THE LAST SENTENCE");
        g.continue_run();
        assert_eq!(g.mode, Mode::Play);
        let run = g.run.as_ref().unwrap();
        assert_eq!((run.zone, run.world.kind), (0, NodeKind::Arrival));
        assert!(
            run.world.mission && run.world.remix,
            "new game plus from the crater"
        );
        let mut words = Vec::new();
        for zone in 0..4 {
            assert_eq!(g.run.as_ref().unwrap().zone, zone);
            let boss = g.run.as_ref().unwrap().route.boss();
            g.go(boss);
            run_down_the_boss(&mut g);
            assert_eq!(
                g.mode,
                Mode::Interlude,
                "zone {zone}'s boss does not end the mission"
            );
            let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
            words.push(s["interlude"]["title"].as_str().unwrap().to_string());
            g.continue_run();
        }
        assert_eq!(
            words,
            [
                "THE YEARS AGAIN",
                "THE KEY AGAIN",
                "THE ORDERS AGAIN",
                "REALITY BREAKS"
            ]
        );
        assert_eq!(g.meta.unlocked, unlocked, "the mission opens no later start");
        // home: the bedroom, the landing, downstairs
        let run = g.run.as_ref().unwrap();
        assert_eq!(Some(run.zone), g.c.mission_zone(), "after the summit, home");
        assert_eq!(g.name_of(&run.route.nodes[run.at]), "Your Bedroom");
        assert!(run.world.guide(&g.c).is_some(), "the cat leads the way out");
        let boss = run.route.boss();
        g.go(boss);
        frames(&mut g, 120, 0);
        let run = g.run.as_mut().unwrap();
        let b = run.world.boss.expect("Chumbles came");
        assert_eq!(g.c.enemies[run.world.enemies[b].def].id, "chumbles");
        // fed: his last bar empty
        run.world.player.iframes = 1_000_000;
        let last = g.c.enemies[run.world.enemies[b].def].phases.len() - 1;
        let e = &mut run.world.enemies[b];
        e.size = ONE;
        e.phase = last;
        e.state = crate::enemy::State::Rest(1000);
        e.hp = 2;
        e.hurt(5);
        for _ in 0..CAT_FED_TICKS + ENDING_TICKS {
            frames(&mut g, 1, 0);
            if g.mode != Mode::Play {
                break;
            }
        }
        assert_eq!(g.mode, Mode::Won);
        assert_eq!(g.meta.mission_wins, 1);
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["verdict"]["title"], "BREAKFAST");
    }

    #[test]
    fn the_last_missions_fights_send_half_as_many_again_and_fire_remixes() {
        let mut g = game_with(r#"{"seen_intro": true, "beaten": ["azathoth"]}"#);
        g.begin();
        g.start_mission();
        g.continue_run();
        let first = g.run.as_ref().unwrap().route.nodes[0].next[0];
        g.go(first);
        let run = g.run.as_ref().unwrap();
        let node = &run.route.nodes[run.at];
        let plain: Vec<usize> = g.c.zones[0].encounters[node.encounter.unwrap()]
            .waves
            .iter()
            .map(|w| w.len())
            .collect();
        let remixed: Vec<usize> = run.world.waves_left.iter().map(|w| w.len()).collect();
        let want: Vec<usize> = plain
            .iter()
            .map(|n| n + (n * 50).div_euclid(100) + (n % 2))
            .collect();
        assert_eq!(remixed, want, "half as many again, rounded up");
        frames(&mut g, 300, 0);
        let w = &g.run.as_ref().unwrap().world;
        assert!(!w.enemies.is_empty() && w.enemies.iter().all(|e| e.remix));
        let ring = g.c.patterns.iter().position(|p| p.id == "ring_16").unwrap();
        let e = &w.enemies[0];
        assert_eq!(g.c.patterns[e.pattern_of(&g.c, ring)].id, "remix_ring_16");
    }

    #[test]
    fn the_chumbles_practice_fights_him_with_everything_and_leaves_the_record_alone() {
        let mut g = game_with(r#"{"seen_intro": true}"#);
        g.begin();
        let before = g.meta.clone();
        g.practice_chumbles(0);
        assert_eq!(g.mode, Mode::Play);
        let run = g.run.as_ref().unwrap();
        assert_eq!(Some(run.zone), g.c.mission_zone());
        let (most, _) = Meta::everything(&g.c).build(&g.c);
        assert_eq!(run.world.player.build.max_health, most.max_health, "maxed out");
        frames(&mut g, 300, 0);
        let run = g.run.as_ref().unwrap();
        let b = run.world.boss.expect("Chumbles came");
        assert_eq!(
            run.world.enemies[b].phase, 0,
            "from his first bar, not fed at once"
        );
        assert!(run.world.fed == 0);
        g.pause();
        g.abandon();
        assert_eq!(g.mode, Mode::Holding);
        assert_eq!(g.meta, before, "the record untouched");
    }

    #[test]
    fn the_chumbles_practice_can_begin_at_any_bar() {
        let mut g = game_with(r#"{"seen_intro": true}"#);
        g.begin();
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        let bars = s["chumbles_bars"].as_array().unwrap().len();
        let home = g.c.mission_zone().unwrap();
        let d =
            g.c.enemies
                .iter()
                .find(|d| d.kind == crate::content::Kind::Cat)
                .unwrap()
                .clone();
        assert_eq!(bars, d.phases.len(), "a button for each of his bars");
        for bar in 1..bars {
            g.practice_chumbles(bar);
            frames(&mut g, 300, 0);
            let w = &g.run.as_ref().unwrap().world;
            let e = &w.enemies[w.boss.expect("Chumbles came")];
            assert_eq!(e.phase, bar, "begun at bar {bar}");
            assert_eq!(e.size, ONE, "grown already");
            assert!(e.health_pct() <= d.phases[bar].from_pct);
            assert!(e.health_pct() > d.phases[bar].from_pct - 10, "on the bar's line");
            assert_eq!(g.run.as_ref().unwrap().zone, home);
            g.pause();
            g.abandon();
        }
    }

    #[test]
    fn a_run_cannot_start_in_a_zone_not_yet_reached() {
        let mut g = game_with(r#"{"seen_intro": true}"#);
        g.begin();
        g.start_run_at(1);
        assert_eq!(g.run.as_ref().unwrap().zone, 0);
    }

    #[test]
    fn dying_mid_fight_credits_its_share_for_each_enemy_killed() {
        let mut g = game_with(r#"{"seen_intro": true, "deaths": 1}"#);
        g.begin();
        g.start_run();
        let first = g.run.as_ref().unwrap().route.nodes[0].next[0];
        g.go(first);
        let run = g.run.as_mut().unwrap();
        let node = run.route.nodes[run.at].clone();
        let total: i32 = g.c.zones[0].encounters[node.encounter.unwrap()]
            .waves
            .iter()
            .map(|w| w.len() as i32)
            .sum();
        run.world.kills = total / 2;
        run.world.player.health = 0;
        frames(&mut g, 2, 0);
        assert_eq!(g.mode, Mode::Dead);
        // clemency_main (3) times the share killed, rounded down; no
        // first-death grant (this record has died before)
        let expect = g.c.zones[0].route.clemency_main * (total / 2) / total;
        assert_eq!(g.meta.clemency, expect, "{} of {total} killed", total / 2);
    }

    #[test]
    fn the_hud_points_to_a_threat_and_to_the_last_hit() {
        let mut g = game_with(r#"{"seen_intro": true}"#);
        g.begin();
        g.start_run();
        let first = g.run.as_ref().unwrap().route.nodes[0].next[0];
        g.go(first);
        let w = &mut g.run.as_mut().unwrap().world;
        let d = g.c.enemy("drifter").unwrap();
        let mut e = crate::enemy::Enemy::new(d, &g.c.enemies[d], w.player.chest().add(v3(0, 0, int(15))), 1);
        e.state = crate::enemy::State::Windup(20);
        w.enemies.push(e);
        w.hit_from = w.player.chest().add(v3(int(5), 0, 0));
        w.hit_age = 0;
        g.draw();
        let h: Value = serde_json::from_str(&g.hud_json()).unwrap();
        let foe = &h["foes"][0];
        // behind the camera: an edge arrow, marked as a threat while it winds up
        assert_eq!(foe["edge"], true);
        assert_eq!(foe["threat"], true);
        assert!(h["hit"]["fade"].as_i64().unwrap() > 90);
    }

    #[test]
    fn controls_turn_bits_into_movement_and_presses() {
        let c = controls(
            bit("forward") | bit("left") | bit("fire"),
            bit("jump") | bit("dash"),
        );
        assert_eq!((c.move_x, c.move_z), (-ONE, ONE));
        assert!(c.fire && c.dash_pressed);
        // a jump pressed and released between two ticks still counts once
        assert!(c.jump_pressed);
        let held_only = controls(bit("jump"), 0);
        assert!(!held_only.jump_pressed, "holding jump does not jump again");
    }

    #[test]
    fn the_camera_sits_behind_and_right_of_the_player_and_never_inside_a_wall() {
        let mut g = game_with(r#"{"seen_intro": true}"#);
        g.begin();
        g.start_run();
        let run = g.run.as_mut().unwrap();
        let cam = play_camera(&run.world, ONE);
        let p = run.world.player.pos;
        // facing −z: the camera is behind (+z) and to the right (+x)
        assert!(cam.eye.z > p.z + int(3) && cam.eye.x > 0, "{:?}", cam.eye);
        // backed against the south gate's wall it pulls in
        run.world.player.pos = v3(0, 0, arena::HALF + int(2));
        run.world.player.prev_pos = run.world.player.pos;
        let near = play_camera(&run.world, ONE);
        let wall = arena::HALF + int(3) - ONE / 4;
        assert!(near.eye.z < wall, "{:?}", near.eye);
    }

    #[test]
    fn at_an_arenas_start_the_camera_stands_in_front_of_the_shut_gate() {
        // the shut gate you came in by stands at the south edge; a camera
        // behind it would show nothing but its back (SECOND-ORDER Z1)
        for zone in [0, 1, 2] {
            let mut g = game_with(r#"{"seen_intro": true, "unlocked": 2}"#);
            g.begin();
            g.start_run_at(zone);
            let first = g.run.as_ref().unwrap().route.nodes[0].next[0];
            for at in [0, first] {
                if at != 0 {
                    g.go(at);
                }
                let cam = play_camera(&g.run.as_ref().unwrap().world, ONE);
                assert!(cam.eye.z < arena::HALF + ONE / 4, "zone {zone}: {:?}", cam.eye);
            }
        }
    }

    #[test]
    fn every_copy_key_the_core_uses_is_in_copy_json() {
        let g = game_with("");
        for key in [
            "callout_recess",
            "callout_fell",
            "callout_fountain",
            "callout_phase",
            "callout_boss_down",
            "callout_cleared",
            "callout_gun",
            "callout_health",
            "callout_speed",
            "callout_jump",
            "callout_jump_full",
            "callout_objection",
            "gate_main",
            "gate_side",
            "prompt_take",
            "hint_arrival",
            "hint_rest",
            "hint_fight",
            "hint_cleared",
            "reward_jump",
            "reward_health",
            "reward_speed",
            "why_owned",
            "why_requires",
            "why_cost",
            "start_later",
            "interlude_next",
            "hud_wall",
            "hint_landing",
            "build_wall",
        ] {
            assert!(g.c.has_copy(key), "copy.json has no {key}");
        }
    }

    #[test]
    fn every_colour_the_core_names_is_in_the_palette() {
        let g = game_with("");
        let src = [
            include_str!("draw.rs"),
            include_str!("draw/beasts.rs"),
            include_str!("arena.rs"),
            include_str!("arena/ice.rs"),
            include_str!("world.rs"),
            include_str!("game.rs"),
        ];
        for s in src {
            for piece in s.split("c(\"").skip(1) {
                let name = piece.split('"').next().unwrap_or("");
                assert!(g.c.pal.has(name), "palette.json has no colour {name}");
            }
        }
        for gun in &g.c.guns {
            assert!(g.c.pal.has(&gun.colour));
        }
    }

    #[test]
    fn scenery_may_be_cut_away_in_front_of_you_and_bodies_may_not() {
        let mut g = game_with(r#"{"seen_intro": true}"#);
        g.begin();
        g.start_run();
        frames(&mut g, 2, 0);
        // every draw says whether it is scenery (the static buffer) or not
        for d in g.draws.chunks(render::DRAW_VALUES) {
            assert_eq!(d[5], (d[0] == render::STATIC_BUFFER) as u32, "{d:?}");
        }
        // in play the view carries the head and the floor under it
        let v = &g.view;
        assert_eq!(v.len(), 31);
        let p = g.run.as_ref().unwrap().world.player.pos;
        assert_eq!((v[19], v[21], v[22]), (p.x, p.z, p.y));
        assert!(v[20] > p.y + int(1));
        // in the menus nothing is cut: the focus is the eye itself
        g.pause();
        g.abandon();
        g.to_holding();
        g.draw();
        let v = &g.view;
        assert_eq!((v[19], v[20], v[21]), (v[16], v[17], v[18]));
    }

    #[test]
    fn the_ice_is_lit_and_outlined_in_its_own_colours() {
        let mut g = game_with(r#"{"seen_intro": true, "unlocked": 1}"#);
        g.begin();
        let city = g.scene_uniforms();
        g.start_run_at(1);
        let ice = g.scene_uniforms();
        assert_ne!(city, ice);
        assert_eq!(g.clear_color(), g.c.pal.c("ice_fog").to_vec());
        // the see-through sizes ride with the scene
        assert_eq!(
            ice[ice.len() - 5..ice.len() - 1],
            [SEE_NEAR, SEE_WIDE_LENS, SEE_WIDE_HEAD, SEE_SHORT]
        );
    }
    /// A record that has beaten the house and owns the river's kit.
    const RIVER_READY: &str = r#"{"seen_intro": true, "unlocked": 3, "beaten": ["bailiff", "witness", "executor"], "owned": ["writ", "gavel", "bench"]}"#;

    #[test]
    fn the_river_stays_shut_until_the_executor_is_beaten_and_its_kit_bought() {
        let mut g = game_with(
            r#"{"seen_intro": true, "unlocked": 2, "beaten": ["bailiff", "witness"], "owned": ["writ", "gavel", "bench"]}"#,
        );
        g.begin();
        assert!(!g.choose_timeline("river"), "the Executor first");
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        let river = &s["timeline"]["lines"][1];
        assert_eq!(river["open"], false);
        assert_eq!(
            river["why"],
            g.c.say("why_locked").replace("{boss}", "THE EXECUTOR").as_str()
        );
        let mut g = game_with(r#"{"seen_intro": true, "beaten": ["executor"], "owned": ["writ", "gavel"]}"#);
        g.begin();
        assert!(!g.choose_timeline("river"), "the Bench too");
        let mut g = game_with(RIVER_READY);
        g.begin();
        assert!(g.choose_timeline("river"));
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["timeline"]["lines"][1]["chosen"], true);
        // the river's year stands below the main line, after the house
        let nodes = s["timeline"]["nodes"].as_array().unwrap();
        let gap = nodes
            .iter()
            .find(|n| n["name"] == "The Water Gap")
            .expect("the Water Gap on the timeline");
        let house = nodes.iter().find(|n| n["name"] == "The Glass House").unwrap();
        assert_eq!(gap["line"], "river");
        assert!(gap["y"].as_i64() > house["y"].as_i64() && gap["x"].as_i64() > house["x"].as_i64());
        assert_eq!(gap["open"], true);
        assert_eq!(house["line"], "both");
    }

    #[test]
    fn on_the_river_the_executors_fall_leads_down_to_the_water_gap_and_on_the_peak_up_the_mountain() {
        for (line, title, next) in [
            ("peak", "THE ORDERS", "the_ascent"),
            ("river", "DOWNRIVER", "water_gap"),
        ] {
            let mut g = game_with(RIVER_READY);
            g.begin();
            g.start_on(line, 2);
            assert_eq!(g.run.as_ref().unwrap().zone, 2, "{line}: a run from the house");
            fell_the_boss(&mut g);
            let run = g.run.as_mut().unwrap();
            let b = run.world.boss.unwrap();
            if let Some(t) = run.world.enemies[b].tank.as_mut() {
                t.survive = 1;
            }
            frames(&mut g, 400, 0);
            assert_eq!(g.mode, Mode::Interlude, "{line}");
            let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
            assert_eq!(s["interlude"]["title"], title, "{line}");
            g.continue_run();
            let run = g.run.as_ref().unwrap();
            assert_eq!(g.c.zones[run.zone].id, next, "{line}");
        }
        // the river opens no later start of the main line
        let mut g = game_with(RIVER_READY);
        g.begin();
        g.start_on("river", 2);
        fell_the_boss(&mut g);
        let run = g.run.as_mut().unwrap();
        let b = run.world.boss.unwrap();
        if let Some(t) = run.world.enemies[b].tank.as_mut() {
            t.survive = 1;
        }
        frames(&mut g, 400, 0);
        assert_eq!(g.meta.unlocked, 3);
    }

    #[test]
    fn the_water_gap_is_its_four_sections_in_order_then_the_boathouse_and_the_oak() {
        let mut g = game_with(RIVER_READY);
        g.begin();
        let gap = g.c.zones.iter().position(|z| z.id == "water_gap").unwrap();
        g.start_on("river", gap);
        let run = g.run.as_ref().unwrap();
        assert_eq!(run.zone, gap);
        let arch: Vec<&str> = run.route.nodes.iter().map(|n| n.archetype.as_str()).collect();
        assert_eq!(arch, ["spillway", "narrows", "falls", "shallows", "rest", "boss"]);
        assert!(run.route.nodes.iter().all(|n| n.next.len() <= 1), "no side paths");
        assert!(
            run.route.nodes[..4].iter().all(|n| n.reward.is_some()),
            "a strand's reward in each section"
        );
        // a section is crossed, not cleared: its way on is open at once
        assert!(run.world.cleared);
        assert!(run.world.course.is_some());
    }

    #[test]
    fn the_oaks_fall_leads_into_the_anomaly_and_the_eraser_ends_the_river_grey_and_not_as_the_game_beaten() {
        let mut g = game_with(RIVER_READY);
        g.begin();
        let gap = g.c.zones.iter().position(|z| z.id == "water_gap").unwrap();
        g.start_on("river", gap);
        let boss = g.run.as_ref().unwrap().route.boss();
        g.go(boss);
        frames(&mut g, 120, 0);
        g.fell_boss();
        frames(&mut g, 400, 0);
        assert_eq!(g.mode, Mode::Interlude, "the Oak's fall goes on");
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["interlude"]["title"], "THE ANOMALY");
        g.continue_run();
        assert_eq!(g.c.zones[g.run.as_ref().unwrap().zone].id, "far_shore");
        // Dagon's fall: down to the Vault
        let boss = g.run.as_ref().unwrap().route.boss();
        g.go(boss);
        frames(&mut g, 120, 0);
        g.fell_boss();
        frames(&mut g, 400, 0);
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["interlude"]["title"], "THE DESCENT");
        g.continue_run();
        assert_eq!(g.c.zones[g.run.as_ref().unwrap().zone].id, "vault");
        // the Eraser fires: the grey ending
        let boss = g.run.as_ref().unwrap().route.boss();
        g.go(boss);
        frames(&mut g, 120, 0);
        let run = g.run.as_mut().unwrap();
        let ch = run
            .world
            .course
            .as_mut()
            .unwrap()
            .charge
            .as_mut()
            .expect("the Eraser's charge");
        for h in ch.held.iter_mut() {
            *h = ch.need;
        }
        run.world.player.iframes = 100_000;
        frames(&mut g, 400, 0);
        assert_eq!(g.mode, Mode::Won);
        assert_eq!((g.meta.river_wins, g.meta.wins), (1, 0));
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["verdict"]["title"], "THE GREY ENDING");
    }
    #[test]
    fn the_far_shore_starts_once_the_oak_has_burnt_and_the_vault_once_dagon_has_fallen() {
        let mut g = game_with(RIVER_READY);
        g.begin();
        let far = g.c.zones.iter().position(|z| z.id == "far_shore").unwrap();
        let vault = g.c.zones.iter().position(|z| z.id == "vault").unwrap();
        assert!(!g.meta.branch_open(&g.c, far), "not before the Oak");
        g.start_on("river", far);
        assert_ne!(
            g.run.as_ref().unwrap().zone,
            far,
            "a refused start begins in 2031"
        );
        g.meta.beat("gallows_oak");
        assert!(g.meta.branch_open(&g.c, far));
        assert!(!g.meta.branch_open(&g.c, vault), "not before Dagon");
        g.meta.beat("dagon");
        assert!(g.meta.branch_open(&g.c, vault));
    }

    #[test]
    fn the_hurricane_has_its_own_storm_sky_and_the_fleet_the_zones() {
        let mut g = game_with(RIVER_READY);
        g.begin();
        g.meta.beat("gallows_oak");
        let far = g.c.zones.iter().position(|z| z.id == "far_shore").unwrap();
        g.start_on("river", far);
        assert_eq!(g.run.as_ref().unwrap().zone, far);
        let sky = |g: &Game| g.scene_uniforms()[3..6].to_vec();
        let fleet = sky(&g);
        assert_eq!(
            fleet,
            g.c.pal.c("sea_sky").to_vec(),
            "the Fleet under the zone's sky"
        );
        let at = g
            .run
            .as_ref()
            .unwrap()
            .route
            .nodes
            .iter()
            .position(|n| n.archetype == "hurricane")
            .unwrap();
        g.go_to(at);
        assert_eq!(
            sky(&g),
            g.c.pal.c("storm_sky").to_vec(),
            "the Hurricane under a storm"
        );
    }
}
