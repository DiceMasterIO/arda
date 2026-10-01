//! Soft snow cover (goal 49): feathered, irregular patch edges that still
//! join pixel for pixel across windows.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use super::*;
use std::path::Path;

fn lib() -> Library {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
    Library::load(&dir).unwrap()
}

fn plain(ppsq: u32) -> RenderOptions {
    RenderOptions {
        ppsq,
        grid: false,
        lighting: false,
    }
}

/// Grass and rock with a lobed snow patch and a few lone snow squares.
fn high_meadow(w: u32, h: u32) -> TacticalLayout {
    let mut l = TacticalLayout::new("meadow", w, h, "grass");
    for y in 0..h {
        for x in 0..w {
            let sq = l.square_mut(x, y).unwrap();
            let (dx, dy) = (f64::from(x) - 14.0, f64::from(y) - 8.0);
            let lobe = 5.0 + 1.5 * (f64::from(x) * 0.9).sin();
            if dx.hypot(dy) < lobe || (x + 3 * y) % 23 == 0 {
                sq.ground = "snow".into();
            } else if y < 3 {
                sq.ground = "rock".into();
            }
            sq.elevation_ft = 3_000;
        }
    }
    l
}

fn window(l: &TacticalLayout, x0: u32, w: u32) -> TacticalLayout {
    let mut out = TacticalLayout::new("window", w, l.height, "grass");
    for y in 0..l.height {
        for x in 0..w {
            *out.square_mut(x, y).unwrap() = l.square(i64::from(x0 + x), i64::from(y)).clone();
        }
    }
    out.origin = Some([i64::from(x0), 0]);
    out
}

#[test]
fn snow_windows_join_seamlessly() {
    let lib = lib();
    let big = high_meadow(36, 16);
    let ppsq = 16;
    let a = render(&window(&big, 0, 26), &lib, 5, &plain(ppsq)).unwrap();
    let b = render(&window(&big, 10, 26), &lib, 5, &plain(ppsq)).unwrap();
    // World columns 15..21 lie at least five squares inside both windows.
    for y in 0..16 * ppsq {
        for x in 15 * ppsq..21 * ppsq {
            assert_eq!(a.get(x, y), b.get(x - 10 * ppsq, y), "pixel {x},{y}");
        }
    }
}

fn luma(p: [u8; 4]) -> f32 {
    0.3 * f32::from(p[0]) + 0.59 * f32::from(p[1]) + 0.11 * f32::from(p[2])
}

#[test]
fn snow_edges_are_feathered_and_off_the_grid() {
    let lib = lib();
    let l = high_meadow(28, 16);
    let ppsq = 16;
    let img = render(&l, &lib, 5, &plain(ppsq)).unwrap();
    let (w, h) = (28 * ppsq, 16 * ppsq);
    // Pixels between the meadow's and the snow's brightness: thin cover
    // (hard-edged squares give about 0.3 of the snow pixels, soft cover
    // about 0.65).
    let (grass, snow) = (
        luma(img.get(14 * ppsq, 15 * ppsq)),
        luma(img.get(14 * ppsq, 8 * ppsq)),
    );
    let (lo, hi) = (grass + 0.25 * (snow - grass), grass + 0.75 * (snow - grass));
    let mut partial = 0;
    let mut whiteish = 0;
    for y in 0..h {
        for x in 0..w {
            let v = luma(img.get(x, y));
            partial += usize::from(v > lo && v < hi);
            whiteish += usize::from(v >= hi);
        }
    }
    assert!(
        partial * 2 > whiteish,
        "{partial} partial against {whiteish} snow pixels: edges are hard"
    );
    // The snow border never follows square edges: along each square
    // boundary column, cover changes between rows as often as anywhere.
    let mut on_line = 0;
    let mut off_line = 0;
    for y in 0..h {
        for x in 1..w {
            let step = (luma(img.get(x, y)) - luma(img.get(x - 1, y))).abs() > 40.0;
            if x % ppsq == 0 {
                on_line += usize::from(step);
            } else {
                off_line += usize::from(step);
            }
        }
    }
    let per_line = on_line as f32;
    let per_other = off_line as f32 / (ppsq - 1) as f32;
    assert!(
        per_line < 2.0 * per_other + 4.0,
        "{per_line} vs {per_other}"
    );
}
