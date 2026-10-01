//! Recipe-5 formed shading of the fine window: the smoothing spline, the
//! multi-scale gradients and the anti-aliased formed footprint (logic/04
//! §atlas-formed).

use super::{
    arid_near, axis_positions, lake_surface_near, position, relief, shore_near, FineAtlas,
    SavedFields, SavedMaterial, BROAD_STEPS, CELL_UM, KM_UM, RING_STEPS,
};
use crate::atlas::{invalid, AxisKernel};
use crate::RenderError;

impl FineAtlas {
    /// Cubic B-spline height (mm) and its exact gradient (Q12 slope) at an
    /// absolute position (logic/04 §atlas-formed close zoom). Piecewise-
    /// bilinear heights give piecewise-constant slopes whose facets, and the
    /// field's own 39 m channel staircases, show at large exports; the
    /// smoothing spline is C2 and stays within the local node range.
    fn cubic(&self, x: i128, y: i128) -> Result<(i64, i64, i64), RenderError> {
        let p = self.clamp_source(x, y)?;
        let o = self.field.origin();
        let s = i128::from(self.field.spacing_um());
        let (w, h) = (
            i128::from(self.field.width()),
            i128::from(self.field.height()),
        );
        let dx = i128::from(p.x_um) - i128::from(o.x_um);
        let dy = i128::from(p.y_um) - i128::from(o.y_um);
        let (ix, iy) = (dx.div_euclid(s), dy.div_euclid(s));
        let tx = (dx.rem_euclid(s) << 16) / s;
        let ty = (dy.rem_euclid(s) << 16) / s;
        // Uniform cubic B-spline: a smoothing (not interpolating) patch whose
        // low-pass removes 39 m D8 staircases that only show below ~12 m/px.
        let weights = |t: i128| {
            let one = 65_536_i128;
            let u = one - t;
            let t2 = (t * t) >> 16;
            let t3 = (t2 * t) >> 16;
            let u2 = (u * u) >> 16;
            let u3 = (u2 * u) >> 16;
            (
                [
                    u3 / 6,
                    (3 * t3 - 6 * t2 + 4 * one) / 6,
                    (-3 * t3 + 3 * t2 + 3 * t + one) / 6,
                    t3 / 6,
                ],
                [
                    -u2 / 2,
                    (3 * t2 - 4 * t) / 2,
                    (-3 * t2 + 2 * t + one) / 2,
                    t2 / 2,
                ],
            )
        };
        let (wx, dwx) = weights(tx);
        let (wy, dwy) = weights(ty);
        let heights = self.field.heights();
        let at = |cx: i128, cy: i128| -> i128 {
            let cx = cx.clamp(0, w - 1);
            let cy = cy.clamp(0, h - 1);
            usize::try_from(cy * w + cx)
                .ok()
                .and_then(|i| heights.get(i))
                .map_or(0, |v| i128::from(v.raw()))
        };
        let (mut hv, mut gx, mut gy) = (0_i128, 0_i128, 0_i128);
        for j in 0..4 {
            let (mut row, mut drow) = (0_i128, 0_i128);
            for i in 0..4 {
                let z = at(ix - 1 + i as i128, iy - 1 + j as i128);
                row += wx[i] * z;
                drow += dwx[i] * z;
            }
            hv += wy[j] * row;
            gx += wy[j] * drow;
            gy += dwy[j] * row;
        }
        let q32 = 1_i128 << 32;
        // Slopes: d(height mm)/d(t) over the spacing in mm, Q12.
        let spacing_mm = s / 1_000;
        let slope = |v: i128| i64::try_from(v * 4_096 / q32 / spacing_mm.max(1)).unwrap_or(0);
        Ok((
            i64::try_from(hv.div_euclid(q32)).map_err(|_| invalid("cubic height overflow"))?,
            slope(gx),
            slope(gy),
        ))
    }

    /// Q12 slope from two heights `2 * half_um` apart.
    pub(super) fn slope_q12(high: i64, low: i64, half_um: i128) -> Result<i64, RenderError> {
        let dz = i128::from(high - low) * 4_096 * 1_000;
        i64::try_from(dz / (2 * half_um)).map_err(|_| invalid("formed slope overflow"))
    }

