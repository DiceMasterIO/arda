//! logic/16 Invariant 5 on the default (recipe 9) world: a cell rendered
//! alone equals the same cell inside a 2 × 1 window, byte for byte, plain
//! and with the world grade. Cell 529,812 differed by 5 bytes plain and 2
//! graded: the compositor stored heights above the layout's lowest square
//! and the grade filled water points from the window's own lattice.

use crate::support::get;
use arda_server::tiles::decode_rgb;
use axum::http::StatusCode;

async fn png(uri: &str) -> arda_server::tiles::Rgb {
    let r = get(uri).await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{uri}: {}",
        String::from_utf8_lossy(&r.body)
    );
    decode_rgb(&r.body).unwrap()
}

async fn joins(gx: u32, gy: u32, q: &str, ppsq: usize) {
    let win = png(&format!(
        "/v1/tactical/window.png?gx0={gx}&gy0={gy}&w=2&h=1&{q}"
    ))
    .await;
    let row = 64 * ppsq * 3;
    for (k, cx) in [gx, gx + 1].into_iter().enumerate() {
        let cell = png(&format!("/v1/tactical/cell/{cx}/{gy}.png?{q}")).await;
        let mut differ = 0_usize;
        for y in 0..64 * ppsq {
            let from = y * win.width as usize * 3 + k * row;
            let a = &win.pixels[from..from + row];
            let b = &cell.pixels[y * row..(y + 1) * row];
            differ += a.iter().zip(b).filter(|(p, q)| p != q).count();
        }
        assert_eq!(
            differ, 0,
            "cell {cx},{gy} ({q}) differs from the window in {differ} bytes"
        );
    }
}

#[tokio::test]
async fn cells_equal_their_windows_byte_for_byte() {
    for (gx, gy) in [(529, 812), (518, 822), (530, 1063)] {
        joins(gx, gy, "ppsq=32", 32).await;
        joins(gx, gy, "ppsq=32&world_grade=1", 32).await;
    }
    joins(529, 812, "ppsq=64", 64).await;
    joins(529, 812, "ppsq=64&world_grade=1", 64).await;
}
