//! This file is the `zoom_terrain` module of the `zoom_continuity` test
//! target (`mod zoom_terrain;` in `tests/zoom_continuity.rs`), not a
//! target of its own. Run its gates with
//! `cargo test --release --test zoom_continuity -- --ignored`.
//!
//! Zoom continuity beyond water and worked land (review 3.6-10; logic/17
//! §continuity): over a spread of natural cells of the MICRO seed 42 world
//! (lowland, upland and steep), the mid-zoom relief's ground height at
//! z9 matches the composed tactical elevation square by square, and the
//! relief's forest matches the tactical woods cell by cell.

use super::{open, world};
use arda_midzoom::{refine_tile, WorldTerrain};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

/// Squares per cell side.
const N: i64 = 64;
/// A square, in micrometres.
const SQUARE_UM: i64 = 1_562_500;

async fn squares(state: &Arc<arda_server::AppState>, gx: i64, gy: i64) -> Option<Vec<Value>> {
    let request = Request::get(format!("/v1/tactical/cell/{gx}/{gy}"))
        .body(Body::empty())
        .unwrap();
    let response = arda_server::router(Arc::clone(state))
        .oneshot(request)
        .await
        .unwrap();
    if response.status() != StatusCode::OK {
        return None;
    }
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let block: Value = serde_json::from_slice(&body).unwrap();
    Some(block["layout"]["squares"].as_array().unwrap().clone())
}

fn wooded(ground: &str) -> bool {
    matches!(ground, "forest_floor" | "leaf_litter" | "moss")
}

/// Natural land cells (no settlement, road or land use), spread over the
/// world, with their world cover and slope.
fn natural(dir: &std::path::Path) -> Vec<((i64, i64), arda_core::Cover, u16)> {
    let world = arda::World::load(dir).unwrap();
    let soc = dir.join("society");
    let (w, _, codes, owners) =
        arda_settle::output::read_landuse(&soc.join("landuse.bin")).unwrap();
    let m = world.manifest();
    let mut out = Vec::new();
    for ay in 0..m.areas_high {
        for ax in 0..m.areas_wide {
            let area = world.read_area(ax, ay).unwrap();
            for y in (8..512_u16).step_by(48) {
                for x in (8..512_u16).step_by(48) {
                    let Ok(c) = area.cell(x, y) else { continue };
                    if c.terrain != arda_core::TerrainKind::Land {
                        continue;
                    }
                    let (gx, gy) = (
                        i64::from(ax) * 512 + i64::from(x),
                        i64::from(ay) * 512 + i64::from(y),
                    );
                    let i = usize::try_from(gy).unwrap() * w + usize::try_from(gx).unwrap();
                    if codes.get(i).copied().unwrap_or(1) != 0
                        || owners.get(i).copied().unwrap_or(1) != 0
                    {
                        continue;
                    }
                    out.push(((gx, gy), c.cover, c.slope_milli_deg));
                }
            }
        }
    }
    out
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "release gate: needs a settled MICRO seed 42 world (cargo test --release --test zoom_continuity -- --ignored)"]
async fn relief_and_tactical_heights_and_woods_agree() {
    let dir = world("ARDA_ZOOM_WORLD", "42", 42, arda::GenerateConfig::MICRO, 9);
    let state = open(&dir);
    let terrain = WorldTerrain::new(Arc::new(arda::World::load(&dir).unwrap())).unwrap();
    let mut cells = natural(&dir);
    cells.sort_by_key(|c| c.0);
    let step = cells.len().div_ceil(48).max(1);
    let picks: Vec<_> = cells.into_iter().step_by(step).collect();
    let (mut errs, mut biases, mut steep) = (Vec::new(), Vec::new(), Vec::new());
    let (mut agree, mut judged) = (0, 0);
    for ((gx, gy), cover, slope) in picks {
        let Some(sq) = squares(&state, gx, gy).await else {
            continue;
        };
        #[allow(clippy::cast_precision_loss)]
        let (x0, y0) = ((gx * N) as f64 * 1.5625, (gy * N) as f64 * 1.5625);
        let tile = refine_tile(&terrain, x0, y0, 100.0, 1.5625).unwrap();
        let (mut sum, mut abs, mut n) = (0.0, 0.0, 0_u32);
        let mut woods = 0_usize;
        for (k, s) in sq.iter().enumerate() {
            let (x, y) = (i64::try_from(k).unwrap() % N, i64::try_from(k).unwrap() / N);
            woods += usize::from(wooded(s["ground"].as_str().unwrap_or("")));
            if s["water_depth_ft"].as_u64().unwrap_or(0) > 0 {
                continue;
            }
            let lx = ((gx * N + x) * SQUARE_UM + SQUARE_UM / 2) - arda_core::FINE_FRAME_OFFSET_UM;
            let ly = ((gy * N + y) * SQUARE_UM + SQUARE_UM / 2) - arda_core::FINE_FRAME_OFFSET_UM;
            let Some(p) = tile.sample(lx, ly) else {
                continue;
            };
            #[allow(clippy::cast_precision_loss)]
            let relief_m = p.height_mm as f64 / 1000.0;
            #[allow(clippy::cast_precision_loss)]
            let tactical_m = s["elevation_ft"].as_f64().unwrap_or(0.0) * 0.3048;
            let d = relief_m - tactical_m;
            sum += d;
            abs += d.abs();
            n += 1;
        }
        if n > 0 {
            let n = f64::from(n);
            errs.push(abs / n);
            biases.push(sum / n);
            if slope > 30_000 {
                steep.push(sum / n);
            }
            println!(
                "cell {gx},{gy} slope {:.1}°: mean |dz| {:.2} m, bias {:+.2} m",
                f64::from(slope) / 1000.0,
                abs / n,
                sum / n
            );
        }
        #[allow(clippy::cast_precision_loss)]
        let share = woods as f64 / sq.len().max(1) as f64;
        let forest = cover == arda_core::Cover::Forest;
        if forest || matches!(cover, arda_core::Cover::Grass | arda_core::Cover::Scrub) {
            judged += 1;
            agree += usize::from(forest == (share >= 0.3));
            println!("cell {gx},{gy}: world {cover:?}, tactical woods {share:.2}");
        }
    }
    errs.sort_by(f64::total_cmp);
    biases.sort_by(f64::total_cmp);
    let median = errs[errs.len() / 2];
    let p90 = errs[errs.len() * 9 / 10];
    let bias = biases[biases.len() / 2];
    println!(
        "{} cells: median |dz| {median:.2} m, p90 {p90:.2} m, median bias {bias:+.2} m; woods agree {agree} of {judged}",
        errs.len()
    );
    assert!(errs.len() >= 20, "too few natural cells sampled");
    // The tactical map rounds to 5 ft (1.52 m) and shapes banks and
    // ledges the relief does not: within about two steps, unbiased.
    assert!(median <= 1.5, "median |dz| {median:.2} m");
    assert!(p90 <= 3.0, "p90 |dz| {p90:.2} m");
    assert!(bias.abs() <= 0.8, "median bias {bias:+.2} m");
    assert!(agree * 10 >= judged * 8, "woods agree {agree} of {judged}");
    // Above 30° the tactical map cuts ledges and cliff faces into the
    // slope, so it stands a little under the relief's smooth surface
    // (+2.2 to +2.7 m on recipe 9): bounded, not yet zero (open-items).
    let worst = steep.iter().copied().fold(0.0_f64, f64::max);
    println!("steep cells: {}, worst bias {worst:+.2} m", steep.len());
    assert!(worst <= 3.5, "a steep cell's bias is {worst:+.2} m");
}
