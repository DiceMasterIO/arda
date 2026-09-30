//! Tactical ways for arda: world-scale roads and crossings turned into 5-ft
//! detail on a [`TacticalLayout`] window — road surfaces, verges, ditches,
//! benches and switchbacks, bridges, fords, ferries and toll houses
//! (goals 37, 43 and 46).
//!
//! [`apply_ways`] overlays the ways onto an existing window and returns a
//! [`Sidecar`] with the scene semantics `Square` cannot carry (bridge decks,
//! difficult terrain, which walls block sight). Everything is computed from
//! world coordinates, so a road crossing two adjacent windows is identical
//! at their shared edge. See `crates/arda-ways/README.md`.

#![deny(unsafe_code)]
// `code-prefs.md` §Q1 bans unwrap/expect *outside* `#[cfg(test)]`.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod curve;
pub mod error;
pub mod fallback;
pub mod input;
pub mod plan;
pub mod raster;
pub mod sidecar;

pub use error::WaysError;
pub use input::{
    Crossing, CrossingKind, FnTerrain, RiverChannel, Road, RoadClass, Terrain, SQUARE_M,
};
pub use sidecar::{Feature, Sidecar, SquareRules};

use arda_tactical::TacticalLayout;
use serde::Serialize;

/// A resolved crossing, for reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CrossingReport {
    /// Crossing id (a JSON string).
    #[serde(with = "input::id_serde")]
    pub id: u32,
    /// Kind.
    pub kind: CrossingKind,
    /// Stone arch (bridges only).
    pub stone: bool,
    /// Span axis: `east_west` or `north_south`.
    pub axis: &'static str,
    /// Squares of water crossed along the axis.
    pub water_squares: i64,
    /// Squares of the whole structure along the axis.
    pub span_squares: i64,
    /// Width across, in squares.
    pub width_squares: i64,
    /// Deck level in feet (bridges).
    pub deck_ft: i16,
}

/// What [`apply_ways`] did, beyond the sidecar.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct WaysReport {
    /// Ways (road segments) that reach the window.
    pub ways: u32,
    /// Crossings resolved near the window.
    pub crossings: Vec<CrossingReport>,
    /// Crossing ids near the window that met no way or no water.
    pub orphans: Vec<String>,
    /// Toll houses and waystations `(crossing id, function)`.
    pub houses: Vec<(String, &'static str)>,
    /// Pieces given switchbacks.
    pub switchbacks: u32,
    /// Pieces still over their class grade.
    pub over_grade: u32,
    /// Junctions near the window.
    pub junctions: u32,
}

/// The sidecar plus a report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaysOutput {
    /// Per-square scene semantics, same size as the layout.
    pub sidecar: Sidecar,
    /// The same rules as `arda-scene`'s `RulesSidecar` format 2 (A8, I9).
    pub rules: arda_scene::RulesSidecar,
    /// Summary.
    pub report: WaysReport,
}

/// Widest crossing accepted, metres (the widest rivers are a few km).
pub const MAX_CROSSING_WIDTH_M: u32 = 5_000;

/// Overlays roads and crossings onto a layout window whose north-west
/// corner is at `window_origin_m` (world metres, on the square lattice).
///
/// Squares untouched by a way keep their ground; touched ones get the
/// canonical vocabulary keys (`cobbles`, `flagstone`, `drystone`,
/// `prop.signpost`, …). Use [`fallback::degrade`] before rendering with a
/// library that lacks some of them. Channels from `terrain` are painted as
/// water first; crossings without a channel get a synthetic one.
///
/// # Errors
/// An off-lattice or out-of-range origin, a malformed layout, or a crossing
/// wider than [`MAX_CROSSING_WIDTH_M`].
pub fn apply_ways(
    layout: &mut TacticalLayout,
    window_origin_m: [f64; 2],
    roads: &[Road],
    crossings: &[Crossing],
    terrain: &dyn Terrain,
    seed: u64,
) -> Result<WaysOutput, WaysError> {
    if layout.squares.len() != layout.width as usize * layout.height as usize {
        return Err(WaysError::BadLayout(layout.name.clone()));
    }
    let win = plan::Window::new(window_origin_m, layout.width, layout.height)?;
    // A crossing's synthetic channel and blend scale with its width: an
    // unchecked u32 width meant a 1e10-square scan and billions of stations.
    if let Some(c) = crossings.iter().find(|c| c.width_m > MAX_CROSSING_WIDTH_M) {
        return Err(WaysError::Limit(format!(
            "crossing {} is {} m wide; the limit is {MAX_CROSSING_WIDTH_M} m",
            c.id, c.width_m
        )));
    }
    let plan = plan::build(win, roads, crossings, terrain, seed);
    let sidecar = raster::apply(layout, &plan, terrain, seed);
    // Convention I10: the layout carries its world origin in squares.
    layout.origin = Some([win.gx0, win.gy0]);
    let rules = sidecar.to_rules();
    let report = WaysReport {
        ways: u32::try_from(
            plan.ways
                .iter()
                .filter(|w| !w.dense.runs.is_empty())
                .count(),
        )
        .unwrap_or(u32::MAX),
        crossings: plan
            .crossings
            .iter()
            .map(|c| CrossingReport {
                id: c.id,
                kind: c.kind,
                stone: c.stone,
                axis: if c.east_west {
                    "east_west"
                } else {
                    "north_south"
                },
                water_squares: c.water.1 - c.water.0 + 1,
                span_squares: c.span.1 - c.span.0 + 1,
                width_squares: c.rows.1 - c.rows.0 + 1,
                deck_ft: c.deck_ft,
            })
            .collect(),
        orphans: plan.orphans.iter().map(ToString::to_string).collect(),
        houses: plan
            .houses
            .iter()
            .map(|h| (h.crossing.to_string(), h.function))
            .collect(),
        switchbacks: plan.ways.iter().map(|w| w.switchbacks).sum(),
        over_grade: plan.ways.iter().map(|w| w.over_grade).sum(),
        junctions: u32::try_from(plan.junctions.len()).unwrap_or(u32::MAX),
    };
    Ok(WaysOutput {
        sidecar,
        rules,
        report,
    })
}
