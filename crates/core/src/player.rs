//! How the Defendant moves: running, up to four jumps, dashing through
//! orbs, the grappling hook, the parry window, the ground pound, and health. Iron Vector's
//! habit is kept: move one axis at a time and stop at whatever is met, with a
//! step up onto anything low enough to walk onto.

use crate::balance::*;
use crate::fx::{self, ONE};
use crate::geom::{facing, right_of, v3, V3};
use crate::level::Level;

/// What the player asks for on one tick. The `*_pressed` fields are true on
/// the tick the key went down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Controls {
    /// strafe, −ONE (left) to ONE (right)
    pub move_x: i32,
    /// forward, −ONE (back) to ONE (forward)
    pub move_z: i32,
    pub jump_pressed: bool,
    pub dash_pressed: bool,
    pub fire: bool,
    pub grapple_held: bool,
    pub grapple_pressed: bool,
    pub parry_pressed: bool,
    pub interact_pressed: bool,
    /// the Injunction's key went down
    pub wall_pressed: bool,
    /// the ground pound's key went down
    pub pound_pressed: bool,
    /// the trigger went down this tick (a saw is thrown on the press)
    pub fire_pressed: bool,
}

/// What a run has made of the body: the upgrades bought and the pickups found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Build {
    pub max_health: i32,
    pub speed_pct: i32,
    pub jumps: i32,
    pub dash_charges: i32,
    pub grapple: bool,
    pub parry: bool,
    pub recess: bool,
    /// the Injunction: a wall in front of you that breaks blue orbs
    pub wall: bool,
    /// in-run rewards: extra pierce on every shot, a dash that clears
    /// orbs, a parry that heals
    pub pierce: i32,
    pub pulse: bool,
    pub mend: bool,
    /// the ground pound: 0 without it, 1 (Gavel Drop), 2 (Gavel II)
    pub pound: i32,
    /// the second levels of the hook, the parry and the wall
    pub grapple_plus: bool,
    pub parry_plus: bool,
    pub wall_plus: bool,
    /// Appeal: a second Recess
    pub appeal: bool,
    /// Hair Trigger and Plating, picked up in the house
    pub trigger: i32,
    pub plating: i32,
    /// Bail: more health when an arena is cleared
    pub bail: i32,
    /// the Lien: a Ripsaw waits at every arrival
    pub lien: bool,
}

impl Default for Build {
    fn default() -> Self {
        Build {
            max_health: BASE_HEALTH,
            speed_pct: 100,
            jumps: BASE_JUMPS,
            dash_charges: DASH_CHARGES,
            grapple: false,
            parry: false,
            recess: false,
            wall: false,
            pierce: 0,
            pulse: false,
            mend: false,
            pound: 0,
            grapple_plus: false,
            parry_plus: false,
            wall_plus: false,
            appeal: false,
            trigger: 0,
            plating: 0,
            bail: 0,
            lien: false,
        }
    }
}

impl Build {
    pub fn grapple_range(&self) -> i32 {
        GRAPPLE_RANGE + if self.grapple_plus { GRAPPLE_PLUS_RANGE } else { 0 }
    }

