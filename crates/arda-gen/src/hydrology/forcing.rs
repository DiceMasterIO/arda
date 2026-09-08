//! Twelve-period forcing; area-water-terrain-realism spec requirements 5–7.
use super::budget::BudgetError;
use super::forcing_tables::{
    DAYLIGHT_SQ_Q24, PROFILE_LAT_MDEG, PROFILE_RAIN_Q32, PROFILE_T_ANOMALY_MICRO,
};
use arda_core::{RainfallMm, TempCentiC};
use std::cmp::Reverse;

/// Fixed non-leap climatological calendar.
pub const MONTH_DAYS: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
/// Fixed calendar month; array indices are always validated by construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Month {
    /// January.
    January,
    /// February.
    February,
    /// March.
    March,
    /// April.
    April,
    /// May.
    May,
    /// June.
    June,
    /// July.
    July,
    /// August.
    August,
    /// September.
    September,
    /// October.
    October,
    /// November.
    November,
    /// December.
    December,
}
impl Month {
    /// January-through-December canonical order.
    pub const ALL: [Self; 12] = [
        Self::January,
        Self::February,
        Self::March,
        Self::April,
        Self::May,
        Self::June,
        Self::July,
        Self::August,
        Self::September,
        Self::October,
        Self::November,
        Self::December,
    ];
    /// Zero-based calendar index.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

const PROFILE_Q: u32 = 1 << 24;
const DAY_Q: u128 = 1 << 24;

/// Latitude in millidegrees within the supported ±80° domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatitudeMilli(i32);
impl LatitudeMilli {
    /// Rejects unsupported astronomical latitude.
    pub fn new(raw: i32) -> Result<Self, BudgetError> {
        if !(-80_000..=80_000).contains(&raw) {
            return Err(BudgetError::InvalidForcing("latitude beyond 80 degrees"));
        }
        Ok(Self(raw))
    }
    /// Signed millidegrees.
    #[must_use]
    pub const fn raw(self) -> i32 {
        self.0
    }
}
/// One month's available climatological inputs; not a weather forecast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PeriodForcing {
    /// Total liquid-plus-solid precipitation depth.
    pub precip_um: u32,
    /// Mean regional air temperature.
    pub temperature: TempCentiC,
    /// Potential open-liquid-water evaporation before availability bounding.
    pub evaporation_um: u32,
}
/// A January-through-December climatological year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct YearForcing {
    /// Calendar-indexed records; all worker boundaries use the same phase.
    pub periods: [PeriodForcing; 12],
}

pub(crate) fn apportion<const N: usize>(
    numerators: [u128; N],
    denominator: u128,
    total: u32,
) -> Result<[u32; N], BudgetError> {
    if denominator == 0 {
        return Err(BudgetError::InvalidForcing("zero apportion denominator"));
    }
    let mut result = [0; N];
    let mut allocated = 0_u32;
    for (r, n) in result.iter_mut().zip(numerators) {
        *r =
            u32::try_from(n / denominator).map_err(|_| BudgetError::Overflow("apportion value"))?;
        allocated = allocated
            .checked_add(*r)
            .ok_or(BudgetError::Overflow("apportion sum"))?;
    }
    let remaining = total
        .checked_sub(allocated)
        .ok_or(BudgetError::InvalidForcing("apportion total below floors"))?;
    let remaining =
        usize::try_from(remaining).map_err(|_| BudgetError::Overflow("apportion count"))?;
    if remaining > N {
        return Err(BudgetError::InvalidForcing("apportion remainder count"));
    }
    let mut order: [usize; N] = std::array::from_fn(|i| i);
    order.sort_by_key(|&i| (Reverse(numerators[i] % denominator), i));
    for &i in order.iter().take(remaining) {
        result[i] = result[i]
            .checked_add(1)
            .ok_or(BudgetError::Overflow("apportion remainder"))?;
    }
    Ok(result)
}

