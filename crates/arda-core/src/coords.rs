//! The two-grid coordinate system (`01-architecture.md`).
//!
//! Distinct newtypes per grid so a cell index can never be used as a
//! square index — `code-prefs.md` §Q1 bans bare integers that cross grids.

/// Cells along one edge of an area tile.
pub const AREA_CELLS: u16 = 512;
/// Ground size of one area cell, in metres.
pub const CELL_SIZE_M: i32 = 100;
/// Tactical squares along one edge of a cell.
pub const BLOCK_SQUARES: u16 = 64;
/// Ground size of one tactical square, in micrometres: a 100 m cell is
/// exactly [`BLOCK_SQUARES`] squares, so one square samples 100/64 m =
/// 1.5625 m of the world. Play calls a square five feet (1.524 m); the
/// 2.5 % horizontal compression is deliberate (logic/09 §square-frame).
/// The one statement of the square size: the old five-foot
/// `SQUARE_SIZE_MM = 1524` was used by nothing and invited misuse (review
/// 3.4-11).
pub const SQUARE_SIZE_UM: i64 = CELL_SIZE_M as i64 * 1_000_000 / BLOCK_SQUARES as i64;

/// Index of a 51.2 km area tile within the continent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AreaCoord {
    /// Tile column.
    pub x: i32,
    /// Tile row.
    pub y: i32,
}

impl AreaCoord {
    /// Builds a tile index.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Directory name for this tile, as `<ax>_<ay>` (`mockup/02`).
    #[must_use]
    pub fn dir_name(self) -> String {
        format!("{:02}_{:02}", self.x, self.y)
    }
}

/// Index of a 100 m cell within one area tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellCoord {
    x: u16,
    y: u16,
}

impl CellCoord {
    /// Builds a cell index, or `None` when either axis is outside the tile.
    #[must_use]
    pub const fn new(x: u16, y: u16) -> Option<Self> {
        if x < AREA_CELLS && y < AREA_CELLS {
            Some(Self { x, y })
        } else {
            None
        }
    }

    /// Cell column.
    #[must_use]
    pub const fn x(self) -> u16 {
        self.x
    }

    /// Cell row.
    #[must_use]
    pub const fn y(self) -> u16 {
        self.y
    }

    /// Row-major offset into a `512 * 512` array.
    #[must_use]
    pub const fn index(self) -> usize {
        self.y as usize * AREA_CELLS as usize + self.x as usize
    }
}

/// Index of a 1 km cell within the continent working grid.
///
/// Continent axes are capped at 4,000 km (`config.rs`), so `u16` holds
/// any coordinate. Unlike `CellCoord` there is no fixed upper bound to
/// validate against here: the grid's own dimensions travel with the
/// overview layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KmCoord {
    /// Column, kilometres from the west edge.
    pub x: u16,
    /// Row, kilometres from the north edge.
    pub y: u16,
}

impl KmCoord {
    /// Builds a continent-cell index.
    #[must_use]
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cell_is_exactly_sixty_four_squares() {
        assert_eq!(SQUARE_SIZE_UM, 1_562_500);
        assert_eq!(
            SQUARE_SIZE_UM * i64::from(BLOCK_SQUARES),
            i64::from(CELL_SIZE_M) * 1_000_000
        );
    }

    #[test]
    fn cell_coord_rejects_out_of_bounds() {
        assert!(CellCoord::new(511, 511).is_some());
        assert!(CellCoord::new(512, 0).is_none());
        assert!(CellCoord::new(0, 512).is_none());
    }

    #[test]
    fn cell_coord_index_is_row_major() {
        let c = CellCoord::new(3, 2).unwrap();
        assert_eq!(c.index(), 2 * 512 + 3);
    }
}

/// Absolute coordinate on the shared 100 m terrain grid.
///
/// Domain-specific bounds are checked against `HydrologyDomain`; this type
/// keeps global coordinates distinct from area-local and kilometre indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalCell {
    /// Column from the modeled domain's west edge.
    pub x: u32,
    /// Row from the modeled domain's north edge.
    pub y: u32,
}
