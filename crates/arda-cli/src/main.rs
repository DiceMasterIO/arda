//! The `arda` binary: `generate` and `export` subcommands (`mockup/01`, `03`).

use anyhow::{bail, Context, Result};
use arda::{
    export_area_with_quality, export_area_with_scale, export_block, export_overview,
    export_overview_with_quality, generate, AreaImageScale, ExportFormat, GenerateConfig,
    ImageQuality, LatitudeBand, SizeKm, World,
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

#[derive(Clone, Copy, Default)]
struct ImageOptions {
    detail: bool,
    quality: Option<ImageQuality>,
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
        Ok(())
    }

    fn quality(self) -> ImageQuality {
        self.quality.unwrap_or_default()
    }
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
    println!("stages: continent → prepared terrain → shared water → areas → blocks");

    let manifest = generate(seed, config, out)?;
    println!(
        "done — {} · {} areas · land {}‰ · {} rivers",
        out.display(),
        manifest.stats.area_count,
        manifest.stats.land_fraction_permille,
        manifest.stats.river_count
    );
    Ok(())
}

fn run_preview(
    seed: u64,
    size: &str,
    micro: bool,
    px: Option<u32>,
    quality: Option<ImageQuality>,
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
        None => export_overview_with_quality(&world, out, quality.unwrap_or_default())?,
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
        let path = export_overview_with_quality(&world, out, image.quality())?;
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
    let path = match format {
        ExportFormat::Png if !image.detail => {
            export_area_with_quality(&world, ax, ay, out, image.quality())?
        }
        _ => {
            let scale = if image.detail {
                AreaImageScale::Detail
            } else {
                AreaImageScale::Preview
            };
            export_area_with_scale(&world, ax, ay, out, format, scale)?
        }
    };
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
            quality,
            out,
        } => run_preview(seed, &size, micro, px, quality, &out),
        Command::Export {
            world,
            area,
            format,
            overview,
            block,
            out,
            detail,
            quality,
        } => run_export(
            &world,
            &area,
            format,
            overview,
            block.as_deref(),
            &out,
            ImageOptions { detail, quality },
        ),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn detail_conflicts_with_overview_and_blocks() {
        for extra in [vec!["--overview"], vec!["--block", "0,0,0,0"]] {
            let mut args = vec![
                "arda", "export", "--world", "missing", "--out", "unused", "--detail",
            ];
            args.extend(extra);
            assert!(Cli::try_parse_from(args).is_err());
        }
    }

    #[test]
    fn detail_json_reports_the_invalid_mode_before_reading_the_world() {
        let error = run_export(
            Path::new("missing-world"),
            "0,0",
            Format::Json,
            false,
            None,
            Path::new("unused-export"),
            ImageOptions {
                detail: true,
                quality: None,
            },
        );
        assert!(
            error.is_err_and(|e| e.to_string() == "--detail is valid only for area PNG exports")
        );
    }

    #[test]
    fn detail_is_an_explicit_area_png_option() {
        let cli = Cli::try_parse_from([
            "arda", "export", "--world", "saved", "--out", "exports", "--detail",
        ]);
        assert!(matches!(
            cli,
            Ok(Cli {
                command: Command::Export {
                    detail: true,
                    format: Format::Png,
                    ..
                }
            })
        ));
    }

    #[test]
    fn png_quality_defaults_to_8k_for_area_and_overview() {
        for mode in [vec!["--area", "1,1"], vec!["--overview"]] {
            let mut args = vec!["arda", "export", "--world", "saved", "--out", "exports"];
            args.extend(mode);
            let Cli {
                command: Command::Export {
                    detail, quality, ..
                },
            } = Cli::try_parse_from(args).unwrap()
            else {
                panic!("expected export")
            };
            assert_eq!(ImageOptions { detail, quality }.quality().pixels(), 8192);
        }
    }

    #[test]
    fn quality_accepts_pixels_and_k_suffixes_and_rejects_invalid_values() {
        for (text, pixels) in [("512", 512), ("513", 513), ("8k", 8192), ("32k", 32768)] {
            let cli = Cli::try_parse_from([
                "arda",
                "export",
                "--world",
                "saved",
                "--out",
                "exports",
                "--quality",
                text,
            ])
            .unwrap();
            let Command::Export {
                quality: Some(quality),
                ..
            } = cli.command
            else {
                panic!("missing quality")
            };
            assert_eq!(quality.pixels(), pixels);
        }
        for text in ["511", "32769", "33k", "bogus"] {
            assert!(Cli::try_parse_from([
                "arda",
                "export",
                "--world",
                "saved",
                "--out",
                "exports",
                "--quality",
                text,
            ])
            .is_err());
        }
    }

    #[test]
    fn explicit_quality_rejects_non_png_modes_before_reading_the_world() {
        let image = ImageOptions {
            detail: false,
            quality: Some(ImageQuality::DEFAULT),
        };
        for (format, overview, block) in [
            (Format::Json, false, None),
            (Format::Json, true, None),
            (Format::Png, false, Some("0,0,0,0")),
        ] {
            let result = run_export(
                Path::new("missing-world"),
                "0,0",
                format,
                overview,
                block,
                Path::new("unused-export"),
                image,
            );
            assert!(result.is_err_and(
                |e| e.to_string() == "--quality is valid only for area or overview PNG exports"
            ));
        }
        assert!(ImageOptions::default()
            .validate(Format::Json, false, None)
            .is_ok());
    }

    #[test]
    fn explicit_quality_conflicts_with_legacy_resolution_flags_and_blocks() {
        for extra in [vec!["--detail"], vec!["--block", "0,0,0,0"]] {
            let mut args = vec![
                "arda",
                "export",
                "--world",
                "saved",
                "--out",
                "exports",
                "--quality",
                "8k",
            ];
            args.extend(extra);
            assert!(Cli::try_parse_from(args).is_err());
        }
        assert!(Cli::try_parse_from([
            "arda",
            "preview",
            "--seed",
            "42",
            "--out",
            "exports",
            "--quality",
            "8k",
            "--px",
            "16",
        ])
        .is_err());
    }
}
