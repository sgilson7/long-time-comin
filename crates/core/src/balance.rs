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

// ---- the Ascent ---------------------------------------------------------------

/// a revolver loads after the trigger has been let be this long
pub const RELOAD_WAIT: i32 = secs(35, 100);
/// grab and throw: an enemy at this share of its health or less, this
/// near and in front of you, can be lifted; the throw's speed, its damage
/// to what it hits and the burst round it
pub const GRAB_PCT: i32 = 50;
pub const GRAB_REACH: i32 = ONE * 35 / 10;
pub const GRAB_HOLD: i32 = secs(3, 1);
pub const THROW_SPEED: i32 = per_tick(32, 1);
pub const THROW_DAMAGE: i32 = 180;
pub const THROW_SPLASH: i32 = 70;
pub const THROW_RADIUS: i32 = int(4);
/// your time stop: this long at first (after the Defendant), a second
/// more for each Stopped Clock, and its cooldown
pub const STOP_BASE: i32 = secs(3, 1);
pub const STOP_PLUS: i32 = secs(1, 1);
pub const STOP_COOLDOWN: i32 = secs(30, 1);
/// the Defendant's hook: it lands this far beside you, and it pulls the
/// Defendant along at this speed until it is this close
pub const LUNGE_HOOK_BESIDE: i32 = int(4);
pub const LUNGE_HOOK_SPEED: i32 = per_tick(36, 1);
pub const LUNGE_HOOK_ARRIVE: i32 = ONE * 15 / 10;
/// its cast waits on the pull at most this long
pub const LUNGE_HOOK_MAX: i32 = secs(1, 1);
/// the Defendant's ground pound: it leaps this high over where you were,
/// then drops at this speed
pub const LUNGE_POUND_HEIGHT: i32 = int(6);
pub const LUNGE_POUND_DROP: i32 = per_tick(40, 1);
// ---- the mountain's beasts --------------------------------------------------
/// a bounding beast (the hart) leaps this often, for this long, through
/// this gravity: about 2 m high at the top of its arc
pub const BOUND_EVERY: i32 = secs(3, 1);
pub const BOUND_TICKS: i32 = secs(9, 10);
pub const BEAST_GRAVITY: i32 = per_tick2(20, 1);
/// how far round you each bound carries it
pub const BOUND_TURN_DEG: i32 = 50;
/// a hopping beast (the toad): short low hops, this often and this long,
/// this far
pub const HOP_EVERY: i32 = secs(1, 1);
pub const HOP_TICKS: i32 = secs(4, 10);
pub const HOP_LENGTH: i32 = ONE * 25 / 10;
/// a wheeling beast (the rook) dives to this far over your chest as it
/// winds up, and its circle closes to this share of its range
pub const WHEEL_DIVE_OVER: i32 = int(4);
pub const WHEEL_DIVE_PCT: i32 = 75;
/// a rooted beast's bite (the gaper): it lunges when you are within its
/// reach and this much more, takes this long to strike, and bites again
/// after this long
pub const BITE_MARGIN: i32 = ONE;
pub const BITE_OUT: i32 = 8;
pub const BITE_EVERY: i32 = secs(14, 10);
/// a surging beast (the freshet) sweeps round a circle at its range from
/// you once in this long, turning toward it this fast
pub const SURGE_PERIOD: i32 = secs(5, 1);
pub const SURGE_TURN: i32 = ONE / 14;
/// a perching beast (the Great Horned): this many perches, this far round
/// the arena's centre and this high over the floor there; it sits this
/// long on each, glides between them this fast, and every `SWOOP_EVERY`th
/// move swoops instead: through where you stood at this speed and this
/// far past, its wings spread this long before
pub const PERCHES: i32 = 6;
pub const PERCH_RING: i32 = int(14);
pub const PERCH_HEIGHT: i32 = int(8);
pub const PERCH_HOLD: i32 = secs(4, 1);
pub const PERCH_GLIDE: i32 = per_tick(14, 1);
pub const SWOOP_EVERY: i32 = 2;
pub const SWOOP_SPEED: i32 = per_tick(22, 1);
pub const SWOOP_PAST: i32 = int(6);
pub const SWOOP_TELL: i32 = secs(4, 5);
/// creatures are fully corrupted by this step of the map (the last main
/// fight's), and a little from the first
pub const CORRUPT_FULL_STEP: i32 = 20;
pub const CORRUPT_LEAST: i32 = ONE / 6;
/// each step along the Ascent's map is this much higher up the mountain
pub const ASCENT_PER_STEP: i32 = int(12);
/// the Idiot God's coming lasts this long before the revelation
pub const ENDING_TICKS: i32 = secs(14, 1);

