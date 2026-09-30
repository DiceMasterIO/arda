//! `<world>/society/` files: versioned JSON documents and zstd rasters.
//!
//! See the crate README for the full format description.

use crate::crossings::{Crossing, Pass};
use crate::error::SettleError;
use crate::model::Settlement;
use crate::realms::Realm;
use crate::roads::Road;
use crate::stats::Stats;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Version of every file this crate writes.
pub const FORMAT_VERSION: u32 = 1;
/// Land-use raster magic.
pub const LANDUSE_MAGIC: &[u8; 8] = b"ARDALND\0";
/// Realm raster magic.
pub const REALMS_MAGIC: &[u8; 8] = b"ARDARLM\0";
/// Fixed zstd level so the bytes are reproducible.
const ZSTD_LEVEL: i32 = 9;

/// `settlements.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementsFile {
    /// Format version.
    pub format_version: u32,
    /// World seed.
    #[serde(with = "crate::ids::string")]
    pub seed: u64,
    /// Grid cells across (100 m each).
    pub width_cells: u32,
    /// Grid cells down.
    pub height_cells: u32,
    /// Settlements by id.
    pub settlements: Vec<Settlement>,
}

/// `roads.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoadsFile {
    /// Format version.
    pub format_version: u32,
    /// Road objects.
    pub roads: Vec<Road>,
    /// Crossing objects.
    pub crossings: Vec<Crossing>,
    /// Pass objects.
    pub passes: Vec<Pass>,
}

/// `realms.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealmsFile {
    /// Format version.
    pub format_version: u32,
    /// Realms by id.
    pub realms: Vec<Realm>,
}

/// A named river.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedRiver {
    /// 1-based id.
    #[serde(with = "crate::ids::string")]
    pub id: u64,
    /// Native name.
    pub name: String,
    /// English gloss of the name.
    pub name_gloss: String,
    /// Mouth, metres.
    pub mouth_m: [i64; 2],
    /// Traced main-stem length, metres.
    pub length_m: u64,
    /// Highest Strahler order.
    pub order: u8,
    /// Main-stem course from the mouth upstream, every 400 m, metres.
    pub course_m: Vec<[i64; 2]>,
}

/// A named mountain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedPeak {
    /// Native name.
    pub name: String,
    /// English gloss of the name.
    pub name_gloss: String,
    /// Summit, metres.
    pub at_m: [i64; 2],
    /// Summit height, metres.
    pub height_m: i32,
}

/// A named culture region.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedRegion {
    /// 1-based id.
    #[serde(with = "crate::ids::string")]
    pub id: u64,
    /// Native name.
    pub name: String,
    /// English gloss of the name.
    pub name_gloss: String,
    /// Culture key.
    pub culture: String,
    /// Approximate land area, hectares.
    pub land_ha: u64,
}

/// `names.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamesFile {
    /// Format version.
    pub format_version: u32,
    /// Rivers.
    pub rivers: Vec<NamedRiver>,
    /// Mountains.
    pub mountains: Vec<NamedPeak>,
    /// Regions.
    pub regions: Vec<NamedRegion>,
}

/// Writes pretty JSON.
///
/// # Errors
/// JSON and I/O errors.
pub fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), SettleError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|source| SettleError::Json {
        path: path.display().to_string(),
        source,
    })?;
    bytes.push(b'\n');
    std::fs::write(path, bytes).map_err(|e| SettleError::io(path, e))
}

/// Reads JSON.
///
/// # Errors
/// JSON and I/O errors.
pub fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, SettleError> {
    let bytes = std::fs::read(path).map_err(|e| SettleError::io(path, e))?;
    serde_json::from_slice(&bytes).map_err(|source| SettleError::Json {
        path: path.display().to_string(),
        source,
    })
}

/// Header plus a zstd payload.
fn write_raster(
    path: &Path,
    magic: &[u8; 8],
    width: usize,
    height: usize,
    payload: &[u8],
) -> Result<(), SettleError> {
    let mut out = Vec::with_capacity(payload.len() / 8 + 32);
    out.extend_from_slice(magic);
    out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&crate::num::u32_of(width).to_le_bytes());
    out.extend_from_slice(&crate::num::u32_of(height).to_le_bytes());
    let packed = zstd::bulk::compress(payload, ZSTD_LEVEL).map_err(|e| SettleError::io(path, e))?;
    out.extend_from_slice(&packed);
    std::fs::write(path, out).map_err(|e| SettleError::io(path, e))
}

