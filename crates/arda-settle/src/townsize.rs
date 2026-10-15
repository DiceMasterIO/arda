//! Town sizes from what feeds and draws them (`logic/08` §settle-tiers;
//! review 3.3-12, 3.3-13).
//!
//! A market town lives on its hinterland: the villages and hamlets whose
//! nearest town it is bring their grain, wool and custom to it. A site's
//! advantages draw trade beyond that hinterland: a harbour or a river
//! mouth, a navigable reach, a confluence or a crossing, a pass, a
//! defensible height, ore or salt nearby. So each town's weight is its
//! hinterland's people times its site advantage, and the urban population
//! is shared in proportion to that weight raised to [`AGGLOMERATION`]
//! (larger markets draw more than their share: agglomeration; assumed,
//! tunable). No rank-size list is imposed: the size distribution follows
//! from the land and the sites, and the rank-size check (logic/08
//! Invariants 4) can fail.

use crate::model::{Settlement, Tier, TOWN_MIN};
use crate::num::{dist_m, isqrt};
use crate::tags;

/// Exponent of the weight, as `(numerator, denominator)`: 3/2 (assumed,
/// tunable).
pub const AGGLOMERATION: (u32, u32) = (3, 2);
/// Hinterland every town is credited with beyond the rural people nearest
/// it, so a town on the coast with little land behind it still trades.
pub const BASE_HINTERLAND: u64 = 300;

/// Site advantage in per-mille (1000 = none): each kind of trade node
/// counts once (assumed, tunable).
#[must_use]
pub fn advantage_pm(bits: u32) -> u64 {
    let has = |b: u32| bits & b != 0;
    let mut a = 1000;
    a += if has(tags::HARBOUR) {
        600
    } else if has(tags::ESTUARY) {
        400
    } else {
        0
    };
    a += if has(tags::NAVIGABLE) { 500 } else { 0 };
    a += if has(tags::CONFLUENCE) {
        300
    } else if has(tags::FORD | tags::BRIDGE) {
        200
    } else {
        0
    };
    a += if has(tags::PASS) { 250 } else { 0 };
    a += if has(tags::DEFENSIBLE) { 150 } else { 0 };
    a += if has(tags::ORE) { 250 } else { 0 };
    a += if has(tags::SALT) { 200 } else { 0 };
    a
}

/// `v` raised to [`AGGLOMERATION`] (3/2): `v · √v`.
fn agglomerate(v: u64) -> u128 {
    debug_assert_eq!(AGGLOMERATION, (3, 2));
    u128::from(v) * u128::from(isqrt(v))
}

/// A town's market draws villages and hamlets within this many metres, a
/// day's ride (the range of a market town's goods; assumed, tunable):
/// rural people beyond it trade at a nearer village or not at all, so a
/// remote town is not inflated by an empty hinterland.
pub const MARKET_RANGE_M: u32 = 24_000;

/// Rural people per town: every village and hamlet within the market
/// range of a town goes to its nearest town in a straight line (ties to
/// the earlier town).
#[must_use]
pub fn hinterlands(settlements: &[Settlement]) -> Vec<(usize, u64)> {
    let towns: Vec<usize> = (0..settlements.len())
        .filter(|&k| settlements[k].tier.is_urban())
        .collect();
    let mut people = vec![0_u64; towns.len()];
    for s in settlements.iter().filter(|s| !s.tier.is_urban()) {
        let near = towns.iter().enumerate().min_by_key(|&(n, &t)| {
            let o = &settlements[t];
            (
                dist_m(
                    i64::from(o.cell_x) - i64::from(s.cell_x),
                    i64::from(o.cell_y) - i64::from(s.cell_y),
                ),
                n,
            )
        });
        if let Some((n, &t)) = near {
            let o = &settlements[t];
            let d = dist_m(
                i64::from(o.cell_x) - i64::from(s.cell_x),
                i64::from(o.cell_y) - i64::from(s.cell_y),
            );
            if d <= MARKET_RANGE_M {
                people[n] += u64::from(s.population);
            }
        }
    }
    towns.into_iter().zip(people).collect()
}

/// Shares `urban` people among the towns and cities of `settlements` by
/// hinterland and site, none below [`TOWN_MIN`], and sets their tiers.
pub fn size(settlements: &mut [Settlement], urban: u64) {
    let land = hinterlands(settlements);
    if land.is_empty() {
        return;
    }
    let weights: Vec<u128> = land
        .iter()
        .map(|&(k, rural)| {
            let a = advantage_pm(settlements[k].tag_bits);
            agglomerate((rural + BASE_HINTERLAND) * a / 1000)
        })
        .collect();
    let floor = u64::from(TOWN_MIN);
    let urban = urban.max(floor * u64::try_from(land.len()).unwrap_or(u64::MAX));
    // Towns the floor holds up are fixed at it; the rest share what is left
    // by weight, until no share falls below the floor.
    let mut fixed = vec![false; land.len()];
    let mut pops = vec![floor; land.len()];
    loop {
        let free_w: u128 = (0..land.len())
            .filter(|&n| !fixed[n])
            .map(|n| weights[n])
            .sum();
        let held = floor * u64::try_from(fixed.iter().filter(|&&f| f).count()).unwrap_or(0);
        let left = u128::from(urban.saturating_sub(held));
        let mut changed = false;
        for n in 0..land.len() {
            if fixed[n] {
                continue;
            }
            let p = (left * weights[n])
                .checked_div(free_w)
                .map_or(floor, |v| u64::try_from(v).unwrap_or(u64::MAX));
            if p < floor {
                fixed[n] = true;
                changed = true;
            }
            pops[n] = p.max(floor);
        }
        if !changed {
            break;
        }
    }
    // Rounding leftovers go to the heaviest town (ties to the earlier one).
    let placed: u64 = pops.iter().sum();
    if let Some(top) = (0..land.len()).max_by(|&a, &b| weights[a].cmp(&weights[b]).then(b.cmp(&a)))
    {
        pops[top] += urban.saturating_sub(placed);
    }
    for (n, &(k, _)) in land.iter().enumerate() {
        let p = u32::try_from(pops[n]).unwrap_or(u32::MAX);
        settlements[k].population = p;
        settlements[k].tier = Tier::of_town(p);
    }
}
