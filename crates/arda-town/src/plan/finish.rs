//! Final assembly: releases unused plots, zones plot cells into fronts,
//! yards and gardens, assigns stable building ids, wealth and storeys, and
//! builds the district list.

use super::assign::{Assigned, PlotInfo};
use super::grid::{Kind, PlanGrid, Side, SQUARE_M};
use super::outline::outline;
use super::place::{merged, Draft};
use super::plots::RawPlot;
use super::types::{
    Bridge, Building, BuildingId, CastleWard, DistrictKind, Focal, Gate, MarketSquare, Plot,
    PlotId, Street, TownPlan, TownWall, WealthLevel,
};
use crate::function::BuildingFunction as F;
use crate::geom::Vec2;
use crate::num::round_u8;
use crate::rng::{hash, unit};
use crate::site::TownSite;
use std::collections::{BTreeMap, BTreeSet};

/// Workshop crafts, `arda-npc` `Craft` keys, assigned in turn.
const CRAFTS: [&str; 12] = [
    "carpentry",
    "weaving",
    "pottery",
    "cobbling",
    "masonry",
    "leatherwork",
    "woodcarving",
    "tinkering",
    "jewellery",
    "glassblowing",
    "painting",
    "cartography",
];

/// Everything the planner produced, ready to assemble.
pub struct Parts<'a> {
    /// The site.
    pub site: &'a TownSite,
    /// Plan seed.
    pub seed: u64,
    /// Absolute height of the market, metres.
    pub base_height_m: f64,
    /// The grid.
    pub grid: PlanGrid,
    /// Focal point.
    pub focal: Focal,
    /// Streets.
    pub streets: Vec<Street>,
    /// Bridge decks.
    pub bridges: Vec<Bridge>,
    /// Square.
    pub square: MarketSquare,
    /// Square radius, metres.
    pub square_r: f64,
    /// Wall ring.
    pub ring: Option<Vec<Vec2>>,
    /// Gates.
    pub gates: Vec<Gate>,
    /// Castle ward.
    pub castle: Option<CastleWard>,
    /// Raw plots.
    pub raw: Vec<RawPlot>,
    /// Raw plot facts.
    pub info: Vec<PlotInfo>,
    /// Assigned groups: assignment, first draft index, draft count.
    pub groups: Vec<(Assigned, usize, usize)>,
    /// Building drafts in id order.
    pub drafts: Vec<Draft>,
    /// Planner notes.
    pub notes: Vec<String>,
}

fn along(a: Side, c: (i64, i64)) -> i64 {
    let (dx, dy) = a.step();
    c.0 * dx + c.1 * dy
}

fn wealth_of(p: &Parts<'_>, d: &Draft, district: Option<DistrictKind>, i: usize) -> u8 {
    let base = f64::from(p.site.wealth);
    let dist = match district {
        Some(DistrictKind::Market) => 40.0,
        Some(DistrictKind::Religious) => 30.0,
        Some(DistrictKind::Residential) => 5.0,
        Some(DistrictKind::Craft) => -15.0,
        Some(DistrictKind::Waterfront) => -5.0,
        Some(DistrictKind::Suburb) => -45.0,
        Some(DistrictKind::Farmstead) => -10.0,
        Some(DistrictKind::Castle) | None => 20.0,
    };
    let func = match d.function {
        F::Manor => 70.0,
        F::Keep => 80.0,
        F::Temple | F::Library => 40.0,
        F::Inn | F::Apothecary | F::MarketHall => 15.0,
        F::Cottage => -45.0,
        F::Tannery | F::Barn | F::Stable => -30.0,
        F::Farmhouse => -10.0,
        _ => 0.0,
    };
    let jitter = (unit(hash(p.seed, 0xEA17, i as u64)) - 0.5) * 36.0;
    round_u8(base + dist + func + jitter)
}