/// Largest raster a reader accepts, in cells: 2^28, over five times the
/// 50 M cells of a 500 × 1000 km world at 100 m.
pub const MAX_RASTER_CELLS: usize = 1 << 28;

/// Reads a raster: `(width, height, payload)`.
fn read_raster(
    path: &Path,
    magic: &[u8; 8],
    bytes_per_cell: usize,
) -> Result<(usize, usize, Vec<u8>), SettleError> {
    let bad = |reason| SettleError::Format {
        path: path.display().to_string(),
        reason,
    };
    let bytes = std::fs::read(path).map_err(|e| SettleError::io(path, e))?;
    if bytes.len() < 20 || &bytes[..8] != magic {
        return Err(bad("bad magic"));
    }
    let word = |k: usize| u32::from_le_bytes([bytes[k], bytes[k + 1], bytes[k + 2], bytes[k + 3]]);
    if word(8) != FORMAT_VERSION {
        return Err(bad("unsupported format version"));
    }
    let w = usize::try_from(word(12)).map_err(|_| bad("width"))?;
    let h = usize::try_from(word(16)).map_err(|_| bad("height"))?;
    // The header is untrusted: bound the claim before any allocation, then
    // stream-decode so memory follows the bytes actually present, never the
    // claimed size (a 20-byte file must not reserve gigabytes).
    let cells = w.checked_mul(h).ok_or(bad("size"))?;
    if cells == 0 || cells > MAX_RASTER_CELLS {
        return Err(bad("size"));
    }
    let n = cells.checked_mul(bytes_per_cell).ok_or(bad("size"))?;
    let mut payload = Vec::new();
    let dec =
        zstd::stream::read::Decoder::new(&bytes[20..]).map_err(|e| SettleError::io(path, e))?;
    std::io::Read::read_to_end(&mut std::io::Read::take(dec, n as u64 + 1), &mut payload)
        .map_err(|e| SettleError::io(path, e))?;
    if payload.len() != n {
        return Err(bad("payload length"));
    }
    Ok((w, h, payload))
}

/// Writes `landuse.bin`: codes (u8 per cell) then owners (u32 LE per cell).
///
/// # Errors
/// I/O errors.
pub fn write_landuse(
    path: &Path,
    width: usize,
    height: usize,
    codes: &[u8],
    owner: &[u32],
) -> Result<(), SettleError> {
    let mut payload = Vec::with_capacity(codes.len() * 5);
    payload.extend_from_slice(codes);
    for o in owner {
        payload.extend_from_slice(&o.to_le_bytes());
    }
    write_raster(path, LANDUSE_MAGIC, width, height, &payload)
}

/// Reads `landuse.bin`: `(width, height, codes, owners)`.
///
/// # Errors
/// I/O and format errors.
pub fn read_landuse(path: &Path) -> Result<(usize, usize, Vec<u8>, Vec<u32>), SettleError> {
    let (w, h, p) = read_raster(path, LANDUSE_MAGIC, 5)?;
    let n = w * h;
    let codes = p[..n].to_vec();
    let owner = p[n..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| u32::from_le_bytes(*c))
        .collect();
    Ok((w, h, codes, owner))
}

/// Writes `realms.bin`: realm id (u16 LE) per cell, 0 on water.
///
/// # Errors
/// I/O errors.
pub fn write_realm_map(
    path: &Path,
    width: usize,
    height: usize,
    map: &[u16],
) -> Result<(), SettleError> {
    let payload: Vec<u8> = map.iter().flat_map(|v| v.to_le_bytes()).collect();
    write_raster(path, REALMS_MAGIC, width, height, &payload)
}

/// Reads `realms.bin`.
///
/// # Errors
/// I/O and format errors.
pub fn read_realm_map(path: &Path) -> Result<(usize, usize, Vec<u16>), SettleError> {
    let (w, h, p) = read_raster(path, REALMS_MAGIC, 2)?;
    Ok((
        w,
        h,
        p.as_chunks::<2>()
            .0
            .iter()
            .map(|c| u16::from_le_bytes(*c))
            .collect(),
    ))
}

/// Writes `stats.json`.
///
/// # Errors
/// JSON and I/O errors.
pub fn write_stats(path: &Path, stats: &Stats) -> Result<(), SettleError> {
    write_json(path, stats)
}
