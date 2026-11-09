//! The diver: the Defendant drawn as Style Lab's skinned mesh, animated by
//! its baked clips (`data/clips.json`, written by `tools/mesh/bake_diver.py`).
//!
//! Every choice is here and the page draws what it is given. Each world tick
//! `Diver::observe` reads the state the core already keeps (the player, the
//! held gun, this tick's events and sounds) and picks two clips: one for the
//! lower body (the gait or the move) and one for the upper body (the gun's
//! hold, fire or reload, or the move). The page gets one record a frame: two
//! pairs of animation rows with a blend each, the model matrix, a bend for
//! the aim, and the grey and hurt mixes. Nothing here feeds back into the
//! world: the diver is drawn only.
//!
//! Events that start a clip ("just cast", "a landing") are held for the
//! clip's length in this module's own countdowns, never in the world.

use crate::balance::*;
use crate::content::Content;
use crate::fx::{self, int, ONE, TURN};
use crate::geom::{v3, Affine, V3};
use crate::player::{Event, Player};
use crate::world::World;
use serde_json::Value;

/// Values in a record.
pub const VALUES: usize = 36;
/// Where each part of a record starts: shown (ONE), the lower rows (row,
/// next row, blend), the upper rows, the model matrix (four columns of
/// three), the aim's bend (four columns of three, in the figure's rest
/// frame, applied to the upper body), how grey, how hurt, and the grey it
/// goes to (the Eraser's).
pub const SHOWN: usize = 0;
pub const LOWER: usize = 1;
pub const UPPER: usize = 4;
pub const MATRIX: usize = 7;
pub const BEND: usize = 19;
pub const GREY: usize = 31;
pub const HURT: usize = 32;
pub const GREY_RGB: usize = 33;

/// Clips are keyed at 30 frames a second; the core ticks 60 times a second.
pub const TICKS_PER_FRAME: i32 = 2;

/// Where the right hand holds a gun in one frame, in the figure's frame
/// (x right, y up, −z ahead).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grip {
    pub at: V3,
    pub muzzle: V3,
    pub up: V3,
}

#[derive(Debug, Clone)]
pub struct Clip {
    pub name: String,
    pub frames: i32,
    pub looped: bool,
    pub row: i32,
    pub prop: String,
    pub grip: Vec<Grip>,
    /// a gait's ground per cycle, Q16 metres (0 for other clips)
    pub stride: i32,
}

#[derive(Debug, Clone)]
pub struct Clips {
    pub clips: Vec<Clip>,
    /// where the upper body bends for the aim, in the figure's frame
    pub pivot: V3,
}

fn mm(v: &Value) -> i32 {
    (v.as_i64().unwrap_or(0) * ONE as i64 / 1000) as i32
}

fn unit(v: &Value) -> i32 {
    (v.as_i64().unwrap_or(0) * ONE as i64 / 10000) as i32
}

impl Clips {
    /// The clip table compiled into the core.
    pub fn load() -> Clips {
        Clips::parse(include_str!("../../../data/clips.json")).expect("data/clips.json")
    }

