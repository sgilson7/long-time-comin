//! Prints the Codex, for reading it through. Ignored.
//! `cargo test -p ltc_core --test codex_print -- --ignored --nocapture`

#[test]
#[ignore]
fn print_the_codex() {
    let g = ltc_core::game::Game::new(
        include_str!("../../../data/weapons.json"),
        include_str!("../../../data/patterns.json"),
        include_str!("../../../data/enemies.json"),
        include_str!("../../../data/zone.json"),
        include_str!("../../../data/upgrades.json"),
        include_str!("../../../data/palette.json"),
        include_str!("../../../data/story.json"),
        include_str!("../../../data/copy.json"),
        "",
    )
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&g.codex_json()).unwrap();
    println!("{}", serde_json::to_string_pretty(&v).unwrap());
}
