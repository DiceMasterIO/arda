//! `arda-refine`: render, export and survey tactical blocks.

use arda_refine::{refine_window, CellKey, RefineError, Source, WorldSource};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(
    name = "arda-refine",
    about = "Tactical terrain refinement for arda worlds"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Render a block (or a window of blocks) to a debug PNG, and optionally
    /// write its layout JSON and rules sidecar.
    Render {
        /// World directory.
        #[arg(long)]
        world: PathBuf,
        /// Global cell `gx,gy` (the window's top-left cell).
        #[arg(long)]
        cell: String,
        /// Window of blocks, e.g. `3x3`.
        #[arg(long)]
        window: Option<String>,
        /// Output PNG.
        #[arg(long)]
        out: PathBuf,
        /// Also write `<stem>.layout.json` and `<stem>.rules.json` next to the PNG.
        #[arg(long)]
        json: bool,
        /// Pixels per square.
        #[arg(long, default_value_t = 12)]
        scale: u32,
    },
    /// Print WFC statistics for a block.
    Stats {
        /// World directory.
        #[arg(long)]
        world: PathBuf,
        /// Global cell `gx,gy`.
        #[arg(long)]
        cell: String,
    },
    /// List candidate cells by type.
    Survey {
        /// World directory.
        #[arg(long)]
        world: PathBuf,
    },
}

fn parse_pair(s: &str, sep: char) -> Result<(i64, i64), String> {
    let (a, b) = s
        .split_once(sep)
        .ok_or_else(|| format!("expected A{sep}B, got {s}"))?;
    let a = a.trim().parse().map_err(|e| format!("{e}"))?;
    let b = b.trim().parse().map_err(|e| format!("{e}"))?;
    Ok((a, b))
}

fn write(path: &PathBuf, bytes: &[u8]) -> Result<(), RefineError> {
    std::fs::write(path, bytes).map_err(|source| RefineError::Io {
        path: path.display().to_string(),
        source,
    })
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("arda-refine: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Render {
            world,
            cell,
            window,
            out,
            json,
            scale,
        } => {
            let world = arda::World::load(&world)?;
            let src = WorldSource::new(&world)?;
            let (gx, gy) = parse_pair(&cell, ',')?;
            let (bw, bh) = match window {
                Some(w) => parse_pair(&w, 'x')?,
                None => (1, 1),
            };
            let (w, h) = (u32::try_from(bw * 64)?, u32::try_from(bh * 64)?);
            // Warm the area caches so the timing is the refinement alone.
            let _ = src.cell(CellKey::new(gx, gy))?;
            let t = Instant::now();
            let map = refine_window(&src, gx * 64, gy * 64, w, h)?;
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            write(&out, &arda_refine::render::render_png(&map, scale)?)?;
            if json {
                let stem = out.with_extension("");
                write(
                    &stem.with_extension("layout.json"),
                    map.layout_json()?.as_bytes(),
                )?;
                write(
                    &stem.with_extension("rules.json"),
                    map.rules_json()?.as_bytes(),
                )?;
                write(
                    &stem.with_extension("meta.json"),
                    map.meta_json()?.as_bytes(),
                )?;
            }
            println!(
                "{}: {}x{} squares, {} placements, relaxed {}, {ms:.1} ms",
                out.display(),
                w,
                h,
                map.layout.placements.len(),
                map.meta.relaxed
            );
        }
        Cmd::Stats { world, cell } => {
            let world = arda::World::load(&world)?;
            let src = WorldSource::new(&world)?;
            let (gx, gy) = parse_pair(&cell, ',')?;
            let t = Instant::now();
            let _ = arda_refine::refine(&src, CellKey::new(gx, gy))?;
            let cold = t.elapsed().as_secs_f64() * 1000.0;
            let t = Instant::now();
            let b = arda_refine::refine(&src, CellKey::new(gx, gy))?;
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            let t = Instant::now();
            let _ = arda_refine::refine_window(&src, gx * 64 + 16, gy * 64 + 16, 32, 32)?;
            let quarter = t.elapsed().as_secs_f64() * 1000.0;
            println!(
                "cold (loads caches) {cold:.1} ms, warm {ms:.1} ms, quarter block {quarter:.1} ms"
            );
            let mut counts = std::collections::BTreeMap::new();
            for g in &b.ground {
                *counts.entry(*g).or_insert(0) += 1;
            }
            println!(
                "cell {gx},{gy}: relaxed {} attempts {} repairs {} items {} {ms:.1} ms",
                b.relaxed,
                b.attempts,
                b.repairs,
                b.items.len()
            );
            println!("{counts:?}");
            let mut assets = std::collections::BTreeMap::new();
            for it in &b.items {
                *assets
                    .entry((it.asset, format!("{:?}", it.kind)))
                    .or_insert(0) += 1;
            }
            println!("{assets:?}");
            let ctx = arda_refine::context::Ctx::gather(&src, CellKey::new(gx, gy))?;
            let pieces = arda_refine::rivers::pieces(&ctx);
            let mut phys =
                arda_refine::terrain::physical(&ctx, &pieces, gx * 64 - 8, gy * 64 - 8, 80);
            arda_refine::terrain::slopes(&mut phys);
            let sh = arda_refine::shape::shapes(&phys, gx * 64, gy * 64, 64);
            let mut hist = [0_u32; 11];
            let mut talus = 0;
            for f in &sh.data {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let b = ((f.hollow + 1.0) * 5.0).round().clamp(0.0, 10.0) as usize;
                hist[b] += 1;
                talus += u32::from(f.talus > 0.3);
            }
            println!("hollow -1..1 by 0.2: {hist:?}; talus squares {talus}");
            println!("{:?}", src.cell(CellKey::new(gx, gy))?);
            println!("tiles {}", arda_refine::tiles::TILE_COUNT);
        }
        Cmd::Survey { world } => {
            let world = arda::World::load(&world)?;
            let src = WorldSource::new(&world)?;
            arda_refine_survey(&src)?;
        }
    }
    Ok(())
}

