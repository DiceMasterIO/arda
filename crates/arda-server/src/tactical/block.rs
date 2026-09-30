//! The seam for world-derived tactical blocks (goals 48 and 67; logic/16
//! §api-tactical).
//!
//! `/v1/tactical/cell/{gx}/{gy}` and `/v1/tactical/window` ask a
//! [`BlockSource`] for the [`Block`] covering a window of world squares:
//! its layout, its per-square rules sidecar (`arda-scene`'s `RulesSidecar`
//! format 2, convention I9), its world origin and provenance metadata. The
//! server builds the scene and the images from that one layout, so they
//! never disagree (goal 48). [`super::refine_blocks::RefineBlocks`] is the
//! real source; [`PendingBlocks`] answers every request with a typed "not
//! yet" error and remains for tests and worlds without a source.

use arda_scene::RulesSidecar;
use arda_tactical::{Library, TacticalLayout};
use serde::Serialize;
use std::collections::BTreeMap;

/// One world-derived tactical block or window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Block {
    /// What the compositor draws; its `origin` is the world square of its
    /// top-left square (I10).
    pub layout: TacticalLayout,
    /// Per-square rules, `arda-scene` format 2, the layout's size.
    pub rules: Option<RulesSidecar>,
    /// The layout's top-left square in world squares (64 per cell, I2).
    pub origin: [i64; 2],
    /// Provenance, for example `source`, `relaxed` and `overlays`.
    pub meta: BTreeMap<String, String>,
}

/// A window of world squares and the overlays wanted on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockRequest {
    /// First square column (64 per cell).
    pub gsx0: i64,
    /// First square row.
    pub gsy0: i64,
    /// Width in squares.
    pub w: u32,
    /// Height in squares.
    pub h: u32,
    /// `Some(cell)`: compose the demo overlays anchored at that cell.
    pub demo_at: Option<[i64; 2]>,
}

impl BlockRequest {
    /// The whole block of global cell `(gx, gy)`.
    #[must_use]
    pub fn cell(gx: u32, gy: u32) -> Self {
        Self {
            gsx0: i64::from(gx) * 64,
            gsy0: i64::from(gy) * 64,
            w: 64,
            h: 64,
            demo_at: None,
        }
    }

    /// The same window grown by `apron` squares on every side, clipped to
    /// `[0, max_x) × [0, max_y)` world squares.
    #[must_use]
    pub fn with_apron(self, apron: u32, (max_x, max_y): (i64, i64)) -> Self {
        let a = i64::from(apron);
        let (x0, y0) = ((self.gsx0 - a).max(0), (self.gsy0 - a).max(0));
        let x1 = (self.gsx0 + i64::from(self.w) + a).min(max_x);
        let y1 = (self.gsy0 + i64::from(self.h) + a).min(max_y);
        Self {
            gsx0: x0,
            gsy0: y0,
            w: u32::try_from(x1 - x0).unwrap_or(self.w),
            h: u32::try_from(y1 - y0).unwrap_or(self.h),
            demo_at: self.demo_at,
        }
    }
}

/// The crate planned to derive tactical blocks from the stored world.
pub const PLANNED_BLOCK_SOURCE: &str = "arda-refine";

/// Why a block source could not supply a block.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BlockError {
    /// No source derives blocks yet; `planned_source` names the crate that will.
    #[error(
        "world-derived tactical blocks are not implemented yet; planned source: {planned_source}"
    )]
    NotYet {
        /// Crate that will provide the blocks.
        planned_source: String,
    },
    /// The source has no block at this cell (for example open sea).
    #[error("no tactical block at cell {gx},{gy}: {reason}")]
    NoBlock {
        /// Global cell column.
        gx: u32,
        /// Global cell row.
        gy: u32,
        /// Why.
        reason: String,
    },
    /// The window is empty, too large or outside the world.
    #[error("bad tactical window: {0}")]
    Window(String),
    /// The source failed.
    #[error("block source failed: {0}")]
    Failed(String),
}

/// Supplies the tactical block covering a window of world squares.
///
/// Implementations must be deterministic: the same world, window, overlays
/// and catalogue version always give an equal block, so bodies and renders
/// can be cached (goal 67).
pub trait BlockSource: Send + Sync + std::fmt::Debug {
    /// The block for `req`, which the server has already checked lies inside
    /// the world. `library` is the catalogue the layout must resolve against.
    ///
    /// # Errors
    /// [`BlockError`] when the block cannot be produced.
    fn block(&self, req: &BlockRequest, library: &Library) -> Result<Block, BlockError>;
}

/// The placeholder source: every request is [`BlockError::NotYet`].
#[derive(Debug, Clone, Copy, Default)]
pub struct PendingBlocks;

impl BlockSource for PendingBlocks {
    fn block(&self, _req: &BlockRequest, _library: &Library) -> Result<Block, BlockError> {
        Err(BlockError::NotYet {
            planned_source: PLANNED_BLOCK_SOURCE.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aprons_grow_and_clip_to_the_world() {
        let r = BlockRequest::cell(1, 0).with_apron(2, (192, 192));
        assert_eq!((r.gsx0, r.gsy0, r.w, r.h), (62, 0, 68, 66));
        let edge = BlockRequest::cell(2, 2).with_apron(2, (192, 192));
        assert_eq!((edge.gsx0, edge.gsy0, edge.w, edge.h), (126, 126, 66, 66));
    }
}
