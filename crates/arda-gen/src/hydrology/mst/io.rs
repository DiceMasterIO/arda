//! Checked private MST I/O and fixed minimum/header records.
use super::hierarchy::Minimum;
use super::routing::{CellIndex, Extent};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};
/// All non-routing producer/storage failures.
#[derive(Debug, thiserror::Error)]
pub enum StageError {
    /// A malformed indexed record, topology binding or lifecycle.
    #[error("invalid MST input: {0}")]
    Invalid(&'static str),
    /// No outgoing witness connects the remaining components.
    #[error("physical MST graph is disconnected")]
    Disconnected,
    /// An accepted nonduplicate edge or a corrupt union link creates a cycle.
    #[error("physical MST contains an unexpected cycle")]
    Cycle,
    /// A declared resource reservation is exhausted.
    #[error("MST exceeded {0}")]
    Limit(&'static str),
    /// Underlying failure on a private file.
    #[error("MST I/O on {path}: {source}")]
    Io {
        /// Exact private path.
        path: PathBuf,
        /// Operating-system failure.
        #[source]
        source: std::io::Error,
    },
}
pub(crate) type Result<T> = std::result::Result<T, StageError>;
/// Actual requested private I/O, excluding the separately admitted accepted sorter.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct IoWork {
    /// Requested payload bytes, including failed attempts.
    pub bytes: u128,
    /// File opens, metadata, seeks and payload calls attempted.
    pub operations: u64,
}
pub(crate) struct Meter {
    pub bytes: u128,
    pub operations: u64,
    pub work: IoWork,
}
impl Meter {
    pub fn new(bytes: u128, operations: u64) -> Self {
        Self {
            bytes,
            operations,
            work: IoWork::default(),
        }
    }
    pub fn charge(&mut self, n: u128) -> Result<()> {
        let bytes = self
            .work
            .bytes
            .checked_add(n)
            .ok_or(StageError::Limit("I/O bytes"))?;
        let operations = self
            .work
            .operations
            .checked_add(1)
            .ok_or(StageError::Limit("I/O operations"))?;
        if bytes > self.bytes {
            return Err(StageError::Limit("I/O bytes"));
        }
        if operations > self.operations {
            return Err(StageError::Limit("I/O operations"));
        }
        self.work = IoWork { bytes, operations };
        Ok(())
    }
}
pub(crate) struct Scratch {
    pub file: File,
    pub path: PathBuf,
}
impl Scratch {
    pub fn create(path: PathBuf, m: &mut Meter) -> Result<Self> {
        m.charge(0)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| StageError::Io {
                path: path.clone(),
                source,
            })?;
        Ok(Self { file, path })
    }
    pub fn length(&self, m: &mut Meter) -> Result<u64> {
        m.charge(0)?;
        self.file
            .metadata()
            .map(|v| v.len())
            .map_err(|source| StageError::Io {
                path: self.path.clone(),
                source,
            })
    }
    fn seek(&mut self, at: u64, m: &mut Meter) -> Result<()> {
        m.charge(0)?;
        self.file
            .seek(SeekFrom::Start(at))
            .map_err(|source| StageError::Io {
                path: self.path.clone(),
                source,
            })?;
        Ok(())
    }
    pub fn read(&mut self, at: u64, b: &mut [u8], m: &mut Meter) -> Result<()> {
        self.seek(at, m)?;
        m.charge(b.len() as u128)?;
        self.file.read_exact(b).map_err(|source| StageError::Io {
            path: self.path.clone(),
            source,
        })
    }
    pub fn write(&mut self, at: u64, b: &[u8], m: &mut Meter) -> Result<()> {
        self.seek(at, m)?;
        m.charge(b.len() as u128)?;
        self.file.write_all(b).map_err(|source| StageError::Io {
            path: self.path.clone(),
            source,
        })
    }
}
pub(crate) fn checksum(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf29ce484222325, |v, &x| {
        (v ^ u64::from(x)).wrapping_mul(0x100000001b3)
    })
}
pub(crate) fn minimum_bytes(m: Minimum) -> [u8; 16] {
    let mut b = [0; 16];
    b[..4].copy_from_slice(&m.at.raw().to_le_bytes());
    b[4..8].copy_from_slice(&m.floor_mm.to_le_bytes());
    let s = checksum(&b[..8]);
    b[8..].copy_from_slice(&s.to_le_bytes());
    b
}
pub(crate) fn minimum(b: &[u8], e: Extent) -> Result<Minimum> {
    if b.len() != 16 {
        return Err(StageError::Invalid("minimum width"));
    }
    let u = |i| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let h = u64::from_le_bytes(
        b[8..]
            .try_into()
            .map_err(|_| StageError::Invalid("minimum checksum"))?,
    );
    if h != checksum(&b[..8]) {
        return Err(StageError::Invalid("minimum checksum"));
    }
    Ok(Minimum {
        at: CellIndex::new(u(0), e).ok_or(StageError::Invalid("minimum extent"))?,
        floor_mm: i32::from_le_bytes(
            b[4..8]
                .try_into()
                .map_err(|_| StageError::Invalid("floor width"))?,
        ),
    })
}
pub(crate) fn header(e: Extent, kind: u8, count: u64, width: u32) -> Result<[u8; 64]> {
    let mut b = [0; 64];
    b[..8].copy_from_slice(b"ARDAMST1");
    b[8] = kind;
    b[16..24].copy_from_slice(&count.to_le_bytes());
    b[24..28].copy_from_slice(&width.to_le_bytes());
    let at = CellIndex::new(e.cells() - 1, e).ok_or(StageError::Invalid("empty extent"))?;
    let (x, y) = e.coordinates(at);
    b[28..32].copy_from_slice(&(x + 1).to_le_bytes());
    b[32..36].copy_from_slice(&(y + 1).to_le_bytes());
    let s = checksum(&b[..56]);
    b[56..].copy_from_slice(&s.to_le_bytes());
    Ok(b)
}
