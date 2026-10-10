#!/usr/bin/env python3
"""Rule 6 (CLAUDE.md): break each behaviour a test covers, run that test,
expect it to fail, and put the code back. A test that still passes with
its behaviour broken does not cover it.

    python3 scripts/break_tests.py            # every break below
    python3 scripts/break_tests.py vent wall  # the breaks whose names hold these words

Each break is (name, file, text to replace, replacement, test filter). The
file is restored after each run, whatever happens. Exits non-zero if any
break leaves its test passing, or names text no longer in its file.
"""
import pathlib, re, subprocess, sys

ROOT = pathlib.Path(__file__).resolve().parents[1]

BREAKS = [
 ("held formation", "crates/core/src/orbs.rs", "let wait = (v.hold + v.stagger * k as i32).max(1);", "let wait = 1;", "a_halo_hangs_still"),
 ("cage closes inward", "crates/core/src/orbs.rs", "Launch::In => from_hub.neg(),", "Launch::In => from_hub,", "a_cage_round_you"),
 ("stagger", "crates/core/src/orbs.rs", "let wait = (v.hold + v.stagger * k as i32).max(1);", "let wait = (v.hold + v.stagger).max(1);", "a_staggered_formation"),
 ("second hold", "crates/core/src/orbs.rs", "if self.flown > r.stop {", "if self.flown > r.stop * 1000 {", "a_refracted_shot"),
 ("blue colours", "crates/core/src/content.rs", 'pub const BLUE: &[&str] = &["cyan", "blue"];', 'pub const BLUE: &[&str] = &["blue"];', "blue_orbs_are"),
 ("the wall breaks blue", "crates/core/src/world.rs", "if o.blue && wall.is_some_and(|w| w.breaks(&o)) {", "if o.blue && wall.is_some_and(|w| w.breaks(&o)) && false {", "the_injunction_breaks"),
 ("the wall's cooldown", "crates/core/src/player.rs", "            self.wall_cooldown = WALL_COOLDOWN;", "            self.wall_cooldown = 0;", "the_wall_stands_half"),
 ("vents", "crates/core/src/world.rs", "            self.player.launch_up(VENT_SPEED);", "            self.player.launch_up(VENT_SPEED / 2);", "an_updraft_vent"),
 ("death burst", "crates/core/src/world.rs", "        let Some(p) = d.burst else {", "        let Some(p) = d.burst.filter(|_| false) else {", "a_shardling_leaves"),
 ("buried is unhurtable", "crates/core/src/enemy.rs", "self.alive() && !self.buried && !matches!", "self.alive() && !matches!", "an_auger_cannot"),
 ("blink", "crates/core/src/enemy.rs", "            if level.ray(at, to.norm(), to.len()).is_none() {\n                return Some(at);", "            if false {\n                return Some(at);", "a_clerk_reappears"),
 ("arrival cache", "crates/core/src/world.rs", "if node.kind == NodeKind::Arrival && !node.cache.is_empty() {", "if node.kind == NodeKind::Arrival && node.cache.is_empty() {", "the_landing_offers"),
 ("cache of two", "crates/core/src/route.rs", "            for k in [a, b] {", "            for k in [a, a] {", "a_route_on_the_ice"),
 ("see-through flag", "crates/core/src/render.rs", "let see_through = (buffer == STATIC_BUFFER) as u32;", "let see_through = 1;", "scenery_may_be_cut"),
 ("zone scene", "crates/core/src/game.rs", "        &self.c.zones[self.run.as_ref().map_or(0, |r| r.zone)]", "        &self.c.zones[0]", "the_ice_is_lit"),
 ("crossing", "crates/core/src/game.rs", "                if next < self.c.built_zones() {", "                if next < 1 {", "felling_the_bailiff_crosses"),
 ("ice reach", "crates/core/src/arena/ice.rs", "            self.solids.push(Solid::new(v3(-o, -m(1), -o), v3(o, 0, o)));", "            self.solids.push(Solid::new(v3(-o, -m(1), -m(10)), v3(o, 0, o)));", "every_ice_gate"),
 ("vent headroom", "crates/core/src/arena/ice.rs", "        self.vent(0, 0, -m(7));", "        self.vent(0, 0, -m(10));", "every_ice_gate"),
 ("unique patterns", "data/enemies.json", '"pattern": "halo"', '"pattern": "ring_16"', "zone_two_has_as_many"),
 ("shut gate is solid", "crates/core/src/arena.rs", "            .push(Solid::stand(g.x, 0, g.z + dm(5), m(5), m(6), dm(4)));", "            .push(Solid::stand(g.x, 0, g.z + dm(5), 0, 0, 0));", "at_an_arenas_start"),
]


def main():
    words = sys.argv[1:]
    bad = 0
    for name, path, old, new, test in BREAKS:
        if words and not any(w in name for w in words):
            continue
        p = ROOT / path
        src = p.read_text()
        if old not in src:
            print(f"{name}: the text to break is gone from {path}")
            bad += 1
            continue
        p.write_text(src.replace(old, new, 1))
        try:
            r = subprocess.run(["cargo", "test", "-q", "-p", "ltc_core", "--lib", test], cwd=ROOT, capture_output=True, text=True)
            out = r.stdout + r.stderr
            m = re.search(r"panicked at [^\n]*\n([^\n]*)", out)
            msg = m.group(1).strip() if m else ("does not compile" if "error[" in out else "")
            ok = r.returncode != 0
            bad += not ok
            print(f"{name}: {'fails, as it should' if ok else 'STILL PASSES'} ({test}) {msg[:140]}")
        finally:
            p.write_text(src)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
