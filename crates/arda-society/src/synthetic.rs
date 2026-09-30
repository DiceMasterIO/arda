//! A synthetic world for the example and tests: three realms, forty
//! settlements and a road network on a 160 × 110 km map with a coast in the
//! west, a river across the middle, forest in the north and ore-bearing
//! hills in the east. Records have exactly the settlement stage's shape.

use crate::input::{Function, Realm, Road, RoadClass, Settlement, Tier, WorldSettlements};
use crate::names;
use crate::num::{dist_m, u32_of};
use crate::rng::Stream;
use crate::tables::Tables;
use std::collections::BTreeMap;

/// `(x_km, y_km, tier, realm)` of the seats and other towns.
const TOWNS: [(i64, i64, Tier, u64); 7] = [
    (14, 58, Tier::City, 1),
    (84, 46, Tier::Town, 2),
    (132, 82, Tier::Town, 3),
    (52, 74, Tier::Town, 1),
    (104, 60, Tier::Town, 2),
    (142, 34, Tier::Town, 3),
    (12, 96, Tier::Town, 1),
];
const CULTURES: [&str; 3] = ["coastal", "heartland", "highland"];

fn river_y(x_km: i64) -> i64 {
    // A meander from the city's estuary eastwards.
    58 - x_km / 8 + ((x_km * 7) % 23 - 11) / 3
}

/// Builds the synthetic world for `seed`.
///
/// # Errors
/// Returns the table error if the embedded name tables fail to load.
pub fn world(seed: u64) -> Result<WorldSettlements, crate::SocietyError> {
    let t = Tables::load()?;
    let mut rng = Stream::new(seed, "synthetic", 0, 0);
    let mut pts: Vec<(i64, i64, Tier)> =
        TOWNS.iter().map(|&(x, y, tier, _)| (x, y, tier)).collect();
    while pts.len() < 40 {
        let village = pts.len() < 7 + 14;
        let x = rng.range(4, 156);
        let y = rng.range(6, 106);
        let gap = if village { 9 } else { 5 };
        if pts.iter().all(|p| (p.0 - x).abs() + (p.1 - y).abs() >= gap) {
            pts.push((x, y, if village { Tier::Village } else { Tier::Hamlet }));
        }
    }
    let mut settlements = Vec::new();
    for (k, &(x, y, tier)) in pts.iter().enumerate() {
        let realm = nearest_seat(x, y);
        let id = u64::try_from(k + 1).unwrap_or(0);
        let mut r = Stream::new(seed, "syn_settlement", id, 0);
        let culture = CULTURES[usize::try_from(realm - 1).unwrap_or(0)];
        let mut name = names::place(&t.names, culture, &mut r);
        while settlements.iter().any(|s: &Settlement| s.name == name) {
            name = names::place(&t.names, culture, &mut r);
        }
        let mut s = describe(id, name, x, y, tier, realm, culture, &mut r);
        s.buildings = mix(&s);
        settlements.push(s);
    }
    let roads = roads(&settlements, &mut rng);
    let realms = (1..=3)
        .map(|id| {
            let mut r = Stream::new(seed, "syn_realm", id, 0);
            Realm {
                id,
                name: names::place(
                    &t.names,
                    CULTURES[usize::try_from(id - 1).unwrap_or(0)],
                    &mut r,
                ),
                seat: id,
                members: settlements
                    .iter()
                    .filter(|s| s.realm_id == id)
                    .map(|s| s.id)
                    .collect(),
            }
        })
        .collect();
    Ok(WorldSettlements {
        format_version: 1,
        settlements,
        roads,
        realms,
        buildings: Vec::new(),
    })
}

fn nearest_seat(x: i64, y: i64) -> u64 {
    TOWNS[..3]
        .iter()
        .min_by_key(|s| (s.0 - x).pow(2) + (s.1 - y).pow(2))
        .map_or(1, |s| s.3)
}

