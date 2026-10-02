//! Zoom continuity of worked land (goal 49; logic/17 §land): roads, field
//! walls and buildings sit in the same squares on the mid-zoom relief at
//! z9 (one pixel per square on MICRO) and on the composed tactical map,
//! because relief tiles draw the tactical layers' own geometry. Also:
//! relief tiles with land join pixel-exactly and a world without society
//! data renders exactly as before.

use super::{open, world};
use arda_midzoom::{render_window, LandWindow, Landscape, Pyramid, ReliefWorld};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;
use tower::ServiceExt;

/// The composed tactical block of cell `(gx, gy)`.
async fn block(state: &Arc<arda_server::AppState>, gx: i64, gy: i64) -> Value {
    let request = Request::get(format!("/v1/tactical/cell/{gx}/{gy}"))
        .body(Body::empty())
        .unwrap();
    let response = arda_server::router(Arc::clone(state))
        .oneshot(request)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "cell {gx},{gy}");
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

/// Cells to compare: settlement centres (buildings, streets, crofts and
/// open fields), stretches of road away from settlements, and the middle
/// of farmed land.
fn cells(dir: &Path) -> (super::Cells, super::Cells, super::Cells) {
    let society = dir.join("society");
    let read = |name: &str| -> Value {
        serde_json::from_slice(&std::fs::read(society.join(name)).unwrap()).unwrap()
    };
    let settlements = read("settlements.json");
    let list = settlements["settlements"].as_array().unwrap();
    let at = |s: &Value| (s["cell_x"].as_i64().unwrap(), s["cell_y"].as_i64().unwrap());
    let mut towns: Vec<(i64, i64)> = list
        .iter()
        .filter(|s| s["tier"] == "village" || s["tier"] == "town")
        .map(at)
        .collect();
    towns.sort_unstable();
    let towns: Vec<_> = towns
        .iter()
        .step_by(towns.len().div_ceil(4).max(1))
        .copied()
        .collect();
    let centres: Vec<(i64, i64)> = list.iter().map(at).collect();
    let far = |c: (i64, i64), d: i64| {
        centres
            .iter()
            .all(|s| (s.0 - c.0).abs() > d || (s.1 - c.1).abs() > d)
    };
    let roads = read("roads.json");
    let mut on_roads: Vec<(i64, i64)> = roads["roads"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["class"] == "road" || r["class"] == "highway" || r["class"] == "track")
        .flat_map(|r| r["segments"].as_array().unwrap().iter())
        .filter_map(|seg| {
            let pts = seg.as_array()?;
            let p = pts.get(pts.len() / 2)?.as_array()?;
            Some((p[0].as_i64()? / 100, p[1].as_i64()? / 100))
        })
        .filter(|&c| far(c, 8))
        .collect();
    on_roads.sort_unstable();
    on_roads.dedup();
    let on_roads = on_roads
        .iter()
        .step_by(on_roads.len().div_ceil(5).max(1))
        .copied()
        .collect();
    let (w, _, codes, _) = arda_settle::output::read_landuse(&society.join("landuse.bin")).unwrap();
    let field = |x: i64, y: i64| {
        usize::try_from(y * i64::try_from(w).unwrap() + x)
            .ok()
            .and_then(|i| codes.get(i))
            == Some(&2)
    };
    let mut farmed: Vec<(i64, i64)> = (0..codes.len())
        .map(|i| {
            let i = i64::try_from(i).unwrap();
            let w = i64::try_from(w).unwrap();
            (i % w, i / w)
        })
        .filter(|&(x, y)| (-1..=1).all(|d| field(x + d, y) && field(x, y + d)))
        .filter(|&c| far(c, 4))
        .collect();
    farmed.sort_unstable();
    let farmed = farmed
        .iter()
        .step_by(farmed.len().div_ceil(5).max(1))
        .copied()
        .collect();
    (towns, on_roads, farmed)
}

#[derive(Default)]
struct Tally {
    road: (usize, usize),
    building: (usize, usize),
    walls: (usize, usize),
}

fn ratio((a, b): (usize, usize)) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let r = a as f64 / b.max(1) as f64;
    r
}

