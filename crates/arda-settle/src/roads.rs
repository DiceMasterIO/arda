//! The road hierarchy (spec step 5; artifact "Roads").
//!
//! Built top-down so lower tiers reuse higher ones: least-cost routes between
//! nearby towns, the cheapest set that connects them all becomes the trunk
//! (highways), trunks are pushed to map edges where land leaves the map,
//! each village is routed to its nearest town, each hamlet to its nearest
//! village, and footpaths join neighbouring villages. Because existing road
//! is cheap, later routes fall onto earlier ones and share trunks; only the
//! new cells of a route become a road object, so there are no parallel
//! duplicates. The trunk is not left a tree: neighbouring towns a long way
//! round by road get a direct highway (`trunk::close_loops`).

use crate::cost::CostSurface;
use crate::grid::Grid;
use crate::model::{Settlement, Tier};
use crate::num::dist_m;
use crate::route::{self, Path};
use crate::trunk;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Road class, shared through `arda-ids`: the world's stored codes with
/// footpath appended (canonical convention I3). Compare importance with
/// [`RoadClass::rank`], never the code.
pub use arda_ids::RoadClass;

/// One road object: the new cells a route added to the network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Road {
    /// 1-based id in build order.
    #[serde(with = "crate::ids::string")]
    pub id: u64,
    /// Class.
    pub class: RoadClass,
    /// Settlement the route starts at.
    #[serde(with = "crate::ids::string")]
    pub from: u64,
    /// Settlement the route ends at, if any.
    #[serde(with = "crate::ids::opt_string")]
    pub to: Option<u64>,
    /// Map edge the route leaves through (`north`, `east`, `south`, `west`).
    pub to_edge: Option<String>,
    /// Length of the whole route, shared stretches included, metres.
    pub length_m: u64,
    /// Straight-line distance between the ends, metres.
    pub straight_m: u64,
    /// Length of the new cells this road added, metres.
    pub new_m: u64,
    /// Height range along the whole route, metres (open ground is < 150).
    pub relief_m: i32,
    /// Ground the whole route crosses, by its median slope.
    pub terrain: Terrain,
    /// Metres of the whole route on 100 m steps steeper than its class
    /// sustains (`cost::max_grade_pct`): where the tactical road must
    /// switch back within the cell (goal 37; review 3.3-7). 0 on older
    /// files.
    #[serde(default)]
    pub steep_m: u64,
    /// New stretches as polylines of `[x_m, y_m]` cell centres.
    pub segments: Vec<Vec<[i64; 2]>>,
}

/// Terrain class of a route, by the median slope along it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terrain {
    /// Median slope under 5° (the artifact's open ground).
    Open,
    /// Median slope 5–12°.
    Hill,
    /// Median slope 12° or more.
    Mountain,
}

impl Terrain {
    /// All classes, gentlest first.
    pub const ALL: [Self; 3] = [Self::Open, Self::Hill, Self::Mountain];

    /// Stored key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Hill => "hill",
            Self::Mountain => "mountain",
        }
    }

    /// Class of a route over `cells`.
    #[must_use]
    pub fn of(g: &Grid, cells: &[usize]) -> Self {
        let mut slopes: Vec<u16> = cells.iter().map(|&c| g.slope_md[c]).collect();
        slopes.sort_unstable();
        let median = slopes.get(slopes.len() / 2).copied().unwrap_or(0);
        if median < 5000 {
            Self::Open
        } else if median < 12_000 {
            Self::Hill
        } else {
            Self::Mountain
        }
    }
}

/// The built network.
#[derive(Debug, Clone, Default)]
pub struct Network {
    /// Road objects in build order.
    pub roads: Vec<Road>,
    /// Full cell path of each road's route (same order as `roads`).
    pub routes: Vec<Vec<usize>>,
    /// Most important road class per cell as a stored code (0 none, see
    /// [`RoadClass::code`]).
    pub raster: Vec<u8>,
    /// Settlements no route could reach.
    pub unreachable: Vec<u64>,
    /// Open-water cells a ferry leg crosses, with the most important class
    /// using each as a stored code. Ferry legs are never road raster
    /// (`roads.bin` is 0 on lake and sea; review 3.3-15).
    pub ferry: BTreeMap<usize, u8>,
}

