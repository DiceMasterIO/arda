//! The realm partition, balanced (`logic/08` §realm-seats step 3,
//! Invariants 5a; OI-53).
//!
//! Every land cell swears to the seat cheapest to reach over the terrain
//! cost surface. Seats are towns, and towns rarely stand where an even
//! split of the land would put them: on a land four times MICRO's the
//! plain flood gave realms from 17 to 111 per mille of the people. So each
//! seat starts the flood with an offset, a handicap for a realm that holds
//! more than its share and a head start for one that holds less (an
//! additively weighted partition), and the offsets are adjusted a few
//! rounds until the shares come within bounds.
//!
//! An additive offset keeps every realm one connected piece around its
//! seat: a cell on the cheapest way from a seat to a cell it owns is owned
//! by it too. A round in which a seat would lose its own cell is undone
//! and the step halved.
//!
//! Realms balance within their landmass: realms joined by land borders
//! form a group (a continent, a large island) and share that group's people
//! and land; a realm alone on its island cannot grow past its shore and
//! keeps a zero offset.

use crate::cost::{self, CostSurface};
use crate::error::SettleError;
use crate::grid::{filled, Grid, OFFSETS8};
use crate::model::Settlement;
use crate::roads::RoadClass;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Toll for stepping over a river of order ≥ 3, metre-equivalents.
pub const RIVER_TOLL: u64 = 800;
/// Toll for stepping onto a ridge crest.
pub const RIDGE_TOLL: u64 = 600;
/// Balancing rounds at most (each is one flood of the whole grid; a
/// 250 × 500 km world converges in about 24).
pub const ROUNDS: usize = 24;
/// Weight of land against people in a realm's share, in quarters, as the
/// seat choice weighs them (`seats.rs`).
const LAND_WEIGHT: i128 = 1;
/// Stop once the largest realm holds at most this many times the
/// smallest's people (per cent; Invariant 5a allows 250)...
const PEOPLE_RATIO_PCT: u64 = 200;
/// ...and land (Invariant 5a allows 450)...
const LAND_RATIO_PCT: u64 = 350;
/// ...and no realm holds more than this share of its fair land (per cent;
/// Invariant 5a allows 200).
const FAIR_LAND_PCT: u64 = 175;

