//! Most-constrained-first wave-function collapse (`logic/03` §Q12).
//!
//! At most [`MAX_ATTEMPTS`] subseeded attempts; on exhaustion the block is
//! filled with the most permissive allowed tile and marked relaxed, so a hard
//! cell can never fail the batch.

use super::constraints::BlockConstraints;
use arda_core::{
    may_adjoin, rng, AreaCoord, Block, CellCoord, SeedKey, SquareCoord, Stage, Tier, TileId,
    AREA_CELLS, BLOCK_SQUARES,
};
use rand_core::RngCore;

/// Attempts before the relaxed fallback.
pub const MAX_ATTEMPTS: u8 = 8;

/// Fills one block.
#[must_use]
pub fn fill_block(
    seed: u64,
    area: AreaCoord,
    at: CellCoord,
    constraints: &BlockConstraints,
) -> Block {
    // The stream is keyed by the block's own absolute position, so content
    // never depends on which blocks were generated before it (§Q4).
    let key_x = area.x * i32::from(AREA_CELLS) + i32::from(at.x());
    let key_y = area.y * i32::from(AREA_CELLS) + i32::from(at.y());

    for attempt in 0..MAX_ATTEMPTS {
        let key = SeedKey::new(Tier::Block, Stage::Blocks, key_x, key_y, attempt);
        if let Some(block) = try_fill(seed, key, constraints) {
            return block;
        }
    }

    // Relaxed fallback, marked so export and the block report can surface it
    // (`logic/03` §Q12).
    let fill = constraints
        .allowed
        .first()
        .copied()
        .unwrap_or(TileId::new(0));
    let mut block = Block::filled(fill);
    block.mark_relaxed();
    block
}

/// One collapse attempt. `None` means a contradiction was reached.
///
/// Options are cached per square and refreshed only for the four neighbours
/// of the square just collapsed; recomputing the whole grid each step made a
/// single block take seconds.
fn try_fill(seed: u64, key: SeedKey, constraints: &BlockConstraints) -> Option<Block> {
    let n = i32::from(BLOCK_SQUARES);
    let stride = usize::from(BLOCK_SQUARES);
    let count = usize::try_from(n * n).ok()?;
    let mut r = rng(seed, key);

    let mut grid: Vec<Option<TileId>> = vec![None; count];
    let mut options: Vec<Vec<TileId>> = vec![constraints.allowed.clone(); count];

    for _ in 0..count {
        // Most-constrained-first: fewest remaining options, ties broken by
        // the lowest index so the choice is order-independent.
        let mut best: Option<usize> = None;
        let mut best_len = usize::MAX;
        for idx in 0..count {
            if grid[idx].is_some() {
                continue;
            }
            let len = options[idx].len();
            if len == 0 {
                return None; // Contradiction.
            }
            if len < best_len {
                best_len = len;
                best = Some(idx);
                if len == 1 {
                    break; // Cannot do better.
                }
            }
        }

        let Some(idx) = best else {
            break; // Everything is decided.
        };
        let pick = options[idx][r.next_u32() as usize % options[idx].len()];
        grid[idx] = Some(pick);
        options[idx] = vec![pick];

        // Refresh only the neighbours this choice can constrain.
        let x = i32::try_from(idx % stride).unwrap_or(0);
        let y = i32::try_from(idx / stride).unwrap_or(0);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= n || ny >= n {
                continue;
            }
            let Ok(nidx) = usize::try_from(ny * n + nx) else {
                continue;
            };
            if grid[nidx].is_some() {
                continue;
            }
            options[nidx].retain(|&candidate| may_adjoin(candidate, pick));
        }
    }

    let mut block = Block::filled(constraints.allowed.first().copied()?);
    for (idx, tile) in grid.iter().enumerate() {
        let tile = (*tile)?;
        let x = u8::try_from(idx % stride).ok()?;
        let y = u8::try_from(idx / stride).ok()?;
        block.set(SquareCoord::new(x, y)?, tile);
    }
    Some(block)
}
