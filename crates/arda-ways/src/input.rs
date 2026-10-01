//! Input records: roads and crossings as `arda-settle` writes them to
//! `society/roads.json`, river channels, and the terrain callback.
//!
//! The road and crossing types mirror `arda-settle`'s `Road`, `RoadClass`,
//! `Crossing` and `CrossingKind` field for field, so its JSON deserialises
//! straight into them (fields this crate does not use are ignored).

use serde::{Deserialize, Serialize};

/// Side of one 5-ft square in world metres: a 100 m cell is a 64 × 64
/// block (goal 42), so a square is exactly 1.5625 m. The value is a binary
/// fraction, so square-centre positions are exact in floating point and
/// two windows compute bit-identical positions for a shared square.
pub const SQUARE_M: f64 = 100.0 / 64.0;

/// Road class with the canonical stored codes (vocabulary I3): `none = 0,
/// track = 1, road = 2, highway = 3, footpath = 4`. Use
/// [`RoadClass::hierarchy`], not the code, to compare importance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoadClass {
    /// No road (raster filler; a road record of this class is ignored).
    None = 0,
    /// Track to a hamlet.
    Track = 1,
    /// Road to a village.
    Road = 2,
    /// Trunk road between towns.
    Highway = 3,
    /// Footpath between neighbours.
    Footpath = 4,
}

impl RoadClass {
    /// The canonical stored code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Importance, lowest first: none, footpath, track, road, highway.
    #[must_use]
    pub const fn hierarchy(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Footpath => 1,
            Self::Track => 2,
            Self::Road => 3,
            Self::Highway => 4,
        }
    }
}

/// Per-class 5-ft geometry, from the "Roads" and "Inside a cell" rules of
/// `docs/capstone/mockup-artifact.md` and goal 37.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClassSpec {
    /// Travelled-surface width in squares.
    pub width_sq: u32,
    /// Verge width each side, in squares.
    pub verge_sq: f64,
    /// Whether the class has side ditches beyond the verge.
    pub ditches: bool,
    /// Steepest sustained grade (rise over run) before switchbacks.
    pub max_grade: f64,
    /// Deck width of a bridge on this class, in squares.
    pub deck_sq: u32,
}

impl RoadClass {
    /// Width, verge, ditch, grade and deck rules for the class. Widths are
    /// the canonical ones (vocabulary I4): highway 5, road 4, track 2 and
    /// footpath 1 squares. `wealth` does not change geometry, only surface.
    #[must_use]
    pub fn spec(self, wealth: u8) -> ClassSpec {
        let _ = wealth;
        match self {
            Self::Highway => ClassSpec {
                width_sq: 5,
                verge_sq: 1.0,
                ditches: true,
                max_grade: 0.08,
                deck_sq: 4,
            },
            Self::Road => ClassSpec {
                width_sq: 4,
                verge_sq: 1.0,
                ditches: false,
                max_grade: 0.10,
                deck_sq: 3,
            },
            Self::Track => ClassSpec {
                width_sq: 2,
                verge_sq: 0.5,
                ditches: false,
                max_grade: 0.14,
                deck_sq: 2,
            },
            Self::Footpath | Self::None => ClassSpec {
                width_sq: 1,
                verge_sq: 0.0,
                ditches: false,
                max_grade: 0.25,
                deck_sq: 2,
            },
        }
    }
}

/// Ids are serialised as JSON strings (vocabulary I5) and read from either
/// a string or a number, so older `roads.json` files still load.
pub mod id_serde {
    use serde::{de, Deserialize, Deserializer, Serializer};

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Raw {
        Num(u64),
        Str(String),
    }

    /// Writes an id as a string.
    ///
    /// # Errors
    /// Serializer failure.
    // serde's serialize_with hands a reference.
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&v.to_string())
    }

    /// Reads an id from a string or a number.
    ///
    /// # Errors
    /// Neither form, or out of range.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        Ok(match Raw::deserialize(d)? {
            Raw::Num(n) => n,
            Raw::Str(s) => s.parse::<u64>().map_err(de::Error::custom)?,
        })
    }
}

fn default_wealth() -> u8 {
    128
}

/// One road object (`arda-settle` `Road`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Road {
    /// 1-based id in build order (u64, as `arda-settle` writes it).
    #[serde(with = "id_serde")]
    pub id: u64,
    /// Class.
    pub class: RoadClass,
    /// Stretches as polylines of `[x_m, y_m]` world metres (x east, y south).
    pub segments: Vec<Vec<[i64; 2]>>,
    /// Wealth of the land the road serves, 0–255 (not in `roads.json`; the
    /// caller fills it from the settlements, default 128). Sets the surface.
    #[serde(default = "default_wealth")]
    pub wealth: u8,
}

