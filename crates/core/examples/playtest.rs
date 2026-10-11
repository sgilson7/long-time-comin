//! A playtest harness: drive the real game core from a terminal, a few
//! commands at a time, and read back what a player would see as text.
//!
//! The core is deterministic, so a session is its command log: each call
//! replays the log from a fresh game, runs the new commands (printing what
//! happens after each), and appends them. Nothing moves between calls, so
//! the player can think as long as they like.
//!
//!     cargo run --release -q -p ltc_core --example playtest -- SESSION_DIR "begin" "intro" "start"
//!     cargo run --release -q -p ltc_core --example playtest -- SESSION_DIR "hold w 2" "status"
//!     cargo run --release -q -p ltc_core --example playtest -- SESSION_DIR help
//!
//! It is a tool, not part of the game: examples may read and write files,
//! the core may not (BUILDER-SPEC R2).

use ltc_core::balance::*;
use ltc_core::fx::{self, int, ONE};
use ltc_core::game::{Game, Mode};
use ltc_core::geom::{v3, yaw_pitch_of, V3};
use ltc_core::player::Controls;
use ltc_core::world::{PickupKind, World};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::PathBuf;

const HELP: &str = "\
Commands (several per call, run in order; time is in seconds of game time):
  help                      this list
  status                    what the screen shows now (also printed after every command)
  begin | intro             title -> sentencing -> Holding (first visit); begin alone on later visits
  holding                   from the indictment or verdict back to the Holding
  shop                      list upgrades, their effects, costs and whether you can buy them
  buy <upgrade_id>          spend Clemency
  start [zone]              start a run (from the Holding); 'start 1' starts on the ice once the Bailiff is beaten
  north                     from the interlude after a zone's boss into the next zone
  codex [guns|enemies|upgrades|rules|controls]   the in-game Codex text
  map                       the run's route: nodes, rewards, where you are
  hold <keys> <s>           hold keys for s seconds. keys: w a s d fire (any mix, e.g. 'hold w d fire 1.5')
  tap <key>                 press once: jump dash parry hook interact wall
  hook <s>                  fire the grappling hook and hold it for s seconds
  turn <deg>                turn the view (positive = left); pitch <deg> tilts it (positive = up)
  aim [n]                   point the view and gun at enemy n from the status list (default: nearest)
  fight <s>                 autopilot for s seconds: aims at the nearest enemy, fires, circle-strafes,
                            dashes from orbs about to hit, jumps ground waves. Stops early if the
                            arena is cleared or you die. You still choose everything else.
  go main|side              walk (with pathfinding) through the open main or side gate
  take                      walk to the nearest pickup and take it (E for guns)
  wait <s>                  stand still
  pause | resume            as Esc and the Resume button
  abandon                   give up the run (counts as a death)
  reset                     wipe the session (a fresh record)
  record <json>             start over from a saved record, e.g. record {\"seen_intro\":true,\"unlocked\":1}
  goto <node name>|boss     straight to a node of this run's map (a measuring shortcut, not play)
  break                     break every weak point the boss has open (to reach a later phase)
  fell                      bring the boss down at once (to see what follows)";

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(dir) = args.next() else {
        eprintln!("usage: playtest SESSION_DIR [commands...]\n{HELP}");
        std::process::exit(2);
    };
    let dir = PathBuf::from(dir);
    fs::create_dir_all(&dir).expect("session dir");
    let log_path = dir.join("log.txt");
    let new: Vec<String> = args.collect();
    if new.iter().any(|c| c == "reset") {
        let _ = fs::remove_file(&log_path);
        println!("session reset");
        return;
    }
    let old: Vec<String> = fs::read_to_string(&log_path)
        .unwrap_or_default()
        .lines()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let (out, log) = session(old, &new);
    fs::write(&log_path, log.join("\n") + "\n").expect("write log");
    println!("{out}");
}

/// One call: replay the log `old`, run the `new` commands, and return what
/// they printed and the log to keep. No files: `main` reads and writes them
/// (and a WebAssembly build can call this from a browser).
pub fn session(old: Vec<String>, new: &[String]) -> (String, Vec<String>) {
    let mut h = Harness::new();
    for c in &old {
        let _ = h.run(c, false);
    }
    h.out.clear();
    let mut kept = Vec::new();
    for c in new {
        h.out.push(format!("$ {c}"));
        match h.run(c, true) {
            Ok(()) => kept.push(c.clone()),
            Err(e) => h.out.push(format!("! {c}: {e}")),
        }
        h.out.push(String::new());
    }
    if new.is_empty() {
        h.status();
    }
    let mut log = old;
    log.extend(kept.into_iter().filter(|c| {
        !matches!(
            c.split_whitespace().next(),
            Some("help" | "status" | "shop" | "codex" | "map")
        )
    }));
    (h.out.join("\n"), log)
}

struct Harness {
    g: Game,
    out: Vec<String>,
    seq: u32,
    t: i32,
    last_hp: i32,
    hits: BTreeMap<String, (i32, i32)>,
}

