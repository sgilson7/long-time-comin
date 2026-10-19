//! What lasts between runs: Clemency, the upgrades it bought, the deaths,
//! the years added to the sentence, and how much of the indictment has been
//! read. The page keeps it in the browser as JSON.

use crate::balance::{
    BASE_HEALTH, BASE_JUMPS, DASH_CHARGES, FIRST_DEATH_GRANT, SENTENCE_BONUS_PCT, STOP_BASE, STOP_PLUS,
};
use crate::content::Content;
use crate::player::Build;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Meta {
    pub clemency: i32,
    pub owned: Vec<String>,
    pub runs: i32,
    pub deaths: i32,
    pub wins: i32,
    /// years added to the sentence, Sifu's ageing as time served
    pub years: i32,
    /// counts of the indictment read so far
    pub counts_read: i32,
    pub seen_intro: bool,
    pub best_nodes: i32,
    pub clemency_earned: i32,
    /// a count is read the first time a run reaches the zone's hardest fights
    pub reached_far: bool,
    /// the furthest zone a run may start in: one past each boss beaten
    pub unlocked: i32,
    /// zone bosses beaten, over every run
    pub bosses_beaten: i32,
    /// boss rushes finished
    pub rush_wins: i32,
    /// the bosses beaten, by id, over every run
    pub beaten: Vec<String>,
    /// last missions won
    pub mission_wins: i32,
    /// the gun a run carries in, by id: the Carbine, or one an Exhibit
    /// bought (empty: the Carbine)
    pub carry: String,
    /// the timeline a run follows past the house: the Peak (empty) or the
    /// River (`river`), chosen on the Holding's timeline
    pub timeline: String,
    /// runs won on the river's timeline (the Gallows Oak burnt)
    pub river_wins: i32,
}

/// The boss whose fall opens the river's timeline.
pub const RIVER_OPENER: &str = "executor";
/// What the river asks of a body: the hook, the ground pound and the Bench.
pub const RIVER_NEEDS: [&str; 3] = ["writ", "gavel", "bench"];

/// What a death adds to the record, for the indictment screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Reading {
    /// every count read so far, the newest last
    pub counts: Vec<String>,
    /// the newest line, a count or an addendum
    pub newest: String,
    pub years_added: i32,
    pub total_years: i32,
    /// the clerk's grant on a first death
    pub grant: i32,
    /// the sentence bonus applied, in percent
    pub bonus_pct: i32,
    /// all the Clemency this death banked
    pub banked: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    Owned,
    Requires(String),
    Cost(i32),
    /// a boss must be beaten first, by id
    Locked(String),
    Unknown,
}

/// The boss whose defeat gives you the time stop.
pub const TIME_STOPPER: &str = "defendant";

impl Meta {
    pub fn load(text: &str, c: &Content) -> Meta {
        let mut m: Meta = serde_json::from_str(text).unwrap_or_default();
        // forget upgrades this build no longer has
        m.owned.retain(|id| c.upgrades.iter().any(|u| &u.id == id));
        m.owned.dedup();
        m.clemency = m.clemency.max(0);
        m.counts_read = m.counts_read.clamp(0, c.story.counts.len() as i32);
        m.unlocked = m.unlocked.clamp(0, c.built_zones() as i32 - 1);
        m
    }

