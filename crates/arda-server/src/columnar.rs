//! Columnar area samples: compact JSON and the `ARDACOLS` binary layout.
//!
//! Both forms carry the same named columns in the same order, one value per
//! cell, row-major (`index = cy * 512 + cx`). Floats are f32; enums are the
//! stored discriminants listed in [`Legend`]; ids use 0 for "none".

use crate::contract::{
    CellSample, CoverDto, LandUseDto, RoadDto, TerrainKindDto, CONTRACT_VERSION,
};
use crate::error::{ServerError, ServerResult};
use schemars::JsonSchema;
use serde::Serialize;
use ts_rs::TS;

/// Binary magic.
pub const MAGIC: &[u8; 8] = b"ARDACOLS";
/// Binary layout version: 2 adds the `land_use` and `realm_id` columns and
/// the `footpath` road code (logic/16 §api-cell-society).
pub const LAYOUT_VERSION: u32 = 2;
/// Fixed header bytes before the column table.
pub const HEADER_BYTES: usize = 48;

/// Enum codes: a column value `i` means the `i`-th entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, JsonSchema)]
pub struct Legend {
    /// Codes of the `terrain` column.
    pub terrain: Vec<TerrainKindDto>,
    /// Codes of the `cover` column.
    pub cover: Vec<CoverDto>,
    /// Codes of the `road` column.
    pub road: Vec<RoadDto>,
    /// Codes of the `land_use` column (0 also when the world has no `society/`).
    pub land_use: Vec<LandUseDto>,
}

impl Default for Legend {
    fn default() -> Self {
        use CoverDto as C;
        Self {
            terrain: vec![
                TerrainKindDto::Sea,
                TerrainKindDto::Land,
                TerrainKindDto::Lake,
            ],
            cover: vec![
                C::Bare,
                C::Grass,
                C::Scrub,
                C::Forest,
                C::Marsh,
                C::Rock,
                C::Ice,
            ],
            road: vec![
                RoadDto::None,
                RoadDto::Track,
                RoadDto::Road,
                RoadDto::Highway,
                RoadDto::Footpath,
            ],
            land_use: (0..=10).map(crate::contract::convert::land_use).collect(),
        }
    }
}

macro_rules! columns {
    ($( $name:ident : $ty:ty => $dtype:ident, |$s:ident| $get:expr, $doc:literal; )*) => {
        /// One array per [`CellSample`] field, each 512 × 512 long.
        #[derive(Debug, Clone, Default, PartialEq, Serialize, TS, JsonSchema)]
        pub struct AreaColumns {
            $( #[doc = $doc] pub $name: Vec<$ty>, )*
        }

        impl AreaColumns {
            fn push(&mut self, sample: &CellSample) {
                $( { let $s = sample; self.$name.push($get); } )*
            }
        }

        /// Column names and binary dtypes, in wire order.
        pub const COLUMNS: &[(&str, Dtype)] = &[ $( (stringify!($name), Dtype::$dtype), )* ];

        fn write_columns(c: &AreaColumns, out: &mut Vec<u8>) {
            $( Dtype::$dtype.write(&c.$name, out); )*
        }
    };
}

/// Binary element type of a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dtype {
    /// Unsigned byte (code 1).
    U8,
    /// Little-endian u32 (code 2).
    U32,
    /// Little-endian f32 (code 3).
    F32,
    /// Little-endian f32 with NaN for `null` (code 3 on the wire).
    OptF32,
}

impl Dtype {
    /// Wire code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::U8 => 1,
            Self::U32 => 2,
            Self::F32 | Self::OptF32 => 3,
        }
    }

    fn write<T: Element>(self, values: &[T], out: &mut Vec<u8>) {
        for v in values {
            v.write(out);
        }
    }
}

trait Element {
    fn write(&self, out: &mut Vec<u8>);
}
impl Element for u8 {
    fn write(&self, out: &mut Vec<u8>) {
        out.push(*self);
    }
}
impl Element for u32 {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
}
impl Element for f32 {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.to_le_bytes());
    }
}
impl Element for Option<f32> {
    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.unwrap_or(f32::NAN).to_le_bytes());
    }
}

// f64 → f32 is the documented column precision.
#[allow(clippy::cast_possible_truncation)]
const fn f(v: f64) -> f32 {
    v as f32
}

const fn terrain_code(t: TerrainKindDto) -> u8 {
    match t {
        TerrainKindDto::Sea => 0,
        TerrainKindDto::Land => 1,
        TerrainKindDto::Lake => 2,
    }
}

const fn cover_code(c: CoverDto) -> u8 {
    match c {
        CoverDto::Bare => 0,
        CoverDto::Grass => 1,
        CoverDto::Scrub => 2,
        CoverDto::Forest => 3,
        CoverDto::Marsh => 4,
        CoverDto::Rock => 5,
        CoverDto::Ice => 6,
    }
}

