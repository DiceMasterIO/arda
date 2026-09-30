//! `/v1/tactical` JSON endpoints, `POST /render` limits and validation, the
//! block-source seam and library validation at startup.

use crate::support::{config, get, library_dir, post, send, state};
use arda_server::tactical::block::{Block, BlockRequest};
use arda_server::tactical::{Anchor, TacticalLimits};
use arda_server::{AppState, BlockError, BlockSource, ServerError, Tactical};
use arda_tactical::{layouts, Library, TacticalLayout};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::sync::Arc;

#[tokio::test]
async fn layouts_lists_every_built_in_layout_with_its_size() {
    let r = get("/v1/tactical/layouts").await;
    assert_eq!(r.status, StatusCode::OK);
    let j = r.json();
    assert_eq!(j["default_ppsq"], 128);
    assert_eq!(j["world_seed"], "42");
    assert_eq!(j["ppsq_options"], serde_json::json!([64, 96, 128]));
    assert!(j["library_version"].as_str().is_some_and(|v| !v.is_empty()));
    let names: Vec<&str> = j["layouts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["name"].as_str().unwrap())
        .collect();
    let mut expected: Vec<String> = layouts::all().into_iter().map(|l| l.name).collect();
    expected.sort();
    assert_eq!(names, expected);
    assert!(names.contains(&"riverside"));
    let river = j["layouts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["name"] == "riverside")
        .unwrap();
    assert_eq!(
        (river["width"].as_u64(), river["height"].as_u64()),
        (Some(30), Some(17))
    );
    let tiles = &river["tiles"];
    assert_eq!(tiles["tile_px"], 512);
    assert_eq!(tiles["image_width_px"], 3840);
    assert_eq!(tiles["image_height_px"], 2176);
    assert_eq!(tiles["max_zoom"], 3);
    assert_eq!(tiles["format"], "webp");
}

#[tokio::test]
async fn layout_json_is_the_tactical_layout() {
    let r = get("/v1/tactical/layout/timber_house").await;
    assert_eq!(r.status, StatusCode::OK);
    // Compare as layouts: f32 positions differ between text and `Value` forms.
    let back: TacticalLayout = serde_json::from_slice(&r.body).unwrap();
    assert_eq!(back, layouts::by_name("timber_house").unwrap());
    let missing = get("/v1/tactical/layout/castle").await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert_eq!(missing.error_code(), "not_found");
    assert_eq!(
        get("/v1/tactical/layout/castle.png").await.error_code(),
        "not_found"
    );
}

#[tokio::test]
async fn a_pending_source_is_a_typed_not_yet_naming_arda_refine() {
    let pending = tokio::task::spawn_blocking(|| {
        AppState::open(&config())
            .unwrap()
            .with_block_source(Box::new(arda_server::tactical::block::PendingBlocks))
    })
    .await
    .unwrap();
    let pending = Arc::new(pending);
    let request = |uri: &str| Request::get(uri).body(Body::empty()).unwrap();
    let r = send(Arc::clone(&pending), request("/v1/tactical/cell/512/1036")).await;
    assert_eq!(r.status, StatusCode::NOT_IMPLEMENTED);
    let j = r.json();
    assert_eq!(j["error"]["code"], "not_implemented");
    assert_eq!(j["error"]["status"], 501);
    assert_eq!(j["planned_source"], "arda-refine");
    let outside = send(Arc::clone(&pending), request("/v1/tactical/cell/1024/0")).await;
    assert_eq!(outside.status, StatusCode::BAD_REQUEST);
    assert_eq!(outside.error_code(), "out_of_range");
    assert_eq!(
        send(pending, request("/v1/tactical/cell/x/0"))
            .await
            .error_code(),
        "bad_request"
    );
}

#[derive(Debug)]
struct Fixed;

impl BlockSource for Fixed {
    fn block(&self, req: &BlockRequest, _lib: &Library) -> Result<Block, BlockError> {
        let (gx, gy) = (
            u32::try_from(req.gsx0 / 64).unwrap(),
            u32::try_from(req.gsy0 / 64).unwrap(),
        );
        if gx == 0 {
            return Err(BlockError::NoBlock {
                gx,
                gy,
                reason: "open sea".into(),
            });
        }
        let mut l = layouts::by_name("wall_junctions").ok_or(BlockError::Failed("none".into()))?;
        l.name = format!("block_{gx}_{gy}");
        let Anchor::Origin(x, y) = Anchor::of_cell(gx, gy) else {
            return Err(BlockError::Failed("origin".into()));
        };
        let rules = arda_scene::RulesSidecar::empty(l.width, l.height);
        l.origin = Some([x, y]);
        Ok(Block {
            layout: l,
            rules: Some(rules),
            origin: [x, y],
            meta: [("source".to_owned(), "test".to_owned())].into(),
        })
    }
}

#[tokio::test]
async fn a_plugged_block_source_serves_through_the_same_route() {
    let plugged = tokio::task::spawn_blocking(|| {
        AppState::open(&config())
            .unwrap()
            .with_block_source(Box::new(Fixed))
    })
    .await
    .unwrap();
    let plugged = Arc::new(plugged);
    let request = |uri: &str| Request::get(uri).body(Body::empty()).unwrap();
    let r = send(Arc::clone(&plugged), request("/v1/tactical/cell/7/9")).await;
    assert_eq!(r.status, StatusCode::OK);
    let j = r.json();
    assert_eq!(j["layout"]["name"], "block_7_9");
    assert_eq!(j["origin"], serde_json::json!([448, 576]));
    assert_eq!(j["rules"]["format_version"], 2);
    assert_eq!(j["meta"]["source"], "test");
    let sea = send(plugged, request("/v1/tactical/cell/0/9")).await;
    assert_eq!(sea.status, StatusCode::NOT_FOUND);
    assert_eq!(sea.error_code(), "no_block");
}

fn riverside_json() -> Vec<u8> {
    serde_json::to_vec(&layouts::by_name("riverside").unwrap()).unwrap()
}

#[tokio::test]
async fn posted_render_matches_the_named_render() {
    let named = get("/v1/tactical/layout/riverside.png?ppsq=64").await;
    let posted = post("/v1/tactical/render?ppsq=64", riverside_json()).await;
    assert_eq!(posted.status, StatusCode::OK);
    assert_eq!(posted.headers["content-type"], "image/png");
    assert_eq!(posted.headers["cache-control"], "no-cache");
    assert_eq!(posted.body, named.body, "same layout hash, same bytes");
    assert_eq!(posted.headers["etag"], named.headers["etag"]);
    assert_eq!(posted.headers["x-arda-cache"], "hit");
    let anchored = post(
        "/v1/tactical/render?ppsq=64&origin=640,128",
        riverside_json(),
    )
    .await;
    assert_eq!(anchored.status, StatusCode::OK);
    assert_eq!(
        anchored.headers["x-arda-cache"], "miss",
        "the origin is part of the key"
    );
    assert_ne!(anchored.body, named.body, "the origin reseeds the art");
    let bad = post("/v1/tactical/render?origin=1", riverside_json()).await;
    assert_eq!(bad.error_code(), "bad_request");
    let on_get = get("/v1/tactical/layout/riverside.png?origin=0,0").await;
    assert_eq!(on_get.error_code(), "bad_request");
}

#[tokio::test]
async fn posted_render_validates_the_layout() {
    let garbage = post("/v1/tactical/render", b"{not json".to_vec()).await;
    assert_eq!(garbage.status, StatusCode::BAD_REQUEST);
    assert_eq!(garbage.error_code(), "bad_request");
    let unknown_field = post(
        "/v1/tactical/render",
        br#"{"name":"x","width":1,"height":1,"squares":[{"ground":"grass"}],"roof":1}"#.to_vec(),
    )
    .await;
    assert_eq!(unknown_field.error_code(), "bad_request");
    let lava = serde_json::to_vec(&TacticalLayout::new("lava", 2, 2, "lava")).unwrap();
    let r = post("/v1/tactical/render", lava).await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.error_code(), "invalid_layout");
    assert!(r.json()["error"]["message"]
        .as_str()
        .unwrap()
        .contains("lava"));
    let mut short = TacticalLayout::new("short", 3, 3, "grass");
    short.squares.pop();
    let r = post("/v1/tactical/render", serde_json::to_vec(&short).unwrap()).await;
    assert_eq!(r.error_code(), "invalid_layout");
    let bad_ppsq = post("/v1/tactical/render?ppsq=100", riverside_json()).await;
    assert_eq!(bad_ppsq.error_code(), "bad_request");
}

#[tokio::test]
async fn posted_render_enforces_the_size_limits() {
    let limit = TacticalLimits::default().max_body_bytes;
    let mut body = riverside_json();
    body.resize(limit + 1, b' ');
    let r = post("/v1/tactical/render", body).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(r.error_code(), "payload_too_large");
    // 128 × 128 squares fit the square limit but not the pixel limit at 128 ppsq.
    let wide = serde_json::to_vec(&TacticalLayout::new("wide", 128, 128, "grass")).unwrap();
    assert!(wide.len() < limit);
    let r = post("/v1/tactical/render?ppsq=128", wide).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(r.json()["error"]["message"]
        .as_str()
        .unwrap()
        .contains("px"));
    let huge = serde_json::to_vec(&TacticalLayout::new("huge", 210, 210, "grass")).unwrap();
    let r = post("/v1/tactical/render?ppsq=64", huge).await;
    assert_eq!(r.error_code(), "payload_too_large");
    assert!(r.json()["error"]["message"]
        .as_str()
        .unwrap()
        .contains("squares"));
}

#[tokio::test]
async fn cors_preflight_allows_posting_layouts_from_localhost() {
    let request = Request::options("/v1/tactical/render")
        .header("origin", "http://localhost:5173")
        .header("access-control-request-method", "POST")
        .header("access-control-request-headers", "content-type")
        .body(Body::empty())
        .unwrap();
    let r = send(state(), request).await;
    let methods = r.headers["access-control-allow-methods"].to_str().unwrap();
    assert!(methods.contains("POST"), "{methods}");
}

#[test]
fn an_invalid_library_fails_startup() {
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("broken-library");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // The real catalogue without its images: every asset fails validation.
    std::fs::copy(library_dir().join("catalog.json"), dir.join("catalog.json")).unwrap();
    let err = Tactical::open(&dir, 42, TacticalLimits::default()).unwrap_err();
    assert!(
        matches!(
            &err,
            ServerError::Tactical(arda_tactical::TacticalError::Invalid(_))
        ),
        "{err}"
    );
    let mut cfg = config();
    cfg.library = dir.join("missing");
    assert!(matches!(
        AppState::open(&cfg),
        Err(ServerError::Tactical(_))
    ));
}

fn post_as(uri: &str, content_type: &str, body: Vec<u8>) -> Request<Body> {
    Request::post(uri)
        .header("content-type", content_type)
        .body(Body::from(body))
        .unwrap()
}

#[tokio::test]
async fn posted_render_requires_a_json_content_type() {
    // A text/plain POST is a CORS "simple request": any web page could fire
    // renders at the loopback service without a preflight.
    let r = send(
        state(),
        post_as(
            "/v1/tactical/render?ppsq=64",
            "text/plain",
            riverside_json(),
        ),
    )
    .await;
    assert_eq!(r.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    assert_eq!(r.error_code(), "unsupported_media_type");
    let ok = send(
        state(),
        post_as(
            "/v1/tactical/render?ppsq=64",
            "application/json; charset=utf-8",
            riverside_json(),
        ),
    )
    .await;
    assert_eq!(ok.status, StatusCode::OK);
}

#[tokio::test]
async fn posted_render_refuses_work_the_size_limits_do_not_see() {
    use arda_tactical::layout::LightSource;
    // Thousands of whole-map lights in a small body: hours of lighting work.
    let mut lit = TacticalLayout::new("lit", 76, 76, "grass");
    lit.lights = vec![
        LightSource {
            x: 0.0,
            y: 0.0,
            radius_ft: u16::MAX,
            colour: [0, 0, 0],
        };
        10_000
    ];
    let r = post(
        "/v1/tactical/render?ppsq=64",
        serde_json::to_vec(&lit).unwrap(),
    )
    .await;
    assert_eq!(r.error_code(), "payload_too_large");
    // A light far off the map overflows the compositor's pixel arithmetic.
    let mut far = TacticalLayout::new("far", 4, 4, "grass");
    far.lights.push(LightSource {
        x: 1.0,
        y: 1e30,
        radius_ft: 10,
        colour: [255, 200, 120],
    });
    let r = post(
        "/v1/tactical/render?ppsq=64",
        serde_json::to_vec(&far).unwrap(),
    )
    .await;
    assert_eq!(r.error_code(), "invalid_layout");
    // A megabyte name keys the cache (uncharged) and floods logs.
    let named = TacticalLayout::new(&"n".repeat(1 << 20), 1, 1, "grass");
    let r = post(
        "/v1/tactical/render?ppsq=64",
        serde_json::to_vec(&named).unwrap(),
    )
    .await;
    assert_eq!(r.error_code(), "invalid_layout");
    let forged = TacticalLayout::new("a\nforged log line", 1, 1, "grass");
    let r = post(
        "/v1/tactical/render?ppsq=64",
        serde_json::to_vec(&forged).unwrap(),
    )
    .await;
    assert_eq!(r.error_code(), "invalid_layout");
    // The built-in layouts still pass.
    let s = state();
    for l in layouts::all() {
        s.tactical.admit(&l, 64).unwrap();
    }
}
