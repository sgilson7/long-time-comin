//! The door the shim calls through: build the game from the data files, then
//! each display frame hand it the clock, the held keys and the mouse
//! movement, and read back what to draw and what the screens say.
//!
//! Modes: the title, the sentencing (the story's opening), the Holding (the
//! hub between runs), play, the paused and map overlays, the indictment
//! read on a death, and the verdict after the Bailiff. Only play runs the
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
use crate::route::{NodeKind, Reward, Route};
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
    Dead,
    Won,
}

struct Run {
    route: Route,
    at: usize,
    visited: Vec<usize>,
    world: World,
    /// Clemency banked from arenas already left
    earned: i32,
    cleared: i32,
    counted: bool,
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
        let hub = arena::build("arrival", hub_seed, &[], &c.pal);
        let mut g = Game {
            c,
            meta,
            mode: Mode::Title,
            run: None,
            reading: None,
            last: (0, 0),
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

    pub fn start_run(&mut self) {
        if !matches!(self.mode, Mode::Holding | Mode::Dead | Mode::Won) {
            return;
        }
        let route = Route::build(&self.c, self.meta.run_seed());
        let (build, gun_level) = self.meta.build(&self.c);
        let player = Player::new(V3::ZERO, 0, build);
        let held = Held::new(self.c.gun("carbine"), gun_level);
        let start = route.start();
        let world = self.world_for(&route, start, player, held, 0);
        self.run = Some(Run {
            route,
            at: start,
            visited: vec![start],
            world,
            earned: 0,
            cleared: 0,
            counted: false,
        });
        self.reading = None;
        self.mode = Mode::Play;
        self.enter();
    }

    fn world_for(&self, route: &Route, at: usize, player: Player, held: Held, seq: u32) -> World {
        let node = &route.nodes[at];
        let exits: Vec<Exit> = node
            .next
            .iter()
            .enumerate()
            .map(|(k, &to)| Exit {
                to,
                main: k == 0,
                reward: route.nodes[to].reward,
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
            self.die();
        }
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
        let cam = play_camera(&run.world, ONE);
        run.world.tick(&self.c, ctl, cam.eye, cam.fwd);
        let node = &run.route.nodes[run.at];
        if run.world.cleared && !run.counted && node.kind == NodeKind::Fight {
            run.counted = true;
            run.cleared += 1;
            run.earned += if node.detour {
                self.c.route.clemency_detour
            } else {
                self.c.route.clemency_main
            };
        }
        match run.world.outcome {
            Outcome::Playing => {}
            Outcome::Through(to) => self.go(to),
            Outcome::Died => self.die(),
            Outcome::Won => {
                let run = self.run.as_mut().unwrap();
                run.earned += self.c.route.clemency_boss;
                run.cleared += 1;
                let (total, cleared) = (run.total(), run.cleared);
                self.meta.record_win(total, cleared);
                self.last = (total, cleared);
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
        run.world = self.world_for(&run.route, to, player, held, seq);
        let name = self.node_name(&run.route.nodes[to]);
        run.world.note("enter", name);
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
        let (total, cleared) = (run.total(), run.cleared);
        let year = self.c.zones[0].year.clone();
        self.reading = Some(self.meta.record_death(&self.c, total, cleared, &year));
        self.last = (total, cleared);
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

    fn build_scene(&mut self) {
        let arena = match &self.run {
            Some(r) => &r.world.arena,
            None => &self.hub,
        };
        let mut b = Batch::new(self.c.pal.c("ink"));
        for s in &arena.shapes {
            b.shape(s);
        }
        let (values, draws) = b.finish(render::STATIC_BUFFER);
        self.static_values = values;
        self.static_draws = draws;
        self.scene_version += 1;
    }

    fn draw(&mut self) {
        let mut b = Batch::new(self.c.pal.c("ink"));
        let clock = (self.menu_us * TICKS_PER_SECOND as i64 / 1_000_000) as i32;
        let in_run = matches!(
            self.mode,
            Mode::Play | Mode::Paused | Mode::Map | Mode::Dead | Mode::Won
        );
        match (&self.run, in_run) {
            (Some(run), true) => {
                self.camera = play_camera(&run.world, self.alpha);
                let p = &run.world.player;
                let head = p.prev_pos.lerp(p.pos, self.alpha).add(v3(0, CAMERA_UP, 0));
                let body = self.camera.eye.dist(head) > CAMERA_HIDE_BODY;
                draw::world(&mut b, &self.c, &run.world, self.alpha, clock, body);
                if self.mode == Mode::Dead || self.mode == Mode::Won {
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
        for pass in 0..render::PASSES {
            draws.extend_from_slice(&self.static_draws[pass]);
            draws.extend_from_slice(&dyn_draws[pass]);
        }
        self.values = values;
        self.draws = draws;
        self.view = self.camera.values(self.aspect());
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
        let l = v3(-ONE * 4 / 10, ONE * 8 / 10, ONE * 45 / 100).norm();
        let mut u = vec![l.x, l.y, l.z];
        u.extend(self.c.pal.c("sky"));
        u.extend(self.c.pal.c("fog"));
        u.extend([int(70), int(520)]);
        u.extend(self.c.pal.c("light"));
        u.extend(self.c.pal.c("window_dark"));
        u.extend(self.c.pal.c("ink"));
        u
    }

    pub fn clear_color(&self) -> Vec<i32> {
        self.c.pal.c("fog").to_vec()
    }

    // ---- the HUD ---------------------------------------------------------

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
        let boss = w.boss.map(|b| {
            let e = &w.enemies[b];
            let words = c.story.bosses.get(&c.enemies[e.def].id);
            json!({
                "name": words.map(|x| x.name.clone()).unwrap_or_default(),
                "title": words.map(|x| x.title.clone()).unwrap_or_default(),
                "pct": e.health_pct(),
                "phase": e.phase + 1,
            })
        });
        let gates: Vec<Value> = w
            .gates
            .iter()
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
                    "reward": dest.reward.map(|r| self.reward_label(r)).unwrap_or_default(),
                    "path": if g.main { c.say("gate_main") } else { c.say("gate_side") },
                }))
            })
            .collect();
        let prompt = w.offer.and_then(|k| match w.pickups[k].kind {
            PickupKind::Gun(g, l) => Some(
                c.say("prompt_take")
                    .replace("{gun}", &format!("{} {}", c.guns[g].name, roman(l)))
                    .replace("{held}", &format!("{} {}", gun.name, roman(w.held.level))),
            ),
            _ => None,
        });
        let hint = match node.kind {
            NodeKind::Arrival => c.say("hint_arrival"),
            NodeKind::Rest => c.say("hint_rest"),
            NodeKind::Fight if run.at == 1 && self.meta.runs == 0 => c.say("hint_fight"),
            NodeKind::Fight if w.cleared => c.say("hint_cleared"),
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
        // the boss has its own bar; every other enemy gets a marker
        let foes: Vec<Value> = w
            .enemies
            .iter()
            .enumerate()
            .filter(|(i, e)| e.hurtable() && Some(*i) != w.boss)
            .map(|(_, e)| self.marker(e.pos, c.enemies[e.def].radius, e.health_pct()))
            .collect();
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
            "gun_trait": gl.trait_text,
            "gun_colour": hex(c.pal.c(&gun.colour)),
            "grapple": p.build.grapple,
            "grapple_ready": p.build.grapple && p.grapple_cooldown == 0,
            "parry": p.build.parry,
            "parry_ready": p.build.parry && p.parry_cooldown == 0,
            "parrying": p.parry_window > 0,
            "recess": p.recess_left,
            "boss": boss,
            "zone": c.zones[0].name,
            "year": c.zones[0].year,
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
        });
        h.to_string()
    }

    /// Where to mark an enemy on screen: over it, or at the screen's edge
    /// pointing the way to it, with the angle the arrow turns (degrees,
    /// clockwise from pointing right).
    fn marker(&self, at: V3, radius: i32, pct: i32) -> Value {
        let cam = &self.camera;
        let over = at.add(v3(0, radius + ONE / 2, 0));
        let screen = cam.screen(self.aspect(), over);
        if let Some((x, y)) = screen {
            if (0..=100 * ONE).contains(&x) && (0..=100 * ONE).contains(&y) {
                return json!({ "x": x / ONE, "y": y / ONE, "edge": false, "pct": pct, "hurt": pct < 100, "angle": 0 });
            }
        }
        let right = cam.fwd.cross(V3::UP).norm();
        let up = right.cross(cam.fwd);
        let rel = at.sub(cam.eye);
        let (sx, sy) = (rel.dot(right), -rel.dot(up));
        let a = fx::atan2(sy, sx);
        let deg = (a as i64 * 360 / fx::TURN as i64) as i32;
        json!({
            "x": 50 + fx::mul(fx::cos(a), int(44)) / ONE,
            "y": 50 + fx::mul(fx::sin(a), int(40)) / ONE,
            "edge": true,
            "pct": pct,
            "hurt": pct < 100,
            "angle": deg,
        })
    }

    fn node_name(&self, n: &crate::route::Node) -> String {
        let key = match n.kind {
            NodeKind::Arrival => "arrival",
            NodeKind::Rest => "rest",
            NodeKind::Boss => "boss",
            NodeKind::Fight => n.archetype.as_str(),
        };
        self.c.place_name(key)
    }

    fn reward_label(&self, r: Reward) -> String {
        match r {
            Reward::Gun(g, l) => format!("{} {}", self.c.guns[g].name, roman(l)),
            Reward::Jump => self.c.say("reward_jump"),
            Reward::Health => self.c.say("reward_health"),
            Reward::Speed => self.c.say("reward_speed"),
        }
    }

    fn reward_icon(r: Reward) -> &'static str {
        match r {
            Reward::Gun(_, _) => "gun",
            Reward::Jump => "jump",
            Reward::Health => "heart",
            Reward::Speed => "speed",
        }
    }

