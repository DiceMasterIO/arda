//! The society band overlay paints the same pixels for any split of the
//! image into bands and strips, and the same bytes twice.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_render::BandOverlay;
use arda_settle::canvas::Canvas;
use arda_settle::render::band::{BandLayer, SocietyBandOverlay};
use arda_settle::render::Society;
use arda_settle::{run, synthetic, PlaceParams};

fn overlay(width: u32, height: u32) -> SocietyBandOverlay {
    overlay_with(width, height, BandLayer::Society)
}

/// Tests run in parallel: each society gets its own directory.
static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn overlay_with(width: u32, height: u32, layer: BandLayer) -> SocietyBandOverlay {
    let params = PlaceParams {
        seed: 42,
        density_per_km2: 15,
    };
    let grid = synthetic::landscape(params.seed).unwrap();
    let society = run(&grid, params).unwrap();
    let dir = std::env::temp_dir().join(format!(
        "arda-settle-band-{}-{width}x{height}-{layer:?}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
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
    SocietyBandOverlay::with_layer(soc, width, height, layer).unwrap()
}

fn grey(width: usize, height: usize) -> Vec<u8> {
    (0..width * height * 3)
        .map(|i| 90 + u8::try_from((i / 3 % 37) % 60).unwrap())
        .collect()
}

fn in_bands(o: &SocietyBandOverlay, (w, h): (usize, usize), cuts: &[usize]) -> Vec<u8> {
    let mut img = grey(w, h);
    let mut y0 = 0;
    for &y1 in cuts.iter().chain(std::iter::once(&h)) {
        o.paint(
            u32::try_from(y0).unwrap(),
            &mut img[y0 * w * 3..y1 * w * 3],
            u32::try_from(w).unwrap(),
        )
        .unwrap();
        y0 = y1;
    }
    img
}

#[test]
fn bands_equal_the_whole_image_and_painting_is_deterministic() {
    let (w, h) = (1280, 960);
    let o = overlay(1280, 960);
    let mut whole = Canvas {
        width: w,
        height: h,
        rgb: grey(w, h),
    };
    o.paint_window(&mut whole, (0, 0));
    let base = grey(w, h);
    let changed = whole.rgb.iter().zip(&base).filter(|(a, b)| a != b).count();
    assert!(changed > 1000, "the overlay painted only {changed} bytes");

    let banded = in_bands(&o, (w, h), &[256, 512, 768]);
    assert_eq!(
        banded, whole.rgb,
        "256-row bands differ from the whole image"
    );
    let odd = in_bands(&o, (w, h), &[1, 77, 300, 301, 899]);
    assert_eq!(odd, whole.rgb, "uneven bands differ from the whole image");
    assert_eq!(banded, in_bands(&o, (w, h), &[256, 512, 768]));
}

#[test]
fn settlement_symbols_and_roads_appear_at_world_scale() {
    // 320 px for a 64 km world: 200 m per pixel, trunk roads and symbols only.
    let o = overlay(320, 240);
    let img = in_bands(&o, (320, 240), &[]);
    assert!(img != grey(320, 240));
}

#[test]
fn windows_of_a_zoomed_image_agree_with_their_union_including_dashed_paths() {
    // 7,680 px for 64 km: 8.3 m per pixel, so footpaths and tracks show.
    let o = overlay(7680, 5760);
    let (ww, wh, origin) = (1100_usize, 300_usize, (2500_usize, 2000_usize));
    let fresh = |w: usize, h: usize, at: (usize, usize)| {
        let mut c = Canvas {
            width: w,
            height: h,
            rgb: (0..w * h)
                .flat_map(|i| {
                    let (x, y) = (at.0 + i % w, at.1 + i / w);
                    let v = 90 + u8::try_from((x * 7 + y * 13) % 60).unwrap();
                    [v, v, v]
                })
                .collect(),
        };
        let before = c.rgb.clone();
        o.paint_window(&mut c, at);
        (c, before)
    };
    let (whole, before) = fresh(ww, wh, origin);
    let changed = whole
        .rgb
        .iter()
        .zip(&before)
        .filter(|(a, b)| a != b)
        .count();
    assert!(changed > 100, "only {changed} bytes painted in the window");
    for (x_cut, y_cut) in [(600, 120), (1, 299), (777, 150)] {
        let parts = [
            (0, 0, x_cut, y_cut),
            (x_cut, 0, ww - x_cut, y_cut),
            (0, y_cut, x_cut, wh - y_cut),
            (x_cut, y_cut, ww - x_cut, wh - y_cut),
        ];
        for (px, py, w, h) in parts {
            let (part, _) = fresh(w, h, (origin.0 + px, origin.1 + py));
            for r in 0..h {
                let a = &part.rgb[r * w * 3..(r + 1) * w * 3];
                let b = &whole.rgb[((py + r) * ww + px) * 3..((py + r) * ww + px + w) * 3];
                assert_eq!(a, b, "window at ({px},{py}) row {r} differs");
            }
        }
    }
}

#[test]
fn settlement_symbols_scale_with_the_output_width() {
    let small = overlay(640, 480).symbol_scale();
    let page = overlay(1280, 960).symbol_scale();
    let wide = overlay(2048, 1536).symbol_scale();
    let huge = overlay(8192, 6144).symbol_scale();
    assert!(small < page && page < wide, "{small} {page} {wide}");
    assert!(wide >= 1.5, "a 2048 px overview draws cities big: {wide}");
    assert!(huge >= wide && huge <= 2.4);
}

/// Paints the window of `o` at `at` over a position-dependent grey.
fn window(o: &SocietyBandOverlay, w: usize, h: usize, at: (usize, usize)) -> Canvas {
    let mut c = Canvas {
        width: w,
        height: h,
        rgb: (0..w * h)
            .flat_map(|i| {
                let (x, y) = (at.0 + i % w, at.1 + i / w);
                let v = 90 + u8::try_from((x * 7 + y * 13) % 60).unwrap();
                [v, v, v]
            })
            .collect(),
    };
    o.paint_window(&mut c, at);
    c
}

#[test]
fn political_tiles_join_without_a_seam() {
    // 1,280 px for 64 km (50 m/px): an overview pyramid level, cut into
    // 256 px slippy tiles. Each tile painted alone equals its place in a
    // 2 × 2 block painted at once, and bands of rows equal the whole.
    let (w, h) = (1280, 960);
    let pol = overlay_with(1280, 960, BandLayer::Political);
    let soc = overlay(1280, 960);
    assert!(pol.palette().is_some() && soc.palette().is_none());
    let whole = window(&pol, w, h, (0, 0));
    assert_ne!(
        whole.rgb,
        window(&soc, w, h, (0, 0)).rgb,
        "the political layer paints bands and borders"
    );
    assert_eq!(in_bands(&pol, (w, h), &[256, 512, 768]), {
        let mut c = Canvas {
            width: w,
            height: h,
            rgb: grey(w, h),
        };
        pol.paint_window(&mut c, (0, 0));
        c.rgb
    });
    for (bx, by) in [(0, 0), (256, 256), (512, 256), (768, 512)] {
        let block = window(&pol, 512, 512, (bx, by));
        for (tx, ty) in [(0, 0), (256, 0), (0, 256), (256, 256)] {
            let tile = window(&pol, 256, 256, (bx + tx, by + ty));
            for r in 0..256 {
                let a = &tile.rgb[r * 256 * 3..(r + 1) * 256 * 3];
                let b = &block.rgb[((ty + r) * 512 + tx) * 3..((ty + r) * 512 + tx + 256) * 3];
                assert_eq!(
                    a,
                    b,
                    "tile ({},{}) row {r} differs from its block",
                    bx + tx,
                    by + ty
                );
            }
        }
        // And the block is the whole image's own pixels.
        for r in 0..512.min(h - by) {
            let a = &block.rgb[r * 512 * 3..(r * 512 + 512.min(w - bx)) * 3];
            let b = &whole.rgb[((by + r) * w + bx) * 3..((by + r) * w + bx + 512.min(w - bx)) * 3];
            assert_eq!(
                a, b,
                "block ({bx},{by}) row {r} differs from the whole image"
            );
        }
    }
}