    pub(super) fn formed_color(
        &self,
        x: i128,
        y: i128,
        height: i32,
        (saved, temperature_centi): (SavedMaterial, i32),
        footprint_um: i128,
        refined: Option<relief::ReliefGeometry>,
    ) -> Result<[u8; 3], RenderError> {
        let sky = self
            .sky
            .as_ref()
            .ok_or_else(|| invalid("formed shading requires the 1 km context"))?;
        let s = i128::from(self.field.spacing_um());
        let h = |dx: i128, dy: i128| -> Result<i64, RenderError> {
            let p = self.clamp_source(x + dx, y + dy)?;
            Ok(i64::from(
                self.height(i128::from(p.x_um), i128::from(p.y_um))?,
            ))
        };
        let fine = |r: i128| -> Result<(i64, i64), RenderError> {
            Ok((
                Self::slope_q12(h(r * s, 0)?, h(-r * s, 0)?, r * s)?,
                Self::slope_q12(h(0, r * s)?, h(0, -r * s)?, r * s)?,
            ))
        };
        let broad = |r_km: i128| -> Result<(i64, i64), RenderError> {
            let r = r_km * KM_UM;
            Ok((
                Self::slope_q12(
                    sky.context_height_mm(x + r, y)?,
                    sky.context_height_mm(x - r, y)?,
                    r,
                )?,
                Self::slope_q12(
                    sky.context_height_mm(x, y + r)?,
                    sky.context_height_mm(x, y - r)?,
                    r,
                )?,
            ))
        };
        // Concavity at the drawn scale: never finer than the pixel footprint.
        let ring = (RING_STEPS * s).max(footprint_um);
        let ring_samples = [h(ring, 0)?, h(-ring, 0)?, h(0, ring)?, h(0, -ring)?];
        let ring_mean = ring_samples.iter().sum::<i64>() / 4;
        let ring_min = ring_samples.iter().copied().min().unwrap_or(0);
        // Close zoom (< 12 m/px) blends to the smoothing spline so the
        // 39 m grid never shows; crisp central differences elsewhere.
        let near = crate::atlas::formed::close_zoom_q12(footprint_um);
        let (g0x, g0y) = fine(1)?;
        // Close-zoom relief (logic/17 §render): a refined lattice
        // supplies the finest light and its own sub-39 m concavity; every
        // broader term still reads the stored field.
        let (gx, gy, height, concavity) = if let Some(r) = refined {
            (
                r.gradient_q12.0,
                r.gradient_q12.1,
                r.height_mm,
                ring_mean - i64::from(height) + r.concavity_mm,
            )
        } else if near > 0 {
            let (bh, bgx, bgy) = self.cubic(x, y)?;
            let mix = |a: i64, b: i64| a + (b - a) * near / 4_096;
            let height =
                i32::try_from(mix(i64::from(height), bh)).map_err(|_| invalid("spline height"))?;
            (
                mix(g0x, bgx),
                mix(g0y, bgy),
                height,
                ring_mean - i64::from(height),
            )
        } else {
            (g0x, g0y, height, ring_mean - i64::from(height))
        };
        Ok(crate::atlas::formed::shade(
            &crate::atlas::formed::FormedInputs {
                height_mm: height,
                position_m: (
                    i64::try_from(x / 1_000_000).unwrap_or(0),
                    i64::try_from(y / 1_000_000).unwrap_or(0),
                ),
                position_dm: (
                    i64::try_from(x / 100_000).unwrap_or(0),
                    i64::try_from(y / 100_000).unwrap_or(0),
                ),
                footprint_um,
                gradients: [(gx, gy), fine(BROAD_STEPS)?, broad(1)?, broad(3)?],
                concavity_mm: concavity,
                sky_q12: i64::from(sky.sample_q12(x, y)?),
                ring_min_mm: ring_min,
                temperature_centi,
                moisture: saved.moisture,
                wetness: saved.wetness,
                shore: saved.shore,
                v6: self.v6,
            },
        ))
    }

    /// Recipe-5 land and sea: each fine sub-sample is classified by the
    /// canonical field itself, so averaging over the pixel footprint yields
    /// an anti-aliased coastline on the 39 m contour (goal 31). Saved
    /// ownership still decides lakes, which never reach this path.
    pub(in crate::atlas) fn sample_formed(
        &self,
        x: AxisKernel,
        y: AxisKernel,
        saved: &SavedFields<'_>,
    ) -> Result<[u8; 3], RenderError> {
        let (lake_surface, shore) = (saved.lake_surface, saved.shore);
        let (xs, xe, nx) = axis_positions(x, self.origin_x, self.field.spacing_um())?;
        let (ys, ye, ny) = axis_positions(y, self.origin_y, self.field.spacing_um())?;
        // Physical pixel size, for detail that must fade before it aliases.
        let footprint = match x {
            AxisKernel::Linear { denominator, .. } => {
                CELL_UM * 1_024 / i128::from(denominator.max(1))
            }
            // A box kernel spans the whole pixel footprint.
            AxisKernel::Box { .. } => (xe - xs).max(CELL_UM),
        };
        if self.sky.is_none() {
            return Err(invalid("formed shading requires the 1 km context"));
        }
        let mut color_sum = [0_u128; 3];
        let mut count = 0_u128;
        for iy in 0..ny {
            let py = position(ys, ye - ys, iy, ny);
            for ix in 0..nx {
                let px = position(xs, xe - xs, ix, nx);
                let point = self.clamp_source(px, py)?;
                let (qx, qy) = (i128::from(point.x_um), i128::from(point.y_um));
                let height = self.height(qx, qy)?;
                let surface =
                    lake_surface_near(px, py, (self.origin_x, self.origin_y), lake_surface)?;
                let arid = arid_near(px, py, (self.origin_x, self.origin_y), saved)?;
                let color = if surface.is_some_and(|s| height < s) {
                    let depth = i64::from(surface.unwrap_or(height)) - i64::from(height);
                    match arid {
                        Some(a) if a.saline > 0 => crate::atlas::formed::saline_lake(depth),
                        _ => crate::atlas::formed::lake(depth, self.v6),
                    }
                } else if height > 0 || arid.is_some_and(|a| a.inland) {
                    let (land, temperature) = self.land_inputs(px, py, height, saved)?;
                    let c =
                        self.formed_color(qx, qy, height, (land, temperature), footprint, None)?;
                    match arid {
                        Some(a) => crate::atlas::formed::pan(c, a.crust, a.mudflat),
                        None => c,
                    }
                } else {
                    let shore_mix = shore_near(px, py, (self.origin_x, self.origin_y), shore)?;
                    self.sea_colour(qx, qy, height, &shore_mix)?
                };
                for (sum, c) in color_sum.iter_mut().zip(color) {
                    *sum += u128::from(c);
                }
                count += 1;
            }
        }
        if count == 0 {
            return Err(invalid("formed footprint is empty"));
        }
        let mut color = [0_u8; 3];
        for (channel, sum) in color.iter_mut().zip(color_sum) {
            *channel = u8::try_from((sum + count / 2) / count)
                .map_err(|_| invalid("formed averaged RGB outside channel range"))?;
        }
        Ok(color)
    }
}
