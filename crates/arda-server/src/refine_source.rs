//! The tactical pipeline's world source with a bounded area cache (review
//! 3.6-8).
//!
//! [`arda_refine::WorldSource`] reads areas through [`arda::World::area`],
//! whose cache is never evicted: a long session that pans the whole map
//! would end up holding every decoded area. [`BoundedSource`] wraps a
//! *generation*: a `WorldSource` over its own freshly loaded `World`. Once
//! queries have touched [`REFINE_AREAS`] distinct areas, the next new area
//! starts a new generation and the old one is freed when its last in-flight
//! user finishes. Every read is a pure read of the stored world, so a
//! generation change never changes an answer (bytes stay deterministic);
//! it only costs re-reading the areas and the shore layer.
//!
//! Admission counts two generations ([`admitted_bytes`]): the live one and
//! one still finishing.

use arda::Cell;
use arda_core::water::PanKind;
use arda_core::{ShoreClass, AREA_CELLS};
use arda_refine::source::{Edge, LakeInfo};
use arda_refine::{CellKey, RefineError, Source, WorldSource};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

/// Distinct areas one generation decodes before the next starts.
pub const REFINE_AREAS: usize = 6;
/// Bound on one decoded area in a generation: its cells, objects and
/// water, and the source's lake, pan and channel-edge index.
pub const AREA_BYTES: u64 =
    (AREA_CELLS as u64) * (AREA_CELLS as u64) * std::mem::size_of::<Cell>() as u64 + (3 << 20);
/// Bound on a generation's fine-height memo (2²¹ samples in a hash map)
/// and its fine-terrain reader.
pub const FINE_MEMO_BYTES: u64 = 104 << 20;

/// Worst-case resident bytes of the refine source, excluding the shore
/// layer (world-dependent, counted at open): two generations of
/// [`REFINE_AREAS`] areas and a fine memo each.
#[must_use]
pub const fn admitted_bytes() -> u64 {
    2 * (REFINE_AREAS as u64 * AREA_BYTES + FINE_MEMO_BYTES)
}

struct Generation {
    source: Arc<WorldSource<'static>>,
    areas: BTreeSet<(i64, i64)>,
}

/// A refinement source over the world at a directory whose decoded areas
/// stay within [`REFINE_AREAS`] per generation; see the module docs.
pub struct BoundedSource {
    dir: PathBuf,
    seed: u64,
    wide_high: (i64, i64),
    has_shore: bool,
    current: Mutex<Generation>,
    generations: std::sync::atomic::AtomicU64,
}

impl std::fmt::Debug for BoundedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundedSource")
            .field("dir", &self.dir)
            .field("cells", &self.wide_high)
            .finish_non_exhaustive()
    }
}

fn open_source(dir: &Path) -> Result<WorldSource<'static>, RefineError> {
    let world = arda::World::load(dir).map_err(RefineError::from)?;
    WorldSource::shared(Arc::new(world))
}

impl BoundedSource {
    /// Opens the world at `dir`.
    ///
    /// # Errors
    /// The world or its fine terrain cannot load.
    pub fn open(dir: &Path) -> Result<Self, RefineError> {
        let source = open_source(dir)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            seed: source.seed(),
            wide_high: source.cells_wide_high(),
            has_shore: source.has_shore(),
            current: Mutex::new(Generation {
                source: Arc::new(source),
                areas: BTreeSet::new(),
            }),
            generations: std::sync::atomic::AtomicU64::new(1),
        })
    }

    /// Generations opened so far (tests).
    #[doc(hidden)]
    #[must_use]
    pub fn generations(&self) -> u64 {
        self.generations.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// The generation to read `at` from, starting a new one when `at`'s
    /// area would be one too many for the current one.
    fn for_cell(&self, at: CellKey) -> Result<Arc<WorldSource<'static>>, RefineError> {
        let c = self.clamp(at);
        let n = i64::from(AREA_CELLS);
        let area = (c.x.div_euclid(n), c.y.div_euclid(n));
        let mut g = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        if g.areas.contains(&area) {
            return Ok(Arc::clone(&g.source));
        }
        if g.areas.len() >= REFINE_AREAS {
            // Loading under the lock keeps one new generation at a time;
            // it happens once per REFINE_AREAS new areas.
            g.source = Arc::new(open_source(&self.dir)?);
            g.areas.clear();
            self.generations
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        g.areas.insert(area);
        Ok(Arc::clone(&g.source))
    }

    /// The current generation, for reads that decode no area.
    fn any(&self) -> Arc<WorldSource<'static>> {
        let g = self.current.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(&g.source)
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
}
