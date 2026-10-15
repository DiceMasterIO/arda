//! A settlement's town plan on world terrain, and its buildings as the NPC
//! generator and the society layer read them (logic/10; adapters A4, A14).

use crate::terrain::Surroundings;
use crate::PeopleError;
use arda_ids::BuildingFunction;
use arda_refine::Source;
use arda_settle::model::Settlement;
use arda_settle::roads::Road;
use arda_town::geom::{v2, Vec2};
use arda_town::plan::grid::SQUARE_M;
use arda_town::site::EnteringRoad;
use arda_town::{TerrainInput, TownPlan, TownSite};
use std::sync::Arc;

/// Radius of terrain a plan sees, metres, by tier: a city spans about
/// 2 km, a hamlet a few hundred metres (assumed, tunable).
#[must_use]
pub const fn plan_radius_m(tier: arda_settle::model::Tier) -> f64 {
    use arda_settle::model::Tier as T;
    match tier {
        T::Hamlet => 700.0,
        T::Village => 1_200.0,
        T::Town => 2_000.0,
        T::City => 2_600.0,
    }
}

/// Metres of plan radius per square root of a person: the ground a city
/// of `p` needs at 150 people a hectare (logic/08 §landuse) is a disc of
/// `√(p / 150 / π)` hectares, about `8.2·√p` m; half again for streets,
/// walls, fields at the gate and the 300 m terrain margin's slack.
const RADIUS_PER_ROOT_PERSON: f64 = 12.0;

/// Radius of terrain the plan of `s` sees, metres: its tier's
/// ([`plan_radius_m`]), or more for a city whose people need more ground
/// (review 3.3-11: cities above about 48,000 ran past a fixed window).
#[must_use]
pub fn plan_radius_for(s: &Settlement) -> f64 {
    plan_radius_m(s.tier).max(RADIUS_PER_ROOT_PERSON * f64::from(s.population).sqrt())
}

/// People the plan's own buildings house (before any off-plan cottage).
#[must_use]
pub fn housed(plan: &TownPlan) -> u64 {
    plan_buildings(plan)
        .iter()
        .map(|b| u64::from(b.capacity))
        .sum()
}

/// A plan stands for its settlement when its buildings house at least half
/// its people; a plan that houses fewer is refused like one the planner
/// cannot draw, and the settlement keeps the buildings of its mix
/// (logic/21 §build-buildings), instead of a handful of plan buildings and
/// a crowd of invented off-plan cottages (review 3.3-11).
#[must_use]
pub fn usable(plan: &TownPlan, population: u32) -> bool {
    housed(plan) * 2 >= u64::from(population)
}

/// The planner's view of a settlement record (the record deserialises
/// into `TownSite` directly; extra fields are ignored).
///
/// # Errors
/// [`PeopleError::Format`] when the record does not fit.
pub fn site(record: &serde_json::Value) -> Result<TownSite, PeopleError> {
    serde_json::from_value(record.clone())
        .map_err(|e| PeopleError::format(std::path::Path::new("settlements.json"), e))
}

fn town_class(c: arda_ids::RoadClass) -> arda_town::site::RoadClass {
    use arda_town::site::RoadClass as T;
    match c {
        arda_ids::RoadClass::None => T::None,
        arda_ids::RoadClass::Track => T::Track,
        arda_ids::RoadClass::Road => T::Road,
        arda_ids::RoadClass::Highway => T::Highway,
        arda_ids::RoadClass::Footpath => T::Footpath,
    }
}

#[allow(clippy::cast_precision_loss)] // world metres are far below 2^52
fn vec(p: [i64; 2]) -> Vec2 {
    v2(p[0] as f64, p[1] as f64)
}

/// The stretches of `roads` within `radius_m` of `centre`, each a run of
/// consecutive vertices inside the radius (one vertex of margin).
#[must_use]
pub fn roads_near(roads: &[Road], centre: Vec2, radius_m: f64) -> Vec<EnteringRoad> {
    let near = |p: [i64; 2]| {
        let q = vec(p);
        ((q.x - centre.x).powi(2) + (q.y - centre.y).powi(2)).sqrt() <= radius_m
    };
    let mut out = Vec::new();
    for r in roads {
        for seg in &r.segments {
            let mut run: Vec<Vec2> = Vec::new();
            for (i, &p) in seg.iter().enumerate() {
                let keep = near(p)
                    || seg.get(i + 1).is_some_and(|&q| near(q))
                    || (i > 0 && seg.get(i - 1).is_some_and(|&q| near(q)));
                if keep {
                    run.push(vec(p));
                } else if run.len() >= 2 {
                    out.push(EnteringRoad {
                        class: town_class(r.class),
                        points: std::mem::take(&mut run),
                    });
                } else {
                    run.clear();
                }
            }
            if run.len() >= 2 {
                out.push(EnteringRoad {
                    class: town_class(r.class),
                    points: run,
                });
            }
        }
    }
    out
}

