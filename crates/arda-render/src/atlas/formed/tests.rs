//! Formed shading tests.
use super::*;

fn flat(height_mm: i32) -> FormedInputs {
    FormedInputs {
        height_mm,
        position_m: (0, 0),
        position_dm: (0, 0),
        footprint_um: 12_500_000,
        gradients: [(0, 0); 4],
        concavity_mm: 0,
        sky_q12: ONE,
        ring_min_mm: 1_000,
        temperature_centi: 1_500 - i32::try_from(i64::from(height_mm) * 65 / 100_000).unwrap(),
        moisture: 160,
        wetness: 0,
        shore: [0; 8],
        v6: true,
    }
}

#[test]
fn north_west_facing_slopes_are_lit_and_south_east_shadowed() {
    let lit = shade(&FormedInputs {
        gradients: [(2_000, 2_000); 4],
        ..flat(800_000)
    });
    let dark = shade(&FormedInputs {
        gradients: [(-2_000, -2_000); 4],
        ..flat(800_000)
    });
    let sum = |c: [u8; 3]| c.iter().map(|&v| u32::from(v)).sum::<u32>();
    assert!(sum(lit) > sum(shade(&flat(800_000))));
    assert!(sum(dark) < sum(shade(&flat(800_000))));
    assert!(
        dark.iter().all(|&v| v > 20),
        "shadows stay painterly: {dark:?}"
    );
}

#[test]
fn high_crests_are_snow_and_lowlands_green() {
    let snow = shade(&flat(3_600_000));
    assert!(snow.iter().all(|&v| v > 200), "{snow:?}");
    let low = shade(&flat(50_000));
    assert!(low[1] > low[2] && low[1] > 90, "{low:?}");
}

#[test]
fn sea_darkens_with_depth_and_coast_is_brightest() {
    let sum = |c: [u8; 3]| c.iter().map(|&v| u32::from(v)).sum::<u32>();
    let coast = sea(500, (0, 0));
    let shelf = sea(40_000, (0, 0));
    let abyss = sea(4_500_000, (0, 0));
    assert!(sum(coast) > sum(shelf) && sum(shelf) > sum(abyss));
    assert!(abyss[2] > abyss[0], "abyss stays blue: {abyss:?}");
}

#[test]
fn gentle_low_shores_are_sand_and_steep_shores_rock() {
    let beach = shade(&FormedInputs {
        ring_min_mm: -2_000,
        ..flat(800)
    });
    assert!(beach[0] > 150 && beach[0] > beach[2], "{beach:?}");
    let cliff = shade(&FormedInputs {
        ring_min_mm: -2_000,
        gradients: [(3_000, -3_000), (0, 0), (0, 0), (0, 0)],
        ..flat(30_000)
    });
    assert!(cliff[1] < beach[1], "{cliff:?} vs {beach:?}");
}

#[test]
fn lake_margin_is_lighter_than_its_depths() {
    let sum = |c: [u8; 3]| c.iter().map(|&v| u32::from(v)).sum::<u32>();
    let margin = lake(500, true);
    let deep = lake(120_000, true);
    assert!(sum(margin) > sum(deep));
    assert!(
        deep[2] > deep[0] && margin[2] > margin[0],
        "lakes stay blue"
    );
}

#[test]
fn soft_light_saturates() {
    assert_eq!(soft_q12(0), 0);
    assert_eq!(soft_q12(20 * ONE), ONE);
    assert_eq!(soft_q12(-20 * ONE), -ONE);
}

#[test]
fn stored_beaches_are_sand_and_stored_cliffs_rock() {
    let mut beach = flat(1_500);
    beach.shore[1] = ONE;
    let mut cliff = flat(1_500);
    cliff.shore[3] = ONE;
    cliff.gradients = [(2_000, 0); 4];
    let (b, c) = (shade(&beach), shade(&cliff));
    assert!(b[0] > 170 && b[0] > b[2] + 30, "sand {b:?}");
    assert!(c[1] < b[1], "cliff {c:?} vs beach {b:?}");
    // Without a shore layer the old heuristic still applies.
    assert_eq!(
        shade(&flat(1_500)),
        shade(&FormedInputs {
            shore: [0; 8],
            ..flat(1_500)
        })
    );
}

#[test]
fn tidal_flats_and_estuaries_tint_the_water() {
    let deep = [20_u8, 80, 120];
    let mut w = [0_i64; 8];
    w[6] = ONE;
    let flat = shore_water(deep, 300, &w);
    assert!(flat[0] > deep[0] + 60, "flat {flat:?}");
    assert_eq!(shore_water(deep, 300, &[0; 8]), deep);
}

#[test]
fn recipe_5_worlds_keep_the_v0_1_look() {
    // logic/04 §atlas-formed recipes: recipe 5 keeps the v0.1 palette and
    // lake stops; recipe 6 draws the v0.2 look.
    let v6 = flat(300_000);
    let v5 = FormedInputs { v6: false, ..v6 };
    assert_eq!(shade(&v5), v5::shade(&v5));
    assert_ne!(shade(&v5), shade(&v6));
    assert_eq!(lake(0, false), [0x6f, 0xb6, 0xcc]);
    assert_eq!(lake(0, true), [0x74, 0xb8, 0xc8]);
}
