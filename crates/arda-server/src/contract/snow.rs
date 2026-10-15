//! Perennial snow proxy (contract §snow). Arda does not simulate snowpack.
//!
//! The rule mirrors the Atlas formed renderer (`logic/04` §atlas-formed snow,
//! `arda-render/src/atlas/formed.rs`) so maps and tactical content agree:
//!
//! 1. Permanent-ice cover is fully snowed; open water (sea, lake) never is.
//! 2. Otherwise cover ramps in with smoothstep as the mean annual temperature
//!    falls from −4 °C to −7 °C (the Alpine equilibrium line sits near −5 °C).
//! 3. Steep ground sheds snow: cover is scaled by `1 − 0.75 · smoothstep`
//!    of the slope gradient from 0.7 (35°) to 1.3 (52.4°).
//! 4. Altitude enters through the standard 6.5 °C/km lapse rate: the peak
//!    fraction re-evaluates rule 2 at the highest fine-terrain point in the
//!    footprint, and the snowline is the lapse-adjusted −5.5 °C altitude.
//! 5. The lapse starts from the surface the saved temperature describes:
//!    sea cells store sea-surface temperature (`arda-gen` `area::temperature`
//!    applies marine lapse at sea level), so their snowline is measured from
//!    sea level, never from the seafloor (integration plan I26).

use super::SnowSample;
use arda_core::biome::{climate, Lines};
use arda_core::{Cover, TerrainKind};

/// Environmental lapse rate, °C per metre of ascent.
pub const LAPSE_C_PER_M: f64 = 0.0065;
/// Mean annual temperature where snow starts, °C (the shared snow line,
/// `arda_core::biome::climate::SNOW_START_CENTI`).
pub const SNOW_START_C: f64 = climate::SNOW_START_CENTI as f64 / 100.0;
/// Mean annual temperature of full snow cover, °C.
pub const SNOW_FULL_C: f64 = climate::SNOW_FULL_CENTI as f64 / 100.0;
/// Temperature reported as the snowline, °C (the ramp midpoint).
pub const SNOWLINE_C: f64 = (SNOW_START_C + SNOW_FULL_C) / 2.0;
/// Sea level, metres: the reference surface of sea cells' temperature.
pub const SEA_LEVEL_M: f64 = 0.0;

fn fraction_at(lines: Lines, temperature_c: f64, slope_deg: f64) -> f64 {
    let gradient = slope_deg.to_radians().tan();
    lines.snow_by_gradient(temperature_c, gradient)
}

/// Evaluates the proxy for a node at `height_m` with the saved climate,
/// under the mid-latitude snow line.
///
/// `peak_m` is the highest fine-terrain point of the footprint, when known.
#[must_use]
pub fn snow(
    cover: Cover,
    terrain: TerrainKind,
    temperature_c: f64,
    height_m: f64,
    slope_deg: f64,
    peak_m: Option<f64>,
) -> SnowSample {
    snow_under(
        Lines::LEGACY,
        (cover, terrain),
        temperature_c,
        height_m,
        slope_deg,
        peak_m,
    )
}

