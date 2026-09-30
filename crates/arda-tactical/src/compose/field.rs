//! Shared compositor plumbing: world coordinates, coarse noise fields and
//! per-square lookup grids.
//!
//! Low-frequency noise (warps, macro tints) is evaluated on a coarse grid
//! every [`STEP`] pixels and bilinearly interpolated, which is visually
//! identical at those feature sizes and far cheaper than per-pixel fractal
//! noise. Every sample position is in *world* squares (layout origin plus
//! local position), so adjacent blocks rendered separately join seamlessly.
// Pixel, grid and cell indices are bounded by the canvas size (at most
// 1024 ppsq × a few hundred squares), so these conversions cannot lose
// meaningful range.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss
)]

use crate::layout::{EdgeAxis, TacticalLayout};
use crate::noise::fbm;
use rayon::prelude::*;

/// Coarse-field spacing in output pixels.
pub const STEP: u32 = 8;
/// Spacing for the smoothest fields (warps, macro tints).
pub const WIDE_STEP: u32 = 16;

/// Where the canvas sits in the world, and at what resolution.
#[derive(Debug, Clone, Copy)]
pub struct Frame {
    /// Output pixels per square.
    pub ppsq: u32,
    /// World square of local square `(0, 0)`.
    pub origin: (i64, i64),
}

impl Frame {
    /// The frame of a layout at `ppsq`.
    #[must_use]
    pub fn of(layout: &TacticalLayout, ppsq: u32) -> Self {
        Self {
            ppsq,
            origin: layout.world_origin(),
        }
    }

    /// World pixel coordinates of local pixel `(px, py)`.
    #[must_use]
    pub fn world_px(&self, px: u32, py: u32) -> (i64, i64) {
        let p = i64::from(self.ppsq);
        (
            self.origin.0 * p + i64::from(px),
            self.origin.1 * p + i64::from(py),
        )
    }

    /// World position of a local pixel centre, in squares.
    #[must_use]
    // World positions stay far inside f32's exact-integer range for maps.
    #[allow(clippy::cast_precision_loss)]
    pub fn world_sq(&self, px: f32, py: f32) -> (f32, f32) {
        // Offset in whole world pixels first (exact in f32 for any map), so
        // every window computes bit-identical positions for a world point.
        let s = self.ppsq as f32;
        let p = i64::from(self.ppsq);
        (
            ((self.origin.0 * p) as f32 + px) / s,
            ((self.origin.1 * p) as f32 + py) / s,
        )
    }
}

/// Fractal noise re-centred on zero and stretched to roughly `[-1, 1]`.
#[must_use]
pub fn centred(seed: u64, x: f32, y: f32, octaves: u32) -> f32 {
    ((fbm(seed, x, y, octaves, None) - 0.5) * 4.0).clamp(-1.0, 1.0)
}

/// A scalar field sampled every [`STEP`] pixels over the canvas.
pub struct Field(Fields<1>);

impl Field {
    /// Evaluates `f(world_x, world_y)` (in squares) on the coarse grid.
    pub fn new(frame: &Frame, width: u32, height: u32, f: impl Fn(f32, f32) -> f32 + Sync) -> Self {
        Self(Fields::new(frame, width, height, STEP, |u, v| [f(u, v)]))
    }

    /// Bilinear sample at pixel `(px, py)` (pixel centres are fine).
    #[must_use]
    pub fn at(&self, px: f32, py: f32) -> f32 {
        self.0.at(px, py)[0]
    }
}

/// `N` scalar fields sampled together every `step` pixels: one bilinear
/// set-up serves all channels.
pub struct Fields<const N: usize> {
    gw: usize,
    gh: usize,
    step: f32,
    v: Vec<[f32; N]>,
}

impl<const N: usize> Fields<N> {
    /// Evaluates `f(world_x, world_y)` (in squares) every `step` pixels.
    pub fn new(
        frame: &Frame,
        width: u32,
        height: u32,
        step: u32,
        f: impl Fn(f32, f32) -> [f32; N] + Sync,
    ) -> Self {
        let step = step.max(1);
        let gw = (width / step + 2) as usize;
        let gh = (height / step + 2) as usize;
        let mut v = vec![[0.0; N]; gw * gh];
        v.par_chunks_mut(gw).enumerate().for_each(|(j, row)| {
            for (i, out) in row.iter_mut().enumerate() {
                let (px, py) = ((i as u32 * step) as f32, (j as u32 * step) as f32);
                let (u, w) = frame.world_sq(px, py);
                *out = f(u, w);
            }
        });
        Self {
            gw,
            gh,
            step: step as f32,
            v,
        }
    }

