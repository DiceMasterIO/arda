//! A shareable `arda-refine` [`Source`]: one opened world read by the
//! block pipeline, the town terrain and the overlays alike.

use arda_refine::source::{Edge, LakeInfo};
use arda_refine::{CellKey, RefineError, Source};
use std::sync::Arc;

/// A world source behind an `Arc`, itself a [`Source`].
#[derive(Clone)]
pub struct SharedSource(pub Arc<dyn Source + Send + Sync>);

impl std::fmt::Debug for SharedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedSource")
            .field("cells", &self.0.cells_wide_high())
            .finish()
    }
}

impl SharedSource {
    /// Opens the world at `dir` (its fine terrain too, when declared).
    ///
    /// # Errors
    /// [`crate::PeopleError::World`] when the world cannot load.
    pub fn open(dir: &std::path::Path) -> Result<Self, crate::PeopleError> {
        let world = arda::World::load(dir).map_err(|e| crate::PeopleError::World(e.to_string()))?;
        let src = arda_refine::WorldSource::shared(Arc::new(world))
            .map_err(|e| crate::PeopleError::World(e.to_string()))?;
        Ok(Self(Arc::new(src)))
    }
}

impl Source for SharedSource {
    fn seed(&self) -> u64 {
        self.0.seed()
    }

    fn cells_wide_high(&self) -> (i64, i64) {
        self.0.cells_wide_high()
    }

    fn cell(&self, at: CellKey) -> Result<arda::Cell, RefineError> {
        self.0.cell(at)
    }

    fn lake(&self, at: CellKey) -> Result<Option<LakeInfo>, RefineError> {
        self.0.lake(at)
    }

    fn edges_touching(&self, at: CellKey) -> Result<Vec<Edge>, RefineError> {
        self.0.edges_touching(at)
    }

    fn fine_mm(&self, kx: i64, ky: i64) -> Result<Option<i32>, RefineError> {
        self.0.fine_mm(kx, ky)
    }
}
