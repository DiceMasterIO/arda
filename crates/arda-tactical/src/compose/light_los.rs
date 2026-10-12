//! Light-pool occlusion: which parts of a pool its light can reach past
//! closed walls (`docs/capstone/logic/11-tactical-art-compositor.md`
//! §lighting).
//!
//! The mask is computed in layout coordinates, on a global grid of
//! [`CELLS`] samples per square, so it does not depend on the clip a tile
//! is rendered with and neighbouring tiles join without seams. Everything
//! is integer arithmetic.
//!
//! Rules:
//! - A wall edge blocks light by [`WallSegment::blocks_light`], the one
//!   function `arda-scene` also uses for `Wall::blocks_light`: windows pass
//!   light, closed doors, gates and secret doors and every other wall block.
//!   Sidecar edge overrides are scene-only and do not reach the picture.
//! - A sample is lit when the segment from the light to the sample crosses
//!   no blocking edge. A segment through a grid vertex is blocked only when
//!   blocking arms lie on both sides of it (the scene's vertex house rule,
//!   `arda-scene` `los.rs`).
//! - A light exactly on a grid line (or vertex) belongs to the square to
//!   its east and south: it is moved one sixteenth of a square that way.
//! - Only walls occlude; props, trees and canopies do not.
//! - The cell mask is softened with two 3-tap box blurs, then sampled
//!   bilinearly per pixel. A pool whose disc reaches no blocking edge gets
//!   no mask and renders exactly as before.

// Geometry is bounded by the layout size (at most a few thousand squares).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use super::lighting::Pool;
use crate::catalog::WallRole;
use crate::layout::{EdgeAxis, TacticalLayout, WallSegment};
use crate::library::Library;
use rayon::prelude::*;

/// Mask samples per square along each axis.
pub const CELLS: i64 = 8;
/// Integer geometry units per square (two per cell, so cell centres are odd).
const U: i64 = 2 * CELLS;

impl WallSegment {
    /// Whether the segment blocks light. A door, gate or secret door is
    /// closed in a layout and blocks; other pieces block when the library
    /// piece blocks sight (a window does not); a missing piece blocks
    /// unless it is a window.
    #[must_use]
    pub fn blocks_light(&self, lib: &Library) -> bool {
        if matches!(self.kind, WallRole::Door | WallRole::Gate) {
            return true;
        }
        match lib.wall_pieces(&self.kit, self.kind).into_iter().next() {
            Some(p) => p.blocks_sight,
            None => self.kind != WallRole::Window,
        }
    }
}

/// The light-blocking edges of a layout (later segments on an edge win).
pub struct LightWalls {
    w: i64,
    h: i64,
    horizontal: Vec<bool>,
    vertical: Vec<bool>,
}

impl LightWalls {
    /// Edges of `layout`, classified with `lib`.
    #[must_use]
    pub fn new(layout: &TacticalLayout, lib: &Library) -> Self {
        let (w, h) = (i64::from(layout.width), i64::from(layout.height));
        let mut s = Self {
            w,
            h,
            horizontal: vec![false; ((h + 1) * w).max(0) as usize],
            vertical: vec![false; (h * (w + 1)).max(0) as usize],
        };
        for seg in &layout.walls {
            let (x, y) = (i64::from(seg.x), i64::from(seg.y));
            if let Some(i) = s.index(seg.axis, x, y) {
                let blocks = seg.blocks_light(lib);
                match seg.axis {
                    EdgeAxis::Horizontal => s.horizontal[i] = blocks,
                    EdgeAxis::Vertical => s.vertical[i] = blocks,
                }
            }
        }
        s
    }

    fn index(&self, axis: EdgeAxis, x: i64, y: i64) -> Option<usize> {
        let ok = match axis {
            EdgeAxis::Horizontal => (0..self.w).contains(&x) && (0..=self.h).contains(&y),
            EdgeAxis::Vertical => (0..=self.w).contains(&x) && (0..self.h).contains(&y),
        };
        ok.then(|| match axis {
            EdgeAxis::Horizontal => (y * self.w + x) as usize,
            EdgeAxis::Vertical => (y * (self.w + 1) + x) as usize,
        })
    }

