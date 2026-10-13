//! `POST /v1/tactical/bake` (1.0 task 04): background, uncached renders of
//! client-edited cells, against the real MICRO seed-42 world.

use crate::support::{config, send, state, Reply};
use arda_server::tactical::bake::{BakeState, QUEUE_JOBS};
use arda_server::tactical::block::BlockRequest;
use arda_server::tactical::cells::APRON;
use arda_server::tactical::splice::splice;
use arda_server::AppState;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

/// The river cell of `cells.rs`.
const RIVER: (u32, u32) = (529, 812);
/// Quick renders: 4096 px a side, max zoom 3, 85 tiles.
const PPSQ: u32 = 64;
const WAIT: Duration = Duration::from_secs(900);

/// A tile set: `((z, x, y), bytes)` in order.
type Tiles = Vec<((u32, u32, u32), Vec<u8>)>;

fn fresh(workers: usize) -> Arc<AppState> {
    let mut c = config();
    c.prefetch_workers = 0;
    c.bake_workers = workers;
    Arc::new(AppState::open(&c).unwrap())
}

async fn fetch(state: &Arc<AppState>, uri: &str) -> Reply {
    send(
        Arc::clone(state),
        Request::get(uri).body(Body::empty()).unwrap(),
    )
    .await
}

async fn post(state: &Arc<AppState>, body: &Value) -> Reply {
    let request = Request::post("/v1/tactical/bake")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap()))
        .unwrap();
    send(Arc::clone(state), request).await
}

async fn delete(state: &Arc<AppState>, uri: &str) -> Reply {
    send(
        Arc::clone(state),
        Request::delete(uri).body(Body::empty()).unwrap(),
    )
    .await
}

/// The unedited layout of `GET /v1/tactical/cell/{gx}/{gy}`.
async fn cell_layout(state: &Arc<AppState>, (gx, gy): (u32, u32)) -> Value {
    let r = fetch(state, &format!("/v1/tactical/cell/{gx}/{gy}")).await;
    assert_eq!(r.status, StatusCode::OK);
    r.json()["layout"].clone()
}

fn body(layout: &Value) -> Value {
    json!({"gx": RIVER.0, "gy": RIVER.1, "ppsq": PPSQ, "layout": layout})
}

