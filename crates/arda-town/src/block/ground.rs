//! Per-square ground, elevation and water depth from the plan grid, and the
//! wall edges the grid implies: the town and castle curtain (`city_wall`,
//! with `gate` pieces across passages and water gates) and bridge parapets.
//! Every value is a pure function of the global square, so blocks join.

use crate::plan::grid::Kind;
use crate::plan::TownPlan;
use crate::rng::{hash_i, noise2, unit};
use crate::site::Tier;
use arda_tactical::catalog::WallRole;
use arda_tactical::layout::EdgeAxis;

/// Metres per foot.
const FT: f64 = 0.3048;
/// Height of the wall walk above the ground, feet.
pub const WALL_WALK_FT: i16 = 20;

/// One square's ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ground {
    /// Ground key (vocabulary).
    pub key: &'static str,
    /// Absolute elevation, feet, in 5-ft steps (convention I20).
    pub elevation_ft: i16,
    /// Water depth, feet (under 5 is shallow, I15).
    pub water_ft: u8,
    /// A walkable deck over water (bridge).
    pub deck: bool,
}

/// Plan kind at a global square (open country outside the grid).
#[must_use]
pub fn kind(plan: &TownPlan, gx: i64, gy: i64) -> Kind {
    plan.grid
        .gidx(gx, gy)
        .map_or(Kind::Open, |k| plan.grid.kind[k])
}

/// Absolute elevation in 5-ft steps of a global square.
#[must_use]
pub fn elevation(plan: &TownPlan, gx: i64, gy: i64) -> i16 {
    let rel = plan
        .grid
        .gidx(gx, gy)
        .map_or(0.0, |k| f64::from(plan.grid.height[k]));
    let ft = (plan.base_height_m + rel) / FT;
    crate::num::round_i16((ft / 5.0).round() * 5.0)
}

/// Squares, greens and baileys are levelled to their centre's height.
fn level(plan: &TownPlan, p: crate::geom::Vec2) -> i16 {
    let (x, y) = crate::plan::square::square_of(p);
    elevation(plan, x, y)
}

/// The flat water level in 5-ft steps (water does not step with terrain).
fn water_elevation(plan: &TownPlan) -> i16 {
    crate::num::round_i16((plan.water_level_m / FT / 5.0).round() * 5.0)
}

fn water_ft(plan: &TownPlan, gx: i64, gy: i64) -> u8 {
    let d = plan.grid.gidx(gx, gy).map_or(3, |k| plan.grid.depth[k]);
    (2 * d).clamp(2, 10)
}

/// Whether the town is poor: its buildings' mean wealth is
/// [`crate::plan::types::WealthLevel::Poor`]. Only a poor town's unpaved
/// streets show mud.
#[must_use]
pub fn poor(plan: &TownPlan) -> bool {
    let n = plan.buildings.len().max(1);
    let sum: usize = plan.buildings.iter().map(|b| usize::from(b.wealth)).sum();
    let mean = u8::try_from(sum / n).unwrap_or(u8::MAX);
    !plan.buildings.is_empty()
        && crate::plan::types::WealthLevel::of(mean) == crate::plan::types::WealthLevel::Poor
}

