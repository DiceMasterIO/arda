//! Canonically indexed private prepared tiles with bounded immutable caching.
#![deny(missing_docs)]

use super::prepared_codec::{
    decode_prepared, encode_prepared, encoded_len, PreparedCell, PreparedFormatError, PreparedTile,
};
use super::prepared_domain::PreparedDomain;
use super::types::PreparedTerrain;
use arda_core::{AreaCoord, CellCoord, GlobalCell, AREA_CELLS};
use std::{
    collections::{BTreeMap, VecDeque},
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const FULL_CELLS: u64 = 512 * 512;
const MAX_ENCODED: u64 = 32 + 10 * FULL_CELLS;

/// Explicit admission for these files and their own buffers/cache. Pure terrain
/// preparation, caller-held arrays and routing pages require separate reservations.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    /// Owned payload allowance, including conservative index/cache/path overhead.
    pub ram_bytes: u64,
    /// All prepared files plus the completed index, simultaneously retained.
    pub scratch_bytes: u64,
    /// Maximum immutable tiles retained; the default sequential scan uses columns().
    pub cache_tiles: u32,
    /// Attempted read/write payload bytes across this handle's lifetime.
    pub io_bytes: u128,
    /// Attempted opens, metadata reads and payload operations.
    pub io_operations: u64,
    /// Attempted tile queries, including cache hits and invalid requested areas.
    pub tile_queries: u64,
}
/// Successful admission counters; failing attempted filesystem operations stay charged.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Work {
    /// Requested file payload bytes.
    pub io_bytes: u128,
    /// Opens, metadata and payload operations attempted.
    pub io_operations: u64,
    /// Tile queries attempted.
    pub tile_queries: u64,
}
/// Malformed authority, incomplete preparation, resource exhaustion or actual I/O error.
#[derive(Debug, thiserror::Error)]
pub enum PreparedError {
    /// Private tile codec rejected an input or record.
    #[error(transparent)]
    Format(#[from] PreparedFormatError),
    /// An index, completion or caller-identity invariant failed.
    #[error("invalid prepared files: {0}")]
    Invalid(&'static str),
    /// The next resource was not admitted.
    #[error("prepared files exceeded {0}")]
    Limit(&'static str),
    /// Original filesystem failure attached to its actual private path.
    #[error("prepared I/O on {path}: {source}")]
    Io {
        /// Private file involved.
        path: PathBuf,
        /// Original operating-system error.
        #[source]
        source: std::io::Error,
    },
}

/// Full physical prepared/index length. The deterministic grid needs only one
/// 8-byte checksum per tile; identities/extents come from the checked domain.
#[must_use]
pub fn scratch_required(domain: PreparedDomain) -> u64 {
    32 + 40 * u64::from(domain.count())
        + 10 * u64::from(domain.width()) * u64::from(domain.height())
}
/// Conservative own payload reservation, checked before cloning directory paths.
/// Includes max wire buffer, cache structs/cells, index overlaps and transient paths.
/// This is a payload allowance; host allocator metadata/process RSS are measured separately.
pub fn ram_required(
    directory: &Path,
    domain: PreparedDomain,
    cache_tiles: u32,
) -> Result<u64, PreparedError> {
    if cache_tiles == 0 || cache_tiles > domain.count() {
        return Err(PreparedError::Invalid("cache count"));
    }
    let paths = u64::try_from(directory.as_os_str().as_encoded_bytes().len())
        .map_err(|_| PreparedError::Limit("path bytes"))?;
    let cells = u64::try_from(std::mem::size_of::<PreparedCell>())
        .map_err(|_| PreparedError::Limit("cell size"))?;
    let required = u128::from(MAX_ENCODED)
        + 32 * u128::from(domain.count())
        + 16384
        + 8 * u128::from(paths)
        + u128::from(cache_tiles) * (u128::from(FULL_CELLS) * u128::from(cells) + 4096);
    u64::try_from(required).map_err(|_| PreparedError::Limit("RAM bytes"))
}
fn admit(directory: &Path, domain: PreparedDomain, limits: Limits) -> Result<(), PreparedError> {
    if ram_required(directory, domain, limits.cache_tiles)? > limits.ram_bytes {
        return Err(PreparedError::Limit("RAM bytes"));
    }
    if scratch_required(domain) > limits.scratch_bytes {
        return Err(PreparedError::Limit("scratch bytes"));
    }
    Ok(())
}
struct Meter {
    limits: Limits,
    work: Work,
}
impl Meter {
    fn io(&mut self, bytes: u128) -> Result<(), PreparedError> {
        let b = self
            .work
            .io_bytes
            .checked_add(bytes)
            .ok_or(PreparedError::Limit("I/O bytes"))?;
        let n = self
            .work
            .io_operations
            .checked_add(1)
            .ok_or(PreparedError::Limit("I/O operations"))?;
        if b > self.limits.io_bytes || n > self.limits.io_operations {
            return Err(PreparedError::Limit("I/O"));
        }
        self.work.io_bytes = b;
        self.work.io_operations = n;
        Ok(())
    }
    fn query(&mut self) -> Result<(), PreparedError> {
        if self.work.tile_queries >= self.limits.tile_queries {
            return Err(PreparedError::Limit("tile queries"));
        }
        self.work.tile_queries += 1;
        Ok(())
    }
}
fn tile_path(directory: &Path, ordinal: u32) -> PathBuf {
    directory.join(format!("prepared-{ordinal:04}.bin"))
}
fn io_error(path: &Path, source: std::io::Error) -> PreparedError {
    PreparedError::Io {
        path: path.to_path_buf(),
        source,
    }
}
// Private accidental-corruption checksum, not an authentication or security primitive.
fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}
fn write_new(path: &Path, bytes: &[u8], meter: &mut Meter) -> Result<(), PreparedError> {
    meter.io(0)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| io_error(path, e))?;
    meter.io(bytes.len() as u128)?;
    file.write_all(bytes).map_err(|e| io_error(path, e))
}
fn read_exact(path: &Path, length: usize, meter: &mut Meter) -> Result<Vec<u8>, PreparedError> {
    meter.io(0)?;
    let mut file = File::open(path).map_err(|e| io_error(path, e))?;
    meter.io(0)?;
    if file.metadata().map_err(|e| io_error(path, e))?.len() != length as u64 {
        return Err(PreparedError::Invalid("file length"));
    }
    meter.io(length as u128)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|_| PreparedError::Limit("allocation"))?;
    bytes.resize(length, 0);
    file.read_exact(&mut bytes).map_err(|e| io_error(path, e))?;
    Ok(bytes)
}

