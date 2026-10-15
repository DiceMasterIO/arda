use super::*;
use crate::continent::aridity::climate_recipe7;

const MID: LatitudeBand = LatitudeBand::new(44, 46);

/// `w` km of flat 50 m land between a 4 km sea on the west and the rim.
fn strip(w: i32, h: i32) -> ContinentGrid {
    let heights = (0..w * h)
        .map(|i| {
            let x = i % w;
            if x < 4 || x + 1 == w {
                -1_000_000
            } else {
                50_000
            }
        })
        .collect();
    ContinentGrid::from_heights(w, h, heights).unwrap()
}

/// Land rainfall at column `x` of the middle row.
fn rain_at(c: &ContinentClimate, w: i32, h: i32, x: i32) -> f64 {
    f64::from(c.rainfall[usize::try_from((h / 2) * w + x).unwrap()])
}

/// Review 3.2-4: recipe 7 averages its warm-up passes, so the leeward end
/// of a flat continent is far wetter than the steady state, in which the
/// moisture decays by the base release, (1 − 1/512) per km. Recipe 9
/// solves the steady state.
#[test]
fn recipe_9_rain_decays_to_the_steady_state_across_a_wide_continent() {
    let (w, h) = (1_006, 9);
    let g = strip(w, h);
    let (c7, c9) = (climate_recipe7(&g, MID), climate_recipe9(&g, MID));
    let (west, east) = (50, 1_004);
    let steady = (1.0 - 1.0 / 512.0_f64).powi(east - west);
    let r7 = rain_at(&c7, w, h, east) / rain_at(&c7, w, h, west);
    let r9 = rain_at(&c9, w, h, east) / rain_at(&c9, w, h, west);
    eprintln!(
        "east/west rain over 1,000 km: steady {steady:.3}, recipe 7 {r7:.3}, recipe 9 {r9:.3}"
    );
    assert!(r7 > steady * 1.8, "recipe 7 leeward edge too wet: {r7:.3}");
    assert!((r9 / steady - 1.0).abs() < 0.1, "recipe 9 steady: {r9:.3}");
    // Same calibration on the windward coast.
    let (w7, w9) = (rain_at(&c7, w, h, west), rain_at(&c9, w, h, west));
    assert!((w9 / w7 - 1.0).abs() < 0.05, "coast rain {w7} vs {w9}");
}

/// A 300 m coastal plateau rising straight from a western sea.
fn plateau_coast(w: i32, h: i32) -> ContinentGrid {
    let heights = (0..w * h)
        .map(|i| {
            let x = i % w;
            if x < 20 || x + 1 == w {
                -1_000_000
            } else {
                300_000
            }
        })
        .collect();
    ContinentGrid::from_heights(w, h, heights).unwrap()
}

/// Largest land rain over the mean of the first 20 km of land, and the
/// largest land rain (mm), middle row.
fn coast_spike(c: &ContinentClimate, w: i32, h: i32) -> (f64, f64) {
    let max = (20..w - 1).map(|x| rain_at(c, w, h, x)).fold(0.0, f64::max);
    let coast = (20..40).map(|x| rain_at(c, w, h, x)).sum::<f64>() / 20.0;
    (max / coast, max)
}

/// Review 3.2-2: recipe 7 smooths land with the sea counted as 0 but then
/// restores the sea's own heights, so the first land cell climbs the whole
/// smoothed coastal height at once and every west coast gets a rain spike
/// (5.4x elsewhere on seed 42). Recipe 9 keeps the smoothed ramp offshore
/// and caps the release at 1/32.
#[test]
fn recipe_9_west_coasts_have_no_rain_spike() {
    let (w, h) = (200, 9);
    let g = plateau_coast(w, h);
    let (s7, s9) = (
        coast_spike(&climate_recipe7(&g, MID), w, h),
        coast_spike(&climate_recipe9(&g, MID), w, h),
    );
    eprintln!(
        "coastal spike (peak over 20 km mean, peak mm): recipe 7 {s7:.2?}, recipe 9 {s9:.2?}"
    );
    assert!(s7.0 >= 4.0, "recipe 7 spike {s7:.2?}");
    assert!(s9.0 <= 2.0, "recipe 9 spike {s9:.2?}");
    assert!(s9.1 * 3.0 < s7.1, "recipe 9 peak {s9:.2?} vs {s7:.2?}");
}