/// Posts `body`, waits for the job and returns its id and status.
async fn bake(state: &Arc<AppState>, body: &Value) -> (String, Value) {
    let r = post(state, body).await;
    assert_eq!(
        r.status,
        StatusCode::ACCEPTED,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    let id = r.json()["job_id"].as_str().unwrap().to_owned();
    let s = Arc::clone(state);
    assert!(tokio::task::spawn_blocking(move || s.bake.wait_idle(WAIT))
        .await
        .unwrap());
    let status = fetch(state, &format!("/v1/tactical/bake/{id}")).await;
    assert_eq!(status.status, StatusCode::OK);
    (id, status.json())
}

/// Every tile of a finished job, in `(z, x, y)` order.
async fn tiles(state: &Arc<AppState>, id: &str) -> Tiles {
    let mut out = Vec::new();
    for z in 0..=3 {
        let n = 1 << z;
        for y in 0..n {
            for x in 0..n {
                let r = fetch(
                    state,
                    &format!("/v1/tactical/bake/{id}/tiles/{z}/{x}/{y}.webp"),
                )
                .await;
                assert_eq!(r.status, StatusCode::OK, "{z}/{x}/{y}");
                assert_eq!(r.headers["content-type"], "image/webp");
                assert_eq!(r.headers["cache-control"], "no-store");
                assert!(r.headers["etag"].to_str().unwrap().starts_with('"'));
                out.push(((z, x, y), r.body));
            }
        }
    }
    out
}

/// The cell route's tile `zxy` at [`PPSQ`] and whether it was a cache hit.
async fn cell_tile(state: &Arc<AppState>, (z, x, y): (u32, u32, u32)) -> (Vec<u8>, String) {
    let (gx, gy) = RIVER;
    let r = fetch(
        state,
        &format!("/v1/tactical/cell/{gx}/{gy}/tiles/{z}/{x}/{y}.webp?ppsq={PPSQ}"),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let hit = r.headers["x-arda-cache"].to_str().unwrap().to_owned();
    (r.body, hit)
}

#[test]
fn the_cell_layout_splices_into_its_apron_unchanged() {
    let s = state();
    let (w, h) = s.query.cells();
    let world = (i64::from(w) * 64, i64::from(h) * 64);
    for (gx, gy) in [RIVER, (RIVER.0 + 1, RIVER.1), (RIVER.0, RIVER.1 + 1)] {
        let req = BlockRequest::cell(gx, gy);
        let apron = s
            .tactical
            .world_block(&req.with_apron(APRON, world))
            .unwrap();
        let cell = s.tactical.world_block(&req).unwrap();
        let out = splice(&apron.layout, &cell.layout, (APRON, APRON)).unwrap();
        assert_eq!(out, apron.layout, "{gx},{gy}");
    }
}

/// An unedited bake equals the cell route's tiles byte for byte, fills no
/// shared cache, and two fresh servers bake identical bytes.
#[tokio::test]
async fn an_unedited_bake_equals_the_cell_tiles_and_caches_nothing() {
    let a = fresh(1);
    let layout = cell_layout(&a, RIVER).await;
    let before = a.tactical.cache_counts();
    let (id, status) = bake(&a, &body(&layout)).await;
    assert_eq!(status["state"], "done", "{status}");
    assert_eq!(status["format_version"], 1);
    assert_eq!(status["tile_count"], 85);
    assert_eq!(status["tiles"]["max_zoom"], 3);
    assert_eq!(status["tiles"]["image_width_px"], 64 * PPSQ);
    assert_eq!(
        status["origin_gs"],
        json!([i64::from(RIVER.0) * 64, i64::from(RIVER.1) * 64])
    );
    assert!(status["error"].is_null());
    assert_eq!(
        a.tactical.cache_counts(),
        before,
        "the bake filled a shared cache"
    );
    let baked = tiles(&a, &id).await;
    let total: usize = baked.iter().map(|(_, b)| b.len()).sum();
    assert_eq!(status["total_bytes"], total);
    for (k, (zxy, bytes)) in baked.iter().enumerate() {
        let (cell, hit) = cell_tile(&a, *zxy).await;
        if k == 0 {
            assert_eq!(hit, "miss", "the bake must not warm the cell route");
        }
        assert!(cell == *bytes, "tile {zxy:?} differs from the cell route");
    }
    // A re-post of a finished job answers 200 with the same id.
    let again = post(&a, &body(&layout)).await;
    assert_eq!(again.status, StatusCode::OK);
    assert_eq!(again.json()["job_id"], id.as_str());
    assert_eq!(again.json()["state"], "done");
    // Determinism: a fresh server bakes the same id and bytes.
    let b = fresh(1);
    let (id_b, _) = bake(&b, &body(&layout)).await;
    assert_eq!(id_b, id);
    assert!(tiles(&b, &id_b).await == baked);
}

/// An edited 4 × 4 patch changes the tiles covering it and nothing at the
/// cell's edges; a request issued while the bake runs does not wait for it.
#[tokio::test]
async fn an_edit_changes_its_tiles_and_requests_do_not_wait_for_bakes() {
    let s = fresh(1);
    let mut layout = cell_layout(&s, RIVER).await;
    let library = fetch(&s, "/v1/tactical/library").await.json();
    let squares = layout["squares"].as_array_mut().unwrap();
    // Squares 41..45 lie inside tile (5, 5) of zoom 3 (512 px = 8 squares).
    let old = squares[41 * 64 + 41]["ground"].clone();
    let new = library["grounds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["water"] != true && g["key"] != old)
        .unwrap()["key"]
        .clone();
    for y in 41..45 {
        for x in 41..45 {
            let sq = &mut squares[y * 64 + x];
            sq["ground"] = new.clone();
            sq["water_depth_ft"] = json!(0);
        }
    }
    s.bake.hold(true);
    let r = post(&s, &body(&layout)).await;
    assert_eq!(r.status, StatusCode::ACCEPTED);
    let id = r.json()["job_id"].as_str().unwrap().to_owned();
    let (b, i) = (Arc::clone(&s), id.clone());
    assert!(
        tokio::task::spawn_blocking(move || b.bake.wait_state(&i, BakeState::Running, WAIT))
            .await
            .unwrap()
    );
    // The bake now holds its lane; the cell route renders and answers.
    let mut unedited: Tiles = Vec::new();
    for z in 0..=3 {
        let n = 1 << z;
        for y in 0..n {
            for x in 0..n {
                unedited.push(((z, x, y), cell_tile(&s, (z, x, y)).await.0));
            }
        }
    }
    let running = fetch(&s, &format!("/v1/tactical/bake/{id}")).await.json();
    assert_eq!(
        running["state"], "running",
        "the request waited for the bake"
    );
    s.bake.hold(false);
    let b = Arc::clone(&s);
    assert!(tokio::task::spawn_blocking(move || b.bake.wait_idle(WAIT))
        .await
        .unwrap());
    let baked = tiles(&s, &id).await;
    let at = |zxy| {
        let find = |v: &Tiles| v.iter().find(|(k, _)| *k == zxy).unwrap().1.clone();
        (find(&baked), find(&unedited))
    };
    let (e, u) = at((3, 5, 5));
    assert!(e != u, "the edited tile did not change");
    let (e, u) = at((0, 0, 0));
    assert!(e != u, "the overview tile did not change");
    // The outer ring of full-resolution tiles holds every edge pixel.
    for i in 0..8 {
        for zxy in [(3, i, 0), (3, i, 7), (3, 0, i), (3, 7, i)] {
            let (e, u) = at(zxy);
            assert!(e == u, "edge tile {zxy:?} changed");
        }
    }
}

#[tokio::test]
async fn the_job_api_is_idempotent_bounded_and_validated() {
    let s = fresh(1);
    s.bake.hold(true);
    let layout = cell_layout(&s, RIVER).await;
    let first = post(&s, &body(&layout)).await;
    assert_eq!(first.status, StatusCode::ACCEPTED);
    let first = first.json();
    let id = first["job_id"].as_str().unwrap().to_owned();
    assert_eq!(first["status_url"], format!("/v1/tactical/bake/{id}"));
    let again = post(&s, &body(&layout)).await.json();
    assert_eq!(again["job_id"], id.as_str());
    let (b, i) = (Arc::clone(&s), id.clone());
    assert!(
        tokio::task::spawn_blocking(move || b.bake.wait_state(&i, BakeState::Running, WAIT))
            .await
            .unwrap()
    );
    let not_done = fetch(&s, &format!("/v1/tactical/bake/{id}/tiles/0/0/0.webp")).await;
    assert_eq!(not_done.status, StatusCode::NOT_FOUND);
    // One running, QUEUE_JOBS waiting, then busy.
    let mut ids = Vec::new();
    for k in 0..=QUEUE_JOBS {
        let mut edited = layout.clone();
        edited["squares"][0]["elevation_ft"] = json!(1000 + k);
        let r = post(&s, &body(&edited)).await;
        if k < QUEUE_JOBS {
            assert_eq!(r.status, StatusCode::ACCEPTED, "{k}");
            assert_eq!(r.json()["state"], "queued");
            ids.push(r.json()["job_id"].as_str().unwrap().to_owned());
        } else {
            assert_eq!(r.status, StatusCode::SERVICE_UNAVAILABLE);
            assert_eq!(r.error_code(), "busy");
        }
    }
    for queued in &ids {
        let r = delete(&s, &format!("/v1/tactical/bake/{queued}")).await;
        assert_eq!(r.status, StatusCode::NO_CONTENT);
    }
    let r = delete(&s, &format!("/v1/tactical/bake/{id}")).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let gone = fetch(&s, &format!("/v1/tactical/bake/{id}")).await;
    assert_eq!(gone.status, StatusCode::NOT_FOUND);
    assert_eq!(gone.error_code(), "not_found");
    let r = delete(&s, &format!("/v1/tactical/bake/{id}")).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    let unknown = fetch(&s, "/v1/tactical/bake/0123456789abcdef0123456789abcdef").await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    s.bake.hold(false);
    let b = Arc::clone(&s);
    assert!(tokio::task::spawn_blocking(move || b.bake.wait_idle(WAIT))
        .await
        .unwrap());
    // The deleted running job was dropped, not stored.
    assert_eq!(
        fetch(&s, &format!("/v1/tactical/bake/{id}")).await.status,
        StatusCode::NOT_FOUND
    );

    // Validation.
    let text = Request::post("/v1/tactical/bake")
        .header("content-type", "text/plain")
        .body(Body::from(serde_json::to_vec(&body(&layout)).unwrap()))
        .unwrap();
    let r = send(Arc::clone(&s), text).await;
    assert_eq!(r.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let mut narrow = layout.clone();
    narrow["width"] = json!(63);
    let squares: Vec<Value> = (0..64)
        .flat_map(|y| (0..63).map(move |x| (x, y)))
        .map(|(x, y)| layout["squares"][y * 64 + x].clone())
        .collect();
    narrow["squares"] = json!(squares);
    let r = post(&s, &body(&narrow)).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.error_code(), "invalid_layout");
    let mut moved = layout.clone();
    moved["origin"] = json!([0, 0]);
    assert_eq!(post(&s, &body(&moved)).await.error_code(), "invalid_layout");
    let mut asset = layout.clone();
    asset["placements"]
        .as_array_mut()
        .unwrap()
        .push(json!({"asset": {"id": "prop.no_such_thing"}, "x": 10.5, "y": 10.5}));
    let r = post(&s, &body(&asset)).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.error_code(), "invalid_layout");
    let mut sea = layout.clone();
    sea["origin"] = json!([0, 0]);
    let r = post(&s, &json!({"gx": 0, "gy": 0, "ppsq": PPSQ, "layout": sea})).await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.error_code(), "no_block");
    let mut extra = body(&layout);
    extra["colour"] = json!("red");
    assert_eq!(post(&s, &extra).await.error_code(), "bad_request");
    let mut ppsq = body(&layout);
    ppsq["ppsq"] = json!(16);
    assert_eq!(post(&s, &ppsq).await.error_code(), "bad_request");
}

#[tokio::test]
async fn baking_can_be_switched_off() {
    let s = fresh(0);
    let layout = cell_layout(&s, RIVER).await;
    let r = post(&s, &body(&layout)).await;
    assert_eq!(r.status, StatusCode::NOT_IMPLEMENTED);
    assert_eq!(r.error_code(), "not_implemented");
}