    /// Bilinear sample of every channel at pixel `(px, py)`.
    #[must_use]
    // Grid indices of in-canvas pixels.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn at(&self, px: f32, py: f32) -> [f32; N] {
        let (gx, gy) = (px / self.step, py / self.step);
        let (fx, fy) = (gx.floor(), gy.floor());
        let (i, j) = (
            (fx.max(0.0) as usize).min(self.gw - 2),
            (fy.max(0.0) as usize).min(self.gh - 2),
        );
        let (tx, ty) = (gx - fx, gy - fy);
        let r0 = j * self.gw + i;
        let r1 = r0 + self.gw;
        let (a, b, c, d) = (&self.v[r0], &self.v[r0 + 1], &self.v[r1], &self.v[r1 + 1]);
        let mut out = [0.0; N];
        for k in 0..N {
            let top = a[k] + (b[k] - a[k]) * tx;
            out[k] = top + (c[k] + (d[k] - c[k]) * tx - top) * ty;
        }
        out
    }
}

/// Wall edges as dense per-edge flags, for fast "is there a wall" checks.
pub struct WallGrid {
    w: i64,
    h: i64,
    horizontal: Vec<bool>,
    vertical: Vec<bool>,
    /// Squares with a wall on or next to them (their 3 × 3 neighbourhood).
    near: Vec<bool>,
}

impl WallGrid {
    /// Builds the grid from a layout's wall segments.
    #[must_use]
    pub fn new(layout: &TacticalLayout) -> Self {
        let (w, h) = (i64::from(layout.width), i64::from(layout.height));
        let n = usize::try_from((w + 1) * (h + 1)).unwrap_or(0);
        let mut horizontal = vec![false; n];
        let mut vertical = vec![false; n];
        let sq = usize::try_from(w * h).unwrap_or(0);
        let mut near = vec![false; sq];
        for seg in &layout.walls {
            let (x, y) = (i64::from(seg.x), i64::from(seg.y));
            let i = usize::try_from(y * (w + 1) + x).unwrap_or(0);
            match seg.axis {
                EdgeAxis::Horizontal => horizontal[i] = true,
                EdgeAxis::Vertical => vertical[i] = true,
            }
            for dy in -2..=1 {
                for dx in -2..=1 {
                    let (sx, sy) = (x + dx, y + dy);
                    if (0..w).contains(&sx) && (0..h).contains(&sy) {
                        near[usize::try_from(sy * w + sx).unwrap_or(0)] = true;
                    }
                }
            }
        }
        Self {
            w,
            h,
            horizontal,
            vertical,
            near,
        }
    }

    /// Whether square `(x, y)` has a wall nearby.
    #[must_use]
    pub fn near(&self, x: i64, y: i64) -> bool {
        if !(0..self.w).contains(&x) || !(0..self.h).contains(&y) {
            return false;
        }
        self.near[usize::try_from(y * self.w + x).unwrap_or(0)]
    }

    fn wall(&self, axis: EdgeAxis, x: i64, y: i64) -> bool {
        if x < 0 || y < 0 || x > self.w || y > self.h {
            return false;
        }
        let i = usize::try_from(y * (self.w + 1) + x).unwrap_or(0);
        match axis {
            EdgeAxis::Horizontal => self.horizontal[i],
            EdgeAxis::Vertical => self.vertical[i],
        }
    }

    fn path_blocked(&self, (ax, ay): (i64, i64), (bx, by): (i64, i64), x_first: bool) -> bool {
        let step_x = |row: i64| {
            let (lo, hi) = (ax.min(bx), ax.max(bx));
            (lo..hi).any(|c| self.wall(EdgeAxis::Vertical, c + 1, row))
        };
        let step_y = |col: i64| {
            let (lo, hi) = (ay.min(by), ay.max(by));
            (lo..hi).any(|r| self.wall(EdgeAxis::Horizontal, col, r + 1))
        };
        if x_first {
            step_x(ay) || step_y(bx)
        } else {
            step_y(ax) || step_x(by)
        }
    }

