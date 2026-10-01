//! A stored world prepared for relief tiles: the fine source plus bounded
//! caches of per-area Atlas contexts and channel edges (pure reads only).

use crate::source::{AreaFacts, WorldTerrain};
use crate::MidzoomError;
use arda::World;
use arda_render::AtlasTerrain;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// One saved area edge, micrometres.
pub const AREA_UM: i64 = 51_200_000_000;
/// Atlas contexts kept at once (each ≈ 15 MB).
const MAX_CONTEXTS: usize = 12;

type Contexts = BTreeMap<(i32, i32), Arc<AtlasTerrain>>;
/// The world's stored shore layer, read on first use (`None` inside when
/// the world has none).
type Shore = Option<Arc<Option<arda_core::ShoreLayer>>>;

/// A world ready for relief rendering.
pub struct ReliefWorld {
    terrain: WorldTerrain,
    /// The tactical layer's view of the world, which relief water is
    /// drawn from (logic/17 §water).
    water: arda_refine::WorldSource<'static>,
    contexts: Mutex<Contexts>,
    /// Serialises context builds so peak memory stays bounded; it also
    /// holds the shore layer, which every context paints from.
    build: Mutex<Shore>,
}

impl std::fmt::Debug for ReliefWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReliefWorld")
            .field("terrain", &self.terrain)
            .finish_non_exhaustive()
    }
}

impl ReliefWorld {
    /// Wraps a loaded world with a formed fine layer.
    ///
    /// # Errors
    /// No fine terrain layer, or it is unreadable.
    pub fn new(world: Arc<World>) -> Result<Self, MidzoomError> {
        Ok(Self {
            water: arda_refine::WorldSource::shared(Arc::clone(&world))?,
            terrain: WorldTerrain::new(world)?,
            contexts: Mutex::new(BTreeMap::new()),
            build: Mutex::new(None),
        })
    }

    /// The fine source.
    #[must_use]
    pub const fn terrain(&self) -> &WorldTerrain {
        &self.terrain
    }

    /// The world as the tactical layer reads it.
    #[must_use]
    pub const fn water_source(&self) -> &arda_refine::WorldSource<'static> {
        &self.water
    }

    /// The world.
    #[must_use]
    pub fn world(&self) -> &World {
        self.terrain.world()
    }

    /// Areas `(wide, high)`.
    #[must_use]
    pub fn areas(&self) -> (i32, i32) {
        let m = self.world().manifest();
        (m.areas_wide, m.areas_high)
    }

    /// The Atlas context of area `(ax, ay)`, built on first use.
    ///
    /// # Errors
    /// The area is outside the world or a layer failed.
    pub fn context(&self, ax: i32, ay: i32) -> Result<Arc<AtlasTerrain>, MidzoomError> {
        if let Some(c) = self.cached(ax, ay)? {
            return Ok(c);
        }
        let mut shore = self.build.lock().map_err(|_| MidzoomError::Poisoned)?;
        if let Some(c) = self.cached(ax, ay)? {
            return Ok(c);
        }
        let layer = match shore.as_ref() {
            Some(layer) => Arc::clone(layer),
            None => Arc::clone(shore.insert(Arc::new(self.world().shore()?))),
        };
        // logic/04 §atlas-formed shore: relief shades coasts like the overview.
        let built = Arc::new(arda::area_atlas_terrain(
            self.world(),
            ax,
            ay,
            layer.as_ref().as_ref(),
        )?);
        let mut cache = self.contexts.lock().map_err(|_| MidzoomError::Poisoned)?;
        if cache.len() >= MAX_CONTEXTS {
            cache.clear();
        }
        cache.insert((ax, ay), Arc::clone(&built));
        Ok(built)
    }

    fn cached(&self, ax: i32, ay: i32) -> Result<Option<Arc<AtlasTerrain>>, MidzoomError> {
        Ok(self
            .contexts
            .lock()
            .map_err(|_| MidzoomError::Poisoned)?
            .get(&(ax, ay))
            .cloned())
    }

    /// Saved facts (cells and channel edges) of area `(ax, ay)`.
    ///
    /// # Errors
    /// The area failed to load.
    pub fn channel_edges(&self, ax: i32, ay: i32) -> Result<Arc<AreaFacts>, MidzoomError> {
        self.terrain.area(ax, ay)
    }
}
