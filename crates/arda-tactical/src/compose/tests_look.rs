//! Tests of the look: world-anchored seams, the 5-ft water line, stochastic
//! tiling, elevation shading, canopy shadows and the parallel blur.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use super::*;
use crate::layout::Placement;
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

/// A world of mixed ground, water and elevation to cut windows from.
fn world(w: u32, h: u32) -> TacticalLayout {
    let mut l = TacticalLayout::new("world", w, h, "grass");
    let keys = ["grass", "dirt", "meadow", "gravel", "moss"];
    for y in 0..h {
        for x in 0..w {
            let sq = l.square_mut(x, y).unwrap();
            sq.ground = keys[((x / 3 + y / 2 + x * y) % 5) as usize].into();
            sq.elevation_ft = 100 + 5 * ((x / 4 + y / 3) % 3) as i16;
            if (5..8).contains(&y) && x > 3 {
                sq.water_depth_ft = if y == 6 { 6 } else { 2 };
            }
        }
    }
    l
}

/// Cuts `w × h` squares at `(x0, y0)` out of `l`, with a world origin.
fn window(l: &TacticalLayout, x0: u32, y0: u32, w: u32, h: u32) -> TacticalLayout {
    let mut out = TacticalLayout::new("window", w, h, "grass");
    for y in 0..h {
        for x in 0..w {
            *out.square_mut(x, y).unwrap() = l.square(i64::from(x0 + x), i64::from(y0 + y)).clone();
        }
    }
    out.origin = Some([i64::from(x0), i64::from(y0)]);
    out
}

#[test]
fn adjacent_windows_join_seamlessly_with_an_origin() {
    let lib = lib();
    let big = world(24, 12);
    let ppsq = 32;
    // Two windows overlapping by twelve squares; the middle of the overlap
    // is far enough (six squares) from both windows' edges for every
    // neighbourhood the compositor reads (elevation smoothing and
    // interpolation, warps, blends) to see identical inputs.
    let a = render(&window(&big, 0, 0, 18, 12), &lib, 3, &plain(ppsq)).unwrap();
    let b = render(&window(&big, 6, 0, 18, 12), &lib, 3, &plain(ppsq)).unwrap();
    let (mut same, mut total) = (0, 0);
    for y in 0..12 * ppsq {
        for x in 0..ppsq {
            // Column 12 of the world is column 12 of `a` and 6 of `b`.
            let pa = a.get(12 * ppsq + x, y);
            let pb = b.get(6 * ppsq + x, y);
            total += 1;
            same += usize::from(pa == pb);
        }
    }
    assert_eq!(same, total, "{same} of {total} shared pixels match");
    // Without an origin the windows would not line up.
    let mut local = window(&big, 6, 0, 18, 12);
    local.origin = None;
    let c = render(&local, &lib, 3, &plain(ppsq)).unwrap();
    let differs = (0..12 * ppsq).any(|y| c.get(6 * ppsq + 5, y) != a.get(12 * ppsq + 5, y));
    assert!(differs, "local coordinates should not match world ones");
}

fn mean_rgb(img: &Rgba, x0: u32, y0: u32, x1: u32, y1: u32) -> [f32; 3] {
    let mut s = [0.0f32; 3];
    let mut n = 0.0;
    for y in y0..y1 {
        for x in x0..x1 {
            let p = img.get(x, y);
            for k in 0..3 {
                s[k] += f32::from(p[k]);
            }
            n += 1.0;
        }
    }
    s.map(|v| v / n)
}

#[test]
fn water_turns_deep_at_five_feet() {
    let lib = lib();
    let textures = ground::TextureSet::new(&lib, 32);
    let shallow = textures.get(WATER_SHALLOW).unwrap().mean;
    let deep = textures.get(WATER_DEEP).unwrap().mean;
    let pond = |depth: u8| {
        let mut l = TacticalLayout::new("pond", 8, 8, "mud");
        for sq in &mut l.squares {
            sq.water_depth_ft = depth;
        }
        let img = render(&l, &lib, 1, &plain(32)).unwrap();
        mean_rgb(&img, 64, 64, 192, 192)
    };
    let dist = |a: [f32; 3], b: [f32; 3]| {
        ((0..3).map(|k| (a[k] - b[k]) * (a[k] - b[k])).sum::<f32>()).sqrt()
    };
    let four = pond(4);
    let five = pond(5);
    assert!(
        dist(four, shallow) < dist(four, deep),
        "4 ft reads as shallow: {four:?}"
    );
    assert!(
        dist(five, deep) < dist(five, shallow),
        "5 ft reads as deep: {five:?}"
    );
}

