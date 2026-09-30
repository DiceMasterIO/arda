//! Nameable natural features: river main stems and mountain peaks.

use crate::error::SettleError;
use crate::field::Integral;
use crate::grid::{filled, Grid};

/// Smallest order that starts a named river at its mouth.
const RIVER_MIN_ORDER: u8 = 3;
/// Named rivers are at least this many cells long.
const RIVER_MIN_CELLS: usize = 30;
/// Peak search radius in cells (2.5 km) and minimum rise above its mean.
const PEAK_R: i64 = 25;
const PEAK_RISE_MM: i64 = 150_000;
/// Most peaks named.
const MAX_PEAKS: usize = 60;

/// A traced river main stem.
#[derive(Debug, Clone)]
pub struct River {
    /// 1-based id.
    pub id: u32,
    /// Mouth (most downstream) cell.
    pub mouth: usize,
    /// Cells from mouth upstream.
    pub cells: Vec<usize>,
    /// Highest Strahler order along it.
    pub order: u8,
}

/// A mountain peak.
#[derive(Debug, Clone)]
pub struct Peak {
    /// Summit cell.
    pub cell: usize,
    /// Summit height, metres.
    pub height_m: i32,
}

/// The downstream neighbour of a channel cell: the adjacent channel cell
/// with more drainage, the least such (the next cell rather than a
/// parallel trunk), lowest index on ties.
fn downstream(g: &Grid, i: usize) -> Option<usize> {
    g.neighbours8(i)
        .map(|(j, _, _)| j)
        .filter(|&j| g.is_watercourse(j) && g.drainage[j] > g.drainage[i])
        .min_by_key(|&j| (g.drainage[j], j))
}

/// Traces main stems: from each mouth (largest first) upstream along the
/// biggest tributary; side branches of order ≥ 3 become rivers of their own.
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
pub fn rivers(g: &Grid) -> Result<(Vec<River>, Vec<u32>), SettleError> {
    let mut down = filled(g.len(), u32::MAX, "flow links")?;
    for (i, d) in down.iter_mut().enumerate() {
        if g.is_watercourse(i) {
            if let Some(j) = downstream(g, i) {
                *d = u32::try_from(j).unwrap_or(u32::MAX);
            }
        }
    }
    let flows_into = |j: usize, i: usize| usize::try_from(down[j]).ok() == Some(i);
    let mut heads: Vec<usize> = (0..g.len())
        .filter(|&i| g.is_watercourse(i) && g.order[i] >= RIVER_MIN_ORDER && down[i] == u32::MAX)
        .collect();
    heads.sort_by_key(|&i| (std::cmp::Reverse(g.drainage[i]), i));
    let mut label = filled(g.len(), 0_u32, "river labels")?;
    let mut out: Vec<River> = Vec::new();
    let mut queue: std::collections::VecDeque<usize> = heads.into_iter().collect();
    while let Some(mouth) = queue.pop_front() {
        if label[mouth] != 0 {
            continue;
        }
        let mut cells = Vec::new();
        let mut cur = mouth;
        let mut order = 0;
        loop {
            if label[cur] != 0 {
                break;
            }
            cells.push(cur);
            order = order.max(g.order[cur]);
            let mut ups: Vec<usize> = g
                .neighbours8(cur)
                .map(|(j, _, _)| j)
                .filter(|&j| g.is_watercourse(j) && label[j] == 0 && flows_into(j, cur))
                .collect();
            ups.sort_by_key(|&j| (std::cmp::Reverse(g.drainage[j]), j));
            let Some((&main, rest)) = ups.split_first() else {
                break;
            };
            queue.extend(
                rest.iter()
                    .copied()
                    .filter(|&j| g.order[j] >= RIVER_MIN_ORDER),
            );
            if g.order[main] < 2 {
                break;
            }
            // Mark before moving on so the side branches cannot claim it.
            label[cur] = u32::MAX;
            cur = main;
        }
        if cells.len() < RIVER_MIN_CELLS {
            for &c in &cells {
                label[c] = u32::MAX;
            }
            continue;
        }
        let id = u32::try_from(out.len() + 1).unwrap_or(u32::MAX - 1);
        for &c in &cells {
            label[c] = id;
        }
        out.push(River {
            id,
            mouth,
            cells,
            order,
        });
    }
    for l in &mut label {
        if *l == u32::MAX {
            *l = 0;
        }
    }
    Ok((out, label))
}

/// The id of the named river nearest `i` within `r` cells, if any.
#[must_use]
pub fn river_near(g: &Grid, label: &[u32], i: usize, r: i64) -> Option<u32> {
    let (x, y) = g.xy(i);
    let mut best: Option<(i64, usize, u32)> = None;
    for oy in -r..=r {
        for ox in -r..=r {
            let Some(j) = g.at(x + ox, y + oy) else {
                continue;
            };
            if label[j] != 0 {
                let d = ox * ox + oy * oy;
                if best.is_none_or(|(bd, bj, _)| (d, j) < (bd, bj)) {
                    best = Some((d, j, label[j]));
                }
            }
        }
    }
    best.map(|(_, _, l)| l)
}

/// Summits standing 150 m above their surrounding 5 km, highest first.
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
pub fn peaks(g: &Grid) -> Result<Vec<Peak>, SettleError> {
    let mean = Integral::new(g.width, g.height, |i| i64::from(g.height_mm[i]))?;
    let mut out: Vec<Peak> = Vec::new();
    // A coarse 5-cell stride finds candidate summits; each is then refined
    // to the true local maximum.
    for y in (0..g.height).step_by(5) {
        for x in (0..g.width).step_by(5) {
            let i = y * g.width + x;
            if !g.is_land(i) || g.height_mm[i] < 400_000 {
                continue;
            }
            let (xi, yi) = g.xy(i);
            let mut top = i;
            for oy in -PEAK_R..=PEAK_R {
                for ox in -PEAK_R..=PEAK_R {
                    if let Some(j) = g.at(xi + ox, yi + oy) {
                        if (g.height_mm[j], std::cmp::Reverse(j))
                            > (g.height_mm[top], std::cmp::Reverse(top))
                        {
                            top = j;
                        }
                    }
                }
            }
            let (tx, ty) = g.xy(top);
            if i64::from(g.height_mm[top]) - mean.mean(tx, ty, PEAK_R) >= PEAK_RISE_MM
                && !out.iter().any(|p| p.cell == top)
            {
                out.push(Peak {
                    cell: top,
                    height_m: g.height_mm[top] / 1000,
                });
            }
        }
    }
    out.sort_by_key(|p| (std::cmp::Reverse(p.height_m), p.cell));
    // Keep summits at least 2.5 km apart.
    let mut kept: Vec<Peak> = Vec::new();
    for p in out {
        let (px, py) = g.xy(p.cell);
        let far = kept.iter().all(|k| {
            let (kx, ky) = g.xy(k.cell);
            (kx - px).abs().max((ky - py).abs()) > PEAK_R
        });
        if far && kept.len() < MAX_PEAKS {
            kept.push(p);
        }
    }
    Ok(kept)
}