fn deg(a: i32) -> i32 {
    (a as i64 * 360 / fx::TURN as i64) as i32
}

fn metres(v: i32) -> String {
    format!("{:.1}", v as f64 / ONE as f64)
}

impl Harness {
    fn new() -> Harness {
        Harness::from_record("")
    }

    fn from_record(saved: &str) -> Harness {
        let g = Game::new(
            include_str!("../../../data/weapons.json"),
            include_str!("../../../data/patterns.json"),
            include_str!("../../../data/enemies.json"),
            include_str!("../../../data/zone.json"),
            include_str!("../../../data/upgrades.json"),
            include_str!("../../../data/palette.json"),
            include_str!("../../../data/story.json"),
            include_str!("../../../data/copy.json"),
            saved,
        )
        .expect("data");
        Harness {
            g,
            out: Vec::new(),
            seq: 0,
            t: 0,
            last_hp: 0,
            hits: BTreeMap::new(),
        }
    }

    fn say(&mut self, s: impl Into<String>) {
        self.out.push(s.into());
    }

    fn run(&mut self, cmd: &str, print: bool) -> Result<(), String> {
        let w: Vec<&str> = cmd.split_whitespace().collect();
        let num = |i: usize| -> Result<f64, String> {
            w.get(i)
                .ok_or("missing number")?
                .parse::<f64>()
                .map_err(|e| e.to_string())
        };
        let mode = self.g.mode();
        match w.first().copied().unwrap_or("") {
            "help" => self.say(HELP),
            "status" => {}
            "record" => {
                let json = cmd.trim_start_matches("record").trim();
                let out = std::mem::take(&mut self.out);
                *self = Harness::from_record(json);
                self.out = out;
            }
            "begin" => self.g.begin(),
            "intro" => {
                if print {
                    let s: serde_json::Value = serde_json::from_str(&self.g.screen_json()).unwrap();
                    for l in s["sentencing"].as_array().unwrap() {
                        self.say(format!("  > {}", l.as_str().unwrap_or("")));
                    }
                }
                self.g.finish_intro();
            }
            "holding" => self.g.to_holding(),
            "start" => {
                if mode != Mode::Holding && mode != Mode::Dead && mode != Mode::Won {
                    return Err(format!("start only from the Holding (now {mode:?})"));
                }
                self.g.to_holding();
                let zone = w.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(0);
                self.g.start_run_at(zone);
                self.last_hp = self.g.world().map_or(0, |w| w.player.health);
            }
            "goto" => {
                self.need_play()?;
                let want = w[1..].join(" ");
                let (route, _) = self.g.route().ok_or("no run")?;
                let to = route
                    .nodes
                    .iter()
                    .position(|n| {
                        (want == "boss" && n.kind == ltc_core::route::NodeKind::Boss)
                            || self.g.name_of(n).eq_ignore_ascii_case(&want)
                    })
                    .ok_or("no such node on this map")?;
                self.g.go_to(to);
            }
            "break" => {
                self.need_play()?;
                self.g.break_weak_points();
            }
            "fell" => {
                self.need_play()?;
                self.g.fell_boss();
            }
            "north" => {
                if mode != Mode::Interlude {
                    return Err(format!("north only from the interlude (now {mode:?})"));
                }
                self.g.continue_run();
            }
            "buy" => {
                let id = w.get(1).ok_or("buy what?")?;
                if !self.g.buy(id) {
                    return Err("cannot buy that now (see shop)".into());
                }
            }
            "shop" => self.shop(),
            "codex" => self.codex(w.get(1).copied()),
            "map" => self.map(),
            "pause" => self.g.pause(),
            "resume" => self.g.resume(),
            "abandon" => {
                self.g.pause();
                self.g.abandon();
            }
            "turn" => self.look(num(1)?, 0.0)?,
            "pitch" => self.look(0.0, num(1)?)?,
            "aim" => {
                let n = w.get(1).and_then(|s| s.parse::<usize>().ok());
                self.aim(n)?;
            }
            "wait" => self.play(num(1)?, &[])?,
            "hold" => {
                let secs: f64 = w
                    .last()
                    .ok_or("hold keys seconds")?
                    .parse()
                    .map_err(|_| "hold <keys> <seconds>")?;
                let keys: Vec<&str> = w[1..w.len() - 1].to_vec();
                self.play(secs, &keys)?;
            }
            "tap" => {
                let k = *w.get(1).ok_or("tap what?")?;
                self.tap(k)?;
            }
            "hook" => {
                let secs = num(1)?;
                self.need_play()?;
                let mut first = true;
                let ticks = (secs * 60.0) as i32;
                for _ in 0..ticks.max(1) {
                    let c = Controls {
                        grapple_pressed: first,
                        grapple_held: true,
                        ..Controls::default()
                    };
                    first = false;
                    if !self.step(&c) {
                        break;
                    }
                }
            }
            "fight" => self.fight(num(1)?)?,
            "go" => self.go(w.get(1).copied().unwrap_or("main"))?,
            "take" => self.take()?,
            other => return Err(format!("unknown command '{other}' (try help)")),
        }
        if print && !matches!(w.first().copied(), Some("help" | "shop" | "codex" | "map")) {
            self.status();
        } else if print {
            // still collect the notes so they are not reported twice
            self.drain_notes(false);
        }
        Ok(())
    }

