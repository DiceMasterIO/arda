//! Freeze one evolved physical domain, then slice its immutable prepared areas.
use crate::continent::{
    bundles::{abs_cell, coarse_height, refine_height, refinement_relief, TileBundle},
    structural_relief, Continent,
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
                uplift.push(coarse_height(&continent.grid, x, y));
            }
        }
        // logic/02 "Regional detail correction": immutable regional relief
        // supplies detail amplitude. Reuse this already-admitted dense field
        // instead of interpolating eight more times for every modeled cell.
        for y in 0..domain.height() {
            for x in 0..domain.width() {
                let x = i32::try_from(x)
                    .map_err(|_| HydrologyError::TerrainPreparation("terrain coordinate"))?;
                let y = i32::try_from(y)
                    .map_err(|_| HydrologyError::TerrainPreparation("terrain coordinate"))?;
                let at = heights.len();
                let relief = cached_relief(&uplift, width, height, x, y, |nx, ny| {
                    coarse_height(&continent.grid, nx, ny)
                });
                let refined = refine_height(seed, uplift[at], relief, x, y);
                heights.push(cached_structural_height(
                    seed,
                    &uplift,
                    (width, height),
                    x,
                    y,
                    refined,
                    |nx, ny| coarse_height(&continent.grid, nx, ny),
                ));
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

// Coordinates and dimensions have passed the domain's i32/u32 checks. Outside
// samples use the public sampler's regional surface; clamping to this cache
// instead would create a different physical condition at the modeled rim.
#[allow(clippy::cast_sign_loss)]
fn cached_relief(
    regional: &[i32],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    outside: impl Fn(i32, i32) -> i32,
) -> i64 {
    refinement_relief(regional[y as usize * width + x as usize], |dx, dy| {
        let (nx, ny) = (x.saturating_add(dx), y.saturating_add(dy));
        if nx >= 0 && ny >= 0 && (nx as usize) < width && (ny as usize) < height {
            regional[ny as usize * width + nx as usize]
        } else {
            outside(nx, ny)
        }
    })
}

// The dense unperturbed regional field is already owned for uplift. Interior
// 5 km gate samples reuse it; true outside samples use the same absolute
// sampler as direct boundary_height. There is no tile or crop-edge clamp.
#[allow(clippy::cast_sign_loss)]
fn cached_structural_height(
    seed: u64,
    regional: &[i32],
    dimensions: (usize, usize),
    x: i32,
    y: i32,
    refined: i32,
    outside: impl Fn(i32, i32) -> i32,
) -> i32 {
    let (width, height) = dimensions;
    let at = y as usize * width + x as usize;
    structural_relief::sample_height(seed, x, y, refined, regional[at], |nx, ny| {
        if nx >= 0 && ny >= 0 && (nx as usize) < width && (ny as usize) < height {
            regional[ny as usize * width + nx as usize]
        } else {
            outside(nx, ny)
        }
    })
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
    fn cached_relief_matches_direct_sampling_at_cuts_and_partial_rims() {
        // The nonlinear field makes a wrongly clamped outside sample visible.
        // The cache crosses x=512 and ends with a partial publication area.
        let (width, height) = (517, 23);
        let field = |x: i32, y: i32| 300_000 + 7 * x * x + 3 * y * y + 5 * x * y;
        let regional: Vec<_> = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| field(i32::try_from(x).unwrap(), i32::try_from(y).unwrap()))
            })
            .collect();
        for y in 0..height {
            for x in 0..width {
                let (x, y) = (i32::try_from(x).unwrap(), i32::try_from(y).unwrap());
                let direct = refinement_relief(field(x, y), |dx, dy| field(x + dx, y + dy));
                assert_eq!(cached_relief(&regional, width, height, x, y, field), direct);
            }
        }
    }

    #[test]
    fn cached_structural_matches_direct_at_cuts_and_outer_rims() {
        // Nonlinear absolute samples expose a cache clamp at either physical rim.
        let (width, height) = (517, 83);
        let field = |x: i32, y: i32| {
            1_300_000
                + 8_000 * x
                + 4_000 * y
                + 9_000 * (x.div_euclid(31) % 7)
                + 6_000 * (y.div_euclid(13) % 11)
        };
        let regional: Vec<_> = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| field(i32::try_from(x).unwrap(), i32::try_from(y).unwrap()))
            })
            .collect();
        for y in [0, 1, 35, 50, 82] {
            for x in [0, 1, 35, 50, 511, 512, 516] {
                let refined = 2_000_000;
                let gate = structural_relief::relief_gate_q16(x, y, field(x, y), field);
                assert!(
                    (1..65_536).contains(&gate),
                    "uninformative gate at ({x},{y})"
                );
                let direct =
                    structural_relief::sample_height(42, x, y, refined, field(x, y), field);
                let cached =
                    cached_structural_height(42, &regional, (width, height), x, y, refined, field);
                assert_eq!(cached, direct, "cached mismatch at ({x},{y})");
            }
        }
        let clamped = |x: i32, y: i32| field(x.clamp(0, 516), y.clamp(0, 82));
        let mut exposed = 0;
        for (x, y) in [(0, 0), (516, 82), (0, 50), (516, 35)] {
            let direct = structural_relief::sample_height(42, x, y, 2_000_000, field(x, y), field);
            let wrongly_clamped =
                cached_structural_height(42, &regional, (width, height), x, y, 2_000_000, clamped);
            exposed += usize::from(direct != wrongly_clamped);
        }
        assert!(exposed > 0, "outer-rim control failed to expose clamping");
    }

    #[test]
    fn structural_preserves_sea_and_flat_gate() {
        let regional = vec![-300_000; 3 * 3];
        assert_eq!(
            cached_structural_height(42, &regional, (3, 3), 1, 1, -100_000, |_, _| 2_000_000),
            -100_000
        );
        let flat = vec![900_000; 3 * 3];
        assert_eq!(
            cached_structural_height(42, &flat, (3, 3), 1, 1, 1_100_000, |_, _| 900_000),
            1_100_000
        );
    }

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
