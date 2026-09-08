//! Physical channel raster and scale-dependent preview marks.

use crate::carto::{lake_colour, land_colour, sea_colour, LAKE_FILL};
use crate::channel_geometry::{
    coverage, hull, strip, terminal_footprint, Point, Polygon, WorkBudget, Q,
};
use crate::{GlobalCell, RenderError};
use arda_core::{AreaCells, CellCoord, Lake, TerrainKind};

/// Resolution of an area image; both scales use the same 100 m terrain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AreaImageScale {
    /// 512 square pixels with a faint mark for subpixel streams.
    #[default]
    Preview,
    /// 4096 square pixels, showing physical channel coverage only.
    Detail,
}

impl AreaImageScale {
    /// Pixels along each side of the image.
    #[must_use]
    pub const fn side(self) -> u32 {
        match self {
            Self::Preview => 512,
            Self::Detail => 4096,
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
const MAX_ROW_REFERENCES: usize = 4_194_304;
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
) -> Result<Vec<Shape>, RenderError> {
    let side = scale.side() as usize;
    let ppc = i64::from(scale.side() / 512);
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
    let point = |node: GlobalCell| -> Point {
        (
            ((i64::from(node.x) - i64::from(origin.x)) * 2 + 1) * ppc * Q / 2,
            ((i64::from(node.y) - i64::from(origin.y)) * 2 + 1) * ppc * Q / 2,
        )
    };
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
        let wa = i64::from(edge.from_width_dm) * ppc * Q / 1000;
        let wb = i64::from(edge.to_width_dm) * ppc * Q / 1000;
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
            let (p, a, b) = strip(
                point(edge.from),
                point(edge.to),
                if is_marker { Q } else { wa },
                if is_marker { Q } else { wb },
            )?;
            append(p, edge.discharge, is_marker)?;
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
    shapes_results(inputs.into_iter().map(Ok), origin, scale)
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

pub(crate) fn raster_results(
    cells: &AreaCells,
    inputs: impl IntoIterator<Item = Result<ChannelInput, RenderError>>,
    origin: GlobalCell,
    scale: AreaImageScale,
    lakes: &[Lake],
) -> Result<Vec<u8>, RenderError> {
    raster_limited_results(cells, inputs, origin, scale, MAX_COVERAGE_WORK, lakes)
}

// At most one colour per area cell: a fixed <=1 MiB lookup, independent of
// detail resolution. None preserves categorical callers without lake surfaces.
fn lake_colours(cells: &AreaCells, lakes: &[Lake]) -> Result<Vec<Option<[u8; 3]>>, RenderError> {
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
    let mut colours = vec![None; COUNT];
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
            if colours[index].is_some() {
                return Err(invalid_lake("duplicate lake cell membership"));
            }
            let depth = u32::try_from(depth)
                .map_err(|_| invalid_lake("lake depth exceeds height range"))?;
            colours[index] = Some(lake_colour(depth));
        }
    }
    Ok(colours)
}

fn raster_limited_results(
    cells: &AreaCells,
    inputs: impl IntoIterator<Item = Result<ChannelInput, RenderError>>,
    origin: GlobalCell,
    scale: AreaImageScale,
    work_limit: u64,
    lakes: &[Lake],
) -> Result<Vec<u8>, RenderError> {
    let lake_colours = lake_colours(cells, lakes)?;
    let shapes = shapes_results(inputs, origin, scale)?;
    let side = scale.side() as usize;
    let ppc = side / 512;
    let references: usize = shapes.iter().map(|s| s.bounds[3] - s.bounds[2] + 1).sum();
    if references > MAX_ROW_REFERENCES {
        return Err(invalid("channel raster exceeds 4194304 row references"));
    }
    let mut work = WorkBudget::new(work_limit);
    let mut rows = vec![Vec::new(); side];
    for (i, s) in shapes.iter().enumerate() {
        for row in &mut rows[s.bounds[2]..=s.bounds[3]] {
            work.charge(1)?;
            row.push(i);
        }
    }
    let mut pixels = vec![Vec::new(); side];
    let mut pixel_capacity = 0;
    let mut rgb = vec![0; side * side * 3];
    for (y, row) in rows.iter().enumerate() {
        for p in &mut pixels {
            p.clear();
        }
        for &i in row {
            for p in &mut pixels[shapes[i].bounds[0]..=shapes[i].bounds[1]] {
                if p.len() == MAX_PIXEL_CANDIDATES {
                    return Err(invalid("pixel exceeds 4096 candidate channel polygons"));
                }
                work.charge(1)?;
                let before = p.capacity();
                p.push(i);
                pixel_capacity += p.capacity() - before;
                if pixel_capacity > MAX_PIXEL_CAPACITY {
                    return Err(invalid(
                        "channel raster exceeds pixel-reference memory budget",
                    ));
                }
            }
        }
        for (x, ids) in pixels.iter().enumerate() {
            let cell_x = u16::try_from(x / ppc).map_err(|_| invalid("raster cell leaves area"))?;
            let cell_y = u16::try_from(y / ppc).map_err(|_| invalid("raster cell leaves area"))?;
            let at =
                CellCoord::new(cell_x, cell_y).ok_or_else(|| invalid("raster cell leaves area"))?;
            let cell = cells.get(at);
            let mut colour = match cell.terrain {
                TerrainKind::Lake => lake_colours
                    .get(usize::from(cell_y) * 512 + usize::from(cell_x))
                    .copied()
                    .flatten()
                    .unwrap_or(LAKE_FILL),
                TerrainKind::Sea => sea_colour(cell.height.raw()),
                TerrainKind::Land => land_colour(cell.height.raw()),
            };
            if cell.terrain == TerrainKind::Land && !ids.is_empty() {
                let shape_slice = shapes.as_slice();
                let select = |marker| {
                    ids.iter().filter_map(move |&i| {
                        let s = &shape_slice[i];
                        (s.marker == marker).then_some((&s.polygon, s.discharge))
                    })
                };
                let pixel_x = u32::try_from(x).map_err(|_| invalid("pixel exceeds raster"))?;
                let pixel_y = u32::try_from(y).map_err(|_| invalid("pixel exceeds raster"))?;
                let physical = coverage(select(false), pixel_x, pixel_y, &mut work)?;
                let marker = coverage(select(true), pixel_x, pixel_y, &mut work)?;
                work.charge(
                    physical.roundings
                        + marker.roundings
                        + (physical.pieces + marker.pieces) as u64,
                )?;
                let alpha = visual_alpha(physical.alpha, marker.alpha);
                let water =
                    channel_colour(physical.maximum_discharge.max(marker.maximum_discharge));
                for k in 0..3 {
                    let blended = (u32::from(colour[k]) * (65_535 - alpha)
                        + u32::from(water[k]) * alpha
                        + 32_767)
                        / 65_535;
                    colour[k] =
                        u8::try_from(blended).map_err(|_| invalid("invalid colour blend"))?;
                }
            }
            rgb[(y * side + x) * 3..(y * side + x) * 3 + 3].copy_from_slice(&colour);
        }
    }
    Ok(rgb)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::{Cell, HeightMm};

    fn land() -> AreaCells {
        AreaCells::flat(Cell {
            terrain: TerrainKind::Land,
            height: HeightMm::new(180_000),
            ..Cell::default()
        })
    }
    fn edge(a: (u32, u32), b: (u32, u32), width: u32) -> ChannelInput {
        ChannelInput {
            from: GlobalCell { x: a.0, y: a.1 },
            to: GlobalCell { x: b.0, y: b.1 },
            from_width_dm: width,
            to_width_dm: width,
            discharge: 1000,
        }
    }
    fn pixel(rgb: &[u8], side: usize, x: usize, y: usize) -> [u8; 3] {
        rgb[(y * side + x) * 3..(y * side + x) * 3 + 3]
            .try_into()
            .unwrap()
    }

    fn lake(id: u32, at: CellCoord, depth: i32) -> Lake {
        Lake {
            global_id: arda_core::hydrology::BasinId(u64::from(id)),
            id,
            surface: HeightMm::new(depth),
            depth_mm: u32::try_from(depth).unwrap(),
            outlet: None,
            cells: vec![at],
        }
    }

    #[test]
    fn supplied_depth_changes_only_saved_lake_pixels_at_both_scales() {
        let mut cells = land();
        let positions = [(0, 0, 13), (10, 10, 28), (511, 511, 3000)];
        let lakes: Vec<_> = positions
            .iter()
            .enumerate()
            .map(|(i, &(x, y, depth))| {
                let at = CellCoord::new(x, y).unwrap();
                cells.set(
                    at,
                    Cell {
                        terrain: TerrainKind::Lake,
                        height: HeightMm::new(0),
                        ..Cell::default()
                    },
                );
                lake(u32::try_from(i + 1).unwrap(), at, depth)
            })
            .collect();
        let saved = lakes.clone();
        let inputs = [edge((10, 11), (10, 10), 141), edge((10, 10), (10, 9), 141)];
        let origin = GlobalCell { x: 0, y: 0 };
        for scale in [AreaImageScale::Preview, AreaImageScale::Detail] {
            let old =
                raster_results(&cells, inputs.into_iter().map(Ok), origin, scale, &[]).unwrap();
            let new =
                raster_results(&cells, inputs.into_iter().map(Ok), origin, scale, &lakes).unwrap();
            let side = scale.side() as usize;
            let ppc = side / 512;
            let mut changed = 0;
            for (i, (a, b)) in old.chunks_exact(3).zip(new.chunks_exact(3)).enumerate() {
                let x = i % side / ppc;
                let y = i / side / ppc;
                if let Some(&(_, _, depth)) = positions
                    .iter()
                    .find(|&&(lx, ly, _)| usize::from(lx) == x && usize::from(ly) == y)
                {
                    assert_eq!(a, LAKE_FILL);
                    assert_eq!(b, lake_colour(u32::try_from(depth).unwrap()));
                    assert_ne!(a, b);
                    changed += 1;
                } else {
                    assert_eq!(a, b, "nonlake pixel changed at {i}");
                }
            }
            assert_eq!(changed, 3 * ppc * ppc);
        }
        assert_eq!(lakes, saved);
        let mut reversed = lakes.clone();
        reversed.reverse();
        assert_eq!(
            lake_colours(&cells, &lakes).unwrap(),
            lake_colours(&cells, &reversed).unwrap()
        );
    }

    #[test]
    fn supplied_lake_surface_and_membership_are_checked_before_raster() {
        let mut cells = land();
        let at = CellCoord::new(1, 1).unwrap();
        let mut record = lake(1, at, 1);
        assert!(lake_colours(&cells, &[record.clone()]).is_err());
        cells.set(
            at,
            Cell {
                terrain: TerrainKind::Lake,
                height: HeightMm::new(0),
                ..Cell::default()
            },
        );
        assert!(lake_colours(&cells, &[record.clone()]).is_ok());
        record.surface = HeightMm::new(0);
        assert!(lake_colours(&cells, &[record.clone()]).is_err());
        record.surface = HeightMm::new(-1);
        assert!(lake_colours(&cells, &[record.clone()]).is_err());
        record.surface = HeightMm::new(1);
        assert!(lake_colours(&cells, &[record.clone(), record.clone()]).is_err());
        record.cells = vec![at; 512 * 512 + 1];
        assert!(lake_colours(&cells, &[record]).is_err());
        assert!(lake_colours(&cells, &[]).unwrap().is_empty());
    }

    #[test]
    fn terminal_point_halo_coverage_preview_and_union_are_canonical() {
        let point = edge((511, 10), (511, 10), 2000);
        let origin = GlobalCell { x: 512, y: 0 };
        let rgb = raster(&land(), [point], origin, AreaImageScale::Preview).unwrap();
        assert_ne!(pixel(&rgb, 512, 0, 10), land_colour(180_000));
        assert_eq!(pixel(&rgb, 512, 1, 10), land_colour(180_000));
        let duplicated = raster(&land(), [point, point], origin, AreaImageScale::Preview).unwrap();
        assert_eq!(rgb, duplicated);
        // Translation to the same local center preserves every physical coverage bit.
        let moved = edge((0, 10), (0, 10), 2000);
        let left = shapes(
            [point],
            GlobalCell { x: 511, y: 0 },
            AreaImageScale::Preview,
        )
        .unwrap();
        let right = shapes([moved], GlobalCell { x: 0, y: 0 }, AreaImageScale::Preview).unwrap();
        assert_eq!(left[0].polygon, right[0].polygon);
        let small = edge((10, 10), (10, 10), 8);
        let preview = shapes([small], GlobalCell { x: 0, y: 0 }, AreaImageScale::Preview).unwrap();
        assert_eq!(preview.len(), 2);
        assert_eq!(preview.iter().filter(|p| p.marker).count(), 1);
        let detail = shapes([small], GlobalCell { x: 0, y: 0 }, AreaImageScale::Detail).unwrap();
        assert_eq!(detail.len(), 1);
        assert!(!detail[0].marker);
        for terrain in [TerrainKind::Sea, TerrainKind::Lake] {
            let cells = AreaCells::flat(Cell {
                terrain,
                ..Cell::default()
            });
            let plain = raster(&cells, [], origin, AreaImageScale::Preview).unwrap();
            assert_eq!(
                raster(&cells, [point], origin, AreaImageScale::Preview).unwrap(),
                plain
            );
        }
        assert!(raster_results(
            &land(),
            [Err(invalid("input failure"))],
            origin,
            AreaImageScale::Preview,
            &[]
        )
        .is_err());
    }

    #[test]
    fn preview_mark_is_faint_separate_and_bounded() {
        assert_eq!(visual_alpha(0, 0), 0);
        assert_eq!(visual_alpha(0, 65_535), 11_822);
        assert_eq!(visual_alpha(65_535, 65_535), 65_535);
        let inputs = [edge((10, 10), (11, 10), 10)];
        let preview = raster(
            &land(),
            inputs,
            GlobalCell { x: 0, y: 0 },
            AreaImageScale::Preview,
        )
        .unwrap();
        let detail_shapes =
            shapes(inputs, GlobalCell { x: 0, y: 0 }, AreaImageScale::Detail).unwrap();
        assert!(detail_shapes.iter().all(|s| !s.marker));
        let colour = pixel(&preview, 512, 10, 10);
        assert_ne!(colour, land_colour(180_000));
        assert_ne!(colour, channel_colour(1000));
        assert_eq!(pixel(&preview, 512, 10, 11), land_colour(180_000));
    }

    #[test]
    fn saved_water_classification_wins_over_channel_geometry() {
        for terrain in [TerrainKind::Sea, TerrainKind::Lake] {
            let mut cells = land();
            let at = CellCoord::new(10, 10).unwrap();
            cells.set(
                at,
                Cell {
                    terrain,
                    height: HeightMm::new(-2000),
                    ..Cell::default()
                },
            );
            let rgb = raster(
                &cells,
                [edge((10, 10), (11, 10), 230_650)],
                GlobalCell { x: 0, y: 0 },
                AreaImageScale::Preview,
            )
            .unwrap();
            assert_eq!(
                pixel(&rgb, 512, 10, 10),
                if terrain == TerrainKind::Sea {
                    sea_colour(-2000)
                } else {
                    [132, 176, 205]
                }
            );
        }
    }

    #[test]
    fn halo_edge_outside_the_area_can_cover_its_pixels() {
        let rgb = raster(
            &land(),
            [edge((511, 10), (511, 11), 2000)],
            GlobalCell { x: 512, y: 0 },
            AreaImageScale::Preview,
        )
        .unwrap();
        assert_ne!(pixel(&rgb, 512, 0, 10), land_colour(180_000));
        assert_eq!(pixel(&rgb, 512, 2, 10), land_colour(180_000));
    }

    #[test]
    fn reversed_input_order_and_exact_duplicates_do_not_change_raster() {
        let a = edge((10, 10), (11, 10), 10);
        let b = edge((11, 10), (12, 11), 20);
        let run = |edges| {
            raster(
                &land(),
                edges,
                GlobalCell { x: 0, y: 0 },
                AreaImageScale::Preview,
            )
            .unwrap()
        };
        assert_eq!(run(vec![a, b]), run(vec![b, a, a]));
        let mut conflicting = a;
        conflicting.discharge += 1;
        assert!(raster(
            &land(),
            [a, conflicting],
            GlobalCell { x: 0, y: 0 },
            AreaImageScale::Preview
        )
        .is_err());
    }
    #[test]
    fn wide_halo_candidates_over_water_still_consume_work_budget() {
        for terrain in [TerrainKind::Sea, TerrainKind::Lake] {
            let cells = AreaCells::flat(Cell {
                terrain,
                ..Cell::default()
            });
            let edges = [edge((255, 255), (256, 256), 200_000)];
            let origin = GlobalCell { x: 0, y: 0 };
            assert!(raster_limited(&cells, edges, origin, AreaImageScale::Preview, 1000).is_err());
            assert!(raster_limited(
                &cells,
                edges,
                origin,
                AreaImageScale::Preview,
                MAX_COVERAGE_WORK
            )
            .is_ok());
        }
    }
}
