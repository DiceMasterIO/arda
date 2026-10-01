//! Footprint quadrature and bilinear reads of the saved 100 m cell fields
//! (wetness, moisture, forest, climate, lake surfaces and shore classes)
//! around an absolute fine position.

use arda_core::TerrainKind;

use super::{SavedFields, SavedMaterial, CELL_UM};
use crate::atlas::{invalid, sample_index, AxisKernel, SAMPLE_SIDE};
use crate::RenderError;

pub(super) fn equivalent_delta(
    high: i32,
    low: i32,
    separation_um: i128,
) -> Result<i64, RenderError> {
    if separation_um <= 0 {
        return Err(invalid("fine derivative has zero physical baseline"));
    }
    let numerator = i128::from(high) - i128::from(low);
    let equivalent = round_div(numerator * 2 * CELL_UM, separation_um);
    i64::try_from(equivalent).map_err(|_| invalid("fine derivative overflow"))
}

pub(super) fn round_div(n: i128, d: i128) -> i128 {
    if n < 0 {
        -((-n + d / 2) / d)
    } else {
        (n + d / 2) / d
    }
}

pub(super) fn axis_positions(
    kernel: AxisKernel,
    origin: i128,
    spacing: u32,
) -> Result<(i128, i128, u32), RenderError> {
    match kernel {
        AxisKernel::Linear {
            low,
            high_weight,
            denominator,
        } => {
            let numerator =
                (i128::from(low) * i128::from(denominator) + i128::from(high_weight)) * CELL_UM;
            let p = origin + round_div(numerator, i128::from(denominator));
            Ok((p, p, 1))
        }
        AxisKernel::Box { start, end } => {
            let a = origin + (i128::from(start) * CELL_UM - CELL_UM / 2);
            let b = origin + (i128::from(end) * CELL_UM - CELL_UM / 2);
            let count = u32::try_from(((b - a) + i128::from(spacing) - 1) / i128::from(spacing))
                .map_err(|_| invalid("fine box quadrature count overflow"))?;
            Ok((a, b, count.max(1)))
        }
    }
}

pub(super) fn position(start: i128, span: i128, index: u32, count: u32) -> i128 {
    if count == 1 && span == 0 {
        start
    } else {
        start + round_div((i128::from(index) * 2 + 1) * span, i128::from(count) * 2)
    }
}

/// Bilinear weights (Q12) of the stored shore classes around a point,
/// indexed by class; all zero without a shore layer.
pub(super) fn shore_near(
    x: i128,
    y: i128,
    origin: (i128, i128),
    shore: &[u8],
) -> Result<[i64; 8], RenderError> {
    let mut mix = [0_i64; 8];
    if shore.len() != SAMPLE_SIDE * SAMPLE_SIDE {
        return Ok(mix);
    }
    let (ox, oy) = origin;
    let qx = (x - ox).div_euclid(CELL_UM).clamp(-1, 511);
    let qy = (y - oy).div_euclid(CELL_UM).clamp(-1, 511);
    let fx = (x - ox - qx * CELL_UM).clamp(0, CELL_UM);
    let fy = (y - oy - qy * CELL_UM).clamp(0, CELL_UM);
    for (cx, wx) in [(qx, CELL_UM - fx), (qx + 1, fx)] {
        for (cy, wy) in [(qy, CELL_UM - fy), (qy + 1, fy)] {
            let i = sample_index(
                i16::try_from(cx).map_err(|_| invalid("shore x conversion"))?,
                i16::try_from(cy).map_err(|_| invalid("shore y conversion"))?,
            )?;
            let class = usize::from(shore[i]).min(7);
            let w = i64::try_from(wx * wy * 4_096 / (CELL_UM * CELL_UM)).unwrap_or(0);
            mix[class] += w;
        }
    }
    Ok(mix)
}

