//! Canonical saved-channel coverage for the opt-in Atlas overview.
use super::{atlas_river_colour, Feature};
use crate::channel_curve::{
    curved_strip, min_width_px_q8, segments as curve_segments, source_width, Network,
};
use crate::channel_geometry::{
    area2, coverage_overview, hull, intersection, rectangle, strip, strip_free, terminal_footprint,
    Point, Polygon, WorkBudget, Q,
};
use crate::RenderError;
use arda_core::{
    hydrology::{ChannelEdge, ReachId},
    AreaCoord, AreaObjects, GlobalCell,
};
use std::collections::{BTreeMap, BTreeSet};

const LEGACY_DISPLAY_WIDTH_DM: u64 = 10_000;
// Recipe 4 uses a narrower symbol to keep major channels legible without
// covering their carved valleys at fitted overview scale.
const FINE_DISPLAY_WIDTH_DM: u64 = 4_000;
// Recipe 5: four times physical width, so trunk rivers read at fitted-world
// scale and small streams appear as the export grows (goals 28, 33).
const FORMED_DISPLAY_WIDTH_DM: u64 = 12_000;
const CONTEXT_CELLS: i64 = 7;
const MAX_CONTEXT_EDGES: usize = 262_144;
const MAX_SHAPES: usize = 1_048_576;
const MAX_VERTICES: usize = 4_194_304;
const MAX_PIXEL_CAPACITY: usize = 1_048_576;
const MAX_PIXEL_CANDIDATES: usize = 4096;
const MAX_WORK: u64 = 250_000_000;
fn bad(reason: &'static str) -> RenderError {
    RenderError::ChannelGeometry { reason }
}
fn context_error(reason: &'static str) -> RenderError {
    RenderError::AtlasContext { reason }
}
fn relevant(at: AreaCoord, edge: ChannelEdge) -> bool {
    let x = i64::from(at.x) * 512;
    let y = i64::from(at.y) * 512;
    let minx = i64::from(edge.from.x.min(edge.to.x));
    let maxx = i64::from(edge.from.x.max(edge.to.x));
    let miny = i64::from(edge.from.y.min(edge.to.y));
    let maxy = i64::from(edge.from.y.max(edge.to.y));
    minx <= x + 511 + CONTEXT_CELLS
        && maxx >= x - CONTEXT_CELLS
        && miny <= y + 511 + CONTEXT_CELLS
        && maxy >= y - CONTEXT_CELLS
}
fn key(e: &ChannelEdge) -> (GlobalCell, GlobalCell) {
    (e.from, e.to)
}
/// Canonical, bounded saved geometry for one Atlas overview area, including its adjacent-area context.
pub struct OverviewChannelContext {
    target: AreaCoord,
    areas_wide: i32,
    areas_high: i32,
    seen: BTreeSet<AreaCoord>,
    edges: Vec<ChannelEdge>,
    /// Braided course cells with their belt width, dm (logic/04
    /// §atlas-formed braids).
    braided: std::collections::BTreeMap<GlobalCell, u32>,
    ready: bool,
}
impl OverviewChannelContext {
    /// Starts a context for one area of the saved exported rectangle.
    pub fn new(target: AreaCoord, areas_wide: i32, areas_high: i32) -> Result<Self, RenderError> {
        if !(1..=78).contains(&areas_wide)
            || !(1..=78).contains(&areas_high)
            || target.x < 0
            || target.y < 0
            || target.x >= areas_wide
            || target.y >= areas_high
        {
            return Err(context_error("invalid Atlas channel target rectangle"));
        }
        Ok(Self {
            target,
            areas_wide,
            areas_high,
            seen: BTreeSet::new(),
            edges: Vec::new(),
            braided: std::collections::BTreeMap::new(),
            ready: false,
        })
    }
    /// Adds one saved area's bounded channel edges and terminal reach points.
    pub fn add_area(&mut self, at: AreaCoord, objects: &AreaObjects) -> Result<(), RenderError> {
        if self.ready {
            return Err(context_error("Atlas channel context already finished"));
        }
        if at.x < 0
            || at.y < 0
            || at.x >= self.areas_wide
            || at.y >= self.areas_high
            || at.x.abs_diff(self.target.x) > 1
            || at.y.abs_diff(self.target.y) > 1
        {
            return Err(context_error(
                "Atlas channel context area outside adjacent neighborhood",
            ));
        }
        if !self.seen.insert(at) {
            return Err(context_error("duplicate Atlas channel context area"));
        }
        for &edge in &objects.channel_edges {
            if relevant(self.target, edge) {
                self.push(edge)?;
            }
        }
        for reach in &objects.global.reaches {
            if !reach.id.is_point() || reach.mean_discharge.raw() < 40 {
                continue;
            }
            if reach.from != reach.to
                || reach.id
                    != ReachId::point(reach.from)
                        .ok_or_else(|| bad("invalid saved terminal identity"))?
            {
                return Err(bad("invalid saved terminal point"));
            }
            let width = arda_core::hydrology::channel_width_dm(reach.mean_discharge)
                .ok_or_else(|| bad("terminal width overflow"))?;
            let edge = ChannelEdge {
                from: reach.from,
                to: reach.to,
                from_width_dm: width,
                to_width_dm: width,
                discharge: reach.mean_discharge,
            };
            if relevant(self.target, edge) {
                self.push(edge)?;
            }
        }
        Ok(())
    }
    /// Marks saved braided course cells (global) with their belt width in
    /// decimetres; recipe-5 overviews draw two extra threads weaving
    /// across the belt there.
    pub fn add_braided(
        &mut self,
        cells: impl IntoIterator<Item = (GlobalCell, u32)>,
    ) -> Result<(), RenderError> {
        if self.ready {
            return Err(context_error("Atlas channel context already finished"));
        }
        for (cell, belt) in cells {
            let e = self.braided.entry(cell).or_insert(belt);
            *e = (*e).max(belt);
        }
        Ok(())
    }
    fn push(&mut self, edge: ChannelEdge) -> Result<(), RenderError> {
        if self.edges.len() == MAX_CONTEXT_EDGES {
            return Err(bad("Atlas overview context exceeds 262144 channel edges"));
        }
        self.edges.push(edge);
        Ok(())
    }
    /// Requires every existing adjacent saved area and canonicalizes duplicate halo records.
    pub fn finish(mut self) -> Result<Self, RenderError> {
        for y in (self.target.y - 1).max(0)..=(self.target.y + 1).min(self.areas_high - 1) {
            for x in (self.target.x - 1).max(0)..=(self.target.x + 1).min(self.areas_wide - 1) {
                if !self.seen.contains(&AreaCoord::new(x, y)) {
                    return Err(context_error("missing adjacent Atlas channel context"));
                }
            }
        }
        self.edges.sort_unstable_by_key(|e| {
            (
                e.from,
                e.to,
                e.from_width_dm,
                e.to_width_dm,
                e.discharge.raw(),
            )
        });
        self.edges.dedup();
        if self.edges.windows(2).any(|p| key(&p[0]) == key(&p[1])) {
            return Err(bad(
                "duplicate channel endpoints disagree about saved fields",
            ));
        }
        self.ready = true;
        Ok(self)
    }
    fn edges(&self, at: AreaCoord) -> Result<&[ChannelEdge], RenderError> {
        if !self.ready || self.target != at {
            return Err(context_error(
                "Atlas channel context target or completeness mismatch",
            ));
        }
        Ok(&self.edges)
    }
}
struct Cap {
    node: GlobalCell,
    points: [Point; 2],
    discharge: u64,
}
struct Shape {
    polygon: Polygon,
    bounds: [u32; 4],
    discharge: u64,
}
fn display_width(width: u32, style: super::ChannelStyle) -> Result<i64, RenderError> {
    if width == 0 {
        return Err(bad("saved channel has zero width"));
    }
    let dm = match style {
        super::ChannelStyle::Legacy => (u64::from(width) * 8).min(LEGACY_DISPLAY_WIDTH_DM),
        super::ChannelStyle::Fine => (u64::from(width) * 3).min(FINE_DISPLAY_WIDTH_DM),
        super::ChannelStyle::Formed => (u64::from(width) * 4).min(FORMED_DISPLAY_WIDTH_DM),
    };
    i64::try_from(dm).map_err(|_| bad("display width conversion overflow"))
}
/// Recipe-5 river water as v0.1 drew it.
const fn formed_river_colour_v5(band: super::RiverBand) -> [u8; 3] {
    match band {
        super::RiverBand::Light => [70, 150, 172],
        super::RiverBand::Mid => [44, 128, 164],
        super::RiverBand::Dark => [28, 100, 150],
    }
}

