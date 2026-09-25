//! Per-crate error enums (`code-prefs.md` §Q4).
//!
//! Load errors name the manifest problem, version skew carries both
//! versions, corruption names the file, range errors carry valid ranges
//! (`03-conventions.md`).

use thiserror::Error;

/// A `GenerateConfig` field outside its valid range (`logic/01` preconditions).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    /// Continent extent outside the supported range.
    #[error("size {width}x{height} km is out of range (each axis must be {min}-{max} km)")]
    SizeOutOfRange {
        /// Requested width in kilometres.
        width: u32,
        /// Requested height in kilometres.
        height: u32,
        /// Smallest supported axis.
        min: u32,
        /// Largest supported axis.
        max: u32,
    },
    /// Latitude band inverted or outside the supported range.
    #[error("latitude band {south}..{north} is out of range (south < north, both within -80..80)")]
    LatitudeOutOfRange {
        /// Requested southern edge.
        south: i16,
        /// Requested northern edge.
        north: i16,
    },
    /// Mean settlement density outside the supported range.
    #[error("density {value} people/km2 is out of range (must be {min}-{max})")]
    DensityOutOfRange {
        /// Requested density.
        value: u16,
        /// Smallest supported density.
        min: u16,
        /// Largest supported density.
        max: u16,
    },
}

/// A byte-level problem inside `arda-core::formats`.
#[derive(Debug, Error)]
pub enum FormatError {
    /// A declared canonical fine terrain layer is missing or invalid.
    #[error("{path}: {source}")]
    Terrain {
        /// Fixed layer path in the world.
        path: String,
        /// Checked terrain header, checksum, resource, or I/O failure.
        #[source]
        source: crate::formats::terrain::TerrainFileError,
    },
    /// A malformed shared-hydrology record or table.
    #[error("{path}: {source}")]
    Hydrology {
        /// File being encoded or decoded.
        path: String,
        /// Checked record, resource-limit, or I/O failure.
        #[source]
        source: crate::formats::hydrology::HydrologyFormatError,
    },
    /// A format-4 area object container or copied authority is invalid.
    #[error("{path}: {source}")]
    Objects {
        /// File being encoded or decoded.
        path: String,
        /// Checked structural, resource-limit, or hydrology failure.
        #[source]
        source: crate::formats::area_objects_v4::ObjectsFormatError,
    },
    /// The underlying reader or writer failed.
    #[error("io error on {path}: {source}")]
    Io {
        /// File being read or written.
        path: String,
        /// Underlying cause.
        #[source]
        source: std::io::Error,
    },
    /// The stream ended before the layout was satisfied.
    #[error("{path} ended after {read} bytes, expected {expected}")]
    UnexpectedEof {
        /// File being read.
        path: String,
        /// Bytes actually available.
        read: usize,
        /// Bytes the layout requires.
        expected: usize,
    },
    /// The file does not start with its layer magic.
    #[error("{path} is not an arda {layer} layer")]
    BadMagic {
        /// File being read.
        path: String,
        /// Layer the caller expected.
        layer: &'static str,
    },
    /// A stored enum discriminant is not one this build knows.
    #[error("{path} holds unknown {field} discriminant {value}")]
    UnknownDiscriminant {
        /// File being read.
        path: String,
        /// Field being decoded.
        field: &'static str,
        /// The unrecognised byte.
        value: u16,
    },
    /// A header declares grid dimensions this build cannot represent: no
    /// existing variant fits (`UnknownDiscriminant.value` is `u16` and would
    /// truncate a `u32` dimension, `UnexpectedEof` would misname a
    /// representable-range problem as a length problem), so this names it
    /// honestly.
    #[error("{path} declares {width}x{height} cells, too large to represent")]
    DimensionsOverflow {
        /// File being read.
        path: String,
        /// Declared width in cells.
        width: u32,
        /// Declared height in cells.
        height: u32,
    },
}

/// A problem opening or reading a stored world (`logic/05`).
#[derive(Debug, Error)]
pub enum LoadError {
    /// `world.json` is absent — the batch never finished (`mockup/02` States).
    #[error("no world.json in {dir}: the world is missing or the batch did not finish")]
    ManifestMissing {
        /// Directory that was opened.
        dir: String,
    },
    /// `world.json` is present but unparseable.
    #[error("world.json in {dir} is unreadable: {reason}")]
    ManifestUnreadable {
        /// Directory that was opened.
        dir: String,
        /// Parser message.
        reason: String,
    },
    /// The world was written by an incompatible format major.
    #[error(
        "world format {found} is not supported by this build ({supported}); regenerate from the seed"
    )]
    VersionSkew {
        /// Major found in the manifest.
        found: u32,
        /// Major this build writes and reads.
        supported: u32,
    },
    /// A layer file is truncated or malformed.
    #[error("corrupt layer: {source}")]
    Corrupt {
        /// Underlying byte-level cause.
        #[from]
        source: FormatError,
    },
    /// A requested coordinate is outside the world.
    #[error("{what} {x},{y} is outside the world ({max_x},{max_y} is the last)")]
    OutOfRange {
        /// Kind of coordinate requested.
        what: &'static str,
        /// Requested column.
        x: i32,
        /// Requested row.
        y: i32,
        /// Last valid column.
        max_x: i32,
        /// Last valid row.
        max_y: i32,
    },
}
