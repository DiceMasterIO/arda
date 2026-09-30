//! `arda-town`: plan overviews, tactical block renders and plan exports for
//! the synthetic sites.

use arda_tactical::{render, Library, RenderOptions};
use arda_town::block::{self, fallback, Window};
use arda_town::function::BuildingFunction as F;
use arda_town::plan::{check, spec, TownPlan};
use arda_town::{samples, TownError};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "arda-town", about = "Town plans and tactical town blocks")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Draw a plan overview PNG.
    Render {
        /// Site name (aldermere, thornby, highcrag, saltwick).
        #[arg(long)]
        site: String,
        /// Output PNG.
        #[arg(long)]
        out: PathBuf,
        /// World seed.
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// Pixels per 5-ft square (0 picks one for a ~1600 px image).
        #[arg(long, default_value_t = 0)]
        px: u32,
    },
    /// Render a tactical block through `arda_tactical::render`.
    Block {
        /// Site name.
        #[arg(long)]
        site: String,
        /// `x,y,w,h` in squares relative to the plan grid origin.
        #[arg(long)]
        window: Option<String>,
        /// Centre a window on `market`, `inn`, `river`, `temple`, `gate`,
        /// `smithy` or `building:<id>` instead.
        #[arg(long)]
        focus: Option<String>,
        /// Window edge for `--focus`, squares.
        #[arg(long, default_value_t = 64)]
        size: i64,
        /// Output PNG.
        #[arg(long)]
        out: PathBuf,
        /// Library directory.
        #[arg(long, default_value = "assets/tactical/placeholder")]
        library: PathBuf,
        /// World seed.
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// Output pixels per square.
        #[arg(long, default_value_t = 48)]
        ppsq: u32,
        /// Draw the square grid.
        #[arg(long)]
        grid: bool,
        /// Also write the canonical block and sidecar as JSON here.
        #[arg(long)]
        json: Option<PathBuf>,
    },
    /// Print a plan summary and points of interest.
    Info {
        /// Site name.
        #[arg(long)]
        site: String,
        /// World seed.
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
    /// Write the plan JSON and the `BuildingSpec` records.
    Export {
        /// Site name.
        #[arg(long)]
        site: String,
        /// Output directory.
        #[arg(long)]
        out: PathBuf,
        /// World seed.
        #[arg(long, default_value_t = 42)]
        seed: u64,
    },
}

fn plan_for(site: &str, seed: u64) -> Result<TownPlan, TownError> {
    let (s, t) = samples::by_name(site).ok_or_else(|| TownError::UnknownSite(site.to_string()))?;
    arda_town::generate(&s, &t, seed)
}

