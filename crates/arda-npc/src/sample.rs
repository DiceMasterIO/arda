//! Plausible sample settlements, standing in for the building stage until it
//! exists. Used by the example and the tests; deterministic in the seed.

use crate::input::{
    Biome, BuildingFunction as F, BuildingId, BuildingSpec, Craft, RealmId,
    SettlementFunction as S, SettlementId, SettlementProfile, Tier,
};
use crate::rng::{Rng, SeedKey};

/// Resident capacity of each building function in the samples.
fn capacity(function: F) -> u16 {
    match function {
        F::House => 6,
        F::Farmhouse => 7,
        F::Cottage => 4,
        F::Inn | F::Manor => 8,
        F::Keep => 14,
        F::Barracks => 20,
        F::Tavern
        | F::Temple
        | F::Guardhouse
        | F::Workshop
        | F::Bakery
        | F::Brewery
        | F::Tannery
        | F::Apothecary
        | F::School => 4,
        F::Smithy | F::Mill => 5,
        F::Shrine | F::Stable | F::Boathouse | F::Library => 3,
        F::Mine | F::LumberCamp => 6,
        F::MarketHall | F::Stall | F::Warehouse | F::Dock => 0,
        F::Market | F::Barn | F::Street | F::Farm => 0,
    }
}

/// Workplace slots: the master plus one per listed worker in the
/// occupation table (see `data/content/occupations.json`).
fn slots(function: F) -> u16 {
    match function {
        F::House | F::Farmhouse | F::Cottage => 0,
        F::Smithy | F::Workshop | F::Stable => 3,
        F::Inn | F::Warehouse | F::Dock => 5,
        F::Tavern | F::Temple | F::MarketHall | F::Guardhouse | F::Library => 4,
        F::Shrine | F::Boathouse | F::Brewery => 3,
        F::Keep => 7,
        F::Barracks => 12,
        F::Manor | F::LumberCamp => 5,
        F::Mine => 8,
        F::Mill | F::Stall | F::Bakery | F::Tannery | F::Apothecary | F::School => 2,
        F::Market | F::Barn | F::Street | F::Farm => 0,
    }
}

const CRAFTS: [Craft; 12] = [
    Craft::Carpentry,
    Craft::Weaving,
    Craft::Pottery,
    Craft::Cobbling,
    Craft::Masonry,
    Craft::Leatherwork,
    Craft::Woodcarving,
    Craft::Tinkering,
    Craft::Jewellery,
    Craft::Glassblowing,
    Craft::Painting,
    Craft::Cartography,
];

/// How many of each workplace a settlement of this size and kind has.
fn workplaces(tier: Tier, population: u32, functions: &[S], wealth: u8) -> Vec<(F, u32)> {
    let has = |f: S| functions.contains(&f);
    let p = population;
    let town = tier >= Tier::Town;
    let village = tier >= Tier::Village;
    let per = |n: u32| p / n;
    let mut out = vec![
        (
            F::Smithy,
            if village {
                (per(450)).max(1)
            } else {
                u32::from(p >= 60)
            },
        ),
        (
            F::Inn,
            if town {
                per(900).max(1)
            } else {
                u32::from(village)
            },
        ),
        (F::Tavern, if town { per(600) } else { u32::from(p >= 300) }),
        (F::Temple, if town { 1 + per(6000) } else { 0 }),
        (
            F::Shrine,
            if town {
                1 + per(5000)
            } else {
                u32::from(p >= 50)
            },
        ),
        (
            F::Mill,
            if has(S::Farming) && village {
                1 + per(1500)
            } else {
                0
            },
        ),
        (
            F::MarketHall,
            u32::from(has(S::Market) && village) + u32::from(tier == Tier::City),
        ),
        (F::Stall, if has(S::Market) && town { per(250) } else { 0 }),
        (
            F::Warehouse,
            if town && (has(S::Market) || has(S::Port)) {
                per(500).max(1)
            } else {
                0
            },
        ),
        (F::Dock, if has(S::Port) { 1 + per(1500) } else { 0 }),
        (
            F::Boathouse,
            if has(S::Fishing) || has(S::Port) {
                1 + per(700)
            } else {
                0
            },
        ),
        (F::Keep, u32::from(town || has(S::Fortress))),
        (
            F::Barracks,
            if has(S::Fortress) || has(S::Capital) || tier == Tier::City {
                1 + per(10_000)
            } else {
                0
            },
        ),
        (F::Guardhouse, if town { per(1500).max(1) } else { 0 }),
        (
            F::Manor,
            if town {
                1 + per(4000)
            } else {
                u32::from(village && wealth > 140)
            },
        ),
        (
            F::Workshop,
            if has(S::Crafting) {
                per(200)
            } else if town {
                per(400)
            } else {
                u32::from(village)
            },
        ),
        (F::Mine, if has(S::Mining) { 1 + per(400) } else { 0 }),
        (
            F::LumberCamp,
            if has(S::Logging) { 1 + per(500) } else { 0 },
        ),
        (
            F::Stable,
            if town {
                1 + per(5000)
            } else {
                u32::from(has(S::Crossing))
            },
        ),
        (F::Bakery, if town { per(500) } else { u32::from(p >= 400) }),
        (F::Brewery, if town { 1 + per(2500) } else { 0 }),
        (F::Tannery, if town { 1 + per(8000) } else { 0 }),
        (F::Apothecary, if town { 1 + per(5000) } else { 0 }),
        (
            F::Library,
            if tier == Tier::City {
                1 + u32::from(has(S::Capital))
            } else {
                0
            },
        ),
        (
            F::School,
            if town && wealth > 100 {
                1 + per(8000)
            } else {
                0
            },
        ),
    ];
    out.retain(|&(_, n)| n > 0);
    out
}

