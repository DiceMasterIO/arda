//! Water balance, tree line and snow line from mean annual climate.

use super::latitude::cold_shift_centi;
use super::{smooth_q12, smoothstep, tan_deg, ONE};

/// Evaporation demand at or below 0 °C, millimetres a year.
pub const PET_BASE_MM: i32 = 300;
/// Extra evaporation demand per degree Celsius above freezing, mm a year.
pub const PET_MM_PER_C: i32 = 35;

/// Potential evaporation, millimetres a year: 300 mm plus 35 mm per °C
/// above freezing (the world's demand since recipe 4; refine used 45 mm per
/// °C until the shared module, review 3.4-9).
#[must_use]
pub fn pet_mm(temp_centi: i32) -> i32 {
    PET_BASE_MM + temp_centi.max(0) * PET_MM_PER_C / 100
}

/// Rainfall over evaporation demand, Q8 (`256` = rain meets demand),
/// clamped to `0..=768`.
#[must_use]
pub fn moisture_index_q8(rain_mm: i32, temp_centi: i32) -> i32 {
    (rain_mm * 256 / pet_mm(temp_centi)).clamp(0, 768)
}

/// Rainfall over evaporation demand as a ratio (`1.0` = rain meets demand):
/// the `f64` mirror of [`moisture_index_q8`] without its clamp.
#[must_use]
pub fn water_balance(rain_mm: f64, temp_c: f64) -> f64 {
    rain_mm / (f64::from(PET_BASE_MM) + f64::from(PET_MM_PER_C) * temp_c.max(0.0))
}

/// Mean annual temperature (centi-°C) where perennial snow starts at
/// mid-latitudes; the Atlas has painted it there since v0.1 (the Alpine
/// equilibrium line sits near -5 °C mean annual).
pub const SNOW_START_CENTI: i32 = -400;
/// Mean annual temperature (centi-°C) of full snow cover at mid-latitudes.
pub const SNOW_FULL_CENTI: i32 = -700;
/// How much warmer a slope facing the sun counts for snow, centi-°C.
pub const SNOW_SUN_CENTI: i32 = 150;
/// How much colder a gully or hollow counts for snow, centi-°C.
pub const SNOW_HOLLOW_CENTI: i32 = 250;
/// How much warmer a wind-scoured ridge counts for snow, centi-°C.
pub const SNOW_RIDGE_CENTI: i32 = 150;
/// Gradient (Q12, rise over run) over which steep faces shed snow: from
/// 0.7 (35°) to 1.3 (52.4°).
pub const SNOW_SHED_GRADIENT_Q12: (i64, i64) = (2_867, 5_325);
/// Share of the snow a face beyond the shed gradient loses, Q12 (3/4).
pub const SNOW_SHED_Q12: i64 = 3_072;

/// Mean annual temperature (centi-°C) where tree growth gives out and where
/// it is full, at mid-latitudes: the tree line of every layer from recipe 9
/// and of the refiner (review 3.4-9).
pub const TREE_LINE_CENTI: (i32, i32) = (-100, 400);
/// World ecology's linear canopy limit before recipe 9 (`fine_materials`
/// replays it byte for byte for recipes 4–8 and legacy terrain).
pub const TREE_LIMIT_V8_CENTI: (i32, i32) = (-400, 500);
/// The Atlas canopy ramp before recipe 9.
pub const ATLAS_CANOPY_V8_CENTI: (i32, i32) = (100, 400);

/// The snow line and tree line at one place, centi-°C ramps `(start,
/// full)`: snow ramps in from `snow.0` (none) to `snow.1` (full cover),
/// tree growth from `tree.0` (none) to `tree.1` (full).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lines {
    /// Perennial snow: none at `.0`, full at `.1` (colder).
    pub snow: (i32, i32),
    /// Tree growth: none at `.0`, full at `.1` (warmer).
    pub tree: (i32, i32),
}

impl Lines {
    /// The lines before recipe 9 (no seasonal shift; the Atlas's canopy
    /// ramp), so older worlds render byte for byte as they did.
    pub const LEGACY: Self = Self {
        snow: (SNOW_START_CENTI, SNOW_FULL_CENTI),
        tree: ATLAS_CANOPY_V8_CENTI,
    };

    /// The mid-latitude lines, no seasonal shift: the refiner's lines on
    /// worlds generated before recipe 9.
    pub const MID: Self = Self {
        snow: (SNOW_START_CENTI, SNOW_FULL_CENTI),
        tree: TREE_LINE_CENTI,
    };

