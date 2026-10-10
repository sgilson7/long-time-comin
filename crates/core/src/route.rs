//! A run's map: zone one as a graph of arenas. One main path runs from the
//! crater where you land to the Court of Hours; between main fights, side
//! paths of one or two harder fights rejoin the next main node, and the last
//! node of each holds a reward you can see on the map before you choose.
//! Every run rolls its own graph from a seed.

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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
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
    pub encounter: Option<usize>,
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
        let mut rng = Rng::new(seed);
        let r = &c.route;
        let mut nodes = Vec::new();
        let node = |kind: NodeKind, archetype: &str, tier: i32, step: i32, rng: &mut Rng| Node {
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
        };
        // the main path
        nodes.push(node(NodeKind::Arrival, "arrival", 0, 0, &mut rng));
        let mut last_arch = String::new();
        for i in 0..r.main_fights {
            let mut arch = last_arch.clone();
            while arch == last_arch {
                arch = c.archetypes[rng.range(0, c.archetypes.len() as i32 - 1) as usize].clone();
            }
            last_arch = arch.clone();
            let tier = r.main_tiers[i];
            let mut n = node(NodeKind::Fight, &arch, tier, STRIDE * (i as i32 + 1), &mut rng);
            n.encounter = Some(pick_encounter(c, tier, &mut rng));
            nodes.push(n);
        }
        let rest_step = STRIDE * (r.main_fights as i32 + 1);
        nodes.push(node(NodeKind::Rest, "rest", 0, rest_step, &mut rng));
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
        let max_tier = c.encounters.iter().map(|e| e.tier).max().unwrap_or(1);
        for (d, &from) in chosen.iter().enumerate() {
            let len = rng.range(1, r.detour_max as i32);
            let tier = (nodes[from].tier + 1).min(max_tier);
            let mut prev = from;
            for k in 0..len {
                let mut arch = c.archetypes[rng.range(0, c.archetypes.len() as i32 - 1) as usize].clone();
                if arch == nodes[prev].archetype {
                    arch = c.archetypes[(c.archetypes.iter().position(|a| *a == arch).unwrap_or(0) + 1)
                        % c.archetypes.len()]
                    .clone();
                }
                let step = nodes[from].step + if len == 1 { STRIDE / 2 } else { 1 + 2 * k };
                let mut n = node(NodeKind::Fight, &arch, tier, step, &mut rng);
                n.detour = true;
                n.lane = 1;
                n.encounter = Some(pick_encounter(c, tier, &mut rng));
                if k == len - 1 {
                    let deep = from * 2 > r.main_fights;
                    n.reward = Some(match bag.get(d).map(|s| s.as_str()).unwrap_or("health") {
                        "weapon" => {
                            let gun = rng.range(0, c.guns.len() as i32 - 1) as usize;
                            Reward::Gun(gun, if deep { 3 } else { 2 })
                        }
                        "jump" => Reward::Jump,
                        "speed" => Reward::Speed,
                        _ => Reward::Health,
                    });
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

fn pick_encounter(c: &Content, tier: i32, rng: &mut Rng) -> usize {
    let fits: Vec<usize> = (0..c.encounters.len())
        .filter(|&i| c.encounters[i].tier == tier)
        .collect();
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
            assert_eq!(kinds.len(), c.route.main_fights + 3, "seed {seed}");
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
                (c.route.detours_min..=c.route.detours_max).contains(&starts.len()),
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
                assert!(len - 1 <= c.route.detour_max);
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
            assert_eq!(c.encounters[e].tier, n.tier);
        }
    }
}
