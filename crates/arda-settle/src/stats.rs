//! Plausibility statistics (artifact "How plausibility is checked").
//!
//! Floating point is used here only for reporting; nothing generated reads it.

use crate::crossings::{Crossing, CrossingKind};
use crate::model::{Settlement, Tier};
use crate::roads::{Network, RoadClass, Terrain};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Routes shorter than this are left out of sinuosity (grid noise dominates).
pub const SINUOSITY_MIN_M: u64 = 1500;

/// A route is a runaway detour above this sinuosity.
pub const RUNAWAY: f64 = 2.5;

/// Distribution of one measure over a set of routes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Spread {
    /// Routes counted.
    pub routes: u64,
    /// Median.
    pub median: f64,
    /// 90th percentile.
    pub p90: f64,
    /// Largest.
    pub max: f64,
}

impl Spread {
    /// Spread of `v`.
    #[must_use]
    pub fn of(mut v: Vec<f64>) -> Self {
        v.sort_by(f64::total_cmp);
        let n = v.len();
        let at = |q: usize| {
            v.get((n.saturating_sub(1) * q) / 100)
                .copied()
                .unwrap_or(0.0)
        };
        Self {
            routes: n as u64,
            median: median(v.clone()),
            p90: at(90),
            max: v.last().copied().unwrap_or(0.0),
        }
    }
}

/// The summary written to `stats.json` and printed by the CLI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Stats {
    /// Population the land supports (density × land).
    pub target_population: u64,
    /// Population placed.
    pub population: u64,
    /// Settlements per tier.
    pub count_by_tier: BTreeMap<String, u64>,
    /// People per tier.
    pub population_by_tier: BTreeMap<String, u64>,
    /// Least-squares slope of ln(population) on ln(rank) over towns and
    /// cities (the rank-size rule is −1).
    pub rank_size_slope: f64,
    /// R² of that fit.
    pub rank_size_r2: f64,
    /// Farmland (fields and orchards) per inhabitant, hectares.
    pub farmland_ha_per_person: f64,
    /// Road kilometres per class (new cells only, so shared stretches count once).
    pub road_km: BTreeMap<String, f64>,
    /// Median route sinuosity (route length ÷ straight line) per class.
    pub sinuosity_median: BTreeMap<String, f64>,
    /// Median sinuosity over every route of 1.5 km or more.
    pub sinuosity_all: f64,
    /// Median sinuosity over open-ground routes (median slope under 5°),
    /// the artifact's 1.2–1.4 check; hill and mountain routes rise above it.
    pub sinuosity_open: f64,
    /// Routes counted in `sinuosity_open`.
    pub open_routes: u64,
    /// Sinuosity per terrain class (`open`, `hill`, `mountain`).
    pub sinuosity_by_terrain: BTreeMap<String, Spread>,
    /// Routes more than [`RUNAWAY`] times their straight line.
    pub runaway_routes: u64,
    /// Crossings per kind.
    pub crossings: BTreeMap<String, u64>,
    /// Passes found.
    pub passes: u64,
    /// Realms.
    pub realms: u64,
    /// Per mille of the realm border that has a river or ridge line within
    /// three cells and runs on it, after snapping (target ≥ 900).
    pub border_on_river_or_ridge_pm: u32,
    /// The same share before the snapping pass.
    pub border_on_river_or_ridge_raw_pm: u32,
    /// Per mille of all realm border on a river or ridge line.
    pub border_natural_of_all_pm: u32,
    /// Baseline: per-mille of all land within two cells of one.
    pub land_near_river_or_ridge_pm: u32,
    /// Settlements no route reached.
    pub unreachable: u64,
    /// Share of towns and cities away from the coast (central places at
    /// inland crossings and basins).
    pub inland_town_share: f64,
    /// Clark–Evans nearest-neighbour index of towns and cities over the
    /// land area: 1 is random, above 1 regular, 2.15 a perfect hexagonal
    /// lattice.
    pub town_nni: f64,
    /// Mean distance from a town to its nearest town, km.
    pub town_nn_km: f64,
    /// Settlements with the `mining` function.
    pub mining_settlements: u64,
    /// Their share of all settlements, per mille.
    pub mining_share_pm: u32,
    /// Per mille of mining settlements with another within
    /// [`MINING_CLUSTER_M`] (1000 means every mine has a neighbour).
    pub mining_clustered_pm: u32,
    /// Cities per realm, seat spread and realm balance.
    #[serde(default)]
    pub realm: crate::realm_stats::RealmStats,
}

