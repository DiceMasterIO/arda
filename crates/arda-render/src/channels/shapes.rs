//! Channel edges as clipped polygon shapes in area-image space: tapered
//! strips, curved recipe-6 courses and junction caps, under fixed edge,
//! shape and vertex caps.

use super::{invalid, AreaImageScale, ChannelInput};
use crate::channel_curve::{
    curved_strip, min_width_px_q8, segments as curve_segments, source_width, Network,
};
use crate::channel_geometry::{hull, strip, strip_free, terminal_footprint, Point, Polygon, Q};
use crate::{GlobalCell, RenderError};
use std::collections::BTreeMap;

pub(super) struct Shape {
    pub(super) polygon: Polygon,
    pub(super) bounds: [usize; 4],
    pub(super) discharge: u64,
    pub(super) marker: bool,
}

pub(super) struct Cap {
    pub(super) node: GlobalCell,
    pub(super) points: [Point; 2],
    pub(super) discharge: u64,
    pub(super) marker: bool,
}

const MAX_EDGES: usize = 262_144;
const MAX_SHAPES: usize = 1_048_576;
const MAX_VERTICES: usize = 4_194_304;

pub(super) fn shape(polygon: Polygon, discharge: u64, marker: bool, side: usize) -> Option<Shape> {
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

/// Recipe-5 minimum on-screen channel width by discharge (v0.1 steps).
pub(super) fn min_width_v5(discharge: u64) -> i64 {
    match discharge {
        q if q >= 50_000 => Q * 8 / 5,
        q if q >= 5_000 => Q,
        q if q >= 1_000 => Q * 7 / 10,
        _ => 0,
    }
}

pub(super) fn shapes_results(
    inputs: impl IntoIterator<Item = Result<ChannelInput, RenderError>>,
    origin: GlobalCell,
    scale: AreaImageScale,
    offset_um: Option<&dyn Fn(GlobalCell) -> (i64, i64)>,
    curved: bool,
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
    // Recipe 6: smooth, relaxed centrelines tapered at sources, exactly as
    // the overview draws them (logic/04 §atlas-formed rivers). Recipe 5
    // keeps straight strips between snapped nodes.
    let network =
        (free && curved).then(|| Network::new(edges.iter().map(|e| (e.from, e.to, e.discharge))));
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
            let min_w = if curved {
                min_width_px_q8(edge.discharge) * Q / 256
            } else {
                min_width_v5(edge.discharge)
            };
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
pub(super) fn shapes(
    inputs: impl IntoIterator<Item = ChannelInput>,
    origin: GlobalCell,
    scale: AreaImageScale,
) -> Result<Vec<Shape>, RenderError> {
    shapes_results(inputs.into_iter().map(Ok), origin, scale, None, false)
}
