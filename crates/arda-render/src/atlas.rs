//! Deterministic atlas palette and relief derived from saved area cells.

use arda_core::{AreaCells, AreaCoord, CellCoord, Lake, TerrainField, TerrainKind};

mod detail;
mod fine;
mod formed;
mod relief;
pub use fine::AtlasFineWorldBounds;
use fine::FineAtlas;
pub use relief::{
    formed_river_centreline, formed_river_rgb, formed_source_width, FormedRiverNetwork,
    ReliefGeometry,
};

use crate::channels::validated_lake_depths;

use crate::{carto::LAKE_FILL, RenderError};

const AREA: i16 = 512;
const CONTEXT_SIDE: usize = 516;
const SAMPLE_SIDE: usize = 514;
const LIGHT_ONE: u16 = 4_096;
const CELL_DIAMETER_MM: i128 = 200_000;
const LIGHT_HORIZONTAL_Q15: i128 = 13_377;
const LIGHT_UP_Q15: i128 = 26_755;
const FLAT_LIGHT_Q15: i128 = LIGHT_UP_Q15;
const RELIEF_STRENGTH: i128 = 3_450;
const MIN_LIGHT: i128 = 1_600;
const MAX_LIGHT: i128 = 5_350;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NeighborState {
    Unset,
    Copied,
    WorldEdge,
}

/// One of the eight saved areas adjacent to the target area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtlasNeighbor {
    /// Area directly north.
    North,
    /// Area diagonally northeast.
    NorthEast,
    /// Area directly east.
    East,
    /// Area diagonally southeast.
    SouthEast,
    /// Area directly south.
    South,
    /// Area diagonally southwest.
    SouthWest,
    /// Area directly west.
    West,
    /// Area diagonally northwest.
    NorthWest,
}

impl AtlasNeighbor {
    const fn index(self) -> usize {
        self as usize
    }
}

/// Nearest two saved-cell rows or columns from each adjacent area.
///
/// The context uses `-2..=513` local coordinates. A direction must be supplied
/// exactly once, either by copying its saved area or marking a world boundary.
#[derive(Debug)]
pub struct AtlasHalo {
    context: Vec<Option<SourceSample>>,
    states: [NeighborState; 8],
}

impl AtlasHalo {
    /// Creates an empty halo whose eight directions must be resolved before use.
    #[must_use]
    pub fn new() -> Self {
        Self {
            context: vec![None; CONTEXT_SIDE * CONTEXT_SIDE],
            states: [NeighborState::Unset; 8],
        }
    }

    /// Copies the nearest two saved-cell rows, columns, or corner from `cells`.
    ///
    /// `cells` must be exactly the named adjacent saved area; the caller must
    /// establish that identity from the world manifest.
    ///
    /// # Errors
    /// Returns [`RenderError::AtlasContext`] if this direction was already set.
    pub fn copy_neighbor(
        &mut self,
        direction: AtlasNeighbor,
        cells: &AreaCells,
    ) -> Result<(), RenderError> {
        self.copy_neighbor_with_lakes(direction, cells, &[])
    }