    fn need_play(&self) -> Result<(), String> {
        if self.g.mode() != Mode::Play {
            return Err(format!("not in play (mode {:?})", self.g.mode()));
        }
        Ok(())
    }

    /// One tick of play; false once the arena is left, the run ends or the
    /// game is not in play.
    fn step(&mut self, c: &Controls) -> bool {
        if self.g.mode() != Mode::Play {
            return false;
        }
        let before = self.g.route().map(|(_, at)| at);
        let h0 = self.g.world().map_or(0, |w| w.player.health);
        self.g.tick(c);
        if let Some(w) = self.g.world() {
            let lost = h0 - w.player.health;
            if lost > 0 {
                let who = w
                    .hit_by
                    .and_then(|i| w.enemies.get(i))
                    .map_or("a fall".to_string(), |e| {
                        self.g.content().enemies[e.def].name.clone()
                    });
                let e = self.hits.entry(who).or_insert((0, 0));
                e.0 += 1;
                e.1 += lost;
            }
        }
        self.drain_notes(true);
        self.g.mode() == Mode::Play && self.g.route().map(|(_, at)| at) == before
    }

    fn play(&mut self, secs: f64, keys: &[&str]) -> Result<(), String> {
        self.need_play()?;
        let has = |k: &str| keys.contains(&k);
        let c = Controls {
            move_x: (has("d") as i32 - has("a") as i32) * ONE,
            move_z: (has("w") as i32 - has("s") as i32) * ONE,
            fire: has("fire"),
            ..Controls::default()
        };
        for _ in 0..((secs * 60.0) as i32).max(1) {
            if !self.step(&c) {
                break;
            }
        }
        Ok(())
    }

    fn tap(&mut self, k: &str) -> Result<(), String> {
        self.need_play()?;
        let mut c = Controls::default();
        match k {
            "jump" | "space" => c.jump_pressed = true,
            "dash" | "shift" => {
                c.dash_pressed = true;
                c.move_z = ONE;
            }
            "parry" => c.parry_pressed = true,
            "hook" => {
                c.grapple_pressed = true;
                c.grapple_held = true;
            }
            "interact" | "e" => c.interact_pressed = true,
            "wall" | "r" => c.wall_pressed = true,
            _ => return Err(format!("tap what? ({k})")),
        }
        self.step(&c);
        for _ in 0..5 {
            self.step(&Controls::default());
        }
        Ok(())
    }

    fn look(&mut self, d_yaw: f64, d_pitch: f64) -> Result<(), String> {
        let w = self.g.world_mut().ok_or("no run")?;
        let p = &mut w.player;
        p.look(fx::deg(d_yaw as i32), fx::deg(d_pitch as i32));
        Ok(())
    }

    fn enemies(w: &World) -> Vec<usize> {
        let chest = w.player.chest();
        let mut v: Vec<usize> = (0..w.enemies.len()).filter(|&i| w.enemies[i].alive()).collect();
        v.sort_by_key(|&i| w.enemies[i].pos.sub(chest).len());
        v
    }

    fn aim(&mut self, n: Option<usize>) -> Result<(), String> {
        let w = self.g.world_mut().ok_or("no run")?;
        let list = Self::enemies(w);
        let i = *list.get(n.unwrap_or(0)).ok_or("no such enemy")?;
        let (yaw, pitch) = cam_aim(w, w.enemies[i].pos);
        w.player.yaw = yaw;
        w.player.pitch = pitch;
        Ok(())
    }

