//! The two-grid coordinate system (`01-architecture.md`).
//!
//! Distinct newtypes per grid so a cell index can never be used as a
//! square index — `code-prefs.md` §Q1 bans bare integers that cross grids.

/// Cells along one edge of an area tile.
pub const AREA_CELLS: u16 = 512;
/// Squares along one edge of a block.
pub const BLOCK_SQUARES: u8 = 64;
/// Ground size of one area cell, in metres.
pub const CELL_SIZE_M: i32 = 100;
/// Ground size of one tactical square, in millimetres (five feet exactly).
pub const SQUARE_SIZE_MM: i32 = 1524;

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

/// Index of a five-foot square within one block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SquareCoord {
    x: u8,
    y: u8,
}

impl SquareCoord {
    /// Builds a square index, or `None` when either axis is outside the block.
    #[must_use]
    pub const fn new(x: u8, y: u8) -> Option<Self> {
        if x < BLOCK_SQUARES && y < BLOCK_SQUARES {
            Some(Self { x, y })
        } else {
            None
        }
    }

    /// Square column.
    #[must_use]
    pub const fn x(self) -> u8 {
        self.x
    }

    /// Square row.
    #[must_use]
    pub const fn y(self) -> u8 {
        self.y
    }

    /// Row-major offset into a `64 * 64` array.
    #[must_use]
    pub const fn index(self) -> usize {
        self.y as usize * BLOCK_SQUARES as usize + self.x as usize
    }
}

/// Index of a 1 km cell within the continent working grid.
///
/// Continent axes are capped at 2,000 km (`config.rs`), so `u16` holds
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

/// Index of a 1 km cell on the continent working grid (`logic/01` step 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContinentCoord {
    /// Column on the 1 km grid.
    pub x: i32,
    /// Row on the 1 km grid.
    pub y: i32,
}

impl ContinentCoord {
    /// Builds a continent-grid index.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn square_coord_rejects_out_of_bounds() {
        assert!(SquareCoord::new(63, 63).is_some());
        assert!(SquareCoord::new(64, 0).is_none());
    }

    #[test]
    fn square_coord_index_is_row_major() {
        let s = SquareCoord::new(5, 4).unwrap();
        assert_eq!(s.index(), 4 * 64 + 5);
    }
}