#[allow(clippy::too_many_arguments)]
fn describe(
    id: u64,
    name: String,
    x: i64,
    y: i64,
    tier: Tier,
    realm: u64,
    culture: &str,
    r: &mut Stream,
) -> Settlement {
    let river = (y - river_y(x)).abs() <= 3;
    let coast = x <= 16;
    let forest = y <= 26;
    let hills = x >= 112;
    let south = y >= 88;
    let pop = match tier {
        Tier::City => r.range(8000, 9500),
        Tier::Town => r.range(1000, 2600),
        Tier::Village => r.range(180, 900),
        Tier::Hamlet => r.range(24, 90),
    };
    let mut tags = Vec::new();
    let mut tag = |c: bool, t: &str| {
        if c {
            tags.push(t.to_string());
        }
    };
    tag(river, "river");
    tag(river && r.chance(400), "ford");
    tag(river && tier.is_urban(), "bridge_site");
    tag(river && x < 40, "navigable");
    tag(river && r.chance(200), "marsh");
    tag(coast, "coast");
    tag(coast && tier != Tier::Hamlet, "harbour");
    tag(coast || (river && r.chance(500)), "fish");
    tag(coast && r.chance(500), "salt");
    tag(forest, "forest");
    tag(forest, "timber");
    tag(hills, "hill");
    tag(hills && r.chance(600), "ore");
    tag(hills && r.chance(300), "defensible");
    tag(hills && y < 50 && r.chance(500), "pass");
    tag(r.chance(150), "spring");
    let urban = tier.is_urban();
    let mut f = Vec::new();
    let mut add = |c: bool, func: Function| {
        if c {
            f.push(func);
        }
    };
    add(!forest || r.chance(400), Function::Farming);
    add(hills || forest || r.chance(350), Function::Pastoral);
    add(tags.iter().any(|t| t == "fish"), Function::Fishing);
    add(
        tier != Tier::Hamlet
            && (tags.iter().any(|t| t == "harbour")
                || (urban && tags.iter().any(|t| t == "navigable"))),
        Function::Port,
    );
    add(
        urban || (tier == Tier::Village && pop >= 450),
        Function::Market,
    );
    add(
        tags.iter().any(|t| t == "ore") && tier != Tier::City,
        Function::Mining,
    );
    add(forest, Function::Logging);
    add(urban, Function::Crafting);
    add(
        id == 3 || (urban && tags.iter().any(|t| t == "defensible" || t == "pass")),
        Function::Fortress,
    );
    add(
        matches!(tier, Tier::Village | Tier::Town) && id % 9 == 4,
        Function::Abbey,
    );
    add(
        tags.iter().any(|t| t == "ford" || t == "bridge_site"),
        Function::Crossing,
    );
    add(id <= 3, Function::Capital);
    if f.is_empty() {
        f.push(Function::Pastoral);
    }
    f.sort_unstable();
    let biome = if hills {
        "highland"
    } else if forest {
        "temperate_forest"
    } else if coast {
        "coastal"
    } else if south {
        "warm_temperate"
    } else {
        "temperate"
    };
    let base = match tier {
        Tier::Hamlet => 45,
        Tier::Village => 85,
        Tier::Town => 135,
        Tier::City => 180,
    };
    let bonus = |func: Function, v: i64| if f.contains(&func) { v } else { 0 };
    let wealth = base
        + bonus(Function::Port, 25)
        + bonus(Function::Market, 10)
        + bonus(Function::Mining, 15)
        + bonus(Function::Capital, 20)
        + r.range(-12, 12);
    Settlement {
        id,
        name,
        tier,
        population: u32_of(pop),
        functions: f,
        wealth: crate::num::u8_of(wealth),
        culture: culture.to_string(),
        realm_id: realm,
        biome: biome.to_string(),
        coastal: coast,
        riverine: river,
        x_m: x * 1000,
        y_m: y * 1000,
        cell_x: u32_of(x * 10),
        cell_y: u32_of(y * 10),
        height_m: crate::num::i32_of(if hills { 300 + x } else { 20 + x / 2 }),
        rank: 0,
        site_tags: tags,
        history: String::new(),
        buildings: BTreeMap::new(),
    }
}