    /// The probe's bot, for a few seconds.
    fn fight(&mut self, secs: f64) -> Result<(), String> {
        self.need_play()?;
        let ticks = (secs * 60.0) as i32;
        let mut was = self.g.world().unwrap().player.pos;
        let mut stuck = 0;
        let mut path: Vec<V3> = Vec::new();
        let mut step_i = 0;
        for k in 0..ticks {
            let w = self.g.world().unwrap();
            if w.cleared {
                self.say("  (arena cleared: autopilot stops)");
                break;
            }
            if k % 60 == 0 {
                stuck = if w.player.pos.dist(was) < int(2) {
                    90
                } else {
                    (stuck - 60).max(0)
                };
                was = w.player.pos;
            }
            stuck -= 1;
            let (mut c, yaw, pitch) = bot(self.g.content(), w, self.t, stuck > 0);
            // no line of sight to the target: walk the grid round toward it
            if let Some(e) = w
                .enemies
                .iter()
                .filter(|e| e.hurtable())
                .min_by_key(|e| e.pos.sub(w.player.chest()).len())
            {
                let chest = w.player.chest();
                let to = e.pos.sub(chest);
                let seen = w.arena.level.ray(chest, to.norm(), to.len()).is_none();
                if !seen {
                    if k % 60 == 0 || path.is_empty() {
                        let ground = w.arena.level.floor_below(e.pos.x, e.pos.y, e.pos.z).unwrap_or(0);
                        path = nav(w, w.player.pos, v3(e.pos.x, ground, e.pos.z)).unwrap_or_default();
                        step_i = 0;
                    }
                    while step_i + 1 < path.len() && path[step_i].sub(w.player.pos).len_xz() < ONE * 6 / 10 {
                        step_i += 1;
                    }
                    if let Some(wp) = path.get(step_i) {
                        let dir = v3(wp.x - w.player.pos.x, 0, wp.z - w.player.pos.z).norm();
                        let f = ltc_core::geom::facing(yaw, 0);
                        let r = ltc_core::geom::right_of(yaw);
                        c.move_z = dir.dot(f).signum() * ONE * (dir.dot(f).abs() > ONE / 3) as i32;
                        c.move_x = dir.dot(r).signum() * ONE * (dir.dot(r).abs() > ONE / 3) as i32;
                    }
                } else {
                    path.clear();
                }
            }
            let wm = self.g.world_mut().unwrap();
            wm.player.yaw = yaw;
            wm.player.pitch = pitch;
            if !self.step(&c) {
                break;
            }
        }
        self.flush_hits();
        if let Some(w) = self.g.world() {
            if let Some(b) = w.boss {
                let e = &w.enemies[b];
                self.out.push(format!(
                    "  (the boss: {}% health, phase {})",
                    e.health_pct(),
                    e.phase + 1
                ));
            }
        }
        Ok(())
    }

    fn go(&mut self, which: &str) -> Result<(), String> {
        self.need_play()?;
        let w = self.g.world().unwrap();
        if !w.cleared {
            return Err("the gates are shut until the arena is cleared".into());
        }
        let gate = w
            .gates
            .iter()
            .find(|g| g.to.is_some() && g.main == (which == "main"))
            .ok_or(format!("no {which} gate here"))?
            .pos;
        self.walk_to(gate, 60 * 40)?;
        Ok(())
    }

    fn take(&mut self) -> Result<(), String> {
        self.need_play()?;
        let w = self.g.world().unwrap();
        let p = w.player.pos;
        let target = w
            .pickups
            .iter()
            .filter(|k| !k.taken)
            .min_by_key(|k| k.pos.dist(p))
            .ok_or("no pickup here")?
            .pos;
        self.walk_to(target, 60 * 20)?;
        let c = Controls {
            interact_pressed: true,
            ..Controls::default()
        };
        self.step(&c);
        self.step(&Controls::default());
        Ok(())
    }

    /// Walk a grid path to `goal`, steering waypoint to waypoint.
    fn walk_to(&mut self, goal: V3, max_ticks: i32) -> Result<(), String> {
        let path = {
            let w = self.g.world().unwrap();
            nav(w, w.player.pos, goal).ok_or("no walking path there")?
        };
        let mut i = 0;
        for _ in 0..max_ticks {
            let Some(w) = self.g.world() else { break };
            let p = w.player.pos;
            while i + 1 < path.len() && path[i].sub(p).len_xz() < ONE * 6 / 10 {
                i += 1;
            }
            let target = if i < path.len() { path[i] } else { goal };
            let (yaw, _) = yaw_pitch_of(target.sub(p));
            let wm = self.g.world_mut().unwrap();
            wm.player.yaw = yaw;
            wm.player.pitch = 0;
            let c = Controls {
                move_z: ONE,
                ..Controls::default()
            };
            if !self.step(&c) {
                return Ok(());
            }
            if i + 1 >= path.len() && goal.sub(p).len_xz() < ONE {
                return Ok(());
            }
        }
        Ok(())
    }

    /// Who hurt you since the last report, and how much.
    fn flush_hits(&mut self) {
        let list: Vec<String> = std::mem::take(&mut self.hits)
            .into_iter()
            .map(|(who, (n, d))| format!("{who} {n}x −{d}"))
            .collect();
        if !list.is_empty() {
            self.out.push(format!("  (hurt by: {})", list.join(", ")));
        }
    }

    fn drain_notes(&mut self, keep: bool) {
        let Some(w) = self.g.world() else { return };
        let mut lines = Vec::new();
        let mut hits = 0;
        let mut kills = 0;
        for n in &w.notes {
            if n.seq <= self.seq {
                continue;
            }
            self.seq = n.seq;
            match n.sound {
                "hurt" => hits += 1,
                "kill" => kills += 1,
                _ => {}
            }
            if !n.text.is_empty() {
                lines.push(format!("  [{}] {}", n.sound, n.text));
            }
        }
        if keep {
            let _ = hits;
            if !self.hits.is_empty() && self.g.mode() != Mode::Play {
                self.flush_hits();
            }
            if kills > 0 {
                lines.push(format!("  (killed {kills})"));
            }
            self.out.extend(lines);
        }
    }

