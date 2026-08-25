//! The `arda` binary: `generate` and `export` subcommands (`mockup/01`, `03`).

use anyhow::{bail, Context, Result};
use arda::{
    export_area, export_block, export_overview, generate, ExportFormat, GenerateConfig,
    LatitudeBand, SizeKm, World,
};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(
    name = "arda",
    version,
    about = "Deterministic procedural worldgen for tabletop"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the batch: continent, areas, blocks.
    Generate {
        /// The sole source of nondeterminism.
        #[arg(long)]
        seed: u64,
        /// Continent extent as WxH in kilometres.
        #[arg(long, default_value = "500x1000")]
        size: String,
        /// Use the 8-tile micro continent (the walking-skeleton slice).
        #[arg(long)]
        micro: bool,
        /// World directory to create.
        #[arg(long)]
        out: PathBuf,
    },
    /// Generate a world and render one overview image of it.
    ///
    /// The quickest way to look at a seed: the default continent takes
    /// well under a minute and lands as a single PNG.
    Preview {
        /// The sole source of nondeterminism.
        #[arg(long)]
        seed: u64,
        /// Continent extent as WxH in kilometres.
        #[arg(long, default_value = "500x1000")]
        size: String,
        /// Use the 8-tile micro continent instead.
        #[arg(long)]
        micro: bool,
        /// Pixels per area tile in the overview.
        #[arg(long, default_value_t = arda::OVERVIEW_PX_PER_AREA)]
        px: u32,
        /// Directory to create; holds `world/` and `overview.png`.
        #[arg(long)]
        out: PathBuf,
    },
    /// Render or serialise part of a generated world.
    Export {
        /// The world directory to read.
        #[arg(long)]
        world: PathBuf,
        /// Area tile as `<ax>,<ay>`. Ignored with `--overview`.
        #[arg(long, default_value = "0,0")]
        area: String,
        /// Output format.
        #[arg(long, value_enum, default_value_t = Format::Png)]
        format: Format,
        /// Render the whole world as one overview image instead of one area.
        #[arg(long)]
        overview: bool,
        /// Render one tactical block (the WFC tile layer) as
        /// `<ax>,<ay>,<cx>,<cy>`. Blocks exist on a 64-cell stride.
        #[arg(long)]
        block: Option<String>,
        /// Directory to write into.
        #[arg(long)]
        out: PathBuf,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Png,
    Json,
}

fn parse_size(text: &str) -> Result<SizeKm> {
    let Some((w, h)) = text.split_once('x') else {
        bail!("size must look like 500x1000, got {text}");
    };
    Ok(SizeKm::new(
        w.parse().context("size width must be a number")?,
        h.parse().context("size height must be a number")?,
    ))
}

fn run_generate(seed: u64, size: &str, micro: bool, out: &Path) -> Result<()> {
    let config = if micro {
        GenerateConfig::MICRO
    } else {
        GenerateConfig::new(parse_size(size)?, LatitudeBand::new(35, 55), 15)?
    };

    println!(
        "arda {} — deterministic worldgen (seed {seed}, size {}x{}km, {} areas)",
        env!("CARGO_PKG_VERSION"),
        config.size_km().width,
        config.size_km().height,
        config.areas_wide() * config.areas_high()
    );
    println!("[1/3] continent   relief · coast");
    println!("[2/3] areas       relief → water");
    println!("[3/3] blocks      64x64 tile-ID grids, zstd-compressed");

    let manifest = generate(seed, config, out)?;
    println!(
        "done — {} · {} areas · land {}‰",
        out.display(),
        manifest.stats.area_count,
        manifest.stats.land_fraction_permille
    );
    Ok(())
}

fn run_preview(seed: u64, size: &str, micro: bool, px: u32, out: &Path) -> Result<()> {
    let config = if micro {
        GenerateConfig::MICRO
    } else {
        GenerateConfig::new(parse_size(size)?, LatitudeBand::new(35, 55), 15)?
    };
    let world_dir = out.join("world");
    std::fs::create_dir_all(&world_dir).context("cannot create the output directory")?;

    println!(
        "arda {} — preview (seed {seed}, {}x{}km, {} areas)",
        env!("CARGO_PKG_VERSION"),
        config.size_km().width,
        config.size_km().height,
        config.areas_wide() * config.areas_high()
    );
    let manifest = generate(seed, config, &world_dir)?;
    println!(
        "  generated {} areas · land {}‰",
        manifest.stats.area_count, manifest.stats.land_fraction_permille
    );

    let world = World::load(&world_dir)?;
    let path = export_overview(&world, out, px)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn run_export(
    world: &Path,
    area: &str,
    format: Format,
    overview: bool,
    block: Option<&str>,
    out: &Path,
) -> Result<()> {
    if let Some(spec) = block {
        let parts: Vec<&str> = spec.split(',').map(str::trim).collect();
        let [ax, ay, cx, cy] = parts.as_slice() else {
            bail!("block must look like 1,1,64,128 (area x, area y, cell x, cell y), got {spec}");
        };
        let world = World::load(world)?;
        std::fs::create_dir_all(out).context("cannot create the output directory")?;
        let fmt = match format {
            Format::Png => ExportFormat::Png,
            Format::Json => ExportFormat::Json,
        };
        let path = export_block(
            &world,
            ax.parse().context("area x must be a number")?,
            ay.parse().context("area y must be a number")?,
            cx.parse().context("cell x must be a number")?,
            cy.parse().context("cell y must be a number")?,
            out,
            fmt,
        )
        .with_context(|| {
            format!("no block at {spec}; blocks are materialised on a 64-cell stride over land")
        })?;
        println!("wrote {}", path.display());
        return Ok(());
    }
    if overview {
        let world = World::load(world)?;
        std::fs::create_dir_all(out).context("cannot create the output directory")?;
        let path = export_overview(&world, out, arda::OVERVIEW_PX_PER_AREA)?;
        println!("wrote {}", path.display());
        return Ok(());
    }
    let Some((ax, ay)) = area.split_once(',') else {
        bail!("area must look like 1,1, got {area}");
    };
    let ax: i32 = ax.trim().parse().context("area x must be a number")?;
    let ay: i32 = ay.trim().parse().context("area y must be a number")?;

    let world = World::load(world)?;
    std::fs::create_dir_all(out).context("cannot create the output directory")?;
    let format = match format {
        Format::Png => ExportFormat::Png,
        Format::Json => ExportFormat::Json,
    };
    let path = export_area(&world, ax, ay, out, format)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Generate {
            seed,
            size,
            micro,
            out,
        } => run_generate(seed, &size, micro, &out),
        Command::Preview {
            seed,
            size,
            micro,
            px,
            out,
        } => run_preview(seed, &size, micro, px, &out),
        Command::Export {
            world,
            area,
            format,
            overview,
            block,
            out,
        } => run_export(&world, &area, format, overview, block.as_deref(), &out),
    }
}