// ---- zone five: inside the Idiot God -------------------------------------

/// The Glyph marches a step this long, every this often, and after this
/// many steps across steps toward you (or away) and turns back
pub const MARCH_EVERY: i32 = secs(7, 10);
pub const MARCH_STEP: i32 = int(2);
pub const MARCH_GLIDE: i32 = 8;
pub const MARCH_ACROSS: i32 = 5;
/// the Etching counts as looked at within this angle of the camera's
/// line, with nothing between
pub const UNSEEN_LOOK: i32 = deg(30);
/// the Render turns this fast on the spot, charges once it faces within
/// this angle of you, for this long, and stands this long after
pub const CHARGE_TURN: i32 = deg(150) / TICKS_PER_SECOND;
pub const CHARGE_AIM: i32 = deg(6);
pub const CHARGE_TICKS: i32 = secs(1, 1);
pub const CHARGE_REST: i32 = secs(9, 10);
/// every Toon bounces on one beat, this long, this high
pub const BOUNCE_TICKS: i32 = secs(4, 5);
pub const BOUNCE_HEIGHT: i32 = int(3);
/// the Reprise walks where you were this long before
pub const ECHO_DELAY: i32 = secs(3, 1);
/// off an edge of the god's floors you fall, and come down again from
/// this high over the floor you fell from (the arena's `wrap`)
pub const WRAP_BELOW: i32 = int(24);
/// the lullaby: how far ahead the staff shows a note
pub const STAFF_AHEAD: i32 = secs(3, 1);
/// where on the staff the line is, and how much is left past the last
/// note shown, in percent of its width
pub const STAFF_LINE_PCT: i32 = 12;
pub const STAFF_END_PCT: i32 = 4;
/// a bubble of stopped time round the god, grown a share each verse
pub const BUBBLE_RADIUS: i32 = int(22);
/// the trumpet's last note rings this long over the stopped god before
/// the ending
pub const ASLEEP_TICKS: i32 = secs(4, 1);

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
/// never more than this many enemies up at once: a wave larger than the
/// room left comes in as room is made, and calls wait
pub const MAX_ALIVE: usize = 15;
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

// ---- the last mission ---------------------------------------------------

/// On the last mission (new game plus) a fight lets this many enemies up
/// at once, sends this much more of each wave (percent), and sends the
/// next wave when this few of the last are left.
pub const REMIX_MAX_ALIVE: usize = 22;
pub const REMIX_EXTRA_PCT: i32 = 50;
pub const REMIX_REMAINDER: i32 = 3;
/// Chumbles: how big he is before his scream (Q16 of his full size), and
/// how long the scream takes, as he grows
pub const CAT_SMALL: i32 = ONE / 3;
pub const CAT_SCREAM: i32 = secs(3, 1);
/// when his last bar empties he is fed: he shrinks back and eats, this long
/// before the mission is won
pub const CAT_FED_TICKS: i32 = secs(6, 1);
/// the cat in the bedroom walks to the door at this speed, after waiting
/// this long
pub const GUIDE_SPEED: i32 = ONE * 3 / 60;
pub const GUIDE_WAIT: i32 = secs(3, 2);

// ---- the arsenal (round 15) ---------------------------------------------------