    pub fn parse(text: &str) -> Result<Clips, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let p = &v["pivot"];
        let pivot = v3(mm(&p[0]), mm(&p[1]), mm(&p[2]));
        let mut clips = Vec::new();
        for c in v["clips"].as_array().ok_or("no clips")? {
            let grip = c["grip"]
                .as_array()
                .map(|g| {
                    g.iter()
                        .map(|f| Grip {
                            at: v3(mm(&f[0]), mm(&f[1]), mm(&f[2])),
                            muzzle: v3(unit(&f[3]), unit(&f[4]), unit(&f[5])),
                            up: v3(unit(&f[6]), unit(&f[7]), unit(&f[8])),
                        })
                        .collect()
                })
                .unwrap_or_default();
            clips.push(Clip {
                name: c["name"].as_str().ok_or("a clip without a name")?.to_string(),
                frames: c["frames"].as_i64().unwrap_or(1).max(1) as i32,
                looped: c["loop"].as_bool().unwrap_or(false),
                row: c["row"].as_i64().unwrap_or(0) as i32,
                prop: c["prop"].as_str().unwrap_or("").to_string(),
                grip,
                stride: mm(&c["stride_mm"]),
            });
        }
        Ok(Clips { clips, pivot })
    }

    /// A clip's number by name; the first clip when the name is unknown.
    pub fn id(&self, name: &str) -> usize {
        self.clips.iter().position(|c| c.name == name).unwrap_or(0)
    }

    pub fn has(&self, name: &str) -> bool {
        self.clips.iter().any(|c| c.name == name)
    }

    pub fn frames(&self, clip: usize) -> i32 {
        self.clips[clip].frames
    }

    /// A clip's whole frame `f`, wrapped when it loops, held on its last
    /// frame when it does not.
    fn whole(&self, clip: usize, f: i32) -> i32 {
        let c = &self.clips[clip];
        if c.looped {
            f.rem_euclid(c.frames)
        } else {
            f.clamp(0, c.frames - 1)
        }
    }

    /// The animation row of a clip's frame (Q16 frames).
    pub fn row(&self, clip: usize, frame: i32) -> i32 {
        self.clips[clip].row + self.whole(clip, frame.div_euclid(ONE))
    }

    /// The two rows a frame (Q16) falls between, and how far between.
    pub fn rows(&self, clip: usize, frame: i32) -> (i32, i32, i32) {
        let w = frame.div_euclid(ONE);
        let c = &self.clips[clip];
        (
            c.row + self.whole(clip, w),
            c.row + self.whole(clip, w + 1),
            frame.rem_euclid(ONE),
        )
    }

    /// Where the right hand holds a gun in a clip's frame (Q16 frames).
    pub fn grip(&self, clip: usize, frame: i32) -> Option<Grip> {
        let w = self.whole(clip, frame.div_euclid(ONE));
        self.clips[clip].grip.get(w as usize).copied()
    }

    /// The frame (Q16) a gait shows at the body's stride (`Player::stride`:
    /// a turn is `STRIDE_METRES` of ground), so the planted foot keeps pace
    /// with the ground.
    pub fn by_stride(&self, clip: usize, stride: i32) -> i32 {
        let c = &self.clips[clip];
        if c.stride <= 0 {
            return 0;
        }
        let ground = int(STRIDE_METRES) as i64;
        let cycle = c.frames as i64 * ONE as i64;
        let f = stride.rem_euclid(TURN) as i64 * ground / c.stride as i64 * cycle / TURN as i64;
        f.rem_euclid(cycle) as i32
    }
}

/// When a track's frame comes from: the clip's own clock since it began, or
/// a frame chosen from state (Q16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum When {
    Clock,
    At(i32),
}

/// What one layer wants this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Want {
    pub clip: usize,
    pub when: When,
    /// start it over even if it is already playing (a shot fired again)
    pub again: bool,
}

/// One layer's clip, since when, and the clip it is fading from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Track {
    pub clip: usize,
    /// the tick it began
    pub start: i32,
    pub when: When,
    /// the clip and frame (Q16) it fades from, and the tick the fade began
    pub from: Option<(usize, i32)>,
    pub faded: i32,
}

impl Track {
    fn new(clip: usize) -> Track {
        Track {
            clip,
            start: 0,
            when: When::Clock,
            from: None,
            faded: 0,
        }
    }

    /// The frame (Q16) at `now` (Q16 ticks).
    pub fn frame(&self, now: i32) -> i32 {
        match self.when {
            When::Clock => (now - self.start * ONE).max(0) / TICKS_PER_FRAME,
            When::At(f) => f,
        }
    }

