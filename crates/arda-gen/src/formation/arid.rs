//! Recipe-7 climate in formation (logic/02 §fine-formation climate runoff
//! and §world-water arid basins, goals 6, 7, 12 and 21).
//!
//! Formation used to assume 500 mm/yr of runoff everywhere. Recipe 7 reads
//! the continent's annual water balance ([`crate::continent::aridity`]):
//! - **Runoff weights.** Contributing area is weighted by runoff relative to
//!   that nominal 500 mm, so channel initiation, stream-power incision and
//!   channel sizing see real discharge: arid ground is drained by fewer,
//!   smaller channels.
//! - **Basin balance.** Each tectonic basin compares its annual inflow (the
//!   runoff of its whole catchment) with the evaporation its closed
//!   depression could sustain at the spill level. Where evaporation wins the
//!   basin is arid: its lake can never reach the spill, so it is terminal.
//! - **Playas.** An arid basin floor is a sediment-filled pan, flooded from
//!   the sink in height order until it covers three times the equilibrium
//!   lake (wet-year highstands; at least a quarter of the depression). Its
//!   floor keeps an eighth of its relief below the pan level, so it keeps
//!   its shape and drainage while the lake the shared hydrology finds at
//!   equilibrium lies on the pan's low centre. The pan's 100 m cells are
//!   recorded: salt crust from the lake's edge over 70% of the dry floor,
//!   mudflats on the margin and on thin valley-floor fingers.
//!
//! Only the bed is shaped: the shared annual solve still routes and
//! balances every litre, evaporation included (goal 21).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashSet};

use arda_core::LatitudeBand;
use rayon::prelude::*;

use super::drainage::{self, neighbour, receiver_index, receivers, upstream_order, FIXED, SELF};
use super::lattice::{alloc, blur_into, Lattice};
use super::macro_view::MacroView;
use super::water::WaterFeatures;
use super::{FormationError, LOWSTAND_MM};
use crate::continent::ContinentGrid;

/// Radius of the box blur over the macro water balance, km.
pub const BALANCE_BLUR_KM: usize = 12;
/// Runoff at which a cell weighs one finest cell of area (Q8 = 256), mm/yr.
pub const RUNOFF_REF_MM: i64 = 500;
/// A basin is arid when its inflow is below this share of the evaporation
/// its depression could sustain at the spill, ‰.
pub const ARID_PERMILLE: i64 = 900;
/// Pan extent as a multiple of the equilibrium lake.
pub const PAN_LAKE_FACTOR: i64 = 3;
/// Least pan extent as a share of the closed depression, ‰.
pub const PAN_MIN_PERMILLE: i64 = 250;
/// The pan floor keeps 1/8 of its relief below the pan level.
pub const PAN_RELIEF_DIV: i64 = 8;
/// Share of the dry pan, by flood order from the equilibrium lake's edge,
/// under salt crust, ‰; mudflat beyond.
pub const CRUST_PERMILLE: usize = 700;
/// Radius (100 m cells) of the opening that keeps salt crust off thin
/// valley-floor fingers of the pan.
pub const CRUST_OPEN_CELLS: i64 = 3;

/// The water balance on the 1 km macro lattice, mm/yr.
pub struct MacroWater {
    /// Runoff `R`.
    pub runoff: Lattice,
    /// Extra loss of standing water `D`.
    pub deficit: Lattice,
}

/// The recipe-7 balance of `grid` at `band` as macro lattices.
///
/// # Errors
/// Allocation failure.
pub fn macro_water(grid: &ContinentGrid, band: LatitudeBand) -> Result<MacroWater, FormationError> {
    let b = crate::continent::aridity::water_balance(grid, band);
    let mut runoff = Lattice::new(b.width, b.height, 1_000_000_000)?;
    let mut deficit = Lattice::new(b.width, b.height, 1_000_000_000)?;
    for (z, &v) in runoff.z.iter_mut().zip(&b.runoff_mm) {
        *z = i32::from(v);
    }
    for (z, &v) in deficit.z.iter_mut().zip(&b.deficit_mm) {
        *z = i32::from(v);
    }
    // Rain on the unformed macro surface spikes on its 1 km cliffs (up to
    // 10 m/yr on seed-42 MICRO); formation reads the regional balance, a
    // 12 km box blur applied twice, so crests and flanks of one range share
    // their runoff instead of the windward cliff alone dissecting.
    let n = b.width * b.height;
    let mut tmp: Vec<i32> = alloc(n)?;
    let mut out: Vec<i32> = alloc(n)?;
    for l in [&mut runoff, &mut deficit] {
        for _ in 0..2 {
            blur_into(&l.z, b.width, b.height, BALANCE_BLUR_KM, &mut tmp, &mut out);
            l.z.copy_from_slice(&out);
        }
    }
    Ok(MacroWater { runoff, deficit })
}

