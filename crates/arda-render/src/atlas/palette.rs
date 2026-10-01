//! The Atlas palette and light: hypsometric land and sea stops, relief light
//! from saved gradients, and ecological land material tints.

use super::{source, SourceSample, LIGHT_ONE};
use crate::RenderError;

const CELL_DIAMETER_MM: i128 = 200_000;
const LIGHT_HORIZONTAL_Q15: i128 = 13_377;
const LIGHT_UP_Q15: i128 = 26_755;
const FLAT_LIGHT_Q15: i128 = LIGHT_UP_Q15;
const RELIEF_STRENGTH: i128 = 3_450;
pub(super) const MIN_LIGHT: i128 = 1_600;
pub(super) const MAX_LIGHT: i128 = 5_350;

const LAND_STOPS: [(i32, [u8; 3]); 7] = [
    (0, [104, 133, 68]),
    (200_000, [132, 151, 78]),
    (500_000, [165, 161, 97]),
    (900_000, [182, 162, 116]),
    (1_400_000, [167, 143, 109]),
    (2_000_000, [137, 128, 112]),
    (2_800_000, [166, 160, 147]),
];
const SEA_STOPS: [(i64, [u8; 3]); 5] = [
    (0, [103, 163, 168]),
    (50_000, [43, 110, 141]),
    (250_000, [20, 72, 110]),
    (1_000_000, [12, 45, 80]),
    (6_000_000, [7, 26, 51]),
];

pub(super) fn gradient_numerators(
    context: &[Option<SourceSample>],
    x: i16,
    y: i16,
    edges: [bool; 4],
) -> Result<(i64, i64), RenderError> {
    let dx = if (edges[3] && x == 0) || (edges[1] && x == 511) {
        let (left, right) = if x == 0 { (0, 1) } else { (510, 511) };
        2 * (i64::from(source(context, right, y)?.height_mm)
            - i64::from(source(context, left, y)?.height_mm))
    } else {
        i64::from(source(context, x + 1, y)?.height_mm)
            - i64::from(source(context, x - 1, y)?.height_mm)
    };
    let dy = if (edges[0] && y == 0) || (edges[2] && y == 511) {
        let (top, bottom) = if y == 0 { (0, 1) } else { (510, 511) };
        2 * (i64::from(source(context, x, bottom)?.height_mm)
            - i64::from(source(context, x, top)?.height_mm))
    } else {
        i64::from(source(context, x, y + 1)?.height_mm)
            - i64::from(source(context, x, y - 1)?.height_mm)
    };
    Ok((dx, dy))
}

pub(super) fn relief_light(dx: i64, dy: i64) -> u16 {
    let dx = i128::from(dx);
    let dy = i128::from(dy);
    let squared = dx * dx + dy * dy + CELL_DIAMETER_MM * CELL_DIAMETER_MM;
    let length = i128::try_from(squared.unsigned_abs().isqrt()).unwrap_or_else(|_| {
        unreachable!("saved i32 height differences yield an i128 normal length")
    });
    let dot =
        dx * LIGHT_HORIZONTAL_Q15 + dy * LIGHT_HORIZONTAL_Q15 + CELL_DIAMETER_MM * LIGHT_UP_Q15;
    let cosine_q15 = dot / length;
    let light = (i128::from(LIGHT_ONE) + (cosine_q15 - FLAT_LIGHT_Q15) * RELIEF_STRENGTH / 32_768)
        .clamp(MIN_LIGHT, MAX_LIGHT);
    u16::try_from(light).unwrap_or_else(|_| unreachable!("clamped Q12 light fits u16"))
}

pub(super) fn interpolate(low: [u8; 3], high: [u8; 3], offset: i64, span: i64) -> [u8; 3] {
    std::array::from_fn(|channel| {
        let value = (i64::from(low[channel]) * (span - offset)
            + i64::from(high[channel]) * offset
            + span / 2)
            / span;
        u8::try_from(value).unwrap_or_else(|_| unreachable!("convex RGB blend fits u8"))
    })
}

pub(super) fn land_palette(height_mm: i32) -> [u8; 3] {
    let height = height_mm.max(0);
    for pair in LAND_STOPS.windows(2) {
        let (lower, low) = pair[0];
        let (upper, high) = pair[1];
        if height < upper {
            return interpolate(
                low,
                high,
                i64::from(height - lower),
                i64::from(upper - lower),
            );
        }
    }
    LAND_STOPS[LAND_STOPS.len() - 1].1
}

