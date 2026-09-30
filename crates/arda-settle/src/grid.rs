//! The compact whole-world working raster every stage reads.
//!
//! One entry per 100 m cell, structure-of-arrays, stitched from every area
//! tile. It carries only the `Cell` fields this stage uses (about 26 bytes a
//! cell), so a full 500 × 1000 km world fits well inside the 16 GiB budget.

use crate::error::SettleError;
use crate::num::{iu, ui};
use arda::{Cover, TerrainKind};

/// Metres along one cell edge.
pub const CELL_M: i64 = 100;

/// Default memory ceiling (goal-prompt §8).
pub const MEMORY_BUDGET: u64 = 16 * 1024 * 1024 * 1024;

/// Working bytes per cell across every raster the stage allocates: the grid
/// itself, derived fields, tags, land use, realm ids and route scratch.
pub const BYTES_PER_CELL: u64 = 128;

/// The stitched per-cell raster.
#[derive(Debug, Clone, Default)]
pub struct Grid {
    /// Cells across.
    pub width: usize,
    /// Cells down.
    pub height: usize,
    /// Elevation in millimetres.
    pub height_mm: Vec<i32>,
    /// Sea, land, or lake.
    pub terrain: Vec<TerrainKind>,
    /// Ground cover.
    pub cover: Vec<Cover>,
    /// Slope in thousandths of a degree.
    pub slope_md: Vec<u16>,
    /// Downslope bearing in degrees.
    pub aspect_deg: Vec<u16>,
    /// Mean annual temperature, hundredths of a degree.
    pub temp_cc: Vec<i16>,
    /// Mean annual rainfall in millimetres.
    pub rain_mm: Vec<u16>,
    /// Soil moisture, 0–255.
    pub moisture: Vec<u8>,
    /// Canopy closure, 0–255.
    pub forest: Vec<u8>,
    /// Upstream cells draining through each cell.
    pub drainage: Vec<u32>,
    /// Strahler order; 0 off a watercourse.
    pub order: Vec<u8>,
    /// Channel width in decimetres.
    pub width_dm: Vec<u32>,
    /// Height above the nearest downstream channel, decimetres.
    pub har_dm: Vec<u16>,
    /// Confluence cells of two sizeable rivers, from the saved segments.
    pub confluences: Vec<usize>,
}

impl Grid {
    /// Allocates a grid of sea cells, refusing sizes over `budget`.
    ///
    /// # Errors
    /// [`SettleError::Budget`] when the estimate exceeds the budget and
    /// [`SettleError::Reserve`] when the allocator refuses.
    pub fn sea(width: usize, height: usize, budget: u64) -> Result<Self, SettleError> {
        let n = width
            .checked_mul(height)
            .ok_or(SettleError::Dimensions("cell count overflows"))?;
        let needed = u64::try_from(n)
            .unwrap_or(u64::MAX)
            .saturating_mul(BYTES_PER_CELL);
        if needed > budget {
            return Err(SettleError::Budget { needed, budget });
        }
        Ok(Self {
            width,
            height,
            height_mm: filled(n, -1000, "heights")?,
            terrain: filled(n, TerrainKind::Sea, "terrain")?,
            cover: filled(n, Cover::Bare, "cover")?,
            slope_md: filled(n, 0, "slope")?,
            aspect_deg: filled(n, 0, "aspect")?,
            temp_cc: filled(n, 1000, "temperature")?,
            rain_mm: filled(n, 800, "rainfall")?,
            moisture: filled(n, 128, "moisture")?,
            forest: filled(n, 0, "forest")?,
            drainage: filled(n, 1, "drainage")?,
            order: filled(n, 0, "order")?,
            width_dm: filled(n, 0, "width")?,
            har_dm: filled(n, 0, "height above river")?,
            confluences: Vec::new(),
        })
    }

    /// Number of cells.
    #[must_use]
    pub fn len(&self) -> usize {
        self.width * self.height
    }

    /// Whether the grid is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Row-major index of `(x, y)`, `None` off the grid.
    #[must_use]
    pub fn at(&self, x: i64, y: i64) -> Option<usize> {
        if x < 0 || y < 0 || x >= ui(self.width) || y >= ui(self.height) {
            None
        } else {
            Some(iu(y) * self.width + iu(x))
        }
    }

    /// Column and row of an index.
    #[must_use]
    pub fn xy(&self, i: usize) -> (i64, i64) {
        (ui(i % self.width), ui(i / self.width))
    }

    /// Dry land.
    #[must_use]
    pub fn is_land(&self, i: usize) -> bool {
        self.terrain[i] == TerrainKind::Land
    }

    /// Sea or lake surface.
    #[must_use]
    pub fn is_water(&self, i: usize) -> bool {
        self.terrain[i] != TerrainKind::Land
    }

    /// A land cell carrying a channel.
    #[must_use]
    pub fn is_watercourse(&self, i: usize) -> bool {
        self.is_land(i) && self.order[i] > 0
    }

    /// Channel width in whole metres.
    #[must_use]
    pub fn width_m(&self, i: usize) -> u32 {
        self.width_dm[i] / 10
    }

    /// Slope in whole degrees, rounded down.
    #[must_use]
    pub fn slope_deg(&self, i: usize) -> u16 {
        self.slope_md[i] / 1000
    }

    /// The eight neighbours of `i` that lie on the grid, with their offsets.
    pub fn neighbours8(&self, i: usize) -> impl Iterator<Item = (usize, i64, i64)> + '_ {
        let (x, y) = self.xy(i);
        OFFSETS8
            .iter()
            .filter_map(move |&(dx, dy)| self.at(x + dx, y + dy).map(|j| (j, dx, dy)))
    }

    /// The four orthogonal neighbours of `i` on the grid.
    pub fn neighbours4(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        let (x, y) = self.xy(i);
        OFFSETS8
            .iter()
            .filter(|(dx, dy)| dx.abs() + dy.abs() == 1)
            .filter_map(move |&(dx, dy)| self.at(x + dx, y + dy))
    }

    /// Land touching the sea on one of its four sides (coast is not stored,
    /// so it is derived from sea neighbours).
    #[must_use]
    pub fn is_coast(&self, i: usize) -> bool {
        self.is_land(i)
            && self
                .neighbours4(i)
                .any(|j| self.terrain[j] == TerrainKind::Sea)
    }

    /// Count of land cells.
    #[must_use]
    pub fn land_cells(&self) -> usize {
        (0..self.len()).filter(|&i| self.is_land(i)).count()
    }
}

/// Neighbour offsets, orthogonal first, in a fixed order for determinism.
pub const OFFSETS8: [(i64, i64); 8] = [
    (1, 0),
    (0, 1),
    (-1, 0),
    (0, -1),
    (1, 1),
    (-1, 1),
    (-1, -1),
    (1, -1),
];

/// Allocates `n` copies of `v`, reporting a refused reservation.
///
/// # Errors
/// [`SettleError::Reserve`] when the allocator refuses.
pub fn filled<T: Clone>(n: usize, v: T, what: &'static str) -> Result<Vec<T>, SettleError> {
    let mut out = Vec::new();
    out.try_reserve_exact(n).map_err(|_| SettleError::Reserve {
        what,
        bytes: u64::try_from(n.saturating_mul(std::mem::size_of::<T>())).unwrap_or(u64::MAX),
    })?;
    out.resize(n, v);
    Ok(out)
}