/// Recipe-7 arid water around a point: bilinear Q12 weights of saline lake,
/// salt crust and mudflat cells, and whether all four cells are land (dry
/// ground below sea level inside the continent is land, not sea).
#[derive(Clone, Copy)]
pub(super) struct AridMix {
    pub saline: i64,
    pub crust: i64,
    pub mudflat: i64,
    pub inland: bool,
}

pub(super) fn salt_near(
    x: i128,
    y: i128,
    origin: (i128, i128),
    saved: &SavedFields<'_>,
) -> Result<AridMix, RenderError> {
    let (ox, oy) = origin;
    let qx = (x - ox).div_euclid(CELL_UM).clamp(-1, 511);
    let qy = (y - oy).div_euclid(CELL_UM).clamp(-1, 511);
    let fx = (x - ox - qx * CELL_UM).clamp(0, CELL_UM);
    let fy = (y - oy - qy * CELL_UM).clamp(0, CELL_UM);
    let mut mix = AridMix {
        saline: 0,
        crust: 0,
        mudflat: 0,
        inland: true,
    };
    for (cx, wx) in [(qx, CELL_UM - fx), (qx + 1, fx)] {
        for (cy, wy) in [(qy, CELL_UM - fy), (qy + 1, fy)] {
            let i = sample_index(
                i16::try_from(cx).map_err(|_| invalid("salt x conversion"))?,
                i16::try_from(cy).map_err(|_| invalid("salt y conversion"))?,
            )?;
            let w = i64::try_from(wx * wy * 4_096 / (CELL_UM * CELL_UM)).unwrap_or(0);
            match saved.salt.get(i).copied() {
                Some(crate::atlas::SALT_SALINE_LAKE) => mix.saline += w,
                Some(crate::atlas::SALT_CRUST) => mix.crust += w,
                Some(crate::atlas::SALT_MUDFLAT) => mix.mudflat += w,
                _ => {}
            }
            mix.inland &= saved.classes.get(i) == Some(&TerrainKind::Land);
        }
    }
    Ok(mix)
}

/// The arid mix around a point, or `None` for worlds without arid water.
pub(super) fn arid_near(
    x: i128,
    y: i128,
    origin: (i128, i128),
    saved: &SavedFields<'_>,
) -> Result<Option<AridMix>, RenderError> {
    (!saved.salt.is_empty())
        .then(|| salt_near(x, y, origin, saved))
        .transpose()
}

/// Highest saved lake surface among the (up to) four cells around a point.
/// A sub-sample is lake water where the fine ground lies below it, so lake
/// shorelines follow the 39 m field (logic/04 §atlas-formed lakes).
pub(super) fn lake_surface_near(
    x: i128,
    y: i128,
    origin: (i128, i128),
    lake_surface: &[i32],
) -> Result<Option<i32>, RenderError> {
    let (ox, oy) = origin;
    let qx = (x - ox).div_euclid(CELL_UM).clamp(-1, 511);
    let qy = (y - oy).div_euclid(CELL_UM).clamp(-1, 511);
    let mut best: Option<i32> = None;
    for cx in [qx, qx + 1] {
        for cy in [qy, qy + 1] {
            let i = sample_index(
                i16::try_from(cx).map_err(|_| invalid("lake x conversion"))?,
                i16::try_from(cy).map_err(|_| invalid("lake y conversion"))?,
            )?;
            let s = lake_surface[i];
            if s != i32::MIN {
                best = Some(best.map_or(s, |b| b.max(s)));
            }
        }
    }
    Ok(best)
}