/// the highest level a gun reaches; the Holding's Armory starts one at
/// most at level 3, and side paths carry it the rest of the way
pub const MAX_GUN_LEVEL: i32 = 5;
/// Contempt: an enemy you kill bursts for this many percent of its own
/// maximum health, on every other enemy within the radius
pub const CONTEMPT_PCT: i32 = 25;
pub const CONTEMPT_RADIUS: i32 = int(3);
/// each further Contempt widens the burst this much, up to three in all
pub const CONTEMPT_WIDEN: i32 = ONE * 3 / 2;
pub const CONTEMPT_MAX: i32 = 3;
/// a Purse is this many times the zone's side-path Clemency
pub const PURSE_TIMES: i32 = 3;

// ---- the Holding's tree (round 16) ---------------------------------------------

/// the tree is drawn in a square this many units across, round its centre
pub const TREE_SIZE: i32 = 1000;
/// the hub in the middle, and the ring every root upgrade stands on
pub const TREE_HUB: i32 = 118;
pub const TREE_RING: i32 = 226;
/// each upgrade further out than the one it needs by this much
pub const TREE_STEP: i32 = 80;
/// a branch's wedge reaches this far out; its name sits this far out
pub const TREE_EDGE: i32 = 494;
/// each wedge, a petal, reaches this far past its outermost upgrade
pub const TREE_PETAL: i32 = 52;
pub const TREE_LABEL: i32 = 150;
/// the room inside each end of a branch's wedge, in halves of one upgrade's
/// share
pub const TREE_GAP_HALVES: i32 = 1;
/// the first branch's wedge starts this far clockwise of straight up
pub const TREE_START: i32 = -crate::fx::QUARTER / 2;
/// no two upgrades closer than this, centre to centre (a node is drawn
/// a little smaller)
pub const TREE_ROOM: i32 = 52;

// ---- the Water Gap (round 17) -----------------------------------------------------

/// the Bench: a plank this wide and this thick, standing this long (Bench
/// III longer), laid again after this; on the ground it is laid where you
/// aim, as far as this
pub const BENCH_SIZE: i32 = int(3);
pub const BENCH_THICK: i32 = ONE / 5;
pub const BENCH_TICKS: i32 = secs(3, 1);
pub const BENCH_LONG_TICKS: i32 = secs(5, 1);
pub const BENCH_COOLDOWN: i32 = secs(4, 1);
pub const BENCH_REACH: i32 = int(12);
/// an emitter in the rock fires only while you are this near
pub const EMITTER_WAKE: i32 = int(45);
/// a mooring post is reached this near
pub const POST_REACH: i32 = int(3);
/// the water returns you to the last post, this share of your health the
/// poorer
pub const DROWN_PCT: i32 = 10;
/// the Summons' Husk grows back over this many ticks once it has gone
/// unhit its time
pub const HUSK_REGROW_TICKS: i32 = secs(1, 1);
/// an aloft flier stays this near its home
pub const ALOFT_LEASH: i32 = int(26);
/// the Gallows Oak: its bar's share that is the climb's (the rest follows
/// the Summons), its eye this far over you, and what it calls at its crown
pub const OAK_CLIMB_BAR_PCT: i32 = 66;
pub const OAK_EYE_OVER: i32 = int(4);
pub const OAK_SUMMONS: &str = "summons";
/// an enemy this far over moving ground's top rides it
pub const ENEMY_RIDE: i32 = int(4);
/// an enemy this far under a course's water has drowned
pub const ENEMY_DROWN: i32 = int(2);

// ---- the Far Shore and the Vault (round 18) ---------------------------------------------

/// under the water: gravity this share of the air's, a stroke up this
/// fast, again after this long
pub const SWIM_GRAVITY_PCT: i32 = 40;
pub const SWIM_STROKE: i32 = per_tick(55, 10);
pub const SWIM_COOLDOWN: i32 = secs(35, 100);
/// a Cockatrice's gaze slows you to this share of your speed, this long
pub const SLOWED_PCT: i32 = 50;
pub const SLOWED_TICKS: i32 = secs(1, 1);
/// a gust's rain slants this long before it blows
pub const WARN: i32 = secs(1, 1);

