//! The `arda` binary: `generate` and `export` subcommands (`mockup/01`, `03`).

use anyhow::{bail, Context, Result};
use arda::{
    export_area_with_quality_and_style, export_area_with_scale, export_block, export_overview,
    export_overview_with_look, generate, AreaImageScale, ExportFormat, GenerateConfig,
    ImageQuality, LatitudeBand, MapStyle, OverviewLook, World,
};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::{Path, PathBuf};

mod generate;
mod society;
mod tactical;
mod tactical_import;

#[cfg(test)]
use generate::parse_latitude;
use generate::{parse_size, run_generate, FineOptions};

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
        /// Terrain source. Fine uses stream-power formation (recipe 7 unless
        /// --recipe says otherwise) and a fixed five-attempt gate.
        #[arg(long, value_enum, default_value_t = Terrain::Legacy)]
        terrain: Terrain,
        /// Fine-terrain recipe: 7 (default: climate, arid basins), 8 (opt-in:
        /// recipe 7 plus alluvial plains), 6 (v0.2 formation), 5 (v0.1
        /// formation, replayed exactly) or 4 (spectral valleys). Fine mode
        /// only.
        #[arg(long, value_parser = clap::value_parser!(u16).range(4..=8))]
        recipe: Option<u16>,
        /// Latitude band as SOUTH,NORTH degrees (default 35,55). Recipe 7
        /// dries the subtropical belt (15-26°), e.g. `--latitude 15,35`.
        #[arg(long)]
        latitude: Option<String>,
        /// Fine-source RAM ceiling in bytes (default 16 GiB; fine mode only).
        #[arg(long)]
        fine_ram_bytes: Option<u128>,
        /// Fine-source file ceiling in bytes (default 4 GiB; fine mode only).
        #[arg(long)]
        fine_file_bytes: Option<u128>,
    },
    /// Generate a world and render one overview image of it.
    ///
    /// Writes a generated world and its cartographic overview.
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
        /// Legacy pixels per area tile; overrides the default 8K long edge.
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..=512), conflicts_with = "quality")]
        px: Option<u32>,
        /// PNG long edge: 512–32768 pixels, or 1k–32k (default: 8k).
        #[arg(long)]
        quality: Option<ImageQuality>,
        /// Cartographic PNG presentation; omission preserves Classic.
        #[arg(long, value_enum, conflicts_with = "px")]
        style: Option<Style>,
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
        /// Export an area PNG at 4096×4096 pixels; the terrain remains 100 m.
        #[arg(long, conflicts_with_all = ["overview", "block", "quality"])]
        detail: bool,
        /// PNG area side or overview long edge: 512–32768, or 1k–32k (default: 8k).
        #[arg(long, conflicts_with = "block")]
        quality: Option<ImageQuality>,
        /// Cartographic PNG presentation; omission preserves Classic.
        #[arg(long, value_enum)]
        style: Option<Style>,
        /// Directory to write into.
        #[arg(long)]
        out: PathBuf,
    },
    /// Settlements, roads, realms, land use and names for a generated world
    /// (writes `<world>/society/`).
    Settle {
        /// World directory.
        #[arg(long)]
        world: PathBuf,
    },
    /// Society over the settled world: town plans, politics, economy,
    /// history and stored notables.
    Society {
        #[command(subcommand)]
        command: society::SocietyCommand,
    },
    /// Tactical battle maps: validate libraries, make placeholders, render.
    Tactical {
        #[command(subcommand)]
        command: tactical::TacticalCommand,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Png,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Style {
    Classic,
    Atlas,
    /// Atlas seen slightly obliquely (goal 24, overviews only, opt-in).
    AtlasOblique,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Terrain {
    Legacy,
    Fine,
}

impl From<Style> for MapStyle {
    fn from(value: Style) -> Self {
        match value {
            Style::Classic => Self::Classic,
            Style::Atlas | Style::AtlasOblique => Self::Atlas,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct ImageOptions {
    detail: bool,
    quality: Option<ImageQuality>,
    style: Option<Style>,
}

impl ImageOptions {
    fn validate(self, format: Format, overview: bool, block: Option<&str>) -> Result<()> {
        if self.detail && (overview || block.is_some() || matches!(format, Format::Json)) {
            bail!("--detail is valid only for area PNG exports");
        }
        if self.quality.is_some() && (block.is_some() || matches!(format, Format::Json)) {
            bail!("--quality is valid only for area or overview PNG exports");
        }
        if self.detail && self.quality.is_some() {
            bail!("--detail cannot be combined with --quality");
        }
        if self.style.is_some() && (block.is_some() || matches!(format, Format::Json)) {
            bail!("--style is valid only for area or overview PNG exports");
        }
        if self.look().oblique && !overview {
            bail!("--style atlas-oblique is valid only for overview exports");
        }
        Ok(())
    }

    fn quality(self) -> ImageQuality {
        self.quality.unwrap_or_default()
    }

    fn style(self) -> MapStyle {
        self.style.map_or(MapStyle::Classic, Into::into)
    }

    fn look(self) -> OverviewLook {
        OverviewLook {
            oblique: matches!(self.style, Some(Style::AtlasOblique)),
        }
    }
}

fn run_preview(
    seed: u64,
    size: &str,
    micro: bool,
    px: Option<u32>,
    quality: Option<ImageQuality>,
    style: Option<Style>,
    out: &Path,
) -> Result<()> {
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
    let path = match px {
        Some(px) => export_overview(&world, out, px)?,
        None => export_overview_with_look(
            &world,
            out,
            quality.unwrap_or_default(),
            style.map_or(MapStyle::Classic, Into::into),
            ImageOptions {
                style,
                ..ImageOptions::default()
            }
            .look(),
        )?,
    };
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
    image: ImageOptions,
) -> Result<()> {
    image.validate(format, overview, block)?;
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
        let path =
            export_overview_with_look(&world, out, image.quality(), image.style(), image.look())?;
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
    let path = match (format, image.detail, image.style()) {
        (ExportFormat::Png, true, MapStyle::Classic) => export_area_with_scale(
            &world,
            ax,
            ay,
            out,
            ExportFormat::Png,
            AreaImageScale::Detail,
        )?,
        (ExportFormat::Png, true, MapStyle::Atlas) => export_area_with_quality_and_style(
            &world,
            ax,
            ay,
            out,
            ImageQuality::new(4096)?,
            MapStyle::Atlas,
        )?,
        (ExportFormat::Png, false, style) => {
            export_area_with_quality_and_style(&world, ax, ay, out, image.quality(), style)?
        }
        (ExportFormat::Json, false, _) => export_area_with_scale(
            &world,
            ax,
            ay,
            out,
            ExportFormat::Json,
            AreaImageScale::Preview,
        )?,
        (ExportFormat::Json, true, _) => unreachable!("validated above"),
    };
    println!("wrote {}", path.display());
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Settle { world } => society::settle(&world),
        Command::Society { command } => society::run(command),
        Command::Tactical { command } => tactical::run(command),
        Command::Generate {
            seed,
            size,
            micro,
            out,
            terrain,
            recipe,
            latitude,
            fine_ram_bytes,
            fine_file_bytes,
        } => run_generate(
            seed,
            &size,
            micro,
            &out,
            terrain,
            FineOptions {
                recipe,
                ram_bytes: fine_ram_bytes,
                file_bytes: fine_file_bytes,
                latitude,
            },
        ),
        Command::Preview {
            seed,
            size,
            micro,
            px,
            quality,
            style,
            out,
        } => run_preview(seed, &size, micro, px, quality, style, &out),
        Command::Export {
            world,
            area,
            format,
            overview,
            block,
            out,
            detail,
            quality,
            style,
        } => run_export(
            &world,
            &area,
            format,
            overview,
            block.as_deref(),
            &out,
            ImageOptions {
                detail,
                quality,
                style,
            },
        ),
    }
}

#[cfg(test)]
mod tests;
