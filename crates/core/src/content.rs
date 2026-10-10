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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    named: BTreeMap<String, Rgb>,
    pub walls: Vec<Rgb>,
    pub greens: Vec<Rgb>,
    pub signs: Vec<Rgb>,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gun {
    pub id: String,
    pub name: String,
    pub blurb: String,
    pub colour: String,
    pub levels: Vec<GunLevel>,
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
    /// at your chest as it is when the orb sets off
    Aim,
    /// at where your chest will be when it arrives, from how you are moving
    Lead,
    Down,
    /// along the line from the enemy to you when it was placed
    Along,
    /// round the formation's centre, in its plane
    Spin,
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
    pub blue: bool,
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
}

impl Volley {
    /// The tick of the pattern's clock this volley's last repeat fires on.
    pub fn last_tick(&self) -> i32 {
        self.delay + (self.repeat - 1) * self.every
    }

    /// How many orbs one repeat places.
    pub fn orbs(&self) -> i32 {
        match (self.shape, self.form) {
            (Shape::Form, Form::Grid | Form::Star) => self.count * self.rows,
            _ => self.count,
        }
    }
}

fn launch_of(s: &str, who: &str) -> Result<Launch, String> {
    Ok(match s {
        "" | "keep" => Launch::Keep,
        "out" => Launch::Out,
        "in" => Launch::In,
        "aim" => Launch::Aim,
        "lead" => Launch::Lead,
        "down" => Launch::Down,
        "along" => Launch::Along,
        "spin" => Launch::Spin,
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
}

#[derive(Debug, Clone, Deserialize)]
struct PhaseFile {
    from_pct: i32,
    summons: Vec<String>,
    attacks: Vec<AttackFile>,
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
}

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attack {
    pub pattern: usize,
    pub windup: i32,
    pub rest: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phase {
    pub from_pct: i32,
    pub summons: Vec<usize>,
    pub attacks: Vec<Attack>,
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
}

/// The rewards a side path or the Antechamber can hold.
pub const REWARDS: &[&str] = &["weapon", "jump", "health", "speed", "pierce", "pulse", "mend"];

/// The palette names whose orbs count as blue: the Injunction breaks them.
pub const BLUE: &[&str] = &["cyan", "blue"];

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
];

// ---- story -----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BossWords {
    pub name: String,
    pub title: String,
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
            })
            .collect();
        out.push(Gun {
            id: w.id,
            name: w.name,
            blurb: w.blurb,
            colour: w.colour,
            levels,
        });
    }
    Ok(out)
}

fn parse_patterns(text: &str, pal: &Palette) -> Result<Vec<Pattern>, String> {
    let f: PatternsFile = serde_json::from_str(text).map_err(|e| format!("patterns: {e}"))?;
    let mut out = Vec::new();
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
                blue: BLUE.contains(&v.colour.as_str()),
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
            });
        }
        out.push(Pattern { id: p.id, volleys });
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
        })
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
            k => return Err(format!("enemies: {} has no kind {k}", e.id)),
        };
        let phases = if kind == Kind::Boss {
            if e.phases.is_empty() {
                return Err(format!("enemies: boss {} has no phases", e.id));
            }
            let mut ps = Vec::new();
            for p in &e.phases {
                let summons = p
                    .summons
                    .iter()
                    .map(|s| {
                        ids.iter()
                            .position(|i| i == s)
                            .ok_or_else(|| format!("enemies: {} summons unknown {s}", e.id))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                ps.push(Phase {
                    from_pct: p.from_pct,
                    summons,
                    attacks: p
                        .attacks
                        .iter()
                        .map(|a| attack(a, &e.id))
                        .collect::<Result<_, _>>()?,
                });
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
            }]
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
    fn the_data_files_parse_and_hold_three_guns_of_three_levels() {
        let c = content();
        assert_eq!(c.guns.len(), 3);
        assert!(c.guns.iter().all(|g| g.levels.len() == 3));
        // the Carbine fires every 105 ms: 6.3 ticks, rounded to 6
        assert_eq!(c.guns[0].level(1).rate, 6);
        assert_eq!(c.guns[0].level(9), c.guns[0].level(3));
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
}
