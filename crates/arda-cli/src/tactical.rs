//! `arda tactical …`: the tactical asset library and map compositor
//! (goals 58–63, `crates/arda-tactical`).

use anyhow::{bail, Context, Result};
use arda_tactical::{
    layouts, library, placeholders, render_with, Library, RenderOptions, Style, TacticalLayout,
    Thresholds,
};
use clap::Subcommand;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub(crate) enum TacticalCommand {
    /// Validate an asset library directory (its `catalog.json` and images).
    Validate {
        /// Library directory.
        dir: PathBuf,
    },
    /// Generate the placeholder library.
    Placeholders {
        /// Directory to write into.
        #[arg(long, default_value = "assets/tactical/placeholder")]
        out: PathBuf,
        /// Generation seed.
        #[arg(long, default_value_t = placeholders::DEFAULT_SEED)]
        seed: u64,
    },
    /// Render a layout (built-in name, `all`, or a layout JSON file) to PNG.
    Render {
        /// A built-in layout (`riverside`, `stone_warehouse`, `timber_house`,
        /// `wall_junctions`, `forest_glade`, `mountain_scree`, `marsh`,
        /// `farm_field`, or the 64 × 64 `benchmark`), `all`, or a path to a
        /// layout `.json` file.
        #[arg(long)]
        layout: String,
        /// Asset library directory.
        #[arg(long, default_value = "assets/tactical/placeholder")]
        library: PathBuf,
        /// Output pixels per square.
        #[arg(long, default_value_t = 128)]
        ppsq: u32,
        /// Output PNG; with `--layout all`, a directory.
        #[arg(long, default_value = "out/tactical")]
        out: PathBuf,
        /// Render seed (texture variants, blends, tag queries).
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Draw the square grid.
        #[arg(long)]
        grid: bool,
        /// Skip the lighting pass (and the grade).
        #[arg(long)]
        no_lighting: bool,
        /// Skip the warm colour grade.
        #[arg(long)]
        no_grade: bool,
    },
}

pub(crate) fn run(command: TacticalCommand) -> Result<()> {
    match command {
        TacticalCommand::Validate { dir } => validate(&dir),
        TacticalCommand::Placeholders { out, seed } => {
            let catalog = placeholders::write(&out, seed)?;
            println!(
                "wrote {} placeholder assets to {}",
                catalog.assets.len(),
                out.display()
            );
            validate(&out)
        }
        TacticalCommand::Render {
            layout,
            library,
            ppsq,
            out,
            seed,
            grid,
            no_lighting,
            no_grade,
        } => {
            let opts = RenderOptions {
                ppsq,
                grid,
                lighting: !no_lighting,
            };
            let style = Style { grade: !no_grade };
            let lib = Library::load(&library)
                .with_context(|| format!("loading {}", library.display()))?;
            if layout == "all" {
                for l in layouts::all() {
                    let suffix = if grid { "_grid" } else { "" };
                    render_one(
                        &l,
                        &lib,
                        seed,
                        (&opts, &style),
                        &out.join(format!("{}{suffix}.png", l.name)),
                    )?;
                }
                return Ok(());
            }
            let l = match layouts::by_name(&layout) {
                Some(l) => l,
                None => {
                    let text = std::fs::read_to_string(&layout).with_context(|| {
                        format!(
                            "`{layout}` is neither a built-in layout ({}) nor a readable file",
                            layouts::NAMES.join(", ")
                        )
                    })?;
                    TacticalLayout::from_json(&text)?
                }
            };
            let target = if out.extension().is_some() {
                out
            } else {
                out.join(format!("{}.png", l.name))
            };
            render_one(&l, &lib, seed, (&opts, &style), &target)
        }
    }
}

fn render_one(
    l: &TacticalLayout,
    lib: &Library,
    seed: u64,
    (opts, style): (&RenderOptions, &Style),
    path: &Path,
) -> Result<()> {
    let img = render_with(l, lib, seed, opts, style)?;
    img.write_png(path)?;
    println!(
        "{}: {}x{} squares -> {} ({}x{} px)",
        l.name,
        l.width,
        l.height,
        path.display(),
        img.width,
        img.height
    );
    Ok(())
}

fn validate(dir: &std::path::Path) -> Result<()> {
    let issues = library::check_dir(dir, &Thresholds::default())
        .with_context(|| format!("reading library {}", dir.display()))?;
    if issues.is_empty() {
        let catalog = library::read_catalog(dir)?;
        println!(
            "{}: ok ({} assets, library {} {}, {} px/square)",
            dir.display(),
            catalog.assets.len(),
            catalog.library,
            catalog.library_version,
            catalog.pixels_per_square
        );
        return Ok(());
    }
    for issue in &issues {
        eprintln!("{issue}");
    }
    bail!(
        "{} failed validation with {} issue(s)",
        dir.display(),
        issues.len()
    )
}
