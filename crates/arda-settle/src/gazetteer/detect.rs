//! Finding the features: seas and bays, lakes, islands, capes, forests and
//! marshes, as masks on the coarse raster ([`super::coarse`]).
//!
//! Rules (logic/15 §name-what-is-named):
//! - a sea is a connected body of sea of at least `sea_ha`; inside it, the
//!   cores of the sea farther than `bay_radius` from land are told apart,
//!   the largest is the sea proper and every other core with its share of
//!   water (each water cell goes to the nearest core) is a bay, or a gulf
//!   if it is itself sea sized;
//! - a lake is a connected lake of at least `lake_ha`;
//! - an island is a body of land (lakes included) other than the largest
//!   of at least `island_ha`;
//! - a cape is a piece of land that survives no opening at `cape_radius`
//!   (too narrow to hold a disc of that radius), joined to the land that
//!   does, and at least `cape_ha` in size; its anchor is the tip;
//! - a forest is connected closed canopy of at least `forest_ha`, a marsh
//!   connected marsh of at least `marsh_ha`.

use super::coarse::{components, edt, is_far, Coarse, Comps, CELL_HA, CELL_M};
use super::shape::{shape, Shape};
use super::{Found, Thresholds};
use crate::grid::Grid;
use crate::num::{iu, ui};
use arda::TerrainKind;
use arda_names::PlaceKind;
use std::collections::VecDeque;

/// What the finders share.
pub struct Ctx<'a> {
    /// The coarse masks.
    pub co: &'a Coarse,
    /// The fine grid.
    pub g: &'a Grid,
    /// World size, metres.
    pub world_m: [i64; 2],
}

impl Ctx<'_> {
    /// Moves a coarse cell's centre onto a fine cell of the right ground:
    /// the nearest cell of its block that is sea (seas, bays), lake (lakes)
    /// or land (the rest).
    fn snap(&self, at: [i64; 2], kind: PlaceKind) -> [i64; 2] {
        let want = |t: TerrainKind| match kind {
            PlaceKind::Sea | PlaceKind::Bay => t == TerrainKind::Sea,
            PlaceKind::Lake => t == TerrainKind::Lake,
            _ => t == TerrainKind::Land,
        };
        let (cx, cy) = (at[0] / 100, at[1] / 100);
        let half = ui(super::coarse::F) / 2 + 1;
        let mut best: Option<(i64, i64, i64)> = None;
        for y in cy - half..=cy + half {
            for x in cx - half..=cx + half {
                if let Some(i) = self.g.at(x, y) {
                    let d = (x - cx).pow(2) + (y - cy).pow(2);
                    if want(self.g.terrain[i]) && best.is_none_or(|b| (d, y, x) < b) {
                        best = Some((d, y, x));
                    }
                }
            }
        }
        best.map_or(at, |(_, y, x)| [x * 100 + 50, y * 100 + 50])
    }
}

/// Paths need at least this many coarse cells of length.
const MIN_PATH: u64 = 8;

/// Turns a shape into a [`Found`].
fn found(cx: &Ctx<'_>, kind: PlaceKind, area: u64, s: &Shape) -> Found {
    let (co, world_m) = (cx.co, cx.world_m);
    let metres = |i: usize| cx.snap(co.centre_m(i), kind);
    let path_m: Option<Vec<[i64; 2]>> = s
        .path
        .as_ref()
        .map(|p| p.iter().map(|&i| metres(i)).collect());
    let [x0, y0, x1, y1] = s.bbox;
    let extent_m = [
        ui(x0) * CELL_M,
        ui(y0) * CELL_M,
        ((ui(x1) + 1) * CELL_M).min(world_m[0]),
        ((ui(y1) + 1) * CELL_M).min(world_m[1]),
    ];
    Found {
        kind,
        anchor_m: metres(s.anchor),
        path_m,
        extent_m,
        area_ha: area * CELL_HA,
        length_m: s.length * u64::try_from(CELL_M).unwrap_or(400),
        detail: 0,
    }
}

/// The largest `cap` components by size, in a fixed order.
fn biggest(list: &[super::coarse::Comp], min_cells: u64, cap: usize) -> Vec<usize> {
    let mut ids: Vec<usize> = (0..list.len())
        .filter(|&k| list[k].count >= min_cells)
        .collect();
    ids.sort_by_key(|&k| (std::cmp::Reverse(list[k].count), list[k].first));
    ids.truncate(cap);
    ids
}

