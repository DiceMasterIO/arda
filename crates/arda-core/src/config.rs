//! Generation configuration and its validation (`logic/01` preconditions).

use crate::coords::AreaCoord;
use crate::error::ConfigError;
use serde::{Deserialize, Serialize};

/// Ground span of one area tile, in kilometres.
const AREA_SIZE_KM: u32 = 51;

const MIN_AXIS_KM: u32 = 64;
const MAX_AXIS_KM: u32 = 4000;
const MIN_DENSITY: u16 = 1;
const MAX_DENSITY: u16 = 200;

/// Continent extent in kilometres (`mockup/01` `--size`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SizeKm {
    /// East-west extent.
    pub width: u32,
    /// North-south extent.
    pub height: u32,
}

impl SizeKm {
    /// Builds a continent extent.
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// The latitude belt the continent sits in (`logic/01` §Q6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LatitudeBand {
    /// Southern edge, degrees north of the equator.
    pub south_deg: i16,
    /// Northern edge, degrees north of the equator.
    pub north_deg: i16,
}

impl LatitudeBand {
    /// Builds a latitude belt.
    #[must_use]
    pub const fn new(south_deg: i16, north_deg: i16) -> Self {
        Self {
            south_deg,
            north_deg,
        }
    }
}

/// A validated generation request. Construct via [`GenerateConfig::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateConfig {
    size_km: SizeKm,
    latitude_band: LatitudeBand,
    mean_density_per_km2: u16,
}

impl GenerateConfig {
    /// The walking-skeleton continent: ~100x200 km, 8 tiles
    /// (`architecture-interview.md` §Q8).
    pub const MICRO: Self = Self {
        size_km: SizeKm::new(102, 204),
        latitude_band: LatitudeBand::new(35, 55),
        mean_density_per_km2: 15,
    };

    /// Validates and builds a configuration.
    ///
    /// # Errors
    /// Returns the offending field and its valid range (`mockup/01` States).
    pub fn new(
        size_km: SizeKm,
        latitude_band: LatitudeBand,
        mean_density_per_km2: u16,
    ) -> Result<Self, ConfigError> {
        if size_km.width < MIN_AXIS_KM
            || size_km.height < MIN_AXIS_KM
            || size_km.width > MAX_AXIS_KM
            || size_km.height > MAX_AXIS_KM
        {
            return Err(ConfigError::SizeOutOfRange {
                width: size_km.width,
                height: size_km.height,
                min: MIN_AXIS_KM,
                max: MAX_AXIS_KM,
            });
        }
        if latitude_band.south_deg >= latitude_band.north_deg
            || latitude_band.south_deg < -80
            || latitude_band.north_deg > 80
        {
            return Err(ConfigError::LatitudeOutOfRange {
                south: latitude_band.south_deg,
                north: latitude_band.north_deg,
            });
        }
        if !(MIN_DENSITY..=MAX_DENSITY).contains(&mean_density_per_km2) {
            return Err(ConfigError::DensityOutOfRange {
                value: mean_density_per_km2,
                min: MIN_DENSITY,
                max: MAX_DENSITY,
            });
        }
        Ok(Self {
            size_km,
            latitude_band,
            mean_density_per_km2,
        })
    }

    /// The continent extent.
    #[must_use]
    pub const fn size_km(self) -> SizeKm {
        self.size_km
    }

    /// The latitude belt.
    #[must_use]
    pub const fn latitude_band(self) -> LatitudeBand {
        self.latitude_band
    }

    /// Mean settlement density in people per square kilometre.
    #[must_use]
    pub const fn mean_density_per_km2(self) -> u16 {
        self.mean_density_per_km2
    }

    /// Area tiles across the continent.
    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub const fn areas_wide(self) -> i32 {
        (self.size_km.width / AREA_SIZE_KM) as i32
    }

    /// Area tiles down the continent.
    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub const fn areas_high(self) -> i32 {
        (self.size_km.height / AREA_SIZE_KM) as i32
    }

    /// Every tile index, row-major — the orchestrator's work list.
    pub fn area_coords(self) -> impl Iterator<Item = AreaCoord> {
        let wide = self.areas_wide();
        let high = self.areas_high();
        (0..high).flat_map(move |y| (0..wide).map(move |x| AreaCoord::new(x, y)))
    }
}

impl Default for GenerateConfig {
    /// The interviewed default continent: 500x1000 km, 35-55°N, 15 people/km2
    /// (`logic/01` preconditions).
    fn default() -> Self {
        Self {
            size_km: SizeKm::new(500, 1000),
            latitude_band: LatitudeBand::new(35, 55),
            mean_density_per_km2: 15,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_the_interviewed_continent() {
        let c = GenerateConfig::default();
        assert_eq!(c.size_km().width, 500);
        assert_eq!(c.size_km().height, 1000);
        assert_eq!(c.latitude_band().south_deg, 35);
        assert_eq!(c.latitude_band().north_deg, 55);
        assert_eq!(c.mean_density_per_km2(), 15);
    }

    #[test]
    fn micro_continent_has_eight_tiles() {
        // architecture-interview.md §Q8: skeleton is ~100x200 km, 8 tiles.
        let c = GenerateConfig::MICRO;
        assert_eq!(c.areas_wide(), 2);
        assert_eq!(c.areas_high(), 4);
        assert_eq!(c.area_coords().count(), 8);
    }

    #[test]
    fn area_coords_are_row_major_and_stable() {
        let coords: Vec<_> = GenerateConfig::MICRO.area_coords().collect();
        assert_eq!(coords[0], AreaCoord::new(0, 0));
        assert_eq!(coords[1], AreaCoord::new(1, 0));
        assert_eq!(coords[2], AreaCoord::new(0, 1));
        assert_eq!(coords[7], AreaCoord::new(1, 3));
    }

    #[test]
    fn rejects_size_outside_the_valid_range() {
        let err =
            GenerateConfig::new(SizeKm::new(10, 10), LatitudeBand::new(35, 55), 15).unwrap_err();
        assert!(matches!(err, ConfigError::SizeOutOfRange { .. }));
    }

    #[test]
    fn rejects_inverted_latitude_band() {
        let err =
            GenerateConfig::new(SizeKm::new(500, 1000), LatitudeBand::new(55, 35), 15).unwrap_err();
        assert!(matches!(err, ConfigError::LatitudeOutOfRange { .. }));
    }

    #[test]
    fn config_error_message_names_the_field_and_range() {
        // mockup/01 States: invalid config exits non-zero naming the field.
        let err =
            GenerateConfig::new(SizeKm::new(10, 10), LatitudeBand::new(35, 55), 15).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("size"), "message was: {msg}");
        assert!(msg.contains("64"), "message was: {msg}");
    }
}