/// Recipe-5 minimum on-screen width by discharge, pixels Q8 (v0.1 steps).
const fn min_width_px_q8_v5(discharge: u64) -> i64 {
    if discharge >= 50_000 {
        410
    } else if discharge >= 5_000 {
        256
    } else {
        0
    }
}

/// Blue-teal formed river water, deepening downstream (goal 28; recipe 6).
pub(crate) const fn formed_river_colour(band: super::RiverBand) -> [u8; 3] {
    match band {
        super::RiverBand::Light => [66, 142, 166],
        super::RiverBand::Mid => [50, 124, 156],
        super::RiverBand::Dark => [36, 104, 146],
    }
}

fn center(node: GlobalCell) -> Point {
    (i64::from(node.x) * Q + Q / 2, i64::from(node.y) * Q + Q / 2)
}
fn to_output(v: i64, source0: i64, pixel0: u32, span: u32) -> Result<i64, RenderError> {
    let n = i128::from(v - source0) * i128::from(span);
    let delta = (n + 256) / 512;
    let base = i128::from(pixel0) * i128::from(Q);
    i64::try_from(base + delta).map_err(|_| bad("Atlas overview channel coordinate overflow"))
}
fn append(
    out: &mut Vec<Shape>,
    polygon: &Polygon,
    discharge: u64,
    at: AreaCoord,
    bounds: [u32; 4],
    work: &mut WorkBudget,
    vertices: &mut usize,
) -> Result<(), RenderError> {
    let source_x = i64::from(at.x) * 512 * Q;
    let source_y = i64::from(at.y) * 512 * Q;
    let source_rect = rectangle(source_x, source_y, source_x + 512 * Q, source_y + 512 * Q);
    let mut roundings = 0;
    let clipped = intersection(polygon, &source_rect, &mut roundings, work)?;
    if clipped.len() < 3 {
        return Ok(());
    }
    let mapped: Polygon = clipped
        .into_iter()
        .map(|p| {
            Ok((
                to_output(p.0, source_x, bounds[0], bounds[1] - bounds[0])?,
                to_output(p.1, source_y, bounds[2], bounds[3] - bounds[2])?,
            ))
        })
        .collect::<Result<_, RenderError>>()?;
    let minx = mapped
        .iter()
        .map(|p| p.0)
        .min()
        .ok_or_else(|| bad("empty mapped channel"))?;
    let maxx = mapped
        .iter()
        .map(|p| p.0)
        .max()
        .ok_or_else(|| bad("empty mapped channel"))?;
    let miny = mapped
        .iter()
        .map(|p| p.1)
        .min()
        .ok_or_else(|| bad("empty mapped channel"))?;
    let maxy = mapped
        .iter()
        .map(|p| p.1)
        .max()
        .ok_or_else(|| bad("empty mapped channel"))?;
    if maxx <= i64::from(bounds[0]) * Q
        || maxy <= i64::from(bounds[2]) * Q
        || minx >= i64::from(bounds[1]) * Q
        || miny >= i64::from(bounds[3]) * Q
    {
        return Ok(());
    }
    let bx0 =
        u32::try_from((minx.div_euclid(Q)).clamp(i64::from(bounds[0]), i64::from(bounds[1] - 1)))
            .map_err(|_| bad("channel x bound"))?;
    let bx1 =
        u32::try_from((maxx.div_euclid(Q)).clamp(i64::from(bounds[0]), i64::from(bounds[1] - 1)))
            .map_err(|_| bad("channel x bound"))?;
    let by0 =
        u32::try_from((miny.div_euclid(Q)).clamp(i64::from(bounds[2]), i64::from(bounds[3] - 1)))
            .map_err(|_| bad("channel y bound"))?;
    let by1 =
        u32::try_from((maxy.div_euclid(Q)).clamp(i64::from(bounds[2]), i64::from(bounds[3] - 1)))
            .map_err(|_| bad("channel y bound"))?;
    if out.len() == MAX_SHAPES {
        return Err(bad("Atlas overview channel shape budget exhausted"));
    }
    *vertices = vertices
        .checked_add(mapped.len())
        .ok_or_else(|| bad("Atlas channel vertex count overflow"))?;
    if *vertices > MAX_VERTICES {
        return Err(bad("Atlas overview exceeds 4194304 channel vertices"));
    }
    out.push(Shape {
        polygon: mapped,
        bounds: [bx0, bx1, by0, by1],
        discharge,
    });
    Ok(())
}
/// Sine of `t / period` turns in Q12 (Bhaskara), for thread weaving.
fn weave_q12(t: i64, period: i64) -> i64 {
    let p = t.rem_euclid(period.max(1)) * 4_096 / period.max(1);
    let half = |u: i64| 16 * u * (4_096 - u) / (5 * 4_096 - 4 * u * (4_096 - u) / 4_096);
    if p < 2_048 {
        half(p * 2)
    } else {
        -half((p - 2_048) * 2)
    }
}