/// A 300 km continent with a 1,500 m north-south range in the middle.
fn ridge(w: i32, h: i32) -> ContinentGrid {
    let heights = (0..w * h)
        .map(|i| {
            let x = i % w;
            if x < 20 || x >= w - 20 {
                -1_000_000
            } else {
                let d = (x - w / 2).abs();
                (1_500_000 - d * 30_000).max(100_000)
            }
        })
        .collect();
    ContinentGrid::from_heights(w, h, heights).unwrap()
}

/// Mean rain over the east flank minus the west flank, middle row, mm.
fn east_minus_west(c: &ContinentClimate, w: i32, h: i32) -> f64 {
    let flank =
        |a: i32, b: i32| (a..b).map(|x| rain_at(c, w, h, x)).sum::<f64>() / f64::from(b - a);
    flank(w / 2 + 5, w / 2 + 45) - flank(w / 2 - 45, w / 2 - 5)
}

/// Review 3.2-3: recipe 7 blows from the west at every latitude. In the
/// trade-wind belt recipe 9's wind is easterly, so the east flank of a
/// range is the wet one there, and the west flank stays wet at 45°.
#[test]
fn recipe_9_trade_winds_put_the_rain_shadow_on_the_west() {
    let (w, h) = (300, 9);
    let g = ridge(w, h);
    let trades = LatitudeBand::new(18, 22);
    let (t7, t9) = (
        east_minus_west(&climate_recipe7(&g, trades), w, h),
        east_minus_west(&climate_recipe9(&g, trades), w, h),
    );
    let m9 = east_minus_west(&climate_recipe9(&g, MID), w, h);
    eprintln!("east minus west flank rain: trades recipe 7 {t7:.0}, recipe 9 {t9:.0}; 45° recipe 9 {m9:.0}");
    assert!(t7 < 0.0, "recipe 7 wets the west flank in the trades");
    assert!(t9 > 0.0, "recipe 9 wets the east flank in the trades");
    assert!(m9 < 0.0, "westerlies still wet the west flank at 45°");
    assert!(easterly(20_000) && easterly(-20_000) && easterly(70_000));
    assert!(!easterly(45_000) && !easterly(-35_000));
}

/// Review 3.2-17: recipe 7's continentality stops at 300 km (3 °C), so an
/// interior 800 km from the sea is as warm as one 300 km inland.
#[test]
fn recipe_9_continental_interiors_keep_cooling_past_300_km() {
    let (w, h) = (1_600, 5);
    let g = strip(w, h);
    let t = |c: &ContinentClimate, x: i32| {
        i32::from(c.temperature[usize::try_from(2 * w + x).unwrap()])
    };
    let (c7, c9) = (climate_recipe7(&g, MID), climate_recipe9(&g, MID));
    // 300 km from the western sea, and 795 km from the eastern rim.
    let (near, far) = (304, 804);
    let (d7, d9) = (t(&c7, near) - t(&c7, far), t(&c9, near) - t(&c9, far));
    eprintln!("300 km minus 795 km inland: recipe 7 {d7}, recipe 9 {d9} centi-°C");
    assert_eq!(d7, 0, "recipe 7 is capped");
    let expected = continentality_centi(795) - continentality_centi(300);
    assert_eq!(i64::from(d9), expected, "recipe 9 keeps cooling");
    assert_eq!(expected, 123);
    assert_eq!(continentality_centi(100), 100);
    assert_eq!(continentality_centi(5_000), 600);
}
