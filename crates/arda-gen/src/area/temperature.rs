//! Annual temperature is finalized only after global fine ocean connectivity.
use crate::continent::{climate::ContinentClimate, ContinentGrid};
use crate::hydrology::HydrologyError;
use arda_core::{HeightMm, TempCentiC};

// Equivalent to i64(height_mm)*650/1_000_000, including truncation toward zero.
// Quotient/remainder decomposition keeps each i32 product in range even at
// either signed-i32 physical height extreme.
fn lapse_centi(height_mm: i32) -> i32 {
    (height_mm / 1_000_000) * 650 + (height_mm % 1_000_000) * 650 / 1_000_000
}

/// Sample the lapse-removed annual reference at absolute 100 m coordinates.
/// Coarse ocean has zero atmospheric elevation; all other coarse cells use
/// signed physical height, including closed negative land. The i32 result is
/// a reference in centi-degrees, not a final temperature and is not clamped.
///
/// # Errors
/// Returns a terrain-preparation error for invalid dimensions or climate shapes.
pub fn sample_temperature_base(
    grid: &ContinentGrid,
    climate: &ContinentClimate,
    abs_x: i32,
    abs_y: i32,
) -> Result<i32, HydrologyError> {
    let (w, h) = (grid.width(), grid.height());
    let count = usize::try_from(i64::from(w) * i64::from(h))
        .map_err(|_| HydrologyError::TerrainPreparation("invalid continent dimensions"))?;
    if w < 1 || h < 1 || climate.temperature.len() != count || climate.ocean.len() != count {
        return Err(HydrologyError::TerrainPreparation(
            "inconsistent continent temperature",
        ));
    }
    Ok(crate::terrain_interpolation::sample(
        abs_x,
        abs_y,
        |kx, ky| {
            let (kx, ky) = (kx.clamp(0, w - 1), ky.clamp(0, h - 1));
            // Positive dimensions and matching slices above prove this index exists.
            let i = usize::try_from(i64::from(ky) * i64::from(w) + i64::from(kx)).unwrap_or(0);
            let elevation = if climate.ocean[i] {
                0
            } else {
                grid.get(kx, ky).raw()
            };
            i32::from(climate.temperature[i]) + lapse_centi(elevation)
        },
    ))
}

/// Apply the physical lapse once, after fine marine membership is authoritative.
/// Nonmarine cells use signed height, including closed negative lake beds.
/// Marine cells use sea level regardless of the seabed elevation. Only this
/// finalized value is passed to monthly forcing and persisted in area cells.
#[must_use]
pub fn temperature_from_base(base_centi: i32, physical: HeightMm, is_marine: bool) -> TempCentiC {
    let elevation = if is_marine { 0 } else { physical.raw() };
    let t = i64::from(base_centi) - i64::from(lapse_centi(elevation));
    // The existing climate clamp proves conversion to i16 cannot fail.
    TempCentiC::new(i16::try_from(t.clamp(-30_000, 30_000)).unwrap_or(0))
}
