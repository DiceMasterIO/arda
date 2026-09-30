//! Every `/v1` endpoint through the real router: bodies, errors, tiles, determinism.

use crate::support::{config, get, get_with, state};
use arda_server::columnar::{COLUMNS, HEADER_BYTES, MAGIC};
use arda_server::AppState;
use axum::http::StatusCode;
use std::sync::Arc;

#[tokio::test]
async fn health_and_world_describe_the_served_world() {
    let health = get("/v1/health").await;
    assert_eq!(health.status, StatusCode::OK);
    assert_eq!(health.json()["seed"], "42");
    let world = get("/v1/world").await.json();
    assert_eq!(world["contract_version"], 2);
    assert_eq!(world["api_version"], "v1");
    assert_eq!(
        (world["areas_wide"].as_i64(), world["areas_high"].as_i64()),
        (Some(2), Some(4))
    );
    assert_eq!(
        (world["cells_wide"].as_u64(), world["cells_high"].as_u64()),
        (Some(1024), Some(2048))
    );
    assert_eq!(world["fine_terrain"]["recipe_version"], 5);
    assert_eq!(world["tiles"]["max_zoom"], 1);
    assert_eq!(world["tiles"]["image_width_px"], 256);
}

#[tokio::test]
async fn cell_and_point_return_contract_json() {
    // An order-4 river cell of MICRO seed 42 on variant-Q terrain.
    let cell = get("/v1/cell/512/1039").await;
    assert_eq!(cell.status, StatusCode::OK);
    let c = cell.json();
    assert_eq!(
        (c["ax"].as_i64(), c["ay"].as_i64(), c["cy"].as_u64()),
        (Some(1), Some(2), Some(15))
    );
    assert_eq!(c["terrain"], "land");
    assert!(
        c["river"]["global_reach_id"].is_string(),
        "a saved order-4 river runs here"
    );
    let point = get("/v1/point?x_m=51234.5&y_m=103917.25").await.json();
    assert_eq!(point["height_source"], "fine");
    assert_eq!(point["cell"]["gx"], 512);
}

#[tokio::test]
async fn area_cells_json_and_binary_carry_the_same_columns() {
    let json = get("/v1/area/1/2/cells").await;
    assert_eq!(json.status, StatusCode::OK);
    let j = json.json();
    assert_eq!(
        (j["gx0"].as_u64(), j["gy0"].as_u64()),
        (Some(512), Some(1024))
    );
    let heights = j["columns"]["height_m"].as_array().unwrap();
    assert_eq!(heights.len(), 512 * 512);
    assert_eq!(j["legend"]["cover"][3], "forest");

    let bin = get("/v1/area/1/2/cells?format=bin").await;
    assert_eq!(bin.status, StatusCode::OK);
    assert_eq!(bin.headers["content-type"], "application/octet-stream");
    let b = &bin.body;
    assert_eq!(&b[..8], MAGIC);
    let word = |at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
    assert_eq!(
        (word(24), word(28), word(32), word(36)),
        (512, 1024, 512, 512)
    );
    assert_eq!(word(40) as usize, COLUMNS.len());
    let offset = word(44) as usize;
    assert!(offset >= HEADER_BYTES && offset.is_multiple_of(4));
    // height_m is the first column: compare a few f32 values with the JSON.
    for i in [0_usize, 12 * 512, 262_143] {
        let v = f32::from_le_bytes(b[offset + 4 * i..offset + 4 * i + 4].try_into().unwrap());
        // JSON prints the shortest f32 decimal, so compare in f32.
        #[allow(clippy::cast_possible_truncation)]
        let from_json = heights[i].as_f64().unwrap() as f32;
        assert_eq!(v.to_bits(), from_json.to_bits());
    }
    let data: usize = COLUMNS
        .iter()
        .map(|(_, d)| 512 * 512 * if d.code() == 1 { 1 } else { 4 })
        .sum();
    assert_eq!(b.len(), offset + data);
}

#[tokio::test]
async fn rivers_and_lakes_list_saved_objects_in_global_cells() {
    let rivers = get("/v1/area/1/2/rivers").await.json();
    let list = rivers["rivers"].as_array().unwrap();
    assert!(!list.is_empty());
    for r in list.iter().take(50) {
        for cell in r["course"].as_array().unwrap() {
            let gx = cell[0].as_u64().unwrap();
            let gy = cell[1].as_u64().unwrap();
            assert!((512..1024).contains(&gx) && (1024..1536).contains(&gy));
        }
        assert!(r["global_id"].is_string());
    }
    let lakes = get("/v1/area/1/2/lakes").await;
    assert_eq!(lakes.status, StatusCode::OK);
    assert!(lakes.json()["lakes"].is_array());
}