    fn set(&mut self, w: Want, tick: i32) {
        if w.clip != self.clip || w.again {
            let at = self.frame(tick * ONE);
            self.from = Some((self.clip, at));
            self.faded = tick;
            self.clip = w.clip;
            self.start = tick;
        }
        self.when = w.when;
    }

    /// The two rows and the blend for the page at `now` (Q16 ticks): while
    /// fading, the old clip's frame to the new one's; else this frame to the
    /// next.
    pub fn rows(&self, clips: &Clips, now: i32) -> (i32, i32, i32) {
        let f = self.frame(now);
        let fading = now - self.faded * ONE;
        match self.from {
            Some((old, at)) if fading < DIVER_FADE_TICKS * ONE => (
                clips.row(old, at),
                clips.row(self.clip, f),
                fading / DIVER_FADE_TICKS,
            ),
            _ => clips.rows(self.clip, f),
        }
    }
}

/// Countdowns, in ticks, for clips an event starts and holds for their
/// length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Held {
    pub land: i32,
    pub pound: i32,
    pub hook: i32,
    pub wall: i32,
    pub stop: i32,
    pub bench: i32,
    pub hurt: i32,
    pub recess: i32,
    pub fire: i32,
    pub throw: i32,
    pub air_jump: i32,
}

impl Held {
    fn tick(&mut self) {
        for t in [
            &mut self.land,
            &mut self.pound,
            &mut self.hook,
            &mut self.wall,
            &mut self.stop,
            &mut self.bench,
            &mut self.hurt,
            &mut self.recess,
            &mut self.fire,
            &mut self.throw,
            &mut self.air_jump,
        ] {
            *t = (*t - 1).max(0);
        }
    }
}

/// The diver's own animation state, kept by the game beside the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diver {
    /// world ticks observed
    pub ticks: i32,
    pub lower: Track,
    pub upper: Track,
    pub held: Held,
    /// the last world sound read
    seq: u32,
    /// the last shot was the second fire
    alt: bool,
    /// a shot, a jump or a cast started this tick (start its clip again)
    fresh: bool,
    jumped: bool,
}

/// What the page and the gun's drawing need from the diver this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    pub record: Vec<i32>,
    /// where the gun is held, in the world: its grip, muzzle and up
    pub grip: Option<Grip>,
    /// whether the gun leaves the aim (a reload, the fan, a throw, a swim):
    /// then it is turned as the hand holds it, else along the aim
    pub free: bool,
}

fn is_gun(c: &Content, prop: &str) -> bool {
    c.guns.iter().any(|g| g.id == prop)
}

impl Diver {
    pub fn new(clips: &Clips) -> Diver {
        let idle = clips.id("holding_idle");
        Diver {
            ticks: 0,
            lower: Track::new(idle),
            upper: Track::new(idle),
            held: Held::default(),
            seq: 0,
            alt: false,
            fresh: false,
            jumped: false,
        }
    }

    fn hold_for(&self, clips: &Clips, name: &str) -> i32 {
        clips.frames(clips.id(name)) * TICKS_PER_FRAME
    }

