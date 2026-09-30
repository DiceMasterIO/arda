//! Building demand from the settlement's mix, and assignment of functions to
//! plots by location rules (goal 36): temples and inns on the square, smiths
//! and guardhouses by the gates, warehouses, mills and tanneries on the
//! water (tanneries downstream and outside), cottages on the fringes.

use super::grid::{Kind, PlanGrid};
use super::plots::RawPlot;
use super::types::{DistrictKind, StreetClass};
use crate::function::{BuildingFunction as F, MixKey};
use crate::geom::Vec2;
use crate::rng::{hash, unit};
use crate::site::{Tier, TownSite};
use std::collections::BTreeMap;

/// Buildings wanted, by function.
#[derive(Debug, Clone, Default)]
pub struct Demand {
    /// Counts per function.
    pub counts: BTreeMap<F, u32>,
    /// Notes on aliases and dropped keys.
    pub notes: Vec<String>,
}

impl Demand {
    /// Reads the site's building mix.
    #[must_use]
    pub fn from_site(site: &TownSite) -> Self {
        let mut d = Self::default();
        for (key, &n) in &site.buildings {
            match F::parse_mix(key) {
                MixKey::Building(f) => {
                    if f.key() != key {
                        d.notes
                            .push(format!("mix `{key}` ×{n} planned as `{}`", f.key()));
                    }
                    *d.counts.entry(f).or_default() += n;
                }
                MixKey::OffPlan(_) => d
                    .notes
                    .push(format!("mix `{key}` ×{n} lies outside the street plan")),
                MixKey::Unknown => d
                    .notes
                    .push(format!("mix `{key}` ×{n} is not a known function")),
            }
        }
        d
    }

    /// Count wanted for `f`.
    #[must_use]
    pub fn get(&self, f: F) -> u32 {
        self.counts.get(&f).copied().unwrap_or(0)
    }
}

/// Facts about one plot used for scoring.
#[derive(Debug, Clone)]
pub struct PlotInfo {
    /// Distance from the plot front to the market, metres.
    pub d_market: f64,
    /// Distance to the nearest gate, metres (large when unwalled).
    pub d_gate: f64,
    /// Water within four squares of the plot.
    pub water: bool,
    /// Fronts the market square.
    pub on_square: bool,
    /// Inside the wall.
    pub inside: bool,
    /// Street class.
    pub class: StreetClass,
    /// Frontage columns.
    pub width: i64,
    /// Mean depth in squares.
    pub depth: f64,
    /// Position along the river (downstream positive), metres.
    pub downstream: f64,
    /// District.
    pub district: DistrictKind,
}

/// Context for computing plot facts.
pub struct InfoCtx<'a> {
    /// Market centre.
    pub market: Vec2,
    /// Gate points.
    pub gates: &'a [Vec2],
    /// Wall ring.
    pub ring: Option<&'a [Vec2]>,
    /// Downstream unit direction, if a river.
    pub downstream: Option<Vec2>,
    /// Radius of the market district.
    pub market_r: f64,
    /// Tier.
    pub tier: Tier,
}

/// Computes plot facts and districts.
#[must_use]
pub fn info(g: &PlanGrid, plots: &[RawPlot], ctx: &InfoCtx<'_>) -> Vec<PlotInfo> {
    plots
        .iter()
        .map(|p| {
            let mid = p.cols[p.cols.len() / 2].front;
            let c = g.centre(mid.0, mid.1);
            let d_market = c.dist(ctx.market);
            let d_gate = ctx.gates.iter().map(|&q| q.dist(c)).fold(1e9, f64::min);
            let water = p.cells().iter().any(|&(i, j)| {
                (-4..=4).any(|d: i64| {
                    g.kind_at(i + d, j) == Kind::Water || g.kind_at(i, j + d) == Kind::Water
                })
            });
            let inside = ctx.ring.is_none_or(|r| crate::geom::inside(r, c));
            let on_square = p.street.is_none();
            let urban = ctx.tier == Tier::Town || ctx.tier == Tier::City;
            let district = if on_square || (urban && d_market < ctx.market_r) {
                DistrictKind::Market
            } else if water {
                DistrictKind::Waterfront
            } else if !inside {
                DistrictKind::Suburb
            } else if urban {
                DistrictKind::Residential
            } else {
                DistrictKind::Farmstead
            };
            PlotInfo {
                d_market,
                d_gate,
                water,
                on_square,
                inside,
                class: p.class,
                width: i64::try_from(p.cols.len()).unwrap_or(0),
                depth: p.mean_len(),
                downstream: ctx.downstream.map_or(0.0, |d| (c - ctx.market).dot(d)),
                district,
            }
        })
        .collect()
}

