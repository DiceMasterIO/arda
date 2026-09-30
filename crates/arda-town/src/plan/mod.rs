//! Town plans in world metres (goals 36, 44, 45).
//!
//! The pipeline, each step reading only earlier outputs:
//! 1. focal point and market ([`focal`]);
//! 2. main streets along the entering roads, market square, castle ward and
//!    wall ring ([`focal`], [`square`], [`wall`]);
//! 3. back lanes, cross alleys and the intramural lane ([`lanes`]);
//! 4. main streets kept off the rivers but where they cross ([`wet`]),
//!    rasterisation onto the square grid, bridges and gates ([`raster`],
//!    [`bridge`]);
//! 5. off-plot buildings: keep, hall, stalls, docks ([`special`]);
//! 6. burgage plots ([`plots`]), function assignment ([`assign`]) and
//!    footprints ([`place`]);
//! 7. zoning, districts, wealth and ids ([`finish`]).
//!
//! If the plots cannot hold the building mix, the core radius grows and the
//! plan is rebuilt (up to five times); the last attempt keeps what fits and
//! notes the rest, so planning never fails a batch (goal 47).

pub mod assign;
pub mod bridge;
pub mod check;
pub mod croft;
pub mod finish;
pub mod focal;
pub mod grid;
pub mod lanes;
pub mod outline;
pub mod params;
pub mod place;
pub mod plots;
pub mod raster;
pub mod spec;
pub mod special;
pub mod square;
pub mod types;
pub mod wall;
pub mod wet;

use crate::error::TownError;
use crate::function::BuildingFunction as F;
use crate::geom::{v2, Vec2};
use crate::rng::{hash, Rng};
use crate::site::{SettlementFunction, TerrainInput, Tier, TownSite};
use assign::{Demand, InfoCtx};
use grid::PlanGrid;
use lanes::{Bounds, LaneSpec};
use params::Params;
use place::Draft;
use std::collections::BTreeMap;
pub use types::*;

/// Attempts with a growing core radius before settling for a partial plan.
const ATTEMPTS: u32 = 6;

/// Plans a town deterministically from the world seed and the site.
///
/// # Errors
/// [`TownError::Site`] for a site with no population.
pub fn generate(
    site: &TownSite,
    terrain: &TerrainInput,
    world_seed: u64,
) -> Result<TownPlan, TownError> {
    if site.population == 0 {
        return Err(TownError::Site {
            site: site.name.clone(),
            message: "population is zero".into(),
        });
    }
    let seed = hash(world_seed, site.id.0, 0x7041);
    let demand = Demand::from_site(site);
    let mut attempt = 0;
    loop {
        let grow = 1.0 + 0.2 * f64::from(attempt);
        let last = attempt + 1 >= ATTEMPTS;
        if let Some(plan) = build(site, terrain, seed, grow, &demand, last) {
            return Ok(plan);
        }
        attempt += 1;
    }
}

fn consume(wanted: &mut BTreeMap<F, u32>, drafts: &[Draft]) {
    for d in drafts.iter().filter(|d| !d.ancillary) {
        if let Some(n) = wanted.get_mut(&d.function) {
            *n = n.saturating_sub(1);
        }
    }
}

fn bbox(points: &[Vec2], margin: f64) -> (Vec2, Vec2) {
    let lo = points.iter().fold(v2(f64::MAX, f64::MAX), |a, p| {
        v2(a.x.min(p.x), a.y.min(p.y))
    });
    let hi = points.iter().fold(v2(f64::MIN, f64::MIN), |a, p| {
        v2(a.x.max(p.x), a.y.max(p.y))
    });
    (lo - v2(margin, margin), hi + v2(margin, margin))
}

fn geom_len(a: &focal::Arm) -> f64 {
    crate::geom::length(&a.points)
}