    /// Read one world tick: its events start countdowns, and the state picks
    /// each layer's clip.
    pub fn observe(&mut self, clips: &Clips, c: &Content, w: &World) {
        self.ticks += 1;
        self.held.tick();
        self.fresh = false;
        self.jumped = false;
        let p = &w.player;
        for ev in &p.events {
            match ev {
                Event::Land => self.held.land = self.hold_for(clips, "land"),
                Event::Pound => self.held.pound = self.hold_for(clips, "pound_land"),
                Event::GrappleOn => self.held.hook = self.hold_for(clips, "hook_fire"),
                Event::Wall => self.held.wall = self.hold_for(clips, "wall"),
                Event::Stop => self.held.stop = self.hold_for(clips, "time_stop"),
                Event::Bench => self.held.bench = self.hold_for(clips, "bench"),
                Event::Hurt(_) => self.held.hurt = self.hold_for(clips, "hurt"),
                Event::Recess => self.held.recess = self.hold_for(clips, "recess"),
                Event::Jump(n) => {
                    self.jumped = true;
                    if *n > 1 {
                        self.held.air_jump = self.hold_for(clips, "air_jump");
                    }
                }
                _ => {}
            }
        }
        let gun = &c.guns[w.held.gun].id;
        let fire = clips.id(&format!("{gun}_fire"));
        let newest = w.notes.iter().map(|n| n.seq).max().unwrap_or(0);
        if newest < self.seq {
            // a new world counts its sounds from the start again
            self.seq = 0;
        }
        for n in w.notes.iter().filter(|n| n.seq > self.seq) {
            match n.sound {
                s if s.starts_with("fire_") || s == "saw_throw" => {
                    self.alt = w.held.fan > 0;
                    self.held.fire = clips.frames(fire) * TICKS_PER_FRAME;
                    self.fresh = true;
                }
                "alt_fire" | "cloture" | "redaction" => {
                    self.alt = true;
                    self.held.fire = clips.frames(self.alt_clip(clips, c, w)) * TICKS_PER_FRAME;
                    self.fresh = true;
                }
                "throw" => self.held.throw = self.hold_for(clips, "throw"),
                _ => {}
            }
        }
        self.seq = newest.max(self.seq);
        let (lower, upper) = self.choose(clips, c, w);
        self.lower.set(lower, self.ticks);
        self.upper.set(upper, self.ticks);
    }

    fn alt_clip(&self, clips: &Clips, c: &Content, w: &World) -> usize {
        let gun = &c.guns[w.held.gun].id;
        let alt = match gun.as_str() {
            "carbine" => "carbine_burst",
            "big_iron" => "big_iron_fan",
            _ => "",
        };
        if clips.has(alt) {
            clips.id(alt)
        } else {
            clips.id(&format!("{gun}_fire"))
        }
    }

    /// The two layers' clips, from the first row of the state table that
    /// holds (the table is in `LTC-GUIDE.md`, step 2, and SECOND-ORDER D-rows
    /// say where it was read another way).
    pub fn choose(&self, clips: &Clips, c: &Content, w: &World) -> (Want, Want) {
        let p = &w.player;
        let on = |name: &str| Want {
            clip: clips.id(name),
            when: When::Clock,
            again: false,
        };
        let both = |x: Want| (x, x);
        if !p.alive() {
            return both(on("death"));
        }
        if w.song.is_some() {
            return (self.legs(clips, p), on("trumpet"));
        }
        if self.held.recess > 0 {
            return both(on("recess"));
        }
        if p.swimming {
            // going down unstroked, it sinks; else it swims
            let name = if p.vel.y < -DIVER_SINKING { "sink" } else { "swim" };
            return both(on(name));
        }
        if p.pounding {
            return both(on("pound_drop"));
        }
        if self.held.pound > 0 {
            return both(on(if p.grounded { "pound_land" } else { "bounce" }));
        }
        if p.dashing() {
            return both(on("dash"));
        }
        if p.grapple.is_some() {
            let upper = if self.held.hook > 0 {
                on("hook_fire")
            } else {
                on("hook_pull")
            };
            return (on("hook_pull"), upper);
        }
        if self.held.bench > 0 {
            return both(on("bench"));
        }
        let legs = self.legs(clips, p);
        if legs.clip == clips.id("air_jump") {
            return both(legs);
        }
        let upper = if self.held.hurt > 0 {
            on("hurt")
        } else if p.parry_window > 0 {
            on("parry")
        } else if self.held.wall > 0 {
            on("wall")
        } else if self.held.stop > 0 {
            on("time_stop")
        } else if p.slash_left > 0 {
            on(if p.slash_lunge { "lunge_slash" } else { "slash" })
        } else if w.grabbed.is_some() {
            let grab = self.hold_for(clips, "grab");
            on(if w.grab_age < grab { "grab" } else { "hold_grabbed" })
        } else if self.held.throw > 0 {
            on("throw")
        } else {
            self.gun(clips, c, w)
        };
        (legs, upper)
    }

