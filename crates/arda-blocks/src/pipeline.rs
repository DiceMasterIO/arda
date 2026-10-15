//! The block pipeline: refine the cells a window covers (cached per cell),
//! assemble them into one layout, then compose the overlays.

use crate::crop::Cut;
use crate::overlays::{compose_all, OverlayCtx, Overlays, Owner};
use crate::BlocksError;
use arda_refine::output::assemble;
use arda_refine::rules::MapMeta;
use arda_refine::{refine, Block, CellKey, Source};
use arda_scene::RulesSidecar;
use arda_tactical::{Library, TacticalLayout};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Squares per cell side (convention I2: a cell is 64 × 64 squares).
pub const SQUARES_PER_CELL: i64 = 64;
/// Largest window side, squares: 4 × 4 blocks (logic/09 Steps 1).
pub const MAX_WINDOW_SQUARES: u32 = 256;
/// Refined blocks kept by default (about 0.3 MB each).
pub const DEFAULT_BLOCK_CACHE: usize = 256;
/// Squares every window is composed beyond its own edges before it is cut
/// out (review 3.5-8, logic/09 §window-crop): an edge on the window's
/// border is decided with both of its squares in view, and a placement up
/// to this far outside folds its footprint's rules into the window. Twice
/// the largest catalogue footprint side (4 squares).
pub const COMPOSE_MARGIN: u32 = 8;

/// A composed window: what the compositor draws and what the scene reads.
#[derive(Debug, Clone, PartialEq)]
pub struct Composed {
    /// The layout, with `origin` set to the window's world square (I10).
    pub layout: TacticalLayout,
    /// The merged rules, `arda-scene` format 2 (I9), with the placements'
    /// catalogue rules folded in.
    pub rules: RulesSidecar,
    /// arda-refine's review record: relaxed fills, WFC attempts, tags.
    pub meta: MapMeta,
    /// Row-major owner of every square after composition.
    pub owners: Vec<Owner>,
    /// Overlay layers that claimed at least one square, in order.
    pub overlays: Vec<&'static str>,
}

/// LRU of refined blocks by cell, bounded by count.
#[derive(Default)]
struct BlockCache {
    tick: u64,
    cap: usize,
    map: BTreeMap<CellKey, (u64, Arc<Block>)>,
}

impl BlockCache {
    fn get(&mut self, k: CellKey) -> Option<Arc<Block>> {
        self.tick += 1;
        let tick = self.tick;
        self.map.get_mut(&k).map(|e| {
            e.0 = tick;
            Arc::clone(&e.1)
        })
    }

    fn put(&mut self, k: CellKey, b: Arc<Block>) {
        self.tick += 1;
        self.map.insert(k, (self.tick, b));
        while self.map.len() > self.cap.max(1) {
            let oldest = self.map.iter().min_by_key(|(_, e)| e.0).map(|(k, _)| *k);
            match oldest {
                Some(o) => {
                    self.map.remove(&o);
                }
                None => break,
            }
        }
    }
}

/// Refines and composes tactical windows of one world.
pub struct Pipeline {
    src: Box<dyn Source + Send + Sync>,
    cache: Mutex<BlockCache>,
    /// The compositor seed placements are resolved with when their
    /// catalogue rules fold into the sidecar; the world seed unless set.
    art_seed: Option<u64>,
}

impl std::fmt::Debug for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pipeline")
            .field("cells", &self.src.cells_wide_high())
            .finish_non_exhaustive()
    }
}

impl Pipeline {
    /// A pipeline over `src`, keeping up to `blocks` refined blocks.
    #[must_use]
    pub fn new(src: Box<dyn Source + Send + Sync>, blocks: usize) -> Self {
        Self {
            src,
            cache: Mutex::new(BlockCache {
                cap: blocks,
                ..BlockCache::default()
            }),
            art_seed: None,
        }
    }

    /// Resolves placements with `seed` when folding their catalogue rules
    /// into the sidecar: the seed the images are rendered with, so the
    /// rules name the takes the image draws (review 3.6-4).
    #[must_use]
    pub const fn with_art_seed(mut self, seed: u64) -> Self {
        self.art_seed = Some(seed);
        self
    }

    /// The seed placements are resolved with ([`Self::with_art_seed`],
    /// else the world seed).
    #[must_use]
    pub fn art_seed(&self) -> u64 {
        self.art_seed.unwrap_or_else(|| self.src.seed())
    }

