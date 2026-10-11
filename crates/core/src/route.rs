//! A run's map: a zone as a graph of arenas. One main path runs from where
//! you arrive (the crater, the landing on the ice) to the zone's boss;
//! between main fights, side paths of one or two harder fights rejoin the
//! next main node, and the last node of each holds a reward you can see on
//! the map before you choose. Every zone of every run rolls its own graph
//! from a seed.

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
    /// a gun, by index, and its level
    Gun(usize, i32),
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
        Route::build_for(c, 0, seed, 1)
    }

    /// Zone `zone`'s map for a player whose Carbine starts at
    /// `carbine_level`: no side path offers a Carbine that is no better
    /// (SECOND-ORDER P6).
    pub fn build_for(c: &Content, zone: usize, seed: u64, carbine_level: i32) -> Route {
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
        };
        // the main path
        let mut arrival = node(NodeKind::Arrival, "arrival", 0, 0, &mut rng);
        // an arrival's cache: two different rewards, take one
        if r.cache_rewards.len() >= 2 {
            let a = rng.range(0, r.cache_rewards.len() as i32 - 1) as usize;
            let mut b = rng.range(0, r.cache_rewards.len() as i32 - 2) as usize;
            if b >= a {
                b += 1;
            }
            for k in [a, b] {
                arrival.cache.push(reward_of(
                    c,
                    zone,
                    &r.cache_rewards[k],
                    r.weapon_levels[0],
                    carbine_level,
                    &mut rng,
                ));
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
            let elite = (!r.miniboss.is_empty() && i == r.miniboss_at)
                .then(|| c.enemy(&r.miniboss))
                .flatten();
            let arch = if elite.is_some() {
                r.miniboss_arena.clone()
            } else {
                arch
            };
            let mut n = node(NodeKind::Fight, &arch, tier, STRIDE * (i as i32 + 1), &mut rng);
            n.encounter = Some(pick_encounter(c, zone, tier, &mut rng));
            n.elite = elite;
            nodes.push(n);
        }
        let rest_step = STRIDE * (r.main_fights as i32 + 1);
        let mut rest = node(NodeKind::Rest, "rest", 0, rest_step, &mut rng);
        // the Antechamber's alternative to its fountain
        if !r.rest_rewards.is_empty() {
            let pick = &r.rest_rewards[rng.range(0, r.rest_rewards.len() as i32 - 1) as usize];
            rest.reward = Some(reward_of(
                c,
                zone,
                pick,
                r.weapon_levels[1],
                carbine_level,
                &mut rng,
            ));
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
        let mut bag = r.rewards.clone();
        // shuffle the reward bag
        for i in (1..bag.len()).rev() {
            let j = rng.range(0, i as i32) as usize;
            bag.swap(i, j);
        }
        let max_tier = z.max_tier();
        for (d, &from) in chosen.iter().enumerate() {
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
                    let pick = bag.get(d).cloned().unwrap_or_else(|| "health".to_string());
                    n.reward = Some(reward_of(
                        c,
                        zone,
                        &pick,
                        r.weapon_levels[deep as usize],
                        carbine_level,
                        &mut rng,
                    ));
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

/// A reward by its name in zone.json; a gun is of `level`, one the zone
/// offers, and never a Carbine no better than the one the run started with.
fn reward_of(c: &Content, zone: usize, name: &str, level: i32, carbine_level: i32, rng: &mut Rng) -> Reward {
    match name {
        "weapon" => {
            let carbine = c.gun("carbine");
            let fits: Vec<usize> = (0..c.guns.len())
                .filter(|&g| g != carbine || level > carbine_level)
                .filter(|&g| c.guns[g].from_zone <= zone)
                .collect();
            let gun = fits[rng.range(0, fits.len() as i32 - 1) as usize];
            Reward::Gun(gun, level)
        }
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

    #[test]
    fn no_side_path_offers_a_carbine_no_better_than_the_one_you_start_with() {
        let c = content();
        let carbine = c.gun("carbine");
        let mut offered_other = false;
        for seed in 0..60 {
            let r = Route::build_for(&c, 0, seed, 2);
            for n in &r.nodes {
                if let Some(Reward::Gun(g, l)) = n.reward {
                    assert!(
                        g != carbine || l > 2,
                        "seed {seed}: Carbine {l} offered to a Carbine II"
                    );
                    offered_other |= g != carbine;
                }
            }
            assert!(
                r.nodes
                    .iter()
                    .any(|n| n.kind == NodeKind::Rest && n.reward.is_some()),
                "the Antechamber offers a choice"
            );
        }
        assert!(offered_other);
    }

    #[test]
    fn a_route_on_the_ice_holds_only_the_ice_and_its_arrival_a_cache_of_two() {
        let c = content();
        let z = &c.zones[1];
        for seed in 0..30 {
            let r = Route::build_for(&c, 1, seed, 1);
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
            let r = Route::build_for(&c, 2, seed, 1);
            let elites: Vec<&Node> = r.nodes.iter().filter(|n| n.elite.is_some()).collect();
            assert_eq!(elites.len(), 1, "one mini-boss");
            assert_eq!(elites[0].archetype, "ballroom");
            assert!(!elites[0].detour);
            // its guns are level 3 or 4
            for n in &r.nodes {
                if let Some(Reward::Gun(_, l)) = n.reward {
                    assert!(l >= 3, "a house gun of level {l}");
                }
            }
            for z in [0, 1] {
                let r = Route::build_for(&c, z, seed, 1);
                for n in &r.nodes {
                    assert!(
                        !matches!(n.reward, Some(Reward::Gun(g, _)) if g == saw),
                        "zone {z} offers the Ripsaw"
                    );
                    assert!(n.elite.is_none());
                }
            }
        }
        // and some house route does offer it
        let offered = (0..60).any(|seed| {
            Route::build_for(&c, 2, seed, 1)
                .nodes
                .iter()
                .any(|n| matches!(n.reward, Some(Reward::Gun(g, _)) if g == saw))
        });
        assert!(offered);
    }
}