/// Assembles the plan.
#[must_use]
pub fn finish(mut p: Parts<'_>) -> TownPlan {
    let g = &mut p.grid;
    // Release plots nobody builds on.
    let used: BTreeSet<usize> = p
        .groups
        .iter()
        .flat_map(|(a, _, _)| a.plots.clone())
        .collect();
    for k in 0..g.kind.len() {
        if g.kind[k] == Kind::Plot {
            let raw = usize::try_from(g.plot[k]).unwrap_or(0).saturating_sub(1);
            if !used.contains(&raw) {
                g.kind[k] = Kind::Open;
            }
        }
        g.plot[k] = 0;
        if g.building[k] == u32::MAX {
            g.building[k] = 0;
        }
    }
    // Buildings on the grid.
    for (i, d) in p.drafts.iter().enumerate() {
        let tag = u32::try_from(i + 1).unwrap_or(u32::MAX);
        for y in d.rect.y0..d.rect.y1 {
            for x in d.rect.x0..d.rect.x1 {
                if let Some(k) = g.gidx(x, y) {
                    g.building[k] = tag;
                    if d.function.walled() {
                        g.kind[k] = Kind::Building;
                    }
                }
            }
        }
    }
    // Plots, zoning and districts.
    let mut plots = Vec::new();
    let mut plot_of_draft: BTreeMap<usize, (PlotId, DistrictKind)> = BTreeMap::new();
    for (gi, (a, first, count)) in p.groups.iter().enumerate() {
        let id = PlotId(u32::try_from(gi).unwrap_or(u32::MAX));
        let m = merged(&p.raw, &a.plots);
        let axis = m.axis;
        let own = &p.drafts[*first..*first + *count];
        let rear = own
            .iter()
            .map(|d| {
                let corners = [(d.rect.x0, d.rect.y0), (d.rect.x1 - 1, d.rect.y1 - 1)];
                corners
                    .iter()
                    .map(|&(x, y)| along(axis, (x - g.gx0, y - g.gy0)))
                    .max()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0);
        let front = own
            .first()
            .map(|d| {
                let corners = [(d.rect.x0, d.rect.y0), (d.rect.x1 - 1, d.rect.y1 - 1)];
                corners
                    .iter()
                    .map(|&(x, y)| along(axis, (x - g.gx0, y - g.gy0)))
                    .min()
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        let sacred = matches!(a.function, F::Temple | F::Shrine);
        let cells = m.cells();
        for &(i, j) in &cells {
            let Some(k) = g.idx(i, j) else { continue };
            g.plot[k] = id.0 + 1;
            if g.kind[k] != Kind::Plot {
                continue;
            }
            let t = along(axis, (i, j));
            g.kind[k] = if sacred {
                Kind::Churchyard
            } else if t < front {
                Kind::Front
            } else if t <= rear + 3 {
                Kind::Yard
            } else {
                Kind::Garden
            };
        }
        let district = if sacred {
            DistrictKind::Religious
        } else {
            p.info[a.plots[0]].district
        };
        let poly: Vec<Vec2> = outline(&cells)
            .iter()
            .map(|&(i, j)| g.vertex(i, j))
            .collect();
        #[allow(clippy::cast_precision_loss)]
        let frontage = m.cols.len() as f64 * SQUARE_M;
        plots.push(Plot {
            id,
            street: m.street.map(|s| p.streets[s].id),
            front: axis.opposite(),
            polygon: poly,
            frontage_m: frontage,
            depth_m: m.mean_len() * SQUARE_M,
            district,
            inside_wall: p.info[a.plots[0]].inside,
        });
        for k in 0..*count {
            plot_of_draft.insert(first + k, (id, district));
        }
    }
    super::croft::fill(g, p.site.tier, p.seed);
    let buildings: Vec<Building> = p
        .drafts
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let pd = plot_of_draft.get(&i).copied();
            let district = pd.map(|x| x.1).or_else(|| {
                matches!(d.function, F::Keep | F::Barracks).then_some(DistrictKind::Castle)
            });
            let wealth = wealth_of(&p, d, district, i);
            let tags = if d.function == F::Workshop {
                vec![format!("craft:{}", CRAFTS[i % CRAFTS.len()])]
            } else {
                Vec::new()
            };
            Building {
                id: BuildingId(u64::try_from(i + 1).unwrap_or(u64::MAX)),
                function: d.function,
                plot: pd.map(|x| x.0),
                footprint: d.rect.polygon(),
                rect: d.rect,
                storeys: d.function.storeys(p.site.tier, wealth),
                wealth,
                wealth_level: WealthLevel::of(wealth),
                front: d.front,
                doors: d.doors.clone(),
                ancillary: d.ancillary,
                tags,
            }
        })
        .collect();
    let mut districts: BTreeMap<DistrictKind, Vec<PlotId>> = BTreeMap::new();
    for pl in &plots {
        districts.entry(pl.district).or_default().push(pl.id);
    }
    let _ = p.square_r;
    let water_level = p
        .grid
        .kind
        .iter()
        .zip(&p.grid.height)
        .filter(|(k, _)| matches!(k, Kind::Water | Kind::Bridge | Kind::WaterGate))
        .map(|(_, &h)| f64::from(h))
        .fold(f64::MAX, f64::min);
    let water_level = if water_level == f64::MAX {
        0.0
    } else {
        water_level
    } + p.base_height_m;
    TownPlan {
        site: p.site.id,
        name: p.site.name.clone(),
        tier: p.site.tier,
        culture: p.site.culture.clone(),
        seed: p.seed,
        base_height_m: p.base_height_m,
        water_level_m: water_level,
        focal: p.focal,
        streets: p.streets,
        bridges: p.bridges,
        square: Some(p.square),
        plots,
        districts: districts.into_iter().collect(),
        wall: p.ring.map(|ring| TownWall {
            ring,
            gates: p.gates,
            thickness_m: super::wall::THICKNESS_M,
        }),
        castle: p.castle,
        buildings,
        notes: p.notes,
        grid: p.grid,
    }
}