impl Network {
    pub(crate) fn on_road(&self) -> impl Fn(usize) -> bool + '_ {
        |i| self.raster[i] != 0 || self.ferry.contains_key(&i)
    }

    /// Whether cell `i` carries a road or a ferry leg.
    #[must_use]
    pub fn carries(&self, i: usize) -> bool {
        self.raster[i] != 0 || self.ferry.contains_key(&i)
    }

    /// The stored class code of the ferry leg over cell `i` (0 if none).
    #[must_use]
    pub fn ferry_class(&self, i: usize) -> u8 {
        self.ferry.get(&i).copied().unwrap_or(0)
    }

    pub(crate) fn commit(&mut self, g: &Grid, path: &Path, class: RoadClass, from: u64, to: End) {
        let mut segments = Vec::new();
        let mut run: Vec<usize> = Vec::new();
        let mut new_m = 0;
        let mut flush = |run: &mut Vec<usize>, segments: &mut Vec<Vec<[i64; 2]>>| {
            if run.len() > 1 {
                new_m += route::length_m(g, run);
                segments.push(polyline(g, run));
            }
            run.clear();
        };
        for (k, &c) in path.cells.iter().enumerate() {
            if g.is_water(c) {
                // A ferry leg ends any new stretch: it is not road.
                flush(&mut run, &mut segments);
            } else if self.raster[c] == 0 {
                if run.is_empty() && k > 0 && !g.is_water(path.cells[k - 1]) {
                    run.push(path.cells[k - 1]);
                }
                run.push(c);
            } else if !run.is_empty() {
                run.push(c);
                flush(&mut run, &mut segments);
            }
        }
        flush(&mut run, &mut segments);
        for &c in &path.cells {
            if g.is_water(c) {
                let code = self.ferry.entry(c).or_insert(0);
                *code = class.max_code(*code);
            } else {
                self.raster[c] = class.max_code(self.raster[c]);
            }
        }
        let (a, b) = (path.cells[0], path.cells[path.cells.len() - 1]);
        let (ax, ay) = g.xy(a);
        let (bx, by) = g.xy(b);
        let (to, to_edge) = match to {
            End::Settlement(id) => (Some(id), None),
            End::Edge(e) => (None, Some(e.to_string())),
        };
        self.roads.push(Road {
            id: u64::try_from(self.roads.len() + 1).unwrap_or(u64::MAX),
            class,
            from,
            to,
            to_edge,
            length_m: route::length_m(g, &path.cells),
            straight_m: u64::from(dist_m(bx - ax, by - ay)),
            new_m,
            relief_m: {
                let hs = path.cells.iter().map(|&c| g.height_mm[c]);
                (hs.clone().max().unwrap_or(0) - hs.min().unwrap_or(0)) / 1000
            },
            terrain: Terrain::of(g, &path.cells),
            steep_m: steep_m(g, &path.cells, class),
            segments,
        });
        self.routes.push(path.cells.clone());
    }
}

/// Metres of `cells` on steps steeper than `class` sustains (open water
/// legs excluded).
#[must_use]
pub fn steep_m(g: &Grid, cells: &[usize], class: RoadClass) -> u64 {
    let max = crate::cost::max_grade_pct(class);
    cells
        .windows(2)
        .filter(|w| !g.is_water(w[0]) && !g.is_water(w[1]))
        .map(|w| {
            let (ax, ay) = g.xy(w[0]);
            let (bx, by) = g.xy(w[1]);
            let len: i64 = if ax != bx && ay != by { 141 } else { 100 };
            let rise = (i64::from(g.height_mm[w[1]]) - i64::from(g.height_mm[w[0]])).abs();
            if rise * 100 > max * len * 1000 {
                u64::try_from(len).unwrap_or(0)
            } else {
                0
            }
        })
        .sum()
}

/// Where a route ends.
#[derive(Debug, Clone, Copy)]
pub(crate) enum End {
    Settlement(u64),
    Edge(&'static str),
}

/// Cell centres in metres with collinear interior points dropped.
fn polyline(g: &Grid, cells: &[usize]) -> Vec<[i64; 2]> {
    let pts: Vec<(i64, i64)> = cells.iter().map(|&c| g.xy(c)).collect();
    let mut out = Vec::new();
    for (k, &(x, y)) in pts.iter().enumerate() {
        let keep = k == 0
            || k + 1 == pts.len()
            || (x - pts[k - 1].0, y - pts[k - 1].1) != (pts[k + 1].0 - x, pts[k + 1].1 - y);
        if keep {
            out.push([x * 100 + 50, y * 100 + 50]);
        }
    }
    out
}

/// Builds the whole network.
#[must_use]
pub fn build(g: &Grid, cs: &CostSurface, settlements: &[Settlement]) -> Network {
    let mut net = Network {
        raster: vec![0; g.len()],
        ..Network::default()
    };
    let mut hubs: Vec<&Settlement> = settlements.iter().filter(|s| s.tier.is_urban()).collect();
    if hubs.is_empty() {
        hubs.extend(
            settlements
                .iter()
                .filter(|s| s.tier == Tier::Village)
                .take(1),
        );
    }
    trunk::trunk(g, cs, &mut net, &hubs);
    trunk::close_loops(g, cs, &mut net, &hubs);
    trunk::edges(g, cs, &mut net, &hubs);
    let villages: Vec<&Settlement> = settlements
        .iter()
        .filter(|s| s.tier == Tier::Village)
        .collect();
    let mut order: Vec<&Settlement> = villages.clone();
    order.sort_by(|a, b| b.population.cmp(&a.population).then(a.id.cmp(&b.id)));
    for v in &order {
        connect_cheapest(g, cs, &mut net, v, &nearest_k(v, &hubs, 3), RoadClass::Road);
    }
    let mut parents: Vec<&Settlement> = hubs.clone();
    parents.extend(villages.iter().copied());
    for h in settlements.iter().filter(|s| s.tier == Tier::Hamlet) {
        connect_cheapest(
            g,
            cs,
            &mut net,
            h,
            &nearest_k(h, &parents, 3),
            RoadClass::Track,
        );
    }
    for v in &order {
        if let Some(n) = nearest(v, &villages, v.id.get()) {
            let d = dist_m(
                ui_of(n.cell_x) - ui_of(v.cell_x),
                ui_of(n.cell_y) - ui_of(v.cell_y),
            );
            if d <= 4000 && v.id < n.id {
                footpath(g, cs, &mut net, v, n);
            }
        }
    }
    net
}

pub(crate) fn ui_of(v: u32) -> i64 {
    i64::from(v)
}

fn nearest<'a>(s: &Settlement, among: &[&'a Settlement], not: u64) -> Option<&'a Settlement> {
    nearest_k(s, among, 1)
        .into_iter()
        .find(|o| o.id != arda_ids::SettlementId(not))
}

