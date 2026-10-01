//! The world-level plan: every way, channel, crossing and toll house near a
//! window, computed from world data alone so that neighbouring windows
//! build identical geometry (goal 46).

pub mod crossing;
pub mod house;
pub mod relevant;
pub mod switchback;
pub mod wet;

use crate::curve::{self, dist, Dense, Piece, Station, P};
use crate::error::WaysError;
use crate::input::{ClassSpec, Crossing, RiverChannel, Road, RoadClass, Terrain, SQUARE_M};
use std::cmp::Reverse;

/// Pieces within this distance of the window are sampled densely. It covers
/// the straightened approaches of any crossing that can reach the window.
pub const DENSE_MARGIN_M: f64 = 400.0;

/// A window on the global square lattice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    /// Global column of the window's west edge.
    pub gx0: i64,
    /// Global row of the window's north edge.
    pub gy0: i64,
    /// Width in squares.
    pub width: u32,
    /// Height in squares.
    pub height: u32,
}

/// Centre of global square `(gx, gy)` in world metres; exact in `f64`.
#[must_use]
#[allow(clippy::cast_precision_loss)] // global square indices are far below 2^52
pub fn square_centre(gx: i64, gy: i64) -> P {
    [(gx as f64 + 0.5) * SQUARE_M, (gy as f64 + 0.5) * SQUARE_M]
}

/// The global square index containing world coordinate `m`.
#[must_use]
#[allow(clippy::cast_possible_truncation)] // world extents are far below 2^63 squares
pub fn square_of(m: f64) -> i64 {
    (m / SQUARE_M).floor() as i64
}

/// Largest window origin, in squares from the world origin, on either axis.
pub const MAX_ORIGIN_SQ: f64 = (1_u64 << 40) as f64;

impl Window {
    /// A window whose north-west corner is `origin_m`, which must lie on the
    /// square lattice.
    ///
    /// # Errors
    /// [`WaysError::UnalignedOrigin`] for an off-lattice origin.
    #[allow(clippy::cast_possible_truncation)] // checked to be integral
    pub fn new(origin_m: [f64; 2], width: u32, height: u32) -> Result<Self, WaysError> {
        let (fx, fy) = (origin_m[0] / SQUARE_M, origin_m[1] / SQUARE_M);
        if fx.fract() != 0.0 || fy.fract() != 0.0 || !fx.is_finite() || !fy.is_finite() {
            return Err(WaysError::UnalignedOrigin {
                x_m: origin_m[0],
                y_m: origin_m[1],
                square_m: SQUARE_M,
            });
        }
        // Square arithmetic adds widths to the origin in i64; keep far from
        // saturation (2^40 squares is 1.7e12 m, beyond any world).
        if fx.abs() > MAX_ORIGIN_SQ || fy.abs() > MAX_ORIGIN_SQ {
            return Err(WaysError::Limit(format!(
                "window origin ({}, {}) m is outside ±{MAX_ORIGIN_SQ} squares",
                origin_m[0], origin_m[1]
            )));
        }
        Ok(Self {
            gx0: fx as i64,
            gy0: fy as i64,
            width,
            height,
        })
    }

    /// Distance in metres from `p` to the window rectangle (zero inside).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // see `square_centre`
    pub fn dist_m(&self, p: P) -> f64 {
        let x0 = self.gx0 as f64 * SQUARE_M;
        let y0 = self.gy0 as f64 * SQUARE_M;
        let x1 = x0 + f64::from(self.width) * SQUARE_M;
        let y1 = y0 + f64::from(self.height) * SQUARE_M;
        let dx = (x0 - p[0]).max(p[0] - x1).max(0.0);
        let dy = (y0 - p[1]).max(p[1] - y1).max(0.0);
        dx.hypot(dy)
    }

    /// Whether axis-aligned bounds `[x0, y0, x1, y1]` come within `margin`
    /// metres of the window.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // see `square_centre`
    pub fn near_bounds(&self, b: [f64; 4], margin: f64) -> bool {
        let x0 = self.gx0 as f64 * SQUARE_M;
        let y0 = self.gy0 as f64 * SQUARE_M;
        let x1 = x0 + f64::from(self.width) * SQUARE_M;
        let y1 = y0 + f64::from(self.height) * SQUARE_M;
        let dx = (b[0] - x1).max(x0 - b[2]).max(0.0);
        let dy = (b[1] - y1).max(y0 - b[3]).max(0.0);
        dx.hypot(dy) <= margin
    }
}

/// One road segment (a way) with its smooth, graded centreline.
#[derive(Debug, Clone)]
pub struct WayPlan {
    /// Road id.
    pub road_id: u64,
    /// Segment index inside the road.
    pub segment: usize,
    /// Road class.
    pub class: RoadClass,
    /// Wealth, 0–255.
    pub wealth: u8,
    /// Class geometry.
    pub spec: ClassSpec,
    /// Polyline vertices after junction snapping.
    pub vertices: Vec<P>,
    /// Every piece of the centreline.
    pub pieces: Vec<Piece>,
    /// Dense stations of the pieces near the window.
    pub dense: Dense,
    /// Pieces given switchbacks.
    pub switchbacks: u32,
    /// Pieces still over the class grade after the widest wave.
    pub over_grade: u32,
}

