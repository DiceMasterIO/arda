//! The fine-flow scratch file format: its checked header and the explicit
//! little-endian 80-byte record codec.

use super::*;

pub(super) fn header(extent: Extent) -> Result<[u8; HEADER]> {
    let last = CellIndex::new(extent.cells() - 1, extent).ok_or_else(|| invalid("empty domain"))?;
    let (x, y) = extent.coordinates(last);
    let mut b = [0; HEADER];
    b[..8].copy_from_slice(b"ARDAFL01");
    b[8..12].copy_from_slice(&(x + 1).to_le_bytes());
    b[12..16].copy_from_slice(&(y + 1).to_le_bytes());
    b[16..24].copy_from_slice(&u64::from(extent.cells()).to_le_bytes());
    b[24..28].copy_from_slice(&80_u32.to_le_bytes());
    b[28..32].copy_from_slice(&4096_u32.to_le_bytes());
    let sum = checksum(&b[..56]);
    b[56..64].copy_from_slice(&sum.to_le_bytes());
    Ok(b)
}

/// Explicit little-endian scratch row; unused payload has one canonical zero encoding.
pub fn encode_record(record: FlowRecord, extent: Extent) -> Result<[u8; ROW]> {
    if record.parent.is_some() && !record.visited || record.lake.is_none() && record.surface_mm != 0
    {
        return Err(invalid("record state"));
    }
    let m = record.metrics;
    if record
        .parent
        .is_some_and(|p| CellIndex::new(p.raw(), extent).is_none())
        || m.hand_at
            .is_some_and(|p| CellIndex::new(p.raw(), extent).is_none())
    {
        return Err(invalid("reference outside domain"));
    }
    // D8 confluences can have eight equal-order tributaries. Two is the
    // Strahler promotion threshold, not the maximum stored contribution count.
    if m.pending > 8 || m.max_in_ties > 8 || (m.hand_at.is_none() && m.hand_distance_mm != u64::MAX)
    {
        return Err(invalid("metrics state"));
    }
    let mut b = [0; ROW];
    b[..16].copy_from_slice(&record.net.to_le_bytes());
    if let Some(p) = record.parent {
        b[16..20].copy_from_slice(&p.raw().to_le_bytes());
    }
    b[20] = u8::from(record.visited)
        | (u8::from(record.parent.is_some()) << 1)
        | (u8::from(record.exterior) << 2)
        | (u8::from(record.lake.is_some()) << 3);
    b[21] = record.selected;
    b[22..26].copy_from_slice(&record.surface_mm.to_le_bytes());
    if let Some(id) = record.lake {
        b[26..34].copy_from_slice(&id.0.to_le_bytes());
    }
    b[34] = u8::from(m.hand_at.is_some());
    b[35] = m.pending;
    b[36] = m.max_in_order;
    b[37] = m.max_in_ties;
    b[38] = m.order;
    b[40..44].copy_from_slice(&m.catchment_cells.to_le_bytes());
    b[44..48].copy_from_slice(&m.drainage_cells.to_le_bytes());
    b[48..56].copy_from_slice(&m.hand_distance_mm.to_le_bytes());
    if let Some(at) = m.hand_at {
        b[56..60].copy_from_slice(&at.raw().to_le_bytes());
    }
    b[60..68].copy_from_slice(&m.scalar_annual.to_le_bytes());
    let sum = checksum(&b[..72]);
    b[72..80].copy_from_slice(&sum.to_le_bytes());
    Ok(b)
}

/// Validate checksum, flags, padding and references before any record enters the cache.
pub fn decode_record(b: &[u8], extent: Extent) -> Result<FlowRecord> {
    if b.len() != ROW
        || b[20] & !15 != 0
        || b[34] > 1
        || b[39] != 0
        || b[68..72].iter().any(|&n| n != 0)
        || u64::from_le_bytes(get(b, 72)?) != checksum(&b[..72])
    {
        return Err(invalid("record integrity"));
    }
    let parent_raw = u32::from_le_bytes(get(b, 16)?);
    let parent = if b[20] & 2 != 0 {
        Some(CellIndex::new(parent_raw, extent).ok_or_else(|| invalid("parent ordinal"))?)
    } else {
        if parent_raw != 0 {
            return Err(invalid("absent parent payload"));
        }
        None
    };
    let id = u64::from_le_bytes(get(b, 26)?);
    let lake = if b[20] & 8 != 0 {
        Some(BasinId(id))
    } else {
        if id != 0 {
            return Err(invalid("absent lake payload"));
        }
        None
    };
    let hand_raw = u32::from_le_bytes(get(b, 56)?);
    let hand_at = if b[34] == 1 {
        Some(CellIndex::new(hand_raw, extent).ok_or_else(|| invalid("HAND ordinal"))?)
    } else {
        if hand_raw != 0 {
            return Err(invalid("absent HAND payload"));
        }
        None
    };
    let record = FlowRecord {
        net: i128::from_le_bytes(get(b, 0)?),
        parent,
        visited: b[20] & 1 != 0,
        selected: b[21],
        exterior: b[20] & 4 != 0,
        lake,
        surface_mm: i32::from_le_bytes(get(b, 22)?),
        metrics: FlowMetrics {
            catchment_cells: u32::from_le_bytes(get(b, 40)?),
            drainage_cells: u32::from_le_bytes(get(b, 44)?),
            pending: b[35],
            max_in_order: b[36],
            max_in_ties: b[37],
            order: b[38],
            hand_distance_mm: u64::from_le_bytes(get(b, 48)?),
            hand_at,
            scalar_annual: u64::from_le_bytes(get(b, 60)?),
        },
    };
    encode_record(record, extent)?;
    Ok(record)
}
