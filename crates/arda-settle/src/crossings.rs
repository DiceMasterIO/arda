//! Crossing objects and passes on the built network (artifact "Roads").
//!
//! Every place a road meets a watercourse is classified: small streams are
//! bridged near settlements and forded elsewhere; middling rivers are bridged
//! on trunk roads or near settlements and forded or ferried otherwise; wide
//! rivers are ferried unless a trunk road and a nearby town justify a bridge.
//! Every stretch of road over open water is a ferry. Where a road climbs over
//! a ridge, the high point of its profile is a pass.

use crate::error::SettleError;
use crate::field::distance_m;
use crate::grid::Grid;
use crate::model::{Settlement, Tier};
use crate::roads::{Network, RoadClass};
use crate::tags::{self, Sites};
use serde::{Deserialize, Serialize};

/// How a road gets over water.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossingKind {
    /// A bridge.
    Bridge,
    /// A ford.
    Ford,
    /// A ferry.
    Ferry,
}

/// One crossing object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crossing {
    /// 1-based id.
    #[serde(with = "crate::ids::string")]
    pub id: u64,
    /// Bridge, ford or ferry.
    pub kind: CrossingKind,
    /// What is crossed: `river`, `sea` or `lake`.
    pub water: String,
    /// Position, metres.
    pub x_m: i64,
    /// Position, metres.
    pub y_m: i64,
    /// Channel width in metres (0 for open water).
    pub width_m: u32,
    /// Strahler order (0 for open water).
    pub order: u8,
    /// Highest road class using the crossing.
    pub road_class: RoadClass,
    /// Name of the river, filled by the naming stage.
    pub river: String,
}

/// One pass object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pass {
    /// 1-based id.
    #[serde(with = "crate::ids::string")]
    pub id: u64,
    /// Position, metres.
    pub x_m: i64,
    /// Position, metres.
    pub y_m: i64,
    /// Height of the high point, metres.
    pub height_m: i32,
    /// Climb from the lowest point on the route's start side, metres.
    pub rise_from_m: i32,
    /// Climb from the lowest point on the route's end side, metres.
    pub rise_to_m: i32,
    /// Road whose profile peaks here.
    #[serde(with = "crate::ids::string")]
    pub road_id: u64,
    /// Name, filled by the naming stage.
    pub name: String,
    /// English gloss of the name.
    pub name_gloss: String,
    /// Grid cell (not serialised).
    #[serde(skip)]
    pub cell: usize,
}

/// Width thresholds, metres.
const SMALL_M: u32 = 14;
const WIDE_M: u32 = 50;
const FORD_MAX_M: u32 = 20;
/// A pass must stand this far above the valleys on both sides, metres.
const PASS_RISE_M: i32 = 120;

