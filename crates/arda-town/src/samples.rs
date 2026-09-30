//! Synthetic sites for tests and renders: a riverside market town, a
//! crossroads village, a walled hilltop town and a fishing hamlet.
//!
//! Roads are given as coarse polylines through 100 m cell centres, exactly
//! as `arda-settle` writes them, so the planner's smoothing is exercised.

use crate::geom::{self, v2, Vec2};
use crate::num::floor_i;
use crate::rng::noise2;
use crate::site::{
    EnteringRoad, RiverLine, RoadClass, SettlementFunction as S, SettlementId, TerrainInput, Tier,
    TownSite,
};
use std::collections::BTreeMap;

/// Names of the synthetic sites.
pub const NAMES: [&str; 4] = ["aldermere", "thornby", "highcrag", "saltwick"];

/// A synthetic site and its terrain by name.
#[must_use]
pub fn by_name(name: &str) -> Option<(TownSite, TerrainInput)> {
    match name {
        "aldermere" => Some(aldermere()),
        "thornby" => Some(thornby()),
        "highcrag" => Some(highcrag()),
        "saltwick" => Some(saltwick()),
        _ => None,
    }
}

/// The building mix `arda-settle` estimates (its `profile::buildings`).
#[must_use]
pub fn estimate_mix(tier: Tier, p: u32, f: &[S]) -> BTreeMap<String, u32> {
    let mut b: BTreeMap<String, u32> = BTreeMap::new();
    let mut put = |k: &str, n: u32| {
        if n > 0 {
            *b.entry(k.to_string()).or_default() += n;
        }
    };
    let urban = matches!(tier, Tier::Town | Tier::City);
    match tier {
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
            put("cottage", p / 40);
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
            put("library", u32::from(tier == Tier::City));
            put("manor", 1 + p / 6000);
        }
    }
    let has = |x: S| f.contains(&x);
    if has(S::Farming) && tier != Tier::Hamlet {
        put("mill", 1 + p / 3000);
    }
    if has(S::Market) {
        put("market_hall", u32::from(urban));
        put("stall", if urban { p / 150 } else { 4 });
    }
    if has(S::Port) {
        put("dock", 1 + p / 2000);
        put("warehouse", if urban { 1 + p / 1500 } else { 1 });
    }
    if has(S::Fishing) {
        put("boathouse", (1 + p / 300).min(6));
    }
    if has(S::Fortress) || has(S::Capital) {
        put("keep", 1);
        put("barracks", u32::from(urban));
        put("guardhouse", 1 + p / 3000);
    }
    b
}

