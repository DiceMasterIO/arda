//! Physical channel raster and scale-dependent preview marks.

use crate::atlas::axis_kernel;
use crate::carto::{lake_colour, land_colour, sea_colour, LAKE_FILL};
use crate::channel_curve::{
    curved_strip, min_width_px_q8, segments as curve_segments, source_width, Network,
};
use crate::channel_geometry::{
    coverage, hull, strip, strip_free, terminal_footprint, Point, Polygon, WorkBudget, Q,
};
use crate::{AtlasTerrain, GlobalCell, ImageQuality, RenderError};
use arda_core::{AreaCells, CellCoord, Lake, TerrainKind};
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

/// Resolution of an area image; every scale uses the same 100 m terrain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AreaImageScale {
    /// 512 square pixels with a faint mark for subpixel streams.
    #[default]
    Preview,
    /// 4096 square pixels, showing physical channel coverage only.
    Detail,
    /// An exact image side, showing physical channel coverage only.
    Custom(ImageQuality),
}

impl AreaImageScale {
    /// Pixels along each side of the image.
    #[must_use]
    pub const fn side(self) -> u32 {
        match self {
            Self::Preview => 512,
            Self::Detail => 4096,
            Self::Custom(quality) => quality.pixels(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ChannelInput {
    pub from: GlobalCell,
    pub to: GlobalCell,
    pub from_width_dm: u32,
    pub to_width_dm: u32,
    pub discharge: u64,
}

struct Shape {
    polygon: Polygon,
    bounds: [usize; 4],
    discharge: u64,
    marker: bool,
}

struct Cap {
    node: GlobalCell,
    points: [Point; 2],
    discharge: u64,
    marker: bool,
}

const MAX_EDGES: usize = 262_144;
const MAX_SHAPES: usize = 1_048_576;
const MAX_VERTICES: usize = 4_194_304;
const MAX_PIXEL_CAPACITY: usize = 1_048_576;
const MAX_PIXEL_CANDIDATES: usize = 4096;
const MAX_COVERAGE_WORK: u64 = 250_000_000;

fn invalid(reason: &'static str) -> RenderError {
    RenderError::ChannelGeometry { reason }
}

fn shape(polygon: Polygon, discharge: u64, marker: bool, side: usize) -> Option<Shape> {
    if polygon.len() < 3 {
        return None;
    }
    let minx = polygon.iter().map(|p| p.0).min()?;
    let maxx = polygon.iter().map(|p| p.0).max()?;
    let miny = polygon.iter().map(|p| p.1).min()?;
    let maxy = polygon.iter().map(|p| p.1).max()?;
    let side_i64 = i64::try_from(side).ok()?;
    let end = side_i64.checked_mul(Q)?;
    if maxx <= 0 || maxy <= 0 || minx >= end || miny >= end {
        return None;
    }
    let pixel = |v: i64| usize::try_from(v.div_euclid(Q).clamp(0, side_i64 - 1)).ok();
    Some(Shape {
        polygon,
        bounds: [pixel(minx)?, pixel(maxx)?, pixel(miny)?, pixel(maxy)?],
        discharge,
        marker,
    })
}

fn shapes_results(
    inputs: impl IntoIterator<Item = Result<ChannelInput, RenderError>>,
    origin: GlobalCell,
    scale: AreaImageScale,
    offset_um: Option<&dyn Fn(GlobalCell) -> (i64, i64)>,
) -> Result<Vec<Shape>, RenderError> {
    let free = offset_um.is_some();
    let side = scale.side() as usize;
    // Q is divisible by 512, so arbitrary integer image sizes retain exact D8
    // steps. With u32 endpoints and widths these products stay below 2^60.
    let ppc_q = i64::from(scale.side()) * (Q / 512);
    let mut edges = Vec::new();
    for edge in inputs {
        let edge = edge?;
        if edges.len() == MAX_EDGES {
            return Err(invalid("area channel halo exceeds 262144 edges"));
        }
        if (edge.from == edge.to && edge.from_width_dm != edge.to_width_dm)
            || edge.from.x.abs_diff(edge.to.x) > 1
            || edge.from.y.abs_diff(edge.to.y) > 1
            || edge.from_width_dm == 0
            || edge.to_width_dm == 0
            || edge.discharge < 40
        {
            return Err(invalid(
                "saved channel has invalid endpoints, width or discharge",
            ));
        }
        edges.push(edge);
    }
    edges.sort_unstable();
    edges.dedup();
    if edges
        .windows(2)
        .any(|p| p[0].from == p[1].from && p[0].to == p[1].to)
    {
        return Err(invalid(
            "duplicate channel endpoints disagree about solved fields",
        ));
    }
    let raw_point = |node: GlobalCell| -> Point {
        let (ox, oy) = offset_um.map_or((0, 0), |f| f(node));
        (
            ((i64::from(node.x) - i64::from(origin.x)) * 2 + 1) * ppc_q / 2
                + ox * ppc_q / 100_000_000,
            ((i64::from(node.y) - i64::from(origin.y)) * 2 + 1) * ppc_q / 2
                + oy * ppc_q / 100_000_000,
        )
    };
    // Recipe 5: smooth, relaxed centrelines tapered at sources, exactly as
    // the overview draws them (logic/04 §atlas-formed rivers).
    let network = free.then(|| Network::new(edges.iter().map(|e| (e.from, e.to, e.discharge))));
    let mut snapped: BTreeMap<GlobalCell, Point> = BTreeMap::new();
    if let Some(net) = &network {
        for e in &edges {
            for cell in [e.from, e.to] {
                snapped.entry(cell).or_insert_with(|| raw_point(cell));
            }
        }
        snapped = net.relax(&snapped);
    }
    let point = |node: GlobalCell| -> Point {
        snapped
            .get(&node)
            .copied()
            .unwrap_or_else(|| raw_point(node))
    };
    let segments_per_edge = curve_segments(i64::from(scale.side()) / 512);
    let mut out = Vec::new();
    let mut caps = Vec::new();
    let mut vertices = 0;
    let mut append = |p, q, marker| -> Result<(), RenderError> {
        if let Some(s) = shape(p, q, marker, side) {
            vertices += s.polygon.len();
            if vertices > MAX_VERTICES || out.len() == MAX_SHAPES {
                return Err(invalid(
                    "area channel geometry exceeds shape or vertex budget",
                ));
            }
            out.push(s);
        }
        Ok(())
    };
    for edge in edges {
        let mut wa = i64::from(edge.from_width_dm) * ppc_q / 1000;
        let mut wb = i64::from(edge.to_width_dm) * ppc_q / 1000;
        if free {
            // Recipe 5: minimum on-screen width by discharge, as in the
            // overview (logic/04 §atlas-formed scale). Q is one pixel.
            let min_w = min_width_px_q8(edge.discharge) * Q / 256;
            wa = wa.max(min_w);
            wb = wb.max(min_w);
            if network.as_ref().is_some_and(|n| n.is_source(edge.from)) {
                wa = source_width(wa);
            }
        }
        let marker = scale == AreaImageScale::Preview && wa < Q && wb < Q;
        for is_marker in [false, true] {
            if is_marker && !marker {
                continue;
            }
            if edge.from == edge.to {
                append(
                    terminal_footprint(point(edge.from), if is_marker { Q } else { wa })?,
                    edge.discharge,
                    is_marker,
                )?;
                continue;
            }
            let (wa, wb) = if is_marker { (Q, Q) } else { (wa, wb) };
            let (pieces, a, b) = match &network {
                Some(net) => {
                    let (before, after) = net.neighbours(edge.from, edge.to);
                    curved_strip(
                        before.map(point),
                        point(edge.from),
                        point(edge.to),
                        after.map(point),
                        (wa, wb),
                        segments_per_edge,
                    )?
                }
                None => {
                    let build = if free { strip_free } else { strip };
                    let (p, a, b) = build(point(edge.from), point(edge.to), wa, wb)?;
                    (vec![p], a, b)
                }
            };
            for p in pieces {
                append(p, edge.discharge, is_marker)?;
            }
            caps.push(Cap {
                node: edge.from,
                points: a,
                discharge: edge.discharge,
                marker: is_marker,
            });
            caps.push(Cap {
                node: edge.to,
                points: b,
                discharge: edge.discharge,
                marker: is_marker,
            });
        }
    }
    caps.sort_unstable_by_key(|c| (c.marker, c.node));
    let mut start = 0;
    while start < caps.len() {
        let mut end = start + 1;
        while end < caps.len()
            && (caps[end].marker, caps[end].node) == (caps[start].marker, caps[start].node)
        {
            end += 1;
        }
        if end - start > 16 {
            return Err(invalid("channel node exceeds the directed D8 degree"));
        }
        let group = &caps[start..end];
        let points = group.iter().flat_map(|c| c.points).collect();
        let q = group
            .iter()
            .map(|c| c.discharge)
            .max()
            .ok_or_else(|| invalid("empty channel join"))?;
        append(hull(points)?, q, caps[start].marker)?;
        start = end;
    }
    Ok(out)
}

#[cfg(test)]
fn shapes(
    inputs: impl IntoIterator<Item = ChannelInput>,
    origin: GlobalCell,
    scale: AreaImageScale,
) -> Result<Vec<Shape>, RenderError> {
    shapes_results(inputs.into_iter().map(Ok), origin, scale, None)
}

fn channel_colour(q: u64) -> [u8; 3] {
    match q {
        0..=3999 => [45, 160, 230],
        4000..=19_999 => [32, 142, 220],
        20_000..=79_999 => [24, 112, 198],
        _ => [16, 72, 156],
    }
}

fn visual_alpha(physical: u16, marker: u16) -> u32 {
    let physical = u32::from(physical);
    let faint = (u32::from(marker) * 46 + 127) / 255;
    physical + (65_535 - physical) * faint / 65_535
}

#[cfg(test)]
pub(crate) fn raster(
    cells: &AreaCells,
    inputs: impl IntoIterator<Item = ChannelInput>,
    origin: GlobalCell,
    scale: AreaImageScale,
) -> Result<Vec<u8>, RenderError> {
    raster_limited(cells, inputs, origin, scale, MAX_COVERAGE_WORK)
}

#[cfg(test)]
fn raster_limited(
    cells: &AreaCells,
    inputs: impl IntoIterator<Item = ChannelInput>,
    origin: GlobalCell,
    scale: AreaImageScale,
    work_limit: u64,
) -> Result<Vec<u8>, RenderError> {
    raster_limited_results(
        cells,
        inputs.into_iter().map(Ok),
        origin,
        scale,
        work_limit,
        &[],
    )
}

#[cfg(test)]
pub(crate) fn raster_results(
    cells: &AreaCells,
    inputs: impl IntoIterator<Item = Result<ChannelInput, RenderError>>,
    origin: GlobalCell,
    scale: AreaImageScale,
    lakes: &[Lake],
) -> Result<Vec<u8>, RenderError> {
    raster_limited_results(cells, inputs, origin, scale, MAX_COVERAGE_WORK, lakes)
}

// At most one depth per area cell: a fixed <=2 MiB lookup, independent of
// detail resolution. None preserves categorical callers without lake surfaces.
pub(crate) fn validated_lake_depths(
    cells: &AreaCells,
    lakes: &[Lake],
) -> Result<Vec<Option<u32>>, RenderError> {
    const COUNT: usize = 512 * 512;
    let invalid_lake = |reason| RenderError::LakeGeometry { reason };
    if lakes.len() > COUNT {
        return Err(invalid_lake("area exceeds lake record limit"));
    }
    let mut memberships = 0usize;
    for lake in lakes {
        if lake.cells.len() > COUNT - memberships {
            return Err(invalid_lake("area exceeds lake membership limit"));
        }
        memberships += lake.cells.len();
    }
    if memberships == 0 {
        return Ok(Vec::new());
    }
    let mut depths = vec![None; COUNT];
    for lake in lakes {
        for &at in &lake.cells {
            let cell = cells.get(at);
            let depth = i64::from(lake.surface.raw()) - i64::from(cell.height.raw());
            if cell.terrain != TerrainKind::Lake || depth <= 0 {
                return Err(invalid_lake(
                    "saved lake surface does not cover its physical bed",
                ));
            }
            let index = usize::from(at.y()) * 512 + usize::from(at.x());
            if depths[index].is_some() {
                return Err(invalid_lake("duplicate lake cell membership"));
            }
            let depth = u32::try_from(depth)
                .map_err(|_| invalid_lake("lake depth exceeds height range"))?;
            depths[index] = Some(depth);
        }
    }
    Ok(depths)
}

fn lake_colours(cells: &AreaCells, lakes: &[Lake]) -> Result<Vec<Option<[u8; 3]>>, RenderError> {
    Ok(validated_lake_depths(cells, lakes)?
        .into_iter()
        .map(|depth| depth.map(lake_colour))
        .collect())
}

/// Prepared geometry and a single reusable scanline. Shape event indexes are
/// O(shapes), rather than O(shapes × image height); candidate storage has a
/// fixed independent cap. No RGB image, mask, or per-image pixel array exists.
pub(crate) struct AreaRaster<'a> {
    cells: &'a AreaCells,
    terrain: Option<&'a AtlasTerrain>,
    lake_colours: Vec<Option<[u8; 3]>>,
    shapes: Vec<Shape>,
    starts: Vec<usize>,
    ends: Vec<usize>,
    next_start: usize,
    next_end: usize,
    active: BTreeSet<usize>,
    pixels: Vec<Vec<usize>>,
    pixel_capacity: usize,
    base_row: Vec<u8>,
    base_cell_y: Option<usize>,
    rgb: Vec<u8>,
    last_y: Option<usize>,
    side: usize,
    work: WorkBudget,
}

fn coverage_work_limit(side: u32) -> u64 {
    // Preserve the preview/detail budget and grow with actual output work.
    // At 32K the limit is 16 billion units, still a finite resource refusal.
    let factor = u64::from(side).div_ceil(4096);
    MAX_COVERAGE_WORK * factor * factor
}

impl<'a> AreaRaster<'a> {
    #[cfg(test)]
    pub(crate) fn new(
        cells: &'a AreaCells,
        inputs: impl IntoIterator<Item = Result<ChannelInput, RenderError>>,
        origin: GlobalCell,
        scale: AreaImageScale,
        lakes: &[Lake],
    ) -> Result<Self, RenderError> {
        Self::new_with_terrain(cells, None, inputs, origin, scale, lakes)
    }

    pub(crate) fn new_with_terrain(
        cells: &'a AreaCells,
        terrain: Option<&'a AtlasTerrain>,
        inputs: impl IntoIterator<Item = Result<ChannelInput, RenderError>>,
        origin: GlobalCell,
        scale: AreaImageScale,
        lakes: &[Lake],
    ) -> Result<Self, RenderError> {
        Self::with_terrain_and_work_limit(
            cells,
            terrain,
            inputs,
            origin,
            scale,
            lakes,
            coverage_work_limit(scale.side()),
        )
    }

    #[cfg(test)]
    fn with_work_limit(
        cells: &'a AreaCells,
        inputs: impl IntoIterator<Item = Result<ChannelInput, RenderError>>,
        origin: GlobalCell,
        scale: AreaImageScale,
        lakes: &[Lake],
        work_limit: u64,
    ) -> Result<Self, RenderError> {
        Self::with_terrain_and_work_limit(cells, None, inputs, origin, scale, lakes, work_limit)
    }

    fn with_terrain_and_work_limit(
        cells: &'a AreaCells,
        terrain: Option<&'a AtlasTerrain>,
        inputs: impl IntoIterator<Item = Result<ChannelInput, RenderError>>,
        origin: GlobalCell,
        scale: AreaImageScale,
        lakes: &[Lake],
        work_limit: u64,
    ) -> Result<Self, RenderError> {
        let lake_colours = lake_colours(cells, lakes)?;
        let offset = |node: GlobalCell| terrain.map_or((0, 0), |t| t.channel_offset_um(node));
        let formed = terrain.is_some_and(AtlasTerrain::is_formed);
        let shapes = shapes_results(
            inputs,
            origin,
            scale,
            if formed { Some(&offset) } else { None },
        )?;
        let mut starts: Vec<_> = (0..shapes.len()).collect();
        let mut ends = starts.clone();
        starts.sort_unstable_by_key(|&i| (shapes[i].bounds[2], i));
        ends.sort_unstable_by_key(|&i| (shapes[i].bounds[3], i));
        let side = scale.side() as usize;
        Ok(Self {
            cells,
            terrain,
            lake_colours,
            shapes,
            starts,
            ends,
            next_start: 0,
            next_end: 0,
            active: BTreeSet::new(),
            pixels: vec![Vec::new(); side],
            pixel_capacity: 0,
            base_row: vec![0; side * 3],
            base_cell_y: None,
            rgb: vec![0; side * 3],
            last_y: None,
            side,
            work: WorkBudget::new(work_limit),
        })
    }

    fn base(&mut self, y: usize) -> Result<(), RenderError> {
        let cell_y = y * 512 / self.side;
        let Some(terrain) = self.terrain else {
            if self.base_cell_y == Some(cell_y) {
                return Ok(());
            }
            for cell_x in 0..512 {
                let at = CellCoord::new(
                    u16::try_from(cell_x).map_err(|_| invalid("raster cell leaves area"))?,
                    u16::try_from(cell_y).map_err(|_| invalid("raster cell leaves area"))?,
                )
                .ok_or_else(|| invalid("raster cell leaves area"))?;
                let cell = self.cells.get(at);
                let colour = match cell.terrain {
                    TerrainKind::Lake => self
                        .lake_colours
                        .get(cell_y * 512 + cell_x)
                        .copied()
                        .flatten()
                        .unwrap_or(LAKE_FILL),
                    TerrainKind::Sea => sea_colour(cell.height.raw()),
                    TerrainKind::Land => land_colour(cell.height.raw()),
                };
                let start = (cell_x * self.side).div_ceil(512);
                let end = ((cell_x + 1) * self.side).div_ceil(512);
                for pixel in self.base_row[start * 3..end * 3].as_chunks_mut::<3>().0 {
                    pixel.copy_from_slice(&colour);
                }
            }
            self.base_cell_y = Some(cell_y);
            return Ok(());
        };

        let pixel_y = u32::try_from(y).map_err(|_| invalid("pixel exceeds raster"))?;
        let side = u32::try_from(self.side).map_err(|_| invalid("pixel exceeds raster"))?;
        let y_kernel = axis_kernel(pixel_y, side)?;
        let cells = self.cells;
        let lake_colours = &self.lake_colours;
        let row_side = self.side;
        // Pixels are pure functions of saved data: render 256-pixel chunks
        // of the row in parallel; output is identical for any thread count.
        self.base_row
            .par_chunks_mut(256 * 3)
            .enumerate()
            .try_for_each(|(chunk, out)| -> Result<(), RenderError> {
                for (k, px) in out.as_chunks_mut::<3>().0.iter_mut().enumerate() {
                    let x = chunk * 256 + k;
                    let cell_x = x * 512 / row_side;
                    let at = CellCoord::new(
                        u16::try_from(cell_x).map_err(|_| invalid("raster cell leaves area"))?,
                        u16::try_from(cell_y).map_err(|_| invalid("raster cell leaves area"))?,
                    )
                    .ok_or_else(|| invalid("raster cell leaves area"))?;
                    let cell = cells.get(at);
                    let x_kernel = axis_kernel(
                        u32::try_from(x).map_err(|_| invalid("pixel exceeds raster"))?,
                        side,
                    )?;
                    let colour = if cell.terrain == TerrainKind::Lake {
                        if terrain.has_lake_depths() {
                            terrain.sample(x_kernel, y_kernel, TerrainKind::Lake)?
                        } else {
                            lake_colours
                                .get(cell_y * 512 + cell_x)
                                .copied()
                                .flatten()
                                .unwrap_or(LAKE_FILL)
                        }
                    } else {
                        let class = terrain.contour_class(x_kernel, y_kernel, at, cell.terrain)?;
                        terrain.sample(x_kernel, y_kernel, class)?
                    };
                    px.copy_from_slice(&colour);
                }
                Ok(())
            })?;
        self.base_cell_y = None;
        Ok(())
    }

    /// Rows may be skipped, but must be requested in increasing order. This
    /// also allows small-window verification at the largest supported size.
    pub(crate) fn row(&mut self, y: usize) -> Result<&[u8], RenderError> {
        if y >= self.side || self.last_y.is_some_and(|last| y <= last) {
            return Err(invalid("area rows must advance within the raster"));
        }
        self.last_y = Some(y);
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
        self.work.charge(self.active.len() as u64)?;
        for p in &mut self.pixels {
            p.clear();
        }
        // Stable shape order retains fixed-point union/rounding behavior.
        for &i in &self.active {
            let bounds = self.shapes[i].bounds;
            for p in &mut self.pixels[bounds[0]..=bounds[1]] {
                if p.len() == MAX_PIXEL_CANDIDATES {
                    return Err(invalid("pixel exceeds 4096 candidate channel polygons"));
                }
                self.work.charge(1)?;
                let before = p.capacity();
                p.push(i);
                self.pixel_capacity += p.capacity() - before;
                if self.pixel_capacity > MAX_PIXEL_CAPACITY {
                    return Err(invalid(
                        "channel raster exceeds pixel-reference memory budget",
                    ));
                }
            }
        }
        let cell_y = y * 512 / self.side;
        self.base(y)?;
        self.rgb.copy_from_slice(&self.base_row);
        for (x, ids) in self.pixels.iter().enumerate() {
            if ids.is_empty() {
                continue;
            }
            let cell_x = x * 512 / self.side;
            let at = CellCoord::new(
                u16::try_from(cell_x).map_err(|_| invalid("raster cell leaves area"))?,
                u16::try_from(cell_y).map_err(|_| invalid("raster cell leaves area"))?,
            )
            .ok_or_else(|| invalid("raster cell leaves area"))?;
            let saved = self.cells.get(at).terrain;
            let class = if let Some(terrain) = self.terrain {
                let side = u32::try_from(self.side).map_err(|_| invalid("pixel exceeds raster"))?;
                let pixel_x = u32::try_from(x).map_err(|_| invalid("pixel exceeds raster"))?;
                let pixel_y = u32::try_from(y).map_err(|_| invalid("pixel exceeds raster"))?;
                terrain.contour_class(
                    axis_kernel(pixel_x, side)?,
                    axis_kernel(pixel_y, side)?,
                    at,
                    saved,
                )?
            } else {
                saved
            };
            if class != TerrainKind::Land {
                continue;
            }
            let shape_slice = self.shapes.as_slice();
            let select = |marker| {
                ids.iter().filter_map(move |&i| {
                    let s = &shape_slice[i];
                    (s.marker == marker).then_some((&s.polygon, s.discharge))
                })
            };
            let pixel_x = u32::try_from(x).map_err(|_| invalid("pixel exceeds raster"))?;
            let pixel_y = u32::try_from(y).map_err(|_| invalid("pixel exceeds raster"))?;
            let physical = coverage(select(false), pixel_x, pixel_y, &mut self.work)?;
            let marker = coverage(select(true), pixel_x, pixel_y, &mut self.work)?;
            self.work.charge(
                physical.roundings + marker.roundings + (physical.pieces + marker.pieces) as u64,
            )?;
            let alpha = visual_alpha(physical.alpha, marker.alpha);
            let water = channel_colour(physical.maximum_discharge.max(marker.maximum_discharge));
            for (value, water) in self.rgb[x * 3..x * 3 + 3].iter_mut().zip(water) {
                let blended =
                    (u32::from(*value) * (65_535 - alpha) + u32::from(water) * alpha + 32_767)
                        / 65_535;
                *value = u8::try_from(blended).map_err(|_| invalid("invalid colour blend"))?;
            }
        }
        Ok(&self.rgb)
    }
}

#[cfg(test)]
fn raster_limited_results(
    cells: &AreaCells,
    inputs: impl IntoIterator<Item = Result<ChannelInput, RenderError>>,
    origin: GlobalCell,
    scale: AreaImageScale,
    work_limit: u64,
    lakes: &[Lake],
) -> Result<Vec<u8>, RenderError> {
    let mut raster = AreaRaster::with_work_limit(cells, inputs, origin, scale, lakes, work_limit)?;
    let side = scale.side() as usize;
    let mut rgb = Vec::with_capacity(side * side * 3);
    for y in 0..side {
        rgb.extend_from_slice(raster.row(y)?);
    }
    Ok(rgb)
}

#[cfg(test)]
mod tests;