/// Two thin threads weaving across the belt of every braided edge
/// (logic/04 §atlas-formed braids). A node's threads sit on the normal of
/// its own downstream edge, so consecutive pieces join; their offsets are
/// sinusoids of absolute position that cross the main channel regularly.
fn braid_threads(
    context: &OverviewChannelContext,
    node: &dyn Fn(GlobalCell) -> Point,
    at: AreaCoord,
    bounds: [u32; 4],
    shapes: &mut Vec<Shape>,
    work: &mut WorkBudget,
    vertices: &mut usize,
) -> Result<(), RenderError> {
    let edges = context.edges(at)?;
    let out: std::collections::BTreeMap<GlobalCell, GlobalCell> = edges
        .iter()
        .filter(|e| e.from != e.to && context.braided.contains_key(&e.from))
        .map(|e| (e.from, e.to))
        .collect();
    let normal = |n: GlobalCell| -> Option<(i64, i64)> {
        let to = *out.get(&n)?;
        let (a, b) = (node(n), node(to));
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = i64::try_from(
            (i128::from(dx) * i128::from(dx) + i128::from(dy) * i128::from(dy)).isqrt(),
        )
        .ok()?;
        (len > 0).then(|| (-dy * 4_096 / len, dx * 4_096 / len))
    };
    let offset = |n: GlobalCell, k: i64, fallback: (i64, i64)| -> Point {
        let belt = i64::from(context.braided.get(&n).copied().unwrap_or(0));
        let amp = belt * Q / 1000 * 9 / 20;
        let t = i64::from(n.x) * (37 + 11 * k) + i64::from(n.y) * (61 - 13 * k);
        let s = weave_q12(t + 997 * k, 700 + 300 * k);
        let nrm = normal(n).unwrap_or(fallback);
        let p = node(n);
        (
            p.0 + nrm.0 * amp / 4_096 * s / 4_096,
            p.1 + nrm.1 * amp / 4_096 * s / 4_096,
        )
    };
    for e in edges {
        let Some(nrm) = (e.from != e.to).then(|| normal(e.from)).flatten() else {
            continue;
        };
        let w = display_width(e.from_width_dm, super::ChannelStyle::Formed)? * Q / 1000;
        let w = w.max(1);
        for k in 0..2 {
            let (a, b) = (offset(e.from, k, nrm), offset(e.to, k, nrm));
            if a == b {
                continue;
            }
            let (poly, _, _) = strip_free(a, b, w, w)?;
            append(
                shapes,
                &poly,
                e.discharge.raw() / 3,
                at,
                bounds,
                work,
                vertices,
            )?;
        }
    }
    Ok(())
}

