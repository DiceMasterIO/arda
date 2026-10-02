//! Zoom continuity of water (goal 49; logic/17 §water): rivers, coasts and
//! inlets sit in the same squares on the mid-zoom relief at z9 (one pixel
//! per square on MICRO) and on the tactical map that arda-refine and
//! arda-blocks compose, because both draw from one water geometry.
//!
//! Run in the release gate:
//! `cargo test --release --test zoom_continuity -- --ignored --nocapture`.
//! Set `ARDA_ZOOM_WORLD` to a MICRO seed 42 world that has been settled
//! and given a society, so the run reuses it; otherwise the test generates
//! one. `ARDA_ZOOM_ARID_WORLD` does the same for the recipe-7 arid world
//! (MICRO seed 74, `--recipe 7 --latitude 15,35`).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

mod zoom_land;

use arda_core::water::PanKind;
use arda_midzoom::{water_mask, Pyramid, ReliefWorld};
use arda_server::{router, AppState, ServerConfig};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tower::ServiceExt;

/// Rivers (a town crossing, a wide trunk, upland streams), the sea inlet
/// at 519,1377 (the "lake" of the hand-off review; MICRO 42 holds no lake
/// cells), coasts and a dry control.
const CELLS: [(i64, i64); 11] = [
    (465, 1166),
    (486, 1168),
    (552, 1149),
    (882, 1184),
    (519, 1377),
    (443, 1054),
    (464, 1444),
    (954, 1312),
    (457, 1390),
    (690, 1386),
    (503, 1292),
];

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The world in `$var`, or one generated (fine `recipe`, `config`),
/// settled and given a society under `target/`.
fn world(var: &str, tag: &str, seed: u64, config: arda::GenerateConfig, recipe: u16) -> PathBuf {
    if let Some(dir) = std::env::var_os(var) {
        return PathBuf::from(dir);
    }
    // Tests sharing a world (water and land on MICRO 42) build it once:
    // concurrent builds into one directory would race.
    static BUILT: std::sync::Mutex<BTreeSet<String>> = std::sync::Mutex::new(BTreeSet::new());
    let mut built = BUILT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = workspace().join(format!(
        "target/zoom-continuity-{tag}-{}",
        std::process::id()
    ));
    if built.contains(tag) {
        return dir;
    }
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
    arda::generate_from_fine_recipe(
        seed,
        config,
        &dir,
        arda::FineDeliveryLimits::default(),
        recipe,
    )
    .expect("generating the MICRO world");
    arda_settle::generate(&dir, arda_settle::grid::MEMORY_BUDGET).expect("settle");
    arda_people::build(&dir).expect("society build");
    built.insert(tag.to_string());
    dir
}

/// The tactical block's squares of cell `(gx, gy)`.
async fn tactical_squares(state: &Arc<AppState>, gx: i64, gy: i64) -> Vec<Value> {
    let request = Request::get(format!("/v1/tactical/cell/{gx}/{gy}"))
        .body(Body::empty())
        .unwrap();
    let response = router(Arc::clone(state)).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK, "cell {gx},{gy}");
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let block: Value = serde_json::from_slice(&body).unwrap();
    block["layout"]["squares"].as_array().unwrap().clone()
}

fn is_water(square: &Value) -> bool {
    square["water_depth_ft"].as_u64().unwrap_or(0) > 0
}

fn open(dir: &Path) -> Arc<AppState> {
    let mut config = ServerConfig::new(dir.to_path_buf());
    config.library = workspace().join("assets/tactical/placeholder");
    Arc::new(AppState::open(&config).unwrap())
}

/// Compares relief z9 water with tactical water cell by cell (each cell's
/// IoU at least 0.9) and returns the pooled `(intersection, union)`.
async fn agreement(dir: &Path, cells: &[(i64, i64)]) -> (usize, usize) {
    let state = open(dir);
    let loaded = Arc::new(arda::World::load(dir).unwrap());
    let m = loaded.manifest();
    let pyramid = Pyramid {
        max_zoom: 4,
        areas_wide: m.areas_wide,
        areas_high: m.areas_high,
    };
    assert_eq!(pyramid.pixel_um(9), 1_562_500, "z9 is one pixel per square");
    let relief = ReliefWorld::new(loaded).unwrap();
    let (mut inter_all, mut union_all) = (0_usize, 0_usize);
    for &(gx, gy) in cells {
        let tactical: Vec<bool> = tactical_squares(&state, gx, gy)
            .await
            .iter()
            .map(is_water)
            .collect();
        let mask = water_mask(&relief, &pyramid, 9, (gx * 64, gy * 64), (64, 64)).unwrap();
        let inter = mask
            .iter()
            .zip(&tactical)
            .filter(|(a, b)| **a && **b)
            .count();
        let union = mask
            .iter()
            .zip(&tactical)
            .filter(|(a, b)| **a || **b)
            .count();
        let (r, t) = (
            mask.iter().filter(|&&w| w).count(),
            tactical.iter().filter(|&&w| w).count(),
        );
        println!("cell {gx},{gy}: relief {r} water squares, tactical {t}, shared {inter}");
        if union == 0 {
            continue;
        }
        #[allow(clippy::cast_precision_loss)]
        let iou = inter as f64 / union as f64;
        assert!(iou >= 0.9, "cell {gx},{gy}: water IoU {iou:.3}");
        inter_all += inter;
        union_all += union;
    }
    (inter_all, union_all)
}

