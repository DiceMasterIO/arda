//! Block generation (`logic/03`).
//!
//! Skeleton scope: the 24-tile vocabulary from `arda-core::tiles`. Build-order
//! step 6 grows this to 200+ tiles after spike S2, without changing these
//! signatures.

pub mod constraints;
pub mod wfc;

pub use constraints::{constraints_for, BlockConstraints};
pub use wfc::{fill_block, MAX_ATTEMPTS};

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{
        may_adjoin, AreaCells, AreaCoord, Cell, CellCoord, HeightMm, SquareCoord, TerrainKind,
        BLOCK_SQUARES,
    };

    fn cc(x: u16, y: u16) -> CellCoord {
        CellCoord::new(x, y).unwrap()
    }

    fn land_grid() -> AreaCells {
        AreaCells::flat(Cell {
            height: HeightMm::new(220_000),
            terrain: TerrainKind::Land,
            ..Cell::default()
        })
    }

    fn sea_grid() -> AreaCells {
        AreaCells::flat(Cell {
            height: HeightMm::new(-40_000),
            terrain: TerrainKind::Sea,
            ..Cell::default()
        })
    }

    #[test]
    fn land_cells_allow_no_deep_water() {
        let c = constraints_for(&land_grid(), cc(20, 20));
        assert!(!c.allowed.is_empty());
        assert!(c.allowed.iter().all(|&t| t.raw() != 0));
    }

    #[test]
    fn sea_cells_allow_only_water_and_shore() {
        let c = constraints_for(&sea_grid(), cc(20, 20));
        assert!(c.allowed.iter().all(|&t| t.raw() <= 7));
    }

    #[test]
    fn a_coastal_cell_registers_its_wet_neighbours() {
        let mut grid = land_grid();
        grid.set(
            cc(21, 20),
            Cell {
                height: HeightMm::new(-9_000),
                terrain: TerrainKind::Sea,
                ..Cell::default()
            },
        );
        let c = constraints_for(&grid, cc(20, 20));
        assert!(c.wet_fraction > 0, "a neighbouring sea cell must register");
    }

    #[test]
    fn fill_is_deterministic() {
        let grid = land_grid();
        let c = constraints_for(&grid, cc(20, 20));
        let a = fill_block(42, AreaCoord::new(1, 1), cc(20, 20), &c);
        let b = fill_block(42, AreaCoord::new(1, 1), cc(20, 20), &c);
        assert_eq!(a, b);
    }

    #[test]
    fn different_cells_get_different_blocks() {
        let grid = land_grid();
        let c = constraints_for(&grid, cc(20, 20));
        let a = fill_block(42, AreaCoord::new(1, 1), cc(20, 20), &c);
        let b = fill_block(42, AreaCoord::new(1, 1), cc(21, 20), &c);
        assert_ne!(a, b);
    }

    #[test]
    fn every_square_is_filled_with_an_allowed_tile() {
        let grid = land_grid();
        let c = constraints_for(&grid, cc(20, 20));
        let block = fill_block(42, AreaCoord::new(0, 0), cc(20, 20), &c);
        for y in 0..BLOCK_SQUARES {
            for x in 0..BLOCK_SQUARES {
                let t = block.square(SquareCoord::new(x, y).unwrap());
                assert!(c.allowed.contains(&t), "square {x},{y} holds a banned tile");
            }
        }
    }

    /// The convergence property the retry ladder exists to protect
    /// (`logic/03` §Q12). A non-relaxed block satisfies adjacency everywhere.
    #[test]
    fn a_converged_block_satisfies_adjacency_everywhere() {
        let grid = land_grid();
        let c = constraints_for(&grid, cc(20, 20));
        let block = fill_block(42, AreaCoord::new(0, 0), cc(20, 20), &c);
        assert!(
            !block.is_relaxed(),
            "an all-land block should converge, not relax"
        );
        for y in 0..BLOCK_SQUARES {
            for x in 0..BLOCK_SQUARES {
                let here = block.square(SquareCoord::new(x, y).unwrap());
                if x + 1 < BLOCK_SQUARES {
                    let right = block.square(SquareCoord::new(x + 1, y).unwrap());
                    assert!(may_adjoin(here, right), "bad horizontal seam at {x},{y}");
                }
                if y + 1 < BLOCK_SQUARES {
                    let down = block.square(SquareCoord::new(x, y + 1).unwrap());
                    assert!(may_adjoin(here, down), "bad vertical seam at {x},{y}");
                }
            }
        }
    }

    #[test]
    fn an_impossible_constraint_set_falls_back_to_a_marked_relaxed_fill() {
        // logic/03 §Q12: never fail the batch. Every tile is compatible with
        // itself, so a non-empty set always tiles somehow — an empty set is
        // the genuinely unsatisfiable case, and it must relax rather than
        // panic or loop.
        let c = BlockConstraints {
            allowed: Vec::new(),
            wet_fraction: 128,
        };
        let block = fill_block(42, AreaCoord::new(0, 0), cc(5, 5), &c);
        assert!(block.is_relaxed());
    }
}