    /// Recesses a run starts with.
    pub fn recesses(&self) -> i32 {
        self.recess as i32 + (self.recess && self.appeal) as i32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grapple {
    pub anchor: V3,
    pub ticks: i32,
    /// the enemy the hook is in, if it hit one: the anchor follows it
    pub enemy: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub pos: V3,
    pub prev_pos: V3,
    pub vel: V3,
    pub yaw: i32,
    pub pitch: i32,
    pub grounded: bool,
    pub coyote: i32,
    pub jumps_used: i32,
    pub build: Build,
    pub health: i32,
    pub dash_left: i32,
    pub dash_ticks: i32,
    pub dash_dir: V3,
    pub dash_timer: i32,
    pub iframes: i32,
    pub grapple: Option<Grapple>,
    pub grapple_cooldown: i32,
    pub parry_window: i32,
    pub parry_cooldown: i32,
    /// ticks the Injunction's wall still stands, and until it can again
    pub wall_left: i32,
    pub wall_cooldown: i32,
    /// Recesses still to come this run
    pub recess_left: i32,
    /// dropping in a ground pound, from this height; its cooldown
    pub pounding: bool,
    pub pound_from: i32,
    pub pound_cooldown: i32,
    pub last_ground: V3,
    /// ticks since a hit landed, for the screen's red flash
    pub hurt_age: i32,
    /// the walk cycle, in angle units, for the drawn legs
    pub stride: i32,
    /// what happened this tick, for sound and the HUD
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Jump(i32),
    Land,
    Dash,
    Hurt(i32),
    Recess,
    Fell,
    GrappleOn,
    GrappleOff,
    Parry,
    /// the hook stopped moving you (an overhang, a corner) and let go
    HookSnag,
    /// the Injunction's wall went up
    Wall,
    /// an updraft threw you up
    Vent,
    /// a ground pound landed
    Pound,
}

impl Player {
    pub fn new(pos: V3, yaw: i32, build: Build) -> Player {
        Player {
            pos,
            prev_pos: pos,
            vel: V3::ZERO,
            yaw,
            pitch: 0,
            grounded: true,
            coyote: 0,
            jumps_used: 0,
            build,
            health: build.max_health,
            dash_left: build.dash_charges,
            dash_ticks: 0,
            dash_dir: V3::ZERO,
            dash_timer: 0,
            iframes: 0,
            grapple: None,
            grapple_cooldown: 0,
            parry_window: 0,
            parry_cooldown: 0,
            wall_left: 0,
            wall_cooldown: 0,
            recess_left: build.recesses(),
            pounding: false,
            pound_from: 0,
            pound_cooldown: 0,
            last_ground: pos,
            hurt_age: 1000,
            stride: 0,
            events: Vec::new(),
        }
    }

    /// Carries a body into the next arena: health, the build and what is
    /// left of Recess stay; motion and timers do not.
    pub fn arrive(&mut self, pos: V3, yaw: i32) {
        let keep = (self.health, self.build, self.recess_left);
        *self = Player::new(pos, yaw, keep.1);
        self.health = keep.0;
        self.recess_left = keep.2;
    }

    pub fn look(&mut self, d_yaw: i32, d_pitch: i32) {
        self.yaw = (self.yaw + d_yaw) & (fx::TURN - 1);
        self.pitch = (self.pitch + d_pitch).clamp(PITCH_MIN, PITCH_MAX);
    }

    pub fn top_speed(&self) -> i32 {
        RUN_SPEED * self.build.speed_pct / 100
    }

    pub fn chest(&self) -> V3 {
        self.pos.add(v3(0, PLAYER_CHEST, 0))
    }

    pub fn dashing(&self) -> bool {
        self.dash_ticks > 0
    }

    pub fn alive(&self) -> bool {
        self.health > 0
    }

    pub fn invulnerable(&self) -> bool {
        self.iframes > 0
    }

    /// Takes a hit unless the body is invulnerable; true if it landed.
    pub fn hurt(&mut self, damage: i32) -> bool {
        if self.invulnerable() || !self.alive() {
            return false;
        }
        let plating = self.build.plating.min(PLATING_MAX);
        let damage = (damage * (100 - PLATING_PCT * plating) / 100).max(1);
        self.health -= damage;
        self.iframes = HURT_IFRAMES;
        self.hurt_age = 0;
        if self.health <= 0 && self.recess_left > 0 {
            self.recess_left -= 1;
            self.health = self.build.max_health * RECESS_HEALTH_PCT / 100;
            self.events.push(Event::Recess);
        }
        self.health = self.health.max(0);
        self.events.push(Event::Hurt(damage));
        true
    }

    /// An updraft under the feet: up at `speed`, the air jumps kept.
    pub fn launch_up(&mut self, speed: i32) {
        if self.vel.y >= speed / 2 {
            return;
        }
        self.release_grapple();
        self.vel.y = speed;
        self.grounded = false;
        self.coyote = 0;
        self.jumps_used = self.jumps_used.max(1);
        self.events.push(Event::Vent);
    }

    pub fn heal(&mut self, amount: i32) {
        self.health = (self.health + amount).min(self.build.max_health);
    }

    fn wish(&self, c: &Controls) -> V3 {
        let f = facing(self.yaw, 0);
        let r = right_of(self.yaw);
        f.scale(c.move_z).add(r.scale(c.move_x)).clamp_len(ONE)
    }

    pub fn start_grapple(&mut self, anchor: V3, enemy: Option<usize>) {
        self.grapple = Some(Grapple {
            anchor,
            ticks: 0,
            enemy,
        });
        self.jumps_used = self.jumps_used.min(1);
        self.events.push(Event::GrappleOn);
    }

    pub fn release_grapple(&mut self) {
        if self.grapple.take().is_some() {
            self.grapple_cooldown = if self.build.grapple_plus {
                GRAPPLE_PLUS_COOLDOWN
            } else {
                GRAPPLE_COOLDOWN
            };
            self.events.push(Event::GrappleOff);
        }
    }

    /// One tick of movement. The world has already started or moved the
    /// grapple's anchor this tick.
    pub fn step(&mut self, c: &Controls, level: &Level) {
        self.prev_pos = self.pos;
        self.events.clear();
        self.iframes = (self.iframes - 1).max(0);
        self.grapple_cooldown = (self.grapple_cooldown - 1).max(0);
        self.parry_window = (self.parry_window - 1).max(0);
        self.parry_cooldown = (self.parry_cooldown - 1).max(0);
        self.hurt_age = (self.hurt_age + 1).min(1000);
        if self.dash_left < self.build.dash_charges {
            self.dash_timer += 1;
            if self.dash_timer >= DASH_RECHARGE {
                self.dash_timer = 0;
                self.dash_left += 1;
            }
        }

        self.wall_left = (self.wall_left - 1).max(0);
        self.wall_cooldown = (self.wall_cooldown - 1).max(0);
        if c.wall_pressed && self.build.wall && self.wall_cooldown == 0 {
            let plus = self.build.wall_plus;
            self.wall_left = if plus { WALL_PLUS_TICKS } else { WALL_TICKS };
            self.wall_cooldown = if plus { WALL_PLUS_COOLDOWN } else { WALL_COOLDOWN };
            self.events.push(Event::Wall);
        }

        if c.parry_pressed && self.build.parry && self.parry_cooldown == 0 {
            let plus = self.build.parry_plus;
            self.parry_window = if plus { PARRY_PLUS_WINDOW } else { PARRY_WINDOW };
            self.parry_cooldown = if plus { PARRY_PLUS_COOLDOWN } else { PARRY_COOLDOWN };
            self.events.push(Event::Parry);
        }

        // the ground pound: in the air, straight down
        self.pound_cooldown = (self.pound_cooldown - 1).max(0);
        if c.pound_pressed
            && self.build.pound > 0
            && !self.grounded
            && !self.pounding
            && self.pound_cooldown == 0
        {
            self.release_grapple();
            self.dash_ticks = 0;
            self.pounding = true;
            self.pound_from = self.pos.y;
        }

        let wish = self.wish(c);
        let top = self.top_speed();
        let flat = v3(self.vel.x, 0, self.vel.z);

        if c.dash_pressed && self.dash_left > 0 && !self.dashing() {
            self.release_grapple();
            self.dash_dir = if wish == V3::ZERO {
                facing(self.yaw, 0)
            } else {
                wish.norm()
            };
            self.dash_ticks = DASH_TICKS;
            self.iframes = self.iframes.max(DASH_IFRAMES);
            self.dash_left -= 1;
            self.events.push(Event::Dash);
        }

        if c.jump_pressed {
            let ground_jump = self.grounded || self.coyote > 0;
            if ground_jump || self.jumps_used < self.build.jumps {
                self.release_grapple();
                self.dash_ticks = 0;
                if ground_jump {
                    self.vel.y = JUMP_SPEED;
                    self.jumps_used = 1;
                } else {
                    self.vel.y = AIR_JUMP_SPEED;
                    self.jumps_used += 1;
                    // an air jump turns you to where you are pushing, unless
                    // you are flying faster than you can run
                    if wish != V3::ZERO && flat.len() <= top + top / 8 {
                        let to = wish.scale(top);
                        self.vel.x = to.x;
                        self.vel.z = to.z;
                    }
                }
                self.grounded = false;
                self.coyote = 0;
                self.events.push(Event::Jump(self.jumps_used));
            }
        }

        if self.pounding {
            self.vel = v3(0, -POUND_SPEED, 0);
            self.iframes = self.iframes.max(2);
        } else if self.dashing() {
            self.dash_ticks -= 1;
            let d = self.dash_dir.scale(DASH_SPEED);
            self.vel = v3(d.x, 0, d.z);
            if self.dash_ticks == 0 {
                let keep = self.dash_dir.scale(top);
                self.vel = v3(keep.x, 0, keep.z);
            }
        } else if let Some(mut g) = self.grapple {
            g.ticks += 1;
            self.grapple = Some(g);
            let to = g.anchor.sub(self.chest());
            let dist = to.len();
            if !c.grapple_held || dist <= GRAPPLE_ARRIVE || g.ticks > GRAPPLE_MAX_TICKS {
                self.release_grapple();
            } else {
                let pull = to.scale(fx::div(GRAPPLE_PULL, dist));
                let steer = wish.scale(AIR_ACCEL);
                self.vel = self.vel.add(pull).add(steer).clamp_len(GRAPPLE_MAX_SPEED);
            }
        } else {
            self.run(wish, top, flat);
            self.vel.y = (self.vel.y - GRAVITY).max(-MAX_FALL);
        }

        let was_grounded = self.grounded;
        self.collide_move(level);
        if self.grounded && self.pounding {
            // the landing: a shockwave (the world's), and a bounce to a
            // little over where the drop began, with the air jumps back
            self.pounding = false;
            self.pound_cooldown = POUND_COOLDOWN;
            self.iframes = self.iframes.max(POUND_IFRAMES);
            let h = (self.pound_from - self.pos.y + POUND_OVER).max(POUND_MIN_BOUNCE);
            self.vel.y = bounce_speed(h);
            self.grounded = false;
            self.coyote = 0;
            self.jumps_used = 1;
            self.last_ground = self.pos;
            self.events.push(Event::Pound);
        } else if self.grounded {
            if !was_grounded {
                self.events.push(Event::Land);
            }
            self.jumps_used = 0;
            self.coyote = COYOTE_TICKS;
            self.last_ground = self.pos;
        } else {
            self.coyote = (self.coyote - 1).max(0);
            if self.coyote == 0 && self.jumps_used == 0 {
                // walked off a ledge: the ground jump is spent
                self.jumps_used = 1;
            }
        }
        if let Some(g) = self.grapple {
            if g.ticks > GRAPPLE_STALL_TICKS && self.pos.dist(self.prev_pos) < ONE / 200 {
                self.release_grapple();
                self.events.push(Event::HookSnag);
            }
        }
        let moved = self.pos.sub(self.prev_pos).len_xz();
        if self.grounded {
            self.stride = (self.stride + fx::mul(moved, fx::TURN / 3)) & (fx::TURN - 1);
        }
        if self.pos.y < KILL_FLOOR {
            self.pounding = false;
            self.pos = self.last_ground.add(v3(0, ONE, 0));
            self.prev_pos = self.pos;
            self.vel = V3::ZERO;
            self.release_grapple();
            self.hurt(FALL_DAMAGE);
            self.events.push(Event::Fell);
        }
    }

    /// Steers the flat velocity toward where the player is pushing. Above
    /// the run speed (after a dash or a grapple) the speed bleeds off slowly
    /// and the push only turns it, so momentum carries.
    fn run(&mut self, wish: V3, top: i32, flat: V3) {
        let accel = if self.grounded { GROUND_ACCEL } else { AIR_ACCEL };
        let speed = flat.len();
        let next = if speed > top {
            let decel = if self.grounded {
                OVERSPEED_DECEL
            } else {
                OVERSPEED_DECEL_AIR
            };
            let turned = flat.add(wish.scale(accel));
            turned.clamp_len((speed - decel).max(top))
        } else {
            let desired = wish.scale(top);
            flat.add(desired.sub(flat).clamp_len(accel))
        };
        self.vel.x = next.x;
        self.vel.z = next.z;
    }

    fn bounds(&self, at: V3) -> (V3, V3) {
        (
            v3(at.x - PLAYER_RADIUS, at.y, at.z - PLAYER_RADIUS),
            v3(at.x + PLAYER_RADIUS, at.y + PLAYER_HEIGHT, at.z + PLAYER_RADIUS),
        )
    }

    /// Moves along x, then z, then y, stopping at whatever is met. Walking
    /// into something no taller than `STEP_UP` climbs onto it.
    fn collide_move(&mut self, level: &Level) {
        let was_grounded = self.grounded;
        self.grounded = false;
        for axis in 0..3 {
            let mut next = self.pos;
            let dv = match axis {
                0 => self.vel.x,
                1 => self.vel.z,
                _ => self.vel.y,
            };
            if dv == 0 {
                continue;
            }
            match axis {
                0 => next.x += dv,
                1 => next.z += dv,
                _ => next.y += dv,
            }
            let (min, max) = self.bounds(next);
            if !level.blocked(min, max) {
                self.pos = next;
                continue;
            }
            if axis < 2 && was_grounded {
                let lifted = v3(next.x, next.y + STEP_UP, next.z);
                let (lmin, lmax) = self.bounds(lifted);
                if !level.blocked(lmin, lmax) {
                    // stand on the highest top the body overlapped
                    let mut top = next.y;
                    level.each_near(min, max, |s| {
                        if s.overlaps(min, max) {
                            top = top.max(s.max.y);
                        }
                    });
                    self.pos = v3(next.x, top, next.z);
                    self.grounded = true;
                    continue;
                }
            }
            let mut edge: Option<i32> = None;
            level.each_near(min, max, |s| {
                if !s.overlaps(min, max) {
                    return;
                }
                let e = match axis {
                    0 if dv > 0 => s.min.x - PLAYER_RADIUS - 1,
                    0 => s.max.x + PLAYER_RADIUS + 1,
                    1 if dv > 0 => s.min.z - PLAYER_RADIUS - 1,
                    1 => s.max.z + PLAYER_RADIUS + 1,
                    _ if dv > 0 => s.min.y - PLAYER_HEIGHT - 1,
                    _ => s.max.y,
                };
                edge = Some(match edge {
                    None => e,
                    Some(o) if dv > 0 => o.min(e),
                    Some(o) => o.max(e),
                });
            });
            let e = edge.unwrap_or(self.pos.x);
            match axis {
                0 => {
                    self.pos.x = if dv > 0 { e.min(next.x) } else { e.max(next.x) };
                    self.vel.x = 0;
                }
                1 => {
                    self.pos.z = if dv > 0 { e.min(next.z) } else { e.max(next.z) };
                    self.vel.z = 0;
                }
                _ => {
                    if dv < 0 {
                        self.pos.y = e;
                        self.grounded = true;
                    } else {
                        self.pos.y = e.min(next.y);
                    }
                    self.vel.y = 0;
                }
            }
            // a position the body cannot be in (an edge found past the start)
            // is refused: stay put on this axis
            let (bmin, bmax) = self.bounds(self.pos);
            if level.blocked(bmin, bmax) {
                match axis {
                    0 => self.pos.x = self.prev_pos.x,
                    1 => self.pos.z = self.prev_pos.z,
                    _ => self.pos.y = self.prev_pos.y.max(self.pos.y),
                }
            }
        }
    }
}

/// The speed up that rises `h` under gravity, as the simulation steps it
/// (speed first, then place): the sum of v, v − g, v − 2g … reaches `h`.
pub fn bounce_speed(h: i32) -> i32 {
    // in Q16, V² = 2GH holds as it does in metres; half a tick's gravity
    // more for the step order
    fx::isqrt((2 * GRAVITY as i64 * h.max(0) as i64) as u64) as i32 + GRAVITY / 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::int;
    use crate::level::Solid;

    fn flat_floor() -> Level {
        Level::new(vec![Solid::new(
            v3(-int(100), -ONE, -int(100)),
            v3(int(100), 0, int(100)),
        )])
    }

    fn idle() -> Controls {
        Controls::default()
    }

    fn settle(p: &mut Player, l: &Level) {
        for _ in 0..10 {
            p.step(&idle(), l);
        }
    }

    #[test]
    fn a_jump_peaks_at_the_height_its_speed_and_gravity_give() {
        // v = 9 m/s, g = 26 m/s²: v²/2g = 81/52 = 1.56 m, less a little
        // for the step order (speed before place, so the first tick climbs
        // v − g/60)
        let l = flat_floor();
        let mut p = Player::new(V3::ZERO, 0, Build::default());
        settle(&mut p, &l);
        p.step(
            &Controls {
                jump_pressed: true,
                ..idle()
            },
            &l,
        );
        let mut top = 0;
        for _ in 0..120 {
            p.step(&idle(), &l);
            top = top.max(p.pos.y);
        }
        assert!(top > ONE * 145 / 100 && top < ONE * 160 / 100, "{top}");
        assert!(p.grounded);
    }

    #[test]
    fn the_jumps_a_build_allows_are_the_jumps_you_get() {
        let l = flat_floor();
        for jumps in 1..=4 {
            let mut p = Player::new(
                V3::ZERO,
                0,
                Build {
                    jumps,
                    ..Build::default()
                },
            );
            settle(&mut p, &l);
            let mut done = 0;
            // press jump every 20 ticks, ten times, in the air
            for k in 0..200 {
                let before = p.jumps_used;
                p.step(
                    &Controls {
                        jump_pressed: k % 20 == 0,
                        ..idle()
                    },
                    &l,
                );
                if p.jumps_used > before {
                    done += 1;
                }
                if p.grounded && k > 0 {
                    break;
                }
            }
            assert_eq!(done, jumps, "a build with {jumps} jumps");
        }
    }

    #[test]
    fn four_jumps_climb_higher_than_one() {
        let l = flat_floor();
        let peak = |jumps: i32| {
            let mut p = Player::new(
                V3::ZERO,
                0,
                Build {
                    jumps,
                    ..Build::default()
                },
            );
            settle(&mut p, &l);
            let mut top = 0;
            for k in 0..240 {
                p.step(
                    &Controls {
                        jump_pressed: k % 22 == 0,
                        ..idle()
                    },
                    &l,
                );
                top = top.max(p.pos.y);
            }
            top
        };
        let (one, four) = (peak(1), peak(4));
        assert!(four > one * 3, "one {one}, four {four}");
    }

    #[test]
    fn running_reaches_top_speed_and_stops_quickly() {
        let l = flat_floor();
        let mut p = Player::new(V3::ZERO, 0, Build::default());
        settle(&mut p, &l);
        for _ in 0..30 {
            p.step(
                &Controls {
                    move_z: ONE,
                    ..idle()
                },
                &l,
            );
        }
        assert_eq!(p.vel.len_xz(), p.top_speed());
        // heading 0 faces −z
        assert!(p.pos.z < -int(3));
        for _ in 0..12 {
            p.step(&idle(), &l);
        }
        assert_eq!(p.vel.len_xz(), 0);
    }

    #[test]
    fn air_control_turns_a_jump_around() {
        // jump running forward, then push back: within a second in the air
        // the body is moving backward
        let l = flat_floor();
        let mut p = Player::new(V3::ZERO, 0, Build::default());
        settle(&mut p, &l);
        for _ in 0..30 {
            p.step(
                &Controls {
                    move_z: ONE,
                    ..idle()
                },
                &l,
            );
        }
        p.step(
            &Controls {
                move_z: ONE,
                jump_pressed: true,
                ..idle()
            },
            &l,
        );
        for _ in 0..20 {
            p.step(
                &Controls {
                    move_z: -ONE,
                    ..idle()
                },
                &l,
            );
        }
        assert!(!p.grounded);
        assert!(p.vel.z > 0, "moving forward still: {}", p.vel.z);
    }

    #[test]
    fn a_dash_covers_ground_fast_and_cannot_be_hurt_through() {
        let l = flat_floor();
        let mut p = Player::new(V3::ZERO, 0, Build::default());
        settle(&mut p, &l);
        p.step(
            &Controls {
                move_x: ONE,
                dash_pressed: true,
                ..idle()
            },
            &l,
        );
        assert!(p.invulnerable());
        assert!(!p.hurt(20));
        assert_eq!(p.health, BASE_HEALTH);
        for _ in 0..DASH_TICKS {
            p.step(&idle(), &l);
        }
        // 24 m/s for 10 ticks = 4 m, to the right (+x at heading 0)
        assert!(p.pos.x > int(37) / 10, "{}", p.pos.x);
        assert_eq!(p.dash_left, DASH_CHARGES - 1);
    }

    #[test]
    fn dash_charges_come_back_one_at_a_time() {
        let l = flat_floor();
        let mut p = Player::new(V3::ZERO, 0, Build::default());
        settle(&mut p, &l);
        for _ in 0..2 {
            p.step(
                &Controls {
                    dash_pressed: true,
                    ..idle()
                },
                &l,
            );
            for _ in 0..DASH_TICKS {
                p.step(&idle(), &l);
            }
        }
        assert_eq!(p.dash_left, 0);
        p.step(
            &Controls {
                dash_pressed: true,
                ..idle()
            },
            &l,
        );
        assert!(!p.dashing(), "no charge, no dash");
        for _ in 0..DASH_RECHARGE {
            p.step(&idle(), &l);
        }
        assert!(p.dash_left >= 1);
    }

    #[test]
    fn a_wall_stops_the_body_and_a_kerb_is_walked_up() {
        let l = Level::new(vec![
            Solid::new(v3(-int(100), -ONE, -int(100)), v3(int(100), 0, int(100))),
            // a kerb 30 cm high from two metres ahead (−z) up to the wall
            Solid::new(v3(-int(5), 0, -int(11)), v3(int(5), ONE * 3 / 10, -int(2))),
            // a wall six metres ahead
            Solid::new(v3(-int(5), 0, -int(12)), v3(int(5), int(5), -int(11))),
        ]);
        let mut p = Player::new(V3::ZERO, 0, Build::default());
        settle(&mut p, &l);
        for _ in 0..240 {
            p.step(
                &Controls {
                    move_z: ONE,
                    ..idle()
                },
                &l,
            );
        }
        assert_eq!(p.pos.y, ONE * 3 / 10, "standing on the kerb");
        // stopped at the wall's face, z = −11, by the body's radius
        assert!((p.pos.z - (-int(11) + PLAYER_RADIUS)).abs() < 4, "{}", p.pos.z);
    }

    #[test]
    fn the_grapple_pulls_toward_its_anchor_and_lets_go_on_arrival() {
        let l = flat_floor();
        let mut p = Player::new(V3::ZERO, 0, Build::default());
        settle(&mut p, &l);
        p.start_grapple(v3(0, int(12), -int(30)), None);
        let held = Controls {
            grapple_held: true,
            ..idle()
        };
        let mut peak_speed = 0;
        for _ in 0..240 {
            p.step(&held, &l);
            peak_speed = peak_speed.max(p.vel.len());
            if p.grapple.is_none() {
                break;
            }
        }
        assert!(p.grapple.is_none(), "let go on arrival");
        assert!(p.pos.z < -int(25), "{}", p.pos.z);
        assert!(peak_speed > RUN_SPEED * 2, "{peak_speed}");
    }

    #[test]
    fn recess_turns_one_killing_blow_and_only_one() {
        let mut p = Player::new(
            V3::ZERO,
            0,
            Build {
                recess: true,
                ..Build::default()
            },
        );
        p.hurt(500);
        assert!(p.alive());
        assert_eq!(p.health, BASE_HEALTH * RECESS_HEALTH_PCT / 100);
        p.iframes = 0;
        p.hurt(500);
        assert!(!p.alive());
    }
}