/// The `k` settlements of `among` nearest `s` in a straight line (excluding `s`).
fn nearest_k<'a>(s: &Settlement, among: &[&'a Settlement], k: usize) -> Vec<&'a Settlement> {
    let mut v: Vec<(u32, arda_ids::SettlementId, &'a Settlement)> = among
        .iter()
        .filter(|o| o.id != s.id)
        .map(|o| {
            let d = dist_m(
                ui_of(o.cell_x) - ui_of(s.cell_x),
                ui_of(o.cell_y) - ui_of(s.cell_y),
            );
            (d, o.id, *o)
        })
        .collect();
    v.sort_by_key(|&(d, id, _)| (d, id));
    v.into_iter().take(k).map(|(_, _, o)| o).collect()
}

/// Routes `a` to whichever candidate is cheapest to reach over the current
/// network ("nearest" by effort, not by crow flight, so a village behind a
/// ridge joins the town down its own valley).
fn connect_cheapest(
    g: &Grid,
    cs: &CostSurface,
    net: &mut Network,
    a: &Settlement,
    candidates: &[&Settlement],
    class: RoadClass,
) {
    // Nothing to join (the stand-in hub itself): not cut off.
    if candidates.is_empty() {
        return;
    }
    let best = {
        let on = net.on_road();
        let targets: Vec<usize> = candidates.iter().map(|b| b.index(g.width)).collect();
        let from = a.index(g.width);
        route::nearest(g, cs, &on, from, &targets, class)
            .map(|(k, p)| (p.cost, candidates[k].id, route::straighten(g, cs, p, class)))
    };
    match best {
        Some((_, id, p)) if p.cells.len() > 1 => {
            net.commit(g, &p, class, a.id.get(), End::Settlement(id.get()));
        }
        Some(_) => {}
        None => net.unreachable.push(a.id.get()),
    }
}

/// A footpath is a short cut between neighbours: it is only worth making
/// when it is one, so a path that would wind round a fjord or a lake to
/// reach the next village is left unbuilt. It is routed over the ground
/// alone, not the road discount, so it takes its own line across the
/// fields; a path that would only retrace roads adds nothing and is not
/// recorded (review 3.3-6).
fn footpath(g: &Grid, cs: &CostSurface, net: &mut Network, a: &Settlement, b: &Settlement) {
    let none = |_: usize| false;
    let path = route::find(
        g,
        cs,
        &none,
        a.index(g.width),
        b.index(g.width),
        RoadClass::Footpath,
    );
    let Some(p) = path.filter(|p| p.cells.len() > 1) else {
        return;
    };
    if p.cells.iter().all(|&c| net.raster[c] != 0) {
        return;
    }
    let straight = u64::from(dist_m(
        ui_of(b.cell_x) - ui_of(a.cell_x),
        ui_of(b.cell_y) - ui_of(a.cell_y),
    ));
    if route::length_m(g, &p.cells) * 100 <= straight * route::DETOUR_CAP_PCT {
        net.commit(
            g,
            &p,
            RoadClass::Footpath,
            a.id.get(),
            End::Settlement(b.id.get()),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_follow_the_canonical_table_and_rank_by_importance() {
        let codes: Vec<u8> = [
            RoadClass::Track,
            RoadClass::Road,
            RoadClass::Highway,
            RoadClass::Footpath,
        ]
        .iter()
        .map(|c| c.code())
        .collect();
        assert_eq!(codes, vec![1, 2, 3, 4]);
        assert!(
            RoadClass::Footpath.rank() < RoadClass::Track.rank()
                && RoadClass::Road.rank() < RoadClass::Highway.rank()
        );
        assert_eq!(RoadClass::Footpath.max_code(RoadClass::Highway.code()), 3);
        assert_eq!(RoadClass::Road.max_code(RoadClass::Footpath.code()), 2);
        assert_eq!(RoadClass::Track.max_code(0), 1);
        let json = serde_json::to_string(&RoadClass::Footpath).unwrap();
        assert_eq!(json, "\"footpath\"");
    }
}
