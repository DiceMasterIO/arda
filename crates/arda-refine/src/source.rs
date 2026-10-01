//! What the refiner reads from a world: cells, lakes, channel edges and the
//! fine terrain lattice, addressed by global coordinates only.
//!
//! [`WorldSource`](crate::WorldSource) reads a stored world; [`GridSource`]
//! holds the same data in memory for synthetic fixtures.

use crate::error::RefineError;
use arda::{Cell, TerrainKind};
use arda_core::water::PanKind;
use std::collections::BTreeMap;

/// A global 100 m cell coordinate. Signed so neighbourhoods may step past
/// the world edge; sources clamp such queries to the nearest real cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellKey {
    /// Column.
    pub x: i64,
    /// Row.
    pub y: i64,
}

impl CellKey {
    /// Builds a key.
    #[must_use]
    pub const fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }

    /// Offsets a key.
    #[must_use]
    pub const fn offset(self, dx: i64, dy: i64) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }
}

/// One saved channel centreline step between neighbouring cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    /// Upstream cell.
    pub from: CellKey,
    /// Downstream cell.
    pub to: CellKey,
    /// Channel width at the upstream end, decimetres.
    pub from_width_dm: u32,
    /// Channel width at the downstream end, decimetres.
    pub to_width_dm: u32,
    /// Mean discharge, thousandths of a cubic metre per second.
    pub discharge_milli: u64,
}

/// A lake as seen from one of its cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LakeInfo {
    /// Water surface, millimetres above sea level.
    pub surface_mm: i32,
    /// Greatest depth, millimetres.
    pub depth_mm: u32,
}

/// Read access to a world, by global coordinates.
pub trait Source {
    /// The world seed.
    fn seed(&self) -> u64;
    /// World extent in cells.
    fn cells_wide_high(&self) -> (i64, i64);
    /// The cell at `at`, clamped into the world.
    ///
    /// # Errors
    /// A layer failed to load.
    fn cell(&self, at: CellKey) -> Result<Cell, RefineError>;
    /// The lake covering `at`, if any.
    ///
    /// # Errors
    /// A layer failed to load.
    fn lake(&self, at: CellKey) -> Result<Option<LakeInfo>, RefineError>;
    /// Channel edges starting or ending in `at`, sorted and deduplicated.
    ///
    /// # Errors
    /// A layer failed to load.
    fn edges_touching(&self, at: CellKey) -> Result<Vec<Edge>, RefineError>;
    /// The fine terrain lattice sample `(kx, ky)` (39.0625 m spacing,
    /// origin at cell `(0, 0)`'s centre in the I1 frame,
    /// `arda_core::FINE_FRAME_OFFSET_UM`) in millimetres, clamped into the
    /// lattice; `None` when the world has no fine terrain.
    ///
    /// # Errors
    /// The terrain file failed to read.
    fn fine_mm(&self, kx: i64, ky: i64) -> Result<Option<i32>, RefineError>;

    /// The dry playa surface of `at` (recipe-7 arid basins: salt crust or
    /// mudflat), if any. Worlds without stored pans answer `Ok(None)`.
    ///
    /// Required, with no default: a wrapping source that forgot to forward
    /// it would silently answer "no pan" (the `SharedSource` bug of v0.4).
    ///
    /// ```compile_fail
    /// use arda_refine::source::{Edge, LakeInfo};
    /// use arda_refine::{CellKey, RefineError, Source};
    ///
    /// /// A wrapper that forwards everything but the playa query.
    /// struct Wrapper(arda_refine::source::GridSource);
    ///
    /// impl Source for Wrapper {
    ///     fn seed(&self) -> u64 { self.0.seed() }
    ///     fn cells_wide_high(&self) -> (i64, i64) { self.0.cells_wide_high() }
    ///     fn cell(&self, at: CellKey) -> Result<arda::Cell, RefineError> { self.0.cell(at) }
    ///     fn lake(&self, at: CellKey) -> Result<Option<LakeInfo>, RefineError> { self.0.lake(at) }
    ///     fn edges_touching(&self, at: CellKey) -> Result<Vec<Edge>, RefineError> {
    ///         self.0.edges_touching(at)
    ///     }
    ///     fn fine_mm(&self, kx: i64, ky: i64) -> Result<Option<i32>, RefineError> {
    ///         self.0.fine_mm(kx, ky)
    ///     }
    /// }
    /// ```
    ///
    /// # Errors
    /// A layer failed to load.
    fn pan(&self, at: CellKey) -> Result<Option<PanKind>, RefineError>;

    /// Clamps a key into the world.
    fn clamp(&self, at: CellKey) -> CellKey {
        let (w, h) = self.cells_wide_high();
        CellKey::new(at.x.clamp(0, w - 1), at.y.clamp(0, h - 1))
    }
}

/// An in-memory world for synthetic fixtures and tests.
#[derive(Debug, Clone)]
pub struct GridSource {
    seed: u64,
    width: i64,
    height: i64,
    cells: Vec<Cell>,
    lakes: BTreeMap<CellKey, LakeInfo>,
    edges: BTreeMap<CellKey, Vec<Edge>>,
    fine: Option<(i64, i64, Vec<i32>)>,
    pans: BTreeMap<CellKey, PanKind>,
}