    fn status(&mut self) {
        let mode = self.g.mode();
        self.say(format!("mode: {mode:?}   game time {:.1}s", self.t as f64 / 60.0));
        let screen: serde_json::Value = serde_json::from_str(&self.g.screen_json()).unwrap();
        match mode {
            Mode::Title => self.say("Title screen. 'begin' to continue."),
            Mode::Intro => self.say("The sentencing is on screen. 'intro' reads it and continues."),
            Mode::Holding => {
                let b = &screen["build"];
                self.say(format!(
                    "THE HOLDING. Clemency {} | deaths {} | sentence {} years | counts read {}/{} | runs {} wins {}",
                    screen["clemency"], screen["deaths"], screen["years"], screen["counts_read"], screen["counts_total"], screen["runs"], screen["wins"]
                ));
                self.say(format!(
                    "  you start with: {} health, {} jumps, {} dashes, {}% speed, {}{}{}{}",
                    b["health"],
                    b["jumps"],
                    b["dashes"],
                    b["speed"],
                    b["gun"].as_str().unwrap_or(""),
                    if b["grapple"] == true { ", hook" } else { "" },
                    if b["parry"] == true { ", parry" } else { "" },
                    if b["recess"] == true { ", recess" } else { "" }
                ));
                self.say("  'shop' lists upgrades; 'start' begins a run.");
            }
            Mode::Dead => {
                let r = &screen["reading"];
                self.say(format!("THE INDICTMENT ({}):", r["title"].as_str().unwrap_or("")));
                self.say(format!("  newest: {}", r["newest"].as_str().unwrap_or("")));
                self.say(format!("  {}", r["time_served"].as_str().unwrap_or("")));
                self.say(format!(
                    "  run earned {} Clemency, cleared {} arenas. 'holding' to go on.",
                    r["clemency"], r["cleared"]
                ));
            }
            Mode::Won => {
                let v = &screen["verdict"];
                self.say(format!("VERDICT: {}", v["title"].as_str().unwrap_or("")));
                for l in v["lines"].as_array().unwrap() {
                    self.say(format!("  > {}", l.as_str().unwrap_or("")));
                }
            }
            Mode::Interlude => {
                let v = &screen["interlude"];
                self.say(format!("INTERLUDE: {}", v["title"].as_str().unwrap_or("")));
                for l in v["lines"].as_array().unwrap() {
                    self.say(format!("  > {}", l.as_str().unwrap_or("")));
                }
                if let Some(n) = v["newest"].as_str().filter(|s| !s.is_empty()) {
                    self.say(format!("  the court reads: {n}"));
                }
                self.say(format!(
                    "  next: {}. 'north' to go on.",
                    v["next"].as_str().unwrap_or("")
                ));
            }
            Mode::Paused => self.say("Paused. 'resume' to continue."),
            Mode::Map => self.say("Map open. 'resume' to close."),
            Mode::Play => self.play_status(),
        }
    }