/// Building-mix estimate in the settlement stage's style.
fn mix(s: &Settlement) -> BTreeMap<String, u32> {
    let p = s.population;
    let mut b: BTreeMap<String, u32> = BTreeMap::new();
    let mut put = |k: &str, n: u32| {
        if n > 0 {
            *b.entry(k.to_string()).or_default() += n;
        }
    };
    match s.tier {
        Tier::Hamlet => {
            put("farmhouse", (p / 6).max(2));
            put("cottage", p / 12);
        }
        Tier::Village => {
            put("farmhouse", p / 10);
            put("cottage", p / 8);
            put("shrine", 1);
            put("inn", u32::from(p >= 250));
            put("tavern", 1);
            put("smithy", 1);
            put("manor", u32::from(p >= 400));
        }
        Tier::Town | Tier::City => {
            put("house", p / 6);
            put("temple", 1 + p / 4000);
            put("inn", 1 + p / 1500);
            put("tavern", p / 1000);
            put("smithy", 1 + p / 1500);
            put("bakery", p / 800);
            put("brewery", 1 + p / 5000);
            put("stable", 1 + p / 2500);
            put("tannery", 1);
            put("apothecary", 1 + p / 5000);
            put("school", u32::from(p >= 3000));
            put("library", u32::from(s.tier == Tier::City));
            put("manor", 1 + p / 6000);
        }
    }
    let urban = s.tier.is_urban();
    if s.has(Function::Farming) && s.tier != Tier::Hamlet {
        put("mill", 1 + p / 3000);
    }
    if s.has(Function::Market) {
        put("market_hall", u32::from(urban));
        put("stall", if urban { p / 150 } else { 4 });
    }
    if s.has(Function::Port) {
        put("dock", 1 + p / 2000);
        put("warehouse", if urban { 1 + p / 1500 } else { 1 });
    }
    if s.has(Function::Fishing) {
        put("boathouse", (1 + p / 300).min(6));
    }
    if s.has(Function::Fortress) || s.has(Function::Capital) {
        put("keep", 1);
        put("barracks", u32::from(urban));
        put("guardhouse", 1 + p / 3000);
    }
    if s.has(Function::Mining) {
        put("mine", (1 + p / 400).min(4));
    }
    if s.has(Function::Logging) {
        put("lumber_camp", 1 + p / 1500);
    }
    if s.has(Function::Crafting) {
        put("workshop", p / 400);
    }
    if s.has(Function::Abbey) {
        put("temple", 1);
        put("library", 1);
    }
    b
}

fn roads(s: &[Settlement], rng: &mut Stream) -> Vec<Road> {
    let d = |a: &Settlement, b: &Settlement| dist_m(a.x_m, a.y_m, b.x_m, b.y_m);
    let mut out: Vec<Road> = Vec::new();
    let link = |out: &mut Vec<Road>,
                a: &Settlement,
                b: &Settlement,
                class: RoadClass,
                rng: &mut Stream| {
        if out.iter().any(|r| {
            (r.from == a.id && r.to == Some(b.id)) || (r.from == b.id && r.to == Some(a.id))
        }) {
            return;
        }
        let straight = d(a, b);
        let length = straight * crate::num::u64_of(rng.range(118, 136)) / 100;
        out.push(Road {
            id: u64::try_from(out.len() + 1).unwrap_or(0),
            class,
            from: a.id,
            to: Some(b.id),
            to_edge: None,
            length_m: length,
            straight_m: straight,
            new_m: length,
            segments: vec![vec![[a.x_m, a.y_m], [b.x_m, b.y_m]]],
        });
    };
    // Highways: a minimum spanning tree over the towns, plus one extra link.
    let urban: Vec<&Settlement> = s.iter().filter(|x| x.tier.is_urban()).collect();
    let mut joined = vec![urban[0]];
    while joined.len() < urban.len() {
        let best = urban
            .iter()
            .filter(|u| !joined.iter().any(|j| j.id == u.id))
            .flat_map(|u| joined.iter().map(move |j| (d(u, j), u.id, *u, *j)))
            .min_by_key(|x| (x.0, x.1));
        let Some((_, _, u, j)) = best else { break };
        link(&mut out, j, u, RoadClass::Highway, rng);
        joined.push(u);
    }
    link(&mut out, urban[3], urban[4], RoadClass::Highway, rng);
    // Villages to the nearest town or already linked village; hamlets by track.
    for tier in [Tier::Village, Tier::Hamlet] {
        for a in s.iter().filter(|x| x.tier == tier) {
            let target = s
                .iter()
                .filter(|b| b.id != a.id && (b.tier > tier || (b.tier == tier && b.id < a.id)))
                .min_by_key(|b| (d(a, b), b.id));
            if let Some(b) = target {
                let class = if tier == Tier::Village {
                    RoadClass::Road
                } else {
                    RoadClass::Track
                };
                link(&mut out, b, a, class, rng);
            }
        }
    }
    // Footpaths between close neighbours.
    for a in s.iter().filter(|x| x.tier == Tier::Hamlet) {
        if let Some(b) = s
            .iter()
            .filter(|b| b.id != a.id)
            .min_by_key(|b| (d(a, b), b.id))
        {
            if d(a, b) < 9000 {
                link(&mut out, a, b, RoadClass::Footpath, rng);
            }
        }
    }
    let edge = s.iter().max_by_key(|x| (x.x_m, x.id)).map_or(1, |x| x.id);
    out.push(Road {
        id: u64::try_from(out.len() + 1).unwrap_or(0),
        class: RoadClass::Road,
        from: edge,
        to: None,
        to_edge: Some("east".to_string()),
        length_m: 9000,
        straight_m: 8000,
        new_m: 9000,
        segments: Vec::new(),
    });
    out
}
