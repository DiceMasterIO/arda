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
use crate::flight::Gate;
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
/// Compositions running at once (review 3.6-8): requests and prefetch
/// workers queue for these, so admission counts this many transient
/// compositions ([`COMPOSE_TRANSIENT_BYTES`] each).
pub const COMPOSE_SLOTS: usize = 2;
/// Worst-case transient bytes of one composition: a 3 × 3-cell window
/// with its apron (204 × 204 squares), its sixteen refined blocks, the
/// overlay plans' working sets and the composed result. A town window on
/// the MICRO fixture peaks near 132 MiB.
pub const COMPOSE_TRANSIENT_BYTES: u64 = 192 << 20;
/// Bound on one refined block in the pipeline's cache: 4,096 squares of
/// ground, height, depth, dryness, fixed choice and rules, 65² corners,
/// the halo and its placements. The MICRO city and forest blocks measure
/// about 210 KB (1,352 and 1,079 placements); the bound keeps 2.5x that
/// (`tests/world_api/admission.rs`).
pub const REFINED_BLOCK_BYTES: u64 = 512 << 10;
/// Demo anchors kept (each holds one generated town plan).
const DEMO_ANCHORS: usize = 16;

/// World-derived blocks from `arda-refine` and `arda-blocks`.
pub struct RefineBlocks {
    pipeline: Pipeline,
    composed: Mutex<ByteLru<BlockRequest, Block>>,
    /// At most [`COMPOSE_SLOTS`] compositions at a time.
    slots: Gate,
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
        Ok(Self::new(pipeline(Box::new(src))))
    }

    /// Wraps an existing pipeline (for synthetic worlds in tests).
    #[must_use]
    pub fn new(pipeline: Pipeline) -> Self {
        Self {
            pipeline,
            composed: Mutex::new(ByteLru::new(COMPOSED_CACHE_BYTES)),
            slots: Gate::new(COMPOSE_SLOTS),
            demos: Mutex::new(BTreeMap::new()),
            society: None,
        }
    }

    /// Blocks over an opened world, with its society overlays when given.
    #[must_use]
    pub fn with_society(src: SharedSource, society: Option<Arc<SocietyOverlays>>) -> Self {
        let mut b = Self::new(pipeline(Box::new(src)));
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

    /// Open sea answers `no_block` for a whole-cell request.
    fn sea(&self, req: &BlockRequest) -> Result<(), BlockError> {
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
        Ok(())
    }

    fn compose(&self, req: &BlockRequest, library: &Library) -> Result<Block, BlockError> {
        self.sea(req)?;
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
            review: c.meta.review,
        })
    }
}

/// A pipeline over `src` whose sidecar folds resolve placements with the
/// world's render seed, the seed the images are drawn with (review 3.6-4).
fn pipeline(src: Box<dyn arda_refine::Source + Send + Sync>) -> Pipeline {
    let seed = super::key::render_seed(src.seed(), &super::Anchor::Origin(0, 0));
    Pipeline::new(src, REFINED_BLOCKS).with_art_seed(seed)
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

/// Entries a `BTreeMap` leaf node holds (std's `B = 6`: `2B - 1`).
const MAP_NODE_ENTRIES: usize = 11;
/// Bytes of one `BTreeMap<String, serde_json::Value>` leaf node: eleven
/// keys and values plus the node header.
const MAP_NODE_BYTES: usize = MAP_NODE_ENTRIES * (24 + 32) + 16;
/// Allocator overhead per heap allocation (header and rounding).
const ALLOC_OVERHEAD: usize = 16;

fn vec_bytes<T>(v: &[T], capacity: usize) -> usize {
    capacity.max(v.len()) * std::mem::size_of::<T>() + ALLOC_OVERHEAD
}

/// Heap bytes of a JSON map beyond its string payloads (counted by the
/// caller through the serialised length): its nodes, one allocation per
/// key and the nested containers.
fn map_overhead<'a>(len: usize, values: impl Iterator<Item = &'a serde_json::Value>) -> usize {
    len.div_ceil(MAP_NODE_ENTRIES) * MAP_NODE_BYTES
        + len * ALLOC_OVERHEAD
        + values.map(value_overhead).sum::<usize>()
}

fn value_overhead(v: &serde_json::Value) -> usize {
    match v {
        serde_json::Value::String(_) => ALLOC_OVERHEAD,
        serde_json::Value::Array(a) => {
            a.capacity() * 32 + ALLOC_OVERHEAD + a.iter().map(value_overhead).sum::<usize>()
        }
        serde_json::Value::Object(m) => map_overhead(m.len(), m.values()),
        _ => 0,
    }
}

/// Resident bytes charged for a composed block (review 3.6-8): every
/// vector at its capacity, the string payloads (bounded by the layout's
/// and rules' JSON length), and each square's `ext` map with its nodes
/// and allocations, which in towns outweigh the squares themselves.
#[must_use]
pub fn block_bytes(b: &Block) -> usize {
    let l = &b.layout;
    let mut bytes = vec_bytes(&l.squares, l.squares.capacity())
        + vec_bytes(&l.walls, l.walls.capacity())
        + vec_bytes(&l.placements, l.placements.capacity())
        + vec_bytes(&l.lights, l.lights.capacity())
        + json_len(l)
        + l.squares.len() * ALLOC_OVERHEAD
        + b.meta
            .iter()
            .map(|(k, v)| k.len() + v.len() + 2 * ALLOC_OVERHEAD + 64)
            .sum::<usize>()
        + 1024;
    if let Some(r) = &b.rules {
        bytes += vec_bytes(&r.squares, r.squares.capacity())
            + vec_bytes(&r.edges, r.edges.capacity())
            + json_len(r)
            + r.squares
                .iter()
                .filter_map(|c| c.ext.as_ref())
                .map(|e| map_overhead(e.len(), e.values()))
                .sum::<usize>();
    }
    bytes
}

/// Serialised length: a bound on the string payloads a value holds. A
/// failed serialisation charges nothing extra (the vectors still count).
fn json_len<T: serde::Serialize>(v: &T) -> usize {
    serde_json::to_vec(v).map_or(0, |v| v.len())
}

impl BlockSource for RefineBlocks {
    fn check(&self, req: &BlockRequest) -> Result<(), BlockError> {
        self.sea(req)
    }

    fn block(&self, req: &BlockRequest, library: &Library) -> Result<Block, BlockError> {
        if let Some(hit) = self.composed.lock().map_err(failed)?.get(req) {
            return Ok(Block::clone(&hit));
        }
        // An abandoned request gives up its wait for a slot.
        let _slot = self.slots.enter().map_err(failed)?;
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
