//! A run's map: a zone as a graph of arenas. One main path runs from where
//! you arrive (the crater, the landing on the ice) to the zone's boss;
//! between main fights, side paths of one or two harder fights rejoin the
//! next main node, and the last node of each holds a reward you can see on
//! the map before you choose. Every zone of every run rolls its own graph
//! from a seed.
//!
//! A side path's reward is rolled from the zone's table, no name twice in a
//! zone and nothing the body could not use: a two-fight path holds a choice
//! of two, and some are sealed, better than the table and unseen until
//! taken. A gun is always an upgrade: a level or two above the gun in hand
//! when it is taken.

use crate::balance::{CONTEMPT_MAX, MAX_GUN_LEVEL, MAX_JUMPS, PURSE_TIMES};
use crate::content::Content;
use crate::rng::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Arrival,
    Fight,
    Rest,
    Boss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reward {
    /// a gun, by index, and the levels it stands above the gun in hand
    Gun(usize, i32),
    /// what you kill bursts
    Contempt,
    /// this much Clemency
    Purse(i32),
    Jump,
    Health,
    Speed,
    /// shots pass through one more enemy
    Pierce,
    /// a dash clears the orbs round you
    Pulse,
    /// a parried orb heals you
    Mend,
    /// guns fire faster
    Trigger,
    /// hits do less
    Plating,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// the zone it is in, by index
    pub zone: usize,
    pub kind: NodeKind,
    pub archetype: String,
    pub tier: i32,
    pub detour: bool,
    pub reward: Option<Reward>,
    pub next: Vec<usize>,
    /// where on the map: steps along the zone, and 0 for the main path or 1
    /// for a side path
    pub step: i32,
    pub lane: i32,
    pub seed: u64,
    /// an index into the zone's encounters
    pub encounter: Option<usize>,
    /// an arrival's cache: rewards to choose one of
    pub cache: Vec<Reward>,
    /// a mini-boss fought alone here, by enemy index
    pub elite: Option<usize>,
    /// rewards beside `reward`: a choice of one when `choose`, else a
    /// sealed pair, both yours
    pub extra: Vec<Reward>,
    pub choose: bool,
    /// its rewards are not shown until it is cleared
    pub sealed: bool,
}

/// What the body already has, so no reward offered is wasted on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Have {
    pub level: i32,
    pub pulse: bool,
    pub mend: bool,
    pub contempt: i32,
    pub jumps: i32,
}

impl Have {
    /// A body that has nothing yet, with a gun of `level` in hand.
    pub fn level(level: i32) -> Have {
        Have {
            level,
            ..Have::default()
        }
    }

    pub fn of(b: &crate::player::Build, level: i32) -> Have {
        Have {
            level,
            pulse: b.pulse,
            mend: b.mend,
            contempt: b.contempt,
            jumps: b.jumps,
        }
    }

