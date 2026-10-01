//! Realm plausibility statistics (goals 35 and 38): cities per realm, how
//! far apart the seats stand, how far people live from one, and how evenly
//! the realms share land and people.
//!
//! Floating point is used here only for reporting; nothing generated reads
//! it.

use crate::model::{Settlement, Tier};
use crate::realms::Realm;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The realm summary written into `stats.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RealmStats {
    /// Cities in each realm, in realm id order.
    pub cities_per_realm: Vec<u64>,
    /// Hexagonal-lattice spacing of `N` seats over the land,
    /// `√(2A / (√3 N))`, km: the distance evenly spread seats would keep.
    pub seat_lattice_km: f64,
    /// Smallest distance from a seat to its nearest other seat, km.
    pub seat_nn_min_km: f64,
    /// Mean of those distances, km.
    pub seat_nn_mean_km: f64,
    /// Median distance from a person to the nearest seat, km.
    pub people_to_seat_median_km: f64,
    /// 90th percentile of that distance, km.
    pub people_to_seat_p90_km: f64,
    /// Largest realm share of the land, per mille.
    pub land_share_max_pm: u32,
    /// Smallest realm share of the land, per mille.
    pub land_share_min_pm: u32,
    /// Largest realm share of the people, per mille.
    pub people_share_max_pm: u32,
    /// Smallest realm share of the people, per mille.
    pub people_share_min_pm: u32,
}

fn km(a: &Settlement, b: &Settlement) -> f64 {
    let dx = f64::from(a.cell_x) - f64::from(b.cell_x);
    let dy = f64::from(a.cell_y) - f64::from(b.cell_y);
    dx.hypot(dy) / 10.0
}

fn share_pm(part: u64, whole: u64) -> u32 {
    u32::try_from(part * 1000 / whole.max(1)).unwrap_or(u32::MAX)
}

/// Gathers the realm summary.
#[must_use]
pub fn gather(settlements: &[Settlement], realms: &[Realm]) -> RealmStats {
    let by_id: BTreeMap<u64, &Settlement> = settlements.iter().map(|s| (s.id.get(), s)).collect();
    let seats: Vec<&Settlement> = realms
        .iter()
        .filter_map(|r| by_id.get(&r.seat).copied())
        .collect();
    let mut out = RealmStats {
        cities_per_realm: realms
            .iter()
            .map(|r| {
                r.settlements
                    .iter()
                    .filter(|id| by_id.get(id).is_some_and(|s| s.tier == Tier::City))
                    .count() as u64
            })
            .collect(),
        ..RealmStats::default()
    };
    let land: u64 = realms.iter().map(|r| r.land_cells).sum();
    let people: u64 = realms.iter().map(|r| r.population).sum();
    let n = realms.len().max(1) as f64;
    let area_km2 = land as f64 / 100.0;
    out.seat_lattice_km = (2.0 * area_km2 / (3.0_f64.sqrt() * n)).sqrt();
    let nn: Vec<f64> = seats
        .iter()
        .map(|a| {
            seats
                .iter()
                .filter(|b| b.id != a.id)
                .map(|b| km(a, b))
                .fold(f64::INFINITY, f64::min)
        })
        .filter(|d| d.is_finite())
        .collect();
    if !nn.is_empty() {
        out.seat_nn_min_km = nn.iter().copied().fold(f64::INFINITY, f64::min);
        out.seat_nn_mean_km = nn.iter().sum::<f64>() / nn.len() as f64;
    }
    // People-weighted distance to the nearest seat.
    let mut dist: Vec<(f64, u64)> = settlements
        .iter()
        .map(|s| {
            let d = seats.iter().map(|t| km(s, t)).fold(f64::INFINITY, f64::min);
            (d, u64::from(s.population))
        })
        .filter(|(d, _)| d.is_finite())
        .collect();
    dist.sort_by(|a, b| a.0.total_cmp(&b.0));
    let total: u64 = dist.iter().map(|d| d.1).sum();
    let at = |q: u64| {
        let mut acc = 0;
        for &(d, p) in &dist {
            acc += p;
            if acc * 100 >= total * q {
                return d;
            }
        }
        0.0
    };
    out.people_to_seat_median_km = at(50);
    out.people_to_seat_p90_km = at(90);
    let land_pm: Vec<u32> = realms
        .iter()
        .map(|r| share_pm(r.land_cells, land))
        .collect();
    let people_pm: Vec<u32> = realms
        .iter()
        .map(|r| share_pm(r.population, people))
        .collect();
    out.land_share_max_pm = land_pm.iter().copied().max().unwrap_or(0);
    out.land_share_min_pm = land_pm.iter().copied().min().unwrap_or(0);
    out.people_share_max_pm = people_pm.iter().copied().max().unwrap_or(0);
    out.people_share_min_pm = people_pm.iter().copied().min().unwrap_or(0);
    out
}