/// Area weight of a cell with `runoff_mm`, Q8 (256 at [`RUNOFF_REF_MM`]),
/// kept within 1/16..4.
#[must_use]
pub fn weight(runoff_mm: i64) -> u16 {
    (runoff_mm.max(0) * 256 / RUNOFF_REF_MM).clamp(16, 1_024) as u16
}

/// Runoff weights of every node of a `w x h` lattice at spacing `d`.
///
/// # Errors
/// Allocation failure.
pub(super) fn weights(
    view: &MacroView<'_>,
    w: usize,
    h: usize,
    d: i64,
) -> Result<Vec<u16>, FormationError> {
    let mut out: Vec<u16> = alloc(w * h)?;
    out.par_iter_mut().enumerate().for_each(|(i, o)| {
        let (x, y) = ((i % w) as i64 * d, (i / w) as i64 * d);
        *o = weight(view.runoff_mm(x, y).unwrap_or(RUNOFF_REF_MM));
    });
    Ok(out)
}

/// An arid endorheic basin and the pan formation builds in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AridBasin {
    /// Sink point, µm.
    pub sink: (i64, i64),
    /// Annual inflow over the catchment, mm × base cells.
    pub inflow: i64,
    /// Evaporation the depression sustains at its spill, mm × base cells.
    pub capacity: i64,
    /// Equilibrium lake area, m².
    pub lake_m2: i64,
    /// Pan area, m².
    pub pan_m2: i64,
}

/// The arid basins among `sinks` (logic/02 §world-water arid basins), on
/// the `w x h` base lattice at spacing `d`, as [`super::macro_view::macro_basins`]
/// sees the warped macro surface.
///
/// # Errors
/// Allocation failure.
pub(super) fn classify(
    view: &MacroView<'_>,
    (w, h, d): (usize, usize, i64),
    sinks: &[(i64, i64)],
) -> Result<Vec<AridBasin>, FormationError> {
    if sinks.is_empty() || view.runoff_mm(0, 0).is_none() {
        return Ok(Vec::new());
    }
    let n = w * h;
    let mut z: Vec<i32> = alloc(n)?;
    let (mut runoff, mut deficit): (Vec<i32>, Vec<i32>) = (alloc(n)?, alloc(n)?);
    z.par_iter_mut()
        .zip(runoff.par_iter_mut())
        .zip(deficit.par_iter_mut())
        .enumerate()
        .for_each(|(i, ((z, r), e))| {
            let (x, y) = ((i % w) as i64 * d, (i / w) as i64 * d);
            *z = view.height(x, y);
            *r = view.runoff_mm(x, y).unwrap_or(0) as i32;
            *e = view.deficit_mm(x, y).unwrap_or(0) as i32;
        });
    let mut flags: Vec<u8> = alloc(n)?;
    for (i, f) in flags.iter_mut().enumerate() {
        let (x, y) = (i % w, i / w);
        let ocean = z[i] <= -LOWSTAND_MM && view.is_ocean(x as i64 * d, y as i64 * d);
        if ocean || x == 0 || y == 0 || x == w - 1 || y == h - 1 {
            *f = FIXED;
        }
    }
    let (mut next, mut closed): (Vec<u32>, Vec<u8>) = (alloc(n)?, alloc(n)?);
    // The spill level of every closed depression.
    let mut level = z.clone();
    drainage::fill(&mut level, w, h, &flags, 0, &mut next, &mut closed)?;
    // Catchments: with each sink fixed, everything draining to it.
    let sink_cell: Vec<usize> = sinks
        .iter()
        .map(|&(x, y)| {
            let cx = ((x + d / 2) / d).clamp(0, w as i64 - 1) as usize;
            let cy = ((y + d / 2) / d).clamp(0, h as i64 - 1) as usize;
            cy * w + cx
        })
        .collect();
    let mut sink_flags = flags.clone();
    for &c in &sink_cell {
        sink_flags[c] = FIXED;
    }
    let mut graded = z.clone();
    drainage::fill(&mut graded, w, h, &sink_flags, 1, &mut next, &mut closed)?;
    drop(next);
    drop(closed);
    let mut rcv: Vec<u8> = alloc(n)?;
    receivers(&graded, w, h, &sink_flags, 0, 0, &mut rcv);
    let mut order: Vec<u32> = alloc(n)?;
    {
        let mut indeg: Vec<u8> = alloc(n)?;
        upstream_order(&rcv, w, h, &mut indeg, &mut order);
    }
    // Downstream first: each cell takes the label of its receiver.
    let mut label = vec![usize::MAX; n];
    for (k, &c) in sink_cell.iter().enumerate() {
        label[c] = k;
    }
    for &i in order.iter().rev() {
        let i = i as usize;
        if rcv[i] != SELF {
            let r = receiver_index(i, w, h, rcv[i]);
            if r != i && label[i] == usize::MAX {
                label[i] = label[r];
            }
        }
    }
    let cell_m2 = (d / 1_000_000) * (d / 1_000_000);
    let mut out = Vec::new();
    for (k, &sink) in sinks.iter().enumerate() {
        let mut inflow = 0_i64;
        let mut floor: Vec<(i32, usize)> = Vec::new();
        for i in 0..n {
            if label[i] != k {
                continue;
            }
            inflow += i64::from(runoff[i]);
            if level[i] > z[i] {
                floor.push((z[i], i));
            }
        }
        let capacity: i64 = floor.iter().map(|&(_, i)| i64::from(deficit[i])).sum();
        if floor.is_empty() || inflow * 1_000 >= capacity * ARID_PERMILLE {
            continue;
        }
        // Equilibrium: the lowest floor whose evaporation balances inflow.
        floor.sort_unstable();
        let mut paid = 0_i64;
        let mut lake_cells = 0_i64;
        for &(_, i) in &floor {
            if paid >= inflow {
                break;
            }
            paid += i64::from(deficit[i]);
            lake_cells += 1;
        }
        let depression = floor.len() as i64;
        let pan_cells = (lake_cells * PAN_LAKE_FACTOR)
            .max(depression * PAN_MIN_PERMILLE / 1_000)
            .min(depression);
        out.push(AridBasin {
            sink,
            inflow,
            capacity,
            lake_m2: lake_cells * cell_m2,
            pan_m2: pan_cells * cell_m2,
        });
    }
    Ok(out)
}

