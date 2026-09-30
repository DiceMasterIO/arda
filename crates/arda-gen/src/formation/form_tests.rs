//! Whole-formation behaviour tests (logic/02 §fine-formation).

use super::drainage::neighbour;
use super::*;

/// A 24 x 24 km island: a 2.4 km ridge in a rim of ocean.
fn island() -> ContinentGrid {
    let (w, h) = (24_i32, 24_i32);
    let heights = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w - 12, i / w - 12);
            let r2 = x * x + y * y;
            if r2 > 90 {
                -200_000
            } else {
                2_400_000 - r2 * 24_000 - (x * 3 + y).abs() * 20_000
            }
        })
        .collect();
    ContinentGrid::from_heights(w, h, heights).unwrap()
}

#[test]
fn formation_is_deterministic_and_thread_count_independent() {
    let grid = island();
    let a = form(42, 0, &grid, 513, 513, u128::MAX).unwrap();
    let b = form(42, 0, &grid, 513, 513, u128::MAX).unwrap();
    assert_eq!(a, b);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let c = pool.install(|| form(42, 0, &grid, 513, 513, u128::MAX).unwrap());
    assert_eq!(a, c);
    let other = form(43, 0, &grid, 513, 513, u128::MAX).unwrap();
    assert_ne!(a.z, other.z, "seed must change the formed surface");
}

#[test]
fn formed_land_drains_everywhere_and_keeps_the_macro_coast() {
    let grid = island();
    let g = form(7, 0, &grid, 513, 513, u128::MAX).unwrap();
    let (w, h) = (g.width, g.height);
    let mut land = 0;
    let mut pits = 0;
    for i in 0..w * h {
        let (x, y) = (i % w, i / w);
        if g.z[i] <= 0 || x == 0 || y == 0 || x == w - 1 || y == h - 1 {
            continue;
        }
        land += 1;
        let lower = (0..8).any(|k| neighbour(i, w, h, k).is_some_and(|n| g.z[n] < g.z[i]));
        pits += usize::from(!lower);
    }
    assert!(land > 50_000, "island interior must remain land: {land}");
    // Floodplain flats may leave isolated equal-height cells; closed
    // depressions must stay negligible (logic/02 §fine-formation lakes).
    assert!(pits * 1000 < land, "{pits} pits in {land} land cells");
    let peak = g.z.iter().copied().max().unwrap();
    assert!((1_200_000..3_500_000).contains(&peak), "peak {peak} mm");
}

#[test]
fn a_large_tectonic_basin_survives_as_a_closed_depression() {
    // 48 x 48 km island rising to a 1.5 km rim around a 26 km wide
    // basin whose floor lies 300 m below the rim's lowest saddle.
    let (w, h) = (48_i32, 48_i32);
    let heights = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w - 24, i / w - 24);
            let r2 = x * x + y * y;
            if r2 > 22 * 22 {
                -300_000
            } else if r2 < 13 * 13 {
                200_000
            } else {
                1_500_000
            }
        })
        .collect();
    let grid = ContinentGrid::from_heights(w, h, heights).unwrap();
    let g = form(3, 0, &grid, 1_229, 1_229, u128::MAX).unwrap();
    // Closure, not just a low floor: filling against the open sea must
    // raise the basin centre, i.e. no canyon drains it and no fill has
    // buried it (logic/02 §fine-formation basins).
    let (w, h) = (g.width, g.height);
    let mut flags = vec![0; w * h];
    drainage::open_sea_flags(&g.z, w, h, &mut flags);
    let mut filled = g.z.clone();
    let (mut next, mut closed) = (vec![0; w * h], vec![0; w * h]);
    drainage::fill(&mut filled, w, h, &flags, 0, &mut next, &mut closed).unwrap();
    let c = 614 * w + 614;
    assert!(
        filled[c] - g.z[c] >= 50_000,
        "basin centre {} mm is not closed (spill {} mm)",
        g.z[c],
        filled[c]
    );
}

#[test]
fn admission_refuses_before_allocating() {
    let grid = island();
    assert!(matches!(
        form(42, 0, &grid, 513, 513, 1 << 20),
        Err(FormationError::ResourceLimit { .. })
    ));
    let full = plan(12_808, 25_608, u128::MAX).unwrap();
    assert!(
        full.peak_bytes < 16 << 30,
        "full world {} bytes",
        full.peak_bytes
    );
}
