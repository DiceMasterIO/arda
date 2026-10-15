//! Boats moored at a town's docks (logic/10 §town-footprints; review
//! 2026-10-15, gallery 10: a port's four docks had one rowboat between
//! them, drawn on the next dock's deck).
//!
//! Each dock moors boats on open water off its seaward end and along its
//! long sides, a boat about every four squares of side, a sailing boat at
//! the end of a long dock: a port shows shipping in proportion to its
//! docks. Every square a boat covers is plan water, off every deck and
//! building, and clear of every other boat: the plan's docks moor in
//! their fixed order, so a boat belongs to one dock in every window.

use super::exterior::kind;
use super::frame::Want;
use super::interior::GProp;
use crate::function::BuildingFunction as F;
use crate::plan::grid::{Kind, SquareRect};
use crate::plan::{Building, TownPlan};
use crate::rng::{hash_i, unit};
use std::collections::BTreeSet;

/// A rowing boat, `(w, h)` squares at rotation 0 (bow north).
const ROWBOAT: (&str, i64, i64) = ("prop.rowboat", 1, 2);
/// A sailing boat, `(w, h)` squares at rotation 0.
const SAILBOAT: (&str, i64, i64) = ("prop.sailboat", 2, 4);
/// Docks at least this long berth a sailing boat off their end.
const SHIP_DOCK: i64 = 6;
/// Squares of side per boat along a dock.
const BERTH: i64 = 4;
/// Share of berths taken (the rest lie empty).
const TAKEN: f64 = 0.8;

/// The boats moored at dock `b`.
#[must_use]
pub fn at_dock(plan: &TownPlan, b: &Building) -> Vec<GProp> {
    let docks: Vec<&Building> = plan
        .buildings
        .iter()
        .filter(|d| d.function == F::Dock)
        .collect();
    let mut taken: BTreeSet<(i64, i64)> = BTreeSet::new();
    for d in &docks {
        let r = d.rect;
        for y in r.y0..r.y1 {
            for x in r.x0..r.x1 {
                taken.insert((x, y));
            }
        }
    }
    for d in docks {
        let boats = moor(plan, d, &mut taken);
        if d.id == b.id {
            return boats;
        }
    }
    Vec::new()
}

/// Whether a square is open water a boat may lie on.
fn open(plan: &TownPlan, taken: &BTreeSet<(i64, i64)>, x: i64, y: i64) -> bool {
    kind(plan, x, y) == Kind::Water
        && !taken.contains(&(x, y))
        && plan
            .grid
            .gidx(x, y)
            .is_some_and(|k| plan.grid.building[k] == 0 && plan.grid.depth[k] >= 1)
}

/// Lays `boat` with its footprint's north-west square at `(x0, y0)`,
/// turned 90° when `across` (bow east), if the water there is open.
fn lay(
    plan: &TownPlan,
    taken: &mut BTreeSet<(i64, i64)>,
    boat: (&'static str, i64, i64),
    (x0, y0): (i64, i64),
    across: bool,
) -> Option<GProp> {
    let (id, w, h) = boat;
    let (w, h) = if across { (h, w) } else { (w, h) };
    let squares: Vec<(i64, i64)> = (y0..y0 + h)
        .flat_map(|y| (x0..x0 + w).map(move |x| (x, y)))
        .collect();
    if !squares.iter().all(|&(x, y)| open(plan, taken, x, y)) {
        return None;
    }
    // A square of water around each boat, so hulls never touch.
    for y in y0 - 1..=y0 + h {
        for x in x0 - 1..=x0 + w {
            taken.insert((x, y));
        }
    }
    #[allow(clippy::cast_precision_loss)] // squares
    let e = [x0 as f64, y0 as f64, (x0 + w) as f64, (y0 + h) as f64];
    Some(GProp {
        want: Want::Id(id),
        x: (e[0] + e[2]) * 0.5,
        y: (e[1] + e[3]) * 0.5,
        rot: if across { 90 } else { 0 },
        extent: e,
    })
}

/// Moors one dock's boats and marks their water taken.
fn moor(plan: &TownPlan, d: &Building, taken: &mut BTreeSet<(i64, i64)>) -> Vec<GProp> {
    let r = d.rect;
    let along_x = r.w() >= r.h();
    let long = if along_x { r.w() } else { r.h() };
    // The seaward end is the one with more open water beyond it.
    let water = |pts: &mut dyn Iterator<Item = (i64, i64)>| {
        pts.filter(|&(x, y)| open(plan, taken, x, y)).count()
    };
    let (lo, hi) = if along_x {
        (
            water(&mut (r.y0..r.y1).map(|y| (r.x0 - 1, y))),
            water(&mut (r.y0..r.y1).map(|y| (r.x1, y))),
        )
    } else {
        (
            water(&mut (r.x0..r.x1).map(|x| (x, r.y0 - 1))),
            water(&mut (r.x0..r.x1).map(|x| (x, r.y1))),
        )
    };
    let seaward_lo = lo > hi;
    let mut out = Vec::new();
    let keep = |k: i64, side: i64| unit(hash_i(plan.seed ^ 0xB0A7 ^ d.id.0, side, k)) < TAKEN;
    if long >= SHIP_DOCK {
        if let Some(p) = ship_off_end(plan, taken, r, along_x, seaward_lo) {
            out.push(p);
        }
    }
    // Rowing boats along both long sides, from the seaward end in.
    for side in 0..2_i64 {
        let mut k = 0;
        while k + ROWBOAT.2 <= long {
            // Offset along the dock from its seaward end.
            let off = if seaward_lo { k } else { long - k - ROWBOAT.2 };
            let at = if along_x {
                let y = if side == 0 { r.y0 - 1 } else { r.y1 };
                (r.x0 + off, y)
            } else {
                let x = if side == 0 { r.x0 - 1 } else { r.x1 };
                (x, r.y0 + off)
            };
            if keep(k, side) {
                if let Some(p) = lay(plan, taken, ROWBOAT, at, along_x) {
                    out.push(p);
                }
            }
            k += BERTH;
        }
    }
    out
}

/// A sailing boat lying off a dock's seaward end, in line with the dock.
fn ship_off_end(
    plan: &TownPlan,
    taken: &mut BTreeSet<(i64, i64)>,
    r: SquareRect,
    along_x: bool,
    seaward_lo: bool,
) -> Option<GProp> {
    let (_, w, h) = SAILBOAT;
    if along_x {
        // Turned to lie east–west: h squares long, w wide.
        let y = r.y0 + (r.h() - w) / 2;
        let x = if seaward_lo { r.x0 - h - 1 } else { r.x1 + 1 };
        lay(plan, taken, SAILBOAT, (x, y), true)
    } else {
        let x = r.x0 + (r.w() - w) / 2;
        let y = if seaward_lo { r.y0 - h - 1 } else { r.y1 + 1 };
        lay(plan, taken, SAILBOAT, (x, y), false)
    }
}