    /// Walls separate `a` from `b` when every simple walk between them
    /// crosses one, so floors stop cleanly at walls yet flow through gaps.
    #[must_use]
    pub fn separated(&self, a: (i64, i64), b: (i64, i64)) -> bool {
        a != b && self.path_blocked(a, b, true) && self.path_blocked(a, b, false)
    }
}

/// Smoothstep on `[0, 1]`.
#[must_use]
pub fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Bilinear weights of the four squares around a position in squares:
/// `[(x, y, weight); 4]` with coordinates clamped to the map.
#[must_use]
// Floors of bounded map positions.
#[allow(clippy::cast_possible_truncation)]
pub fn corners(layout: &TacticalLayout, wx: f32, wy: f32) -> [(i64, i64, f32); 4] {
    let (gx, gy) = (wx - 0.5, wy - 0.5);
    let (fx, fy) = (gx.floor(), gy.floor());
    let (tx, ty) = (smooth(gx - fx), smooth(gy - fy));
    let (x0, y0) = (fx as i64, fy as i64);
    let clamp = |x: i64, y: i64| {
        (
            x.clamp(0, i64::from(layout.width) - 1),
            y.clamp(0, i64::from(layout.height) - 1),
        )
    };
    let c = [
        clamp(x0, y0),
        clamp(x0 + 1, y0),
        clamp(x0, y0 + 1),
        clamp(x0 + 1, y0 + 1),
    ];
    [
        (c[0].0, c[0].1, (1.0 - tx) * (1.0 - ty)),
        (c[1].0, c[1].1, tx * (1.0 - ty)),
        (c[2].0, c[2].1, (1.0 - tx) * ty),
        (c[3].0, c[3].1, tx * ty),
    ]
}

/// [`corners`] for a position in *world* squares: the fractional weights
/// come from the world position, so every window computes them bit for
/// bit alike, and only the integer square indices are made local.
#[must_use]
pub fn corners_world(
    layout: &TacticalLayout,
    origin: (i64, i64),
    wx: f32,
    wy: f32,
) -> [(i64, i64, f32); 4] {
    let (gx, gy) = (wx - 0.5, wy - 0.5);
    let (fx, fy) = (gx.floor(), gy.floor());
    let (tx, ty) = (smooth(gx - fx), smooth(gy - fy));
    let (x0, y0) = (fx as i64 - origin.0, fy as i64 - origin.1);
    let clamp = |x: i64, y: i64| {
        (
            x.clamp(0, i64::from(layout.width) - 1),
            y.clamp(0, i64::from(layout.height) - 1),
        )
    };
    let c = [
        clamp(x0, y0),
        clamp(x0 + 1, y0),
        clamp(x0, y0 + 1),
        clamp(x0 + 1, y0 + 1),
    ];
    [
        (c[0].0, c[0].1, (1.0 - tx) * (1.0 - ty)),
        (c[1].0, c[1].1, tx * (1.0 - ty)),
        (c[2].0, c[2].1, (1.0 - tx) * ty),
        (c[3].0, c[3].1, tx * ty),
    ]
}

/// The local square containing a position in world squares, clamped.
#[must_use]
pub fn world_square(layout: &TacticalLayout, origin: (i64, i64), wx: f32, wy: f32) -> (i64, i64) {
    (
        (wx.floor() as i64 - origin.0).clamp(0, i64::from(layout.width) - 1),
        (wy.floor() as i64 - origin.1).clamp(0, i64::from(layout.height) - 1),
    )
}

/// The unwarped square containing `(u, v)` (local squares), clamped.
#[must_use]
// Floors of bounded map positions.
#[allow(clippy::cast_possible_truncation)]
pub fn own_square(layout: &TacticalLayout, u: f32, v: f32) -> (i64, i64) {
    (
        (u.floor() as i64).clamp(0, i64::from(layout.width) - 1),
        (v.floor() as i64).clamp(0, i64::from(layout.height) - 1),
    )
}

/// Rounds and clamps to a byte.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped first
pub fn byte(v: f32) -> u8 {
    (v + 0.5).clamp(0.0, 255.0) as u8
}
