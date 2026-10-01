//! End-to-end acceptance (integration plan §7; goal-prompt roadmap step 3,
//! goal 70): seed → world → settlements → society → one tactical village
//! map with its people, served over HTTP.
//!
//! Run in the release gate:
//! `cargo test --release --test e2e_village -- --ignored --nocapture`.
//! It writes the village's PNG, layout, scene and notable roster to
//! `out/e2e/` for the maintainer to look at (goal-prompt §7).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_server::{router, AppState, ServerConfig};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower::ServiceExt;

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

struct Reply {
    status: StatusCode,
    body: Vec<u8>,
    etag: Option<String>,
}

impl Reply {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&self.body)))
    }
}

async fn get(state: &Arc<AppState>, uri: &str) -> Reply {
    let request = Request::get(uri).body(Body::empty()).unwrap();
    let response = router(Arc::clone(state)).oneshot(request).await.unwrap();
    let status = response.status();
    let etag = response
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .to_vec();
    Reply { status, body, etag }
}

async fn ok(state: &Arc<AppState>, uri: &str) -> Reply {
    let r = get(state, uri).await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{uri}: {}",
        String::from_utf8_lossy(&r.body)
    );
    r
}

fn open(world: &Path) -> Arc<AppState> {
    let mut config = ServerConfig::new(world.to_path_buf());
    config.overview.tile_base_px = 512;
    config.overview.max_quality_px = 1024;
    config.library = workspace().join("assets/tactical/placeholder");
    Arc::new(AppState::open(&config).unwrap())
}

