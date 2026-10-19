//! The Water Gap's things that move or change: its emitters (the weir-lamp,
//! the root-eye, the spray-eye), the Gallows Oak's eye, and the Summons.

use crate::content::{Content, EnemyDef, Rgb};
use crate::enemy::Enemy;
use crate::geom::V3;
use crate::render::Batch;

/// Draws a river enemy; false for any other, which is drawn elsewhere.
pub fn draw(
    _b: &mut Batch,
    _c: &Content,
    _e: &Enemy,
    d: &EnemyDef,
    _at: V3,
    _s: i32,
    _paint: &dyn Fn(Rgb) -> Rgb,
) -> bool {
    matches!(d.id.as_str(), "gallows_oak")
}