/// Prepared one-area coverage over exact global output pixel bounds.
pub(super) struct ChannelTile {
    style: super::ChannelStyle,
    /// Recipe-6 formed rivers (curved, tapered, v0.2 colours); false keeps
    /// the recipe-5 drawing (logic/04 §atlas-formed recipes).
    v6: bool,
    shapes: Vec<Shape>,
    bounds: [u32; 4],
    starts: Vec<usize>,
    ends: Vec<usize>,
    next_start: usize,
    next_end: usize,
    active: BTreeSet<usize>,
    pixels: Vec<Vec<usize>>,
    pixel_capacity: usize,
    last_y: Option<u32>,
}
impl ChannelTile {
    pub(super) fn new(
        context: &OverviewChannelContext,
        at: AreaCoord,
        bounds: [u32; 4],
        canvas: [u32; 2],
        style: super::ChannelStyle,
    ) -> Result<Self, RenderError> {
        Self::new_with_terrain(context, at, bounds, canvas, style, None)
    }

    /// With recipe-5 terrain, channel vertices snap to the valley floor
    /// exactly as area renders do (logic/04 §atlas-formed rivers).
    pub(super) fn new_with_terrain(
        context: &OverviewChannelContext,
        at: AreaCoord,
        bounds: [u32; 4],
        canvas: [u32; 2],
        style: super::ChannelStyle,
        terrain: Option<&crate::AtlasTerrain>,
    ) -> Result<Self, RenderError> {
        let formed = terrain.filter(|t| t.is_formed());
        let v6 = formed.is_none_or(crate::AtlasTerrain::is_formed_v6);
        // Snapped node positions, computed once per node.
        let mut snapped: BTreeMap<GlobalCell, Point> = BTreeMap::new();
        if let Some(t) = formed {
            for e in context.edges(at)? {
                for cell in [e.from, e.to] {
                    snapped.entry(cell).or_insert_with(|| {
                        let (cx, cy) = center(cell);
                        let (ox, oy) = t.channel_offset_um(cell);
                        (cx + ox * Q / 100_000_000, cy + oy * Q / 100_000_000)
                    });
                }
            }
        }
        let network = formed.filter(|_| v6).map(|_| {
            Network::new(
                context
                    .edges(at)
                    .into_iter()
                    .flatten()
                    .map(|e| (e.from, e.to, e.discharge.raw())),
            )
        });
        if let Some(net) = &network {
            snapped = net.relax(&snapped);
        }
        let segments_per_edge = curve_segments(i64::from(bounds[1] - bounds[0]) / 512);
        let node = |cell: GlobalCell| -> Point {
            snapped.get(&cell).copied().unwrap_or_else(|| center(cell))
        };

        let build = if formed.is_some() { strip_free } else { strip };
        let aw =
            u32::try_from(context.areas_wide).map_err(|_| RenderError::ExactOverviewDimensions)?;
        let ah =
            u32::try_from(context.areas_high).map_err(|_| RenderError::ExactOverviewDimensions)?;
        let ax = u32::try_from(at.x).map_err(|_| RenderError::ExactOverviewDimensions)?;
        let ay = u32::try_from(at.y).map_err(|_| RenderError::ExactOverviewDimensions)?;
        if bounds
            != [
                ax * canvas[0] / aw,
                (ax + 1) * canvas[0] / aw,
                ay * canvas[1] / ah,
                (ay + 1) * canvas[1] / ah,
            ]
        {
            return Err(context_error(
                "Atlas channel context rectangle differs from raster partition",
            ));
        }
        if bounds[0] >= bounds[1] || bounds[2] >= bounds[3] {
            return Err(RenderError::ExactOverviewDimensions);
        }
        let mut shapes = Vec::new();
        let mut caps = Vec::new();
        let mut work = WorkBudget::new(MAX_WORK);
        let mut vertices = 0usize;
        for &edge in context.edges(at)? {
            if (edge.from == edge.to && edge.from_width_dm != edge.to_width_dm)
                || edge.from.x.abs_diff(edge.to.x) > 1
                || edge.from.y.abs_diff(edge.to.y) > 1
                || edge.discharge.raw() < 40
            {
                return Err(bad("invalid saved channel endpoints or discharge"));
            }
            let mut wa = display_width(edge.from_width_dm, style)? * Q / 1000;
            let mut wb = display_width(edge.to_width_dm, style)? * Q / 1000;
            if matches!(style, super::ChannelStyle::Formed) {
                // Minimum on-screen width by discharge (goals 7, 28): one
                // output pixel is 512 / span saved cells.
                let span = i64::from(bounds[1] - bounds[0]).max(1);
                let min_px_q8 = if v6 {
                    min_width_px_q8(edge.discharge.raw())
                } else {
                    min_width_px_q8_v5(edge.discharge.raw())
                };
                let min_w = min_px_q8 * 512 * Q / (span * 256);
                wa = wa.max(min_w);
                wb = wb.max(min_w);
            }
            if edge.from == edge.to {
                append(
                    &mut shapes,
                    &terminal_footprint(node(edge.from), wa)?,
                    edge.discharge.raw(),
                    at,
                    bounds,
                    &mut work,
                    &mut vertices,
                )?;
                continue;
            }
            let (pieces, a, b) = match &network {
                // Recipe 5: smooth centreline, tapered at sources (goal 28).
                Some(net) => {
                    if net.is_source(edge.from) {
                        wa = source_width(wa);
                    }
                    let (before, after) = net.neighbours(edge.from, edge.to);
                    curved_strip(
                        before.map(node),
                        node(edge.from),
                        node(edge.to),
                        after.map(node),
                        (wa, wb),
                        segments_per_edge,
                    )?
                }
                None => {
                    let (poly, a, b) = build(node(edge.from), node(edge.to), wa, wb)?;
                    (vec![poly], a, b)
                }
            };
            for poly in &pieces {
                append(
                    &mut shapes,
                    poly,
                    edge.discharge.raw(),
                    at,
                    bounds,
                    &mut work,
                    &mut vertices,
                )?;
            }
            caps.push(Cap {
                node: edge.from,
                points: a,
                discharge: edge.discharge.raw(),
            });
            caps.push(Cap {
                node: edge.to,
                points: b,
                discharge: edge.discharge.raw(),
            });
        }
        if matches!(style, super::ChannelStyle::Formed) && !context.braided.is_empty() {
            braid_threads(
                context,
                &node,
                at,
                bounds,
                &mut shapes,
                &mut work,
                &mut vertices,
            )?;
        }
        caps.sort_unstable_by_key(|c| c.node);
        let mut start = 0;
        while start < caps.len() {
            let mut end = start + 1;
            while end < caps.len() && caps[end].node == caps[start].node {
                end += 1;
            }
            if end - start > 16 {
                return Err(bad("Atlas channel node exceeds directed D8 degree"));
            }
            let points = caps[start..end].iter().flat_map(|c| c.points).collect();
            let q = caps[start..end]
                .iter()
                .map(|c| c.discharge)
                .max()
                .ok_or_else(|| bad("empty Atlas channel join"))?;
            append(
                &mut shapes,
                &hull(points)?,
                q,
                at,
                bounds,
                &mut work,
                &mut vertices,
            )?;
            start = end;
        }
        let mut starts: Vec<usize> = (0..shapes.len()).collect();
        let mut ends = starts.clone();
        starts.sort_unstable_by_key(|&i| (shapes[i].bounds[2], i));
        ends.sort_unstable_by_key(|&i| (shapes[i].bounds[3], i));
        let pixel_width = usize::try_from(bounds[1] - bounds[0])
            .map_err(|_| RenderError::ExactOverviewDimensions)?;
        Ok(Self {
            style,
            v6,
            shapes,
            bounds,
            starts,
            ends,
            next_start: 0,
            next_end: 0,
            active: BTreeSet::new(),
            pixels: vec![Vec::new(); pixel_width],
            pixel_capacity: 0,
            last_y: None,
        })
    }
    pub(super) fn blend_row(
        &mut self,
        y: u32,
        rgb: &mut [u8],
        features: &[Feature],
    ) -> Result<(), RenderError> {
        if y < self.bounds[2]
            || y >= self.bounds[3]
            || features.len() != (self.bounds[1] - self.bounds[0]) as usize
            || rgb.len() != features.len() * 3
        {
            return Err(RenderError::ExactOverviewDimensions);
        }
        if self.last_y.is_some_and(|previous| y <= previous) {
            return Err(context_error("Atlas channel rows must advance"));
        }
        self.last_y = Some(y);
        let mut work = WorkBudget::new(MAX_WORK);
        while let Some(&i) = self.ends.get(self.next_end) {
            if self.shapes[i].bounds[3] >= y {
                break;
            }
            self.active.remove(&i);
            self.next_end += 1;
        }
        while let Some(&i) = self.starts.get(self.next_start) {
            if self.shapes[i].bounds[2] > y {
                break;
            }
            if self.shapes[i].bounds[3] >= y {
                self.active.insert(i);
            }
            self.next_start += 1;
        }
        work.charge(self.active.len() as u64)?;
        for p in &mut self.pixels {
            p.clear();
        }
        for &i in &self.active {
            let bounds = self.shapes[i].bounds;
            for x in bounds[0]..=bounds[1] {
                let at = (x - self.bounds[0]) as usize;
                let p = &mut self.pixels[at];
                if p.len() == MAX_PIXEL_CANDIDATES {
                    return Err(bad("Atlas pixel exceeds 4096 candidate channel polygons"));
                }
                work.charge(1)?;
                let before = p.capacity();
                p.push(i);
                self.pixel_capacity += p.capacity() - before;
                if self.pixel_capacity > MAX_PIXEL_CAPACITY {
                    return Err(bad(
                        "Atlas channel raster exceeds pixel-reference memory budget",
                    ));
                }
            }
        }
        for (x, ids) in self.pixels.iter_mut().enumerate() {
            ids.sort_unstable_by_key(|&i| (std::cmp::Reverse(area2(&self.shapes[i].polygon)), i));
            if ids.is_empty() || features[x] != Feature::AtlasLand {
                continue;
            }
            let global_x = self.bounds[0]
                .checked_add(u32::try_from(x).map_err(|_| RenderError::ExactOverviewDimensions)?)
                .ok_or(RenderError::ExactOverviewDimensions)?;
            let c = coverage_overview(
                ids.iter()
                    .map(|&i| (&self.shapes[i].polygon, self.shapes[i].discharge)),
                global_x,
                y,
                &mut work,
            )?;
            work.charge(c.roundings + c.pieces as u64)?;
            if c.alpha == 0 {
                continue;
            }
            let band = super::river_band(c.maximum_discharge).unwrap_or(super::RiverBand::Light);
            let water = match self.style {
                super::ChannelStyle::Formed if self.v6 => formed_river_colour(band),
                super::ChannelStyle::Formed => formed_river_colour_v5(band),
                _ => atlas_river_colour(band),
            };
            let a = match self.style {
                super::ChannelStyle::Legacy | super::ChannelStyle::Formed => u32::from(c.alpha),
                super::ChannelStyle::Fine => u32::from(c.alpha) * 13 / 16,
            };
            for k in 0..3 {
                rgb[x * 3 + k] = u8::try_from(
                    (u32::from(rgb[x * 3 + k]) * (65535 - a) + u32::from(water[k]) * a + 32767)
                        / 65535,
                )
                .map_err(|_| bad("Atlas channel colour overflow"))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overview::ChannelStyle;
    use crate::{AtlasHalo, AtlasNeighbor, AtlasTerrain, OverviewRaster};
    use arda_core::{Cell, DischargeMilli, HeightMm, TerrainKind};

    fn edge(a: (u32, u32), b: (u32, u32), width: u32) -> ChannelEdge {
        ChannelEdge {
            from: GlobalCell { x: a.0, y: a.1 },
            to: GlobalCell { x: b.0, y: b.1 },
            from_width_dm: width,
            to_width_dm: width,
            discharge: DischargeMilli::new(40_000),
        }
    }
    fn objects() -> AreaObjects {
        let mut o = AreaObjects::empty();
        o.channel_edges = vec![
            edge((511, 510), (511, 511), 500),
            edge((511, 511), (512, 512), 500),
            edge((512, 511), (512, 512), 500),
            edge((512, 512), (513, 513), 500),
        ];
        o
    }
    #[test]
    fn braided_edges_gain_two_weaving_threads() {
        let at = AreaCoord::new(0, 0);
        let mut o = AreaObjects::empty();
        o.channel_edges = (100..130)
            .map(|x| edge((x, 200), (x + 1, 200), 200))
            .collect();
        let build = |braided: bool| {
            let mut c = OverviewChannelContext::new(at, 1, 1).unwrap();
            c.add_area(at, &o).unwrap();
            if braided {
                c.add_braided((100..120).map(|x| (GlobalCell { x, y: 200 }, 3_000)))
                    .unwrap();
            }
            let c = c.finish().unwrap();
            ChannelTile::new(&c, at, [0, 512, 0, 512], [512, 512], ChannelStyle::Formed).unwrap()
        };
        let (plain, braided) = (build(false), build(true));
        // 20 braided edges, two threads each.
        assert_eq!(braided.shapes.len(), plain.shapes.len() + 40);
        // Threads leave the channel axis (y = 200.5 cells) by up to 45% of
        // the 300 m belt, i.e. 1.35 cells.
        let spread = braided.shapes[plain.shapes.len()..]
            .iter()
            .flat_map(|s| s.polygon.iter().map(|p| (p.1 - (200 * Q + Q / 2)).abs()))
            .max()
            .unwrap();
        assert!(spread > Q / 2 && spread < 2 * Q, "{spread}");
        // Without stored forms nothing changes.
        let c = OverviewChannelContext::new(at, 1, 1).unwrap();
        assert!(c.braided.is_empty());
    }

    fn context(at: AreaCoord, o: &AreaObjects) -> OverviewChannelContext {
        let mut c = OverviewChannelContext::new(at, 2, 2).unwrap();
        for y in 0..2 {
            for x in 0..2 {
                c.add_area(AreaCoord::new(x, y), o).unwrap();
            }
        }
        c.finish().unwrap()
    }
    fn area(kind: TerrainKind) -> arda_core::AreaCells {
        arda_core::AreaCells::flat(Cell {
            height: HeightMm::new(200_000),
            terrain: kind,
            ..Cell::default()
        })
    }
    fn terrain(cells: &arda_core::AreaCells) -> AtlasTerrain {
        let mut halo = AtlasHalo::new();
        for d in [
            AtlasNeighbor::North,
            AtlasNeighbor::NorthEast,
            AtlasNeighbor::East,
            AtlasNeighbor::SouthEast,
            AtlasNeighbor::South,
            AtlasNeighbor::SouthWest,
            AtlasNeighbor::West,
            AtlasNeighbor::NorthWest,
        ] {
            halo.mark_world_edge(d).unwrap();
        }
        AtlasTerrain::new(cells, halo).unwrap()
    }
    fn decode(png: &[u8]) -> Vec<u8> {
        let mut reader = png::Decoder::new(png).read_info().unwrap();
        let mut pixels = vec![0; reader.output_buffer_size()];
        reader.next_frame(&mut pixels).unwrap();
        pixels
    }
    #[test]
    fn rectangular_both_axis_seams_confluence_and_band_stream_match_buffered() {
        let cells = area(TerrainKind::Land);
        let o = objects();
        let mut buffered = OverviewRaster::new_exact(2, 2, 513, 517).unwrap();
        for y in (0..2).rev() {
            for x in (0..2).rev() {
                let at = AreaCoord::new(x, y);
                buffered
                    .push_atlas_with_channels(at, &cells, &terrain(&cells), &context(at, &o))
                    .unwrap();
            }
        }
        let expected = decode(&buffered.finish().unwrap());
        let mut streamed = Vec::new();
        crate::write_atlas_overview_png_with_channels(2, 2, 513, 517, &mut streamed, |at| {
            Ok::<_, RenderError>((cells.clone(), terrain(&cells), context(at, &o)))
        })
        .unwrap();
        assert_eq!(decode(&streamed), expected);
        // The connected geometry changes pixels in both unequal-width areas around the crossing.
        let mut plain = OverviewRaster::new_exact(2, 2, 513, 517).unwrap();
        for y in 0..2 {
            for x in 0..2 {
                let at = AreaCoord::new(x, y);
                let empty = AreaObjects::empty();
                plain
                    .push_atlas_with_channels(at, &cells, &terrain(&cells), &context(at, &empty))
                    .unwrap();
            }
        }
        let base = decode(&plain.finish().unwrap());
        assert_ne!(expected, base);
        assert!((253..260).any(|x| (251..266)
            .any(|y| expected[(y * 513 + x) * 3..(y * 513 + x) * 3 + 3]
                != base[(y * 513 + x) * 3..(y * 513 + x) * 3 + 3])));

        let mut fine_stream = Vec::new();
        crate::write_atlas_overview_png_with_channels_recipe4(
            2,
            2,
            513,
            517,
            &mut fine_stream,
            |at| Ok::<_, RenderError>((cells.clone(), terrain(&cells), context(at, &o))),
        )
        .unwrap();
        let fine = decode(&fine_stream);
        let changed = |image: &[u8]| {
            image
                .as_chunks::<3>()
                .0
                .iter()
                .zip(base.as_chunks::<3>().0.iter())
                .filter(|(pixel, ground)| pixel != ground)
                .count()
        };
        assert!(changed(&fine) > 0);
        assert!(changed(&fine) < changed(&expected));
        let colour_shift = |image: &[u8]| {
            image
                .iter()
                .zip(&base)
                .map(|(colour, ground)| colour.abs_diff(*ground) as u64)
                .sum::<u64>()
        };
        assert!(colour_shift(&fine) < colour_shift(&expected));
    }
    #[test]
    fn sea_and_lake_own_the_pixel_even_under_a_saved_strip() {
        let o = objects();
        for kind in [TerrainKind::Sea, TerrainKind::Lake] {
            let cells = area(kind);
            let mut with = OverviewRaster::new_exact(2, 2, 513, 517).unwrap();
            let mut without = OverviewRaster::new_exact(2, 2, 513, 517).unwrap();
            for y in 0..2 {
                for x in 0..2 {
                    let at = AreaCoord::new(x, y);
                    with.push_atlas_with_channels(at, &cells, &terrain(&cells), &context(at, &o))
                        .unwrap();
                    without
                        .push_atlas_with_channels(
                            at,
                            &cells,
                            &terrain(&cells),
                            &context(at, &AreaObjects::empty()),
                        )
                        .unwrap();
                }
            }
            assert_eq!(
                decode(&with.finish().unwrap()),
                decode(&without.finish().unwrap())
            );
        }
    }
    #[test]
    fn context_edge_resource_cap_refuses_instead_of_omitting() {
        let mut c = OverviewChannelContext::new(AreaCoord::new(0, 0), 1, 1).unwrap();
        let repeated = edge((1, 1), (2, 1), 100);
        for _ in 0..MAX_CONTEXT_EDGES {
            c.push(repeated).unwrap();
        }
        assert!(matches!(
            c.push(repeated),
            Err(RenderError::ChannelGeometry { .. })
        ));
    }

    #[test]
    fn cap_extremes_missing_neighbor_and_wrong_rectangle_are_typed() {
        assert_eq!(
            display_width(u32::MAX, ChannelStyle::Legacy).unwrap(),
            10_000
        );
        assert_eq!(display_width(904, ChannelStyle::Legacy).unwrap(), 7_232);
        assert_eq!(display_width(u32::MAX, ChannelStyle::Fine).unwrap(), 4_000);
        assert_eq!(display_width(904, ChannelStyle::Fine).unwrap(), 2_712);
        assert!(display_width(0, ChannelStyle::Legacy).is_err());
        assert!(display_width(0, ChannelStyle::Fine).is_err());
        let mut c = OverviewChannelContext::new(AreaCoord::new(0, 0), 2, 1).unwrap();
        c.add_area(AreaCoord::new(0, 0), &AreaObjects::empty())
            .unwrap();
        assert!(matches!(c.finish(), Err(RenderError::AtlasContext { .. })));
        let mut c = OverviewChannelContext::new(AreaCoord::new(0, 0), 1, 1).unwrap();
        c.add_area(AreaCoord::new(0, 0), &AreaObjects::empty())
            .unwrap();
        let c = c.finish().unwrap();
        assert!(matches!(
            ChannelTile::new(
                &c,
                AreaCoord::new(0, 0),
                [0, 2, 0, 1],
                [1, 1],
                ChannelStyle::Legacy,
            ),
            Err(RenderError::AtlasContext { .. })
        ));
        let mut c = OverviewChannelContext::new(AreaCoord::new(0, 0), 1, 1).unwrap();
        assert!(matches!(
            c.add_area(AreaCoord::new(i32::MIN, 0), &AreaObjects::empty()),
            Err(RenderError::AtlasContext { .. })
        ));
    }
}
