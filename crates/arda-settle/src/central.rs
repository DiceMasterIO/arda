//! Town sites by central-place logic (Christaller, Lösch; artifact "Where
//! people settle").
//!
//! A market town lives on the farmland it serves, and it serves the land a
//! farmer can reach, trade in and leave again within a day. So a town site
//! is scored first by its **market catchment**: the arable share of the
//! 16 km square around it. A coastal site loses the half of its catchment
//! that is sea, which is why harbours matter without dominating. The
//! natural nodes of travel add to that: a harbour or river mouth, a
//! confluence or crossing, a navigable reach, a defensible hill or a pass,
//! each counted once per kind so a fjord mouth tagged with everything does
//! not outscore a fertile basin by default.
//!
//! Towns then share the land out between them. With `n` towns over `A` of
//! land, a hexagonal market lattice has a spacing of `d = √(2A / (√3 n))`.
//! Each placed town tolls the score of every site within `d` in proportion
//! to how near it is, and no two towns stand closer than
//! [`SPACING_PCT`] of `d` (never under the artifact's 8 km). When the land
//! cannot hold every town at that spacing, the spacing is relaxed step by
//! step. The result is a regular pattern (nearest-neighbour index well
//! above 1) that still follows the rivers and the good land.

use crate::error::SettleError;
use crate::field::{rise, Integral};
use crate::grid::Grid;
use crate::num::{dist_m, isqrt, sat_u32};
use crate::place::TOWN_SPACING_M;
use crate::rng::Stream;
use crate::suitability::Suitability;
use crate::tags::{self, Sites};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Half-edge of the market catchment square, cells (8 km).
pub const MARKET_R: i64 = 80;
/// Tried spacings as per cent of the lattice spacing, strictest first.
pub const SPACING_PCT: [u64; 5] = [62, 54, 46, 38, 30];
/// Largest toll a town lays on a site right beside it.
const TOLL_MAX: u64 = 320;
/// Lowest town score taken at all.
const MIN_SCORE: u32 = 300;
/// Score jitter, per mille.
const JITTER: i64 = 40;

/// A chosen town site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TownSite {
    /// Grid cell.
    pub cell: usize,
    /// Score when chosen.
    pub score: u32,
}

/// Town score of a cell: catchment and local site blended, plus one bonus
/// per kind of node.
#[must_use]
pub fn town_score(base: u16, catchment_pm: i64, bits: u32, jitter: i64) -> u32 {
    let has = |b: u32| bits & b != 0;
    let port = if has(tags::HARBOUR) {
        130
    } else if has(tags::ESTUARY) {
        80
    } else {
        0
    };
    let node = if has(tags::CONFLUENCE) {
        130
    } else if has(tags::FORD | tags::BRIDGE) {
        100
    } else {
        0
    };
    let extra = if has(tags::NAVIGABLE) { 80 } else { 0 }
        + if has(tags::DEFENSIBLE) { 30 } else { 0 }
        + if has(tags::PASS) { 40 } else { 0 };
    let catch = rise(catchment_pm, 50, 450);
    sat_u32(i64::from(base) * 45 / 100 + catch * 55 / 100 + port + node + extra + jitter)
}

/// The market-lattice spacing for `n` towns over `land_cells` hectares,
/// metres.
#[must_use]
pub fn lattice_spacing_m(land_cells: u64, n: u64) -> u32 {
    if n == 0 {
        return 0;
    }
    // d² = 2A / (√3 n), with A in m² (one cell is 10,000 m²).
    let d2 = land_cells.saturating_mul(20_000).saturating_mul(1000) / (1732 * n);
    u32::try_from(isqrt(d2)).unwrap_or(u32::MAX)
}