/// Builds the pan of every arid basin on the fine lattice `g` and records
/// its 100 m cells in `features.pan_cells` (logic/02 §world-water arid
/// basins). Returns the number of fine nodes regraded.
///
/// # Errors
/// Allocation failure.
pub fn carve_pans(
    g: &mut Lattice,
    basins: &[AridBasin],
    (sink_radius_um, cell_um): (i64, i64),
    features: &mut WaterFeatures,
) -> Result<usize, FormationError> {
    let (w, d) = (g.width, g.spacing_um);
    let node_m2 = (d / 1_000_000) * (d / 1_000_000);
    let mut cells: BTreeMap<(u32, u32), u8> = BTreeMap::new();
    let mut total = 0;
    for b in basins {
        let target = (b.pan_m2 / node_m2.max(1)).max(1) as usize;
        let (pan, level) = flood(g, b.sink, sink_radius_um, target);
        let count = pan.len();
        if count == 0 {
            continue;
        }
        let lake_n = ((b.lake_m2 / node_m2.max(1)) as usize).min(count);
        let crust_from = (count - lake_n).max(1);
        let top = i64::from(level);
        for (r, &c) in pan.iter().enumerate() {
            // Sediment fill: the floor's relief below the level shrinks to
            // 1/PAN_RELIEF_DIV of itself, keeping its shape (and so the
            // routes and the lake outline), and never lifts above the level.
            let below = (top - i64::from(g.z[c])).max(0);
            g.z[c] = i32::try_from(top - below / PAN_RELIEF_DIV).unwrap_or(g.z[c]);
            let (x_um, y_um) = ((c % w) as i64 * d, (c / w) as i64 * d);
            let key = (
                ((x_um + cell_um / 2) / cell_um) as u32,
                ((y_um + cell_um / 2) / cell_um) as u32,
            );
            // Salt crust from the lake's edge over most of the dry floor;
            // mudflats on the outer margin.
            let kind = u8::from((r - lake_n.min(r)) * 1_000 >= crust_from * CRUST_PERMILLE);
            let e = cells.entry(key).or_insert(kind);
            *e = (*e).min(kind);
        }
        total += count;
    }
    open_crust(&mut cells);
    features
        .pan_cells
        .extend(cells.into_iter().map(|((x, y), k)| (x, y, k)));
    features.pan_cells.sort_unstable();
    features.pan_cells.dedup_by_key(|c| (c.0, c.1));
    Ok(total)
}

