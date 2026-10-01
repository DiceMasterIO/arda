//! Recipe gating (logic/02 §fine-formation recipes): recipe 5 replays v0.1
//! formation and publishes no shore layer or water forms; recipe 6 is the
//! default. The byte-for-byte pins live in `tests/golden_recipes.rs`.

use super::*;

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
    assert_eq!(Recipe::DEFAULT, Recipe::V6);
    for recipe in [Recipe::V5, Recipe::V6] {
        assert_eq!(Recipe::from_version(recipe.version()), Some(recipe));
    }
    for unknown in [0, 4, 7] {
        assert_eq!(Recipe::from_version(unknown), None);
    }
    assert_eq!(
        arda_core::FINE_TERRAIN_RECIPE_VERSION,
        Recipe::DEFAULT.version()
    );
}

#[test]
fn recipe_5_is_deterministic_and_publishes_no_new_layers() {
    let grid = island();
    let form5 = || form_world(42, 0, &grid, (513, 513), u128::MAX, Recipe::V5).unwrap();
    let a = form5();
    assert!(a.shore.is_none() && a.water.is_none());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap();
    assert_eq!(a.lattice, pool.install(form5).lattice);
    let v6 = form_world(42, 0, &grid, (513, 513), u128::MAX, Recipe::V6).unwrap();
    assert!(v6.shore.is_some() && v6.water.is_some());
    assert_ne!(a.lattice.z, v6.lattice.z, "recipe 6 changes the surface");
    assert_eq!(
        form(42, 0, &grid, 513, 513, u128::MAX).unwrap(),
        v6.lattice,
        "form() is the default recipe"
    );
}