fn street_specs(
    site: &TownSite,
    terrain: &TerrainInput,
    params: &Params,
    fc: &Focal,
    seed: u64,
) -> (
    Vec<focal::Arm>,
    MarketSquare,
    Option<Vec<Vec2>>,
    Vec<LaneSpec>,
    f64,
) {
    let r_ext = params.r_core * 1.7 + 90.0;
    let mut arms = focal::arms(terrain, fc, params, seed, r_ext);
    for a in &mut arms {
        let half_w = f64::from(a.class.width_squares().max(2)) * grid::SQUARE_M * 0.5;
        a.points = wet::dry(&a.points, half_w, &terrain.rivers, &*terrain.water);
    }
    let mut rng = Rng::new(seed, "square");
    let sq = square::market(fc, &arms, params, &mut rng);
    let mut enclose: Vec<Vec2> = sq.polygon.clone();
    if params.castle {
        enclose.push(fc.feature);
    }
    let axis = arms
        .iter()
        .max_by(|a, b| geom_len(a).total_cmp(&geom_len(b)))
        .map_or(v2(1.0, 0.0), |a| {
            (a.points[a.points.len() - 1] - a.points[0]).norm()
        });
    let ring = params
        .walled
        .then(|| wall::ring(fc.market, params, terrain, seed, &enclose, 30.0, axis));
    let sq_r = square::radius(&sq, fc.market);
    let bounds = Bounds {
        market: fc.market,
        square_r: sq_r + 6.0,
        r_core: params.r_core,
        ring: ring
            .as_deref()
            .map(|r| (r, wall::THICKNESS_M * 0.5 + params.lane_w + 5.0)),
        terrain,
        castle: params.castle.then_some((fc.feature, 34.0)),
    };
    let mut specs: Vec<LaneSpec> = arms
        .iter()
        .map(|a| LaneSpec {
            class: StreetClass::Main,
            points: a.points.clone(),
            // Road widths in squares follow convention I4, so streets join
            // the tactical roads beyond the plan edge.
            width_m: f64::from(a.class.width_squares().max(2)) * grid::SQUARE_M,
        })
        .collect();
    specs.extend(lanes::lanes(&arms, params, &bounds, seed));
    if fc.kind == FocalKind::Harbour {
        specs.extend(lanes::strand(
            fc.market,
            fc.feature,
            params.r_core * 0.9,
            params,
            terrain,
        ));
    }
    if let Some(r) = &ring {
        let inset = wall::THICKNESS_M * 0.5 + params.lane_w * 0.5 + 0.8;
        specs.push(lanes::intramural(r, fc.market, inset, params.lane_w));
    }
    let _ = site;
    (arms, sq, ring, specs, sq_r)
}