    /// Whether the edge blocks light (false outside the map).
    fn blocked(&self, axis: EdgeAxis, x: i64, y: i64) -> bool {
        self.index(axis, x, y).is_some_and(|i| match axis {
            EdgeAxis::Horizontal => self.horizontal[i],
            EdgeAxis::Vertical => self.vertical[i],
        })
    }

    /// Whether any blocking edge touches squares `[x0, x1] × [y0, y1]`.
    fn any_in(&self, (x0, y0): (i64, i64), (x1, y1): (i64, i64)) -> bool {
        (y0..=y1).any(|y| {
            (x0..=x1).any(|x| {
                self.blocked(EdgeAxis::Horizontal, x, y) || self.blocked(EdgeAxis::Vertical, x, y)
            })
        })
    }

    /// Whether the light at `p` reaches `q` (both in 1/16 squares, neither
    /// on a grid line): no blocking edge is crossed.
    fn sees(&self, p: (i64, i64), q: (i64, i64)) -> bool {
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        if dx != 0 {
            let (lo, hi) = (p.0.min(q.0), p.0.max(q.0));
            let mut k = lo.div_euclid(U) + 1;
            while k * U < hi {
                let (mut n, mut d) = (p.1 * dx + (k * U - p.0) * dy, dx);
                if d < 0 {
                    (n, d) = (-n, -d);
                }
                let hit = if n.rem_euclid(U * d) == 0 {
                    self.vertex_blocks(k, n / (U * d), dx, dy)
                } else {
                    self.blocked(EdgeAxis::Vertical, k, n.div_euclid(U * d))
                };
                if hit {
                    return false;
                }
                k += 1;
            }
        }
        if dy != 0 {
            let (lo, hi) = (p.1.min(q.1), p.1.max(q.1));
            let mut j = lo.div_euclid(U) + 1;
            while j * U < hi {
                let (mut n, mut d) = (p.0 * dy + (j * U - p.1) * dx, dy);
                if d < 0 {
                    (n, d) = (-n, -d);
                }
                // Vertices were handled on the vertical lines.
                if n.rem_euclid(U * d) != 0
                    && self.blocked(EdgeAxis::Horizontal, n.div_euclid(U * d), j)
                {
                    return false;
                }
                j += 1;
            }
        }
        true
    }

    /// A line of direction `(dx, dy)` through vertex `(vx, vy)` is blocked
    /// only when blocking arms lie on both sides of it.
    fn vertex_blocks(&self, vx: i64, vy: i64, dx: i64, dy: i64) -> bool {
        let mut sides = [false; 2];
        let arms = [
            (EdgeAxis::Vertical, vx, vy - 1, -dx),
            (EdgeAxis::Vertical, vx, vy, dx),
            (EdgeAxis::Horizontal, vx, vy, -dy),
            (EdgeAxis::Horizontal, vx - 1, vy, dy),
        ];
        for (axis, x, y, side) in arms {
            if self.blocked(axis, x, y) {
                if side > 0 {
                    sides[0] = true;
                } else if side < 0 {
                    sides[1] = true;
                }
            }
        }
        sides[0] && sides[1]
    }
}

/// A pool's visibility, 0–255 per cell, on the global cell grid.
pub struct LightMask {
    ox: i64,
    oy: i64,
    w: usize,
    h: usize,
    cells: Vec<u8>,
    ppsq: i64,
}

impl LightMask {
    /// Visibility at pixel `(px, py)`, 0–255, bilinear between cell centres.
    #[must_use]
    pub fn weight(&self, px: i64, py: i64) -> u32 {
        let d = 2 * self.ppsq;
        let t = |p: i64| (2 * p + 1) * CELLS - self.ppsq;
        let (tx, ty) = (t(px), t(py));
        let (cx, fx) = (tx.div_euclid(d), tx.rem_euclid(d));
        let (cy, fy) = (ty.div_euclid(d), ty.rem_euclid(d));
        let at = |x: i64, y: i64| {
            let xi = (x - self.ox).clamp(0, self.w as i64 - 1) as usize;
            let yi = (y - self.oy).clamp(0, self.h as i64 - 1) as usize;
            i64::from(self.cells[yi * self.w + xi])
        };
        let top = at(cx, cy) * (d - fx) + at(cx + 1, cy) * fx;
        let bottom = at(cx, cy + 1) * (d - fx) + at(cx + 1, cy + 1) * fx;
        let v = (top * (d - fy) + bottom * fy + d * d / 2) / (d * d);
        u32::try_from(v).unwrap_or(0)
    }
}