fn overall(inter: usize, union: usize) {
    #[allow(clippy::cast_precision_loss)]
    let iou = inter as f64 / union as f64;
    println!("overall IoU {iou:.4} over {union} squares");
    assert!(iou >= 0.95, "overall water IoU {iou:.3}");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "release gate: needs a MICRO seed 42 world (cargo test --release --test zoom_continuity -- --ignored)"]
async fn relief_and_tactical_water_agree() {
    let dir = world("ARDA_ZOOM_WORLD", "42", 42, arda::GenerateConfig::MICRO, 6);
    let (inter, union) = agreement(&dir, &CELLS).await;
    assert!(union > 2_000, "the cells hold water ({union} squares)");
    overall(inter, union);
}

/// Global cells.
type Cells = Vec<(i64, i64)>;

/// Every `step`-th of `cells`, sorted, so the pick is spread and stable.
fn spread(mut cells: Vec<(i64, i64)>, want: usize) -> Vec<(i64, i64)> {
    cells.sort_unstable();
    let step = cells.len().div_ceil(want).max(1);
    cells.into_iter().step_by(step).collect()
}

/// Shore cells of saline (terminal) lakes, and crust and mudflat cells of
/// the playas, in global cells.
fn arid_cells(dir: &Path) -> (Cells, Cells, Cells) {
    let world = arda::World::load(dir).unwrap();
    let m = world.manifest();
    let (mut lake, mut crust, mut mud) = (BTreeSet::new(), Vec::new(), Vec::new());
    for ay in 0..m.areas_high {
        for ax in 0..m.areas_wide {
            let area = world.read_area(ax, ay).unwrap();
            let water = area.water().expect("recipe 7 stores water forms");
            let (ox, oy) = (i64::from(ax) * 512, i64::from(ay) * 512);
            for (form, cells) in water.lakes.iter().zip(area.lakes()) {
                if form.saline {
                    lake.extend(
                        cells
                            .cells
                            .iter()
                            .map(|c| (ox + i64::from(c.x()), oy + i64::from(c.y()))),
                    );
                }
            }
            for run in &water.pans {
                let list = match run.kind {
                    PanKind::SaltCrust => &mut crust,
                    PanKind::Mudflat => &mut mud,
                };
                list.extend(
                    (run.x0..run.x0 + run.len).map(|x| (ox + i64::from(x), oy + i64::from(run.y))),
                );
            }
        }
    }
    // Shore cells: lake cells with a dry four-neighbour.
    let shore = lake
        .iter()
        .copied()
        .filter(|&(x, y)| {
            [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|(dx, dy)| !lake.contains(&(x + dx, y + dy)))
        })
        .collect();
    (shore, crust, mud)
}

/// The recipe-7 arid world (goal 49 on goal 12's basins): relief and
/// tactical water agree along a terminal saline lake's shore and over its
/// salt pan, and the pan is dry `salt_crust` and `mudflat` ground on the
/// tactical map.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "release gate: needs the MICRO seed 74 recipe-7 world (cargo test --release --test zoom_continuity -- --ignored)"]
async fn relief_and_tactical_water_agree_on_an_arid_world() {
    let band = arda::LatitudeBand::new(15, 35);
    let config =
        arda::GenerateConfig::new(arda::GenerateConfig::MICRO.size_km(), band, 15).unwrap();
    let dir = world("ARDA_ZOOM_ARID_WORLD", "arid74", 74, config, 7);
    let (shore, crust, mud) = arid_cells(&dir);
    assert!(
        !shore.is_empty() && !crust.is_empty() && !mud.is_empty(),
        "seed 74 at 15-35° holds a saline lake on a salt pan"
    );
    let (crust, mud) = (spread(crust, 3), spread(mud, 3));
    let mut cells = spread(shore, 8);
    cells.extend(&crust);
    cells.extend(&mud);
    let (inter, union) = agreement(&dir, &cells).await;
    assert!(
        union > 1_000,
        "the lake shore holds water ({union} squares)"
    );
    overall(inter, union);
    // The pan is ground, not water, at tactical scale.
    let state = open(&dir);
    for (kind, picks) in [("salt_crust", &crust), ("mudflat", &mud)] {
        let mut found = 0;
        for &(gx, gy) in picks {
            for square in tactical_squares(&state, gx, gy).await {
                if square["ground"] == kind {
                    assert!(!is_water(&square), "{kind} at {gx},{gy} is dry");
                    found += 1;
                }
            }
        }
        println!("{kind}: {found} squares over {} cells", picks.len());
        assert!(found > 0, "the pan cells show {kind}");
    }
}
