use super::*;

#[test]
fn supplied_surface_is_preserved_without_coast_or_height_correction() {
    let grid = ContinentGrid::from_heights(2, 2, vec![-17, 23, 41, 59]).unwrap();
    assert_eq!(grid.get(0, 0).raw(), -17);
    assert_eq!(grid.get(1, 0).raw(), 23);
    assert_eq!(grid.get(-1, 2).raw(), 41);
    assert_eq!(grid.get(2, 2).raw(), 59);
    assert_eq!(grid.land_fraction_permille(), 750);
    for (w, h, values) in [
        (0, 2, vec![]),
        (-2, -2, vec![0; 4]),
        (2, 2, vec![0; 3]),
        (i32::MAX, 2, vec![]),
    ] {
        assert_eq!(
            ContinentGrid::from_heights(w, h, values),
            Err(ContinentGridError)
        );
    }
}

#[test]
fn micro_continent_grid_is_the_configured_size() {
    let grid = generate_continent(42, GenerateConfig::MICRO);
    assert_eq!(grid.width(), 102);
    assert_eq!(grid.height(), 204);
}

#[test]
fn generation_is_deterministic() {
    let a = generate_continent(42, GenerateConfig::MICRO);
    let b = generate_continent(42, GenerateConfig::MICRO);
    assert_eq!(a, b);
}

#[test]
fn fine_macro_profile_is_repeatable_and_keeps_the_ocean_rim() {
    let fine = generate_continent_attempt_fine(42, GenerateConfig::MICRO, 0);
    assert_eq!(
        fine,
        generate_continent_attempt_fine(42, GenerateConfig::MICRO, 0)
    );
    assert_ne!(
        fine,
        generate_continent_attempt(42, GenerateConfig::MICRO, 0)
    );
    for y in 0..fine.height() {
        for x in 0..fine.width() {
            if rim_forced_ocean(x, y, fine.width(), fine.height(), RIM_MARGIN) {
                assert!(fine.get(x, y).raw() < 0, "fine rim became land at {x},{y}");
            }
        }
    }
}

#[test]
fn different_seeds_give_different_continents() {
    let a = generate_continent(42, GenerateConfig::MICRO);
    let b = generate_continent(43, GenerateConfig::MICRO);
    assert_ne!(a, b);
}

#[test]
fn every_edge_cell_is_ocean() {
    // logic/01 invariant: every map-edge cell is ocean (mockup Q22).
    let grid = generate_continent(42, GenerateConfig::MICRO);
    let (w, h) = (grid.width(), grid.height());
    for x in 0..w {
        assert!(grid.get(x, 0).raw() < 0, "top edge at {x} is land");
        assert!(grid.get(x, h - 1).raw() < 0, "bottom edge at {x} is land");
    }
    for y in 0..h {
        assert!(grid.get(0, y).raw() < 0, "left edge at {y} is land");
        assert!(grid.get(w - 1, y).raw() < 0, "right edge at {y} is land");
    }
}

#[test]
fn generated_ocean_rim_retains_shallow_depths() {
    let grid = generate_continent(42, GenerateConfig::MICRO);
    let rim = (0..grid.height())
        .flat_map(|y| (0..grid.width()).map(move |x| (x, y)))
        .filter(|&(x, y)| rim_forced_ocean(x, y, grid.width(), grid.height(), RIM_MARGIN))
        .map(|(x, y)| grid.get(x, y).raw())
        .collect::<Vec<_>>();
    let shallow = rim
        .iter()
        .copied()
        .filter(|&depth| (-1_050_000..-1).contains(&depth))
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        shallow.len() > 1,
        "the ocean rim lost shallow depth variation"
    );
}

#[test]
fn some_attempt_passes_the_land_fraction_gate() {
    // `logic/01` §Q9 gates land fraction at 25-90% and rerolls up to five
    // times on failure, so the property is that the ladder finds an
    // acceptable continent — not that attempt 0 always does. Asserting the
    // latter made this fail on a seed the batch handles fine.
    for seed in [1u64, 7, 42, 99] {
        let ok = (0..5).any(|attempt| {
            let g = generate_continent_attempt(seed, GenerateConfig::MICRO, attempt);
            (250..=900).contains(&g.land_fraction_permille())
        });
        assert!(ok, "seed {seed} failed the gate on all five attempts");
    }
}

#[test]
fn relief_has_real_range() {
    // A flat plate would pass the other tests; assert the continent
    // actually has mountains.
    let grid = generate_continent(42, GenerateConfig::MICRO);
    let max = (0..grid.height())
        .flat_map(|y| (0..grid.width()).map(move |x| (x, y)))
        .map(|(x, y)| grid.get(x, y).raw())
        .max()
        .unwrap_or(0);
    assert!(max > 400_000, "highest point is only {max} mm");
}

#[test]
#[ignore = "full seed-42 macro comparison for terrain-profile evaluation"]
fn measure_seed42_fine_macro_against_delivered_profile() {
    let config = GenerateConfig::new(
        arda_core::SizeKm::new(500, 1000),
        arda_core::LatitudeBand::new(35, 55),
        15,
    )
    .unwrap();
    let summarize = |label: &str, grid: &ContinentGrid| {
        let mut land = Vec::new();
        let mut plain = Vec::new();
        let mut mountain = Vec::new();
        for y in 0..grid.height() {
            for x in 0..grid.width() {
                let h = grid.get(x, y).raw();
                if h > 0 {
                    land.push(h);
                }
                if (154..205).contains(&x) && (512..563).contains(&y) {
                    plain.push(h);
                }
                if (256..307).contains(&x) && (205..256).contains(&y) {
                    mountain.push(h);
                }
            }
        }
        for values in [&mut land, &mut plain, &mut mountain] {
            values.sort_unstable();
        }
        let percentile = |v: &[i32], p: usize| v[v.len() * p / 100] / 1000;
        eprintln!(
            "{label}: land={}‰, land p50/p95/max={} / {} / {} m, >2850m={}‰ land, plain p5/p95={} / {} m, mountain p5/p95={} / {} m",
            grid.land_fraction_permille(),
            percentile(&land, 50),
            percentile(&land, 95),
            land.last().unwrap() / 1000,
            land.iter().filter(|&&h| h > 2_850_000).count() * 1000 / land.len(),
            percentile(&plain, 5),
            percentile(&plain, 95),
            percentile(&mountain, 5),
            percentile(&mountain, 95),
        );
    };
    let old = generate_continent_attempt(42, config, 0);
    let fine = generate_continent_attempt_fine(42, config, 0);
    summarize("delivered", &old);
    summarize("fine", &fine);

    let sim = SimExtent {
        width: 250,
        height: 500,
    };
    let plates = plates::seed_plates(42, sim, 0);
    let old_uplift = tectonics::run_tectonics(42, &plates, sim, SKELETON_STEPS);
    let fine_uplift = tectonics::run_tectonics_fine(42, &plates, sim, SKELETON_STEPS);
    let mut released_floor: Vec<_> = old_uplift
        .iter()
        .zip(&fine_uplift)
        .filter_map(|(&old, &new)| (old == -220_000).then_some(new))
        .collect();
    released_floor.sort_unstable();
    released_floor.dedup();
    eprintln!(
        "clamped rift cells={} of {}; fine distinct heights at those sites={}",
        old_uplift.iter().filter(|&&h| h == -220_000).count(),
        old_uplift.len(),
        released_floor.len(),
    );
}