fn chain_weights(lat: i32, chain: &[usize]) -> Result<[u32; 5], BudgetError> {
    let mut result = [0; 5];
    if lat <= PROFILE_LAT_MDEG[chain[0]] {
        result[chain[0]] = PROFILE_Q;
        return Ok(result);
    }
    for pair in chain.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if lat <= PROFILE_LAT_MDEG[b] {
            let span = i64::from(PROFILE_LAT_MDEG[b] - PROFILE_LAT_MDEG[a]);
            let delta = i64::from(lat - PROFILE_LAT_MDEG[a]);
            let upper = (delta * i64::from(PROFILE_Q) + span / 2) / span;
            // Both are nonnegative fractions of PROFILE_Q on this branch.
            let upper =
                u32::try_from(upper).map_err(|_| BudgetError::Overflow("profile weight"))?;
            result[a] = PROFILE_Q - upper;
            result[b] = upper;
            return Ok(result);
        }
    }
    result[chain[chain.len() - 1]] = PROFILE_Q;
    Ok(result)
}

fn profile_weights(lat: LatitudeMilli, distance: u16) -> Result<[u32; 5], BudgetError> {
    let a = lat.raw().abs();
    let coast = chain_weights(a, &[0, 1, 2])?;
    let inland = chain_weights(a, &[0, 1, 3, 4])?;
    let d = u128::from(distance.min(300));
    let nums =
        std::array::from_fn(|i| u128::from(coast[i]) * (300 - d) + u128::from(inland[i]) * d);
    apportion(nums, 300, PROFILE_Q)
}

/// Sum of squared normalized daylight hours for one calendar month, in Q24.
pub fn daylight_factor_q24(lat: LatitudeMilli, month: Month) -> Result<u32, BudgetError> {
    let month = month.index();
    let offset =
        u32::try_from(lat.raw() + 80_000).map_err(|_| BudgetError::InvalidForcing("latitude"))?;
    let lo = usize::try_from(offset / 100).map_err(|_| BudgetError::Overflow("daylength row"))?;
    let hi = (lo + 1).min(1600);
    let fraction = u64::from(offset % 100);
    let n = u64::from(DAYLIGHT_SQ_Q24[lo][month]) * (100 - fraction)
        + u64::from(DAYLIGHT_SQ_Q24[hi][month]) * fraction;
    u32::try_from(n / 100).map_err(|_| BudgetError::Overflow("daylength factor"))
}

/// USGS OFR2025-1021 equations 4–5, converted to micrometres/month.
pub fn hamon_um(temperature: TempCentiC, daylight_sq_q24: u32) -> Result<u32, BudgetError> {
    let t = i128::from(temperature.raw());
    if !(-10_000..=5000).contains(&t) || u128::from(daylight_sq_q24) > 124 * DAY_Q {
        return Err(BudgetError::InvalidForcing(
            "Hamon temperature/daylength domain",
        ));
    }
    if t <= 0 {
        return Ok(0);
    }
    // SVD in micrograms/m³. At 50°C the unrounded numerator is <8.1e15.
    let svd_n = 5_018_000_i128 * 100_000_000
        + 3231 * t * 100_000_000
        + 81_847_000 * t * t
        + 31_243 * t * t * t;
    let svd = u128::try_from(svd_n / 100_000_000)
        .map_err(|_| BudgetError::Overflow("saturated vapour density"))?;
    let n = 1397_u128
        .checked_mul(svd)
        .and_then(|n| n.checked_mul(u128::from(daylight_sq_q24)))
        .ok_or(BudgetError::Overflow("Hamon numerator"))?;
    u32::try_from(n / (10_000_000 * DAY_Q)).map_err(|_| BudgetError::Overflow("Hamon result"))
}

fn round_signed(n: i128, d: i128) -> i128 {
    if n >= 0 {
        (n + d / 2) / d
    } else {
        -((-n + d / 2) / d)
    }
}