    /// The lower body: in the air, landing, the gait or standing.
    fn legs(&self, clips: &Clips, p: &Player) -> Want {
        let on = |name: &str, again: bool| Want {
            clip: clips.id(name),
            when: When::Clock,
            again,
        };
        if !p.grounded {
            if self.held.air_jump > 0 {
                return on("air_jump", self.jumped);
            }
            if p.vel.y > DIVER_APEX {
                return on("jump", self.jumped);
            }
            return on(if p.vel.y < -DIVER_APEX { "fall" } else { "air" }, false);
        }
        if self.held.land > 0 {
            return on("land", false);
        }
        let v = p.vel;
        if v.len_xz() > DIVER_STILL {
            let ahead = crate::geom::facing(p.yaw, 0);
            let right = crate::geom::right_of(p.yaw);
            let (f, r) = (v.dot(ahead), v.dot(right));
            let name = if f.abs() >= r.abs() {
                if f >= 0 {
                    "run"
                } else {
                    "backpedal"
                }
            } else if r > 0 {
                "strafe_right"
            } else {
                "strafe_left"
            };
            let clip = clips.id(name);
            return Want {
                clip,
                when: When::At(clips.by_stride(clip, p.stride)),
                again: false,
            };
        }
        on("holding_idle", false)
    }

    /// The upper body with the gun: firing, the fan, a reload, the saw's
    /// rev, the spin, or the gun's hold.
    fn gun(&self, clips: &Clips, c: &Content, w: &World) -> Want {
        let g = &c.guns[w.held.gun];
        let id = g.id.as_str();
        let clip = |name: &str, when: When, again: bool| Want {
            clip: clips.id(name),
            when,
            again,
        };
        let fire = format!("{id}_fire");
        if id == "big_iron" && w.held.fan > 0 {
            return clip("big_iron_fan", When::Clock, false);
        }
        let level = g.level(w.held.level);
        let reload = format!("{id}_reload");
        if w.held.reloading(level.mag) && clips.has(&reload) {
            // the cylinder's progress picks the frame, so the last shell
            // goes in as the clip ends
            let r = clips.id(&reload);
            let all = (level.mag * level.shell).max(1) as i64;
            let done = (w.held.rounds * level.shell + w.held.loading) as i64;
            let f = done * clips.frames(r) as i64 * ONE as i64 / all;
            return clip(&reload, When::At(f as i32), false);
        }
        if id == "ripsaw" && w.revving > 0 && clips.has("ripsaw_rev") {
            return clip("ripsaw_rev", When::Clock, false);
        }
        if id == "filibuster" && w.held.spin > 0 && clips.has(&fire) {
            // the spin picks the frame: winding up as it spins up
            let f = clips.id(&fire);
            let at = fx::mul(w.held.spin, clips.frames(f) * ONE - ONE);
            return clip(&fire, When::At(at), false);
        }
        if self.held.fire > 0 {
            let f = if self.alt {
                self.alt_clip(clips, c, w)
            } else {
                clips.id(&fire)
            };
            // a one-shot clip starts over at each shot; a loop runs on
            let again = self.fresh && !clips.clips[f].looped;
            return Want {
                clip: f,
                when: When::Clock,
                again,
            };
        }
        // the hold: the carbine's for the rifles; each other gun's fire
        // clip, held on its first frame
        if matches!(id, "carbine" | "redactor" | "filibuster") || !clips.has(&fire) {
            clip("holding_idle", When::Clock, false)
        } else {
            clip(&fire, When::At(0), false)
        }
    }

