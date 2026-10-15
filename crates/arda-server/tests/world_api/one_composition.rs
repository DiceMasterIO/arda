//! One composition per apron window, shared by the image and the scene
//! (review 3.5-8, 3.6-3, 3.6-4; logic/16 §api-tactical, logic/12
//! §scene-tokens), on the society fixture's town of settlement 10:
//!
//! - `/scene` and the cell body carry the same scene, drawn with the
//!   render seed the image uses;
//! - a window's body is its cells' bodies side by side: squares, walls,
//!   edge rules, placements and lights, borders included;
//! - every NPC token stands in exactly one cell, workers stand in their
//!   workplace by day wherever it lies, and no token stands on furniture.

use crate::support::{get_with, library_dir, society_state, Reply};
use arda_scene::RulesSidecar;
use arda_tactical::catalog::Layer;
use arda_tactical::{Library, TacticalLayout};
use axum::http::StatusCode;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

/// The contract kit's town cell (settlement 10).
const TOWN: (u32, u32) = (479, 1245);

fn lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| Library::load(&library_dir()).unwrap())
}

async fn ok(uri: &str) -> Value {
    let r: Reply = get_with(society_state(), uri, None).await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{uri}: {}",
        String::from_utf8_lossy(&r.body)
    );
    r.json()
}

#[tokio::test]
async fn the_scene_route_and_the_cell_body_carry_one_scene() {
    let (gx, gy) = TOWN;
    let body = ok(&format!("/v1/tactical/cell/{gx}/{gy}")).await;
    let scene = ok(&format!("/v1/tactical/cell/{gx}/{gy}/scene?time=day")).await;
    assert_eq!(scene["scene"]["seed"], body["render_seed"]);
    // The body's scene carries `origin_gs`; `/scene` carries it in its
    // envelope (API.md).
    let mut inner = body["scene"].clone();
    let origin = inner.as_object_mut().unwrap().remove("origin_gs");
    assert_eq!(origin.as_ref(), Some(&scene["origin_gs"]));
    let differ: Vec<&String> = inner
        .as_object()
        .unwrap()
        .iter()
        .filter(|(k, v)| scene["scene"].get(k.as_str()) != Some(v))
        .map(|(k, _)| k)
        .collect();
    assert!(differ.is_empty(), "the scenes differ in {differ:?}");
    assert_eq!(scene["scene"], inner);
}

fn layout_of(v: &Value) -> TacticalLayout {
    serde_json::from_value(v["layout"].clone()).unwrap()
}

fn rules_of(v: &Value) -> RulesSidecar {
    serde_json::from_value(v["rules"].clone()).unwrap()
}

#[tokio::test]
async fn a_window_body_is_its_cell_bodies_side_by_side() {
    let (gx, gy) = TOWN;
    let win = ok(&format!(
        "/v1/tactical/window?gx0={gx}&gy0={gy}&w=2&h=1&ppsq=32"
    ))
    .await;
    let (wl, wr) = (layout_of(&win), rules_of(&win));
    for k in 0..2_u32 {
        let cell = ok(&format!("/v1/tactical/cell/{}/{gy}", gx + k)).await;
        let cut = arda_blocks::Cut {
            x: 64 * k,
            y: 0,
            w: 64,
            h: 64,
        };
        let (cl, cr) = (layout_of(&cell), rules_of(&cell));
        let from_window = arda_blocks::crop::layout(&wl, cut, &cl.name);
        assert_eq!(from_window.squares, cl.squares, "cell {k} squares");
        assert_eq!(from_window.walls, cl.walls, "cell {k} walls");
        assert_eq!(from_window.placements, cl.placements, "cell {k} placements");
        assert_eq!(from_window.lights, cl.lights, "cell {k} lights");
        assert_eq!(arda_blocks::crop::rules(&wr, cut), cr, "cell {k} rules");
    }
}

/// Squares a prop (furniture, a crate, a stall) stands on.
fn furniture(layout: &TacticalLayout, seed: u64) -> BTreeSet<(u32, u32)> {
    let placed = arda_scene::squares::resolve_known(layout, lib(), seed);
    placed
        .iter()
        .filter(|p| p.asset.layer == Layer::Prop)
        .flat_map(|p| arda_scene::squares::covered(p.rect, layout.width, layout.height))
        .collect()
}

#[tokio::test]
async fn tokens_stand_once_at_work_by_day_and_never_on_furniture() {
    let state = society_state();
    let people = state.people.clone().unwrap();
    let notables: BTreeMap<String, arda_npc::Npc> = people
        .notables_of(10)
        .iter()
        .map(|n| (n.id.0.to_string(), n.clone()))
        .collect();
    let (gx, gy) = TOWN;
    for time in ["day", "night"] {
        let mut seen: BTreeMap<String, (u32, u32)> = BTreeMap::new();
        for dy in 0..3_u32 {
            for dx in 0..3_u32 {
                let (cx, cy) = (gx + dx - 1, gy + dy - 1);
                let scene = ok(&format!("/v1/tactical/cell/{cx}/{cy}/scene?time={time}")).await;
                let body = ok(&format!("/v1/tactical/cell/{cx}/{cy}")).await;
                let layout = layout_of(&body);
                let rules = rules_of(&body);
                let seed: u64 = body["render_seed"].as_str().unwrap().parse().unwrap();
                let props = furniture(&layout, seed);
                for t in scene["tokens"].as_array().unwrap() {
                    let id = t["npc_id"].as_str().unwrap().to_owned();
                    let (x, y) = (
                        u32::try_from(t["x"].as_u64().unwrap()).unwrap(),
                        u32::try_from(t["y"].as_u64().unwrap()).unwrap(),
                    );
                    assert!(
                        seen.insert(id.clone(), (cx, cy)).is_none(),
                        "{time}: {id} stands in cell {:?} and in {cx},{cy}",
                        seen[&id]
                    );
                    assert!(
                        !props.contains(&(x, y)),
                        "{time}: {id} stands on furniture at {x},{y} of {cx},{cy}"
                    );
                    let ext = rules.cell(x, y).and_then(|c| c.ext.clone()).unwrap();
                    assert_eq!(ext.get("building"), Some(&t["building_id"]), "{id}");
                    if t["settlement_id"] != "10" {
                        continue;
                    }
                    let n = &notables[&id];
                    let want = match (time, n.workplace_building) {
                        ("day", Some(w)) => (w.0, "worker"),
                        _ => (n.home_building.0, "resident"),
                    };
                    assert_eq!(t["building_id"], want.0.to_string(), "{time}: {id}");
                    assert_eq!(t["kind"], want.1, "{time}: {id}");
                }
            }
        }
        assert!(!seen.is_empty(), "{time}: no tokens around the town");
    }
}
