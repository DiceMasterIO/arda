//! Diagnostic: form a recipe-5 surface and dump it for inspection.
//! Usage: form_fine_terrain <seed> <width_km> <height_km> <out.bin>
#![allow(missing_docs, clippy::unwrap_used, clippy::cast_precision_loss)]

use arda_core::{GenerateConfig, LatitudeBand, SizeKm};
use arda_gen::continent::generate_continent_attempt_formed;
use arda_gen::formation::{form, FINE_SPACING_UM};
use arda_gen::orchestrator::fine_source::{admit, FineSourceLimits};
use std::io::Write;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let seed: u64 = a[1].parse().unwrap();
    let config = GenerateConfig::new(
        SizeKm::new(a[2].parse().unwrap(), a[3].parse().unwrap()),
        LatitudeBand::new(35, 55),
        15,
    )
    .unwrap();
    let limits = FineSourceLimits {
        max_ram_bytes: u128::MAX,
        max_file_bytes: u128::MAX,
    };
    let plan = admit(config, limits).unwrap();
    let t = std::time::Instant::now();
    let grid = generate_continent_attempt_formed(seed, config, 0);
    let g = form(seed, 0, &grid, plan.fine_width, plan.fine_height, 16 << 30).unwrap();
    eprintln!("formed {}x{} in {:?}", g.width, g.height, t.elapsed());
    let mut f = std::io::BufWriter::new(std::fs::File::create(&a[4]).unwrap());
    for v in [
        g.width as f32,
        g.height as f32,
        FINE_SPACING_UM as f32 / 1e6,
        0.0,
        0.0,
    ] {
        f.write_all(&v.to_le_bytes()).unwrap();
    }
    for &z in &g.z {
        f.write_all(&(z as f32 / 1000.0).to_le_bytes()).unwrap();
    }
}
