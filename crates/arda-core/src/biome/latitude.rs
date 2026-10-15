//! Where a world lies on the globe, and what latitude does to its biomes:
//! which way the sun shines from, and how seasonal the climate is.
//!
//! The world stores only mean annual temperature. A tree line or a snow line
//! drawn on that alone puts boreal forest nowhere: the northern forest edge
//! lies near -5 to -10 °C mean annual, the Alpine tree line near 0 °C,
//! because summers at high latitude are long and warm for their annual mean.
//! [`cold_shift_centi`] moves both lines colder poleward of 45°, so a
//! high-latitude world grows taiga under its snow line (review 3.2-8).

use super::ONE;

/// The latitude band of a world and the number of 100 m cell rows across
/// it. Row 0 is the band's north edge (the continent's frame, logic/01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    north_mdeg: i32,
    south_mdeg: i32,
    rows: u32,
    seasonal: bool,
}

/// A northern mid-latitude frame (the default continent's 35–55°N), for
/// callers and tests without a world.
pub const NORTHERN: Frame = Frame {
    north_mdeg: 55_000,
    south_mdeg: 35_000,
    rows: 2,
    seasonal: false,
};

/// The latitude of a 45°N reference point, millidegrees: a northern
/// mid-latitude point where the seasonal shift is still zero.
pub const NORTH_MDEG: i32 = 45_000;

impl Frame {
    /// A band from `south_deg` to `north_deg` over `rows` cell rows, with
    /// no seasonal shift of the tree and snow lines.
    #[must_use]
    pub const fn new(south_deg: i16, north_deg: i16, rows: u32) -> Self {
        Self {
            north_mdeg: north_deg as i32 * 1_000,
            south_mdeg: south_deg as i32 * 1_000,
            rows,
            seasonal: false,
        }
    }

    /// The same frame with the seasonal shift of the tree and snow lines on
    /// or off (on for recipe-9 worlds, whose ecology was drawn with it).
    #[must_use]
    pub const fn with_seasons(mut self, on: bool) -> Self {
        self.seasonal = on;
        self
    }

    /// Whether the tree and snow lines shift with latitude.
    #[must_use]
    pub const fn seasonal(&self) -> bool {
        self.seasonal
    }

    /// The tree and snow lines at latitude `lat_mdeg`: shifted poleward of
    /// 45° when the frame is seasonal, the mid-latitude lines otherwise.
    #[must_use]
    pub fn lines(&self, lat_mdeg: i32) -> super::Lines {
        if self.seasonal {
            super::Lines::at(lat_mdeg)
        } else {
            super::Lines::MID
        }
    }

    /// Latitude of cell row `row` (clamped into the band), millidegrees.
    #[must_use]
    pub fn latitude_mdeg(&self, row: i64) -> i32 {
        let last = i64::from(self.rows.max(2) - 1);
        let span = i64::from(self.north_mdeg - self.south_mdeg);
        let lat = i64::from(self.north_mdeg) - span * row.clamp(0, last) / last;
        i32::try_from(lat).unwrap_or(self.north_mdeg)
    }

    /// Latitude at a fractional cell row, degrees (for tactical squares:
    /// `row = square_y / 64`).
    #[must_use]
    pub fn latitude_deg(&self, row: f64) -> f64 {
        let last = f64::from(self.rows.max(2) - 1);
        let span = f64::from(self.north_mdeg - self.south_mdeg) / 1_000.0;
        f64::from(self.north_mdeg) / 1_000.0 - span * row.clamp(0.0, last) / last
    }
}

/// Latitude (millidegrees, absolute) beyond which the seasonal shift starts.
pub const SEASONAL_FROM_MDEG: i32 = 45_000;
/// Colder tree and snow lines per degree beyond [`SEASONAL_FROM_MDEG`],
/// centi-degrees Celsius.
pub const SEASONAL_CENTI_PER_DEG: i32 = 40;
/// The largest seasonal shift, centi-degrees Celsius.
pub const SEASONAL_MAX_CENTI: i32 = 1_000;

/// How much colder than at mid-latitudes the tree and snow lines lie at
/// `lat_mdeg` (either hemisphere), centi-degrees Celsius, `<= 0`: none
/// within 45° of the equator, 4 °C at 55°, 8 °C at 65°, at most 10 °C.
#[must_use]
pub fn cold_shift_centi(lat_mdeg: i32) -> i32 {
    let beyond = (lat_mdeg.saturating_abs() - SEASONAL_FROM_MDEG).max(0);
    -(i32::try_from(i64::from(beyond) * i64::from(SEASONAL_CENTI_PER_DEG) / 1_000)
        .unwrap_or(SEASONAL_MAX_CENTI)
        .min(SEASONAL_MAX_CENTI))
}

/// Latitude (millidegrees) over which the sun's side turns from south to
/// north across the equator.
pub const TROPIC_FADE_MDEG: i32 = 10_000;

/// Which way the midday sun shines from, Q12: [`ONE`] poleward of 10°N
/// (from the south), `-ONE` poleward of 10°S (from the north), linear
/// between, so no consumer assumes the northern hemisphere.
#[must_use]
pub fn hemisphere_q12(lat_mdeg: i32) -> i64 {
    (i64::from(lat_mdeg) * ONE / i64::from(TROPIC_FADE_MDEG)).clamp(-ONE, ONE)
}

/// [`hemisphere_q12`] for a latitude in degrees, `-1..=1`.
#[must_use]
pub fn hemisphere(lat_deg: f64) -> f64 {
    (lat_deg * 1_000.0 / f64::from(TROPIC_FADE_MDEG)).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_run_from_the_north_edge_to_the_south_edge() {
        let f = Frame::new(35, 55, 2_040);
        assert_eq!(f.latitude_mdeg(0), 55_000);
        assert_eq!(f.latitude_mdeg(2_039), 35_000);
        assert_eq!(f.latitude_mdeg(5_000), 35_000);
        assert!((f.latitude_deg(1_019.5) - 45.0).abs() < 1e-9);
        let south = Frame::new(-50, -30, 100);
        assert_eq!(south.latitude_mdeg(99), -50_000);
    }

    #[test]
    fn the_seasonal_shift_starts_at_45_degrees_in_both_hemispheres() {
        assert_eq!(cold_shift_centi(30_000), 0);
        assert_eq!(cold_shift_centi(45_000), 0);
        assert_eq!(cold_shift_centi(55_000), -400);
        assert_eq!(cold_shift_centi(-65_000), -800);
        assert_eq!(cold_shift_centi(80_000), -1_000);
    }

    #[test]
    fn the_sun_shines_from_the_equator_side() {
        assert_eq!(hemisphere_q12(45_000), ONE);
        assert_eq!(hemisphere_q12(-45_000), -ONE);
        assert_eq!(hemisphere_q12(0), 0);
        assert_eq!(hemisphere_q12(5_000), ONE / 2);
        assert!((hemisphere(-5.0) + 0.5).abs() < 1e-12);
    }
}
