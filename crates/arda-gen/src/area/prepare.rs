//! Freeze one evolved physical domain, then slice its immutable prepared areas.
use crate::continent::{
    bundles::{abs_cell, coarse_height, refine_height, TileBundle},
    Continent,
};
use crate::hydrology::{
    prepared_domain::PreparedDomain, HydrologyError, PreparedExtent, PreparedTerrain,
};
use arda_core::{HeightMm, RainfallMm, AREA_CELLS};

/// Physical heights shared by publication areas and modeled fringe cells.
pub(crate) struct SharedTerrain {
    width: usize,
    height: usize,
    heights: Vec<i32>,
}

impl SharedTerrain {
    /// Logic/02 "Shared terrain correction": area coordinates partition output,
    /// never the physical boundary conditions or upstream catchment.
    pub(crate) fn build(
        seed: u64,
        continent: &Continent,
        domain: PreparedDomain,
    ) -> Result<Self, HydrologyError> {
        let width = domain.width() as usize;
        let height = domain.height() as usize;
        super::evolution::scratch_bytes(width, height)?;
        super::temperature::sample_temperature_base(&continent.grid, &continent.climate, 0, 0)?;
        let count = width * height;
        let mut heights = Vec::new();
        let mut uplift = Vec::new();
        heights.try_reserve_exact(count).map_err(|_| {
            HydrologyError::TerrainPreparation("physical terrain allocation refused")
        })?;
        uplift.try_reserve_exact(count).map_err(|_| {
            HydrologyError::TerrainPreparation("regional terrain allocation refused")
        })?;
        for y in 0..domain.height() {
            for x in 0..domain.width() {
                let x = i32::try_from(x)
                    .map_err(|_| HydrologyError::TerrainPreparation("terrain coordinate"))?;
                let y = i32::try_from(y)
                    .map_err(|_| HydrologyError::TerrainPreparation("terrain coordinate"))?;
                let coarse = coarse_height(&continent.grid, x, y);
                heights.push(refine_height(seed, coarse, x, y));
                uplift.push(coarse);
            }
        }
        super::evolution::evolve(&mut heights, &uplift, width, height)?;
        Ok(Self {
            width,
            height,
            heights,
        })
    }
}

/// Copy the modeled cells; padding outside `valid` is never serialized.
///
/// # Errors
/// Returns a terrain preparation error for invalid domain dimensions or inconsistent climate.
pub(crate) fn prepare_area_terrain(
    terrain: &SharedTerrain,
    continent: &Continent,
    bundle: &TileBundle,
    valid: PreparedExtent,
) -> Result<PreparedTerrain, HydrologyError> {
    if valid.width == 0
        || valid.height == 0
        || valid.width > AREA_CELLS
        || valid.height > AREA_CELLS
    {
        return Err(HydrologyError::TerrainPreparation(
            "invalid prepared extent",
        ));
    }
    let invalid = || HydrologyError::TerrainPreparation("prepared area outside physical domain");
    let (x0, y0) = abs_cell(bundle.area, 0, 0);
    let x0 = usize::try_from(x0).map_err(|_| invalid())?;
    let y0 = usize::try_from(y0).map_err(|_| invalid())?;
    if x0
        .checked_add(usize::from(valid.width))
        .is_none_or(|end| end > terrain.width)
        || y0
            .checked_add(usize::from(valid.height))
            .is_none_or(|end| end > terrain.height)
    {
        return Err(invalid());
    }
    // Reject malformed climate before allocating the prepared copy.
    super::temperature::sample_temperature_base(&continent.grid, &continent.climate, 0, 0)?;
    let count = usize::from(AREA_CELLS) * usize::from(AREA_CELLS);
    let mut heights = vec![HeightMm::new(0); count];
    for y in 0..usize::from(valid.height) {
        for x in 0..usize::from(valid.width) {
            heights[y * usize::from(AREA_CELLS) + x] =
                HeightMm::new(terrain.heights[(y0 + y) * terrain.width + x0 + x]);
        }
    }
    let annual_rain = super::area_rainfall(bundle)
        .into_iter()
        .map(RainfallMm::new)
        .collect();
    let mut temperature_base_centi = Vec::with_capacity(count);
    for y in 0..AREA_CELLS {
        for x in 0..AREA_CELLS {
            let (ax, ay) = abs_cell(bundle.area, x, y);
            temperature_base_centi.push(super::temperature::sample_temperature_base(
                &continent.grid,
                &continent.climate,
                ax,
                ay,
            )?);
        }
    }
    Ok(PreparedTerrain {
        area: bundle.area,
        valid,
        heights,
        annual_rain,
        temperature_base_centi,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        continent::{build_continent, bundles::bundle_for},
        hydrology::types::MarineBoundary,
    };
    use arda_core::{AreaCoord, GenerateConfig};

    #[test]
    fn neighboring_prepared_areas_slice_one_surface_in_either_order() {
        let continent = build_continent(42, GenerateConfig::MICRO, 2);
        let terrain = SharedTerrain {
            width: 1029,
            height: 3,
            heights: (0..3087).map(|i| 100_000 + i).collect(),
        };
        let mut forward = Vec::new();
        for ax in 0..3 {
            let bundle = bundle_for(42, &continent, AreaCoord::new(ax, 0));
            let valid = PreparedExtent {
                width: if ax == 2 { 5 } else { 512 },
                height: 3,
                boundary: MarineBoundary::default(),
            };
            let prepared = prepare_area_terrain(&terrain, &continent, &bundle, valid).unwrap();
            for y in 0..3 {
                for x in 0..usize::from(valid.width) {
                    assert_eq!(
                        prepared.heights[y * 512 + x].raw(),
                        terrain.heights[y * 1029 + usize::try_from(ax).unwrap() * 512 + x]
                    );
                }
            }
            forward.push(prepared);
        }
        for ax in (0..3).rev() {
            let bundle = bundle_for(42, &continent, AreaCoord::new(ax, 0));
            let again = prepare_area_terrain(
                &terrain,
                &continent,
                &bundle,
                forward[usize::try_from(ax).unwrap()].valid,
            )
            .unwrap();
            assert_eq!(again, forward[usize::try_from(ax).unwrap()]);
        }
        let outside = bundle_for(42, &continent, AreaCoord::new(3, 0));
        assert!(prepare_area_terrain(&terrain, &continent, &outside, forward[0].valid).is_err());
    }
}