/// [`snow`] under the snow line `lines` (a recipe-9 world's line at the
/// cell's latitude, `arda_core::biome::Lines::at`); the reported snowline
/// height is the lapse-adjusted midpoint of that line.
#[must_use]
pub fn snow_under(
    lines: Lines,
    (cover, terrain): (Cover, TerrainKind),
    temperature_c: f64,
    height_m: f64,
    slope_deg: f64,
    peak_m: Option<f64>,
) -> SnowSample {
    let surface_m = if terrain == TerrainKind::Sea {
        SEA_LEVEL_M
    } else {
        height_m
    };
    let mid_c = if lines == Lines::LEGACY {
        SNOWLINE_C
    } else {
        f64::from(lines.snow.0 + lines.snow.1) / 200.0
    };
    let snowline_m = surface_m + (temperature_c - mid_c) / LAPSE_C_PER_M;
    let rule = |t: f64| match (cover, terrain) {
        (Cover::Ice, _) => 1.0,
        (_, TerrainKind::Sea | TerrainKind::Lake) => 0.0,
        _ => fraction_at(lines, t, slope_deg),
    };
    let fraction = rule(temperature_c);
    let peak_fraction = peak_m.map(|peak| {
        let rise = (peak - height_m).max(0.0);
        rule(temperature_c - rise * LAPSE_C_PER_M).max(fraction)
    });
    SnowSample {
        fraction,
        peak_fraction,
        perennial: cover == Cover::Ice || fraction >= 0.5,
        snowline_m,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recipe_9_high_latitude_line_snows_colder() {
        // At 65° the shared snow line starts at -12 °C: -9 °C is bare there.
        let north = Lines::at(65_000);
        let s = snow_under(
            north,
            (Cover::Grass, TerrainKind::Land),
            -9.0,
            300.0,
            2.0,
            None,
        );
        assert!(s.fraction.abs() < 1e-12 && !s.perennial);
        let mid = snow(Cover::Grass, TerrainKind::Land, -9.0, 300.0, 2.0, None);
        assert!((mid.fraction - 1.0).abs() < 1e-12);
    }

    #[test]
    fn warm_lowland_is_bare_and_cold_crest_is_snowed() {
        let warm = snow(Cover::Grass, TerrainKind::Land, 12.0, 200.0, 2.0, None);
        assert!(warm.fraction.abs() < 1e-12 && !warm.perennial);
        let cold = snow(Cover::Rock, TerrainKind::Land, -9.0, 3600.0, 10.0, None);
        assert!((cold.fraction - 1.0).abs() < 1e-12 && cold.perennial);
    }

    #[test]
    fn ramp_midpoint_is_half_and_steep_faces_shed_snow() {
        let mid = snow(Cover::Bare, TerrainKind::Land, -5.5, 3000.0, 0.0, None);
        assert!((mid.fraction - 0.5).abs() < 1e-12);
        let cliff = snow(Cover::Rock, TerrainKind::Land, -9.0, 3000.0, 60.0, None);
        assert!((cliff.fraction - 0.25).abs() < 1e-12);
    }

    #[test]
    fn ice_cover_is_always_snowed_and_open_water_never() {
        let ice = snow(Cover::Ice, TerrainKind::Land, 3.0, 100.0, 0.0, None);
        assert!((ice.fraction - 1.0).abs() < 1e-12 && ice.perennial);
        let sea = snow(Cover::Bare, TerrainKind::Sea, -20.0, -5.0, 0.0, None);
        assert!(sea.fraction.abs() < 1e-12);
    }

    #[test]
    fn altitude_lowers_peak_temperature_and_sets_the_snowline() {
        // -3.5 C at 2000 m: bare at the node, the 800 m higher peak is at -8.7 C.
        let s = snow(
            Cover::Grass,
            TerrainKind::Land,
            -3.5,
            2000.0,
            5.0,
            Some(2800.0),
        );
        assert!(s.fraction.abs() < 1e-12);
        assert_eq!(s.peak_fraction, Some(1.0));
        assert!((s.snowline_m - (2000.0 + 2.0 / LAPSE_C_PER_M)).abs() < 1e-9);
    }

    #[test]
    fn sea_snowline_is_measured_from_sea_level_not_the_seafloor() {
        // Regression (I26): a 3 km deep sea cell at 10 °C used to report its
        // snowline 3 km too low, below sea level.
        let deep = snow(Cover::Bare, TerrainKind::Sea, 10.0, -3000.0, 0.0, None);
        let shallow = snow(Cover::Bare, TerrainKind::Sea, 10.0, -5.0, 0.0, None);
        let expected = SEA_LEVEL_M + (10.0 - SNOWLINE_C) / LAPSE_C_PER_M;
        assert!(
            (deep.snowline_m - expected).abs() < 1e-9,
            "{}",
            deep.snowline_m
        );
        assert!((deep.snowline_m - shallow.snowline_m).abs() < 1e-12);
        assert!(deep.snowline_m > 0.0);
        // Land and lake cells keep their own surface.
        let land = snow(Cover::Grass, TerrainKind::Land, 10.0, 400.0, 0.0, None);
        assert!((land.snowline_m - (400.0 + (10.0 - SNOWLINE_C) / LAPSE_C_PER_M)).abs() < 1e-9);
    }
}
