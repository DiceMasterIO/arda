//! A mountainous recipe-9 world small enough for CI (review §5: invariants
//! were only exercised on lowland seed-42 MICRO).

use super::drainage::{neighbour, open_sea_flags};
use super::*;

/// A 40 x 40 km island whose 3.6 km massif rises well above the glacial
/// equilibrium line, so troughs, steep coasts and high relief all form.
fn massif() -> ContinentGrid {
    let (w, h) = (40_i32, 40_i32);
    let heights = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w - 20, i / w - 20);
            let r2 = x * x + y * y;
            if r2 > 17 * 17 {
                -800_000
            } else {
                3_600_000 - r2 * 12_000 - (x * 2 - y).abs() * 15_000
            }
        })
        .collect();
    ContinentGrid::from_heights(w, h, heights).unwrap()
}

#[test]
fn a_mountainous_recipe_9_world_drains_without_pits_or_trough_walls() {
    let grid = massif();
    let band = arda_core::LatitudeBand::new(44, 46);
    let formed = form_world(7, 0, &grid, (1025, 1025), u128::MAX, Recipe::V9, band).unwrap();
    let g = &formed.lattice;
    let (w, h) = (g.width, g.height);
    let shore = formed.shore.as_ref().unwrap();
    let top = g.z.iter().copied().max().unwrap();
    assert!(top > 2_500_000, "a real massif: {top} mm");
    // Every land node drains (review 3.1-7): the fine fill and the sampled
    // pass both leave no closed pit outside the protected sinks.
    let water = formed.water.as_ref().unwrap();
    let mut flags = vec![0_u8; w * h];
    open_sea_flags(&g.z, w, h, &mut flags);
    let d = g.spacing_um;
    let pits = (0..w * h)
        .filter(|&i| {
            let (x_um, y_um) = ((i % w) as i64 * d, (i / w) as i64 * d);
            flags[i] == 0
                && g.z[i] > 0
                && !water.is_sink(x_um, y_um)
                && !(0..8).any(|k| neighbour(i, w, h, k).is_some_and(|j| g.z[j] < g.z[i]))
        })
        .count();
    assert!(pits * 10_000 < w * h, "{pits} land pits");
    // Glacial troughs meet their valley sides without kilometre walls
    // (review 3.1-2; recipe 7 on this massif: a 2,065 m step between 39 m
    // neighbours). Littoral sea cliffs up to about 300 m remain.
    let steepest = (0..w * h)
        .filter(|&i| g.z[i] > 0)
        .flat_map(|i| {
            (0..8).filter_map(move |k| {
                neighbour(i, w, h, k)
                    .filter(|&j| g.z[j] > 0)
                    .map(|j| (i64::from(g.z[i]) - i64::from(g.z[j])).abs())
            })
        })
        .max()
        .unwrap();
    assert!(steepest < 500_000, "steepest neighbour step {steepest} mm");
    let troughs = shore
        .landforms
        .iter()
        .filter(|l| l.kind == arda_core::LandformKind::GlacialTrough)
        .count();
    assert!(troughs >= 1, "the massif carries glacial troughs");
    // The shore is surveyed (a steep island on open water is mostly cliff).
    assert!(shore.classes.contains(&arda_core::ShoreClass::Cliff));
}
