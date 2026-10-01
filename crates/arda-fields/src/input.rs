//! Inputs: the land-use raster, the terrain callback, culture, wealth,
//! settlements and roads. All positions are world metres, y south-positive.

use serde::{Deserialize, Serialize};

/// Land use of one 100 m cell (goal 39; `04-settlements-roads.md` step 4),
/// exactly the canonical classes of `vocabulary.md`. Cells without land use
/// (wild land, settlement cores) are `None` in a [`LandUseMap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LandUse {
    /// Ploughland: open-field strips near villages, enclosed fields elsewhere.
    Arable,
    /// Grazing.
    Pasture,
    /// Fruit trees.
    Orchard,
    /// Retained woodland, usually on slopes.
    Woodland,
    /// Hay meadow.
    Meadow,
    /// Resting ploughland.
    Fallow,
    /// A watermill site on a river.
    Mill,
    /// A resource site: an adit on steep ground, an open quarry otherwise.
    MineQuarry,
    /// A farmstead: farmhouse, barn and yard.
    Farmstead,
}

/// A land-use raster lookup by 100 m cell coordinates.
pub trait LandUseMap {
    /// The class of cell `(cx, cy)`, which covers world
    /// `[100·cx, 100·cx + 100)` m; `None` for land without a use.
    fn class_at(&self, cx: i64, cy: i64) -> Option<LandUse>;
}

/// A dense land-use raster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandUseGrid {
    /// Cell coordinates of the raster's north-west cell.
    pub origin: (i64, i64),
    /// Width in cells.
    pub width: u32,
    /// Height in cells.
    pub height: u32,
    /// Row-major classes.
    pub cells: Vec<Option<LandUse>>,
    /// Class reported outside the raster.
    pub outside: Option<LandUse>,
}

/// Most cells a [`LandUseGrid`] may hold: 2^28, over five times a full
/// 500 × 1000 km world at 100 m.
pub const MAX_LANDUSE_CELLS: u64 = 1 << 28;

impl LandUseGrid {
    /// A raster of one class.
    ///
    /// # Errors
    /// [`crate::FieldsError::LandUseSize`] above [`MAX_LANDUSE_CELLS`] or
    /// when the allocation is refused (review round 2 #38: the size was
    /// allocated unchecked).
    pub fn filled(
        origin: (i64, i64),
        width: u32,
        height: u32,
        class: Option<LandUse>,
    ) -> Result<Self, crate::FieldsError> {
        let n = u64::from(width) * u64::from(height);
        let refuse = || crate::FieldsError::LandUseSize { width, height };
        if n > MAX_LANDUSE_CELLS {
            return Err(refuse());
        }
        let n = usize::try_from(n).map_err(|_| refuse())?;
        let mut cells = Vec::new();
        cells.try_reserve_exact(n).map_err(|_| refuse())?;
        cells.resize(n, class);
        Ok(Self {
            origin,
            width,
            height,
            cells,
            outside: None,
        })
    }

    /// Sets one cell; cells off the raster are ignored.
    pub fn set(&mut self, cx: i64, cy: i64, class: Option<LandUse>) {
        if let Some(i) = self.index(cx, cy) {
            if let Some(slot) = self.cells.get_mut(i) {
                *slot = class;
            }
        }
    }

    fn index(&self, cx: i64, cy: i64) -> Option<usize> {
        let x = usize::try_from(cx - self.origin.0).ok()?;
        let y = usize::try_from(cy - self.origin.1).ok()?;
        (x < self.width as usize && y < self.height as usize).then(|| y * self.width as usize + x)
    }
}

impl LandUseMap for LandUseGrid {
    fn class_at(&self, cx: i64, cy: i64) -> Option<LandUse> {
        self.index(cx, cy)
            .and_then(|i| self.cells.get(i).copied())
            .unwrap_or(self.outside)
    }
}

/// What the terrain callback reports at a point.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TerrainSample {
    /// Ground (or water-surface) height in metres.
    pub height_m: f64,
    /// Water depth in metres; zero is dry land.
    pub water_depth_m: f64,
}

