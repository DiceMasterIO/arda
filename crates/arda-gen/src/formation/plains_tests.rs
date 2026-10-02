//! Recipe-8 plains (logic/02 §fine-formation plains).

use super::*;

const D: i64 = 39_062_500;
const W: usize = 201;
const H: usize = 600;
const XC: usize = 100;

/// A straight valley along x = 100 draining south to a sea row, its bed
/// falling `bed_mm` per cell and its sides rising `side_mm` per cell.
fn valley(bed_mm: i32, side_mm: i32) -> Lattice {
    let mut g = Lattice::new(W, H, D).unwrap();
    for y in 0..H {
        for x in 0..W {
            let bed = 2_000 + (H - 1 - y) as i32 * bed_mm;
            g.z[y * W + x] = bed + (x as i32 - XC as i32).abs() * side_mm;
        }
    }
    for x in 0..W {
        g.z[(H - 1) * W + x] = -2_000;
    }
    g
}

fn lowland(_: i64, _: i64) -> Setting {
    Setting {
        belt_m: 50,
        runoff_q8: 256,
        sink: false,
    }
}

fn hills(_: i64, _: i64) -> Setting {
    Setting {
        belt_m: 900,
        ..lowland(0, 0)
    }
}

/// Cells across row `y` within `tol_mm` of the channel cell.
fn floor_cells(g: &Lattice, y: usize, tol_mm: i32) -> usize {
    let c = g.z[y * W + XC];
    (0..W)
        .filter(|&x| (g.z[y * W + x] - c).abs() <= tol_mm)
        .count()
}

#[test]
fn a_low_gradient_lowland_valley_gets_a_wide_flat_floor() {
    // 0.5‰ bed, 1° sides: a V-shaped valley 4 km above the sea.
    let mut g = valley(20, 680);
    let before = g.clone();
    let y = 500;
    assert!(floor_cells(&before, y, 1_000) <= 3);
    let changed = alluvial_floors(&mut g, &lowland, 7).unwrap();
    assert!(changed > 1_000, "{changed}");
    // About 180 km² at row 500: half-width 60 m × 180^0.4 ≈ 480 m, so the
    // floor is 20-35 cells (0.8-1.4 km) wide, all within 1 m of bankfull.
    let wide = floor_cells(&g, y, 1_000);
    assert!((16..=40).contains(&wide), "floor cells {wide}");
    // The floor beside the channel is nearly flat across: no step over
    // 60 mm (0.09°).
    for x in (XC - 7..XC - 1).chain(XC + 1..XC + 7) {
        let s = (g.z[y * W + x + 1] - g.z[y * W + x]).abs();
        assert!(s <= 60, "step {s} at {x}");
    }
    // The valley side beyond the floor is untouched.
    assert_eq!(g.z[y * W + XC + 60], before.z[y * W + XC + 60]);
    // The floor rises gently away from the channel, so it drains to it.
    assert!(g.z[y * W + XC + 4] > g.z[y * W + XC + 1]);
    assert!(g.z[y * W + XC + 1] > g.z[y * W + XC]);
}

#[test]
fn the_floor_widens_downstream_with_discharge() {
    let mut g = valley(20, 680);
    alluvial_floors(&mut g, &lowland, 7).unwrap();
    let (up, down) = (floor_cells(&g, 150, 1_000), floor_cells(&g, 560, 1_000));
    assert!(down > up, "upstream {up}, downstream {down}");
}

#[test]
fn a_bluff_bounds_the_floor_on_a_steeper_valley_side() {
    // 2° sides: the floor is cut into the side up to the 3 D bluff, and the
    // side above it keeps its slope.
    let mut g = valley(20, 1_360);
    let before = g.clone();
    alluvial_floors(&mut g, &lowland, 7).unwrap();
    let y = 500;
    let wide = floor_cells(&g, y, 1_000);
    assert!((8..=30).contains(&wide), "floor cells {wide}");
    assert_eq!(g.z[y * W + XC + 40], before.z[y * W + XC + 40]);
    // The bluff is steeper than the side it was cut into.
    let steepest = (XC + 1..XC + 40)
        .map(|x| g.z[y * W + x + 1] - g.z[y * W + x])
        .max()
        .unwrap();
    assert!(steepest > 1_360 * 3 / 2, "steepest step {steepest}");
}

#[test]
fn a_confined_valley_keeps_its_v_shape() {
    // 8° sides: the bluff would stand a few cells from the channel, a
    // trench rather than a floodplain, so no floor is built.
    let mut g = valley(20, 5_500);
    let before = g.clone();
    alluvial_floors(&mut g, &lowland, 7).unwrap();
    assert!(g.z == before.z, "a confined valley was floored");
}

#[test]
fn hill_country_and_steep_channels_keep_their_valleys() {
    let mut g = valley(20, 680);
    let before = g.clone();
    assert_eq!(alluvial_floors(&mut g, &hills, 7).unwrap(), 0);
    assert_eq!(g, before);
    // A 4% channel is not alluvial either.
    let mut g = valley(1_600, 680);
    let before = g.clone();
    assert_eq!(alluvial_floors(&mut g, &lowland, 7).unwrap(), 0);
    assert_eq!(g, before);
}

#[test]
fn plains_are_deterministic_across_thread_counts() {
    let run = || {
        let mut g = valley(20, 680);
        apply(&mut g, &lowland, 3).unwrap();
        g
    };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    assert_eq!(run(), pool.install(run));
}
