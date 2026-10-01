//! Deterministic atlas palette and relief derived from saved area cells.

use arda_core::{AreaCells, AreaCoord, CellCoord, Lake, TerrainField, TerrainKind};

mod detail;
mod fine;
mod formed;
mod halo;
mod palette;
mod relief;
mod sampling;
pub use fine::AtlasFineWorldBounds;
use fine::FineAtlas;
use halo::NeighborState;
pub use halo::{AtlasHalo, AtlasNeighbor};
#[cfg(test)]
use palette::land_material;
use palette::{
    gradient_numerators, land_material_ecology, modulate, relief_light, sea_palette,
    water_depth_palette,
};
pub use relief::{formed_river_rgb, ReliefGeometry, ReliefSurface};

use crate::channels::validated_lake_depths;

use crate::{carto::LAKE_FILL, RenderError};

const AREA: i16 = 512;
const CONTEXT_SIDE: usize = 516;
/// No arid water in a cell.
pub const SALT_NONE: u8 = 0;
/// A cell of a saline (terminal) lake.
pub const SALT_SALINE_LAKE: u8 = 1;
/// A dry cell of a salt pan's crust.
pub const SALT_CRUST: u8 = 2;
/// A dry cell of a salt pan's mudflat margin.
pub const SALT_MUDFLAT: u8 = 3;
const SAMPLE_SIDE: usize = 514;
const LIGHT_ONE: u16 = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SourceSample {
    height_mm: i32,
    class: TerrainKind,
    lake_depth_mm: Option<u32>,
    wetness: u8,
    moisture: u8,
    forest_density: u8,
    temperature_centi: i16,
    channel_width_dm: u32,
}

/// Atlas palette, Q12 light, and terrain ownership for one saved area.
///
/// Samples cover local coordinates `-1..=512`, including real neighboring
/// cells and independently clamped outer-world interpolation ghosts.
#[derive(Debug)]
pub struct AtlasTerrain {
    palette: Vec<[u8; 3]>,
    light: Vec<u16>,
    classes: Vec<TerrainKind>,
    heights: Vec<i32>,
    wetness: Vec<u8>,
    moisture: Vec<u8>,
    forest_density: Vec<u8>,
    temperature: Vec<i16>,
    /// Saved lake water surface per sample (`i32::MIN` off lakes), for
    /// recipe-5 shoreline contouring.
    lake_surface: Vec<i32>,
    /// Saved channel width per sample, for recipe-5 meanders.
    channel_width: Vec<u32>,
    /// Stored shore class per sample (0 = none or no shore layer).
    shore: Vec<u8>,
    /// Arid water per sample (recipe 7; empty before): [`SALT_NONE`],
    /// [`SALT_SALINE_LAKE`], [`SALT_CRUST`] or [`SALT_MUDFLAT`].
    salt: Vec<u8>,
    has_lake_depths: bool,
    /// The saved courses already meander (stored water forms): the
    /// synthetic floodplain meander is not added on top.
    data_meanders: bool,
    fine: Option<FineAtlas>,
}

impl AtlasTerrain {
    /// Marks the world's saved channel courses as meandering in the data
    /// (logic/02 §world-water), so recipe-5 rendering adds no synthetic
    /// meander (logic/04 §atlas-formed rivers).
    #[must_use]
    pub fn with_data_meanders(mut self) -> Self {
        self.data_meanders = true;
        self
    }

    /// Derives deterministic colors and relief from a saved area and complete halo.
    ///
    /// # Errors
    /// Returns [`RenderError::AtlasContext`] for missing, duplicate, or
    /// topologically inconsistent neighbor context.
    pub fn new(cells: &AreaCells, halo: AtlasHalo) -> Result<Self, RenderError> {
        Self::new_with_lakes(cells, &[], halo)
    }

    /// Uses a canonical fine-height window for land material and relief.
    ///
    /// Saved cells continue to own coast, lake, river, and wetness decisions.
    /// `world_bounds` contains the first and last canonical fine-source nodes,
    /// not the edge of this area or its fine window. Only a halo-marked outer
    /// world edge permits clamping a query beyond those source nodes.
    ///
    /// # Errors
    /// Returns invalid context, geometry, or insufficient fine-window coverage.
    pub fn new_with_fine(
        cells: &AreaCells,
        lakes: &[Lake],
        halo: AtlasHalo,
        area: AreaCoord,
        world_bounds: AtlasFineWorldBounds,
        fine_window: TerrainField,
    ) -> Result<Self, RenderError> {
        Self::new_with_fine_context(
            cells,
            lakes,
            halo,
            area,
            world_bounds,
            fine_window,
            None,
            false,
        )
    }

