//! Crossings on a rasterised river ([`Terrain::rivers_rasterised`]).
//!
//! The caller's raster is the river the map shows, so crossings are found
//! on it rather than at the world's crossing records: wherever a way's
//! centreline runs over river water it gets a bridge or a ford, laid across
//! the rows where that water is narrowest nearby and spanning all of it.
//! A world crossing record lends its id and kind to the wet stretch nearest
//! it; stretches no record claims are bridged on highways and roads and
//! forded on tracks and footpaths. Crossings of open water (ferries over
//! sea or lake) keep their records' positions and synthetic channels.
//!
//! Everything is computed from world geometry and the raster, which are
//! pure functions of position, so neighbouring windows lay the same
//! crossings (goal 46).

use super::crossing::{along, finish, plan_one, rows_for, CrossingPlan, Lay};
use super::{is_water, square_of, ChannelPlan, WayPlan, Window};
use crate::curve::{dist, Dense, P};
use crate::input::{Crossing, CrossingKind, RoadClass, Terrain, SQUARE_M};
use arda_tactical::noise::hash2;

/// Wet stretches of a way closer than this are one crossing, metres.
const MERGE_M: f64 = 3.0 * SQUARE_M;
/// Stretches further than this from the window are not laid, metres.
pub const REACH_M: f64 = 150.0;
/// A crossing record lends itself to the wet stretch nearest it within
/// this distance, metres (settle puts records at cell centres).
const ATTACH_M: f64 = 120.0;
/// Stations further than this from the window are not searched, metres:
/// every stretch near a record that can reach a laid stretch is seen.
const SEARCH_M: f64 = super::DENSE_MARGIN_M;
/// Rows the span may move across its axis to find the narrowest water.
const SHIFT_SQ: i64 = 3;
/// How far from the way the water may start, squares.
const SEED_SQ: i64 = 4;
/// Dry squares a span bridges between two stretches of water: at least the
/// straight approach, so no water is left under it.
const GAP_SQ: i64 = 5;
/// Longest water run one crossing spans, squares; longer runs (a way
/// running along the river) are left alone.
const MAX_RUN_SQ: i64 = 96;
/// Set in the id of a crossing no record names.
pub const SYNTHETIC_ID: u32 = 0x4000_0000;
/// Crossings laid per window at most.
const MAX_CROSSINGS: usize = 256;

/// A run of a way's centreline over water.
#[derive(Debug, Clone, Copy)]
struct Stretch {
    way: usize,
    s0: f64,
    s1: f64,
    at: P,
}

/// Plans every crossing that can reach `win`.
pub fn plan_all(
    win: Window,
    crossings: &[Crossing],
    ways: &mut [WayPlan],
    channels: &mut Vec<ChannelPlan>,
    terrain: &dyn Terrain,
) -> (Vec<CrossingPlan>, Vec<u32>) {
    let mut sorted: Vec<&Crossing> = crossings.iter().collect();
    sorted.sort_by_key(|c| c.id);
    let (mut out, mut orphans) = (Vec::new(), Vec::new());
    let mut reserved: Vec<(usize, f64, f64)> = Vec::new();
    #[allow(clippy::cast_precision_loss)] // world metres are far below 2^52
    let pos = |c: &Crossing| [c.x_m as f64, c.y_m as f64];
    let near = |c: &Crossing| win.dist_m(pos(c)) <= f64::from(c.width_m) * 1.5 + REACH_M;
    // Open water first, as before.
    for c in sorted.iter().filter(|c| c.water != "river" && near(c)) {
        match plan_one(c, pos(c), ways, channels, terrain) {
            Some(p) => out.push(p),
            None => orphans.push(c.id),
        }
    }
    reindex(ways);
    let mut used = vec![false; sorted.len()];
    let mut failed: Vec<(usize, i64, i64)> = Vec::new();
    while out.len() < MAX_CROSSINGS {
        let all = stretches(win, ways, channels, terrain);
        let open = all.iter().find(|s| {
            win.dist_m(s.at) <= REACH_M && !overlaps(&reserved, s) && !failed.contains(&key(s))
        });
        let Some(&st) = open else { break };
        // A record goes to the stretch nearest it, not the first laid.
        let rec = attach(&sorted, &used, &st, ways).filter(|&i| {
            let at = pos(sorted[i]);
            let d = dist(at, st.at);
            all.iter().all(|o| dist(at, o.at) >= d)
        });
        let (id, kind) = match rec {
            Some(i) => (sorted[i].id, sorted[i].kind),
            None => (
                synthetic_id(&st, &ways[st.way]),
                default_kind(ways[st.way].class),
            ),
        };
        match lay(id, kind, &st, ways, channels, terrain) {
            Some((plan, arcs)) => {
                if let Some(i) = rec {
                    used[i] = true;
                }
                let (a, b) = arcs.unwrap_or((st.s0, st.s1));
                reserved.push((plan.way, a.min(st.s0), b.max(st.s1)));
                out.push(plan);
                reindex(ways);
            }
            None => failed.push(key(&st)),
        }
    }
    for (i, c) in sorted.iter().enumerate() {
        if c.water == "river" && !used[i] && near(c) {
            orphans.push(c.id);
        }
    }
    orphans.sort_unstable();
    (out, orphans)
}

