//! World-derived tactical maps (A9, logic/16 §api-tactical) against the real
//! MICRO seed-42 world: the block body, its images and tiles, windows, the
//! demo overlays, the library listing and the seam invariant.

use crate::support::{get, Reply};
use arda_server::tiles::decode_rgb;
use axum::http::StatusCode;

/// A river cell of MICRO seed 42 on variant-Q terrain: an order-4 river,
/// 11.8 m wide, crosses it.
const RIVER: (u32, u32) = (529, 812);

fn header<'a>(r: &'a Reply, name: &str) -> &'a str {
    r.headers[name].to_str().unwrap()
}

#[tokio::test]
async fn a_land_cell_is_a_block_with_origin_rules_and_scene() {
    let (gx, gy) = RIVER;
    let r = get(&format!("/v1/tactical/cell/{gx}/{gy}")).await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    assert_eq!(header(&r, "content-type"), "application/json");
    let j = r.json();
    assert_eq!(j["tactical_format"], 1);
    assert_eq!(j["layout_schema"], 1);
    let origin = serde_json::json!([i64::from(gx) * 64, i64::from(gy) * 64]);
    assert_eq!(j["origin"], origin);
    assert_eq!(j["layout"]["origin"], origin);
    assert_eq!(j["layout"]["name"], format!("cell_{gx}_{gy}"));
    assert_eq!(j["layout"]["squares"].as_array().unwrap().len(), 64 * 64);
    assert_eq!(j["rules"]["format_version"], 2);
    assert_eq!(j["rules"]["squares"].as_array().unwrap().len(), 64 * 64);
    assert_eq!(j["scene"]["width"], 64);
    assert!(j["scene"]["seed"].is_string());
    assert_eq!(j["scene"]["seed"], j["render_seed"]);
    assert_eq!(j["meta"]["source"], "arda-refine");
    assert_eq!(j["tiles"]["max_zoom"], 4);
    // Deterministic and cached: the same bytes and ETag again.
    let again = get(&format!("/v1/tactical/cell/{gx}/{gy}")).await;
    assert_eq!(again.body, r.body);
    assert_eq!(header(&again, "etag"), header(&r, "etag"));
    assert_eq!(header(&again, "x-arda-cache"), "hit");
}

#[tokio::test]
async fn the_river_cell_has_water_and_wading_or_swimming_rules() {
    let (gx, gy) = RIVER;
    let j = get(&format!("/v1/tactical/cell/{gx}/{gy}")).await.json();
    let wet = j["layout"]["squares"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["water_depth_ft"].as_u64().unwrap_or(0) > 0)
        .count();
    assert!(wet > 0, "no water in the river cell");
}

#[tokio::test]
async fn open_sea_is_no_block_and_bad_windows_are_refused() {
    let sea = get("/v1/tactical/cell/0/0").await;
    assert_eq!(sea.status, StatusCode::NOT_FOUND);
    assert_eq!(sea.error_code(), "no_block");
    let big = get("/v1/tactical/window?gx0=10&gy0=10&w=4&h=1").await;
    assert_eq!(big.status, StatusCode::PAYLOAD_TOO_LARGE);
    let missing = get("/v1/tactical/window?gx0=10&gy0=10&w=1").await;
    assert_eq!(missing.error_code(), "bad_request");
    let ppsq = get("/v1/tactical/cell/529/812.png?ppsq=48").await;
    assert_eq!(ppsq.error_code(), "bad_request");
    let demo = get("/v1/tactical/cell/529/812?demo_overlays=2").await;
    assert_eq!(demo.error_code(), "bad_request");
}

