//! The data files the core reads, parsed and cross-checked here, so a
//! mistake in a data file is an error the page can show rather than a fight
//! that quietly breaks. Units in the files are whole numbers in named units
//! (cm, cm/s, ms, degrees); `cm`, `cms`, `ms` and friends turn them into the
//! Q16 per-tick values the simulation uses.

use crate::balance::TICKS_PER_SECOND;
use crate::fx::{self, ONE};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

pub type Rgb = [i32; 3];

/// Centimetres to Q16 metres.
pub fn cm(x: i32) -> i32 {
    ((x as i64 * ONE as i64) / 100) as i32
}

/// cm/s to Q16 metres per tick.
pub fn cms(x: i32) -> i32 {
    ((x as i64 * ONE as i64) / (100 * TICKS_PER_SECOND as i64)) as i32
}

/// cm/s² to Q16 metres per tick².
pub fn cms2(x: i32) -> i32 {
    ((x as i64 * ONE as i64) / (100 * (TICKS_PER_SECOND * TICKS_PER_SECOND) as i64)) as i32
}

/// Milliseconds to ticks, rounded to the nearest.
pub fn ms(x: i32) -> i32 {
    (x * TICKS_PER_SECOND + 500) / 1000
}

/// Degrees a second to angle units a tick.
pub fn degs(x: i32) -> i32 {
    fx::deg(x) / TICKS_PER_SECOND
}

/// A colour written `#rrggbb`, as Q16 channels.
pub fn parse_hex(s: &str) -> Result<Rgb, String> {
    let h = s.strip_prefix('#').unwrap_or(s);
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("not a colour: {s}"));
    }
    let ch = |i: usize| i32::from_str_radix(&h[i..i + 2], 16).unwrap_or(0) * ONE / 255;
    Ok([ch(0), ch(2), ch(4)])
}

/// A Q16 colour as the page writes it in CSS.
pub fn hex(c: Rgb) -> String {
    let ch = |v: i32| (v.clamp(0, ONE) * 255 / ONE).clamp(0, 255);
    format!("#{:02x}{:02x}{:02x}", ch(c[0]), ch(c[1]), ch(c[2]))
}

// ---- palette ---------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct PaletteFile {
    named: BTreeMap<String, String>,
    walls: Vec<String>,
    greens: Vec<String>,
    signs: Vec<String>,
    #[serde(default)]
    wards: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    named: BTreeMap<String, Rgb>,
    pub walls: Vec<Rgb>,
    pub greens: Vec<Rgb>,
    pub signs: Vec<Rgb>,
    /// the orb colours the Injunction breaks at each of its wards, from
    /// the first
    pub wards: Vec<Vec<String>>,
}

impl Palette {
    pub fn parse(text: &str) -> Result<Palette, String> {
        let f: PaletteFile = serde_json::from_str(text).map_err(|e| format!("palette: {e}"))?;
        let list = |v: &[String]| v.iter().map(|s| parse_hex(s)).collect::<Result<Vec<_>, _>>();
        let mut named = BTreeMap::new();
        for (k, v) in &f.named {
            named.insert(k.clone(), parse_hex(v).map_err(|e| format!("palette {k}: {e}"))?);
        }
        let p = Palette {
            named,
            walls: list(&f.walls)?,
            greens: list(&f.greens)?,
            signs: list(&f.signs)?,
            wards: f.wards.clone(),
        };
        if p.walls.is_empty() || p.greens.is_empty() || p.signs.is_empty() {
            return Err("palette: walls, greens and signs each need a colour".into());
        }
        Ok(p)
    }

    /// A named colour. A name the file lacks is drawn in the ink, and the
    /// test `every_colour_the_core_names_is_in_the_palette` keeps that from
    /// happening.
    pub fn c(&self, name: &str) -> Rgb {
        self.named
            .get(name)
            .or_else(|| self.named.get("ink"))
            .copied()
            .unwrap_or([0, 0, 0])
    }

    /// The ward an Injunction needs to break orbs of this colour: 1 for
    /// the first; 0 for colours no wall breaks.
    pub fn ward(&self, name: &str) -> i32 {
        self.wards
            .iter()
            .position(|w| w.iter().any(|n| n == name))
            .map_or(0, |k| k as i32 + 1)
    }

    pub fn has(&self, name: &str) -> bool {
        self.named.contains_key(name)
    }
}

// ---- weapons ---------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct WeaponsFile {
    weapons: Vec<WeaponFile>,
}

#[derive(Debug, Clone, Deserialize)]
struct WeaponFile {
    id: String,
    name: String,
    blurb: String,
    colour: String,
    levels: Vec<GunLevelFile>,
    /// the first zone (by index) whose side paths offer it
    #[serde(default)]
    from_zone: usize,
    /// its second fire, on the right mouse button
    alt: AltFile,
}

#[derive(Debug, Clone, Deserialize)]
struct AltFile {
    kind: String,
    name: String,
    damage: i32,
    #[serde(default = "one")]
    count: i32,
    #[serde(default)]
    speed_cms: i32,
    #[serde(default)]
    size_cm: i32,
    #[serde(default)]
    life_ms: i32,
    #[serde(default)]
    pierce: i32,
    #[serde(default)]
    splash_cm: i32,
    #[serde(default)]
    splash: i32,
    #[serde(default)]
    spread_deg: i32,
    cooldown_ms: i32,
}

/// What a gun's second fire does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AltKind {
    /// `count` heavy rounds one behind another along the aim
    Burst,
    /// one heavy round that passes through enemies
    Slug,
    /// `count` bombs in a fan
    Cluster,
    /// every lodged blade bursts and comes back
    Detonate,
    /// every round left in the cylinder, one after another, a little wild
    Fan,
}

/// A gun's second fire, at level 1; each level above adds `ALT_LEVEL_PCT`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alt {
    pub kind: AltKind,
    pub name: String,
    pub damage: i32,
    pub count: i32,
    pub speed: i32,
    pub size: i32,
    pub life: i32,
    pub pierce: i32,
    pub splash_radius: i32,
    pub splash: i32,
    pub spread: i32,
    pub cooldown: i32,
}

/// each gun level above the first adds this much to its second fire's damage
pub const ALT_LEVEL_PCT: i32 = 25;

impl Alt {
    pub fn damage_at(&self, level: i32) -> i32 {
        self.damage * (100 + ALT_LEVEL_PCT * (level.clamp(1, 4) - 1)) / 100
    }
}

#[derive(Debug, Clone, Deserialize)]
struct GunLevelFile {
    damage: i32,
    rate_ms: i32,
    pellets: i32,
    spread_deg: i32,
    speed_cms: i32,
    size_cm: i32,
    life_ms: i32,
    splash_cm: i32,
    splash: i32,
    pierce: i32,
    // a saw gun: how many blades, what a rev does, how much a blade
    // deals before it comes back, and how long it takes to come back
    #[serde(default)]
    saws: i32,
    #[serde(default)]
    rev_damage: i32,
    #[serde(default)]
    rev_ms: i32,
    #[serde(default)]
    teeth: i32,
    #[serde(default)]
    recall_ms: i32,
    /// a revolver's cylinder, and the time to load one round into it
    #[serde(default)]
    mag: i32,
    #[serde(default)]
    shell_ms: i32,
}

/// One level of one gun, in simulation units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GunLevel {
    pub damage: i32,
    pub rate: i32,
    pub pellets: i32,
    pub spread: i32,
    pub speed: i32,
    pub size: i32,
    pub life: i32,
    pub splash_radius: i32,
    pub splash: i32,
    pub pierce: i32,
    pub saw: Option<SawLevel>,
    /// rounds in the cylinder (0: no reloading), and ticks to load one
    pub mag: i32,
    pub shell: i32,
}

/// A saw gun's level: blades that lodge in what they hit and rev while
/// the trigger is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SawLevel {
    /// blades out at once
    pub saws: i32,
    /// damage a lodged blade deals each `rev_every` ticks while revving
    pub rev_damage: i32,
    pub rev_every: i32,
    /// the damage a blade deals in all before it comes back
    pub teeth: i32,
    /// ticks until a spent blade can be thrown again
    pub recall: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gun {
    pub id: String,
    pub name: String,
    pub blurb: String,
    pub colour: String,
    pub levels: Vec<GunLevel>,
    pub from_zone: usize,
    pub alt: Alt,
}