    /// The world seed.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.src.seed()
    }

    /// World extent in cells.
    #[must_use]
    pub fn cells(&self) -> (i64, i64) {
        self.src.cells_wide_high()
    }

    /// The refined block of `cell`, cached.
    ///
    /// # Errors
    /// Refinement failures.
    pub fn refined(&self, cell: CellKey) -> Result<Arc<Block>, BlocksError> {
        if let Some(b) = self.lock()?.get(cell) {
            return Ok(b);
        }
        let b = Arc::new(refine(self.src.as_ref(), cell)?);
        self.lock()?.put(cell, Arc::clone(&b));
        Ok(b)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, BlockCache>, BlocksError> {
        self.cache.lock().map_err(|_| BlocksError::Poisoned)
    }

    /// Refines the missing cells on the shared rayon pool (bounded, never
    /// a thread per request: review 3.6-12); each block is a pure function
    /// of its cell, so scheduling never changes a result.
    fn refine_all(&self, cells: &[CellKey]) -> Result<Vec<Arc<Block>>, BlocksError> {
        use rayon::prelude::*;
        cells.par_iter().map(|&c| self.refined(c)).collect()
    }

    /// Whether `cell` is open sea with no land square (logic/09 Branches).
    ///
    /// # Errors
    /// Refinement or layer failures.
    pub fn is_open_sea(&self, cell: CellKey) -> Result<bool, BlocksError> {
        if self.src.cell(cell)?.terrain != arda::TerrainKind::Sea {
            return Ok(false);
        }
        Ok(self.refined(cell)?.depth_ft.iter().all(|d| *d > 0))
    }

    /// Composes the window `[gsx0, gsx0 + w) × [gsy0, gsy0 + h)` of world
    /// squares: refined terrain, then `overlays` (ways, fields, town).
    ///
    /// # Errors
    /// [`BlocksError::Window`] for an empty, oversized or out-of-world
    /// window; refinement and overlay failures.
    pub fn window(
        &self,
        (gsx0, gsy0, w, h): (i64, i64, u32, u32),
        library: &Library,
        overlays: Option<&dyn Overlays>,
    ) -> Result<Composed, BlocksError> {
        let refuse = |reason: String| BlocksError::Window {
            gsx0,
            gsy0,
            w,
            h,
            reason,
        };
        let max = MAX_WINDOW_SQUARES + 2 * 8;
        if w == 0 || h == 0 || w > max || h > max {
            return Err(refuse(format!("sides must be 1..={max} squares")));
        }
        let (cw, ch) = self.cells();
        let (x1, y1) = (gsx0 + i64::from(w), gsy0 + i64::from(h));
        if gsx0 < 0 || gsy0 < 0 || x1 > cw * SQUARES_PER_CELL || y1 > ch * SQUARES_PER_CELL {
            return Err(refuse("outside the world".into()));
        }
        // Compose the window grown by the margin (clipped to the world),
        // then cut the window out of it.
        let m = i64::from(COMPOSE_MARGIN);
        let (gx0, gy0) = ((gsx0 - m).max(0), (gsy0 - m).max(0));
        let gx1 = (x1 + m).min(cw * SQUARES_PER_CELL);
        let gy1 = (y1 + m).min(ch * SQUARES_PER_CELL);
        let grown = (
            gx0,
            gy0,
            u32::try_from(gx1 - gx0).unwrap_or(w),
            u32::try_from(gy1 - gy0).unwrap_or(h),
        );
        let whole = self.compose(grown, library, overlays)?;
        let cut = Cut {
            x: u32::try_from(gsx0 - gx0).unwrap_or(0),
            y: u32::try_from(gsy0 - gy0).unwrap_or(0),
            w,
            h,
        };
        Ok(whole.cut(cut, &window_name(gsx0, gsy0, w, h)))
    }

    /// Composes the window as it stands (no margin): refined terrain, the
    /// overlays, the placements' catalogue rules folded in.
    fn compose(
        &self,
        (gsx0, gsy0, w, h): (i64, i64, u32, u32),
        library: &Library,
        overlays: Option<&dyn Overlays>,
    ) -> Result<Composed, BlocksError> {
        let (x1, y1) = (gsx0 + i64::from(w), gsy0 + i64::from(h));
        let n = SQUARES_PER_CELL;
        let cells: Vec<CellKey> = (gsy0.div_euclid(n)..=(y1 - 1).div_euclid(n))
            .flat_map(|cy| {
                (gsx0.div_euclid(n)..=(x1 - 1).div_euclid(n)).map(move |cx| CellKey::new(cx, cy))
            })
            .collect();
        let blocks: Vec<Arc<Block>> = self.refine_all(&cells)?;
        let name = window_name(gsx0, gsy0, w, h);
        let refs: Vec<&Block> = blocks.iter().map(AsRef::as_ref).collect();
        let map = assemble(&name, &refs, gsx0, gsy0, w, h);
        let (mut layout, mut rules) = (map.layout, map.rules);
        let mut owners: Vec<Owner> = layout
            .squares
            .iter()
            .map(|s| {
                if s.water_depth_ft > 0 {
                    Owner::Water
                } else {
                    Owner::Natural
                }
            })
            .collect();
        layout.origin = Some([gsx0, gsy0]);
        let applied = match overlays {
            Some(o) => {
                let base = layout.clone();
                let ctx = OverlayCtx {
                    gsx0,
                    gsy0,
                    width: w,
                    height: h,
                    seed: self.seed(),
                    base: &base,
                    library,
                };
                compose_all(&ctx, o, &mut layout, &mut rules, &mut owners)?
            }
            None => crate::overlays::Applied::default(),
        };
        layout.origin = Some([gsx0, gsy0]);
        crate::crop::snap_positions(&mut layout);
        // The rules contract (logic/12 §scene-sidecar): every placement's
        // catalogue blocking, sight blocking, canopy and cover join the
        // sidecar, so a reader of the rules alone (tokens, the rules
        // endpoint) sees what the scene sees. Resolved with the art seed
        // the image is drawn with (review 3.6-4).
        let placed = arda_scene::squares::resolve_known(&layout, library, self.art_seed());
        arda_scene::squares::fold_placements(&mut rules, &placed);
        // Relaxed overlay fills (the town WFC's) join the refiner's review
        // record (goal 47).
        let mut meta = map.meta;
        if !applied.review.is_empty() {
            meta.relaxed = true;
            meta.review.extend(&applied.review);
            meta.review.sort_unstable();
            meta.review.dedup();
        }
        Ok(Composed {
            layout,
            rules,
            meta,
            owners,
            overlays: applied.layers,
        })
    }
}

