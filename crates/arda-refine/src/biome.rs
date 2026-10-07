//! Biome signals read from one cell's saved fields: how marshy, arid,
//! snowbound, alpine, craggy or coastal the cell is, each in `0..=1`.
//!
//! Worlds often leave `cover` coarse (a whole continent of `grass`), so the
//! ground and scatter rules lean on the climate and landform fields that
//! every world carries: wetness, rainfall against warmth, temperature,
//! slope and height. Each signal is a pure function of one cell, so cell
//! interpolation of it is the same from every block (seam-stable).

use crate::noise::smoothstep;
use arda::{Cell, Cover, TerrainKind};

/// Wetness (`0..=1`) where standing-water ground starts to become marsh,
/// and where it is marsh outright. A wetness of 1.0 is a wetland.
pub const MARSH_WETNESS: (f64, f64) = (0.8, 0.98);

/// Mean annual temperature (°C) at which ground starts to lie under snow,
/// and at which it is snowbound (the refiner's snow prior).
pub const SNOW_C: (f64, f64) = (1.5, -2.5);

/// Mean annual temperature (°C) where tree growth gives out: above it a
/// cell is below the tree line, below the second value fully alpine.
pub const TREELINE_C: (f64, f64) = (3.5, -1.0);

/// Land height (m) under which a land cell counts as lying at sea level,
/// and the height by which it no longer does.
pub const SEA_LEVEL_M: (f64, f64) = (0.02, 0.6);

fn land(c: &Cell) -> bool {
    c.terrain == TerrainKind::Land
}

fn temp_c(c: &Cell) -> f64 {
    f64::from(c.temperature.raw()) / 100.0
}

/// How much of the cell is marsh: marsh cover, or standing-water wetness
/// near saturation.
#[must_use]
pub fn marshy(c: &Cell) -> f64 {
    if !land(c) {
        return 0.0;
    }
    let wet = f64::from(c.wetness) / 255.0;
    let flat = 1.0 - smoothstep(4.0, 12.0, f64::from(c.slope_milli_deg) / 1000.0);
    let bog = smoothstep(MARSH_WETNESS.0, MARSH_WETNESS.1, wet) * flat;
    if c.cover == Cover::Marsh {
        1.0
    } else {
        bog
    }
}

/// Aridity from rainfall against warmth: `0` where rain covers what the
/// warmth evaporates, `1` on dry steppe. The water need is a simple
/// temperature proxy (300 mm plus 45 mm per °C above freezing); a world
/// without a rainfall layer (all zeros) reads as unknown, not desert.
#[must_use]
pub fn arid(c: &Cell) -> f64 {
    let rain = f64::from(c.rainfall.raw());
    if !land(c) || rain <= 0.0 {
        return 0.0;
    }
    let need = 300.0 + 45.0 * temp_c(c).max(0.0);
    smoothstep(0.75, 0.35, rain / need)
}

/// How snowbound the cell is, from its mean annual temperature.
#[must_use]
pub fn snowy(c: &Cell) -> f64 {
    smoothstep(SNOW_C.0, SNOW_C.1, temp_c(c))
}

/// How far above the tree line the cell lies.
#[must_use]
pub fn alpine(c: &Cell) -> f64 {
    if !land(c) {
        return 0.0;
    }
    smoothstep(TREELINE_C.0, TREELINE_C.1, temp_c(c))
}

/// How craggy the cell is: its mean slope from steep to mountain wall.
#[must_use]
pub fn craggy(c: &Cell) -> f64 {
    if !land(c) {
        return 0.0;
    }
    smoothstep(18.0, 34.0, f64::from(c.slope_milli_deg) / 1000.0)
}

/// Whether a land cell lies at sea level (the coast strip): `1` at 0 m,
/// fading out by [`SEA_LEVEL_M`]. Water cells answer `0`.
#[must_use]
pub fn coastal(c: &Cell) -> f64 {
    if !land(c) {
        return 0.0;
    }
    let h = f64::from(c.height.raw()) / 1000.0;
    1.0 - smoothstep(SEA_LEVEL_M.0, SEA_LEVEL_M.1, h)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{HeightMm, RainfallMm, TempCentiC};

    fn cell(f: impl FnOnce(&mut Cell)) -> Cell {
        let mut c = Cell {
            terrain: TerrainKind::Land,
            cover: Cover::Grass,
            temperature: TempCentiC::new(1000),
            rainfall: RainfallMm::new(900),
            height: HeightMm::new(50_000),
            ..Cell::default()
        };
        f(&mut c);
        c
    }

    #[test]
    fn saturated_ground_is_marsh_and_damp_ground_is_not() {
        assert!(marshy(&cell(|c| c.wetness = 255)) > 0.99);
        assert!(marshy(&cell(|c| c.wetness = 200)) < 0.01);
        assert!((marshy(&cell(|c| c.cover = Cover::Marsh)) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn steppe_rainfall_is_arid_and_a_missing_layer_is_not() {
        let steppe = cell(|c| {
            c.rainfall = RainfallMm::new(400);
            c.temperature = TempCentiC::new(1280);
        });
        assert!(arid(&steppe) > 0.75, "{}", arid(&steppe));
        let wet = cell(|c| c.rainfall = RainfallMm::new(1200));
        assert!(arid(&wet) < 1e-9);
        assert!(arid(&cell(|c| c.rainfall = RainfallMm::new(0))) < 1e-9);
        // Cold country evaporates little: 380 mm at -7 °C is not steppe.
        let tundra = cell(|c| {
            c.rainfall = RainfallMm::new(380);
            c.temperature = TempCentiC::new(-700);
        });
        assert!(arid(&tundra) < 1e-9);
    }

    #[test]
    fn peaks_are_snowbound_and_alpine() {
        let peak = cell(|c| c.temperature = TempCentiC::new(-686));
        assert!(snowy(&peak) > 0.99 && alpine(&peak) > 0.99);
        let valley = cell(|_| {});
        assert!(snowy(&valley) < 1e-9 && alpine(&valley) < 1e-9);
    }

    #[test]
    fn sea_level_land_is_coastal() {
        assert!(coastal(&cell(|c| c.height = HeightMm::new(1))) > 0.99);
        assert!(coastal(&cell(|_| {})) < 1e-9);
        assert!(coastal(&cell(|c| c.terrain = TerrainKind::Sea)) < 1e-9);
    }
}
