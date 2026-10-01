//! Tactical prefetch (goal 67): neighbours warm in the background, the
//! request answers at once, and warmed responses equal cold ones.

use crate::support::{config, get, send, Reply};
use arda_server::tactical::block::BlockRequest;
use arda_server::tactical::Anchor;
use arda_server::AppState;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};

const WORLD: (i64, i64) = (1024 * 64, 2048 * 64);

fn fresh(workers: usize) -> Arc<AppState> {
    let mut c = config();
    c.prefetch_workers = workers;
    Arc::new(AppState::open(&c).unwrap())
}

async fn post(state: &Arc<AppState>, body: &Value) -> Reply {
    let request = Request::post("/v1/tactical/prefetch")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap()))
        .unwrap();
    send(Arc::clone(state), request).await
}

async fn fetch(state: &Arc<AppState>, uri: &str) -> Reply {
    send(
        Arc::clone(state),
        Request::get(uri).body(Body::empty()).unwrap(),
    )
    .await
}

#[tokio::test]
async fn neighbours_warm_in_the_background_without_changing_bytes() {
    let state = fresh(2);
    let (w, h) = state.query.cells();
    assert_eq!((i64::from(w) * 64, i64::from(h) * 64), WORLD);
    let start = Instant::now();
    let r = post(&state, &json!({"gx": 529, "gy": 812, "ppsq": 16})).await;
    assert_eq!(
        r.status,
        StatusCode::ACCEPTED,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    assert!(
        start.elapsed() < Duration::from_millis(500),
        "the request must not wait"
    );
    let body = r.json();
    let cells = body["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 8);
    assert_eq!(
        (cells[0]["gx"].as_u64(), cells[0]["gy"].as_u64()),
        (Some(528), Some(811))
    );
    assert!(cells
        .iter()
        .all(|c| c["status"] == "queued" && c["image"] == true));
    // A repeat is deduplicated against cells still queued or warming.
    let again = post(&state, &json!({"gx": 529, "gy": 812, "ppsq": 16}))
        .await
        .json();
    assert!(again["cells"]
        .as_array()
        .unwrap()
        .iter()
        .all(|c| c["status"] == "pending" || c["status"] == "queued"));
    let s = Arc::clone(&state);
    assert!(
        tokio::task::spawn_blocking(move || s.prefetch.wait_idle(Duration::from_secs(600)))
            .await
            .unwrap()
    );
    // Every neighbour's body and image pyramid is warm; the centre is not.
    for c in cells {
        let (gx, gy) = (c["gx"].as_u64().unwrap(), c["gy"].as_u64().unwrap());
        let json = fetch(&state, &format!("/v1/tactical/cell/{gx}/{gy}?ppsq=16")).await;
        assert_eq!(json.headers["x-arda-cache"], "hit", "{gx},{gy}");
        let req = BlockRequest::cell(u32::try_from(gx).unwrap(), u32::try_from(gy).unwrap());
        assert!(
            state.tactical.image_warm(&req, WORLD, 16).unwrap(),
            "{gx},{gy}"
        );
    }
    assert!(!state
        .tactical
        .image_warm(&BlockRequest::cell(529, 812), WORLD, 16)
        .unwrap());
    // Warmed responses are the cold ones, byte for byte.
    for uri in [
        "/v1/tactical/cell/530/812?ppsq=16",
        "/v1/tactical/cell/530/812/tiles/0/0/0.webp?ppsq=16",
        "/v1/tactical/cell/528/813.png?ppsq=16",
    ] {
        let warm = fetch(&state, uri).await;
        let cold = get(uri).await;
        assert_eq!(warm.status, StatusCode::OK, "{uri}");
        assert_eq!(warm.body, cold.body, "{uri}");
    }
}

#[tokio::test]
async fn the_cache_key_carries_seed_coordinates_and_catalogue_version() {
    let state = crate::support::state();
    let t = &state.tactical;
    let (_, a) = t
        .world_render(&BlockRequest::cell(529, 812), WORLD, 16, false)
        .unwrap();
    let (_, b) = t
        .world_render(&BlockRequest::cell(530, 812), WORLD, 16, false)
        .unwrap();
    assert_eq!(a.world_seed, 42);
    assert_eq!(a.anchor, Anchor::of_cell(529, 812));
    assert_eq!(b.anchor, Anchor::of_cell(530, 812));
    assert_eq!(a.library_version, t.library_version());
    assert!(!a.library_version.is_empty());
    assert_ne!(a, b);
}

#[tokio::test]
async fn prefetch_requests_are_validated_and_bounded() {
    let state = fresh(1);
    for (body, status, code) in [
        (
            json!({"gx": 529, "gy": 812, "radius": 3}),
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        (
            json!({"gx": 529, "gy": 812, "radius": 0}),
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        (
            json!({"gx": 529, "gy": 812, "ppsq": 50}),
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        (
            json!({"gx": 1024, "gy": 0}),
            StatusCode::BAD_REQUEST,
            "out_of_range",
        ),
        (
            json!({"gx": 1, "gy": 1, "colour": 1}),
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        (json!({"gy": 1}), StatusCode::BAD_REQUEST, "bad_request"),
    ] {
        let r = post(&state, &body).await;
        assert_eq!(
            (r.status, r.error_code().as_str()),
            (status, code),
            "{body}"
        );
    }
    let plain = Request::post("/v1/tactical/prefetch")
        .body(Body::from(r#"{"gx":1,"gy":1}"#))
        .unwrap();
    assert_eq!(
        send(Arc::clone(&state), plain).await.status,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    // Radius 2 at the world's corner: the rings are clipped, nearest first,
    // and 128 px per square warms images for one neighbour only.
    let r = post(&state, &json!({"gx": 0, "gy": 0, "radius": 2, "ppsq": 128}))
        .await
        .json();
    let cells = r["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 8);
    assert_eq!(cells.iter().filter(|c| c["image"] == true).count(), 1);
    // Workers off: nothing is queued.
    let off = fresh(0);
    let r = post(&off, &json!({"gx": 529, "gy": 812})).await.json();
    assert!(r["cells"]
        .as_array()
        .unwrap()
        .iter()
        .all(|c| c["status"] == "dropped"));
    assert_eq!(r["queue_len"], 0);
}
