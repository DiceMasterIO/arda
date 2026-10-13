//! The gazetteer: the world's large natural features, named, placed and
//! ranked (`society/gazetteer.json`, logic/15 §name-what-is-named).
//!
//! Detection works on a coarse raster of the [`Grid`] ([`coarse`]), so a
//! 50-million-cell world stays cheap, and uses no randomness at all: the
//! same grid gives the same features. Names come last, from the same
//! world-wide scope as every other name, in this fixed order after the
//! regions: seas and bays, lakes, islands, capes, forests, marshes, mountain
//! ranges; within a kind, biggest first. Nothing named earlier changes.
//!
//! Features by kind and the size from which they count:
//!
//! | kind | rule | default size |
//! | --- | --- | --- |
//! | sea (or gulf) | connected sea | 2,000 km² |
//! | bay | core of a sea cut off by a narrow mouth | 30 km² |
//! | lake | connected lake | 5 km² |
//! | island | land other than the largest body | 4 km² |
//! | cape | land too narrow for a 3.2 km disc, joined to land that is not | 5 km² |
//! | forest | connected closed canopy | 100 km² |
//! | marsh | connected marsh | 20 km² |
//! | mountain | three or more linked summits within 40 km | |
//!
//! Rivers are listed from `names.json`, with their courses as label paths.

mod coarse;
mod detect;
mod list;
mod name;
mod range;
mod shape;

use crate::features::Peak;
use crate::grid::Grid;
use crate::num::ui;
pub use arda_names::PlaceKind;
use serde::{Deserialize, Serialize};

pub use list::{list, points, tier_rank, Filter, GazetteerItem, GazetteerList, GazetteerPoint};
pub use name::name_features;

/// `gazetteer.json` format version.
pub const FORMAT_VERSION: u32 = 1;

/// Sizes and limits for feature detection. Hectares are world cells.
#[derive(Debug, Clone, Copy)]
pub struct Thresholds {
    /// Smallest sea.
    pub sea_ha: u64,
    /// Smallest bay or gulf.
    pub bay_ha: u64,
    /// Smallest lake.
    pub lake_ha: u64,
    /// Smallest island.
    pub island_ha: u64,
    /// Smallest cape.
    pub cape_ha: u64,
    /// Smallest forest.
    pub forest_ha: u64,
    /// Smallest marsh.
    pub marsh_ha: u64,
    /// Radius of the disc that separates a bay from its sea, coarse cells.
    pub bay_radius: usize,
    /// Radius of the disc a cape is too narrow for, coarse cells.
    pub cape_radius: usize,
    /// Summits farther apart than this are never one range, metres.
    pub range_link_m: i64,
    /// Fewest summits in a range.
    pub range_min_peaks: usize,
    /// Most seas kept.
    pub max_seas: usize,
    /// Most bays kept.
    pub max_bays: usize,
    /// Most lakes kept.
    pub max_lakes: usize,
    /// Most islands kept.
    pub max_islands: usize,
    /// Most capes kept.
    pub max_capes: usize,
    /// Most forests kept.
    pub max_forests: usize,
    /// Most marshes kept.
    pub max_marshes: usize,
    /// Most ranges kept.
    pub max_ranges: usize,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            sea_ha: 200_000,
            bay_ha: 3_000,
            lake_ha: 500,
            island_ha: 400,
            cape_ha: 500,
            forest_ha: 10_000,
            marsh_ha: 2_000,
            bay_radius: 8,
            cape_radius: 8,
            range_link_m: 40_000,
            range_min_peaks: 3,
            max_seas: 8,
            max_bays: 16,
            max_lakes: 48,
            max_islands: 40,
            max_capes: 24,
            max_forests: 32,
            max_marshes: 24,
            max_ranges: 16,
        }
    }
}

/// A feature found, before it has a name.
#[derive(Debug, Clone)]
pub struct Found {
    /// What it is (`Mountain` is a range).
    pub kind: PlaceKind,
    /// Label anchor, metres.
    pub anchor_m: [i64; 2],
    /// Label path for long features, metres.
    pub path_m: Option<Vec<[i64; 2]>>,
    /// Bounding box, metres: `x0, y0, x1, y1`.
    pub extent_m: [i64; 4],
    /// Area, hectares (0 for ranges and rivers).
    pub area_ha: u64,
    /// Length, metres.
    pub length_m: u64,
    /// Summits in a range, Strahler order of a river, else 0.
    pub detail: u32,
}

