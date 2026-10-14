//! Memory admission beyond the configured caches (review 3.6-8; goal-prompt
//! §8, logic/16 Invariant 6).
//!
//! [`crate::ServerConfig::admitted_bytes`] sums the configured budgets.
//! The server also holds things no flag sizes: the tactical pipeline
//! (concurrent compositions, the refined-block cache, the refine source's
//! decoded areas, town plans), and what the world and library bring (the
//! decoded library and its scaled textures, the parsed `society/` files,
//! the shore layer). The first group has fixed bounds, counted at
//! configuration time ([`pipeline_bytes`]); the second is measured once
//! the world and library are open ([`Resident::measure`]) and
//! [`crate::AppState::open`] refuses to start when the sum would exceed
//! the ceiling.

use crate::refine_source;
use crate::tactical::refine_blocks::{
    COMPOSE_SLOTS, COMPOSE_TRANSIENT_BYTES, REFINED_BLOCKS, REFINED_BLOCK_BYTES,
};
use crate::tactical::world_routes::WINDOW_PPSQ;
use crate::tactical::PPSQ_OPTIONS;
use arda_tactical::Library;
use std::path::Path;

/// Town plans `arda-people` keeps (its `PLAN_CACHE`).
pub const PLAN_CACHE: u64 = 64;
/// Bound on one resident town plan. The largest MICRO city (population
/// 9,293) serialises to 1.7 MB; a plan in memory is charged at twice its
/// JSON. A world whose largest plan is bigger is charged its own size
/// ([`Resident::plans`]).
pub const PLAN_BYTES: u64 = 4 << 20;
/// Parsed `society/` files are charged at this multiple of their size.
pub const SOCIETY_FACTOR: u64 = 2;
/// Shore layers held: two refine-source generations and the relief world.
pub const SHORE_COPIES: u64 = 3;

/// The tactical pipeline's fixed bounds: [`COMPOSE_SLOTS`] transient
/// compositions, the refined-block cache, the refine source's two
/// generations of decoded areas and the town-plan cache.
#[must_use]
pub const fn pipeline_bytes() -> u64 {
    COMPOSE_SLOTS as u64 * COMPOSE_TRANSIENT_BYTES
        + REFINED_BLOCKS as u64 * REFINED_BLOCK_BYTES
        + refine_source::admitted_bytes()
        + PLAN_CACHE * PLAN_BYTES
}

/// What the opened world and library hold, measured at startup.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Resident {
    /// Decoded library images and the scaled texture sets of every served
    /// ppsq.
    pub library: u64,
    /// The parsed `society/` files.
    pub society: u64,
    /// The world's shore layer, [`SHORE_COPIES`] times.
    pub shore: u64,
    /// Town plans beyond the [`PLAN_BYTES`] bound already admitted.
    pub plans: u64,
}

impl Resident {
    /// Measures `library`, the `society/` directory under `world_dir`, a
    /// `cells` = (columns, rows) shore grid and the largest town plan
    /// `largest_plan` (serialised bytes), if any.
    #[must_use]
    pub fn measure(
        library: &Library,
        world_dir: &Path,
        cells: (u32, u32),
        largest_plan: Option<u64>,
    ) -> Self {
        let plan = largest_plan.map_or(0, |b| b.saturating_mul(2));
        Self {
            library: library_bytes(library),
            society: dir_bytes(&world_dir.join("society")).saturating_mul(SOCIETY_FACTOR),
            shore: u64::from(cells.0)
                * u64::from(cells.1)
                * std::mem::size_of::<arda_core::ShoreClass>() as u64
                * SHORE_COPIES,
            plans: plan.saturating_sub(PLAN_BYTES).saturating_mul(PLAN_CACHE),
        }
    }

    /// Sum of the terms.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.library + self.society + self.shore + self.plans
    }
}

/// Decoded images plus every served ppsq's scaled ground and water
/// textures (4 bytes a pixel).
#[must_use]
pub fn library_bytes(library: &Library) -> u64 {
    let assets = &library.catalog.assets;
    let images: u64 = assets
        .iter()
        .filter_map(|a| library.image(&a.id))
        .map(|img| img.data.len() as u64)
        .sum();
    let textures: u64 = PPSQ_OPTIONS
        .iter()
        .chain(&WINDOW_PPSQ)
        .map(|&p| {
            assets
                .iter()
                .filter(|a| a.class.is_texture())
                .map(|a| u64::from(a.footprint.w * p) * u64::from(a.footprint.h * p) * 4)
                .sum::<u64>()
        })
        .sum();
    images + textures
}

fn dir_bytes(dir: &Path) -> u64 {
    std::fs::read_dir(dir).map_or(0, |entries| {
        entries
            .filter_map(Result::ok)
            .filter_map(|e| e.metadata().ok())
            .filter(std::fs::Metadata::is_file)
            .map(|m| m.len())
            .sum()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pipeline_bound_counts_every_holder() {
        let p = pipeline_bytes();
        assert!(p >= COMPOSE_SLOTS as u64 * COMPOSE_TRANSIENT_BYTES);
        assert!(p >= refine_source::admitted_bytes() + PLAN_CACHE * PLAN_BYTES);
        assert!(p < 2 << 30, "{p}");
    }

    #[test]
    fn a_large_plan_is_charged_its_own_size() {
        let lib = Library::load_stack(&std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/tactical/placeholder"
        )))
        .unwrap();
        let small = Resident::measure(&lib, Path::new("/nonexistent"), (8, 8), Some(1 << 20));
        assert_eq!(small.plans, 0);
        let big = Resident::measure(&lib, Path::new("/nonexistent"), (8, 8), Some(4 << 20));
        assert_eq!(big.plans, (4 << 20) * PLAN_CACHE);
        assert!(big.library > 0 && big.shore == 64 * SHORE_COPIES);
    }
}
