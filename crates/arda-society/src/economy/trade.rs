//! Gravity-model trade along roads (deliverable 1).
//!
//! Per good, each supplier splits its surplus over reachable settlements in
//! deficit, in proportion to `deficit × attraction / (distance² + c)`. Every
//! load is accounted for: produced = used at home + exported + stored, and
//! exports = imports (tests/economy.rs).

use super::production::Basket;
use crate::ctx::Ctx;
use crate::graph::Reach;
use crate::input::Function;
use crate::num::{u64_of_u128, u64_of_usize};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Allocation rounds per good.
const ROUNDS: usize = 4;
/// Demanders each supplier considers per round.
const FAN_OUT: usize = 12;
/// Markets each supplier considers: its nearest settlements by road
/// (review round 2, #28). A full reach out to the longest good range held
/// 10^8-10^9 map entries on a full world; this keeps the trade step
/// O(settlements × MARKETS) in memory and time (assumed, tunable).
pub const MARKETS: usize = 48;
/// Softening term of the distance decay, km².
const DECAY_C: u128 = 9;

/// One settlement-to-settlement shipment of one good per year.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeFlow {
    /// Good key.
    pub good: String,
    /// Exporting settlement.
    #[serde(with = "crate::ids::str")]
    pub from: u64,
    /// Importing settlement.
    #[serde(with = "crate::ids::str")]
    pub to: u64,
    /// Loads per year.
    pub amount: u64,
    /// Value per year, silver pieces.
    pub value_sp: u64,
    /// Road distance, metres.
    pub distance_m: u64,
    /// Settlements on the way, both ends included.
    #[serde(with = "crate::ids::vec")]
    pub path: Vec<u64>,
    /// Road ids on the way, in travel order.
    #[serde(with = "crate::ids::vec")]
    pub roads: Vec<u64>,
}

/// Where each load of one settlement went.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    /// Loads produced.
    pub production: Basket,
    /// Loads wanted.
    pub demand: Basket,
    /// Own production used at home.
    pub local_use: Basket,
    /// Loads sent out.
    pub exports: Basket,
    /// Loads brought in.
    pub imports: Basket,
    /// Surplus nobody could reach.
    pub stored: Basket,
    /// Demand left unmet.
    pub shortfall: Basket,
}

fn put(b: &mut Basket, k: &str, v: u64) {
    if v > 0 {
        *b.entry(k.to_string()).or_default() += v;
    }
}

fn attraction(ctx: &Ctx<'_>, j: usize) -> u128 {
    let s = ctx.s(j);
    let hub = s.has(Function::Market) || s.has(Function::Port);
    1 + if hub { 2 } else { 0 } + u128::from(s.tier.is_urban())
}

/// Runs trade for every good; returns per-node ledgers and all flows.
#[must_use]
pub fn trade(ctx: &Ctx<'_>, prod: &[Basket], dem: &[Basket]) -> (Vec<Ledger>, Vec<TradeFlow>) {
    let n = ctx.n();
    let cutoff = ctx
        .t
        .goods
        .goods
        .iter()
        .map(|g| u64::from(g.range_km))
        .max()
        .unwrap_or(0)
        * 1000;
    let reach: Vec<Reach> = (0..n)
        .map(|i| ctx.graph.reach_nearest(i, cutoff, MARKETS))
        .collect();
    let attract: Vec<u128> = (0..n).map(|j| attraction(ctx, j)).collect();
    let mut ledgers: Vec<Ledger> = (0..n)
        .map(|i| Ledger {
            production: prod[i].clone(),
            demand: dem[i].clone(),
            ..Ledger::default()
        })
        .collect();
    let mut flows = Vec::new();
    for good in &ctx.t.goods.goods {
        let key = good.key.as_str();
        let range_m = u64::from(good.range_km) * 1000;
        let amt = |b: &Basket| b.get(key).copied().unwrap_or(0);
        let mut supply = vec![0_u64; n];
        let mut deficit = vec![0_u64; n];
        for i in 0..n {
            let (p, d) = (amt(&prod[i]), amt(&dem[i]));
            let local = p.min(d);
            put(&mut ledgers[i].local_use, key, local);
            supply[i] = p - local;
            deficit[i] = d - local;
        }
        let mut moved: BTreeMap<(usize, usize), u64> = BTreeMap::new();
        for _ in 0..ROUNDS {
            let mut any = false;
            for i in 0..n {
                if supply[i] == 0 {
                    continue;
                }
                let mut cand: Vec<(u128, usize)> = reach[i]
                    .dist
                    .iter()
                    .filter(|&(&j, &d)| j != i && deficit[j] > 0 && d <= range_m)
                    .map(|(&j, &d)| {
                        let km = u128::from(d / 1000);
                        let w =
                            u128::from(deficit[j]) * attract[j] * 1_000_000 / (km * km + DECAY_C);
                        (w.max(1), j)
                    })
                    .collect();
                cand.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
                cand.truncate(FAN_OUT);
                let total: u128 = cand.iter().map(|c| c.0).sum();
                if total == 0 {
                    continue;
                }
                let budget = u128::from(supply[i]);
                for &(w, j) in &cand {
                    let share = u64_of_u128(budget * w / total)
                        .min(deficit[j])
                        .min(supply[i]);
                    if share > 0 {
                        supply[i] -= share;
                        deficit[j] -= share;
                        *moved.entry((i, j)).or_default() += share;
                        any = true;
                    }
                }
                // Floor rounding can starve small suppliers: hand the rest to
                // the strongest pull so every round makes progress.
                if let Some(&(_, j)) = cand.iter().find(|c| deficit[c.1] > 0) {
                    let rest = supply[i].min(deficit[j]);
                    if rest > 0 && supply[i] < u64_of_usize(FAN_OUT) * 2 {
                        supply[i] -= rest;
                        deficit[j] -= rest;
                        *moved.entry((i, j)).or_default() += rest;
                        any = true;
                    }
                }
            }
            if !any {
                break;
            }
        }
        for i in 0..n {
            put(&mut ledgers[i].stored, key, supply[i]);
            put(&mut ledgers[i].shortfall, key, deficit[i]);
        }
        for (&(i, j), &amount) in &moved {
            put(&mut ledgers[i].exports, key, amount);
            put(&mut ledgers[j].imports, key, amount);
            let r = &reach[i];
            flows.push(TradeFlow {
                good: good.key.clone(),
                from: ctx.s(i).id,
                to: ctx.s(j).id,
                amount,
                value_sp: amount * u64::from(good.value_sp),
                distance_m: r.dist.get(&j).copied().unwrap_or(0),
                path: r.nodes_to(j).into_iter().map(|k| ctx.s(k).id).collect(),
                roads: r.roads_to(j),
            });
        }
    }
    (ledgers, flows)
}
