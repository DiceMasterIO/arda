//! World agreement and seams on a generated world. Ignored by default
//! because it needs the fixture; run with
//! `ARDA_REFINE_WORLD=out/micro42 cargo test -p arda-refine --release -- --ignored`.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unwrap_used,
    missing_docs,
    clippy::cast_precision_loss
)]

use arda::TerrainKind;
use arda_refine::{refine, CellKey, Source, WorldSource};

fn world() -> arda::World {
    let dir = std::env::var("ARDA_REFINE_WORLD").unwrap_or_else(|_| "../../out/micro42".into());
    arda::World::load(std::path::Path::new(&dir)).unwrap()
}

#[test]
#[ignore = "needs a generated world"]
fn a_generated_world_agrees_with_its_cells() {
    let w = world();
    let src = WorldSource::new(&w).unwrap();
    let (mut n, mut off, mut river_ok, mut rivers) = (0, 0, 0, 0);
    for y in (520..1200).step_by(37) {
        for x in (420..980).step_by(41) {
            let k = CellKey::new(x, y);
            let c = src.cell(k).unwrap();
            if c.terrain != TerrainKind::Land {
                continue;
            }
            let b = refine(&src, k).unwrap();
            assert!(!b.relaxed, "{k:?} relaxed");
            let dry: Vec<f64> = (0..4096)
                .filter(|&i| b.depth_ft[i] == 0)
                .map(|i| f64::from(b.elevation_ft[i]) * 0.3048)
                .collect();
            if dry.len() > 2000 {
                let mean = dry.iter().sum::<f64>() / dry.len() as f64;
                let h = f64::from(c.height.raw()) / 1000.0;
                let slope = f64::from(c.slope_milli_deg) / 1000.0;
                let err = (mean - h).abs() / (3.0 + 0.9 * slope);
                if err > 1.0 {
                    off += 1;
                    println!("{k:?} mean {mean:.1} vs cell {h:.1}, slope {slope:.1}");
                }
                n += 1;
            }
            if c.watercourse_order > 0 {
                rivers += 1;
                if b.depth_ft.iter().any(|&d| d > 0) {
                    river_ok += 1;
                }
            }
        }
    }
    println!("{n} cells, {off} outside tolerance, rivers {river_ok}/{rivers}");
    // The fine layer keeps sub-cell hollows that the coarse cell height
    // (a point sample) does not see; allow a few such cells.
    assert!(off * 20 <= n, "{off} of {n} cells outside tolerance");
    assert_eq!(river_ok, rivers);
}

#[test]
#[ignore = "needs a generated world"]
fn a_generated_river_window_is_seamless() {
    let w = world();
    let (cx, cy) = (530, 810);
    let get = |x: i64, y: i64| {
        let src = WorldSource::new(&w).unwrap();
        refine(&src, CellKey::new(x, y)).unwrap()
    };
    for dy in -1..=1 {
        for dx in -1..=1 {
            let a = get(cx + dx, cy + dy);
            let east = get(cx + dx + 1, cy + dy);
            let south = get(cx + dx, cy + dy + 1);
            for k in 0..64i64 {
                let h = *a.halo.get(a.origin.0 + 64, a.origin.1 + k).unwrap();
                let i = usize::try_from(k).unwrap() * 64;
                assert_eq!(h, (east.elevation_ft[i], east.depth_ft[i]));
                let h = *a.halo.get(a.origin.0 + k, a.origin.1 + 64).unwrap();
                let i = usize::try_from(k).unwrap();
                assert_eq!(h, (south.elevation_ft[i], south.depth_ft[i]));
            }
            for k in 0..=64usize {
                assert_eq!(a.corners[k * 65 + 64], east.corners[k * 65]);
                assert_eq!(a.corners[64 * 65 + k], south.corners[k]);
            }
        }
    }
}