    /// Copies saved neighbor samples with validated lake depths.
    ///
    /// # Errors
    /// Returns invalid lake geometry or an already resolved direction.
    pub fn copy_neighbor_with_lakes(
        &mut self,
        direction: AtlasNeighbor,
        cells: &AreaCells,
        lakes: &[Lake],
    ) -> Result<(), RenderError> {
        if self.states[direction.index()] != NeighborState::Unset {
            return Err(invalid("neighbor direction already resolved"));
        }
        let (target_x, target_y, source_x, source_y, width, height) = match direction {
            AtlasNeighbor::North => (0, -2, 0, 510, 512, 2),
            AtlasNeighbor::NorthEast => (512, -2, 0, 510, 2, 2),
            AtlasNeighbor::East => (512, 0, 0, 0, 2, 512),
            AtlasNeighbor::SouthEast => (512, 512, 0, 0, 2, 2),
            AtlasNeighbor::South => (0, 512, 0, 0, 512, 2),
            AtlasNeighbor::SouthWest => (-2, 512, 510, 0, 2, 2),
            AtlasNeighbor::West => (-2, 0, 510, 0, 2, 512),
            AtlasNeighbor::NorthWest => (-2, -2, 510, 510, 2, 2),
        };
        let lake_depths = validated_lake_depths(cells, lakes)?;
        for dy in 0..height {
            for dx in 0..width {
                let source = CellCoord::new(source_x + dx, source_y + dy)
                    .ok_or_else(|| invalid("neighbor source coordinate outside area"))?;
                let target = context_index(
                    target_x
                        + i16::try_from(dx).map_err(|_| invalid("neighbor column conversion"))?,
                    target_y + i16::try_from(dy).map_err(|_| invalid("neighbor row conversion"))?,
                )?;
                let cell = cells.get(source);
                self.context[target] = Some(SourceSample {
                    height_mm: cell.height.raw(),
                    class: cell.terrain,
                    wetness: cell.wetness,
                    moisture: cell.moisture,
                    forest_density: cell.forest_density,
                    temperature_centi: cell.temperature.raw(),
                    channel_width_dm: cell.watercourse_width_dm,
                    lake_depth_mm: lake_depths
                        .get(usize::from(source.y()) * 512 + usize::from(source.x()))
                        .copied()
                        .flatten(),
                });
            }
        }
        self.states[direction.index()] = NeighborState::Copied;
        Ok(())
    }

    /// Marks a direction absent beyond the saved world's manifest bounds.
    ///
    /// This is valid only when manifest bounds prove that the named adjacent
    /// area does not exist. A missing or corrupt in-world area is an error.
    ///
    /// # Errors
    /// Returns [`RenderError::AtlasContext`] if this direction was already set.
    pub fn mark_world_edge(&mut self, direction: AtlasNeighbor) -> Result<(), RenderError> {
        if self.states[direction.index()] != NeighborState::Unset {
            return Err(invalid("neighbor direction already resolved"));
        }
        self.states[direction.index()] = NeighborState::WorldEdge;
        Ok(())
    }
}

impl Default for AtlasHalo {
    fn default() -> Self {
        Self::new()
    }
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

    /// Returns the derived color at one saved cell center.
    #[must_use]
    pub fn colour(&self, at: CellCoord) -> [u8; 3] {
        let x = match axis_kernel(u32::from(at.x()), 512) {
            Ok(kernel) => kernel,
            Err(_) => {
                unreachable!("bounds-checked CellCoord yields an exact-center x kernel at T=512")
            }
        };
        let y = match axis_kernel(u32::from(at.y()), 512) {
            Ok(kernel) => kernel,
            Err(_) => {
                unreachable!("bounds-checked CellCoord yields an exact-center y kernel at T=512")
            }
        };
        let index = match sample_index(
            i16::try_from(at.x()).unwrap_or_else(|_| unreachable!("CellCoord x fits i16")),
            i16::try_from(at.y()).unwrap_or_else(|_| unreachable!("CellCoord y fits i16")),
        ) {
            Ok(index) => index,
            Err(_) => unreachable!("bounds-checked CellCoord lies inside atlas sample grid"),
        };
        match self.sample(x, y, self.classes[index]) {
            Ok(rgb) => rgb,
            Err(_) => unreachable!("exact-center kernel retains the center's stored terrain class"),
        }
    }

    /// Whether recipe-5 formed shading and thalweg rivers apply.
    pub(crate) fn is_formed(&self) -> bool {
        self.fine.as_ref().is_some_and(FineAtlas::is_formed)
    }