/// Owner (seat slot + 1) of every cell and its cost, from a flood whose
/// seats start at `offsets`; ties go to the lower slot.
fn flood(
    g: &Grid,
    cs: &CostSurface,
    ridge: &[bool],
    seats: &[usize],
    offsets: &[u64],
) -> Result<(Vec<u16>, Vec<u64>), SettleError> {
    let mut best = filled(g.len(), u64::MAX, "allegiance cost")?;
    let mut map = filled(g.len(), 0_u16, "realm map")?;
    let mut heap = BinaryHeap::new();
    for (slot, (&c, &o)) in seats.iter().zip(offsets).enumerate() {
        let id = u16::try_from(slot + 1).unwrap_or(u16::MAX);
        if o < best[c] {
            best[c] = o;
            map[c] = id;
            heap.push(Reverse((o, id, c)));
        }
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
            let Some(mut c) = cost::travel(g, cs, a, b, (dx, dy), RoadClass::Road) else {
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
    Ok((map, best))
}

/// Land cells and people per slot.
#[must_use]
pub fn tally(g: &Grid, map: &[u16], settlements: &[Settlement], n: usize) -> (Vec<u64>, Vec<u64>) {
    let mut land = vec![0_u64; n];
    let mut people = vec![0_u64; n];
    for (i, &m) in map.iter().enumerate() {
        if m > 0 && g.is_land(i) {
            if let Some(l) = land.get_mut(usize::from(m) - 1) {
                *l += 1;
            }
        }
    }
    for s in settlements {
        let m = map[s.index(g.width)];
        if let Some(p) = people.get_mut(usize::from(m).wrapping_sub(1)) {
            *p += u64::from(s.population);
        }
    }
    (land, people)
}

/// Each slot's weighted share (land and people per million, weighted by
/// [`LAND_WEIGHT`] in quarters), so the fair share is 4,000,000 / n.
fn weights(land: &[u64], people: &[u64]) -> Vec<i128> {
    let lt = land.iter().sum::<u64>().max(1);
    let pt = people.iter().sum::<u64>().max(1);
    land.iter()
        .zip(people)
        .map(|(&l, &p)| {
            LAND_WEIGHT * i128::from(l * 1_000_000 / lt)
                + (4 - LAND_WEIGHT) * i128::from(p * 1_000_000 / pt)
        })
        .collect()
}

fn imbalance(w: &[i128]) -> u128 {
    let fair = 4_000_000 / i128::try_from(w.len().max(1)).unwrap_or(1);
    w.iter()
        .map(|&v| u128::try_from((v - fair) * (v - fair)).unwrap_or(u128::MAX))
        .sum()
}

/// Whether the shares are within the stopping bounds.
fn settled(land: &[u64], people: &[u64]) -> bool {
    let ratio_ok = |v: &[u64], pct: u64| {
        let hi = v.iter().copied().max().unwrap_or(0);
        let lo = v.iter().copied().min().unwrap_or(0);
        hi * 100 <= lo.saturating_mul(pct)
    };
    let n = u64::try_from(land.len().max(1)).unwrap_or(1);
    let fair_ok = land
        .iter()
        .all(|&l| l * n * 100 <= land.iter().sum::<u64>() * FAIR_LAND_PCT);
    ratio_ok(people, PEOPLE_RATIO_PCT) && ratio_ok(land, LAND_RATIO_PCT) && fair_ok
}

/// One group of realms joined by land borders: its slots, its offsets'
/// step and its best imbalance so far.
struct Group {
    slots: Vec<usize>,
    step: i128,
    score: u128,
    done: bool,
}

impl Group {
    /// The group's weighted shares and whether it is within bounds.
    fn measure(&self, land: &[u64], people: &[u64]) -> (Vec<i128>, bool) {
        let l: Vec<u64> = self.slots.iter().map(|&k| land[k]).collect();
        let p: Vec<u64> = self.slots.iter().map(|&k| people[k]).collect();
        (weights(&l, &p), settled(&l, &p))
    }

    /// Offsets one step on from `offsets`, given the shares `w`; the
    /// group's least offset is zero.
    fn advance(&self, offsets: &[u64], w: &[i128]) -> Vec<u64> {
        let fair = 4_000_000 / i128::try_from(self.slots.len().max(1)).unwrap_or(1);
        let moved: Vec<i128> = self
            .slots
            .iter()
            .zip(w)
            .map(|(&k, &v)| i128::from(offsets[k]) + self.step * (v - fair) / fair)
            .collect();
        let low = moved.iter().copied().min().unwrap_or(0);
        moved
            .iter()
            .map(|&t| u64::try_from(t - low).unwrap_or(u64::MAX))
            .collect()
    }
}

/// The balanced partition of `seats` (cells, in slot order): owner slot + 1
/// per cell (water included; the caller clears it).
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
pub fn partition(
    g: &Grid,
    cs: &CostSurface,
    ridge: &[bool],
    seats: &[usize],
    settlements: &[Settlement],
) -> Result<Vec<u16>, SettleError> {
    let n = seats.len();
    let mut offsets = vec![0_u64; n];
    let (first, best) = flood(g, cs, ridge, seats, &offsets)?;
    // The step is the mean cost from a land cell to its seat: a realm's
    // reach, in the flood's own units.
    let (sum, count) = (0..g.len())
        .filter(|&i| g.is_land(i) && best[i] != u64::MAX)
        .fold((0_u128, 0_u128), |(s, c), i| {
            (s + u128::from(best[i]), c + 1)
        });
    drop(best);
    let step = i128::try_from(sum / count.max(1)).unwrap_or(0);
    // Steps below a 64th of the first move borders by a cell or two.
    let floor = (step / 64).max(1);
    let (land, people) = tally(g, &first, settlements, n);
    let mut groups: Vec<Group> = groups(g, &first, n)
        .into_iter()
        .filter(|slots| slots.len() >= 2)
        .map(|slots| Group {
            slots,
            step,
            score: 0,
            done: false,
        })
        .collect();
    let mut shares = Vec::with_capacity(groups.len());
    for gr in &mut groups {
        let (w, done) = gr.measure(&land, &people);
        gr.score = imbalance(&w);
        gr.done = done;
        shares.push(w);
    }
    let mut moved = false;
    for _ in 0..ROUNDS {
        if groups.iter().all(|gr| gr.done || gr.step < floor) {
            break;
        }
        let mut trial = offsets.clone();
        for (gr, w) in groups.iter().zip(&shares) {
            if !(gr.done || gr.step < floor) {
                for (&k, o) in gr.slots.iter().zip(gr.advance(&offsets, w)) {
                    trial[k] = o;
                }
            }
        }
        let (map, _) = flood(g, cs, ridge, seats, &trial)?;
        let (land, people) = tally(g, &map, settlements, n);
        for (gr, w) in groups.iter_mut().zip(shares.iter_mut()) {
            if gr.done || gr.step < floor {
                continue;
            }
            let kept = gr
                .slots
                .iter()
                .all(|&k| usize::from(map[seats[k]]) == k + 1);
            let (tw, done) = gr.measure(&land, &people);
            let ts = imbalance(&tw);
            if kept && ts < gr.score {
                for &k in &gr.slots {
                    offsets[k] = trial[k];
                }
                gr.score = ts;
                gr.done = done;
                *w = tw;
                moved = true;
                // A step that helped may be too timid: grow it.
                gr.step = gr.step * 3 / 2;
            } else {
                gr.step /= 2;
            }
        }
    }
    if !moved {
        return Ok(first);
    }
    // The groups were judged on floods that also moved the others; draw the
    // partition of the offsets kept, which every seat must still hold.
    let (map, _) = flood(g, cs, ridge, seats, &offsets)?;
    let kept = seats
        .iter()
        .enumerate()
        .all(|(slot, &c)| usize::from(map[c]) == slot + 1);
    Ok(if kept { map } else { first })
}

/// Groups of slots whose realms are joined by land borders, each in slot
/// order, ordered by their first slot.
#[must_use]
pub fn groups(g: &Grid, map: &[u16], n: usize) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(parent: &mut [usize], mut k: usize) -> usize {
        while parent[k] != k {
            parent[k] = parent[parent[k]];
            k = parent[k];
        }
        k
    }
    for i in (0..g.len()).filter(|&i| g.is_land(i)) {
        let (x, y) = g.xy(i);
        for (dx, dy) in [(1_i64, 0_i64), (0, 1)] {
            let Some(j) = g.at(x + dx, y + dy) else {
                continue;
            };
            let (a, b) = (usize::from(map[i]), usize::from(map[j]));
            if g.is_land(j) && a != b && a > 0 && b > 0 && a <= n && b <= n {
                let (ra, rb) = (root(&mut parent, a - 1), root(&mut parent, b - 1));
                if ra != rb {
                    parent[ra.max(rb)] = ra.min(rb);
                }
            }
        }
    }
    let mut out: Vec<Vec<usize>> = Vec::new();
    let mut at = vec![usize::MAX; n];
    for k in 0..n {
        let r = root(&mut parent, k);
        if at[r] == usize::MAX {
            at[r] = out.len();
            out.push(Vec::new());
        }
        out[at[r]].push(k);
    }
    out
}

/// The slot of the realm with the fewest people of any group of realms
/// joined by land, when the largest realm of that group holds more than
/// 2.5 times its people (logic/08 Invariants 5a): a seat the partition
/// cannot give a fair share, because it stands too near larger neighbours
/// or on a narrow peninsula. `None` when every group is within the bound.
#[must_use]
pub fn starved(g: &Grid, map: &[u16], settlements: &[Settlement], n: usize) -> Option<usize> {
    let (_, people) = tally(g, map, settlements, n);
    groups(g, map, n)
        .into_iter()
        .filter(|slots| slots.len() >= 2)
        .filter_map(|slots| {
            let most = slots.iter().map(|&k| people[k]).max()?;
            let least = slots.iter().copied().min_by_key(|&k| (people[k], k))?;
            (people[least] * 5 < most * 2).then_some((people[least], least))
        })
        .min()
        .map(|(_, k)| k)
}