/// how far the Captured Dragon reaches from the mainmast on its chain
pub const CHAIN_LEASH: i32 = 24 * ONE;

/// how near the chained dragon's mast the HUD says how to free it
pub const DRAGON_HINT_NEAR: i32 = 40 * ONE;

/// a rogue wave (the Hurricane): this high, rising out of the sea this
/// long before it rolls, this wide from crest to foot either way, rolling this
/// fast and in again this often; in the water it shoves a swimmer this hard at its
/// crest
pub const ROGUE_HEIGHT: i32 = int(7);
pub const ROGUE_WARN: i32 = secs(2, 1);
pub const ROGUE_HALF: i32 = int(22);
pub const ROGUE_SPEED: i32 = per_tick(13, 1);
pub const ROGUE_PERIOD: i32 = secs(14, 1);
pub const ROGUE_SWIM_PUSH: i32 = ratio(1, 30);
/// a green wash over a deck: a wall this high over the floor and this deep,
/// running this fast, its foam line showing this long first; what it
/// catches under its height is shoved this hard while in it, thrown this
/// fast along it and up, and hurt this much, once a wash
pub const WASH_HEIGHT: i32 = ratio(6, 5);
pub const WASH_THICK: i32 = int(1);
pub const WASH_SPEED: i32 = per_tick(8, 1);
pub const WASH_WARN: i32 = secs(3, 2);
pub const WASH_PUSH: i32 = ratio(1, 25);
pub const WASH_KNOCK: i32 = per_tick(11, 1);
pub const WASH_HEAVE: i32 = per_tick(4, 1);
pub const WASH_DAMAGE: i32 = 8;

/// the map: the timeline as a hologram over the crater, its root (2031) at
/// the top and each later zone a row below; the main line's column and the
/// River's either side of the middle, rows this far apart, each row a little
/// further back; the camera this far in front, looking at its middle; each
/// zone's diorama this wide, turning once in this long
pub const ATLAS_CENTRE: crate::geom::V3 = crate::geom::v3(0, int(34), int(-6));
pub const ATLAS_ROW: i32 = ratio(23, 10);
pub const ATLAS_COL: i32 = ratio(19, 10);
pub const ATLAS_LEAN: i32 = ratio(3, 10);
pub const ATLAS_BACK: i32 = int(12);
pub const ATLAS_DIORAMA: i32 = ratio(17, 10);
pub const ATLAS_SPIN_TICKS: i32 = 60 * 40;
pub const ATLAS_BEAM: i32 = ratio(6, 100);

/// a zone's walkthrough: the route it shows (a fixed seed, so it is the same
/// each time); a course flown at this speed, each stop shown at least and at
/// most this long; an arena that is not a course circled this long, from this
/// angle through this much, this far out (within these) and this high; the
/// camera this far behind and above the line, looking this far ahead; the
/// parked body's shield
pub const TOUR_SEED: u64 = 7;
pub const TOUR_SPEED: i32 = per_tick(24, 1);
pub const TOUR_MIN: i32 = 60 * 6;
pub const TOUR_MAX: i32 = 60 * 14;
pub const TOUR_CIRCLE: i32 = 60 * 7;
pub const TOUR_ORBIT_FROM: i32 = deg(200);
pub const TOUR_ORBIT_SWEEP: i32 = deg(110);
pub const TOUR_ORBIT_MIN: i32 = int(14);
pub const TOUR_ORBIT_MAX: i32 = int(34);
pub const TOUR_ORBIT_UP: i32 = int(10);
pub const TOUR_BEHIND: i32 = int(10);
pub const TOUR_ABOVE: i32 = int(7);
pub const TOUR_AHEAD: i32 = int(18);
pub const TOUR_SHIELD: i32 = 60 * 60;

/// the open sea (the Far Shore): you may sink this far under the surface,
/// at no more than this speed, and stay in it this long, before it takes
/// you back to your post
pub const SEA_DEPTH: i32 = int(8);
pub const SEA_SINK: i32 = per_tick(2, 1);
pub const SEA_BREATH: i32 = secs(12, 1);
