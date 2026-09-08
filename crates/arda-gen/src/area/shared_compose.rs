//! Area fields and lake fragments derived only from prepared terrain and solved shared water.
#![deny(missing_docs)]

use super::temperature::temperature_from_base;
use super::{channel_width_dm, fields};
use crate::hydrology::types::PreparedTerrain;
use arda_core::hydrology::{BasinId, GlobalLake};
use arda_core::{
    AreaCells, Cell, CellCoord, Cover, DischargeMilli, Lake, RainfallMm, TerrainKind, AREA_CELLS,
};
use std::collections::BTreeMap;

/// Immutable final flow/occupancy fields for one 100 m cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolvedCell {
    /// Actual rim-connected marine membership.
    pub marine: bool,
    /// Connected representative lake, present only on positive-depth wet cells.
    pub lake: Option<BasinId>,
    /// Distinct upstream contributing cells on the final dry flow graph.
    pub drainage_cells: u32,
    /// Representative mean annual discharge on a dry cell.
    pub discharge: DischargeMilli,
    /// Strahler order; zero below the 40 L/s initiation threshold.
    pub order: u8,
    /// Height above the nearest downstream channel, or u32::MAX when none exists.
    pub hand_mm: u32,
    /// Mean discharge of that downstream channel; zero on the channel itself.
    pub hand_discharge: DischargeMilli,
}

impl Default for SolvedCell {
    fn default() -> Self {
        Self {
            marine: false,
            lake: None,
            drainage_cells: 0,
            discharge: DischargeMilli::new(0),
            order: 0,
            hand_mm: u32::MAX,
            hand_discharge: DischargeMilli::new(0),
        }
    }
}