impl Composed {
    /// The sub-window `cut` of this composition, named `name`
    /// ([`crate::crop`]): squares, owners, rules, border walls and edge
    /// rules, the placements and lights anchored in it, and the review
    /// squares in it.
    #[must_use]
    pub fn cut(&self, cut: Cut, name: &str) -> Self {
        let width = self.layout.width;
        let layout = crate::crop::layout(&self.layout, cut, name);
        let rules = crate::crop::rules(&self.rules, cut);
        let owners = cut
            .indices(width)
            .filter_map(|i| self.owners.get(i).copied())
            .collect();
        let mut meta = self.meta.clone();
        meta.name = name.to_owned();
        (meta.width, meta.height) = (cut.w, cut.h);
        meta.review = self
            .meta
            .review
            .iter()
            .filter_map(|&i| {
                let (x, y) = (i % width.max(1), i / width.max(1));
                let inside = x >= cut.x && y >= cut.y && x < cut.x + cut.w && y < cut.y + cut.h;
                inside.then(|| (y - cut.y) * cut.w + (x - cut.x))
            })
            .collect();
        // Placement records describe the refined layout before overlays;
        // keep those of the placements the cut holds while they still
        // line up with the layout.
        meta.placements = if self.meta.placements.len() == self.layout.placements.len() {
            self.layout
                .placements
                .iter()
                .zip(&self.meta.placements)
                .filter(|(p, _)| cut.holds(p.x, p.y))
                .map(|(_, m)| m.clone())
                .collect()
        } else {
            Vec::new()
        };
        Self {
            layout,
            rules,
            meta,
            owners,
            overlays: self.overlays.clone(),
        }
    }
}

/// `cell_<gx>_<gy>` for a whole cell, else `window_<gsx0>_<gsy0>_<w>x<h>`
/// (logic/09 Outcomes).
#[must_use]
pub fn window_name(gsx0: i64, gsy0: i64, w: u32, h: u32) -> String {
    let n = SQUARES_PER_CELL;
    let whole = gsx0 % n == 0 && gsy0 % n == 0 && i64::from(w) == n && i64::from(h) == n;
    if whole {
        format!("cell_{}_{}", gsx0 / n, gsy0 / n)
    } else {
        format!("window_{gsx0}_{gsy0}_{w}x{h}")
    }
}