    /// Recipe-5 river vertex offset from the saved cell centre, micrometres.
    pub(crate) fn channel_offset_um(&self, node: arda_core::GlobalCell) -> (i64, i64) {
        let Some(fine) = self.fine.as_ref() else {
            return (0, 0);
        };
        let (x, y) = (
            i128::from(node.x) * 100_000_000,
            i128::from(node.y) * 100_000_000,
        );
        let (tx, ty) = fine.thalweg_offset_um(x, y).unwrap_or((0, 0));
        let width = fine
            .local_cell(x, y)
            .and_then(|(cx, cy)| sample_index(cx, cy).ok())
            .map_or(0, |i| self.channel_width[i]);
        let (mx, my) = if self.data_meanders {
            (0, 0)
        } else {
            fine.meander_offset_um(x, y, width).unwrap_or((0, 0))
        };
        (tx + mx, ty + my)
    }

    /// Selects the formed look for the world's fine-terrain recipe
    /// (logic/04 §atlas-formed recipes): recipe 6 and later draw the v0.2
    /// look (palette, surface detail, curved rivers); recipe 5 renders
    /// exactly as v0.1 drew it. Constructors default to recipe 6.
    #[must_use]
    pub fn with_recipe(mut self, recipe_version: u16) -> Self {
        if let Some(fine) = self.fine.as_mut() {
            fine.set_recipe(recipe_version);
        }
        self
    }

    /// Whether this formed terrain draws the recipe-6 look.
    pub(crate) fn is_formed_v6(&self) -> bool {
        self.fine
            .as_ref()
            .is_some_and(|f| f.is_formed() && f.is_v6())
    }

    /// Paints recipe-5 shores from the stored shore layer (logic/04
    /// §atlas-formed shore, goals 16 and 31) instead of the height-and-slope
    /// guess: beaches, shingle, cliffs, rocky shores, marsh, tidal flats and
    /// estuaries each get their own material.
    #[must_use]
    pub fn with_shore(mut self, layer: &arda_core::ShoreLayer, area: AreaCoord) -> Self {
        let (gx0, gy0) = (i64::from(area.x) * 512, i64::from(area.y) * 512);
        for y in -1..=AREA {
            for x in -1..=AREA {
                let (gx, gy) = (gx0 + i64::from(x), gy0 + i64::from(y));
                let (Ok(gx), Ok(gy), Ok(i)) =
                    (u32::try_from(gx), u32::try_from(gy), sample_index(x, y))
                else {
                    continue;
                };
                self.shore[i] = layer.class_at(gx, gy) as u8;
            }
        }
        self
    }

    pub(crate) fn has_lake_depths(&self) -> bool {
        self.has_lake_depths
    }