/// Crust stays only where a disc of [`CRUST_OPEN_CELLS`] fits inside it
/// (a morphological opening): the flood reaches up valley floors as thin
/// fingers, which are washes of fan mud, not evaporite. They become
/// mudflat.
fn open_crust(cells: &mut BTreeMap<(u32, u32), u8>) {
    let r = CRUST_OPEN_CELLS;
    let crust = |m: &BTreeMap<(u32, u32), u8>, x: i64, y: i64| {
        let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
            return false;
        };
        m.get(&(x, y)) == Some(&0)
    };
    let disc = |x: i64, y: i64, f: &dyn Fn(i64, i64) -> bool| {
        (-r..=r).all(|dy| (-r..=r).all(|dx| dx * dx + dy * dy > r * r || f(x + dx, y + dy)))
    };
    let core: HashSet<(u32, u32)> = cells
        .iter()
        .filter(|&(&(x, y), &k)| {
            k == 0 && disc(i64::from(x), i64::from(y), &|a, b| crust(cells, a, b))
        })
        .map(|(&c, _)| c)
        .collect();
    let opened: HashSet<(u32, u32)> = cells
        .iter()
        .filter(|&(&(x, y), &k)| {
            k == 0
                && !disc(i64::from(x), i64::from(y), &|a, b| {
                    let (Ok(a), Ok(b)) = (u32::try_from(a), u32::try_from(b)) else {
                        return true;
                    };
                    !core.contains(&(a, b))
                })
        })
        .map(|(&c, _)| c)
        .collect();
    for (c, k) in cells.iter_mut() {
        if *k == 0 && !opened.contains(c) {
            *k = 1;
        }
    }
}

/// The pan of one basin on a drained lattice: the protected sink disc
/// (lowest first), then a priority flood outward in height order until
/// `target` nodes or the spill, where a popped node lies below the level
/// so far. Every node draining to the disc has a descending path to a disc
/// neighbour, and all of those enter the queue at the start, so the level
/// only falls past the spill. Returns the nodes in pan order and the level.
fn flood(g: &Lattice, sink: (i64, i64), radius_um: i64, target: usize) -> (Vec<usize>, i32) {
    let (w, h, d) = (g.width, g.height, g.spacing_um);
    let r = radius_um / d.max(1) + 1;
    let (sx, sy) = (sink.0.div_euclid(d), sink.1.div_euclid(d));
    let mut disc: Vec<(i32, usize)> = Vec::new();
    for y in (sy - r).max(1)..=(sy + r).min(h as i64 - 2) {
        for x in (sx - r).max(1)..=(sx + r).min(w as i64 - 2) {
            let (dx, dy) = (i128::from(x * d - sink.0), i128::from(y * d - sink.1));
            if dx * dx + dy * dy <= i128::from(radius_um) * i128::from(radius_um) {
                let i = y as usize * w + x as usize;
                disc.push((g.z[i], i));
            }
        }
    }
    disc.sort_unstable();
    let mut seen: HashSet<usize> = disc.iter().map(|&(_, i)| i).collect();
    let mut pan: Vec<usize> = disc.iter().map(|&(_, i)| i).collect();
    let mut level = disc.iter().map(|&(z, _)| z).max().unwrap_or(i32::MIN);
    let mut heap = BinaryHeap::new();
    for &(_, c) in &disc {
        for k in 0..8 {
            if let Some(nb) = neighbour(c, w, h, k) {
                let (x, y) = (nb % w, nb / w);
                let rim = x == 0 || y == 0 || x == w - 1 || y == h - 1;
                if !rim && seen.insert(nb) {
                    heap.push((Reverse(g.z[nb]), Reverse(nb)));
                }
            }
        }
    }
    let mut outer = i32::MIN;
    while let Some((Reverse(zc), Reverse(c))) = heap.pop() {
        if zc < outer || pan.len() >= target {
            break;
        }
        outer = zc;
        level = level.max(zc);
        pan.push(c);
        for k in 0..8 {
            if let Some(nb) = neighbour(c, w, h, k) {
                let (x, y) = (nb % w, nb / w);
                let rim = x == 0 || y == 0 || x == w - 1 || y == h - 1;
                if !rim && seen.insert(nb) {
                    heap.push((Reverse(g.z[nb]), Reverse(nb)));
                }
            }
        }
    }
    (pan, level)
}

#[cfg(test)]
#[path = "arid_tests.rs"]
mod tests;
