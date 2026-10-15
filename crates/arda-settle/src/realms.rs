//! Realms (spec step 6; `logic/06` steps 1–3).
//!
//! 1. Seats are chosen in `seats.rs` (spread, balanced large towns).
//! 2. Allegiance ([`territory`]): every cell swears to the seat cheapest to
//!    reach over the terrain cost surface (one multi-source Dijkstra); ties
//!    go to the earlier seat. This runs right after placement, so the
//!    primate capitals and the city cap (`primacy.rs`) size towns by the
//!    realm they really end up in (review 3.3-8).
//! 3. Borders: crossing a river of order ≥ 3 or a ridge crest carries a
//!    fixed toll, so where one lies within a couple of cells of the
//!    equal-cost line the border snaps onto it.
//! 4. Ranking ([`finish`]): once populations are final, realms are ranked
//!    by their capitals, so realm 1 has the largest; settlements take the
//!    realm of their cell.
//!
//! Every land cell belongs to exactly one realm, so borders are closed.

use crate::cost::{self, CostSurface};
use crate::error::SettleError;
use crate::grid::{filled, Grid, OFFSETS8};
use crate::model::Settlement;
use crate::roads::RoadClass;
use crate::snap;
use crate::tags::Sites;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

/// A unit border edge between two cell corners.
type Segment = ((i64, i64), (i64, i64));

/// Toll for stepping over a river of order ≥ 3, metre-equivalents.
const RIVER_TOLL: u64 = 800;
/// Toll for stepping onto a ridge crest.
const RIDGE_TOLL: u64 = 600;

/// One realm, as written to `society/realms.json`.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Realm {
    /// 1-based id.
    #[serde(with = "crate::ids::string")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub id: u64,
    /// Name, filled by the naming stage.
    pub name: String,
    /// English gloss of the name ("Oakmark").
    pub name_gloss: String,
    /// Seat settlement id.
    #[serde(with = "crate::ids::string")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub seat: u64,
    /// Culture of the seat.
    pub culture: String,
    /// Member settlement ids.
    #[serde(with = "crate::ids::vec_string")]
    #[cfg_attr(feature = "schema", schemars(with = "Vec<String>"))]
    pub settlements: Vec<u64>,
    /// Total population of members.
    pub population: u64,
    /// Land cells (hectares).
    pub land_cells: u64,
    /// Realms sharing a land border.
    #[serde(with = "crate::ids::vec_string")]
    #[cfg_attr(feature = "schema", schemars(with = "Vec<String>"))]
    pub neighbours: Vec<u64>,
    /// Land borders as polylines of `[x_m, y_m]` cell corners.
    pub borders: Vec<Vec<[i64; 2]>>,
}

/// The realm partition.
#[derive(Debug, Clone)]
pub struct Realms {
    /// Realm records.
    pub realms: Vec<Realm>,
    /// Realm id per cell; 0 on water.
    pub map: Vec<u16>,
    /// Per mille of the border that has a river (order ≥ 3) or ridge crest
    /// within three cells and runs on it (a cell beside the border edge is
    /// on or next to the line), after snapping.
    pub natural_border_pm: u32,
    /// The same share before snapping.
    pub raw_border_pm: u32,
    /// Per mille of all land border on a line, after snapping.
    pub natural_all_pm: u32,
    /// The same share over all land cells, as a baseline.
    pub natural_land_pm: u32,
}

/// Ridge crests, thickened by a cell so diagonal steps cannot slip through.
fn ridges(g: &Grid, sites: &Sites) -> Result<Vec<bool>, SettleError> {
    let crest = |i: usize| g.is_land(i) && g.drainage[i] <= 1 && sites.prominence_m[i] >= 40;
    let mut out = filled(g.len(), false, "ridge mask")?;
    for (i, o) in out.iter_mut().enumerate() {
        *o = crest(i) || g.neighbours4(i).any(crest);
    }
    Ok(out)
}

/// The land partition before realms are ranked: per land cell the slot
/// (1-based, in the order the seats were given) of the seat it swears to.
#[derive(Debug, Clone)]
pub struct Territory {
    /// Seat slot + 1 per cell; 0 on water.
    pub map: Vec<u16>,
    /// See [`Realms::natural_border_pm`].
    pub natural_border_pm: u32,
    /// See [`Realms::raw_border_pm`].
    pub raw_border_pm: u32,
    /// See [`Realms::natural_all_pm`].
    pub natural_all_pm: u32,
    /// See [`Realms::natural_land_pm`].
    pub natural_land_pm: u32,
}

impl Territory {
    /// Seat slot of every settlement (index into the seats the territory
    /// was drawn for).
    #[must_use]
    pub fn slots(&self, g: &Grid, settlements: &[Settlement]) -> Vec<usize> {
        settlements
            .iter()
            .map(|s| usize::from(self.map[s.index(g.width)].max(1)) - 1)
            .collect()
    }
}

