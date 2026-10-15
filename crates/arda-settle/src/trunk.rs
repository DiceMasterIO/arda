//! The trunk of the road hierarchy (spec §roads, artifact "Roads"): the
//! highways that join the towns, and the highways pushed from the nearest
//! town to wherever the land runs off the map.

use crate::cost::CostSurface;
use crate::grid::Grid;
use crate::model::Settlement;
use crate::num::{dist_m, ui};
use crate::roads::{ui_of, End, Network, RoadClass};
use crate::route;

pub(crate) fn connect(
    g: &Grid,
    cs: &CostSurface,
    net: &mut Network,
    a: &Settlement,
    b: &Settlement,
    class: RoadClass,
) {
    let path = {
        let on = net.on_road();
        route::find(g, cs, &on, a.index(g.width), b.index(g.width), class)
    };
    match path {
        Some(p) if p.cells.len() > 1 => {
            net.commit(g, &p, class, a.id.get(), End::Settlement(b.id.get()));
        }
        Some(_) => {}
        None => net.unreachable.push(a.id.get()),
    }
}

/// Minimum spanning tree over least-cost routes to each town's four nearest
/// towns (plus bridging pairs if that graph falls apart), then routed
/// cheapest first so later trunks share earlier ones.
pub fn trunk(g: &Grid, cs: &CostSurface, net: &mut Network, hubs: &[&Settlement]) {
    let n = hubs.len();
    let d = |i: usize, j: usize| straight_m(hubs[i], hubs[j]);
    let pairs = neighbour_pairs(hubs);
    let none = |_: usize| false;
    let mut edges: Vec<(u64, usize, usize)> = pairs
        .iter()
        .filter_map(|&(i, j)| {
            route::find(
                g,
                cs,
                &none,
                hubs[i].index(g.width),
                hubs[j].index(g.width),
                RoadClass::Highway,
            )
            .map(|p| (p.cost, i, j))
        })
        .collect();
    edges.sort_unstable();
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    let mut tree = Vec::new();
    for &(c, i, j) in &edges {
        let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
        if ri != rj {
            parent[ri] = rj;
            tree.push((c, i, j));
        }
    }
    // Bridge any components the nearest-neighbour graph left apart.
    loop {
        let mut best: Option<(u32, usize, usize)> = None;
        for i in 0..n {
            for j in (i + 1)..n {
                if root(&mut parent, i) != root(&mut parent, j)
                    && best.is_none_or(|(bd, _, _)| d(i, j) < bd)
                {
                    best = Some((d(i, j), i, j));
                }
            }
        }
        let Some((_, i, j)) = best else { break };
        let (ri, rj) = (root(&mut parent, i), root(&mut parent, j));
        parent[ri] = rj;
        tree.push((u64::MAX, i, j));
    }
    for (_, i, j) in tree {
        connect(g, cs, net, hubs[i], hubs[j], RoadClass::Highway);
    }
}

fn straight_m(a: &Settlement, b: &Settlement) -> u32 {
    dist_m(
        ui_of(a.cell_x) - ui_of(b.cell_x),
        ui_of(a.cell_y) - ui_of(b.cell_y),
    )
}