/// Accepts each canonical tile once, in any preparation completion order. Only
/// finish writes the index, so a reader cannot mistake an incomplete set for complete.
/// The parent owns a fresh private directory and cleanup on success or failure.
pub struct PreparedWriter {
    directory: PathBuf,
    domain: PreparedDomain,
    checksums: Vec<Option<u64>>,
    meter: Meter,
}
impl PreparedWriter {
    /// Admits owned storage before allocation. Does not enumerate or alter files.
    pub fn new(
        directory: &Path,
        domain: PreparedDomain,
        limits: Limits,
    ) -> Result<Self, PreparedError> {
        admit(directory, domain, limits)?;
        let mut checksums = Vec::new();
        checksums
            .try_reserve_exact(domain.count() as usize)
            .map_err(|_| PreparedError::Limit("index allocation"))?;
        checksums.resize(domain.count() as usize, None);
        Ok(Self {
            directory: directory.to_path_buf(),
            domain,
            checksums,
            meter: Meter {
                limits,
                work: Work::default(),
            },
        })
    }
    /// Writes one full preparation's modeled crop. Rejects duplicate/wrong tiles
    /// before touching their files; any I/O failure aborts the private transaction.
    pub fn write(&mut self, prepared: &PreparedTerrain) -> Result<(), PreparedError> {
        let ordinal = self
            .domain
            .ordinal(prepared.area)
            .ok_or(PreparedError::Invalid("area"))?;
        let entry = self
            .domain
            .entry(ordinal)
            .ok_or(PreparedError::Invalid("index entry"))?;
        if entry.valid != prepared.valid || self.checksums[ordinal as usize].is_some() {
            return Err(PreparedError::Invalid("duplicate or wrong tile"));
        }
        let bytes = encode_prepared(prepared)?;
        write_new(
            &tile_path(&self.directory, ordinal),
            &bytes,
            &mut self.meter,
        )?;
        self.checksums[ordinal as usize] = Some(checksum(&bytes));
        Ok(())
    }
    /// Writes the complete canonical index last and preserves the same resource
    /// counters in the returned immutable reader. Never emits final area files.
    pub fn finish(mut self) -> Result<PreparedReader, PreparedError> {
        if self.checksums.iter().any(Option::is_none) {
            return Err(PreparedError::Invalid("incomplete index"));
        }
        let mut bytes = Vec::with_capacity(32 + 8 * self.checksums.len());
        bytes.extend_from_slice(&self.domain.encode());
        let mut checksums = Vec::with_capacity(self.checksums.len());
        for value in self.checksums {
            let value = value.ok_or(PreparedError::Invalid("missing checksum"))?;
            bytes.extend_from_slice(&value.to_le_bytes());
            checksums.push(value);
        }
        write_new(
            &self.directory.join("prepared-index.bin"),
            &bytes,
            &mut self.meter,
        )?;
        PreparedReader::from_parts(self.directory, self.domain, checksums, self.meter)
    }
    /// Attempted resources so far; the parent shared-stage ledger includes them.
    #[must_use]
    // Retained for storage validation and optional stage diagnostics.
    #[allow(dead_code)]
    pub fn work(&self) -> Work {
        self.meter.work
    }
}