    /// The lines at latitude `lat_mdeg`: the mid-latitude ramps moved colder
    /// by the seasonal shift ([`cold_shift_centi`]).
    #[must_use]
    pub fn at(lat_mdeg: i32) -> Self {
        let s = cold_shift_centi(lat_mdeg);
        Self {
            snow: (SNOW_START_CENTI + s, SNOW_FULL_CENTI + s),
            tree: (TREE_LINE_CENTI.0 + s, TREE_LINE_CENTI.1 + s),
        }
    }

    /// Perennial snow, Q12, at effective temperature `t_eff_centi` (after
    /// sun, hollow and ridge corrections) on ground of gradient
    /// `gradient_q12`: the Atlas rule.
    #[must_use]
    pub fn snow_q12(&self, t_eff_centi: i64, gradient_q12: i64) -> i64 {
        let shed = SNOW_SHED_Q12
            * smooth_q12(
                SNOW_SHED_GRADIENT_Q12.0,
                SNOW_SHED_GRADIENT_Q12.1,
                gradient_q12,
            )
            / ONE;
        smooth_q12(i64::from(self.snow.0), i64::from(self.snow.1), t_eff_centi) * (ONE - shed) / ONE
    }

    /// Tree growth, Q12, at `temp_centi`.
    #[must_use]
    pub fn growth_q12(&self, temp_centi: i64) -> i64 {
        smooth_q12(i64::from(self.tree.0), i64::from(self.tree.1), temp_centi)
    }

    /// Perennial snow `0..=1` at `temp_c` on ground of `gradient` (rise
    /// over run): the `f64` mirror of [`Lines::snow_q12`].
    #[must_use]
    pub fn snow_by_gradient(&self, temp_c: f64, gradient: f64) -> f64 {
        let shed = 0.75 * smoothstep(0.7, 1.3, gradient);
        smoothstep(
            f64::from(self.snow.0) / 100.0,
            f64::from(self.snow.1) / 100.0,
            temp_c,
        ) * (1.0 - shed)
    }

    /// Perennial snow `0..=1` at `temp_c` on a slope of `slope_deg`.
    #[must_use]
    pub fn snow(&self, temp_c: f64, slope_deg: f64) -> f64 {
        self.snow_by_gradient(temp_c, tan_deg(slope_deg))
    }

    /// Tree growth `0..=1` at `temp_c`: the `f64` mirror of
    /// [`Lines::growth_q12`].
    #[must_use]
    pub fn growth(&self, temp_c: f64) -> f64 {
        smoothstep(
            f64::from(self.tree.0) / 100.0,
            f64::from(self.tree.1) / 100.0,
            temp_c,
        )
    }

    /// How far above the tree line: `1 - growth`.
    #[must_use]
    pub fn alpine(&self, temp_c: f64) -> f64 {
        1.0 - self.growth(temp_c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaporation_demand_is_the_worlds() {
        assert_eq!(pet_mm(-500), 300);
        assert_eq!(pet_mm(1_000), 650);
        assert_eq!(moisture_index_q8(650, 1_000), 256);
        assert!((water_balance(650.0, 10.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn snow_starts_at_minus_four_at_mid_latitudes_and_colder_poleward() {
        let mid = Lines::at(45_000);
        assert_eq!(mid.snow, (-400, -700));
        assert_eq!(mid.tree, (-100, 400));
        assert_eq!(mid.snow_q12(-400, 0), 0);
        assert_eq!(mid.snow_q12(-700, 0), ONE);
        assert!(mid.snow(0.0, 10.0).abs() < 1e-12);
        let north = Lines::at(65_000);
        assert_eq!(north.snow, (-900, -1_200));
        assert_eq!(north.tree, (-600, -100));
        // Boreal forest grows at -5 °C at 65°, not at 45°.
        assert!(north.growth(-5.0) > 0.0 && mid.growth(-5.0) == 0.0);
    }

    #[test]
    fn steep_faces_shed_three_quarters_and_the_mirrors_agree() {
        let l = Lines::LEGACY;
        assert_eq!(l.snow_q12(-900, 6_000), ONE / 4);
        assert!((l.snow(-9.0, 60.0) - 0.25).abs() < 1e-12);
        for t in [-350_i32, -450, -550, -650, -750] {
            let q = l.snow_q12(i64::from(t), 0) as f64 / ONE as f64;
            assert!((q - l.snow(f64::from(t) / 100.0, 0.0)).abs() < 2e-3, "{t}");
            let g = l.growth_q12(i64::from(t + 700)) as f64 / ONE as f64;
            assert!((g - l.growth(f64::from(t + 700) / 100.0)).abs() < 2e-3);
        }
        assert!((l.alpine(5.0)).abs() < 1e-12);
    }
}
