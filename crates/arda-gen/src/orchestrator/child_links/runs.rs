//! Sorted run files of (parent, child) pairs: the 16-byte pair codec and
//! the budgeted sequential writer and reader.
use super::*;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Pair {
    pub(super) parent: u64,
    pub(super) child: u64,
}
impl Pair {
    pub(super) fn encode(self) -> [u8; 16] {
        let mut b = [0; 16];
        b[..8].copy_from_slice(&self.parent.to_le_bytes());
        b[8..].copy_from_slice(&self.child.to_le_bytes());
        b
    }
    pub(super) fn decode(b: &[u8]) -> Result<Self> {
        if b.len() != 16 {
            return Err(ChildError::Corrupt("pair width"));
        }
        let parent = u64::from_le_bytes(
            b[..8]
                .try_into()
                .map_err(|_| ChildError::Corrupt("parent"))?,
        );
        let child = u64::from_le_bytes(
            b[8..]
                .try_into()
                .map_err(|_| ChildError::Corrupt("child"))?,
        );
        if parent == child {
            return Err(ChildError::Corrupt("self child"));
        }
        Ok(Self { parent, child })
    }
}
pub(super) struct Writer {
    pub(super) file: File,
    pub(super) buffer: Vec<u8>,
    pub(super) written: u64,
}
impl Writer {
    pub(super) fn create(path: &Path, budget: &mut Budget) -> Result<Self> {
        budget.io(0)?;
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        Ok(Self {
            file,
            buffer: Vec::with_capacity(BLOCK),
            written: 0,
        })
    }
    pub(super) fn row(&mut self, row: &[u8], budget: &mut Budget) -> Result<()> {
        if row.len() > BLOCK {
            return Err(ChildError::Invalid("row exceeds I/O buffer"));
        }
        if self.buffer.len() + row.len() > BLOCK {
            self.flush(budget)?;
        }
        self.buffer.extend_from_slice(row);
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
    pub(super) fn finish(mut self, expected: u64, budget: &mut Budget) -> Result<()> {
        self.flush(budget)?;
        if self.written != expected {
            return Err(ChildError::Corrupt("writer count"));
        }
        budget.io(0)?;
        self.file.sync_all()?;
        budget.io(0)?;
        if self.file.metadata()?.len() != expected {
            return Err(ChildError::Corrupt("writer length"));
        }
        Ok(())
    }
    // Drop only closes the file. Failed/dirty buffers are never implicitly retried.
}
pub(super) struct Reader {
    pub(super) file: File,
    pub(super) buffer: Vec<u8>,
    pub(super) at: usize,
    pub(super) end: usize,
    pub(super) remaining: u64,
    pub(super) previous: Option<Pair>,
    pub(super) head: Option<Pair>,
}
impl Reader {
    pub(super) fn open(path: &Path, rows: u64, budget: &mut Budget) -> Result<Self> {
        budget.io(0)?;
        let file = File::open(path)?;
        budget.io(0)?;
        if file.metadata()?.len() != mul(rows, 16)? {
            return Err(ChildError::Corrupt("run length"));
        }
        Ok(Self {
            file,
            buffer: vec![0; BLOCK],
            at: 0,
            end: 0,
            remaining: rows,
            previous: None,
            head: None,
        })
    }
    pub(super) fn advance(&mut self, budget: &mut Budget) -> Result<()> {
        if self.at == self.end {
            if self.remaining == 0 {
                self.head = None;
                return Ok(());
            }
            let rows = self.remaining.min(256);
            let bytes = size(mul(rows, 16)?)?;
            budget.io(bytes)?;
            self.file.read_exact(&mut self.buffer[..bytes])?;
            self.at = 0;
            self.end = bytes;
            self.remaining -= rows;
        }
        let p = Pair::decode(&self.buffer[self.at..self.at + 16])?;
        self.at += 16;
        if let Some(old) = self.previous {
            match budget.compare(&old, &p)? {
                Ordering::Greater => return Err(ChildError::Corrupt("unordered run")),
                Ordering::Equal => return Err(ChildError::Duplicate),
                Ordering::Less => {}
            }
        }
        self.previous = Some(p);
        self.head = Some(p);
        Ok(())
    }
}