impl Gun {
    /// Level `n`, counted from 1, clamped to the levels there are.
    pub fn level(&self, n: i32) -> &GunLevel {
        let i = (n.max(1) as usize - 1).min(self.levels.len() - 1);
        &self.levels[i]
    }
}

// ---- patterns --------------------------------------------------------------

fn one() -> i32 {
    1
}

#[derive(Debug, Clone, Deserialize)]
struct PatternsFile {
    patterns: Vec<PatternFile>,
}

#[derive(Debug, Clone, Deserialize)]
struct PatternFile {
    id: String,
    volleys: Vec<VolleyFile>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct VolleyFile {
    shape: String,
    #[serde(default = "one")]
    count: i32,
    speed_cms: i32,
    #[serde(default)]
    accel_cms2: i32,
    #[serde(default)]
    min_cms: i32,
    #[serde(default)]
    max_cms: i32,
    #[serde(default)]
    curl_degs: i32,
    #[serde(default)]
    alternate: bool,
    size_cm: i32,
    colour: String,
    #[serde(default)]
    spin_deg: i32,
    #[serde(default = "one")]
    repeat: i32,
    #[serde(default)]
    every_ms: i32,
    #[serde(default)]
    delay_ms: i32,
    #[serde(default)]
    pitch_deg: i32,
    #[serde(default)]
    spread_deg: i32,
    #[serde(default)]
    rings: i32,
    #[serde(default)]
    gold: bool,
    #[serde(default)]
    aim: bool,
    #[serde(default)]
    damage: i32,
    // a held formation (shape "form") and a second stop
    #[serde(default)]
    form: String,
    #[serde(default)]
    rows: i32,
    #[serde(default)]
    at: String,
    #[serde(default)]
    at_cm: i32,
    #[serde(default)]
    plane: String,
    #[serde(default)]
    radius_cm: i32,
    #[serde(default)]
    hold_ms: i32,
    #[serde(default)]
    stagger_ms: i32,
    #[serde(default)]
    launch: String,
    #[serde(default)]
    stop_ms: i32,
    #[serde(default)]
    rehold_ms: i32,
    #[serde(default)]
    relaunch: String,
    #[serde(default)]
    relaunch_cms: i32,
    // the house's: ricochets, lobbed orbs that plant a pattern where they
    // land, a burst that sweeps, a lane's length
    #[serde(default)]
    bounce: i32,
    #[serde(default)]
    fall_cms2: i32,
    #[serde(default)]
    plant: String,
    #[serde(default)]
    sweep_deg: i32,
    #[serde(default)]
    length_cm: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Ring,
    Aimed,
    Sphere,
    Wave,
    /// orbs placed in a shape and held before they fly
    Form,
}

/// The shape a held formation's orbs are placed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// `count` round a circle of `radius`
    Ring,
    /// `count` across by `rows` up, `radius` apart
    Grid,
    /// `count` across, over `radius` either side
    Line,
    /// `count` stacked up to `radius` above the anchor
    Column,
    /// `count` arms of `rows` orbs, out to `radius`
    Star,
    /// `count` along a spiral of `rows` turns, out to `radius`
    Spiral,
    /// `count` along an arc of `spread` at `radius`, centred on you
    Arc,
    /// `count` across like a line, rising and falling `rows` times
    Ribbon,
    /// two walls of `count` orbs each, either side of the line to you,
    /// `radius` out from it and `length` long, open at both ends
    Lane,
    /// a half sphere of `rows` rings round the anchor at `radius`, the
    /// lowest of `count` orbs, each ring turned half a gap from the last
    Dome,
}

/// Where a formation is placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// round the enemy
    Body,
    /// `at` metres from the enemy toward you
    Front,
    /// round your chest
    Target,
    /// `at` metres over your chest
    Above,
}

/// The way a held orb sets off when its hold ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Launch {
    /// on along the way it was going (an aimed orb's aim)
    Keep,
    /// away from the formation's centre
    Out,
    /// toward the formation's centre
    In,
    /// toward the formation's centre, and spent there: a closing dome
    Gather,
    /// at your chest as it is when the orb sets off
    Aim,
    /// at where your chest will be when it arrives, from how you are moving
    Lead,
    Down,
    /// along the line from the enemy to you when it was placed
    Along,
    /// round the formation's centre, in its plane
    Spin,
    /// straight across to the formation's middle line: a lane's walls close
    Close,
}

/// A second hold: after `stop` ticks of flight the orb stops, waits
/// `wait`, and sets off again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rehold {
    pub stop: i32,
    pub wait: i32,
    pub launch: Launch,
    pub speed: i32,
}

/// One volley in simulation units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Volley {
    pub shape: Shape,
    pub count: i32,
    pub speed: i32,
    pub accel: i32,
    pub min_speed: i32,
    pub max_speed: i32,
    pub curl: i32,
    pub alternate: bool,
    /// the orb's radius; for a wave, how far it travels
    pub size: i32,
    pub colour: String,
    pub spin: i32,
    pub repeat: i32,
    pub every: i32,
    pub delay: i32,
    pub pitch: i32,
    pub spread: i32,
    pub rings: i32,
    pub gold: bool,
    pub aim: bool,
    /// health a hit takes
    pub damage: i32,
    /// the Injunction breaks it
    /// the Injunction's ward that breaks it (0: none does)
    pub ward: i32,
    pub form: Form,
    /// a grid's rows, a star's orbs to an arm, a spiral's turns, a
    /// ribbon's rises
    pub rows: i32,
    pub anchor: Anchor,
    /// the anchor's distance (Front) or height (Above)
    pub at: i32,
    /// a formation in the floor's plane (flat) or standing, facing you
    pub flat: bool,
    pub radius: i32,
    /// ticks every orb hangs before it flies, and more for each orb after
    /// the first
    pub hold: i32,
    pub stagger: i32,
    pub launch: Launch,
    pub rehold: Option<Rehold>,
    /// walls it glances off before a wall stops it
    pub bounce: i32,
    /// gravity on it, per tick per tick: a lobbed orb
    pub fall: i32,
    /// the pattern a lobbed orb fires where it lands
    pub plant: Option<usize>,
    /// an aimed volley's repeats sweep across this angle, one end to the other
    pub sweep: i32,
    /// a lane's walls' length
    pub length: i32,
}

impl Volley {
    /// The tick of the pattern's clock this volley's last repeat fires on.
    pub fn last_tick(&self) -> i32 {
        self.delay + (self.repeat - 1) * self.every
    }

    /// How many orbs one repeat places.
    pub fn orbs(&self) -> i32 {
        match (self.shape, self.form) {
            (Shape::Form, Form::Grid | Form::Star | Form::Dome) => self.count * self.rows,
            (Shape::Form, Form::Lane) => self.count * 2,
            _ => self.count,
        }
    }
}

