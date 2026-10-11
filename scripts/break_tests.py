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
 ("the wall's cooldown", "crates/core/src/player.rs", "            self.wall_cooldown = if plus { WALL_PLUS_COOLDOWN } else { WALL_COOLDOWN };", "            self.wall_cooldown = 0;", "the_wall_stands_half"),
 ("vents", "crates/core/src/world.rs", "            self.player.launch_up(VENT_SPEED);", "            self.player.launch_up(VENT_SPEED / 2);", "an_updraft_vent"),
 ("death burst", "crates/core/src/world.rs", "        let Some(p) = d.burst else {", "        let Some(p) = d.burst.filter(|_| false) else {", "a_shardling_leaves"),
 ("buried is unhurtable", "crates/core/src/enemy.rs", "self.alive() && !self.buried && !matches!", "self.alive() && !matches!", "an_auger_cannot"),
 ("blink", "crates/core/src/enemy.rs", "            if level.ray(at, to.norm(), to.len()).is_none() {\n                return Some(at);", "            if false {\n                return Some(at);", "a_clerk_reappears"),
 ("arrival cache", "crates/core/src/world.rs", "if node.kind == NodeKind::Arrival && !node.cache.is_empty() {", "if node.kind == NodeKind::Arrival && node.cache.is_empty() {", "the_landing_offers"),
 ("cache of two", "crates/core/src/route.rs", "            for k in [a, b] {", "            for k in [a, a] {", "a_route_on_the_ice"),
 ("see-through flag", "crates/core/src/render.rs", "let see_through = (buffer == STATIC_BUFFER) as u32;", "let see_through = 1;", "scenery_may_be_cut"),
 ("zone scene", "crates/core/src/game.rs", "        &self.c.zones[self.run.as_ref().map_or(0, |r| r.zone)]", "        &self.c.zones[0]", "the_ice_is_lit"),
 ("crossing", "crates/core/src/game.rs", "                if next < self.c.built_zones() {\n                    // the next zone: the interlude first", "                if next < 1 {\n                    // the next zone: the interlude first", "felling_each_boss_crosses"),
 ("ice reach", "crates/core/src/arena/ice.rs", "            self.solids.push(Solid::new(v3(-o, -m(1), -o), v3(o, 0, o)));", "            self.solids.push(Solid::new(v3(-o, -m(1), -m(10)), v3(o, 0, o)));", "every_ice_gate"),
 ("vent headroom", "crates/core/src/arena/ice.rs", "        self.vent(0, 0, -m(7));", "        self.vent(0, 0, -m(10));", "every_ice_gate"),
 ("unique patterns", "data/enemies.json", '"pattern": "halo"', '"pattern": "ring_16"', "zone_two_has_as_many"),
 ("shut gate is solid", "crates/core/src/arena.rs", "            .push(Solid::stand(g.x, 0, g.z + dm(5), m(5), m(6), dm(4)));", "            .push(Solid::stand(g.x, 0, g.z + dm(5), 0, 0, 0));", "at_an_arenas_start"),
 # round five: the house
 ("nav: no step up a sheer side", "crates/core/src/nav.rs", "if h == i32::MIN || h > y + AGENT_STEP || h < y - AGENT_DROP {", "if h == i32::MIN || h > y + AGENT_STEP * 100 || h < y - AGENT_DROP {", "a_gallery_with_no_stair"),
 ("nav: a wall stands in the way", "crates/core/src/nav.rs", "                    if free {\n                        heights", "                    if free || true {\n                        heights", "a_path_goes_round_a_wall"),
 ("nav: cover", "crates/core/src/nav.rs", "            let tall = level.blocked(", "            let tall = false && level.blocked(", "places_beside_a_wall_are_cover"),
 ("squad: cover is hidden", "crates/core/src/squad.rs", "        if clear(level, m.chest, head(cpos)) {\n            continue;\n        }", "", "an_agent_takes_cover"),
 ("squad: crossfire apart", "crates/core/src/squad.rs", "            if apart >= CROSSFIRE_APART {", "            if apart >= CROSSFIRE_APART * 100 {", "two_rifles_round_you"),
 ("squad: the flanker goes behind", "crates/core/src/squad.rs", "let spot = open_near(nav, level, m, want, int(8), |p| behind(m, p));", "let spot = open_near(nav, level, m, want, int(8), |p| !behind(m, p));", "an_outrider_goes_round"),
 ("the shield", "crates/core/src/world.rs", "                let shielded =\n                    d.armour > 0", "                let shielded =\n                    false && d.armour > 0", "a_warders_shield"),
 ("the sniper's lock", "crates/core/src/enemy.rs", "                if t == attack.lock {\n                    self.locked = Some(target);", "                if false {\n                    self.locked = Some(target);", "a_marksmans_aim_locks"),
 ("the glass breaks", "crates/core/src/world.rs", "        if !started && front.z + tank::HULL_LEN / 2 > line {", "        if !started && front.z + tank::HULL_LEN / 2 > line + int(100) {", "the_executor_comes_through"),
 ("the tank's armour", "crates/core/src/world.rs", "                        take(t, Target::Armour(i));", "                        take(t, Target::Weak(i, 0));", "the_executor_is_hurt_only"),
 ("cannons grow", "crates/core/src/tank.rs", "        .take(phase + 1)", "        .take(1)", "the_executor_is_hurt_only"),
 ("the last phase is outlasted", "crates/core/src/enemy.rs", "                    if t.survive == 0 {\n                        self.hp = 0;", "                    if false {\n                        self.hp = 0;", "the_executors_last_phase"),
 ("the pound's bounce", "crates/core/src/player.rs", "            self.vel.y = bounce_speed(h);", "            self.vel.y = 0;", "a_ground_pound_drops"),
 ("the pound's shockwave", "crates/core/src/world.rs", "            if near && self.enemies[i].hurt(damage) {", "            if false && near && self.enemies[i].hurt(damage) {", "a_ground_pound_drops"),
 ("the saw's rev", "crates/core/src/world.rs", "                    if let (Some(r), true) = (rev, on_beat) {", "                    if let (Some(r), true) = (rev, false) {", "a_ripsaw_blade_lodges"),
 ("blades in hand", "crates/core/src/weapons.rs", "        if !pull.pressed || held.cooldown > 0 || held.saws_out >= saw.saws {", "        if !pull.pressed || held.cooldown > 0 {", "a_ripsaw_throws"),
 ("hair trigger", "crates/core/src/weapons.rs", "    (base * 100 / (100 + crate::balance::TRIGGER_PCT * triggers.max(0))).max(1)", "    base.max(1)", "hair_triggers"),
 ("the chief's cap", "crates/core/src/world.rs", "            if e.summon_timer >= d.summon_every && mine < d.summon_cap {\n                e.summon_timer = 0;\n                call.extend(d.summon.iter().take((d.summon_cap - mine).max(0) as usize));", "            if e.summon_timer >= d.summon_every {\n                e.summon_timer = 0;\n                call.extend(d.summon.iter());", "the_section_chief_calls"),
 ("his men withdraw", "crates/core/src/world.rs", "                if !self.enemies[k].alive() && e.alive() {", "                if false && !self.enemies[k].alive() && e.alive() {", "the_section_chief_calls"),
 ("ricochet", "crates/core/src/orbs.rs", "            if self.bounce > 0 {", "            if self.bounce > 1000 {", "a_ricochet_glances"),
 ("the lob's arc", "crates/core/src/orbs.rs", "                    let up = rise / ticks + v.fall * ticks / 2;", "                    let up = rise / ticks;", "a_lobbed_canister"),
 ("the sweep", "crates/core/src/orbs.rs", "                -v.sweep / 2 + v.sweep / (v.repeat - 1) * repeat", "                0 * repeat", "a_sweeping_burst"),
 ("the lane closes", "crates/core/src/orbs.rs", "                if self.pos.sub(self.hub).dot(self.axis) > 0 {", "                if self.pos.sub(self.hub).dot(self.axis) < 0 {", "a_lanes_walls"),
 ("the mini-boss node", "crates/core/src/route.rs", "            n.elite = elite;", "            n.elite = None;", "the_house_route_has"),
 ("the ripsaw's zone", "crates/core/src/route.rs", "                .filter(|&g| c.guns[g].from_zone <= zone)", "                .filter(|&g| c.guns[g].from_zone <= 9)", "the_house_route_has"),
 ("appeal", "crates/core/src/meta.rs", '                "appeal" => b.appeal = true,', '                "appeal" => {}', "the_houses_upgrades"),
 ("good conduct", "crates/core/src/meta.rs", "        let banked = earned * (100 + self.sentence_bonus_pct() + self.conduct_pct(c)) / 100;", "        let banked = earned * (100 + self.sentence_bonus_pct()) / 100;", "the_houses_upgrades"),
 ("the house's own patterns", "data/enemies.json", '"pattern": "burst_fire"', '"pattern": "spiral_quad"', "the_house_has_seven"),
 ("a tank phase needs weak points", "crates/core/src/content.rs", "                    if k < last && (p.weak.is_empty() || p.weak.iter().any(|w| w.hp <= 0 || w.radius <= 0)) {", "                    if false {", "a_tank_phase_with_nothing"),
 ("the lab's walls", "crates/core/src/arena/house.rs", "            let deep = -LAB_DEPTH - m(1);", "            let deep = -m(1);", "every_house_gate"),
 ("a hit is heard", "crates/core/src/world.rs", "            self.note(\"hurt\", String::new());\n            if self.player.recess_left < recess {", "            if self.player.recess_left < recess {", "an_orbs_hit_is_heard"),
 # round six
 ("the second fire's cooldown", "crates/core/src/weapons.rs", "    held.alt_cooldown = a.cooldown;", "    held.alt_cooldown = 0;", "every_gun_has_a_second_fire"),
 ("the blades shatter", "crates/core/src/world.rs", "                blasts.push((s.pos, enemy, weak, base + s.teeth / 2));", "", "the_ripsaws_second_fire"),
 ("the rush goes on", "crates/core/src/game.rs", "                if run.rush {", "                if false && run.rush {", "a_boss_rush_runs"),
 ("the rush waits for a win", "crates/core/src/game.rs", " || !self.meta.rush_open()", "", "a_boss_rush_runs"),
 ("the test save owns it all", "crates/core/src/meta.rs", "            owned: c.upgrades.iter().map(|u| u.id.clone()).collect(),", "            owned: Vec::new(),", "the_test_save_holds"),
 # round seven
 ("grab only the injured", "crates/core/src/world.rs", "                    && e.health_pct() <= GRAB_PCT\n", "", "an_injured_enemy_can_be_grabbed"),
 ("the throw's damage", "crates/core/src/world.rs", "            damage: THROW_DAMAGE,", "            damage: 0,", "an_injured_enemy_can_be_grabbed"),
 ("stopped orbs", "crates/core/src/world.rs", "        if !stopped {\n            self.orbs_step(c, chest);", "        if true {\n            self.orbs_step(c, chest);", "your_time_stop_holds"),
 ("held still by its stop", "crates/core/src/world.rs", "        let frozen = self.stop.is_some_and(|s| s.caster.is_some());", "        let frozen = false;", "the_defendant_stops_time"),
 ("drawn into the maw", "crates/core/src/world.rs", "        p.pos = p.pos.add(to.norm().scale(pull.min(to.len())));", "", "when_the_defendant_falls"),
 ("the summit melts", "crates/core/src/world.rs", "            fx::div(self.ending.min(ENDING_TICKS) * ONE, ENDING_TICKS * ONE)", "            0", "when_the_defendant_falls"),
 ("the mech ascends", "crates/core/src/enemy.rs", "(secs(9, 1), int(7), secs(4, 1))", "(secs(9, 1), 0, secs(4, 1))", "the_pioneer_boosts"),
 ("the mech boosts", "crates/core/src/enemy.rs", "(secs(3, 1), secs(1, 2), per_tick(22, 1))", "(secs(3, 1), 0, per_tick(22, 1))", "the_pioneer_boosts"),
 ("the escort is small", "crates/core/src/tank.rs", "            scale: fx::div(d.radius, crate::content::cm(320)).clamp(ONE / 4, ONE * 2),", "            scale: ONE,", "the_escort_is_a_smaller"),
 ("the escort falls", "crates/core/src/world.rs", "            self.enemies[i].hp = 0;\n            self.fell(c, vec![i]);\n            return;", "            return;", "the_escort_is_a_smaller"),
 ("a dry cylinder waits", "crates/core/src/weapons.rs", " || held.dry))", "))", "the_big_iron_fires_six"),
 ("one shell at a time", "crates/core/src/weapons.rs", "                self.rounds += 1;", "                self.rounds = mag;", "every_gun_has_a_second_fire"),
 ("the clocks wait for the Defendant", "crates/core/src/meta.rs", "        if !u.unlock.is_empty() && !self.beaten.contains(&u.unlock) {", "        if false {", "the_stopped_clocks_wait"),
 ("the time stop comes with the win", "crates/core/src/meta.rs", "            b.stop_ticks += STOP_BASE;", "            b.stop_ticks += 0;", "the_stopped_clocks_wait"),
 ("each step is higher", "crates/core/src/arena/ascent.rs", "        let depth = rise + m(80);", "        let depth = m(80);", "every_gate_on_the_mountain"),
 ("the mini-boss list", "crates/core/src/route.rs", "            let mb = r.minibosses.iter().find(|m| m.at == i);", "            let mb = r.minibosses.iter().find(|m| m.at == 99);", "the_house_route_has"),
 ("the ascent's own patterns", "data/enemies.json", '"pattern": "proxy_burst"', '"pattern": "burst_fire"', "the_ascent_has_seven"),
 ("safe ground", "crates/core/src/player.rs", "            if self.safe_ground(level) {", "            if true {", "a_fall_returns_you"),
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
