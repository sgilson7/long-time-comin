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

// ---- the parry (Objection) ---------------------------------------------

pub const PARRY_WINDOW: i32 = secs(1, 4);
pub const PARRY_COOLDOWN: i32 = secs(9, 10);
/// a gold orb this close to the chest while the window is open is turned
pub const PARRY_REACH: i32 = ONE * 25 / 10;
pub const PARRY_SPEED: i32 = per_tick(60, 1);
pub const PARRY_DAMAGE: i32 = 60;

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
pub const HEAL_ON_CLEAR: i32 = 12;
/// a gate is passed by walking into a circle this wide around it
pub const GATE_REACH: i32 = ONE * 22 / 10;
/// after an arena is cleared, the next wave of reward dust settles this long
pub const CLEAR_PAUSE: i32 = secs(1, 1);
/// a new wave comes when this few enemies of the last one are left
pub const WAVE_REMAINDER: usize = 1;
/// the court's Recess: a killing blow leaves you on this many percent
pub const RECESS_HEALTH_PCT: i32 = 35;

// ---- the camera ----------------------------------------------------------

pub const FOV: i32 = deg(72);
pub const CAMERA_BACK: i32 = ONE * 5;
pub const CAMERA_UP: i32 = ONE * 21 / 10;
pub const CAMERA_RIGHT: i32 = ONE * 9 / 10;
/// a camera pulled in nearer than this to the head stops drawing the body,
/// which would fill the screen
pub const CAMERA_HIDE_BODY: i32 = ONE * 16 / 10;
/// the camera stays this far in front of a wall it backs into
pub const CAMERA_MARGIN: i32 = ONE * 3 / 10;
pub const PITCH_MIN: i32 = -deg(70);
pub const PITCH_MAX: i32 = deg(80);
/// angle units of turn per pixel of mouse movement (about 0.1°)
pub const LOOK_PER_PIXEL: i32 = 17;
/// a shot's aim ray looks this far for something to hit
pub const AIM_RANGE: i32 = int(220);

// ---- damage -----------------------------------------------------------------

pub const ORB_DAMAGE: i32 = 14;
pub const WAVE_DAMAGE: i32 = 18;
/// Orbs are judged smaller than they are drawn, as in Returnal: a graze
/// that looks close is a miss.
pub const ORB_HIT_PCT: i32 = 70;