/// How a road gets over water (`arda-settle` `CrossingKind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossingKind {
    /// A bridge.
    Bridge,
    /// A ford.
    Ford,
    /// A ferry.
    Ferry,
}

/// One crossing object (`arda-settle` `Crossing`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crossing {
    /// 1-based id (u64, as `arda-settle` writes it).
    #[serde(with = "id_serde")]
    pub id: u64,
    /// Bridge, ford or ferry.
    pub kind: CrossingKind,
    /// What is crossed: `river`, `sea` or `lake`.
    #[serde(default)]
    pub water: String,
    /// Position, metres east.
    pub x_m: i64,
    /// Position, metres south.
    pub y_m: i64,
    /// Channel width in metres (0 for open water).
    pub width_m: u32,
    /// Strahler order (0 for open water).
    #[serde(default)]
    pub order: u8,
    /// Highest road class using the crossing.
    pub road_class: RoadClass,
    /// Name of the river.
    #[serde(default)]
    pub river: String,
}

/// A river channel at 5-ft resolution: centreline, bank-to-bank width and
/// thalweg depth. Squares whose centres lie within `width_m / 2` of the
/// (smoothed) centreline are water.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiverChannel {
    /// Stable id (for reports).
    pub id: u64,
    /// Centreline in world metres, upstream first.
    pub centreline: Vec<[f64; 2]>,
    /// Bank-to-bank width, metres.
    pub width_m: f64,
    /// Depth at the centreline, metres.
    pub depth_m: f64,
}

/// The terrain callback: ground height and slope at world positions, and
/// the river channels the window may meet. Implementations must be pure
/// functions of the position, so that neighbouring windows agree.
pub trait Terrain {
    /// Ground height in metres at `(x_m, y_m)`.
    fn height_m(&self, x_m: f64, y_m: f64) -> f64;

    /// Slope as rise over run; defaults to a central difference of
    /// [`Terrain::height_m`] over one square.
    fn slope(&self, x_m: f64, y_m: f64) -> f64 {
        let d = SQUARE_M;
        let gx = (self.height_m(x_m + d, y_m) - self.height_m(x_m - d, y_m)) / (2.0 * d);
        let gy = (self.height_m(x_m, y_m + d) - self.height_m(x_m, y_m - d)) / (2.0 * d);
        gx.hypot(gy)
    }

    /// River channels near the window; none by default.
    fn channels(&self) -> &[RiverChannel] {
        &[]
    }

    /// Whether the caller rasterises river water itself
    /// ([`Terrain::river_water`]). Then its channels only guide crossings
    /// (flow direction, width, depth) and paint no water, and every place a
    /// way runs over that water becomes a bridge or ford (`plan::wet`).
    fn rivers_rasterised(&self) -> bool {
        false
    }

    /// Whether global square `(gx, gy)` is river water in the caller's
    /// raster; only asked when [`Terrain::rivers_rasterised`].
    fn river_water(&self, gx: i64, gy: i64) -> bool {
        let _ = (gx, gy);
        false
    }

    /// Whether the ground at global square `(gx, gy)` is wet enough for
    /// water to stand in a ditch (a marsh or low floodplain cell). Dry by
    /// default: ditches then hold no pools.
    fn wet_ground(&self, gx: i64, gy: i64) -> bool {
        let _ = (gx, gy);
        false
    }
}

/// A [`Terrain`] from a height closure plus a channel list.
pub struct FnTerrain<F: Fn(f64, f64) -> f64> {
    /// Height in metres at a world position.
    pub height: F,
    /// River channels.
    pub channels: Vec<RiverChannel>,
}

impl<F: Fn(f64, f64) -> f64> Terrain for FnTerrain<F> {
    fn height_m(&self, x_m: f64, y_m: f64) -> f64 {
        (self.height)(x_m, y_m)
    }

    fn channels(&self) -> &[RiverChannel] {
        &self.channels
    }
}

/// Rounds feet to the canonical 5-ft elevation steps (vocabulary I20).
#[must_use]
pub fn step5(ft: i16) -> i16 {
    let r = (i32::from(ft) + 2).div_euclid(5) * 5;
    i16::try_from(r).unwrap_or(if ft < 0 { i16::MIN + 3 } else { i16::MAX - 2 })
}

/// Water this deep or deeper is swimming; shallower is wading and
/// difficult terrain (vocabulary I15).
pub const SWIM_FT: u8 = 5;

/// Metres to whole feet, rounded (internal resolution before [`step5`]).
#[must_use]
#[allow(clippy::cast_possible_truncation)] // clamped to the i16 range first
pub fn m_to_ft(m: f64) -> i16 {
    (m / 0.3048)
        .round()
        .clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16
}
