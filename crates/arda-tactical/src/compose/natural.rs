//! Natural rock faces (OI-38): wall runs tagged `natural` are drawn as one
//! smoothed outline per chain of edges instead of kit pieces on square
//! edges, so a cave's walls read as rock, not as stair steps.
//!
//! The layout keeps its unit edges (rules, sight and the scene see the
//! same walls); only the drawing changes. Each chain of natural edges is
//! traced through the lattice vertices, its vertices are nudged by a
//! hashed jitter (never the ends of an open chain) and the polyline is
//! rounded by corner cutting. The kit's run piece is then swept along the
//! rounded line, its band following the curve. Between the square edges
//! and the rounded line the ground of the wrong side shows (floors stop at
//! walls); those pixels — the even-odd fill of the square chain together
//! with its rounded twin — are repainted with the ground mirrored across
//! the nearest square edge, so the rock and the floor meet at the curve.
//!
//! Everything is integer or plain `f32` arithmetic in a fixed order, so the
//! same inputs give the same pixels.

use super::stamp::{Op, Stamp};
use super::{pick, stamp_of, Pose, Sprites};
use crate::catalog::WallRole;
use crate::layout::{EdgeAxis, TacticalLayout, WallSegment};
use crate::library::Library;
use crate::noise::{hash2, unit};
use crate::raster::Rgba;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// A unit edge `(axis, x, y)` as in [`WallSegment`].
pub type EdgeKey = (EdgeAxis, u32, u32);

/// Largest corner cut, squares: the rounding radius of a long wall's
/// corner.
const CUT_MAX: f32 = 0.5;
/// Corner-cutting passes.
const PASSES: usize = 3;
/// Largest jitter of a vertex on either axis, squares.
const JITTER: f32 = 0.12;
/// Edge of the output tiles, squares.
const TILE: u32 = 8;

/// A chain of natural edges: its lattice vertices in order and its kit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    /// Vertices; a closed chain does not repeat its first.
    pub vertices: Vec<(u32, u32)>,
    /// Whether the last vertex joins the first.
    pub closed: bool,
    /// Wall kit of the chain's first edge.
    pub kit: String,
}

/// Whether a segment is drawn as a natural outline: a plain run tagged
/// [`crate::layout::TAG_NATURAL`].
#[must_use]
pub fn is_natural(w: &WallSegment) -> bool {
    w.has_tag(crate::layout::TAG_NATURAL) && w.drawn_role() == WallRole::Run
}

fn ends((axis, x, y): EdgeKey) -> [(u32, u32); 2] {
    match axis {
        EdgeAxis::Horizontal => [(x, y), (x + 1, y)],
        EdgeAxis::Vertical => [(x, y), (x, y + 1)],
    }
}

/// The natural edges' chains. At a vertex the arms pair in north, east,
/// south, west order (first with second, third with fourth); an unpaired
/// arm ends an open chain.
#[must_use]
pub fn chains(edges: &BTreeMap<EdgeKey, &WallSegment>) -> Vec<Chain> {
    // Arms of each vertex in N, E, S, W order.
    let mut arms: BTreeMap<(u32, u32), [Option<EdgeKey>; 4]> = BTreeMap::new();
    for &e in edges.keys() {
        let [a, b] = ends(e);
        let (da, db) = match e.0 {
            EdgeAxis::Horizontal => (1, 3),
            EdgeAxis::Vertical => (2, 0),
        };
        arms.entry(a).or_default()[da] = Some(e);
        arms.entry(b).or_default()[db] = Some(e);
    }
    let partner = |v: (u32, u32), e: EdgeKey| -> Option<EdgeKey> {
        let present: Vec<EdgeKey> = arms.get(&v)?.iter().flatten().copied().collect();
        let i = present.iter().position(|&a| a == e)?;
        present.get(i ^ 1).copied()
    };
    let other = |e: EdgeKey, v: (u32, u32)| {
        let [a, b] = ends(e);
        if a == v {
            b
        } else {
            a
        }
    };
    let kit = |e: EdgeKey| edges.get(&e).map_or_else(String::new, |w| w.kit.clone());
    let mut seen: BTreeSet<EdgeKey> = BTreeSet::new();
    let mut out = Vec::new();
    let walk = |start: (u32, u32), e0: EdgeKey, seen: &mut BTreeSet<EdgeKey>| -> Chain {
        let mut vertices = vec![start];
        let (mut v, mut e) = (start, e0);
        let mut closed = false;
        loop {
            seen.insert(e);
            v = other(e, v);
            match partner(v, e) {
                Some(n) if n == e0 && v == start => {
                    closed = true;
                    break;
                }
                Some(n) if !seen.contains(&n) => {
                    vertices.push(v);
                    e = n;
                }
                _ => {
                    vertices.push(v);
                    break;
                }
            }
        }
        Chain {
            vertices,
            closed,
            kit: kit(e0),
        }
    };
    // Open chains first, from their unpaired ends.
    for (&v, a) in &arms {
        let present: Vec<EdgeKey> = a.iter().flatten().copied().collect();
        if present.len() % 2 == 1 {
            if let Some(&e) = present.last() {
                if !seen.contains(&e) {
                    out.push(walk(v, e, &mut seen));
                }
            }
        }
    }
    for &e in edges.keys() {
        if !seen.contains(&e) {
            out.push(walk(ends(e)[0], e, &mut seen));
        }
    }
    out
}