    /// Would the reward `name` be any use?
    fn wants(&self, name: &str) -> bool {
        match name {
            "weapon" => self.level < MAX_GUN_LEVEL,
            "pulse" => !self.pulse,
            "mend" => !self.mend,
            "contempt" => self.contempt < CONTEMPT_MAX,
            "jump" => self.jumps < MAX_JUMPS,
            _ => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub nodes: Vec<Node>,
    /// how many steps the map spans, for laying it out
    pub steps: i32,
}

/// Steps between main nodes, so a two-node side path fits between them.
pub const STRIDE: i32 = 4;

impl Route {
    pub fn build(c: &Content, seed: u64) -> Route {
        Route::build_for(c, 0, seed, &Have::level(1))
    }

    /// Zone `zone`'s map for a body that has `have`: nothing offered is
    /// wasted on it (SECOND-ORDER P6, Q60).
    pub fn build_for(c: &Content, zone: usize, seed: u64, have: &Have) -> Route {
        let mut rng = Rng::new(seed ^ (zone as u64).wrapping_mul(0x9E37_79B9));
        let z = &c.zones[zone];
        let r = &z.route;
        let mut nodes = Vec::new();
        let node = |kind: NodeKind, archetype: &str, tier: i32, step: i32, rng: &mut Rng| Node {
            zone,
            kind,
            archetype: archetype.to_string(),
            tier,
            detour: false,
            reward: None,
            next: Vec::new(),
            step,
            lane: 0,
            seed: rng.next(),
            encounter: None,
            cache: Vec::new(),
            elite: None,
            extra: Vec::new(),
            choose: false,
            sealed: false,
        };
        if r.course {
            return Route::course(c, zone, have, &mut rng);
        }
        // the main path
        let mut arrival = node(NodeKind::Arrival, "arrival", 0, 0, &mut rng);
        // names offered anywhere in this zone so far: nothing twice
        let mut used: Vec<String> = Vec::new();
        // an arrival's cache: two different rewards, take one
        if r.cache_rewards.len() >= 2 {
            for _ in 0..2 {
                let name = draw(&r.cache_rewards, &mut used, have, true, &mut rng);
                arrival
                    .cache
                    .push(reward_of(c, zone, &name, r.weapon_steps[0], &mut rng));
            }
        }
        nodes.push(arrival);
        let mut last_arch = String::new();
        for i in 0..r.main_fights {
            let mut arch = last_arch.clone();
            while arch == last_arch {
                arch = z.archetypes[rng.range(0, z.archetypes.len() as i32 - 1) as usize].clone();
            }
            last_arch = arch.clone();
            let tier = r.main_tiers[i];
            let mb = r.minibosses.iter().find(|m| m.at == i);
            let elite = mb.and_then(|m| c.enemy(&m.enemy));
            let arch = mb.map_or(arch, |m| m.arena.clone());
            let mut n = node(NodeKind::Fight, &arch, tier, STRIDE * (i as i32 + 1), &mut rng);
            n.encounter = Some(pick_encounter(c, zone, tier, &mut rng));
            n.elite = elite;
            nodes.push(n);
        }
        let rest_step = STRIDE * (r.main_fights as i32 + 1);
        let mut rest = node(NodeKind::Rest, "rest", 0, rest_step, &mut rng);
        // the Antechamber's alternative to its fountain
        if !r.rest_rewards.is_empty() {
            let pick = draw(&r.rest_rewards, &mut used, have, true, &mut rng);
            rest.reward = Some(reward_of(c, zone, &pick, r.weapon_steps[1], &mut rng));
        }
        nodes.push(rest);
        nodes.push(node(NodeKind::Boss, "boss", 0, rest_step + STRIDE, &mut rng));
        let mains = nodes.len();
        for i in 0..mains - 1 {
            nodes[i].next.push(i + 1);
        }

        // side paths: from main fights 1..=main_fights, each to the next main node
        let mut gaps: Vec<usize> = (1..=r.main_fights).collect();
        let want = rng.range(r.detours_min as i32, r.detours_max as i32) as usize;
        let mut chosen = Vec::new();
        while chosen.len() < want.min(gaps.len()) {
            let k = rng.range(0, gaps.len() as i32 - 1) as usize;
            chosen.push(gaps.remove(k));
        }
        chosen.sort();
        let max_tier = z.max_tier();
        for &from in chosen.iter() {
            let len = rng.range(1, r.detour_max as i32);
            let tier = (nodes[from].tier + 1).min(max_tier);
            let mut prev = from;
            for k in 0..len {
                let arches = &z.archetypes;
                let mut arch = arches[rng.range(0, arches.len() as i32 - 1) as usize].clone();
                if arch == nodes[prev].archetype {
                    arch = arches[(arches.iter().position(|a| *a == arch).unwrap_or(0) + 1) % arches.len()]
                        .clone();
                }
                let step = nodes[from].step + if len == 1 { STRIDE / 2 } else { 1 + 2 * k };
                let mut n = node(NodeKind::Fight, &arch, tier, step, &mut rng);
                n.detour = true;
                n.lane = 1;
                n.encounter = Some(pick_encounter(c, zone, tier, &mut rng));
                if k == len - 1 {
                    let deep = from * 2 > r.main_fights;
                    let step = r.weapon_steps[deep as usize];
                    let first = draw(&r.rewards, &mut used, have, true, &mut rng);
                    if rng.chance(r.sealed_pct) {
                        // sealed: a gun a level higher than the table's,
                        // or two rewards that are both yours
                        n.sealed = true;
                        if first == "weapon" {
                            n.reward = Some(reward_of(c, zone, &first, step + 1, &mut rng));
                        } else {
                            n.reward = Some(reward_of(c, zone, &first, step, &mut rng));
                            let second = draw(&r.rewards, &mut used, have, false, &mut rng);
                            n.extra.push(reward_of(c, zone, &second, step, &mut rng));
                        }
                    } else {
                        n.reward = Some(reward_of(c, zone, &first, step, &mut rng));
                        if len > 1 {
                            // the longer path: a choice of two
                            let second = draw(&r.rewards, &mut used, have, true, &mut rng);
                            n.extra.push(reward_of(c, zone, &second, step, &mut rng));
                            n.choose = true;
                        }
                    }
                }
                let idx = nodes.len();
                nodes.push(n);
                nodes[prev].next.push(idx);
                prev = idx;
            }
            nodes[prev].next.push(from + 1);
        }
        Route {
            nodes,
            steps: rest_step + STRIDE,
        }
    }

    /// A zone of courses: its sections in order (the first is the
    /// arrival), each holding a strand's reward rolled from the table, then
    /// the rest and the boss. No side paths.
    fn course(c: &Content, zone: usize, have: &Have, rng: &mut Rng) -> Route {
        let z = &c.zones[zone];
        let r = &z.route;
        let mut used: Vec<String> = Vec::new();
        let mut nodes: Vec<Node> = Vec::new();
        for (i, arch) in z.archetypes.iter().enumerate() {
            // a rest listed among the sections stands where it is listed
            let kind = if arch == "rest" {
                NodeKind::Rest
            } else if i == 0 {
                NodeKind::Arrival
            } else {
                NodeKind::Fight
            };
            let name = draw(&r.rewards, &mut used, have, true, rng);
            let reward = Some(reward_of(c, zone, &name, r.weapon_steps[0], rng));
            // a fought zone's sections (the Far Shore's) bring waves too: an
            // encounter of the section's tier, when the zone lists any
            let tier = r.main_tiers.get(i).copied().unwrap_or(0);
            let encounter =
                (i > 0 && tier > 0 && !z.encounters.is_empty()).then(|| pick_encounter(c, zone, tier, rng));
            nodes.push(Node {
                zone,
                kind,
                archetype: arch.clone(),
                tier,
                detour: false,
                reward,
                next: Vec::new(),
                step: STRIDE * i as i32,
                lane: 0,
                seed: rng.next(),
                encounter,
                cache: Vec::new(),
                elite: None,
                extra: Vec::new(),
                choose: false,
                sealed: false,
            });
        }
        let rest_step = STRIDE * z.archetypes.len() as i32;
        let listed = z.archetypes.iter().any(|a| a == "rest");
        let mut rest = Node {
            kind: NodeKind::Rest,
            archetype: "rest".into(),
            reward: None,
            step: rest_step,
            seed: rng.next(),
            ..nodes[0].clone()
        };
        if !r.rest_rewards.is_empty() {
            let pick = draw(&r.rest_rewards, &mut used, have, true, rng);
            rest.reward = Some(reward_of(c, zone, &pick, r.weapon_steps[1], rng));
        }
        if let Some(n) = nodes.iter_mut().find(|n| n.kind == NodeKind::Rest) {
            // the listed rest holds the rest's reward, not a strand's
            n.reward = rest.reward;
        }
        if !listed {
            nodes.push(rest);
        }
        nodes.push(Node {
            kind: NodeKind::Boss,
            archetype: "boss".into(),
            reward: None,
            step: rest_step + STRIDE,
            seed: rng.next(),
            ..nodes[0].clone()
        });
        for i in 0..nodes.len() - 1 {
            nodes[i].next.push(i + 1);
        }
        Route {
            nodes,
            steps: rest_step + STRIDE,
        }
    }

    /// A boss rush's map for zone `zone`: its boss alone.
    pub fn rush(zone: usize, seed: u64) -> Route {
        Route {
            nodes: vec![Node {
                zone,
                kind: NodeKind::Boss,
                archetype: "boss".to_string(),
                tier: 0,
                detour: false,
                reward: None,
                next: Vec::new(),
                step: 0,
                lane: 0,
                seed,
                encounter: None,
                cache: Vec::new(),
                elite: None,
                extra: Vec::new(),
                choose: false,
                sealed: false,
            }],
            steps: 1,
        }
    }

    /// The node every run starts on.
    pub fn start(&self) -> usize {
        0
    }

    pub fn boss(&self) -> usize {
        self.nodes
            .iter()
            .position(|n| n.kind == NodeKind::Boss)
            .unwrap_or(0)
    }
}

/// A name from `table`, weighted by how often it is listed: not one in
/// `used` (which it joins), nothing `have` could not use, and a gun only
/// when `guns`. Health when nothing is left.
fn draw(table: &[String], used: &mut Vec<String>, have: &Have, guns: bool, rng: &mut Rng) -> String {
    let fits: Vec<&String> = table
        .iter()
        .filter(|n| !used.contains(n) && have.wants(n) && (guns || n.as_str() != "weapon"))
        .collect();
    let name = if fits.is_empty() {
        "health".to_string()
    } else {
        fits[rng.range(0, fits.len() as i32 - 1) as usize].clone()
    };
    used.push(name.clone());
    name
}

/// A reward by its name in zone.json; a gun is one the zone offers, `step`
/// levels above the gun in hand when it is taken.
fn reward_of(c: &Content, zone: usize, name: &str, step: i32, rng: &mut Rng) -> Reward {
    match name {
        "weapon" => {
            let fits: Vec<usize> = (0..c.guns.len())
                .filter(|&g| c.guns[g].from_zone <= zone)
                .collect();
            let gun = fits[rng.range(0, fits.len() as i32 - 1) as usize];
            Reward::Gun(gun, step.max(1))
        }
        "contempt" => Reward::Contempt,
        "purse" => Reward::Purse(PURSE_TIMES * c.zones[zone].route.clemency_detour),
        "jump" => Reward::Jump,
        "speed" => Reward::Speed,
        "pierce" => Reward::Pierce,
        "pulse" => Reward::Pulse,
        "mend" => Reward::Mend,
        "trigger" => Reward::Trigger,
        "plating" => Reward::Plating,
        _ => Reward::Health,
    }
}

fn pick_encounter(c: &Content, zone: usize, tier: i32, rng: &mut Rng) -> usize {
    let list = &c.zones[zone].encounters;
    let tier = tier.min(c.zones[zone].max_tier());
    let fits: Vec<usize> = (0..list.len()).filter(|&i| list[i].tier == tier).collect();
    if fits.is_empty() {
        return 0;
    }
    fits[rng.range(0, fits.len() as i32 - 1) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;

    #[test]
    fn the_main_path_runs_from_the_crater_to_the_court_through_every_main_fight() {
        let c = content();
        for seed in 0..40 {
            let r = Route::build(&c, seed);
            // follow the first exit of every node: the main path
            let mut at = r.start();
            let mut kinds = vec![r.nodes[at].kind];
            while let Some(&n) = r.nodes[at].next.first() {
                at = n;
                kinds.push(r.nodes[at].kind);
            }
            assert_eq!(kinds.len(), c.zones[0].route.main_fights + 3, "seed {seed}");
            assert_eq!(kinds[0], NodeKind::Arrival);
            assert_eq!(kinds[kinds.len() - 2], NodeKind::Rest);
            assert_eq!(*kinds.last().unwrap(), NodeKind::Boss);
            assert!(r.nodes[r.boss()].next.is_empty());
        }
    }

    #[test]
    fn every_side_path_ends_in_a_reward_and_rejoins_the_next_main_node() {
        let c = content();
        for seed in 0..40 {
            let r = Route::build(&c, seed);
            let starts: Vec<usize> = (0..r.nodes.len())
                .filter(|&i| !r.nodes[i].detour && r.nodes[i].next.len() == 2)
                .collect();
            assert!(
                (c.zones[0].route.detours_min..=c.zones[0].route.detours_max).contains(&starts.len()),
                "seed {seed}: {} side paths",
                starts.len()
            );
            for s in starts {
                let mut at = r.nodes[s].next[1];
                let mut len = 1;
                while r.nodes[at].detour {
                    assert!(
                        r.nodes[at].tier > r.nodes[s].tier.min(3),
                        "harder than the main fight"
                    );
                    let n = &r.nodes[at];
                    if !r.nodes[n.next[0]].detour {
                        assert!(
                            n.reward.is_some(),
                            "seed {seed}: the last side node holds a reward"
                        );
                    }
                    at = n.next[0];
                    len += 1;
                }
                assert_eq!(at, s + 1, "seed {seed}: rejoins the next main node");
                assert!(len - 1 <= c.zones[0].route.detour_max);
            }
        }
    }

    #[test]
    fn different_seeds_make_different_maps() {
        let c = content();
        let a = Route::build(&c, 1);
        let b = Route::build(&c, 2);
        assert_eq!(a, Route::build(&c, 1));
        assert_ne!(a, b);
    }

    #[test]
    fn every_fight_has_an_encounter_of_its_tier() {
        let c = content();
        let r = Route::build(&c, 5);
        for n in r.nodes.iter().filter(|n| n.kind == NodeKind::Fight) {
            let e = n.encounter.expect("a fight has an encounter");
            assert_eq!(c.zones[0].encounters[e].tier, n.tier);
        }
    }

    /// Every reward a route offers: arrival caches, side paths (each
    /// path's whole prize) and the Antechamber's.
    fn prizes(r: &Route) -> Vec<Vec<Reward>> {
        r.nodes
            .iter()
            .filter_map(|n| {
                n.reward.map(|x| {
                    let mut all = vec![x];
                    all.extend(n.extra.iter().copied());
                    all
                })
            })
            .collect()
    }

    #[test]
    fn side_path_guns_are_upgrades_and_nothing_offered_is_wasted() {
        let c = content();
        let have = Have {
            level: 2,
            pulse: true,
            mend: false,
            contempt: CONTEMPT_MAX,
            jumps: MAX_JUMPS,
        };
        let mut guns = 0;
        for zone in 0..3 {
            for seed in 0..80 {
                let r = Route::build_for(&c, zone, seed, &have);
                for x in prizes(&r).into_iter().flatten() {
                    if let Reward::Gun(_, step) = x {
                        assert!(step >= 1, "zone {zone} seed {seed}: a gun no better");
                        guns += 1;
                    }
                    assert!(
                        !matches!(x, Reward::Pulse | Reward::Contempt | Reward::Jump),
                        "zone {zone} seed {seed}: {x:?} offered to a body that has it"
                    );
                }
                assert!(
                    r.nodes
                        .iter()
                        .any(|n| n.kind == NodeKind::Rest && n.reward.is_some()),
                    "the Antechamber offers a choice"
                );
            }
        }
        assert!(guns > 0);
        // a gun at the last level is offered no gun
        let top = Have::level(MAX_GUN_LEVEL);
        for seed in 0..80 {
            let r = Route::build_for(&c, 0, seed, &top);
            assert!(
                !prizes(&r)
                    .into_iter()
                    .flatten()
                    .any(|x| matches!(x, Reward::Gun(..))),
                "seed {seed}: a gun offered past level {MAX_GUN_LEVEL}"
            );
        }
    }

    #[test]
    fn side_paths_vary_hold_no_name_twice_and_a_long_one_holds_a_choice() {
        let c = content();
        let (mut sealed, mut choices, mut kinds) = (0, 0, std::collections::BTreeSet::new());
        for seed in 0..200 {
            let r = Route::build(&c, seed);
            let mut names = Vec::new();
            for (i, n) in r.nodes.iter().enumerate() {
                if !n.detour || n.reward.is_none() {
                    continue;
                }
                // a path two fights long: the node before this one is a side node too
                let long = r.nodes.iter().any(|m| m.detour && m.next.contains(&i));
                if n.sealed {
                    sealed += 1;
                    assert!(!n.choose, "a sealed prize is no choice");
                } else {
                    assert_eq!(n.choose, long, "seed {seed}: a choice exactly on a long path");
                    assert_eq!(n.extra.len(), long as usize);
                    choices += long as usize;
                }
                for x in std::iter::once(n.reward.unwrap()).chain(n.extra.iter().copied()) {
                    let name = std::mem::discriminant(&x);
                    assert!(!names.contains(&name), "seed {seed}: {x:?} twice in a zone");
                    names.push(name);
                    kinds.insert(format!("{:?}", x).split('(').next().unwrap().to_string());
                }
            }
        }
        // nor anywhere else in the zone, its cache and its Antechamber:
        // health alone, what is offered once nothing else is left, may come
        // again
        for zone in 0..3 {
            for seed in 0..60 {
                let r = Route::build_for(&c, zone, seed, &Have::level(1));
                let all: Vec<_> = prizes(&r)
                    .into_iter()
                    .flatten()
                    .chain(r.nodes[0].cache.iter().copied())
                    .collect();
                for (k, x) in all.iter().enumerate().filter(|(_, x)| **x != Reward::Health) {
                    assert!(
                        all[k + 1..]
                            .iter()
                            .all(|y| std::mem::discriminant(x) != std::mem::discriminant(y)),
                        "zone {zone} seed {seed}: {x:?} offered twice"
                    );
                }
            }
        }
        assert!(
            sealed > 40 && sealed < 400,
            "about one side path in four is sealed: {sealed}"
        );
        assert!(choices > 50, "{choices} choices");
        for k in [
            "Gun", "Contempt", "Purse", "Pulse", "Mend", "Pierce", "Jump", "Health", "Speed",
        ] {
            assert!(kinds.contains(k), "zone one's side paths never offer {k}");
        }
    }

    #[test]
    fn a_route_on_the_ice_holds_only_the_ice_and_its_arrival_a_cache_of_two() {
        let c = content();
        let z = &c.zones[1];
        for seed in 0..30 {
            let r = Route::build_for(&c, 1, seed, &Have::level(1));
            assert!(r.nodes.iter().all(|n| n.zone == 1));
            for n in r.nodes.iter().filter(|n| n.kind == NodeKind::Fight) {
                assert!(z.archetypes.contains(&n.archetype), "{}", n.archetype);
                assert_eq!(z.encounters[n.encounter.unwrap()].tier, n.tier.min(z.max_tier()));
            }
            let cache = &r.nodes[r.start()].cache;
            assert_eq!(cache.len(), 2, "seed {seed}");
            assert_ne!(cache[0], cache[1]);
        }
        // zone one's crater has none
        assert!(Route::build(&c, 3).nodes[0].cache.is_empty());
    }

    #[test]
    fn the_house_route_has_its_mini_boss_and_only_the_house_offers_the_ripsaw() {
        let c = content();
        let saw = c.gun("ripsaw");
        for seed in 0..30 {
            let r = Route::build_for(&c, 2, seed, &Have::level(1));
            let elites: Vec<&Node> = r.nodes.iter().filter(|n| n.elite.is_some()).collect();
            // the Blaster in the Great Hall, the Section Chief in the Ballroom
            assert_eq!(elites.len(), 2, "two mini-bosses");
            let rooms: Vec<&str> = elites.iter().map(|n| n.archetype.as_str()).collect();
            assert_eq!(rooms, ["hall", "ballroom"]);
            assert!(elites.iter().all(|n| !n.detour));
            // its deep side paths' guns stand two levels up
            for n in r.nodes.iter().filter(|n| n.detour && !n.sealed) {
                if let Some(Reward::Gun(_, step)) = n.reward {
                    assert!((1..=2).contains(&step), "a house gun {step} up");
                }
            }
            for z in [0, 1] {
                let r = Route::build_for(&c, z, seed, &Have::level(1));
                for n in &r.nodes {
                    assert!(
                        !matches!(n.reward, Some(Reward::Gun(g, _)) if g == saw),
                        "zone {z} offers the Ripsaw"
                    );
                }
                // zone two's Pioneer; none in zone one
                let elites = r.nodes.iter().filter(|n| n.elite.is_some()).count();
                assert_eq!(elites, z, "zone {z}");
            }
        }
        // and some house route does offer it
        let offered = (0..60).any(|seed| {
            Route::build_for(&c, 2, seed, &Have::level(1))
                .nodes
                .iter()
                .any(|n| matches!(n.reward, Some(Reward::Gun(g, _)) if g == saw))
        });
        assert!(offered);
    }
}