/// Moves a coordinate in 1/16 squares off a grid line, to its east/south.
fn off_line(v: i64) -> i64 {
    if v.rem_euclid(U) == 0 {
        v + 1
    } else {
        v
    }
}

/// The mask of `pool`, or `None` when its disc reaches no blocking edge.
#[must_use]
pub fn build(walls: &LightWalls, pool: &Pool, ppsq: u32) -> Option<LightMask> {
    let units = |v: f32| (v * U as f32 + 0.5).floor() as i64;
    let p = (off_line(units(pool.at.0)), off_line(units(pool.at.1)));
    let reach = (i64::from(pool.radius_ft) * U + 4) / 5;
    let sq = |v: i64| v.div_euclid(U);
    let lo = (sq(p.0 - reach) - 1, sq(p.1 - reach) - 1);
    let hi = (sq(p.0 + reach) + 1, sq(p.1 + reach) + 1);
    if !walls.any_in(lo, hi) {
        return None;
    }
    let (ox, oy) = (
        (p.0 - reach).div_euclid(2) - 3,
        (p.1 - reach).div_euclid(2) - 3,
    );
    let (w, h) = (
        usize::try_from((p.0 + reach).div_euclid(2) + 4 - ox).ok()?,
        usize::try_from((p.1 + reach).div_euclid(2) + 4 - oy).ok()?,
    );
    let far = (reach + 6) * (reach + 6);
    let mut cells = vec![255u8; w * h];
    cells.par_chunks_mut(w).enumerate().for_each(|(r, row)| {
        let qy = 2 * (oy + r as i64) + 1;
        for (c, v) in row.iter_mut().enumerate() {
            let q = (2 * (ox + c as i64) + 1, qy);
            let (ex, ey) = (q.0 - p.0, q.1 - p.1);
            if ex * ex + ey * ey <= far && !walls.sees(p, q) {
                *v = 0;
            }
        }
    });
    for _ in 0..2 {
        cells = blur3(&blur3(&cells, w, h, true), w, h, false);
    }
    Some(LightMask {
        ox,
        oy,
        w,
        h,
        cells,
        ppsq: i64::from(ppsq),
    })
}

/// One 3-tap box blur along rows (`horizontal`) or columns, edges clamped.
fn blur3(src: &[u8], w: usize, h: usize, horizontal: bool) -> Vec<u8> {
    let mut out = vec![0u8; src.len()];
    for y in 0..h {
        for x in 0..w {
            let (n, i) = if horizontal { (w, x) } else { (h, y) };
            let get = |k: usize| {
                let (xx, yy) = if horizontal { (k, y) } else { (x, k) };
                u32::from(src[yy * w + xx])
            };
            let s = get(i.saturating_sub(1)) + get(i) + get((i + 1).min(n - 1));
            out[y * w + x] = u8::try_from((s + 1) / 3).unwrap_or(255);
        }
    }
    out
}

/// Gives every pool that touches pixels `area` and a blocking edge its
/// mask. Pools wholly outside `area` are left alone: they paint nothing
/// there, and the mask of any other pool does not depend on `area`.
pub fn attach(
    pools: &mut [Pool],
    layout: &TacticalLayout,
    lib: &Library,
    ppsq: u32,
    area: [u32; 4],
) {
    if pools.is_empty() || layout.walls.is_empty() {
        return;
    }
    let walls = LightWalls::new(layout, lib);
    let [x0, y0, x1, y1] = area.map(i64::from);
    pools.par_iter_mut().for_each(|p| {
        let near = p.x + p.r > x0 && p.x - p.r < x1 && p.y + p.r > y0 && p.y - p.r < y1;
        if near {
            p.mask = build(&walls, p, ppsq);
        }
    });
}