/// Draws the territory of `seats` (settlement indices, in slot order):
/// every land cell swears to the seat cheapest to reach over the terrain
/// cost surface `cs`, ties to the lower slot, and borders snap onto rivers
/// and ridges. This is the final partition: city sizing and the city cap
/// read it before land use and roads exist (review 3.3-8), so it uses no
/// road discount.
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
pub fn territory(
    g: &Grid,
    sites: &Sites,
    cs: &CostSurface,
    seats: &[usize],
    settlements: &[Settlement],
) -> Result<Territory, SettleError> {
    let ridge = ridges(g, sites)?;
    let no_road = |_: usize| false;
    let mut best = filled(g.len(), u64::MAX, "allegiance cost")?;
    let mut map = filled(g.len(), 0_u16, "realm map")?;
    let mut heap = BinaryHeap::new();
    for (slot, &k) in seats.iter().enumerate() {
        let c = settlements[k].index(g.width);
        let id = u16::try_from(slot + 1).unwrap_or(u16::MAX);
        best[c] = 0;
        map[c] = id;
        heap.push(Reverse((0_u64, id, c)));
    }
    while let Some(Reverse((d, id, a))) = heap.pop() {
        if d > best[a] || map[a] != id {
            continue;
        }
        let (ax, ay) = g.xy(a);
        for &(dx, dy) in &OFFSETS8 {
            let Some(b) = g.at(ax + dx, ay + dy) else {
                continue;
            };
            let Some(mut c) = cost::step(g, cs, &no_road, a, b, (dx, dy), RoadClass::Road) else {
                continue;
            };
            if g.is_watercourse(b) && g.order[b] >= 3 && !g.is_watercourse(a) {
                c += RIVER_TOLL;
            }
            if ridge[b] && !ridge[a] {
                c += RIDGE_TOLL;
            }
            let nd = d + c;
            if nd < best[b] || (nd == best[b] && id < map[b]) {
                best[b] = nd;
                map[b] = id;
                heap.push(Reverse((nd, id, b)));
            }
        }
    }
    drop(best);
    for (i, m) in map.iter_mut().enumerate() {
        if !g.is_land(i) {
            *m = 0;
        }
    }
    let line: Vec<bool> = (0..g.len())
        .map(|i| ridge[i] || (g.is_watercourse(i) && g.order[i] >= 3))
        .collect();
    let (raw_border_pm, _) = snap::on_line_share(g, &map, &line)?;
    let seat_cells: Vec<usize> = seats
        .iter()
        .map(|&k| settlements[k].index(g.width))
        .collect();
    let raw_map = map.clone();
    snap::snap(g, &mut map, &line, &seat_cells)?;
    let (mut natural_border_pm, mut natural_all_pm) = snap::on_line_share(g, &map, &line)?;
    // Snapping must never pull a border off a line it already ran on: when
    // the repaired partition is worse than the raw one, the raw one stands.
    if natural_border_pm < raw_border_pm {
        map = raw_map;
        (natural_border_pm, natural_all_pm) = snap::on_line_share(g, &map, &line)?;
    }
    let natural_land_pm = natural_land(g, &map, &line);
    Ok(Territory {
        map,
        natural_border_pm,
        raw_border_pm,
        natural_all_pm,
        natural_land_pm,
    })
}

/// Ranks the realms of a territory by their seats' final population
/// (realm 1 has the largest capital, ties by settlement id), relabels the
/// map, sets every settlement's `realm_id` and builds the records.
#[must_use]
pub fn finish(
    g: &Grid,
    territory: Territory,
    seats: &[usize],
    settlements: &mut [Settlement],
) -> Realms {
    let mut order: Vec<usize> = (0..seats.len()).collect();
    order.sort_by(|&a, &b| {
        let (sa, sb) = (&settlements[seats[a]], &settlements[seats[b]]);
        sb.population.cmp(&sa.population).then(sa.id.cmp(&sb.id))
    });
    // `relabel[slot + 1]` is the realm id of that slot.
    let mut relabel = vec![0_u16; seats.len() + 1];
    for (rank, &slot) in order.iter().enumerate() {
        relabel[slot + 1] = u16::try_from(rank + 1).unwrap_or(u16::MAX);
    }
    let Territory {
        mut map,
        natural_border_pm,
        raw_border_pm,
        natural_all_pm,
        natural_land_pm,
    } = territory;
    for m in &mut map {
        *m = relabel.get(usize::from(*m)).copied().unwrap_or(0);
    }
    for s in settlements.iter_mut() {
        s.realm_id = arda_ids::RealmId(u64::from(map[s.index(g.width)]));
    }
    let ranked: Vec<usize> = order.iter().map(|&slot| seats[slot]).collect();
    let realms = records(g, &map, &ranked, settlements);
    Realms {
        realms,
        map,
        natural_border_pm,
        raw_border_pm,
        natural_all_pm,
        natural_land_pm,
    }
}

