//! Prosperity over time. Each settlement grows from a modest start towards
//! its present wealth, knocked back by the disasters and wars that hit it;
//! each shock fades over a generation. The last sample is the present wealth
//! exactly, so the curve ends in the input world.

use super::founding::Founding;
use super::{EventKind, Timeline};
use crate::ctx::Ctx;
use crate::economy::EconomyRun;
use crate::num::u8_of;
use serde::{Deserialize, Serialize};

/// Sample spacing, years.
const STEP: i32 = 25;
/// Years for a shock to fade.
const FADE: i64 = 35;

/// Direction of a settlement's fortunes now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Trend {
    /// Trade surplus, growing.
    Rising,
    /// Holding steady.
    Stable,
    /// Trade deficit or war, shrinking.
    Declining,
    /// Climbing back after a recent blow.
    Recovering,
}

/// Prosperity history of one settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Prosperity {
    /// Present wealth, 0–255 (the input value).
    pub present: u8,
    /// Current direction.
    pub trend: Trend,
    /// `[year, wealth]` samples from founding to present.
    pub samples: Vec<[i32; 2]>,
}

fn shock(kind: EventKind) -> i64 {
    match kind {
        EventKind::Plague => 34,
        EventKind::BorderShift => 26,
        EventKind::Fire | EventKind::MineCollapse => 22,
        EventKind::Flood => 18,
        EventKind::Famine => 16,
        _ => 0,
    }
}

/// Prosperity per node; `at_war` lists realm ids at war now.
#[must_use]
pub fn prosperity(
    ctx: &Ctx<'_>,
    f: &Founding,
    tl: &Timeline,
    econ: &EconomyRun,
    at_war: &[u64],
) -> Vec<Prosperity> {
    // Each settlement's shocks in timeline order, gathered in one pass
    // (review round 2 #29: a scan of every event per settlement).
    let mut shocks: std::collections::BTreeMap<u64, Vec<(i32, i64)>> = Default::default();
    for e in &tl.events {
        let hit = (e.year, shock(e.kind) * i64::from(e.severity.max(1)));
        if hit.1 <= 0 {
            continue;
        }
        for (k, id) in e.settlements.iter().enumerate() {
            if !e.settlements[..k].contains(id) {
                shocks.entry(*id).or_default().push(hit);
            }
        }
    }
    (0..ctx.n())
        .map(|i| {
            let s = ctx.s(i);
            let hits: &[(i32, i64)] = shocks.get(&s.id).map_or(&[], Vec::as_slice);
            let w = i64::from(s.wealth);
            let start = w * 35 / 100 + 10;
            let (f0, p) = (f.year[i], ctx.present);
            let span = i64::from((p - f0).max(1));
            let at = |y: i32| -> i64 {
                let base = start + (w - start) * i64::from(y - f0) / span;
                let loss: i64 = hits
                    .iter()
                    .filter(|h| h.0 <= y)
                    .map(|h| h.1 * (FADE - i64::from(y - h.0)).max(0) / FADE)
                    .sum();
                base - loss
            };
            let mut samples = Vec::new();
            let mut y = f0;
            while y < p {
                samples.push([y, i32::from(u8_of(at(y).max(1)))]);
                y = (y / STEP + 1) * STEP;
            }
            samples.push([p, i32::from(s.wealth)]);
            let recent = hits.iter().any(|h| p - h.0 <= 15 && h.1 >= 30);
            let per_head = econ.per_node[i].trade_balance_sp / i64::from(s.population.max(1));
            // Towns live on services the ledger does not count, so a trade
            // deficit only means decline when it is deep.
            let slack = if s.tier.is_urban() { 12 } else { 2 };
            let war_border = at_war.contains(&s.realm_id) && ctx.is_border(i);
            let trend = if recent {
                Trend::Recovering
            } else if war_border || per_head < -slack {
                Trend::Declining
            } else if per_head > slack / 2 {
                Trend::Rising
            } else {
                Trend::Stable
            };
            Prosperity {
                present: s.wealth,
                trend,
                samples,
            }
        })
        .collect()
}
