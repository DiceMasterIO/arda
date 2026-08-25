//! Shared types, the subseeded PRNG, and the only byte codec in the workspace.
//!
//! See `docs/capstone/01-architecture.md` for the crate boundary rules.

pub mod coords;
pub mod fixed;

pub use coords::{
    AreaCoord, CellCoord, ContinentCoord, SquareCoord, AREA_CELLS, BLOCK_SQUARES, CELL_SIZE_M,
    SQUARE_SIZE_MM,
};
pub use fixed::{DischargeMilli, HeightMm, RainfallMm, TempCentiC};
