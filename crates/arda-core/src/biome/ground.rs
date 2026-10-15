//! Ground from climate and landform: vegetation potential, bare-rock
//! exposure, ice, marsh and the world's cover thresholds.
//!
//! Rock exposure is one field for the world map and the battle map (review
//! 3.4-2): bare rock breaks out where the slope is too steep for soil to
//! hold, and vegetation is a co-equal input. Roots hold soil on slopes that
//! would otherwise shed it, so the slope at which rock appears rises with
//! the canopy the climate supports; above the tree line frost shattering
//! lowers it. World ecology classifies a cell's cover from the exposure at
//! the cell's slope; the refiner draws rock squares from the exposure at
//! each square's slope with the same climate.

use super::{smooth_q12, smoothstep, ONE};

/// Slope (milli-degrees) where bare rock starts on open, frost-free ground
/// (half exposed at 35°, the refiner's crag ramp before the shared rule).
pub const ROCK_BARE_MILLI: i64 = 26_000;
/// Slope (milli-degrees) from the start of exposure to full bare rock.
pub const ROCK_SPAN_MILLI: i64 = 18_000;
/// How far a closed canopy raises the rock slope, milli-degrees (half
/// exposed at 45° under full forest).
pub const ROCK_VEG_HOLD_MILLI: i64 = 10_000;
/// How far frost shattering above the tree line lowers it, milli-degrees.
pub const ROCK_FROST_MILLI: i64 = 8_000;

/// Moisture index (Q8) under which no canopy grows.
pub const CANOPY_DRY_Q8: i32 = 77;
/// Moisture index span (Q8) over which canopy rises to full.
pub const CANOPY_WET_SPAN_Q8: i32 = 179;

/// The water side of the canopy the climate supports, Q12, from the moisture
/// index (world ecology's `water_limit`).
#[must_use]
pub fn water_limit_q12(moisture_index_q8: i32) -> i64 {
    (i64::from(moisture_index_q8 - CANOPY_DRY_Q8) * ONE / i64::from(CANOPY_WET_SPAN_Q8))
        .clamp(0, ONE)
}

/// The `f64` mirror of [`water_limit_q12`] from a water balance ratio.
#[must_use]
pub fn water_limit(balance: f64) -> f64 {
    ((balance * 256.0 - f64::from(CANOPY_DRY_Q8)) / f64::from(CANOPY_WET_SPAN_Q8)).clamp(0.0, 1.0)
}

/// Vegetation potential, Q12: the canopy warmth (`growth_q12`) and water
/// allow before slope and patchiness.
#[must_use]
pub fn veg_potential_q12(growth_q12: i64, moisture_index_q8: i32) -> i64 {
    growth_q12.clamp(0, ONE) * water_limit_q12(moisture_index_q8) / ONE
}

/// The `f64` mirror of [`veg_potential_q12`].
#[must_use]
pub fn veg_potential(growth: f64, balance: f64) -> f64 {
    growth.clamp(0.0, 1.0) * water_limit(balance)
}

/// Bare-rock exposure, Q12, of ground sloping `slope_milli` milli-degrees
/// under a canopy of `veg_q12` (the world's forest density) and tree growth
/// `growth_q12`.
#[must_use]
pub fn rock_q12(slope_milli: i64, veg_q12: i64, growth_q12: i64) -> i64 {
    let lo = ROCK_BARE_MILLI + ROCK_VEG_HOLD_MILLI * veg_q12.clamp(0, ONE) / ONE
        - ROCK_FROST_MILLI * (ONE - growth_q12.clamp(0, ONE)) / ONE;
    smooth_q12(lo, lo + ROCK_SPAN_MILLI, slope_milli)
}

/// The `f64` mirror of [`rock_q12`] for a slope in degrees.
#[must_use]
pub fn rock(slope_deg: f64, veg: f64, growth: f64) -> f64 {
    let ms = |m: i64| m as f64 / 1_000.0;
    let lo = ms(ROCK_BARE_MILLI) + ms(ROCK_VEG_HOLD_MILLI) * veg.clamp(0.0, 1.0)
        - ms(ROCK_FROST_MILLI) * (1.0 - growth.clamp(0.0, 1.0));
    smoothstep(lo, lo + ms(ROCK_SPAN_MILLI), slope_deg)
}