/// Correlation of an image's luma with itself shifted by `dx` pixels.
fn shift_correlation(img: &Rgba, dx: u32) -> f32 {
    let l = |x: u32, y: u32| {
        let p = img.get(x, y);
        f32::from(p[0]) + f32::from(p[1]) + f32::from(p[2])
    };
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for y in 0..img.height {
        for x in 0..img.width - dx {
            a.push(l(x, y));
            b.push(l(x + dx, y));
        }
    }
    let n = a.len() as f32;
    let (ma, mb) = (a.iter().sum::<f32>() / n, b.iter().sum::<f32>() / n);
    let cov: f32 = a.iter().zip(&b).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let va: f32 = a.iter().map(|x| (x - ma) * (x - ma)).sum();
    let vb: f32 = b.iter().map(|y| (y - mb) * (y - mb)).sum();
    cov / (va * vb).sqrt()
}

#[test]
fn stochastic_tiling_hides_the_texture_period() {
    let lib = lib();
    let ppsq = 32;
    let meadow = TacticalLayout::new("meadow", 16, 8, "grass");
    let img = render(&meadow, &lib, 2, &plain(ppsq)).unwrap();
    // Plain tiling repeats every texture width (two squares) exactly.
    let period = 2 * ppsq;
    let r = shift_correlation(&img, period);
    assert!(r < 0.35, "correlation at the texture period is {r:.2}");
}

#[test]
fn stepped_slopes_shade_smoothly_and_cliffs_get_ledges() {
    let ppsq = 32;
    let mut slope = TacticalLayout::new("slope", 12, 16, "grass");
    let mut cliff = TacticalLayout::new("cliff", 12, 16, "grass");
    for y in 0..16u32 {
        for x in 0..12u32 {
            // A steady grade arrives as 5-ft steps every two squares.
            slope.square_mut(x, y).unwrap().elevation_ft = 200 - 5 * (y / 2) as i16;
            cliff.square_mut(x, y).unwrap().elevation_ft = if y < 8 { 220 } else { 200 };
        }
    }
    let spread = |l: &TacticalLayout| {
        let frame = field::Frame::of(l, ppsq);
        let t = terrain::Terrain::new(l, &frame, 1);
        let (w, h) = (l.width * ppsq, l.height * ppsq);
        let hs = t.heights(w, h);
        let col = 6 * ppsq;
        let shades: Vec<f32> = (3 * ppsq..13 * ppsq)
            .map(|y| t.shade(&hs, w, col, y))
            .collect();
        let (lo, hi) = shades
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), &v| (a.min(v), b.max(v)));
        hi - lo
    };
    let smooth = spread(&slope);
    assert!(
        smooth < 0.08,
        "a stepped grade varies by {smooth:.3} down the slope"
    );
    let steep = spread(&cliff);
    assert!(steep > 0.2, "a 20-ft cliff shades by only {steep:.3}");
}

#[test]
fn canopy_shadows_are_short_offsets() {
    let lib = lib();
    let ppsq = 32;
    let mut l = TacticalLayout::new("tree", 10, 10, "dirt");
    l.placements.push(Placement {
        asset: AssetRef::Id("veg.tree_oak".into()),
        x: 4.0,
        y: 4.0,
        rotation: 0,
        mirror: false,
    });
    let style = Style { grade: false };
    let lit = RenderOptions {
        lighting: true,
        ..plain(ppsq)
    };
    let on = render_with(&l, &lib, 1, &lit, &style).unwrap();
    let off = render_with(&l, &lib, 1, &plain(ppsq), &style).unwrap();
    let luma = |img: &Rgba, x: u32, y: u32| {
        let p = img.get(x, y);
        u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])
    };
    // The oak is 3 squares across: radius 1.5 squares = 48 px, centred at
    // (128, 128). Just past its south-east edge the ground is in shadow...
    let near = 128 + 40;
    assert!(
        luma(&on, near, near) * 100 < luma(&off, near, near) * 92,
        "no shadow at the crown's edge"
    );
    // ...but a crown radius beyond that edge it is lit again.
    let far = 128 + 48 * 3 / 2 + 20;
    assert!(
        luma(&on, far, far) * 100 > luma(&off, far, far) * 97,
        "shadow is swept too far"
    );
}
