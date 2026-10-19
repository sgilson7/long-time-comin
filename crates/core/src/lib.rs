//! ltc_core: every rule of Long Time Comin'.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.
#![allow(clippy::needless_range_loop, clippy::collapsible_match)]
// the HUD is one large json! literal
#![recursion_limit = "256"]

pub mod arena;
pub mod balance;
pub mod codex;
pub mod content;
pub mod course;
pub mod draw;
pub mod enemy;
pub mod fx;
pub mod game;
pub mod geom;
pub mod level;
pub mod mesh;
pub mod meta;
pub mod nav;
pub mod orbs;
pub mod player;
pub mod render;
pub mod rng;
pub mod route;
pub mod song;
pub mod squad;
pub mod tank;
pub mod tree;
pub mod weapons;
pub mod world;