/// The terrain at any world point, e.g. sampled from `arda` area cells.
pub trait Terrain: Sync {
    /// Terrain at world `(x_m, y_m)`.
    fn sample(&self, x_m: f64, y_m: f64) -> TerrainSample;
}

impl<F: Fn(f64, f64) -> TerrainSample + Sync> Terrain for F {
    fn sample(&self, x_m: f64, y_m: f64) -> TerrainSample {
        self(x_m, y_m)
    }
}

/// The landscape region, which picks the default field boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Region {
    /// Clay vales: hedgerows.
    Lowland,
    /// Stony hills: drystone walls.
    Upland,
    /// Both: walls on steep or stony ground, hedges elsewhere.
    Mixed,
}

/// Settlement tiers, as in `04-settlements-roads.md` step 3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// A few households; enclosed fields only.
    Hamlet,
    /// Open-field village.
    Village,
    /// Market town.
    Town,
    /// City.
    City,
}

/// A neighbouring settlement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settlement {
    /// Centre, world metres east.
    pub x_m: f64,
    /// Centre, world metres south.
    pub y_m: f64,
    /// Size tier.
    pub tier: Tier,
    /// Inhabitants.
    pub population: u32,
}

/// Road classes with the world's stored codes (`vocabulary.md`, I3):
/// `none = 0, track = 1, road = 2, highway = 3, footpath = 4`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum RoadClass {
    /// No road.
    None = 0,
    /// Track to a hamlet.
    Track = 1,
    /// Road to a village.
    Road = 2,
    /// Trunk road between towns.
    Highway = 3,
    /// Footpath.
    Footpath = 4,
}

impl RoadClass {
    /// The stored code.
    #[must_use]
    pub fn code(self) -> u8 {
        self as u8
    }

    /// The class of a stored code; unknown codes are `None`.
    #[must_use]
    pub fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Track,
            2 => Self::Road,
            3 => Self::Highway,
            4 => Self::Footpath,
            _ => Self::None,
        }
    }

    /// Carriageway width in squares (I4): highway 5, road 4, track 2,
    /// footpath 1.
    #[must_use]
    pub fn width_sq(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Track => 2,
            Self::Road => 4,
            Self::Highway => 5,
            Self::Footpath => 1,
        }
    }

    /// Half the carriageway width, in squares.
    #[must_use]
    pub fn half_width_sq(self) -> f64 {
        f64::from(self.width_sq()) / 2.0
    }

    /// The ground key of its surface.
    #[must_use]
    pub fn ground(self) -> &'static str {
        match self {
            Self::Highway => "cobbles",
            Self::Road => "gravel",
            Self::Track | Self::Footpath | Self::None => "dirt",
        }
    }
}

/// A road polyline in world metres.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Road {
    /// Class.
    pub class: RoadClass,
    /// Vertices in world metres.
    pub points: Vec<[f64; 2]>,
}

/// Everything [`crate::fields_window`] reads.
pub struct FieldInputs<'a> {
    /// Land use per 100 m cell.
    pub landuse: &'a dyn LandUseMap,
    /// Terrain callback.
    pub terrain: &'a dyn Terrain,
    /// Culture name, e.g. `human`; see [`crate::boundary::kit_for`].
    pub culture: &'a str,
    /// Landscape region.
    pub region: Region,
    /// Wealth, 0–255 as in the settlement records.
    pub wealth: u8,
    /// Settlements near the window (within a couple of kilometres).
    pub settlements: &'a [Settlement],
    /// Roads near the window.
    pub roads: &'a [Road],
    /// The squares the settlements' own plans occupy, by global square.
    /// When set, fields run right up to these footprints and the
    /// density-based core discs are not reserved.
    pub cores: Option<&'a dyn Fn(i64, i64) -> bool>,
    /// Which points the plan treats as water that fields stop at, by world
    /// metres. When unset, every point the terrain reports wet. A caller
    /// whose terrain knows rivers only roughly can limit this to open
    /// water, so fields meet the river's real banks, which it keeps.
    pub barrier_water: Option<&'a (dyn Fn(f64, f64) -> bool + Sync)>,
}
