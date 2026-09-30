//! Mid-zoom relief tiles (`/v1/tiles/relief/{z}/{x}/{y}.webp`): levels past
//! the overview pyramid's native zoom, rendered on demand by arda-midzoom
//! from ~10 m refined heights, cached and deterministic, joining exactly.

use crate::cache::ByteLru;
use crate::error::{lock, ServerError, ServerResult};
use arda_midzoom::pyramid::MAX_EXTRA_LEVELS;
use arda_midzoom::{MidzoomError, Pyramid, ReliefWorld};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Relief limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReliefLimits {
    /// Byte budget for encoded relief tiles.
    pub cache_bytes: usize,
}

impl Default for ReliefLimits {
    fn default() -> Self {
        Self {
            cache_bytes: 128 << 20,
        }
    }
}

/// Working set beyond the tile cache: a dozen area Atlas contexts, fine
/// chunks, area facts and one render in flight per worker, rounded up.
pub const RELIEF_WORKING_BYTES: u64 = 768 << 20;

/// Relief levels of one served world.
pub struct Relief {
    world: Option<ReliefWorld>,
    pyramid: Pyramid,
    tiles: Mutex<ByteLru<(u32, u32, u32), Vec<u8>>>,
}

impl std::fmt::Debug for Relief {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Relief")
            .field("pyramid", &self.pyramid)
            .field("enabled", &self.world.is_some())
            .finish_non_exhaustive()
    }
}

fn internal(e: &MidzoomError) -> ServerError {
    match e {
        MidzoomError::OutOfPyramid { .. } => ServerError::NotFound(e.to_string()),
        MidzoomError::ResourceLimit(_) => ServerError::ResourceLimit(e.to_string()),
        _ => ServerError::Internal(format!("relief: {e}")),
    }
}

impl Relief {
    /// Opens relief levels for the world at `dir` below overview zoom
    /// `max_zoom`; worlds without recipe-5 fine terrain serve none.
    ///
    /// # Errors
    /// The world or its fine layer failed to load.
    pub fn open(dir: &Path, max_zoom: u32, limits: ReliefLimits) -> ServerResult<Self> {
        let world = Arc::new(arda::World::load(dir)?);
        let m = world.manifest();
        let pyramid = Pyramid {
            max_zoom,
            areas_wide: m.areas_wide,
            areas_high: m.areas_high,
        };
        let formed = m.fine_terrain.is_some_and(|f| f.recipe_version >= 5);
        let world = if formed {
            Some(ReliefWorld::new(world).map_err(|e| internal(&e))?)
        } else {
            None
        };
        Ok(Self {
            world,
            pyramid,
            tiles: Mutex::new(ByteLru::new(limits.cache_bytes)),
        })
    }

    /// Deepest relief zoom (the overview's own when relief is unavailable).
    #[must_use]
    pub const fn max_zoom(&self) -> u32 {
        if self.world.is_some() {
            self.pyramid.max_zoom + MAX_EXTRA_LEVELS
        } else {
            self.pyramid.max_zoom
        }
    }

    /// The lossless WebP of relief tile `(z, x, y)`, rendering on a miss.
    ///
    /// # Errors
    /// [`ServerError::NotFound`] outside the relief levels; render failures.
    pub fn tile(&self, z: u32, x: u32, y: u32) -> ServerResult<Arc<Vec<u8>>> {
        let world = self.world.as_ref().ok_or_else(|| {
            ServerError::NotFound("this world has no formed fine terrain for relief".into())
        })?;
        self.pyramid.check(z, x, y).map_err(|e| internal(&e))?;
        let key = (z, x, y);
        if let Some(hit) = lock(&self.tiles)?.get(&key) {
            return Ok(hit);
        }
        let rgba =
            arda_midzoom::render_tile(world, &self.pyramid, z, x, y).map_err(|e| internal(&e))?;
        let webp = Arc::new(crate::tactical::images::encode_webp(
            &rgba.pixels,
            rgba.width,
        )?);
        let size = webp.len();
        Ok(lock(&self.tiles)?.insert(key, webp, size))
    }
}
