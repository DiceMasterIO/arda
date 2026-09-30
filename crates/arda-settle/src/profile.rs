//! Functions, wealth, biome, culture and building mix per settlement (spec
//! step 3, goal 36).

use crate::culture::CultureMap;
use crate::grid::Grid;
use crate::model::{Biome, Function, Settlement, Tier};
use crate::num::{sat_u32, sat_u8};
use crate::ore;
use crate::rng::hash;
use crate::suitability::Suitability;
use crate::tags::{self, Sites};
use arda_ids::BuildingFunction as F;
use std::collections::{BTreeMap, BTreeSet};

/// Ore within this many cells makes a mining settlement.
const ORE_REACH: i64 = 15;

/// The settlements that mine: for each mineral district, the
/// [`ore::MINERS_PER_DISTRICT`] non-city settlements nearest its ore
/// within [`ORE_REACH`] cells (ties by id).
#[must_use]
pub fn miners(g: &Grid, sites: &Sites, seed: u64, settlements: &[Settlement]) -> BTreeSet<u64> {
    let mut by_district: BTreeMap<(i64, i64), Vec<(i64, u64)>> = BTreeMap::new();
    for s in settlements.iter().filter(|s| s.tier != Tier::City) {
        let (x, y) = (i64::from(s.cell_x), i64::from(s.cell_y));
        let nearest = (-ORE_REACH..=ORE_REACH)
            .flat_map(|oy| (-ORE_REACH..=ORE_REACH).map(move |ox| (ox, oy)))
            .filter(|&(ox, oy)| ox * ox + oy * oy <= ORE_REACH * ORE_REACH)
            .filter(|&(ox, oy)| {
                g.at(x + ox, y + oy)
                    .is_some_and(|j| sites.tags[j] & tags::ORE != 0)
            })
            .min_by_key(|&(ox, oy)| (ox * ox + oy * oy, oy, ox));
        let Some((ox, oy)) = nearest else { continue };
        if let Some(d) = ore::district_of(seed, x + ox, y + oy) {
            by_district
                .entry(d)
                .or_default()
                .push((ox * ox + oy * oy, s.id.get()));
        }
    }
    let mut out = BTreeSet::new();
    for mut v in by_district.into_values() {
        v.sort_unstable();
        out.extend(
            v.into_iter()
                .take(ore::MINERS_PER_DISTRICT)
                .map(|(_, id)| id),
        );
    }
    out
}

/// Fills functions, wealth, biome, culture, coast and river flags and the
/// building mix. Capital is added later by the realm stage.
pub fn describe(
    g: &Grid,
    suit: &Suitability,
    cultures: &CultureMap,
    seed: u64,
    miner: bool,
    s: &mut Settlement,
) {
    let i = s.index(g.width);
    let t = s.tag_bits;
    let (x, y) = (i64::from(s.cell_x), i64::from(s.cell_y));
    let arable = i64::from(suit.arable_pm[i]);
    let has = |bit: u32| t & bit != 0;
    let urban = s.tier.is_urban();
    let mut f = Vec::new();
    let mut add = |cond: bool, func: Function| {
        if cond {
            f.push(func);
        }
    };
    add(arable >= 150, Function::Farming);
    add(arable < 350, Function::Pastoral);
    add(has(tags::FISH), Function::Fishing);
    add(
        s.tier != Tier::Hamlet
            && (has(tags::HARBOUR) || has(tags::ESTUARY) || (urban && has(tags::NAVIGABLE))),
        Function::Port,
    );
    add(
        urban || (s.tier == Tier::Village && s.population >= 450),
        Function::Market,
    );
    add(miner, Function::Mining);
    add(has(tags::TIMBER), Function::Logging);
    add(urban, Function::Crafting);
    add(
        (urban && (has(tags::HILL) || has(tags::PASS)))
            || (s.tier == Tier::Village && has(tags::DEFENSIBLE) && has(tags::PASS)),
        Function::Fortress,
    );
    let key = u64::try_from(i).unwrap_or(0);
    add(
        matches!(s.tier, Tier::Village | Tier::Town)
            && !has(tags::HARBOUR)
            && hash(seed, "abbey", key, 0).is_multiple_of(14),
        Function::Abbey,
    );
    add(
        has(tags::FORD) || has(tags::BRIDGE) || has(tags::CONFLUENCE),
        Function::Crossing,
    );
    if f.is_empty() {
        f.push(Function::Pastoral);
    }
    f.sort_unstable();
    s.functions = f;
    s.coastal = has(tags::COAST) || has(tags::HARBOUR);
    s.riverine = has(tags::RIVER) || has(tags::NAVIGABLE);
    s.culture = cultures.at(x, y).key().to_string();
    s.biome = biome(g, t, i);
    s.wealth = wealth(s, arable, seed);
    s.buildings = buildings(s);
}