/// Steps 1–2: MICRO seed 42 with fine terrain, then settle and society.
fn world() -> PathBuf {
    let dir = workspace().join(format!("target/e2e-village-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.parent().unwrap()).unwrap();
    let manifest = arda::generate_from_fine_recipe(
        42,
        arda::GenerateConfig::MICRO,
        &dir,
        arda::FineDeliveryLimits::default(),
        6,
    )
    .expect("generating the MICRO world");
    assert_eq!(
        manifest.fine_terrain.map(|f| f.recipe_version),
        Some(6),
        "the village gate runs on recipe 6"
    );
    arda_settle::generate(&dir, arda_settle::grid::MEMORY_BUDGET).expect("settle");
    arda_people::build(&dir).expect("society build");
    dir
}

fn format_one(path: &Path) {
    let v: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(v["format_version"], 1, "{}", path.display());
}

/// Step 3: the riverine village with the largest population, ties by id.
fn pick(settlements: &Value) -> Value {
    let mut villages: Vec<&Value> = settlements["settlements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["tier"] == "village" && s["riverine"] == true)
        .collect();
    assert!(
        !villages.is_empty(),
        "a MICRO world without a river village is itself a finding"
    );
    let id = |s: &Value| s["id"].as_str().unwrap().parse::<u64>().unwrap();
    villages.sort_by_key(|s| (std::cmp::Reverse(s["population"].as_u64().unwrap()), id(s)));
    villages[0].clone()
}

fn plan_building_ids(plan: &Value) -> BTreeSet<String> {
    plan["buildings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].as_str().unwrap().to_owned())
        .collect()
}

/// Crossing records at least this far from every settlement: roads there
/// are the world's ways, not town streets.
const RURAL_M: i64 = 700;

/// Step 10: road crossings on the refined river. Around the first rural
/// records, every bridge deck and ford stands in the river the map shows,
/// and no road surface stands in water.
async fn crossings_sit_on_the_river(state: &Arc<AppState>, dir: &Path, all: &Value) {
    let roads: Value =
        serde_json::from_slice(&std::fs::read(dir.join("society/roads.json")).unwrap()).unwrap();
    let towns: Vec<(i64, i64)> = all["settlements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| (s["x_m"].as_i64().unwrap(), s["y_m"].as_i64().unwrap()))
        .collect();
    let (mut bridges, mut fords, mut windows) = (0, 0, 0);
    for c in roads["crossings"].as_array().unwrap() {
        let (x, y) = (c["x_m"].as_i64().unwrap(), c["y_m"].as_i64().unwrap());
        let rural = towns
            .iter()
            .all(|&(tx, ty)| (tx - x).abs().max((ty - y).abs()) > RURAL_M);
        if c["water"] != "river" || !rural {
            continue;
        }
        let uri = format!(
            "/v1/tactical/window?gx0={}&gy0={}&w=3&h=3",
            x / 100 - 1,
            y / 100 - 1
        );
        let w = ok(state, &uri).await.json();
        let squares = w["layout"]["squares"].as_array().unwrap();
        let rules = w["rules"]["squares"].as_array().unwrap();
        for (i, (q, r)) in squares.iter().zip(rules).enumerate() {
            let feature = r["ext"]["feature"].as_str().unwrap_or("");
            let wet = q["water_depth_ft"].as_u64().unwrap() > 0;
            match feature {
                "bridge" => {
                    assert!(wet, "{uri}: bridge deck on dry land at square {i}");
                    assert_eq!(r["deck"], true, "{uri}: square {i}");
                    bridges += 1;
                }
                "ford" => {
                    assert!(wet, "{uri}: ford on dry land at square {i}");
                    fords += 1;
                }
                "road" | "ruts" | "verge" | "shoulder" | "abutment" => {
                    assert!(!wet, "{uri}: {feature} in water at square {i}");
                }
                _ => {}
            }
        }
        windows += 1;
        if windows == 6 {
            break;
        }
    }
    assert!(
        bridges + fords > 0,
        "no crossing in {windows} rural windows"
    );
    println!("{windows} rural crossing windows: {bridges} deck and {fords} ford squares");
}

/// Step 12: town bridges on the refined river. The first riverine town
/// with a bridge: every square of its first deck is a deck over water in
/// the tactical window around it.
async fn town_bridges_sit_on_the_river(state: &Arc<AppState>, all: &Value) {
    let mut towns: Vec<&Value> = all["settlements"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["riverine"] == true && matches!(s["tier"].as_str(), Some("town" | "city")))
        .collect();
    towns.sort_by_key(|s| s["id"].as_str().unwrap().parse::<u64>().unwrap());
    for town in towns {
        let id = town["id"].as_str().unwrap();
        let plan = ok(state, &format!("/v1/settlements/{id}/plan"))
            .await
            .json();
        let Some(bridge) = plan["bridges"].as_array().and_then(|b| b.first()) else {
            continue;
        };
        let along_x = bridge["along_x"].as_bool().unwrap();
        let squares: Vec<(i64, i64)> = bridge["rows"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|r| {
                let r: Vec<i64> = r
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_i64().unwrap())
                    .collect();
                (r[1]..=r[2]).map(move |a| if along_x { (a, r[0]) } else { (r[0], a) })
            })
            .collect();
        let (cx, cy) = (squares[0].0.div_euclid(64), squares[0].1.div_euclid(64));
        let (gx0, gy0) = ((cx - 1) * 64, (cy - 1) * 64);
        let uri = format!("/v1/tactical/window?gx0={}&gy0={}&w=3&h=3", cx - 1, cy - 1);
        let w = ok(state, &uri).await.json();
        let layout = w["layout"]["squares"].as_array().unwrap();
        let rules = w["rules"]["squares"].as_array().unwrap();
        let mut decks = 0;
        for (x, y) in squares {
            let (lx, ly) = (x - gx0, y - gy0);
            if !(0..192).contains(&lx) || !(0..192).contains(&ly) {
                continue;
            }
            let i = usize::try_from(ly * 192 + lx).unwrap();
            assert_eq!(rules[i]["deck"], true, "{uri}: town bridge square {x},{y}");
            assert!(
                layout[i]["water_depth_ft"].as_u64().unwrap() > 0,
                "{uri}: town bridge square {x},{y} over dry land"
            );
            decks += 1;
        }
        assert!(decks > 0, "{uri}: no deck square of {id}'s bridge");
        println!("town {id}: first bridge, {decks} deck squares over water");
        return;
    }
    panic!("no riverine town with a bridge");
}

/// Budgets of goal 50 at 64 px per square: a battle-map block and a
/// quarter-block window.
const BLOCK_BUDGET: Duration = Duration::from_millis(500);
const QUARTER_BUDGET: Duration = Duration::from_millis(250);

/// Goal 50 (release build, idle machine): a freshly opened server serves
/// the village's centre block as a 64 px-per-square PNG in under 500 ms,
/// its plan, refinement, overlays, render and encode all cold, and then a
/// quarter-block window it has not rendered in under 250 ms. Best of three
/// fresh servers, so a loaded machine does not flake.
async fn cold_images_are_quick(dir: &Path, gx: u64, gy: u64) {
    let (mut block, mut quarter) = (Duration::MAX, Duration::MAX);
    for _ in 0..3 {
        let state = open(dir);
        let t = Instant::now();
        let png = ok(&state, &format!("/v1/tactical/cell/{gx}/{gy}.png?ppsq=64")).await;
        block = block.min(t.elapsed());
        assert_eq!(png.body[..8], *b"\x89PNG\r\n\x1a\n");
        // A quarter block across the corner of four cells.
        let (x, y) = (gx * 64 + 48, gy * 64 + 48);
        let uri = format!("/v1/tactical/window.png?gsx={x}&gsy={y}&w=32&h=32&ppsq=64");
        let t = Instant::now();
        let q = ok(&state, &uri).await;
        quarter = quarter.min(t.elapsed());
        let info = png::Decoder::new(std::io::Cursor::new(&q.body))
            .read_info()
            .unwrap()
            .info()
            .clone();
        assert_eq!((info.width, info.height), (32 * 64, 32 * 64));
    }
    println!("goal 50: cold block {block:?}, cold quarter window {quarter:?} (best of 3)");
    assert!(block < BLOCK_BUDGET, "cold block took {block:?}");
    assert!(
        quarter < QUARTER_BUDGET,
        "cold quarter window took {quarter:?}"
    );
}

#[test]
#[ignore = "release gate: generates a world (cargo test --release --test e2e_village -- --ignored)"]
fn a_village_tactical_map_with_its_people_is_served_over_http() {
    let dir = world();
    let soc = dir.join("society");
    for f in [
        "settlements.json",
        "roads.json",
        "realms.json",
        "names.json",
        "society.json",
        "notables.json",
    ] {
        format_one(&soc.join(f));
    }
    for f in ["landuse.bin", "realms.bin"] {
        assert!(soc.join(f).is_file(), "{f}");
    }
    let rt = tokio::runtime::Runtime::new().unwrap();
    let state = open(&dir);
    rt.block_on(async {
        // Step 5: world routes.
        let w = ok(&state, "/v1/world").await.json();
        assert_eq!(w["contract_version"], 2);
        let all = ok(&state, "/v1/settlements").await.json();
        let village = pick(&all);
        let id = village["id"].as_str().unwrap().to_owned();
        let (gx, gy) = (
            village["cell_x"].as_u64().unwrap(),
            village["cell_y"].as_u64().unwrap(),
        );
        let cell = ok(&state, &format!("/v1/cell/{gx}/{gy}")).await.json();
        assert_eq!(cell["contract_version"], 2);
        // Step 6: settlement routes.
        let listed = ok(&state, "/v1/settlements?tier=village").await.json();
        assert!(listed["settlements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == id.as_str()));
        let plan = ok(&state, &format!("/v1/settlements/{id}/plan"))
            .await
            .json();
        let in_plan = plan_building_ids(&plan);
        assert!(in_plan.len() > 20, "{} plan buildings", in_plan.len());
        // Step 7: the centre cell's tactical block, with buildings and a road.
        let uri = format!("/v1/tactical/cell/{gx}/{gy}");
        let block_reply = ok(&state, &uri).await;
        let block = block_reply.json();
        assert_eq!(block["layout"]["width"], 64);
        assert_eq!(block["layout"]["height"], 64);
        assert_eq!(block["rules"]["format_version"], 2);
        let overlays = block["meta"]["overlays"].as_str().unwrap().to_owned();
        assert!(overlays.contains("town"), "no buildings: {overlays}");
        // Within the village's reach its streets replace the world's road
        // (logic/09 §reservations): the road arrives as a street.
        let streets = block["layout"]["squares"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|q| matches!(q["ground"].as_str(), Some("dirt" | "cobbles" | "mud")))
            .count();
        assert!(streets > 64, "no street: {streets} street squares");
        assert!(
            block["layout"]["walls"].as_array().unwrap().len() > 20,
            "building walls"
        );
        // Fields: the village core is left to the town (logic/09
        // §reservations), so its fields ring lies in the 3 × 3 window.
        let window = ok(
            &state,
            &format!("/v1/tactical/window?gx0={}&gy0={}&w=3&h=3", gx - 1, gy - 1),
        )
        .await
        .json();
        let wo = window["meta"]["overlays"].as_str().unwrap();
        assert!(wo.contains("fields"), "no fields around the village: {wo}");
        // Step 8: scene tokens resolve to NPCs of the plan's buildings.
        let scene = ok(&state, &format!("{uri}/scene")).await.json();
        let tokens = scene["tokens"].as_array().unwrap();
        assert!(!tokens.is_empty(), "no NPC tokens on the village block");
        for t in tokens {
            let npc = ok(
                &state,
                &format!("/v1/npc/{}", t["npc_id"].as_str().unwrap()),
            )
            .await
            .json();
            let b = t["building_id"].as_str().unwrap();
            assert!(
                npc["home_building"] == b || npc["workplace_building"] == b,
                "token {t} is not in its NPC's building"
            );
            assert!(in_plan.contains(b), "building {b} is not in the plan");
            assert_eq!(t["settlement_id"], id.as_str());
            assert!(npc["sheet"].is_object());
            let (x, y) = (t["x"].as_u64().unwrap(), t["y"].as_u64().unwrap());
            assert!(x < 64 && y < 64);
        }
        crossings_sit_on_the_river(&state, &dir, &all).await;
        town_bridges_sit_on_the_river(&state, &all).await;
        let roster = ok(&state, &format!("/v1/settlements/{id}/npcs"))
            .await
            .json();
        let notables = roster["npcs"].as_array().unwrap();
        assert!(notables.iter().any(|n| n["notable"] == true));
        // Step 9: images.
        let png = ok(&state, &format!("{uri}.png")).await;
        let decoder = png::Decoder::new(std::io::Cursor::new(&png.body));
        let info = decoder.read_info().unwrap().info().clone();
        assert_eq!((info.width, info.height), (64 * 128, 64 * 128));
        // Step 11: determinism across requests and a fresh router.
        let again = ok(&state, &uri).await;
        assert_eq!(again.body, block_reply.body);
        assert_eq!(again.etag, block_reply.etag);
        let fresh = open(&dir);
        assert_eq!(ok(&fresh, &uri).await.body, block_reply.body);
        let scene_again = ok(&fresh, &format!("{uri}/scene")).await.json();
        assert_eq!(scene_again["tokens"], scene["tokens"]);
        // Step 12: goal 50, cold images (see `cold_images_are_quick`).
        cold_images_are_quick(&dir, gx, gy).await;
        // Step 13: artefacts for review.
        let out = workspace().join("out/e2e");
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join(format!("village_{id}_cell.png")), &png.body).unwrap();
        std::fs::write(
            out.join(format!("village_{id}_block.json")),
            &block_reply.body,
        )
        .unwrap();
        std::fs::write(
            out.join(format!("village_{id}_scene.json")),
            serde_json::to_vec_pretty(&scene).unwrap(),
        )
        .unwrap();
        std::fs::write(
            out.join(format!("village_{id}_notables.json")),
            serde_json::to_vec_pretty(&roster).unwrap(),
        )
        .unwrap();
        let window_png = ok(
            &state,
            &format!(
                "/v1/tactical/window.png?gx0={}&gy0={}&w=3&h=3",
                gx - 1,
                gy - 1
            ),
        )
        .await;
        std::fs::write(out.join(format!("village_{id}_3x3.png")), &window_png.body).unwrap();
        println!(
            "village {id} {} ({} people) at cell {gx},{gy}: {} tokens, overlays {overlays}",
            village["name"],
            village["population"],
            tokens.len()
        );
    });
    drop(state);
    town_wfc_relaxed_rate(&dir);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Goal 47 on the town WFC: over every plan of the world (interiors and
/// outdoor chunks), under 1 % of the problems fall back to the relaxed
/// fill.
fn town_wfc_relaxed_rate(dir: &Path) {
    let world = arda_people::World::open(dir).unwrap();
    let mut total = arda_town::block::wfc::Report::default();
    for s in &world.files.settlements.settlements {
        if let Some(plan) = world.plan(s.id.get()).unwrap().as_ref() {
            total.add(&arda_town::block::wfc::report(plan));
        }
    }
    #[allow(clippy::cast_precision_loss)]
    let rate = total.relaxed() as f64 / total.problems().max(1) as f64;
    println!(
        "town WFC: {} of {} problems relaxed ({:.3} %): {:?}",
        total.relaxed(),
        total.problems(),
        rate * 100.0,
        total.relaxed_by_function
    );
    assert!(total.problems() > 10_000, "{total:?}");
    assert!(rate < 0.01, "town WFC relaxed-fill rate {rate}: {total:?}");
}
