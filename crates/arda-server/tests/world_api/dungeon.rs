//! `/v1/tactical/dungeon`, `/dungeon.png` and `/dungeon/scene`.

use crate::support::get;
use arda_server::tiles::decode_rgb;
use axum::http::StatusCode;

#[tokio::test]
async fn dungeon_json_is_deterministic_and_complete() {
    let uri = "/v1/tactical/dungeon?seed=9&kind=dungeon&w=32&h=24";
    let a = get(uri).await;
    assert_eq!(a.status, StatusCode::OK);
    let b = get(uri).await;
    assert_eq!(a.body, b.body);
    let v = a.json();
    assert_eq!(v["layout"]["width"], 32);
    assert_eq!(v["layout"]["height"], 24);
    assert_eq!(v["rules"]["format_version"], 2);
    // logic/16 §api-conventions: u64 seeds travel as decimal strings.
    assert_eq!(v["params"]["seed"], "9");
    assert!(!v["rooms"].as_array().unwrap().is_empty());
    let exits: Vec<&str> = v["exits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["kind"].as_str().unwrap())
        .collect();
    assert!(exits.contains(&"stairs_up") && exits.contains(&"stairs_down"));
}

#[tokio::test]
async fn caves_follow_world_cells_and_render() {
    let a = get("/v1/tactical/dungeon?gx=3&gy=4&kind=cave&w=24&h=24").await;
    assert_eq!(a.status, StatusCode::OK);
    let b = get("/v1/tactical/dungeon?gx=3&gy=4&kind=cave&w=24&h=24").await;
    assert_eq!(
        a.body, b.body,
        "a world cell always opens onto the same cave"
    );
    let c = get("/v1/tactical/dungeon?gx=4&gy=4&kind=cave&w=24&h=24").await;
    assert_ne!(a.body, c.body);
    let png = get("/v1/tactical/dungeon.png?gx=3&gy=4&kind=cave&w=24&h=24&ppsq=64").await;
    assert_eq!(png.status, StatusCode::OK);
    let img = decode_rgb(&png.body).unwrap();
    assert_eq!((img.width, img.height), (24 * 64, 24 * 64));
    let scene = get("/v1/tactical/dungeon/scene?gx=3&gy=4&kind=cave&w=24&h=24").await;
    assert_eq!(scene.status, StatusCode::OK);
    assert_eq!(scene.json()["width"], 24);
}

#[tokio::test]
async fn bad_dungeon_queries_are_refused() {
    for uri in [
        "/v1/tactical/dungeon?kind=dungeon",
        "/v1/tactical/dungeon?seed=1&kind=maze",
        "/v1/tactical/dungeon?seed=1&w=8",
        "/v1/tactical/dungeon?seed=1&gx=1&gy=1",
        "/v1/tactical/dungeon.png?seed=1&ppsq=7",
    ] {
        assert_eq!(get(uri).await.status, StatusCode::BAD_REQUEST, "{uri}");
    }
}

/// Review 3.6-4 for dungeons: the scene names the seed the image is drawn
/// with (the render seed of the level's name), so it resolves the same
/// assets the image shows.
#[tokio::test]
async fn the_dungeon_scene_uses_the_image_seed() {
    let q = "seed=9&kind=dungeon&w=32&h=24";
    let d = get(&format!("/v1/tactical/dungeon?{q}")).await.json();
    let name = d["layout"]["name"].as_str().unwrap().to_owned();
    let scene = get(&format!("/v1/tactical/dungeon/scene?{q}")).await.json();
    let state = crate::support::state();
    let want = arda_server::tactical::key::render_seed(
        state.tactical.world_seed(),
        &arda_server::tactical::Anchor::Name(name),
    );
    assert_eq!(scene["seed"], want.to_string());
}