fn reindex(ways: &mut [WayPlan]) {
    for w in ways.iter_mut() {
        w.dense = Dense::new(std::mem::take(&mut w.dense.runs));
    }
}

fn key(s: &Stretch) -> (usize, i64, i64) {
    (s.way, square_of(s.at[0]), square_of(s.at[1]))
}

fn overlaps(reserved: &[(usize, f64, f64)], s: &Stretch) -> bool {
    reserved
        .iter()
        .any(|&(w, a, b)| w == s.way && s.s0 <= b && s.s1 >= a)
}

/// Every wet stretch of every way whose middle lies within [`SEARCH_M`] of
/// the window, by way and arc length.
fn stretches(
    win: Window,
    ways: &[WayPlan],
    channels: &[ChannelPlan],
    terrain: &dyn Terrain,
) -> Vec<Stretch> {
    let mut out = Vec::new();
    for (wi, w) in ways.iter().enumerate() {
        for run in &w.dense.runs {
            let mut cur: Option<(f64, f64)> = None;
            let mut flush = |cur: Option<(f64, f64)>| {
                if let Some((s0, s1)) = cur {
                    let mid = (s0 + s1) / 2.0;
                    let at = run
                        .iter()
                        .min_by(|a, b| (a.s - mid).abs().total_cmp(&(b.s - mid).abs()))
                        .map_or([0.0, 0.0], |st| st.p);
                    if win.dist_m(at) <= SEARCH_M {
                        out.push(Stretch {
                            way: wi,
                            s0,
                            s1,
                            at,
                        });
                    }
                }
            };
            for st in run {
                if win.dist_m(st.p) > SEARCH_M
                    || !is_water(channels, terrain, square_of(st.p[0]), square_of(st.p[1]))
                {
                    continue;
                }
                match cur.as_mut() {
                    Some(c) if st.s - c.1 <= MERGE_M => c.1 = st.s,
                    _ => {
                        flush(cur);
                        cur = Some((st.s, st.s));
                    }
                }
            }
            flush(cur);
        }
    }
    out
}