    fn play_status(&mut self) {
        let g = &self.g;
        let w = g.world().unwrap();
        let (route, at) = g.route().unwrap();
        let p = &w.player;
        let c = g.content();
        let gun = &c.guns[w.held.gun];
        let hud: serde_json::Value = serde_json::from_str(&g.hud_json()).unwrap();
        let mut lines = vec![format!(
            "ARENA {} ({}) {} | health {}/{} | jumps {}/{} | dashes {}/{} | gun {} {} | Clemency this run {}",
            g.name_of(&route.nodes[at]),
            route.nodes[at].archetype,
            if w.cleared { "CLEARED" } else { "FIGHTING" },
            p.health,
            p.build.max_health,
            (p.build.jumps - p.jumps_used).max(0),
            p.build.jumps,
            p.dash_left,
            p.build.dash_charges,
            gun.name,
            hud["gun_level"].as_str().unwrap_or(""),
            hud["clemency"]
        )];
        lines.push(format!(
            "  you: at ({}, {}, {}) m, facing {}°, {}{}{}",
            metres(p.pos.x),
            metres(p.pos.y),
            metres(p.pos.z),
            deg(p.yaw),
            if p.grounded { "on the ground" } else { "in the air" },
            if p.build.grapple {
                if p.grapple_cooldown == 0 {
                    ", hook ready"
                } else {
                    ", hook cooling"
                }
            } else {
                ""
            },
            if p.build.parry {
                if p.parry_cooldown == 0 {
                    ", parry ready"
                } else {
                    ", parry cooling"
                }
            } else {
                ""
            },
        ));
        if let Some(b) = w.boss {
            let e = &w.enemies[b];
            lines.push(format!(
                "  BOSS {}: {}% health, phase {}",
                c.enemies[e.def].name,
                e.health_pct(),
                e.phase + 1
            ));
        }
        let chest = p.chest();
        let list = Self::enemies(w);
        if !list.is_empty() {
            lines.push(format!(
                "  enemies ({} alive, {} more to come):",
                list.len(),
                w.waves_left.iter().map(|v| v.len()).sum::<usize>()
            ));
        }
        for (n, &i) in list.iter().enumerate().take(8) {
            let e = &w.enemies[i];
            let to = e.pos.sub(chest);
            let (yaw, _) = yaw_pitch_of(to);
            let rel = deg(fx::wrap(yaw - p.yaw));
            let seen = w.arena.level.ray(chest, to.norm(), to.len()).is_none();
            let state = match e.state {
                ltc_core::enemy::State::Spawning(_) => "appearing",
                ltc_core::enemy::State::Windup(_) => "WINDING UP",
                ltc_core::enemy::State::Casting(_) => "FIRING",
                ltc_core::enemy::State::Rest(_) => "resting",
                ltc_core::enemy::State::Changing(_) => "changing phase",
                ltc_core::enemy::State::Ready(_) => "READY (waiting for its partner)",
            };
            lines.push(format!(
                "   {n}: {} {}hp/{} {} m away, bearing {}° ({}), {} m {} you, {}{}",
                c.enemies[e.def].name,
                e.hp,
                e.max_hp,
                metres(to.len_xz()),
                rel,
                if rel.abs() < 20 {
                    "ahead"
                } else if rel > 0 {
                    "left"
                } else {
                    "right"
                },
                metres(to.y.abs()),
                if to.y >= 0 { "above" } else { "below" },
                state,
                if seen { "" } else { ", behind cover" }
            ));
        }
        // the orbs that matter: how many, and how many will pass within 2 m in the next second
        let mut close = 0;
        let mut soon = 0;
        let mut by_side: BTreeMap<&str, i32> = BTreeMap::new();
        for o in &w.orbs {
            let d = o.pos.dist(chest);
            if d < int(10) {
                close += 1;
            }
            let ahead = o.pos.add(o.dir.scale(o.speed * 60));
            let closest = closest_on_segment(o.pos, ahead, chest);
            if closest < int(2) {
                soon += 1;
                let (yaw, _) = yaw_pitch_of(o.pos.sub(chest));
                let rel = deg(fx::wrap(yaw - p.yaw));
                let side = if rel.abs() < 45 {
                    "front"
                } else if rel.abs() > 135 {
                    "behind"
                } else if rel > 0 {
                    "left"
                } else {
                    "right"
                };
                *by_side.entry(side).or_default() += 1;
            }
        }
        if !w.orbs.is_empty() {
            lines.push(format!(
                "  orbs: {} in the air, {} within 10 m, {} on course to pass within 2 m in the next second {:?}",
                w.orbs.len(),
                close,
                soon,
                by_side
            ));
        }
        for wv in &w.waves {
            let d = p.pos.sub(wv.centre).len_xz();
            if d > wv.radius {
                lines.push(format!(
                    "  GROUND WAVE spreading toward you: reaches you in {:.1}s",
                    (d - wv.radius) as f64 / wv.speed.max(1) as f64 / 60.0
                ));
            }
        }
        for k in w.pickups.iter().filter(|k| !k.taken) {
            let what = match k.kind {
                PickupKind::Heal(n) => format!("heal +{n}"),
                PickupKind::MaxHealth(n) => format!("max health +{n}"),
                PickupKind::Speed(n) => format!("speed +{n}%"),
                PickupKind::Jump => "extra jump".to_string(),
                PickupKind::Gun(gi, l) => format!("gun: {} level {l} (press E: 'take')", c.guns[gi].name),
                PickupKind::Pierce => "Piercing Rounds".to_string(),
                PickupKind::Pulse => "Dash Pulse".to_string(),
                PickupKind::Mend => "Mending Parry".to_string(),
                PickupKind::Trigger => "Hair Trigger".to_string(),
                PickupKind::Plating => "Plating".to_string(),
            };
            let what = if k.group != 0 {
                format!("{what} [a choice: 'take' it with E; the other is lost]")
            } else {
                what
            };
            lines.push(format!(
                "  pickup: {what}, {} m away",
                metres(k.pos.sub(p.pos).len_xz())
            ));
        }
        if w.cleared {
            for gt in w.gates.iter().filter(|gt| gt.to.is_some()) {
                let to = gt.to.unwrap();
                let n = &route.nodes[to];
                let reward = n
                    .reward
                    .map(|r| format!(" — reward: {}", g.reward_label(r)))
                    .unwrap_or_default();
                lines.push(format!(
                    "  gate ({}): to {}{} ('go {}')",
                    if gt.main { "main" } else { "side" },
                    g.name_of(n),
                    reward,
                    if gt.main { "main" } else { "side" }
                ));
            }
        }
        if let Some(t) = hud["prompt"].as_str() {
            lines.push(format!("  prompt: {t}"));
        }
        if let Some(t) = hud["hint"].as_str().filter(|t| !t.is_empty()) {
            lines.push(format!("  hint on screen: {t}"));
        }
        self.last_hp = p.health;
        self.out.extend(lines);
    }

