//! Tactical PNGs and WebP tiles: headers, cache hits, ETags, determinism,
//! pyramid bounds and cached latency.

use crate::support::{get, library_dir, send, state, Reply};
use arda_server::tactical::{Anchor, Tactical, TacticalLimits};
use arda_server::tiles::decode_rgb;
use arda_tactical::layouts;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use std::time::{Duration, Instant};

fn webp_rgb(bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut d = image_webp::WebPDecoder::new(std::io::Cursor::new(bytes)).unwrap();
    let (w, h) = d.dimensions();
    let channels = if d.has_alpha() { 4 } else { 3 };
    let mut buf = vec![0; d.output_buffer_size().unwrap()];
    d.read_image(&mut buf).unwrap();
    let rgb = buf
        .chunks(channels)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect();
    (w, h, rgb)
}

fn header<'a>(r: &'a Reply, name: &str) -> &'a str {
    r.headers[name].to_str().unwrap()
}

#[tokio::test]
async fn layout_png_is_cached_deterministic_and_etagged() {
    let uri = "/v1/tactical/layout/stone_warehouse.png?ppsq=64&grid=1";
    let first = get(uri).await;
    assert_eq!(first.status, StatusCode::OK);
    assert_eq!(header(&first, "content-type"), "image/png");
    assert_eq!(header(&first, "cache-control"), "no-cache");
    assert!(header(&first, "server-timing").starts_with("tactical;dur="));
    let img = decode_rgb(&first.body).unwrap();
    assert_eq!((img.width, img.height), (18 * 64, 13 * 64));
    let second = get(uri).await;
    assert_eq!(header(&second, "x-arda-cache"), "hit");
    assert_eq!(second.body, first.body);
    let etag = header(&first, "etag");
    assert_eq!(header(&second, "etag"), etag);
    assert!(etag.starts_with('"') && etag.len() == 34);
    let without_grid = get("/v1/tactical/layout/stone_warehouse.png?ppsq=64&grid=0").await;
    assert_ne!(without_grid.body, first.body, "grid is part of the key");
    let request = Request::get(uri)
        .header("if-none-match", etag)
        .body(Body::empty())
        .unwrap();
    let revalidated = send(state(), request).await;
    assert_eq!(revalidated.status, StatusCode::NOT_MODIFIED);
    assert!(revalidated.body.is_empty());
    for bad in ["?ppsq=100", "?ppsq=x", "?grid=2", "?grid=true"] {
        let r = get(&format!("/v1/tactical/layout/stone_warehouse.png{bad}")).await;
        assert_eq!(r.error_code(), "bad_request", "{bad}");
    }
}

fn render(world_seed: u64, anchor: Anchor) -> Vec<u8> {
    let t = Tactical::open(&library_dir(), world_seed, TacticalLimits::default()).unwrap();
    let l = layouts::by_name("wall_junctions").unwrap();
    let key = t.key(&l, anchor, 64, false).unwrap();
    t.png(&l, &key).unwrap().0.bytes.to_vec()
}

#[test]
fn renders_are_byte_identical_across_fresh_caches_and_seeded_by_world_and_anchor() {
    let name = || Anchor::Name("wall_junctions".into());
    let base = render(42, name());
    assert_eq!(base, render(42, name()));
    assert_ne!(base, render(43, name()), "the world seed reseeds the art");
    assert_ne!(
        base,
        render(42, Anchor::Origin(640, 128)),
        "a world origin reseeds the art"
    );
}

#[tokio::test]
async fn tiles_are_webp_bounded_and_match_the_render() {
    // riverside at 64 ppsq is 1920 × 1088: max zoom 2 with a 4 × 3 grid there.
    let base = "/v1/tactical/layout/riverside";
    let t = get(&format!("{base}/tiles/2/0/0.webp?ppsq=64")).await;
    assert_eq!(t.status, StatusCode::OK);
    assert_eq!(header(&t, "content-type"), "image/webp");
    assert_eq!(&t.body[..4], b"RIFF");
    assert_eq!(&t.body[8..12], b"WEBP");
    let (w, h, tile) = webp_rgb(&t.body);
    assert_eq!((w, h), (512, 512));
    let png = decode_rgb(&get(&format!("{base}.png?ppsq=64")).await.body).unwrap();
    assert_eq!((png.width, png.height), (1920, 1088));
    for row in [0_usize, 200, 511] {
        let src = &png.pixels[row * 1920 * 3..row * 1920 * 3 + 512 * 3];
        assert_eq!(&tile[row * 512 * 3..(row + 1) * 512 * 3], src, "row {row}");
    }
    let again = get(&format!("{base}/tiles/2/0/0.webp?ppsq=64")).await;
    assert_eq!(header(&again, "x-arda-cache"), "hit");
    assert_eq!(again.body, t.body);
    for ok in ["0/0/0", "1/1/1", "2/3/2"] {
        let r = get(&format!("{base}/tiles/{ok}.webp?ppsq=64")).await;
        assert_eq!(r.status, StatusCode::OK, "{ok}");
    }
    for outside in ["3/0/0", "2/4/0", "2/0/3", "1/2/0", "0/1/0", "0/0/1"] {
        let r = get(&format!("{base}/tiles/{outside}.webp?ppsq=64")).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{outside}");
        assert_eq!(r.error_code(), "not_found");
    }
    let png_suffix = get(&format!("{base}/tiles/0/0/0.png")).await;
    assert_eq!(png_suffix.status, StatusCode::NOT_FOUND);
    let unknown = get("/v1/tactical/layout/castle/tiles/0/0/0.webp").await;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    assert_eq!(
        get(&format!("{base}/tiles/a/0/0.webp")).await.error_code(),
        "bad_request"
    );
}

#[tokio::test]
async fn a_cached_png_returns_in_under_5_ms() {
    let uri = "/v1/tactical/layout/riverside.png";
    assert_eq!(get(uri).await.status, StatusCode::OK);
    let mut best = Duration::MAX;
    for _ in 0..5 {
        let start = Instant::now();
        let r = get(uri).await;
        best = best.min(start.elapsed());
        assert_eq!(header(&r, "x-arda-cache"), "hit");
    }
    assert!(
        best < Duration::from_millis(5),
        "cached request took {best:?}"
    );
}
