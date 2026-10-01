//! Broad horizon exposure: a globally aligned 1 km sky-light field and its
//! 1 km height context, shared by local slope lighting and the recipe-5
//! broad gradients (logic/04 §atlas-formed).

use arda_core::{AreaCoord, TerrainField, TerrainPoint};

use super::{AREA_UM, CELL_UM, KM_UM};
use crate::atlas::invalid;
use crate::RenderError;

const SKY_RADIUS_KM: i64 = 8;
const SKY_RADII_KM: [i64; 4] = [1, 2, 4, 8];
const SKY_DIRECTIONS: [(i64, i64, i64); 8] = [
    (1, 0, 65_536),
    (-1, 0, 65_536),
    (0, 1, 65_536),
    (0, -1, 65_536),
    (1, 1, 92_682),
    (-1, 1, 92_682),
    (1, -1, 92_682),
    (-1, -1, 92_682),
];
const LIGHT_ONE_Q12: i128 = 4_096;

const MAX_SKY_DARK_Q12: i128 = 737; // 18% ceiling

/// A globally aligned 1 km field of broad horizon exposure. Its source
/// heights come from the same immutable fine terrain as local slope lighting.
#[derive(Debug)]
pub(super) struct SkyShade {
    first_x_km: i64,
    first_y_km: i64,
    width: usize,
    height: usize,
    pub(super) light_q12: Vec<u16>,
    // The 1 km horizon context itself, for broad recipe-5 gradients.
    context_x_km: i64,
    context_y_km: i64,
    context_width: usize,
    context_height: usize,
    context_mm: Vec<i32>,
}

fn sky_visible_axis(area: i32) -> Result<(i64, i64), RenderError> {
    let origin = i128::from(area) * AREA_UM;
    let low = (origin - CELL_UM / 2).div_euclid(KM_UM);
    let last = origin + AREA_UM - CELL_UM / 2;
    let high = -(-last).div_euclid(KM_UM);
    Ok((
        i64::try_from(low).map_err(|_| invalid("sky x/y node underflow"))?,
        i64::try_from(high).map_err(|_| invalid("sky x/y node overflow"))?,
    ))
}

impl SkyShade {
    pub(super) fn new(area: AreaCoord, context: &TerrainField) -> Result<Self, RenderError> {
        if i128::from(context.spacing_um()) != KM_UM
            || i128::from(context.origin().x_um).rem_euclid(KM_UM) != 0
            || i128::from(context.origin().y_um).rem_euclid(KM_UM) != 0
        {
            return Err(invalid("sky context is not on the global 1 km lattice"));
        }
        let (x0, x1) = sky_visible_axis(area.x)?;
        let (y0, y1) = sky_visible_axis(area.y)?;
        let (hx0, hx1) = (x0 - SKY_RADIUS_KM, x1 + SKY_RADIUS_KM);
        let (hy0, hy1) = (y0 - SKY_RADIUS_KM, y1 + SKY_RADIUS_KM);
        let hw = usize::try_from(hx1 - hx0 + 1).map_err(|_| invalid("sky width overflow"))?;
        let hh = usize::try_from(hy1 - hy0 + 1).map_err(|_| invalid("sky height overflow"))?;
        if hw > 72 || hh > 72 {
            return Err(invalid("sky context exceeds the per-area bound"));
        }
        let count = hw
            .checked_mul(hh)
            .ok_or_else(|| invalid("sky context count overflow"))?;
        let mut heights = Vec::new();
        heights
            .try_reserve_exact(count)
            .map_err(|_| invalid("sky context allocation failed"))?;
        for y in hy0..=hy1 {
            for x in hx0..=hx1 {
                let point = TerrainPoint {
                    x_um: i64::try_from(i128::from(x) * KM_UM)
                        .map_err(|_| invalid("sky context x overflow"))?,
                    y_um: i64::try_from(i128::from(y) * KM_UM)
                        .map_err(|_| invalid("sky context y overflow"))?,
                };
                heights.push(
                    context
                        .sample(point)
                        .ok_or_else(|| invalid("sky context omits a horizon node"))?
                        .raw(),
                );
                // Retained for recipe-5 broad gradients.
            }
        }
        let w = usize::try_from(x1 - x0 + 1).map_err(|_| invalid("sky width overflow"))?;
        let h = usize::try_from(y1 - y0 + 1).map_err(|_| invalid("sky height overflow"))?;
        let mut light_q12 = Vec::new();
        light_q12
            .try_reserve_exact(w * h)
            .map_err(|_| invalid("sky light allocation failed"))?;
        let at = |x: i64, y: i64| -> Result<i64, RenderError> {
            let ix = usize::try_from(x - hx0).map_err(|_| invalid("sky node west of context"))?;
            let iy = usize::try_from(y - hy0).map_err(|_| invalid("sky node north of context"))?;
            if ix >= hw || iy >= hh {
                return Err(invalid("sky node outside context"));
            }
            heights
                .get(iy * hw + ix)
                .copied()
                .map(i64::from)
                .ok_or_else(|| invalid("sky node missing from context"))
        };
        for y in y0..=y1 {
            for x in x0..=x1 {
                let center = at(x, y)?;
                let mut sum = 0_i128;
                for (dx, dy, length_q16) in SKY_DIRECTIONS {
                    let mut horizon_q16 = 0_i128;
                    for radius in SKY_RADII_KM {
                        let rise = (at(x + dx * radius, y + dy * radius)? - center).max(0);
                        let slope_q16 = i128::from(rise) * 65_536 * 65_536
                            / (i128::from(radius) * 1_000_000 * i128::from(length_q16));
                        horizon_q16 = horizon_q16.max(slope_q16);
                    }
                    sum += horizon_q16;
                }
                let mean_q16 = (sum + 4) / 8;
                // Eighteen percent max darkening at 0.25 mean horizon slope.
                // A broad, symmetric exposure term avoids hard cast shadows
                // and does not amplify the 39 m grooves used by local light.
                let dark_q12 = ((mean_q16 * LIGHT_ONE_Q12 * 18 + 25 * 65_536 / 2) / (25 * 65_536))
                    .min(MAX_SKY_DARK_Q12);
                light_q12.push(
                    u16::try_from(LIGHT_ONE_Q12 - dark_q12)
                        .map_err(|_| invalid("sky light outside Q12"))?,
                );
            }
        }
        Ok(Self {
            first_x_km: x0,
            first_y_km: y0,
            width: w,
            height: h,
            light_q12,
            context_x_km: hx0,
            context_y_km: hy0,
            context_width: hw,
            context_height: hh,
            context_mm: heights,
        })
    }

