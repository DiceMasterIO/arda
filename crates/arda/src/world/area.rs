//! One loaded area tile: its cells, saved objects and optional stored water.

use arda_core::{AreaCells, AreaObjects, Cell, Lake, LoadError, RiverSegment};

/// One loaded area tile.
pub struct Area {
    pub(super) cells: AreaCells,
    pub(super) objects: AreaObjects,
    pub(super) water: Option<arda_core::water::AreaWater>,
}

impl Area {
    /// Reads one cell.
    ///
    /// # Errors
    /// [`LoadError::OutOfRange`] when the coordinates leave the tile.
    pub fn cell(&self, x: u16, y: u16) -> Result<&Cell, LoadError> {
        let at = arda_core::CellCoord::new(x, y).ok_or(LoadError::OutOfRange {
            what: "cell",
            x: i32::from(x),
            y: i32::from(y),
            max_x: i32::from(arda_core::AREA_CELLS) - 1,
            max_y: i32::from(arda_core::AREA_CELLS) - 1,
        })?;
        Ok(self.cells.get(at))
    }

    /// River segments in this tile.
    #[must_use]
    pub fn rivers(&self) -> &[RiverSegment] {
        &self.objects.rivers
    }

    /// Lakes in this tile.
    #[must_use]
    pub fn lakes(&self) -> &[Lake] {
        &self.objects.lakes
    }

    /// Saved physical channel centreline edges touching this tile.
    #[must_use]
    pub fn channel_edges(&self) -> &[arda_core::hydrology::ChannelEdge] {
        &self.objects.channel_edges
    }

    /// The raw cell grid, for renderers.
    #[must_use]
    pub const fn cells(&self) -> &AreaCells {
        &self.cells
    }

    /// Transfers the cell grid to an uncached streaming overview export.
    pub(crate) fn into_cells(self) -> AreaCells {
        self.cells
    }

    /// Transfers the cells and saved objects to the Atlas overview exporter.
    pub(crate) fn into_render_parts(self) -> (AreaCells, AreaObjects) {
        (self.cells, self.objects)
    }

    /// The raw object lists, for renderers.
    #[must_use]
    pub const fn objects(&self) -> &AreaObjects {
        &self.objects
    }

    /// Stored river and lake forms (logic/02 §world-water): per river
    /// segment and lake, in the order of [`Self::rivers`] and
    /// [`Self::lakes`]. `None` for worlds whose recipe does not publish
    /// them.
    #[must_use]
    pub const fn water(&self) -> Option<&arda_core::water::AreaWater> {
        self.water.as_ref()
    }
}

impl std::fmt::Debug for Area {
    /// Summary only — an area holds 262,144 cells, so the full grid is never
    /// formatted.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Area")
            .field("rivers", &self.objects.rivers.len())
            .field("lakes", &self.objects.lakes.len())
            .finish_non_exhaustive()
    }
}
