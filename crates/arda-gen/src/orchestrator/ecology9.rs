//! Recipe-9 world ecology: the shared climate-to-biome rules
//! (`arda_core::biome`) at the world's latitude (logic/02 §ground material,
//! recipe 9; review §4 item 6).
//!
//! What changes against recipes 4–8 ([`super::fine_materials`]):
//!
//! - Tree growth follows the shared tree line, which moves colder poleward
//!   of 45°, so a high-latitude world grows boreal forest below -4 °C
//!   (review 3.2-8).
//! - Ice is the ground under the full snow line, at any height, not only
//!   above 3,000 m (review 3.2-8).
//! - Rock is the shared bare-rock exposure with vegetation as a co-equal
//!   input: the canopy the climate supports holds soil on steeper ground,
//!   and the same exposure thins the canopy, so the tactical refiner draws
//!   rock exactly where this classification does (review 3.4-2).
//! - Shade favours the slope facing away from the sun in either hemisphere,
//!   read from the continuous aspect (review 3.2-7).
//! - Soil patchiness comes from rotated, phase-offset noise, not
//!   axis-aligned lattices (review 3.2-11).
//! - Wetness is the smoothed, halo-read index the area composition stored
//!   (`area::fields9`, review 3.2-16); it is not recomputed here.

use crate::noise::rotated_noise;
use arda_core::biome::{aspect, climate, ground, Frame, ONE};
use arda_core::{Cell, Cover};

const SEED_BROAD: u64 = 0x6c41_1f7e_8bce_029d;
const SEED_LOCAL: u64 = 0xc193_4af2_119b_6e7d;

/// Derives moisture, canopy and cover of a land cell at absolute cell
/// `(gx, gy)` of a world placed by `frame`.
pub(super) fn derive(cell: &mut Cell, seed: u64, gx: i32, gy: i32, frame: Frame) {
    let lat = frame.latitude_mdeg(i64::from(gy));
    let lines = frame.lines(lat);
    let temp = i32::from(cell.temperature.raw());
    let mi = climate::moisture_index_q8(i32::from(cell.rainfall.raw()), temp);
    let broad = rotated_noise(seed ^ SEED_BROAD, i64::from(gx), i64::from(gy), 128, 0);
    let local = rotated_noise(seed ^ SEED_LOCAL, i64::from(gx), i64::from(gy), 40, 1);
    let soil = broad * 18 / 32_768 + local * 8 / 32_768;
    cell.moisture = u8::try_from((35 + mi * 120 / 256 + soil).clamp(1, 255)).unwrap_or(u8::MAX);

    let growth = lines.growth_q12(i64::from(temp));
    let tl = i32::try_from(growth * 255 / ONE).unwrap_or(0);
    let wl = i32::try_from(ground::water_limit_q12(mi) * 255 / ONE).unwrap_or(0);
    let potential = tl * wl / 255 * 210 / 255;
    let patch = (broad * 145 + local * 60) / 32_768;
    // Dry country keeps denser woods on the slope facing away from the sun.
    let pole = -aspect::sunward_q12(cell.aspect_deg, lat);
    let shade = if cell.slope_milli_deg >= 1_000 && mi < 256 {
        i32::try_from(pole * 22 / ONE).unwrap_or(0)
    } else {
        0
    };
    let canopy0 = (potential + patch * tl * wl / (255 * 255) + shade).clamp(0, 255);
    let exposure = ground::rock_q12(
        i64::from(cell.slope_milli_deg),
        i64::from(canopy0) * ONE / 255,
        growth,
    );
    let canopy = i32::try_from(i64::from(canopy0) * (ONE - exposure) / ONE).unwrap_or(0);
    cell.forest_density = u8::try_from(canopy.clamp(0, 255)).unwrap_or(u8::MAX);

    cell.cover = if temp <= lines.snow.1 && exposure < ground::ROCK_COVER_Q12 {
        Cover::Ice
    } else if exposure >= ground::ROCK_COVER_Q12 {
        Cover::Rock
    } else if cell.cover == Cover::Marsh && cell.moisture >= ground::MARSH_MOISTURE {
        Cover::Marsh
    } else if mi < ground::BARE_BELOW_Q8 {
        Cover::Bare
    } else if canopy >= ground::FOREST_CANOPY {
        Cover::Forest
    } else if mi < ground::SCRUB_BELOW_Q8 && cell.temperature.raw() > ground::SCRUB_ABOVE_CENTI {
        Cover::Scrub
    } else {
        Cover::Grass
    };
    if matches!(cell.cover, Cover::Ice | Cover::Rock) {
        cell.forest_density = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{HeightMm, RainfallMm, TempCentiC, TerrainKind};

    fn cell(temp_centi: i16, slope_milli: u16) -> Cell {
        Cell {
            terrain: TerrainKind::Land,
            height: HeightMm::new(400_000),
            slope_milli_deg: slope_milli,
            aspect_deg: 90,
            temperature: TempCentiC::new(temp_centi),
            rainfall: RainfallMm::new(900),
            wetness: 90,
            ..Cell::default()
        }
    }

    /// A frame whose row `gy` lies at `lat` degrees.
    fn at(lat: i16) -> Frame {
        Frame::new(lat, lat + 1, 2).with_seasons(true)
    }

    fn run(mut c: Cell, frame: Frame) -> Cell {
        derive(&mut c, 42, 100, 1, frame);
        c
    }

    #[test]
    fn boreal_forest_grows_below_minus_four_at_high_latitude() {
        // Review 3.2-8: no forest anywhere colder than -4 °C.
        let mid = run(cell(-500, 3_000), at(45));
        let north = run(cell(-500, 3_000), at(66));
        assert_ne!(mid.cover, Cover::Forest);
        assert_eq!(north.cover, Cover::Forest, "{north:?}");
        assert!(north.forest_density > mid.forest_density);
    }

    #[test]
    fn ice_lies_under_the_snow_line_at_any_height() {
        let mut low = cell(-800, 2_000);
        low.height = HeightMm::new(900_000);
        assert_eq!(run(low, at(45)).cover, Cover::Ice);
        // Not under the snow line: no ice, however high.
        let mut high = cell(-500, 2_000);
        high.height = HeightMm::new(3_500_000);
        assert_ne!(run(high, at(45)).cover, Cover::Ice);
    }

    #[test]
    fn a_wooded_slope_holds_its_forest_where_open_ground_is_rock() {
        // 40°: rock without a canopy, forest under a closed one (review 3.4-2).
        let wet = run(cell(1_000, 40_000), at(45));
        assert_eq!(wet.cover, Cover::Forest, "{wet:?}");
        let mut dry = cell(1_000, 40_000);
        dry.rainfall = RainfallMm::new(250);
        assert_eq!(run(dry, at(45)).cover, Cover::Rock);
        // Above the tree line frost bares open ground from about 30°.
        assert_eq!(run(cell(-300, 32_000), at(45)).cover, Cover::Rock);
    }

    #[test]
    fn the_shaded_slope_follows_the_hemisphere() {
        let mut c = cell(1_500, 12_000);
        c.rainfall = RainfallMm::new(520);
        let (mut north_face, mut south_face) = (c, c);
        north_face.aspect_deg = aspect::NORTH;
        south_face.aspect_deg = 180;
        let n = |f| run(north_face, f).forest_density;
        let s = |f| run(south_face, f).forest_density;
        assert!(n(at(45)) > s(at(45)));
        assert!(n(at(-46)) < s(at(-46)));
    }
}
