//! Every feel number in one place. Rates are written per second (or per
//! second squared) as fractions and turned into Q16 per tick by `per_tick`
//! and `per_tick2`, so each reads in the unit a person would say it in.
//!
//! Every number here is the agent's guess until Sam plays it (SECOND-ORDER
//! T1). Tests pin what the numbers do, not whether that is fun.

use crate::fx::{deg, int, ratio, ONE};

pub const TICKS_PER_SECOND: i32 = 60;

/// m/s to Q16 metres per tick.
pub const fn per_tick(num: i32, den: i32) -> i32 {
    ratio(num, den * TICKS_PER_SECOND)
}

/// m/s² to Q16 metres per tick per tick.
pub const fn per_tick2(num: i32, den: i32) -> i32 {
    ratio(num, den * TICKS_PER_SECOND * TICKS_PER_SECOND)
}

/// Seconds to ticks.
pub const fn secs(num: i32, den: i32) -> i32 {
    num * TICKS_PER_SECOND / den
}

// ---- the player's body -------------------------------------------------

/// The body is a capsule drawn as a person: this tall, this wide.
pub const PLAYER_HEIGHT: i32 = ONE * 18 / 10;
pub const PLAYER_RADIUS: i32 = ONE * 4 / 10;
/// the height of the chest, where orbs are judged against the body's middle
pub const PLAYER_CHEST: i32 = ONE * 11 / 10;
/// A ledge this high or lower is walked up without a jump.
pub const STEP_UP: i32 = ONE / 2;

pub const RUN_SPEED: i32 = per_tick(95, 10);
pub const GROUND_ACCEL: i32 = per_tick2(90, 1);
/// Air control close to the ground's, as in Returnal and Saros: you can
/// turn a jump around in the air.
pub const AIR_ACCEL: i32 = per_tick2(55, 1);
/// Faster than the run speed (a dash, a grapple) bleeds off this fast.
pub const OVERSPEED_DECEL: i32 = per_tick2(30, 1);
pub const OVERSPEED_DECEL_AIR: i32 = per_tick2(8, 1);
pub const GRAVITY: i32 = per_tick2(26, 1);
pub const MAX_FALL: i32 = per_tick(40, 1);
pub const JUMP_SPEED: i32 = per_tick(9, 1);
/// Each jump after the first, in the air.
pub const AIR_JUMP_SPEED: i32 = per_tick(85, 10);
/// A jump pressed this soon after walking off a ledge still counts as the
/// ground jump.
pub const COYOTE_TICKS: i32 = 6;
pub const BASE_JUMPS: i32 = 1;
pub const MAX_JUMPS: i32 = 4;

pub const DASH_SPEED: i32 = per_tick(24, 1);
pub const DASH_TICKS: i32 = 10;
/// invulnerable for this long from the start of a dash
pub const DASH_IFRAMES: i32 = 13;
pub const DASH_CHARGES: i32 = 2;
pub const DASH_RECHARGE: i32 = secs(11, 10);

pub const BASE_HEALTH: i32 = 100;
/// after a hit, no further hit lands for this long
pub const HURT_IFRAMES: i32 = secs(6, 10);
/// fall below this and the floor catches you, at a cost
pub const KILL_FLOOR: i32 = -int(30);
pub const FALL_DAMAGE: i32 = 15;

// ---- the grappling hook (the Writ of Reach) ----------------------------

pub const GRAPPLE_RANGE: i32 = int(48);
pub const GRAPPLE_PULL: i32 = per_tick2(80, 1);
pub const GRAPPLE_MAX_SPEED: i32 = per_tick(30, 1);
/// the hook lets go this close to its anchor
pub const GRAPPLE_ARRIVE: i32 = int(2);
pub const GRAPPLE_COOLDOWN: i32 = secs(1, 1);
/// a hook held longer than this lets go on its own
pub const GRAPPLE_MAX_TICKS: i32 = secs(3, 1);
/// a hook that lands this far or less below a ledge's top lifts you over it
pub const MANTLE_REACH: i32 = ONE * 15 / 10;
/// a hook that has not moved you for this long lets go, and says so
pub const GRAPPLE_STALL_TICKS: i32 = 12;

// ---- the parry (Objection) ---------------------------------------------

pub const PARRY_WINDOW: i32 = secs(3, 10);
pub const PARRY_COOLDOWN: i32 = secs(9, 10);
/// a gold orb this close to the chest while the window is open is turned
pub const PARRY_REACH: i32 = ONE * 25 / 10;
pub const PARRY_SPEED: i32 = per_tick(60, 1);
pub const PARRY_DAMAGE: i32 = 60;

// ---- the Injunction (the projected wall) ---------------------------------

/// the wall stands this long
pub const WALL_TICKS: i32 = secs(1, 2);
pub const WALL_COOLDOWN: i32 = secs(4, 1);
/// its middle is this far in front of the chest, along the way you face
pub const WALL_AHEAD: i32 = ONE * 18 / 10;
/// this wide either side of the middle, and from a little under the feet
/// to this high over them
pub const WALL_HALF_WIDTH: i32 = ONE * 24 / 10;
pub const WALL_HEIGHT: i32 = ONE * 34 / 10;

