//! One area's prepared channel coverage, blended row by row into the
//! streamed Atlas overview.

use super::shapes::{append, braid_threads, center, display_width, Cap, Shape};
use super::{
    bad, context_error, formed_river_colour, formed_river_colour_v5, min_width_px_q8_v5,
    OverviewChannelContext, MAX_PIXEL_CANDIDATES, MAX_PIXEL_CAPACITY, MAX_WORK,
};
use crate::channel_curve::{
    curved_strip, min_width_px_q8, segments as curve_segments, source_width, Network,
};
use crate::channel_geometry::{
    area2, coverage_overview, hull, strip, strip_free, terminal_footprint, Point, WorkBudget, Q,
};
use crate::overview::{atlas_river_colour, ChannelStyle, Feature, RiverBand};
use crate::RenderError;
use arda_core::{AreaCoord, GlobalCell};
use std::collections::{BTreeMap, BTreeSet};

/// Prepared one-area coverage over exact global output pixel bounds.
pub(in crate::overview) struct ChannelTile {
    style: ChannelStyle,
    /// Recipe-6 formed rivers (curved, tapered, v0.2 colours); false keeps
    /// the recipe-5 drawing (logic/04 §atlas-formed recipes).
    v6: bool,
    pub(super) shapes: Vec<Shape>,
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
    pub(in crate::overview) fn new(
        context: &OverviewChannelContext,
        at: AreaCoord,
        bounds: [u32; 4],
        canvas: [u32; 2],
        style: ChannelStyle,
    ) -> Result<Self, RenderError> {
        Self::new_with_terrain(context, at, bounds, canvas, style, None)
    }

    /// With recipe-5 terrain, channel vertices snap to the valley floor
    /// exactly as area renders do (logic/04 §atlas-formed rivers).
    pub(in crate::overview) fn new_with_terrain(
        context: &OverviewChannelContext,
        at: AreaCoord,
        bounds: [u32; 4],
        canvas: [u32; 2],
        style: ChannelStyle,
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
            if matches!(style, ChannelStyle::Formed) {
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
        if matches!(style, ChannelStyle::Formed) && !context.braided.is_empty() {
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
    pub(in crate::overview) fn blend_row(
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
            let band = crate::overview::river_band(c.maximum_discharge).unwrap_or(RiverBand::Light);
            let water = match self.style {
                ChannelStyle::Formed if self.v6 => formed_river_colour(band),
                ChannelStyle::Formed => formed_river_colour_v5(band),
                _ => atlas_river_colour(band),
            };
            let a = match self.style {
                ChannelStyle::Legacy | ChannelStyle::Formed => u32::from(c.alpha),
                ChannelStyle::Fine => u32::from(c.alpha) * 13 / 16,
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