#[allow(clippy::too_many_lines)]
fn build(
    site: &TownSite,
    terrain: &TerrainInput,
    seed: u64,
    grow: f64,
    demand: &Demand,
    last: bool,
) -> Option<TownPlan> {
    let params = Params::for_site(site, grow);
    let fc = focal::find(site, terrain, &params);
    let (_arms, sq, ring, specs, sq_r) = street_specs(site, terrain, &params, &fc, seed);
    let r_ext = params.r_core * 1.7 + 90.0;
    let (lo, hi) = bbox(&[fc.market, fc.feature, site.position()], r_ext + 40.0);
    let base_height_m = (terrain.height)(fc.market);
    let mut g = PlanGrid::new(lo, hi, terrain, base_height_m);
    let urban = matches!(site.tier, Tier::Town | Tier::City);
    let streets: Vec<Street> = bridge::cut(&g, specs)
        .into_iter()
        .enumerate()
        .map(|(i, s)| Street {
            id: StreetId(u16::try_from(i).unwrap_or(u16::MAX)),
            paved: urban
                && (s.class == StreetClass::Main || site.tier == Tier::City)
                && site.wealth >= 70,
            class: s.class,
            points: s.points,
            width_m: s.width_m,
        })
        .collect();
    raster::square(&mut g, &sq);
    let castle = params.castle.then(|| square::castle(&fc, &g, site.tier));
    if let Some(c) = &castle {
        raster::castle(&mut g, c);
    }
    if let Some(r) = &ring {
        raster::wall(&mut g, r, wall::THICKNESS_M);
    }
    let bridges = raster::streets(&mut g, &streets);
    let gates = ring
        .as_deref()
        .map(|r| raster::gates(&mut g, r, &streets, &terrain.rivers))
        .unwrap_or_default();
    g.compute_depth();

    let mut wanted = demand.counts.clone();
    let mut drafts: Vec<Draft> = Vec::new();
    let mut rng = Rng::new(seed, "special");
    if let Some(c) = &castle {
        let d = special::castle(
            &mut g,
            c,
            wanted.get(&F::Barracks).copied().unwrap_or(0) > 0,
        );
        consume(&mut wanted, &d);
        drafts.extend(d);
    }
    let halls = wanted.get(&F::MarketHall).copied().unwrap_or(0);
    let stalls = wanted.get(&F::Stall).copied().unwrap_or(0);
    let d = special::square(&mut g, fc.market, halls, stalls, site.tier, &mut rng);
    consume(&mut wanted, &d);
    drafts.extend(d);
    let fishing = site.has(SettlementFunction::Fishing);
    let docks = wanted.get(&F::Dock).copied().unwrap_or(0);
    let jetty = u32::from(fishing && docks == 0);
    let boats = wanted.get(&F::Boathouse).copied().unwrap_or(0);
    if docks + jetty + boats > 0 {
        let mut d = special::shore(&mut g, fc.feature, docks + jetty, boats, &mut rng);
        let mut spare = jetty;
        for x in d.iter_mut().rev() {
            if x.function == F::Dock && spare > 0 {
                x.ancillary = true;
                spare -= 1;
            }
        }
        consume(&mut wanted, &d);
        drafts.extend(d);
    }

    let raw = plots::generate(
        &mut g,
        &plots::PlotInputs {
            streets: &streets,
            square: Some(&sq),
            market: fc.market,
            ring: ring.as_deref(),
            params: &params,
            seed,
        },
    );
    let gate_pts: Vec<Vec2> = gates
        .iter()
        .filter(|g| g.street.is_some())
        .map(|g| g.point)
        .collect();
    let downstream = terrain.rivers.first().and_then(|r| {
        let n = r.points.len();
        (n >= 2).then(|| (r.points[n - 1] - r.points[0]).norm())
    });
    let craft_dir = downstream.or_else(|| gate_pts.first().map(|&p| (p - fc.market).norm()));
    let market_r = match site.tier {
        Tier::City => 120.0,
        Tier::Town => 80.0,
        _ => 45.0,
    };
    let mut info = assign::info(
        &g,
        &raw,
        &InfoCtx {
            market: fc.market,
            gates: &gate_pts,
            ring: ring.as_deref(),
            downstream,
            market_r,
            tier: site.tier,
        },
    );
    if let Some(dir) = craft_dir {
        for (p, inf) in raw.iter().zip(info.iter_mut()) {
            let mid = p.cols[p.cols.len() / 2].front;
            let c = g.centre(mid.0, mid.1);
            if inf.district == DistrictKind::Residential && (c - fc.market).norm().dot(dir) > 0.6 {
                inf.district = DistrictKind::Craft;
            }
        }
    }
    wanted.retain(|_, n| *n > 0);
    let (assigned, unplaced) =
        assign::assign(&raw, &info, &wanted, site.tier, params.r_core, seed, last);
    if std::env::var_os("ARDA_TOWN_DEBUG").is_some() {
        eprintln!(
            "grow {grow:.1}: raw plots {}, unplaced {:?}",
            raw.len(),
            unplaced
        );
    }
    if !unplaced.is_empty() && !last {
        return None;
    }
    let mut prng = Rng::new(seed, "place");
    let mut failed: BTreeMap<F, u32> = unplaced;
    let mut groups = Vec::new();
    let mut used: Vec<bool> = vec![false; raw.len()];
    for a in &assigned {
        for &i in &a.plots {
            used[i] = true;
        }
    }
    let mut spare = 0usize;
    for a in &assigned {
        let mut a = a.clone();
        let mut d = place::place(&g, &raw, &a, &params, site.tier, &mut prng);
        // A plot too shallow or ragged for its building: move to the next
        // free single plot in value order.
        while d.is_empty() && a.plots.len() == 1 && spare < raw.len() {
            if !used[spare] {
                used[spare] = true;
                a.plots = vec![spare];
                d = place::place(&g, &raw, &a, &params, site.tier, &mut prng);
            }
            spare += 1;
        }
        if d.is_empty() {
            *failed.entry(a.function).or_default() += 1;
            continue;
        }
        groups.push((a, drafts.len(), d.len()));
        drafts.extend(d);
    }
    if std::env::var_os("ARDA_TOWN_DEBUG").is_some() && !failed.is_empty() {
        eprintln!("grow {grow:.1}: failed {failed:?}");
    }
    if !failed.is_empty() && !last {
        return None;
    }
    let mut notes = demand.notes.clone();
    for (f, n) in &failed {
        notes.push(format!("{n} × `{}` did not fit the plan", f.key()));
    }
    Some(finish::finish(finish::Parts {
        site,
        seed,
        base_height_m,
        grid: g,
        focal: fc,
        streets,
        bridges,
        square: sq,
        square_r: sq_r,
        ring,
        gates,
        castle,
        raw,
        info,
        groups,
        drafts,
        notes,
    }))
}
