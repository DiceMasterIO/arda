//! The tactical pipeline's world source with a bounded area cache (review
//! 3.6-8).
//!
//! [`arda_refine::WorldSource`] reads areas through [`arda::World::area`],
//! whose cache is never evicted: a long session that pans the whole map
//! would end up holding every decoded area. [`BoundedSource`] keeps one
//! `WorldSource` per decoded area instead, each over its own freshly
//! loaded `World` (a manifest read) and a [`WorldSource::sibling`] of the
//! base source, so it shares the base's fine-terrain reader, which was
//! verified once at open, and its shore layer. At most [`REFINE_AREAS`]
//! areas are kept, least recently used evicted first; an evicted area is
//! freed when its last in-flight reader finishes. Every read is a pure
//! read of the stored world, so eviction never changes an answer (bytes
//! stay deterministic); it only costs decoding the area again.
//!
//! This replaced generations of six areas that reopened the whole world
//! (re-verifying the 1.25 GB fine-terrain file of a default-size world,
//! about 0.3 s, under the source's lock): readers spread over more areas
//! than a generation held reopened it every few reads and never finished.
//! Now opening an area costs a manifest read outside the lock, and only
//! readers whose areas no longer fit [`REFINE_AREAS`] decode again.
//!
//! Admission counts [`REFINE_AREAS`] kept areas, as many evicted ones still
//! in use, and one fine memo ([`admitted_bytes`]).

use arda::Cell;
use arda_core::water::PanKind;
use arda_core::{ShoreClass, AREA_CELLS};
use arda_refine::source::{Edge, LakeInfo};
use arda_refine::{CellKey, RefineError, Source, WorldSource};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// Decoded areas kept: room for the serving lanes (two compositions, two
/// prefetch workers, a bake) on one or two areas each, and for the
/// startup warm-up's batches ([`WARM_AREAS`]).
pub const REFINE_AREAS: usize = 8;
/// The most distinct areas the plans warmed together at startup may read.
pub const WARM_AREAS: usize = REFINE_AREAS;
/// Bound on one decoded area: its cells, objects and water, and the
/// source's lake, pan and channel-edge index.
pub const AREA_BYTES: u64 =
    (AREA_CELLS as u64) * (AREA_CELLS as u64) * std::mem::size_of::<Cell>() as u64 + (3 << 20);
/// Bound on the shared fine-height memo (2²¹ samples in a hash map) and
/// the fine-terrain reader.
pub const FINE_MEMO_BYTES: u64 = 104 << 20;

/// Worst-case resident bytes of the refine source, excluding the shore
/// layer (world-dependent, counted at open): [`REFINE_AREAS`] kept areas,
/// as many evicted ones still being read, and the one fine memo.
#[must_use]
pub const fn admitted_bytes() -> u64 {
    2 * REFINE_AREAS as u64 * AREA_BYTES + FINE_MEMO_BYTES
}

/// Kept area sources by area, with their last-use tick.
#[derive(Default)]
struct Areas {
    tick: u64,
    map: BTreeMap<(i64, i64), (u64, Arc<WorldSource<'static>>)>,
}

/// A refinement source over the world at a directory whose decoded areas
/// stay within [`REFINE_AREAS`]; see the module docs.
pub struct BoundedSource {
    dir: PathBuf,
    seed: u64,
    wide_high: (i64, i64),
    has_shore: bool,
    /// The verified source: fine terrain, shore and biome frame; it never
    /// decodes an area.
    base: Arc<WorldSource<'static>>,
    areas: Mutex<Areas>,
    /// [`REFINE_AREAS`], or a test's smaller bound.
    keep: usize,
    area_opens: AtomicU64,
    verifications: AtomicU64,
}

impl std::fmt::Debug for BoundedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundedSource")
            .field("dir", &self.dir)
            .field("cells", &self.wide_high)
            .finish_non_exhaustive()
    }
}

fn load(dir: &Path) -> Result<Arc<arda::World>, RefineError> {
    Ok(Arc::new(arda::World::load(dir).map_err(RefineError::from)?))
}

impl BoundedSource {
    /// Opens the world at `dir`, verifying its fine terrain and shore
    /// layer once.
    ///
    /// # Errors
    /// The world or its fine terrain cannot load.
    pub fn open(dir: &Path) -> Result<Self, RefineError> {
        Self::keeping(dir, REFINE_AREAS)
    }

