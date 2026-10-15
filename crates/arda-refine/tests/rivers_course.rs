//! Rivers at square scale follow the via-cell rule and the formed thalweg
//! (review 3.4-7), and their banks are shaped narrowly, never leaving water
//! above the land beside it (review 3.4-8).
#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    missing_docs
)]

use arda::{Cell, Cover, TerrainKind};
use arda_core::{DischargeMilli, HeightMm, RainfallMm, TempCentiC};
use arda_refine::context::Ctx;
use arda_refine::source::Edge;
use arda_refine::terrain::{physical, Water};
use arda_refine::{refine, CellKey, GridSource};

fn land(h_mm: i32) -> Cell {
    Cell {
        height: HeightMm::new(h_mm),
        terrain: TerrainKind::Land,
        cover: Cover::Grass,
        slope_milli_deg: 2_000,
        temperature: TempCentiC::new(1_000),
        rainfall: RainfallMm::new(800),
        moisture: 120,
        wetness: 80,
        height_above_river_dm: 30,
        ..Cell::default()
    }
}

fn edge(a: (i64, i64), b: (i64, i64)) -> Edge {
    Edge {
        from: CellKey::new(a.0, a.1),
        to: CellKey::new(b.0, b.1),
        from_width_dm: 60,
        to_width_dm: 60,
        discharge_milli: 2_000,
    }
}

/// A plain falling south-east with a river (2,3) → (3,3) ↘ (4,4) → (5,4);
/// (4,3) is lower than (3,4), so it is the diagonal step's via cell.
fn diagonal_world() -> GridSource {
    let mut g = GridSource::new(9, 9, 9, land(60_000));
    for y in 0..9_i64 {
        for x in 0..9_i64 {
            let mut c = land(60_000 - 1_000 * i32::try_from(x + y).unwrap());
            if (x, y) == (4, 3) {
                c.height = HeightMm::new(52_000);
            }
            if [(2, 3), (3, 3), (4, 4), (5, 4)].contains(&(x, y)) {
                c.watercourse_order = 2;
                c.watercourse_width_dm = 60;
                c.discharge = DischargeMilli::new(2_000);
                c.height_above_river_dm = 0;
            }
            g.set(CellKey::new(x, y), c);
        }
    }
    for (a, b) in [((2, 3), (3, 3)), ((3, 3), (4, 4)), ((4, 4), (5, 4))] {
        g.add_edge(edge(a, b));
    }
    g
}

fn water(b: &arda_refine::Block, i: usize, j: usize) -> bool {
    b.depth_ft[j * 64 + i] > 0
}

#[test]
fn a_diagonal_step_runs_through_its_via_cell_not_the_corner() {
    let g = diagonal_world();
    // The four-cell corner of (3,3) and (4,4) is global square (256, 256).
    let corner_clear = |b: &arda_refine::Block| {
        let (x0, y0) = b.origin;
        (0..64).all(|j| {
            (0..64).all(|i| {
                let (gx, gy) = (x0 + i64::from(i), y0 + i64::from(j));
                let near = (gx - 256).abs() <= 2 && (gy - 256).abs() <= 2;
                !(near && water(b, i as usize, j as usize))
            })
        })
    };
    for c in [(3, 3), (4, 4), (3, 4)] {
        let b = refine(&g, CellKey::new(c.0, c.1)).unwrap();
        assert!(corner_clear(&b), "water at the corner in block {c:?}");
    }
    let via = refine(&g, CellKey::new(4, 3)).unwrap();
    let wet = (0..64 * 64).filter(|&k| via.depth_ft[k] > 0).count();
    assert!(wet > 20, "{wet} water squares in the via cell");
    // The other orthogonal cell stays dry.
    let other = refine(&g, CellKey::new(3, 4)).unwrap();
    assert!(other.depth_ft.iter().all(|&d| d == 0));
}

#[test]
fn channels_pass_through_the_formed_thalwegs() {
    let mut g = diagonal_world();
    // Thalwegs 25 m north-west of (2,3)'s and (3,3)'s centres.
    g.set_thalweg(CellKey::new(2, 3), -160, -160);
    g.set_thalweg(CellKey::new(3, 3), 120, -200);
    let ctx = Ctx::gather(&g, CellKey::new(3, 3)).unwrap();
    let pieces = arda_refine::rivers::cell_pieces(&ctx, CellKey::new(3, 3));
    let t = (
        3.0 * 64.0 + 32.0 + 12.0 / 1.5625,
        3.0 * 64.0 + 32.0 - 20.0 / 1.5625,
    );
    let near = pieces
        .iter()
        .flat_map(|p| p.pts.clone())
        .map(|(x, y)| ((x - t.0).powi(2) + (y - t.1).powi(2)).sqrt())
        .fold(f64::INFINITY, f64::min);
    assert!(near < 0.5, "centreline {near:.2} squares from the thalweg");
}

#[test]
fn banks_are_shaped_narrowly_and_water_never_stands_above_them() {
    let g = diagonal_world();
    for c in [(2, 3), (3, 3), (4, 3), (4, 4)] {
        let k = CellKey::new(c.0, c.1);
        let ctx = Ctx::gather(&g, k).unwrap();
        let pieces = arda_refine::rivers::pieces(&ctx);
        let phys = physical(&ctx, &pieces, k.x * 64, k.y * 64, 64);
        for (i, p) in phys.data.iter().enumerate() {
            if p.water == Water::Dry && p.river_d.is_finite() && (p.elev_m - p.land_m).abs() > 0.05
            {
                assert!(
                    p.river_d <= 8.0,
                    "{c:?}: shaped {:.1} squares from the bank",
                    p.river_d
                );
            }
            if p.water != Water::River {
                continue;
            }
            let (ix, iy) = ((i % 64) as i64, (i / 64) as i64);
            for dy in -2..=2 {
                for dx in -2..=2 {
                    if let Some(q) = phys.get(k.x * 64 + ix + dx, k.y * 64 + iy + dy) {
                        if q.water == Water::Dry {
                            assert!(
                                q.elev_m >= p.elev_m - 0.01,
                                "{c:?}: dry land {:.2} m under water at {:.2} m; dry d {:.2} land {:.2}, water d {:.2} at ({ix},{iy}) off ({dx},{dy})",
                                q.elev_m,
                                p.elev_m, q.river_d, q.land_m, p.river_d
                            );
                        }
                    }
                }
            }
        }
    }
}
