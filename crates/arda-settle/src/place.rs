//! Settlement placement by carrying capacity (spec step 3; artifact "Where
//! people settle").
//!
//! Population is the region's density times its land. Three tenths live in
//! towns sized by the rank-size rule, just under half in villages of a few
//! hundred, the rest in hamlets. Placement is greedy, best site first, within
//! each tier, with the artifact's spacing disks.

use crate::central;
use crate::error::SettleError;
use crate::grid::Grid;
use crate::model::{Settlement, Tier, TOWN_MIN};
use crate::num::{dist_m, sat_u32, ui};
use crate::rng::Stream;
use crate::suitability::Suitability;
use crate::tags::{self, Sites};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};

/// Minimum distance between two towns (a little under the medieval market
/// distance), metres.
pub const TOWN_SPACING_M: u32 = 8000;
/// Villages keep this far from towns.
pub const VILLAGE_FROM_TOWN_M: u32 = 1500;
/// Villages keep this far from each other.
pub const VILLAGE_SPACING_M: u32 = 2000;
/// Hamlets keep this far from anything.
pub const HAMLET_SPACING_M: u32 = 900;

/// Urban and village shares of the population, per-mille.
const URBAN_PM: u64 = 300;
const VILLAGE_PM: u64 = 470;
/// Minimum tier scores for a site to be taken at all.
const MIN_SCORE: [u32; 3] = [300, 220, 140];
/// Score jitter so the pattern is not mechanical, per-mille.
const JITTER: i64 = 40;

/// Placement input knobs.
#[derive(Debug, Clone, Copy)]
pub struct PlaceParams {
    /// World seed.
    pub seed: u64,
    /// People per km² of land.
    pub density_per_km2: u32,
}

/// Placement outcome.
#[derive(Debug, Clone)]
pub struct Placement {
    /// Every settlement, towns first (by rank), then villages, then hamlets.
    pub settlements: Vec<Settlement>,
    /// The population the land supports.
    pub target_population: u64,
    /// People the site list could not house.
    pub unplaced: u64,
}

/// Harmonic number H(n) in millionths.
fn harmonic_micro(n: u64) -> u64 {
    (1..=n).map(|k| 1_000_000 / k).sum()
}

/// Number of towns and the first town's size: as many towns as the
/// rank-size rule allows while the smallest is still a town.
#[must_use]
pub fn rank_size(urban: u64) -> (u64, u64) {
    let mut best = (0, 0);
    for n in 1..=10_000_u64 {
        let p1 = urban * 1_000_000 / harmonic_micro(n).max(1);
        if p1 / n < u64::from(TOWN_MIN) {
            break;
        }
        best = (n, p1);
    }
    best
}

/// Spatial buckets of placed settlements, 1 km on a side.
struct Buckets {
    wide: i64,
    cells: BTreeMap<i64, Vec<(i64, i64, Tier)>>,
}

impl Buckets {
    fn new(width: usize) -> Self {
        Self {
            wide: ui(width) / 10 + 1,
            cells: BTreeMap::new(),
        }
    }

    fn add(&mut self, x: i64, y: i64, tier: Tier) {
        self.cells
            .entry((y / 10) * self.wide + x / 10)
            .or_default()
            .push((x, y, tier));
    }

    /// Settlements within `reach_m` whose tier index (0 urban, 1 village,
    /// 2 hamlet) is at most `tier_ix`.
    fn count(&self, x: i64, y: i64, reach_m: u32, tier_ix: usize) -> u32 {
        let r = i64::from(reach_m) / 1000 + 1;
        let mut n = 0;
        for by in (y / 10 - r)..=(y / 10 + r) {
            for bx in (x / 10 - r)..=(x / 10 + r) {
                if bx < 0 || by < 0 || bx >= self.wide {
                    continue;
                }
                if let Some(list) = self.cells.get(&(by * self.wide + bx)) {
                    n += list
                        .iter()
                        .filter(|&&(px, py, t)| {
                            let ix = match t {
                                Tier::City | Tier::Town => 0,
                                Tier::Village => 1,
                                Tier::Hamlet => 2,
                            };
                            ix <= tier_ix && dist_m(px - x, py - y) < reach_m
                        })
                        .count();
                }
            }
        }
        u32::try_from(n).unwrap_or(u32::MAX)
    }