const fn road_code(r: RoadDto) -> u8 {
    match r {
        RoadDto::None => 0,
        RoadDto::Track => 1,
        RoadDto::Road => 2,
        RoadDto::Highway => 3,
        RoadDto::Footpath => 4,
    }
}

const fn land_use_code(l: Option<LandUseDto>) -> u8 {
    match l {
        None | Some(LandUseDto::None) => 0,
        Some(LandUseDto::Built) => 1,
        Some(LandUseDto::Field) => 2,
        Some(LandUseDto::Pasture) => 3,
        Some(LandUseDto::Orchard) => 4,
        Some(LandUseDto::Woodland) => 5,
        Some(LandUseDto::Mill) => 6,
        Some(LandUseDto::Mine) => 7,
        Some(LandUseDto::Meadow) => 8,
        Some(LandUseDto::Fallow) => 9,
        Some(LandUseDto::Farmstead) => 10,
    }
}

/// A decimal id string as a u32 column value, 0 for none.
fn id_code(id: Option<&str>) -> u32 {
    id.and_then(|v| v.parse().ok()).unwrap_or(0)
}

columns! {
    height_m: f32 => F32, |s| f(s.height_m), "Elevation, m.";
    terrain: u8 => U8, |s| terrain_code(s.terrain), "Terrain code ([`Legend::terrain`]).";
    cover: u8 => U8, |s| cover_code(s.cover), "Cover code ([`Legend::cover`]).";
    slope_deg: f32 => F32, |s| f(s.slope_deg), "Slope, degrees.";
    aspect_deg: f32 => F32, |s| f(s.aspect_deg), "Downslope bearing, degrees.";
    temperature_c: f32 => F32, |s| f(s.temperature_c), "Mean annual temperature, °C.";
    rainfall_mm: f32 => F32, |s| f(s.rainfall_mm), "Annual rainfall, mm.";
    moisture: f32 => F32, |s| f(s.moisture), "Soil moisture, 0–1.";
    wetness: f32 => F32, |s| f(s.wetness), "Standing-water tendency, 0–1.";
    forest_density: f32 => F32, |s| f(s.forest_density), "Canopy closure, 0–1.";
    drainage_area_km2: f32 => F32, |s| f(s.drainage_area_km2), "Drainage area, km².";
    discharge_m3s: f32 => F32, |s| f(s.discharge_m3s), "Mean discharge, m³/s.";
    watercourse_order: u8 => U8, |s| s.watercourse_order, "Strahler order, 0 = none.";
    watercourse_width_m: f32 => F32, |s| f(s.watercourse_width_m), "Channel width, m.";
    height_above_river_m: f32 => F32, |s| f(s.height_above_river_m), "Height above channel, m.";
    road: u8 => U8, |s| road_code(s.road), "Road code ([`Legend::road`]).";
    built_by: u32 => U32, |s| id_code(s.built_by.as_deref()), "Settlement id, 0 = none.";
    land_use: u8 => U8, |s| land_use_code(s.land_use), "Land-use code ([`Legend::land_use`]).";
    realm_id: u32 => U32, |s| id_code(s.realm_id.as_deref()), "Realm id, 0 = none.";
    is_coast: u8 => U8, |s| u8::from(s.coast.is_coast), "1 on coast cells.";
    coast_distance_m: Option<f32> => OptF32, |s| s.coast.distance_m.map(f), "Coast distance, m; null beyond 5 km.";
    snow_fraction: f32 => F32, |s| f(s.snow.fraction), "Snow proxy of the cell, 0–1.";
    snow_peak_fraction: Option<f32> => OptF32, |s| s.snow.peak_fraction.map(f), "Snow proxy at the footprint peak.";
    snow_perennial: u8 => U8, |s| u8::from(s.snow.perennial), "1 when perennially snowed.";
    snowline_m: f32 => F32, |s| f(s.snow.snowline_m), "Proxy snowline altitude, m.";
    river_segment: u32 => U32, |s| s.river.as_ref().map_or(0, |r| r.segment_id), "Owning river segment id, 0 = none.";
    lake_id: u32 => U32, |s| s.lake.as_ref().map_or(0, |l| l.lake_id), "Covering lake id, 0 = none.";
    lake_depth_m: Option<f32> => OptF32, |s| s.lake.as_ref().map(|l| f(l.depth_m)), "Lake depth of the cell, m.";
    fine_centre_m: Option<f32> => OptF32, |s| s.fine.map(|x| f(x.centre_m)), "Fine height at the cell centre, m.";
    fine_min_m: Option<f32> => OptF32, |s| s.fine.map(|x| f(x.min_m)), "Fine footprint minimum, m.";
    fine_max_m: Option<f32> => OptF32, |s| s.fine.map(|x| f(x.max_m)), "Fine footprint maximum, m.";
}