/// The terrain around `(x_m, y_m)` as `arda-town` reads it: heights from
/// the world's cells; the water arda-refine draws, square for square: its
/// lake and sea shores ([`arda_refine::region::WaterRegion::standing`],
/// review 3.3-2: not whole 100 m cells) and its rivers (their centrelines
/// and widths, and their water); and the settlement stage's roads. A plan
/// on it never stands a building on water the tactical map shows (review
/// 3.5-3); marsh pools, at most a foot deep, are made ground.
///
/// # Errors
/// [`PeopleError::World`] when a layer fails to load.
pub fn terrain(
    src: &dyn Source,
    at: (f64, f64),
    radius_m: f64,
    roads: &[Road],
) -> Result<TerrainInput, PeopleError> {
    let s = Arc::new(Surroundings::around(src, at, radius_m + 300.0)?);
    let pieces = crate::rivers::pieces_around(src, at, radius_m + 300.0)?;
    let rivers = crate::rivers::lines(&pieces, v2(at.0, at.1));
    let river = Arc::new(arda_refine::water::RiverWater::new(pieces));
    let standing = Arc::new(standing_region(src, at, radius_m + 300.0)?);
    let hs = Arc::clone(&s);
    let height: arda_town::site::Field = Box::new(move |p: Vec2| hs.height_m(p.x, p.y));
    let water: arda_town::site::Mask = Box::new(move |p: Vec2| {
        let (u, v) = (p.x / SQUARE_M, p.y / SQUARE_M);
        standing.standing(u, v, false).is_some() || crate::rivers::in_water(&river, p)
    });
    let mut t = TerrainInput::from_height(height, water, 5.0);
    t.rivers = rivers;
    t.roads = roads_near(roads, v2(at.0, at.1), radius_m);
    Ok(t)
}

/// The refined lake and sea water of the squares within `radius_m` of
/// `at` (clamped into the world).
fn standing_region(
    src: &dyn Source,
    (x_m, y_m): (f64, f64),
    radius_m: f64,
) -> Result<arda_refine::region::WaterRegion, PeopleError> {
    let (w, h) = src.cells_wide_high();
    let n = arda_refine::context::N;
    #[allow(clippy::cast_possible_truncation)] // world metres fit i64
    let sq = |m: f64, max: i64| ((m / SQUARE_M).floor() as i64).clamp(0, max * n - 1);
    let squares = (
        sq(x_m - radius_m, w),
        sq(y_m - radius_m, h),
        sq(x_m + radius_m, w),
        sq(y_m + radius_m, h),
    );
    arda_refine::region::WaterRegion::gather(
        src,
        squares,
        arda_refine::region::RiverStyle::default(),
    )
    .map_err(|e| PeopleError::World(e.to_string()))
}

/// The settlement's town plan, or `None` when the planner refuses the site
/// or the plan cannot house half its people ([`usable`]); the settlement
/// then keeps its derived buildings.
///
/// # Errors
/// Record or world failures (a refused plan is `Ok(None)`).
pub fn plan(
    src: &dyn Source,
    settlement: &Settlement,
    record: &serde_json::Value,
    roads: &[Road],
) -> Result<Option<TownPlan>, PeopleError> {
    if settlement.population == 0 {
        return Ok(None);
    }
    #[allow(clippy::cast_precision_loss)]
    let at = (settlement.x_m as f64, settlement.y_m as f64);
    let t = terrain(src, at, plan_radius_for(settlement), roads)?;
    Ok(arda_town::generate(&site(record)?, &t, src.seed())
        .ok()
        .filter(|p| usable(p, settlement.population)))
}

/// Wealth a prosperity sample needs to count towards a golden age.
pub const GOLDEN_WEALTH: i32 = 160;

/// Amends a plan's building-material facts with the settlement's history
/// (logic/10 §building-materials): its great fires and whether it stayed
/// prosperous (at least [`GOLDEN_WEALTH`]) through at least half of four or
/// more prosperity samples. Layout-neutral: only kits and floors change.
pub fn read_history(plan: &mut TownPlan, society: &arda_society::Society) {
    let id = plan.site.0;
    let fires = society
        .history
        .events
        .iter()
        .filter(|e| e.kind == arda_society::history::EventKind::Fire && e.settlements.contains(&id))
        .count();
    plan.fabric.fires = u8::try_from(fires).unwrap_or(u8::MAX);
    plan.fabric.golden_age = society
        .settlements
        .iter()
        .find(|s| s.id == id)
        .is_some_and(|s| {
            let samples = &s.prosperity.samples;
            let good = samples.iter().filter(|x| x[1] >= GOLDEN_WEALTH).count();
            samples.len() >= 4 && 2 * good >= samples.len()
        });
}

