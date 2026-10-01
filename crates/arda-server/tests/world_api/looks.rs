//! The opt-in looks: the oblique overview pyramid (goal 24, `?oblique=1`,
//! `style=atlas-oblique`) and the world grade of tactical images (goal 49,
//! `?world_grade=1`). Defaults stay byte-identical; both options are
//! deterministic, and the world grade keeps the seam invariant.

use crate::support::get;
use arda_server::tiles::decode_rgb;
use axum::http::StatusCode;

/// The MICRO seed-42 river cell of `cells.rs`.
const RIVER: (u32, u32) = (529, 812);

fn ok(uri: &str, r: &crate::support::Reply) {
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{uri}: {}",
        String::from_utf8_lossy(&r.body)
    );
}

#[tokio::test]
async fn the_oblique_pyramid_is_opt_in_and_deterministic() {
    for uri in [
        "/v1/tiles/overview/1/0/0.png",
        "/v1/tiles/overview/1/0/0.png?oblique=0",
        "/v1/tiles/overview/1/0/1.png?oblique=1",
    ] {
        ok(uri, &get(uri).await);
    }
    let plain = get("/v1/tiles/overview/1/0/1.png").await.body;
    let off = get("/v1/tiles/overview/1/0/1.png?oblique=0").await.body;
    let on = get("/v1/tiles/overview/1/0/1.png?oblique=1").await.body;
    assert_eq!(plain, off, "oblique=0 is the default pyramid");
    assert_ne!(plain, on, "the oblique pyramid differs");
    let bad = get("/v1/tiles/overview/1/0/1.png?oblique=yes").await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);

    let atlas = get("/v1/overview.png?quality=512").await;
    let oblique = get("/v1/overview.png?quality=512&style=atlas-oblique").await;
    ok("oblique overview", &oblique);
    let (a, o) = (
        decode_rgb(&atlas.body).unwrap(),
        decode_rgb(&oblique.body).unwrap(),
    );
    assert_eq!((a.width, a.height), (o.width, o.height));
    assert_ne!(a.pixels, o.pixels);
    // Coasts never move: the open-sea corner is identical.
    let w = a.width as usize;
    for y in 0..8 {
        let row = y * w * 3;
        assert_eq!(a.pixels[row..row + 24], o.pixels[row..row + 24], "row {y}");
    }
}

#[tokio::test]
async fn the_world_grade_is_opt_in_and_moves_toward_the_world_colour() {
    let (gx, gy) = RIVER;
    let base = format!("/v1/tactical/cell/{gx}/{gy}.png?ppsq=16");
    let plain = get(&base).await;
    ok(&base, &plain);
    let off = get(&format!("{base}&world_grade=0")).await;
    assert_eq!(plain.body, off.body, "world_grade=0 is the default image");
    let graded = get(&format!("{base}&world_grade=1")).await;
    ok("graded", &graded);
    assert_ne!(plain.body, graded.body);
    let bad = get(&format!("{base}&world_grade=2")).await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);

    // The relief level of one pixel per square, cropped to the cell.
    let world = get("/v1/world").await.json();
    let z = world["tiles"]["relief_max_zoom"].as_u64().unwrap();
    // MICRO: the pyramid spans the 4-area (2048-cell) long side.
    let px = (256_u32 << z) / 2048;
    assert!(px >= 32, "the deepest relief level resolves a cell");
    let (x0, y0) = (gx * px, gy * px);
    let n = px;
    let (tx, ty) = (x0 / 256, y0 / 256);
    let tile = get(&format!("/v1/tiles/relief/{z}/{tx}/{ty}.webp")).await;
    ok("relief", &tile);
    let mut d = image_webp::WebPDecoder::new(std::io::Cursor::new(&tile.body)).unwrap();
    let mut rgba = vec![0; d.output_buffer_size().unwrap()];
    d.read_image(&mut rgba).unwrap();
    let (ox, oy) = (x0 - tx * 256, y0 - ty * 256);
    let mut world_mean = [0_f64; 3];
    let mut count = 0.0;
    for y in oy..(oy + n).min(256) {
        for x in ox..(ox + n).min(256) {
            let i = ((y * 256 + x) * 4) as usize;
            for c in 0..3 {
                world_mean[c] += f64::from(rgba[i + c]);
            }
            count += 1.0;
        }
    }
    let world_mean = world_mean.map(|v| v / count);
    let mean = |bytes: &[u8]| {
        let img = decode_rgb(bytes).unwrap();
        let mut m = [0_f64; 3];
        for p in img.pixels.chunks(3) {
            for c in 0..3 {
                m[c] += f64::from(p[c]);
            }
        }
        let count = f64::from(u32::try_from(img.pixels.len() / 3).unwrap());
        m.map(|v| v / count)
    };
    let dist = |m: [f64; 3]| {
        (0..3)
            .map(|c| (m[c] - world_mean[c]).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    let (before, after) = (dist(mean(&plain.body)), dist(mean(&graded.body)));
    assert!(
        after < before * 0.75,
        "graded mean is not closer to the world: {before:.1} -> {after:.1}"
    );
}

/// logic/16 Invariant 5 holds with the world grade: neighbouring graded
/// cells equal the same pixels of the graded 2 × 1 window.
#[tokio::test]
async fn graded_neighbours_join_the_graded_window_pixel_for_pixel() {
    let (gx, gy) = RIVER;
    let q = "ppsq=16&world_grade=1";
    let uri = format!("/v1/tactical/window.png?gx0={gx}&gy0={gy}&w=2&h=1&{q}");
    let win = get(&uri).await;
    ok(&uri, &win);
    let win = decode_rgb(&win.body).unwrap();
    assert_eq!((win.width, win.height), (128 * 16, 64 * 16));
    for (k, cx) in [gx, gx + 1].into_iter().enumerate() {
        let cell = decode_rgb(
            &get(&format!("/v1/tactical/cell/{cx}/{gy}.png?{q}"))
                .await
                .body,
        )
        .unwrap();
        let row = 1024 * 3;
        let mut differ = 0_usize;
        for y in 0..1024_usize {
            let from = y * win.width as usize * 3 + k * row;
            let a = &win.pixels[from..from + row];
            let b = &cell.pixels[y * row..(y + 1) * row];
            differ += a.iter().zip(b).filter(|(p, q)| p != q).count();
        }
        assert_eq!(differ, 0, "graded cell {cx},{gy}: {differ} bytes differ");
    }
}