    fn shop(&mut self) {
        let s: serde_json::Value = serde_json::from_str(&self.g.screen_json()).unwrap();
        self.say(format!("SHOP (Clemency {}):", s["clemency"]));
        for u in s["upgrades"].as_array().unwrap() {
            let state = if u["owned"] == true {
                "OWNED".to_string()
            } else if u["can"] == true {
                format!("buy for {}", u["cost"])
            } else {
                format!("{} ({})", u["cost"], u["why"].as_str().unwrap_or(""))
            };
            self.say(format!(
                "  {} [{}] {} — {}",
                u["name"].as_str().unwrap_or(""),
                u["id"].as_str().unwrap_or(""),
                state,
                u["text"].as_str().unwrap_or("")
            ));
        }
    }

    fn codex(&mut self, part: Option<&str>) {
        let v: serde_json::Value = serde_json::from_str(&self.g.codex_json()).unwrap();
        let part = part.unwrap_or("controls");
        let pretty = serde_json::to_string_pretty(&v[part]).unwrap_or_default();
        self.say(pretty);
    }

    fn map(&mut self) {
        let Some((route, at)) = self.g.route() else {
            self.say("no run");
            return;
        };
        let mut lines = Vec::new();
        for (i, n) in route.nodes.iter().enumerate() {
            let next: Vec<String> = n.next.iter().map(|j| j.to_string()).collect();
            lines.push(format!(
                "  {i}{} {} {:?} tier {}{}{} -> [{}]",
                if i == at { " (YOU)" } else { "" },
                self.g.name_of(n),
                n.kind,
                n.tier,
                if n.detour { " side path" } else { "" },
                n.reward
                    .map(|r| format!(" reward: {}", self.g.reward_label(r)))
                    .unwrap_or_default(),
                next.join(", ")
            ));
        }
        self.out.extend(lines);
    }
}

/// The yaw and pitch that put the crosshair on `target`: shots follow the
/// camera's ray, which sits above and right of the chest, so aim from the
/// eye, a few times over as the eye moves with the aim.
fn cam_aim(w: &World, target: V3) -> (i32, i32) {
    let mut wc = w.clone();
    let (mut y, mut pt) = yaw_pitch_of(target.sub(w.player.chest()));
    for _ in 0..4 {
        wc.player.yaw = y;
        wc.player.pitch = pt.clamp(PITCH_MIN, PITCH_MAX);
        let eye = ltc_core::game::play_camera(&wc, ONE).eye;
        let r = yaw_pitch_of(target.sub(eye));
        y = r.0;
        pt = r.1;
    }
    (y, pt.clamp(PITCH_MIN, PITCH_MAX))
}

fn closest_on_segment(a: V3, b: V3, p: V3) -> i32 {
    let ab = b.sub(a);
    let l2 = ab.dot(ab);
    if l2 == 0 {
        return p.dist(a);
    }
    let t = fx::div(p.sub(a).dot(ab), l2).clamp(0, ONE);
    a.add(ab.scale(t)).dist(p)
}