/// Snaps a polyline to 100 m cell centres, as `arda-settle` roads are.
#[must_use]
pub fn coarse(points: &[Vec2]) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::new();
    for p in geom::resample(points, 100.0) {
        #[allow(clippy::cast_precision_loss)]
        let c = v2(
            (floor_i(p.x / 100.0) as f64 + 0.5) * 100.0,
            (floor_i(p.y / 100.0) as f64 + 0.5) * 100.0,
        );
        if out.last() != Some(&c) {
            out.push(c);
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn site(
    id: u64,
    name: &str,
    tier: Tier,
    pop: u32,
    functions: Vec<S>,
    wealth: u8,
    culture: &str,
    pos: (i64, i64),
    tags: &[&str],
) -> TownSite {
    let buildings = estimate_mix(tier, pop, &functions);
    TownSite {
        id: SettlementId(id),
        name: name.into(),
        tier,
        population: pop,
        coastal: functions.contains(&S::Fishing) || functions.contains(&S::Port),
        riverine: tags.contains(&"river"),
        functions,
        wealth,
        culture: culture.into(),
        realm_id: 1,
        biome: "temperate".into(),
        x_m: pos.0,
        y_m: pos.1,
        site_tags: tags.iter().map(|s| (*s).to_string()).collect(),
        buildings,
    }
}

fn road(class: RoadClass, pts: &[Vec2]) -> EnteringRoad {
    EnteringRoad {
        class,
        points: coarse(pts),
    }
}

/// A market town of 2,600 at a bridge over a meandering river.
#[must_use]
pub fn aldermere() -> (TownSite, TerrainInput) {
    let (cx, cy) = (48_030.0, 72_010.0);
    let s = site(
        101,
        "Aldermere",
        Tier::Town,
        2600,
        vec![S::Crafting, S::Crossing, S::Farming, S::Market],
        150,
        "heartland",
        (48_030, 72_010),
        &["bridge_site", "river"],
    );
    let river_x = move |y: f64| cx - 190.0 + 55.0 * geom::sin_cos((y - cy) / 210.0).0;
    let river: Vec<Vec2> = (0..=60)
        .map(|k| {
            let y = cy - 1500.0 + 50.0 * f64::from(k);
            v2(river_x(y), y)
        })
        .collect();
    let rline = river.clone();
    let water = Box::new(move |p: Vec2| geom::dist_to(&rline, p) < 12.0);
    let rline2 = river.clone();
    let height = Box::new(move |p: Vec2| {
        let d = geom::dist_to(&rline2, p);
        38.0 + (d * 0.03).min(14.0) + 2.5 * noise2(11, p.x / 260.0, p.y / 260.0)
            - if d < 12.0 { 2.0 } else { 0.0 }
    });
    let mut t = TerrainInput::from_height(height, water, 3.0);
    t.rivers.push(RiverLine {
        points: river,
        width_m: 24.0,
    });
    t.roads.push(road(
        RoadClass::Highway,
        &[
            v2(cx - 1400.0, cy + 160.0),
            v2(cx - 500.0, cy + 60.0),
            v2(cx - 80.0, cy - 10.0),
            v2(cx + 600.0, cy - 90.0),
            v2(cx + 1400.0, cy - 40.0),
        ],
    ));
    t.roads.push(road(
        RoadClass::Road,
        &[
            v2(cx + 900.0, cy - 1200.0),
            v2(cx + 420.0, cy - 560.0),
            v2(cx + 60.0, cy - 60.0),
            v2(cx, cy),
        ],
    ));
    t.roads.push(road(
        RoadClass::Road,
        &[
            v2(cx, cy),
            v2(cx + 90.0, cy + 120.0),
            v2(cx + 300.0, cy + 600.0),
            v2(cx + 200.0, cy + 1300.0),
        ],
    ));
    (s, t)
}

/// A farming village of 320 where two roads cross.
#[must_use]
pub fn thornby() -> (TownSite, TerrainInput) {
    let (cx, cy) = (21_450.0, 33_380.0);
    let s = site(
        202,
        "Thornby",
        Tier::Village,
        320,
        vec![S::Farming, S::Market],
        110,
        "heartland",
        (21_450, 33_380),
        &[],
    );
    let height = Box::new(|p: Vec2| 60.0 + 9.0 * noise2(5, p.x / 400.0, p.y / 400.0));
    let mut t = TerrainInput::from_height(height, Box::new(|_| false), 3.0);
    t.roads.push(road(
        RoadClass::Road,
        &[
            v2(cx - 60.0, cy - 1300.0),
            v2(cx + 10.0, cy - 300.0),
            v2(cx - 20.0, cy + 400.0),
            v2(cx + 120.0, cy + 1300.0),
        ],
    ));
    t.roads.push(road(
        RoadClass::Track,
        &[
            v2(cx - 1300.0, cy + 200.0),
            v2(cx - 400.0, cy + 60.0),
            v2(cx + 300.0, cy - 40.0),
            v2(cx + 1300.0, cy - 260.0),
        ],
    ));
    (s, t)
}

/// A walled town of 1,800 under a castle on a hill.
#[must_use]
pub fn highcrag() -> (TownSite, TerrainInput) {
    let (cx, cy) = (90_120.0, 15_640.0);
    let s = site(
        303,
        "Highcrag",
        Tier::Town,
        1800,
        vec![S::Crafting, S::Fortress, S::Market],
        130,
        "highland",
        (90_120, 15_640),
        &["defensible", "hill"],
    );
    let peak = v2(cx + 40.0, cy - 30.0);
    let height = Box::new(move |p: Vec2| {
        let u = p.dist(peak) / 230.0;
        55.0 + 48.0 / (1.0 + u * u) + 4.0 * noise2(9, p.x / 180.0, p.y / 180.0)
    });
    let mut t = TerrainInput::from_height(height, Box::new(|_| false), 3.0);
    t.roads.push(road(
        RoadClass::Highway,
        &[
            v2(cx - 1400.0, cy + 900.0),
            v2(cx - 500.0, cy + 350.0),
            v2(cx - 60.0, cy + 60.0),
            v2(cx + 400.0, cy + 200.0),
            v2(cx + 1400.0, cy + 300.0),
        ],
    ));
    t.roads.push(road(
        RoadClass::Road,
        &[
            v2(cx - 100.0, cy + 80.0),
            v2(cx - 350.0, cy - 500.0),
            v2(cx - 700.0, cy - 1300.0),
        ],
    ));
    (s, t)
}

/// A fishing hamlet of 60 on a cove.
#[must_use]
pub fn saltwick() -> (TownSite, TerrainInput) {
    let (cx, cy) = (12_310.0, 58_870.0);
    let s = site(
        404,
        "Saltwick",
        Tier::Hamlet,
        60,
        vec![S::Fishing, S::Pastoral],
        70,
        "coastal",
        (12_310, 58_870),
        &["coast", "fish", "harbour"],
    );
    let shore = move |x: f64| {
        cy + 110.0 + 45.0 * geom::sin_cos((x - cx) / 160.0).0
            - 60.0 / (1.0 + ((x - cx) / 120.0) * ((x - cx) / 120.0))
    };
    let water = Box::new(move |p: Vec2| p.y > shore(p.x));
    let height = Box::new(move |p: Vec2| {
        let d = shore(p.x) - p.y;
        (d * 0.06).clamp(-3.0, 40.0) + 3.0 * noise2(21, p.x / 150.0, p.y / 150.0)
    });
    let mut t = TerrainInput::from_height(height, water, 3.0);
    t.roads.push(road(
        RoadClass::Track,
        &[
            v2(cx - 1200.0, cy + 60.0),
            v2(cx - 300.0, cy + 30.0),
            v2(cx + 250.0, cy - 10.0),
            v2(cx + 1200.0, cy + 80.0),
        ],
    ));
    t.roads.push(road(
        RoadClass::Footpath,
        &[
            v2(cx + 20.0, cy),
            v2(cx - 80.0, cy - 600.0),
            v2(cx - 60.0, cy - 1300.0),
        ],
    ));
    (s, t)
}
