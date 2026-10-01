//! World agreement (goal 42): heights, slopes, water and tree density follow
//! the cell data.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unwrap_used,
    missing_docs,
    clippy::cast_precision_loss
)]

mod common;
use arda::TerrainKind;
use arda_refine::scatter::Kind;
use arda_refine::terrain::atan_deg;
use arda_refine::{refine, CellKey, Source};

const FT: f64 = 0.3048;

#[test]
fn block_mean_height_tracks_the_cell() {
    let src = common::world(42);
    for y in 0..common::SEA_ROW {
        for x in 0..i64::from(common::W) {
            let k = CellKey::new(x, y);
            let c = src.cell(k).unwrap();
            // Shore and channel cells are reshaped toward the water level on
            // purpose; they are checked by the water test instead.
            let wet_near = (-1..=1).any(|dy| {
                (-1..=1).any(|dx| src.cell(k.offset(dx, dy)).unwrap().terrain != TerrainKind::Land)
            });
            if c.terrain != TerrainKind::Land || c.watercourse_order > 0 || wet_near {
                continue;
            }
            let b = refine(&src, k).unwrap();
            let dry: Vec<f64> = (0..4096)
                .filter(|&i| b.depth_ft[i] == 0)
                .map(|i| f64::from(b.elevation_ft[i]) * FT)
                .collect();
            let mean = dry.iter().sum::<f64>() / dry.len() as f64;
            let h = f64::from(c.height.raw()) / 1000.0;
            let slope = f64::from(c.slope_milli_deg) / 1000.0;
            // Curvature and river carving move the mean a little; steep
            // cells get proportionally more room.
            let tol = 2.5 + 0.15 * slope;
            assert!(
                (mean - h).abs() < tol,
                "cell {x},{y}: mean {mean:.1} vs {h:.1} (tol {tol:.1})"
            );
        }
    }
}

#[test]
fn slope_distribution_matches_the_cell_slope() {
    let src = common::world(42);
    for k in [
        CellKey::new(2, 9),
        CellKey::new(4, 11),
        CellKey::new(1, 2),
        CellKey::new(13, 9),
        CellKey::new(3, 12),
    ] {
        let c = src.cell(k).unwrap();
        let b = refine(&src, k).unwrap();
        // Mean slope over 8-square baselines, from the stored feet.
        let e = |x: usize, y: usize| f64::from(b.elevation_ft[y * 64 + x]) * FT;
        let mut sum = 0.0;
        let mut n = 0.0;
        for y in (4..56).step_by(4) {
            for x in (4..56).step_by(4) {
                let gx = (e(x + 8, y) - e(x, y)) / (8.0 * 1.5625);
                let gy = (e(x, y + 8) - e(x, y)) / (8.0 * 1.5625);
                sum += atan_deg((gx * gx + gy * gy).sqrt());
                n += 1.0;
            }
        }
        let mean = sum / n;
        let cell = f64::from(c.slope_milli_deg) / 1000.0;
        assert!(
            (mean - cell).abs() < 3.0 + 0.3 * cell,
            "cell {k:?}: squares {mean:.1} vs cell {cell:.1} degrees"
        );
    }
}

#[test]
fn water_is_where_the_cells_say_and_nowhere_else() {
    let src = common::world(42);
    for y in 0..i64::from(common::W) {
        for x in 0..i64::from(common::W) {
            let k = CellKey::new(x, y);
            let c = src.cell(k).unwrap();
            let b = refine(&src, k).unwrap();
            let wet = b.depth_ft.iter().filter(|&&d| d > 0).count();
            let near_standing = (-1..=1).any(|dy| {
                (-1..=1).any(|dx| src.cell(k.offset(dx, dy)).unwrap().terrain != TerrainKind::Land)
            });
            let near_river = (-1..=1).any(|dy| {
                (-1..=1).any(|dx| src.cell(k.offset(dx, dy)).unwrap().watercourse_order > 0)
            });
            let marsh = common::MARSH.contains(&(x, y));
            match c.terrain {
                TerrainKind::Sea | TerrainKind::Lake => {
                    assert!(
                        wet > 4096 * 6 / 10,
                        "{k:?} water cell only {wet} wet squares"
                    );
                }
                TerrainKind::Land if c.watercourse_order > 0 => {
                    assert!(wet > 64, "{k:?} river cell only {wet} wet squares");
                }
                TerrainKind::Land if !near_standing && !near_river && !marsh => {
                    assert_eq!(wet, 0, "{k:?} dry cell has {wet} wet squares");
                }
                TerrainKind::Land if !marsh => {
                    assert!(wet < 4096 / 2, "{k:?} land cell mostly water ({wet})");
                }
                TerrainKind::Land => {}
            }
        }
    }
}

#[test]
fn tree_density_follows_forest_density() {
    let src = common::world(42);
    // Trees grow in groves and glades (logic/09 §scatter), so one cell's
    // count varies with where the groves fall; each column of four cells
    // shares one forest density, and its mean is what must follow it.
    let mut pts = Vec::new();
    for x in 0..9 {
        let mut sum = 0.0;
        let mut fd = 0.0;
        for y in 1..5 {
            let k = CellKey::new(x, y);
            fd = f64::from(src.cell(k).unwrap().forest_density) / 255.0;
            let b = refine(&src, k).unwrap();
            sum += b.items.iter().filter(|i| i.kind == Kind::TreeLarge).count() as f64;
        }
        pts.push((fd, sum / 4.0));
    }
    // Trees per unit density is roughly constant (within 35 %)...
    let dense: Vec<f64> = pts
        .iter()
        .filter(|p| p.0 > 0.5)
        .map(|p| p.1 / p.0)
        .collect();
    let mean = dense.iter().sum::<f64>() / dense.len() as f64;
    for (fd, t) in &pts {
        if *fd > 0.3 {
            let ratio = t / fd / mean;
            assert!(
                (0.65..1.35).contains(&ratio),
                "fd {fd:.2}: {t} trees, ratio {ratio:.2}"
            );
        }
    }
    // ...and the sparse cells get far fewer.
    let sparse: f64 = pts.iter().filter(|p| p.0 < 0.1).map(|p| p.1).sum::<f64>()
        / pts.iter().filter(|p| p.0 < 0.1).count().max(1) as f64;
    let full = pts.iter().filter(|p| p.0 > 0.85).map(|p| p.1).sum::<f64>()
        / pts.iter().filter(|p| p.0 > 0.85).count().max(1) as f64;
    assert!(
        full > 60.0 && sparse < full * 0.2,
        "full {full}, sparse {sparse}"
    );
}