/// Finds every crossing on the network.
///
/// # Errors
/// [`SettleError::Reserve`] when a raster cannot be allocated.
pub fn crossings(
    g: &Grid,
    net: &Network,
    settlements: &[Settlement],
) -> Result<(Vec<Crossing>, Vec<usize>), SettleError> {
    let centres = |min: Tier| {
        let mut v = vec![false; g.len()];
        for s in settlements.iter().filter(|s| s.tier >= min) {
            v[s.index(g.width)] = true;
        }
        v
    };
    let near_village = {
        let c = centres(Tier::Village);
        distance_m(g, |i| c[i], 3001)?
    };
    let near_town = {
        let c = centres(Tier::Town);
        distance_m(g, |i| c[i], 3001)?
    };
    let mut seen = vec![false; g.len()];
    let mut out = Vec::new();
    let mut cells = Vec::new();
    for start in 0..g.len() {
        if seen[start] || net.raster[start] == 0 {
            continue;
        }
        let river = g.is_watercourse(start);
        let open = g.is_water(start);
        if !river && !open {
            continue;
        }
        // One crossing per connected stretch of road over the same kind of water.
        let same =
            |i: usize| net.raster[i] != 0 && g.is_watercourse(i) == river && g.is_water(i) == open;
        let mut stack = vec![start];
        seen[start] = true;
        let mut widest = start;
        let mut class: Option<RoadClass> = None;
        while let Some(c) = stack.pop() {
            let here = RoadClass::from_code(net.raster[c]).filter(|r| *r != RoadClass::None);
            class = match (class, here) {
                (Some(a), Some(b)) if b.rank() > a.rank() => Some(b),
                (None, b) => b,
                (a, _) => a,
            };
            if g.width_dm[c] > g.width_dm[widest]
                || (g.width_dm[c] == g.width_dm[widest] && c < widest)
            {
                widest = c;
            }
            for (j, _, _) in g.neighbours8(c) {
                if !seen[j] && same(j) {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        let road_class = class.unwrap_or(RoadClass::Footpath);
        let w = g.width_m(widest);
        let trunk = road_class == RoadClass::Highway;
        let settled = near_village[widest] <= 1000;
        let kind = if open {
            CrossingKind::Ferry
        } else if w < SMALL_M {
            if settled || trunk {
                CrossingKind::Bridge
            } else {
                CrossingKind::Ford
            }
        } else if w <= WIDE_M {
            if trunk || settled {
                CrossingKind::Bridge
            } else if w <= FORD_MAX_M {
                CrossingKind::Ford
            } else {
                CrossingKind::Ferry
            }
        } else if trunk && near_town[widest] <= 3000 {
            CrossingKind::Bridge
        } else {
            CrossingKind::Ferry
        };
        let (x, y) = g.xy(widest);
        let water = if river {
            "river"
        } else if g.terrain[widest] == arda::TerrainKind::Lake {
            "lake"
        } else {
            "sea"
        };
        out.push(Crossing {
            id: u64::try_from(out.len() + 1).unwrap_or(u64::MAX),
            kind,
            water: water.to_string(),
            x_m: x * 100 + 50,
            y_m: y * 100 + 50,
            width_m: if river { w } else { 0 },
            order: if river { g.order[widest] } else { 0 },
            road_class,
            river: String::new(),
        });
        cells.push(widest);
    }
    Ok((out, cells))
}

/// Finds passes: the high point of a route that stands at least 120 m above
/// the valleys on both sides and sits on or beside a saddle.
#[must_use]
pub fn passes(g: &Grid, sites: &Sites, net: &Network) -> Vec<Pass> {
    let mut found: Vec<Pass> = Vec::new();
    let mut order: Vec<usize> = (0..net.roads.len()).collect();
    // Higher classes claim a pass first.
    order.sort_by_key(|&k| (std::cmp::Reverse(net.roads[k].class.rank()), k));
    for k in order {
        let path = &net.routes[k];
        let Some((top, &peak)) = path
            .iter()
            .enumerate()
            .max_by_key(|&(n, &c)| (g.height_mm[c], std::cmp::Reverse(n)))
        else {
            continue;
        };
        let low = |cells: &[usize]| cells.iter().map(|&c| g.height_mm[c]).min();
        let (Some(a), Some(b)) = (low(&path[..=top]), low(&path[top..])) else {
            continue;
        };
        let h = g.height_mm[peak];
        let (ra, rb) = ((h - a) / 1000, (h - b) / 1000);
        let (x, y) = g.xy(peak);
        let saddle = (-3..=3_i64).any(|oy| {
            (-3..=3_i64).any(|ox| {
                g.at(x + ox, y + oy)
                    .is_some_and(|j| sites.tags[j] & tags::PASS != 0)
            })
        });
        let dup = found.iter().any(|p| {
            let (px, py) = g.xy(p.cell);
            (px - x).abs() <= 10 && (py - y).abs() <= 10
        });
        if ra >= PASS_RISE_M && rb >= PASS_RISE_M && saddle && !dup {
            found.push(Pass {
                id: u64::try_from(found.len() + 1).unwrap_or(u64::MAX),
                x_m: x * 100 + 50,
                y_m: y * 100 + 50,
                height_m: h / 1000,
                rise_from_m: ra,
                rise_to_m: rb,
                road_id: net.roads[k].id,
                name: String::new(),
                name_gloss: String::new(),
                cell: peak,
            });
        }
    }
    found
}
