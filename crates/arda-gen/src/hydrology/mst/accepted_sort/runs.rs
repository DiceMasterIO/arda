//! Checksummed run files: the fixed header, 32-byte edge rows, and the
//! budgeted sequential writer and reader shared by runs and the final file.
use super::*;

pub(super) fn header(extent: Extent, count: u64) -> Result<[u8; 32]> {
    let last = CellIndex::new(extent.cells() - 1, extent).ok_or(SortError::Invalid("extent"))?;
    let (x, y) = extent.coordinates(last);
    let mut b = [0; 32];
    b[..8].copy_from_slice(MAGIC);
    b[8..12].copy_from_slice(&(x + 1).to_le_bytes());
    b[12..16].copy_from_slice(&(y + 1).to_le_bytes());
    b[16..24].copy_from_slice(&count.to_le_bytes());
    b[24..28].copy_from_slice(&32u32.to_le_bytes());
    Ok(b)
}
pub(super) fn checksum(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf29ce484222325u64, |h, &v| {
        (h ^ u64::from(v)).wrapping_mul(0x100000001b3)
    })
}
pub(super) fn encode(e: Saddle) -> [u8; 32] {
    let mut b = [0; 32];
    b[..24].copy_from_slice(&e.encode());
    let c = checksum(&b[..24]);
    b[24..].copy_from_slice(&c.to_le_bytes());
    b
}
pub(super) fn decode(b: &[u8], extent: Extent) -> Result<Saddle> {
    if b.len() != 32 {
        return Err(SortError::Corrupt("row width"));
    }
    let c = u64::from_le_bytes(
        b[24..]
            .try_into()
            .map_err(|_| SortError::Corrupt("checksum width"))?,
    );
    if checksum(&b[..24]) != c {
        return Err(SortError::Corrupt("checksum"));
    }
    Saddle::decode(
        b[..24]
            .try_into()
            .map_err(|_| SortError::Corrupt("payload width"))?,
        extent,
    )
    .ok_or(SortError::Corrupt("saddle"))
}
pub(super) struct Writer {
    pub(super) file: File,
    pub(super) buffer: Vec<u8>,
    pub(super) written: u64,
}
impl Writer {
    pub(super) fn create(
        path: &Path,
        extent: Extent,
        count: u64,
        budget: &mut Budget,
    ) -> Result<Self> {
        budget.io(0)?;
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        let mut buffer = Vec::with_capacity(BLOCK);
        buffer.extend_from_slice(&header(extent, count)?);
        Ok(Self {
            file,
            buffer,
            written: 0,
        })
    }
    pub(super) fn row(&mut self, e: Saddle, budget: &mut Budget) -> Result<()> {
        if self.buffer.len() + 32 > BLOCK {
            self.flush(budget)?;
        }
        self.buffer.extend_from_slice(&encode(e));
        Ok(())
    }
    pub(super) fn flush(&mut self, budget: &mut Budget) -> Result<()> {
        if !self.buffer.is_empty() {
            budget.io(self.buffer.len())?;
            self.file.write_all(&self.buffer)?;
            self.written = add(self.written, self.buffer.len() as u64)?;
            self.buffer.clear();
        }
        Ok(())
    }
    pub(super) fn finish(mut self, count: u64, budget: &mut Budget) -> Result<()> {
        self.flush(budget)?;
        let expected = add(32, mul(ROW, count)?)?;
        if self.written != expected {
            return Err(SortError::Corrupt("writer count"));
        }
        budget.io(0)?;
        self.file.sync_all()?;
        budget.io(0)?;
        if self.file.metadata()?.len() != expected {
            return Err(SortError::Corrupt("writer length"));
        }
        Ok(())
    }
}
// No flush-on-drop: a failed private buffer is never silently retried.
pub(super) struct Reader {
    pub(super) file: File,
    pub(super) buffer: Vec<u8>,
    pub(super) at: usize,
    pub(super) end: usize,
    pub(super) remaining: u64,
    pub(super) previous: Option<Saddle>,
    pub(super) head: Option<Saddle>,
}
impl Reader {
    pub(super) fn open(
        path: &Path,
        extent: Extent,
        count: u64,
        budget: &mut Budget,
    ) -> Result<Self> {
        budget.io(0)?;
        let mut file = File::open(path)?;
        budget.io(0)?;
        if file.metadata()?.len() != add(32, mul(ROW, count)?)? {
            return Err(SortError::Corrupt("file length"));
        }
        let mut h = [0; 32];
        budget.io(32)?;
        file.read_exact(&mut h)?;
        if h != header(extent, count)? {
            return Err(SortError::Corrupt("header"));
        }
        Ok(Self {
            file,
            buffer: vec![0; BLOCK],
            at: 0,
            end: 0,
            remaining: count,
            previous: None,
            head: None,
        })
    }
    pub(super) fn advance(&mut self, extent: Extent, budget: &mut Budget) -> Result<()> {
        if self.at == self.end {
            if self.remaining == 0 {
                self.head = None;
                return Ok(());
            }
            let rows = self.remaining.min(128);
            let bytes = size(mul(ROW, rows)?)?;
            budget.io(bytes)?;
            self.file.read_exact(&mut self.buffer[..bytes])?;
            self.remaining -= rows;
            self.at = 0;
            self.end = bytes;
        }
        let e = decode(&self.buffer[self.at..self.at + 32], extent)?;
        self.at += 32;
        if let Some(old) = self.previous {
            match budget.compare(extent, old, e)? {
                Ordering::Less => {}
                Ordering::Equal => return Err(SortError::Duplicate),
                Ordering::Greater => return Err(SortError::Corrupt("run order")),
            }
        }
        self.previous = Some(e);
        self.head = Some(e);
        Ok(())
    }
}
