//! Immutable area-sized climate windows; shared coordinates determine all samples.
use super::budget::BudgetError;
use super::forcing::{
    apportion, daylight_factor_q24, hamon_um, profile_year, LatitudeMilli, Month, PeriodForcing,
    YearForcing,
};
use crate::continent::{climate::ContinentClimate, ContinentGrid};
use arda_core::hydrology::HydrologyDomain;
use arda_core::{AreaCoord, GlobalCell, KmCoord, LatitudeBand, RainfallMm, TempCentiC};

/// An area's coarse climate halo, independent of final neighbor-area files.
#[derive(Debug, Clone)]
pub struct ForcingWindow {
    origin: KmCoord,
    width: u16,
    height: u16,
    world_width: u32,
    world_height: u32,
    domain: HydrologyDomain,
    band: LatitudeBand,
    nodes: Vec<YearForcing>,
}
fn bad(s: &'static str) -> BudgetError {
    BudgetError::InvalidForcing(s)
}
fn latitude(band: LatitudeBand, row_fine: u32, height: u32) -> Result<LatitudeMilli, BudgetError> {
    let extent = 10_u32
        .checked_mul(height.saturating_sub(1))
        .ok_or(bad("latitude extent"))?;
    let n = i64::from(band.north_deg) * 1000;
    let span = i64::from(band.north_deg - band.south_deg) * 1000;
    let value = n - span * i64::from(row_fine.min(extent)) / i64::from(extent.max(1));
    LatitudeMilli::new(i32::try_from(value).map_err(|_| bad("latitude conversion"))?)
}
impl ForcingWindow {
    /// Precomputes only this area's coarse halo; caller retains it during preparation.
    pub fn for_area(
        grid: &ContinentGrid,
        climate: &ContinentClimate,
        band: LatitudeBand,
        domain: HydrologyDomain,
        area: AreaCoord,
    ) -> Result<Self, BudgetError> {
        let w = u32::try_from(grid.width()).map_err(|_| bad("grid width"))?;
        let h = u32::try_from(grid.height()).map_err(|_| bad("grid height"))?;
        if w == 0 || h == 0 || w > 4000 || h > 4000 {
            return Err(bad("grid dimensions"));
        }
        let max_x = (w * 10).max((w / 51).max(1) * 512);
        let max_y = (h * 10).max((h / 51).max(1) * 512);
        if domain.width_cells < w * 10
            || domain.height_cells < h * 10
            || domain.width_cells > max_x
            || domain.height_cells > max_y
        {
            return Err(bad("forcing domain bounds"));
        }
        let count = usize::try_from(w * h).map_err(|_| bad("grid length"))?;
        if climate.rainfall.len() != count
            || climate.temperature.len() != count
            || climate.ocean_distance_km.len() != count
        {
            return Err(bad("climate grid shape"));
        }
        let fx = u32::try_from(area.x)
            .map_err(|_| bad("negative area column"))?
            .checked_mul(512)
            .ok_or(bad("area column overflow"))?;
        let fy = u32::try_from(area.y)
            .map_err(|_| bad("negative area row"))?
            .checked_mul(512)
            .ok_or(bad("area row overflow"))?;
        if fx >= domain.width_cells || fy >= domain.height_cells {
            return Err(bad("area outside forcing domain"));
        }
        let x0 = (fx / 10).min(w - 1);
        let y0 = (fy / 10).min(h - 1);
        let x1 = ((fx + 512).min(w * 10 - 1) / 10 + 1).min(w - 1);
        let y1 = ((fy + 512).min(h * 10 - 1) / 10 + 1).min(h - 1);
        let width = u16::try_from(x1 - x0 + 1).map_err(|_| bad("forcing window width"))?;
        let height = u16::try_from(y1 - y0 + 1).map_err(|_| bad("forcing window height"))?;
        if width > 55 || height > 55 {
            return Err(bad("forcing window budget"));
        }
        let mut nodes = Vec::with_capacity(usize::from(width) * usize::from(height));
        for y in y0..=y1 {
            let lat = latitude(band, y * 10, h)?;
            for x in x0..=x1 {
                let i = usize::try_from(y * w + x).map_err(|_| bad("forcing grid index"))?;
                nodes.push(profile_year(
                    lat,
                    climate.ocean_distance_km[i],
                    RainfallMm::new(climate.rainfall[i]),
                    TempCentiC::new(climate.temperature[i]),
                )?);
            }
        }
        Ok(Self {
            origin: KmCoord::new(
                u16::try_from(x0).map_err(|_| bad("origin x"))?,
                u16::try_from(y0).map_err(|_| bad("origin y"))?,
            ),
            width,
            height,
            world_width: w,
            world_height: h,
            domain,
            band,
            nodes,
        })
    }
    fn node(&self, x: u32, y: u32) -> Result<&YearForcing, BudgetError> {
        let dx = x
            .checked_sub(u32::from(self.origin.x))
            .ok_or(bad("sample west of halo"))?;
        let dy = y
            .checked_sub(u32::from(self.origin.y))
            .ok_or(bad("sample north of halo"))?;
        if dx >= u32::from(self.width) || dy >= u32::from(self.height) {
            return Err(bad("sample outside forcing halo"));
        }
        let i =
            usize::try_from(dy * u32::from(self.width) + dx).map_err(|_| bad("sample index"))?;
        self.nodes.get(i).ok_or(bad("forcing window storage shape"))
    }
    /// Adds monthly anomalies to the canonical prepared annual temperature.
    /// The caller already removed the coarse altitude lapse and applied the
    /// final physical-height lapse exactly once; this method never adds it again.
    pub fn sample_with_annual_temperature(
        &self,
        at: GlobalCell,
        annual: TempCentiC,
    ) -> Result<YearForcing, BudgetError> {
        let mut year = self.sample(at)?;
        let sum: i32 = year
            .periods
            .iter()
            .map(|p| i32::from(p.temperature.raw()))
            .sum();
        let mean = sum / 12;
        let residual = mean * 12 - sum;
        let lat = latitude(self.band, at.y, self.world_height)?;
        for (m, p) in year.periods.iter_mut().enumerate() {
            let adjustment = if m < usize::try_from(residual.unsigned_abs())
                .map_err(|_| bad("temperature residual"))?
            {
                residual.signum()
            } else {
                0
            };
            let t = i32::from(p.temperature.raw()) - mean + i32::from(annual.raw()) + adjustment;
            let temperature =
                TempCentiC::new(i16::try_from(t).map_err(|_| bad("physical monthly temperature"))?);
            p.temperature = temperature;
            p.evaporation_um = hamon_um(temperature, daylight_factor_q24(lat, Month::ALL[m])?)?;
        }
        Ok(year)
    }
    /// Samples the same absolute 100 m point identically from overlapping windows.
    pub fn sample(&self, at: GlobalCell) -> Result<YearForcing, BudgetError> {
        if at.x >= self.domain.width_cells || at.y >= self.domain.height_cells {
            return Err(bad("sample beyond world"));
        }
        let x = at.x.min(self.world_width * 10 - 1);
        let y = at.y.min(self.world_height * 10 - 1);
        let x0 = x / 10;
        let y0 = y / 10;
        let x1 = (x0 + 1).min(self.world_width - 1);
        let y1 = (y0 + 1).min(self.world_height - 1);
        let corners = [
            self.node(x0, y0)?,
            self.node(x1, y0)?,
            self.node(x0, y1)?,
            self.node(x1, y1)?,
        ];
        let (dx, dy) = (x % 10, y % 10);
        let weights = [
            (10 - dx) * (10 - dy),
            dx * (10 - dy),
            (10 - dx) * dy,
            dx * dy,
        ];
        let mut pn = [0_u128; 12];
        let mut tn = [0_i128; 12];
        for (corner, weight) in corners.into_iter().zip(weights) {
            for m in 0..12 {
                pn[m] += u128::from(corner.periods[m].precip_um) * u128::from(weight);
                tn[m] += i128::from(corner.periods[m].temperature.raw()) * i128::from(weight);
            }
        }
        let annual = u32::try_from((pn.iter().sum::<u128>() + 50) / 100)
            .map_err(|_| bad("interpolated annual rain"))?;
        let rain = apportion(pn, 100, annual)?;
        let lat = latitude(self.band, at.y, self.world_height)?;
        let mut periods = [PeriodForcing::default(); 12];
        for m in 0..12 {
            let t = if tn[m] >= 0 {
                (tn[m] + 50) / 100
            } else {
                -((-tn[m] + 50) / 100)
            };
            let temperature =
                TempCentiC::new(i16::try_from(t).map_err(|_| bad("interpolated temperature"))?);
            periods[m] = PeriodForcing {
                precip_um: rain[m],
                temperature,
                evaporation_um: hamon_um(temperature, daylight_factor_q24(lat, Month::ALL[m])?)?,
            };
        }
        Ok(YearForcing { periods })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn window(origin_x: u16, width: u16) -> ForcingWindow {
        let mut nodes = Vec::new();
        for y in 0..2_u32 {
            for x in u32::from(origin_x)..u32::from(origin_x + width) {
                let mut year = YearForcing::default();
                year.periods[0].precip_um = 600_000 + 200_000 * x + 100_000 * y;
                nodes.push(year);
            }
        }
        ForcingWindow {
            origin: KmCoord::new(origin_x, 0),
            width,
            height: 2,
            world_width: 3,
            world_height: 2,
            domain: HydrologyDomain {
                width_cells: 512,
                height_cells: 512,
                exported_areas_wide: 1,
                exported_areas_high: 1,
            },
            band: LatitudeBand::new(35, 55),
            nodes,
        }
    }
    #[test]
    fn physical_annual_temperature_changes_evaporation_without_rain_drift() {
        let w = window(0, 3);
        let at = GlobalCell { x: 12, y: 5 };
        let cold = w
            .sample_with_annual_temperature(at, TempCentiC::new(-650))
            .unwrap();
        let warm = w
            .sample_with_annual_temperature(at, TempCentiC::new(650))
            .unwrap();
        assert_eq!(
            cold.periods
                .iter()
                .map(|p| i32::from(p.temperature.raw()))
                .sum::<i32>(),
            -650 * 12
        );
        assert_eq!(
            warm.periods
                .iter()
                .map(|p| i32::from(p.temperature.raw()))
                .sum::<i32>(),
            650 * 12
        );
        assert_eq!(
            cold.periods.map(|p| p.precip_um),
            warm.periods.map(|p| p.precip_um)
        );
        assert!(
            warm.periods.iter().map(|p| p.evaporation_um).sum::<u32>()
                > cold.periods.iter().map(|p| p.evaporation_um).sum::<u32>()
        );
    }
    #[test]
    fn affine_annual_rain_survives_every_fine_offset() {
        let w = window(0, 3);
        for y in 0..10 {
            for x in 0..20 {
                let year = w.sample(GlobalCell { x, y }).unwrap();
                let sum = year.periods.iter().map(|p| p.precip_um).sum::<u32>();
                assert_eq!(sum, 600_000 + 20_000 * x + 10_000 * y);
            }
        }
    }
    #[test]
    fn overlapping_worker_windows_return_identical_global_forcing() {
        let a = window(0, 3);
        let b = window(1, 2);
        let p = GlobalCell { x: 12, y: 5 };
        assert_eq!(a.sample(p).unwrap(), b.sample(p).unwrap());
        assert!(b.sample(GlobalCell { x: 0, y: 0 }).is_err());
    }
    #[test]
    fn exported_overshoot_uses_the_coarse_rim_without_fabricating_zero_rain() {
        let w = window(0, 3);
        assert_eq!(
            w.sample(GlobalCell { x: 29, y: 19 }).unwrap(),
            w.sample(GlobalCell { x: 511, y: 511 }).unwrap()
        );
        assert!(w.sample(GlobalCell { x: 512, y: 0 }).is_err());
    }
}
