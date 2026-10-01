//! Arda's world-query contract and its HTTP service (goal-prompt goals 65–66).
//!
//! [`contract`] defines the versioned [`contract::CellSample`] every consumer
//! reads; [`query::WorldQuery`] produces it from a stored world with bounded
//! caches; [`routes::router`] serves it under `/v1`, together with the NPC
//! routes of [`npc`]. [`tactical`] serves battle maps under `/v1/tactical`.
//! See `API.md`.

// `code-prefs.md` §Q1 bans unwrap/expect *outside* `#[cfg(test)]`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod bindings;
pub mod cache;
pub mod columnar;
pub mod contract;
pub mod derived;
pub mod dto;
pub mod error;
pub mod fine;
pub mod npc;
pub mod npc_dto;
pub mod npcs;
pub mod overview;
pub mod people;
pub mod query;
pub mod relief;
pub mod routes;
pub mod schema;
pub mod serve;
pub mod sheet_map;
pub mod society_cells;
pub mod tactical;
pub mod tiles;

pub use contract::{CellSample, PointSample, CONTRACT_VERSION};
pub use error::{ServerError, ServerResult};
pub use overview::OverviewLimits;
pub use query::{QueryLimits, WorldQuery};
pub use relief::ReliefLimits;
pub use routes::router;
pub use tactical::block::{BlockError, BlockSource};
pub use tactical::{Tactical, TacticalLimits};

use std::path::PathBuf;

/// The goal-prompt §8 memory ceiling; configured budgets must fit inside it.
pub const MEMORY_CEILING_BYTES: u64 = 16 << 30;

/// Everything needed to serve one world.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// World directory (holds `world.json`).
    pub world: PathBuf,
    /// Query cache budgets.
    pub query: QueryLimits,
    /// Overview limits and cache budgets.
    pub overview: OverviewLimits,
    /// Tactical asset library directory (`--library`).
    pub library: PathBuf,
    /// Tactical request limits and cache budgets.
    pub tactical: TacticalLimits,
    /// Mid-zoom relief tile cache budget.
    pub relief: ReliefLimits,
    /// Area bodies built at once; further `/area/.../cells` requests wait.
    pub area_builds: usize,
    /// Connection limits.
    pub serve: serve::ServeLimits,
    /// Tactical prefetch worker threads (goal 67; 0 disables prefetch).
    pub prefetch_workers: usize,
    /// Sheet mapping file (`--sheet-mapping`, goal 69): reshapes every NPC
    /// the server sends; `None` serves Arda's own shape.
    pub sheet_mapping: Option<PathBuf>,
}

/// Worst-case transient bytes of one `/area/.../cells` build: 262,144
/// contract samples, their columns and the encoded body.
pub const AREA_BUILD_BYTES: u64 = 512 << 20;

/// Largest response body a connection can hold while a client reads it
/// (the ~38 MB JSON area body, rounded up).
pub const MAX_BODY_BYTES: u64 = 48 << 20;

impl ServerConfig {
    /// Default limits for `world`.
    #[must_use]
    pub fn new(world: PathBuf) -> Self {
        Self {
            world,
            query: QueryLimits::default(),
            overview: OverviewLimits::default(),
            library: PathBuf::from(tactical::DEFAULT_LIBRARY),
            tactical: TacticalLimits::default(),
            relief: ReliefLimits::default(),
            area_builds: 2,
            serve: serve::ServeLimits::default(),
            prefetch_workers: tactical::prefetch::DEFAULT_WORKERS,
            sheet_mapping: None,
        }
    }

    /// Worst-case resident bytes: every cache full, the decoded pyramid base,
    /// `area_builds` transient area builds (samples, columns, encoding, and
    /// the 3 × 3 neighbourhood load for coast derivation), one buffered
    /// response body per open connection, and the tactical caches plus one
    /// transient render in the request lane and, with prefetch workers, one
    /// in the prefetch lane.
    #[must_use]
    pub fn admitted_bytes(&self) -> u64 {
        let base = u64::from(self.overview.tile_base_px).pow(2) * 3;
        let caches = self.query.total() as u64 + 2 * self.overview.cache_bytes as u64;
        let builds = self.area_builds.max(1) as u64 * AREA_BUILD_BYTES;
        let bodies = self.serve.max_connections.max(1) as u64 * MAX_BODY_BYTES;
        caches
            .saturating_add(base)
            .saturating_add(builds)
            .saturating_add(bodies)
            .saturating_add(self.tactical.admitted_bytes())
            .saturating_add(if self.prefetch_workers > 0 {
                self.tactical.transient_render_bytes()
            } else {
                0
            })
            .saturating_add(self.relief.cache_bytes as u64 + relief::RELIEF_WORKING_BYTES)
    }

    /// Refuses configurations whose worst case exceeds [`MEMORY_CEILING_BYTES`].
    ///
    /// # Errors
    /// [`ServerError::ResourceLimit`].
    pub fn admit(&self) -> ServerResult<()> {
        let need = self.admitted_bytes();
        if need > MEMORY_CEILING_BYTES {
            return Err(ServerError::ResourceLimit(format!(
                "configured budgets need {need} bytes, above the {MEMORY_CEILING_BYTES}-byte ceiling"
            )));
        }
        Ok(())
    }
}