    /// Bilinear 1 km context height at an absolute position.
    pub(super) fn context_height_mm(&self, x_um: i128, y_um: i128) -> Result<i64, RenderError> {
        let fx = x_um - i128::from(self.context_x_km) * KM_UM;
        let fy = y_um - i128::from(self.context_y_km) * KM_UM;
        let ix = usize::try_from(fx.div_euclid(KM_UM)).map_err(|_| invalid("broad x west"))?;
        let iy = usize::try_from(fy.div_euclid(KM_UM)).map_err(|_| invalid("broad y north"))?;
        if ix + 1 >= self.context_width || iy + 1 >= self.context_height {
            return Err(invalid("broad gradient outside 1 km context"));
        }
        let tx = fx.rem_euclid(KM_UM);
        let ty = fy.rem_euclid(KM_UM);
        let at = |x: usize, y: usize| i128::from(self.context_mm[y * self.context_width + x]);
        let top = at(ix, iy) * (KM_UM - tx) + at(ix + 1, iy) * tx;
        let bottom = at(ix, iy + 1) * (KM_UM - tx) + at(ix + 1, iy + 1) * tx;
        i64::try_from((top * (KM_UM - ty) + bottom * ty) / (KM_UM * KM_UM))
            .map_err(|_| invalid("broad height overflow"))
    }

    pub(super) fn sample_q12(&self, x_um: i128, y_um: i128) -> Result<u16, RenderError> {
        let x_km =
            i64::try_from(x_um.div_euclid(KM_UM)).map_err(|_| invalid("sky sample x overflow"))?;
        let y_km =
            i64::try_from(y_um.div_euclid(KM_UM)).map_err(|_| invalid("sky sample y overflow"))?;
        let ix = usize::try_from(x_km - self.first_x_km)
            .map_err(|_| invalid("sky sample west of area"))?;
        let iy = usize::try_from(y_km - self.first_y_km)
            .map_err(|_| invalid("sky sample north of area"))?;
        if ix >= self.width || iy >= self.height {
            return Err(invalid("sky sample outside area nodes"));
        }
        let fx = x_um.rem_euclid(KM_UM);
        let fy = y_um.rem_euclid(KM_UM);
        if (fx > 0 && ix + 1 >= self.width) || (fy > 0 && iy + 1 >= self.height) {
            return Err(invalid("sky sample lacks interpolation neighbor"));
        }
        let x1 = (ix + 1).min(self.width - 1);
        let y1 = (iy + 1).min(self.height - 1);
        let at = |x: usize, y: usize| i128::from(self.light_q12[y * self.width + x]);
        let top = (at(ix, iy) * (KM_UM - fx) + at(x1, iy) * fx + KM_UM / 2) / KM_UM;
        let bottom = (at(ix, y1) * (KM_UM - fx) + at(x1, y1) * fx + KM_UM / 2) / KM_UM;
        u16::try_from((top * (KM_UM - fy) + bottom * fy + KM_UM / 2) / KM_UM)
            .map_err(|_| invalid("sky interpolation outside Q12"))
    }
}
