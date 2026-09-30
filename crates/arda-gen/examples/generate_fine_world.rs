//! Qualify the complete opt-in pipeline from an already admitted fine source.
use arda_core::{GenerateConfig, LatitudeBand, SizeKm};
use arda_gen::{
    orchestrator::{generate_world_from_fine_terrain, FineDeliveryLimits},
    HydrologyLimits,
};
use std::{env, error::Error, path::Path, time::Instant};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 7 {
        return Err("usage: generate_fine_world <seed> <attempt> <recipe-version> <width-km> <height-km> <fine-terrain-file> <new-world-directory>".into());
    }
    let config = GenerateConfig::new(
        SizeKm::new(args[3].parse()?, args[4].parse()?),
        LatitudeBand::new(35, 55),
        15,
    )?;
    let started = Instant::now();
    let recipe_version = args[2].parse()?;
    let limits = if recipe_version == 4 {
        FineDeliveryLimits::default().world
    } else {
        HydrologyLimits::default()
    };
    let manifest = generate_world_from_fine_terrain(
        args[0].parse()?,
        arda_core::FineTerrainDescriptor {
            recipe_version,
            attempt: args[1].parse()?,
        },
        config,
        Path::new(&args[5]),
        Path::new(&args[6]),
        limits,
    )?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    eprintln!(
        "Complete world generation: {:.3} seconds",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
