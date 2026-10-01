//! Metered scratch files: the I/O meter, positioned page reads and writes,
//! page geometry and the checked table header.

use super::*;

pub(super) struct Meter {
    pub(super) limits: DiskLimits,
    pub(super) work: DiskWork,
}
impl Meter {
    pub(super) fn charge(&mut self, n: u128) -> Result<()> {
        let bytes = self
            .work
            .bytes
            .checked_add(n)
            .ok_or(DiskError::Limit("I/O bytes"))?;
        let operations = self
            .work
            .operations
            .checked_add(1)
            .ok_or(DiskError::Limit("I/O operations"))?;
        if bytes > self.limits.io_bytes {
            return Err(DiskError::Limit("I/O bytes"));
        }
        if operations > self.limits.io_operations {
            return Err(DiskError::Limit("I/O operations"));
        }
        self.work = DiskWork { bytes, operations };
        Ok(())
    }
}
pub(super) struct Scratch {
    pub(super) file: File,
    pub(super) path: PathBuf,
}
impl Scratch {
    pub(super) fn length(&self, m: &mut Meter) -> Result<u64> {
        m.charge(0)?;
        self.file
            .metadata()
            .map(|v| v.len())
            .map_err(|source| DiskError::Io {
                path: self.path.clone(),
                source,
            })
    }
    pub(super) fn create(path: PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|source| DiskError::Io {
                path: path.clone(),
                source,
            })?;
        Ok(Self { file, path })
    }
    pub(super) fn seek(&mut self, at: u64, m: &mut Meter) -> Result<()> {
        m.charge(0)?;
        self.file
            .seek(SeekFrom::Start(at))
            .map_err(|source| DiskError::Io {
                path: self.path.clone(),
                source,
            })?;
        Ok(())
    }
    pub(super) fn write(&mut self, at: u64, b: &[u8], m: &mut Meter) -> Result<()> {
        self.seek(at, m)?;
        m.charge(b.len() as u128)?;
        self.file.write_all(b).map_err(|source| DiskError::Io {
            path: self.path.clone(),
            source,
        })
    }
    pub(super) fn read(&mut self, at: u64, b: &mut [u8], m: &mut Meter) -> Result<()> {
        self.seek(at, m)?;
        m.charge(b.len() as u128)?;
        self.file.read_exact(b).map_err(|source| DiskError::Io {
            path: self.path.clone(),
            source,
        })
    }
}
pub(super) struct Cached {
    pub(super) bytes: Box<[u8; PAGE]>,
    pub(super) dirty: bool,
}
pub(super) fn slot_bytes(kind: usize) -> usize {
    if kind == 0 {
        UNION_BYTES
    } else {
        NODE_BYTES
    }
}
pub(super) fn slots(kind: usize) -> u64 {
    (PAGE / slot_bytes(kind)) as u64
}
pub(super) fn pages(rows: u64, kind: usize) -> u64 {
    rows.div_ceil(slots(kind))
}
pub(super) fn header(e: Extent, kind: usize, count: u64, width: u32) -> Result<[u8; 64]> {
    let mut h = [0; 64];
    h[..8].copy_from_slice(b"ARDAHSP1");
    h[8] = u8::try_from(kind).map_err(|_| DiskError::Invalid("header kind"))?;
    h[16..24].copy_from_slice(&count.to_le_bytes());
    h[24..28].copy_from_slice(&width.to_le_bytes());
    h[28..32].copy_from_slice(
        &u32::try_from(PAGE)
            .map_err(|_| DiskError::Invalid("page width"))?
            .to_le_bytes(),
    );
    let at = crate::hydrology::routing::CellIndex::new(e.cells() - 1, e)
        .ok_or(DiskError::Invalid("extent"))?;
    let (x, y) = e.coordinates(at);
    h[32..36].copy_from_slice(&(x + 1).to_le_bytes());
    h[36..40].copy_from_slice(&(y + 1).to_le_bytes());
    let sum = private_rows::checksum(&h[..56]);
    h[56..].copy_from_slice(&sum.to_le_bytes());
    Ok(h)
}
