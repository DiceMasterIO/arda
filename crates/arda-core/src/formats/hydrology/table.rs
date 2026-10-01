//! Format-4 global tables: stable table tags, fixed-record codecs, the
//! table header, and streaming span readers and writers.

use super::*;
use std::io::{Read, Seek, SeekFrom, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
/// Stable format-4 table tags. Unknown values are rejected.
pub enum TableKind {
    /// One root metadata row.
    Metadata = 1,
    /// Fixed physical basin-tree nodes.
    Basins = 2,
    /// Streamed child basin IDs referenced by node spans.
    Children = 3,
    /// Connected representative wet components.
    Lakes = 4,
    /// Authoritative solved channel reaches.
    Reaches = 5,
    /// Canonical cross-area transfer records.
    Crossings = 6,
    /// Saved physical channel edge geometry.
    Channels = 11,
    /// Compact annual catchment identities; seasonal tags 7 through 10 are retired.
    Catchments = 12,
}
impl TableKind {
    pub(super) fn parse(v: u32) -> Result<Self> {
        Ok(match v {
            1 => Self::Metadata,
            2 => Self::Basins,
            3 => Self::Children,
            4 => Self::Lakes,
            5 => Self::Reaches,
            6 => Self::Crossings,
            11 => Self::Channels,
            12 => Self::Catchments,
            _ => return Err(HydrologyFormatError::UnknownTag),
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Canonical record ordering key, independent of Rust struct layout.
pub struct RecordKey(#[doc = "Four canonical unsigned key components."] pub [u64; 4]);
/// Sealed explicit codec implementations, shared by global tables and area copies.
#[allow(private_bounds)]
pub trait FixedRecord: Wire {
    /// Stable table tag for this explicit record schema.
    const KIND: TableKind;
    /// Exact byte width, independent of Rust padding.
    const WIDTH: usize = Self::SIZE;
    /// Whether the complete table is strictly sorted by unique keys.
    const KEYED: bool = true;
    /// Canonical comparison key; link tables validate order within each span.
    fn key(&self) -> RecordKey;
}
macro_rules! record {
    ($t:ty,$kind:ident,$key:expr) => {
        impl FixedRecord for $t {
            const KIND: TableKind = TableKind::$kind;
            fn key(&self) -> RecordKey {
                RecordKey(($key)(self))
            }
        }
    };
}
record!(HydrologyMetadata, Metadata, |_: &HydrologyMetadata| [
    0, 0, 0, 0
]);
record!(BasinNodeRow, Basins, |v: &BasinNodeRow| [v.id.0, 0, 0, 0]);
impl FixedRecord for BasinId {
    const KIND: TableKind = TableKind::Children;
    const KEYED: bool = false;
    fn key(&self) -> RecordKey {
        RecordKey([self.0, 0, 0, 0])
    }
}
record!(GlobalLake, Lakes, |v: &GlobalLake| [v.basin.0, 0, 0, 0]);
record!(GlobalReach, Reaches, |v: &GlobalReach| [v.id.0, 0, 0, 0]);
record!(SharedCrossing, Crossings, |v: &SharedCrossing| [
    u64::from(v.id.low.x),
    u64::from(v.id.low.y),
    u64::from(v.id.high.x),
    u64::from(v.id.high.y)
]);
record!(AnnualCatchment, Catchments, |v: &AnnualCatchment| [
    v.catchment.0,
    0,
    0,
    0
]);
record!(ChannelEdge, Channels, |v: &ChannelEdge| [
    u64::from(v.from.x),
    u64::from(v.from.y),
    u64::from(v.to.x),
    u64::from(v.to.y)
]);

/// Encode one normalized record using its explicit little-endian schema.
pub fn encode_record<T: FixedRecord>(record: &T) -> Result<Vec<u8>> {
    let mut w = Encoder {
        bytes: Vec::with_capacity(T::WIDTH),
    };
    record.put(&mut w)?;
    require(
        w.bytes.len() == T::WIDTH,
        "record width differs from schema",
    )?;
    Ok(w.bytes)
}
/// Decode exactly one record, rejecting truncation, trailing bytes, tags, and invalid fields.
pub fn decode_record<T: FixedRecord>(bytes: &[u8]) -> Result<T> {
    if bytes.len() < T::WIDTH {
        return Err(HydrologyFormatError::Truncated);
    }
    if bytes.len() > T::WIDTH {
        return Err(HydrologyFormatError::TrailingBytes);
    }
    let mut r = Decoder::new(bytes);
    let value = T::get(&mut r)?;
    r.finish()?;
    Ok(value)
}
#[derive(Debug, Clone, Copy)]
/// The explicit 32-byte global table header.
pub struct TableHeader {
    /// Stable table tag identifying the record schema.
    pub kind: TableKind,
    /// Exact encoded byte width of every record.
    pub record_bytes: u32,
    /// Number of records, checked before reading or allocation.
    pub count: u64,
    /// Checked record count multiplied by record width.
    pub payload_bytes: u64,
}
impl TableHeader {
    /// Build an overflow-checked header for a declared fixed record count.
    pub fn for_type<T: FixedRecord>(count: u64) -> Result<Self> {
        Ok(Self {
            kind: T::KIND,
            record_bytes: u32::try_from(T::WIDTH).map_err(|_| HydrologyFormatError::Overflow)?,
            count,
            payload_bytes: count
                .checked_mul(T::WIDTH as u64)
                .ok_or(HydrologyFormatError::Overflow)?,
        })
    }
    /// Encode this global header in its fixed 32-byte representation.
    pub fn encode(self) -> Result<[u8; 32]> {
        let mut w = Encoder {
            bytes: Vec::with_capacity(32),
        };
        w.bytes.extend(TABLE_MAGIC);
        (self.kind as u32).put(&mut w)?;
        self.record_bytes.put(&mut w)?;
        self.count.put(&mut w)?;
        self.payload_bytes.put(&mut w)?;
        w.bytes
            .try_into()
            .map_err(|_| HydrologyFormatError::Invalid("header width"))
    }
    /// Decode a complete header and verify its count-to-payload arithmetic.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let mut r = Decoder::new(bytes);
        require(r.take(8)? == TABLE_MAGIC, "bad hydrology table magic")?;
        let h = Self {
            kind: TableKind::parse(u32::get(&mut r)?)?,
            record_bytes: u32::get(&mut r)?,
            count: u64::get(&mut r)?,
            payload_bytes: u64::get(&mut r)?,
        };
        r.finish()?;
        require(
            h.count
                .checked_mul(u64::from(h.record_bytes))
                .ok_or(HydrologyFormatError::Overflow)?
                == h.payload_bytes,
            "table count/size disagree",
        )?;
        Ok(h)
    }
    fn validate<T: FixedRecord>(
        self,
        file_bytes: u64,
        max_bytes: u64,
        max_records: u64,
    ) -> Result<()> {
        require(
            self.kind == T::KIND && self.record_bytes as usize == T::WIDTH,
            "wrong hydrology table schema",
        )?;
        if self.count > max_records || file_bytes > max_bytes {
            return Err(HydrologyFormatError::Limit(
                "global table exceeds declared reader limit",
            ));
        }
        require(
            TABLE_HEADER_BYTES
                .checked_add(self.payload_bytes)
                .ok_or(HydrologyFormatError::Overflow)?
                == file_bytes,
            "table length disagrees with header",
        )?;
        require(
            self.kind != TableKind::Metadata || self.count == 1,
            "metadata table must contain exactly one row",
        )
    }
}
/// Constant-memory random or streaming reader; construction validates length before allocating a row.
pub struct TableReader<R, T> {
    reader: R,
    header: TableHeader,
    next: u64,
    previous: Option<RecordKey>,
    _record: std::marker::PhantomData<T>,
}
impl<R: Read + Seek, T: FixedRecord> TableReader<R, T> {
    /// Validate file length, schema, and limits before creating a constant-memory table reader.
    pub fn open(mut reader: R, max_bytes: u64, max_records: u64) -> Result<Self> {
        let len = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;
        let mut bytes = [0; 32];
        reader.read_exact(&mut bytes)?;
        let header = TableHeader::decode(&bytes)?;
        header.validate::<T>(len, max_bytes, max_records)?;
        Ok(Self {
            reader,
            header,
            next: 0,
            previous: None,
            _record: std::marker::PhantomData,
        })
    }
    /// Return the validated number of rows in this table.
    pub fn count(&self) -> u64 {
        self.header.count
    }
    /// Read and validate one indexed row without materializing the table.
    pub fn read_at(&mut self, index: u64) -> Result<T> {
        require(index < self.header.count, "record index leaves table")?;
        let offset = index
            .checked_mul(T::WIDTH as u64)
            .and_then(|v| v.checked_add(TABLE_HEADER_BYTES))
            .ok_or(HydrologyFormatError::Overflow)?;
        self.reader.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0; T::WIDTH];
        self.reader.read_exact(&mut bytes)?;
        decode_record(&bytes)
    }
    /// Stream one row, checking unique ascending keys for keyed tables.
    pub fn next_record(&mut self) -> Result<Option<T>> {
        if self.next == self.header.count {
            return Ok(None);
        }
        let value = self.read_at(self.next)?;
        if T::KEYED {
            require(
                self.previous.is_none_or(|p| p < value.key()),
                "table keys are not unique ascending",
            )?;
            self.previous = Some(value.key());
        }
        self.next += 1;
        Ok(Some(value))
    }
}
/// Borrowed linked-table range; each yielded row is checked without collecting children or carries.
pub struct SpanReader<'a, R, T> {
    table: &'a mut TableReader<R, T>,
    next: u64,
    end: u64,
    previous: Option<RecordKey>,
}
impl<R: Read + Seek, T: FixedRecord> TableReader<R, T> {
    /// Validate a linked range and stream unique ascending child IDs or carry stream keys.
    pub fn span(&mut self, span: TableSpan) -> Result<SpanReader<'_, R, T>> {
        span.validate_total(self.header.count)?;
        Ok(SpanReader {
            table: self,
            next: span.offset,
            end: span.offset + span.count,
            previous: None,
        })
    }
}
impl<R: Read + Seek, T: FixedRecord> Iterator for SpanReader<'_, R, T> {
    type Item = Result<T>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next == self.end {
            return None;
        }
        let result = self.table.read_at(self.next).and_then(|v| {
            require(
                self.previous.is_none_or(|p| p < v.key()),
                "linked span is not unique ascending",
            )?;
            self.previous = Some(v.key());
            Ok(v)
        });
        self.next += 1;
        if result.is_err() {
            self.next = self.end;
        }
        Some(result)
    }
}
/// Check that row-ordered nonempty spans partition a flat linked table exactly once.
/// Empty spans have the canonical zero offset and consume no records.
pub fn validate_span_partition(
    spans: impl IntoIterator<Item = TableSpan>,
    total: u64,
) -> Result<()> {
    let mut next = 0_u64;
    for span in spans {
        span.validate_total(total)?;
        if span.count == 0 {
            continue;
        }
        require(
            span.offset == next,
            "linked spans overlap, contain gaps, or are reordered",
        )?;
        next = next
            .checked_add(span.count)
            .ok_or(HydrologyFormatError::Overflow)?;
    }
    require(next == total, "linked table has unreferenced records")
}

