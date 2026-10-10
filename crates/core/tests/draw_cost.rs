//! What a scene costs to draw: instances and vertices per mesh. Ignored: it
//! prints. `cargo test -p ltc_core --test draw_cost -- --ignored --nocapture`

use ltc_core::arena;
use ltc_core::content::{Content, Files};
use ltc_core::mesh::{self, Mesh};
use ltc_core::render::{Batch, INSTANCE_VALUES, PASSES};

#[test]
#[ignore]
fn print_the_cost_of_each_archetype() {
    let c = Content::parse(&Files {
        weapons: include_str!("../../../data/weapons.json"),
        patterns: include_str!("../../../data/patterns.json"),
        enemies: include_str!("../../../data/enemies.json"),
        zone: include_str!("../../../data/zone.json"),
        upgrades: include_str!("../../../data/upgrades.json"),
        palette: include_str!("../../../data/palette.json"),
        story: include_str!("../../../data/story.json"),
        copy: include_str!("../../../data/copy.json"),
    })
    .unwrap();
    for kind in ["arrival", "plaza", "market", "boss"] {
        let a = arena::build(kind, 9, &[arena::Side::North, arena::Side::East], &c.pal);
        let mut b = Batch::new([0; 3]);
        for s in &a.shapes {
            b.shape(s);
        }
        let (_, draws) = b.finish(0);
        let mut total = 0;
        let mut line = String::new();
        for (pass, list) in draws.iter().enumerate().take(PASSES) {
            for d in list.chunks(5) {
                let m = Mesh::ALL[d[1] as usize];
                let verts = mesh::vertices(m).len() / mesh::VERTEX_VALUES;
                total += verts * d[3] as usize;
                line += &format!(" {m:?}/{pass}:{}x{verts}", d[3]);
            }
        }
        let _ = INSTANCE_VALUES;
        println!("{kind}: {total} vertices;{line}");
    }
}
