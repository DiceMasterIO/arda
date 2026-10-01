//! Planner inputs: the settlement record and the local terrain.
//!
//! [`TownSite`] mirrors one record of `<world>/society/settlements.json`
//! written by `arda-settle` (goal 04, step 8): the same field names and enum
//! spellings, so a record deserialises directly. Unknown fields are ignored,
//! because the record carries more than the planner needs.

use crate::geom::{v2, Vec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// Settlement id, as in `settlements.json` (1-based). Serialised as a JSON
/// string; numbers are accepted on input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct SettlementId(
    #[serde(with = "crate::ids")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub u64,
);

/// Settlement size class (`arda-settle` `Tier`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// A handful of farms, 12–80 people.
    Hamlet,
    /// A village of a few hundred.
    Village,
    /// A town, 1,000–8,000 people.
    Town,
    /// A city, 8,000 people or more.
    City,
}

impl Tier {
    /// Towns and cities are walled (goal 44, "walls and gates").
    #[must_use]
    pub const fn is_walled(self) -> bool {
        matches!(self, Self::Town | Self::City)
    }
}

/// What a settlement lives on (`arda-settle` `Function`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettlementFunction {
    /// Arable fields.
    Farming,
    /// Herds and flocks.
    Pastoral,
    /// Inshore or river fishing.
    Fishing,
    /// A harbour with sea or river trade.
    Port,
    /// A regular market.
    Market,
    /// Mines or quarries.
    Mining,
    /// Forestry and charcoal.
    Logging,
    /// Workshops and guilds.
    Crafting,
    /// A garrison or castle.
    Fortress,
    /// A religious house.
    Abbey,
    /// A ford, bridge or ferry.
    Crossing,
    /// The seat of a realm.
    Capital,
}

/// One settlement record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TownSite {
    /// Stable identifier.
    pub id: SettlementId,
    /// Display name.
    pub name: String,
    /// Size class.
    pub tier: Tier,
    /// Inhabitants.
    pub population: u32,
    /// Economic functions.
    pub functions: Vec<SettlementFunction>,
    /// Overall wealth, 0–255.
    pub wealth: u8,
    /// Culture key (`heartland`, `highland`, `sylvan`, `coastal`,
    /// `southern`, `borderland`; others fall back by hash).
    pub culture: String,
    /// Owning realm; a JSON string as `arda-settle` writes it (I5), numbers
    /// are also read.
    #[serde(default, with = "crate::ids")]
    pub realm_id: u64,
    /// Biome name.
    #[serde(default)]
    pub biome: String,
    /// On a sea coast.
    #[serde(default)]
    pub coastal: bool,
    /// On a river.
    #[serde(default)]
    pub riverine: bool,
    /// Centre, metres east of the world's west edge.
    pub x_m: i64,
    /// Centre, metres south of the world's north edge.
    pub y_m: i64,
    /// Descriptive site tags (`ford`, `bridge_site`, `harbour`,
    /// `defensible`, `hill`, …).
    #[serde(default)]
    pub site_tags: Vec<String>,
    /// Estimated building mix: building-function key → count.
    #[serde(default)]
    pub buildings: BTreeMap<String, u32>,
}

impl TownSite {
    /// Whether the record lists `f`.
    #[must_use]
    pub fn has(&self, f: SettlementFunction) -> bool {
        self.functions.contains(&f)
    }

    /// Whether the record carries a site tag.
    #[must_use]
    pub fn tagged(&self, tag: &str) -> bool {
        self.site_tags.iter().any(|t| t == tag)
    }

    /// The centre in world metres.
    #[must_use]
    pub fn position(&self) -> Vec2 {
        // Settlement coordinates are whole metres well inside f64's exact range.
        #[allow(clippy::cast_precision_loss)]
        v2(self.x_m as f64, self.y_m as f64)
    }

    /// Parses one record from JSON.
    ///
    /// # Errors
    /// Malformed JSON or missing required fields.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Road class with the world's stored codes (vocabulary.md I3):
/// `none = 0, track = 1, road = 2, highway = 3, footpath = 4`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoadClass {
    /// No road.
    None,
    /// Track to a hamlet.
    Track,
    /// Road to a village.
    Road,
    /// Trunk road between towns.
    Highway,
    /// Footpath.
    Footpath,
}

impl RoadClass {
    /// Stored raster code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Track => 1,
            Self::Road => 2,
            Self::Highway => 3,
            Self::Footpath => 4,
        }
    }

    /// Carriageway width in squares (vocabulary.md I4).
    #[must_use]
    pub const fn width_squares(self) -> u32 {
        match self {
            Self::None => 0,
            Self::Track => 2,
            Self::Road => 4,
            Self::Highway => 5,
            Self::Footpath => 1,
        }
    }

    /// Importance, highest for highways.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Footpath => 1,
            Self::Track => 2,
            Self::Road => 3,
            Self::Highway => 4,
        }
    }
}