    /// Whether any settlement closer than `limit(tier)` metres exists.
    fn crowded(&self, x: i64, y: i64, reach_m: u32, limit: impl Fn(Tier) -> u32) -> bool {
        let r = i64::from(reach_m) / 1000 + 1;
        for by in (y / 10 - r)..=(y / 10 + r) {
            for bx in (x / 10 - r)..=(x / 10 + r) {
                if bx < 0 || by < 0 || bx >= self.wide {
                    continue;
                }
                if let Some(list) = self.cells.get(&(by * self.wide + bx)) {
                    if list
                        .iter()
                        .any(|&(px, py, t)| dist_m(px - x, py - y) < limit(t))
                    {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Per-site score jitter.
fn jitter(seed: u64, i: usize) -> i64 {
    Stream::new(seed, "site", u64::try_from(i).unwrap_or(0)).jitter(JITTER)
}

/// Crowding toll per tier: (radius m, score per neighbour of that tier or
/// higher). Towns use the central-place toll instead (`central.rs`).
const TOLL: [(u32, u32); 3] = [(0, 0), (5000, 110), (2500, 70)];

/// Village and hamlet site score: harbours and river nodes add a little.
/// Towns are scored by `central::town_score`.
fn tier_score(base: u16, bits: u32, tier: usize, jitter: i64) -> u32 {
    let bonus = |bit: u32, v: [i64; 3]| if bits & bit != 0 { v[tier] } else { 0 };
    let s = i64::from(base)
        + bonus(tags::HARBOUR, [240, 80, 20])
        + bonus(tags::NAVIGABLE, [220, 40, 0])
        + bonus(tags::ESTUARY, [150, 40, 0])
        + bonus(tags::CONFLUENCE, [160, 60, 0])
        + bonus(tags::FORD | tags::BRIDGE, [80, 50, 0])
        + bonus(tags::DEFENSIBLE, [40, 20, 0])
        + bonus(tags::PASS, [60, 20, 0])
        + jitter;
    sat_u32(s)
}

/// Places every settlement.
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
pub fn place(
    g: &Grid,
    sites: &Sites,
    suit: &Suitability,
    p: PlaceParams,
) -> Result<Placement, SettleError> {
    let land_km2_x100 = ui(g.land_cells());
    let target = u64::try_from(land_km2_x100).unwrap_or(0) * u64::from(p.density_per_km2) / 100;
    let urban = target * URBAN_PM / 1000;
    let villages = target * VILLAGE_PM / 1000;
    let hamlets = target - urban - villages;
    let (towns, p1) = rank_size(urban);

    let mut out: Vec<Settlement> = Vec::new();
    let mut buckets = Buckets::new(g.width);
    let mut placed_people = 0_u64;
    // Towns by central-place logic (`central.rs`), best site first, so the
    // first is the largest.
    for (k, t) in central::place_towns(g, sites, suit, p.seed, towns)?
        .into_iter()
        .enumerate()
    {
        let rank = u64::try_from(k + 1).unwrap_or(u64::MAX);
        let pop = u32::try_from(p1 / rank).unwrap_or(u32::MAX);
        let tier = Tier::of_town(pop);
        let (x, y) = g.xy(t.cell);
        placed_people += u64::from(pop);
        buckets.add(x, y, tier);
        out.push(new_settlement(
            g, sites, t.cell, tier, pop, t.score, rank, 0,
        ));
    }
    for (tier_ix, budget) in [(1_usize, villages), (2, hamlets)] {
        let ranked: Vec<(u32, usize)> = (0..g.len())
            .filter(|&i| suit.score[i] > 0)
            .map(|i| {
                (
                    tier_score(suit.score[i], sites.tags[i], tier_ix, jitter(p.seed, i)),
                    i,
                )
            })
            .filter(|&(s, _)| s >= MIN_SCORE[tier_ix])
            .collect();
        // Best site first, with a crowding toll: each neighbour of the same
        // or higher tier within the toll radius lowers a site's score, so
        // once the best coast and valley sites are taken the next pick goes
        // to the next valley rather than packing the coast. Scores are
        // re-evaluated lazily when the placement count has moved on.
        let mut heap: BinaryHeap<(u32, Reverse<usize>, usize)> = ranked
            .into_iter()
            .map(|(sc, i)| (sc, Reverse(i), 0))
            .collect();
        let mut used = 0_u64;
        let mut placed_here = 0_usize;
        while let Some((score, Reverse(i), stamp)) = heap.pop() {
            if used >= budget {
                break;
            }
            if stamp != placed_here {
                let (x, y) = g.xy(i);
                let (reach, toll) = TOLL[tier_ix];
                let n = buckets.count(x, y, reach, tier_ix);
                let base = tier_score(suit.score[i], sites.tags[i], tier_ix, jitter(p.seed, i));
                let eff = base.saturating_sub(toll * n);
                if eff >= MIN_SCORE[tier_ix] {
                    heap.push((eff, Reverse(i), placed_here));
                }
                continue;
            }
            let (x, y) = g.xy(i);
            let crowded = match tier_ix {
                1 => buckets.crowded(x, y, VILLAGE_SPACING_M, |t| match t {
                    Tier::Town | Tier::City => VILLAGE_FROM_TOWN_M,
                    Tier::Village => VILLAGE_SPACING_M,
                    Tier::Hamlet => HAMLET_SPACING_M,
                }),
                _ => buckets.crowded(x, y, HAMLET_SPACING_M, |_| HAMLET_SPACING_M),
            };
            if crowded {
                continue;
            }
            let mut s = Stream::new(p.seed, "population", u64::try_from(i).unwrap_or(0));
            let share = i64::from(suit.arable_pm[i]);
            let (tier, pop) = match tier_ix {
                // A better-farmed site feeds a larger village or hamlet.
                1 => (
                    Tier::Village,
                    sat_u32((120 + share * 450 / 1000 + s.jitter(40)).clamp(100, 600)),
                ),
                _ => (
                    Tier::Hamlet,
                    sat_u32((12 + share * 68 / 1000 + s.jitter(8)).clamp(12, 80)),
                ),
            };
            used += u64::from(pop);
            placed_here += 1;
            buckets.add(x, y, tier);
            out.push(new_settlement(g, sites, i, tier, pop, score, 0, tier_ix));
        }
        placed_people += used;
    }
    for (n, s) in out.iter_mut().enumerate() {
        s.id = arda_ids::SettlementId(u64::try_from(n + 1).unwrap_or(0));
    }
    Ok(Placement {
        settlements: out,
        target_population: target,
        unplaced: target.saturating_sub(placed_people),
    })
}

#[allow(clippy::too_many_arguments)]
fn new_settlement(
    g: &Grid,
    sites: &Sites,
    i: usize,
    tier: Tier,
    population: u32,
    score: u32,
    rank: u64,
    tier_ix: usize,
) -> Settlement {
    let (x, y) = g.xy(i);
    Settlement {
        id: arda_ids::SettlementId(0),
        name: String::new(),
        tier,
        population,
        functions: Vec::new(),
        wealth: 0,
        culture: String::new(),
        realm_id: arda_ids::RealmId(0),
        biome: crate::model::Biome::default(),
        coastal: false,
        riverine: false,
        name_gloss: String::new(),
        x_m: x * 100 + 50,
        y_m: y * 100 + 50,
        cell_x: sat_u32(x),
        cell_y: sat_u32(y),
        height_m: g.height_mm[i] / 1000,
        rank: if tier_ix == 0 {
            u32::try_from(rank).unwrap_or(0)
        } else {
            0
        },
        site_tags: tags::names(sites.tags[i]),
        history: String::new(),
        buildings: BTreeMap::new(),
        tongue: None,
        tag_bits: sites.tags[i],
        score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_size_keeps_the_smallest_a_town() {
        let (n, p1) = rank_size(27_000);
        assert!(n >= 5);
        assert!(p1 / n >= u64::from(TOWN_MIN));
        let (n2, p2) = rank_size(27_000);
        assert_eq!((n, p1), (n2, p2));
        assert_eq!(rank_size(500), (0, 0));
    }
}