impl WayPlan {
    /// Reach of the way's widest band (surface, verge, ditch, shoulder), metres.
    #[must_use]
    pub fn reach_m(&self) -> f64 {
        (f64::from(self.spec.width_sq) / 2.0 + self.spec.verge_sq + 3.0) * SQUARE_M
    }
}

/// A junction where a way's end meets another way's interior.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Junction {
    /// Junction point on the main way.
    pub at: P,
    /// Index of the main way.
    pub main: usize,
    /// Index of the joining way.
    pub branch: usize,
    /// Unit direction from the junction into the branch.
    pub branch_dir: P,
}

/// A river channel with its dense centreline.
#[derive(Debug, Clone)]
pub struct ChannelPlan {
    /// Channel id; a synthetic channel carries its crossing's id (ids are
    /// only reported, never looked up).
    pub id: u64,
    /// Width, metres.
    pub width_m: f64,
    /// Depth at the centreline, metres.
    pub depth_m: f64,
    /// Dense centreline.
    pub dense: Dense,
    /// Only guides crossings: the terrain rasterises this river's water
    /// ([`Terrain::rivers_rasterised`]), so the channel paints none.
    pub guide: bool,
}

impl ChannelPlan {
    /// Builds a channel from a description.
    #[must_use]
    pub fn new(c: &RiverChannel) -> Self {
        let runs = curve::shapes(&c.centreline)
            .into_iter()
            .map(|s| Piece::new(s, None, 0.0, 0.0))
            .fold(Vec::<Station>::new(), |mut acc, p| {
                let st = p.stations();
                let skip = usize::from(!acc.is_empty());
                acc.extend(st.into_iter().skip(skip));
                acc
            });
        Self {
            id: c.id,
            width_m: c.width_m.max(SQUARE_M),
            depth_m: c.depth_m.max(0.3),
            dense: Dense::new(vec![runs]),
            guide: false,
        }
    }

    /// Distance from `p` to the centreline if within `reach`.
    #[must_use]
    pub fn dist(&self, p: P, reach: f64) -> Option<f64> {
        self.dense.nearest(p, reach).map(|h| h.d)
    }

    /// Whether the square centred at `p` is water.
    #[must_use]
    pub fn is_water(&self, p: P) -> bool {
        self.dist(p, self.width_m / 2.0).is_some()
    }
}

/// Whether global square `(gx, gy)` is water: the terrain's raster when it
/// has one, plus every channel that is not only a guide.
#[must_use]
pub fn is_water(channels: &[ChannelPlan], terrain: &dyn Terrain, gx: i64, gy: i64) -> bool {
    (terrain.rivers_rasterised() && terrain.river_water(gx, gy))
        || channels
            .iter()
            .any(|c| !c.guide && c.is_water(square_centre(gx, gy)))
}

/// Everything planned for a window.
#[derive(Debug, Clone)]
pub struct Plan {
    /// The window.
    pub window: Window,
    /// Ways, highest class first.
    pub ways: Vec<WayPlan>,
    /// Channels, given and synthetic.
    pub channels: Vec<ChannelPlan>,
    /// Junctions.
    pub junctions: Vec<Junction>,
    /// Crossings that could be resolved against a way and a channel.
    pub crossings: Vec<crossing::CrossingPlan>,
    /// Crossing ids near the window that met no way or no water.
    pub orphans: Vec<u64>,
    /// Toll houses and waystations.
    pub houses: Vec<house::House>,
}

/// Plans every way, channel, crossing and toll house that can reach `win`.
#[must_use]
pub fn build(
    win: Window,
    roads: &[Road],
    crossings: &[Crossing],
    terrain: &dyn Terrain,
    seed: u64,
    standing: &crate::standing::Standing,
) -> Plan {
    build_inner(win, roads, crossings, terrain, seed, standing, true)
}

pub(crate) fn build_inner(
    win: Window,
    roads: &[Road],
    crossings: &[Crossing],
    terrain: &dyn Terrain,
    seed: u64,
    standing: &crate::standing::Standing,
    filter: bool,
) -> Plan {
    let guide = terrain.rivers_rasterised();
    let mut channels: Vec<ChannelPlan> = terrain
        .channels()
        .iter()
        .map(|c| ChannelPlan {
            guide,
            ..ChannelPlan::new(c)
        })
        .collect();
    let (mut ways, junctions) = plan_ways(win, roads, terrain, filter);
    let (crossings, orphans) = if guide {
        wet::plan_all(win, crossings, &mut ways, &mut channels, terrain, standing)
    } else {
        crossing::plan_all(win, crossings, &mut ways, &mut channels, terrain, standing)
    };
    let houses = house::plan_all(&crossings, &ways, &channels, seed);
    Plan {
        window: win,
        ways,
        channels,
        junctions,
        crossings,
        orphans,
        houses,
    }
}