fn launch_of(s: &str, who: &str) -> Result<Launch, String> {
    Ok(match s {
        "" | "keep" => Launch::Keep,
        "out" => Launch::Out,
        "in" => Launch::In,
        "gather" => Launch::Gather,
        "aim" => Launch::Aim,
        "lead" => Launch::Lead,
        "down" => Launch::Down,
        "along" => Launch::Along,
        "spin" => Launch::Spin,
        "close" => Launch::Close,
        x => return Err(format!("patterns: {who} has no launch {x}")),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pattern {
    pub id: String,
    pub volleys: Vec<Volley>,
}

impl Pattern {
    /// Ticks from the first volley to the last.
    pub fn length(&self) -> i32 {
        self.volleys.iter().map(|v| v.last_tick()).max().unwrap_or(0)
    }
}

// ---- enemies ---------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
struct EnemiesFile {
    enemies: Vec<EnemyFile>,
}

#[derive(Debug, Clone, Deserialize)]
struct AttackFile {
    pattern: String,
    windup_ms: i32,
    rest_ms: i32,
    /// the aim stops following you this long before it fires
    #[serde(default)]
    lock_ms: i32,
    /// fired over cover: it needs no line of sight
    #[serde(default)]
    indirect: bool,
    /// a tank's attack fired from every cannon at once
    #[serde(default)]
    cannons: bool,
    /// as its windup ends, time stops this long for all but its caster
    #[serde(default)]
    stop_ms: i32,
    /// taken one time in this many it comes up, passed over the rest
    #[serde(default)]
    once_in: i32,
    /// what the caster does with its body: "hook" or "pound"
    #[serde(default, rename = "move")]
    moves: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PartFile {
    /// "hull" or "turret"
    on: String,
    /// in the part's frame: x right, y up, z forward, cm
    at_cm: [i32; 3],
    #[serde(default)]
    radius_cm: i32,
    #[serde(default)]
    hp: i32,
    /// a cannon's aim off the part's facing, degrees
    #[serde(default)]
    yaw_deg: i32,
    #[serde(default)]
    pitch_deg: i32,
}

#[derive(Debug, Clone, Deserialize)]
struct PhaseFile {
    #[serde(default)]
    from_pct: i32,
    #[serde(default)]
    summons: Vec<String>,
    attacks: Vec<AttackFile>,
    /// a tank's phase: the weak points it opens, the cannons it grows,
    /// and, for its last, how long you must outlast it
    #[serde(default)]
    weak: Vec<PartFile>,
    #[serde(default)]
    cannons: Vec<PartFile>,
    #[serde(default)]
    survive_ms: i32,
}

#[derive(Debug, Clone, Deserialize)]
struct EnemyFile {
    id: String,
    name: String,
    kind: String,
    hp: i32,
    radius_cm: i32,
    hover_cm: i32,
    speed_cms: i32,
    range_cm: i32,
    contact: i32,
    clemency: i32,
    attacks: Vec<AttackFile>,
    #[serde(default)]
    phases: Vec<PhaseFile>,
    /// a pattern fired once where it dies
    #[serde(default)]
    burst: String,
    /// an agent's part in its squad
    #[serde(default)]
    role: String,
    /// shots from within this many degrees of its facing do nothing
    #[serde(default)]
    armour_deg: i32,
    /// it calls these while it lives, one batch each `summon_ms`, while
    /// fewer than `summon_cap` of its own are up; below `rally_pct` health
    /// it calls `rally` once
    #[serde(default)]
    summon: Vec<String>,
    #[serde(default)]
    summon_ms: i32,
    #[serde(default)]
    summon_cap: i32,
    #[serde(default)]
    rally_pct: i32,
    #[serde(default)]
    rally: Vec<String>,
    /// a bar of its own on the HUD
    #[serde(default)]
    elite: bool,
    /// a beast's way of moving: bound, hop, wheel, root or surge
    #[serde(default)]
    gait: String,
    /// a rooted beast bites this far when you come this close
    #[serde(default)]
    bite_cm: i32,
    /// what it splits into where it dies
    #[serde(default)]
    splits: Vec<String>,
    /// the god's lullaby: the song you play it to sleep with
    #[serde(default)]
    lullaby: Option<LullabyFile>,
}

#[derive(Debug, Clone, Deserialize)]
struct VerseFile {
    /// each note: the half beat it falls on, and its key (0 to 4)
    notes: Vec<[i32; 2]>,
    /// the verse's length in half beats
    length: i32,
}

#[derive(Debug, Clone, Deserialize)]
struct OnBeatFile {
    pattern: String,
    /// fired every this many beats, this many beats into the count
    every: i32,
    #[serde(default)]
    at: i32,
    /// from this verse on (counted from 0)
    #[serde(default)]
    from_verse: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct LullabyFile {
    beat_ms: i32,
    count_in: i32,
    pass_pct: i32,
    great_ms: i32,
    good_ms: i32,
    verses: Vec<VerseFile>,
    beats: Vec<OnBeatFile>,
    /// fired when a verse is played badly and the god stirs
    stir: String,
    /// the finale lasts at least this many beats: the last verse, played
    /// over the god as time stops round it
    finale_beats: i32,
    /// how long the trumpet takes to come out, before the first verse
    intro_ms: i32,
}

/// One verse of the lullaby.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verse {
    /// each note: the tick it falls on (from the verse's first beat) and
    /// its key
    pub notes: Vec<(i32, usize)>,
    pub length: i32,
}

/// A pattern the god fires on the song's beat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnBeat {
    pub pattern: usize,
    pub every: i32,
    pub at: i32,
    pub from_verse: usize,
}

/// The song the Defendant plays the Idiot God to sleep with: verses the
/// dancers pipe and you play back, on a beat, in time (ticks throughout).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lullaby {
    pub beat: i32,
    pub count_in: i32,
    pub pass_pct: i32,
    pub great: i32,
    pub good: i32,
    pub verses: Vec<Verse>,
    pub beats: Vec<OnBeat>,
    pub stir: usize,
    pub finale_beats: i32,
    pub intro: i32,
}

/// The keys of the lullaby's five notes, by the action each is pressed
/// with: the trumpet frees them from the gun and the abilities.
pub const NOTE_KEYS: [&str; 5] = ["fire", "parry", "interact", "wall", "grapple"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Turret,
    Floater,
    Walker,
    Moth,
    Boss,
    /// hovers, and at the end of each rest reappears somewhere else in sight
    Blinker,
    /// under the floor between attacks, where it cannot be hurt
    Burrower,
    /// a person: walks the floors, takes cover, works with its squad
    Agent,
    /// the Executor: a tracked tank, hurt only in its weak points
    Tank,
    /// a mech after Iron Vector's: glides, quick-boosts, ascends
    Mech,
    /// the Defendant: moves as you do, and stops time
    Double,
    /// the mountain's creatures: each moves by its gait
    Beast,
    /// the Idiot God: phases as a boss's, each its own bar; when the last
    /// empties it does not die, and the lullaby begins
    God,
}

/// How a beast moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gait {
    None,
    /// stands, then leaps far in a high arc to a new place round you
    Bound,
    /// short low hops, often
    Hop,
    /// circles high over you, and dives as it winds up
    Wheel,
    /// never moves; with a bite, lunges at you when you come near
    Root,
    /// rushes along the ground in sweeping curves past you
    Surge,
    /// sits on high perches round the arena, flies between them, and now
    /// and then swoops through where you stand
    Perch,
    /// steps sideways in the air, a step at a time, then a step toward
    /// you, then back the other way (the Glyph)
    March,
    /// moves only while you are not looking at it, and then fast (the
    /// Etching)
    Unseen,
    /// turns on the spot until it faces you, then charges straight, as
    /// tank controls do (the Render)
    Charge,
    /// bounces on a beat every one of its kind shares (the Toon)
    Bounce,
    /// walks the path you walked a few seconds before (the Reprise)
    Echo,
}

/// An agent's part in its squad.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    None,
    /// takes cover, steps out to fire; a partner in a crossfire
    Rifle,
    /// goes to your back
    Flank,
    /// keeps far and high, aims with a sight line
    Sniper,
    /// carries a shield and walks at you
    Shield,
    /// lobs over cover
    Grenadier,
    /// runs straight at you
    Rush,
    /// while it lives the squad rests less and rings you
    Command,
    /// the Section Chief: stays back and calls agents
    Chief,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attack {
    pub pattern: usize,
    pub windup: i32,
    pub rest: i32,
    pub lock: i32,
    pub indirect: bool,
    pub cannons: bool,
    pub stop: i32,
    /// taken one time in this many (1: every time)
    pub once_in: i32,
    pub moves: Lunge,
}

/// What a skirmisher does with its body in an attack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lunge {
    None,
    /// throws a chain to beside you in its windup and pulls itself there
    /// as it casts
    Hook,
    /// leaps over you in its windup and drops as it casts
    Pound,
}

/// Which part of a tank a weak point or cannon is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum On {
    Hull,
    Turret,
}

