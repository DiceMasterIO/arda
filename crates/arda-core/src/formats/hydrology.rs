//! Explicit little-endian format-4 hydrology records; no Rust padding on disk.
//! Global tables stream fixed records. Only bounded area contexts own vectors.
#![deny(missing_docs)]
use crate::hydrology::*;
use crate::{AreaCoord, DischargeMilli, GlobalCell, HeightMm};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Read, Seek, SeekFrom, Write},
};

/// Supported physical hydrology model revision.
pub const MODEL_REVISION: u32 = 2;
/// Number of seconds in the representative 365-day year.
pub const ANNUAL_SECONDS: u128 = 31_536_000;
const TABLE_MAGIC: [u8; 8] = *b"ARDAHYD4";
const CONTEXT_MAGIC: [u8; 8] = *b"ARDACTX4";
/// Encoded byte width of the global table header.
pub const TABLE_HEADER_BYTES: u64 = 32;

#[derive(Debug)]
/// Malformed data, bounded-resource failure, or underlying storage error.
pub enum HydrologyFormatError {
    /// Required bytes are absent.
    Truncated,
    /// Bytes remain after a complete record.
    TrailingBytes,
    /// An enum or boolean tag is not recognized.
    UnknownTag,
    /// A physical or structural invariant is violated.
    Invalid(&'static str),
    /// Checked size or offset arithmetic overflowed.
    Overflow,
    /// A caller-declared allocation or table limit was exceeded.
    Limit(&'static str),
    /// Underlying file access failed.
    Io(std::io::Error),
}
impl From<std::io::Error> for HydrologyFormatError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            Self::Truncated
        } else {
            Self::Io(e)
        }
    }
}
impl std::fmt::Display for HydrologyFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => f.write_str("truncated hydrology record"),
            Self::TrailingBytes => f.write_str("trailing hydrology bytes"),
            Self::UnknownTag => f.write_str("unknown hydrology tag"),
            Self::Invalid(reason) => write!(f, "invalid hydrology record: {reason}"),
            Self::Overflow => f.write_str("hydrology size arithmetic overflow"),
            Self::Limit(reason) => write!(f, "hydrology resource limit: {reason}"),
            Self::Io(e) => std::fmt::Display::fmt(e, f),
        }
    }
}
impl std::error::Error for HydrologyFormatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}
type Result<T> = std::result::Result<T, HydrologyFormatError>;
fn require(ok: bool, reason: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(HydrologyFormatError::Invalid(reason))
    }
}