    /// The upper body's bend for the aim, in the figure's rest frame: a share
    /// of the pitch, about the pivot (the old figure tilted its head).
    pub fn bend(clips: &Clips, pitch: i32) -> Affine {
        let a = pitch * DIVER_AIM_BEND_PCT / 100;
        Affine::translate(clips.pivot)
            .then(&Affine::rot_x(a))
            .then(&Affine::translate(clips.pivot.neg()))
    }

    /// The record and the gun's place for the body at `feet` facing `yaw`,
    /// `now` in Q16 ticks.
    pub fn shown(&self, clips: &Clips, c: &Content, p: &Player, feet: V3, now: i32) -> Shown {
        let bend = Diver::bend(clips, p.pitch);
        let hurt = if p.hurt_age < 6 { DIVER_HURT_MIX } else { 0 };
        let record = record(
            clips,
            (&self.lower, &self.upper),
            feet,
            p.yaw,
            &bend,
            (p.grey, hurt, c.pal.c("erased_grey")),
            now,
        );
        let figure = Affine::translate(feet).then(&Affine::rot_y(p.yaw));
        let to_world = figure.then(&bend);
        let clip = &clips.clips[self.upper.clip];
        let grip = clips.grip(self.upper.clip, self.upper.frame(now)).map(|g| Grip {
            at: to_world.apply(g.at),
            muzzle: to_world.apply_dir(g.muzzle),
            up: to_world.apply_dir(g.up),
        });
        let name = clip.name.as_str();
        let free = !is_gun(c, &clip.prop)
            || name.ends_with("_reload")
            || matches!(name, "big_iron_fan" | "throw" | "grab" | "hold_grabbed");
        Shown { record, grip, free }
    }

    /// The menus' figure: idle on the menu clock, the gun held level.
    pub fn standing(clips: &Clips, c: &Content, at: V3, yaw: i32, clock: i32) -> Shown {
        let idle = clips.id("idle");
        let track = Track::new(idle);
        let now = clock * ONE;
        let bend = Diver::bend(clips, 0);
        let record = record(
            clips,
            (&track, &track),
            at,
            yaw,
            &bend,
            (0, 0, c.pal.c("erased_grey")),
            now,
        );
        let figure = Affine::translate(at).then(&Affine::rot_y(yaw));
        let grip = clips.grip(idle, track.frame(now)).map(|g| Grip {
            at: figure.apply(g.at),
            muzzle: figure.apply_dir(g.muzzle),
            up: figure.apply_dir(g.up),
        });
        Shown {
            record,
            grip,
            free: false,
        }
    }
}