/// Chooses `n` town sites, best first (rank order).
///
/// # Errors
/// [`SettleError::Reserve`] when the catchment raster cannot be allocated.
pub fn place_towns(
    g: &Grid,
    sites: &Sites,
    suit: &Suitability,
    seed: u64,
    n: u64,
) -> Result<Vec<TownSite>, SettleError> {
    let arable = Integral::new(g.width, g.height, |i| {
        i64::from(sites.tags[i] & tags::ARABLE != 0)
    })?;
    let score_of = |i: usize| {
        let (x, y) = g.xy(i);
        let j = Stream::new(seed, "site", u64::try_from(i).unwrap_or(0)).jitter(JITTER);
        town_score(
            suit.score[i],
            arable.permille(x, y, MARKET_R),
            sites.tags[i],
            j,
        )
    };
    let ranked: Vec<(u32, usize)> = (0..g.len())
        .filter(|&i| suit.score[i] > 0)
        .map(|i| (score_of(i), i))
        .filter(|&(s, _)| s >= MIN_SCORE)
        .collect();
    let d = u64::from(lattice_spacing_m(
        u64::try_from(g.land_cells()).unwrap_or(0),
        n,
    ));
    let mut best: Vec<TownSite> = Vec::new();
    for pct in SPACING_PCT {
        let min_m = u32::try_from(d * pct / 100)
            .unwrap_or(u32::MAX)
            .max(TOWN_SPACING_M);
        let got = greedy(g, &ranked, n, u32::try_from(d).unwrap_or(u32::MAX), min_m);
        let done = got.len() >= usize::try_from(n).unwrap_or(usize::MAX);
        if got.len() > best.len() {
            best = got;
        }
        if done || min_m == TOWN_SPACING_M {
            break;
        }
    }
    Ok(best)
}

/// Best site first with a distance-weighted toll within `d` metres of every
/// placed town and a hard spacing of `min_m`; scores are re-evaluated
/// lazily when the town count has moved on.
fn greedy(g: &Grid, ranked: &[(u32, usize)], n: u64, d: u32, min_m: u32) -> Vec<TownSite> {
    let mut heap: BinaryHeap<(u32, Reverse<usize>, usize, u32)> = ranked
        .iter()
        .map(|&(sc, i)| (sc, Reverse(i), 0, sc))
        .collect();
    let mut placed: Vec<(i64, i64, TownSite)> = Vec::new();
    while let Some((score, Reverse(i), stamp, b)) = heap.pop() {
        if u64::try_from(placed.len()).unwrap_or(u64::MAX) >= n {
            break;
        }
        let (x, y) = g.xy(i);
        let near = |&(px, py, _): &(i64, i64, TownSite)| dist_m(px - x, py - y);
        if stamp != placed.len() {
            let toll: u64 = placed
                .iter()
                .map(near)
                .filter(|&m| m < d)
                .map(|m| TOLL_MAX * u64::from(d - m) / u64::from(d.max(1)))
                .sum();
            let eff = u64::from(b).saturating_sub(toll);
            let eff = u32::try_from(eff).unwrap_or(0);
            if eff >= MIN_SCORE {
                heap.push((eff, Reverse(i), placed.len(), b));
            }
            continue;
        }
        if placed.iter().map(near).any(|m| m < min_m) {
            continue;
        }
        placed.push((x, y, TownSite { cell: i, score }));
    }
    placed.into_iter().map(|(_, _, t)| t).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fertile_basin_can_beat_a_bare_harbour() {
        let harbour = town_score(700, 150, tags::HARBOUR | tags::ESTUARY, 0);
        let basin = town_score(700, 450, tags::CONFLUENCE, 0);
        assert!(basin > harbour, "{basin} vs {harbour}");
        // Stacked tags count once per kind.
        let fjord = town_score(700, 150, tags::HARBOUR | tags::ESTUARY | tags::COAST, 0);
        assert_eq!(fjord, harbour);
    }

    #[test]
    fn lattice_spacing_follows_area_per_town() {
        // 600,000 ha and 9 towns: about 667 km² a town, 27.7 km apart.
        let d = lattice_spacing_m(600_000, 9);
        assert!((27_000..28_500).contains(&d), "{d}");
        assert_eq!(lattice_spacing_m(1000, 0), 0);
    }
}
