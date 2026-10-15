//! Recipe gating (logic/02 §fine-formation recipes): recipe 5 replays v0.1
//! formation and publishes no shore layer or water forms; recipe 9 is the
//! default. The byte-for-byte pins live in `tests/golden_recipes.rs`.

use super::*;

const BAND: LatitudeBand = LatitudeBand::new(35, 55);

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
fn recipe_numbers_round_trip() {
    assert_eq!(Recipe::DEFAULT, Recipe::V9);
    assert_eq!(Recipe::LATEST, Recipe::V9);
    for recipe in [Recipe::V5, Recipe::V6, Recipe::V7, Recipe::V8, Recipe::V9] {
        assert_eq!(Recipe::from_version(recipe.version()), Some(recipe));
    }
    for unknown in [0, 4, 10] {
        assert_eq!(Recipe::from_version(unknown), None);
    }
    assert_eq!(
        arda_core::FINE_TERRAIN_RECIPE_VERSION,
        Recipe::DEFAULT.version()
    );
    assert_eq!(
        arda_core::FINE_TERRAIN_LATEST_RECIPE_VERSION,
        Recipe::LATEST.version()
    );
    // Recipe 9 is the default on every path (manifest constant, formation,
    // the fine-source orchestrator).
    assert_eq!(
        crate::orchestrator::FineRecipe::DEFAULT.version(),
        arda_core::FINE_TERRAIN_RECIPE_VERSION
    );
    assert_eq!(arda_core::FINE_TERRAIN_RECIPE_VERSION, 9);
}

#[test]
fn recipe_5_is_deterministic_and_publishes_no_new_layers() {
    let grid = island();
    let form5 = || form_world(42, 0, &grid, (513, 513), u128::MAX, Recipe::V5, BAND).unwrap();
    let a = form5();
    assert!(a.shore.is_none() && a.water.is_none());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    assert_eq!(a.lattice, pool.install(form5).lattice);
    let v6 = form_world(42, 0, &grid, (513, 513), u128::MAX, Recipe::V6, BAND).unwrap();
    assert!(v6.shore.is_some() && v6.water.is_some());
    assert_ne!(a.lattice.z, v6.lattice.z, "recipe 6 changes the surface");
    let v7 = form_world(42, 0, &grid, (513, 513), u128::MAX, Recipe::V7, BAND).unwrap();
    assert!(v7.shore.is_some() && v7.water.is_some());
}

#[test]
fn recipe_9_is_deterministic_and_differs_from_recipe_7() {
    let grid = island();
    let form9 = || form_world(42, 0, &grid, (513, 513), u128::MAX, Recipe::V9, BAND).unwrap();
    let a = form9();
    assert!(a.shore.is_some() && a.water.is_some());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    let b = pool.install(form9);
    assert_eq!(a.lattice, b.lattice, "recipe 9 is thread-count independent");
    assert_eq!(
        form(42, 0, &grid, 513, 513, u128::MAX).unwrap(),
        a.lattice,
        "form() is the default recipe"
    );
    assert_eq!(a.shore, b.shore);
    let v7 = form_world(42, 0, &grid, (513, 513), u128::MAX, Recipe::V7, BAND).unwrap();
    assert_ne!(a.lattice.z, v7.lattice.z, "recipe 9 changes the surface");
}