fn cells_of(ha: u64) -> u64 {
    ha.div_ceil(CELL_HA)
}

/// Every component of `comps` that passes the size test, as features.
fn from_comps(cx: &Ctx<'_>, comps: &Comps, kind: PlaceKind, min_ha: u64, cap: usize) -> Vec<Found> {
    let co = cx.co;
    biggest(&comps.list, cells_of(min_ha), cap)
        .into_iter()
        .map(|k| {
            let c = &comps.list[k];
            let id = u32::try_from(k + 1).unwrap_or(u32::MAX);
            let s = shape(co, &comps.label, id, c.bbox, MIN_PATH);
            found(cx, kind, c.count, &s)
        })
        .collect()
}

/// Seas and their bays.
pub fn seas(cx: &Ctx<'_>, t: &Thresholds) -> Vec<Found> {
    let co = cx.co;
    let sea = components(co.w, co.h, &co.sea, false);
    let mut out = from_comps(cx, &sea, PlaceKind::Sea, t.sea_ha, t.max_seas);
    let big: Vec<bool> = sea
        .list
        .iter()
        .map(|c| c.count >= cells_of(t.sea_ha))
        .collect();
    let r2 = ui(t.bay_radius).pow(2);
    let dist = edt(co.w, co.h, &co.sea);
    let core: Vec<bool> = co
        .sea
        .iter()
        .zip(&dist)
        .map(|(&s, &d)| s && !is_far(d) && d >= r2)
        .collect();
    let cores = components(co.w, co.h, &core, false);
    // Every sea cell goes to its nearest core (steps over water).
    let mut basin = vec![0_u32; co.w * co.h];
    let mut queue: VecDeque<usize> = VecDeque::new();
    for (i, &c) in cores.label.iter().enumerate() {
        if c != 0 {
            basin[i] = c;
            queue.push_back(i);
        }
    }
    while let Some(i) = queue.pop_front() {
        let (x, y) = (ui(i % co.w), ui(i / co.w));
        for (dx, dy) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
            if let Some(j) = co.at(x + dx, y + dy) {
                if co.sea[j] && basin[j] == 0 {
                    basin[j] = basin[i];
                    queue.push_back(j);
                }
            }
        }
    }
    let mut share = vec![0_u64; cores.list.len() + 1];
    for &b in &basin {
        share[iu(i64::from(b))] += 1;
    }
    // The sea proper in each big body: its largest share.
    let mut main = vec![0_u32; sea.list.len() + 1];
    for (k, c) in cores.list.iter().enumerate() {
        let id = u32::try_from(k + 1).unwrap_or(u32::MAX);
        let body = iu(i64::from(sea.label[c.first]));
        let cur = iu(i64::from(main[body]));
        if main[body] == 0
            || (share[k + 1], std::cmp::Reverse(id)) > (share[cur], std::cmp::Reverse(main[body]))
        {
            main[body] = id;
        }
    }
    let mut bays: Vec<usize> = (0..cores.list.len())
        .filter(|&k| {
            let c = &cores.list[k];
            let body = iu(i64::from(sea.label[c.first]));
            let id = u32::try_from(k + 1).unwrap_or(u32::MAX);
            big.get(body.wrapping_sub(1)).copied().unwrap_or(false)
                && main[body] != id
                && share[k + 1] >= cells_of(t.bay_ha)
        })
        .collect();
    bays.sort_by_key(|&k| (std::cmp::Reverse(share[k + 1]), cores.list[k].first));
    bays.truncate(t.max_bays);
    for k in bays {
        let id = u32::try_from(k + 1).unwrap_or(u32::MAX);
        let (mut x0, mut y0, mut x1, mut y1) = (co.w, co.h, 0, 0);
        for (i, &b) in basin.iter().enumerate() {
            if b == id {
                let (x, y) = (i % co.w, i / co.w);
                (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
            }
        }
        let s = shape(co, &basin, id, [x0, y0, x1, y1], MIN_PATH);
        let kind = if share[k + 1] >= cells_of(t.sea_ha) {
            PlaceKind::Sea
        } else {
            PlaceKind::Bay
        };
        out.push(found(cx, kind, share[k + 1], &s));
    }
    out
}

