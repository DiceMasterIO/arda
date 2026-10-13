//! Effect art in `GET /v1/tactical/library` and `GET
//! /v1/tactical/library/asset/{id}.png` (1.0 task 05).

use crate::support::{get, library_dir, send, state};
use arda_server::tactical::encode::encode_png;
use arda_tactical::Library;
use axum::body::Body;
use axum::http::{Request, StatusCode};

fn decode(bytes: &[u8]) -> (u32, u32, png::ColorType, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).unwrap();
    buf.truncate(info.buffer_size());
    (info.width, info.height, info.color_type, buf)
}

#[tokio::test]
async fn the_library_lists_effects_and_damage_props() {
    let j = get("/v1/tactical/library").await.json();
    let effects = j["effects"].as_array().unwrap();
    assert!(!effects.is_empty());
    let frost = effects.iter().find(|e| e["id"] == "effect.frost").unwrap();
    assert_eq!(frost["kind"], "texture");
    assert_eq!(frost["footprint"], serde_json::json!([2, 2]));
    assert_eq!(frost["variants"], 3);
    assert!(frost["tags"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t == "free:effect:texture"));
    let scorch = effects
        .iter()
        .find(|e| e["id"] == "effect.scorch_mark")
        .unwrap();
    assert_eq!(scorch["kind"], "decal");
    let ids: Vec<&str> = effects.iter().map(|e| e["id"].as_str().unwrap()).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted, "sorted by id");
    let assets = j["assets"].as_array().unwrap();
    for id in ["prop.rubble_heap", "prop.broken_deck", "prop.charred_floor"] {
        let a = assets.iter().find(|a| a["id"] == id).unwrap();
        assert!(a["tags"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t == "free:damage"));
    }
    assert!(!assets
        .iter()
        .any(|a| a["id"].as_str().unwrap().starts_with("effect.")));
}

#[tokio::test]
async fn an_asset_route_serves_the_library_image() {
    let lib = Library::load(&library_dir()).unwrap();
    for id in [
        "effect.frost.0",
        "effect.scorch_mark.2",
        "prop.barrel",
        "ground.grass.0",
    ] {
        let r = get(&format!("/v1/tactical/library/asset/{id}.png")).await;
        assert_eq!(r.status, StatusCode::OK, "{id}");
        assert_eq!(r.headers["content-type"], "image/png");
        assert_eq!(r.headers["cache-control"], "no-cache");
        let img = lib.image(id).unwrap();
        assert_eq!(
            r.body,
            encode_png(img.width, img.height, &img.data).unwrap(),
            "{id}"
        );
        let (w, h, _, _) = decode(&r.body);
        assert_eq!((w, h), (img.width, img.height));
    }
    // An effect keeps its alpha channel, pixel for pixel.
    let r = get("/v1/tactical/library/asset/effect.fire.1.png").await;
    let (_, _, colour, pixels) = decode(&r.body);
    assert_eq!(colour, png::ColorType::Rgba);
    assert_eq!(pixels, lib.image("effect.fire.1").unwrap().data);
    assert!(pixels.chunks(4).any(|p| p[3] > 0 && p[3] < 255));
}

#[tokio::test]
async fn an_asset_route_revalidates_by_etag() {
    let uri = "/v1/tactical/library/asset/effect.web.0.png";
    let first = get(uri).await;
    let etag = first.headers["etag"].to_str().unwrap().to_owned();
    assert!(etag.starts_with('"') && etag.len() == 34);
    let request = Request::get(uri)
        .header("if-none-match", &etag)
        .body(Body::empty())
        .unwrap();
    let again = send(state(), request).await;
    assert_eq!(again.status, StatusCode::NOT_MODIFIED);
    assert!(again.body.is_empty());
}

#[tokio::test]
async fn an_asset_route_refuses_unknown_and_malformed_ids() {
    let missing = get("/v1/tactical/library/asset/effect.nothing.0.png").await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert_eq!(missing.error_code(), "not_found");
    for bad in [
        "effect.frost.0",
        "Effect.Frost.png",
        "a..b.png",
        ".png",
        "a%20b.png",
    ] {
        let r = get(&format!("/v1/tactical/library/asset/{bad}")).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{bad}");
        assert_eq!(r.error_code(), "bad_request", "{bad}");
    }
}
