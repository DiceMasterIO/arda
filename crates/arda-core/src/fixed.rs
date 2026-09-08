//! Q-format fixed-point newtypes (`05-dependencies.md` §Q7).
//!
//! Sim-facing values are integers so the world is bit-identical across
//! platforms — `architecture-interview.md` §Q4 forbids float variance in
//! sim paths.

/// Elevation in millimetres above sea level; negative below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct HeightMm(i32);

impl HeightMm {
    /// Sea level.
    pub const SEA_LEVEL: Self = Self(0);

    /// Wraps a raw millimetre count.
    #[must_use]
    pub const fn new(mm: i32) -> Self {
        Self(mm)
    }

    /// The raw millimetre count.
    #[must_use]
    pub const fn raw(self) -> i32 {
        self.0
    }

    /// Truncated whole metres.
    #[must_use]
    pub const fn whole_metres(self) -> i32 {
        self.0 / 1000
    }
}

/// Temperature in hundredths of a degree Celsius.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TempCentiC(i16);

impl TempCentiC {
    /// Wraps a raw hundredth-degree count.
    #[must_use]
    pub const fn new(centi: i16) -> Self {
        Self(centi)
    }

    /// The raw hundredth-degree count.
    #[must_use]
    pub const fn raw(self) -> i16 {
        self.0
    }

    /// Truncated whole degrees Celsius.
    #[must_use]
    pub const fn whole_degrees(self) -> i16 {
        self.0 / 100
    }
}

/// Annual rainfall in millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RainfallMm(u16);

impl RainfallMm {
    /// Wraps a raw millimetre count.
    #[must_use]
    pub const fn new(mm: u16) -> Self {
        Self(mm)
    }

    /// The raw millimetre count.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// River discharge in thousandths of a cubic metre per second.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DischargeMilli(u64);

impl DischargeMilli {
    /// Wraps a raw thousandth-cumec count.
    #[must_use]
    pub const fn new(milli: u64) -> Self {
        Self(milli)
    }

    /// The raw thousandth-cumec count.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_round_trips_millimetres() {
        let h = HeightMm::new(1_524_000);
        assert_eq!(h.raw(), 1_524_000);
        assert_eq!(h.whole_metres(), 1524);
    }

    #[test]
    fn height_below_sea_level_is_negative() {
        assert_eq!(HeightMm::new(-2_000_000).whole_metres(), -2000);
    }

    #[test]
    fn temp_holds_the_habitable_range() {
        assert_eq!(TempCentiC::new(-4000).whole_degrees(), -40);
        assert_eq!(TempCentiC::new(5000).whole_degrees(), 50);
    }
}
