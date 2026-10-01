//! Explicit little-endian format-4 hydrology records; no Rust padding on disk.
//! Global tables stream fixed records. Only bounded area contexts own vectors.
#![deny(missing_docs)]
use crate::hydrology::*;
use crate::{AreaCoord, DischargeMilli, GlobalCell, HeightMm};

mod context;
mod table;
pub use context::{
    decode_area_context, encode_area_context, encode_area_context_with_limits,
    validate_area_context_domain, validate_spill_domain, ContextLimits,
};
pub use table::{
    decode_record, encode_record, validate_span_partition, FixedRecord, RecordKey, SpanReader,
    TableHeader, TableKind, TableReader, TableWriter,
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

#[cfg(test)]
mod tests;
