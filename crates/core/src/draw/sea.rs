//! The Far Shore's Drowned Fleet and its beasts, Dagon and its tentacles,
//! and the Vault's machines.

use crate::content::{Content, EnemyDef, Rgb};
use crate::enemy::Enemy;
use crate::geom::V3;
use crate::render::Batch;

/// Draws one of these; false for any other, which is drawn elsewhere.
pub fn draw(
    _b: &mut Batch,
    _c: &Content,
    _e: &Enemy,
    _d: &EnemyDef,
    _at: V3,
    _s: i32,
    _paint: &dyn Fn(Rgb) -> Rgb,
) -> bool {
    false
}