/// Invalid shared authority, physical geometry, arithmetic or memory admission.
#[derive(Debug, thiserror::Error)]
pub enum ComposeError {
    /// Prepared or solved data cannot describe this exported area.
    #[error("invalid shared area composition: {0}")]
    Invalid(&'static str),
    /// Output allocation exceeded its admitted payload.
    #[error("shared area composition resource limit")]
    Limit,
    /// Physical channel width cannot be represented.
    #[error(transparent)]
    Width(#[from] crate::hydrology::budget::BudgetError),
}

const COUNT: usize = 512 * 512;

/// Conservative own reservation for output cells, relief copy and local lake memberships.
/// Borrowed prepared, solved and global inputs are reserved by the caller.
#[must_use]
pub fn required_ram(lakes: usize) -> Option<u64> {
    let lake_count = u64::try_from(lakes).ok()?;
    lake_count.checked_mul(512)?.checked_add(32 * 1024 * 1024)
}

fn lake_index(global: &[GlobalLake], id: BasinId) -> Result<usize, ComposeError> {
    global
        .binary_search_by_key(&id, |l| l.basin)
        .map_err(|_| ComposeError::Invalid("missing global lake"))
}

/// Composes a full exported area without local lake inference or coarse-water injection.
/// The shared stage supplies river fragments/edges separately; this function owns cell fields
/// and positive-depth local lake membership. Inputs remain immutable on success and failure.
pub fn compose_shared(
    prepared: &PreparedTerrain,
    water: &[SolvedCell],
    global: &[GlobalLake],
    ram_bytes: u64,
) -> Result<(AreaCells, Vec<Lake>), ComposeError> {
    if prepared.valid.width != AREA_CELLS
        || prepared.valid.height != AREA_CELLS
        || prepared.heights.len() != COUNT
        || prepared.annual_rain.len() != COUNT
        || prepared.temperature_base_centi.len() != COUNT
        || water.len() != COUNT
        || prepared.area.x < 0
        || prepared.area.y < 0
    {
        return Err(ComposeError::Invalid("area shape or coordinate"));
    }
    let origin = |v: i32| {
        let base = u32::try_from(v).ok()?.checked_mul(u32::from(AREA_CELLS))?;
        base.checked_add(u32::from(AREA_CELLS) - 1)?;
        Some(base)
    };
    let ax = origin(prepared.area.x).ok_or(ComposeError::Invalid("global area extent"))?;
    let ay = origin(prepared.area.y).ok_or(ComposeError::Invalid("global area extent"))?;
    if global.iter().any(|l| {
        l.deepest_bed >= l.surface
            || l.submerged_cells == 0
            || u64::try_from(l.annual_outflow.0 / 31_536_000).ok() != Some(l.mean_outflow.raw())
            || (l.annual_outflow.0 > 0 && l.outlet.is_none_or(|s| s.sill > l.surface))
    }) {
        return Err(ComposeError::Invalid(
            "global lake geometry or annual outflow",
        ));
    }
    if global.windows(2).any(|p| p[0].basin >= p[1].basin) {
        return Err(ComposeError::Invalid("global lake order"));
    }
    if required_ram(global.len()).is_none_or(|n| n > ram_bytes) {
        return Err(ComposeError::Limit);
    }
    let mut heights = Vec::new();
    heights
        .try_reserve_exact(COUNT)
        .map_err(|_| ComposeError::Limit)?;
    heights.extend(prepared.heights.iter().map(|h| h.raw()));
    let mut cells = AreaCells::flat(Cell::default());
    let mut counts = BTreeMap::<BasinId, usize>::new();
    for y in 0..AREA_CELLS {
        for x in 0..AREA_CELLS {
            let at = CellCoord::new(x, y).ok_or(ComposeError::Invalid("local coordinate"))?;
            let i = at.index();
            let solved = water[i];
            let physical = prepared.heights[i];
            let terrain = match (solved.marine, solved.lake) {
                (true, Some(_)) => return Err(ComposeError::Invalid("lake overlaps marine cell")),
                (true, None) => {
                    if physical.raw() > 0 {
                        return Err(ComposeError::Invalid("positive marine bed"));
                    }
                    TerrainKind::Sea
                }
                (false, Some(id)) => {
                    let l = &global[lake_index(global, id)?];
                    if physical >= l.surface || physical < l.deepest_bed {
                        return Err(ComposeError::Invalid("invalid positive-depth lake member"));
                    }
                    *counts.entry(id).or_default() += 1;
                    TerrainKind::Lake
                }
                (false, None) => TerrainKind::Land,
            };
            let land = terrain == TerrainKind::Land;
            if land && (solved.order > 0) != (solved.discharge.raw() >= 40) {
                return Err(ComposeError::Invalid(
                    "channel initiation disagrees with discharge",
                ));
            }
            if land && solved.order > 0 && (solved.hand_mm != 0 || solved.hand_discharge.raw() != 0)
            {
                return Err(ComposeError::Invalid("channel HAND is not zero"));
            }
            if !land
                && (solved.order > 0 || solved.drainage_cells > 0 || solved.discharge.raw() > 0)
            {
                return Err(ComposeError::Invalid("standing water has dry river fields"));
            }
            let (slope_milli_deg, aspect_deg) =
                fields::slope_and_aspect(&heights, i32::from(x), i32::from(y));
            let drainage = if land { solved.drainage_cells } else { 0 };
            let discharge = if land {
                solved.discharge
            } else {
                DischargeMilli::new(0)
            };
            let order = if land { solved.order } else { 0 };
            let hand_mm = if land { solved.hand_mm } else { 0 };
            let cover = if !land {
                Cover::Bare
            } else if order == 0
                && fields::floodplain(hand_mm) == fields::Floodplain::Marsh
                && solved.hand_discharge.raw() >= 200
            {
                Cover::Marsh
            } else {
                Cover::Grass
            };
            cells.set(
                at,
                Cell {
                    height: physical,
                    terrain,
                    cover,
                    slope_milli_deg,
                    aspect_deg,
                    temperature: temperature_from_base(
                        prepared.temperature_base_centi[i],
                        physical,
                        solved.marine,
                    ),
                    rainfall: if land {
                        prepared.annual_rain[i]
                    } else {
                        RainfallMm::new(0)
                    },
                    drainage_area_cells: drainage,
                    discharge,
                    watercourse_order: order,
                    watercourse_width_dm: if order > 0 {
                        channel_width_dm(discharge)?
                    } else {
                        0
                    },
                    height_above_river_dm: u16::try_from(hand_mm / 100).unwrap_or(u16::MAX),
                    wetness: if land {
                        fields::wetness(drainage, slope_milli_deg)
                    } else {
                        255
                    },
                    ..Cell::default()
                },
            );
        }
    }
    let mut lakes = Vec::new();
    lakes
        .try_reserve_exact(counts.len())
        .map_err(|_| ComposeError::Limit)?;
    let mut indices = BTreeMap::new();
    for (id, count) in counts {
        let l = &global[lake_index(global, id)?];
        if u32::try_from(count).map_err(|_| ComposeError::Limit)? > l.submerged_cells {
            return Err(ComposeError::Invalid("fragment exceeds global membership"));
        }
        let mut members = Vec::new();
        members
            .try_reserve_exact(count)
            .map_err(|_| ComposeError::Limit)?;
        let local_id = u32::try_from(lakes.len() + 1).map_err(|_| ComposeError::Limit)?;
        let outlet = if l.annual_outflow.0 > 0 {
            l.outlet.and_then(|s| {
                CellCoord::new(
                    u16::try_from(s.from.x.checked_sub(ax)?).ok()?,
                    u16::try_from(s.from.y.checked_sub(ay)?).ok()?,
                )
            })
        } else {
            None
        };
        indices.insert(id, lakes.len());
        lakes.push(Lake {
            global_id: id,
            id: local_id,
            surface: l.surface,
            depth_mm: 0,
            outlet,
            cells: members,
        });
    }
    for y in 0..AREA_CELLS {
        for x in 0..AREA_CELLS {
            let at = CellCoord::new(x, y).ok_or(ComposeError::Invalid("local coordinate"))?;
            if let Some(id) = water[at.index()].lake {
                let &i = indices
                    .get(&id)
                    .ok_or(ComposeError::Invalid("fragment identity"))?;
                let l = &mut lakes[i];
                let depth = u32::try_from(
                    i64::from(l.surface.raw()) - i64::from(prepared.heights[at.index()].raw()),
                )
                .map_err(|_| ComposeError::Invalid("lake depth"))?;
                l.depth_mm = l.depth_mm.max(depth);
                l.cells.push(at);
            }
        }
    }
    Ok((cells, lakes))
}

#[cfg(test)]
#[path = "shared_compose_tests.rs"]
mod shared_compose_tests;