fn arda_refine_survey(src: &WorldSource) -> Result<(), RefineError> {
    use arda::{Cover, TerrainKind};
    let (w, h) = src.cells_wide_high();
    let names = [
        "river",
        "forest_edge",
        "lake_shore",
        "sea_coast",
        "scree",
        "marsh",
        "high_meadow",
        "pasture",
        "stream_valley",
    ];
    let mut found: Vec<Vec<(i64, i64)>> = vec![Vec::new(); names.len()];
    for y in (2..h - 2).step_by(2) {
        for x in (2..w - 2).step_by(2) {
            let k = CellKey::new(x, y);
            let c = src.cell(k)?;
            let mut n = Vec::new();
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx != 0 || dy != 0 {
                        n.push(src.cell(k.offset(dx, dy))?);
                    }
                }
            }
            let count = |f: &dyn Fn(&arda::Cell) -> bool| n.iter().filter(|m| f(m)).count();
            let slope = f64::from(c.slope_milli_deg) / 1000.0;
            let temp = f64::from(c.temperature.raw()) / 100.0;
            let lake = count(&|m| m.terrain == TerrainKind::Lake);
            let sea = count(&|m| m.terrain == TerrainKind::Sea);
            let fd = |m: &arda::Cell| i32::from(m.forest_density);
            let dense = n.iter().any(|m| fd(m) > 160);
            let open = n
                .iter()
                .any(|m| m.terrain == TerrainKind::Land && fd(m) < 40);
            let grass = n
                .iter()
                .filter(|m| m.cover == Cover::Grass && fd(m) < 60)
                .count();
            let edge = (dense && open && (50..200).contains(&fd(&c)))
                || (c.cover == Cover::Forest && fd(&c) > 120 && (2..=5).contains(&grass));

            let land = c.terrain == TerrainKind::Land;
            let tags = [
                land && c.watercourse_width_dm >= 60
                    && slope < 6.0
                    && c.cover != Cover::Marsh
                    && sea + lake == 0,
                land && edge && slope < 12.0,
                (1..=7).contains(&lake) && slope < 15.0,
                (1..=7).contains(&sea) && slope < 15.0,
                land && c.cover == Cover::Rock && (28.0..42.0).contains(&slope),
                land && c.cover == Cover::Marsh && count(&|m| m.cover == Cover::Marsh) >= 5,
                land && c.cover == Cover::Grass && temp < 5.0 && fd(&c) < 60,
                land && c.cover == Cover::Grass && temp > 8.0 && slope < 4.0 && fd(&c) < 40,
                land && (1..=2).contains(&c.watercourse_order) && slope > 6.0,
            ];
            for (i, t) in tags.iter().enumerate() {
                if *t && found[i].len() < 40 {
                    found[i].push((x, y));
                }
            }
        }
    }
    for (name, cells) in names.iter().zip(found) {
        let mut scored = Vec::new();
        for (x, y) in cells {
            let b = arda_refine::refine(src, CellKey::new(x, y))?;
            let wet = b.depth_ft.iter().filter(|&&d| d > 0).count() * 100 / 4096;
            let trees = b
                .items
                .iter()
                .filter(|i| i.kind == arda_refine::scatter::Kind::TreeLarge)
                .count();
            scored.push(format!("({x},{y}) wet {wet}% trees {trees}"));
        }
        println!("{name}: {}", scored.join("; "));
    }
    Ok(())
}