/// Inland share, nearest-neighbour index and mean nearest-town distance
/// (km) of the towns and cities, over `land_cells` hectares.
#[must_use]
pub fn town_pattern(settlements: &[Settlement], land_cells: u64) -> (f64, f64, f64) {
    let towns: Vec<&Settlement> = settlements.iter().filter(|s| s.tier.is_urban()).collect();
    if towns.len() < 2 {
        return (0.0, 0.0, 0.0);
    }
    let n = towns.len() as f64;
    let inland = towns.iter().filter(|s| !s.coastal).count() as f64 / n;
    let nn: f64 = towns
        .iter()
        .map(|a| {
            towns
                .iter()
                .filter(|b| b.id != a.id)
                .map(|b| {
                    let dx = f64::from(a.cell_x) - f64::from(b.cell_x);
                    let dy = f64::from(a.cell_y) - f64::from(b.cell_y);
                    dx.hypot(dy) * 100.0
                })
                .fold(f64::INFINITY, f64::min)
        })
        .sum::<f64>()
        / n;
    let area_m2 = land_cells as f64 * 10_000.0;
    let expected = 0.5 * (area_m2 / n).sqrt();
    (inland, nn / expected, nn / 1000.0)
}

/// Two mining settlements this close count as one cluster.
pub const MINING_CLUSTER_M: u32 = 4000;

/// Mining count, share and clustering (see [`Stats`]).
#[must_use]
pub fn mining(settlements: &[Settlement]) -> (u64, u32, u32) {
    let m: Vec<&Settlement> = settlements
        .iter()
        .filter(|s| s.has(crate::model::Function::Mining))
        .collect();
    let paired = m
        .iter()
        .filter(|a| {
            m.iter().any(|b| {
                b.id != a.id
                    && crate::num::dist_m(
                        i64::from(a.cell_x) - i64::from(b.cell_x),
                        i64::from(a.cell_y) - i64::from(b.cell_y),
                    ) <= MINING_CLUSTER_M
            })
        })
        .count();
    let pm = |a: usize, b: usize| u32::try_from(a * 1000 / b.max(1)).unwrap_or(0);
    (
        m.len() as u64,
        pm(m.len(), settlements.len()),
        pm(paired, m.len()),
    )
}

const fn class_key(c: RoadClass) -> &'static str {
    c.key()
}

fn tier_key(t: Tier) -> &'static str {
    match t {
        Tier::Hamlet => "hamlet",
        Tier::Village => "village",
        Tier::Town => "town",
        Tier::City => "city",
    }
}

