//! Economy: production, trade flows, road traffic and key goods per market.

pub mod production;
pub mod trade;

use crate::ctx::Ctx;
use crate::input::Function;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub use trade::{Ledger, TradeFlow};

/// How a key good moves through a market.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GoodRole {
    /// Mostly sent out.
    Export,
    /// Mostly brought in.
    Import,
    /// Mostly carried through to elsewhere.
    Transit,
}

/// One of a market's key trade goods.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyGood {
    /// Good key.
    pub good: String,
    /// Dominant direction.
    pub role: GoodRole,
    /// Value handled per year, silver pieces.
    pub value_sp: u64,
}

/// Economy of one settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementEconomy {
    /// Where every load went.
    pub ledger: Ledger,
    /// Export value minus import value, silver pieces per year.
    pub trade_balance_sp: i64,
    /// Whether it is a market (market or port function).
    pub market: bool,
    /// Key trade goods (markets only), most valuable first.
    pub key_goods: Vec<KeyGood>,
}

/// Traffic on one road.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoadTraffic {
    /// Road id.
    #[serde(with = "crate::ids::str")]
    pub road: u64,
    /// Value carried per year, silver pieces.
    pub value_sp: u64,
    /// Most valuable goods carried, up to three.
    pub goods: Vec<String>,
}

/// World totals for one good.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoodTotals {
    /// Good key.
    pub good: String,
    /// Readable name.
    pub label: String,
    /// Value of one load, silver pieces.
    pub value_sp: u32,
    /// Loads produced.
    pub produced: u64,
    /// Loads used where made.
    pub local_use: u64,
    /// Loads traded.
    pub traded: u64,
    /// Loads stored unsold.
    pub stored: u64,
    /// Loads wanted but missing.
    pub shortfall: u64,
}

/// World economy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Economy {
    /// Totals per good, table order.
    pub goods: Vec<GoodTotals>,
    /// All flows.
    pub flows: Vec<TradeFlow>,
    /// Traffic per road, busiest first.
    pub road_traffic: Vec<RoadTraffic>,
}

/// Economy results plus per-node settlement economies (input order).
#[derive(Debug, Clone)]
pub struct EconomyRun {
    /// World economy.
    pub economy: Economy,
    /// Per node.
    pub per_node: Vec<SettlementEconomy>,
}

fn sum(b: &production::Basket, k: &str) -> u64 {
    b.get(k).copied().unwrap_or(0)
}

fn value(ctx: &Ctx<'_>, b: &production::Basket) -> i64 {
    let v: u64 = b
        .iter()
        .map(|(k, &n)| n * u64::from(ctx.t.good(k).map_or(0, |g| g.value_sp)))
        .sum();
    crate::num::i64_of(v)
}

/// Runs production and trade.
#[must_use]
pub fn run(ctx: &Ctx<'_>) -> EconomyRun {
    let prod: Vec<_> = (0..ctx.n())
        .map(|i| production::production(ctx, i))
        .collect();
    let dem: Vec<_> = (0..ctx.n())
        .map(|i| production::demand(ctx, i, &prod[i]))
        .collect();
    let (ledgers, flows) = trade::trade(ctx, &prod, &dem);
    let mut traffic: BTreeMap<u64, BTreeMap<String, u64>> = BTreeMap::new();
    let mut handled: BTreeMap<u64, BTreeMap<String, [u64; 3]>> = BTreeMap::new();
    for f in &flows {
        for &r in &f.roads {
            *traffic
                .entry(r)
                .or_default()
                .entry(f.good.clone())
                .or_default() += f.value_sp;
        }
        let last = f.path.len().saturating_sub(1);
        for (k, &sid) in f.path.iter().enumerate() {
            let slot = if k == 0 {
                0
            } else if k == last {
                1
            } else {
                2
            };
            handled
                .entry(sid)
                .or_default()
                .entry(f.good.clone())
                .or_default()[slot] += f.value_sp;
        }
    }
    let per_node = ledgers
        .into_iter()
        .enumerate()
        .map(|(i, ledger)| {
            let s = ctx.s(i);
            let market = s.has(Function::Market) || s.has(Function::Port);
            let key_goods = if market {
                key_goods(handled.get(&s.id))
            } else {
                Vec::new()
            };
            SettlementEconomy {
                trade_balance_sp: value(ctx, &ledger.exports) - value(ctx, &ledger.imports),
                ledger,
                market,
                key_goods,
            }
        })
        .collect::<Vec<_>>();
    let goods = ctx
        .t
        .goods
        .goods
        .iter()
        .map(|g| {
            let tot = |f: fn(&Ledger) -> &production::Basket| -> u64 {
                per_node.iter().map(|e| sum(f(&e.ledger), &g.key)).sum()
            };
            GoodTotals {
                good: g.key.clone(),
                label: g.label.clone(),
                value_sp: g.value_sp,
                produced: tot(|l| &l.production),
                local_use: tot(|l| &l.local_use),
                traded: tot(|l| &l.exports),
                stored: tot(|l| &l.stored),
                shortfall: tot(|l| &l.shortfall),
            }
        })
        .collect();
    let mut road_traffic: Vec<RoadTraffic> = traffic
        .into_iter()
        .map(|(road, by_good)| {
            let mut top: Vec<(String, u64)> = by_good.into_iter().collect();
            top.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            RoadTraffic {
                road,
                value_sp: top.iter().map(|t| t.1).sum(),
                goods: top.into_iter().take(3).map(|t| t.0).collect(),
            }
        })
        .collect();
    road_traffic.sort_by(|a, b| b.value_sp.cmp(&a.value_sp).then(a.road.cmp(&b.road)));
    EconomyRun {
        economy: Economy {
            goods,
            flows,
            road_traffic,
        },
        per_node,
    }
}

fn key_goods(handled: Option<&BTreeMap<String, [u64; 3]>>) -> Vec<KeyGood> {
    let Some(h) = handled else {
        return Vec::new();
    };
    let mut out: Vec<KeyGood> = h
        .iter()
        .map(|(good, &[ex, im, tr])| {
            let role = if ex >= im && ex >= tr {
                GoodRole::Export
            } else if im >= tr {
                GoodRole::Import
            } else {
                GoodRole::Transit
            };
            KeyGood {
                good: good.clone(),
                role,
                value_sp: ex + im + tr,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        b.value_sp
            .cmp(&a.value_sp)
            .then_with(|| a.good.cmp(&b.good))
    });
    out.truncate(4);
    out
}
