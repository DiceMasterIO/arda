//! Fixed-width little-endian byte cursors for private rows: the checksummed
//! writer and the strict reader with canonical optional and spill fields.

use super::*;

pub(super) struct Write<const N: usize> {
    bytes: [u8; N],
    at: usize,
}
impl<const N: usize> Write<N> {
    pub(super) fn new() -> Self {
        Self {
            bytes: [0; N],
            at: 0,
        }
    }
    pub(super) fn b(&mut self, b: &[u8]) {
        self.bytes[self.at..self.at + b.len()].copy_from_slice(b);
        self.at += b.len();
    }
    pub(super) fn u8(&mut self, v: u8) {
        self.b(&[v]);
    }
    pub(super) fn u32(&mut self, v: u32) {
        self.b(&v.to_le_bytes());
    }
    pub(super) fn i32(&mut self, v: i32) {
        self.b(&v.to_le_bytes());
    }
    pub(super) fn u64(&mut self, v: u64) {
        self.b(&v.to_le_bytes());
    }
    pub(super) fn opt64(&mut self, v: Option<u64>) {
        self.u8(u8::from(v.is_some()));
        self.u64(v.unwrap_or(0));
    }
    pub(super) fn opt32(&mut self, v: Option<i32>) {
        self.u8(u8::from(v.is_some()));
        self.i32(v.unwrap_or(0));
    }
    pub(super) fn cell(&mut self, v: GlobalCell) {
        self.u32(v.x);
        self.u32(v.y);
    }
    pub(super) fn pending(&mut self, v: Option<PendingSpill>) {
        self.u8(u8::from(v.is_some()));
        if let Some(v) = v {
            self.u32(v.from.raw());
            self.u8(u8::from(v.to.is_some()));
            self.u32(v.to.map_or(0, CellIndex::raw));
            self.i32(v.sill_mm);
            self.u64(v.source_base);
        } else {
            self.b(&[0; 21]);
        }
    }
    pub(super) fn finish(mut self) -> [u8; N] {
        let sum = checksum(&self.bytes[..N - 8]);
        self.bytes[N - 8..].copy_from_slice(&sum.to_le_bytes());
        self.bytes
    }
}
pub(super) struct Read<'a> {
    b: &'a [u8],
    at: usize,
}
impl<'a> Read<'a> {
    pub(super) fn new(b: &'a [u8], n: usize) -> Result<Self> {
        if b.len() != n || n < 8 {
            return Err(RowError("width"));
        }
        let tail: u64 = u64::from_le_bytes(
            b[n - 8..]
                .try_into()
                .map_err(|_| RowError("checksum width"))?,
        );
        if tail != checksum(&b[..n - 8]) {
            return Err(RowError("checksum"));
        }
        Ok(Self {
            b: &b[..n - 8],
            at: 0,
        })
    }
    pub(super) fn b<const N: usize>(&mut self) -> Result<[u8; N]> {
        let end = self.at.checked_add(N).ok_or(RowError("offset overflow"))?;
        let out = self
            .b
            .get(self.at..end)
            .ok_or(RowError("truncated"))?
            .try_into()
            .map_err(|_| RowError("width"))?;
        self.at = end;
        Ok(out)
    }
    pub(super) fn u8(&mut self) -> Result<u8> {
        Ok(self.b::<1>()?[0])
    }
    pub(super) fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.b()?))
    }
    pub(super) fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.b()?))
    }
    pub(super) fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.b()?))
    }
    pub(super) fn flag(&mut self) -> Result<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(RowError("tag")),
        }
    }
    pub(super) fn opt64(&mut self) -> Result<Option<u64>> {
        let yes = self.flag()?;
        let v = self.u64()?;
        if !yes && v != 0 {
            return Err(RowError("absent payload"));
        }
        Ok(yes.then_some(v))
    }
    pub(super) fn opt32(&mut self) -> Result<Option<i32>> {
        let yes = self.flag()?;
        let v = self.i32()?;
        if !yes && v != 0 {
            return Err(RowError("absent payload"));
        }
        Ok(yes.then_some(v))
    }
    pub(super) fn cell(&mut self) -> Result<GlobalCell> {
        Ok(GlobalCell {
            x: self.u32()?,
            y: self.u32()?,
        })
    }
    pub(super) fn pending(&mut self, e: Extent, n: u64) -> Result<Option<PendingSpill>> {
        let yes = self.flag()?;
        let from = self.u32()?;
        let to_yes = self.flag()?;
        let to = self.u32()?;
        let sill = self.i32()?;
        let base = self.u64()?;
        if !yes {
            if from != 0 || to_yes || to != 0 || sill != 0 || base != 0 {
                return Err(RowError("absent spill payload"));
            }
            return Ok(None);
        }
        if !to_yes && to != 0 {
            return Err(RowError("absent endpoint payload"));
        }
        let from = CellIndex::new(from, e).ok_or(RowError("spill source extent"))?;
        let to = if to_yes {
            Some(CellIndex::new(to, e).ok_or(RowError("spill target extent"))?)
        } else {
            None
        };
        if base >= n {
            return Err(RowError("source component range"));
        }
        validate_geometry(e, from, to)?;
        Ok(Some(PendingSpill {
            from,
            to,
            sill_mm: sill,
            source_base: base,
        }))
    }
    pub(super) fn finish(self) -> Result<()> {
        if self.b[self.at..].iter().any(|&v| v != 0) {
            Err(RowError("reserved padding"))
        } else {
            Ok(())
        }
    }
}