/// One corner-cutting pass: each segment keeps its middle, cut by a
/// quarter of its length at most [`CUT_MAX`] from each end; an open
/// line keeps its ends.
fn cut(points: &[(f32, f32)], closed: bool) -> Vec<(f32, f32)> {
    let n = points.len();
    if n < 3 {
        return points.to_vec();
    }
    let segs = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(2 * n);
    if !closed {
        out.push(points[0]);
    }
    for i in 0..segs {
        let (a, b) = (points[i], points[(i + 1) % n]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt();
        let k = if len > 0.0 {
            (CUT_MAX / len).min(0.25)
        } else {
            0.0
        };
        if closed || i > 0 {
            out.push((a.0 + k * dx, a.1 + k * dy));
        }
        if closed || i + 1 < segs {
            out.push((b.0 - k * dx, b.1 - k * dy));
        }
    }
    if !closed {
        out.push(points[n - 1]);
    }
    out
}

/// The rounded outline of a chain, squares. The chain's unit edges are
/// replaced by their midpoints, which puts a one-square staircase on a
/// straight diagonal and chamfers every corner; each midpoint is nudged by
/// a hash of its world position, then the line is cut [`PASSES`] times.
/// An open chain keeps its two end vertices.
#[must_use]
pub fn outline(c: &Chain, seed: u64, origin: (i64, i64)) -> Vec<(f32, f32)> {
    let v = &c.vertices;
    let n = v.len();
    let segs = if c.closed { n } else { n.saturating_sub(1) };
    #[allow(clippy::cast_precision_loss)] // lattice vertices are small
    let at = |(x, y): (u32, u32)| (x as f32, y as f32);
    let mut pts = Vec::with_capacity(segs + 2);
    if !c.closed {
        if let Some(&first) = v.first() {
            pts.push(at(first));
        }
    }
    for i in 0..segs {
        let (a, b) = (v[i], v[(i + 1) % n]);
        // Twice the midpoint, in world half squares, keys the jitter.
        let (wx, wy) = (
            2 * origin.0 + i64::from(a.0) + i64::from(b.0),
            2 * origin.1 + i64::from(a.1) + i64::from(b.1),
        );
        let jx = (unit(hash2(seed ^ 0x4E47, wx, wy)) * 2.0 - 1.0) * JITTER;
        let jy = (unit(hash2(seed ^ 0x4E48, wx, wy)) * 2.0 - 1.0) * JITTER;
        let (pa, pb) = (at(a), at(b));
        pts.push(((pa.0 + pb.0) / 2.0 + jx, (pa.1 + pb.1) / 2.0 + jy));
    }
    if !c.closed {
        if let Some(&last) = v.last() {
            pts.push(at(last));
        }
    }
    for _ in 0..PASSES {
        pts = cut(&pts, c.closed);
    }
    pts
}

/// Output tiles: pixels written by the fill or the sweep, and the sweep's
/// distance buffer.
struct Tiles {
    side: u32,
    w: u32,
    h: u32,
    tiles: BTreeMap<(u32, u32), (Rgba, Vec<f32>)>,
}

impl Tiles {
    fn new(ppsq: u32, w: u32, h: u32) -> Self {
        Self {
            side: TILE * ppsq,
            w,
            h,
            tiles: BTreeMap::new(),
        }
    }

    fn slot(&mut self, x: u32, y: u32) -> Option<(&mut Rgba, &mut Vec<f32>, u32, u32)> {
        if x >= self.w || y >= self.h {
            return None;
        }
        let (tx, ty) = (x / self.side, y / self.side);
        let (tw, th) = (
            self.side.min(self.w - tx * self.side),
            self.side.min(self.h - ty * self.side),
        );
        let (img, dist) = self.tiles.entry((tx, ty)).or_insert_with(|| {
            (
                Rgba::new(tw, th),
                vec![f32::INFINITY; tw as usize * th as usize],
            )
        });
        Some((img, dist, x - tx * self.side, y - ty * self.side))
    }

    fn into_ops(self, stamp: Stamp) -> Vec<Op> {
        let side = i64::from(self.side);
        self.tiles
            .into_iter()
            .map(|((tx, ty), (img, _))| Op {
                sprite: Arc::new(img),
                ox: i64::from(tx) * side,
                oy: i64::from(ty) * side,
                stamp,
            })
            .collect()
    }
}

/// Pixels of the even-odd fill of `lines` (pixel coordinates) inside
/// `area`, row by row.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped pixel rows
fn even_odd(lines: &[Vec<(f32, f32)>], area: [u32; 4]) -> Vec<(u32, u32)> {
    let [x0, y0, x1, y1] = area;
    let mut rows: BTreeMap<u32, Vec<f32>> = BTreeMap::new();
    for line in lines {
        for s in line.windows(2) {
            let (a, b) = (s[0], s[1]);
            if a.1 == b.1 {
                continue;
            }
            let (lo, hi) = if a.1 < b.1 { (a.1, b.1) } else { (b.1, a.1) };
            // Rows whose centre lies in [lo, hi).
            let r0 = (lo - 0.5).ceil().max(y0 as f32) as u32;
            let r1 = ((hi - 0.5).ceil().max(0.0) as u32).min(y1);
            for r in r0..r1 {
                let yc = r as f32 + 0.5;
                let x = a.0 + (yc - a.1) * (b.0 - a.0) / (b.1 - a.1);
                rows.entry(r).or_default().push(x);
            }
        }
    }
    let mut out = Vec::new();
    for (r, mut xs) in rows {
        xs.sort_by(f32::total_cmp);
        for pair in xs.as_chunks::<2>().0 {
            let from = (pair[0] - 0.5).ceil().max(x0 as f32) as u32;
            let to = ((pair[1] - 0.5).ceil().max(0.0) as u32).min(x1);
            out.extend((from..to).map(|x| (x, r)));
        }
    }
    out
}

/// The nearest point of the natural unit edges around pixel `p` (pixel
/// coordinates), searched among the edges of its square and of the
/// squares meeting its corners.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // pixel positions
fn nearest_edge(edges: &BTreeSet<EdgeKey>, p: (f32, f32), ppsq: f32) -> Option<(f32, f32)> {
    let (sx, sy) = ((p.0 / ppsq).floor(), (p.1 / ppsq).floor());
    let mut best: Option<((f32, f32), f32)> = None;
    for dy in -1..=2_i32 {
        for dx in -1..=2_i32 {
            let (x, y) = (sx + dx as f32, sy + dy as f32);
            if x < 0.0 || y < 0.0 {
                continue;
            }
            for axis in [EdgeAxis::Horizontal, EdgeAxis::Vertical] {
                if !edges.contains(&(axis, x as u32, y as u32)) {
                    continue;
                }
                let a = (x * ppsq, y * ppsq);
                let b = match axis {
                    EdgeAxis::Horizontal => (a.0 + ppsq, a.1),
                    EdgeAxis::Vertical => (a.0, a.1 + ppsq),
                };
                let n = (
                    p.0.clamp(a.0.min(b.0), a.0.max(b.0)),
                    p.1.clamp(a.1.min(b.1), a.1.max(b.1)),
                );
                let d = (n.0 - p.0) * (n.0 - p.0) + (n.1 - p.1) * (n.1 - p.1);
                if best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((n, d));
                }
            }
        }
    }
    best.map(|(n, _)| n)
}