/// Saved mean annual temperature interpolated over land cells and lapsed at
/// 6.5 C/km from the interpolated saved cell height to the fine height
/// (logic/04 §atlas-formed snow).
pub(super) fn saved_temperature(
    x: i128,
    y: i128,
    origin: (i128, i128),
    classes: &[TerrainKind],
    climate: (&[i16], &[i32]),
    fine_height_mm: i32,
) -> Result<i32, RenderError> {
    let (temperature, heights) = climate;
    if temperature.len() != classes.len() || heights.len() != classes.len() {
        return Err(invalid("fine saved climate has wrong dimensions"));
    }
    let (ox, oy) = origin;
    let qx = (x - ox).div_euclid(CELL_UM).clamp(-1, 511);
    let qy = (y - oy).div_euclid(CELL_UM).clamp(-1, 511);
    let fx = (x - ox - qx * CELL_UM).clamp(0, CELL_UM);
    let fy = (y - oy - qy * CELL_UM).clamp(0, CELL_UM);
    let (mut t_sum, mut h_sum, mut w_sum) = (0_i128, 0_i128, 0_i128);
    for (cx, wx) in [(qx, CELL_UM - fx), (qx + 1, fx)] {
        for (cy, wy) in [(qy, CELL_UM - fy), (qy + 1, fy)] {
            let i = sample_index(
                i16::try_from(cx).map_err(|_| invalid("climate x conversion"))?,
                i16::try_from(cy).map_err(|_| invalid("climate y conversion"))?,
            )?;
            if classes[i] == TerrainKind::Land {
                let w = wx * wy;
                t_sum += i128::from(temperature[i]) * w;
                h_sum += i128::from(heights[i]) * w;
                w_sum += w;
            }
        }
    }
    if w_sum == 0 {
        return Ok(1_500 - i32::try_from(i64::from(fine_height_mm) * 65 / 100_000).unwrap_or(0));
    }
    let t = t_sum / w_sum;
    let h = h_sum / w_sum;
    let lapse = (i128::from(fine_height_mm) - h) * 65 / 100_000;
    i32::try_from(t - lapse).map_err(|_| invalid("temperature overflow"))
}

pub(super) fn saved_material(
    x: i128,
    y: i128,
    origin: (i128, i128),
    classes: &[TerrainKind],
    wetness: &[u8],
    moisture: &[u8],
    forest_density: &[u8],
) -> Result<Option<SavedMaterial>, RenderError> {
    let (ox, oy) = origin;
    if classes.len() != SAMPLE_SIDE * SAMPLE_SIDE
        || wetness.len() != classes.len()
        || moisture.len() != classes.len()
        || forest_density.len() != classes.len()
    {
        return Err(invalid("fine saved context has wrong dimensions"));
    }
    let qx = (x - ox).div_euclid(CELL_UM).clamp(-1, 511);
    let qy = (y - oy).div_euclid(CELL_UM).clamp(-1, 511);
    let fx = (x - ox - qx * CELL_UM).clamp(0, CELL_UM);
    let fy = (y - oy - qy * CELL_UM).clamp(0, CELL_UM);
    let mut sum = [0_i128; 3];
    let mut weight_sum = 0_i128;
    for (cx, wx) in [(qx, CELL_UM - fx), (qx + 1, fx)] {
        for (cy, wy) in [(qy, CELL_UM - fy), (qy + 1, fy)] {
            let i = sample_index(
                i16::try_from(cx).map_err(|_| invalid("wetness x conversion"))?,
                i16::try_from(cy).map_err(|_| invalid("wetness y conversion"))?,
            )?;
            if classes[i] == TerrainKind::Land {
                let w = wx * wy;
                sum[0] += i128::from(wetness[i]) * w;
                sum[1] += i128::from(moisture[i]) * w;
                sum[2] += i128::from(forest_density[i]) * w;
                weight_sum += w;
            }
        }
    }
    if weight_sum == 0 {
        Ok(None)
    } else {
        let read = |v: i128| {
            u8::try_from((v + weight_sum / 2) / weight_sum)
                .map_err(|_| invalid("material interpolation overflow"))
        };
        Ok(Some(SavedMaterial {
            wetness: read(sum[0])?,
            moisture: read(sum[1])?,
            forest_density: read(sum[2])?,
            shore: [0; 8],
        }))
    }
}
