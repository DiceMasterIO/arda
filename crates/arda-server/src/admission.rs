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

/// Bytes of town plans `arda-people` keeps (its `PLAN_CACHE_BYTES`, each
/// plan charged its square grid plus twice its JSON,
/// `arda_people::people::plan_bytes`). The newest plan stays even past
/// the budget, so the largest plan is added once ([`Resident::plans`]).
pub const PLAN_CACHE_BYTES: u64 = arda_people::people::PLAN_CACHE_BYTES;
/// Parsed `society/` files are charged at this multiple of their size.
pub const SOCIETY_FACTOR: u64 = 2;
/// An indexed `notables.json`'s header line (its index) is charged at
/// this multiple of its size: parsed, every notable id is a vector entry
/// and a map entry, about three times its text (assumed).
pub const NOTABLES_INDEX_FACTOR: u64 = 3;
/// Shore layers held: the refine source's (shared by its area sources)
/// and the relief world's.
pub const SHORE_COPIES: u64 = 2;

/// The tactical pipeline's fixed bounds: [`COMPOSE_SLOTS`] transient
/// compositions, the refined-block cache, the refine source's decoded
/// areas and the town-plan cache's byte budget.
#[must_use]
pub const fn pipeline_bytes() -> u64 {
    COMPOSE_SLOTS as u64 * COMPOSE_TRANSIENT_BYTES
        + REFINED_BLOCKS as u64 * REFINED_BLOCK_BYTES
        + refine_source::admitted_bytes()
        + PLAN_CACHE_BYTES
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
    /// The largest town plan, which the plan cache may hold beyond
    /// [`PLAN_CACHE_BYTES`].
    pub plans: u64,
}

impl Resident {
    /// Measures `library`, the `society/` directory under `world_dir`, a
    /// `cells` = (columns, rows) shore grid and the largest town plan
    /// `largest_plan` (`arda_people::people::plan_bytes`), if any.
    #[must_use]
    pub fn measure(
        library: &Library,
        world_dir: &Path,
        cells: (u32, u32),
        largest_plan: Option<u64>,
    ) -> Self {
        Self {
            library: library_bytes(library),
            society: society_bytes(&world_dir.join("society")),
            shore: u64::from(cells.0)
                * u64::from(cells.1)
                * std::mem::size_of::<arda_core::ShoreClass>() as u64
                * SHORE_COPIES,
            plans: largest_plan.unwrap_or(0),
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

/// The parsed `society/` files: each at [`SOCIETY_FACTOR`] times its size,
/// except an indexed `notables.json` (format 2, read lazily by
/// `arda_people::Notables`), whose header index is parsed at startup
/// ([`NOTABLES_INDEX_FACTOR`]) and whose settlement lines are parsed on
/// first use and then kept, so they stay charged at [`SOCIETY_FACTOR`]
/// (OI-51).
fn society_bytes(dir: &Path) -> u64 {
    let notables = dir.join("notables.json");
    let all = dir_bytes(dir);
    let Some(header) = indexed_header(&notables) else {
        return all.saturating_mul(SOCIETY_FACTOR);
    };
    let rest = all.saturating_sub(header);
    rest.saturating_mul(SOCIETY_FACTOR)
        .saturating_add(header.saturating_mul(NOTABLES_INDEX_FACTOR))
}

/// Bytes of the header line of an indexed `notables.json`, newline
/// included; `None` for a missing file or a format-1 one.
fn indexed_header(path: &Path) -> Option<u64> {
    use std::io::BufRead;
    let file = std::fs::File::open(path).ok()?;
    let mut line = Vec::new();
    std::io::BufReader::new(file)
        .read_until(b'\n', &mut line)
        .ok()?;
    let text = std::str::from_utf8(&line).ok()?;
    text.trim_end()
        .ends_with("\"settlements\":[")
        .then(|| u64::try_from(line.len()).unwrap_or(u64::MAX))
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
        assert!(p >= refine_source::admitted_bytes() + PLAN_CACHE_BYTES);
        assert!(p < 2 << 30, "{p}");
    }

    #[test]
    fn the_notables_index_and_its_lines_are_charged_apart() {
        // OI-51: an indexed `notables.json` holds its parsed header index
        // from startup and each settlement's line once it is asked for.
        let lib = Library::load_stack(&std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/tactical/placeholder"
        )))
        .unwrap();
        let dir = std::env::temp_dir().join(format!("arda-admission-{}", std::process::id()));
        let soc = dir.join("society");
        std::fs::create_dir_all(&soc).unwrap();
        let mut header = "{\"format_version\":2,\"index\":[],\"settlements\":[".to_owned();
        header.insert_str(1, &" ".repeat(1000 - header.len()));
        let body = "x".repeat(9000);
        std::fs::write(soc.join("notables.json"), format!("{header}\n{body}")).unwrap();
        std::fs::write(soc.join("settlements.json"), "y".repeat(500)).unwrap();
        let r = Resident::measure(&lib, &dir, (8, 8), None);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            r.society,
            500 * SOCIETY_FACTOR + 1001 * NOTABLES_INDEX_FACTOR + 9000 * SOCIETY_FACTOR
        );
    }

    #[test]
    fn the_largest_plan_is_charged_once_beyond_the_plan_budget() {
        let lib = Library::load_stack(&std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../assets/tactical/placeholder"
        )))
        .unwrap();
        let none = Resident::measure(&lib, Path::new("/nonexistent"), (8, 8), None);
        assert_eq!(none.plans, 0);
        let big = Resident::measure(&lib, Path::new("/nonexistent"), (8, 8), Some(500 << 20));
        assert_eq!(big.plans, 500 << 20);
        assert!(big.library > 0 && big.shore == 64 * SHORE_COPIES);
    }
}