    // ---- the screens ---------------------------------------------------

    /// Everything the menus show, for the mode the game is in.
    pub fn screen_json(&self) -> String {
        let c = &self.c;
        let m = &self.meta;
        let upgrades: Vec<Value> = c
            .upgrades
            .iter()
            .map(|u| {
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
                    Err(Refusal::Unknown) => String::new(),
                };
                json!({
                    "id": u.id,
                    "name": u.name,
                    "text": u.text,
                    "cost": u.cost,
                    "icon": u.icon,
                    "owned": m.owns(&u.id),
                    "can": m.check(c, &u.id).is_ok(),
                    "why": why,
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
            "build": {
                "health": build.max_health,
                "jumps": build.jumps,
                "dashes": build.dash_charges,
                "speed": build.speed_pct,
                "grapple": build.grapple,
                "parry": build.parry,
                "recess": build.recess,
                "gun": format!("{} {}", c.guns[c.gun("carbine")].name, roman(gun_level)),
            },
            "zones": c.zones.iter().map(|z| json!({
                "name": z.name,
                "year": z.year,
                "built": z.built,
                "boss": c.story.bosses.get(&z.boss).map(|b| b.name.clone()).unwrap_or_default(),
            })).collect::<Vec<_>>(),
            "guns": c.guns.iter().map(|g| json!({ "id": g.id, "name": g.name, "blurb": g.blurb })).collect::<Vec<_>>(),
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
                        "y": if n.lane == 0 { 40 } else { 74 },
                        "kind": match n.kind { NodeKind::Arrival => "arrival", NodeKind::Fight => "fight", NodeKind::Rest => "rest", NodeKind::Boss => "boss" },
                        "name": self.node_name(n),
                        "detour": n.detour,
                        "reward": n.reward.map(|x| self.reward_label(x)).unwrap_or_default(),
                        "icon": n.reward.map(Game::reward_icon).unwrap_or(""),
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
            s["map"] = json!({ "nodes": nodes, "edges": edges, "here": self.node_name(&r.nodes[run.at]) });
            s["run"] = json!({ "clemency": run.total(), "cleared": run.cleared });
        }
        if let Some(rd) = &self.reading {
            s["reading"] = json!({
                "title": c.story.death_title,
                "counts": rd.counts,
                "newest": rd.newest,
                "time_served": c.story.time_served
                    .replace("{years}", &rd.years_added.to_string())
                    .replace("{total}", &rd.total_years.to_string()),
                "clemency": self.last.0,
                "cleared": self.last.1,
            });
        }
        if self.mode == Mode::Won {
            s["verdict"] = json!({
                "title": c.story.victory_title,
                "lines": c.story.victory,
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
    Camera {
        eye: pivot.add(back.scale(dist)),
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
        "uniforms": [["uLightDir", 3], ["uSky", 3], ["uFog", 3], ["uFogRange", 2], ["uLight", 3], ["uWindowDark", 3], ["uInk", 3]],
        "view_values": 16,
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
        assert_eq!(s["clemency"], 7);
        g.to_holding();
        assert_eq!(g.mode, Mode::Holding);
        // 7 Clemency: Fleet I costs 10 and Constitution I costs 8
        assert!(!g.buy("fleet_1"));
        assert!(!g.buy("constitution_1"));
        g.grant(1);
        assert!(g.buy("constitution_1"));
    }

    #[test]
    fn felling_the_bailiff_reads_the_verdict_and_banks_the_boss_clemency() {
        let mut g = game_with(r#"{"seen_intro": true}"#);
        g.begin();
        g.start_run();
        // straight to the court
        let boss = g.run.as_ref().unwrap().route.boss();
        g.go(boss);
        assert_eq!(g.run.as_ref().unwrap().world.kind, NodeKind::Boss);
        frames(&mut g, 120, 0);
        let run = g.run.as_mut().unwrap();
        let b = run.world.boss.expect("the Bailiff came");
        // its last phase, nearly down; the next hit is the last
        run.world.enemies[b].phase = 2;
        run.world.enemies[b].hp = 1;
        run.world.enemies[b].state = crate::enemy::State::Rest(1000);
        run.world.player.iframes = 100_000;
        run.world.enemies[b].hurt(5);
        frames(&mut g, 200, 0);
        assert_eq!(g.mode, Mode::Won);
        assert_eq!(g.meta.wins, 1);
        // zone.json's clemency_boss is 25
        assert!(g.meta.clemency >= 25, "{}", g.meta.clemency);
        let s: Value = serde_json::from_str(&g.screen_json()).unwrap();
        assert_eq!(s["verdict"]["title"], g.c.story.victory_title.as_str());
        g.to_holding();
        assert_eq!(g.mode, Mode::Holding);
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
        ] {
            assert!(g.c.has_copy(key), "copy.json has no {key}");
        }
    }

    #[test]
    fn every_colour_the_core_names_is_in_the_palette() {
        let g = game_with("");
        let src = [
            include_str!("draw.rs"),
            include_str!("arena.rs"),
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
}