    pub(crate) fn sample(
        &self,
        x: AxisKernel,
        y: AxisKernel,
        class: TerrainKind,
    ) -> Result<[u8; 3], RenderError> {
        validate_kernel(x)?;
        validate_kernel(y)?;
        if let Some(fine) = self.fine.as_ref().filter(|f| f.is_formed()) {
            {
                return fine.sample_formed(
                    x,
                    y,
                    &fine::SavedFields {
                        classes: &self.classes,
                        wetness: &self.wetness,
                        moisture: &self.moisture,
                        forest_density: &self.forest_density,
                        temperature: &self.temperature,
                        heights: &self.heights,
                        lake_surface: &self.lake_surface,
                        shore: &self.shore,
                    },
                );
            }
        }
        if class == TerrainKind::Land {
            if let Some(fine) = &self.fine {
                return fine.sample(
                    x,
                    y,
                    &self.classes,
                    &self.wetness,
                    &self.moisture,
                    &self.forest_density,
                );
            }
        }
        let mut palette_sum = [0_u64; 3];
        let mut light_sum = 0_u64;
        let mut weight_sum = 0_u64;
        let mut visit = |x: i16, y: i16, weight: u64| -> Result<(), RenderError> {
            let index = sample_index(x, y)?;
            if self.classes[index] == class {
                for (sum, channel) in palette_sum.iter_mut().zip(self.palette[index]) {
                    *sum += u64::from(channel) * weight;
                }
                light_sum += u64::from(self.light[index]) * weight;
                weight_sum += weight;
            }
            Ok(())
        };
        match (x, y) {
            (
                AxisKernel::Linear {
                    low: xl,
                    high_weight: xh,
                    denominator: xd,
                },
                AxisKernel::Linear {
                    low: yl,
                    high_weight: yh,
                    denominator: yd,
                },
            ) => {
                for (cx, wx) in [(xl, xd - xh), (xl + 1, xh)] {
                    for (cy, wy) in [(yl, yd - yh), (yl + 1, yh)] {
                        visit(cx, cy, u64::from(wx) * u64::from(wy))?;
                    }
                }
            }
            (
                AxisKernel::Linear {
                    low,
                    high_weight,
                    denominator,
                },
                AxisKernel::Box { start, end },
            ) => {
                for (cx, wx) in [(low, denominator - high_weight), (low + 1, high_weight)] {
                    for cy in start..end {
                        visit(
                            cx,
                            i16::try_from(cy).map_err(|_| invalid("box row conversion"))?,
                            u64::from(wx),
                        )?;
                    }
                }
            }
            (
                AxisKernel::Box { start, end },
                AxisKernel::Linear {
                    low,
                    high_weight,
                    denominator,
                },
            ) => {
                for cx in start..end {
                    for (cy, wy) in [(low, denominator - high_weight), (low + 1, high_weight)] {
                        visit(
                            i16::try_from(cx).map_err(|_| invalid("box column conversion"))?,
                            cy,
                            u64::from(wy),
                        )?;
                    }
                }
            }
            (AxisKernel::Box { start: xs, end: xe }, AxisKernel::Box { start: ys, end: ye }) => {
                for cx in xs..xe {
                    for cy in ys..ye {
                        visit(
                            i16::try_from(cx).map_err(|_| invalid("box column conversion"))?,
                            i16::try_from(cy).map_err(|_| invalid("box row conversion"))?,
                            1,
                        )?;
                    }
                }
            }
        }
        if weight_sum == 0 {
            return Err(invalid("no sample has the owning terrain class"));
        }
        let mut palette = [0; 3];
        for (channel, sum) in palette.iter_mut().zip(palette_sum) {
            *channel = u8::try_from((sum + weight_sum / 2) / weight_sum)
                .map_err(|_| invalid("palette average outside RGB bounds"))?;
        }
        let light = u16::try_from((light_sum + weight_sum / 2) / weight_sum)
            .map_err(|_| invalid("light average outside Q12 bounds"))?;
        Ok(modulate(palette, light))
    }