/// One entry of `society/gazetteer.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GazetteerFeature {
    /// Stable id, 1-based in file order.
    #[serde(with = "crate::ids::string")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub id: u64,
    /// `sea`, `bay`, `lake`, `island`, `cape`, `forest`, `marsh`,
    /// `mountain` (a range) or `river`.
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub kind: PlaceKind,
    /// Native name.
    pub name: String,
    /// English gloss.
    pub name_gloss: String,
    /// Culture key of the speech at the anchor.
    pub culture: String,
    /// Label anchor, metres on the world grid.
    pub anchor_m: [i64; 2],
    /// Label path (at most 24 points, reading left to right) for long
    /// features; the course of a river, from its mouth.
    pub path_m: Option<Vec<[i64; 2]>>,
    /// Bounding box, metres: `x0, y0, x1, y1`.
    pub extent_m: [i64; 4],
    /// Area, hectares (0 for ranges and rivers).
    pub area_ha: u64,
    /// Length of the path or main axis, metres.
    pub length_m: u64,
    /// Importance, 1 (largest) to 5.
    pub rank: u8,
    /// Zoom from which a map shows the label (0 is the whole world).
    pub min_zoom: u8,
}

/// `gazetteer.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GazetteerFile {
    /// Format version.
    pub format_version: u32,
    /// Features, by kind then rank.
    pub features: Vec<GazetteerFeature>,
}

/// Every feature of `g` in naming order. `peaks` are the world's named
/// summits.
#[must_use]
pub fn find(g: &Grid, peaks: &[Peak], t: &Thresholds) -> Vec<Found> {
    let co = coarse::Coarse::of(g);
    let cx = detect::Ctx {
        co: &co,
        g,
        world_m: [ui(g.width) * 100, ui(g.height) * 100],
    };
    let mut out = detect::seas(&cx, t);
    out.extend(detect::lakes(&cx, t));
    out.extend(detect::islands(&cx, t));
    out.extend(detect::capes(&cx, t));
    out.extend(detect::forests(&cx, t));
    out.extend(detect::marshes(&cx, t));
    out.extend(range::ranges(g, peaks, t));
    out
}

/// Importance of a feature, 1 (largest) to 5.
#[must_use]
pub fn rank_of(kind: PlaceKind, area_ha: u64, detail: u32) -> u8 {
    match kind {
        PlaceKind::Sea => {
            if area_ha >= 1_000_000 {
                1
            } else {
                2
            }
        }
        PlaceKind::Bay => {
            if area_ha >= 50_000 {
                3
            } else {
                4
            }
        }
        PlaceKind::Island => match area_ha {
            500_000.. => 2,
            20_000.. => 3,
            _ => 4,
        },
        PlaceKind::Lake => match area_ha {
            20_000.. => 3,
            2_000.. => 4,
            _ => 5,
        },
        PlaceKind::Forest => {
            if area_ha >= 200_000 {
                3
            } else {
                4
            }
        }
        PlaceKind::Marsh => {
            if area_ha >= 20_000 {
                4
            } else {
                5
            }
        }
        PlaceKind::Mountain => match detail {
            8.. => 2,
            5.. => 3,
            _ => 4,
        },
        PlaceKind::River => match detail {
            6.. => 2,
            5 => 3,
            4 => 4,
            _ => 5,
        },
        _ => 4,
    }
}

/// Zoom from which a label of `rank` is shown.
#[must_use]
pub const fn min_zoom_of(rank: u8) -> u8 {
    rank.saturating_sub(1)
}

/// Position of a kind in the file.
pub(crate) const fn kind_order(kind: PlaceKind) -> u8 {
    match kind {
        PlaceKind::Sea => 0,
        PlaceKind::Bay => 1,
        PlaceKind::Mountain => 2,
        PlaceKind::Island => 3,
        PlaceKind::Lake => 4,
        PlaceKind::River => 5,
        PlaceKind::Forest => 6,
        PlaceKind::Marsh => 7,
        _ => 8,
    }
}
