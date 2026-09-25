//! Versioned, streaming storage for one canonical fine-height lattice.
//!
//! The 88-byte header is explicit little-endian: magic[8], major/minor u16,
//! finalized u32, origin x/y i64, spacing/width/height/reserved u32,
//! payload bytes u64, BLAKE3[32]. The digest covers the first 56 finalized
//! header bytes and every row-major i32 height byte. Incomplete writers leave
//! finalized=0 and cannot be opened. This is not a world-manifest version.

use std::io::{self, Read, Seek, SeekFrom, Write};

use blake3::Hasher;
use thiserror::Error;

use crate::{
    terrain::{interpolate_stencil, sample_stencil, validated_geometry},
    HeightMm, TerrainFieldError, TerrainPoint,
};

const MAGIC: [u8; 8] = *b"ARDATERR";
const MAJOR: u16 = 1;
const MINOR: u16 = 0;
const HEADER_BYTES: u64 = 88;
const HEADER_LEN: usize = 88;
const GEOMETRY_BYTES: usize = 56;
const VERIFY_CHUNK: usize = 65_536;

/// A malformed canonical-terrain file, budget refusal, or I/O failure.
#[derive(Debug, Error)]
pub enum TerrainFileError {
    /// The lattice geometry itself is not representable.
    #[error("invalid terrain geometry: {0}")]
    Geometry(#[from] TerrainFieldError),
    /// The file header is missing, incomplete or contains invalid flags.
    #[error("invalid terrain header: {0}")]
    InvalidHeader(&'static str),
    /// This codec does not read the declared version.
    #[error("unsupported terrain format version {major}.{minor}")]
    UnsupportedVersion {
        /// Stored major version.
        major: u16,
        /// Stored minor version.
        minor: u16,
    },
    /// The declared byte length differs from the actual file length.
    #[error("terrain payload length does not match file length")]
    LengthMismatch,
    /// The geometry and payload digest does not match.
    #[error("terrain geometry or payload checksum mismatch")]
    ChecksumMismatch,
    /// The next row does not contain exactly width samples.
    #[error("terrain row width does not match header")]
    RowWidthMismatch,
    /// More than the declared number of rows were offered.
    #[error("too many terrain rows")]
    TooManyRows,
    /// A new streamed field requires an empty destination.
    #[error("terrain destination is not empty")]
    DestinationNotEmpty,
    /// An earlier write failed, so the destination cannot be finalized.
    #[error("terrain writer is poisoned by a previous write failure")]
    WriterPoisoned,
    /// Finalization was requested before every declared row was written.
    #[error("incomplete terrain rows")]
    IncompleteRows,
    /// A byte count, offset or sample index overflowed.
    #[error("terrain file arithmetic overflow")]
    ArithmeticOverflow,
    /// The requested transient storage was not admitted or could not reserve.
    #[error("terrain reader RAM budget exceeded")]
    ResourceLimit,
    /// Reading, seeking or writing failed.
    #[error("terrain I/O failed: {0}")]
    Io(#[from] io::Error),
}

fn payload_bytes(width: u32, height: u32) -> Result<u64, TerrainFileError> {
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|n| n.checked_mul(4))
        .ok_or(TerrainFileError::ArithmeticOverflow)
}

fn header(
    origin: TerrainPoint,
    spacing_um: u32,
    width: u32,
    height: u32,
    payload: u64,
    finalized: bool,
) -> [u8; HEADER_LEN] {
    let mut h = [0_u8; HEADER_LEN];
    h[..8].copy_from_slice(&MAGIC);
    h[8..10].copy_from_slice(&MAJOR.to_le_bytes());
    h[10..12].copy_from_slice(&MINOR.to_le_bytes());
    h[12..16].copy_from_slice(&u32::from(finalized).to_le_bytes());
    h[16..24].copy_from_slice(&origin.x_um.to_le_bytes());
    h[24..32].copy_from_slice(&origin.y_um.to_le_bytes());
    h[32..36].copy_from_slice(&spacing_um.to_le_bytes());
    h[36..40].copy_from_slice(&width.to_le_bytes());
    h[40..44].copy_from_slice(&height.to_le_bytes());
    h[48..56].copy_from_slice(&payload.to_le_bytes());
    h
}

fn u16_at(h: &[u8; HEADER_LEN], at: usize) -> u16 {
    u16::from_le_bytes([h[at], h[at + 1]])
}
fn u32_at(h: &[u8; HEADER_LEN], at: usize) -> u32 {
    u32::from_le_bytes([h[at], h[at + 1], h[at + 2], h[at + 3]])
}
fn i64_at(h: &[u8; HEADER_LEN], at: usize) -> i64 {
    i64::from_le_bytes([
        h[at],
        h[at + 1],
        h[at + 2],
        h[at + 3],
        h[at + 4],
        h[at + 5],
        h[at + 6],
        h[at + 7],
    ])
}
fn u64_at(h: &[u8; HEADER_LEN], at: usize) -> u64 {
    u64::from_le_bytes([
        h[at],
        h[at + 1],
        h[at + 2],
        h[at + 3],
        h[at + 4],
        h[at + 5],
        h[at + 6],
        h[at + 7],
    ])
}

/// Writes exactly one validated row at a time without owning the whole field.
///
/// The caller creates the destination transactionally and owns any outer
/// `BufWriter` memory. This writer keeps only a fixed header and four-byte
/// sample scratch. Dropping it before `finish` leaves finalized=0.
pub struct TerrainFileWriter<W: Write + Seek> {
    inner: W,
    final_header: [u8; HEADER_LEN],
    hasher: Hasher,
    width: u32,
    height: u32,
    rows: u32,
    poisoned: bool,
}

impl<W: Write + Seek> TerrainFileWriter<W> {
    /// Starts a new terrain file at the underlying writer's beginning.
    pub fn new(
        mut inner: W,
        origin: TerrainPoint,
        spacing_um: u32,
        width: u32,
        height: u32,
    ) -> Result<Self, TerrainFileError> {
        validated_geometry(origin, spacing_um, width, height)?;
        let payload = payload_bytes(width, height)?;
        let final_header = header(origin, spacing_um, width, height, payload, true);
        let provisional = header(origin, spacing_um, width, height, payload, false);
        if inner.seek(SeekFrom::End(0))? != 0 {
            return Err(TerrainFileError::DestinationNotEmpty);
        }
        inner.seek(SeekFrom::Start(0))?;
        inner.write_all(&provisional)?;
        let mut hasher = Hasher::new();
        hasher.update(&final_header[..GEOMETRY_BYTES]);
        Ok(Self {
            inner,
            final_header,
            hasher,
            width,
            height,
            rows: 0,
            poisoned: false,
        })
    }

    /// Appends one full row in row-major order.
    pub fn write_row(&mut self, row: &[HeightMm]) -> Result<(), TerrainFileError> {
        if self.poisoned {
            return Err(TerrainFileError::WriterPoisoned);
        }
        if self.rows == self.height {
            return Err(TerrainFileError::TooManyRows);
        }
        if row.len()
            != usize::try_from(self.width).map_err(|_| TerrainFileError::ArithmeticOverflow)?
        {
            return Err(TerrainFileError::RowWidthMismatch);
        }
        for &height in row {
            let bytes = height.raw().to_le_bytes();
            if let Err(error) = self.inner.write_all(&bytes) {
                self.poisoned = true;
                return Err(TerrainFileError::Io(error));
            }
            self.hasher.update(&bytes);
        }
        self.rows += 1;
        Ok(())
    }

    /// Completes the file only after all rows, then returns the owned writer.
    pub fn finish(mut self) -> Result<W, TerrainFileError> {
        if self.poisoned {
            return Err(TerrainFileError::WriterPoisoned);
        }
        if self.rows != self.height {
            return Err(TerrainFileError::IncompleteRows);
        }
        let end = HEADER_BYTES
            .checked_add(u64_at(&self.final_header, 48))
            .ok_or(TerrainFileError::ArithmeticOverflow)?;
        if self.inner.seek(SeekFrom::End(0))? != end {
            return Err(TerrainFileError::LengthMismatch);
        }
        let digest = self.hasher.finalize();
        self.final_header[GEOMETRY_BYTES..].copy_from_slice(digest.as_bytes());
        self.inner.seek(SeekFrom::Start(0))?;
        self.inner.write_all(&self.final_header)?;
        self.inner.flush()?;
        self.inner.seek(SeekFrom::Start(end))?;
        Ok(self.inner)
    }
}

/// Validated file-backed canonical terrain with two row buffers.
///
/// `open` verifies the exact length and BLAKE3 geometry+payload checksum
/// before exposing any sample. The caller's RAM budget must cover a 64 KiB
/// streaming-verification buffer **plus** two decoded-byte rows. These are
/// transient reader allocations; the caller-owned `Read + Seek` object is
/// excluded. External mutation of that object/file after validation is not
/// supported; a later short read is reported as I/O failure.
pub struct TerrainFileReader<R: Read + Seek> {
    inner: R,
    origin: TerrainPoint,
    spacing_um: u32,
    width: u32,
    height: u32,
    row_bytes: usize,
    rows: [Vec<u8>; 2],
    cached_y: [Option<u32>; 2],
}

impl<R: Read + Seek> TerrainFileReader<R> {
    /// Opens a fully finalized, checksum-verified terrain file.
    pub fn open(mut inner: R, max_transient_bytes: u64) -> Result<Self, TerrainFileError> {
        inner.seek(SeekFrom::Start(0))?;
        let mut h = [0_u8; HEADER_LEN];
        inner.read_exact(&mut h)?;
        if h[..8] != MAGIC {
            return Err(TerrainFileError::InvalidHeader("magic"));
        }
        let major = u16_at(&h, 8);
        let minor = u16_at(&h, 10);
        if major != MAJOR || minor != MINOR {
            return Err(TerrainFileError::UnsupportedVersion { major, minor });
        }
        if u32_at(&h, 12) != 1 {
            return Err(TerrainFileError::InvalidHeader("not finalized"));
        }
        if u32_at(&h, 44) != 0 {
            return Err(TerrainFileError::InvalidHeader("reserved"));
        }
        let origin = TerrainPoint {
            x_um: i64_at(&h, 16),
            y_um: i64_at(&h, 24),
        };
        let spacing_um = u32_at(&h, 32);
        let width = u32_at(&h, 36);
        let height = u32_at(&h, 40);
        validated_geometry(origin, spacing_um, width, height)?;
        let payload = payload_bytes(width, height)?;
        if u64_at(&h, 48) != payload {
            return Err(TerrainFileError::LengthMismatch);
        }
        let expected = HEADER_BYTES
            .checked_add(payload)
            .ok_or(TerrainFileError::ArithmeticOverflow)?;
        if inner.seek(SeekFrom::End(0))? != expected {
            return Err(TerrainFileError::LengthMismatch);
        }
        let row_bytes = usize::try_from(u64::from(width) * 4)
            .map_err(|_| TerrainFileError::ArithmeticOverflow)?;
        let budget = u64::try_from(VERIFY_CHUNK)
            .map_err(|_| TerrainFileError::ArithmeticOverflow)?
            .checked_add(
                u64::try_from(row_bytes).map_err(|_| TerrainFileError::ArithmeticOverflow)? * 2,
            )
            .ok_or(TerrainFileError::ArithmeticOverflow)?;
        if budget > max_transient_bytes {
            return Err(TerrainFileError::ResourceLimit);
        }
        let mut verify = Vec::new();
        verify
            .try_reserve_exact(VERIFY_CHUNK)
            .map_err(|_| TerrainFileError::ResourceLimit)?;
        verify.resize(VERIFY_CHUNK, 0);
        let mut hasher = Hasher::new();
        hasher.update(&h[..GEOMETRY_BYTES]);
        inner.seek(SeekFrom::Start(HEADER_BYTES))?;
        let mut remaining = payload;
        while remaining > 0 {
            let chunk = usize::try_from(remaining.min(
                u64::try_from(VERIFY_CHUNK).map_err(|_| TerrainFileError::ArithmeticOverflow)?,
            ))
            .map_err(|_| TerrainFileError::ArithmeticOverflow)?;
            inner.read_exact(&mut verify[..chunk])?;
            hasher.update(&verify[..chunk]);
            remaining -= u64::try_from(chunk).map_err(|_| TerrainFileError::ArithmeticOverflow)?;
        }
        if h[GEOMETRY_BYTES..] != *hasher.finalize().as_bytes() {
            return Err(TerrainFileError::ChecksumMismatch);
        }
        drop(verify);
        let mut rows = [Vec::new(), Vec::new()];
        for row in &mut rows {
            row.try_reserve_exact(row_bytes)
                .map_err(|_| TerrainFileError::ResourceLimit)?;
            row.resize(row_bytes, 0);
        }
        Ok(Self {
            inner,
            origin,
            spacing_um,
            width,
            height,
            row_bytes,
            rows,
            cached_y: [None, None],
        })
    }

    /// First lattice position.
    #[must_use]
    pub const fn origin(&self) -> TerrainPoint {
        self.origin
    }
    /// Distance between stored samples, in micrometres.
    #[must_use]
    pub const fn spacing_um(&self) -> u32 {
        self.spacing_um
    }
    /// Number of samples on the X axis.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }
    /// Number of samples on the Y axis.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    fn load_row(&mut self, slot: usize, y: u32) -> Result<(), TerrainFileError> {
        if self.cached_y[slot] == Some(y) {
            return Ok(());
        }
        let other = 1 - slot;
        if self.cached_y[other] == Some(y) {
            self.rows.swap(slot, other);
            self.cached_y.swap(slot, other);
            return Ok(());
        }
        // A short read may have overwritten part of the previous row. Never
        // let a later query observe that partial buffer as a cached row.
        self.cached_y[slot] = None;
        let offset = HEADER_BYTES
            .checked_add(
                u64::from(y)
                    * u64::try_from(self.row_bytes)
                        .map_err(|_| TerrainFileError::ArithmeticOverflow)?,
            )
            .ok_or(TerrainFileError::ArithmeticOverflow)?;
        self.inner.seek(SeekFrom::Start(offset))?;
        self.inner.read_exact(&mut self.rows[slot])?;
        self.cached_y[slot] = Some(y);
        Ok(())
    }

    /// Samples the verified field with the same integer bilinear rule as
    /// [`crate::TerrainField`], returning `None` strictly outside coverage.
    pub fn sample(&mut self, point: TerrainPoint) -> Result<Option<HeightMm>, TerrainFileError> {
        let Some(stencil) =
            sample_stencil(self.origin, self.spacing_um, self.width, self.height, point)
        else {
            return Ok(None);
        };
        self.load_row(0, stencil.y)?;
        self.load_row(1, stencil.y + 1)?;
        let x = usize::try_from(stencil.x).map_err(|_| TerrainFileError::ArithmeticOverflow)?;
        let at = |row: &[u8], column: usize| -> Result<HeightMm, TerrainFileError> {
            let start = column
                .checked_mul(4)
                .ok_or(TerrainFileError::ArithmeticOverflow)?;
            let bytes = row
                .get(start..start + 4)
                .ok_or(TerrainFileError::LengthMismatch)?;
            let array: [u8; 4] = bytes
                .try_into()
                .map_err(|_| TerrainFileError::LengthMismatch)?;
            Ok(HeightMm::new(i32::from_le_bytes(array)))
        };
        let corners = [
            at(&self.rows[0], x)?,
            at(&self.rows[0], x + 1)?,
            at(&self.rows[1], x)?,
            at(&self.rows[1], x + 1)?,
        ];
        interpolate_stencil(&stencil, corners)
            .ok_or(TerrainFileError::ArithmeticOverflow)
            .map(Some)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{self, File, OpenOptions},
        io::Write,
    };

    use super::*;
    use crate::{formats::TempDir, TerrainField};

    const BUDGET: u64 = 65_536 + 2 * 4 * 4;
    const ORIGIN: TerrainPoint = TerrainPoint {
        x_um: -91,
        y_um: 23,
    };
    const HEIGHTS: [i32; 12] = [-9, 0, 7, 12, 5, -2, 21, -14, 17, 8, -31, 4];

    fn create(path: &std::path::Path) {
        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .open(path)
            .unwrap();
        let mut writer = TerrainFileWriter::new(file, ORIGIN, 7, 4, 3).unwrap();
        for row in HEIGHTS.as_chunks::<4>().0 {
            writer
                .write_row(&row.iter().copied().map(HeightMm::new).collect::<Vec<_>>())
                .unwrap();
        }
        writer.finish().unwrap();
    }

    fn error_for_bytes(bytes: &[u8]) -> TerrainFileError {
        let temp = TempDir::new();
        let path = temp.path().join("field.bin");
        fs::write(&path, bytes).unwrap();
        match TerrainFileReader::open(File::open(path).unwrap(), BUDGET) {
            Ok(_) => panic!("corrupt file was accepted"),
            Err(error) => error,
        }
    }

    #[test]
    fn real_file_matches_owned_field_across_cache_order_and_signed_fractions() {
        let temp = TempDir::new();
        let path = temp.path().join("field.bin");
        create(&path);
        let mut reader = TerrainFileReader::open(File::open(&path).unwrap(), BUDGET).unwrap();
        let owned =
            TerrainField::new(ORIGIN, 7, 4, 3, HEIGHTS.map(HeightMm::new).to_vec()).unwrap();
        assert_eq!(
            (
                reader.origin(),
                reader.spacing_um(),
                reader.width(),
                reader.height()
            ),
            (ORIGIN, 7, 4, 3)
        );
        for &y in &[23, 37, 26, 30, 24, 37, 22, 38] {
            for x in -92..=-69 {
                let point = TerrainPoint { x_um: x, y_um: y };
                assert_eq!(
                    reader.sample(point).unwrap(),
                    owned.sample(point),
                    "{point:?}"
                );
            }
        }
    }

    #[test]
    fn header_payload_version_and_completion_are_validated() {
        let temp = TempDir::new();
        let path = temp.path().join("field.bin");
        create(&path);
        let bytes = fs::read(&path).unwrap();
        let mut changed = bytes.clone();
        changed[HEADER_LEN + 9] ^= 1;
        assert!(matches!(
            error_for_bytes(&changed),
            TerrainFileError::ChecksumMismatch
        ));
        let mut changed = bytes.clone();
        changed[16] ^= 1;
        assert!(matches!(
            error_for_bytes(&changed),
            TerrainFileError::ChecksumMismatch
        ));
        let mut changed = bytes.clone();
        changed[8] = 2;
        assert!(matches!(
            error_for_bytes(&changed),
            TerrainFileError::UnsupportedVersion { .. }
        ));
        let mut changed = bytes.clone();
        changed[10] = 1;
        assert!(matches!(
            error_for_bytes(&changed),
            TerrainFileError::UnsupportedVersion { .. }
        ));
        let mut changed = bytes.clone();
        changed[44] = 1;
        assert!(matches!(
            error_for_bytes(&changed),
            TerrainFileError::InvalidHeader("reserved")
        ));
        let mut changed = bytes.clone();
        changed[12] = 0;
        assert!(matches!(
            error_for_bytes(&changed),
            TerrainFileError::InvalidHeader("not finalized")
        ));
        let mut changed = bytes.clone();
        changed[48] ^= 1;
        assert!(matches!(
            error_for_bytes(&changed),
            TerrainFileError::LengthMismatch
        ));
        assert!(matches!(
            error_for_bytes(&bytes[..bytes.len() - 1]),
            TerrainFileError::LengthMismatch
        ));
        let mut changed = bytes;
        changed.push(0);
        assert!(matches!(
            error_for_bytes(&changed),
            TerrainFileError::LengthMismatch
        ));
    }

    #[test]
    fn writer_rejects_wrong_rows_nonempty_dest_and_low_reader_budget() {
        let temp = TempDir::new();
        let path = temp.path().join("partial.bin");
        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        let mut writer = TerrainFileWriter::new(file, ORIGIN, 7, 4, 3).unwrap();
        assert!(matches!(
            writer.write_row(&[HeightMm::new(1)]),
            Err(TerrainFileError::RowWidthMismatch)
        ));
        writer.write_row(&[HeightMm::new(1); 4]).unwrap();
        assert!(matches!(
            writer.finish(),
            Err(TerrainFileError::IncompleteRows)
        ));
        assert!(matches!(
            TerrainFileReader::open(File::open(&path).unwrap(), BUDGET),
            Err(TerrainFileError::InvalidHeader("not finalized"))
        ));
        assert!(matches!(
            TerrainFileWriter::new(
                OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&path)
                    .unwrap(),
                ORIGIN,
                7,
                4,
                3
            ),
            Err(TerrainFileError::DestinationNotEmpty)
        ));

        let full = temp.path().join("full.bin");
        create(&full);
        assert!(matches!(
            TerrainFileReader::open(File::open(full).unwrap(), BUDGET - 1),
            Err(TerrainFileError::ResourceLimit)
        ));

        let extra = temp.path().join("extra.bin");
        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .open(extra)
            .unwrap();
        let mut writer = TerrainFileWriter::new(file, ORIGIN, 7, 4, 3).unwrap();
        for _ in 0..3 {
            writer.write_row(&[HeightMm::new(-1); 4]).unwrap();
        }
        assert!(matches!(
            writer.write_row(&[HeightMm::new(-1); 4]),
            Err(TerrainFileError::TooManyRows)
        ));
        writer.finish().unwrap();
    }

    #[test]
    fn short_row_read_invalidates_cache_and_can_be_retried() {
        let temp = TempDir::new();
        let path = temp.path().join("field.bin");
        create(&path);
        let original = fs::read(&path).unwrap();
        let mut reader = TerrainFileReader::open(File::open(&path).unwrap(), BUDGET).unwrap();
        let p0 = TerrainPoint {
            x_um: -90,
            y_um: 24,
        };
        let p1 = TerrainPoint {
            x_um: -90,
            y_um: 37,
        };
        reader.sample(p0).unwrap();
        OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(HEADER_BYTES + 2 * 16 + 2)
            .unwrap();
        assert!(matches!(reader.sample(p1), Err(TerrainFileError::Io(_))));
        let mut restore = OpenOptions::new().write(true).open(&path).unwrap();
        restore.write_all(&original).unwrap();
        assert_eq!(reader.sample(p1).unwrap(), Some(HeightMm::new(16)));
        assert_eq!(reader.sample(p0).unwrap(), Some(HeightMm::new(-6)));
    }
}
