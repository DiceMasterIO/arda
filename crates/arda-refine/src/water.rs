//! Refined river water as a pure function of global squares, for layers
//! that must agree square for square with the channels the blocks draw
//! (arda-blocks lays road crossings on it).
//!
//! A square is river water exactly when a block marks it so: its centre
//! lies within the half-width of some centreline piece
//! ([`crate::rivers::bank_distance`] `<= 0`). Standing water (lakes, sea,
//! pools) is not included.

use crate::context::Ctx;
use crate::error::RefineError;
use crate::rivers::{bank_distance, cell_pieces, Piece};
use crate::source::{CellKey, Source};
use std::collections::HashMap;

/// The centreline pieces of cell `c` (empty for a cell without channels or
/// outside the world).
///
/// # Errors
/// A world layer failed to read.
pub fn cell_channels(src: &dyn Source, c: CellKey) -> Result<Vec<Piece>, RefineError> {
    let (w, h) = src.cells_wide_high();
    if c.x < 0 || c.y < 0 || c.x >= w || c.y >= h || src.edges_touching(c)?.is_empty() {
        return Ok(Vec::new());
    }
    let ctx = Ctx::gather(src, c)?;
    Ok(cell_pieces(&ctx, c))
}

/// Side of the square tiles pieces are binned by.
const TILE: f64 = 16.0;

/// River pieces with their bounds, queried square by square.
#[derive(Debug, Clone, Default)]
pub struct RiverWater {
    pieces: Vec<(Piece, [f64; 4])>,
    /// Pieces whose bounds reach each [`TILE`]-square tile.
    tiles: HashMap<(i64, i64), Vec<u32>>,
}

#[allow(clippy::cast_possible_truncation)] // square coordinates fit i64
fn tile(v: f64) -> i64 {
    (v / TILE).floor() as i64
}

impl RiverWater {
    /// Water from `pieces` (global square units, as [`cell_channels`]
    /// returns them).
    #[must_use]
    pub fn new(pieces: impl IntoIterator<Item = Piece>) -> Self {
        let pieces = pieces
            .into_iter()
            .filter(|p| p.pts.len() >= 2)
            .map(|p| {
                let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
                for (&(x, y), &h) in p.pts.iter().zip(&p.half) {
                    b = [
                        b[0].min(x - h),
                        b[1].min(y - h),
                        b[2].max(x + h),
                        b[3].max(y + h),
                    ];
                }
                (p, b)
            })
            .collect::<Vec<_>>();
        let mut tiles: HashMap<(i64, i64), Vec<u32>> = HashMap::new();
        for (k, (_, b)) in pieces.iter().enumerate() {
            let k = u32::try_from(k).unwrap_or(u32::MAX);
            for ty in tile(b[1])..=tile(b[3]) {
                for tx in tile(b[0])..=tile(b[2]) {
                    tiles.entry((tx, ty)).or_default().push(k);
                }
            }
        }
        Self { pieces, tiles }
    }

    /// Every piece of the channel cells of `cells`.
    ///
    /// # Errors
    /// A world layer failed to read.
    pub fn of_cells(src: &dyn Source, cells: &[CellKey]) -> Result<Self, RefineError> {
        let mut all = Vec::new();
        for &c in cells {
            all.extend(cell_channels(src, c)?);
        }
        Ok(Self::new(all))
    }

    /// The pieces.
    pub fn pieces(&self) -> impl Iterator<Item = &Piece> {
        self.pieces.iter().map(|(p, _)| p)
    }

    /// Whether global square `(gx, gy)` is river water.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // global squares are far below 2^52
    pub fn is_water(&self, gx: i64, gy: i64) -> bool {
        self.contains((gx as f64 + 0.5, gy as f64 + 0.5))
    }

    /// Whether the point `q` (global square units) lies within a channel's
    /// banks; at a square's centre this is [`Self::is_water`].
    #[must_use]
    pub fn contains(&self, q: (f64, f64)) -> bool {
        let Some(near) = self.tiles.get(&(tile(q.0), tile(q.1))) else {
            return false;
        };
        near.iter().any(|&k| {
            self.pieces
                .get(usize::try_from(k).unwrap_or(usize::MAX))
                .is_some_and(|(p, b)| {
                    q.0 >= b[0] && q.0 <= b[2] && q.1 >= b[1] && q.1 <= b[3] && {
                        (1..p.pts.len()).any(|i| bank_distance(p, i, q).0 <= 0.0)
                    }
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::refine;
    use crate::context::N;
    use crate::synthetic::{world, RIVER_CELL};

    #[test]
    fn river_water_matches_the_refined_blocks() {
        let src = world(7);
        let mut checked = 0;
        for c in [RIVER_CELL, RIVER_CELL.offset(0, -1), CellKey::new(10, 5)] {
            let cells: Vec<CellKey> = (-1..=1)
                .flat_map(|dy| (-1..=1).map(move |dx| c.offset(dx, dy)))
                .collect();
            let water = RiverWater::of_cells(&src, &cells).unwrap();
            let block = refine(&src, c).unwrap();
            for y in 0..N {
                for x in 0..N {
                    let (gx, gy) = (c.x * N + x, c.y * N + y);
                    if water.is_water(gx, gy) {
                        checked += 1;
                        let k = usize::try_from(y * N + x).unwrap();
                        assert!(block.depth_ft[k] > 0, "square {gx},{gy} is river water");
                    }
                }
            }
        }
        assert!(checked > 0, "the synthetic world has rivers");
    }
}
