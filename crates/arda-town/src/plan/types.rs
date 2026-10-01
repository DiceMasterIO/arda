//! The town plan: streets, square, plots, districts, wall and buildings in
//! world metres, plus the square-resolution grid they were built on.

use super::grid::{PlanGrid, Side, SquareRect};
use crate::function::BuildingFunction;
use crate::geom::Vec2;
use crate::site::{SettlementId, Tier};
use serde::{Deserialize, Serialize};

/// Street index within a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct StreetId(pub u16);

/// Plot index within a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct PlotId(pub u32);

/// Building id, unique within the settlement and 1-based like `arda-npc`'s
/// `BuildingId`; stable for a given seed and site, and the key NPC homes and
/// workplaces refer to (goal 45). Serialised as a JSON string (I5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct BuildingId(
    #[serde(with = "crate::ids")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub u64,
);

impl BuildingId {
    /// World-unique key `s<settlement>-b<building>`.
    #[must_use]
    pub fn global_key(self, site: SettlementId) -> String {
        format!("s{}-b{}", site.0, self.0)
    }
}

/// Street hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum StreetClass {
    /// A main street following an entering road.
    Main,
    /// A back lane or branch lane.
    Lane,
    /// A short cross alley between a main street and a back lane.
    Alley,
    /// The lane running inside the town wall.
    Intramural,
}

/// A street centreline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Street {
    /// Index.
    pub id: StreetId,
    /// Class.
    pub class: StreetClass,
    /// Centreline in world metres, starting at its junction end.
    pub points: Vec<Vec2>,
    /// Carriageway width in metres.
    pub width_m: f64,
    /// Cobbled rather than dirt.
    pub paved: bool,
}

/// What the town grew around.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum FocalKind {
    /// A bridge or ford.
    Crossing,
    /// A road junction.
    Crossroads,
    /// A harbour or landing.
    Harbour,
    /// A castle.
    Castle,
    /// A single road.
    Roadside,
}

/// The focal point and where the market sits.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Focal {
    /// Kind.
    pub kind: FocalKind,
    /// The feature itself (bridge, junction, quay or keep).
    pub feature: Vec2,
    /// The market square centre.
    pub market: Vec2,
}

/// The market square or village green.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MarketSquare {
    /// Outline in world metres.
    pub polygon: Vec<Vec2>,
    /// A grass green rather than a paved square.
    pub green: bool,
}

/// Functional districts (goal 36: layout reflects function).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum DistrictKind {
    /// Around the market square.
    Market,
    /// Houses.
    Residential,
    /// Workshops, smiths and tanners.
    Craft,
    /// Along the river or shore.
    Waterfront,
    /// Temple and churchyard.
    Religious,
    /// The castle ward.
    Castle,
    /// Outside the walls.
    Suburb,
    /// Village and hamlet farmsteads.
    Farmstead,
}

/// A plot of land along a street frontage (burgage or toft).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Plot {
    /// Index.
    pub id: PlotId,
    /// The street it fronts; `None` for the market square.
    pub street: Option<StreetId>,
    /// Direction the frontage faces (towards the street).
    pub front: Side,
    /// Outline in world metres (rectilinear).
    pub polygon: Vec<Vec2>,
    /// Frontage width in metres.
    pub frontage_m: f64,
    /// Mean depth in metres.
    pub depth_m: f64,
    /// District.
    pub district: DistrictKind,
    /// Inside the town wall (always true for unwalled settlements).
    pub inside_wall: bool,
}

/// A gate in the town wall.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Gate {
    /// Centre in world metres.
    pub point: Vec2,
    /// The street it carries, or `None` for a water gate.
    pub street: Option<StreetId>,
}

/// A bridge deck where a street crosses a river: rows of deck squares,
/// each spanning the water from bank to bank.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Bridge {
    /// The street it carries.
    pub street: StreetId,
    /// Rows run west–east (`true`) or north–south.
    pub along_x: bool,
    /// Rows as `(across, from, to)` in global squares, `from..=to` along
    /// the row: a row is square row `across` when `along_x`, else column.
    pub rows: Vec<(i64, i64, i64)>,
}

/// The town wall.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TownWall {
    /// Closed centreline ring in world metres.
    pub ring: Vec<Vec2>,
    /// Gates, including water gates.
    pub gates: Vec<Gate>,
    /// Thickness in metres.
    pub thickness_m: f64,
}

/// A castle ward with its own curtain wall.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CastleWard {
    /// Inner bailey, global squares.
    pub bailey: SquareRect,
    /// Side of the ward gate.
    pub gate: Side,
}

/// Coarse wealth band for kits and dressing (vocabulary `wealth:*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum WealthLevel {
    /// Wattle, dirt floors.
    Poor,
    /// Timber, planks.
    Modest,
    /// Stone, rugs.
    Wealthy,
}

impl WealthLevel {
    /// Band of a 0–255 wealth value.
    #[must_use]
    pub const fn of(w: u8) -> Self {
        match w {
            0..=84 => Self::Poor,
            85..=169 => Self::Modest,
            _ => Self::Wealthy,
        }
    }

