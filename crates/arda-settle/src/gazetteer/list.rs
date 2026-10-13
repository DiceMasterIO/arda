//! The served list: features and settlements together, filtered
//! (`GET /v1/gazetteer`, logic/16 §api-gazetteer).

use super::{GazetteerFeature, GazetteerFile};
use crate::model::{Settlement, Tier};
use crate::naming::kind_of;
use arda_names::PlaceKind;
use serde::{Deserialize, Serialize};

/// A settlement as a map point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GazetteerPoint {
    /// The settlement's id (a decimal string).
    #[serde(with = "crate::ids::string")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub id: u64,
    /// `hamlet`, `village`, `town` or `city`.
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub kind: PlaceKind,
    /// Name.
    pub name: String,
    /// English gloss.
    pub name_gloss: String,
    /// Culture key.
    pub culture: String,
    /// Centre, metres on the world grid.
    pub at_m: [i64; 2],
    /// Importance, 2 (city) to 5 (hamlet).
    pub rank: u8,
    /// Zoom from which a map shows the label.
    pub min_zoom: u8,
}

/// One entry of the served list: told apart by `kind` (a natural feature
/// has `anchor_m`, a settlement `at_m`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum GazetteerItem {
    /// A natural feature.
    Feature(GazetteerFeature),
    /// A settlement.
    Point(GazetteerPoint),
}

impl GazetteerItem {
    /// Importance, 1 to 5.
    #[must_use]
    pub fn rank(&self) -> u8 {
        match self {
            Self::Feature(f) => f.rank,
            Self::Point(p) => p.rank,
        }
    }

    /// Kind.
    #[must_use]
    pub fn kind(&self) -> PlaceKind {
        match self {
            Self::Feature(f) => f.kind,
            Self::Point(p) => p.kind,
        }
    }

    /// Bounding box, metres.
    #[must_use]
    pub fn extent_m(&self) -> [i64; 4] {
        match self {
            Self::Feature(f) => f.extent_m,
            Self::Point(p) => [p.at_m[0], p.at_m[1], p.at_m[0], p.at_m[1]],
        }
    }

    /// Zoom from which a map shows the label.
    #[must_use]
    pub fn min_zoom(&self) -> u8 {
        match self {
            Self::Feature(f) => f.min_zoom,
            Self::Point(p) => p.min_zoom,
        }
    }
}

/// `GET /v1/gazetteer` body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct GazetteerList {
    /// Format version (1).
    pub format_version: u32,
    /// Natural features, then settlements by id.
    pub features: Vec<GazetteerItem>,
}

/// Rank and first zoom of a settlement tier.
#[must_use]
pub const fn tier_rank(t: Tier) -> (u8, u8) {
    match t {
        Tier::City => (2, 1),
        Tier::Town => (3, 2),
        Tier::Village => (4, 4),
        Tier::Hamlet => (5, 6),
    }
}

/// The settlements as map points, in id order.
#[must_use]
pub fn points(settlements: &[Settlement]) -> Vec<GazetteerPoint> {
    let mut out: Vec<GazetteerPoint> = settlements
        .iter()
        .map(|s| {
            let (rank, min_zoom) = tier_rank(s.tier);
            GazetteerPoint {
                id: s.id.get(),
                kind: kind_of(s.tier),
                name: s.name.clone(),
                name_gloss: s.name_gloss.clone(),
                culture: s.culture.clone(),
                at_m: [s.x_m, s.y_m],
                rank,
                min_zoom,
            }
        })
        .collect();
    out.sort_by_key(|p| p.id);
    out
}

/// Which entries to keep.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    /// Keep only these kinds.
    pub kinds: Option<Vec<PlaceKind>>,
    /// Keep entries whose extent meets this box (`x0, y0, x1, y1`, metres).
    pub bbox: Option<[i64; 4]>,
    /// Keep entries of this rank or more important (a smaller number).
    pub max_rank: Option<u8>,
}

impl Filter {
    fn keeps(&self, item: &GazetteerItem) -> bool {
        if self.max_rank.is_some_and(|m| item.rank() > m) {
            return false;
        }
        if self
            .kinds
            .as_ref()
            .is_some_and(|k| !k.contains(&item.kind()))
        {
            return false;
        }
        self.bbox.is_none_or(|b| {
            let e = item.extent_m();
            e[0] <= b[2] && e[2] >= b[0] && e[1] <= b[3] && e[3] >= b[1]
        })
    }
}

/// The list for a gazetteer file and the settlements, filtered.
#[must_use]
pub fn list(file: &GazetteerFile, settlements: &[Settlement], filter: &Filter) -> GazetteerList {
    let features = file
        .features
        .iter()
        .cloned()
        .map(GazetteerItem::Feature)
        .chain(points(settlements).into_iter().map(GazetteerItem::Point))
        .filter(|i| filter.keeps(i))
        .collect();
    GazetteerList {
        format_version: super::FORMAT_VERSION,
        features,
    }
}
