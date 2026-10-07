//! Tests of the climate dryness tint and the flattened texture variants
//! (no blocky checker where variants meet).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

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

fn mean_rgb(img: &Rgba) -> [f32; 3] {
    let mut s = [0.0f64; 3];
    for p in img.data.as_chunks::<4>().0 {
        for k in 0..3 {
            s[k] += f64::from(p[k]);
        }
    }
    let n = (img.data.len() / 4) as f64;
    s.map(|v| (v / n) as f32)
}

fn field(ground: &str, dryness: u8) -> TacticalLayout {
    let mut l = TacticalLayout::new("dry", 8, 8, ground);
    for sq in &mut l.squares {
        sq.dryness = dryness;
    }
    l
}

#[test]
fn dryness_turns_turf_toward_straw_and_leaves_stone_alone() {
    let lib = lib();
    let lush = render(&field("grass", 0), &lib, 5, &plain(32)).unwrap();
    let dry = render(&field("grass", 255), &lib, 5, &plain(32)).unwrap();
    let (l, d) = (mean_rgb(&lush), mean_rgb(&dry));
    assert!(
        d[0] / d[1] > l[0] / l[1] + 0.1,
        "dry grass is redder: {l:?} → {d:?}"
    );
    let green = |c: [f32; 3]| c[1] / (c[0] + c[1] + c[2]);
    assert!(green(d) < green(l) - 0.03, "and less green: {l:?} → {d:?}");
    let a = render(&field("gravel", 0), &lib, 5, &plain(32)).unwrap();
    let b = render(&field("gravel", 255), &lib, 5, &plain(32)).unwrap();
    assert_eq!(a, b, "gravel takes no tint");
}

#[test]
fn dry_layouts_join_seamlessly_across_windows() {
    let lib = lib();
    let (w, h) = (24_u32, 10_u32);
    let mut big = TacticalLayout::new("world", w, h, "grass");
    for y in 0..h {
        for x in 0..w {
            let sq = big.square_mut(x, y).unwrap();
            sq.dryness = (x * 10) as u8;
            if (x + y) % 7 == 0 {
                sq.ground = "dirt".into();
            }
        }
    }
    let cut = |x0: u32| {
        let mut out = TacticalLayout::new("window", 18, h, "grass");
        for y in 0..h {
            for x in 0..18 {
                *out.square_mut(x, y).unwrap() =
                    big.square(i64::from(x0 + x), i64::from(y)).clone();
            }
        }
        out.origin = Some([i64::from(x0), 0]);
        out
    };
    let ppsq = 32;
    let a = render(&cut(0), &lib, 3, &plain(ppsq)).unwrap();
    let b = render(&cut(6), &lib, 3, &plain(ppsq)).unwrap();
    for y in 0..h * ppsq {
        for x in 0..ppsq {
            assert_eq!(a.get(12 * ppsq + x, y), b.get(6 * ppsq + x, y), "{x},{y}");
        }
    }
}

#[test]
fn flattened_variants_share_one_tone_and_keep_their_grain() {
    let lib = lib();
    let ppsq = 32;
    let set = ground::TextureSet::new(&lib, ppsq);
    let tex = set.get("grass").unwrap();
    assert!(tex.variants.len() > 1);
    let originals: Vec<Rgba> = lib
        .textures("grass")
        .iter()
        .map(|a| {
            lib.image(&a.id)
                .unwrap()
                .resized(a.footprint.w * ppsq, a.footprint.h * ppsq)
        })
        .collect();
    let spread = |imgs: &[&Rgba]| {
        let means: Vec<f32> = imgs.iter().map(|i| mean_rgb(i)[1]).collect();
        let lo = means.iter().copied().fold(f32::MAX, f32::min);
        let hi = means.iter().copied().fold(f32::MIN, f32::max);
        hi - lo
    };
    let before = spread(&originals.iter().collect::<Vec<_>>());
    let after = spread(&tex.variants.iter().collect::<Vec<_>>());
    assert!(
        after <= 0.5 * before + 0.5,
        "tone spread {before} → {after}"
    );
    // Grain survives: neighbouring texels still differ.
    let v = &tex.variants[0];
    let mut grain = 0.0;
    for x in 1..v.width {
        grain += (f32::from(v.get(x, 3)[1]) - f32::from(v.get(x - 1, 3)[1])).abs();
    }
    assert!(grain / v.width as f32 > 1.0, "grain {grain}");
}