/// Makes a settlement a realm seat (capital and fortress) and refreshes its buildings.
pub fn make_capital(s: &mut Settlement) {
    // A seat holds its realm from a castle.
    for f in [Function::Capital, Function::Fortress] {
        if !s.has(f) {
            s.functions.push(f);
        }
    }
    s.functions.sort_unstable();
    s.wealth = s.wealth.saturating_add(20);
    s.buildings = buildings(s);
}

fn biome(g: &Grid, t: u32, i: usize) -> Biome {
    let temp = g.temp_cc[i];
    if t & tags::MOUNTAIN != 0 {
        if temp < 500 {
            Biome::Alpine
        } else {
            Biome::Highland
        }
    } else if t & tags::MARSH != 0 {
        Biome::Wetland
    } else if g.moisture[i] < 100 {
        Biome::Steppe
    } else if t & tags::FOREST != 0 {
        if temp < 600 {
            Biome::BorealForest
        } else {
            Biome::TemperateForest
        }
    } else if t & tags::COAST != 0 {
        Biome::Coastal
    } else if temp >= 1500 {
        Biome::WarmTemperate
    } else {
        Biome::Temperate
    }
}

fn wealth(s: &Settlement, arable: i64, seed: u64) -> u8 {
    let base = match s.tier {
        Tier::Hamlet => 45,
        Tier::Village => 85,
        Tier::Town => 135,
        Tier::City => 180,
    };
    let bonus = |f: Function, v: i64| if s.has(f) { v } else { 0 };
    let noise = i64::try_from(hash(seed, "wealth", s.id.get(), 0) % 21).unwrap_or(0) - 10;
    sat_u8(
        base + arable / 40
            + bonus(Function::Port, 25)
            + bonus(Function::Market, 10)
            + bonus(Function::Mining, 15)
            + bonus(Function::Crafting, 10)
            + bonus(Function::Abbey, 5)
            - if arable < 150 { 10 } else { 0 }
            + noise,
    )
}

/// Estimated building mix, by the shared building functions (I6).
#[must_use]
pub fn buildings(s: &Settlement) -> BTreeMap<F, u32> {
    let p = s.population;
    let mut b: BTreeMap<F, u32> = BTreeMap::new();
    let mut put = |k: F, n: u32| {
        if n > 0 {
            *b.entry(k).or_default() += n;
        }
    };
    let urban = s.tier.is_urban();
    match s.tier {
        Tier::Hamlet => {
            put(F::Farmhouse, (p / 6).max(2));
            put(F::Cottage, p / 12);
            put(F::Barn, (p / 12).max(1));
        }
        Tier::Village => {
            put(F::Farmhouse, p / 10);
            put(F::Cottage, p / 8);
            put(F::Barn, p / 20);
            put(F::Shrine, 1);
            put(F::Inn, u32::from(p >= 250));
            put(F::Tavern, 1);
            put(F::Smithy, 1);
            put(F::Manor, u32::from(p >= 400));
        }
        Tier::Town | Tier::City => {
            put(F::House, p / 6);
            put(F::Cottage, p / 40);
            put(F::Temple, 1 + p / 4000);
            put(F::Inn, 1 + p / 1500);
            put(F::Tavern, p / 1000);
            put(F::Smithy, 1 + p / 1500);
            put(F::Bakery, p / 800);
            put(F::Brewery, 1 + p / 5000);
            put(F::Stable, 1 + p / 2500);
            put(F::Tannery, 1);
            put(F::Apothecary, 1 + p / 5000);
            put(F::School, u32::from(p >= 3000));
            put(F::Library, u32::from(s.tier == Tier::City));
            put(F::Manor, 1 + p / 6000);
        }
    }
    let has = |f: Function| s.has(f);
    if has(Function::Farming) && s.tier != Tier::Hamlet {
        put(F::Mill, 1 + p / 3000);
    }
    if has(Function::Market) {
        put(F::MarketHall, u32::from(urban));
        put(F::Stall, if urban { p / 150 } else { 4 });
    }
    if has(Function::Port) {
        put(F::Dock, 1 + p / 2000);
        put(F::Warehouse, if urban { 1 + p / 1500 } else { 1 });
    }
    if has(Function::Fishing) {
        put(F::Boathouse, (1 + p / 300).min(6));
    }
    if has(Function::Fortress) || has(Function::Capital) {
        put(F::Keep, 1);
        put(F::Barracks, u32::from(urban));
        put(F::Guardhouse, 1 + p / 3000);
    }
    if has(Function::Capital) {
        put(F::Manor, 2);
        put(F::Library, 1);
    }
    if has(Function::Mining) {
        put(F::Mine, (1 + p / 400).min(4));
    }
    if has(Function::Logging) {
        put(F::LumberCamp, 1 + sat_u32(i64::from(p / 1500)));
    }
    if has(Function::Crafting) {
        put(F::Workshop, p / 400);
    }
    if has(Function::Abbey) {
        put(F::Temple, 1);
        put(F::Library, 1);
    }
    b
}
