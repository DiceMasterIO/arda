//! Production and demand per settlement from functions, land use, buildings
//! and site tags (deliverable 1; goal 39 land use).

use crate::ctx::Ctx;
use std::collections::BTreeMap;

/// Site yields (salt pans, ore outcrops) stop growing with population here.
const SITE_CAP: u64 = 1000;

/// Loads per good.
pub type Basket = BTreeMap<String, u64>;

fn add(b: &mut Basket, good: &str, amount: u64) {
    if amount > 0 {
        *b.entry(good.to_string()).or_default() += amount;
    }
}

/// Yearly production of node `i`, loads per good.
#[must_use]
pub fn production(ctx: &Ctx<'_>, i: usize) -> Basket {
    let g = &ctx.t.goods;
    let s = ctx.s(i);
    let pop = u64::from(s.population);
    let share = u64::from(g.land_share_pct.get(s.tier.key()).copied().unwrap_or(50));
    let land_workers = pop * share / 100;
    let biome = g.biomes.get(&s.biome).cloned().unwrap_or_default();
    let mult = |good: &str| u64::from(biome.mult_pct.get(good).copied().unwrap_or(100));
    let land: Vec<(&str, u64)> = s
        .functions
        .iter()
        .filter_map(|f| {
            let w = g.function_weight.get(f.key()).copied()?;
            g.land
                .contains_key(f.key())
                .then_some((f.key(), u64::from(w)))
        })
        .collect();
    let total_w: u64 = land.iter().map(|&(_, w)| w).sum();
    let mut out = Basket::new();
    for &(func, w) in &land {
        let workers = land_workers * w / total_w.max(1);
        for y in g.land.get(func).map_or(&[][..], Vec::as_slice) {
            add(
                &mut out,
                &y.0,
                workers * u64::from(y.1) * mult(&y.0) / 10_000,
            );
        }
        if func == "farming" {
            for y in &biome.extra {
                add(&mut out, &y.0, workers * u64::from(y.1) / 100);
            }
        }
    }
    for b in ctx.buildings_of(i) {
        let yields = match (b.craft(), b.function.as_str()) {
            (Some(craft), "workshop") => g.crafts.get(craft),
            (_, func) => g.buildings.get(func),
        };
        for y in yields.map_or(&[][..], Vec::as_slice) {
            add(&mut out, &y.0, u64::from(y.1));
        }
    }
    for tag in &s.site_tags {
        for y in g.sites.get(tag).map_or(&[][..], Vec::as_slice) {
            add(&mut out, &y.0, pop.min(SITE_CAP) * u64::from(y.1) / 100);
        }
    }
    out
}

/// Yearly demand of node `i` given its production (processing inputs
/// included: looms want wool, forges want ore).
#[must_use]
pub fn demand(ctx: &Ctx<'_>, i: usize, prod: &Basket) -> Basket {
    let s = ctx.s(i);
    let pop = u64::from(s.population);
    let mut out = Basket::new();
    for good in &ctx.t.goods.goods {
        if good.urban_only && !s.tier.is_urban() {
            continue;
        }
        let mut d = pop * u64::from(good.demand_per_100) / 100;
        if good.wealth_scaled {
            d = d * u64::from(s.wealth) / 128;
        }
        add(&mut out, &good.key, d);
    }
    for (output, input) in &ctx.t.goods.inputs {
        let made = prod.get(output).copied().unwrap_or(0);
        add(&mut out, &input.good, made * u64::from(input.per) / 10);
    }
    out
}