    /// Vocabulary tag value.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Poor => "poor",
            Self::Modest => "modest",
            Self::Wealthy => "wealthy",
        }
    }
}

/// A doorway: on edge `side` of global square `(x, y)` inside the building.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Door {
    /// Square column inside the building.
    pub x: i64,
    /// Square row inside the building.
    pub y: i64,
    /// Which edge of that square.
    pub side: Side,
}

impl Door {
    /// The square just outside the door.
    #[must_use]
    pub const fn outside(&self) -> (i64, i64) {
        let (dx, dy) = self.side.step();
        (self.x + dx, self.y + dy)
    }
}

/// A building on the plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Building {
    /// Index.
    pub id: BuildingId,
    /// Function (vocabulary key).
    pub function: BuildingFunction,
    /// The plot it stands on, if any.
    pub plot: Option<PlotId>,
    /// Footprint polygon in world metres.
    pub footprint: Vec<Vec2>,
    /// Footprint in global squares (the same rectangle).
    pub rect: SquareRect,
    /// Storeys above ground.
    pub storeys: u8,
    /// Wealth 0–255.
    pub wealth: u8,
    /// Wealth band.
    pub wealth_level: WealthLevel,
    /// Side facing the street.
    pub front: Side,
    /// Doors; the first faces the street.
    pub doors: Vec<Door>,
    /// Not in the building mix (barns behind farmhouses, jetties).
    pub ancillary: bool,
    /// Free `key:value` tags, e.g. `craft:weaving` on workshops (I6).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// A complete town plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct TownPlan {
    /// Settlement id.
    pub site: SettlementId,
    /// Settlement name.
    pub name: String,
    /// Tier.
    pub tier: Tier,
    /// Culture key.
    pub culture: String,
    /// Plan seed (world seed mixed with the site id), as a JSON string.
    #[serde(with = "crate::ids")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub seed: u64,
    /// Ground height at the market, metres above sea level; grid heights
    /// are relative to it.
    pub base_height_m: f64,
    /// Water surface level within the plan, metres above sea level.
    pub water_level_m: f64,
    /// Focal point.
    pub focal: Focal,
    /// Streets.
    pub streets: Vec<Street>,
    /// Bridges carrying streets over rivers.
    #[serde(default)]
    pub bridges: Vec<Bridge>,
    /// Market square or green.
    pub square: Option<MarketSquare>,
    /// Plots.
    pub plots: Vec<Plot>,
    /// Districts and their plots.
    pub districts: Vec<(DistrictKind, Vec<PlotId>)>,
    /// Town wall, for walled tiers.
    pub wall: Option<TownWall>,
    /// Castle ward, for fortress sites.
    pub castle: Option<CastleWard>,
    /// Buildings.
    pub buildings: Vec<Building>,
    /// Planner notes: aliases used, off-plan and unplaced mix entries.
    pub notes: Vec<String>,
    /// The square-resolution index (not serialised).
    #[serde(skip)]
    pub grid: PlanGrid,
    /// Interior salts by building position, drawn on first use
    /// ([`crate::block::interior::variety::salts`]; not serialised).
    #[serde(skip)]
    pub interior_salts: std::sync::OnceLock<Vec<u8>>,
}

impl TownPlan {
    /// A building by id.
    #[must_use]
    pub fn building(&self, id: BuildingId) -> Option<&Building> {
        self.buildings
            .get(usize::try_from(id.0.checked_sub(1)?).ok()?)
    }

    /// Serialises the plan (without the grid) as pretty JSON.
    ///
    /// # Errors
    /// Serialisation failure.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// The interior salt of `b` (goal 64: adjacent buildings never share
    /// an interior), drawn for the whole plan on first use.
    #[must_use]
    pub fn interior_salt(&self, b: &Building) -> u8 {
        let salts = self
            .interior_salts
            .get_or_init(|| crate::block::interior::variety::salts(self));
        usize::try_from(b.id.0.saturating_sub(1))
            .ok()
            .and_then(|i| salts.get(i).copied())
            .unwrap_or(0)
    }

    /// Global square origin of the plan grid.
    #[must_use]
    pub const fn origin(&self) -> (i64, i64) {
        (self.grid.gx0, self.grid.gy0)
    }

    /// Grid size in squares.
    #[must_use]
    pub const fn size(&self) -> (i64, i64) {
        (self.grid.w, self.grid.h)
    }
}

#[cfg(all(test, feature = "schema"))]
mod schema_tests {
    use super::TownPlan;

    #[test]
    fn town_plan_schema_is_draft_2020_12_with_known_fields() {
        let schema = serde_json::to_value(schemars::schema_for!(TownPlan)).unwrap();
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        for key in ["site", "streets", "plots", "buildings"] {
            assert!(schema["properties"][key].is_object(), "{key}");
        }
        assert_eq!(schema["properties"]["seed"]["type"], "string");
        assert!(schema["properties"]["grid"].is_null());
    }
}
