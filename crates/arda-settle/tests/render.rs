//! The overlay draws deterministically onto a page and changes it.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_settle::canvas::Canvas;
use arda_settle::render::{self, Society};
use arda_settle::{run, synthetic, PlaceParams};

#[test]
fn the_overlay_draws_the_same_page_twice() {
    let params = PlaceParams {
        seed: 42,
        density_per_km2: 15,
    };
    let grid = synthetic::landscape(params.seed).unwrap();
    let society = run(&grid, params).unwrap();
    let dir = std::env::temp_dir().join(format!("arda-settle-render-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    arda_settle::write(
        &dir.join(arda_settle::SOCIETY_DIR),
        &grid,
        params.seed,
        &society,
    )
    .unwrap();
    let soc = Society::read(&dir).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let blank = Canvas {
        width: 1280,
        height: 960,
        rgb: vec![120; 1280 * 960 * 3],
    };
    let draw = |full: bool| {
        let mut c = blank.clone();
        render::draw(&mut c, &soc, params.seed, full).unwrap();
        c
    };
    let (a, b) = (draw(true), draw(true));
    assert_eq!(a.rgb, b.rgb, "drawing is not deterministic");
    assert_eq!((a.width, a.height), (1280, 960));
    let changed = a.rgb.iter().zip(&blank.rgb).filter(|(x, y)| x != y).count();
    assert!(changed > a.rgb.len() / 4, "only {changed} bytes changed");
    let page = draw(false);
    assert!(page.width <= 1280 && page.height <= 960 && page.width > 600);
}