/// Interpolates observed seasonal shapes while retaining the supplied annual rain.
pub fn profile_year(
    lat: LatitudeMilli,
    distance_to_ocean_km: u16,
    rainfall: RainfallMm,
    annual_t: TempCentiC,
) -> Result<YearForcing, BudgetError> {
    let weights = profile_weights(lat, distance_to_ocean_km)?;
    let shift = if lat.raw() < 0 { 6 } else { 0 };
    let mut pn = [0_u128; 12];
    let mut tn = [0_i128; 12];
    for m in 0..12 {
        let source = (m + shift) % 12;
        for i in 0..5 {
            pn[m] += u128::from(weights[i]) * u128::from(PROFILE_RAIN_Q32[i][source]);
            tn[m] += i128::from(weights[i]) * i128::from(PROFILE_T_ANOMALY_MICRO[i][source]);
        }
    }
    let annual_um = u32::from(rainfall.raw()) * 1000;
    for p in &mut pn {
        *p = p
            .checked_mul(u128::from(annual_um))
            .ok_or(BudgetError::Overflow("monthly rain numerator"))?;
    }
    let precip = apportion(pn, u128::from(PROFILE_Q) * (1_u128 << 32), annual_um)?;
    let mean_n: i128 = tn
        .iter()
        .zip(MONTH_DAYS)
        .map(|(t, d)| t * i128::from(d))
        .sum();
    let denom = 365 * i128::from(PROFILE_Q) * 10_000;
    let mut periods = [PeriodForcing::default(); 12];
    for m in 0..12 {
        let n = i128::from(annual_t.raw()) * denom + 365 * tn[m] - mean_n;
        let t = i16::try_from(round_signed(n, denom))
            .map_err(|_| BudgetError::InvalidForcing("seasonal temperature overflow"))?;
        let temperature = TempCentiC::new(t);
        periods[m] = PeriodForcing {
            precip_um: precip[m],
            temperature,
            evaporation_um: hamon_um(temperature, daylight_factor_q24(lat, Month::ALL[m])?)?,
        };
    }
    Ok(YearForcing { periods })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observed_monthly_splits_preserve_maximum_annual_precipitation() {
        for lat in [
            -80_000, -55_000, -42_000, -23_500, 0, 23_500, 42_000, 55_000, 80_000,
        ] {
            for d in [0, 150, 300] {
                let y = profile_year(
                    LatitudeMilli::new(lat).unwrap(),
                    d,
                    RainfallMm::new(u16::MAX),
                    TempCentiC::new(1200),
                )
                .unwrap();
                assert_eq!(
                    y.periods
                        .iter()
                        .map(|m| u64::from(m.precip_um))
                        .sum::<u64>(),
                    65_535_000
                );
                let weighted: i64 = y
                    .periods
                    .iter()
                    .zip(MONTH_DAYS)
                    .map(|(m, d)| i64::from(m.temperature.raw()) * i64::from(d))
                    .sum();
                assert!((weighted - 1200 * 365).abs() <= 183);
            }
        }
    }
    #[test]
    fn southern_profiles_have_the_opposite_season_without_yearly_rain_drift() {
        let get = |lat| {
            profile_year(
                LatitudeMilli::new(lat).unwrap(),
                300,
                RainfallMm::new(800),
                TempCentiC::new(1200),
            )
            .unwrap()
        };
        let n = get(45_383);
        let s = get(-45_383);
        assert!(n.periods[6].temperature.raw() > n.periods[0].temperature.raw());
        assert!(s.periods[0].temperature.raw() > s.periods[6].temperature.raw());
        assert_eq!(n.periods.iter().map(|m| m.precip_um).sum::<u32>(), 800_000);
        assert_eq!(s.periods.iter().map(|m| m.precip_um).sum::<u32>(), 800_000);
    }
    #[test]
    fn hamon_matches_a_dimensionally_independent_constant_daylength_fixture() {
        assert_eq!(
            hamon_um(TempCentiC::new(2000), 30 * (1 << 24)).unwrap(),
            72_308
        );
        assert_eq!(hamon_um(TempCentiC::new(0), 30 * (1 << 24)).unwrap(), 0);
        assert_eq!(
            hamon_um(TempCentiC::new(5000), 124 * (1 << 24)).unwrap(),
            1_397_750
        );
        assert!(hamon_um(TempCentiC::new(5001), 30 * (1 << 24)).is_err());
        assert!(LatitudeMilli::new(80_001).is_err());
    }
    #[test]
    fn interpolation_has_no_abrupt_regime_switch() {
        for lat in [1367, 23500, 39485, 42000, 45383, 51479, 55000, 62667] {
            let a = profile_year(
                LatitudeMilli::new(lat - 1).unwrap(),
                150,
                RainfallMm::new(800),
                TempCentiC::new(1800),
            )
            .unwrap();
            let b = profile_year(
                LatitudeMilli::new(lat + 1).unwrap(),
                150,
                RainfallMm::new(800),
                TempCentiC::new(1800),
            )
            .unwrap();
            for (a, b) in a.periods.iter().zip(b.periods) {
                assert!(a.precip_um.abs_diff(b.precip_um) <= 20);
                assert!(
                    (i32::from(a.temperature.raw()) - i32::from(b.temperature.raw())).abs() <= 1
                );
            }
        }
    }
}
