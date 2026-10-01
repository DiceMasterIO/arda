//! `arda generate`: the world size, latitude band and fine-terrain options
//! (recipe and resource ceilings).

use crate::Terrain;
use anyhow::{bail, Context, Result};
use arda::{
    generate, generate_from_fine_recipe, FineDeliveryLimits, GenerateConfig, LatitudeBand, SizeKm,
};
use std::path::Path;

pub(crate) fn parse_size(text: &str) -> Result<SizeKm> {
    let Some((w, h)) = text.split_once('x') else {
        bail!("size must look like 500x1000, got {text}");
    };
    Ok(SizeKm::new(
        w.parse().context("size width must be a number")?,
        h.parse().context("size height must be a number")?,
    ))
}

/// Fine-terrain generation options (`--recipe` and resource ceilings).
#[derive(Clone)]
pub(crate) struct FineOptions {
    pub(crate) recipe: Option<u16>,
    pub(crate) ram_bytes: Option<u128>,
    pub(crate) file_bytes: Option<u128>,
    pub(crate) latitude: Option<String>,
}

/// Parses `SOUTH,NORTH` degrees.
pub(crate) fn parse_latitude(text: &str) -> Result<LatitudeBand> {
    let Some((s, n)) = text.split_once(',') else {
        bail!("latitude must look like 35,55, got {text}");
    };
    Ok(LatitudeBand::new(
        s.trim()
            .parse()
            .context("southern latitude must be a number")?,
        n.trim()
            .parse()
            .context("northern latitude must be a number")?,
    ))
}

pub(crate) fn run_generate(
    seed: u64,
    size: &str,
    micro: bool,
    out: &Path,
    terrain: Terrain,
    fine: FineOptions,
) -> Result<()> {
    let FineOptions {
        recipe,
        ram_bytes: fine_ram_bytes,
        file_bytes: fine_file_bytes,
        latitude,
    } = fine;
    let band = match latitude.as_deref() {
        Some(text) => parse_latitude(text)?,
        None => GenerateConfig::MICRO.latitude_band(),
    };
    let config = if micro {
        GenerateConfig::new(GenerateConfig::MICRO.size_km(), band, 15)?
    } else {
        GenerateConfig::new(parse_size(size)?, band, 15)?
    };

    println!(
        "arda {} — deterministic worldgen (seed {seed}, size {}x{}km, {} areas)",
        env!("CARGO_PKG_VERSION"),
        config.size_km().width,
        config.size_km().height,
        config.areas_wide() * config.areas_high()
    );
    if terrain == Terrain::Legacy && (fine_ram_bytes.is_some() || fine_file_bytes.is_some()) {
        bail!("fine resource ceilings require --terrain fine");
    }
    if terrain == Terrain::Legacy && recipe.is_some() {
        bail!("--recipe requires --terrain fine");
    }
    println!("stages: continent → prepared terrain → shared water → areas → blocks");

    let manifest = match terrain {
        Terrain::Legacy => generate(seed, config, out)?,
        Terrain::Fine => {
            let mut limits = FineDeliveryLimits::default();
            if let Some(bytes) = fine_ram_bytes {
                limits.source.max_ram_bytes = bytes;
            }
            if let Some(bytes) = fine_file_bytes {
                limits.source.max_file_bytes = bytes;
            }
            let recipe = recipe.unwrap_or(arda::FINE_TERRAIN_RECIPE_VERSION);
            generate_from_fine_recipe(seed, config, out, limits, recipe)?
        }
    };
    println!(
        "done — {} · {} areas · land {}‰ · {} rivers",
        out.display(),
        manifest.stats.area_count,
        manifest.stats.land_fraction_permille,
        manifest.stats.river_count
    );
    Ok(())
}
