//! Diagnostic: form a recipe-5 MICRO-sized surface and print what the
//! world-water passes built (logic/02 §world-water).
//! Usage: water_diag <seed>
#![allow(missing_docs, clippy::unwrap_used, clippy::cast_precision_loss)]

use arda_core::GenerateConfig;
use arda_gen::continent::generate_continent_attempt_formed;
use arda_gen::formation::form_with_water;
use arda_gen::orchestrator::fine_source::{admit, FineSourceLimits};

fn main() {
    let seed: u64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let config = GenerateConfig::MICRO;
    let limits = FineSourceLimits {
        max_ram_bytes: u128::MAX,
        max_file_bytes: u128::MAX,
    };
    let plan = admit(config, limits).unwrap();
    let grid = generate_continent_attempt_formed(seed, config, 0);
    let t = std::time::Instant::now();
    let (_, f) =
        form_with_water(seed, 0, &grid, plan.fine_width, plan.fine_height, 16 << 30).unwrap();
    let st = f.stats;
    println!("formed in {:?}", t.elapsed());
    println!("{st:?}");
    if st.meander_valley_q8 > 0 {
        println!(
            "meander sinuosity {:.3}",
            st.meander_course_q8 as f64 / st.meander_valley_q8 as f64
        );
    }
    if std::env::var_os("SINKS").is_some() {
        for k in &f.sinks {
            println!(
                "sink {:?} at cell ({}, {}) radius {} m",
                k.kind,
                k.x_um / 100_000_000,
                k.y_um / 100_000_000,
                k.radius_um / 1_000_000
            );
        }
    }
    println!(
        "sinks {} braided cells {} deltas {:?} dolines {}",
        f.sinks.len(),
        f.braided_cells.len(),
        f.deltas,
        f.dolines.len()
    );
}