/// Lakes.
pub fn lakes(cx: &Ctx<'_>, t: &Thresholds) -> Vec<Found> {
    let co = cx.co;
    let c = components(co.w, co.h, &co.lake, false);
    from_comps(cx, &c, PlaceKind::Lake, t.lake_ha, t.max_lakes)
}

/// Islands: every body of land but the largest.
pub fn islands(cx: &Ctx<'_>, t: &Thresholds) -> Vec<Found> {
    let co = cx.co;
    let dry: Vec<bool> = co.sea.iter().map(|&s| !s).collect();
    let mut c = components(co.w, co.h, &dry, true);
    let mainland = (0..c.list.len())
        .max_by_key(|&k| (c.list[k].count, std::cmp::Reverse(c.list[k].first)))
        .unwrap_or(0);
    if let Some(m) = c.list.get_mut(mainland) {
        m.count = 0;
    }
    from_comps(cx, &c, PlaceKind::Island, t.island_ha, t.max_islands)
}

/// Forests.
pub fn forests(cx: &Ctx<'_>, t: &Thresholds) -> Vec<Found> {
    let co = cx.co;
    let c = components(co.w, co.h, &co.forest, true);
    from_comps(cx, &c, PlaceKind::Forest, t.forest_ha, t.max_forests)
}

/// Marshes.
pub fn marshes(cx: &Ctx<'_>, t: &Thresholds) -> Vec<Found> {
    let co = cx.co;
    let c = components(co.w, co.h, &co.marsh, true);
    from_comps(cx, &c, PlaceKind::Marsh, t.marsh_ha, t.max_marshes)
}

/// Capes: land too narrow for a disc of `cape_radius`, hanging off land
/// that is not.
pub fn capes(cx: &Ctx<'_>, t: &Thresholds) -> Vec<Found> {
    let co = cx.co;
    let dry: Vec<bool> = co.sea.iter().map(|&s| !s).collect();
    let r2 = ui(t.cape_radius).pow(2);
    let to_sea = edt(co.w, co.h, &dry);
    let core: Vec<bool> = dry
        .iter()
        .zip(&to_sea)
        .map(|(&l, &d)| l && !is_far(d) && d >= r2)
        .collect();
    if !core.iter().any(|&c| c) {
        return Vec::new();
    }
    let not_core: Vec<bool> = core.iter().map(|&c| !c).collect();
    let to_core = edt(co.w, co.h, &not_core);
    let narrow: Vec<bool> = dry
        .iter()
        .zip(&to_core)
        .map(|(&l, &d)| l && d > r2)
        .collect();
    let c = components(co.w, co.h, &narrow, false);
    let mut keep: Vec<usize> = Vec::new();
    for (k, comp) in c.list.iter().enumerate() {
        if comp.count < cells_of(t.cape_ha) {
            continue;
        }
        let id = u32::try_from(k + 1).unwrap_or(u32::MAX);
        let [x0, y0, x1, y1] = comp.bbox;
        let mut joined = false;
        'scan: for y in y0..=y1 {
            for x in x0..=x1 {
                if c.label[y * co.w + x] != id {
                    continue;
                }
                for (dx, dy) in [(1, 0), (0, 1), (-1, 0), (0, -1)] {
                    if let Some(j) = co.at(ui(x) + dx, ui(y) + dy) {
                        if dry[j] && to_core[j] <= r2 {
                            joined = true;
                            break 'scan;
                        }
                    }
                }
            }
        }
        if joined {
            keep.push(k);
        }
    }
    keep.sort_by_key(|&k| (std::cmp::Reverse(c.list[k].count), c.list[k].first));
    keep.truncate(t.max_capes);
    keep.into_iter()
        .map(|k| {
            let comp = &c.list[k];
            let id = u32::try_from(k + 1).unwrap_or(u32::MAX);
            let [x0, y0, x1, y1] = comp.bbox;
            let mut tip = (0_i64, comp.first);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let i = y * co.w + x;
                    if c.label[i] == id && to_core[i] > tip.0 {
                        tip = (to_core[i], i);
                    }
                }
            }
            let s = Shape {
                anchor: tip.1,
                path: None,
                length: u64::try_from(tip.0).map_or(1, |d| crate::num::isqrt(d).max(1)),
                bbox: comp.bbox,
            };
            found(cx, PlaceKind::Cape, comp.count, &s)
        })
        .collect()
}