/// Median of a list (0 when empty).
#[must_use]
pub fn median(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// Least-squares slope and R² of ln(pop) on ln(rank) for urban settlements.
#[must_use]
pub fn rank_size_fit(settlements: &[Settlement]) -> (f64, f64) {
    let mut pops: Vec<f64> = settlements
        .iter()
        .filter(|s| s.tier.is_urban())
        .map(|s| f64::from(s.population))
        .collect();
    pops.sort_by(|a, b| b.total_cmp(a));
    if pops.len() < 2 {
        return (0.0, 0.0);
    }
    let pts: Vec<(f64, f64)> = pops
        .iter()
        .enumerate()
        .map(|(k, p)| (crate::num::ln((k + 1) as f64), crate::num::ln(*p)))
        .collect();
    let n = pts.len() as f64;
    let mx = pts.iter().map(|p| p.0).sum::<f64>() / n;
    let my = pts.iter().map(|p| p.1).sum::<f64>() / n;
    let sxx: f64 = pts.iter().map(|p| (p.0 - mx).powi(2)).sum();
    let sxy: f64 = pts.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
    let syy: f64 = pts.iter().map(|p| (p.1 - my).powi(2)).sum();
    if sxx == 0.0 || syy == 0.0 {
        return (0.0, 0.0);
    }
    let slope = sxy / sxx;
    (slope, (sxy * sxy) / (sxx * syy))
}

/// Per-route sinuosities for routes of at least [`SINUOSITY_MIN_M`],
/// optionally of one road class and one terrain class.
#[must_use]
pub fn sinuosities(net: &Network, class: Option<RoadClass>, terrain: Option<Terrain>) -> Vec<f64> {
    net.roads
        .iter()
        .filter(|r| class.is_none_or(|c| r.class == c) && r.straight_m >= SINUOSITY_MIN_M)
        .filter(|r| terrain.is_none_or(|t| r.terrain == t))
        .map(|r| r.length_m as f64 / r.straight_m as f64)
        .collect()
}

/// Gathers every statistic.
#[must_use]
pub fn gather(
    settlements: &[Settlement],
    target_population: u64,
    farmland_cells: u64,
    net: &Network,
    crossings: &[Crossing],
    passes: usize,
    realms: usize,
) -> Stats {
    let mut s = Stats {
        target_population,
        ..Stats::default()
    };
    for t in [Tier::Hamlet, Tier::Village, Tier::Town, Tier::City] {
        let of: Vec<&Settlement> = settlements.iter().filter(|x| x.tier == t).collect();
        s.count_by_tier.insert(tier_key(t).into(), of.len() as u64);
        let p: u64 = of.iter().map(|x| u64::from(x.population)).sum();
        s.population_by_tier.insert(tier_key(t).into(), p);
        s.population += p;
    }
    (s.rank_size_slope, s.rank_size_r2) = rank_size_fit(settlements);
    s.farmland_ha_per_person = if s.population == 0 {
        0.0
    } else {
        farmland_cells as f64 / s.population as f64
    };
    for c in RoadClass::ROADS {
        let m: u64 = net
            .roads
            .iter()
            .filter(|r| r.class == c)
            .map(|r| r.new_m)
            .sum();
        s.road_km.insert(class_key(c).into(), m as f64 / 1000.0);
        s.sinuosity_median
            .insert(class_key(c).into(), median(sinuosities(net, Some(c), None)));
    }
    let all = sinuosities(net, None, None);
    s.runaway_routes = all.iter().filter(|&&v| v > RUNAWAY).count() as u64;
    s.sinuosity_all = median(all);
    let open = sinuosities(net, None, Some(Terrain::Open));
    s.open_routes = open.len() as u64;
    s.sinuosity_open = median(open);
    for t in Terrain::ALL {
        s.sinuosity_by_terrain
            .insert(t.key().into(), Spread::of(sinuosities(net, None, Some(t))));
    }
    for k in [
        CrossingKind::Bridge,
        CrossingKind::Ford,
        CrossingKind::Ferry,
    ] {
        let key = match k {
            CrossingKind::Bridge => "bridge",
            CrossingKind::Ford => "ford",
            CrossingKind::Ferry => "ferry",
        };
        s.crossings.insert(
            key.into(),
            crossings.iter().filter(|c| c.kind == k).count() as u64,
        );
    }
    s.passes = passes as u64;
    s.realms = realms as u64;
    s.unreachable = net.unreachable.len() as u64;
    (
        s.mining_settlements,
        s.mining_share_pm,
        s.mining_clustered_pm,
    ) = mining(settlements);
    s
}
