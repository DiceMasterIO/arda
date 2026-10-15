//! Height above the nearest water where the stored HAND is unknown
//! (`logic/08` §settle-refusals rule 4; review 3.3-3).
//!
//! The generator stores `u16::MAX` decimetres when its HAND solve found no
//! downstream channel for a cell, typically low ground that drains straight
//! into the sea or a lake. Read as a height, that placeholder lets a cell
//! half a metre above the tide pass the 1.5 m flood rule and count as
//! arable. Here every such cell is measured instead: its height above the
//! level of the nearest open water (sea level for the sea, the spill level
//! of a lake, the bed of a channel), the nearest by chamfer distance. A
//! cell with no water anywhere on the grid stays unknown and is stored as
//! `0`, so it is refused rather than waved through.

use crate::error::SettleError;
use crate::grid::{filled, Grid};
use crate::num::sat_u16;
use arda::TerrainKind;

/// The generator's "no downstream channel" placeholder.
pub const UNKNOWN_DM: u16 = u16::MAX;

/// What [`resolve`] changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Resolved {
    /// Cells whose placeholder was replaced by a measured height.
    pub measured: u64,
    /// Cells with no water on the grid at all, stored as 0 (refused).
    pub refused: u64,
}

/// Water level per cell (mm), `None` on dry land: 0 on the sea, the spill
/// level of its lake on a lake cell, the bed on a channel.
fn levels(g: &Grid) -> Result<Vec<Option<i32>>, SettleError> {
    let mut out = filled(g.len(), None, "water levels")?;
    let mut lake_id = filled(g.len(), u32::MAX, "lake ids")?;
    let mut spills: Vec<i32> = Vec::new();
    for start in 0..g.len() {
        if g.terrain[start] != TerrainKind::Lake || lake_id[start] != u32::MAX {
            continue;
        }
        let id = u32::try_from(spills.len()).unwrap_or(u32::MAX);
        let (mut spill, mut highest_bed) = (i32::MAX, i32::MIN);
        let mut stack = vec![start];
        lake_id[start] = id;
        while let Some(c) = stack.pop() {
            highest_bed = highest_bed.max(g.height_mm[c]);
            for j in g.neighbours4(c) {
                if g.terrain[j] == TerrainKind::Lake {
                    if lake_id[j] == u32::MAX {
                        lake_id[j] = id;
                        stack.push(j);
                    }
                } else if g.is_land(j) {
                    spill = spill.min(g.height_mm[j]);
                }
            }
        }
        // A lake with no land shore (it only meets the sea) stands at its
        // highest bed cell.
        spills.push(if spill == i32::MAX {
            highest_bed
        } else {
            spill
        });
    }
    for (i, o) in out.iter_mut().enumerate() {
        *o = match g.terrain[i] {
            TerrainKind::Sea => Some(0),
            TerrainKind::Lake => spills
                .get(usize::try_from(lake_id[i]).unwrap_or(usize::MAX))
                .copied(),
            TerrainKind::Land if g.order[i] > 0 => Some(g.height_mm[i]),
            TerrainKind::Land => None,
        };
    }
    Ok(out)
}

/// Replaces every [`UNKNOWN_DM`] on land with the height above the level of
/// the nearest open water (two-pass chamfer propagation of the nearest
/// source's level; ties keep the first source met in scan order, so the
/// result is deterministic).
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
pub fn resolve(g: &mut Grid) -> Result<Resolved, SettleError> {
    if !g
        .har_dm
        .iter()
        .zip(&g.terrain)
        .any(|(&h, &t)| h == UNKNOWN_DM && t == TerrainKind::Land)
    {
        return Ok(Resolved::default());
    }
    let (w, h) = (g.width, g.height);
    let mut level = levels(g)?;
    let mut dist = filled(g.len(), u32::MAX, "water distance")?;
    for (d, l) in dist.iter_mut().zip(&level) {
        if l.is_some() {
            *d = 0;
        }
    }
    let relax =
        |dist: &mut Vec<u32>, level: &mut Vec<Option<i32>>, i: usize, j: usize, step: u32| {
            let c = dist[j].saturating_add(step);
            if c < dist[i] {
                dist[i] = c;
                level[i] = level[j];
            }
        };
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            if x > 0 {
                relax(&mut dist, &mut level, i, i - 1, 100);
            }
            if y > 0 {
                relax(&mut dist, &mut level, i, i - w, 100);
                if x > 0 {
                    relax(&mut dist, &mut level, i, i - w - 1, 141);
                }
                if x + 1 < w {
                    relax(&mut dist, &mut level, i, i - w + 1, 141);
                }
            }
        }
    }
    for y in (0..h).rev() {
        for x in (0..w).rev() {
            let i = y * w + x;
            if x + 1 < w {
                relax(&mut dist, &mut level, i, i + 1, 100);
            }
            if y + 1 < h {
                relax(&mut dist, &mut level, i, i + w, 100);
                if x + 1 < w {
                    relax(&mut dist, &mut level, i, i + w + 1, 141);
                }
                if x > 0 {
                    relax(&mut dist, &mut level, i, i + w - 1, 141);
                }
            }
        }
    }
    let mut out = Resolved::default();
    for (i, lv) in level.iter().enumerate() {
        if !g.is_land(i) || g.har_dm[i] != UNKNOWN_DM {
            continue;
        }
        match *lv {
            Some(l) => {
                let above_mm = i64::from(g.height_mm[i]) - i64::from(l);
                // Stay below the placeholder so a measured cell never reads
                // as unknown again.
                g.har_dm[i] = sat_u16(above_mm / 100).min(UNKNOWN_DM - 1);
                out.measured += 1;
            }
            None => {
                g.har_dm[i] = 0;
                out.refused += 1;
            }
        }
    }
    Ok(out)
}