/// Varies exposed rock with measured slope and snow with saved elevation.
///
/// All weights are Q12 integers. Opposing saved cells are 200 m apart, so
/// their millimetre height difference provides a bounded physical gradient.
#[cfg(test)]
pub(super) fn land_material(height_mm: i32, dx: i64, dy: i64, wetness: u8) -> [u8; 3] {
    land_material_ecology(height_mm, dx, dy, wetness, 0, 0)
}

/// Climate and canopy affect the ground before rock and snow override it.
/// Old saved worlds have both ecology fields zero and keep their original
/// palette byte for byte.
pub(super) fn land_material_ecology(
    height_mm: i32,
    dx: i64,
    dy: i64,
    wetness: u8,
    moisture: u8,
    forest_density: u8,
) -> [u8; 3] {
    let ground = land_palette(height_mm);
    let base = if moisture == 0 && forest_density == 0 {
        wetness_tint(ground, wetness)
    } else {
        let climate = if moisture < 150 {
            blend(
                ground,
                [184, 163, 106],
                i128::from(150 - moisture) * 2_300 / 150,
            )
        } else {
            blend(
                ground,
                [91, 143, 77],
                i128::from(moisture - 150) * 2_300 / 105,
            )
        };
        blend(
            climate,
            [43, 89, 58],
            i128::from(forest_density) * 3_250 / 255,
        )
    };
    let squared = i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy);
    let slope_mm = i128::try_from(squared.unsigned_abs().isqrt())
        .unwrap_or_else(|_| unreachable!("i32 saved heights yield a bounded slope"));
    let rock_q12 = ((slope_mm - 35_000) * 3_500 / 170_000).clamp(0, 3_500);
    let rock_altitude_q12 =
        ((i128::from(height_mm) - 1_000_000) * 4_096 / 1_000_000).clamp(0, 4_096);
    let rock = blend([111, 105, 87], [114, 111, 105], rock_altitude_q12);
    let colour = blend(base, rock, rock_q12);
    let snow_altitude_q12 =
        ((i128::from(height_mm) - 2_850_000) * 4_096 / 1_450_000).clamp(0, 4_096);
    let snow_shelter_q12 = 4_096 - rock_q12 * 3 / 4;
    blend(
        colour,
        [229, 228, 223],
        snow_altitude_q12 * snow_shelter_q12 / 4_096,
    )
}

// Saved wetness is a drainage/slope indicator, so tint precedes rock and snow.
pub(super) fn wetness_tint(base: [u8; 3], wetness: u8) -> [u8; 3] {
    let wet = i128::from(wetness);
    let weight_q12 = wet * 2_048 / (wet + 12);
    blend(base, [81, 126, 73], weight_q12)
}

pub(super) fn blend(a: [u8; 3], b: [u8; 3], weight_q12: i128) -> [u8; 3] {
    debug_assert!((0..=4_096).contains(&weight_q12));
    std::array::from_fn(|channel| {
        let value = (i128::from(a[channel]) * (4_096 - weight_q12)
            + i128::from(b[channel]) * weight_q12
            + 2_048)
            / 4_096;
        u8::try_from(value).unwrap_or_else(|_| unreachable!("convex RGB blend fits u8"))
    })
}

pub(super) fn sea_palette(height_mm: i32) -> [u8; 3] {
    water_depth_palette((-i64::from(height_mm)).max(0))
}

pub(super) fn water_depth_palette(depth: i64) -> [u8; 3] {
    for pair in SEA_STOPS.windows(2) {
        let (lower, low) = pair[0];
        let (upper, high) = pair[1];
        if depth < upper {
            return interpolate(low, high, depth - lower, upper - lower);
        }
    }
    SEA_STOPS[SEA_STOPS.len() - 1].1
}

pub(super) fn modulate(palette: [u8; 3], light: u16) -> [u8; 3] {
    std::array::from_fn(|channel| {
        let value = (u32::from(palette[channel]) * u32::from(light) + u32::from(LIGHT_ONE) / 2)
            / u32::from(LIGHT_ONE);
        u8::try_from(value.min(255))
            .unwrap_or_else(|_| unreachable!("clamped RGB modulation fits u8"))
    })
}