/// The ground of a global square; `poor` is [`poor`] of the plan.
#[must_use]
pub fn at(plan: &TownPlan, poor: bool, gx: i64, gy: i64) -> Ground {
    let seed = plan.seed;
    #[allow(clippy::cast_precision_loss)]
    let (fx, fy) = (gx as f64, gy as f64);
    let n = noise2(seed ^ 0x6d, fx / 9.0, fy / 9.0);
    let h = unit(hash_i(seed ^ 0x61, gx, gy));
    let urban = matches!(plan.tier, Tier::Town | Tier::City);
    let elevation_ft = elevation(plan, gx, gy);
    let mut g = Ground {
        key: "grass",
        elevation_ft,
        water_ft: 0,
        deck: false,
    };
    g.key = match kind(plan, gx, gy) {
        Kind::Open => {
            if n > 0.35 {
                "pasture"
            } else {
                "meadow"
            }
        }
        Kind::Water | Kind::WaterGate => {
            g.water_ft = water_ft(plan, gx, gy);
            g.elevation_ft = water_elevation(plan);
            "mud"
        }
        Kind::Bridge => {
            g.water_ft = water_ft(plan, gx, gy);
            g.elevation_ft = water_elevation(plan);
            g.deck = true;
            "mud"
        }
        Kind::Street => {
            let paved = plan
                .grid
                .gidx(gx, gy)
                .and_then(|k| {
                    plan.streets
                        .get(usize::from(plan.grid.street[k]).checked_sub(1)?)
                })
                .is_some_and(|s| s.paved);
            // Mud only on a poor town's streets, and there in scattered
            // squares where the street noise runs high, never in blotches.
            if paved {
                "cobbles"
            } else if poor && n > 0.68 && h < 0.18 {
                "mud"
            } else {
                "dirt"
            }
        }
        Kind::Gate => "cobbles",
        Kind::Square => {
            g.elevation_ft = level(plan, plan.focal.market);
            if plan.tier == Tier::City {
                "flagstone"
            } else {
                "cobbles"
            }
        }
        Kind::Wall => {
            g.elevation_ft += WALL_WALK_FT;
            "flagstone"
        }
        Kind::Front => {
            if urban {
                if h < 0.5 {
                    "packed_earth"
                } else {
                    "gravel"
                }
            } else {
                "grass"
            }
        }
        // A working yard is beaten earth from fence to fence.
        Kind::Yard => "packed_earth",
        Kind::Garden => {
            // A plot's back garden is dug as one bed or left as grass, so
            // its edges follow the plot's fences, never a noise contour.
            let plot = plan.grid.gidx(gx, gy).map_or(0, |k| plan.grid.plot[k]);
            if unit(hash_i(seed ^ 0x6A4D, i64::from(plot), 0)) < 0.6 {
                "farmland"
            } else {
                "grass"
            }
        }
        Kind::Bailey => {
            g.elevation_ft = level(plan, plan.focal.feature);
            "flagstone"
        }
        Kind::Green => {
            g.elevation_ft = level(plan, plan.focal.market);
            "grass"
        }
        Kind::Churchyard => "grass",
        Kind::Croft => croft(plan, gx, gy, n),
        Kind::Building | Kind::Plot => "planks",
    };
    g
}