    /// Recipe-5 multi-scale shading and terrain-driven materials
    /// (logic/04 §atlas-formed). The fine window must extend 312.5 m plus
    /// the pixel footprint beyond the area; the 1 km context supplies broad
    /// gradients and sky exposure.
    ///
    /// # Errors
    /// Returns invalid context, geometry, or insufficient coverage.
    pub fn new_with_fine_formed(
        cells: &AreaCells,
        lakes: &[Lake],
        halo: AtlasHalo,
        area: AreaCoord,
        world_bounds: AtlasFineWorldBounds,
        fine_window: TerrainField,
        sky_context: TerrainField,
    ) -> Result<Self, RenderError> {
        Self::new_with_fine_context(
            cells,
            lakes,
            halo,
            area,
            world_bounds,
            fine_window,
            Some(sky_context),
            true,
        )
    }

    /// Adds broad sky exposure from a globally aligned 1 km context sampled
    /// from the same canonical source. Only recipe-4 worlds opt in; the
    /// original fine constructor retains recipe-2 radiance exactly.
    ///
    /// # Errors
    /// Returns invalid context, geometry, or missing 8 km horizon coverage.
    pub fn new_with_fine_sky(
        cells: &AreaCells,
        lakes: &[Lake],
        halo: AtlasHalo,
        area: AreaCoord,
        world_bounds: AtlasFineWorldBounds,
        fine_window: TerrainField,
        sky_context: TerrainField,
    ) -> Result<Self, RenderError> {
        Self::new_with_fine_context(
            cells,
            lakes,
            halo,
            area,
            world_bounds,
            fine_window,
            Some(sky_context),
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_fine_context(
        cells: &AreaCells,
        lakes: &[Lake],
        halo: AtlasHalo,
        area: AreaCoord,
        world_bounds: AtlasFineWorldBounds,
        fine_window: TerrainField,
        sky_context: Option<TerrainField>,
        formed: bool,
    ) -> Result<Self, RenderError> {
        let edges = [
            halo.states[AtlasNeighbor::North.index()] == NeighborState::WorldEdge,
            halo.states[AtlasNeighbor::East.index()] == NeighborState::WorldEdge,
            halo.states[AtlasNeighbor::South.index()] == NeighborState::WorldEdge,
            halo.states[AtlasNeighbor::West.index()] == NeighborState::WorldEdge,
        ];
        let mut terrain = Self::new_with_lakes(cells, lakes, halo)?;
        terrain.fine = Some(FineAtlas::new(
            area,
            world_bounds,
            fine_window,
            edges,
            sky_context,
            formed,
        )?);
        Ok(terrain)
    }

    /// Derives Atlas colors with validated saved lake depths.
    ///
    /// # Errors
    /// Returns invalid lake geometry or incomplete Atlas context.
    pub fn new_with_lakes(
        cells: &AreaCells,
        lakes: &[Lake],
        mut halo: AtlasHalo,
    ) -> Result<Self, RenderError> {
        if halo.states.contains(&NeighborState::Unset) {
            return Err(invalid("neighbor direction unset"));
        }
        for (diagonal, first, second) in [
            (
                AtlasNeighbor::NorthEast,
                AtlasNeighbor::North,
                AtlasNeighbor::East,
            ),
            (
                AtlasNeighbor::SouthEast,
                AtlasNeighbor::South,
                AtlasNeighbor::East,
            ),
            (
                AtlasNeighbor::SouthWest,
                AtlasNeighbor::South,
                AtlasNeighbor::West,
            ),
            (
                AtlasNeighbor::NorthWest,
                AtlasNeighbor::North,
                AtlasNeighbor::West,
            ),
        ] {
            let outside = halo.states[first.index()] == NeighborState::WorldEdge
                || halo.states[second.index()] == NeighborState::WorldEdge;
            if (halo.states[diagonal.index()] == NeighborState::WorldEdge) != outside {
                return Err(invalid("diagonal contradicts cardinal world bounds"));
            }
        }
        let lake_depths = validated_lake_depths(cells, lakes)?;
        let has_lake_depths = !lake_depths.is_empty();
        for y in 0..512 {
            for x in 0..512 {
                let at = CellCoord::new(x, y)
                    .ok_or_else(|| invalid("target coordinate outside area"))?;
                let index = context_index(
                    i16::try_from(x).map_err(|_| invalid("target column conversion"))?,
                    i16::try_from(y).map_err(|_| invalid("target row conversion"))?,
                )?;
                let cell = cells.get(at);
                halo.context[index] = Some(SourceSample {
                    height_mm: cell.height.raw(),
                    class: cell.terrain,
                    wetness: cell.wetness,
                    moisture: cell.moisture,
                    forest_density: cell.forest_density,
                    temperature_centi: cell.temperature.raw(),
                    channel_width_dm: cell.watercourse_width_dm,
                    lake_depth_mm: lake_depths
                        .get(usize::from(y) * 512 + usize::from(x))
                        .copied()
                        .flatten(),
                });
            }
        }
        let edges = [
            halo.states[AtlasNeighbor::North.index()] == NeighborState::WorldEdge,
            halo.states[AtlasNeighbor::East.index()] == NeighborState::WorldEdge,
            halo.states[AtlasNeighbor::South.index()] == NeighborState::WorldEdge,
            halo.states[AtlasNeighbor::West.index()] == NeighborState::WorldEdge,
        ];
        let mut terrain = Self {
            palette: vec![[0; 3]; SAMPLE_SIDE * SAMPLE_SIDE],
            light: vec![LIGHT_ONE; SAMPLE_SIDE * SAMPLE_SIDE],
            classes: vec![TerrainKind::Sea; SAMPLE_SIDE * SAMPLE_SIDE],
            heights: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
            wetness: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
            moisture: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
            forest_density: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
            temperature: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
            lake_surface: vec![i32::MIN; SAMPLE_SIDE * SAMPLE_SIDE],
            channel_width: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
            shore: vec![0; SAMPLE_SIDE * SAMPLE_SIDE],
            salt: Vec::new(),
            has_lake_depths,
            data_meanders: false,
            fine: None,
        };
        let start_x = if edges[3] { 0 } else { -1 };
        let end_x = if edges[1] { AREA - 1 } else { AREA };
        let start_y = if edges[0] { 0 } else { -1 };
        let end_y = if edges[2] { AREA - 1 } else { AREA };
        for y in start_y..=end_y {
            for x in start_x..=end_x {
                let source = source(&halo.context, x, y)?;
                let index = sample_index(x, y)?;
                terrain.classes[index] = source.class;
                terrain.heights[index] = source.height_mm;
                terrain.wetness[index] = source.wetness;
                terrain.moisture[index] = source.moisture;
                terrain.forest_density[index] = source.forest_density;
                terrain.temperature[index] = source.temperature_centi;
                terrain.channel_width[index] = source.channel_width_dm;
                if source.class == TerrainKind::Lake {
                    if let Some(depth) = source.lake_depth_mm {
                        terrain.lake_surface[index] = source
                            .height_mm
                            .saturating_add(i32::try_from(depth).unwrap_or(i32::MAX));
                    }
                }
                match source.class {
                    TerrainKind::Land => {
                        let (dx, dy) = gradient_numerators(&halo.context, x, y, edges)?;
                        terrain.palette[index] = land_material_ecology(
                            source.height_mm,
                            dx,
                            dy,
                            source.wetness,
                            source.moisture,
                            source.forest_density,
                        );
                        terrain.light[index] = relief_light(dx, dy);
                    }
                    TerrainKind::Sea => terrain.palette[index] = sea_palette(source.height_mm),
                    TerrainKind::Lake => {
                        terrain.palette[index] = source
                            .lake_depth_mm
                            .map(|depth| water_depth_palette(i64::from(depth)))
                            .unwrap_or(LAKE_FILL);
                    }
                }
            }
        }
        for y in -1..=512 {
            for x in -1..=512 {
                if (edges[3] && x == -1)
                    || (edges[1] && x == 512)
                    || (edges[0] && y == -1)
                    || (edges[2] && y == 512)
                {
                    let clamped_x = x.clamp(start_x, end_x);
                    let clamped_y = y.clamp(start_y, end_y);
                    let from = sample_index(clamped_x, clamped_y)?;
                    let to = sample_index(x, y)?;
                    terrain.palette[to] = terrain.palette[from];
                    terrain.light[to] = terrain.light[from];
                    terrain.classes[to] = terrain.classes[from];
                    terrain.heights[to] = terrain.heights[from];
                    terrain.wetness[to] = terrain.wetness[from];
                    terrain.moisture[to] = terrain.moisture[from];
                    terrain.forest_density[to] = terrain.forest_density[from];
                    terrain.temperature[to] = terrain.temperature[from];
                    terrain.lake_surface[to] = terrain.lake_surface[from];
                    terrain.channel_width[to] = terrain.channel_width[from];
                }
            }
        }
        Ok(terrain)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AxisKernel {
    Linear {
        low: i16,
        high_weight: u32,
        denominator: u32,
    },
    Box {
        start: u16,
        end: u16,
    },
}

pub(crate) fn axis_kernel(pixel: u32, pixels_in_area: u32) -> Result<AxisKernel, RenderError> {
    if !(1..=32_768).contains(&pixels_in_area) || pixel >= pixels_in_area {
        return Err(invalid("pixel or area width outside atlas limits"));
    }
    if pixels_in_area >= 512 {
        let denominator = i64::from(pixels_in_area) * 2;
        let numerator = (i64::from(pixel) * 2 + 1) * i64::from(AREA) - i64::from(pixels_in_area);
        let low = numerator.div_euclid(denominator);
        let high_weight = numerator.rem_euclid(denominator);
        Ok(AxisKernel::Linear {
            low: i16::try_from(low).map_err(|_| invalid("linear coordinate conversion"))?,
            high_weight: u32::try_from(high_weight)
                .map_err(|_| invalid("linear weight conversion"))?,
            denominator: u32::try_from(denominator)
                .map_err(|_| invalid("linear denominator conversion"))?,
        })
    } else {
        let start = pixel * 512 / pixels_in_area;
        let end = ((pixel + 1) * 512 / pixels_in_area).max(start + 1);
        Ok(AxisKernel::Box {
            start: u16::try_from(start).map_err(|_| invalid("box start conversion"))?,
            end: u16::try_from(end).map_err(|_| invalid("box end conversion"))?,
        })
    }
}

fn validate_kernel(kernel: AxisKernel) -> Result<(), RenderError> {
    match kernel {
        AxisKernel::Linear {
            low,
            high_weight,
            denominator,
        } if (-1..=511).contains(&low)
            && (1..=65_536).contains(&denominator)
            && high_weight < denominator =>
        {
            Ok(())
        }
        AxisKernel::Box { start, end } if start < end && end <= 512 => Ok(()),
        _ => Err(invalid("axis kernel outside atlas sample bounds")),
    }
}

fn context_index(x: i16, y: i16) -> Result<usize, RenderError> {
    if !(-2..=513).contains(&x) || !(-2..=513).contains(&y) {
        return Err(invalid("context coordinate outside -2..=513"));
    }
    let column =
        usize::try_from(i32::from(x) + 2).map_err(|_| invalid("context column conversion"))?;
    let row = usize::try_from(i32::from(y) + 2).map_err(|_| invalid("context row conversion"))?;
    Ok(row * CONTEXT_SIDE + column)
}

fn sample_index(x: i16, y: i16) -> Result<usize, RenderError> {
    if !(-1..=512).contains(&x) || !(-1..=512).contains(&y) {
        return Err(invalid("sample coordinate outside -1..=512"));
    }
    let column =
        usize::try_from(i32::from(x) + 1).map_err(|_| invalid("sample column conversion"))?;
    let row = usize::try_from(i32::from(y) + 1).map_err(|_| invalid("sample row conversion"))?;
    Ok(row * SAMPLE_SIDE + column)
}

fn source(context: &[Option<SourceSample>], x: i16, y: i16) -> Result<SourceSample, RenderError> {
    context[context_index(x, y)?].ok_or_else(|| invalid("missing saved neighbor sample"))
}

fn invalid(reason: &'static str) -> RenderError {
    RenderError::AtlasContext { reason }
}

#[cfg(test)]
mod tests;