/// Baseline for the border share: per mille of land within two cells of a
/// river or ridge line.
fn natural_land(g: &Grid, map: &[u16], line: &[bool]) -> u32 {
    let near = |i: usize| {
        let (x, y) = g.xy(i);
        (-2..=2_i64).any(|oy| (-2..=2_i64).any(|ox| g.at(x + ox, y + oy).is_some_and(|j| line[j])))
    };
    let (mut land, mut land_near) = (0_u64, 0_u64);
    for i in (0..g.len()).filter(|&i| map[i] != 0) {
        land += 1;
        land_near += u64::from(near(i));
    }
    u32::try_from(land_near * 1000 / land.max(1)).unwrap_or(0)
}

fn records(g: &Grid, map: &[u16], seats: &[usize], settlements: &[Settlement]) -> Vec<Realm> {
    let mut realms: Vec<Realm> = seats
        .iter()
        .enumerate()
        .map(|(r, &k)| Realm {
            id: u64::try_from(r + 1).unwrap_or(0),
            name: String::new(),
            name_gloss: String::new(),
            seat: settlements[k].id.get(),
            culture: settlements[k].culture.clone(),
            settlements: Vec::new(),
            population: 0,
            land_cells: 0,
            neighbours: Vec::new(),
            borders: Vec::new(),
        })
        .collect();
    for s in settlements {
        if let Some(r) = realms.get_mut(
            usize::try_from(s.realm_id.get())
                .unwrap_or(0)
                .wrapping_sub(1),
        ) {
            r.settlements.push(s.id.get());
            r.population += u64::from(s.population);
        }
    }
    for &m in map {
        if m > 0 {
            if let Some(r) = realms.get_mut(usize::from(m) - 1) {
                r.land_cells += 1;
            }
        }
    }
    // Unit border edges between land cells of different realms, keyed by
    // realm pair, as corner-to-corner segments.
    let mut edges: BTreeMap<(u16, u16), Vec<Segment>> = BTreeMap::new();
    for i in 0..g.len() {
        let a = map[i];
        if a == 0 {
            continue;
        }
        let (x, y) = g.xy(i);
        for (dx, dy) in [(1_i64, 0_i64), (0, 1)] {
            let Some(j) = g.at(x + dx, y + dy) else {
                continue;
            };
            let b = map[j];
            if b == 0 || b == a {
                continue;
            }
            let seg = if dx == 1 {
                ((x + 1, y), (x + 1, y + 1))
            } else {
                ((x, y + 1), (x + 1, y + 1))
            };
            edges.entry((a.min(b), a.max(b))).or_default().push(seg);
        }
    }
    for ((a, b), segs) in edges {
        let lines = chain(&segs);
        for (r, other) in [(a, b), (b, a)] {
            if let Some(realm) = realms.get_mut(usize::from(r) - 1) {
                realm.neighbours.push(u64::from(other));
                realm.borders.extend(lines.iter().cloned());
            }
        }
    }
    realms
}

/// Chains unit segments into polylines (metres), deterministically.
fn chain(segs: &[Segment]) -> Vec<Vec<[i64; 2]>> {
    let mut at: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
    for (k, &(p, q)) in segs.iter().enumerate() {
        at.entry(p).or_default().push(k);
        at.entry(q).or_default().push(k);
    }
    let mut used = BTreeSet::new();
    let mut out = Vec::new();
    // Start from dead ends first so open borders come out whole.
    let mut starts: Vec<(i64, i64)> = at
        .iter()
        .filter(|(_, v)| v.len() != 2)
        .map(|(&p, _)| p)
        .collect();
    starts.extend(at.keys().copied());
    for start in starts {
        while let Some(&k) = at
            .get(&start)
            .and_then(|v| v.iter().find(|k| !used.contains(*k)))
        {
            let mut line = vec![start];
            let mut cur = start;
            let mut next = Some(k);
            while let Some(k) = next {
                used.insert(k);
                let (p, q) = segs[k];
                cur = if p == cur { q } else { p };
                line.push(cur);
                next = at
                    .get(&cur)
                    .and_then(|v| v.iter().find(|k| !used.contains(*k)).copied());
            }
            out.push(simplify(&line));
        }
    }
    out
}

fn simplify(line: &[(i64, i64)]) -> Vec<[i64; 2]> {
    let mut out = Vec::new();
    for (k, &(x, y)) in line.iter().enumerate() {
        let keep = k == 0
            || k + 1 == line.len()
            || (x - line[k - 1].0, y - line[k - 1].1) != (line[k + 1].0 - x, line[k + 1].1 - y);
        if keep {
            out.push([x * 100, y * 100]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chaining_joins_segments() {
        let segs = [((0, 0), (0, 1)), ((0, 1), (0, 2)), ((0, 2), (1, 2))];
        let lines = chain(&segs);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], vec![[0, 0], [0, 200], [100, 200]]);
    }
}
