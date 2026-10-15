//! Play elevation at any global square, exactly as the blocks draw it
//! (logic/09 §elevation), for layers that level ground they own against
//! the refined terrain beside it: a town's building floors are level with
//! the street at their door wherever the window is cut (goal 46).
//!
//! A query gathers the context and channel pieces the square's own block
//! reads ([`Ctx::gather`], [`crate::rivers::pieces`]), once per cell, and
//! evaluates the block's physical field at that one square, so the answer
//! is the block's `elevation_ft` there, bank shaping and shores included.

use crate::block::contour_ft;
use crate::context::{Ctx, N};
use crate::error::RefineError;
use crate::rivers::Piece;
use crate::source::{CellKey, Source};
use crate::terrain::{physical, Phys, Water};
use std::collections::BTreeMap;

/// Refined play elevation by global square, caching each cell's context.
#[derive(Debug, Default)]
pub struct Heights {
    cells: BTreeMap<CellKey, (Ctx, Vec<Piece>)>,
}

impl Heights {
    /// An empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The `elevation_ft` the refined block of square `(gx, gy)` gives it:
    /// absolute feet in 5-ft steps (water squares give their bed).
    ///
    /// # Errors
    /// A world layer failed to read.
    pub fn elevation_ft(&mut self, src: &dyn Source, gx: i64, gy: i64) -> Result<i16, RefineError> {
        Ok(self.phys(src, gx, gy)?.map_or(0, |p| contour_ft(p.elev_m)))
    }

    /// Whether the refined block of square `(gx, gy)` lays lake or sea
    /// water there (a ferry's open water, review 3.5-9).
    ///
    /// # Errors
    /// A world layer failed to read.
    pub fn open_water(&mut self, src: &dyn Source, gx: i64, gy: i64) -> Result<bool, RefineError> {
        Ok(self
            .phys(src, gx, gy)?
            .is_some_and(|p| matches!(p.water, Water::Lake | Water::Sea)))
    }

    fn phys(&mut self, src: &dyn Source, gx: i64, gy: i64) -> Result<Option<Phys>, RefineError> {
        let cell = CellKey::new(gx.div_euclid(N), gy.div_euclid(N));
        let (ctx, pieces) = match self.cells.entry(cell) {
            std::collections::btree_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::btree_map::Entry::Vacant(e) => {
                let ctx = Ctx::gather(src, cell)?;
                let pieces = crate::rivers::pieces(&ctx);
                e.insert((ctx, pieces))
            }
        };
        let grid = physical(ctx, pieces, gx, gy, 1);
        Ok(grid.data.first().copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::refine;
    use crate::synthetic::{world, MARSH, RIVER_CELL, SEA_ROW};

    /// A point query gives the block's own elevation, square for square,
    /// over a river, a sea shore, a marsh and dry ground.
    #[test]
    fn heights_match_the_refined_blocks() {
        let src = world(7);
        let mut heights = Heights::new();
        let cells = [
            RIVER_CELL,
            RIVER_CELL.offset(1, 0),
            CellKey::new(5, SEA_ROW - 1),
            CellKey::new(MARSH[0].0, MARSH[0].1),
            CellKey::new(3, 3),
        ];
        let mut levels = std::collections::BTreeSet::new();
        for c in cells {
            let block = refine(&src, c).unwrap();
            for k in (0..usize::try_from(N * N).unwrap()).step_by(7) {
                let (x, y) = (i64::try_from(k).unwrap() % N, i64::try_from(k).unwrap() / N);
                let (gx, gy) = (c.x * N + x, c.y * N + y);
                let e = heights.elevation_ft(&src, gx, gy).unwrap();
                assert_eq!(e, block.elevation_ft[k], "square {gx},{gy}");
                levels.insert(e);
            }
        }
        assert!(levels.len() > 3, "{levels:?}");
    }
}