pub(super) fn to_p(v: [i64; 2]) -> P {
    #[allow(clippy::cast_precision_loss)] // world metres are far below 2^52
    [v[0] as f64, v[1] as f64]
}

/// Distance from `p` to a raw polyline and whether the foot is an end vertex.
pub(super) fn raw_dist(v: &[P], p: P) -> (f64, bool) {
    let mut best = (f64::MAX, false);
    for (i, w) in v.windows(2).enumerate() {
        let d = [w[1][0] - w[0][0], w[1][1] - w[0][1]];
        let l2 = d[0] * d[0] + d[1] * d[1];
        let t = if l2 > 0.0 {
            (((p[0] - w[0][0]) * d[0] + (p[1] - w[0][1]) * d[1]) / l2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let dd = dist(p, curve::lerp(w[0], w[1], t));
        let at_end = (i == 0 && t <= 0.0) || (i + 2 == v.len() && t >= 1.0);
        if dd < best.0 {
            best = (dd, at_end);
        }
    }
    best
}

/// The nearest point of a way's full curve to `p` (window-independent).
fn curve_nearest(w: &WayPlan, p: P, reach: f64) -> Option<curve::Hit> {
    let runs: Vec<Vec<Station>> = w
        .pieces
        .iter()
        .filter(|pc| {
            let b = pc.bounds();
            p[0] >= b[0] - reach
                && p[0] <= b[2] + reach
                && p[1] >= b[1] - reach
                && p[1] <= b[3] + reach
        })
        .map(Piece::stations)
        .collect();
    Dense::new(runs).nearest(p, reach)
}

fn plan_ways(
    win: Window,
    roads: &[Road],
    terrain: &dyn Terrain,
    filter: bool,
) -> (Vec<WayPlan>, Vec<Junction>) {
    let keep = if filter {
        relevant::relevant_roads(win, roads)
    } else {
        vec![true; roads.len()]
    };
    let mut order: Vec<&Road> = roads
        .iter()
        .zip(&keep)
        .filter(|(_, &k)| k)
        .map(|(r, _)| r)
        .collect();
    order.retain(|r| r.class != RoadClass::None);
    order.sort_by_key(|r| (Reverse(r.class.hierarchy()), r.id));
    let mut ways: Vec<WayPlan> = Vec::new();
    let mut junctions = Vec::new();
    for road in order {
        let spec = road.class.spec(road.wealth);
        for (si, seg) in road.segments.iter().enumerate() {
            let mut v: Vec<P> = seg.iter().copied().map(to_p).collect();
            if v.len() < 2 {
                continue;
            }
            let idx = ways.len();
            for end in [0, v.len() - 1] {
                let e = v[end];
                let next = if end == 0 { v[1] } else { v[end - 1] };
                for (mi, m) in ways.iter().enumerate() {
                    let (d, at_end) = raw_dist(&m.vertices, e);
                    if d > 1.0 || at_end {
                        continue;
                    }
                    if let Some(h) = curve_nearest(m, e, 40.0) {
                        v[end] = h.foot;
                        junctions.push(Junction {
                            at: h.foot,
                            main: mi,
                            branch: idx,
                            branch_dir: curve::unit([next[0] - h.foot[0], next[1] - h.foot[1]]),
                        });
                        break;
                    }
                }
            }
            ways.push(plan_way(win, road, si, v, spec, terrain));
        }
    }
    (ways, junctions)
}

fn plan_way(
    win: Window,
    road: &Road,
    segment: usize,
    v: Vec<P>,
    spec: ClassSpec,
    terrain: &dyn Terrain,
) -> WayPlan {
    let mut pieces = Vec::new();
    let (mut switchbacks, mut over_grade, mut s) = (0, 0, 0.0);
    for shape in curve::shapes(&v) {
        let (a, b) = shape.ends();
        let (z0, z1) = (terrain.height_m(a[0], a[1]), terrain.height_m(b[0], b[1]));
        let (mut pc, over) = switchback::piece(shape, z0, z1, &spec);
        switchbacks += u32::from(pc.zig.is_some());
        over_grade += u32::from(over);
        pc.s0 = s;
        s += pc.len;
        pieces.push(pc);
    }
    let mut runs: Vec<Vec<Station>> = Vec::new();
    let mut open = false;
    for pc in &pieces {
        if !win.near_bounds(pc.bounds(), DENSE_MARGIN_M) {
            open = false;
            continue;
        }
        let mut st = pc.stations();
        if pc.zig.is_none() {
            // Off switchbacks the road follows the ground; on them it keeps
            // the constant design grade and cuts or fills to it.
            for x in &mut st {
                x.z = terrain.height_m(x.p[0], x.p[1]);
            }
        }
        match runs.last_mut() {
            Some(run) if open => run.extend(st.into_iter().skip(1)),
            _ => runs.push(st),
        }
        open = true;
    }
    WayPlan {
        road_id: road.id,
        segment,
        class: road.class,
        wealth: road.wealth,
        spec,
        vertices: v,
        pieces,
        dense: Dense::new(runs),
        switchbacks,
        over_grade,
    }
}
