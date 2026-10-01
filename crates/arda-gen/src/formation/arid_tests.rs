//! Recipe-7 arid basins (logic/02 §world-water arid basins): the basin
//! balance on a synthetic macro bowl and the playa built in it.
#![allow(clippy::unwrap_used)]

use super::*;
use crate::formation::macro_view::macro_basins;
use crate::formation::FINE_SPACING_UM;

const KM: i64 = 1_000_000_000;

fn isqrt(v: i64) -> i64 {
    super::super::incision::isqrt(v as u64) as i64
}

/// A 64 km island: sea beyond 28 km from the centre, a 600 m rim ring,
/// and a closed bowl inside 16 km falling to 100 m at the centre.
fn island() -> Lattice {
    let (w, h) = (64, 64);
    let mut g = Lattice::new(w, h, KM).unwrap();
    for (i, z) in g.z.iter_mut().enumerate() {
        let (dx, dy) = ((i % w) as i64 - 32, (i / w) as i64 - 32);
        let r = isqrt(dx * dx + dy * dy);
        *z = if r > 28 {
            -1_000_000
        } else if r > 16 {
            600_000
        } else {
            (100_000 + r * r * 1_900) as i32
        };
    }
    g
}

fn uniform(value: i32) -> Lattice {
    let mut g = Lattice::new(64, 64, KM).unwrap();
    g.z.fill(value);
    g
}

/// The arid basins of the island under uniform runoff and deficit.
fn classify_island(runoff_mm: i32, deficit_mm: i32) -> (Vec<(i64, i64)>, Vec<AridBasin>) {
    let height = island();
    let relief = uniform(0);
    let (runoff, deficit) = (uniform(runoff_mm), uniform(deficit_mm));
    let ocean = super::super::basins::lowstand_ocean(&height.z, 64, 64, LOWSTAND_MM).unwrap();
    let view = MacroView {
        height: &height,
        relief: &relief,
        belt: &relief,
        ocean: &ocean,
        seed: 7,
        extent_um: (63 * KM, 63 * KM),
        v6: true,
        water: Some((&runoff, &deficit)),
    };
    let d = FINE_SPACING_UM << 5;
    let n = (63 * KM / d) as usize + 1;
    let sinks = macro_basins(&view, n, n, d).unwrap();
    let arid = classify(&view, (n, n, d), &sinks).unwrap();
    (sinks, arid)
}

#[test]
fn an_arid_basin_is_terminal_with_a_pan_larger_than_its_lake() {
    // Subtropical: 150 mm runoff, 950 mm extra loss from standing water.
    let (sinks, arid) = classify_island(150, 950);
    assert_eq!(sinks.len(), 1);
    assert_eq!(arid.len(), 1, "{arid:?}");
    let b = arid[0];
    assert_eq!(b.sink, sinks[0]);
    assert!(b.inflow * 1_000 < b.capacity * ARID_PERMILLE, "{b:?}");
    assert!(b.lake_m2 > 0 && b.lake_m2 < b.pan_m2, "{b:?}");
    // The pan never exceeds the closed bowl (~16 km radius).
    assert!(b.pan_m2 <= 820_000_000, "{b:?}");
}

#[test]
fn a_humid_basin_is_not_arid() {
    // Temperate: 600 mm runoff, 100 mm extra loss: the lake spills.
    let (sinks, arid) = classify_island(600, 100);
    assert_eq!(sinks.len(), 1);
    assert!(arid.is_empty(), "{arid:?}");
}

#[test]
fn without_climate_no_basin_is_classified() {
    // Recipes 5 and 6 have no water balance: nothing changes.
    let height = island();
    let relief = uniform(0);
    let view = MacroView {
        height: &height,
        relief: &relief,
        belt: &relief,
        ocean: &[],
        seed: 7,
        extent_um: (63 * KM, 63 * KM),
        v6: true,
        water: None,
    };
    assert!(classify(&view, (10, 10, KM), &[(5 * KM, 5 * KM)])
        .unwrap()
        .is_empty());
}

#[test]
fn weights_follow_runoff_relative_to_the_nominal_500_mm() {
    assert_eq!(weight(500), 256);
    assert_eq!(weight(150), 76);
    assert_eq!(weight(0), 16, "a floor keeps every cell contributing");
    assert_eq!(weight(5_000), 1_024, "capped at four times");
}

