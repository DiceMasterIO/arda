//! Fine-world ecology derived from saved climate, water and relief.
//!
//! This runs after the shared physical water solve, so the stored material
//! fields describe the same terrain and drainage that Atlas later displays.
//! All variation is deterministic in absolute cell coordinates; area borders
//! do not introduce a new random seed or a rectangular colour boundary.

use crate::noise::value_noise;
use arda_core::{AreaCells, AreaCoord, Cell, CellCoord, Cover, TerrainKind, AREA_CELLS};

/// Populate material fields for the opt-in fine-terrain path.
pub(super) fn apply(cells: &mut AreaCells, seed: u64, area: AreaCoord) {
    let ox = area.x * i32::from(AREA_CELLS);
    let oy = area.y * i32::from(AREA_CELLS);
    for y in 0..AREA_CELLS {
        for x in 0..AREA_CELLS {
            let Some(at) = CellCoord::new(x, y) else {
                continue;
            };
            let mut cell = *cells.get(at);
            if cell.terrain == TerrainKind::Land {
                derive(&mut cell, seed, ox + i32::from(x), oy + i32::from(y));
                cells.set(at, cell);
            }
        }
    }
}

fn derive(cell: &mut Cell, seed: u64, gx: i32, gy: i32) {
    // TWI is logarithmic in contributing area and inverse slope. The previous
    // division by milli-degrees pinned almost every flat plain cell to 255.
    let area_log = i32::try_from(cell.drainage_area_cells.max(1).ilog2()).unwrap_or(31);
    let slope_log = i32::try_from(u32::from(cell.slope_milli_deg.max(50)).ilog2()).unwrap_or(16);
    let wetness = (28 + area_log * 15 + (10 - slope_log) * 16).clamp(0, 255);
    cell.wetness = u8::try_from(wetness).unwrap_or(u8::MAX);

    // A simple annual water balance: rainfall relative to temperature-driven
    // evaporation demand. A given rainfall is thus wetter in a cold valley
    // than on a warm plain, as the capstone geography model requires.
    let temp_centi_c = i32::from(cell.temperature.raw());
    let potential_evap_mm = 300 + temp_centi_c.max(0) * 35 / 100;
    let moisture_index_q8 =
        (i32::from(cell.rainfall.raw()) * 256 / potential_evap_mm).clamp(0, 768);
    // Fine plains can have nearly level beds. Their D8 contributing areas and
    // height-above-river ownership then form artificial straight polygons.
    // Keep those measurements in the saved cell, but do not paint them across
    // the vegetation map. Slowly varying soil patchiness is independent of
    // routing and remains continuous at area boundaries.
    let broad = value_noise(seed ^ 0x6c41_1f7e_8bce_029d, gx, gy, 128);
    let local = value_noise(seed ^ 0xc193_4af2_119b_6e7d, gx, gy, 40);
    let soil_variation = broad * 18 / 32_768 + local * 8 / 32_768;
    // Zero marks older cells without ecology. An arid recipe-4 land cell uses
    // one so Atlas still displays dry ground rather than its old green tint.
    cell.moisture =
        u8::try_from((35 + moisture_index_q8 * 120 / 256 + soil_variation).clamp(1, 255))
            .unwrap_or(u8::MAX);

    // Independent growth limits keep forests off cold summits, thin soils and
    // arid ground. Slow absolute-coordinate noise creates clearings and cores
    // large enough to remain legible in a world overview.
    let temperature_limit = ((i32::from(cell.temperature.raw()) + 400) * 255 / 900).clamp(0, 255);
    let water_limit = ((moisture_index_q8 - 77) * 255 / 179).clamp(0, 255);
    let slope_limit = ((44_000 - i32::from(cell.slope_milli_deg)) * 255 / 12_000).clamp(0, 255);
    let potential = temperature_limit * water_limit / 255 * slope_limit / 255 * 210 / 255;
    let patch = (broad * 145 + local * 60) / 32_768;
    let aspect = i32::from(cell.aspect_deg);
    let northness = if aspect <= 180 {
        90 - aspect
    } else {
        aspect - 270
    };
    let shade_bonus = if cell.slope_milli_deg >= 1_000 && moisture_index_q8 < 256 {
        northness * 22 / 90
    } else {
        0
    };
    let canopy = (potential + patch * temperature_limit * water_limit / (255 * 255) + shade_bonus)
        .clamp(0, 255);
    cell.forest_density = u8::try_from(canopy).unwrap_or(u8::MAX);

    let height_m = cell.height.raw() / 1_000;
    cell.cover = if height_m > 3_000 && cell.temperature.raw() < -300 {
        Cover::Ice
    } else if cell.slope_milli_deg > 38_000
        || (cell.slope_milli_deg > 28_000 && cell.temperature.raw() < -200)
    {
        Cover::Rock
    } else if cell.cover == Cover::Marsh && cell.moisture >= 125 {
        Cover::Marsh
    } else if moisture_index_q8 < 65 {
        Cover::Bare
    } else if canopy >= 145 {
        Cover::Forest
    } else if moisture_index_q8 < 165 && cell.temperature.raw() > -400 {
        Cover::Scrub
    } else {
        Cover::Grass
    };
    // Floodplain marsh keeps its canopy: riparian ground is among the greenest
    // on a plain, and marsh ownership steps with D8 height above the river.
    if matches!(cell.cover, Cover::Ice | Cover::Rock) {
        cell.forest_density = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{HeightMm, RainfallMm, TempCentiC};

    fn plain(rainfall: u16) -> Cell {
        Cell {
            terrain: TerrainKind::Land,
            height: HeightMm::new(250_000),
            slope_milli_deg: 120,
            temperature: TempCentiC::new(1_000),
            rainfall: RainfallMm::new(rainfall),
            drainage_area_cells: 4,
            height_above_river_dm: 200,
            ..Cell::default()
        }
    }

    #[test]
    fn fine_plain_wetness_has_headroom() {
        let mut c = plain(600);
        derive(&mut c, 42, 100, 200);
        assert!((80..220).contains(&c.wetness));
        assert!(c.moisture > 0);
    }

    #[test]
    fn rainfall_changes_plant_water_and_cover() {
        let mut dry = plain(260);
        let mut wet = plain(950);
        derive(&mut dry, 42, 100, 200);
        derive(&mut wet, 42, 100, 200);
        assert!(wet.moisture > dry.moisture);
        assert!(wet.forest_density > dry.forest_density);
    }

    #[test]
    fn routing_ownership_does_not_make_polygonal_ecology_on_flat_ground() {
        let mut headwater = plain(600);
        let mut trunk = headwater;
        trunk.drainage_area_cells = 65_536;
        trunk.height_above_river_dm = 0;
        derive(&mut headwater, 42, 100, 200);
        derive(&mut trunk, 42, 100, 200);
        assert_ne!(headwater.wetness, trunk.wetness);
        assert_eq!(headwater.moisture, trunk.moisture);
        assert_eq!(headwater.forest_density, trunk.forest_density);
        assert_eq!(headwater.cover, trunk.cover);
    }

    #[test]
    fn floodplain_marsh_keeps_the_surrounding_canopy() {
        // Marsh ownership follows per-cell D8 height above the river. Removing
        // canopy there painted pale stair-stepped strips along plain rivers.
        let mut dry = plain(600);
        let mut marsh = dry;
        marsh.cover = Cover::Marsh;
        derive(&mut dry, 42, 100, 200);
        derive(&mut marsh, 42, 100, 200);
        assert_eq!(marsh.cover, Cover::Marsh);
        assert!(marsh.forest_density > 0);
        assert_eq!(marsh.forest_density, dry.forest_density);
    }

    #[test]
    fn crossing_whole_degree_does_not_create_an_ecology_edge() {
        let mut below = plain(600);
        let mut above = below;
        below.temperature = TempCentiC::new(999);
        above.temperature = TempCentiC::new(1_000);
        derive(&mut below, 42, 100, 200);
        derive(&mut above, 42, 100, 200);
        assert!(below.moisture.abs_diff(above.moisture) <= 1);
        assert!(below.forest_density.abs_diff(above.forest_density) <= 1);
    }

    #[test]
    fn arid_plain_is_bare_before_scrub_is_considered() {
        let mut arid = plain(100);
        derive(&mut arid, 42, 100, 200);
        assert_eq!(arid.cover, Cover::Bare);
    }

    #[test]
    fn absolute_patch_coordinates_are_repeatable() {
        let mut first = plain(650);
        let mut second = first;
        derive(&mut first, 42, 511, 912);
        derive(&mut second, 42, 511, 912);
        assert_eq!(first, second);
    }

    #[test]
    fn moderate_climate_forms_forest_and_clearings_at_map_scale() {
        let mut forest = 0;
        let mut low = u8::MAX;
        let mut high = 0;
        for y in (0..512).step_by(4) {
            for x in (0..512).step_by(4) {
                let mut c = plain(600);
                derive(&mut c, 42, x, y);
                forest += usize::from(c.cover == Cover::Forest);
                low = low.min(c.forest_density);
                high = high.max(c.forest_density);
            }
        }
        assert!(forest > 1_638 && forest < 14_746, "forest cells: {forest}");
        assert!(high - low > 75, "canopy range: {low}..{high}");
    }
}
