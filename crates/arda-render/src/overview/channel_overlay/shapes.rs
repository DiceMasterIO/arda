//! Channel shapes in output pixel space: display widths, polygon append
//! under the shape and vertex caps, and weaving braid threads.

use super::{
    bad, OverviewChannelContext, FINE_DISPLAY_WIDTH_DM, FORMED_DISPLAY_WIDTH_DM,
    LEGACY_DISPLAY_WIDTH_DM, MAX_SHAPES, MAX_VERTICES,
};
use crate::channel_geometry::{intersection, rectangle, strip_free, Point, Polygon, WorkBudget, Q};
use crate::overview::ChannelStyle;
use crate::RenderError;
use arda_core::{AreaCoord, GlobalCell};

pub(super) struct Cap {
    pub(super) node: GlobalCell,
    pub(super) points: [Point; 2],
    pub(super) discharge: u64,
}
pub(super) struct Shape {
    pub(super) polygon: Polygon,
    pub(super) bounds: [u32; 4],
    pub(super) discharge: u64,
}
pub(super) fn display_width(width: u32, style: ChannelStyle) -> Result<i64, RenderError> {
    if width == 0 {
        return Err(bad("saved channel has zero width"));
    }
    let dm = match style {
        ChannelStyle::Legacy => (u64::from(width) * 8).min(LEGACY_DISPLAY_WIDTH_DM),
        ChannelStyle::Fine => (u64::from(width) * 3).min(FINE_DISPLAY_WIDTH_DM),
        ChannelStyle::Formed => (u64::from(width) * 4).min(FORMED_DISPLAY_WIDTH_DM),
    };
    i64::try_from(dm).map_err(|_| bad("display width conversion overflow"))
}

pub(super) fn center(node: GlobalCell) -> Point {
    (i64::from(node.x) * Q + Q / 2, i64::from(node.y) * Q + Q / 2)
}
pub(super) fn to_output(v: i64, source0: i64, pixel0: u32, span: u32) -> Result<i64, RenderError> {
    let n = i128::from(v - source0) * i128::from(span);
    let delta = (n + 256) / 512;
    let base = i128::from(pixel0) * i128::from(Q);
    i64::try_from(base + delta).map_err(|_| bad("Atlas overview channel coordinate overflow"))
}
pub(super) fn append(
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
pub(super) fn weave_q12(t: i64, period: i64) -> i64 {
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
pub(super) fn braid_threads(
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
        let w = display_width(e.from_width_dm, ChannelStyle::Formed)? * Q / 1000;
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
