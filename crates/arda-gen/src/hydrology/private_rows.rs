//! Explicit checked private hierarchy records, including accidental-corruption checksums.
use super::hierarchy::{ElderDeath, ElderLink, Minimum, NodeRow, PendingSpill, UnionRow};
use super::routing::{CellIndex, Extent};
use arda_core::{
    formats::hydrology::{encode_record, BasinNodeRow, TableSpan},
    hydrology::{BasinId, JunctionId, ReachId, ReceivingAccount, SpillConnection},
    GlobalCell, HeightMm,
};
/// Encoded union slot, including 8-byte checksum; never Rust struct layout.
pub const UNION_BYTES: usize = 96;
/// Encoded temporary node slot, including 8-byte checksum.
pub const NODE_BYTES: usize = 80;
/// Encoded private elder-link row, including 8-byte checksum.
pub const ELDER_BYTES: usize = 56;
/// Canonical row/schema/integrity failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid private hierarchy row: {0}")]
pub struct RowError(
    /// Stable schema failure category.
    pub &'static str,
);
type Result<T> = std::result::Result<T, RowError>;
/// FNV-1a protects private rows against accidental byte corruption; it is not authentication.
pub fn checksum(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf29ce484222325, |v, &b| {
        (v ^ u64::from(b)).wrapping_mul(0x100000001b3)
    })
}
struct Write<const N: usize> {
    bytes: [u8; N],
    at: usize,
}
impl<const N: usize> Write<N> {
    fn new() -> Self {
        Self {
            bytes: [0; N],
            at: 0,
        }
    }
    fn b(&mut self, b: &[u8]) {
        self.bytes[self.at..self.at + b.len()].copy_from_slice(b);
        self.at += b.len();
    }
    fn u8(&mut self, v: u8) {
        self.b(&[v]);
    }
    fn u32(&mut self, v: u32) {
        self.b(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.b(&v.to_le_bytes());
    }
    fn u64(&mut self, v: u64) {
        self.b(&v.to_le_bytes());
    }
    fn opt64(&mut self, v: Option<u64>) {
        self.u8(u8::from(v.is_some()));
        self.u64(v.unwrap_or(0));
    }
    fn opt32(&mut self, v: Option<i32>) {
        self.u8(u8::from(v.is_some()));
        self.i32(v.unwrap_or(0));
    }
    fn cell(&mut self, v: GlobalCell) {
        self.u32(v.x);
        self.u32(v.y);
    }
    fn pending(&mut self, v: Option<PendingSpill>) {
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
    fn finish(mut self) -> [u8; N] {
        let sum = checksum(&self.bytes[..N - 8]);
        self.bytes[N - 8..].copy_from_slice(&sum.to_le_bytes());
        self.bytes
    }
}
struct Read<'a> {
    b: &'a [u8],
    at: usize,
}
impl<'a> Read<'a> {
    fn new(b: &'a [u8], n: usize) -> Result<Self> {
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
    fn b<const N: usize>(&mut self) -> Result<[u8; N]> {
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
    fn u8(&mut self) -> Result<u8> {
        Ok(self.b::<1>()?[0])
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.b()?))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.b()?))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.b()?))
    }
    fn flag(&mut self) -> Result<bool> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(RowError("tag")),
        }
    }
    fn opt64(&mut self) -> Result<Option<u64>> {
        let yes = self.flag()?;
        let v = self.u64()?;
        if !yes && v != 0 {
            return Err(RowError("absent payload"));
        }
        Ok(yes.then_some(v))
    }
    fn opt32(&mut self) -> Result<Option<i32>> {
        let yes = self.flag()?;
        let v = self.i32()?;
        if !yes && v != 0 {
            return Err(RowError("absent payload"));
        }
        Ok(yes.then_some(v))
    }
    fn cell(&mut self) -> Result<GlobalCell> {
        Ok(GlobalCell {
            x: self.u32()?,
            y: self.u32()?,
        })
    }
    fn pending(&mut self, e: Extent, n: u64) -> Result<Option<PendingSpill>> {
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
    fn finish(self) -> Result<()> {
        if self.b[self.at..].iter().any(|&v| v != 0) {
            Err(RowError("reserved padding"))
        } else {
            Ok(())
        }
    }
}
fn coord_ok(e: Extent, c: GlobalCell) -> bool {
    let last = CellIndex::new(e.cells() - 1, e);
    last.is_some_and(|at| {
        let (x, y) = e.coordinates(at);
        c.x <= x && c.y <= y
    })
}
fn validate_geometry(e: Extent, from: CellIndex, to: Option<CellIndex>) -> Result<()> {
    let (x, y) = e.coordinates(from);
    if let Some(to) = to {
        let (a, b) = e.coordinates(to);
        if from == to || x.abs_diff(a) > 1 || y.abs_diff(b) > 1 {
            return Err(RowError("spill adjacency"));
        }
    } else {
        let last = CellIndex::new(e.cells() - 1, e).ok_or(RowError("empty extent"))?;
        let (a, b) = e.coordinates(last);
        if !(x == 0 || y == 0 || x == a || y == b) {
            return Err(RowError("missing target away from rim"));
        }
    }
    Ok(())
}
fn optional_index(v: Option<u64>, n: u64) -> Result<()> {
    if v.is_some_and(|x| x >= n) {
        Err(RowError("index range"))
    } else {
        Ok(())
    }
}
/// Encode and validate a union slot at its indexed location.
pub fn encode_union(
    v: UnionRow,
    e: Extent,
    at: u64,
    unions: u64,
    nodes: u64,
) -> Result<[u8; UNION_BYTES]> {
    let mut w = Write::new();
    w.u8(1);
    w.u8(u8::from(v.minimum.is_some()));
    if let Some(m) = v.minimum {
        w.u32(m.at.raw());
        w.i32(m.floor_mm);
    } else {
        w.b(&[0; 8]);
    }
    w.u64(v.parent);
    w.u8(v.rank);
    w.opt32(v.joined_at);
    w.opt64(v.component);
    w.opt32(v.event);
    w.opt64(v.event_base);
    w.u64(v.elder);
    w.u8(u8::from(v.elder_death.is_some()));
    w.opt64(v.elder_death.and_then(|d| d.parent.map(|p| p.0)));
    w.pending(v.elder_death.map(|d| d.spill));
    let b = w.finish();
    decode_union(&b, e, at, unions, nodes)?;
    Ok(b)
}
/// Decode one initialized fixed union row; zero slots remain uninitialized.
pub fn decode_union(b: &[u8], e: Extent, at: u64, unions: u64, nodes: u64) -> Result<UnionRow> {
    let mut r = Read::new(b, UNION_BYTES)?;
    if r.u8()? != 1 {
        return Err(RowError("union initialized tag"));
    }
    let has = r.flag()?;
    let raw = r.u32()?;
    let floor = r.i32()?;
    let minimum = if has {
        Some(Minimum {
            at: CellIndex::new(raw, e).ok_or(RowError("minimum extent"))?,
            floor_mm: floor,
        })
    } else {
        if raw != 0 || floor != 0 {
            return Err(RowError("absent minimum payload"));
        }
        None
    };
    let parent = r.u64()?;
    let rank = r.u8()?;
    let joined_at = r.opt32()?;
    let component = r.opt64()?;
    let event = r.opt32()?;
    let event_base = r.opt64()?;
    let elder = r.u64()?;
    let death = r.flag()?;
    let dp = r.opt64()?.map(BasinId);
    let ds = r.pending(e, nodes)?;
    let elder_death = if death {
        Some(ElderDeath {
            parent: dp,
            spill: ds.ok_or(RowError("elder death lacks spill"))?,
        })
    } else {
        if dp.is_some() || ds.is_some() {
            return Err(RowError("absent death payload"));
        }
        None
    };
    r.finish()?;
    if unions == 0
        || at >= unions
        || parent >= unions
        || elder >= unions
        || rank > 63
        || has != (at + 1 < unions)
        || (parent == at) != joined_at.is_none()
    {
        return Err(RowError("union identity/root"));
    }
    if event.is_none() && event_base.is_some() {
        return Err(RowError("base lacks event"));
    }
    optional_index(component, nodes)?;
    optional_index(event_base, nodes)?;
    if let Some(d) = elder_death {
        if at + 1 == unions
            || d.parent.is_some_and(|p| p.0 >> 63 != 0)
            || d.parent == minimum.map(|m| BasinId(e.anchor_key(m.at)))
        {
            return Err(RowError("elder parent identity"));
        }
    }
    Ok(UnionRow {
        minimum,
        parent,
        rank,
        joined_at,
        component,
        event,
        event_base,
        elder,
        elder_death,
    })
}
/// Encode and validate a temporary node slot at its indexed location.
pub fn encode_node(
    v: NodeRow,
    e: Extent,
    at: u64,
    leaves: u64,
    nodes: u64,
) -> Result<[u8; NODE_BYTES]> {
    let mut w = Write::new();
    w.u8(1);
    w.cell(v.anchor);
    w.i32(v.floor_mm);
    w.i32(v.birth_mm);
    w.u8(u8::from(v.leaf));
    w.opt64(v.parent);
    w.pending(v.spill);
    w.opt64(v.id.map(|v| v.0));
    w.opt64(v.retained_parent);
    let b = w.finish();
    decode_node(&b, e, at, leaves, nodes)?;
    Ok(b)
}
/// Decode a fixed temporary node, checking topology ranges and normalization.
pub fn decode_node(b: &[u8], e: Extent, at: u64, leaves: u64, nodes: u64) -> Result<NodeRow> {
    let mut r = Read::new(b, NODE_BYTES)?;
    if r.u8()? != 1 {
        return Err(RowError("node initialized tag"));
    }
    let anchor = r.cell()?;
    let floor_mm = r.i32()?;
    let birth_mm = r.i32()?;
    let leaf = r.flag()?;
    let parent = r.opt64()?;
    let spill = r.pending(e, nodes)?;
    let id = r.opt64()?.map(BasinId);
    let retained_parent = r.opt64()?;
    r.finish()?;
    if at >= nodes
        || leaf != (at < leaves)
        || !coord_ok(e, anchor)
        || floor_mm > birth_mm
        || (leaf && floor_mm != birth_mm)
    {
        return Err(RowError("node identity/floor"));
    }
    for p in [parent, retained_parent] {
        optional_index(p, nodes)?;
        if p.is_some_and(|p| p <= at) {
            return Err(RowError("parent order"));
        }
    }
    if parent.is_some() && spill.is_none() {
        return Err(RowError("parent lacks spill"));
    }
    if spill.is_some_and(|s| s.sill_mm < birth_mm) {
        return Err(RowError("spill below birth"));
    }
    if let Some(id) = id {
        let key = (u64::from(anchor.y) << 32) | u64::from(anchor.x);
        if (leaf && id.0 != key) || (!leaf && id.0 >> 63 != 1) {
            return Err(RowError("final ID namespace"));
        }
    }
    Ok(NodeRow {
        anchor,
        floor_mm,
        birth_mm,
        leaf,
        parent,
        spill,
        id,
        retained_parent,
    })
}
fn validate_elder(v: ElderLink, e: Extent) -> Result<()> {
    let key = |id: BasinId| GlobalCell {
        x: u32::try_from(id.0 & u64::from(u32::MAX)).unwrap_or(0),
        y: u32::try_from(id.0 >> 32).unwrap_or(0),
    };
    if !coord_ok(e, key(v.leaf))
        || v.elder_parent.is_some_and(|p| !coord_ok(e, key(p)))
        || !coord_ok(e, v.spill.from)
        || v.spill.to.is_some_and(|c| !coord_ok(e, c))
    {
        return Err(RowError("elder coordinate extent"));
    }
    let row = BasinNodeRow {
        id: v.leaf,
        parent: v.elder_parent,
        anchor: v.spill.from,
        floor: v.spill.sill,
        children: TableSpan::default(),
        spill: Some(v.spill),
    };
    encode_record(&row).map_err(|_| RowError("elder spill/core contract"))?;
    if v.spill.receiving == ReceivingAccount::Lake(v.leaf) {
        return Err(RowError("self receiving elder"));
    }
    if v.spill.to.is_none() {
        let last = CellIndex::new(e.cells() - 1, e).ok_or(RowError("empty extent"))?;
        let (x, y) = e.coordinates(last);
        let c = v.spill.from;
        if !(c.x == 0 || c.y == 0 || c.x == x || c.y == y) {
            return Err(RowError("elder export away from rim"));
        }
    }
    Ok(())
}
/// Encode an exclusive elder row; the physical spill uses the core tag meanings.
pub fn encode_elder(v: ElderLink, e: Extent) -> Result<[u8; ELDER_BYTES]> {
    validate_elder(v, e)?;
    let mut w = Write::new();
    w.u64(v.leaf.0);
    w.opt64(v.elder_parent.map(|p| p.0));
    w.cell(v.spill.from);
    w.u8(u8::from(v.spill.to.is_some()));
    w.cell(v.spill.to.unwrap_or(GlobalCell { x: 0, y: 0 }));
    w.i32(v.spill.sill.raw());
    let (tag, id) = match v.spill.receiving {
        ReceivingAccount::Reach(id) => (0, id.0),
        ReceivingAccount::Lake(id) => (1, id.0),
        ReceivingAccount::Sea => (2, 0),
        ReceivingAccount::DomainExport => (3, 0),
        ReceivingAccount::Junction(id) => (4, id.0),
    };
    w.u8(tag);
    w.u64(id);
    Ok(w.finish())
}
/// Decode an exact private elder row and reject tags, trailing bytes and corruption.
pub fn decode_elder(b: &[u8], e: Extent) -> Result<ElderLink> {
    let mut r = Read::new(b, ELDER_BYTES)?;
    let leaf = BasinId(r.u64()?);
    let elder_parent = r.opt64()?.map(BasinId);
    let from = r.cell()?;
    let has = r.flag()?;
    let c = r.cell()?;
    if !has && c != (GlobalCell { x: 0, y: 0 }) {
        return Err(RowError("absent global endpoint payload"));
    }
    let to = has.then_some(c);
    let sill = HeightMm::new(r.i32()?);
    let tag = r.u8()?;
    let id = r.u64()?;
    let receiving = match (tag, id) {
        (0, id) => ReceivingAccount::Reach(ReachId(id)),
        (1, id) => ReceivingAccount::Lake(BasinId(id)),
        (2, 0) => ReceivingAccount::Sea,
        (3, 0) => ReceivingAccount::DomainExport,
        (4, id) => ReceivingAccount::Junction(JunctionId(id)),
        _ => return Err(RowError("receiving tag/payload")),
    };
    r.finish()?;
    let out = ElderLink {
        leaf,
        elder_parent,
        spill: SpillConnection {
            from,
            to,
            sill,
            receiving,
        },
    };
    validate_elder(out, e)?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hydrology::hierarchy_tests::{between, m, outside, run};
    fn fix_checksum(b: &mut [u8]) {
        let n = b.len();
        let s = checksum(&b[..n - 8]);
        b[n - 8..].copy_from_slice(&s.to_le_bytes());
    }
    #[test]
    fn real_builder_rows_roundtrip_and_every_changed_byte_is_detected() {
        let e = Extent::new(8, 3).unwrap();
        let ms = [m(e, 1, -20), m(e, 3, -10), m(e, 5, 0)];
        let es = [
            between(e, ms[0], ms[1], 5),
            between(e, ms[1], ms[2], 5),
            outside(e, ms[2], 9),
        ];
        let (_, s) = run(e, &ms, &es);
        for (i, row) in s.unions.iter().enumerate() {
            if let Some(row) = row {
                let b = encode_union(*row, e, i as u64, 4, 5).unwrap();
                let got = decode_union(&b, e, i as u64, 4, 5).unwrap();
                assert_eq!(encode_union(got, e, i as u64, 4, 5).unwrap(), b);
                for j in 0..b.len() {
                    let mut bad = b;
                    bad[j] ^= 1;
                    assert!(decode_union(&bad, e, i as u64, 4, 5).is_err());
                }
                for n in 0..b.len() {
                    assert!(decode_union(&b[..n], e, i as u64, 4, 5).is_err());
                }
            }
        }
        for (i, row) in s.nodes.iter().enumerate() {
            if let Some(row) = row {
                let b = encode_node(*row, e, i as u64, 3, 5).unwrap();
                let got = decode_node(&b, e, i as u64, 3, 5).unwrap();
                assert_eq!(encode_node(got, e, i as u64, 3, 5).unwrap(), b);
                for j in 0..b.len() {
                    let mut bad = b;
                    bad[j] ^= 1;
                    assert!(decode_node(&bad, e, i as u64, 3, 5).is_err());
                }
            }
        }
        for row in s.elders {
            let b = encode_elder(row, e).unwrap();
            assert_eq!(decode_elder(&b, e).unwrap(), row);
            for j in 0..b.len() {
                let mut bad = b;
                bad[j] ^= 1;
                assert!(decode_elder(&bad, e).is_err());
            }
        }
    }
    #[test]
    fn canonical_tags_padding_ranges_and_absent_payloads_are_checked_even_with_matching_checksum() {
        let e = Extent::new(6, 3).unwrap();
        let ms = [m(e, 1, 0), m(e, 3, 1)];
        let (_, s) = run(e, &ms, &[between(e, ms[0], ms[1], 5), outside(e, ms[1], 9)]);
        let union = encode_union(s.unions[0].unwrap(), e, 0, 3, 3).unwrap();
        for (i, v) in [(0, 2), (1, 2), (18, 64), (86, 1)] {
            let mut b = union;
            b[i] = v;
            fix_checksum(&mut b);
            assert!(decode_union(&b, e, 0, 3, 3).is_err(), "index {i}");
        }
        let node = encode_node(s.nodes[0].unwrap(), e, 0, 2, 3).unwrap();
        for (i, v) in [(0, 2), (17, 2), (71, 1)] {
            let mut b = node;
            b[i] = v;
            fix_checksum(&mut b);
            assert!(decode_node(&b, e, 0, 2, 3).is_err());
        }
        let mut invalid = s.nodes[0].unwrap();
        invalid.parent = Some(0);
        assert!(encode_node(invalid, e, 0, 2, 3).is_err());
        invalid.parent = Some(99);
        assert!(encode_node(invalid, e, 0, 2, 3).is_err());
        let mut elder = encode_elder(s.elders[0], e).unwrap();
        elder[38] = 9;
        fix_checksum(&mut elder);
        assert!(decode_elder(&elder, e).is_err());
    }
}