fn function(f: arda_town::BuildingFunction) -> Option<BuildingFunction> {
    BuildingFunction::from_key(f.key())
}

/// The plan's buildings as `arda-npc` specs (adapter A4: capacity and
/// workplace slots from the plan, logic/10 §town-capacity). A partial plan
/// that houses fewer than `population` people gets outlying cottages after
/// its last id (off the plan, so never in a tactical block), at the
/// settlement's `wealth`.
#[must_use]
pub fn plan_specs(plan: &TownPlan, population: u32, wealth: u8) -> Vec<arda_npc::BuildingSpec> {
    let mut out = plan_buildings(plan);
    pad_housing(&mut out, population, plan.site.0, wealth);
    out
}

/// Fingerprint of the buildings a settlement's notables are generated in:
/// 32 hex digits of a digest of their specs (ids, functions, capacities,
/// slots, wealth). `arda society build` stores it beside the notables; a
/// plan the server draws again must give the same (review 3.3-5).
#[must_use]
pub fn fingerprint(specs: &[arda_npc::BuildingSpec]) -> String {
    let bytes = serde_json::to_vec(specs).unwrap_or_default();
    arda_settle::stamp::hex(&arda_ids::hash::digest(&[&bytes]))
}

fn pad_housing(out: &mut Vec<arda_npc::BuildingSpec>, population: u32, site: u64, wealth: u8) {
    let capacity = arda_town::plan::spec::capacity(arda_town::BuildingFunction::Cottage);
    let mut housed: u64 = out.iter().map(|b| u64::from(b.capacity)).sum();
    let mut next = out.iter().map(|b| b.id.0).max().unwrap_or(0) + 1;
    while housed < u64::from(population) {
        out.push(arda_npc::BuildingSpec {
            id: arda_npc::BuildingId(next),
            settlement_id: arda_npc::SettlementId(site),
            function: BuildingFunction::Cottage,
            tags: vec!["off_plan".into()],
            capacity,
            workplace_slots: 0,
            wealth,
        });
        housed += u64::from(capacity.max(1));
        next += 1;
    }
}

fn plan_buildings(plan: &TownPlan) -> Vec<arda_npc::BuildingSpec> {
    arda_town::plan::spec::specs(plan)
        .into_iter()
        .filter_map(|b| {
            Some(arda_npc::BuildingSpec {
                id: arda_npc::BuildingId(b.id.0),
                settlement_id: arda_npc::SettlementId(b.settlement_id.0),
                function: function(b.function)?,
                tags: b.tags,
                capacity: b.capacity,
                workplace_slots: b.workplace_slots,
                wealth: b.wealth,
            })
        })
        .collect()
}

/// The plan's buildings as `arda-society` explicit buildings, so society's
/// role slots name plan buildings (adapter A14).
#[must_use]
pub fn society_buildings(plan: &TownPlan) -> Vec<arda_society::input::BuildingSpec> {
    plan_buildings(plan)
        .into_iter()
        .map(|b| arda_society::input::BuildingSpec {
            id: b.id.0,
            settlement_id: b.settlement_id.0,
            function: b.function.key().to_string(),
            tags: b.tags,
        })
        .collect()
}

/// Buildings derived from the settlement's building mix, with the ids
/// `arda-society` gave them, sized by `arda-town`'s capacity table; when
/// the mix houses too few people, cottages are added after the last id
/// (logic/10 §town-capacity: a settlement always houses its population).
#[must_use]
pub fn mix_specs(
    settlement: &Settlement,
    refs: &[arda_society::buildings::BuildingRef],
) -> Vec<arda_npc::BuildingSpec> {
    let sid = arda_npc::SettlementId(settlement.id.get());
    let spec = |id: u64, f: BuildingFunction, tags: Vec<String>| {
        let town: Option<arda_town::BuildingFunction> =
            serde_json::from_value(serde_json::Value::from(f.key())).ok();
        let (capacity, slots) = town.map_or((0, 0), |t| {
            (
                arda_town::plan::spec::capacity(t),
                arda_town::plan::spec::workplace_slots(t),
            )
        });
        arda_npc::BuildingSpec {
            id: arda_npc::BuildingId(id),
            settlement_id: sid,
            function: f,
            tags,
            capacity,
            workplace_slots: slots,
            wealth: settlement.wealth,
        }
    };
    let mut out: Vec<arda_npc::BuildingSpec> = refs
        .iter()
        .filter_map(|b| {
            let f = BuildingFunction::from_key(&b.function)?;
            Some(spec(b.id, f, b.tags.clone()))
        })
        .collect();
    pad_housing(&mut out, settlement.population, sid.0, settlement.wealth);
    out
}
