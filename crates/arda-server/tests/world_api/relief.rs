//! `/v1/tiles/relief`: mid-zoom relief levels past the overview pyramid.

use crate::support::get;
use axum::http::StatusCode;

fn decode(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut d = image_webp::WebPDecoder::new(std::io::Cursor::new(bytes)).unwrap();
    let (w, h) = d.dimensions();
    let mut buf = vec![0; d.output_buffer_size().unwrap()];
    d.read_image(&mut buf).unwrap();
    (w, h, buf)
}

#[tokio::test]
async fn relief_levels_continue_the_overview_pyramid() {
    let world = get("/v1/world").await.json();
    assert_eq!(world["tiles"]["relief_max_zoom"], 9);
    // Level 5 (25 m/px): the mountain tile at 88 km E, 153.5 km S.
    let uri = "/v1/tiles/relief/5/13/23.webp";
    let tile = get(uri).await;
    assert_eq!(tile.status, StatusCode::OK, "{uri}");
    assert_eq!(tile.headers["content-type"], "image/webp");
    let (w, h, rgba) = decode(&tile.body);
    assert_eq!((w, h), (256, 256));
    assert!(rgba.chunks(4).all(|p| p[3] == 255), "land tile is opaque");
    assert_eq!(get(uri).await.body, tile.body, "cached and deterministic");
    for bad in [
        "/v1/tiles/relief/1/0/0.webp",
        "/v1/tiles/relief/10/0/0.webp",
        "/v1/tiles/relief/5/32/0.webp",
        "/v1/tiles/relief/5/13/23.png",
    ] {
        let r = get(bad).await;
        assert_eq!(
            (r.status, r.error_code().as_str()),
            (StatusCode::NOT_FOUND, "not_found"),
            "{bad}"
        );
    }
}
