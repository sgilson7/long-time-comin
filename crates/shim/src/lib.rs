//! ltc_shim: moves values between the page and `ltc_core::game`.
//!
//! Builder requirement: the shim decides nothing. Each function is one call
//! into the core and a conversion of its arguments or result. The core speaks
//! Q16.16 integers; WebGL wants `f32`, so values on the way out are divided by
//! the core's own `ONE`, and the page's float clock and mouse movement are
//! turned into the integers the core expects. `crates/core/tests/boundary.rs`
//! fails if a branch, a loop or the `?` operator appears here.

use ltc_core::{fx, game};
use wasm_bindgen::prelude::*;

const ONE: f32 = fx::ONE as f32;

fn floats(v: Vec<i32>) -> Vec<f32> {
    v.into_iter().map(|x| x as f32 / ONE).collect()
}

#[wasm_bindgen]
pub struct Game {
    inner: game::Game,
}

#[wasm_bindgen]
impl Game {
    #[allow(clippy::too_many_arguments)]
    #[wasm_bindgen(constructor)]
    pub fn new(
        weapons: &str,
        patterns: &str,
        enemies: &str,
        zone: &str,
        upgrades: &str,
        palette: &str,
        story: &str,
        copy: &str,
        saved: &str,
    ) -> Result<Game, JsValue> {
        game::Game::new(
            weapons, patterns, enemies, zone, upgrades, palette, story, copy, saved,
        )
        .map(|inner| Game { inner })
        .map_err(|e| JsValue::from_str(&e))
    }

    /// CSS size and device pixel ratio in; drawing-buffer size out.
    pub fn resize(&mut self, css_w: u32, css_h: u32, dpr: f64) -> Vec<i32> {
        self.inner
            .resize(css_w as i32, css_h as i32, (dpr * 100.0) as i32)
            .to_vec()
    }

    /// The page's clock in milliseconds, held action bits, mouse movement in CSS pixels.
    pub fn advance(&mut self, now_ms: f64, bits: u32, dx: f64, dy: f64) {
        self.inner.advance(
            (now_ms * 1000.0) as i64,
            bits,
            (dx * 100.0) as i32,
            (dy * 100.0) as i32,
        )
    }

    pub fn view(&self) -> Vec<f32> {
        floats(self.inner.view())
    }

    pub fn instances(&self) -> Vec<f32> {
        floats(self.inner.instances())
    }

    pub fn static_instances(&self) -> Vec<f32> {
        floats(self.inner.static_instances())
    }

    pub fn draws(&self) -> Vec<u32> {
        self.inner.draws()
    }

    pub fn uniforms(&self) -> Vec<f32> {
        floats(self.inner.scene_uniforms())
    }

    pub fn clear_color(&self) -> Vec<f32> {
        floats(self.inner.clear_color())
    }

    pub fn scene_version(&self) -> u32 {
        self.inner.scene_version()
    }

    pub fn hud(&self) -> String {
        self.inner.hud_json()
    }

    pub fn screen(&self) -> String {
        self.inner.screen_json()
    }

    pub fn codex(&self) -> String {
        self.inner.codex_json()
    }

    pub fn begin(&mut self) {
        self.inner.begin()
    }

    pub fn finish_intro(&mut self) {
        self.inner.finish_intro()
    }

    pub fn replay_intro(&mut self) {
        self.inner.replay_intro()
    }

    pub fn buy(&mut self, id: &str) -> bool {
        self.inner.buy(id)
    }

    pub fn start_run(&mut self) {
        self.inner.start_run()
    }

    /// A run from the start of a later zone, once its way is open.
    pub fn start_run_at(&mut self, zone: u32) {
        self.inner.start_run_at(zone as usize)
    }

    /// From the interlude into the next zone.
    pub fn continue_run(&mut self) {
        self.inner.continue_run()
    }

    pub fn pause(&mut self) {
        self.inner.pause()
    }

    pub fn resume(&mut self) {
        self.inner.resume()
    }

    pub fn toggle_map(&mut self) {
        self.inner.toggle_map()
    }

    pub fn abandon(&mut self) {
        self.inner.abandon()
    }

    pub fn to_holding(&mut self) {
        self.inner.to_holding()
    }

    pub fn reset(&mut self) {
        self.inner.reset()
    }

    /// The record, for the page to keep in local storage.
    pub fn saved(&self) -> String {
        self.inner.saved()
    }
}

#[wasm_bindgen]
pub fn numbers() -> String {
    game::numbers()
}

#[wasm_bindgen]
pub fn mesh(id: u32) -> Vec<f32> {
    floats(game::mesh_vertices(id))
}

/// White noise for the page's sound, from the core's seeded generator.
#[wasm_bindgen]
pub fn noise(n: u32, seed: u32) -> Vec<f32> {
    floats(game::noise(n, seed))
}