impl GridSource {
    /// A `width × height` world filled with one cell.
    #[must_use]
    pub fn new(seed: u64, width: u16, height: u16, fill: Cell) -> Self {
        let (w, h) = (i64::from(width.max(1)), i64::from(height.max(1)));
        let n = usize::from(width.max(1)) * usize::from(height.max(1));
        Self {
            seed,
            width: w,
            height: h,
            cells: vec![fill; n],
            lakes: BTreeMap::new(),
            edges: BTreeMap::new(),
            fine: None,
            pans: BTreeMap::new(),
        }
    }

    fn index(&self, at: CellKey) -> usize {
        let c = self.clamp(at);
        usize::try_from(c.y * self.width + c.x).unwrap_or(0)
    }

    /// Replaces one cell; keys outside the world are ignored.
    pub fn set(&mut self, at: CellKey, cell: Cell) {
        if self.clamp(at) == at {
            let i = self.index(at);
            if let Some(slot) = self.cells.get_mut(i) {
                *slot = cell;
            }
        }
    }

    /// Mutable access to one cell (clamped).
    pub fn cell_mut(&mut self, at: CellKey) -> Option<&mut Cell> {
        let i = self.index(at);
        self.cells.get_mut(i)
    }

    /// Marks a cell as lake with the given surface and depth.
    pub fn set_lake(&mut self, at: CellKey, info: LakeInfo) {
        if let Some(c) = self.cell_mut(at) {
            c.terrain = TerrainKind::Lake;
        }
        self.lakes.insert(at, info);
    }

    /// Adds a channel edge between neighbouring cells.
    pub fn add_edge(&mut self, edge: Edge) {
        for key in [edge.from, edge.to] {
            let list = self.edges.entry(key).or_default();
            if !list.contains(&edge) {
                list.push(edge);
                list.sort();
            }
        }
    }

    /// Marks a land cell as a dry playa surface.
    pub fn set_pan(&mut self, at: CellKey, kind: PanKind) {
        self.pans.insert(at, kind);
    }

    /// Installs a fine terrain lattice of `w × h` samples.
    pub fn set_fine(&mut self, w: u32, h: u32, samples: Vec<i32>) {
        self.fine = Some((i64::from(w), i64::from(h), samples));
    }
}

impl Source for GridSource {
    fn seed(&self) -> u64 {
        self.seed
    }

    fn cells_wide_high(&self) -> (i64, i64) {
        (self.width, self.height)
    }

    fn cell(&self, at: CellKey) -> Result<Cell, RefineError> {
        Ok(self.cells.get(self.index(at)).copied().unwrap_or_default())
    }

    fn lake(&self, at: CellKey) -> Result<Option<LakeInfo>, RefineError> {
        Ok(self.lakes.get(&self.clamp(at)).copied())
    }

    fn pan(&self, at: CellKey) -> Result<Option<PanKind>, RefineError> {
        Ok(self.pans.get(&self.clamp(at)).copied())
    }

    fn edges_touching(&self, at: CellKey) -> Result<Vec<Edge>, RefineError> {
        Ok(self.edges.get(&at).cloned().unwrap_or_default())
    }

    fn fine_mm(&self, kx: i64, ky: i64) -> Result<Option<i32>, RefineError> {
        let Some((w, h, samples)) = &self.fine else {
            return Ok(None);
        };
        let x = kx.clamp(0, w - 1);
        let y = ky.clamp(0, h - 1);
        Ok(usize::try_from(y * w + x)
            .ok()
            .and_then(|i| samples.get(i).copied()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// The `compile_fail` example on [`Source::pan`] with the one missing
    /// method added: it compiles, so that example fails only for the pan.
    struct Wrapper(GridSource);

    impl Source for Wrapper {
        fn seed(&self) -> u64 {
            self.0.seed()
        }
        fn cells_wide_high(&self) -> (i64, i64) {
            self.0.cells_wide_high()
        }
        fn cell(&self, at: CellKey) -> Result<Cell, RefineError> {
            self.0.cell(at)
        }
        fn lake(&self, at: CellKey) -> Result<Option<LakeInfo>, RefineError> {
            self.0.lake(at)
        }
        fn edges_touching(&self, at: CellKey) -> Result<Vec<Edge>, RefineError> {
            self.0.edges_touching(at)
        }
        fn fine_mm(&self, kx: i64, ky: i64) -> Result<Option<i32>, RefineError> {
            self.0.fine_mm(kx, ky)
        }
        fn pan(&self, at: CellKey) -> Result<Option<PanKind>, RefineError> {
            self.0.pan(at)
        }
    }

    #[test]
    fn a_wrapper_must_forward_the_playa_query() {
        let mut grid = GridSource::new(3, 3, 3, Cell::default());
        grid.set_pan(CellKey::new(2, 1), PanKind::SaltCrust);
        let w = Wrapper(grid);
        assert_eq!(w.pan(CellKey::new(2, 1)).unwrap(), Some(PanKind::SaltCrust));
        assert_eq!(w.pan(CellKey::new(0, 0)).unwrap(), None);
    }
}