/// Immutable indexed source with a bounded FIFO tile cache. Sequential global rows
/// need at most domain.columns() cached tiles, including a partial eastern fringe.
/// No source file is regenerated or found by directory enumeration.
pub struct PreparedReader {
    directory: PathBuf,
    domain: PreparedDomain,
    checksums: Vec<u64>,
    cache: BTreeMap<u32, PreparedTile>,
    fifo: VecDeque<u32>,
    meter: Meter,
}
impl PreparedReader {
    fn from_parts(
        directory: PathBuf,
        domain: PreparedDomain,
        checksums: Vec<u64>,
        meter: Meter,
    ) -> Result<Self, PreparedError> {
        let mut fifo = VecDeque::new();
        fifo.try_reserve_exact(meter.limits.cache_tiles as usize)
            .map_err(|_| PreparedError::Limit("cache index allocation"))?;
        Ok(Self {
            directory,
            domain,
            checksums,
            cache: BTreeMap::new(),
            fifo,
            meter,
        })
    }
    /// Reopens only a completed index for the exact request-derived domain.
    /// The expected domain bounds byte allocation before index bytes are read.
    // Retained for storage validation and optional stage diagnostics.
    #[allow(dead_code)]
    pub fn open(
        directory: &Path,
        domain: PreparedDomain,
        limits: Limits,
    ) -> Result<Self, PreparedError> {
        admit(directory, domain, limits)?;
        let mut meter = Meter {
            limits,
            work: Work::default(),
        };
        let bytes = read_exact(
            &directory.join("prepared-index.bin"),
            32 + 8 * domain.count() as usize,
            &mut meter,
        )?;
        if bytes[..32] != domain.encode() {
            return Err(PreparedError::Invalid("index geometry"));
        }
        let checksums = bytes[32..]
            .as_chunks::<8>()
            .0
            .iter()
            .map(|row| u64::from_le_bytes(*row))
            .collect();
        Self::from_parts(directory.to_path_buf(), domain, checksums, meter)
    }
    /// Reads a canonical tile, validating exact length, checksum and indexed crop
    /// before caching it. Evicted immutable data remains in its private source file.
    pub fn tile(&mut self, area: AreaCoord) -> Result<&PreparedTile, PreparedError> {
        self.meter.query()?;
        let ordinal = self
            .domain
            .ordinal(area)
            .ok_or(PreparedError::Invalid("area"))?;
        if !self.cache.contains_key(&ordinal) {
            if self.cache.len() >= self.meter.limits.cache_tiles as usize {
                let old = self
                    .fifo
                    .pop_front()
                    .ok_or(PreparedError::Invalid("cache FIFO"))?;
                self.cache.remove(&old);
            }
            let entry = self
                .domain
                .entry(ordinal)
                .ok_or(PreparedError::Invalid("index entry"))?;
            let bytes = read_exact(
                &tile_path(&self.directory, ordinal),
                encoded_len(entry.valid)?,
                &mut self.meter,
            )?;
            if checksum(&bytes) != self.checksums[ordinal as usize] {
                return Err(PreparedError::Invalid("tile checksum"));
            }
            let tile = decode_prepared(area, entry.valid, &bytes)?;
            self.cache.insert(ordinal, tile);
            self.fifo.push_back(ordinal);
        }
        self.cache
            .get(&ordinal)
            .ok_or(PreparedError::Invalid("cache entry"))
    }
    /// Reads an actual modeled fine cell, rejecting unmodeled partial-tile padding.
    pub fn cell(&mut self, at: GlobalCell) -> Result<PreparedCell, PreparedError> {
        if at.x >= self.domain.width() || at.y >= self.domain.height() {
            return Err(PreparedError::Invalid("cell"));
        }
        let n = u32::from(AREA_CELLS);
        let area = AreaCoord::new(
            i32::try_from(at.x / n).map_err(|_| PreparedError::Invalid("area x"))?,
            i32::try_from(at.y / n).map_err(|_| PreparedError::Invalid("area y"))?,
        );
        let local = CellCoord::new(
            u16::try_from(at.x % n).map_err(|_| PreparedError::Invalid("cell x"))?,
            u16::try_from(at.y % n).map_err(|_| PreparedError::Invalid("cell y"))?,
        )
        .ok_or(PreparedError::Invalid("local cell"))?;
        self.tile(area)?
            .get(local)
            .copied()
            .ok_or(PreparedError::Invalid("cropped cell"))
    }
    /// Streams fresh global row-major routing records from actual indexed files.
    /// I/O and checksum errors remain typed items, never shortened EOF or zeros.
    /// Ocean connectivity is solved only after this unclassified stream is stored.
    pub fn routing_records(
        &mut self,
    ) -> impl Iterator<Item = Result<super::routing::CellRecord, PreparedError>> + '_ {
        let domain = self.domain;
        (0..domain.width() * domain.height()).map(move |raw| {
            self.cell(GlobalCell {
                x: raw % domain.width(),
                y: raw / domain.width(),
            })
            .map(|cell| super::routing::CellRecord::prepared(cell.height.raw(), false))
        })
    }

    /// Attempted resources; cache policy affects costs but never values.
    #[must_use]
    // Retained for storage validation and optional stage diagnostics.
    #[allow(dead_code)]
    pub fn work(&self) -> Work {
        self.meter.work
    }
    /// Canonical request-derived domain.
    #[must_use]
    pub fn domain(&self) -> PreparedDomain {
        self.domain
    }
}

#[cfg(test)]
#[path = "prepared_files_tests.rs"]
mod tests;
