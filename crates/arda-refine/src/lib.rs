//! Tactical terrain refinement: any 100 m world cell as a 64 × 64 block of
//! 5-ft squares with natural ground, water, rocks and vegetation (goals 42,
//! 43, 46, 47 and 50).
//!
//! The pipeline per block, deterministic from the seed and global
//! coordinates only:
//! 1. continuous elevation from the fine terrain plus rotated detail noise,
//!    quantised to 5-ft contour steps ([`terrain`]);
//! 2. rivers, lake and sea shores and cliffs placed first, with channel
//!    crossings fixed from coarse data ([`rivers`], [`fixed`]);
//! 3. border corners fixed from shared data before the WFC ([`borders`]);
//! 4. WFC ground over a corner-tile vocabulary ([`tiles`], [`wfc`]),
//!    weighted by cover, forest density, wetness, slope and aspect
//!    ([`prior`]);
//! 5. Poisson-disk scatter of trees, rocks, logs and reeds ([`scatter`]);
//! 6. an SRD rules sidecar ([`rules`]).
//!
//! Output is the `TacticalLayout` JSON of `arda-tactical` ([`layout`]).

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod block;
pub mod borders;
pub mod classes;
pub mod context;
pub mod density;
pub mod ecology;
pub mod error;
pub mod fixed;
pub mod grid;
pub mod hash;
pub mod layout;
pub mod math;
pub mod noise;
pub mod output;
pub mod pools;
pub mod prior;
pub mod region;
pub mod render;
pub mod rivers;
pub mod rules;
pub mod scatter;
pub mod shape;
pub mod source;
pub mod synthetic;
pub mod terrain;
pub mod tiles;
pub mod trails;
pub mod water;
pub mod wfc;
mod world_source;

pub use block::{refine, Block};
pub use error::RefineError;
pub use output::Map;
pub use source::{CellKey, GridSource, Source};
pub use world_source::WorldSource;

/// Largest window side, in squares.
pub const MAX_WINDOW: u32 = 64 * 16;

/// Refines one cell into a `TacticalLayout` map with its rules sidecar.
///
/// # Errors
/// The cell lies outside the world, or a layer failed to read.
pub fn refine_block(src: &dyn Source, cell: CellKey) -> Result<Map, RefineError> {
    let (w, h) = src.cells_wide_high();
    if cell.x < 0 || cell.y < 0 || cell.x >= w || cell.y >= h {
        return Err(RefineError::OutOfWorld {
            what: "cell",
            x: cell.x,
            y: cell.y,
        });
    }
    let b = refine(src, cell)?;
    let name = format!("arda-{}-{}", cell.x, cell.y);
    Ok(output::assemble(
        &name,
        &[b],
        cell.x * 64,
        cell.y * 64,
        64,
        64,
    ))
}

/// Refines the blocks covering a window of global squares and returns one
/// layout across their edges. Blocks run on parallel threads; each is a
/// pure function of its cell, so the result does not depend on scheduling.
///
/// # Errors
/// The window is empty, too large or outside the world, or a layer failed.
pub fn refine_window(
    src: &(dyn Source + Sync),
    x0: i64,
    y0: i64,
    w: u32,
    h: u32,
) -> Result<Map, RefineError> {
    if w == 0 || h == 0 || w > MAX_WINDOW || h > MAX_WINDOW {
        return Err(RefineError::Window {
            w,
            h,
            max: MAX_WINDOW,
        });
    }
    let (cw, ch) = src.cells_wide_high();
    let (x1, y1) = (x0 + i64::from(w) - 1, y0 + i64::from(h) - 1);
    if x0 < 0 || y0 < 0 || x1 >= cw * 64 || y1 >= ch * 64 {
        return Err(RefineError::OutOfWorld {
            what: "window",
            x: x0,
            y: y0,
        });
    }
    let cells: Vec<CellKey> = (y0.div_euclid(64)..=y1.div_euclid(64))
        .flat_map(|cy| (x0.div_euclid(64)..=x1.div_euclid(64)).map(move |cx| CellKey::new(cx, cy)))
        .collect();
    let threads = std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get);
    let mut blocks = Vec::with_capacity(cells.len());
    for chunk in cells.chunks(threads.max(1)) {
        let results: Vec<Result<Block, RefineError>> = std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|&c| s.spawn(move || refine(src, c)))
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap_or(Err(RefineError::Poisoned)))
                .collect()
        });
        for r in results {
            blocks.push(r?);
        }
    }
    let name = format!("arda-window-{x0}-{y0}-{w}x{h}");
    Ok(output::assemble(&name, &blocks, x0, y0, w, h))
}