/// The ground of a croft square by its parcel's use; the footprint's
/// outer rim is rough grass and scrub.
fn croft(plan: &TownPlan, gx: i64, gy: i64, n: f64) -> &'static str {
    use crate::plan::croft::{parcel, rim, use_of, Use};
    let seed = plan.seed;
    if rim(seed, |x, y| kind(plan, x, y) == Kind::Open, gx, gy) {
        return if n > 0.0 { "scrub" } else { "meadow" };
    }
    let p = parcel(seed, gx, gy);
    match use_of(p) {
        Use::Paddock => "pasture",
        Use::Orchard => "grass",
        // One dug ground per parcel: its hedges are its edges.
        Use::Kitchen => "farmland",
        Use::Meadow => "meadow",
        Use::Yard => "packed_earth",
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Band {
    Wall,
    Passage,
    Other,
}

fn band(k: Kind) -> Band {
    match k {
        Kind::Wall => Band::Wall,
        Kind::Gate | Kind::WaterGate => Band::Passage,
        _ => Band::Other,
    }
}

/// The curtain-wall or parapet piece on the edge between two squares.
fn between(a: Kind, b: Kind) -> Option<(WallRole, &'static str)> {
    match (band(a), band(b)) {
        (Band::Wall, Band::Other | Band::Passage) | (Band::Other | Band::Passage, Band::Wall) => {
            Some((WallRole::Run, "city_wall"))
        }
        (Band::Passage, Band::Other) | (Band::Other, Band::Passage) => {
            Some((WallRole::Gate, "city_wall"))
        }
        _ => match (a, b) {
            (Kind::Bridge, Kind::Water) | (Kind::Water, Kind::Bridge) => {
                Some((WallRole::Run, "stone"))
            }
            _ => None,
        },
    }
}

/// Wall edges implied by the grid inside a window, as
/// `((x, y, axis), (role, kit))` in global squares. Horizontal edges run
/// for rows `y0..=y1`, vertical ones for columns `x0..=x1`.
#[must_use]
pub fn edges(
    plan: &TownPlan,
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
) -> Vec<crate::block::interior::Edge> {
    let mut out = Vec::new();
    for y in y0..=y1 {
        for x in x0..x1 {
            if let Some(v) = between(kind(plan, x, y - 1), kind(plan, x, y)) {
                out.push(((x, y, EdgeAxis::Horizontal), v));
            }
        }
    }
    for y in y0..y1 {
        for x in x0..=x1 {
            if let Some(v) = between(kind(plan, x - 1, y), kind(plan, x, y)) {
                out.push(((x, y, EdgeAxis::Vertical), v));
            }
        }
    }
    out
}

/// The fence or hedge between two squares of the built-up footprint:
/// hedges between croft parcels and along a plot's back onto open country,
/// wattle between neighbouring plots' yards and gardens and between a plot
/// and the crofts behind it. A few edges stay open as gaps and gates.
fn fence(plan: &TownPlan, a: (i64, i64), b: (i64, i64)) -> Option<(WallRole, &'static str)> {
    use crate::plan::croft::{parcel, rim, use_of};
    let (ka, kb) = (kind(plan, a.0, a.1), kind(plan, b.0, b.1));
    let back = |k: Kind| matches!(k, Kind::Yard | Kind::Garden);
    let open = |x: i64, y: i64| kind(plan, x, y) == Kind::Open;
    let plot = |p: (i64, i64)| plan.grid.gidx(p.0, p.1).map_or(0, |k| plan.grid.plot[k]);
    let seed = plan.seed;
    let urban = matches!(plan.tier, Tier::Town | Tier::City);
    let kit = match (ka, kb) {
        (Kind::Croft, Kind::Croft) => {
            let (pa, pb) = (parcel(seed, a.0, a.1), parcel(seed, b.0, b.1));
            let split = pa != pb && (use_of(pa) != use_of(pb) || (pa ^ pb) & 4 == 0);
            (split && !rim(seed, open, a.0, a.1) && !rim(seed, open, b.0, b.1)).then_some("hedge")
        }
        (Kind::Croft, k) | (k, Kind::Croft) if back(k) => Some("wattle"),
        (Kind::Open, k) | (k, Kind::Open) if back(k) => Some("hedge"),
        (Kind::Garden, Kind::Garden) if !urban && plot(a) != plot(b) => Some("wattle"),
        _ => None,
    }?;
    let gap = unit(hash_i(seed ^ 0xF3C5, a.0 + b.0, a.1 * 3 + b.1)) < 0.07;
    (!gap).then_some((WallRole::Run, kit))
}

/// Fences and hedges of the footprint inside a window, in global squares,
/// on the same edge lattice as [`edges`].
#[must_use]
pub fn fences(
    plan: &TownPlan,
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
) -> Vec<crate::block::interior::Edge> {
    let mut out = Vec::new();
    for y in y0..=y1 {
        for x in x0..x1 {
            if let Some(v) = fence(plan, (x, y - 1), (x, y)) {
                out.push(((x, y, EdgeAxis::Horizontal), v));
            }
        }
    }
    for y in y0..y1 {
        for x in x0..=x1 {
            if let Some(v) = fence(plan, (x - 1, y), (x, y)) {
                out.push(((x, y, EdgeAxis::Vertical), v));
            }
        }
    }
    out
}