    /// [`BoundedSource::open`] keeping `keep` areas (tests of eviction).
    ///
    /// # Errors
    /// As [`BoundedSource::open`].
    #[doc(hidden)]
    pub fn keeping(dir: &Path, keep: usize) -> Result<Self, RefineError> {
        let base = WorldSource::shared(load(dir)?)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            seed: base.seed(),
            wide_high: base.cells_wide_high(),
            has_shore: base.has_shore(),
            base: Arc::new(base),
            areas: Mutex::new(Areas::default()),
            keep: keep.max(1),
            area_opens: AtomicU64::new(0),
            verifications: AtomicU64::new(1),
        })
    }

    /// Area sources opened so far, evictions included (tests).
    #[doc(hidden)]
    #[must_use]
    pub fn area_opens(&self) -> u64 {
        self.area_opens.load(Ordering::SeqCst)
    }

    /// Times this source opened and verified the fine-terrain and shore
    /// layers: once, at [`BoundedSource::open`] (tests).
    #[doc(hidden)]
    #[must_use]
    pub fn verifications(&self) -> u64 {
        self.verifications.load(Ordering::SeqCst)
    }

    /// Whether every kept area source shares the base's verified layers
    /// (tests).
    #[doc(hidden)]
    #[must_use]
    pub fn areas_share_layers(&self) -> bool {
        self.lock()
            .map
            .values()
            .all(|(_, s)| s.shares_layers_with(&self.base))
    }

    fn lock(&self) -> MutexGuard<'_, Areas> {
        self.areas.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The source to read `at`'s area from, opening it on a miss (a
    /// manifest read, outside the lock) and evicting the least recently
    /// used area past [`REFINE_AREAS`].
    fn for_cell(&self, at: CellKey) -> Result<Arc<WorldSource<'static>>, RefineError> {
        let c = self.clamp(at);
        let n = i64::from(AREA_CELLS);
        let area = (c.x.div_euclid(n), c.y.div_euclid(n));
        {
            let mut g = self.lock();
            g.tick += 1;
            let tick = g.tick;
            if let Some(hit) = g.map.get_mut(&area) {
                hit.0 = tick;
                return Ok(Arc::clone(&hit.1));
            }
        }
        let fresh = Arc::new(self.base.sibling(load(&self.dir)?)?);
        let mut g = self.lock();
        g.tick += 1;
        let tick = g.tick;
        if let Some(raced) = g.map.get_mut(&area) {
            // Another reader opened it meanwhile: share theirs.
            raced.0 = tick;
            return Ok(Arc::clone(&raced.1));
        }
        self.area_opens.fetch_add(1, Ordering::SeqCst);
        g.map.insert(area, (tick, Arc::clone(&fresh)));
        while g.map.len() > self.keep {
            let oldest = g.map.iter().min_by_key(|(_, e)| e.0).map(|(k, _)| *k);
            if let Some(k) = oldest {
                g.map.remove(&k);
            }
        }
        Ok(fresh)
    }

    /// The verified base source, for reads that decode no area.
    fn any(&self) -> Arc<WorldSource<'static>> {
        Arc::clone(&self.base)
    }
}

impl Source for BoundedSource {
    fn seed(&self) -> u64 {
        self.seed
    }

    fn cells_wide_high(&self) -> (i64, i64) {
        self.wide_high
    }

    fn cell(&self, at: CellKey) -> Result<Cell, RefineError> {
        self.for_cell(at)?.cell(at)
    }

    fn lake(&self, at: CellKey) -> Result<Option<LakeInfo>, RefineError> {
        self.for_cell(at)?.lake(at)
    }

    fn edges_touching(&self, at: CellKey) -> Result<Vec<Edge>, RefineError> {
        self.for_cell(at)?.edges_touching(at)
    }

    fn fine_mm(&self, kx: i64, ky: i64) -> Result<Option<i32>, RefineError> {
        self.any().fine_mm(kx, ky)
    }

    fn pan(&self, at: CellKey) -> Result<Option<PanKind>, RefineError> {
        self.for_cell(at)?.pan(at)
    }

    fn shore(&self, at: CellKey) -> Result<ShoreClass, RefineError> {
        self.any().shore(at)
    }

    fn has_shore(&self) -> bool {
        self.has_shore
    }

    fn biome_frame(&self) -> arda_core::biome::Frame {
        self.any().biome_frame()
    }

    fn thalweg(&self, at: CellKey) -> Result<Option<(i16, i16)>, RefineError> {
        self.for_cell(at)?.thalweg(at)
    }
}
