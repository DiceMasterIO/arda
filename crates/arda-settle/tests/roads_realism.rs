//! The road network closes loops, footpaths are real paths, and road steps
//! respect their class's grade (review 3.3-6, 3.3-7; logic/08 §roads).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use arda_settle::cost::{max_grade_pct, CostSurface};
use arda_settle::grid::Grid;
use arda_settle::model::{Settlement, Tier};
use arda_settle::num::dist_m;
use arda_settle::roads::RoadClass;
use arda_settle::{route, run, synthetic, PlaceParams, Society};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::sync::LazyLock;

static WORLD: LazyLock<(Grid, Society)> = LazyLock::new(|| {
    let params = PlaceParams {
        seed: 42,
        density_per_km2: 15,
    };
    let g = synthetic::landscape(params.seed).unwrap();
    let s = run(&g, params).unwrap();
    (g, s)
});

/// Shortest distance over road cells from `a` to `b`, metres.
fn by_road(g: &Grid, raster: &[u8], a: usize, b: usize) -> Option<u64> {
    let mut dist = vec![u64::MAX; g.len()];
    let mut heap = BinaryHeap::from([Reverse((0_u64, a))]);
    dist[a] = 0;
    while let Some(Reverse((d, i))) = heap.pop() {
        if i == b {
            return Some(d);
        }
        if d > dist[i] {
            continue;
        }
        for (j, dx, dy) in g.neighbours8(i) {
            if raster[j] == 0 {
                continue;
            }
            let nd = d + if dx != 0 && dy != 0 { 141 } else { 100 };
            if nd < dist[j] {
                dist[j] = nd;
                heap.push(Reverse((nd, j)));
            }
        }
    }
    None
}

#[test]
fn every_footpath_adds_a_path_of_its_own() {
    let (_, s) = &*WORLD;
    let paths: Vec<_> = s
        .network
        .roads
        .iter()
        .filter(|r| r.class == RoadClass::Footpath)
        .collect();
    for r in &paths {
        assert!(
            !r.segments.is_empty() && r.new_m > 0,
            "footpath {} adds nothing",
            r.id
        );
    }
}

#[test]
fn neighbouring_towns_are_not_joined_only_by_a_long_way_round() {
    let (g, s) = &*WORLD;
    let towns: Vec<_> = s.settlements.iter().filter(|x| x.tier.is_urban()).collect();
    assert!(towns.len() >= 4);
    let cs = CostSurface {
        base: vec![100; g.len()],
    };
    let none = |_: usize| false;
    for a in &towns {
        let mut near: Vec<_> = towns.iter().filter(|b| b.id != a.id).collect();
        let d = |b: &&&arda_settle::model::Settlement| {
            dist_m(
                i64::from(a.cell_x) - i64::from(b.cell_x),
                i64::from(a.cell_y) - i64::from(b.cell_y),
            )
        };
        near.sort_by_key(|b| (d(b), b.id));
        for b in near.into_iter().take(3) {
            let straight = u64::from(d(&b));
            let (ia, ib) = (a.index(g.width), b.index(g.width));
            let road = by_road(g, &s.network.raster, ia, ib).unwrap();
            // A direct way that is itself a long way round (a fjord, a
            // lake) excuses the network.
            let direct = route::find(g, &cs, &none, ia, ib, RoadClass::Highway)
                .map_or(u64::MAX, |p| route::length_m(g, &p.cells));
            assert!(
                road * 100 <= straight * 200 || direct * 100 > straight * 180,
                "{} to {}: {road} m by road for {straight} m apart (direct {direct} m)",
                a.id,
                b.id
            );
        }
    }
}

/// Open grass, every cell a land cell at `h(x, y)` millimetres.
fn open(w: usize, h: usize, height: impl Fn(i64, i64) -> i64) -> (Grid, CostSurface) {
    let mut g = Grid::sea(w, h, arda_settle::grid::MEMORY_BUDGET).unwrap();
    for i in 0..g.len() {
        let (x, y) = g.xy(i);
        g.terrain[i] = arda::TerrainKind::Land;
        g.cover[i] = arda::Cover::Grass;
        g.height_mm[i] = i32::try_from(height(x, y)).unwrap();
    }
    let cs = CostSurface {
        base: vec![100; g.len()],
    };
    (g, cs)
}

#[test]
fn four_towns_on_a_square_are_not_joined_by_a_spanning_tree_alone() {
    // Towns on the corners of a 10 km square: a spanning tree leaves one
    // pair 30 km apart by road.
    let (g, cs) = open(140, 140, |_, _| 10_000);
    let corners = [(20, 20), (120, 20), (120, 120), (20, 120)];
    let towns: Vec<Settlement> = corners
        .iter()
        .enumerate()
        .map(|(k, &(x, y))| Settlement::bare(u64::try_from(k + 1).unwrap(), x, y, Tier::Town, 3000))
        .collect();
    let net = arda_settle::roads::build(&g, &cs, &towns);
    for (k, a) in towns.iter().enumerate() {
        let b = &towns[(k + 1) % 4];
        let road = by_road(&g, &net.raster, a.index(g.width), b.index(g.width)).unwrap();
        assert!(
            road <= 10_000 * 3 / 2,
            "{} to {}: {road} m by road for 10 km",
            a.id,
            b.id
        );
    }
}

#[test]
fn a_highway_climbs_a_ten_per_cent_slope_across_it_not_straight_up() {
    // A 10 % grade to the south: straight up is over a highway's 8 %, a
    // diagonal across the slope (7.1 %) is within it.
    let (g, cs) = open(80, 80, |_, y| y * 10_000);
    let none = |_: usize| false;
    let (a, b) = (g.at(40, 10).unwrap(), g.at(40, 50).unwrap());
    let p = route::find(&g, &cs, &none, a, b, RoadClass::Highway).unwrap();
    let max = max_grade_pct(RoadClass::Highway);
    let over = p
        .cells
        .windows(2)
        .filter(|w| {
            let (ax, ay) = g.xy(w[0]);
            let (bx, by) = g.xy(w[1]);
            let len = if ax != bx && ay != by { 141 } else { 100 };
            let rise = i64::from((g.height_mm[w[1]] - g.height_mm[w[0]]).abs());
            rise * 100 > max * len * 1000
        })
        .count();
    assert!(
        over * 10 <= p.cells.len(),
        "{over} of {} steps over 8 %",
        p.cells.len()
    );
}
