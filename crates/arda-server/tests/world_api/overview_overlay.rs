//! Task 15: `?overlay=society|none` on `/v1/overview.png` and the overview
//! tiles cut from it.

use crate::support::{get_with, society_dir, society_state};
use arda::{ImageQuality, MapStyle, OverviewLook, World};
use arda_server::tiles::decode_rgb;
use axum::http::StatusCode;

/// The library's plain Atlas overview of the society world at 512.
fn plain_bytes() -> Vec<u8> {
    let dir = std::env::temp_dir().join(format!("arda-overlay-plain-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let world = World::load(society_dir()).unwrap();
    let path = arda::export_overview_with_look(
        &world,
        &dir,
        ImageQuality::new(512).unwrap(),
        MapStyle::Atlas,
        OverviewLook::default(),
    )
    .unwrap();
    let bytes = std::fs::read(path).unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    bytes
}

#[tokio::test]
async fn overlay_none_is_the_plain_export_and_society_only_adds_pixels() {
    let state = society_state();
    let uri = |o: &str| format!("/v1/overview.png?quality=512{o}");
    let none = get_with(state.clone(), &uri("&overlay=none"), None).await;
    assert_eq!(none.status, StatusCode::OK);
    assert_eq!(
        none.body,
        plain_bytes(),
        "overlay=none is the plain overview"
    );

    let soc = get_with(state.clone(), &uri("&overlay=society"), None).await;
    assert_eq!(soc.status, StatusCode::OK);
    assert_ne!(soc.body, none.body);
    // Absent, a world with society serves it; the bytes are deterministic.
    let default = get_with(state.clone(), &uri(""), None).await;
    assert_eq!(default.body, soc.body);
    let again = get_with(state.clone(), &uri("&overlay=society"), None).await;
    assert_eq!(again.body, soc.body);

    let (a, b) = (
        decode_rgb(&none.body).unwrap(),
        decode_rgb(&soc.body).unwrap(),
    );
    assert_eq!((a.width, a.height), (b.width, b.height));
    let changed = a
        .pixels
        .chunks_exact(3)
        .zip(b.pixels.chunks_exact(3))
        .filter(|(x, y)| x != y)
        .count();
    let total = (a.width * a.height) as usize;
    assert!(changed > 0, "the society layer painted nothing");
    assert!(changed < total / 2, "{changed} of {total} pixels changed");
    // The sea is untouched: pixels deep in the north-west corner agree.
    assert_eq!(a.pixels[..300], b.pixels[..300]);
}

#[tokio::test]
async fn overlay_values_are_validated_and_tiles_follow_the_overlay() {
    let state = society_state();
    let bad = get_with(
        state.clone(),
        "/v1/overview.png?quality=512&overlay=roads",
        None,
    )
    .await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
    let oblique = get_with(
        state.clone(),
        "/v1/overview.png?quality=512&style=atlas-oblique&overlay=society",
        None,
    )
    .await;
    assert_eq!(oblique.status, StatusCode::BAD_REQUEST);
    let tile = |o: &str| format!("/v1/tiles/overview/1/0/1.png?labels=0{o}");
    let none = get_with(state.clone(), &tile("&overlay=none"), None).await;
    let soc = get_with(state.clone(), &tile("&overlay=society"), None).await;
    let default = get_with(state, &tile(""), None).await;
    assert_eq!(none.status, StatusCode::OK);
    assert_ne!(none.body, soc.body);
    assert_eq!(default.body, soc.body);
}
