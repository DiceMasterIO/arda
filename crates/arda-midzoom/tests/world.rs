//! Relief tiles on a real seed-42 MICRO fine world: pixel-exact seams,
//! determinism, the river overlay and the warm-tile budget.
//!
//! World resolution: `$ARDA_MIDZOOM_TEST_WORLD`, then `<workspace>/out/micro42`
//! (`arda generate --seed 42 --micro --terrain fine`), else one generated
//! into `target/arda-midzoom-fixture/`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    missing_docs
)]

#[path = "../../../tests/support/fixture_dir.rs"]
mod fixture_dir;

use arda_midzoom::{render_tile, render_window, Pyramid, ReliefWorld};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

fn workspace() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn loads(dir: &Path) -> bool {
    arda::World::load(dir).is_ok_and(|w| w.manifest().fine_terrain.is_some() && w.seed() == 42)
}

fn world_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        if let Some(dir) = std::env::var_os("ARDA_MIDZOOM_TEST_WORLD") {
            return PathBuf::from(dir);
        }
        let out = workspace().join("out/micro42");
        if loads(&out) {
            return out;
        }
        let dir = workspace().join("target/arda-midzoom-fixture/micro42");
        fixture_dir::ensure(&dir, loads, |tmp| {
            arda::generate_from_fine_source(
                42,
                arda::GenerateConfig::MICRO,
                tmp,
                arda::FineDeliveryLimits::default(),
            )
            .expect("generating the MICRO fixture world");
        });
        dir
    })
}

fn relief() -> &'static (ReliefWorld, Pyramid) {
    static R: OnceLock<(ReliefWorld, Pyramid)> = OnceLock::new();
    R.get_or_init(|| {
        let world = Arc::new(arda::World::load(world_dir()).unwrap());
        let m = world.manifest();
        let pyramid = Pyramid {
            max_zoom: 4,
            areas_wide: m.areas_wide,
            areas_high: m.areas_high,
        };
        (ReliefWorld::new(world).unwrap(), pyramid)
    })
}

/// A mountain tile (area 1,2 at 88 km E, 153.5 km S) at level `z`.
fn mountain(z: u32) -> (u32, u32) {
    let (_, p) = relief();
    let px = |m: i64| u32::try_from(m * 1_000_000 / p.pixel_um(z) / 256).unwrap();
    (px(88_000), px(153_500))
}

#[test]
fn adjacent_tiles_join_pixel_exactly() {
    let (rw, p) = relief();
    for z in [6, 8] {
        let (x, y) = mountain(z);
        let a = render_tile(rw, p, z, x, y).unwrap();
        let right = render_tile(rw, p, z, x + 1, y).unwrap();
        let below = render_tile(rw, p, z, x, y + 1).unwrap();
        let big = render_window(
            rw,
            p,
            z,
            (i64::from(x) * 256, i64::from(y) * 256),
            (512, 512),
        )
        .unwrap();
        for row in 0..256_usize {
            let wide = &big.pixels[row * 512 * 4..(row + 1) * 512 * 4];
            assert_eq!(
                &wide[..1024],
                &a.pixels[row * 1024..(row + 1) * 1024],
                "z{z} row {row}"
            );
            assert_eq!(
                &wide[1024..],
                &right.pixels[row * 1024..(row + 1) * 1024],
                "z{z} row {row} (east)"
            );
            let low = &big.pixels[(row + 256) * 512 * 4..(row + 256) * 512 * 4 + 1024];
            assert_eq!(
                low,
                &below.pixels[row * 1024..(row + 1) * 1024],
                "z{z} row {row} (south)"
            );
        }
        assert_eq!(a, render_tile(rw, p, z, x, y).unwrap(), "deterministic");
    }
}

#[test]
fn tiles_outside_the_relief_levels_are_refused() {
    let (rw, p) = relief();
    assert!(
        render_tile(rw, p, 4, 0, 0).is_err(),
        "native level is the overview's"
    );
    assert!(
        render_tile(rw, p, 6, 64, 0).is_err(),
        "column past the grid"
    );
    assert!(
        render_tile(rw, p, 13, 0, 0).is_err(),
        "deeper than the relief limit"
    );
}

#[test]
fn off_world_pixels_are_transparent_and_land_is_opaque() {
    let (rw, p) = relief();
    // Level 5: the world is 2048 × 4096 px of an 8192 px square.
    let east = render_tile(rw, p, 5, 20, 3).unwrap();
    assert!(east.pixels.chunks(4).all(|px| px[3] == 0));
    let (x, y) = mountain(6);
    let land = render_tile(rw, p, 6, x, y).unwrap();
    assert!(land.pixels.chunks(4).all(|px| px[3] == 255));
}

#[test]
fn saved_rivers_are_drawn_in_blue_teal() {
    let (rw, p) = relief();
    let (x, y) = mountain(6);
    let mut river = 0;
    for dx in 0..2 {
        let t = render_tile(rw, p, 6, x + dx, y).unwrap();
        river += t
            .pixels
            .chunks(4)
            .filter(|px| {
                i32::from(px[2]) > i32::from(px[0]) + 60 && i32::from(px[1]) > i32::from(px[0]) + 40
            })
            .count();
    }
    assert!(river > 200, "{river} river pixels");
}

/// Wall-clock bound, a release gate like the one in `checks.rs`: run with
/// `cargo test -p arda-midzoom --test world -- --ignored`.
#[test]
#[ignore = "wall-clock timing; release gate"]
fn a_warm_256_px_tile_at_ten_metres_renders_within_budget() {
    let (rw, p) = relief();
    let z = 7; // 6.25 m/px on 9.765625 m nodes
    assert_eq!(p.subdivisions(z), 4);
    let (x, y) = mountain(z);
    let _ = render_tile(rw, p, z, x, y).unwrap();
    // Best of three warm renders: the workspace suite runs every test
    // binary at once, and one scheduling stall must not fail the budget.
    let ms = (1..=3)
        .map(|dx| {
            let start = std::time::Instant::now();
            let _ = render_tile(rw, p, z, x + dx, y).unwrap();
            start.elapsed().as_millis()
        })
        .min()
        .unwrap();
    assert!(ms < 150, "warm relief tile took {ms} ms");
}
