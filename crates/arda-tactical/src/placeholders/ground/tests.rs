//! Ground texture tests: seams and shared layouts.
use super::*;
use crate::validate::seam_ratio;

#[test]
fn every_texture_is_seamless() {
    for (kind, _, _) in TYPES {
        let img = texture(kind, 11, 12, 32);
        let r = seam_ratio(&img);
        assert!(r < 1.5, "{kind} seam ratio {r}");
        let img = texture(kind, 13, 12, 64);
        let r = seam_ratio(&img);
        assert!(r < 1.5, "{kind} seam ratio {r} at 64 px");
    }
}

fn luma(img: &Rgba) -> Vec<f32> {
    img.data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| f32::from(p[0]) + f32::from(p[1]) + f32::from(p[2]))
        .collect()
}

fn correlation(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len() as f32;
    let (ma, mb) = (a.iter().sum::<f32>() / n, b.iter().sum::<f32>() / n);
    let cov: f32 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let va: f32 = a.iter().map(|x| (x - ma) * (x - ma)).sum();
    let vb: f32 = b.iter().map(|y| (y - mb) * (y - mb)).sum();
    cov / (va * vb).sqrt()
}

/// Regression: variants with different layouts cross-faded into
/// ghosted grout lines. Variants of a kind now share their layout.
#[test]
fn structured_variants_share_their_layout() {
    for kind in STRUCTURED {
        let a = luma(&texture(kind, 1, 99, 32));
        let b = luma(&texture(kind, 2, 99, 32));
        let other = luma(&texture(kind, 2, 98, 32));
        let same = correlation(&a, &b);
        assert!(same > 0.5, "{kind}: variants correlate only {same:.2}");
        // Plank rows, rug motifs and furrows are fixed, so only cell and
        // slab layouts differ between layout seeds.
        if matches!(kind, "cobbles" | "stone_floor" | "flagstone") {
            assert!(
                same > correlation(&a, &other) + 0.2,
                "{kind}: layout does not drive the match"
            );
        }
    }
}