/// Constant-memory writer that enforces declared count and canonical keyed order.
pub struct TableWriter<W, T> {
    writer: W,
    count: u64,
    written: u64,
    previous: Option<RecordKey>,
    _record: std::marker::PhantomData<T>,
}
impl<W: Write, T: FixedRecord> TableWriter<W, T> {
    /// Write a checked header and begin a fixed-count table transaction.
    pub fn create(mut writer: W, count: u64) -> Result<Self> {
        let header = TableHeader::for_type::<T>(count)?;
        require(
            T::KIND != TableKind::Metadata || count == 1,
            "metadata count",
        )?;
        writer.write_all(&header.encode()?)?;
        Ok(Self {
            writer,
            count,
            written: 0,
            previous: None,
            _record: std::marker::PhantomData,
        })
    }
    /// Validate and write a single row, enforcing declared count and canonical order.
    pub fn write_record(&mut self, value: &T) -> Result<()> {
        require(self.written < self.count, "more rows than declared")?;
        if T::KEYED {
            require(
                self.previous.is_none_or(|p| p < value.key()),
                "table keys are not unique ascending",
            )?;
        }
        let bytes = encode_record(value)?;
        self.writer.write_all(&bytes)?;
        self.previous = Some(value.key());
        self.written += 1;
        Ok(())
    }
    /// Reject incomplete tables and flush the completed writer.
    pub fn finish(mut self) -> Result<W> {
        require(self.written == self.count, "fewer rows than declared")?;
        self.writer.flush()?;
        Ok(self.writer)
    }
}
