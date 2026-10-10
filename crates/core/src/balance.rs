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