/// Each town paired with its four nearest towns in a straight line, as
/// sorted, deduplicated index pairs.
fn neighbour_pairs(hubs: &[&Settlement]) -> Vec<(usize, usize)> {
    let n = hubs.len();
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for i in 0..n {
        let mut by: Vec<usize> = (0..n).filter(|&j| j != i).collect();
        by.sort_by_key(|&j| (straight_m(hubs[i], hubs[j]), j));
        for &j in by.iter().take(4) {
            pairs.push((i.min(j), i.max(j)));
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    pairs
}

/// Neighbouring towns whose way by road runs more than this many per cent
/// of their straight distance get a direct highway (assumed, tunable;
/// review 3.3-6).
pub const LOOP_RATIO_PCT: u64 = 200;
/// ... when that highway is itself no longer than this many per cent of the
/// straight distance (a link round a fjord closes nothing).
pub const LOOP_LINK_PCT: u64 = 160;

/// Closes the trunk's loops: a spanning tree joins every town but leaves
/// neighbours a long way round (towns 54 km apart and 179 km by road on
/// MICRO). Each pair of neighbouring towns, nearest first, whose way over
/// the network runs beyond [`LOOP_RATIO_PCT`] of the straight line gets a
/// direct highway of its own, kept when it is within [`LOOP_LINK_PCT`].
pub fn close_loops(g: &Grid, cs: &CostSurface, net: &mut Network, hubs: &[&Settlement]) {
    let mut pairs = neighbour_pairs(hubs);
    pairs.sort_by_key(|&(i, j)| (straight_m(hubs[i], hubs[j]), i, j));
    let none = |_: usize| false;
    for (i, j) in pairs {
        let (a, b) = (hubs[i], hubs[j]);
        let straight = u64::from(straight_m(a, b));
        let (ca, cb) = (a.index(g.width), b.index(g.width));
        if within_by_road(g, net, ca, cb, straight * LOOP_RATIO_PCT / 100) {
            continue;
        }
        let Some(p) = route::find(g, cs, &none, ca, cb, RoadClass::Highway) else {
            continue;
        };
        if p.cells.len() > 1 && route::length_m(g, &p.cells) * 100 <= straight * LOOP_LINK_PCT {
            net.commit(
                g,
                &p,
                RoadClass::Highway,
                a.id.get(),
                End::Settlement(b.id.get()),
            );
        }
    }
}

/// Whether `b` lies within `limit` metres of `a` over road and ferry cells
/// (A*, octile lengths, cut off at `limit`).
fn within_by_road(g: &Grid, net: &Network, a: usize, b: usize, limit: u64) -> bool {
    use std::cmp::Reverse;
    use std::collections::{BTreeMap, BinaryHeap};
    if !net.carries(a) || !net.carries(b) {
        return false;
    }
    let (bx, by) = g.xy(b);
    let h = |i: usize| {
        let (x, y) = g.xy(i);
        let (dx, dy) = ((x - bx).unsigned_abs(), (y - by).unsigned_abs());
        let (lo, hi) = (dx.min(dy), dx.max(dy));
        lo * 141 + (hi - lo) * 100
    };
    let mut best: BTreeMap<usize, u64> = BTreeMap::from([(a, 0)]);
    let mut heap = BinaryHeap::from([Reverse((h(a), 0_u64, a))]);
    while let Some(Reverse((f, d, i))) = heap.pop() {
        if f > limit {
            return false;
        }
        if i == b {
            return true;
        }
        if best.get(&i).is_some_and(|&v| d > v) {
            continue;
        }
        for (j, dx, dy) in g.neighbours8(i) {
            if !net.carries(j) {
                continue;
            }
            let nd = d + if dx != 0 && dy != 0 { 141 } else { 100 };
            if best.get(&j).is_none_or(|&v| nd < v) {
                best.insert(j, nd);
                heap.push(Reverse((nd + h(j), nd, j)));
            }
        }
    }
    false
}

/// Pushes a highway from the nearest town to the middle of every long run
/// of land along each map edge (where the region's roads leave the map).
pub fn edges(g: &Grid, cs: &CostSurface, net: &mut Network, hubs: &[&Settlement]) {
    let (w, h) = (ui(g.width), ui(g.height));
    let sides: [(&'static str, Vec<(i64, i64)>); 4] = [
        ("north", (0..w).map(|x| (x, 0)).collect()),
        ("east", (0..h).map(|y| (w - 1, y)).collect()),
        ("south", (0..w).map(|x| (x, h - 1)).collect()),
        ("west", (0..h).map(|y| (0, y)).collect()),
    ];
    for (name, cells) in sides {
        let mut runs: Vec<(usize, usize)> = Vec::new();
        let mut start = None;
        for (k, &(x, y)) in cells.iter().enumerate() {
            let land = g.at(x, y).is_some_and(|i| g.is_land(i));
            match (land, start) {
                (true, None) => start = Some(k),
                (false, Some(s)) => {
                    runs.push((s, k));
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(s) = start {
            runs.push((s, cells.len()));
        }
        for (s, e) in runs.into_iter().filter(|(s, e)| e - s >= 100) {
            let mid = cells[(s + e) / 2];
            let Some(exit) = g.at(mid.0, mid.1) else {
                continue;
            };
            let Some(from) = hubs.iter().min_by_key(|t| {
                (
                    dist_m(ui_of(t.cell_x) - mid.0, ui_of(t.cell_y) - mid.1),
                    t.id,
                )
            }) else {
                continue;
            };
            let path = {
                let on = net.on_road();
                route::find(g, cs, &on, from.index(g.width), exit, RoadClass::Highway)
            };
            if let Some(p) = path.filter(|p| p.cells.len() > 1) {
                net.commit(g, &p, RoadClass::Highway, from.id.get(), End::Edge(name));
            }
        }
    }
}