// ---- the ground pound (Gavel Drop) -------------------------------------------

/// in the air, C drops you straight down this fast
pub const POUND_SPEED: i32 = per_tick(34, 1);
/// the shockwave where you land: its reach and its damage, and Gavel II's
pub const POUND_RADIUS: i32 = ONE * 55 / 10;
pub const POUND_DAMAGE: i32 = 45;
pub const POUND_PLUS_RADIUS: i32 = ONE * 75 / 10;
pub const POUND_PLUS_DAMAGE: i32 = 80;
/// the bounce carries you this far over the height you dropped from, and
/// at least this far over the floor
pub const POUND_OVER: i32 = ONE * 12 / 10;
pub const POUND_MIN_BOUNCE: i32 = int(3);
pub const POUND_COOLDOWN: i32 = secs(2, 1);
/// no hit lands while you drop, nor this long after the landing
pub const POUND_IFRAMES: i32 = secs(1, 4);

// ---- rewards and upgrades added with the house ---------------------------------

/// Hair Trigger: each one makes guns fire this many percent faster
pub const TRIGGER_PCT: i32 = 15;
/// Plating: each one takes this many percent off every hit, up to two
pub const PLATING_PCT: i32 = 20;
pub const PLATING_MAX: i32 = 2;
/// the Writ of Reach II: the hook reaches further and is back sooner
pub const GRAPPLE_PLUS_RANGE: i32 = int(16);
pub const GRAPPLE_PLUS_COOLDOWN: i32 = secs(1, 2);
/// Objection II: a longer parry, back sooner
pub const PARRY_PLUS_WINDOW: i32 = secs(45, 100);
pub const PARRY_PLUS_COOLDOWN: i32 = secs(6, 10);
/// Injunction II: the wall stands longer, back sooner
pub const WALL_PLUS_TICKS: i32 = secs(8, 10);
pub const WALL_PLUS_COOLDOWN: i32 = secs(3, 1);
/// Good Conduct: this many percent more Clemency earned
pub const CONDUCT_PCT: i32 = 15;
/// Bail: this much more health when an arena is cleared
pub const BAIL_HEAL: i32 = 15;

// ---- the house's agents and the Executor ----------------------------------------

/// an agent turns to face where it walks or aims at most this fast
pub const AGENT_TURN: i32 = deg(9);
/// a shield turns slower, so you can get round it: 72° a second, slower
/// than you run round it at its range
pub const SHIELD_TURN: i32 = deg(6) / 5;
/// a step an agent climbs; a drop it will take
pub const AGENT_STEP: i32 = ONE * 6 / 10;
pub const AGENT_DROP: i32 = int(6);
/// an agent gives up on reaching a place after this long and takes another
pub const AGENT_GIVE_UP: i32 = secs(6, 1);
/// the place an agent steps out to fire from is this near its cover
pub const PEEK_REACH: i32 = int(3);
/// you see what is within this angle of where you face; a flanker goes
/// further round than it
pub const VIEW_HALF: i32 = deg(55);
/// a crossfire's two agents stand at least this far apart round you
pub const CROSSFIRE_APART: i32 = deg(90);
/// partners in a crossfire or a ring wait for each other at most this long
pub const SYNC_WAIT: i32 = secs(5, 2);
/// a Handler alive cuts every agent's rest by this many percent
pub const COMMAND_PCT: i32 = 30;
/// the Executor: how fast its hull and turret turn, its ram, its contact
/// push, and how far it rises in its last phase
pub const TANK_TURN: i32 = deg(1);
pub const TANK_TURRET_TURN: i32 = deg(2) / 5;
pub const TANK_ACCEL: i32 = per_tick2(5, 1);
pub const TANK_RAM_SPEED: i32 = per_tick(15, 1);
pub const TANK_PUSH: i32 = per_tick(14, 1);
pub const TANK_RISE: i32 = int(5);
/// the screen shakes this long, this far, when it comes through the glass
pub const SHAKE_TICKS: i32 = secs(1, 1);
pub const SHAKE_SIZE: i32 = ONE * 35 / 100;

// ---- zone two's ground ------------------------------------------------------

/// an updraft vent throws a body up at this speed (about 6 m)
pub const VENT_SPEED: i32 = per_tick(175, 10);

// ---- orbs ---------------------------------------------------------------

/// an orb lives this long at most
pub const ORB_LIFE: i32 = secs(9, 1);
/// and no more than this many are alive in an arena at once
pub const MAX_ORBS: usize = 900;
/// a ground wave rises this high: stand on it and it hits, jump over it and it misses
pub const WAVE_HEIGHT: i32 = ONE * 7 / 10;
pub const WAVE_WIDTH: i32 = ONE * 6 / 10;