#[tokio::test]
async fn overview_png_and_tiles_are_pngs_within_the_pyramid() {
    let png = get("/v1/overview.png?quality=512").await;
    assert_eq!(png.status, StatusCode::OK);
    assert_eq!(png.headers["content-type"], "image/png");
    let image = arda_server::tiles::decode_rgb(&png.body).unwrap();
    assert_eq!((image.width, image.height), (256, 512));
    for uri in [
        "/v1/tiles/overview/0/0/0.png",
        "/v1/tiles/overview/1/0/1.png",
        "/v1/tiles/overview/1/1/1.png",
    ] {
        let tile = get(uri).await;
        assert_eq!(tile.status, StatusCode::OK, "{uri}");
        let t = arda_server::tiles::decode_rgb(&tile.body).unwrap();
        assert_eq!((t.width, t.height), (256, 256));
    }
    for uri in [
        "/v1/tiles/overview/2/0/0.png",
        "/v1/tiles/overview/0/1/0.png",
        "/v1/tiles/overview/1/2/0.png",
        "/v1/tiles/overview/1/0/2.png",
        "/v1/tiles/overview/99/0/0.png",
        "/v1/tiles/overview/0/0/0.jpg",
    ] {
        let r = get(uri).await;
        assert_eq!(
            (r.status, r.error_code().as_str()),
            (StatusCode::NOT_FOUND, "not_found"),
            "{uri}"
        );
    }
}

#[tokio::test]
async fn errors_are_typed_json_with_documented_statuses() {
    let cases = [
        ("/v1/cell/1024/0", StatusCode::BAD_REQUEST, "out_of_range"),
        ("/v1/cell/0/2048", StatusCode::BAD_REQUEST, "out_of_range"),
        ("/v1/cell/x/0", StatusCode::BAD_REQUEST, "bad_request"),
        ("/v1/cell/-1/0", StatusCode::BAD_REQUEST, "bad_request"),
        (
            "/v1/area/2/0/cells",
            StatusCode::BAD_REQUEST,
            "out_of_range",
        ),
        (
            "/v1/area/-1/0/rivers",
            StatusCode::BAD_REQUEST,
            "out_of_range",
        ),
        (
            "/v1/area/0/4/lakes",
            StatusCode::BAD_REQUEST,
            "out_of_range",
        ),
        (
            "/v1/area/0/0/cells?format=csv",
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        ("/v1/point?x_m=0", StatusCode::BAD_REQUEST, "bad_request"),
        (
            "/v1/point?x_m=NaN&y_m=0",
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        (
            "/v1/point?x_m=-1&y_m=0",
            StatusCode::BAD_REQUEST,
            "out_of_range",
        ),
        (
            "/v1/overview.png?quality=2048",
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        (
            "/v1/overview.png?style=sepia",
            StatusCode::BAD_REQUEST,
            "bad_request",
        ),
        ("/v1/nothing", StatusCode::NOT_FOUND, "not_found"),
        ("/v2/world", StatusCode::NOT_FOUND, "not_found"),
    ];
    for (uri, status, code) in cases {
        let r = get(uri).await;
        assert_eq!(r.status, status, "{uri}");
        assert_eq!(r.headers["content-type"], "application/json", "{uri}");
        assert_eq!(r.error_code(), code, "{uri}");
        assert_eq!(r.json()["error"]["status"], status.as_u16(), "{uri}");
    }
}

#[tokio::test]
async fn localhost_origins_get_cors_and_others_do_not() {
    let ok = get_with(state(), "/v1/health", Some("http://localhost:5173")).await;
    assert_eq!(
        ok.headers["access-control-allow-origin"],
        "http://localhost:5173"
    );
    let no = get_with(state(), "/v1/health", Some("https://example.com")).await;
    assert!(no.headers.get("access-control-allow-origin").is_none());
}

#[tokio::test]
async fn identical_requests_give_identical_bytes_even_from_a_cold_state() {
    let uris = [
        "/v1/world",
        "/v1/cell/700/1500",
        "/v1/point?x_m=61000.5&y_m=150000.25",
        "/v1/area/0/2/cells?format=bin",
        "/v1/area/1/1/rivers",
        "/v1/overview.png?quality=512",
        "/v1/tiles/overview/1/1/0.png",
    ];
    let cold = Arc::new(
        tokio::task::spawn_blocking(|| AppState::open(&config()).unwrap())
            .await
            .unwrap(),
    );
    for uri in uris {
        let first = get(uri).await;
        let again = get(uri).await;
        let fresh = get_with(Arc::clone(&cold), uri, None).await;
        assert_eq!(first.status, StatusCode::OK, "{uri}");
        assert!(
            first.body == again.body && first.body == fresh.body,
            "{uri} is not deterministic"
        );
    }
}

#[tokio::test]
async fn area_builds_wait_for_a_free_build_slot() {
    // Each area body is ~100 MB of transient samples; admission counts
    // `area_builds` of them, so a request must not start one past the gate.
    let state = Arc::new(AppState::open(&config()).unwrap());
    let all = u32::try_from(config().area_builds).unwrap();
    let held = state.area_builds.acquire_many(all).await.unwrap();
    let waiting = get_with(Arc::clone(&state), "/v1/area/0/0/cells?format=bin", None);
    tokio::pin!(waiting);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(500), &mut waiting)
            .await
            .is_err(),
        "an area build started while every slot was taken"
    );
    drop(held);
    let reply = tokio::time::timeout(std::time::Duration::from_secs(120), waiting)
        .await
        .unwrap();
    assert_eq!(reply.status, StatusCode::OK);
}