/// A road polyline passing through or ending at the settlement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnteringRoad {
    /// Road class.
    pub class: RoadClass,
    /// Polyline in world metres; it may be coarse (100 m cell centres).
    pub points: Vec<Vec2>,
}

/// A river centreline in world metres.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiverLine {
    /// Centreline, upstream first.
    pub points: Vec<Vec2>,
    /// Channel width in metres.
    pub width_m: f64,
}

/// A scalar field over world metres.
pub type Field = Box<dyn Fn(Vec2) -> f64 + Send + Sync>;
/// A boolean mask over world metres.
pub type Mask = Box<dyn Fn(Vec2) -> bool + Send + Sync>;

/// The terrain around a site, as callbacks over world metres.
pub struct TerrainInput {
    /// Ground or water-surface height, metres.
    pub height: Field,
    /// Ground slope, degrees.
    pub slope: Field,
    /// Open water (river, lake or sea).
    pub water: Mask,
    /// River centrelines crossing the site.
    pub rivers: Vec<RiverLine>,
    /// Roads entering the site.
    pub roads: Vec<EnteringRoad>,
}

impl fmt::Debug for TerrainInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerrainInput")
            .field("rivers", &self.rivers)
            .field("roads", &self.roads)
            .finish_non_exhaustive()
    }
}

impl TerrainInput {
    /// Flat dry land with no rivers or roads.
    #[must_use]
    pub fn flat() -> Self {
        Self {
            height: Box::new(|_| 0.0),
            slope: Box::new(|_| 0.0),
            water: Box::new(|_| false),
            rivers: Vec::new(),
            roads: Vec::new(),
        }
    }

    /// A terrain whose slope is derived from the height field by central
    /// differences over `step` metres.
    #[must_use]
    pub fn from_height(height: Field, water: Mask, step: f64) -> Self {
        let h = std::sync::Arc::new(height);
        let hs = std::sync::Arc::clone(&h);
        Self {
            height: Box::new(move |p| h(p)),
            slope: Box::new(move |p| slope_deg(&*hs, p, step)),
            water,
            rivers: Vec::new(),
            roads: Vec::new(),
        }
    }
}

/// Slope in degrees by central differences (a deterministic arctangent).
#[must_use]
pub fn slope_deg(h: &dyn Fn(Vec2) -> f64, p: Vec2, step: f64) -> f64 {
    let dx = (h(p + v2(step, 0.0)) - h(p - v2(step, 0.0))) / (2.0 * step);
    let dy = (h(p + v2(0.0, step)) - h(p - v2(0.0, step))) / (2.0 * step);
    let g = (dx * dx + dy * dy).sqrt();
    atan_deg(g)
}

/// Arctangent in degrees for `g ≥ 0`, by series with argument reduction.
#[must_use]
pub fn atan_deg(g: f64) -> f64 {
    let (x, flip) = if g > 1.0 { (1.0 / g, true) } else { (g, false) };
    // atan(x) = 2·atan(x / (1 + sqrt(1 + x²))) twice brings x under 0.2.
    let mut y = x;
    for _ in 0..2 {
        y /= 1.0 + (1.0 + y * y).sqrt();
    }
    let y2 = y * y;
    let mut term = y;
    let mut sum = y;
    for k in 1..12 {
        term *= -y2;
        sum += term / f64::from(2 * k + 1);
    }
    let a = sum * 4.0;
    let r = if flip {
        std::f64::consts::FRAC_PI_2 - a
    } else {
        a
    };
    r.to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_parses_with_extra_fields() {
        let json = r#"{"id":7,"name":"Ashford","tier":"village","population":320,
            "functions":["farming","market"],"wealth":120,"culture":"heartland",
            "realm_id":"1","biome":"temperate","coastal":false,"riverine":true,
            "x_m":1250,"y_m":900,"cell_x":12,"cell_y":9,"height_m":40,"rank":0,
            "site_tags":["ford"],"history":"grew at the ford","buildings":{"cottage":40}}"#;
        let s = TownSite::from_json(json).unwrap();
        assert_eq!(s.id, SettlementId(7));
        assert_eq!(s.tier, Tier::Village);
        assert!(s.has(SettlementFunction::Market));
        assert_eq!(s.buildings.get("cottage"), Some(&40));
    }

    #[test]
    fn arctangent_is_accurate() {
        for i in 0..50 {
            let g = f64::from(i) * 0.21;
            assert!((atan_deg(g) - g.atan().to_degrees()).abs() < 1e-7, "{g}");
        }
    }
}