struct Encoder {
    bytes: Vec<u8>,
}
struct Decoder<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Decoder<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .ok_or(HydrologyFormatError::Overflow)?;
        let out = self
            .bytes
            .get(self.at..end)
            .ok_or(HydrologyFormatError::Truncated)?;
        self.at = end;
        Ok(out)
    }
    fn finish(self) -> Result<()> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(HydrologyFormatError::TrailingBytes)
        }
    }
}
trait Wire: Sized {
    const SIZE: usize;
    fn put(&self, w: &mut Encoder) -> Result<()>;
    fn get(r: &mut Decoder<'_>) -> Result<Self>;
    fn check(&self) -> Result<()> {
        Ok(())
    }
}
macro_rules! integer {
    ($t:ty,$n:expr) => {
        impl Wire for $t {
            const SIZE: usize = $n;
            fn put(&self, w: &mut Encoder) -> Result<()> {
                w.bytes.extend_from_slice(&self.to_le_bytes());
                Ok(())
            }
            fn get(r: &mut Decoder<'_>) -> Result<Self> {
                let b = r.take($n)?;
                Ok(<$t>::from_le_bytes(
                    b.try_into().map_err(|_| HydrologyFormatError::Truncated)?,
                ))
            }
        }
    };
}
integer!(u8, 1);
integer!(u16, 2);
integer!(u32, 4);
integer!(u64, 8);
integer!(u128, 16);
integer!(i32, 4);
impl Wire for bool {
    const SIZE: usize = 1;
    fn put(&self, w: &mut Encoder) -> Result<()> {
        u8::from(*self).put(w)
    }
    fn get(r: &mut Decoder<'_>) -> Result<Self> {
        match u8::get(r)? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(HydrologyFormatError::UnknownTag),
        }
    }
}
macro_rules! wrapper {
    ($t:ty,$raw:ty,$ctor:expr,$extract:expr) => {
        impl Wire for $t {
            const SIZE: usize = <$raw as Wire>::SIZE;
            fn put(&self, w: &mut Encoder) -> Result<()> {
                ($extract)(self).put(w)
            }
            fn get(r: &mut Decoder<'_>) -> Result<Self> {
                Ok(($ctor)(<$raw>::get(r)?))
            }
        }
    };
}
wrapper!(HeightMm, i32, HeightMm::new, |v: &HeightMm| v.raw());
wrapper!(
    DischargeMilli,
    u64,
    DischargeMilli::new,
    |v: &DischargeMilli| v.raw()
);
wrapper!(TerminalId, u64, TerminalId, |v: &TerminalId| v.0);
wrapper!(BasinId, u64, BasinId, |v: &BasinId| v.0);
wrapper!(ReachId, u64, ReachId, |v: &ReachId| v.0);
wrapper!(JunctionId, u64, JunctionId, |v: &JunctionId| v.0);
wrapper!(CatchmentId, u64, CatchmentId, |v: &CatchmentId| v.0);
impl Wire for Litres {
    const SIZE: usize = 16;
    fn put(&self, w: &mut Encoder) -> Result<()> {
        self.check()?;
        self.0.put(w)
    }
    fn get(r: &mut Decoder<'_>) -> Result<Self> {
        let v = Self(u128::get(r)?);
        v.check()?;
        Ok(v)
    }
}
impl<T: Wire, const N: usize> Wire for [T; N] {
    const SIZE: usize = T::SIZE * N;
    fn put(&self, w: &mut Encoder) -> Result<()> {
        for v in self {
            v.put(w)?;
        }
        Ok(())
    }
    fn get(r: &mut Decoder<'_>) -> Result<Self> {
        let mut v = Vec::with_capacity(N);
        for _ in 0..N {
            v.push(T::get(r)?);
        }
        v.try_into()
            .map_err(|_| HydrologyFormatError::Invalid("array count"))
    }
}
impl<T: Wire> Wire for Option<T> {
    const SIZE: usize = 1 + T::SIZE;
    fn put(&self, w: &mut Encoder) -> Result<()> {
        match self {
            None => {
                0_u8.put(w)?;
                w.bytes.resize(w.bytes.len() + T::SIZE, 0);
                Ok(())
            }
            Some(v) => {
                1_u8.put(w)?;
                v.put(w)
            }
        }
    }
    fn get(r: &mut Decoder<'_>) -> Result<Self> {
        match u8::get(r)? {
            0 => {
                require(
                    r.take(T::SIZE)?.iter().all(|&v| v == 0),
                    "absent optional payload is not zero",
                )?;
                Ok(None)
            }
            1 => Ok(Some(T::get(r)?)),
            _ => Err(HydrologyFormatError::UnknownTag),
        }
    }
}
macro_rules! structure {($t:ty,[$($f:ident:$ty:ty),* $(,)?],$valid:expr)=>{impl Wire for $t {
 const SIZE:usize=0 $(+<$ty as Wire>::SIZE)*;
 fn put(&self,w:&mut Encoder)->Result<()>{self.check()?;$(self.$f.put(w)?;)*Ok(())}
 fn get(r:&mut Decoder<'_>)->Result<Self>{let value=Self{$($f:<$ty>::get(r)?,)*};value.check()?;Ok(value)}
 fn check(&self)->Result<()>{($valid)(self)}
}};}
structure!(GlobalCell,[x:u32,y:u32],|_:&GlobalCell|Ok(()));
structure!(AreaCoord,[x:i32,y:i32],|v:&AreaCoord|require(v.x>=0&&v.y>=0,"negative area coordinate"));
structure!(CrossingId,[low:GlobalCell,high:GlobalCell],|v:&CrossingId|require(v.low<v.high&&neighbor(v.low,v.high),"crossing ID is not canonical neighboring pair"));
fn neighbor(a: GlobalCell, b: GlobalCell) -> bool {
    a != b && a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1
}
impl Wire for ReceivingAccount {
    const SIZE: usize = 9;
    fn put(&self, w: &mut Encoder) -> Result<()> {
        let (tag, id): (u8, u64) = match self {
            Self::Reach(x) => (0, x.0),
            Self::Lake(x) => (1, x.0),
            Self::Sea => (2, 0),
            Self::DomainExport => (3, 0),
            Self::Junction(x) => (4, x.0),
        };
        tag.put(w)?;
        id.put(w)
    }
    fn get(r: &mut Decoder<'_>) -> Result<Self> {
        let tag = u8::get(r)?;
        let id = u64::get(r)?;
        match tag {
            0 => Ok(Self::Reach(ReachId(id))),
            1 => Ok(Self::Lake(BasinId(id))),
            4 => Ok(Self::Junction(JunctionId(id))),
            2 | 3 => {
                require(id == 0, "terminal account has nonzero payload")?;
                Ok(if tag == 2 {
                    Self::Sea
                } else {
                    Self::DomainExport
                })
            }
            _ => Err(HydrologyFormatError::UnknownTag),
        }
    }
}
structure!(SpillConnection,[from:GlobalCell,to:Option<GlobalCell>,sill:HeightMm,receiving:ReceivingAccount],|v:&SpillConnection|match (v.to,v.receiving){(None,ReceivingAccount::DomainExport)=>Ok(()),(Some(to),r)=>require(neighbor(v.from,to)&&!matches!(r,ReceivingAccount::DomainExport)&&!matches!(r,ReceivingAccount::Junction(id) if id.cell()!=to),"spill destination is inconsistent"),_=>Err(HydrologyFormatError::Invalid("only domain export may omit destination"))});
fn annual_mean(volume: Litres, mean: DischargeMilli) -> Result<()> {
    require(
        volume.0 / ANNUAL_SECONDS == u128::from(mean.raw()),
        "annual volume and mean discharge disagree",
    )
}
fn annual_balance(v: &AnnualWaterBalance) -> Result<()> {
    require(
        v.land_loss.0 <= v.land_precipitation.0,
        "land loss exceeds precipitation",
    )?;
    let source = v
        .land_precipitation
        .0
        .checked_add(v.lake_precipitation.0)
        .ok_or(HydrologyFormatError::Overflow)?;
    let sinks = [
        v.land_loss,
        v.lake_evaporation,
        v.marginal_evaporation,
        v.sea_outflow,
        v.domain_outflow,
    ]
    .iter()
    .try_fold(0u128, |sum, v| {
        sum.checked_add(v.0).ok_or(HydrologyFormatError::Overflow)
    })?;
    require(source == sinks, "annual water balance does not close")
}
structure!(AnnualWaterBalance,[land_precipitation:Litres,land_loss:Litres,lake_precipitation:Litres,lake_evaporation:Litres,marginal_evaporation:Litres,sea_outflow:Litres,domain_outflow:Litres],annual_balance);
structure!(HydrologyDomain,[width_cells:u32,height_cells:u32,exported_areas_wide:u32,exported_areas_high:u32],|v:&HydrologyDomain|require(v.width_cells>0&&v.height_cells>0&&v.exported_areas_wide>0&&v.exported_areas_high>0&&u64::from(v.exported_areas_wide)*512<=u64::from(v.width_cells)&&u64::from(v.exported_areas_high)*512<=u64::from(v.height_cells),"invalid modeled domain"));
structure!(GlobalLake,[basin:BasinId,surface:HeightMm,deepest_bed:HeightMm,submerged_cells:u32,outlet:Option<SpillConnection>,annual_outflow:Litres,mean_outflow:DischargeMilli],|v:&GlobalLake|{
 require(v.submerged_cells>0&&v.deepest_bed<v.surface,"empty lake or nonpositive depth")?;
 require(v.outlet.is_none_or(|spill|spill.sill>=v.surface),"lake surface exceeds physical spill")?;
 require(v.annual_outflow.0==0||v.outlet.is_some_and(|spill|spill.sill==v.surface),"supported outflow lacks reached spill")?;
 annual_mean(v.annual_outflow,v.mean_outflow)
});
structure!(AnnualCatchment,[catchment:CatchmentId,terminal:GlobalCell,contributing_cells:u32,basin:Option<BasinId>,representative_lake:Option<BasinId>,potential_spill:Option<SpillConnection>,receiving:ReceivingAccount],|v:&AnnualCatchment|{
 require(v.contributing_cells>0,"empty catchment")?;
 require(v.basin.is_some()==v.potential_spill.is_some()&&(v.representative_lake.is_none()||v.basin.is_some()),"inconsistent catchment basin links")?;
 require(v.potential_spill.is_none_or(|spill|spill.receiving==v.receiving),"catchment spill destination disagrees")?;
 require(v.basin.is_some()||matches!(v.receiving,ReceivingAccount::Sea|ReceivingAccount::DomainExport),"open catchment has closed receiving account")
});
structure!(GlobalReach,[id:ReachId,from:GlobalCell,to:GlobalCell,receiving:ReceivingAccount,catchment:CatchmentId,drainage_cells:u32,annual_volume:Litres,mean_discharge:DischargeMilli],|v:&GlobalReach|{
    require((v.from == v.to) == v.id.is_point() && v.drainage_cells > 0, "invalid reach geometry or drainage area")?;
    require(v.id.start() == v.from, "reach identity disagrees with start")?;
    require(!matches!(v.receiving,ReceivingAccount::Junction(id) if id.cell()!=v.to), "reach junction disagrees with endpoint")?;
    if v.id.is_point() {
        require(v.annual_volume.0 > 0, "point reach lacks positive annual flow")?;
        require(match v.receiving {
            ReceivingAccount::Lake(_) => v.mean_discharge.raw() >= 40,
            ReceivingAccount::DomainExport => true,
            _ => false,
        }, "point reach lacks supported basin or domain terminus")?;
    }
    annual_mean(v.annual_volume, v.mean_discharge)
});
structure!(SharedCrossing,[id:CrossingId,from:GlobalCell,to:GlobalCell,reach:ReachId,catchment:CatchmentId,drainage_cells:u32,annual_volume:Litres,mean_discharge:DischargeMilli,receiving:ReceivingAccount],|v:&SharedCrossing|{require(v.from.min(v.to)==v.id.low&&v.from.max(v.to)==v.id.high&&neighbor(v.from,v.to)&&v.drainage_cells>0&&(v.from.x/512!=v.to.x/512||v.from.y/512!=v.to.y/512),"crossing witness is inconsistent or not cross-area")?;annual_mean(v.annual_volume,v.mean_discharge)});
structure!(ChannelEdge,[from:GlobalCell,to:GlobalCell,from_width_dm:u32,to_width_dm:u32,discharge:DischargeMilli],|v:&ChannelEdge|require(neighbor(v.from,v.to)&&v.from_width_dm>0&&v.to_width_dm>0&&v.discharge.raw()>=40,"invalid saved channel edge"));
structure!(HydrologyMetadata,[model_revision:u32,domain:HydrologyDomain,basin_count:u64,lake_count:u64,reach_count:u64,crossing_count:u64,catchment_count:u64,budget:AnnualWaterBalance],|v:&HydrologyMetadata|require(v.model_revision==MODEL_REVISION,"unsupported hydrology model revision"));

/// Range measured in records, not byte offsets. Global children stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TableSpan {
    /// First linked record index; zero for an empty span.
    pub offset: u64,
    /// Number of records, checked before reading or allocation.
    pub count: u64,
}
impl TableSpan {
    /// Check canonical range normalization and containment in a linked table.
    pub fn validate_total(self, total: u64) -> Result<()> {
        self.check()?;
        require(
            self.offset
                .checked_add(self.count)
                .ok_or(HydrologyFormatError::Overflow)?
                <= total,
            "table span leaves linked table",
        )
    }
}
structure!(TableSpan,[offset:u64,count:u64],|v:&TableSpan|{v.offset.checked_add(v.count).ok_or(HydrologyFormatError::Overflow)?;require(v.count!=0||v.offset==0,"empty span offset is not canonical zero")});
#[derive(Debug, Clone, PartialEq, Eq)]
/// Fixed basin row; its children remain in a separately streamed ID table.
pub struct BasinNodeRow {
    /// Canonical physical basin identity.
    pub id: BasinId,
    /// Canonical parent identity, absent only at the physical root.
    pub parent: Option<BasinId>,
    /// Absolute physical cell anchoring this basin.
    pub anchor: GlobalCell,
    /// Lowest physical bed elevation in millimetres.
    pub floor: HeightMm,
    /// Offset and count into the separate child-ID table.
    pub children: TableSpan,
    /// Physical neighboring spill witness, when this node has an outlet.
    pub spill: Option<SpillConnection>,
}
structure!(BasinNodeRow,[id:BasinId,parent:Option<BasinId>,anchor:GlobalCell,floor:HeightMm,children:TableSpan,spill:Option<SpillConnection>],|v:&BasinNodeRow|{require(v.parent!=Some(v.id),"basin is its own parent")?;require(v.spill.as_ref().is_none_or(|s|s.sill.raw()>=v.floor.raw()),"basin spill below floor")});
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
    fn parse(v: u32) -> Result<Self> {
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

#[derive(Debug, Clone, Copy)]
/// Caller-selected hard limits for standalone area context decoding and encoding.
pub struct ContextLimits {
    /// Maximum encoded bytes accepted for one context.
    pub max_bytes: u64,
    /// Maximum total count across the four logical context tables.
    pub max_records: u32,
}
impl Default for ContextLimits {
    fn default() -> Self {
        Self {
            max_bytes: 64 * 1024 * 1024,
            max_records: 1_048_576,
        }
    }
}
fn ordered<T: FixedRecord>(values: &[T]) -> Result<()> {
    for pair in values.windows(2) {
        require(
            pair[0].key() < pair[1].key(),
            "context records are not unique ascending",
        )?;
    }
    Ok(())
}
fn validate_context(ctx: &AreaHydrologyContext) -> Result<()> {
    require(
        ctx.model_revision == MODEL_REVISION,
        "unsupported area hydrology model",
    )?;
    ordered(&ctx.lakes)?;
    ordered(&ctx.catchments)?;
    ordered(&ctx.crossings)?;
    ordered(&ctx.reaches)?;
    let owners: BTreeSet<_> = ctx.catchments.iter().map(|v| v.catchment).collect();
    let reaches: BTreeMap<_, _> = ctx.reaches.iter().map(|v| (v.id, v)).collect();
    for reach in &ctx.reaches {
        require(
            owners.contains(&reach.catchment),
            "reach lacks copied catchment",
        )?;
    }
    for crossing in &ctx.crossings {
        let reach = reaches
            .get(&crossing.reach)
            .ok_or(HydrologyFormatError::Invalid("crossing lacks copied reach"))?;
        require(
            owners.contains(&crossing.catchment)
                && reach.catchment == crossing.catchment
                && reach.drainage_cells == crossing.drainage_cells
                && reach.annual_volume == crossing.annual_volume
                && reach.mean_discharge == crossing.mean_discharge
                && reach.receiving == crossing.receiving,
            "crossing disagrees with copied reach",
        )?;
    }
    // Referenced basin/downstream identities may lie outside this bounded area copy.
    // The global indexed authority validates their existence, without recursive loading.
    Ok(())
}
const CONTEXT_HEADER_BYTES: usize = 28;
fn context_limits(counts: [u32; 4], limits: ContextLimits) -> Result<u64> {
    let widths = [
        GlobalLake::WIDTH,
        AnnualCatchment::WIDTH,
        SharedCrossing::WIDTH,
        GlobalReach::WIDTH,
    ];
    let total: u64 = counts.iter().map(|&v| u64::from(v)).sum();
    let bytes = counts.into_iter().zip(widths).try_fold(
        CONTEXT_HEADER_BYTES as u64,
        |sum, (count, width)| {
            sum.checked_add(
                u64::from(count)
                    .checked_mul(width as u64)
                    .ok_or(HydrologyFormatError::Overflow)?,
            )
            .ok_or(HydrologyFormatError::Overflow)
        },
    )?;
    if total > u64::from(limits.max_records) || bytes > limits.max_bytes {
        return Err(HydrologyFormatError::Limit(
            "area context exceeds declared limits",
        ));
    }
    Ok(bytes)
}
fn append_records<T: FixedRecord>(out: &mut Vec<u8>, records: &[T]) -> Result<()> {
    for record in records {
        out.extend(encode_record(record)?);
    }
    Ok(())
}
fn read_records<T: FixedRecord>(r: &mut Decoder<'_>, count: u32) -> Result<Vec<T>> {
    let mut records = Vec::with_capacity(count as usize);
    for _ in 0..count {
        records.push(decode_record(r.take(T::WIDTH)?)?);
    }
    Ok(records)
}
/// Encode a standalone area copy using the default bounded context limits.
pub fn encode_area_context(ctx: &AreaHydrologyContext) -> Result<Vec<u8>> {
    encode_area_context_with_limits(ctx, ContextLimits::default())
}
/// Encode a standalone area copy with explicit hard resource limits.
pub fn encode_area_context_with_limits(
    ctx: &AreaHydrologyContext,
    limits: ContextLimits,
) -> Result<Vec<u8>> {
    let count = |n| u32::try_from(n).map_err(|_| HydrologyFormatError::Overflow);
    let counts = [
        count(ctx.lakes.len())?,
        count(ctx.catchments.len())?,
        count(ctx.crossings.len())?,
        count(ctx.reaches.len())?,
    ];
    let bytes = context_limits(counts, limits)?;
    validate_context(ctx)?;
    let mut out = Encoder {
        bytes: Vec::with_capacity(
            usize::try_from(bytes).map_err(|_| HydrologyFormatError::Overflow)?,
        ),
    };
    out.bytes.extend(CONTEXT_MAGIC);
    ctx.model_revision.put(&mut out)?;
    for n in counts {
        n.put(&mut out)?;
    }
    append_records(&mut out.bytes, &ctx.lakes)?;
    append_records(&mut out.bytes, &ctx.catchments)?;
    append_records(&mut out.bytes, &ctx.crossings)?;
    append_records(&mut out.bytes, &ctx.reaches)?;
    require(
        out.bytes.len() as u64 == bytes,
        "context output size mismatch",
    )?;
    Ok(out.bytes)
}
/// Decode bounded area copies using the identical fixed global-record codecs.
pub fn decode_area_context(bytes: &[u8], limits: ContextLimits) -> Result<AreaHydrologyContext> {
    if bytes.len() as u64 > limits.max_bytes {
        return Err(HydrologyFormatError::Limit("area context byte limit"));
    }
    let mut r = Decoder::new(bytes);
    require(r.take(8)? == CONTEXT_MAGIC, "bad area hydrology magic")?;
    let model_revision = u32::get(&mut r)?;
    require(
        model_revision == MODEL_REVISION,
        "unsupported area hydrology model",
    )?;
    let mut counts = [0; 4];
    for count in &mut counts {
        *count = u32::get(&mut r)?;
    }
    require(
        context_limits(counts, limits)? == bytes.len() as u64,
        "context length disagrees with counts",
    )?;
    let ctx = AreaHydrologyContext {
        model_revision,
        lakes: read_records(&mut r, counts[0])?,
        catchments: read_records(&mut r, counts[1])?,
        crossings: read_records(&mut r, counts[2])?,
        reaches: read_records(&mut r, counts[3])?,
    };
    r.finish()?;
    validate_context(&ctx)?;
    Ok(ctx)
}

/// Validate copied geometry against the modeled rectangle after area decoding.
///
/// The caller may derive this rectangle from the validated manifest; no global
/// table or neighboring area reads are needed. Referenced record existence stays
/// the global authority's responsibility.
///
/// # Errors
/// Rejects coordinates outside the domain and exports away from its actual rim.
pub fn validate_area_context_domain(
    ctx: &AreaHydrologyContext,
    domain: HydrologyDomain,
) -> Result<()> {
    domain.check()?;
    let inside = |c: GlobalCell| {
        require(
            c.x < domain.width_cells && c.y < domain.height_cells,
            "copied hydrology coordinate leaves modeled domain",
        )
    };
    let rim = |c: GlobalCell| {
        require(
            c.x == 0 || c.y == 0 || c.x == domain.width_cells - 1 || c.y == domain.height_cells - 1,
            "domain-export endpoint is not on modeled boundary",
        )
    };
    let account = |a: ReceivingAccount| match a {
        ReceivingAccount::Reach(id) => inside(id.start()),
        ReceivingAccount::Junction(id) => inside(id.cell()),
        ReceivingAccount::Lake(_) | ReceivingAccount::Sea | ReceivingAccount::DomainExport => {
            Ok(())
        }
    };
    for lake in &ctx.lakes {
        if let Some(spill) = lake.outlet {
            validate_spill_domain(&spill, domain)?;
            account(spill.receiving)?;
        }
    }
    for owner in &ctx.catchments {
        inside(owner.terminal)?;
        account(owner.receiving)?;
        if let Some(spill) = owner.potential_spill {
            validate_spill_domain(&spill, domain)?;
            account(spill.receiving)?;
        } else if owner.receiving == ReceivingAccount::DomainExport {
            rim(owner.terminal)?;
        }
    }
    for reach in &ctx.reaches {
        inside(reach.from)?;
        inside(reach.to)?;
        account(reach.receiving)?;
        if reach.receiving == ReceivingAccount::DomainExport {
            rim(reach.to)?;
        }
    }
    for crossing in &ctx.crossings {
        inside(crossing.from)?;
        inside(crossing.to)?;
        inside(crossing.id.low)?;
        inside(crossing.id.high)?;
        account(crossing.receiving)?;
    }
    Ok(())
}

/// Validate the domain-dependent part after decoding against root metadata.
pub fn validate_spill_domain(spill: &SpillConnection, domain: HydrologyDomain) -> Result<()> {
    spill.check()?;
    domain.check()?;
    let inside = |c: GlobalCell| c.x < domain.width_cells && c.y < domain.height_cells;
    require(inside(spill.from), "spill source leaves modeled domain")?;
    match spill.to {
        Some(to) => require(inside(to), "spill receiver leaves modeled domain"),
        None => require(
            spill.from.x == 0
                || spill.from.y == 0
                || spill.from.x + 1 == domain.width_cells
                || spill.from.y + 1 == domain.height_cells,
            "domain-export spill is not on modeled boundary",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    fn spill() -> SpillConnection {
        SpillConnection {
            from: GlobalCell { x: 511, y: 10 },
            to: Some(GlobalCell { x: 512, y: 11 }),
            sill: HeightMm::new(4000),
            receiving: ReceivingAccount::Sea,
        }
    }
    fn state() -> AnnualCatchment {
        AnnualCatchment {
            catchment: CatchmentId(7),
            terminal: GlobalCell { x: 500, y: 10 },
            contributing_cells: 100,
            basin: Some(BasinId(55)),
            representative_lake: Some(BasinId(999)),
            potential_spill: Some(spill()),
            receiving: ReceivingAccount::Sea,
        }
    }
    fn reach() -> GlobalReach {
        GlobalReach {
            id: ReachId(4004),
            from: GlobalCell { x: 500, y: 0 },
            to: GlobalCell { x: 550, y: 50 },
            receiving: ReceivingAccount::Sea,
            catchment: CatchmentId(7),
            drainage_cells: 100,
            annual_volume: Litres(31_536_000_000),
            mean_discharge: DischargeMilli::new(1000),
        }
    }
    fn lake() -> GlobalLake {
        GlobalLake {
            basin: BasinId(999),
            surface: HeightMm::new(4000),
            deepest_bed: HeightMm::new(3000),
            submerged_cells: 1,
            outlet: Some(spill()),
            annual_outflow: Litres(1),
            mean_outflow: DischargeMilli::new(0),
        }
    }
    fn context() -> AreaHydrologyContext {
        let reach = reach();
        AreaHydrologyContext {
            model_revision: 2,
            lakes: vec![lake()],
            catchments: vec![state()],
            crossings: vec![SharedCrossing {
                id: CrossingId {
                    low: spill().from,
                    high: spill().to.unwrap(),
                },
                from: spill().from,
                to: spill().to.unwrap(),
                reach: reach.id,
                catchment: reach.catchment,
                drainage_cells: reach.drainage_cells,
                annual_volume: reach.annual_volume,
                mean_discharge: reach.mean_discharge,
                receiving: reach.receiving,
            }],
            reaches: vec![reach],
        }
    }
    fn metadata() -> HydrologyMetadata {
        HydrologyMetadata {
            model_revision: 2,
            domain: HydrologyDomain {
                width_cells: 1024,
                height_cells: 1024,
                exported_areas_wide: 2,
                exported_areas_high: 2,
            },
            basin_count: 10,
            lake_count: 2,
            reach_count: 8,
            crossing_count: 3,
            catchment_count: 10,
            budget: AnnualWaterBalance {
                land_precipitation: Litres(100),
                land_loss: Litres(40),
                lake_precipitation: Litres(20),
                lake_evaporation: Litres(30),
                marginal_evaporation: Litres(10),
                sea_outflow: Litres(25),
                domain_outflow: Litres(15),
            },
        }
    }
    fn roundtrip<T: FixedRecord + std::fmt::Debug + PartialEq>(v: &T) {
        let b = encode_record(v).unwrap();
        assert_eq!(b.len(), T::WIDTH);
        assert_eq!(&decode_record::<T>(&b).unwrap(), v);
        for n in 0..b.len() {
            assert!(decode_record::<T>(&b[..n]).is_err());
        }
        let mut trailing = b.clone();
        trailing.push(0);
        assert!(decode_record::<T>(&trailing).is_err());
    }
    #[test]
    fn exact_schema_widths_and_channel_little_endian() {
        assert_eq!(ChannelEdge::WIDTH, 32);
        assert_eq!(SpillConnection::SIZE, 30);
        assert_eq!(GlobalLake::WIDTH, 75);
        assert_eq!(GlobalReach::WIDTH, 69);
        assert_eq!(SharedCrossing::WIDTH, 85);
        assert_eq!(AnnualCatchment::WIDTH, 78);
        assert_eq!(HydrologyMetadata::WIDTH, 172);
        assert_eq!(BasinNodeRow::WIDTH, 76);
        let v = ChannelEdge {
            from: GlobalCell {
                x: 0x01020304,
                y: 9,
            },
            to: GlobalCell {
                x: 0x01020305,
                y: 10,
            },
            from_width_dm: 70000,
            to_width_dm: 230650,
            discharge: DischargeMilli::new(0x0102030405060708),
        };
        let b = encode_record(&v).unwrap();
        assert_eq!(&b[..4], &[4, 3, 2, 1]);
        assert_eq!(&b[24..32], &[8, 7, 6, 5, 4, 3, 2, 1]);
        roundtrip(&v);
    }
    #[test]
    fn every_annual_authority_roundtrips() {
        let c = context();
        roundtrip(&c.lakes[0]);
        roundtrip(&c.catchments[0]);
        roundtrip(&c.crossings[0]);
        roundtrip(&c.reaches[0]);
        roundtrip(&metadata());
        roundtrip(&BasinNodeRow {
            id: BasinId(999),
            parent: None,
            anchor: GlobalCell { x: 2, y: 3 },
            floor: HeightMm::new(-1),
            children: TableSpan {
                offset: 0,
                count: 999999999,
            },
            spill: None,
        });
    }
    #[test]
    fn context_roundtrip_keeps_merged_parent_and_positive_sub_litre_per_second_outflow() {
        let c = context();
        let b = encode_area_context(&c).unwrap();
        assert_eq!(
            decode_area_context(&b, ContextLimits::default()).unwrap(),
            c
        );
        assert_eq!(
            encode_area_context(&decode_area_context(&b, ContextLimits::default()).unwrap())
                .unwrap(),
            b
        );
    }
    #[test]
    fn every_context_truncation_and_trailing_byte_fails() {
        let b = encode_area_context(&context()).unwrap();
        for n in 0..b.len() {
            assert!(
                decode_area_context(&b[..n], ContextLimits::default()).is_err(),
                "prefix {n}"
            );
        }
        let mut b = b;
        b.push(0);
        assert!(decode_area_context(&b, ContextLimits::default()).is_err());
    }
    #[test]
    fn empty_context_is_revision_two_and_has_exact_header() {
        let c = AreaHydrologyContext::default();
        assert_eq!(c.model_revision, 2);
        let b = encode_area_context(&c).unwrap();
        assert_eq!(b.len(), 28);
        assert_eq!(&b[..8], b"ARDACTX4");
        assert_eq!(
            decode_area_context(&b, ContextLimits::default()).unwrap(),
            c
        );
    }
    #[test]
    fn annual_budget_surface_and_revision_invariants_are_enforced() {
        let mut v = lake();
        v.deepest_bed = v.surface;
        assert!(encode_record(&v).is_err());
        v = lake();
        v.outlet = None;
        assert!(encode_record(&v).is_err());
        v = lake();
        v.mean_outflow = DischargeMilli::new(1);
        assert!(encode_record(&v).is_err());
        let mut m = metadata();
        m.budget.domain_outflow.0 += 1;
        assert!(encode_record(&m).is_err());
        m = metadata();
        m.budget.land_precipitation = Litres(u128::MAX);
        assert!(matches!(
            encode_record(&m),
            Err(HydrologyFormatError::Overflow)
        ));
        m = metadata();
        m.model_revision = 1;
        assert!(encode_record(&m).is_err());
        let mut bytes = encode_area_context(&context()).unwrap();
        bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
        assert!(decode_area_context(&bytes, ContextLimits::default()).is_err());
        let mut c = context();
        c.lakes.clear();
        c.catchments[0].representative_lake = None;
        assert!(
            encode_area_context(&c).is_ok(),
            "dry physical basin has no fake lake"
        );
        for tag in 7..=10 {
            assert!(TableKind::parse(tag).is_err());
        }
    }
    #[test]
    fn reserved_tags_and_absent_payloads_are_rejected() {
        let mut b = encode_record(&lake()).unwrap();
        b[20] = 2;
        assert!(decode_record::<GlobalLake>(&b).is_err());
        let node = BasinNodeRow {
            id: BasinId(4),
            parent: None,
            anchor: GlobalCell { x: 0, y: 0 },
            floor: HeightMm::new(0),
            children: TableSpan::default(),
            spill: None,
        };
        let mut b = encode_record(&node).unwrap();
        b[8] = 7;
        assert!(decode_record::<BasinNodeRow>(&b).is_err());
        b = encode_record(&node).unwrap();
        b[9] = 1;
        assert!(decode_record::<BasinNodeRow>(&b).is_err());
        let mut b = encode_record(&reach()).unwrap();
        b[24] = 99;
        assert!(decode_record::<GlobalReach>(&b).is_err());
        b = encode_record(&reach()).unwrap();
        b[25] = 1;
        assert!(decode_record::<GlobalReach>(&b).is_err());
    }
    #[test]
    fn streaming_tables_reject_counts_sizes_tags_order_and_partial_writes() {
        let mut writer = TableWriter::<_, GlobalReach>::create(Vec::new(), 2).unwrap();
        writer.write_record(&reach()).unwrap();
        assert!(writer.write_record(&reach()).is_err());
        let mut next = reach();
        next.id = ReachId(4007);
        writer.write_record(&next).unwrap();
        let b = writer.finish().unwrap();
        let mut reader = TableReader::<_, GlobalReach>::open(Cursor::new(&b), 10000, 2).unwrap();
        assert_eq!(reader.count(), 2);
        assert_eq!(reader.read_at(1).unwrap(), next);
        assert!(reader.read_at(2).is_err());
        assert_eq!(reader.next_record().unwrap().unwrap(), reach());
        assert_eq!(reader.next_record().unwrap().unwrap(), next);
        assert!(reader.next_record().unwrap().is_none());
        let mut bad = b.clone();
        bad[8..12].copy_from_slice(&999_u32.to_le_bytes());
        assert!(TableReader::<_, GlobalReach>::open(Cursor::new(bad), 10000, 2).is_err());
        let mut bad = b.clone();
        bad[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
        assert!(TableReader::<_, GlobalReach>::open(Cursor::new(bad), 10000, u64::MAX).is_err());
        for n in 0..b.len() {
            assert!(TableReader::<_, GlobalReach>::open(Cursor::new(&b[..n]), 10000, 2).is_err());
        }
        let mut trailing = b.clone();
        trailing.push(0);
        assert!(TableReader::<_, GlobalReach>::open(Cursor::new(trailing), 10000, 2).is_err());
        assert!(TableWriter::<_, GlobalReach>::create(Vec::new(), 1)
            .unwrap()
            .finish()
            .is_err());
        assert!(TableHeader::for_type::<GlobalReach>(u64::MAX).is_err());
    }
    #[test]
    fn internal_references_are_checked_without_world_closure() {
        let mut c = context();
        c.reaches.clear();
        assert!(encode_area_context(&c).is_err());
        c = context();
        c.catchments.clear();
        assert!(encode_area_context(&c).is_err());
        c = context();
        c.crossings[0].annual_volume.0 += 1;
        assert!(encode_area_context(&c).is_err());
        c = context();
        c.reaches[0].receiving = ReceivingAccount::Reach(ReachId(9000));
        c.crossings[0].receiving = c.reaches[0].receiving;
        assert!(
            encode_area_context(&c).is_ok(),
            "external closure must not load world"
        );
        assert!(TableSpan {
            offset: u64::MAX,
            count: 1
        }
        .validate_total(u64::MAX)
        .is_err());
        assert!(TableSpan {
            offset: 9,
            count: 2
        }
        .validate_total(10)
        .is_err());
    }
    #[test]
    fn child_ranges_stream_without_root_vectors() {
        let mut w = TableWriter::<_, BasinId>::create(Vec::new(), 4).unwrap();
        for id in [2, 3, 2, 9] {
            w.write_record(&BasinId(id)).unwrap();
        }
        let mut r =
            TableReader::<_, BasinId>::open(Cursor::new(w.finish().unwrap()), 1000, 4).unwrap();
        assert_eq!(
            r.span(TableSpan {
                offset: 0,
                count: 2
            })
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .unwrap(),
            vec![BasinId(2), BasinId(3)]
        );
        assert!(r
            .span(TableSpan {
                offset: 0,
                count: 4
            })
            .unwrap()
            .collect::<Result<Vec<_>>>()
            .is_err());
        assert!(validate_span_partition(
            [
                TableSpan {
                    offset: 0,
                    count: 2
                },
                TableSpan::default(),
                TableSpan {
                    offset: 2,
                    count: 2
                }
            ],
            4
        )
        .is_ok());
        assert!(validate_span_partition(
            [
                TableSpan {
                    offset: 0,
                    count: 2
                },
                TableSpan {
                    offset: 1,
                    count: 2
                }
            ],
            4
        )
        .is_err());
    }

    #[test]
    fn limits_fail_before_count_driven_allocation() {
        let b = encode_area_context(&context()).unwrap();
        let limits = ContextLimits {
            max_bytes: 100,
            max_records: 1,
        };
        assert!(decode_area_context(&b, limits).is_err());
        assert!(encode_area_context_with_limits(&context(), limits).is_err());
        let mut b = encode_area_context(&AreaHydrologyContext::default()).unwrap();
        b[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode_area_context(&b, ContextLimits::default()).is_err());
    }
    #[test]
    fn only_actual_modeled_boundary_can_export_without_neighbor() {
        let domain = HydrologyDomain {
            width_cells: 1024,
            height_cells: 1024,
            exported_areas_wide: 2,
            exported_areas_high: 2,
        };
        let mut s = spill();
        s.to = None;
        s.receiving = ReceivingAccount::DomainExport;
        assert!(validate_spill_domain(&s, domain).is_err());
        s.from.x = 0;
        assert!(validate_spill_domain(&s, domain).is_ok());
    }
    #[test]
    fn junction_accounts_keep_width_and_potential_references_need_no_outgoing_table() {
        let mut g = reach();
        let junction = JunctionId::at(g.to);
        g.receiving = ReceivingAccount::Junction(junction);
        let bytes = encode_record(&g).unwrap();
        assert_eq!(bytes.len(), 69);
        assert_eq!(bytes[24], 4);
        assert_eq!(&bytes[25..33], &junction.0.to_le_bytes());
        roundtrip(&g);
        let mut bad = bytes.clone();
        bad[24] = 5;
        assert!(decode_record::<GlobalReach>(&bad).is_err());
        let mut bad = g.clone();
        bad.receiving = ReceivingAccount::Junction(JunctionId(junction.0 + 1));
        assert!(encode_record(&bad).is_err());
        bad = g.clone();
        bad.id = ReachId(g.id.0 + 8);
        assert!(encode_record(&bad).is_err());
        let mut c = AreaHydrologyContext {
            reaches: vec![g],
            catchments: vec![state()],
            ..AreaHydrologyContext::default()
        };
        let potential = c.catchments[0].potential_spill.as_mut().unwrap();
        let to = potential.to.unwrap();
        potential.receiving = ReceivingAccount::Junction(JunctionId::at(to));
        c.catchments[0].receiving = potential.receiving;
        let bytes = encode_area_context(&c).unwrap();
        assert_eq!(
            decode_area_context(&bytes, ContextLimits::default()).unwrap(),
            c
        );
        c.catchments[0].potential_spill.as_mut().unwrap().receiving =
            ReceivingAccount::Junction(JunctionId(0));
        assert!(encode_area_context(&c).is_err());
    }
    #[test]
    fn point_reaches_preserve_width_and_validate_terminal_flow_semantics() {
        let mut p = reach();
        p.id = ReachId::point(p.from).unwrap();
        p.to = p.from;
        p.receiving = ReceivingAccount::Lake(BasinId(55));
        roundtrip(&p);
        assert_eq!(encode_record(&p).unwrap().len(), 69);
        let mut bad = p.clone();
        bad.to.x += 1;
        assert!(encode_record(&bad).is_err());
        bad = p.clone();
        bad.id = ReachId::from_step(
            p.from,
            GlobalCell {
                x: p.from.x + 1,
                y: p.from.y,
            },
        )
        .unwrap();
        assert!(encode_record(&bad).is_err());
        for receiving in [
            ReceivingAccount::Sea,
            ReceivingAccount::Reach(ReachId(4)),
            ReceivingAccount::Junction(JunctionId::at(p.from)),
        ] {
            bad = p.clone();
            bad.receiving = receiving;
            assert!(encode_record(&bad).is_err());
        }
        p.annual_volume = Litres(40 * 31_536_000);
        p.mean_discharge = DischargeMilli::new(40);
        roundtrip(&p);
        p.annual_volume = Litres(40 * 31_536_000 - 1);
        p.mean_discharge = DischargeMilli::new(39);
        assert!(encode_record(&p).is_err());
        p.receiving = ReceivingAccount::DomainExport;
        roundtrip(&p);
        p.annual_volume = Litres(1);
        p.mean_discharge = DischargeMilli::new(0);
        roundtrip(&p);
        p.annual_volume = Litres(0);
        assert!(encode_record(&p).is_err());
    }
}