/// The rows of `sprite` its band covers: `(centre row, half thickness)`.
#[allow(clippy::cast_precision_loss)] // sprite rows are small
fn band(sprite: &Rgba) -> Option<(f32, f32)> {
    let opaque = |y: u32| (0..sprite.width).any(|x| sprite.get(x, y)[3] > 0);
    let top = (0..sprite.height).find(|&y| opaque(y))?;
    let bottom = (0..sprite.height).rev().find(|&y| opaque(y))?;
    Some(((top + bottom) as f32 / 2.0, (bottom - top + 1) as f32 / 2.0))
}

/// Sweeps `sprite`'s band along `line` (pixel coordinates).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)] // clamped pixel positions
fn sweep(tiles: &mut Tiles, line: &[(f32, f32)], sprite: &Rgba, (centre, half): (f32, f32)) {
    let reach = half + 0.5;
    let sw = sprite.width as f32;
    let mut along = 0.0_f32;
    for s in line.windows(2) {
        let (a, b) = (s[0], s[1]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = dx * dx + dy * dy;
        let len = len2.sqrt();
        if len2 <= 0.0 {
            continue;
        }
        let xa = (a.0.min(b.0) - reach - 1.0).floor().max(0.0) as u32;
        let xb = (a.0.max(b.0) + reach + 1.0).ceil().max(0.0) as u32;
        let ya = (a.1.min(b.1) - reach - 1.0).floor().max(0.0) as u32;
        let yb = (a.1.max(b.1) + reach + 1.0).ceil().max(0.0) as u32;
        for y in ya..yb {
            for x in xa..xb {
                let p = (x as f32 + 0.5, y as f32 + 0.5);
                let (px, py) = (p.0 - a.0, p.1 - a.1);
                let t = ((px * dx + py * dy) / len2).clamp(0.0, 1.0);
                let (nx, ny) = (a.0 + t * dx - p.0, a.1 + t * dy - p.1);
                let dist = (nx * nx + ny * ny).sqrt();
                if dist > reach {
                    continue;
                }
                let side = if dx * py - dy * px < 0.0 { -1.0 } else { 1.0 };
                let v = (centre + side * dist).round();
                let u = (along + t * len).rem_euclid(sw).floor();
                if v < 0.0 || v >= sprite.height as f32 {
                    continue;
                }
                let px_ = sprite.get((u as u32).min(sprite.width - 1), v as u32);
                if px_[3] == 0 {
                    continue;
                }
                if let Some((img, d, lx, ly)) = tiles.slot(x, y) {
                    let i = ly as usize * img.width as usize + lx as usize;
                    if dist < d[i] {
                        d[i] = dist;
                        img.set(lx, ly, px_);
                    }
                }
            }
        }
        along += len;
    }
}

/// Draws the layout's natural walls: returns the ground repaint ops (to
/// go under every sprite) and the outline ops (drawn with the walls).
/// `ground` is the canvas with the ground painted inside `area`.
#[allow(clippy::cast_precision_loss)] // pixel positions
pub(super) fn draw(
    sprites: &mut Sprites<'_>,
    lib: &Library,
    seed: u64,
    layout: &TacticalLayout,
    edges: &BTreeMap<EdgeKey, &WallSegment>,
    ground: &Rgba,
    area: [u32; 4],
) -> (Vec<Op>, Vec<Op>) {
    if edges.is_empty() {
        return (Vec::new(), Vec::new());
    }
    let ppsq = sprites.ppsq;
    let s = ppsq as f32;
    let origin = layout.world_origin();
    let keys: BTreeSet<EdgeKey> = edges.keys().copied().collect();
    let mut fill = Tiles::new(ppsq, ground.width, ground.height);
    let mut walls = Tiles::new(ppsq, ground.width, ground.height);
    let mut stamp = None;
    for c in chains(edges) {
        let px = |pts: Vec<(f32, f32)>, closed: bool| {
            let mut v: Vec<(f32, f32)> = pts.into_iter().map(|(x, y)| (x * s, y * s)).collect();
            if closed {
                if let Some(&f) = v.first() {
                    v.push(f);
                }
            }
            v
        };
        let square = px(
            c.vertices
                .iter()
                .map(|&(x, y)| (x as f32, y as f32))
                .collect(),
            c.closed,
        );
        let round = px(outline(&c, seed, origin), c.closed);
        // Between the square chain and its rounded twin, the far side's
        // ground mirrored across the nearest square edge.
        for (x, y) in even_odd(&[square, round.clone()], area) {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            let Some(n) = nearest_edge(&keys, p, s) else {
                continue;
            };
            let q = ((2.0 * n.0 - p.0).floor(), (2.0 * n.1 - p.1).floor());
            let inside = q.0 >= area[0] as f32
                && q.1 >= area[1] as f32
                && q.0 < area[2] as f32
                && q.1 < area[3] as f32;
            if !inside {
                continue;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // checked above
            let mut c = ground.get(q.0 as u32, q.1 as u32);
            c[3] = 255;
            if let Some((img, _, lx, ly)) = fill.slot(x, y) {
                img.set(lx, ly, c);
            }
        }
        let pieces = lib.wall_pieces(&c.kit, WallRole::Run);
        let (vx, vy) = c.vertices.first().copied().unwrap_or((0, 0));
        let Some(a) = pick(
            &pieces,
            seed,
            origin.0 + i64::from(vx),
            origin.1 + i64::from(vy),
        ) else {
            continue;
        };
        let Some(sprite) = sprites.get(a, Pose::turned(0, false)) else {
            continue;
        };
        if let Some(b) = band(&sprite) {
            sweep(&mut walls, &round, &sprite, b);
            stamp.get_or_insert_with(|| stamp_of(a, ppsq, 100));
        }
    }
    let flat = Stamp {
        height: None,
        occludes: false,
        crown: None,
    };
    let outline = stamp.map_or_else(Vec::new, |st| walls.into_ops(st));
    (fill.into_ops(flat), outline)
}

#[cfg(test)]
#[path = "natural_tests.rs"]
mod tests;