/// `/v1/area/{ax}/{ay}/cells` JSON body.
#[derive(Debug, Clone, PartialEq, Serialize, TS, JsonSchema)]
pub struct AreaCells {
    /// Cell contract version.
    pub contract_version: u32,
    /// Area column.
    pub ax: i32,
    /// Area row.
    pub ay: i32,
    /// Global column of local x = 0.
    pub gx0: u32,
    /// Global row of local y = 0.
    pub gy0: u32,
    /// Columns per row (512).
    pub width: u32,
    /// Rows (512).
    pub height: u32,
    /// Enum code tables.
    pub legend: Legend,
    /// Values.
    pub columns: AreaColumns,
}

/// Builds the columnar body from row-major samples of one area.
///
/// # Errors
/// [`ServerError::Internal`] when `samples` is empty.
pub fn build(samples: &[CellSample]) -> ServerResult<AreaCells> {
    let first = samples
        .first()
        .ok_or_else(|| ServerError::Internal("empty area".into()))?;
    let mut columns = AreaColumns::default();
    for s in samples {
        columns.push(s);
    }
    Ok(AreaCells {
        contract_version: CONTRACT_VERSION,
        ax: first.ax,
        ay: first.ay,
        gx0: first.gx,
        gy0: first.gy,
        width: u32::from(arda_core::AREA_CELLS),
        height: u32::from(arda_core::AREA_CELLS),
        legend: Legend::default(),
        columns,
    })
}

/// Encodes the documented `ARDACOLS` v2 binary layout (see `API.md`).
///
/// # Errors
/// [`ServerError::ResourceLimit`] when the buffer cannot be reserved.
pub fn encode(body: &AreaCells) -> ServerResult<Vec<u8>> {
    let n = (body.width * body.height) as usize;
    let data: usize = COLUMNS
        .iter()
        .map(|(_, d)| n * if *d == Dtype::U8 { 1 } else { 4 })
        .sum();
    let table: usize = COLUMNS.iter().map(|(name, _)| 2 + name.len()).sum();
    let data_offset = (HEADER_BYTES + table).div_ceil(4) * 4;
    let mut out = Vec::new();
    out.try_reserve_exact(data_offset + data)
        .map_err(|_| ServerError::ResourceLimit("binary area allocation".into()))?;
    let count =
        u32::try_from(COLUMNS.len()).map_err(|_| ServerError::Internal("columns".into()))?;
    let offset = u32::try_from(data_offset).map_err(|_| ServerError::Internal("offset".into()))?;
    out.extend_from_slice(MAGIC);
    for word in [LAYOUT_VERSION, body.contract_version] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend_from_slice(&body.ax.to_le_bytes());
    out.extend_from_slice(&body.ay.to_le_bytes());
    for word in [body.gx0, body.gy0, body.width, body.height, count, offset] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    for (name, dtype) in COLUMNS {
        out.push(u8::try_from(name.len()).map_err(|_| ServerError::Internal("name".into()))?);
        out.extend_from_slice(name.as_bytes());
        out.push(dtype.code());
    }
    out.resize(data_offset, 0);
    write_columns(&body.columns, &mut out);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_keys_match_the_binary_column_table_in_order() {
        let json = serde_json::to_value(AreaColumns::default()).unwrap();
        let keys: Vec<_> = json.as_object().unwrap().keys().cloned().collect();
        let mut names: Vec<_> = COLUMNS.iter().map(|(n, _)| (*n).to_owned()).collect();
        // serde_json maps are sorted; compare as sets, then check the table has no duplicates.
        names.sort();
        assert_eq!(keys, names);
        names.dedup();
        assert_eq!(names.len(), COLUMNS.len());
    }

    #[test]
    fn header_is_48_bytes_and_data_is_four_byte_aligned() {
        let body = AreaCells {
            contract_version: CONTRACT_VERSION,
            ax: 1,
            ay: 2,
            gx0: 512,
            gy0: 1024,
            width: 0,
            height: 0,
            legend: Legend::default(),
            columns: AreaColumns::default(),
        };
        let bytes = encode(&body).unwrap();
        assert_eq!(&bytes[..8], MAGIC);
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
        assert_eq!(word(8), LAYOUT_VERSION);
        assert_eq!(word(24), 512);
        assert_eq!(word(40) as usize, COLUMNS.len());
        let offset = word(44) as usize;
        assert_eq!(offset % 4, 0);
        assert_eq!(bytes.len(), offset, "no cells, no data");
        assert_eq!(bytes[HEADER_BYTES] as usize, "height_m".len());
    }

    #[test]
    fn legend_codes_match_the_column_codes() {
        let legend = Legend::default();
        for (i, r) in legend.road.iter().enumerate() {
            assert_eq!(usize::from(road_code(*r)), i);
        }
        for (i, l) in legend.land_use.iter().enumerate() {
            assert_eq!(usize::from(land_use_code(Some(*l))), i);
            // Legend code i is the stored landuse.bin code i.
            assert_eq!(usize::from(arda_ids::LandUse::ALL[i].code()), i);
        }
        assert_eq!(legend.land_use.len(), arda_ids::LandUse::ALL.len());
        assert_eq!(land_use_code(None), 0);
        assert_eq!(id_code(Some("4000000000")), 4_000_000_000);
        assert_eq!(id_code(None), 0);
    }
}