/// The unused crossing record nearest the stretch, preferring the way's
/// own class.
fn attach(sorted: &[&Crossing], used: &[bool], st: &Stretch, ways: &[WayPlan]) -> Option<usize> {
    let class = ways[st.way].class;
    sorted
        .iter()
        .enumerate()
        .filter(|(i, c)| !used[*i] && c.water == "river")
        .filter_map(|(i, c)| {
            #[allow(clippy::cast_precision_loss)] // world metres are far below 2^52
            let d = dist([c.x_m as f64, c.y_m as f64], st.at);
            let reach = ATTACH_M + f64::from(c.width_m) * 1.5;
            (d <= reach).then_some((d + if c.road_class == class { 0.0 } else { 60.0 }, i))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|x| x.1)
}

/// Kind of a crossing no record names: roads bridge, tracks ford.
const fn default_kind(class: RoadClass) -> CrossingKind {
    match class {
        RoadClass::Highway | RoadClass::Road => CrossingKind::Bridge,
        RoadClass::Track | RoadClass::Footpath | RoadClass::None => CrossingKind::Ford,
    }
}

/// A stable id for a crossing no record names, above settle's range.
fn synthetic_id(st: &Stretch, way: &WayPlan) -> u32 {
    let (gx, gy) = (square_of(st.at[0]), square_of(st.at[1]));
    let h = hash2(
        (u64::from(way.road_id) << 16) ^ u64::try_from(way.segment).unwrap_or(0),
        gx,
        gy,
    );
    SYNTHETIC_ID | u32::try_from(h & 0x3FFF_FFFF).unwrap_or(0)
}

/// Lays a crossing of `kind` over the stretch: axis across the guiding
/// channel's flow, rows shifted up to [`SHIFT_SQ`] to the narrowest water.
#[allow(clippy::cast_precision_loss)] // small square offsets
fn lay(
    id: u32,
    kind: CrossingKind,
    st: &Stretch,
    ways: &mut [WayPlan],
    channels: &[ChannelPlan],
    terrain: &dyn Terrain,
) -> Option<(CrossingPlan, Option<(f64, f64)>)> {
    let hit = ways[st.way].dense.nearest(st.at, 50.0)?;
    let centre = hit.foot;
    let guide = channels
        .iter()
        .enumerate()
        .filter(|(_, c)| c.guide)
        .filter_map(|(i, c)| c.dense.nearest(centre, 150.0).map(|h| (h.d, i, h.dir)))
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let (channel, flow, width_m) = match guide {
        Some((_, i, dir)) => (i, dir, channels[i].width_m),
        None => (usize::MAX, [-hit.dir[1], hit.dir[0]], 2.0 * SQUARE_M),
    };
    let east_west = flow[1].abs() >= flow[0].abs();
    let mid = square_of(along(east_west, centre));
    let across = if east_west { [0.0, 1.0] } else { [1.0, 0.0] };
    // (water length, rows, water, centre) of the narrowest placing.
    type Placing = (i64, (i64, i64), (i64, i64), P);
    let mut best: Option<Placing> = None;
    for k in (0..=SHIFT_SQ).flat_map(|d| if d == 0 { vec![0] } else { vec![-d, d] }) {
        let off = k as f64 * SQUARE_M;
        let c = [centre[0] + across[0] * off, centre[1] + across[1] * off];
        let rows = rows_for(kind, &ways[st.way], east_west, c);
        let Some(water) = water_run(channels, terrain, east_west, rows, mid) else {
            continue;
        };
        let len = water.1 - water.0;
        if best.is_none_or(|b| len < b.0) {
            best = Some((len, rows, water, c));
        }
    }
    let (_, rows, water, centre) = best?;
    let l = Lay {
        id,
        kind,
        way: st.way,
        channel,
        centre,
        east_west,
        rows,
        water,
        width_m,
    };
    finish(&l, ways, terrain)
}

/// The along-axis squares of the water under `rows` next to `mid`: from the
/// water nearest the way, grown while more water follows within
/// [`GAP_SQ`] squares in any row.
fn water_run(
    channels: &[ChannelPlan],
    terrain: &dyn Terrain,
    east_west: bool,
    rows: (i64, i64),
    mid: i64,
) -> Option<(i64, i64)> {
    let wet = |a: i64| {
        (rows.0..=rows.1).any(|r| {
            let (gx, gy) = if east_west { (a, r) } else { (r, a) };
            is_water(channels, terrain, gx, gy)
        })
    };
    let seed = (0..=SEED_SQ)
        .flat_map(|d| [mid - d, mid + d])
        .find(|&a| wet(a))?;
    let (mut lo, mut hi) = (seed, seed);
    while let Some(a) = (hi + 1..=hi + 1 + GAP_SQ).find(|&a| wet(a)) {
        hi = a;
        if hi - lo > MAX_RUN_SQ {
            return None;
        }
    }
    while let Some(a) = (lo - 1 - GAP_SQ..lo).rev().find(|&a| wet(a)) {
        lo = a;
        if hi - lo > MAX_RUN_SQ {
            return None;
        }
    }
    Some((lo, hi))
}