// ---- pickups and the run -----------------------------------------------

pub const PICKUP_REACH: i32 = int(2);
pub const HEALTH_PICKUP: i32 = 25;
pub const SPEED_PICKUP_PCT: i32 = 8;
pub const HEAL_ON_CLEAR: i32 = 20;
/// a gate is passed by walking into a circle this wide around it
pub const GATE_REACH: i32 = ONE * 22 / 10;
/// after an arena is cleared, the next wave of reward dust settles this long
pub const CLEAR_PAUSE: i32 = secs(1, 1);
/// a new wave comes when this few enemies of the last one are left
pub const WAVE_REMAINDER: i32 = 0;
/// and comes this long after the last one is down, so waves never pile up
pub const WAVE_GAP: i32 = secs(3, 2);
/// an enemy that has not seen you for this long comes back where it can
pub const LOST_TICKS: i32 = secs(15, 1);
/// the court's Recess: a killing blow leaves you on this many percent
pub const RECESS_HEALTH_PCT: i32 = 35;

// ---- the in-run rewards that change play ----------------------------------

/// Piercing Rounds: shots pass through this many more enemies
pub const PIERCE_BONUS: i32 = 1;
/// Dash Pulse: a dash clears enemy orbs this close to you, at most this often
pub const PULSE_RADIUS: i32 = ONE * 25 / 10;
pub const PULSE_COOLDOWN: i32 = secs(3, 1);
/// Mending Parry: each orb you turn back heals this much
pub const MEND_HEAL: i32 = 8;
/// the Antechamber's fountain restores this share of maximum health
pub const REST_HEAL_PCT: i32 = 50;
/// a run that ends mid-fight is credited this share of the arena's
/// Clemency for each enemy of it killed, and a first death this grant
pub const FIRST_DEATH_GRANT: i32 = 8;
/// every ten years of sentence add this many percent to Clemency earned
pub const SENTENCE_BONUS_PCT: i32 = 5;
/// the test save's Clemency, to buy what a later build adds
pub const TEST_SAVE_CLEMENCY: i32 = 5000;
/// the tier of the zone's hardest main fights: the first run to reach one
/// hears a count of the indictment
pub const FAR_TIER: i32 = 3;

// ---- the HUD ---------------------------------------------------------------

/// a hint stays on screen this long
pub const HINT_TICKS: i32 = secs(6, 1);
/// the arc toward a hit's source fades over this long
pub const HIT_ARC_TICKS: i32 = secs(3, 4);
/// the jump cue shows this long before a ground wave reaches you
pub const JUMP_CUE_TICKS: i32 = secs(1, 2);
/// an orb this close to the camera, or behind the body and within the
/// second distance of the camera, is drawn as a faint glow
pub const ORB_FADE_NEAR: i32 = ONE * 3;
pub const ORB_FADE_BEHIND: i32 = ONE * 9;
/// a held orb swells to full size over the last of its hold
pub const HOLD_TELL: i32 = secs(1, 2);

// ---- the camera ----------------------------------------------------------

pub const FOV: i32 = deg(72);
pub const CAMERA_BACK: i32 = ONE * 64 / 10;
pub const CAMERA_UP: i32 = ONE * 24 / 10;
pub const CAMERA_RIGHT: i32 = ONE;
/// a camera pulled in nearer than this to the head stops drawing the body,
/// which would fill the screen
pub const CAMERA_HIDE_BODY: i32 = ONE * 16 / 10;
/// the camera stays this far in front of a wall it backs into
pub const CAMERA_MARGIN: i32 = ONE * 3 / 10;
/// The scenery between the camera and the head is cut away so it never
/// hides you: everything within `SEE_NEAR` of the lens, and a cone from
/// the lens to `SEE_SHORT` short of the head, `SEE_WIDE_LENS` wide at the
/// lens and `SEE_WIDE_HEAD` near the head, solid within `SEE_SOLID_PCT`
/// percent of its width and screen-door transparent beyond.
pub const SEE_NEAR: i32 = ONE * 9 / 10;
pub const SEE_WIDE_LENS: i32 = ONE * 7 / 10;
pub const SEE_WIDE_HEAD: i32 = ONE * 16 / 10;
pub const SEE_SHORT: i32 = ONE * 8 / 10;
pub const SEE_SOLID_PCT: i32 = 60;
pub const PITCH_MIN: i32 = -deg(70);
pub const PITCH_MAX: i32 = deg(80);
/// angle units of turn per pixel of mouse movement (about 0.1°)
pub const LOOK_PER_PIXEL: i32 = 17;
/// a shot's aim ray looks this far for something to hit
pub const AIM_RANGE: i32 = int(220);

// ---- damage -----------------------------------------------------------------

pub const ORB_DAMAGE: i32 = 12;
pub const WAVE_DAMAGE: i32 = 18;
/// Orbs are judged smaller than they are drawn, as in Returnal: a graze
/// that looks close is a miss.
pub const ORB_HIT_PCT: i32 = 62;