    /// Resolves an unambiguous shoreline from saved height samples.
    pub(crate) fn contour_class(
        &self,
        x: AxisKernel,
        y: AxisKernel,
        owner: CellCoord,
        saved: TerrainKind,
    ) -> Result<TerrainKind, RenderError> {
        let (
            AxisKernel::Linear {
                low: xl,
                high_weight: xh,
                denominator: xd,
            },
            AxisKernel::Linear {
                low: yl,
                high_weight: yh,
                denominator: yd,
            },
        ) = (x, y)
        else {
            return Ok(saved);
        };
        if saved == TerrainKind::Lake || (xh == 0 && yh == 0) {
            return Ok(saved);
        }
        let ox = i16::try_from(owner.x()).map_err(|_| invalid("owner column conversion"))?;
        let oy = i16::try_from(owner.y()).map_err(|_| invalid("owner row conversion"))?;
        if self.classes[sample_index(ox, oy)?] != saved {
            return Err(invalid("atlas terrain disagrees with saved cell class"));
        }
        // A one-cell-wide saved island or strait must remain visible even when
        // neighboring heights have much larger magnitudes than its own.
        for (ax, ay, bx, by) in [(ox - 1, oy, ox + 1, oy), (ox, oy - 1, ox, oy + 1)] {
            let a = self.classes[sample_index(ax, ay)?];
            let b = self.classes[sample_index(bx, by)?];
            if a == b && a != saved && a != TerrainKind::Lake {
                return Ok(saved);
            }
        }
        let mut classes = [TerrainKind::Sea; 4];
        // Four i32 heights times Q16 axis weights fit i128 at the 32K limit.
        let mut height_sum = 0_i128;
        for (i, (cx, cy, weight)) in [
            (xl, yl, i128::from(xd - xh) * i128::from(yd - yh)),
            (xl + 1, yl, i128::from(xh) * i128::from(yd - yh)),
            (xl, yl + 1, i128::from(xd - xh) * i128::from(yh)),
            (xl + 1, yl + 1, i128::from(xh) * i128::from(yh)),
        ]
        .into_iter()
        .enumerate()
        {
            let index = sample_index(cx, cy)?;
            let class = self.classes[index];
            if class == TerrainKind::Lake {
                return Ok(saved);
            }
            classes[i] = class;
            let height = i128::from(self.heights[index]);
            let directed = if class == TerrainKind::Land {
                height.max(1)
            } else {
                height.min(-1)
            };
            height_sum += directed * weight;
        }
        if classes[0] == classes[3] && classes[1] == classes[2] && classes[0] != classes[1] {
            return Ok(saved);
        }
        Ok(match height_sum.cmp(&0) {
            std::cmp::Ordering::Greater => TerrainKind::Land,
            std::cmp::Ordering::Less => TerrainKind::Sea,
            std::cmp::Ordering::Equal => saved,
        })
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

fn gradient_numerators(
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

fn relief_light(dx: i64, dy: i64) -> u16 {
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

fn interpolate(low: [u8; 3], high: [u8; 3], offset: i64, span: i64) -> [u8; 3] {
    std::array::from_fn(|channel| {
        let value = (i64::from(low[channel]) * (span - offset)
            + i64::from(high[channel]) * offset
            + span / 2)
            / span;
        u8::try_from(value).unwrap_or_else(|_| unreachable!("convex RGB blend fits u8"))
    })
}

fn land_palette(height_mm: i32) -> [u8; 3] {
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
fn land_material(height_mm: i32, dx: i64, dy: i64, wetness: u8) -> [u8; 3] {
    land_material_ecology(height_mm, dx, dy, wetness, 0, 0)
}

/// Climate and canopy affect the ground before rock and snow override it.
/// Old saved worlds have both ecology fields zero and keep their original
/// palette byte for byte.
fn land_material_ecology(
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
fn wetness_tint(base: [u8; 3], wetness: u8) -> [u8; 3] {
    let wet = i128::from(wetness);
    let weight_q12 = wet * 2_048 / (wet + 12);
    blend(base, [81, 126, 73], weight_q12)
}

fn blend(a: [u8; 3], b: [u8; 3], weight_q12: i128) -> [u8; 3] {
    debug_assert!((0..=4_096).contains(&weight_q12));
    std::array::from_fn(|channel| {
        let value = (i128::from(a[channel]) * (4_096 - weight_q12)
            + i128::from(b[channel]) * weight_q12
            + 2_048)
            / 4_096;
        u8::try_from(value).unwrap_or_else(|_| unreachable!("convex RGB blend fits u8"))
    })
}

fn sea_palette(height_mm: i32) -> [u8; 3] {
    water_depth_palette((-i64::from(height_mm)).max(0))
}

fn water_depth_palette(depth: i64) -> [u8; 3] {
    for pair in SEA_STOPS.windows(2) {
        let (lower, low) = pair[0];
        let (upper, high) = pair[1];
        if depth < upper {
            return interpolate(low, high, depth - lower, upper - lower);
        }
    }
    SEA_STOPS[SEA_STOPS.len() - 1].1
}

fn modulate(palette: [u8; 3], light: u16) -> [u8; 3] {
    std::array::from_fn(|channel| {
        let value = (u32::from(palette[channel]) * u32::from(light) + u32::from(LIGHT_ONE) / 2)
            / u32::from(LIGHT_ONE);
        u8::try_from(value.min(255))
            .unwrap_or_else(|_| unreachable!("clamped RGB modulation fits u8"))
    })
}

fn invalid(reason: &'static str) -> RenderError {
    RenderError::AtlasContext { reason }
}

#[cfg(test)]
mod tests;
