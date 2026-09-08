//! Bounded area-local IDs over an exclusive set of already extracted channel cells.
use super::extraction::{CellEnd, OwnedChannelCell};
use arda_core::{AreaCoord, CellCoord, RiverSegment, Terminus};

const CELLS: usize = 512 * 512;
/// Invalid local authority, source-reader failure or insufficient reservation.
#[derive(Debug, thiserror::Error)]
pub enum LocalRiverError<E> {
    /// Caller-owned immutable row stream failed.
    #[error("local river source failed")]
    Source(#[source] E),
    /// Coordinate, order, unique membership, count or local graph is inconsistent.
    #[error("invalid local river extraction: {0}")]
    Invalid(&'static str),
    /// Checked allocation or byte admission failed.
    #[error("local river extraction resource limit")]
    Limit,
}
/// Conservative payload for local IDs, records, single-cell course allocations and end scratch.
#[must_use]
pub fn required_ram(channel_cells: u32) -> Option<u64> {
    u64::from(channel_cells)
        .checked_mul(512)?
        .checked_add(4096 + 4 * CELLS as u64)
}
/// Number and link exactly the already solved dry initiated cells of one exported area.
/// Each course contains one exclusively owned cell; actual edge endpoints and all
/// branches remain in the copied global authority and independent channel geometry.
/// # Errors
/// Rejects unsorted/repeated/foreign cells, wrong counts, bad channel values, missing
/// unique local downstream owners, input failures and insufficient reservations.
pub fn compose<E>(
    area: AreaCoord,
    rows: impl IntoIterator<Item = Result<OwnedChannelCell, E>>,
    count: u32,
    ram_bytes: u64,
) -> Result<Vec<RiverSegment>, LocalRiverError<E>> {
    let ax = u32::try_from(area.x).map_err(|_| LocalRiverError::Invalid("negative area"))?;
    let ay = u32::try_from(area.y).map_err(|_| LocalRiverError::Invalid("negative area"))?;
    let maximum = usize::try_from(count).map_err(|_| LocalRiverError::Limit)?;
    if maximum > CELLS || required_ram(count).is_none_or(|n| n > ram_bytes) {
        return Err(LocalRiverError::Limit);
    }
    let mut owners = Vec::new();
    owners
        .try_reserve_exact(CELLS)
        .map_err(|_| LocalRiverError::Limit)?;
    owners.resize(CELLS, 0_u32);
    let mut rivers = Vec::new();
    rivers
        .try_reserve_exact(maximum)
        .map_err(|_| LocalRiverError::Limit)?;
    let mut ends = Vec::new();
    ends.try_reserve_exact(maximum)
        .map_err(|_| LocalRiverError::Limit)?;
    let mut last = None;
    for row in rows {
        let row = row.map_err(LocalRiverError::Source)?;
        if rivers.len() == maximum
            || row.at.x / 512 != ax
            || row.at.y / 512 != ay
            || row.order == 0
            || row.discharge.raw() < 40
            || row.width_dm == 0
        {
            return Err(LocalRiverError::Invalid("owned cell extent/count/metrics"));
        }
        if row.reach.start() != row.at || matches!(row.end,CellEnd::Junction{at,..} if at!=row.to) {
            return Err(LocalRiverError::Invalid("source or target identity"));
        }
        let x = u16::try_from(row.at.x % 512).map_err(|_| LocalRiverError::Limit)?;
        let y = u16::try_from(row.at.y % 512).map_err(|_| LocalRiverError::Limit)?;
        let cell = CellCoord::new(x, y).ok_or(LocalRiverError::Invalid("local coordinate"))?;
        let at = cell.index();
        if owners[at] != 0 || last.is_some_and(|v| v >= at) {
            return Err(LocalRiverError::Invalid("owned course order or overlap"));
        }
        last = Some(at);
        let id = u32::try_from(rivers.len() + 1).map_err(|_| LocalRiverError::Limit)?;
        owners[at] = id;
        let mut course = Vec::new();
        course
            .try_reserve_exact(1)
            .map_err(|_| LocalRiverError::Limit)?;
        course.push(cell);
        let ends_here = if row.to.x / 512 != ax || row.to.y / 512 != ay {
            None
        } else {
            Some(row.end)
        };
        let terminus = match ends_here {
            None | Some(CellEnd::DomainExport) => Terminus::OffTile,
            Some(CellEnd::Sea) => Terminus::Sea,
            Some(CellEnd::Lake) => Terminus::Lake,
            Some(CellEnd::Basin) => Terminus::Basin,
            Some(CellEnd::Junction { branches, .. }) if branches >= 2 => Terminus::Divergence,
            Some(CellEnd::Junction { branches: 1, .. }) => Terminus::Junction,
            _ => return Err(LocalRiverError::Invalid("empty dry junction")),
        };
        ends.push(ends_here);
        rivers.push(RiverSegment {
            global_id: row.reach,
            id,
            order: row.order,
            width_dm: row.width_dm,
            discharge: row.discharge,
            feeds: None,
            ends: terminus,
            course,
        });
    }
    if rivers.len() != maximum {
        return Err(LocalRiverError::Invalid("missing owned cells"));
    }
    for (i, end) in ends.into_iter().enumerate() {
        if rivers[i].ends != Terminus::Junction {
            continue;
        }
        let Some(CellEnd::Junction { at, branches: 1 }) = end else {
            return Err(LocalRiverError::Invalid("single-feed ending"));
        };
        let x = u16::try_from(at.x % 512).map_err(|_| LocalRiverError::Limit)?;
        let y = u16::try_from(at.y % 512).map_err(|_| LocalRiverError::Limit)?;
        let target = owners[CellCoord::new(x, y)
            .ok_or(LocalRiverError::Invalid("target cell"))?
            .index()];
        if target == 0 || target == rivers[i].id {
            return Err(LocalRiverError::Invalid("missing or self local feed"));
        }
        rivers[i].feeds = Some(target);
    }
    Ok(rivers)
}