/// Shared service state.
#[derive(Debug)]
pub struct AppState {
    /// Contract queries.
    pub query: WorldQuery,
    /// Overview renders and tiles.
    pub overview: overview::Overview,
    /// NPC generation and the demo town.
    pub npc: npc::NpcService,
    /// Tactical library, layouts and render caches.
    pub tactical: Tactical,
    /// Mid-zoom relief tiles past the overview's native zoom.
    pub relief: relief::Relief,
    /// Gate on concurrent area builds ([`ServerConfig::area_builds`] permits).
    pub area_builds: tokio::sync::Semaphore,
    /// The world's settlements, plans and stored notables, when it has a
    /// `society/` directory (`arda settle`, `arda society build`).
    pub people: Option<std::sync::Arc<arda_people::World>>,
    /// The tactical prefetch queue and workers.
    pub prefetch: tactical::prefetch::Prefetcher,
    /// The checked `--sheet-mapping`, applied to every NPC served.
    pub sheet_mapping: Option<sheet_map::SheetMapping>,
}

impl AppState {
    /// Admits the configuration, loads and validates the tactical library,
    /// then opens the world.
    ///
    /// # Errors
    /// Admission, library validation, world loading or limit failures.
    pub fn open(config: &ServerConfig) -> ServerResult<Self> {
        config.admit()?;
        // Goal 69: a bad mapping refuses startup before the world loads.
        let sheet_mapping = config
            .sheet_mapping
            .as_deref()
            .map(sheet_map::SheetMapping::load)
            .transpose()?;
        let query = WorldQuery::open(&config.world, config.query)?;
        let mut tactical = Tactical::open(&config.library, query.world().seed(), config.tactical)?;
        // Adapter A9: world-derived blocks from arda-refine composed with the
        // world's society overlays (logic/16 §api-tactical);
        // `with_block_source` can replace them.
        let src = arda_people::shared::SharedSource::open(&config.world)
            .map_err(|e| ServerError::Internal(e.to_string()))?;
        let society_dir = config.world.join("society");
        let people = if society_dir.join("settlements.json").exists() {
            Some(std::sync::Arc::new(
                arda_people::World::with_source(src.clone(), &society_dir)
                    .map_err(|e| ServerError::Internal(e.to_string()))?,
            ))
        } else {
            None
        };
        if let Some(w) = &people {
            // Towns and cities have the costliest plans: draw them now, so
            // their first tactical block is quick (goal 50).
            w.warm_towns();
        }
        let overlays = people
            .as_ref()
            .map(|w| arda_blocks::society::SocietyOverlays::open(std::sync::Arc::clone(w)))
            .transpose()
            .map_err(|e| ServerError::Internal(e.to_string()))?
            .map(std::sync::Arc::new);
        tactical.set_block_source(Box::new(
            tactical::refine_blocks::RefineBlocks::with_society(src, overlays),
        ));
        let overview = overview::Overview::new(&query, config.overview)?;
        let relief =
            relief::Relief::open(&config.world, overview.pyramid().max_zoom, config.relief)?;
        if let Some(world) = relief.shared_world() {
            tactical.set_tint_source(world);
        }
        Ok(Self {
            query,
            overview,
            relief,
            npc: npc::NpcService::default(),
            tactical,
            area_builds: tokio::sync::Semaphore::new(config.area_builds.max(1)),
            people,
            prefetch: tactical::prefetch::Prefetcher::new(config.prefetch_workers),
            sheet_mapping,
        })
    }

    /// Replaces the sheet mapping (tests and embedders).
    #[must_use]
    pub fn with_sheet_mapping(mut self, mapping: Option<sheet_map::SheetMapping>) -> Self {
        self.sheet_mapping = mapping;
        self
    }

    /// Sends a body holding NPCs through the sheet mapping, if any.
    ///
    /// # Errors
    /// Serialisation or mapping failures.
    pub fn npc_response<T: serde::Serialize>(
        &self,
        body: &T,
        at: sheet_map::NpcAt,
    ) -> ServerResult<axum::response::Response> {
        sheet_map::respond(self.sheet_mapping.as_ref(), body, at)
    }

    /// Plugs in the source of world-derived tactical blocks.
    #[must_use]
    pub fn with_block_source(mut self, source: Box<dyn BlockSource>) -> Self {
        self.tactical.set_block_source(source);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_budgets_fit_the_ceiling_and_oversized_ones_are_refused() {
        let config = ServerConfig::new(PathBuf::from("unused"));
        assert!(config.admit().is_ok());
        // Caches, the pyramid base, two area builds, 64 buffered bodies,
        // the tactical caches sized for world cells at 128 px per square
        // (logic/16 §api-cache: renders and tiles within 4 GiB), one
        // transient render per lane (request and prefetch) and the one
        // transient encode both lanes share (review round 2 #32).
        assert!(config.admitted_bytes() < 15 << 30);
        assert!(config.tactical.admitted_bytes() < 6 << 30);
        assert!(
            config.tactical.admitted_bytes()
                > config.tactical.transient_render_bytes()
                    + config.tactical.transient_encode_bytes()
        );
        let mut big = config.clone();
        big.query.area_bytes = 16 << 30;
        assert!(matches!(big.admit(), Err(ServerError::ResourceLimit(_))));
    }

    #[test]
    fn prefetch_workers_admit_one_more_transient_render() {
        let mut off = ServerConfig::new(PathBuf::from("unused"));
        off.prefetch_workers = 0;
        let on = ServerConfig::new(PathBuf::from("unused"));
        assert_eq!(
            on.admitted_bytes() - off.admitted_bytes(),
            on.tactical.transient_render_bytes()
        );
        assert!(on.admit().is_ok());
    }

    #[test]
    fn admission_counts_every_concurrent_area_build_and_buffered_body() {
        let config = ServerConfig::new(PathBuf::from("unused"));
        let mut builds = config.clone();
        builds.area_builds = 40;
        assert!(matches!(builds.admit(), Err(ServerError::ResourceLimit(_))));
        let mut connections = config;
        connections.serve.max_connections = 1_000;
        assert!(matches!(
            connections.admit(),
            Err(ServerError::ResourceLimit(_))
        ));
    }
}