/// A weak point or a cannon on a tank, in its part's frame (Q16 metres:
/// x right, y up, z forward).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Part {
    pub on: On,
    pub at: [i32; 3],
    pub radius: i32,
    pub hp: i32,
    pub yaw: i32,
    pub pitch: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phase {
    pub from_pct: i32,
    pub summons: Vec<usize>,
    pub attacks: Vec<Attack>,
    pub weak: Vec<Part>,
    pub cannons: Vec<Part>,
    pub survive: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnemyDef {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    pub hp: i32,
    pub radius: i32,
    pub hover: i32,
    pub speed: i32,
    pub range: i32,
    pub contact: i32,
    pub clemency: i32,
    /// a boss's phases; every other enemy has one phase of its attacks
    pub phases: Vec<Phase>,
    /// the pattern it fires once where it dies
    pub burst: Option<usize>,
    pub role: Role,
    /// half the angle in front of it that shots glance off; 0 for none
    pub armour: i32,
    pub summon: Vec<usize>,
    pub summon_every: i32,
    pub summon_cap: i32,
    pub rally_pct: i32,
    pub rally: Vec<usize>,
    pub elite: bool,
    pub gait: Gait,
    /// how far a rooted beast's bite reaches (0: it does not bite)
    pub bite: i32,
    /// what it splits into where it dies
    pub splits: Vec<usize>,
    pub lullaby: Option<Lullaby>,
}

// ---- the zones and their routes ---------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RouteRules {
    pub main_fights: usize,
    pub main_tiers: Vec<i32>,
    pub detours_min: usize,
    pub detours_max: usize,
    pub detour_max: usize,
    pub rewards: Vec<String>,
    pub clemency_main: i32,
    pub clemency_detour: i32,
    pub clemency_boss: i32,
    #[serde(default)]
    pub rest_rewards: Vec<String>,
    /// rewards a zone's arrival offers as a choice of one (zone two's
    /// expedition cache); none in zone one
    #[serde(default)]
    pub cache_rewards: Vec<String>,
    /// the levels a gun on a near and on a deep side path has
    #[serde(default = "two_three")]
    pub weapon_levels: [i32; 2],
    /// mini-bosses, each fought alone in its own arena on a main fight
    #[serde(default)]
    pub minibosses: Vec<Miniboss>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Miniboss {
    pub enemy: String,
    /// the main fight it takes, counted from 0
    pub at: usize,
    pub arena: String,
}

fn two_three() -> [i32; 2] {
    [2, 3]
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EncounterFile {
    pub tier: i32,
    pub waves: Vec<Vec<String>>,
}

/// The palette names a zone's scene is lit and outlined with.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Scene {
    pub sky: String,
    pub fog: String,
    pub light: String,
    pub window_dark: String,
    pub ink: String,
    /// where the fog starts and is full, in metres
    pub fog_near_m: i32,
    pub fog_far_m: i32,
    /// how far space bends away from you, in percent (zone five: the
    /// scene curls up round you in the distance, and breathes)
    #[serde(default)]
    pub warp_pct: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ZoneFileEntry {
    id: String,
    name: String,
    year: String,
    built: bool,
    boss: String,
    #[serde(default)]
    scene: Option<Scene>,
    #[serde(default)]
    route: Option<RouteRules>,
    #[serde(default)]
    archetypes: Vec<String>,
    #[serde(default)]
    names: BTreeMap<String, String>,
    #[serde(default)]
    encounters: Vec<EncounterFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ZoneFile {
    zones: Vec<ZoneFileEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Encounter {
    pub tier: i32,
    pub waves: Vec<Vec<usize>>,
}

/// A zone of the route. A sealed zone has only its name, year and boss.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    pub id: String,
    pub name: String,
    pub year: String,
    pub built: bool,
    pub boss: String,
    pub scene: Scene,
    pub route: RouteRules,
    pub archetypes: Vec<String>,
    pub names: BTreeMap<String, String>,
    pub encounters: Vec<Encounter>,
}

impl Zone {
    pub fn place_name(&self, key: &str) -> String {
        self.names.get(key).cloned().unwrap_or_else(|| key.to_string())
    }

    /// The highest encounter tier the zone has.
    pub fn max_tier(&self) -> i32 {
        self.encounters.iter().map(|e| e.tier).max().unwrap_or(1)
    }
}

// ---- upgrades --------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct UpgradesFile {
    upgrades: Vec<Upgrade>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Upgrade {
    pub id: String,
    pub name: String,
    pub cost: i32,
    pub requires: String,
    pub effect: String,
    pub amount: i32,
    pub icon: String,
    /// the Holding's column it stands in
    pub branch: String,
    /// a boss that must have been beaten before it can be bought
    #[serde(default)]
    pub unlock: String,
}

/// The rewards a side path or the Antechamber can hold.
pub const REWARDS: &[&str] = &[
    "weapon", "jump", "health", "speed", "pierce", "pulse", "mend", "trigger", "plating",
];

const EFFECTS: &[&str] = &[
    "max_health",
    "speed",
    "jumps",
    "dash",
    "grapple",
    "parry",
    "recess",
    "armory",
    "wall",
    "pound",
    "pound_plus",
    "grapple_plus",
    "parry_plus",
    "wall_plus",
    "wall_ward",
    "appeal",
    "precedent",
    "conduct",
    "bail",
    "lien",
    "stop",
    "stop_plus",
];

/// The Holding's columns, in order.
pub const BRANCHES: &[&str] = &["body", "motion", "arms", "writs"];

// ---- story -----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BossWords {
    pub name: String,
    pub title: String,
    /// a bar of its own for each phase, by these names (the Idiot God)
    #[serde(default)]
    pub bars: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Story {
    pub case: String,
    pub sentencing: Vec<String>,
    pub counts: Vec<String>,
    pub addenda: Vec<String>,
    /// the indictment's heading on a death, by the zone you died in
    pub death_titles: BTreeMap<String, String>,
    pub time_served: String,
    pub victory_title: String,
    pub victory: Vec<String>,
    /// the verdict's picture, web/art/<verdict_art>.png
    pub verdict_art: String,
    pub bosses: BTreeMap<String, BossWords>,
    /// read between a zone's boss and the next zone, by the zone just left
    #[serde(default)]
    pub interludes: BTreeMap<String, Interlude>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Interlude {
    pub title: String,
    pub lines: Vec<String>,
    /// its picture, web/art/<art>.png
    pub art: String,
    /// the button that goes on into the next zone
    #[serde(default)]
    pub go: String,
}

// ---- everything ------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    pub pal: Palette,
    pub guns: Vec<Gun>,
    pub patterns: Vec<Pattern>,
    pub enemies: Vec<EnemyDef>,
    pub zones: Vec<Zone>,
    pub upgrades: Vec<Upgrade>,
    pub story: Story,
    copy: BTreeMap<String, Value>,
}

/// The data files, by name, as the page passes them in.
pub struct Files<'a> {
    pub weapons: &'a str,
    pub patterns: &'a str,
    pub enemies: &'a str,
    pub zone: &'a str,
    pub upgrades: &'a str,
    pub palette: &'a str,
    pub story: &'a str,
    pub copy: &'a str,
}

impl Content {
    pub fn parse(f: &Files) -> Result<Content, String> {
        let pal = Palette::parse(f.palette)?;
        let guns = parse_guns(f.weapons, &pal)?;
        let patterns = parse_patterns(f.patterns, &pal)?;
        let enemies = parse_enemies(f.enemies, &patterns)?;
        let file: ZoneFile = serde_json::from_str(f.zone).map_err(|e| format!("zone: {e}"))?;
        let find_enemy = |id: &str| {
            enemies
                .iter()
                .position(|e| e.id == id)
                .ok_or_else(|| format!("zone: no enemy {id}"))
        };
        let mut zones = Vec::new();
        for z in file.zones {
            let mut encounters = Vec::new();
            for e in &z.encounters {
                let waves = e
                    .waves
                    .iter()
                    .map(|w| w.iter().map(|id| find_enemy(id)).collect::<Result<Vec<_>, _>>())
                    .collect::<Result<Vec<_>, _>>()?;
                if waves.is_empty() || waves.iter().any(|w| w.is_empty()) {
                    return Err(format!("zone {}: an encounter has an empty wave", z.id));
                }
                encounters.push(Encounter { tier: e.tier, waves });
            }
            let (Some(route), Some(scene)) = (z.route.clone(), z.scene.clone()) else {
                if z.built {
                    return Err(format!("zone {}: a built zone needs a route and a scene", z.id));
                }
                zones.push(Zone {
                    id: z.id,
                    name: z.name,
                    year: z.year,
                    built: false,
                    boss: z.boss,
                    scene: Scene {
                        sky: "sky".into(),
                        fog: "fog".into(),
                        light: "light".into(),
                        window_dark: "window_dark".into(),
                        ink: "ink".into(),
                        fog_near_m: 70,
                        fog_far_m: 520,
                        warp_pct: 0,
                    },
                    route: RouteRules {
                        main_fights: 0,
                        main_tiers: Vec::new(),
                        detours_min: 0,
                        detours_max: 0,
                        detour_max: 0,
                        rewards: Vec::new(),
                        clemency_main: 0,
                        clemency_detour: 0,
                        clemency_boss: 0,
                        rest_rewards: Vec::new(),
                        cache_rewards: Vec::new(),
                        weapon_levels: two_three(),
                        minibosses: Vec::new(),
                    },
                    archetypes: Vec::new(),
                    names: z.names,
                    encounters,
                });
                continue;
            };
            if z.built {
                let r = &route;
                if r.main_tiers.len() != r.main_fights {
                    return Err(format!("zone {}: main_tiers needs one tier per main fight", z.id));
                }
                for t in r.main_tiers.iter().flat_map(|t| [*t, *t + 1]) {
                    let top = encounters.iter().map(|e| e.tier).max().unwrap_or(0);
                    if !encounters.iter().any(|e| e.tier == t.min(top)) {
                        return Err(format!("zone {}: no encounter of tier {t}", z.id));
                    }
                }
                for reward in r.rewards.iter().chain(&r.rest_rewards).chain(&r.cache_rewards) {
                    if !REWARDS.contains(&reward.as_str()) {
                        return Err(format!("zone {}: no such reward {reward}", z.id));
                    }
                }
                if z.archetypes.is_empty() {
                    return Err(format!("zone {}: no archetypes", z.id));
                }
                find_enemy(&z.boss)?;
                for mb in &r.minibosses {
                    find_enemy(&mb.enemy)?;
                    if mb.at >= r.main_fights || mb.arena.is_empty() {
                        return Err(format!(
                            "zone {}: the mini-boss needs a main fight and an arena",
                            z.id
                        ));
                    }
                }
                for name in [
                    &scene.sky,
                    &scene.fog,
                    &scene.light,
                    &scene.window_dark,
                    &scene.ink,
                ] {
                    if !pal.has(name) {
                        return Err(format!("zone {}: the scene names no colour {name}", z.id));
                    }
                }
            }
            zones.push(Zone {
                id: z.id,
                name: z.name,
                year: z.year,
                built: z.built,
                boss: z.boss,
                scene,
                route,
                archetypes: z.archetypes,
                names: z.names,
                encounters,
            });
        }
        if !zones.first().is_some_and(|z| z.built) {
            return Err("zone: the first zone must be built".into());
        }
        let upgrades: UpgradesFile =
            serde_json::from_str(f.upgrades).map_err(|e| format!("upgrades: {e}"))?;
        for u in &upgrades.upgrades {
            if !EFFECTS.contains(&u.effect.as_str()) {
                return Err(format!("upgrades: {} has no effect {}", u.id, u.effect));
            }
            if !BRANCHES.contains(&u.branch.as_str()) {
                return Err(format!("upgrades: {} is in no branch {}", u.id, u.branch));
            }
            if !u.requires.is_empty() && !upgrades.upgrades.iter().any(|o| o.id == u.requires) {
                return Err(format!("upgrades: {} requires unknown {}", u.id, u.requires));
            }
        }
        let story: Story = serde_json::from_str(f.story).map_err(|e| format!("story: {e}"))?;
        let copy: BTreeMap<String, Value> = serde_json::from_str(f.copy).map_err(|e| format!("copy: {e}"))?;
        Ok(Content {
            pal,
            guns,
            patterns,
            enemies,
            zones,
            upgrades: upgrades.upgrades,
            story,
            copy,
        })
    }

    /// A string from copy.json; a missing key shows as itself, and the test
    /// `every_copy_key_the_core_uses_is_in_copy_json` keeps that from shipping.
    pub fn say(&self, key: &str) -> String {
        self.copy
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or(key)
            .to_string()
    }

    /// A list of two-string rows from copy.json (the controls table).
    pub fn copy_rows(&self, key: &str) -> Vec<(String, String)> {
        self.copy
            .get(key)
            .and_then(|v| v.as_array())
            .map(|rows| {
                rows.iter()
                    .filter_map(|r| Some((r.get(0)?.as_str()?.to_string(), r.get(1)?.as_str()?.to_string())))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn has_copy(&self, key: &str) -> bool {
        self.copy.get(key).is_some_and(|v| v.is_string())
    }

    pub fn gun(&self, id: &str) -> usize {
        self.guns.iter().position(|g| g.id == id).unwrap_or(0)
    }

    pub fn enemy(&self, id: &str) -> Option<usize> {
        self.enemies.iter().position(|e| e.id == id)
    }

    /// The zones a run can be in, in order.
    pub fn built_zones(&self) -> usize {
        self.zones.iter().take_while(|z| z.built).count()
    }

    /// The zone an enemy belongs to, by index, if one lists it.
    pub fn zone_of_enemy(&self, def: usize) -> Option<usize> {
        // what another splits into is of that one's zone
        if let Some(parent) = self.enemies.iter().position(|e| e.splits.contains(&def)) {
            if parent != def {
                return self.zone_of_enemy(parent);
            }
        }
        let id = &self.enemies[def].id;
        self.zones.iter().position(|z| {
            &z.boss == id
                || z.encounters
                    .iter()
                    .any(|e| e.waves.iter().any(|w| w.contains(&def)))
        })
    }
}

fn parse_guns(text: &str, pal: &Palette) -> Result<Vec<Gun>, String> {
    let f: WeaponsFile = serde_json::from_str(text).map_err(|e| format!("weapons: {e}"))?;
    let mut out = Vec::new();
    for w in f.weapons {
        if w.levels.is_empty() {
            return Err(format!("weapons: {} has no levels", w.id));
        }
        if !pal.has(&w.colour) {
            return Err(format!("weapons: {} names no colour {}", w.id, w.colour));
        }
        let levels = w
            .levels
            .iter()
            .map(|l| GunLevel {
                damage: l.damage,
                rate: ms(l.rate_ms).max(1),
                pellets: l.pellets.max(1),
                spread: fx::deg(l.spread_deg),
                speed: cms(l.speed_cms),
                size: cm(l.size_cm),
                life: ms(l.life_ms),
                splash_radius: cm(l.splash_cm),
                splash: l.splash,
                pierce: l.pierce,
                mag: l.mag.max(0),
                shell: ms(l.shell_ms).max(1),
                saw: (l.saws > 0).then(|| SawLevel {
                    saws: l.saws,
                    rev_damage: l.rev_damage,
                    rev_every: ms(l.rev_ms).max(1),
                    teeth: l.teeth.max(1),
                    recall: ms(l.recall_ms),
                }),
            })
            .collect();
        out.push(Gun {
            id: w.id,
            name: w.name,
            blurb: w.blurb,
            colour: w.colour,
            levels,
            from_zone: w.from_zone,
            alt: Alt {
                kind: match w.alt.kind.as_str() {
                    "burst" => AltKind::Burst,
                    "slug" => AltKind::Slug,
                    "cluster" => AltKind::Cluster,
                    "detonate" => AltKind::Detonate,
                    "fan" => AltKind::Fan,
                    k => return Err(format!("weapons: a gun has no alt fire {k}")),
                },
                name: w.alt.name.clone(),
                damage: w.alt.damage,
                count: w.alt.count.max(1),
                speed: cms(w.alt.speed_cms),
                size: cm(w.alt.size_cm),
                life: ms(w.alt.life_ms),
                pierce: w.alt.pierce,
                splash_radius: cm(w.alt.splash_cm),
                splash: w.alt.splash,
                spread: fx::deg(w.alt.spread_deg),
                cooldown: ms(w.alt.cooldown_ms).max(1),
            },
        });
    }
    Ok(out)
}

fn parse_patterns(text: &str, pal: &Palette) -> Result<Vec<Pattern>, String> {
    let f: PatternsFile = serde_json::from_str(text).map_err(|e| format!("patterns: {e}"))?;
    let mut out = Vec::new();
    let mut plants: Vec<(usize, usize, String)> = Vec::new();
    for p in f.patterns {
        let mut volleys = Vec::new();
        for v in p.volleys {
            let shape = match v.shape.as_str() {
                "ring" => Shape::Ring,
                "aimed" => Shape::Aimed,
                "sphere" => Shape::Sphere,
                "wave" => Shape::Wave,
                "form" => Shape::Form,
                s => return Err(format!("patterns: {} has no shape {s}", p.id)),
            };
            if !pal.has(&v.colour) {
                return Err(format!("patterns: {} names no colour {}", p.id, v.colour));
            }
            if v.repeat > 1 && v.every_ms <= 0 {
                return Err(format!("patterns: {} repeats with no every_ms", p.id));
            }
            let speed = cms(v.speed_cms);
            let form = match v.form.as_str() {
                "" | "ring" => Form::Ring,
                "grid" => Form::Grid,
                "line" => Form::Line,
                "column" => Form::Column,
                "star" => Form::Star,
                "spiral" => Form::Spiral,
                "arc" => Form::Arc,
                "ribbon" => Form::Ribbon,
                "lane" => Form::Lane,
                "dome" => Form::Dome,
                s => return Err(format!("patterns: {} has no form {s}", p.id)),
            };
            let anchor = match v.at.as_str() {
                "" | "self" => Anchor::Body,
                "front" => Anchor::Front,
                "target" => Anchor::Target,
                "above" => Anchor::Above,
                s => return Err(format!("patterns: {} has no anchor {s}", p.id)),
            };
            let flat = match v.plane.as_str() {
                "" | "flat" => true,
                "face" => false,
                s => return Err(format!("patterns: {} has no plane {s}", p.id)),
            };
            let launch = launch_of(&v.launch, &p.id)?;
            let rehold = if v.stop_ms > 0 {
                Some(Rehold {
                    stop: ms(v.stop_ms).max(1),
                    wait: ms(v.rehold_ms),
                    launch: launch_of(&v.relaunch, &p.id)?,
                    speed: cms(if v.relaunch_cms > 0 {
                        v.relaunch_cms
                    } else {
                        v.speed_cms
                    }),
                })
            } else {
                None
            };
            if form == Form::Lane && v.length_cm <= 0 {
                return Err(format!("patterns: {} is a lane with no length_cm", p.id));
            }
            if shape == Shape::Form && v.radius_cm <= 0 && form != Form::Column {
                return Err(format!("patterns: {} is a formation with no radius_cm", p.id));
            }
            let fastest = rehold.map_or(speed, |r| r.speed.max(speed));
            volleys.push(Volley {
                shape,
                count: v.count.max(1),
                speed,
                accel: cms2(v.accel_cms2),
                min_speed: if v.min_cms > 0 { cms(v.min_cms) } else { 0 },
                max_speed: if v.max_cms > 0 {
                    cms(v.max_cms).max(fastest)
                } else {
                    fastest
                },
                curl: degs(v.curl_degs),
                alternate: v.alternate,
                size: cm(v.size_cm),
                ward: pal.ward(&v.colour),
                colour: v.colour,
                spin: fx::deg(v.spin_deg),
                repeat: v.repeat.max(1),
                every: ms(v.every_ms),
                delay: ms(v.delay_ms),
                pitch: fx::deg(v.pitch_deg),
                spread: fx::deg(v.spread_deg),
                rings: v.rings.max(1),
                gold: v.gold,
                aim: v.aim,
                damage: if v.damage > 0 {
                    v.damage
                } else if shape == Shape::Wave {
                    crate::balance::WAVE_DAMAGE
                } else {
                    crate::balance::ORB_DAMAGE
                },
                form,
                rows: v.rows.max(1),
                anchor,
                at: cm(v.at_cm),
                flat,
                radius: cm(v.radius_cm),
                hold: ms(v.hold_ms),
                stagger: ms(v.stagger_ms),
                launch,
                rehold,
                bounce: v.bounce.max(0),
                fall: cms2(v.fall_cms2),
                plant: None,
                sweep: fx::deg(v.sweep_deg),
                length: cm(v.length_cm),
            });
            plants.push((out.len(), volleys.len() - 1, v.plant.clone()));
        }
        out.push(Pattern { id: p.id, volleys });
    }
    // a lobbed orb's pattern, by name, once every pattern is known
    for (p, v, name) in plants {
        if name.is_empty() {
            continue;
        }
        let k = out
            .iter()
            .position(|q| q.id == name)
            .ok_or_else(|| format!("patterns: {} plants no pattern {name}", out[p].id))?;
        if k == p {
            return Err(format!("patterns: {name} plants itself"));
        }
        out[p].volleys[v].plant = Some(k);
    }
    Ok(out)
}

fn parse_enemies(text: &str, patterns: &[Pattern]) -> Result<Vec<EnemyDef>, String> {
    let f: EnemiesFile = serde_json::from_str(text).map_err(|e| format!("enemies: {e}"))?;
    let ids: Vec<String> = f.enemies.iter().map(|e| e.id.clone()).collect();
    let attack = |a: &AttackFile, who: &str| -> Result<Attack, String> {
        let pattern = patterns
            .iter()
            .position(|p| p.id == a.pattern)
            .ok_or_else(|| format!("enemies: {who} names no pattern {}", a.pattern))?;
        Ok(Attack {
            pattern,
            windup: ms(a.windup_ms),
            rest: ms(a.rest_ms),
            lock: ms(a.lock_ms),
            indirect: a.indirect,
            cannons: a.cannons,
            stop: ms(a.stop_ms),
            once_in: a.once_in.max(1),
            moves: match a.moves.as_str() {
                "" => Lunge::None,
                "hook" => Lunge::Hook,
                "pound" => Lunge::Pound,
                m => return Err(format!("enemies: {who} has no move {m}")),
            },
        })
    };
    let part = |f: &PartFile, who: &str| -> Result<Part, String> {
        Ok(Part {
            on: match f.on.as_str() {
                "hull" => On::Hull,
                "turret" => On::Turret,
                x => return Err(format!("enemies: {who} has a part on no {x}")),
            },
            at: [cm(f.at_cm[0]), cm(f.at_cm[1]), cm(f.at_cm[2])],
            radius: cm(f.radius_cm),
            hp: f.hp,
            yaw: fx::deg(f.yaw_deg),
            pitch: fx::deg(f.pitch_deg),
        })
    };
    let find = |s: &str, who: &str| {
        ids.iter()
            .position(|i| i == s)
            .ok_or_else(|| format!("enemies: {who} summons unknown {s}"))
    };
    let mut out = Vec::new();
    for e in &f.enemies {
        let kind = match e.kind.as_str() {
            "turret" => Kind::Turret,
            "floater" => Kind::Floater,
            "walker" => Kind::Walker,
            "moth" => Kind::Moth,
            "boss" => Kind::Boss,
            "blinker" => Kind::Blinker,
            "burrower" => Kind::Burrower,
            "agent" => Kind::Agent,
            "tank" => Kind::Tank,
            "mech" => Kind::Mech,
            "double" => Kind::Double,
            "beast" => Kind::Beast,
            "god" => Kind::God,
            k => return Err(format!("enemies: {} has no kind {k}", e.id)),
        };
        let role = match e.role.as_str() {
            "" => Role::None,
            "rifle" => Role::Rifle,
            "flank" => Role::Flank,
            "sniper" => Role::Sniper,
            "shield" => Role::Shield,
            "grenadier" => Role::Grenadier,
            "rush" => Role::Rush,
            "command" => Role::Command,
            "chief" => Role::Chief,
            r => return Err(format!("enemies: {} has no role {r}", e.id)),
        };
        if kind == Kind::Agent && role == Role::None {
            return Err(format!("enemies: agent {} has no role", e.id));
        }
        let gait = match e.gait.as_str() {
            "" => Gait::None,
            "bound" => Gait::Bound,
            "hop" => Gait::Hop,
            "wheel" => Gait::Wheel,
            "root" => Gait::Root,
            "surge" => Gait::Surge,
            "perch" => Gait::Perch,
            "march" => Gait::March,
            "unseen" => Gait::Unseen,
            "charge" => Gait::Charge,
            "bounce" => Gait::Bounce,
            "echo" => Gait::Echo,
            g => return Err(format!("enemies: {} has no gait {g}", e.id)),
        };
        if (kind == Kind::Beast) != (gait != Gait::None) {
            return Err(format!(
                "enemies: {}: a beast, and only a beast, has a gait",
                e.id
            ));
        }
        let phases = if matches!(kind, Kind::Boss | Kind::Tank | Kind::Double | Kind::God) {
            if e.phases.is_empty() {
                return Err(format!("enemies: boss {} has no phases", e.id));
            }
            let mut ps = Vec::new();
            for p in &e.phases {
                let summons = p
                    .summons
                    .iter()
                    .map(|s| find(s, &e.id))
                    .collect::<Result<Vec<_>, _>>()?;
                ps.push(Phase {
                    from_pct: p.from_pct,
                    summons,
                    attacks: p
                        .attacks
                        .iter()
                        .map(|a| attack(a, &e.id))
                        .collect::<Result<_, _>>()?,
                    weak: p.weak.iter().map(|w| part(w, &e.id)).collect::<Result<_, _>>()?,
                    cannons: p
                        .cannons
                        .iter()
                        .map(|w| part(w, &e.id))
                        .collect::<Result<_, _>>()?,
                    survive: ms(p.survive_ms),
                });
            }
            if kind == Kind::Tank {
                // every phase but the last opens weak points to break; the
                // last is outlasted
                let last = ps.len() - 1;
                for (k, p) in ps.iter().enumerate() {
                    if k < last && (p.weak.is_empty() || p.weak.iter().any(|w| w.hp <= 0 || w.radius <= 0)) {
                        return Err(format!(
                            "enemies: {} phase {} has no weak point to break",
                            e.id,
                            k + 1
                        ));
                    }
                    if k == last && p.survive <= 0 && p.weak.is_empty() {
                        return Err(format!(
                            "enemies: {} last phase has nothing to break or outlast",
                            e.id
                        ));
                    }
                }
            }
            ps
        } else {
            vec![Phase {
                from_pct: 100,
                summons: Vec::new(),
                attacks: e
                    .attacks
                    .iter()
                    .map(|a| attack(a, &e.id))
                    .collect::<Result<_, _>>()?,
                weak: Vec::new(),
                cannons: Vec::new(),
                survive: 0,
            }]
        };
        let list = |v: &[String]| v.iter().map(|s| find(s, &e.id)).collect::<Result<Vec<_>, _>>();
        let pattern_of = |id: &str| {
            patterns
                .iter()
                .position(|p| p.id == id)
                .ok_or_else(|| format!("enemies: {} names no pattern {id}", e.id))
        };
        if (kind == Kind::God) != e.lullaby.is_some() {
            return Err(format!("enemies: {}: a god, and only a god, has a lullaby", e.id));
        }
        let lullaby = match &e.lullaby {
            None => None,
            Some(l) => {
                let beat = ms(l.beat_ms);
                if beat < 4 || beat % 2 != 0 || l.verses.is_empty() {
                    return Err(format!(
                        "enemies: {}'s lullaby needs verses and an even beat",
                        e.id
                    ));
                }
                let mut verses = Vec::new();
                for v in &l.verses {
                    let mut notes = Vec::new();
                    for [at, key] in &v.notes {
                        if *key < 0 || *key as usize >= NOTE_KEYS.len() || *at < 0 || *at >= v.length {
                            return Err(format!("enemies: {}'s lullaby has a note out of its verse", e.id));
                        }
                        notes.push((at * beat / 2, *key as usize));
                    }
                    if notes.is_empty() || notes.windows(2).any(|w| w[1].0 <= w[0].0) {
                        return Err(format!("enemies: {}'s lullaby has a verse out of order", e.id));
                    }
                    verses.push(Verse {
                        notes,
                        length: v.length * beat / 2,
                    });
                }
                Some(Lullaby {
                    beat,
                    count_in: l.count_in,
                    pass_pct: l.pass_pct,
                    great: ms(l.great_ms),
                    good: ms(l.good_ms),
                    verses,
                    beats: l
                        .beats
                        .iter()
                        .map(|b| {
                            Ok(OnBeat {
                                pattern: pattern_of(&b.pattern)?,
                                every: b.every.max(1),
                                at: b.at,
                                from_verse: b.from_verse,
                            })
                        })
                        .collect::<Result<_, String>>()?,
                    stir: pattern_of(&l.stir)?,
                    finale_beats: l.finale_beats,
                    intro: ms(l.intro_ms),
                })
            }
        };
        out.push(EnemyDef {
            id: e.id.clone(),
            name: e.name.clone(),
            kind,
            hp: e.hp,
            radius: cm(e.radius_cm),
            hover: cm(e.hover_cm),
            speed: cms(e.speed_cms),
            range: cm(e.range_cm),
            contact: e.contact,
            clemency: e.clemency,
            phases,
            burst: if e.burst.is_empty() {
                None
            } else {
                Some(
                    patterns
                        .iter()
                        .position(|p| p.id == e.burst)
                        .ok_or_else(|| format!("enemies: {} bursts into no pattern {}", e.id, e.burst))?,
                )
            },
            role,
            armour: fx::deg(e.armour_deg),
            summon: list(&e.summon)?,
            summon_every: ms(e.summon_ms),
            summon_cap: e.summon_cap,
            rally_pct: e.rally_pct,
            rally: list(&e.rally)?,
            elite: e.elite,
            gait,
            bite: cm(e.bite_cm),
            splits: list(&e.splits)?,
            lullaby,
        });
    }
    Ok(out)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// The real data files, read at compile time.
    pub fn content() -> Content {
        Content::parse(&files()).expect("the data files parse")
    }

    pub fn files() -> Files<'static> {
        Files {
            weapons: include_str!("../../../data/weapons.json"),
            patterns: include_str!("../../../data/patterns.json"),
            enemies: include_str!("../../../data/enemies.json"),
            zone: include_str!("../../../data/zone.json"),
            upgrades: include_str!("../../../data/upgrades.json"),
            palette: include_str!("../../../data/palette.json"),
            story: include_str!("../../../data/story.json"),
            copy: include_str!("../../../data/copy.json"),
        }
    }

    #[test]
    fn the_data_files_parse_and_hold_five_guns_of_four_levels() {
        let c = content();
        assert_eq!(c.guns.len(), 5);
        // the Big Iron: six rounds, the Ascent's
        let iron = &c.guns[c.gun("big_iron")];
        assert!(iron.levels.iter().all(|l| l.mag == 6));
        assert_eq!(iron.from_zone, 3);
        assert!(c.guns.iter().all(|g| g.levels.len() == 4));
        // the Ripsaw is the saw gun, and only the house offers it
        let saw = &c.guns[c.gun("ripsaw")];
        assert!(saw.levels.iter().all(|l| l.saw.is_some()));
        assert_eq!(saw.from_zone, 2);
        // the Carbine fires every 105 ms: 6.3 ticks, rounded to 6
        assert_eq!(c.guns[0].level(1).rate, 6);
        assert_eq!(c.guns[0].level(9), c.guns[0].level(4));
    }

    #[test]
    fn units_convert_as_written() {
        // 100 cm is one metre; 6000 cm/s is one metre a tick; 500 ms is 30 ticks
        assert_eq!(cm(100), ONE);
        assert_eq!(cms(6000), ONE);
        assert_eq!(ms(500), 30);
        assert_eq!(degs(60), fx::deg(1));
        assert_eq!(parse_hex("#ff0000").unwrap(), [ONE, 0, 0]);
        assert!(parse_hex("#ff00").is_err());
    }

    #[test]
    fn a_pattern_naming_a_missing_colour_is_refused() {
        let mut f = files();
        let bad = f.patterns.replace("\"violet\"", "\"ultraviolet\"");
        f.patterns = Box::leak(bad.into_boxed_str());
        let e = Content::parse(&f).unwrap_err();
        assert!(e.contains("ultraviolet"), "{e}");
    }

    #[test]
    fn an_encounter_naming_a_missing_enemy_is_refused() {
        let mut f = files();
        let bad = f.zone.replacen("\"drifter\"", "\"dragon\"", 1);
        f.zone = Box::leak(bad.into_boxed_str());
        let e = Content::parse(&f).unwrap_err();
        assert!(e.contains("dragon"), "{e}");
    }

    #[test]
    fn the_boss_has_three_phases_that_start_lower_each_time() {
        let c = content();
        let b = &c.enemies[c.enemy("bailiff").unwrap()];
        assert_eq!(b.kind, Kind::Boss);
        let starts: Vec<i32> = b.phases.iter().map(|p| p.from_pct).collect();
        assert_eq!(starts, vec![100, 66, 33]);
    }

    /// The regular enemies a zone's encounters use, and its boss.
    fn roster(c: &Content, z: usize) -> (Vec<usize>, usize) {
        let mut ids: Vec<usize> = c.zones[z]
            .encounters
            .iter()
            .flat_map(|e| e.waves.iter().flatten().copied())
            .collect();
        ids.sort();
        ids.dedup();
        (ids, c.enemy(&c.zones[z].boss).unwrap())
    }

    #[test]
    fn zone_two_has_as_many_enemies_as_zone_one_and_none_of_its_patterns() {
        let c = content();
        let (one, bailiff) = roster(&c, 0);
        let (two, witness) = roster(&c, 1);
        assert_eq!(one.len(), 7, "zone one: six and the Clerk");
        assert_eq!(two.len(), one.len());
        assert!(one.iter().all(|e| !two.contains(e)), "no enemy is in both zones");
        let patterns = |ids: &[usize], boss: usize| {
            let mut v: Vec<usize> = ids
                .iter()
                .chain(std::iter::once(&boss))
                .flat_map(|&e| {
                    c.enemies[e]
                        .phases
                        .iter()
                        .flat_map(|p| p.attacks.iter().map(|a| a.pattern))
                })
                .chain(ids.iter().filter_map(|&e| c.enemies[e].burst))
                .collect();
            v.sort();
            v.dedup();
            v
        };
        let p1 = patterns(&one, bailiff);
        let p2 = patterns(&two, witness);
        assert!(
            p2.iter().all(|p| !p1.contains(p)),
            "zone two shares no pattern with zone one"
        );
        // and every volley of zone two holds its orbs before they fly, or stops them in flight
        for &p in &p2 {
            for v in &c.patterns[p].volleys {
                let held = v.shape == Shape::Form || v.rehold.is_some() || v.delay > 0;
                assert!(
                    held,
                    "{}: a volley that neither holds nor stops",
                    c.patterns[p].id
                );
            }
            assert!(
                c.patterns[p]
                    .volleys
                    .iter()
                    .any(|v| v.shape == Shape::Form || v.rehold.is_some()),
                "{}",
                c.patterns[p].id
            );
        }
    }

    #[test]
    fn a_zone_naming_a_colour_the_palette_lacks_is_refused() {
        let mut f = files();
        let bad = f.zone.replacen("\"ice_ink\"", "\"ice_ink_missing\"", 1);
        f.zone = Box::leak(bad.into_boxed_str());
        let e = Content::parse(&f).unwrap_err();
        assert!(e.contains("ice_ink_missing"), "{e}");
    }

    #[test]
    fn the_house_has_seven_new_people_none_of_whose_patterns_came_before() {
        let c = content();
        let (one, _) = roster(&c, 0);
        let (two, _) = roster(&c, 1);
        let (three, boss) = roster(&c, 2);
        assert_eq!(three.len(), 7);
        assert!(
            three.iter().all(|&e| c.enemies[e].kind == Kind::Agent),
            "all of them people"
        );
        assert_eq!(c.enemies[boss].kind, Kind::Tank);
        let pats = |ids: &[usize]| -> Vec<usize> {
            ids.iter()
                .flat_map(|&e| {
                    c.enemies[e]
                        .phases
                        .iter()
                        .flat_map(|p| p.attacks.iter().map(|a| a.pattern))
                })
                .collect()
        };
        let before: Vec<usize> = pats(&one).into_iter().chain(pats(&two)).collect();
        let chief = c.enemy("chief").unwrap();
        for p in pats(&three).into_iter().chain(pats(&[boss, chief])) {
            assert!(!before.contains(&p), "{} came before", c.patterns[p].id);
        }
        // the roles a squad needs are all there
        let mut roles: Vec<Role> = three.iter().map(|&e| c.enemies[e].role).collect();
        roles.dedup();
        assert_eq!(roles.len(), 7, "{roles:?}");
    }

    #[test]
    fn the_ascent_has_its_assassins_and_its_creatures_none_of_whose_patterns_came_before() {
        let c = content();
        let earlier: Vec<usize> = (0..3)
            .flat_map(|z| {
                let (r, boss) = roster(&c, z);
                r.into_iter().chain(std::iter::once(boss))
            })
            .chain(["chief", "pioneer", "escort"].iter().filter_map(|id| c.enemy(id)))
            .collect();
        let (met, boss) = roster(&c, 3);
        let zone = &c.zones[3].id;
        let own: Vec<usize> = (0..c.enemies.len())
            .filter(|&e| c.zone_of_enemy(e) == Some(3) && e != boss)
            .collect();
        let agents = own.iter().filter(|&&e| c.enemies[e].kind == Kind::Agent).count();
        let creatures: Vec<usize> = own
            .iter()
            .copied()
            .filter(|&e| c.enemies[e].kind != Kind::Agent)
            .collect();
        assert_eq!(
            (agents, creatures.len()),
            (7, 7),
            "seven assassins and seven creatures of {zone}"
        );
        assert!(
            creatures.iter().all(|e| met.contains(e)),
            "every creature is met on the climb"
        );
        assert!(
            met.iter().any(|e| earlier.contains(e)),
            "some enemies from the zones below"
        );
        // mostly the mountain's creatures
        let all: Vec<usize> = c.zones[3]
            .encounters
            .iter()
            .flat_map(|e| e.waves.iter().flatten().copied())
            .collect();
        let beasts = all.iter().filter(|e| creatures.contains(e)).count();
        assert!(
            beasts * 3 > all.len() * 2,
            "{beasts} of {} are creatures",
            all.len()
        );
        // big fights: a wave of eight at the least, ten from the second
        // tier, and never more than can be up at once
        for e in &c.zones[3].encounters {
            for w in &e.waves {
                let least = if e.tier >= 2 { 10 } else { 8 };
                assert!(
                    w.len() >= least && w.len() <= crate::balance::MAX_ALIVE,
                    "a wave of {}",
                    w.len()
                );
            }
        }
        let owl = c.enemy("great_horned").unwrap();
        assert!(c.enemies[owl].elite && c.enemies[owl].gait == Gait::Perch);
        assert!(c.zones[3]
            .route
            .minibosses
            .iter()
            .any(|m| m.enemy == "great_horned"));
        let four = own;
        assert_eq!(c.enemies[boss].kind, Kind::Double);
        let pats = |ids: &[usize]| -> Vec<usize> {
            ids.iter()
                .flat_map(|&e| {
                    c.enemies[e]
                        .phases
                        .iter()
                        .flat_map(|p| p.attacks.iter().map(|a| a.pattern))
                })
                .collect()
        };
        let before = pats(&earlier);
        for p in pats(&four).into_iter().chain(pats(&[boss])) {
            assert!(!before.contains(&p), "{} came before", c.patterns[p].id);
        }
        // the Defendant stops time in every phase
        assert!(c.enemies[boss]
            .phases
            .iter()
            .all(|p| p.attacks.iter().any(|a| a.stop > 0)));
    }

    #[test]
    fn a_tank_phase_with_nothing_to_break_is_refused() {
        let mut f = files();
        let bad = f.enemies.replacen("\"weak\"", "\"weak_gone\"", 1);
        f.enemies = Box::leak(bad.into_boxed_str());
        let e = Content::parse(&f).unwrap_err();
        assert!(e.contains("no weak point"), "{e}");
    }
}