/// A settlement profile with sample defaults (realm 1, temperate biome,
/// coastal when it has a port, riverine when it fishes or has a crossing).
#[must_use]
pub fn profile(
    id: u64,
    name: &str,
    tier: Tier,
    population: u32,
    functions: &[S],
    wealth: u8,
    culture: &str,
) -> SettlementProfile {
    SettlementProfile {
        id: SettlementId(id),
        name: name.to_string(),
        tier,
        population,
        functions: functions.to_vec(),
        wealth,
        culture: culture.to_string(),
        realm_id: RealmId(1),
        biome: Biome::Temperate,
        coastal: functions.contains(&S::Port),
        riverine: functions.contains(&S::Fishing) || functions.contains(&S::Crossing),
        ancestry_mix: None,
        tongue: None,
    }
}

/// Sample buildings for a profile: workplaces by tier, size and functions,
/// then dwellings until everyone has a home with about 8% to spare.
#[must_use]
pub fn buildings(seed: u64, profile: &SettlementProfile) -> Vec<BuildingSpec> {
    let (sid, tier, population, wealth) =
        (profile.id, profile.tier, profile.population, profile.wealth);
    let functions = &profile.functions;
    let mut rng = SeedKey::settlement(seed, sid).rng("sample-buildings");
    let mut buildings: Vec<BuildingSpec> = Vec::new();
    let mut craft = 0;
    for (function, count) in workplaces(tier, population, functions, wealth) {
        for _ in 0..count {
            let tags = if function == F::Workshop {
                craft += 1;
                BuildingSpec::craft_tags(CRAFTS[(craft - 1) % CRAFTS.len()])
            } else {
                Vec::new()
            };
            push(&mut buildings, sid, wealth, function, tags, &mut rng);
        }
    }
    // Dwellings: farmhouses and cottages in the country, houses in town,
    // until capacity exceeds the population by about 8%.
    let (farm_pm, cottage_pm) = match tier {
        Tier::Hamlet | Tier::Village => (600, 300),
        Tier::Town => (100, 200),
        Tier::City => (20, 100),
    };
    let target = population + population / 12 + 4;
    let mut room: u32 = buildings.iter().map(|b| u32::from(b.capacity)).sum();
    while room < target {
        let roll = rng.below(1000);
        let f = if roll < farm_pm {
            F::Farmhouse
        } else if roll < farm_pm + cottage_pm {
            F::Cottage
        } else {
            F::House
        };
        room += u32::from(capacity(f));
        push(&mut buildings, sid, wealth, f, Vec::new(), &mut rng);
    }
    buildings
}

fn push(
    buildings: &mut Vec<BuildingSpec>,
    settlement_id: SettlementId,
    wealth: u8,
    function: F,
    tags: Vec<String>,
    rng: &mut Rng,
) {
    let spread = rng.range(0, 60);
    let wealth =
        u8::try_from((u32::from(wealth) + spread).saturating_sub(30).min(255)).unwrap_or(wealth);
    let n = u64::try_from(buildings.len()).unwrap_or(0);
    buildings.push(BuildingSpec {
        id: BuildingId(n + 1),
        settlement_id,
        function,
        tags,
        capacity: capacity(function),
        workplace_slots: slots(function),
        wealth,
    });
}

/// World seed of the market-town example.
pub const MARKET_TOWN_SEED: u64 = 0x00A2_DA5E;

/// The market-town example (`examples/town.rs`, `GET /v1/npc/demo`):
/// Wendlebrook, a heartland port and market town of 1,500 people, with its
/// sample buildings. Returns the world seed, profile and buildings.
#[must_use]
pub fn market_town() -> (u64, SettlementProfile, Vec<BuildingSpec>) {
    let functions = [S::Market, S::Port, S::Crafting, S::Farming, S::Fishing];
    let town = profile(
        7,
        "Wendlebrook",
        Tier::Town,
        1_500,
        &functions,
        150,
        "heartland",
    );
    let buildings = buildings(MARKET_TOWN_SEED, &town);
    (MARKET_TOWN_SEED, town, buildings)
}
