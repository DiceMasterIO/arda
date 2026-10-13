//! The society band overlay paints the same pixels for any split of the
//! image into bands and strips, and the same bytes twice.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_render::BandOverlay;
use arda_settle::canvas::Canvas;
use arda_settle::render::band::SocietyBandOverlay;
use arda_settle::render::Society;
use arda_settle::{run, synthetic, PlaceParams};

fn overlay(width: u32, height: u32) -> SocietyBandOverlay {
    let params = PlaceParams {
        seed: 42,
        density_per_km2: 15,
    };
    let grid = synthetic::landscape(params.seed).unwrap();
    let society = run(&grid, params).unwrap();
    let dir = std::env::temp_dir().join(format!(
        "arda-settle-band-{}-{width}x{height}",
        std::process::id()
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
    SocietyBandOverlay::new(soc, width, height).unwrap()
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