    pub fn save(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn owns(&self, id: &str) -> bool {
        self.owned.iter().any(|o| o == id)
    }

    pub fn check(&self, c: &Content, id: &str) -> Result<(), Refusal> {
        let u = c.upgrades.iter().find(|u| u.id == id).ok_or(Refusal::Unknown)?;
        if self.owns(id) {
            return Err(Refusal::Owned);
        }
        if !u.unlock.is_empty() && !self.beaten.contains(&u.unlock) {
            return Err(Refusal::Locked(u.unlock.clone()));
        }
        if !u.requires.is_empty() && !self.owns(&u.requires) {
            return Err(Refusal::Requires(u.requires.clone()));
        }
        if self.clemency < u.cost {
            return Err(Refusal::Cost(u.cost - self.clemency));
        }
        Ok(())
    }

    pub fn buy(&mut self, c: &Content, id: &str) -> Result<(), Refusal> {
        self.check(c, id)?;
        let cost = c.upgrades.iter().find(|u| u.id == id).map_or(0, |u| u.cost);
        self.clemency -= cost;
        self.owned.push(id.to_string());
        Ok(())
    }

    /// The guns a run may carry in: the Carbine, and each an Exhibit bought.
    pub fn carry_choices(&self, c: &Content) -> Vec<usize> {
        let mut out = vec![c.gun("carbine")];
        for u in c
            .upgrades
            .iter()
            .filter(|u| u.effect == "exhibit" && self.owns(&u.id))
        {
            let g = c.gun(&u.gun);
            if c.guns.get(g).is_some_and(|x| x.id == u.gun) && !out.contains(&g) {
                out.push(g);
            }
        }
        out
    }

    /// Chooses the gun runs carry in; false if it is not one of the choices.
    pub fn choose_carry(&mut self, c: &Content, id: &str) -> bool {
        let g = c.gun(id);
        if c.guns.get(g).is_none_or(|x| x.id != id) || !self.carry_choices(c).contains(&g) {
            return false;
        }
        self.carry = id.to_string();
        true
    }

    /// The gun a run carries in, by index: the one chosen while it is still
    /// a choice, else the Carbine.
    pub fn carried(&self, c: &Content) -> usize {
        let g = c.gun(&self.carry);
        let choices = self.carry_choices(c);
        if !self.carry.is_empty() && choices.contains(&g) && c.guns[g].id == self.carry {
            g
        } else {
            choices[0]
        }
    }

    /// The body a run starts with, and the level of the gun it carries in.
    pub fn build(&self, c: &Content) -> (Build, i32) {
        let mut b = Build {
            max_health: BASE_HEALTH,
            jumps: BASE_JUMPS,
            dash_charges: DASH_CHARGES,
            ..Build::default()
        };
        let mut gun_level = 1;
        for u in c.upgrades.iter().filter(|u| self.owns(&u.id)) {
            match u.effect.as_str() {
                "max_health" => b.max_health += u.amount,
                "speed" => b.speed_pct += u.amount,
                "jumps" => b.jumps += u.amount,
                "dash" => b.dash_charges += u.amount,
                "grapple" => b.grapple = true,
                "parry" => b.parry = true,
                "recess" => b.recess = true,
                "wall" => b.wall = true,
                "pound" => b.pound = b.pound.max(1),
                "pound_plus" => b.pound = 2,
                "grapple_plus" => b.grapple_plus = true,
                "parry_plus" => b.parry_plus = true,
                "wall_plus" => b.wall_plus = true,
                // its amount is the ward it opens; the first is the wall's own
                "wall_ward" => b.wall_ward = b.wall_ward.max(u.amount - 1),
                "appeal" => b.appeal = true,
                "precedent" => b.pierce += u.amount,
                "bail" => b.bail += u.amount,
                "lien" => b.lien = true,
                "stop_plus" => b.stop_ticks += STOP_PLUS,
                "bench" => b.bench = b.bench.max(u.amount),
                "armory" => gun_level = gun_level.max(u.amount),
                _ => {}
            }
        }
        // the time stop is yours once you have beaten the one who stops it
        if self.beaten.iter().any(|x| x == TIME_STOPPER) {
            b.stop_ticks += STOP_BASE;
        } else {
            b.stop_ticks = 0;
        }
        (b, gun_level)
    }

    /// A run ended in death: bank its Clemency, add the years, read the
    /// next count (or an addendum once every count is read).
    /// The next line of the indictment: the next count, or an addendum
    /// once every count is read.
    pub fn read_next(&mut self, c: &Content, year: &str) -> String {
        let s = &c.story;
        if (self.counts_read as usize) < s.counts.len() {
            self.counts_read += 1;
            return s.counts[self.counts_read as usize - 1].clone();
        }
        let k = (self.deaths as usize) % s.addenda.len().max(1);
        s.addenda
            .get(k)
            .cloned()
            .unwrap_or_default()
            .replace("{year}", year)
    }

    /// Percent added to Clemency earned: `SENTENCE_BONUS_PCT` for each ten
    /// years of sentence.
    pub fn sentence_bonus_pct(&self) -> i32 {
        SENTENCE_BONUS_PCT * (self.years / 10)
    }

    /// Percent added by Good Conduct.
    pub fn conduct_pct(&self, c: &Content) -> i32 {
        c.upgrades
            .iter()
            .filter(|u| u.effect == "conduct" && self.owns(&u.id))
            .map(|u| u.amount)
            .sum()
    }

    /// A run ended in death: bank its Clemency (with the sentence bonus,
    /// and the clerk's grant on a first death), add the years, read the next
    /// line of the indictment.
    pub fn record_death(&mut self, c: &Content, earned: i32, nodes: i32, year: &str) -> Reading {
        self.runs += 1;
        let grant = if self.deaths == 0 { FIRST_DEATH_GRANT } else { 0 };
        let bonus_pct = self.sentence_bonus_pct() + self.conduct_pct(c);
        let banked = self.bank(c, earned, nodes) + grant;
        self.clemency += grant;
        self.clemency_earned += grant;
        self.deaths += 1;
        let years_added = self.deaths;
        self.years += years_added;
        let newest = self.read_next(c, year);
        Reading {
            counts: c.story.counts[..self.counts_read as usize].to_vec(),
            newest,
            years_added,
            total_years: self.years,
            grant,
            bonus_pct,
            banked,
        }
    }

    /// A finished boss rush: bank its Clemency.
    pub fn record_rush(&mut self, c: &Content, earned: i32, nodes: i32) -> i32 {
        self.runs += 1;
        self.rush_wins += 1;
        self.bank(c, earned, nodes)
    }

    /// The boss rush opens once the game has been beaten.
    pub fn rush_open(&self) -> bool {
        self.wins > 0
    }

    /// The last mission opens once the Idiot God (the last zone's boss)
    /// has been put to sleep, and while its own zone is built.
    pub fn mission_open(&self, c: &Content) -> bool {
        let last = c.built_zones().saturating_sub(1);
        c.mission_zone().is_some()
            && c.story.mission.is_some()
            && self.beaten.iter().any(|b| *b == c.zones[last].boss)
    }

    /// Why the river's timeline is shut, or nothing when it is open: its
    /// boss not yet beaten, or a part of its kit not yet bought.
    pub fn river_shut(&self, c: &Content) -> Option<Refusal> {
        if !self.beaten.iter().any(|b| b == RIVER_OPENER) {
            return Some(Refusal::Locked(RIVER_OPENER.to_string()));
        }
        RIVER_NEEDS
            .iter()
            .find(|id| !self.owns(id) && c.upgrades.iter().any(|u| u.id == **id))
            .map(|id| Refusal::Requires(id.to_string()))
    }

    /// Is a run on the river's timeline?
    pub fn on_river(&self, c: &Content) -> bool {
        self.timeline == "river" && self.river_shut(c).is_none()
    }

    /// A won run on the river's timeline: bank its Clemency. It is not the
    /// game beaten (the Idiot God is still to sleep).
    pub fn record_river(&mut self, c: &Content, earned: i32, nodes: i32) -> i32 {
        self.runs += 1;
        self.river_wins += 1;
        self.bosses_beaten += 1;
        self.bank(c, earned, nodes)
    }

    /// A won last mission: bank its Clemency.
    pub fn record_mission(&mut self, c: &Content, earned: i32, nodes: i32) -> i32 {
        self.runs += 1;
        self.mission_wins += 1;
        self.bank(c, earned, nodes)
    }

    /// A record with everything: every upgrade bought, every count read,
    /// every zone open and the game beaten, for trying the game out.
    pub fn everything(c: &Content) -> Meta {
        Meta {
            clemency: crate::balance::TEST_SAVE_CLEMENCY,
            owned: c.upgrades.iter().map(|u| u.id.clone()).collect(),
            runs: 1,
            deaths: 1,
            wins: 1,
            years: 1,
            counts_read: c.story.counts.len() as i32,
            seen_intro: true,
            best_nodes: 0,
            clemency_earned: 0,
            reached_far: true,
            unlocked: c.built_zones() as i32 - 1,
            bosses_beaten: c.built_zones() as i32,
            rush_wins: 0,
            carry: String::new(),
            mission_wins: 0,
            timeline: String::new(),
            river_wins: 0,
            beaten: c
                .zones
                .iter()
                .filter(|z| z.built)
                .map(|z| z.boss.clone())
                .collect(),
        }
    }

    /// A boss beaten: kept, for what it opens.
    pub fn beat(&mut self, boss: &str) {
        if !self.beaten.iter().any(|b| b == boss) {
            self.beaten.push(boss.to_string());
        }
    }

    /// A won run: bank its Clemency; the first win reads a count.
    pub fn record_win(&mut self, c: &Content, earned: i32, nodes: i32, year: &str) -> (i32, Option<String>) {
        self.runs += 1;
        self.wins += 1;
        self.bosses_beaten += 1;
        let banked = self.bank(c, earned, nodes);
        let newest = if self.wins == 1 {
            Some(self.read_next(c, year))
        } else {
            None
        };
        (banked, newest)
    }

    /// A zone's boss is down and the run goes on to zone `next`: the next
    /// zone may be started in from now on, and the first such crossing
    /// reads a count.
    pub fn record_crossing(&mut self, c: &Content, next: usize, year: &str) -> Option<String> {
        self.bosses_beaten += 1;
        if next < c.built_zones() {
            // a zone off the main line opens no later start of the main line
            self.unlocked = self.unlocked.max(next as i32).min(c.built_zones() as i32 - 1);
        }
        (self.bosses_beaten == 1).then(|| self.read_next(c, year))
    }

    /// Adds a run's Clemency with the sentence bonus; returns what was added.
    fn bank(&mut self, c: &Content, earned: i32, nodes: i32) -> i32 {
        let banked = earned * (100 + self.sentence_bonus_pct() + self.conduct_pct(c)) / 100;
        self.clemency += banked;
        self.clemency_earned += banked;
        self.best_nodes = self.best_nodes.max(nodes);
        banked
    }

    /// A different graph every run, the same one for the same record.
    pub fn run_seed(&self) -> u64 {
        0x5EED_0000 + self.runs as u64 * 7919 + self.deaths as u64 * 104_729
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::balance::GRAPPLE_RANGE as GRAPPLE_RANGE_BASE;
    use crate::content::tests::content;

    #[test]
    fn buying_spends_clemency_and_needs_what_it_requires() {
        let c = content();
        let mut m = Meta {
            clemency: 30,
            ..Meta::default()
        };
        assert_eq!(
            m.buy(&c, "constitution_2"),
            Err(Refusal::Requires("constitution_1".into()))
        );
        assert_eq!(m.buy(&c, "constitution_1"), Ok(()));
        assert_eq!(m.clemency, 22);
        assert_eq!(m.buy(&c, "constitution_1"), Err(Refusal::Owned));
        assert_eq!(m.buy(&c, "recess"), Err(Refusal::Cost(8)));
        assert_eq!(m.buy(&c, "nothing"), Err(Refusal::Unknown));
    }

    #[test]
    fn upgrades_make_the_body_a_run_starts_with() {
        let c = content();
        let m = Meta {
            owned: vec![
                "constitution_1".into(),
                "constitution_2".into(),
                "spare_leap".into(),
                "writ".into(),
                "armory".into(),
            ],
            ..Meta::default()
        };
        let (b, gun) = m.build(&c);
        // 100 + 20 + 25 from upgrades.json
        assert_eq!(b.max_health, 145);
        assert_eq!(b.jumps, 2);
        assert!(b.grapple && !b.parry);
        assert_eq!(gun, 2);
    }

    #[test]
    fn each_death_reads_the_next_count_and_adds_more_years() {
        let c = content();
        let mut m = Meta::default();
        let a = m.record_death(&c, 5, 3, "2031");
        assert_eq!(a.newest, c.story.counts[0]);
        assert_eq!((a.years_added, a.total_years), (1, 1));
        let b = m.record_death(&c, 4, 2, "2031");
        assert_eq!(b.newest, c.story.counts[1]);
        assert_eq!(b.counts.len(), 2);
        // Sifu's count: 1, then 2 more, then 3 more
        let d = m.record_death(&c, 0, 1, "2031");
        assert_eq!(d.total_years, 6);
        // 5 + 4 earned, and the clerk's 8 for the first death
        assert_eq!(m.clemency, 9 + FIRST_DEATH_GRANT);
        for _ in 0..20 {
            m.record_death(&c, 0, 0, "2031");
        }
        assert_eq!(m.counts_read as usize, c.story.counts.len());
        let late = m.record_death(&c, 0, 0, "2031");
        assert!(late.newest.starts_with("ADDENDUM"), "{}", late.newest);
        assert!(!late.newest.contains("{year}"));
    }

    #[test]
    fn a_long_sentence_earns_more_and_a_first_win_reads_a_count() {
        let c = content();
        let mut m = Meta {
            years: 25,
            ..Meta::default()
        };
        // 25 years: two tens, so +10%
        let (banked, newest) = m.record_win(&c, 40, 8, "2031");
        assert_eq!(banked, 44);
        assert_eq!(newest.as_deref(), Some(c.story.counts[0].as_str()));
        let (_, again) = m.record_win(&c, 0, 8, "2031");
        assert!(again.is_none(), "only the first win reads a count");
    }

    #[test]
    fn a_save_from_another_build_loads_without_its_unknown_upgrades() {
        let c = content();
        let m = Meta::load(
            r#"{"clemency": 12, "owned": ["writ", "jetpack"], "deaths": 3}"#,
            &c,
        );
        assert_eq!(m.owned, vec!["writ".to_string()]);
        assert_eq!(m.deaths, 3);
        assert_eq!(Meta::load("not json", &c), Meta::default());
        let back = Meta::load(&m.save(), &c);
        assert_eq!(back, m);
    }

    #[test]
    fn the_stopped_clocks_wait_for_the_defendant_and_the_time_stop_comes_with_beating_it() {
        let c = content();
        let mut m = Meta {
            clemency: 10_000,
            ..Meta::default()
        };
        assert_eq!(
            m.check(&c, "stopped_clock_1"),
            Err(Refusal::Locked("defendant".into()))
        );
        assert_eq!(m.build(&c).0.stop_ticks, 0, "no time stop yet");
        m.beat(TIME_STOPPER);
        assert_eq!(m.build(&c).0.stop_ticks, STOP_BASE);
        for id in ["stopped_clock_1", "stopped_clock_2", "stopped_clock_3"] {
            m.buy(&c, id).unwrap();
        }
        assert_eq!(
            m.build(&c).0.stop_ticks,
            STOP_BASE + 3 * STOP_PLUS,
            "six seconds at most"
        );
    }

    #[test]
    fn the_test_save_holds_everything_and_opens_the_boss_rush() {
        let c = content();
        let m = Meta::everything(&c);
        assert!(c.upgrades.iter().all(|u| m.owns(&u.id)));
        assert_eq!(m.unlocked, c.built_zones() as i32 - 1);
        assert_eq!(m.counts_read, c.story.counts.len() as i32);
        assert!(m.rush_open());
        assert!(!Meta::default().rush_open());
        // and it survives a save and a load
        assert_eq!(Meta::load(&m.save(), &c), m);
    }

    #[test]
    fn the_houses_upgrades_make_the_body_and_good_conduct_earns_more() {
        let c = content();
        let mut m = Meta {
            clemency: 10_000,
            ..Meta::default()
        };
        for id in [
            "constitution_1",
            "recess",
            "appeal",
            "gavel",
            "gavel_2",
            "armory",
            "precedent",
            "bail",
            "lien",
            "writ",
            "writ_2",
        ] {
            m.buy(&c, id).unwrap_or_else(|e| panic!("{id}: {e:?}"));
        }
        let (b, _) = m.build(&c);
        assert_eq!(b.pound, 2);
        assert_eq!(b.recesses(), 2, "Appeal is a second Recess");
        assert_eq!(b.pierce, 1, "Precedent");
        assert_eq!(b.bail, 15);
        assert!(b.lien && b.grapple && b.grapple_plus);
        assert!(b.grapple_range() > GRAPPLE_RANGE_BASE);
        let before = m.clemency;
        m.buy(&c, "conduct").unwrap();
        let spent = before - m.clemency;
        m.record_death(&c, 100, 3, "2056");
        // 15% more, no sentence bonus yet, and the first offence's grant
        assert_eq!(m.clemency - (before - spent), 115 + FIRST_DEATH_GRANT);
    }

    #[test]
    fn an_exhibit_waits_for_its_boss_and_lets_a_run_carry_its_gun_in() {
        let c = content();
        let mut m = Meta {
            clemency: 1000,
            ..Meta::default()
        };
        assert_eq!(m.carried(&c), c.gun("carbine"), "the Carbine before any Exhibit");
        assert!(
            !m.choose_carry(&c, "scattergun"),
            "not before its Exhibit is bought"
        );
        assert_eq!(
            m.check(&c, "exhibit_d"),
            Err(Refusal::Locked("bailiff".to_string()))
        );
        m.buy(&c, "exhibit_b").unwrap();
        m.beat("bailiff");
        m.buy(&c, "exhibit_d").unwrap();
        assert_eq!(
            m.carry_choices(&c),
            [c.gun("carbine"), c.gun("scattergun"), c.gun("filibuster")]
        );
        assert!(m.choose_carry(&c, "filibuster"));
        assert_eq!(m.carried(&c), c.gun("filibuster"));
        assert!(!m.choose_carry(&c, "ripsaw"), "its Exhibit is not bought");
        assert_eq!(m.carried(&c), c.gun("filibuster"));
        // the choice is kept with the record
        assert_eq!(Meta::load(&m.save(), &c).carried(&c), c.gun("filibuster"));
        // a record that names a gun it has no Exhibit for carries the Carbine
        let odd = Meta::load(r#"{"carry": "big_iron"}"#, &c);
        assert_eq!(odd.carried(&c), c.gun("carbine"));
    }

    #[test]
    fn the_holding_starts_a_gun_at_level_three_at_most() {
        let c = content();
        let m = Meta::everything(&c);
        let (_, level) = m.build(&c);
        assert_eq!(level, 3);
        assert!(c
            .upgrades
            .iter()
            .filter(|u| u.effect == "armory")
            .all(|u| u.amount <= 3));
    }
}
