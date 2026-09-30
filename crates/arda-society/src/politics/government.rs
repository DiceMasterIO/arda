//! Government type and rank per realm, chosen from what the realm is
//! (a trading seat leans republican, many abbeys lean theocratic).

use crate::ctx::Ctx;
use crate::economy::EconomyRun;
use crate::input::{Function, Tier};
use crate::num::i64_of;
use crate::rng::hash;

/// Government key and rank key of one realm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regime {
    /// Government key in `politics.json`.
    pub government: String,
    /// `kingdom`, `duchy` or `county`.
    pub rank: String,
}

/// One regime per realm, in [`Ctx::realms`] order.
#[must_use]
pub fn choose(ctx: &Ctx<'_>, econ: &EconomyRun) -> Vec<Regime> {
    ctx.realms
        .iter()
        .map(|r| {
            let seat = ctx.s(r.seat);
            let m = r.members.len();
            let towns = r
                .members
                .iter()
                .filter(|&&i| ctx.s(i).tier.is_urban())
                .count();
            let rank = if seat.tier == Tier::City && m >= 12 {
                "kingdom"
            } else if seat.tier.is_urban() && m >= 6 {
                "duchy"
            } else {
                "county"
            };
            let abbeys = r
                .members
                .iter()
                .filter(|&&i| ctx.s(i).has(Function::Abbey))
                .count();
            let trade = econ.per_node[r.seat].trade_balance_sp.abs()
                + i64_of(
                    econ.per_node[r.seat]
                        .key_goods
                        .iter()
                        .map(|k| k.value_sp)
                        .sum::<u64>(),
                );
            let noise = |k: u64| i64_of(hash(ctx.seed, "government", r.id, k) % 45);
            let port_seat = seat.has(Function::Port) && seat.has(Function::Market);
            let mut options: Vec<(i64, &str)> = vec![(60 + noise(0), "monarchy")];
            if port_seat {
                options.push((25 + (trade / 4000).min(40) + noise(1), "merchant_republic"));
            }
            if abbeys > 0 || seat.has(Function::Abbey) {
                let n = i64::try_from(abbeys).unwrap_or(0);
                options.push((20 + n * 18 + noise(2), "theocracy"));
            }
            if towns >= 4 {
                let n = i64::try_from(towns).unwrap_or(0);
                options.push((22 + n * 4 + noise(3), "elective_crown"));
            }
            let government = options
                .into_iter()
                .filter(|(_, k)| ctx.t.politics.governments.contains_key(*k))
                .max_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(a.1)))
                .map_or("monarchy", |o| o.1);
            Regime {
                government: government.to_string(),
                rank: rank.to_string(),
            }
        })
        .collect()
}
