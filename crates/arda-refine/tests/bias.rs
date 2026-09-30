//! No grid artefacts (goal 43, 49): neither the elevation detail nor the
//! ground borders prefer the square grid's axes. Both are tested with
//! orientation histograms: an isotropic field puts 4 × 15 / 360 = 16.7 % of
//! its gradients within 7.5 degrees of an axis.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unwrap_used,
    missing_docs,
    clippy::cast_precision_loss
)]

mod common;
use arda::{Cell, Cover, TerrainKind};
use arda_core::HeightMm;
use arda_refine::context::Ctx;
use arda_refine::terrain::land_height;
use arda_refine::{refine, CellKey, GridSource};

/// Fraction of gradient directions within 7.5 degrees of an axis, weighted
/// by gradient magnitude.
fn axis_fraction(samples: &[(f64, f64)]) -> f64 {
    let (mut near, mut total) = (0.0, 0.0);
    for &(gx, gy) in samples {
        let m = (gx * gx + gy * gy).sqrt();
        if m < 1e-12 {
            continue;
        }
        let (ax, ay) = (gx.abs() / m, gy.abs() / m);
        // Within 7.5 degrees of an axis: the larger component's cosine.
        if ax.max(ay) > 0.991_444_861 {
            near += m;
        }
        total += m;
    }
    near / total
}

fn flat(seed: u64, cover: Cover, fd: u8) -> GridSource {
    GridSource::new(
        seed,
        12,
        12,
        Cell {
            height: HeightMm::new(50_000),
            terrain: TerrainKind::Land,
            cover,
            forest_density: fd,
            moisture: 140,
            wetness: 60,
            height_above_river_dm: 80,
            slope_milli_deg: 12_000,
            ..Cell::default()
        },
    )
}

#[test]
fn elevation_detail_is_isotropic() {
    let src = flat(11, Cover::Rock, 0);
    let ctx = Ctx::gather(&src, CellKey::new(5, 5)).unwrap();
    let mut g = Vec::new();
    for j in 0..160 {
        for i in 0..160 {
            let (u, v) = (
                5.0 * 64.0 + f64::from(i) * 0.61,
                5.0 * 64.0 + f64::from(j) * 0.57,
            );
            let h = 0.05;
            let gx = land_height(&ctx, u + h, v) - land_height(&ctx, u - h, v);
            let gy = land_height(&ctx, u, v + h) - land_height(&ctx, u, v - h);
            g.push((gx, gy));
        }
    }
    let f = axis_fraction(&g);
    assert!((f - 0.1667).abs() < 0.03, "axis fraction {f:.3}");
    // Directional variograms at a lag of 6 squares agree within 12 %.
    let gamma = |dx: f64, dy: f64| {
        let mut s = 0.0;
        for k in 0..40_000 {
            let (u, v) = (
                3.0 * 64.0 + 8.0 + f64::from(k % 200) * 1.37,
                3.0 * 64.0 + 8.0 + f64::from(k / 200) * 1.41,
            );
            let d = land_height(&ctx, u + dx, v + dy) - land_height(&ctx, u, v);
            s += d * d;
        }
        s
    };
    let r = 6.0 / std::f64::consts::SQRT_2;
    let v = [gamma(6.0, 0.0), gamma(0.0, 6.0), gamma(r, r), gamma(r, -r)];
    let mean = v.iter().sum::<f64>() / 4.0;
    for x in v {
        assert!((x / mean - 1.0).abs() < 0.1, "variograms {v:?}");
    }
}

#[test]
fn ground_borders_are_isotropic() {
    // Grassland where grass and meadow have similar weight gives many
    // borders; measure their orientation from a smoothed class indicator.
    let mut g = Vec::new();
    let (mut h_changes, mut v_changes) = (0usize, 0usize);
    for seed in 0..3 {
        let src = flat(20 + seed, Cover::Grass, 0);
        for k in [CellKey::new(5, 5), CellKey::new(6, 5), CellKey::new(5, 6)] {
            let b = refine(&src, k).unwrap();
            let ind: Vec<f64> = b
                .ground
                .iter()
                .map(|g| f64::from(u8::from(*g == "grass")))
                .collect();
            for y in 0..64 {
                for x in 0..64 {
                    if x + 1 < 64 && b.ground[y * 64 + x] != b.ground[y * 64 + x + 1] {
                        h_changes += 1;
                    }
                    if y + 1 < 64 && b.ground[y * 64 + x] != b.ground[(y + 1) * 64 + x] {
                        v_changes += 1;
                    }
                }
            }
            // 7 × 7 box blur, then central differences.
            let blur = |x: usize, y: usize| {
                let mut s = 0.0;
                for dy in 0..7 {
                    for dx in 0..7 {
                        s += ind[(y + dy - 3) * 64 + x + dx - 3];
                    }
                }
                s / 49.0
            };
            for y in (6..58).step_by(2) {
                for x in (6..58).step_by(2) {
                    g.push((
                        blur(x + 1, y) - blur(x - 1, y),
                        blur(x, y + 1) - blur(x, y - 1),
                    ));
                }
            }
        }
    }
    let ratio = h_changes as f64 / v_changes as f64;
    assert!(
        (0.85..1.18).contains(&ratio),
        "east/south border ratio {ratio:.3}"
    );
    let f = axis_fraction(&g);
    assert!(f < 0.1667 + 0.07, "axis fraction {f:.3}");
}