fn record(
    clips: &Clips,
    (lower, upper): (&Track, &Track),
    feet: V3,
    yaw: i32,
    bend: &Affine,
    (grey, hurt, ash): (i32, i32, crate::content::Rgb),
    now: i32,
) -> Vec<i32> {
    let mut out = Vec::with_capacity(VALUES);
    out.push(ONE);
    for t in [lower, upper] {
        let (a, b, k) = t.rows(clips, now);
        out.extend([int(a), int(b), k]);
    }
    let m = Affine::translate(feet).then(&Affine::rot_y(yaw));
    for a in [&m, bend] {
        for v in [a.c0, a.c1, a.c2, a.t] {
            out.extend([v.x, v.y, v.z]);
        }
    }
    out.extend([grey, hurt]);
    out.extend(ash);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Content;
    use crate::fx::deg;
    use crate::player::{Build, Player};
    use crate::world::World;

    fn content() -> Content {
        crate::content::tests::content()
    }

    /// A body standing in the first fight of a seeded run.
    fn world(c: &Content) -> World {
        let r = crate::route::Route::build(c, 11);
        let n = r
            .nodes
            .iter()
            .find(|n| n.kind == crate::route::NodeKind::Fight)
            .unwrap();
        let p = Player::new(V3::ZERO, 0, Build::default());
        World::new(c, n, &[], p, crate::weapons::Held::new(0, 1), 1)
    }

    fn pick(d: &Diver, clips: &Clips, c: &Content, w: &World) -> (String, String) {
        let (l, u) = d.choose(clips, c, w);
        (clips.clips[l.clip].name.clone(), clips.clips[u.clip].name.clone())
    }

    #[test]
    fn the_clip_table_has_all_fifty_clips_in_row_order() {
        let clips = Clips::load();
        assert_eq!(clips.clips.len(), 50);
        let mut row = 0;
        for c in &clips.clips {
            assert_eq!(c.row, row, "{} starts where the last ended", c.name);
            assert_eq!(c.grip.len(), c.frames as usize, "{} has a grip a frame", c.name);
            row += c.frames;
        }
    }

    #[test]
    fn dead_picks_death_ahead_of_every_other_row() {
        let c = content();
        let clips = Clips::load();
        let mut w = world(&c);
        let mut d = Diver::new(&clips);
        // everything else at once: in the air, dashing, hooked, hurt
        w.player.grounded = false;
        w.player.vel = v3(0, ONE / 10, 0);
        d.held.hurt = 10;
        d.held.pound = 10;
        w.player.health = 0;
        assert_eq!(pick(&d, &clips, &c, &w), ("death".into(), "death".into()));
    }

    #[test]
    fn airborne_and_rising_picks_jump_and_a_second_jump_somersaults() {
        let c = content();
        let clips = Clips::load();
        let mut w = world(&c);
        let mut d = Diver::new(&clips);
        w.player.grounded = false;
        w.player.vel = v3(0, DIVER_APEX + ONE / 100, 0);
        w.player.jumps_used = 1;
        assert_eq!(pick(&d, &clips, &c, &w).0, "jump");
        // an air jump (Event::Jump(2)) holds air_jump for its length, both layers
        w.player.jumps_used = 2;
        w.player.events = vec![Event::Jump(2)];
        d.observe(&clips, &c, &w);
        assert_eq!(pick(&d, &clips, &c, &w), ("air_jump".into(), "air_jump".into()));
        assert_eq!(
            d.held.air_jump,
            clips.frames(clips.id("air_jump")) * TICKS_PER_FRAME
        );
    }

    #[test]
    fn a_gait_frame_comes_from_the_stride_and_keeps_the_planted_foot_on_the_ground() {
        let clips = Clips::load();
        let run = clips.id("run");
        let c = &clips.clips[run];
        assert!(c.stride > 0, "the bake measured the run's stride");
        // a turn of stride is STRIDE_METRES of ground; the clip covers
        // c.stride a cycle, so a quarter of the clip's stride of ground is a
        // quarter of its cycle, worked by hand from the two lengths
        let quarter = (c.stride as i64 / 4 * TURN as i64 / int(STRIDE_METRES) as i64) as i32;
        let f = clips.by_stride(run, quarter);
        let want = c.frames * ONE / 4;
        assert!(
            (f - want).abs() < ONE / 8,
            "frame {f} for a quarter cycle, want {want}"
        );
        assert_eq!(clips.by_stride(run, 0), 0);
    }

    #[test]
    fn the_upper_body_follows_the_held_gun() {
        let c = content();
        let clips = Clips::load();
        let mut w = world(&c);
        let d = Diver::new(&clips);
        w.held = crate::weapons::Held::new(c.gun("carbine"), 1);
        assert_eq!(pick(&d, &clips, &c, &w).1, "holding_idle");
        w.held = crate::weapons::Held::new(c.gun("big_iron"), 1);
        let (_, u) = d.choose(&clips, &c, &w);
        assert_eq!(clips.clips[u.clip].name, "big_iron_fire");
        assert_eq!(
            u.when,
            When::At(0),
            "the Big Iron's hold is its fire clip's first frame"
        );
        w.held.fan = 3;
        assert_eq!(pick(&d, &clips, &c, &w).1, "big_iron_fan");
    }

    #[test]
    fn a_shot_plays_the_held_guns_fire_clip() {
        let c = content();
        let clips = Clips::load();
        let mut w = world(&c);
        let mut d = Diver::new(&clips);
        w.held = crate::weapons::Held::new(c.gun("scattergun"), 1);
        w.note("fire_scatter", String::new());
        d.observe(&clips, &c, &w);
        assert_eq!(clips.clips[d.upper.clip].name, "scattergun_fire");
        assert_eq!(d.upper.when, When::Clock);
        assert_eq!(d.held.fire, clips.frames(d.upper.clip) * TICKS_PER_FRAME);
    }

    #[test]
    fn a_record_is_the_right_length_and_fades_between_clips() {
        let c = content();
        let clips = Clips::load();
        let w = world(&c);
        let mut d = Diver::new(&clips);
        d.observe(&clips, &c, &w);
        let s = d.shown(&clips, &c, &w.player, w.player.pos, d.ticks * ONE);
        assert_eq!(s.record.len(), VALUES);
        assert_eq!(s.record[SHOWN], ONE);
        // a switch fades from the old clip's frame to the new one's
        let death = clips.id("death");
        d.lower.set(
            Want {
                clip: death,
                when: When::Clock,
                again: false,
            },
            d.ticks,
        );
        let (a, b, k) = d.lower.rows(&clips, d.ticks * ONE + ONE);
        assert_eq!(b, clips.clips[death].row);
        assert_ne!(a, b);
        assert_eq!(k, ONE / DIVER_FADE_TICKS);
    }

    #[test]
    fn the_bend_turns_the_upper_body_up_with_the_aim_about_the_pivot() {
        let clips = Clips::load();
        let b = Diver::bend(&clips, deg(60));
        let moved = b.apply(clips.pivot).dist(clips.pivot);
        assert!(moved < ONE / 1000, "the pivot stays, within rounding: {moved}");
        let ahead = b.apply_dir(v3(0, 0, -ONE));
        assert!(ahead.y > 0, "aiming up bends the chest up: {ahead:?}");
        let _ = Player::new(V3::ZERO, 0, Build::default());
    }

    #[test]
    fn the_hand_holding_the_gun_is_within_the_measured_gap_of_where_shots_leave() {
        // The game's hand (draw.rs hand_of) and muzzle (world.rs) are 0.4 m
        // right of the body and well ahead of it; the diver's short arms
        // in the suit hold the gun closer. The gun is drawn in the diver's
        // hand and shots keep leaving from the game's place (decision 3),
        // so this gap shows in play: about 430 mm in the holds, 611 mm at
        // the launcher's recoil; held here under 620 mm, measured
        // from the bake, so a clip that drifts further is caught
        // (SECOND-ORDER D5).
        let c = content();
        let clips = Clips::load();
        let hand = v3(ONE * 4 / 10, PLAYER_CHEST + ONE / 10, -ONE * 6 / 10);
        let mut worst = (0, String::new());
        for (k, clip) in clips.clips.iter().enumerate() {
            let held =
                is_gun(&c, &clip.prop) && !clip.name.ends_with("_reload") && clip.name != "big_iron_fan";
            let aim =
                clip.name.ends_with("_fire") || clip.name.ends_with("_burst") || clip.name.contains("idle");
            if !(held && aim) {
                continue;
            }
            for f in 0..clip.frames {
                let g = clips.grip(k, f * ONE).unwrap();
                let gap = g.at.dist(hand);
                if gap > worst.0 {
                    worst = (gap, format!("{} frame {f}", clip.name));
                }
            }
        }
        let limit = ONE * 62 / 100;
        assert!(worst.0 <= limit, "{} mm at {}", worst.0 * 1000 / ONE, worst.1);
        assert!(worst.0 > ONE / 10, "a gap was measured at all");
    }
}
