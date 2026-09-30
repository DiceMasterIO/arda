//! Builds synthetic windows — a highway stone bridge, a track ford, a river
//! ferry and a switchback mountain road — overlays them with
//! `arda_ways::apply_ways`, degrades missing assets to the placeholder
//! library's nearest ones, and renders `out/ways/*.png` with the layout
//! JSON, the sidecar JSON and a report of fallbacks.
//!
//! Run with `cargo run -p arda-ways --example crossings --release`.

mod scenes;

use arda_tactical::layout::{AssetRef, Placement};
use arda_tactical::noise::hash2;
use arda_tactical::{render, Library, RenderOptions, TacticalLayout};
use arda_ways::fallback::{self, Log};
use arda_ways::input::{m_to_ft, step5};
use arda_ways::plan::ChannelPlan;
use arda_ways::{apply_ways, FnTerrain, WaysError, SQUARE_M};
use std::path::{Path, PathBuf};

const SEED: u64 = 37;

fn write(path: &Path, text: &str) -> Result<(), WaysError> {
    std::fs::write(path, text).map_err(|source| WaysError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[allow(clippy::cast_precision_loss)] // 24-bit value
fn unit(h: u64) -> f64 {
    (h >> 40) as f64 / f64::from(1u32 << 24)
}

/// The window a previous stage would hand over: ground, elevation and
/// scattered vegetation, kept off the water.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)] // small indices
fn base_layout(s: &scenes::Scene, channels: &[ChannelPlan]) -> TacticalLayout {
    let mut l = TacticalLayout::new(s.name, s.width, s.height, s.ground);
    let (ox, oy) = (s.origin_m[0] / SQUARE_M, s.origin_m[1] / SQUARE_M);
    let wet = |x: f64, y: f64, pad: f64| {
        channels
            .iter()
            .any(|c| c.dist([x, y], c.width_m / 2.0 + pad).is_some())
    };
    let mountain = s.ground == "scrub";
    for y in 0..s.height {
        for x in 0..s.width {
            let (wx, wy) = (
                (ox + f64::from(x) + 0.5) * SQUARE_M,
                (oy + f64::from(y) + 0.5) * SQUARE_M,
            );
            let h = hash2(SEED, i64::from(x) + ox as i64, i64::from(y) + oy as i64);
            if let Some(sq) = l.square_mut(x, y) {
                // The terrain stage's 5-ft contour steps (goal 43).
                sq.elevation_ft = step5(m_to_ft((s.height_fn)(wx, wy)));
                if mountain && unit(h) < 0.08 {
                    "scree".clone_into(&mut sq.ground);
                }
            }
            let r = unit(h.rotate_left(17));
            if wet(wx, wy, 2.0 * SQUARE_M) || r > 0.05 {
                continue;
            }
            let id = match (mountain, (r * 100.0) as u32) {
                (true, 0..=1) => "veg.tree_pine",
                (true, 2) => "veg.rock_large",
                (true, _) => "veg.rock_small",
                (false, 0) => "veg.tree_oak",
                (false, 1) => "veg.tree_elm",
                (false, 2) => "veg.tree_birch",
                (false, 3) => "veg.bush_flowering",
                (false, _) => "veg.bush",
            };
            l.placements.push(Placement {
                asset: AssetRef::Id(id.into()),
                x: x as f32 + 0.5,
                y: y as f32 + 0.5,
                rotation: ((h >> 8) % 4) as u16 * 90,
                mirror: false,
            });
        }
    }
    l
}

fn run() -> Result<(), WaysError> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lib = Library::load(&root.join("assets/tactical/placeholder"))?;
    let out = root.join("out/ways");
    std::fs::create_dir_all(&out).map_err(|source| WaysError::Io {
        path: out.clone(),
        source,
    })?;
    let mut all = Log::default();
    let mut reports = Vec::new();
    for s in scenes::all() {
        let terrain = FnTerrain {
            height: s.height_fn,
            channels: s.channels.clone(),
        };
        let chans: Vec<ChannelPlan> = s.channels.iter().map(ChannelPlan::new).collect();
        let mut layout = base_layout(&s, &chans);
        let ways = apply_ways(
            &mut layout,
            s.origin_m,
            &s.roads,
            &s.crossings,
            &terrain,
            SEED,
        )?;
        write(
            &out.join(format!("{}.layout.json", s.name)),
            &layout.to_json()?,
        )?;
        write(
            &out.join(format!("{}.sidecar.json", s.name)),
            &ways.sidecar.to_json()?,
        )?;
        let log = fallback::degrade(&mut layout, &lib);
        all.merge(&log);
        let opts = RenderOptions {
            ppsq: 64,
            grid: true,
            lighting: true,
        };
        let img = render(&layout, &lib, SEED, &opts)?;
        let png = out.join(format!("{}.png", s.name));
        img.write_png(&png)?;
        println!(
            "{}: {} ways, {} crossings, {} houses, {} switchback pieces -> {}",
            s.name,
            ways.report.ways,
            ways.report.crossings.len(),
            ways.report.houses.len(),
            ways.report.switchbacks,
            png.display()
        );
        reports.push(serde_json::json!({ "scene": s.name, "report": ways.report }));
    }
    let fallbacks = all.records();
    for f in &fallbacks {
        println!(
            "fallback {:?}: {} -> {} ({}x)",
            f.kind, f.wanted, f.used, f.count
        );
    }
    let doc = serde_json::json!({ "scenes": reports, "fallbacks": fallbacks });
    write(
        &out.join("report.json"),
        &(serde_json::to_string_pretty(&doc)? + "\n"),
    )?;
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("crossings: {e}");
        std::process::exit(1);
    }
}
