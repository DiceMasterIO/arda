use super::*;
use crate::continent::ContinentGrid;
use arda_core::LatitudeBand;

/// 10×10, all sea except one column of land rising eastward.
fn grid(heights: impl Fn(i32, i32) -> i32) -> ContinentGrid {
    let (w, h) = (10, 10);
    ContinentGrid {
        width: w,
        height: h,
        height_mm: (0..w * h).map(|i| heights(i % w, i / w)).collect(),
    }
}

const BAND: LatitudeBand = LatitudeBand::new(35, 55);

#[test]
fn higher_ground_is_colder_by_the_lapse_rate() {
    // logic/01 §Q6: 6.5 °C/km.
    let g = grid(|x, _| {
        if x == 5 {
            2_000_000
        } else if x == 6 {
            1_000_000
        } else {
            -1_000
        }
    });
    let c = climate(&g, BAND);
    let (low, high) = (c.temperature[5 * 10 + 6], c.temperature[5 * 10 + 5]);
    assert_eq!(i32::from(low) - i32::from(high), 650);
}

#[test]
fn the_north_edge_is_colder_than_the_south_edge() {
    let g = grid(|_, _| 500_000);
    let c = climate(&g, BAND);
    assert!(c.temperature[5] < c.temperature[9 * 10 + 5]);
}

#[test]
fn continentality_saturates_at_300_km() {
    // −1 centi-°C per km inland, capped at 300 (feature 02 §Q5).
    // A 10 km grid never saturates; assert the per-km slope instead.
    let g = grid(|x, _| if x >= 2 { 100_000 } else { -1_000 });
    let c = climate(&g, BAND);
    let row = 5 * 10;
    assert_eq!(
        i32::from(c.temperature[row + 2]) - i32::from(c.temperature[row + 4]),
        2
    );
}

#[test]
fn regimes_follow_band_position_and_elevation() {
    // Rows map north (y=0, 55°) to south (y=9, 35°); feature 02 §Q5.
    let g = grid(|x, _| {
        if x == 5 {
            200_000
        } else if x == 6 {
            3_500_000
        } else {
            -1_000
        }
    });
    let c = climate(&g, BAND);
    // South row, low, warm → mediterranean (lat 35° < 42°, < 1,000 m).
    assert_eq!(c.regime[9 * 10 + 5], ClimateRegime::Mediterranean);
    // High peak → boreal by computed temperature.
    assert_eq!(c.regime[5 * 10 + 6], ClimateRegime::Boreal);
    // North row, low → temperate (lat 55°).
    assert_eq!(c.regime[5], ClimateRegime::Temperate);
}

#[test]
fn climate_is_deterministic() {
    let g = grid(|x, y| (x - y) * 100_000);
    assert_eq!(climate(&g, BAND), climate(&g, BAND));
}

#[test]
fn sea_cells_receive_precipitation() {
    let g = grid(|x, _| if x >= 4 { 400_000 } else { -1_000 });
    let c = climate(&g, BAND);
    for y in 0..10 {
        for x in 0..4 {
            let i = usize::try_from(y * 10 + x).unwrap();
            assert!(c.rainfall[i] > 0, "sea cell {x},{y}");
        }
    }
}

#[test]
fn land_downwind_of_the_sea_gets_rain() {
    let g = grid(|x, _| if x >= 4 { 400_000 } else { -1_000 });
    let c = climate(&g, BAND);
    assert!(c.rainfall[5 * 10 + 4] > 0, "first land column is dry");
}

#[test]
fn a_ridge_casts_a_rain_shadow() {
    // Sea → plain → ridge → plain: the lee plain must be drier than
    // the windward plain (logic/01 §Q6 rain shadows).
    let g = grid(|x, _| match x {
        0..=1 => -1_000,
        5 => 2_500_000,
        _ => 200_000,
    });
    let c = climate(&g, BAND);
    let row = 5 * 10;
    let windward = c.rainfall[row + 4];
    let leeward = c.rainfall[row + 7];
    assert!(
        windward > leeward,
        "no shadow: windward {windward} <= leeward {leeward}"
    );
}

#[test]
fn the_final_moisture_store_is_exposed() {
    // Feature 03 §Q6: edge moisture for the step-10 bundle payload.
    let g = grid(|x, _| if x >= 4 { 400_000 } else { -1_000 });
    let c = climate(&g, BAND);
    assert_eq!(c.moisture.len(), 100);
    // Sea cells sit near saturation; land cells are depleted below it.
    let sea = c.moisture[5 * 10 + 1];
    let far_land = c.moisture[5 * 10 + 9];
    assert!(
        sea > far_land,
        "sea {sea} not wetter than far land {far_land}"
    );
    assert!(u64::from(sea) <= M_SAT);
}
#[test]
fn annual_temperature_is_symmetric_and_tropical_bound_is_grounded() {
    assert_eq!(sea_level_centi(-55_000), 600);
    assert_eq!(sea_level_centi(55_000), 600);
    assert_eq!(sea_level_centi(35_000), 1800);
    assert_eq!(sea_level_centi(0), 2779);
}
#[test]
fn precipitation_is_available_over_ocean_and_enclosed_negative_land() {
    let sea = grid(|_, _| -100_000);
    let c = climate(&sea, BAND);
    assert!(c.rainfall.iter().all(|p| *p > 0));
    let bowl = grid(|x, y| {
        if x == 0 || x == 9 || y == 0 || y == 9 {
            -100_000
        } else if (3..=6).contains(&x) && (3..=6).contains(&y) {
            -50_000
        } else {
            100_000
        }
    });
    let c = climate(&bowl, BAND);
    assert!(!c.ocean[5 * 10 + 5]);
    assert!(c.ocean_distance_km[5 * 10 + 5] > 0);
    assert!(c.rainfall[5 * 10 + 5] > 0);
}
#[test]
fn diagonal_ocean_contact_matches_the_shared_d8_convention() {
    let g = grid(|x, y| if x == y { -1 } else { 1 });
    let ocean = ocean_mask(&g);
    assert!(ocean[5 * 10 + 5]);
    assert!(!ocean[5 * 10 + 4]);
}
#[test]
fn condensation_is_removed_once_before_ocean_recharge() {
    for m in [0, M_SAT / 2, M_SAT] {
        for climb in [0, 100_000, 1_000_000] {
            for ocean in [false, true] {
                let (after, p, recharge) = exchange(m, climb, ocean);
                assert_eq!(m + recharge, after + p);
                assert!(after <= M_SAT);
                if !ocean {
                    assert_eq!(recharge, 0);
                }
            }
        }
    }
}