/// One assigned building: function and the plots it spans.
#[derive(Debug, Clone)]
pub struct Assigned {
    /// Function.
    pub function: F,
    /// Raw plot indices, in frontage order.
    pub plots: Vec<usize>,
}

fn score(f: F, nth: u32, p: &PlotInfo, r_core: f64, jitter: f64) -> f64 {
    let main = p.class == StreetClass::Main;
    let sq = if p.on_square { -400.0 } else { 0.0 };
    let edge = (r_core - p.d_market).abs();
    match f {
        F::Temple | F::Shrine | F::MarketHall => sq + p.d_market - p.depth * 3.0,
        F::Inn if nth == 0 => sq + p.d_market * if main { 1.0 } else { 3.0 },
        F::Inn | F::Guardhouse | F::Barracks | F::Stable => {
            p.d_gate + if main { 0.0 } else { 60.0 } + if p.inside { 0.0 } else { 30.0 }
        }
        F::Tavern => p.d_market * 0.6 + jitter * 80.0 + if main { 0.0 } else { 20.0 },
        F::Manor => {
            (p.d_market - r_core * 0.45).abs() - p.depth * 3.0 - p.width as f64 * 2.0
                + if p.on_square { 150.0 } else { 0.0 }
        }
        F::Smithy => (if p.d_gate < 1e8 { p.d_gate } else { edge }) + if main { 0.0 } else { 80.0 },
        F::Warehouse => {
            if p.water {
                p.d_market * 0.3 - 300.0
            } else {
                p.d_market + 60.0
            }
        }
        F::Mill => (if p.water { -400.0 } else { 0.0 }) - p.d_market * 0.4,
        F::Tannery => {
            (if p.water { -300.0 } else { 0.0 }) - p.downstream * 0.8 - p.d_market * 0.4
                + if p.inside { 60.0 } else { -100.0 }
        }
        F::Brewery => (if p.water { -80.0 } else { 0.0 }) + p.d_market * 0.5 + jitter * 60.0,
        F::Bakery => p.d_market * 0.7 + jitter * 120.0,
        F::Apothecary | F::Library | F::School => p.d_market + jitter * 30.0,
        F::Workshop => {
            (if p.district == DistrictKind::Craft {
                -200.0
            } else {
                0.0
            }) + p.d_market * 0.3
                + jitter * 100.0
        }
        F::Farmhouse | F::Barn => -(p.width as f64 * p.depth) + jitter * 30.0,
        F::Cottage => {
            -p.d_market
                + if p.class == StreetClass::Main {
                    60.0
                } else {
                    0.0
                }
        }
        F::Keep => sq + p.d_market - p.width as f64 * 5.0,
        _ => p.d_market + jitter * 10.0,
    }
}

/// Order in which functions pick plots.
const ORDER: [F; 23] = [
    F::Keep,
    F::Temple,
    F::MarketHall,
    F::Shrine,
    F::Inn,
    F::Manor,
    F::Guardhouse,
    F::Barracks,
    F::Mill,
    F::Warehouse,
    F::Tannery,
    F::Smithy,
    F::Brewery,
    F::Library,
    F::School,
    F::Apothecary,
    F::Tavern,
    F::Bakery,
    F::Workshop,
    F::Stable,
    F::Farmhouse,
    F::Cottage,
    F::House,
];

/// Plots needed side by side for `f` given the typical plot width.
#[must_use]
pub fn span(f: F, typical: i64, tier: Tier) -> usize {
    let ((w0, _), _) = f.size(tier);
    let n = (i64::from(w0) + typical - 2) / typical.max(1);
    usize::try_from(n.max(1)).unwrap_or(1)
}