/// A drained cone at 100 m spacing falling to a sink at its centre, with a
/// side valley cut into its east flank.
fn cone() -> Lattice {
    let (w, h) = (81, 81);
    let mut g = Lattice::new(w, h, 100_000_000).unwrap();
    for (i, z) in g.z.iter_mut().enumerate() {
        let (dx, dy) = ((i % w) as i64 - 40, (i / w) as i64 - 40);
        let valley = if dy.abs() <= 1 && dx > 0 {
            2_000 * dx
        } else {
            0
        };
        *z = (5_000 + 600 * (dx * dx + dy * dy) - valley) as i32;
    }
    g
}

#[test]
fn a_pan_fills_the_basin_floor_and_records_crust_and_mudflat() {
    let mut g = cone();
    let before = g.clone();
    let basin = AridBasin {
        sink: (40 * 100_000_000, 40 * 100_000_000),
        inflow: 1,
        capacity: 10,
        lake_m2: 200 * 10_000,
        pan_m2: 900 * 10_000,
    };
    let mut features = WaterFeatures::default();
    let n = carve_pans(&mut g, &[basin], (300_000_000, 100_000_000), &mut features).unwrap();
    assert!((850..=950).contains(&n), "{n} nodes");
    let changed: Vec<usize> = (0..g.z.len()).filter(|&i| g.z[i] != before.z[i]).collect();
    assert!(!changed.is_empty() && changed.len() <= n);
    // Sediment fill: the floor only rises and keeps an eighth of its relief.
    let range = |z: &[i32]| {
        let v: Vec<i64> = changed.iter().map(|&i| i64::from(z[i])).collect();
        v.iter().max().unwrap() - v.iter().min().unwrap()
    };
    assert!(changed.iter().all(|&i| g.z[i] > before.z[i]));
    assert!(range(&g.z) <= range(&before.z) / PAN_RELIEF_DIV + 1);
    let kinds = |k: u8| features.pan_cells.iter().filter(|c| c.2 == k).count();
    assert!(kinds(0) > 0 && kinds(1) > 0, "{:?}", (kinds(0), kinds(1)));
    // Crust lies inward of the mudflat margin.
    let mean_r2 = |k: u8| {
        let cells: Vec<_> = features.pan_cells.iter().filter(|c| c.2 == k).collect();
        let s: i64 = cells
            .iter()
            .map(|c| {
                let (dx, dy) = (i64::from(c.0) - 40, i64::from(c.1) - 40);
                dx * dx + dy * dy
            })
            .sum();
        s / cells.len() as i64
    };
    assert!(mean_r2(0) < mean_r2(1));
    assert!(features
        .pan_cells
        .windows(2)
        .all(|p| (p[0].0, p[0].1) < (p[1].0, p[1].1)));
}

#[test]
fn a_pan_stops_at_the_spill() {
    // A target larger than the bowl: the flood stops where it would spill
    // over the rim into the next, lower catchment.
    let (w, h) = (61, 21);
    let mut g = Lattice::new(w, h, 100_000_000).unwrap();
    for (i, z) in g.z.iter_mut().enumerate() {
        let (x, y) = ((i % w) as i64, (i / w) as i64 - 10);
        let bowl = |cx: i64, floor: i64| floor + 300 * ((x - cx).pow(2) + y * y);
        *z = bowl(15, 10_000).min(bowl(45, 1_000)) as i32;
    }
    let before = g.clone();
    let basin = AridBasin {
        sink: (15 * 100_000_000, 10 * 100_000_000),
        inflow: 1,
        capacity: 10,
        lake_m2: 10 * 10_000,
        pan_m2: 10_000 * 10_000,
    };
    let mut features = WaterFeatures::default();
    carve_pans(&mut g, &[basin], (150_000_000, 100_000_000), &mut features).unwrap();
    for i in 0..g.z.len() {
        if i % w >= 31 {
            assert_eq!(g.z[i], before.z[i], "beyond the rim at {}", i % w);
        }
    }
    assert!((0..g.z.len()).any(|i| g.z[i] != before.z[i]));
}