fn write(path: &Path, text: &str) -> Result<(), TownError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|source| TownError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(path, text).map_err(|source| TownError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn parse_window(plan: &TownPlan, s: &str) -> Result<Window, TownError> {
    let v: Vec<i64> = s.split(',').filter_map(|p| p.trim().parse().ok()).collect();
    let [x, y, w, h] = v[..] else {
        return Err(TownError::Window(format!("`{s}` is not x,y,w,h")));
    };
    let (ox, oy) = plan.origin();
    Ok(Window {
        x: ox + x,
        y: oy + y,
        w,
        h,
    })
}

/// A window of `size` squares centred on a point of interest.
fn focus_window(plan: &TownPlan, what: &str, size: i64) -> Result<Window, TownError> {
    let centre = |x: i64, y: i64| Window {
        x: x - size / 2,
        y: y - size / 2,
        w: size,
        h: size,
    };
    let by = |f: F| plan.buildings.iter().find(|b| b.function == f);
    let mid = |r: &arda_town::plan::grid::SquareRect| ((r.x0 + r.x1) / 2, (r.y0 + r.y1) / 2);
    let pick = match what {
        "market" => Some(arda_town::plan::square::square_of(plan.focal.market)),
        "river" => Some(arda_town::plan::square::square_of(plan.focal.feature)),
        "inn" => by(F::Inn).or_else(|| by(F::Tavern)).map(|b| mid(&b.rect)),
        "temple" => by(F::Temple)
            .or_else(|| by(F::Shrine))
            .map(|b| mid(&b.rect)),
        "smithy" => by(F::Smithy).map(|b| mid(&b.rect)),
        "gate" => plan
            .wall
            .as_ref()
            .and_then(|w| w.gates.first())
            .map(|g| arda_town::plan::square::square_of(g.point)),
        other => other
            .strip_prefix("building:")
            .and_then(|n| n.parse().ok())
            .and_then(|n| plan.building(arda_town::plan::BuildingId(n)))
            .map(|b| mid(&b.rect)),
    };
    pick.map(|(x, y)| centre(x, y))
        .ok_or_else(|| TownError::Window(format!("nothing to focus on for `{what}`")))
}

fn run(cli: Cli) -> Result<(), TownError> {
    match cli.cmd {
        Cmd::Render {
            site,
            out,
            seed,
            px,
        } => {
            let plan = plan_for(&site, seed)?;
            println!("{}", check::summary(&plan));
            if let Some(dir) = out.parent() {
                std::fs::create_dir_all(dir).map_err(|source| TownError::Io {
                    path: dir.to_path_buf(),
                    source,
                })?;
            }
            let px = if px == 0 {
                arda_town::render::auto_px(&plan, 1600)
            } else {
                px
            };
            arda_town::render::plan_png(&plan, px).write_png(&out)?;
            println!("wrote {}", out.display());
        }
        Cmd::Block {
            site,
            window,
            focus,
            size,
            out,
            library,
            seed,
            ppsq,
            grid,
            json,
        } => {
            let plan = plan_for(&site, seed)?;
            let win = match (window, focus) {
                (Some(w), _) => parse_window(&plan, &w)?,
                (None, Some(f)) => focus_window(&plan, &f, size)?,
                (None, None) => focus_window(&plan, "market", size)?,
            };
            let blk = block::generate(&plan, win)?;
            let lib = Library::load(&library)?;
            let res = fallback::resolve(&blk, &lib, plan.seed);
            let opts = RenderOptions {
                ppsq,
                grid,
                lighting: true,
            };
            let img = render(&res.layout, &lib, plan.seed, &opts)?;
            if let Some(dir) = out.parent() {
                std::fs::create_dir_all(dir).map_err(|source| TownError::Io {
                    path: dir.to_path_buf(),
                    source,
                })?;
            }
            img.write_png(&out)?;
            let (ox, oy) = plan.origin();
            println!(
                "block {},{} {}x{} (plan-relative {},{}): {} buildings, {} walls, {} props, {} lights",
                win.x, win.y, win.w, win.h, win.x - ox, win.y - oy,
                blk.buildings.len(), blk.layout.walls.len(), blk.layout.placements.len(), blk.layout.lights.len()
            );
            for f in &res.fallbacks {
                println!(
                    "fallback {:?} {} -> {} ×{}",
                    f.kind, f.wanted, f.used, f.count
                );
            }
            if let Some(j) = json {
                write(&j, &blk.to_json()?)?;
            }
            println!("wrote {}", out.display());
        }
        Cmd::Info { site, seed } => {
            let plan = plan_for(&site, seed)?;
            println!("{}", check::summary(&plan));
            let (ox, oy) = plan.origin();
            let (w, h) = plan.size();
            println!("grid origin {ox},{oy} size {w}x{h} squares");
            for (f, n) in check::built_mix(&plan) {
                println!("  {:<12} {n}", f.key());
            }
            for n in &plan.notes {
                println!("note: {n}");
            }
        }
        Cmd::Export { site, out, seed } => {
            let plan = plan_for(&site, seed)?;
            write(&out.join(format!("{site}-plan.json")), &plan.to_json()?)?;
            let specs = serde_json::to_string_pretty(&spec::specs(&plan))?;
            write(&out.join(format!("{site}-buildings.json")), &specs)?;
            println!("wrote {}", out.display());
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("arda-town: {e}");
            ExitCode::FAILURE
        }
    }
}
