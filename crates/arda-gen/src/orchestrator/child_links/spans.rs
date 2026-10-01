//! Paged lookup of a parent's child span in the finished span file, and the
//! 24-byte span row codec.
use super::*;

impl DiskChildLinks {
    fn span_row(&mut self, index: u64) -> Result<(u64, TableSpan)> {
        let page = index / PAGE_ROWS;
        if self.cache_page != Some(page) {
            let first = mul(page, PAGE_ROWS)?;
            let rows = (self.span_count - first).min(PAGE_ROWS);
            let bytes = size(mul(rows, 24)?)?;
            self.budget.io(0)?;
            let file = self
                .span_file
                .as_mut()
                .ok_or(ChildError::Invalid("missing span file"))?;
            file.seek(SeekFrom::Start(mul(first, 24)?))?;
            self.budget.io(bytes)?;
            file.read_exact(&mut self.cache[..bytes])?;
            let mut previous = None;
            for row in self.cache[..bytes].as_chunks::<24>().0 {
                let (p, s) = decode_span(row, self.count)?;
                if let Some((old, prior)) = previous {
                    if self.budget.compare(&old, &p)? != Ordering::Less || prior != s.offset {
                        return Err(ChildError::Corrupt("span page order"));
                    }
                }
                previous = Some((p, add(s.offset, s.count)?));
            }
            let (_, initial) = decode_span(&self.cache[..24], self.count)?;
            let (_, final_row) = decode_span(&self.cache[bytes - 24..bytes], self.count)?;
            if (first == 0 && initial.offset != 0)
                || (first + rows == self.span_count
                    && add(final_row.offset, final_row.count)? != self.count)
            {
                return Err(ChildError::Corrupt("span coverage"));
            }
            self.cache_page = Some(page);
            self.cache_rows = size(rows)?;
        }
        let at = size(index % PAGE_ROWS)?;
        if at >= self.cache_rows {
            return Err(ChildError::Corrupt("span row index"));
        }
        decode_span(&self.cache[at * 24..(at + 1) * 24], self.count)
    }
    /// Returns the exact child slice; an absent parent has canonical span `(0,0)`.
    ///
    /// # Errors
    /// Unfinished/poisoned state, malformed private bytes and exhausted runtime lookup budgets fail.
    pub fn span(&mut self, parent: BasinId) -> Result<TableSpan> {
        let result = self.span_inner(parent);
        self.abort(result)
    }
    fn span_inner(&mut self, parent: BasinId) -> Result<TableSpan> {
        match self.stage {
            Stage::Ready => {}
            Stage::Writing => return Err(ChildError::Invalid("unfinished")),
            Stage::Poisoned => return Err(ChildError::Poisoned),
        }
        let (mut lo, mut hi) = (0, self.span_count);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let (key, value) = self.span_row(mid)?;
            match self.budget.compare(&key, &parent.0)? {
                Ordering::Less => lo = mid + 1,
                Ordering::Greater => hi = mid,
                Ordering::Equal => return Ok(value),
            }
        }
        Ok(TableSpan {
            offset: 0,
            count: 0,
        })
    }
}

pub(super) fn write_span(
    writer: &mut Writer,
    parent: u64,
    offset: u64,
    count: u64,
    budget: &mut Budget,
) -> Result<()> {
    let mut bytes = [0; 24];
    bytes[..8].copy_from_slice(&parent.to_le_bytes());
    bytes[8..16].copy_from_slice(&offset.to_le_bytes());
    bytes[16..].copy_from_slice(&count.to_le_bytes());
    writer.row(&bytes, budget)
}
pub(super) fn decode_span(bytes: &[u8], total: u64) -> Result<(u64, TableSpan)> {
    if bytes.len() != 24 {
        return Err(ChildError::Corrupt("span width"));
    }
    let mut values = [0u64; 3];
    for (i, v) in values.iter_mut().enumerate() {
        *v = u64::from_le_bytes(
            bytes[i * 8..i * 8 + 8]
                .try_into()
                .map_err(|_| ChildError::Corrupt("span integer"))?,
        );
    }
    if values[2] == 0 || add(values[1], values[2])? > total {
        return Err(ChildError::Corrupt("span bounds"));
    }
    Ok((
        values[0],
        TableSpan {
            offset: values[1],
            count: values[2],
        },
    ))
}
