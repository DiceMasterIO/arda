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
