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
/// Balancing rounds at most (each is one flood of the whole grid).
pub const ROUNDS: usize = 10;
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
fn tally(g: &Grid, map: &[u16], settlements: &[Settlement], n: usize) -> (Vec<u64>, Vec<u64>) {
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
    let (map, best) = flood(g, cs, ridge, seats, &offsets)?;
    if n < 2 {
        return Ok(map);
    }
    // The step is the mean cost from a land cell to its seat: a realm's
    // reach, in the flood's own units.
    let (sum, count) = (0..g.len())
        .filter(|&i| g.is_land(i) && best[i] != u64::MAX)
        .fold((0_u128, 0_u128), |(s, c), i| {
            (s + u128::from(best[i]), c + 1)
        });
    drop(best);
    let mut step = i128::try_from(sum / count.max(1)).unwrap_or(0);
    let (land, people) = tally(g, &map, settlements, n);
    let mut done = settled(&land, &people);
    let mut w = weights(&land, &people);
    let mut score = imbalance(&w);
    let mut best_map = map;
    let fair = 4_000_000 / i128::try_from(n).unwrap_or(1);
    for _ in 0..ROUNDS {
        if done || step == 0 {
            break;
        }
        let mut trial: Vec<i128> = offsets
            .iter()
            .zip(&w)
            .map(|(&o, &v)| i128::from(o) + step * (v - fair) / fair)
            .collect();
        let low = trial.iter().copied().min().unwrap_or(0);
        for t in &mut trial {
            *t -= low;
        }
        let trial: Vec<u64> = trial
            .iter()
            .map(|&t| u64::try_from(t).unwrap_or(u64::MAX))
            .collect();
        let (map, _) = flood(g, cs, ridge, seats, &trial)?;
        let kept = seats
            .iter()
            .enumerate()
            .all(|(slot, &c)| usize::from(map[c]) == slot + 1);
        if !kept {
            step /= 2;
            continue;
        }
        let (land, people) = tally(g, &map, settlements, n);
        let tw = weights(&land, &people);
        let ts = imbalance(&tw);
        if ts < score {
            offsets = trial;
            w = tw;
            score = ts;
            best_map = map;
            done = settled(&land, &people);
        } else {
            step /= 2;
        }
    }
    Ok(best_map)
}