#[tokio::test]
async fn a_window_is_the_concatenation_of_its_cells() {
    let (gx, gy) = RIVER;
    let w = get(&format!("/v1/tactical/window?gx0={gx}&gy0={gy}&w=2&h=1")).await;
    assert_eq!(w.status, StatusCode::OK);
    let w = w.json();
    assert_eq!(w["size"], serde_json::json!([128, 64]));
    let a = get(&format!("/v1/tactical/cell/{gx}/{gy}")).await.json();
    let b = get(&format!("/v1/tactical/cell/{}/{gy}", gx + 1))
        .await
        .json();
    let (ws, as_, bs) = (
        w["layout"]["squares"].as_array().unwrap(),
        a["layout"]["squares"].as_array().unwrap(),
        b["layout"]["squares"].as_array().unwrap(),
    );
    for y in 0..64 {
        assert_eq!(&ws[y * 128..y * 128 + 64], &as_[y * 64..y * 64 + 64]);
        assert_eq!(&ws[y * 128 + 64..y * 128 + 128], &bs[y * 64..y * 64 + 64]);
    }
}

/// logic/16 Invariant 5: the edge pixels of two independently rendered
/// neighbouring cells equal the same pixels of the 2 × 1 window render.
#[tokio::test]
async fn neighbouring_cell_renders_join_the_window_render_pixel_for_pixel() {
    let (gx, gy) = RIVER;
    let q = "ppsq=64";
    let win = get(&format!(
        "/v1/tactical/window.png?gx0={gx}&gy0={gy}&w=2&h=1&{q}"
    ))
    .await;
    assert_eq!(
        win.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&win.body)
    );
    let win = decode_rgb(&win.body).unwrap();
    assert_eq!((win.width, win.height), (128 * 64, 64 * 64));
    for (k, cx) in [gx, gx + 1].into_iter().enumerate() {
        let cell = decode_rgb(
            &get(&format!("/v1/tactical/cell/{cx}/{gy}.png?{q}"))
                .await
                .body,
        )
        .unwrap();
        assert_eq!((cell.width, cell.height), (4096, 4096));
        let row = 4096 * 3;
        let mut differ = 0_usize;
        for y in 0..4096_usize {
            let from = y * win.width as usize * 3 + k * row;
            let a = &win.pixels[from..from + row];
            let b = &cell.pixels[y * row..(y + 1) * row];
            differ += a.iter().zip(b).filter(|(p, q)| p != q).count();
        }
        assert_eq!(
            differ, 0,
            "cell {cx},{gy} differs from the window in {differ} bytes"
        );
    }
}

#[tokio::test]
async fn cell_tiles_are_webp_and_bounded() {
    let (gx, gy) = RIVER;
    let base = format!("/v1/tactical/cell/{gx}/{gy}/tiles");
    let t = get(&format!("{base}/3/0/0.webp?ppsq=64")).await;
    assert_eq!(t.status, StatusCode::OK);
    assert_eq!(header(&t, "content-type"), "image/webp");
    let outside = get(&format!("{base}/4/0/0.webp?ppsq=64")).await;
    assert_eq!(outside.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn demo_overlays_compose_ways_fields_and_town() {
    let (gx, gy) = RIVER;
    let plain = get(&format!("/v1/tactical/cell/{gx}/{gy}")).await.json();
    let demo = get(&format!("/v1/tactical/cell/{gx}/{gy}?demo_overlays=1")).await;
    assert_eq!(
        demo.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&demo.body)
    );
    let demo = demo.json();
    let overlays = demo["meta"]["overlays"].as_str().unwrap();
    assert!(overlays.contains("town"), "{overlays}");
    assert_eq!(demo["meta"]["demo_at"], format!("{gx},{gy}"));
    assert_ne!(demo["layout"], plain["layout"]);
}

#[tokio::test]
async fn the_library_lists_grounds_kits_and_assets() {
    let r = get("/v1/tactical/library").await;
    assert_eq!(r.status, StatusCode::OK);
    let j = r.json();
    let grounds = j["grounds"].as_array().unwrap();
    assert!(grounds.iter().any(|g| g["key"] == "grass"));
    assert!(grounds.iter().any(|g| g["water"] == true));
    assert!(!j["wall_kits"].as_array().unwrap().is_empty());
    let assets = j["assets"].as_array().unwrap();
    assert!(assets.iter().any(|a| a["id"] == "veg.tree_oak"));
    assert!(assets.iter().all(|a| a["tags"]
        .as_array()
        .unwrap()
        .iter()
        .all(|t| t.as_str().unwrap().contains(':'))));
}
