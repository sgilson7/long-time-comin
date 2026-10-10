//! What lasts between runs: Clemency, the upgrades it bought, the deaths,
//! the years added to the sentence, and how much of the indictment has been
//! read. The page keeps it in the browser as JSON.

use crate::balance::{BASE_HEALTH, BASE_JUMPS, DASH_CHARGES};
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
                "armory" => gun_level = gun_level.max(u.amount),
                _ => {}
            }
        }
        (b, gun_level)
    }

    /// A run ended in death: bank its Clemency, add the years, read the
    /// next count (or an addendum once every count is read).
    pub fn record_death(&mut self, c: &Content, earned: i32, nodes: i32, year: &str) -> Reading {
        self.runs += 1;
        self.deaths += 1;
        self.bank(earned, nodes);
        let years_added = self.deaths;
        self.years += years_added;
        let s = &c.story;
        let newest = if (self.counts_read as usize) < s.counts.len() {
            self.counts_read += 1;
            s.counts[self.counts_read as usize - 1].clone()
        } else {
            let k = (self.deaths as usize) % s.addenda.len().max(1);
            s.addenda
                .get(k)
                .cloned()
                .unwrap_or_default()
                .replace("{year}", year)
        };
        Reading {
            counts: s.counts[..self.counts_read as usize].to_vec(),
            newest,
            years_added,
            total_years: self.years,
        }
    }

    pub fn record_win(&mut self, earned: i32, nodes: i32) {
        self.runs += 1;
        self.wins += 1;
        self.bank(earned, nodes);
    }

    fn bank(&mut self, earned: i32, nodes: i32) {
        self.clemency += earned;
        self.clemency_earned += earned;
        self.best_nodes = self.best_nodes.max(nodes);
    }

    /// A different graph every run, the same one for the same record.
    pub fn run_seed(&self) -> u64 {
        0x5EED_0000 + self.runs as u64 * 7919 + self.deaths as u64 * 104_729
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(m.clemency, 9);
        for _ in 0..20 {
            m.record_death(&c, 0, 0, "2031");
        }
        assert_eq!(m.counts_read as usize, c.story.counts.len());
        let late = m.record_death(&c, 0, 0, "2031");
        assert!(late.newest.starts_with("ADDENDUM"), "{}", late.newest);
        assert!(!late.newest.contains("{year}"));
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
}
