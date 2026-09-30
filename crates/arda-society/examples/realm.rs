//! Synthetic three-realm world: simulates its society and prints a
//! gazetteer. Also writes `out/society/realm.json`.
//!
//! `cargo run -p arda-society --example realm [seed]`

use arda_society::{gazetteer, output, simulate_society, synthetic};
use std::path::Path;

fn main() -> Result<(), arda_society::SocietyError> {
    let seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(42);
    let world = synthetic::world(seed)?;
    let society = simulate_society(seed, &world)?;
    print!("{}", gazetteer::render(&society, &world));
    output::write_json(&society, Path::new("out/society/realm.json"))?;
    Ok(())
}
