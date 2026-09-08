//! Bounded read-only adjacency for final flow metrics and saved physical edges.
#![deny(missing_docs)]

use super::fine_flow::{FlowStore, DIRECTIONS};
use super::routing::{CellIndex, Extent, OutletKind, Receiver, RoutingStore};
use arda_core::hydrology::Litres;

/// Maximum routing/flow record reads in one incident query; callers reserve this before calling.
pub const MAX_RECORD_READS: u64 = 18;

/// One actual directed fine-tree edge, including zero-flow original terrestrial drainage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetricEdge {
    /// Actual upstream physical cell.
    pub from: CellIndex,
    /// Actual downstream cell (possibly marine); None only at an actual exterior rim.
    pub to: Option<CellIndex>,
    /// Whole annual litres; zero original edges carry geography but initiate no channel.
    pub annual: Litres,
}

/// A failed read or inconsistent completed fine-tree authority.
#[derive(Debug, thiserror::Error)]
pub enum AdjacencyError<R, F> {
    /// Original physical receiver source error.
    #[error("flow adjacency terrain read failed")]
    Routing(#[source] R),
    /// Original completed flow record error.
    #[error("flow adjacency record read failed")]
    Store(#[source] F),
    /// Completed graph geometry or flow state is invalid.
    #[error("invalid final flow adjacency: {0}")]
    Invalid(&'static str),
}

/// Fixed-size result retaining the original two backend error types.
pub type IncidentResult<R, F> = Result<[Option<MetricEdge>; 9], AdjacencyError<R, F>>;

fn neighbor(extent: Extent, at: CellIndex, direction: usize) -> Option<CellIndex> {
    let last = CellIndex::new(extent.cells() - 1, extent)?;
    let (max_x, max_y) = extent.coordinates(last);
    let (x, y) = extent.coordinates(at);
    let (dx, dy) = DIRECTIONS[direction];
    let x = u32::try_from(i64::from(x) + i64::from(dx)).ok()?;
    let y = u32::try_from(i64::from(y) + i64::from(dy)).ok()?;
    if x > max_x || y > max_y {
        return None;
    }
    CellIndex::new(y * (max_x + 1) + x, extent)
}

/// Read at most two records per D8 point and return a fixed-size incident edge set.
/// This requires a successfully completed fine-flow solve; it never repairs or reroutes input.
/// Positive amounts follow signed subtree flow. Zero original receiver edges keep their
/// original downhill/flat direction; zero selected saddles are unsupported and omitted.
/// Same-lake internal edges are omitted after checking common level, for collapsed metrics.
/// Each nonboundary edge appears in queries at both endpoints; persistence emits it at `from`.
pub fn incident<R: RoutingStore, F: FlowStore>(
    routing: &mut R,
    flow: &mut F,
    at: CellIndex,
) -> IncidentResult<R::Error, F::Error> {
    let extent = routing.extent();
    if flow.extent() != extent || CellIndex::new(at.raw(), extent).is_none() {
        return Err(AdjacencyError::Invalid("extent or coordinate"));
    }
    let terrain = routing.read(at).map_err(AdjacencyError::Routing)?;
    if terrain.is_marine() {
        return Ok([None; 9]);
    }
    let own = flow.read(at).map_err(AdjacencyError::Store)?;
    if !own.visited {
        return Err(AdjacencyError::Invalid("unfinished fine flow"));
    }
    let receiver = terrain
        .receiver(extent, at)
        .ok_or(AdjacencyError::Invalid("unset receiver"))?;
    let mut edges = [None; 9];
    let direct_export = receiver == Receiver::Stop(OutletKind::DomainExport);
    let mut exterior_original = direct_export;
    let mut exterior = if direct_export || own.exterior {
        if direct_export && own.exterior {
            return Err(AdjacencyError::Invalid("duplicate rim edge"));
        }
        let (x, y) = extent.coordinates(at);
        let last = CellIndex::new(extent.cells() - 1, extent)
            .ok_or(AdjacencyError::Invalid("empty extent"))?;
        let (mx, my) = extent.coordinates(last);
        if x != 0 && y != 0 && x != mx && y != my {
            return Err(AdjacencyError::Invalid("interior export"));
        }
        Some(None)
    } else {
        None
    };
    for (direction, slot) in edges[..8].iter_mut().enumerate() {
        let selected = own.selected & (1 << direction) != 0;
        let Some(other_at) = neighbor(extent, at, direction) else {
            if selected {
                return Err(AdjacencyError::Invalid("selected outside direction"));
            }
            continue;
        };
        let other_terrain = routing.read(other_at).map_err(AdjacencyError::Routing)?;
        let forward = receiver == Receiver::Cell(other_at);
        if other_terrain.is_marine() {
            if forward || selected {
                if exterior.is_some() || forward && selected {
                    return Err(AdjacencyError::Invalid("duplicate sea edge"));
                }
                exterior = Some(Some(other_at));
                exterior_original = forward;
            }
            continue;
        }
        let other = flow.read(other_at).map_err(AdjacencyError::Store)?;
        if !other.visited {
            return Err(AdjacencyError::Invalid("unfinished neighbor"));
        }
        if selected != (other.selected & (1 << (7 - direction)) != 0) {
            return Err(AdjacencyError::Invalid("asymmetric saddle"));
        }
        let backward = other_terrain.receiver(extent, other_at) == Some(Receiver::Cell(at));
        if forward && backward || selected && (forward || backward) {
            return Err(AdjacencyError::Invalid("duplicate tree edge"));
        }
        let original = forward || backward;
        let (child, parent, net) = match (own.parent == Some(other_at), other.parent == Some(at)) {
            (true, false) => (at, other_at, own.net),
            (false, true) => (other_at, at, other.net),
            (false, false) if !selected && !original => continue,
            _ => return Err(AdjacencyError::Invalid("tree parent authority")),
        };
        if !selected && !original {
            return Err(AdjacencyError::Invalid("unwitnessed parent edge"));
        }
        if own.lake.is_some() && own.lake == other.lake {
            if own.surface_mm != other.surface_mm
                || own.surface_mm <= terrain.height()
                || own.surface_mm <= other_terrain.height()
            {
                return Err(AdjacencyError::Invalid("disconnected lake identity"));
            }
            continue;
        }
        let (from, to) = if net > 0 {
            (child, parent)
        } else if net < 0 {
            (parent, child)
        } else if forward {
            (at, other_at)
        } else if backward {
            (other_at, at)
        } else {
            continue;
        };
        *slot = Some(MetricEdge {
            from,
            to: Some(to),
            annual: Litres(net.unsigned_abs()),
        });
    }
    match (own.parent, exterior) {
        (None, Some(to)) => {
            let annual = u128::try_from(own.net)
                .map_err(|_| AdjacencyError::Invalid("external water import"))?;
            if annual > 0 || exterior_original {
                edges[8] = Some(MetricEdge {
                    from: at,
                    to,
                    annual: Litres(annual),
                });
            }
        }
        (Some(_), None) => {}
        _ => return Err(AdjacencyError::Invalid("exterior parent authority")),
    }
    Ok(edges)
}