/// Assigns functions to plots. `wanted` excludes buildings placed off-plot.
/// Returns the assignments and the counts that found no plot.
#[must_use]
pub fn assign(
    plots: &[RawPlot],
    info: &[PlotInfo],
    wanted: &BTreeMap<F, u32>,
    tier: Tier,
    r_core: f64,
    seed: u64,
    partial: bool,
) -> (Vec<Assigned>, BTreeMap<F, u32>) {
    let mut widths: Vec<i64> = info.iter().map(|p| p.width).collect();
    widths.sort_unstable();
    let typical = widths.get(widths.len() / 2).copied().unwrap_or(5).max(1);
    let need: usize = wanted
        .iter()
        .map(|(&f, &n)| span(f, typical, tier) * usize::try_from(n).unwrap_or(0))
        .sum();
    let mut unplaced: BTreeMap<F, u32> = BTreeMap::new();
    if plots.len() < need && !partial {
        unplaced.insert(
            F::House,
            u32::try_from(need - plots.len()).unwrap_or(u32::MAX),
        );
        return (Vec::new(), unplaced);
    }
    let mut taken = vec![false; plots.len()];
    // Neighbours: plots in the same run keyed by order.
    let mut by_run: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    for (i, p) in plots.iter().enumerate() {
        by_run.insert((p.run, p.order), i);
    }
    let mut out = Vec::new();
    let mass = |f: F| matches!(f, F::House | F::Cottage | F::Farmhouse);
    // Plots are in claim order, which is value order: the core first.
    let core = need + need / 8 + 2;
    for f in ORDER.into_iter().filter(|&f| !mass(f)) {
        let n = wanted.get(&f).copied().unwrap_or(0);
        let ((w0, _), _) = f.size(tier);
        let k = usize::try_from(w0).unwrap_or(4).saturating_sub(1).max(1);
        for nth in 0..n {
            let mut ranked: Vec<(f64, usize)> = (0..plots.len())
                .filter(|&i| !taken[i])
                .map(|i| {
                    let j = unit(hash(seed, u64::from(nth) ^ 0x51, i as u64));
                    let far = if i > core { 2000.0 } else { 0.0 };
                    (score(f, nth, &info[i], r_core, j) + far, i)
                })
                .collect();
            ranked.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let chosen = ranked
                .iter()
                .find_map(|&(_, i)| group_at(plots, &by_run, &taken, i, k));
            let Some(group) = chosen else {
                *unplaced.entry(f).or_default() += 1;
                continue;
            };
            for &i in &group {
                taken[i] = true;
            }
            out.push(Assigned {
                function: f,
                plots: group,
            });
        }
    }
    // Homes take the nearest free plots: farmhouses the largest of them,
    // cottages the outermost, town houses the rest.
    let count = |f: F| usize::try_from(wanted.get(&f).copied().unwrap_or(0)).unwrap_or(0);
    let (n_farm, n_cot, n_house) = (count(F::Farmhouse), count(F::Cottage), count(F::House));
    let mut homes: Vec<usize> = (0..plots.len())
        .filter(|&i| !taken[i])
        .take(n_farm + n_cot + n_house)
        .collect();
    let short = (n_farm + n_cot + n_house).saturating_sub(homes.len());
    let mut by_area = homes.clone();
    by_area.sort_by(|&a, &b| {
        let area = |i: usize| info[i].width as f64 * info[i].depth;
        area(b).total_cmp(&area(a)).then(a.cmp(&b))
    });
    let farms: Vec<usize> = by_area.into_iter().take(n_farm).collect();
    homes.retain(|i| !farms.contains(i));
    let cottages: Vec<usize> = homes.iter().rev().take(n_cot).copied().collect();
    homes.retain(|i| !cottages.contains(i));
    for (f, list) in [
        (F::Farmhouse, farms),
        (F::Cottage, cottages),
        (F::House, homes),
    ] {
        for i in list {
            out.push(Assigned {
                function: f,
                plots: vec![i],
            });
        }
    }
    if short > 0 {
        *unplaced.entry(F::House).or_default() += u32::try_from(short).unwrap_or(u32::MAX);
    }
    (out, unplaced)
}

/// Plots around `i` in its run, consecutive and free, whose frontage adds
/// up to at least `need_w` columns (at most four plots).
fn group_at(
    plots: &[RawPlot],
    by_run: &BTreeMap<(u32, u32), usize>,
    taken: &[bool],
    i: usize,
    need_w: usize,
) -> Option<Vec<usize>> {
    let p = &plots[i];
    if p.cols.len() >= need_w {
        return Some(vec![i]);
    }
    for start in (p.order.saturating_sub(3)..=p.order).rev() {
        let mut g = Vec::new();
        let mut width = 0;
        for o in start..start + 4 {
            let Some(j) = by_run.get(&(p.run, o)).copied().filter(|&j| !taken[j]) else {
                break;
            };
            g.push(j);
            width += plots[j].cols.len();
            if width >= need_w {
                break;
            }
        }
        if width >= need_w && g.contains(&i) && adjacent_chain(plots, &g) {
            return Some(g);
        }
    }
    None
}

/// Consecutive plots in a run must share an axis and touch along u.
fn adjacent_chain(plots: &[RawPlot], g: &[usize]) -> bool {
    g.windows(2).all(|w| {
        let (a, b) = (&plots[w[0]], &plots[w[1]]);
        if a.axis != b.axis {
            return false;
        }
        let (Some(la), Some(fb)) = (a.cols.last(), b.cols.first()) else {
            return false;
        };
        let vertical = matches!(a.axis, super::grid::Side::North | super::grid::Side::South);
        let du = if vertical {
            fb.front.0 - la.front.0
        } else {
            fb.front.1 - la.front.1
        };
        du.abs() == 1
    })
}
