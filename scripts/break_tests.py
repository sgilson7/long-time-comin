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
 ("cage closes inward", "crates/core/src/orbs.rs", "Launch::In | Launch::Gather => from_hub.neg(),", "Launch::In | Launch::Gather => from_hub,", "a_cage_round_you"),
 ("stagger", "crates/core/src/orbs.rs", "let wait = (v.hold + v.stagger * k as i32).max(1);", "let wait = (v.hold + v.stagger).max(1);", "a_staggered_formation"),
 ("second hold", "crates/core/src/orbs.rs", "if self.flown > r.stop {", "if self.flown > r.stop * 1000 {", "a_refracted_shot"),
 ("blue colours", "data/palette.json", '"wards": [\n    [\n      "cyan",', '"wards": [\n    [\n      "lime",', "blue_orbs_are"),
 ("the wall breaks blue", "crates/core/src/world.rs", "if o.ward > 0 && o.ward <= wards && wall.is_some_and(|w| w.breaks(&o)) {", "if o.ward > 0 && o.ward <= wards && wall.is_some_and(|w| w.breaks(&o)) && false {", "the_injunction_breaks"),
 ("the wall's cooldown", "crates/core/src/player.rs", "            self.wall_cooldown = if plus { WALL_PLUS_COOLDOWN } else { WALL_COOLDOWN };", "            self.wall_cooldown = 0;", "the_wall_stands_half"),
 ("vents", "crates/core/src/world.rs", "            self.player.launch_up(VENT_SPEED);", "            self.player.launch_up(VENT_SPEED / 2);", "an_updraft_vent"),
 ("death burst", "crates/core/src/world.rs", "        let Some(p) = d.burst else {", "        let Some(p) = d.burst.filter(|_| false) else {", "a_shardling_leaves"),
 ("buried is unhurtable", "crates/core/src/enemy.rs", "self.alive() && !self.buried && !matches!", "self.alive() && !matches!", "an_auger_cannot"),
 ("blink", "crates/core/src/enemy.rs", "            if level.ray(at, to.norm(), to.len()).is_none() {\n                return Some(at);", "            if false {\n                return Some(at);", "a_clerk_reappears"),
 ("arrival cache", "crates/core/src/world.rs", "if node.kind == NodeKind::Arrival && !node.cache.is_empty() {", "if node.kind == NodeKind::Arrival && node.cache.is_empty() {", "the_landing_offers"),
 ("cache of two", "crates/core/src/route.rs", "            for _ in 0..2 {\n                let name = draw(&r.cache_rewards", "            for _ in 0..1 {\n                let name = draw(&r.cache_rewards", "a_route_on_the_ice"),
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
 ("the second fire's cooldown", "crates/core/src/weapons.rs", "    held.alt_cooldown = a.cooldown;\n    let damage = if a.kind == AltKind::Cloture {", "    held.alt_cooldown = 0;\n    let damage = if a.kind == AltKind::Cloture {", "every_gun_has_a_second_fire"),
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
 ("the blaster is small", "crates/core/src/tank.rs", "            scale: fx::div(d.radius, crate::content::cm(320)).clamp(ONE / 4, ONE * 2),", "            scale: ONE,", "the_blaster_is_a_smaller"),
 ("the blaster falls", "crates/core/src/world.rs", "            self.enemies[i].hp = 0;\n            self.fell(c, vec![i]);\n            return;", "            return;", "the_blaster_is_a_smaller"),
 ("a dry cylinder waits", "crates/core/src/weapons.rs", " || held.dry))", "))", "the_big_iron_fires_six"),
 ("one shell at a time", "crates/core/src/weapons.rs", "                self.rounds += 1;", "                self.rounds = mag;", "every_gun_has_a_second_fire"),
 ("the clocks wait for the Defendant", "crates/core/src/meta.rs", "        if !u.unlock.is_empty() && !self.beaten.contains(&u.unlock) {", "        if false {", "the_stopped_clocks_wait"),
 ("the time stop comes with the win", "crates/core/src/meta.rs", "            b.stop_ticks += STOP_BASE;", "            b.stop_ticks += 0;", "the_stopped_clocks_wait"),
 ("each step is higher", "crates/core/src/arena/ascent.rs", "        let depth = rise + m(80);", "        let depth = m(80);", "every_gate_on_the_mountain"),
 ("the mini-boss list", "crates/core/src/route.rs", "            let mb = r.minibosses.iter().find(|m| m.at == i);", "            let mb = r.minibosses.iter().find(|m| m.at == 99);", "the_house_route_has"),
 ("the ascent's own patterns", "data/enemies.json", '"pattern": "proxy_burst"', '"pattern": "burst_fire"', "the_ascent_has_its_assassins"),
 ("safe ground", "crates/core/src/player.rs", "            if self.safe_ground(level) {", "            if true {", "a_fall_returns_you"),
 # round eight: the Defendant's moves
 ("a dome rises in rows", "crates/core/src/orbs.rs", "                let pitch = fx::TURN / 4 * j / rows;", "                let pitch = 0;", "a_dome_is_a_half_sphere"),
 ("a gathering orb is spent", "crates/core/src/orbs.rs", "        if !alive || gathered {", "        if !alive || (gathered && false) {", "the_defendants_stop_lays_a_dome"),
 ("the dome's shells come apart", "data/patterns.json", '"hold_ms": 650,\n          "launch": "gather"', '"hold_ms": 500,\n          "launch": "gather"', "the_defendants_stop_lays_a_dome"),
 ("the stop one time in four", "crates/core/src/enemy.rs", "            let due = self.passes % a.once_in == 0;", "            let due = true;", "the_defendant_stops_time_only"),
 ("the hook's cast waits", "crates/core/src/enemy.rs", "            State::Casting(0) if self.hooking() => {", "            State::Casting(0) if false => {", "the_defendant_hooks"),
 ("the pound leaps", "crates/core/src/enemy.rs", "                LUNGE_POUND_HEIGHT,\n", "                0,\n", "the_defendant_hooks"),
 # round nine: the mountain's creatures, the wall's wards
 ("each ward breaks more", "crates/core/src/world.rs", "if o.ward > 0 && o.ward <= wards && wall", "if o.ward > 0 && o.ward <= 1 && wall", "each_ward_bought"),
 ("a ward's amount is its ward", "crates/core/src/meta.rs", "b.wall_ward = b.wall_ward.max(u.amount - 1),", "b.wall_ward = b.wall_ward.max(u.amount - 2),", "each_ward_bought"),
 ("a bound is high", "crates/core/src/enemy.rs", "BEAST_GRAVITY * ticks / 2 + fx::div(to.y - self.pos.y, t),", "fx::div(to.y - self.pos.y, t),", "a_hart_bounds"),
 ("a bound goes round you", "crates/core/src/enemy.rs", "let a = yaw + fx::deg(BOUND_TURN_DEG) * self.orbit;", "let a = yaw;", "a_hart_bounds"),
 ("a hop is short", "crates/core/src/enemy.rs", "self.pos.add(way.add(side).norm().scale(HOP_LENGTH))", "self.pos.add(way.add(side).norm().scale(HOP_LENGTH * 3))", "a_toad_hops"),
 ("a rook dives", "crates/core/src/enemy.rs", "let y = if diving {\n                    target.y + WHEEL_DIVE_OVER", "let y = if false {\n                    target.y + WHEEL_DIVE_OVER", "a_rook_circles"),
 ("a bite reaches you", "crates/core/src/enemy.rs", "|| (e.reach > 0 && orbs::within(e.head(), chest, reach))", "|| false", "a_gaper_stays_rooted"),
 ("a freshet rushes round", "crates/core/src/enemy.rs", "let a = self.age * fx::TURN / SURGE_PERIOD * self.orbit;", "let a = 0;", "a_freshet_rushes"),
 ("corruption by height", "crates/core/src/world.rs", "        e.corrupt = self.corrupt;", "        e.corrupt = CORRUPT_LEAST;", "more_corrupted_the_higher"),
 ("a beast's body clears the floor", "crates/core/src/enemy.rs", "let body = d.radius.min(d.hover * 9 / 10).max(ONE / 10);", "let body = d.radius;", "a_toad_hops"),

 ("a mini-boss is not the zone's boss", "crates/core/src/world.rs", "            Kind::Boss | Kind::Tank | Kind::Double | Kind::God | Kind::Cat | Kind::Oak\n        ) && !d.elite", "            Kind::Boss | Kind::Tank | Kind::Double | Kind::God | Kind::Cat | Kind::Oak\n        )", "the_blaster_is_a_smaller"),
 # round ten: the Great Horned, big waves
 ("the owl perches high", "crates/core/src/enemy.rs", "Some(v3(p.x, f + PERCH_HEIGHT, p.z))", "Some(v3(p.x, f, p.z))", "the_great_horned_perches"),
 ("the owl swoops through you", "crates/core/src/enemy.rs", "self.fly_to(target.add(way.scale(SWOOP_PAST)), SWOOP_SPEED);", "self.fly_to(self.perches[self.perch], SWOOP_SPEED);", "the_great_horned_perches"),
 ("the owl holds fire in flight", "crates/core/src/enemy.rs", "self.state = State::Rest(t.max(2));", "self.state = State::Rest(t);", "the_great_horned_holds"),
 ("fifteen at most", "crates/core/src/world.rs", "        most.saturating_sub(self.alive_enemies())", "        usize::MAX - most", "never_more_than_fifteen"),
 ("the big wave comes in as room is made", "crates/core/src/world.rs", "                let n = self.room().min(self.waves_left[0].len());\n                let more", "                let n = 0;\n                let more", "never_more_than_fifteen"),

    # round eleven: the Idiot God
 ("song: great and good", "crates/core/src/song.rs", "let j = if off <= l.great { Judge::Great } else { Judge::Good };", "let j = Judge::Great;", "a_note_is_judged"),
 ("song: a verse played well", "crates/core/src/song.rs", "if hit * 100 >= l.pass_pct * n {", "if hit * 100 > l.pass_pct * n * 2 {", "the_dancers_pipe_a_verse"),
 ("song: the god stirs", "crates/core/src/song.rs", "h.stirred = true;", "", "a_verse_played_badly"),
 ("song: the finale", "crates/core/src/song.rs", "h.finale = true;", "", "every_verse_played_well"),
 ("song: fired on the beat", "crates/core/src/song.rs", "h.fire.push(b.pattern);", "", "the_god_fires_on_the_beat"),
 ("the god cannot be killed", "crates/core/src/enemy.rs", "self.hp = (self.hp - damage).max(self.floor_hp);", "self.hp -= damage;", "the_gods_last_bar_does_not_kill"),
 ("the trumpet puts the gun away", "crates/core/src/world.rs", "fire: false,\n                fire_pressed: false,", "fire: ctl.fire,\n                fire_pressed: ctl.fire_pressed,", "the_gods_last_bar_does_not_kill"),
 ("the song puts it to sleep", "crates/core/src/world.rs", "god.hp = 0;", "", "played_through_the_lullaby"),
 ("falling comes round", "crates/core/src/world.rs", "if self.arena.wrap > 0 && p.pos.y < -WRAP_BELOW {", "if false {", "off_an_edge_inside_the_god"),
 ("through a seam", "crates/core/src/world.rs", "if let Some(k) = seams.iter().position(|s| s.crossed(p.prev_pos, p.pos)) {", "if let Some(k) = seams.iter().position(|s| s.crossed(p.prev_pos, p.pos) && false) {", "through_a_seam_you_come_out"),
 ("a seam turns you", "crates/core/src/arena.rs", "fx::wrap(other.yaw - self.yaw + fx::HALF)", "fx::wrap(fx::HALF)", "through_a_seam_you_come_out"),
 ("orbs through seams", "crates/core/src/world.rs", "if let Some(k) = seams.iter().position(|s| s.crossed(o.prev, o.pos)) {", "if let Some(k) = seams.iter().position(|s| s.crossed(o.prev, o.pos) && false) {", "an_orb_through_a_seam"),
 ("the Reprise's trail", "crates/core/src/world.rs", "e.trail = trail;", "e.trail = trail.map(|_| V3::ZERO);", "a_reprise_walks"),
 ("the Etching, a statue", "crates/core/src/enemy.rs", "if self.seen || dist < d.radius {", "if dist < d.radius {", "an_etching_moves_and_fires"),
 ("the Etching holds its fire", "crates/core/src/enemy.rs", "        if d.kind == Kind::Beast && d.gait == Gait::Unseen && self.seen {\n            return deeds;", "        if d.kind == Kind::Beast && d.gait == Gait::Unseen && self.seen && false {\n            return deeds;", "an_etching_moves_and_fires"),
 ("the gaze", "crates/core/src/world.rs", "dir.dot(fwd) >= look", "dir.dot(fwd) >= ONE * 2", "an_etching_you_look_at"),
 ("the Render charges straight", "crates/core/src/enemy.rs", "let next = self.pos.add(self.bite_dir.scale(d.speed));", "let next = self.pos.add(flat.norm().scale(d.speed));", "a_render_turns_on_the_spot"),
 ("the Render turns on the spot", "crates/core/src/enemy.rs", "self.yaw = fx::wrap(self.yaw + off.clamp(-CHARGE_TURN, CHARGE_TURN));", "self.yaw = want;", "a_render_turns_on_the_spot"),
 ("the Toons' shared beat", "crates/core/src/enemy.rs", "let t = self.beat % BOUNCE_TICKS;", "let t = (self.beat + self.age) % BOUNCE_TICKS;", "every_toon_bounces"),
 ("the Glyph marches across", "crates/core/src/enemy.rs", "v3(-way.z * self.orbit, 0, way.x * self.orbit)", "way", "a_glyph_marches"),
 ("the Recursion splits", "crates/core/src/world.rs", ".take(self.room())", ".take(0)", "a_recursion_splits"),
 ("the Etching is an engraving", "crates/core/src/draw/god.rs", "    b.style = PASS_DITHER;", "    b.style = PASS_SOLID;", "each_of_zone_fives_enemies"),
 ("the Render is a PlayStation's", "crates/core/src/draw/god.rs", "    b.style = PASS_PS1;", "    b.style = PASS_SOLID;", "each_of_zone_fives_enemies"),
 ("the Reprise is a negative", "crates/core/src/draw/god.rs", "    b.style = PASS_NEGATIVE;", "    b.style = PASS_SOLID;", "each_of_zone_fives_enemies"),
 ("the Glyph's two frames", "crates/core/src/draw/god.rs", "let frame = &GLYPH[(e.moves & 1) as usize];", "let frame = &GLYPH[0];", "the_glyph_has_two_frames"),
 ("the god's eyes shut as it sleeps", "crates/core/src/draw/god.rs", "let open = fx::mul(base * ONE, awake) / ONE;", "let open = base;", "the_gods_eyes_open"),
 ("the god's eyes by phase", "crates/core/src/draw/god.rs", "        0 => EYES / 3,", "        0 => EYES,", "the_gods_eyes_open"),
 ("the trumpet's valves", "crates/core/src/draw/god.rs", "    for k in -1..=1 {\n        let p = valves", "    for k in 0..0 {\n        let p = valves", "the_trumpet_is_brass"),
 ("the Penrose foot faces back", "crates/core/src/arena/god.rs", "let foot = self.frame(v3(-mid, 0, SEAM_Z), fx::HALF,", "let foot = self.frame(v3(-mid, 0, SEAM_Z), 0,", "the_penrose_stairs_top"),
 ("seams in pairs", "crates/core/src/arena/god.rs", "let sa = self.frame(a.0, a.1, half, height, film, k + 1);", "let sa = self.frame(a.0, a.1, half, height, film, k);", "the_doors_seams_come_in_pairs"),
 ("the god's arenas come round", "crates/core/src/arena/god.rs", "        self.wrap = WRAP;\n", "        self.wrap = 0;\n", "the_world_is_drawn_again"),
 ("a floor under the Penrose seam", "crates/core/src/arena/god.rs", "        self.slab(-LANE, SEAM_Z, -WELL, LANE, 0, stone);\n", "        self.slab(-LANE, WELL, -WELL, LANE, 0, stone);\n", "the_penrose_stairs_top"),
 ("room for the god", "crates/core/src/arena/god.rs", "        self.arch(0, m(12), true, m(6), m(4), bone);\n", "        self.post(0, 0, -m(40), m(3), m(10), bone);\n        self.arch(0, m(12), true, m(6), m(4), bone);\n", "the_heart_leaves_the_god_room"),
 ("the god's stairs", "crates/core/src/arena/god.rs", "        let n = ((to - from).abs() + RISE - 1) / RISE;", "        if to > from {\n            return;\n        }\n        let n = ((to - from).abs() + RISE - 1) / RISE;", "every_gate_in_the_god"),
 ("over nothing, the last floor", "crates/core/src/world.rs", "if self.arena.level.floor_below(over.x, over.y, over.z).is_none() {", "if false {", "falling_where_there_is_no_floor"),
 ("a bar for each phase", "crates/core/src/game.rs", "\"bars_left\": bars - bar,", "\"bars_left\": bars,", "the_god_has_a_bar"),

 # the lullaby practice
 ("practice: straight to the song", "crates/core/src/game.rs", "                Outcome::Playing => self.lull(),", "                Outcome::Playing => {}", "the_lullaby_practice"),
 ("practice: nothing on the record", "crates/core/src/game.rs", "            if self.run.as_ref().is_some_and(|r| r.practice) {\n                self.end_practice();", "            if false {\n                self.end_practice();", "the_lullaby_practice"),
 # the last sentence
 ("last: home after the summit", "crates/core/src/game.rs", "            Some(home) if mission && zone == self.c.mission_break() => home,", "            Some(_) if mission && zone == self.c.mission_break() => zone + 1,", "the_last_mission_opens"),
 ("last: opens once the god sleeps", "crates/core/src/meta.rs", "            && self.beaten.iter().any(|b| *b == c.zones[last].boss)", "            && !c.zones[last].boss.is_empty()", "the_last_mission_opens"),
 ("last: half as many again", "crates/core/src/world.rs", "            let extra = ((wave.len() as i32 * REMIX_EXTRA_PCT + 99) / 100) as usize;", "            let extra = 0;", "the_last_missions_fights"),
 ("last: remixed patterns", "crates/core/src/enemy.rs", "            (true, Some(k)) => k,", "            (true, Some(_)) => pattern,", "the_last_missions_fights"),
 ("last: the scream cannot be hurt", "crates/core/src/enemy.rs", "                    State::Changing(CAT_SCREAM + 1)", "                    State::Rest(1)", "chumbles_screams"),
 ("last: a gait a bar", "crates/core/src/enemy.rs", "            d.phases[self.phase.min(d.phases.len() - 1)].gait", "            d.gait", "each_of_chumbles_bars"),
 ("last: the cat knows the perches", "crates/core/src/world.rs", "                .map(|p| p.add(v3(0, d.hover, 0)))\n                .collect();", "                .map(|p| p.add(v3(0, d.hover, 0)))\n                .take(0)\n                .collect();", "each_of_chumbles_bars"),
 ("last: fed, he sleeps", "crates/core/src/world.rs", "        if self.fed >= CAT_FED_TICKS {", "        if self.fed >= CAT_FED_TICKS * 1000 {", "fed_he_shrinks"),
 ("last: reality breaks", "crates/core/src/world.rs", "        if self.mission {\n            let p = &mut self.player;", "        if false {\n            let p = &mut self.player;", "the_defendants_fall_breaks_reality"),
 ("last: the cat walks out", "crates/core/src/world.rs", "        let walked = fx::mul(GUIDE_SPEED, (self.tick - GUIDE_WAIT).max(0) * ONE);", "        let walked = 0;", "the_cat_walks_out"),
 ("last: a cat's phases have gaits", "crates/core/src/content.rs", "                if (kind == Kind::Cat) != !p.gait.is_empty() {", "                if false {", "a_cats_phases"),
 ("last: no remix of a remix", "crates/core/src/content.rs", "            if out[k].remix.is_some() {", "            if false {", "a_remix_must_name"),
 ("practice: Chumbles maxed out", "crates/core/src/game.rs", "        let (build, gun_level) = Meta::everything(&self.c).build(&self.c);", "        let (build, gun_level) = self.meta.build(&self.c);", "the_chumbles_practice"),
 ("practice: Chumbles from any bar", "crates/core/src/world.rs", "            let bar = self.from_bar.min(d.phases.len().max(1) - 1);", "            let bar = 0 * self.from_bar.min(d.phases.len().max(1) - 1);", "can_begin_at_any_bar"),
 ("practice: a later bar, grown", "crates/core/src/world.rs", "                e.size = ONE;\n                e.pos.y = feet + d.hover;", "                e.pos.y = feet + d.hover;", "can_begin_at_any_bar"),
 ("practice: the bars offered", "crates/core/src/game.rs", ".map(|b| b.bars.clone()).unwrap_or_default(),", ".map(|b| b.bars[1..].to_vec()).unwrap_or_default(),", "can_begin_at_any_bar"),
 ("practice: only a god is lulled", "crates/core/src/game.rs", "            if sings && e.floor_hp > 0 && w.song.is_none() {", "            if e.floor_hp > 0 && w.song.is_none() {", "the_chumbles_practice"),

 # Chumbles drawn
 ("cat: feet on the floor at any size", "crates/core/src/draw/cat.rs", "let floor = v3(at.x, at.y - fx::mul(d.hover, e.size), at.z);", "let floor = v3(at.x, at.y - d.hover, at.z);", "small_he_is_a_third"),
 ("cat: as big as his sphere", "crates/core/src/draw/cat.rs", "let scale = fx::mul(e.size, fx::div(d.radius, MODEL_RADIUS));", "let scale = fx::mul(e.size, fx::div(d.radius, MODEL_RADIUS)) / 2;", "chumbles_fills_the_sphere"),
 ("cat: asleep in a loaf", "crates/core/src/draw/cat.rs", "        return loaf(age);", "        return stand();", "asleep_his_eyes_are_shut"),
 # the house (home.rs)
 ("home: the landing's door walled off", "crates/core/src/arena/home.rs", "                self.home_landing();", "                self.home_landing();\n                self.solids.push(Solid::new(v3(-fx::int(6), 0, -fx::int(9)), v3(fx::int(6), fx::int(3), -fx::int(8))));", "every_home_room_builds"),
 ("home: the cat's way blocked", "crates/core/src/arena/home.rs", "                self.home_bedroom();", "                self.home_bedroom();\n                self.solids.push(Solid::new(v3(fx::int(1), 0, -fx::int(5)), v3(fx::int(2), fx::int(1), -fx::int(4))));", "the_cat_walks_from"),
 ("home: a solid fountain", "crates/core/src/arena/home.rs", "                self.home_landing();", "                self.home_landing();\n                self.solids.push(Solid::new(v3(-fx::int(1), 0, -fx::int(1)), v3(fx::int(1), fx::int(1), fx::int(1))));", "the_landing_keeps"),
 ("home: the fridge perch against the wall", "crates/core/src/arena/home.rs", "        self.perches.push(p([(fx0 + fx1) / 2, 360, fz - 30]));", "        self.perches.push(p([(fx0 + fx1) / 2, 360, fz - 230]));", "downstairs_has_room"),
 ("home: a block mid-room", "crates/core/src/arena/home.rs", "            \"boss\" => self.home_downstairs(),", "            \"boss\" => {\n                self.home_downstairs();\n                self.solids.push(Solid::new(v3(-fx::int(2), 0, -fx::int(2)), v3(fx::int(2), fx::int(2), fx::int(2))));\n            }", "downstairs_leaves_long_lanes"),
 # round fifteen: the arsenal
 ("arsenal: the Filibuster spins up", "crates/core/src/weapons.rs", "            held.spin = (held.spin + ONE / sp.up).min(ONE);", "            held.spin = 0;", "the_filibuster_fires_faster"),
 ("arsenal: held, the charge carries over", "crates/core/src/weapons.rs", "        held.charge += 1000;\n        if !pull.held {", "        held.charge = (held.charge + 1000).min(every);\n        if !pull.held {", "the_filibuster_fires_faster"),
 ("arsenal: cloture spends the spin", "crates/core/src/weapons.rs", "        held.spin = 0;\n        held.charge = 0;\n        d\n", "        d\n", "cloture_spends_the_spin"),
 ("arsenal: gold is never erased", "crates/core/src/world.rs", "    !o.gold && ward > 0 && ((1..=ward).contains(&o.ward) || ward > WARDS)", "    ward > 0", "a_redactor_round_erases"),
 ("arsenal: a round is spent on an orb", "crates/core/src/world.rs", "                        len = t;\n                        spent = true;", "                        len = t;\n                        spent = false;", "a_redactor_round_erases"),
 ("arsenal: the bar hits once", "crates/core/src/world.rs", "            if s.passed.contains(&i) || !self.enemies[i].hurtable() {", "            if !self.enemies[i].hurtable() {", "a_redaction_bar_erases"),
 ("arsenal: contempt chains", "crates/core/src/world.rs", "            if orbs::within(self.enemies[j].pos, at, r) && self.enemies[j].hurt(damage) {\n                killed.push(j);\n            }", "            if orbs::within(self.enemies[j].pos, at, r) {\n                self.enemies[j].hurt(damage);\n            }", "contempt_bursts_what_you_kill"),
 ("arsenal: a boss never bursts", "crates/core/src/world.rs", "        if n <= 0 || Some(i) == self.boss {", "        if n <= 0 {", "a_boss_takes_contempt"),
 ("arsenal: a gun stands above the one in hand", "crates/core/src/world.rs", "PickupKind::Gun(g, (held_level + step).clamp(1, MAX_GUN_LEVEL))", "PickupKind::Gun(g, step.clamp(1, MAX_GUN_LEVEL))", "a_side_paths_gun_stands_above"),
 ("arsenal: a choice of two", "crates/core/src/world.rs", "                let group = if self.choose && all.len() > 1 { 3 } else { 0 };", "                let group = 0;", "a_side_paths_gun_stands_above"),
 ("arsenal: the purse", "crates/core/src/world.rs", "                    self.earned += n;\n", "", "a_purse_banks"),
 ("arsenal: carried", "crates/core/src/meta.rs", "            g\n        } else {\n            choices[0]\n        }", "            choices[0]\n        } else {\n            choices[0]\n        }", "an_exhibit_waits"),
 ("arsenal: carry only what an Exhibit bought", "crates/core/src/meta.rs", "|| !self.carry_choices(c).contains(&g)", "", "an_exhibit_waits"),
 ("arsenal: a run carries in its gun", "crates/core/src/game.rs", "        let held = Held::new(self.meta.carried(&self.c), gun_level);\n        let start = route.start();", "        let held = Held::new(self.c.gun(\"carbine\"), gun_level);\n        let start = route.start();", "a_run_carries_in"),
 ("arsenal: no gun past level five", "crates/core/src/route.rs", "            \"weapon\" => self.level < MAX_GUN_LEVEL,", "            \"weapon\" => true,", "side_path_guns_are_upgrades"),
 ("arsenal: no pulse twice", "crates/core/src/route.rs", "            \"pulse\" => !self.pulse,", "            \"pulse\" => true,", "side_path_guns_are_upgrades"),
 ("arsenal: no name twice on side paths", "crates/core/src/route.rs", ".filter(|n| !used.contains(n) && have.wants(n)", ".filter(|n| have.wants(n)", "side_paths_vary_hold"),
 ("arsenal: a long path holds a choice", "crates/core/src/route.rs", "                        if len > 1 {", "                        if len > 5 {", "side_paths_vary_hold"),
 ("arsenal: nothing twice in a zone", "crates/core/src/route.rs", "            let pick = draw(&r.rest_rewards, &mut used, have, true, &mut rng);", "            let pick = draw(&r.rest_rewards, &mut Vec::new(), have, true, &mut rng);", "side_paths_vary_hold"),
 # round sixteen: the Holding's tree
 ("tree: a step out from what it needs", "crates/core/src/tree.rs", "            place(k, at_h, ring + 1, parent, angle_of, places);", "            place(k, at_h, ring, parent, angle_of, places);", "every_upgrade_that_needs_nothing_stands_on_the_ring"),
 ("tree: each its own share", "crates/core/src/tree.rs", "            at_h += 2 * leaves(k, parent);", "            at_h += 0;", "no_two_upgrades_crowd"),
 ("tree: a branch's wedge", "crates/core/src/tree.rs", "        cursor += TREE_GAP_HALVES;\n        for &r in list {", "        for &r in list {", "each_branch_bursts_out"),
 ("tree: the petal's reach", "crates/core/src/tree.rs", "            reach: (TREE_RING + outer * TREE_STEP + TREE_PETAL).min(TREE_EDGE),", "            reach: TREE_RING,", "each_branch_bursts_out"),
 # round seventeen: the Water Gap
 ("river: Bound Over on one tick", "crates/core/src/player.rs", "                    flat = v3(d.x, 0, d.z);", "", "bound_over_dash_and_jump"),
 ("river: a Bench is stood on", "crates/core/src/player.rs", "                if let Some(top) = level.one_way_catch(min, max, self.pos.y) {", "                if let Some(top) = level.one_way_catch(min, max, self.pos.y).filter(|_| false) {", "a_bench_laid_in_the_air"),
 ("river: a Bench is taken up", "crates/core/src/course.rs", "                    self.arena.level.dynamic[b.0].on = false;", "", "a_bench_laid_in_the_air"),
 ("river: the water's cost", "crates/core/src/course.rs", "            let cost = (pl.build.max_health * DROWN_PCT / 100).max(1);", "            let cost = 0;", "the_water_returns_you"),
 ("river: a lever opens its gate", "crates/core/src/course.rs", "                level.dynamic[door.dynamic].on = false;", "", "a_lever_hooked_opens"),
 ("river: a gate never shuts on you", "crates/core/src/course.rs", "                        door.left = 1;", "                        d.on = true;", "a_lever_hooked_opens"),
 ("river: a lever answers the hook's strike", "crates/core/src/course.rs", "                    self.hooked.is_some_and(|t| near_box(&sw.at, t, ONE / 2))", "                    self.hooked.is_some_and(|t| near_box(&sw.at, t, -ONE))", "the_spillway_can_be_crossed"),
 ("river: a root rises", "crates/core/src/course.rs", "            let delta = m.corner(self.tick).sub(d.solid.min);", "            let delta = V3::ZERO;", "a_root_rises_and_sinks"),
 ("river: a root carries you down", "crates/core/src/course.rs", "                self.player.pos = self.player.pos.add(delta);", "", "a_root_rises_and_sinks"),
 ("river: a pound breaks rotten timber", "crates/core/src/course.rs", "                level.dynamic[r].on = false;", "", "a_pound_breaks_a_rotten"),
 ("river: a drum-cap doubles the bounce", "crates/core/src/course.rs", "                self.player.vel.y = bounce_speed(h * 2);", "                self.player.vel.y = bounce_speed(h);", "a_pound_breaks_a_rotten"),
 ("river: a curtain's hole", "crates/core/src/orbs.rs", "                    if (hole_i..hole_i + g).contains(&i) && (hole_j..hole_j + g).contains(&j) {", "                    if false {", "a_curtain_leaves_a_hole"),
 ("river: nothing erases the river's orbs", "crates/core/src/world.rs", "    !o.gold && ward > 0 && o.ward <= WARDS && ((1..=ward).contains(&o.ward) || ward > WARDS)", "    !o.gold && ward > 0 && ((1..=ward).contains(&o.ward) || ward > WARDS)", "the_wards_a_redactor_counts"),
 ("river: shut until the Executor falls", "crates/core/src/meta.rs", "        if !self.beaten.iter().any(|b| b == RIVER_OPENER) {", "        if false {", "the_river_stays_shut"),
 ("river: the fork after the house", "crates/core/src/game.rs", "                return b;", "                let _ = b;", "on_the_river_the_executors_fall"),
 ("river: the Oak's own verdict", "crates/core/src/game.rs", "                    let banked = self.meta.record_river(&self.c, total, cleared);", "                    let banked = self.meta.record_win(&self.c, total, cleared, \"\").0;", "the_oaks_fall_ends_the_river"),
 ("river: a course is crossed, not cleared", "crates/core/src/course.rs", "        self.cleared = self.kind != crate::route::NodeKind::Boss;", "        self.cleared = false;", "the_water_gap_is_its_four"),
 ("river: the Oak's bar is the climb", "crates/core/src/course.rs", "                o.hp = climb_line + ((max - climb_line) as i64 * left as i64 / span as i64) as i32;", "                o.hp = max;", "the_oak_can_be_climbed"),
 ("river: the crown calls the Summons", "crates/core/src/course.rs", "                    self.elite = Some(i);", "", "the_oak_can_be_climbed"),
 ("river: the Husk first", "crates/core/src/enemy.rs", "            self.husk -= damage;", "            self.husk -= 0;\n            self.hp -= damage;", "the_summons_husk"),
 ("river: staggered, double damage", "crates/core/src/enemy.rs", "        let damage = if self.stagger > 0 { damage * 2 } else { damage };", "", "the_summons_husk"),
 ("river: the Husk grows back", "crates/core/src/enemy.rs", "            if self.unhit > d.husk_regen && self.husk < self.husk_max {", "            if false {", "the_summons_husk"),

 # round eighteen: the Far Shore and the Vault
 ("far: a gust shoves", "crates/core/src/course.rs", "                shove = shove.add(v3(push.x, 0, push.z));", "", "a_gust_shoves"),
 ("far: the shove reaches the body", "crates/core/src/player.rs", "        self.vel.x += self.shove.x;", "", "a_gust_shoves"),
 ("far: lightning strikes", "crates/core/src/course.rs", "                    self.player.hurt(st.damage);", "", "lightning_is_marked"),
 ("far: the current to the eye", "crates/core/src/course.rs", "                shove = shove.add(v3(u.eye.x - p.x, 0, u.eye.z - p.z).norm().scale(u.pull));", "", "under_the_water_you_fall"),
 ("far: lighter under the water", "crates/core/src/player.rs", "                    GRAVITY * SWIM_GRAVITY_PCT / 100,", "                    GRAVITY,", "under_the_water_you_fall"),
 ("far: a stroke up", "crates/core/src/player.rs", "            self.vel.y = self.vel.y.max(SWIM_STROKE);", "", "under_the_water_you_fall"),
 ("far: wreckage turns", "crates/core/src/course.rs", "            let turned = crate::geom::rotate_y(r, self.orbit.wrapping_mul(tick));", "            let turned = r;", "wreckage_turns_round"),
 ("far: a crew rides its deck", "crates/core/src/course.rs", "                        e.pos = e.pos.add(delta);", "", "wreckage_turns_round"),
 ("far: overboard is drowned", "crates/core/src/course.rs", "            if e.pos.y < k.water - ENEMY_DROWN {", "            if false {", "a_fought_course_opens"),
 ("far: a fought course is shut until clear", "crates/core/src/course.rs", "        self.cleared = !k.fought && self.kind != crate::route::NodeKind::Boss;", "        self.cleared = self.kind != crate::route::NodeKind::Boss;", "a_fought_course_opens"),
 ("far: the Eraser charges in the ring", "crates/core/src/course.rs", "                    ch.held[n] += 1;", "", "the_erasers_charge"),
 ("far: your older self cannot be hurt", "crates/core/src/course.rs", "                e.shielded = true;", "", "the_erasers_charge"),
 ("far: the gaze slows", "crates/core/src/world.rs", "                    self.player.slowed = SLOWED_TICKS;", "", "a_cockatrices_gaze"),
 ("far: later river starts wait for their boss", "crates/core/src/meta.rs", "        self.river_shut(c).is_none() && (first || before.is_some_and(|p| self.beaten.contains(&p.boss)))", "        self.river_shut(c).is_none()", "the_far_shore_starts_once"),

 ("far: a rising deck lands you", "crates/core/src/course.rs", "                if over && b.pos.y >= top - ONE / 4 && b.pos.y < d.solid.max.y {", "                if false {", "a_deck_rising_into_a_body"),
 ("far: the last overspeed bleeds off", "crates/core/src/player.rs", "        let next = if speed - decel > top {", "        let next = if speed > top {", "let_go_at_full_speed"),
 ("far: the boss comes where the course puts it", "crates/core/src/world.rs", "            .filter(|_| matches!(d.kind, Kind::Boss | Kind::Double))", "            .filter(|_| false)", "dagon_and_your_older_self"),
 ("far: the Vault rests in the Lock", "crates/core/src/arena/vault.rs", "            \"rest\" => self.the_lock(),", "", "the_vaults_rest_is_the_lock"),
 ("far: a section's own sky", "crates/core/src/game.rs", "            .and_then(|n| z.scenes.get(&n.archetype))", "            .and_then(|_| None)", "the_hurricane_has_its_own"),
 ("far: a ship strikes her colours", "crates/core/src/course.rs", "            if !k.flags[n].struck && k.flags[n].crew.iter().all(down) {", "            if false {", "each_ship_of_the_fleet_strikes"),
 ("far: the dragon is held by its locks", "crates/core/src/course.rs", "                .any(|&l| foe(l).is_some_and(|f| self.enemies[f].alive()));", "                .any(|_| false);", "the_captured_dragon_keeps_to_its_chain"),
 ("far: the chain's reach", "crates/core/src/enemy.rs", "            if let Some((at, reach)) = self.leash {", "            if let Some((at, reach)) = self.leash.filter(|_| false) {", "on_its_chain_the_dragon_swoops"),
 ("far: a rogue wave lifts its ship", "crates/core/src/course/waves.rs", "        m.corner(tick).add(v3(0, self.lift_of(i, tick), 0))", "        m.corner(tick)", "a_rogue_wave_lifts_a_deck"),
 ("far: a lifted deck carries its rider", "crates/core/src/course.rs", "                self.player.pos = self.player.pos.add(delta);", "", "a_rogue_wave_lifts_a_deck"),
 ("far: a wash throws you", "crates/core/src/course/waves.rs", "                pl.vel = v3(pl.vel.x + knock.x, pl.vel.y.max(WASH_HEAVE), pl.vel.z + knock.z);", "", "a_wash_shoves"),
 ("far: a wash hurts", "crates/core/src/course/waves.rs", "                pl.hurt(WASH_DAMAGE);", "", "a_wash_shoves"),
 ("far: a wash hurts once", "crates/core/src/course/waves.rs", "            if k.waves.struck[n] != cycle {", "            if true {", "a_wash_shoves"),
 ("far: a wash is jumped", "crates/core/src/course/waves.rs", "            if p.y - floor >= WASH_HEIGHT {", "            if p.y - floor >= WASH_HEIGHT * 3 {", "a_wash_shoves"),
 ("far: a rogue wave shoves a swimmer", "crates/core/src/course/waves.rs", "fx::mul(ROGUE_SWIM_PUSH, s)", "0", "a_rogue_wave_shoves"),
 ("far: a rogue wave rolls past the line before it comes again", "crates/core/src/arena/sea/hurricane.rs", "        let speed = ROGUE_SPEED;", "        let speed = ROGUE_SPEED / 2;", "a_rogue_wave_lifts_no_ship"),
 ("far: the Hurricane has its waves", "crates/core/src/arena/sea/hurricane.rs", "        self.rogue_wave(v3(ONE, 0, ROGUE_SLANT), ROGUE_PHASE);", "", "the_hurricane_is_crossed_through"),
 ("far: the waves are drawn", "crates/core/src/draw/waves.rs", "    let Some(k) = &w.course else { return };", "    let Some(k) = w.course.as_ref().filter(|_| false) else { return };", "the_hurricanes_waves_draw"),
 ("far: the start boat rides the rogue", "crates/core/src/arena/sea/hurricane.rs", "        self.jolly_boat(a.stern, a.crest());", "", "the_boat_you_start_on_rides"),
 ("far: the decks are cut away", "crates/core/src/draw.rs", "    b.scenery = true;\n    course(b, c, w);", "    course(b, c, w);", "the_fleets_decks_and_masts_are_cut"),
 ("far: moving scenery marked to cut", "crates/core/src/render.rs", "let see_through = (buffer == STATIC_BUFFER || half == SCENERY) as u32;", "let see_through = (buffer == STATIC_BUFFER) as u32;", "scenery_that_moves_is_cut"),
 ("far: the sea lets you swim", "crates/core/src/course.rs", "            p.y < k.water - k.depth || k.wet > SEA_BREATH", "            p.y < k.water || k.wet > SEA_BREATH", "overboard_in_the_open_sea"),
 ("far: the sea slows your fall", "crates/core/src/player.rs", "(GRAVITY * SWIM_GRAVITY_PCT / 100, SEA_SINK)", "(GRAVITY, MAX_FALL)", "overboard_in_the_open_sea"),
 ("far: the sea's breath", "crates/core/src/course.rs", "            p.y < k.water - k.depth || k.wet > SEA_BREATH", "            p.y < k.water - k.depth", "the_sea_takes_you"),
 ("river: the Oak's leaves hang under its crown", "crates/core/src/arena/river/oak.rs", "v3(at.x, top - m(4), at.z), v3(w + m(2), m(4), w + m(2)), leaf);", "v3(at.x, top + m(4), at.z), v3(w, m(5), w), leaf);", "nothing_leafy_stands_over"),
 ("river: the drum-cap is broad", "crates/core/src/arena/river/falls.rs", "        self.drum_cap(0, S3 + m(1), -m(94), m(4));", "        self.drum_cap(0, S3 + m(1), -m(96), m(2));", "a_pound_thrown_early"),
 ("map: 2031 at the top", "crates/core/src/atlas.rs", "        let y = ATLAS_ROW * (rows - 1) / 2 - ATLAS_ROW * row;", "        let y = ATLAS_ROW * row - ATLAS_ROW * (rows - 1) / 2;", "the_root_is_at_the_top"),
 ("map: the River on the right", "crates/core/src/atlas.rs", "                at: place((f + 1 + n) as i32, 1),", "                at: place((f + 1 + n) as i32, -1),", "the_river_forks_after"),
 ("map: a walkthrough keeps to the main path", "crates/core/src/tour.rs", "    while let Some(&to) = route.nodes[at].next.first() {", "    while let Some(&to) = route.nodes[at].next.last() {", "a_walkthrough_runs_the_main_path"),
 ("map: the camera flies on", "crates/core/src/tour.rs", "            let s = fx::mul(total, f);", "            let s = 0;", "a_course_is_flown_along"),
 ("map: the body is parked", "crates/core/src/tour.rs", "        p.pos = start;", "", "a_course_is_flown_along"),
 ("map: on to the next stop", "crates/core/src/tour.rs", "            let next = (self.at + 1) % self.stops.len();", "            let next = self.at;", "each_stop_gives_way"),
 ("map: a shut zone cannot start", "crates/core/src/game.rs", "        if self.mode != Mode::Tour || !self.zone_open(zone, &line) {", "        if self.mode != Mode::Tour {", "the_map_opens_from_the_holding"),
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