/// The run probe's bot (crates/core/tests/run_probe.rs).
fn bot(c: &ltc_core::content::Content, w: &World, t: i32, stuck: bool) -> (Controls, i32, i32) {
    let p = &w.player;
    let chest = p.chest();
    let mut ctl = Controls::default();
    let (mut yaw, mut pitch) = (p.yaw, p.pitch);
    // a shield facing you is passed over for another target while one is up
    let shielded = |e: &&ltc_core::enemy::Enemy| {
        let d = &c.enemies[e.def];
        d.armour > 0 && chest.sub(e.pos).norm().dot(ltc_core::geom::facing(e.yaw, 0)) > fx::cos(d.armour)
    };
    let target = w
        .enemies
        .iter()
        .filter(|e| e.hurtable())
        .min_by_key(|e| e.pos.sub(chest).len() + if shielded(e) { int(40) } else { 0 });
    if let Some(e) = target {
        let flank = shielded(&e);
        // a tank: its nearest weak point, else round behind its turret
        let mut aim_at = e.pos;
        let mut behind: Option<V3> = None;
        if let Some(tk) = e.tank.as_ref() {
            let d = &c.enemies[e.def];
            let weak = ltc_core::tank::weak_points(e, d);
            if let Some((at, _, _)) = weak.iter().min_by_key(|(at, _, _)| at.sub(chest).len()) {
                aim_at = *at;
            }
            behind = Some(e.pos.add(ltc_core::geom::facing(tk.turret, 0).scale(-int(14))));
        }
        let (y, pt) = cam_aim(w, aim_at);
        yaw = y;
        pitch = pt;
        ctl.fire = true;
        let to = e.pos.sub(chest);
        let d = to.len_xz();
        let seen = w.arena.level.ray(chest, to.norm(), to.len()).is_none();
        ctl.move_x = if (t / 150) % 2 == 0 { ONE } else { -ONE };
        ctl.move_z = if !seen || d > int(16) {
            ONE
        } else if d < int(9) {
            -ONE
        } else {
            0
        };
        // round a shield: run hard to one side, dashing
        if flank {
            ctl.move_x = if (t / 240) % 2 == 0 { ONE } else { -ONE };
            ctl.move_z = if d > int(8) { ONE } else { 0 };
            ctl.dash_pressed = t % 50 == 0;
        }
        // round behind the tank's turret: strafe toward that side
        if let Some(b) = behind {
            let r = ltc_core::geom::right_of(yaw);
            let side = b.sub(p.pos).dot(r);
            ctl.move_x = if side > 0 { ONE } else { -ONE };
            ctl.move_z = if d < int(10) {
                -ONE
            } else if d > int(18) {
                ONE
            } else {
                0
            };
        }
        // a saw gun throws on a press and revs while held
        if c.guns[w.held.gun].level(w.held.level).saw.is_some() {
            ctl.fire_pressed = t % 12 == 0;
        }
        // a ground pound onto what is near below
        if p.build.pound > 0 && !p.grounded && p.vel.y < 0 && p.pos.y > int(2) && d < int(6) {
            ctl.pound_pressed = true;
        }
    }
    for o in &w.orbs {
        let soon = o.pos.add(o.dir.scale(o.speed * 8));
        if soon.dist(chest) < int(2) || o.pos.dist(chest) < int(2) {
            ctl.dash_pressed = true;
            ctl.move_x = if (t / 150) % 2 == 0 { ONE } else { -ONE };
            break;
        }
    }
    // the Injunction: blue orbs coming at you from in front
    if p.build.wall && p.wall_cooldown == 0 {
        let f = ltc_core::geom::facing(yaw, 0);
        let coming = w.orbs.iter().any(|o| {
            let rel = o.pos.sub(chest);
            o.blue && o.speed > 0 && rel.dot(f) > 0 && rel.len() < int(7) && o.dir.dot(rel.norm()) < -ONE / 2
        });
        ctl.wall_pressed = coming;
    }
    if stuck {
        ctl.jump_pressed = t % 20 == 0;
        ctl.move_x = if (t / 90) % 2 == 0 { ONE } else { -ONE };
    }
    for wv in &w.waves {
        let d = p.pos.sub(wv.centre).len_xz();
        if d > wv.radius && d - wv.radius < wv.speed * 10 && p.grounded {
            ctl.jump_pressed = true;
        }
    }
    (ctl, yaw, pitch)
}

/// A walking path over a half-metre grid, as the body moves (step up to
/// STEP_UP, drop down, no jumps), from `from` to the cell nearest `to`.
fn nav(w: &World, from: V3, to: V3) -> Option<Vec<V3>> {
    let l = &w.arena.level;
    let grid = ONE / 2;
    let snap = |v: i32| (v + if v >= 0 { grid / 2 } else { -grid / 2 }) / grid * grid;
    let fits = |p: V3| {
        let r = PLAYER_RADIUS;
        !l.blocked(
            v3(p.x - r, p.y + ONE / 20, p.z - r),
            v3(p.x + r, p.y + PLAYER_HEIGHT, p.z + r),
        )
    };
    let start = v3(snap(from.x), from.y, snap(from.z));
    let mut prev: BTreeMap<(i32, i32, i32), (i32, i32, i32)> = BTreeMap::new();
    let mut q = VecDeque::new();
    q.push_back(start);
    prev.insert((start.x, start.y, start.z), (start.x, start.y, start.z));
    let mut best = start;
    while let Some(p) = q.pop_front() {
        if p.sub(to).len_xz() < best.sub(to).len_xz() {
            best = p;
        }
        if p.sub(to).len_xz() < ONE && (p.y - to.y).abs() < int(2) {
            best = p;
            break;
        }
        for (dx, dz) in [(grid, 0), (-grid, 0), (0, grid), (0, -grid)] {
            let (nx, nz) = (p.x + dx, p.z + dz);
            if nx.abs() > int(45) || nz.abs() > int(45) {
                continue;
            }
            let r = PLAYER_RADIUS;
            let (lo, hi) = (v3(nx - r, -int(10), nz - r), v3(nx + r, p.y + STEP_UP, nz + r));
            let mut top: Option<i32> = None;
            l.each_near(lo, hi, |s| {
                let over = s.min.x < hi.x && s.max.x > lo.x && s.min.z < hi.z && s.max.z > lo.z;
                if over && s.max.y <= p.y + STEP_UP {
                    top = Some(top.map_or(s.max.y, |t: i32| t.max(s.max.y)));
                }
            });
            let Some(y) = top else { continue };
            let n = v3(nx, y, nz);
            if y < p.y - int(6) || !fits(n) {
                continue;
            }
            let key = (n.x, n.y, n.z);
            if let std::collections::btree_map::Entry::Vacant(e) = prev.entry(key) {
                e.insert((p.x, p.y, p.z));
                q.push_back(n);
            }
        }
    }
    let mut path = vec![best];
    let mut k = (best.x, best.y, best.z);
    while let Some(&pk) = prev.get(&k) {
        if pk == k {
            break;
        }
        path.push(v3(pk.0, pk.1, pk.2));
        k = pk;
    }
    path.reverse();
    Some(path)
}
