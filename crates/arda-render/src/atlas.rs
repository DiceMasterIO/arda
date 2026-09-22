//! Deterministic atlas palette and relief derived from saved area cells.

use arda_core::{AreaCells, CellCoord, TerrainKind};

use crate::{carto::LAKE_FILL, RenderError};

const AREA: i16 = 512;
const CONTEXT_SIDE: usize = 516;
const SAMPLE_SIDE: usize = 514;
const LIGHT_ONE: u16 = 4_096;
const CELL_DIAMETER_MM: i128 = 200_000;
const LIGHT_HORIZONTAL_Q15: i128 = 13_377;
const LIGHT_UP_Q15: i128 = 26_755;
const FLAT_LIGHT_Q15: i128 = LIGHT_UP_Q15;
const RELIEF_STRENGTH: i128 = 2_048;
const MIN_LIGHT: i128 = 2_048;
const MAX_LIGHT: i128 = 5_120;

const LAND_STOPS: [(i32, [u8; 3]); 7] = [
    (0, [104, 133, 68]),
    (200_000, [132, 151, 78]),
    (500_000, [165, 161, 97]),
    (900_000, [182, 162, 116]),
    (1_400_000, [167, 143, 109]),
    (2_000_000, [146, 143, 133]),
    (2_800_000, [232, 232, 226]),
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
}

impl AtlasTerrain {
    /// Derives deterministic colors and relief from a saved area and complete halo.
    ///
    /// # Errors
    /// Returns [`RenderError::AtlasContext`] for missing, duplicate, or
    /// topologically inconsistent neighbor context.
    pub fn new(cells: &AreaCells, mut halo: AtlasHalo) -> Result<Self, RenderError> {
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
                match source.class {
                    TerrainKind::Land => {
                        let (dx, dy) = gradient_numerators(&halo.context, x, y, edges)?;
                        terrain.palette[index] = land_palette(source.height_mm);
                        terrain.light[index] = relief_light(dx, dy);
                    }
                    TerrainKind::Sea => terrain.palette[index] = sea_palette(source.height_mm),
                    TerrainKind::Lake => terrain.palette[index] = LAKE_FILL,
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

    pub(crate) fn sample(
        &self,
        x: AxisKernel,
        y: AxisKernel,
        class: TerrainKind,
    ) -> Result<[u8; 3], RenderError> {
        validate_kernel(x)?;
        validate_kernel(y)?;
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

fn sea_palette(height_mm: i32) -> [u8; 3] {
    let depth = (-i64::from(height_mm)).max(0);
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