/// Compares one cell's relief land at z9 with its tactical block.
fn compare(block: &Value, land: &LandWindow, tally: &mut Tally, (gx, gy): (i64, i64)) {
    let rules = block["rules"]["squares"].as_array().unwrap();
    let ext = |i: usize, key: &str| rules[i]["ext"][key].clone();
    let mut cell = Tally::default();
    for i in 0..64 * 64 {
        let (px, py) = (i % 64, i / 64);
        let feature = ext(i, "feature");
        let tactical_road = feature == "road" || feature == "ruts";
        let relief_road = land.road_at(px, py).is_some();
        cell.road.0 += usize::from(tactical_road && relief_road);
        cell.road.1 += usize::from(tactical_road || relief_road);
        let tactical_building = !ext(i, "building").is_null();
        let relief_building = land.building_at(px, py);
        cell.building.0 += usize::from(tactical_building && relief_building);
        cell.building.1 += usize::from(tactical_building || relief_building);
    }
    // Every wall between two fields: a relief hedge or wall line within
    // a square of it.
    for wall in block["layout"]["walls"].as_array().unwrap() {
        let kit = wall["kit"].as_str().unwrap_or("");
        if !matches!(kit, "hedge" | "drystone" | "wattle" | "timber" | "stone") {
            continue;
        }
        let (x, y) = (wall["x"].as_u64().unwrap(), wall["y"].as_u64().unwrap());
        let (x, y) = (usize::try_from(x).unwrap(), usize::try_from(y).unwrap());
        let (a, b) = if wall["axis"] == "vertical" {
            ((x.wrapping_sub(1), y), (x, y))
        } else {
            ((x, y.wrapping_sub(1)), (x, y))
        };
        if a.0 >= 64 || a.1 >= 64 || b.0 >= 64 || b.1 >= 64 {
            continue;
        }
        let (fa, fb) = (ext(a.1 * 64 + a.0, "field"), ext(b.1 * 64 + b.0, "field"));
        if fa.is_null() || fb.is_null() || fa == fb {
            continue;
        }
        cell.walls.1 += 1;
        cell.walls.0 += usize::from(land.boundary_at(a.0, a.1) || land.boundary_at(b.0, b.1));
    }
    println!(
        "cell {gx},{gy}: road {}/{} building {}/{} field walls {}/{}",
        cell.road.0, cell.road.1, cell.building.0, cell.building.1, cell.walls.0, cell.walls.1
    );
    for (t, c) in [
        (&mut tally.road, cell.road),
        (&mut tally.building, cell.building),
        (&mut tally.walls, cell.walls),
    ] {
        t.0 += c.0;
        t.1 += c.1;
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "release gate: needs a settled MICRO seed 42 world (cargo test --release --test zoom_continuity -- --ignored)"]
async fn relief_and_tactical_land_agree() {
    let dir = world("ARDA_ZOOM_WORLD", "42", 42, arda::GenerateConfig::MICRO, 6);
    let state = open(&dir);
    let loaded = Arc::new(arda::World::load(&dir).unwrap());
    let m = loaded.manifest();
    let pyramid = Pyramid {
        max_zoom: 4,
        areas_wide: m.areas_wide,
        areas_high: m.areas_high,
    };
    let land = Landscape::open(&dir).unwrap().expect("a settled world");
    let relief = ReliefWorld::new(loaded)
        .unwrap()
        .with_landscape(land.clone());
    let (towns, roads, farmed) = cells(&dir);
    assert!(!towns.is_empty() && !roads.is_empty() && !farmed.is_empty());
    let mut tally = Tally::default();
    for &(gx, gy) in towns.iter().chain(&roads).chain(&farmed) {
        let block = block(&state, gx, gy).await;
        let window =
            LandWindow::gather(&relief, &land, &pyramid, 9, (gx * 64, gy * 64), (64, 64)).unwrap();
        compare(&block, &window, &mut tally, (gx, gy));
    }
    let (road, building, walls) = (ratio(tally.road), ratio(tally.building), ratio(tally.walls));
    println!(
        "road IoU {road:.4} over {} squares; building IoU {building:.4} over {}; \
         field walls on relief lines {walls:.4} of {}",
        tally.road.1, tally.building.1, tally.walls.1
    );
    assert!(tally.road.1 > 500 && tally.building.1 > 200 && tally.walls.1 > 200);
    assert!(road >= 0.85, "road IoU {road:.3}");
    assert!(building >= 0.95, "building IoU {building:.3}");
    assert!(walls >= 0.95, "field walls on relief lines {walls:.3}");
}

#[test]
#[ignore = "release gate: needs a settled MICRO seed 42 world (cargo test --release --test zoom_continuity -- --ignored)"]
fn land_tiles_join_exactly_and_plain_worlds_are_unchanged() {
    let dir = world("ARDA_ZOOM_WORLD", "42", 42, arda::GenerateConfig::MICRO, 6);
    let loaded = Arc::new(arda::World::load(&dir).unwrap());
    let m = loaded.manifest();
    let pyramid = Pyramid {
        max_zoom: 4,
        areas_wide: m.areas_wide,
        areas_high: m.areas_high,
    };
    let land = Landscape::open(&dir).unwrap().expect("a settled world");
    let plain = ReliefWorld::new(Arc::clone(&loaded)).unwrap();
    let relief = ReliefWorld::new(loaded).unwrap().with_landscape(land);
    // A village (the first one) at z7, z8 and z10.
    let towns = cells(&dir).0;
    let (gx, gy) = towns[0];
    for z in [7, 8, 10] {
        let c = |g: i64| {
            let p = g * 64 * (1 << z) / (1 << 9);
            (p / 256 - 1) * 256
        };
        let origin = (c(gx), c(gy));
        let whole = render_window(&relief, &pyramid, z, origin, (512, 512)).unwrap();
        for (dx, dy) in [(0, 0), (256, 0), (0, 256), (256, 256)] {
            let at = (origin.0 + dx, origin.1 + dy);
            let part = render_window(&relief, &pyramid, z, at, (256, 256)).unwrap();
            let (dx, dy) = (usize::try_from(dx).unwrap(), usize::try_from(dy).unwrap());
            for row in 0..256_usize {
                let a = ((row + dy) * 512 + dx) * 4;
                let b = row * 256 * 4;
                assert_eq!(
                    whole.pixels[a..a + 1024],
                    part.pixels[b..b + 1024],
                    "z{z} tile {dx},{dy} row {row}"
                );
            }
        }
        // The village's own tile: land drawn with society, not without.
        let at = (origin.0 + 256, origin.1 + 256);
        let with = render_window(&relief, &pyramid, z, at, (256, 256)).unwrap();
        let without = render_window(&plain, &pyramid, z, at, (256, 256)).unwrap();
        assert!(with.pixels != without.pixels, "z{z}: land is drawn");
    }
}