/// Exposure (Q12) at or above which a world cell's cover is rock (recipe 9):
/// open ground from 38°, as before, cold ground from 30°, a closed canopy
/// from 48°.
pub const ROCK_COVER_Q12: i64 = 3 * ONE / 4;
/// How much steeper than its cell's mean slope a cell's steepest squares
/// run, milli-degrees: a cell is craggy (strata, cliff bands) at the
/// exposure of this much more slope.
pub const CRAG_LEAD_MILLI: i64 = 8_000;
/// Slope (milli-degrees) above which a recipe ≤ 8 cell is rock.
pub const ROCK_V8_MILLI: u16 = 38_000;
/// Slope (milli-degrees) above which a cold recipe ≤ 8 cell is rock.
pub const COLD_ROCK_V8_MILLI: u16 = 28_000;
/// Temperature (centi-°C) under which the cold rock slope applies (≤ 8).
pub const COLD_ROCK_V8_CENTI: i16 = -200;
/// Height (m) a recipe ≤ 8 ice cell must exceed.
pub const ICE_V8_HEIGHT_M: i32 = 3_000;
/// Temperature (centi-°C) a recipe ≤ 8 ice cell must be under.
pub const ICE_V8_CENTI: i16 = -300;

/// Moisture index (Q8) under which land is bare.
pub const BARE_BELOW_Q8: i32 = 65;
/// Canopy (0–255) from which a cell is forest.
pub const FOREST_CANOPY: i32 = 145;
/// Moisture index (Q8) under which open land is scrub rather than grass.
pub const SCRUB_BELOW_Q8: i32 = 165;
/// Temperature (centi-°C) scrub needs.
pub const SCRUB_ABOVE_CENTI: i16 = -400;
/// Moisture (0–255) a floodplain marsh cell keeps its marsh at.
pub const MARSH_MOISTURE: u8 = 125;
/// Wetness (`0..=1`) where standing-water ground starts to become marsh in
/// the refiner, and where it is marsh outright.
pub const MARSH_WETNESS: (f64, f64) = (0.5, 0.7);
/// Water balance where ground starts to read arid, and where it is dry
/// steppe (the refiner's aridity).
pub const ARID_BALANCE: (f64, f64) = (0.75, 0.35);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vegetation_holds_soil_on_steeper_ground() {
        // Open, frost-free ground: half rock at 35°.
        assert_eq!(rock_q12(35_000, 0, ONE), ONE / 2);
        // A closed canopy holds soil to 45°.
        assert_eq!(rock_q12(35_000, ONE, ONE), 0);
        assert_eq!(rock_q12(45_000, ONE, ONE), ONE / 2);
        // Above the tree line frost bares it from 27°.
        assert_eq!(rock_q12(27_000, 0, 0), ONE / 2);
        assert!((rock(45.0, 1.0, 1.0) - 0.5).abs() < 1e-12);
        assert!((rock(27.0, 0.0, 0.0) - 0.5).abs() < 1e-12);
        // World rock cover: open ground from 38°, as before recipe 9.
        assert!(rock_q12(37_500, 0, ONE) < ROCK_COVER_Q12);
        assert!(rock_q12(38_500, 0, ONE) >= ROCK_COVER_Q12);
    }

    #[test]
    fn potential_needs_both_warmth_and_water() {
        assert_eq!(veg_potential_q12(ONE, 256), ONE);
        assert_eq!(veg_potential_q12(0, 768), 0);
        assert_eq!(veg_potential_q12(ONE, 60), 0);
        assert!((veg_potential(1.0, 1.0) - 1.0).abs() < 1e-12);
        assert!((water_limit(0.5) - water_limit_q12(128) as f64 / 4_096.0).abs() < 1e-3);
    }
}
