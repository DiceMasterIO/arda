//! Cliff strata and slope ledges (review gallery 2026-10-15, images 12 and
//! 13): a cliff's bands follow the contours and keep one phase from cell to
//! cell instead of staggering into bricks, and vegetation hides the 5-ft
//! ledges.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use super::field::Frame;
use super::sample::{sample_finish, CellTable, KeyTextures, TextureSet};
use super::*;
use std::path::Path;

const PPSQ: u32 = 32;
/// Canvas side in squares and pixels.
const SIDE: u32 = 16;
const PX: u32 = SIDE * PPSQ;

fn lib() -> Library {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    Library::load(&dir).unwrap()
}

/// Heights (1/256 ft) of a plane rising `(sx, sy)` units per pixel.
fn plane(sx: i32, sy: i32) -> Vec<i32> {
    (0..PX * PX)
        .map(|i| sx * (i % PX) as i32 + sy * (i / PX) as i32)
        .collect()
}

/// The cliff key's luma at every pixel, sampled the old way (`None`) or
/// along the contours of `heights`. The placeholder art is tagged
/// structured (aligned, no offsets); generated cliffs are natural, so the
/// first placeholder variant is loaded as a natural texture, whose cells
/// take random offsets.
fn cliff(heights: Option<&[i32]>) -> Vec<f32> {
    let lib = lib();
    let set = TextureSet::new(&lib, PPSQ);
    let art = set.get("cliff").unwrap();
    let tex = KeyTextures::new("cliff", vec![art.variants[0].clone()], false);
    let layout = TacticalLayout::new("cliff", SIDE, SIDE, "cliff");
    let frame = Frame::of(&layout, PPSQ);
    let cells = CellTable::new(&tex, 7, &frame, SIDE, SIDE);
    let mut out = Vec::with_capacity((PX * PX) as usize);
    for py in 0..PX {
        for px in 0..PX {
            let c = match heights {
                Some(h) => super::strata::sample(
                    &tex,
                    &cells,
                    &frame,
                    (px, py),
                    (0.0, 0.0),
                    None,
                    Some((h, PX)),
                ),
                None => sample_finish(&tex, &cells, &frame, (px, py), (0.0, 0.0), None),
            };
            out.push(0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]);
        }
    }
    out
}

/// Mean absolute luma step along x and along y.
fn steps(img: &[f32]) -> (f32, f32) {
    let at = |x: u32, y: u32| img[(y * PX + x) as usize];
    let (mut dx, mut dy) = (0.0, 0.0);
    for y in 0..PX - 1 {
        for x in 0..PX - 1 {
            dx += (at(x + 1, y) - at(x, y)).abs();
            dy += (at(x, y + 1) - at(x, y)).abs();
        }
    }
    (dx, dy)
}

#[test]
fn cliff_bands_follow_an_east_facing_contour() {
    // The face rises eastward: its contours, and so its strata, run
    // north–south. The art's bands run east–west.
    let (ox, oy) = steps(&cliff(None));
    assert!(
        oy > 1.3 * ox,
        "the art's bands run east-west: {ox:.0} {oy:.0}"
    );
    let (nx, ny) = steps(&cliff(Some(&plane(64, 0))));
    assert!(nx > 1.3 * ny, "bands across x {nx:.0}, across y {ny:.0}");
}

/// Correlation of the row profiles (mean luma per row) of two column
/// strips `PPSQ` pixels wide.
fn strip_correlation(img: &[f32], a: u32, b: u32) -> f32 {
    let profile = |x0: u32| -> Vec<f32> {
        (0..PX)
            .map(|y| {
                (x0..x0 + PPSQ)
                    .map(|x| img[(y * PX + x) as usize])
                    .sum::<f32>()
                    / PPSQ as f32
            })
            .collect()
    };
    let (p, q) = (profile(a), profile(b));
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
    let (mp, mq) = (mean(&p), mean(&q));
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (x, y) in p.iter().zip(&q) {
        sxy += (x - mp) * (y - mq);
        sxx += (x - mp) * (x - mp);
        syy += (y - mq) * (y - mq);
    }
    sxy / (sxx * syy).sqrt().max(1e-6)
}

#[test]
fn cliff_bands_keep_their_phase_across_cells() {
    // A south-facing face: bands east–west, as the art draws them. Strips
    // several cells apart show the same bands instead of staggered bricks.
    let new = cliff(Some(&plane(0, 64)));
    let old = cliff(None);
    let (a, b) = (2 * PPSQ, 11 * PPSQ);
    let (cn, co) = (strip_correlation(&new, a, b), strip_correlation(&old, a, b));
    assert!(cn > 0.6, "bands in phase: {cn:.2} (old tiling {co:.2})");
    assert!(cn > co + 0.2, "{cn:.2} against {co:.2}");
}

#[test]
fn vegetation_hides_the_ledges() {
    let mut l = TacticalLayout::new("cliff", 12, 16, "grass");
    for y in 0..16u32 {
        for x in 0..12u32 {
            l.square_mut(x, y).unwrap().elevation_ft = 300 - 10 * y as i16;
        }
    }
    let frame = Frame::of(&l, PPSQ);
    let t = terrain::Terrain::new(&l, &frame, 1);
    let (w, h) = (l.width * PPSQ, l.height * PPSQ);
    let hs = t.heights(w, h);
    let spread = |ledges: f32| {
        let v: Vec<f32> = (3 * PPSQ..13 * PPSQ)
            .map(|y| t.shade(&hs, w, 6 * PPSQ, y, ledges))
            .collect();
        let (lo, hi) = v
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &s| (a.min(s), b.max(s)));
        hi - lo
    };
    let (bare, turf) = (spread(1.0), spread(0.0));
    assert!(bare > 0.2, "bare rock shows its ledges: {bare:.3}");
    assert!(
        turf < 0.5 * bare,
        "turf hides them: {turf:.3} against {bare:.3}"
    );
}
