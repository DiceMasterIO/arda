//! Per-area derived layers: coast facts and saved river/lake membership.

use crate::contract::coast::{AreaCoast, KindWindow, HALO_CELLS};
use arda::Area;
use arda_core::AREA_CELLS;
use std::sync::Arc;

const SIDE: usize = AREA_CELLS as usize;

/// Derived facts for every cell of one area.
#[derive(Debug)]
pub struct AreaDerived {
    /// Coast flags and capped distances.
    pub coast: AreaCoast,
    /// Per cell, 1 + index into `Area::rivers()` of the owning segment; 0 when none.
    pub river_of: Vec<u32>,
    /// Per cell, 1 + index into `Area::lakes()` of the covering lake; 0 when none.
    pub lake_of: Vec<u32>,
}

impl AreaDerived {
    /// Heap bytes, for cache accounting.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.coast.bytes() + (self.river_of.len() + self.lake_of.len()) * 4
    }
}

/// The 3 × 3 block of areas around a target; `None` outside the world.
pub type Neighbourhood = [[Option<Arc<Area>>; 3]; 3];

/// Builds the haloed terrain-kind window for the centre of `hood`.
#[must_use]
pub fn kind_window(hood: &Neighbourhood) -> KindWindow {
    let width = SIDE + 2 * HALO_CELLS;
    let mut kinds = Vec::with_capacity(width * width);
    for wy in 0..width {
        for wx in 0..width {
            // Offset from the centre area's origin, in cells.
            let (ox, oy) = (wx + SIDE - HALO_CELLS, wy + SIDE - HALO_CELLS);
            let (bx, by) = (ox / SIDE, oy / SIDE);
            let kind = hood[by][bx].as_ref().and_then(|area| {
                let (cx, cy) = (ox % SIDE, oy % SIDE);
                let cx = u16::try_from(cx).ok()?;
                let cy = u16::try_from(cy).ok()?;
                area.cell(cx, cy).ok().map(|c| c.terrain)
            });
            kinds.push(kind);
        }
    }
    KindWindow {
        width,
        height: width,
        kinds,
    }
}

/// Cell → owning river segment. Where courses overlap (confluences), the
/// highest Strahler order wins, then the lowest segment id.
#[must_use]
pub fn river_index(area: &Area) -> Vec<u32> {
    let rivers = area.rivers();
    let mut of = vec![0_u32; SIDE * SIDE];
    for (n, segment) in rivers.iter().enumerate() {
        let Ok(tag) = u32::try_from(n + 1) else {
            break;
        };
        for at in &segment.course {
            let slot = &mut of[at.index()];
            let replace = match slot.checked_sub(1).and_then(|i| rivers.get(i as usize)) {
                None => true,
                Some(held) => {
                    (segment.order, std::cmp::Reverse(segment.id))
                        > (held.order, std::cmp::Reverse(held.id))
                }
            };
            if replace {
                *slot = tag;
            }
        }
    }
    of
}

/// Cell → covering lake; the lowest lake id wins on overlap.
#[must_use]
pub fn lake_index(area: &Area) -> Vec<u32> {
    let lakes = area.lakes();
    let mut of = vec![0_u32; SIDE * SIDE];
    for (n, lake) in lakes.iter().enumerate() {
        let Ok(tag) = u32::try_from(n + 1) else {
            break;
        };
        for at in &lake.cells {
            let slot = &mut of[at.index()];
            let replace = match slot.checked_sub(1).and_then(|i| lakes.get(i as usize)) {
                None => true,
                Some(held) => lake.id < held.id,
            };
            if replace {
                *slot = tag;
            }
        }
    }
    of
}

/// Derives every per-area layer for the centre of `hood`.
#[must_use]
pub fn derive(hood: &Neighbourhood, centre: &Area) -> AreaDerived {
    AreaDerived {
        coast: AreaCoast::derive(&kind_window(hood), HALO_CELLS, SIDE),
        river_of: river_index(centre),
        lake_of: lake_index(centre),
    }
}
