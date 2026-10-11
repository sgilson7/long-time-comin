//! What lasts between runs: Clemency, the upgrades it bought, the deaths,
//! the years added to the sentence, and how much of the indictment has been
//! read. The page keeps it in the browser as JSON.

use crate::balance::{BASE_HEALTH, BASE_JUMPS, DASH_CHARGES, FIRST_DEATH_GRANT, SENTENCE_BONUS_PCT};
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
}

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
    Unknown,
}

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

    /// The body a run starts with, and the level of the Carbine it carries.
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
                "appeal" => b.appeal = true,
                "precedent" => b.pierce += u.amount,
                "bail" => b.bail += u.amount,
                "lien" => b.lien = true,
                "armory" => gun_level = gun_level.max(u.amount),
                _ => {}
            }
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
        self.unlocked = self.unlocked.max(next as i32).min(c.built_zones() as i32 - 1);
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
    fn the_houses_upgrades_make_the_body_and_good_conduct_earns_more() {
        let c = content();
        let mut m = Meta {
            clemency: 10_000,
            ..Meta::default()
        };
        for id in ["constitution_1", "recess", "appeal", "gavel", "gavel_2", "armory", "precedent", "bail", "lien", "writ", "writ_2"] {
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
}
