//! [`RefineBlocks`]: the real [`BlockSource`] (adapter A9, logic/16
//! §api-tactical). It refines the requested window with `arda-refine`
//! (blocks cached per cell by `arda-blocks`), composes the optional
//! overlays (ways, then fields, then town; logic/09 §reservations) and
//! returns one layout with its format-2 rules and world origin.
//!
//! When the world has a `society/` directory, blocks carry its roads,
//! fields and towns ([`arda_blocks::society::SocietyOverlays`], A9); the
//! demo samples stay behind `?demo_overlays=1`, anchored at a chosen cell.

use super::block::{Block, BlockError, BlockRequest, BlockSource};
use crate::cache::ByteLru;
use arda_blocks::demo::DemoOverlays;
use arda_blocks::society::SocietyOverlays;
use arda_blocks::{BlocksError, Overlays, Pipeline};
use arda_people::shared::SharedSource;
use arda_refine::{CellKey, WorldSource};
use arda_tactical::Library;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Refined blocks kept in the pipeline's cache.
pub const REFINED_BLOCKS: usize = 256;
/// Bytes of composed blocks kept.
pub const COMPOSED_CACHE_BYTES: usize = 256 << 20;
/// Demo anchors kept (each holds one generated town plan).
const DEMO_ANCHORS: usize = 16;

/// World-derived blocks from `arda-refine` and `arda-blocks`.
pub struct RefineBlocks {
    pipeline: Pipeline,
    composed: Mutex<ByteLru<BlockRequest, Block>>,
    demos: Mutex<BTreeMap<[i64; 2], Arc<DemoOverlays>>>,
    society: Option<Arc<SocietyOverlays>>,
}

impl std::fmt::Debug for RefineBlocks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RefineBlocks")
            .field("pipeline", &self.pipeline)
            .finish_non_exhaustive()
    }
}

fn failed(e: impl std::fmt::Display) -> BlockError {
    BlockError::Failed(e.to_string())
}

impl RefineBlocks {
    /// Opens the stored world at `dir` for refinement.
    ///
    /// # Errors
    /// [`BlockError::Failed`] when the world or its fine terrain cannot load.
    pub fn open(dir: &Path) -> Result<Self, BlockError> {
        let world = arda::World::load(dir).map_err(failed)?;
        let src = WorldSource::shared(Arc::new(world)).map_err(failed)?;
        Ok(Self::new(Pipeline::new(Box::new(src), REFINED_BLOCKS)))
    }

    /// Wraps an existing pipeline (for synthetic worlds in tests).
    #[must_use]
    pub fn new(pipeline: Pipeline) -> Self {
        Self {
            pipeline,
            composed: Mutex::new(ByteLru::new(COMPOSED_CACHE_BYTES)),
            demos: Mutex::new(BTreeMap::new()),
            society: None,
        }
    }

    /// Blocks over an opened world, with its society overlays when given.
    #[must_use]
    pub fn with_society(src: SharedSource, society: Option<Arc<SocietyOverlays>>) -> Self {
        let mut b = Self::new(Pipeline::new(Box::new(src), REFINED_BLOCKS));
        b.society = society;
        b
    }

    fn demo(&self, at: [i64; 2]) -> Result<Arc<DemoOverlays>, BlockError> {
        let mut demos = self.demos.lock().map_err(failed)?;
        if let Some(d) = demos.get(&at) {
            return Ok(Arc::clone(d));
        }
        if demos.len() >= DEMO_ANCHORS {
            demos.pop_first();
        }
        let d = Arc::new(DemoOverlays::at_cell(at[0], at[1]));
        demos.insert(at, Arc::clone(&d));
        Ok(d)
    }

    fn compose(&self, req: &BlockRequest, library: &Library) -> Result<Block, BlockError> {
        let n = 64_i64;
        let whole_cell = req.gsx0 % n == 0
            && req.gsy0 % n == 0
            && i64::from(req.w) == n
            && i64::from(req.h) == n;
        if whole_cell {
            let cell = CellKey::new(req.gsx0 / n, req.gsy0 / n);
            if self.pipeline.is_open_sea(cell).map_err(blocks_error)? {
                return Err(BlockError::NoBlock {
                    gx: u32::try_from(cell.x).unwrap_or(u32::MAX),
                    gy: u32::try_from(cell.y).unwrap_or(u32::MAX),
                    reason: "open sea: no land square in the block".into(),
                });
            }
        }
        let demo = req.demo_at.map(|at| self.demo(at)).transpose()?;
        let overlays = match (&demo, &self.society) {
            (Some(d), _) => Some(d.as_ref() as &dyn Overlays),
            (None, Some(s)) => Some(s.as_ref() as &dyn Overlays),
            (None, None) => None,
        };
        let c = self
            .pipeline
            .window((req.gsx0, req.gsy0, req.w, req.h), library, overlays)
            .map_err(blocks_error)?;
        let mut meta = BTreeMap::new();
        meta.insert("source".into(), "arda-refine".into());
        meta.insert(
            "generator".into(),
            format!(
                "arda-refine {0}, arda-blocks {0}",
                env!("CARGO_PKG_VERSION")
            ),
        );
        meta.insert("relaxed".into(), c.meta.relaxed.to_string());
        meta.insert("review_squares".into(), c.meta.review.len().to_string());
        meta.insert("overlays".into(), c.overlays.join(","));
        if let Some(at) = req.demo_at {
            meta.insert("demo_at".into(), format!("{},{}", at[0], at[1]));
        } else if self.society.is_some() {
            meta.insert("society".into(), "1".into());
        }
        // The fine lattice is read through the shared I1 cell frame
        // (`arda_core::FINE_FRAME_OFFSET_UM`), as `/v1/point` reads it.
        meta.insert("fine_frame".into(), "i1".into());
        Ok(Block {
            origin: [req.gsx0, req.gsy0],
            layout: c.layout,
            rules: Some(c.rules),
            meta,
        })
    }
}

fn blocks_error(e: BlocksError) -> BlockError {
    match e {
        BlocksError::Window { .. } => BlockError::Window(e.to_string()),
        BlocksError::OpenSea { gx, gy } => BlockError::NoBlock {
            gx: u32::try_from(gx).unwrap_or(u32::MAX),
            gy: u32::try_from(gy).unwrap_or(u32::MAX),
            reason: "open sea".into(),
        },
        other => BlockError::Failed(other.to_string()),
    }
}

/// Approximate resident bytes of a composed block.
fn block_bytes(b: &Block) -> usize {
    let squares = b.layout.squares.len();
    squares * (40 + 48) + (b.layout.placements.len() + b.layout.walls.len()) * 64 + 1024
}

impl BlockSource for RefineBlocks {
    fn block(&self, req: &BlockRequest, library: &Library) -> Result<Block, BlockError> {
        if let Some(hit) = self.composed.lock().map_err(failed)?.get(req) {
            return Ok(Block::clone(&hit));
        }
        let b = self.compose(req, library)?;
        let bytes = block_bytes(&b);
        self.composed
            .lock()
            .map_err(failed)?
            .insert(*req, Arc::new(b.clone()), bytes);
        Ok(b)
    }
}
