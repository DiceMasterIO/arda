//! Opt-in oblique look for Atlas overviews (goal 24: "a slightly oblique,
//! soft 3-D feel"). Defaults never reach this module.
//!
//! The overview is drawn as seen from a viewer tilted a little to the south.
//! Every point moves north on the image by `tilt · surface height`, with the
//! surface taken from a smooth 400 m field (sea and lake level are 0, so
//! coastlines never move). South-facing range flanks therefore stretch and
//! north-facing ones compress, while fine relief keeps its shape: it is only
//! translated. The warp is applied to the finished image rows, so the
//! shader, rivers and coasts are untouched and every pixel still comes from
//! the default render.
//!
//! Three gentle range-scale terms complete the depth cue, all bounded to a
//! few per cent so nothing turns plastic:
//! - aerial perspective: low ground and valleys deep below their
//!   surroundings sit under more air, so they take a cool blue-grey veil;
//! - valley occlusion: kilometre-scale hollows darken slightly, crests lift;
//! - sky light: range flanks turned away from the north-west sun take a
//!   faint blue fill, and high ground a faint warm cast.

use crate::RenderError;
use arda_core::{AreaCells, AreaCoord};

/// Q12 one.
pub(super) const ONE: i64 = 4_096;
/// Saved cells per field node (400 m; [`NODE_MM`] matches).
const NODE_CELLS: usize = 4;
/// Nodes per saved area.
const AREA_NODES: usize = 512 / NODE_CELLS;
/// Node spacing, millimetres.
const NODE_MM: i64 = 4 * 100_000;
/// Default tilt: northward shift per metre of surface height, Q12 (0.5,
/// a view about 27° off nadir).
pub const DEFAULT_TILT_Q12: i64 = 2_048;

/// The smooth surface fields that drive the oblique look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObliqueRelief {
    /// World extent in saved cells.
    cells: (u32, u32),
    /// Field nodes across and down.
    nodes: (usize, usize),
    /// Mean water-level surface per 400 m node, millimetres (never below 0).
    surface: Vec<i32>,
    /// Surface blurred to about 1.2 km (hollows and crests).
    hollow: Vec<i32>,
    /// Surface blurred to about 5 km (the ground valleys sit within).
    broad: Vec<i32>,
    /// Northward shift per unit height, Q12.
    tilt_q12: i64,
}

/// Collects saved areas into an [`ObliqueRelief`].
#[derive(Debug)]
pub struct ObliqueReliefBuilder {
    areas: (usize, usize),
    surface: Vec<i32>,
    filled: Vec<bool>,
}

impl ObliqueReliefBuilder {
    /// An empty builder for an `areas_wide × areas_high` world.
    ///
    /// # Errors
    /// Non-positive or oversized area counts.
    pub fn new(areas_wide: i32, areas_high: i32) -> Result<Self, RenderError> {
        let w = usize::try_from(areas_wide).map_err(|_| RenderError::ExactOverviewDimensions)?;
        let h = usize::try_from(areas_high).map_err(|_| RenderError::ExactOverviewDimensions)?;
        if w == 0 || h == 0 || w > 78 || h > 78 {
            return Err(RenderError::ExactOverviewDimensions);
        }
        let n = w * h * AREA_NODES * AREA_NODES;
        let mut surface = Vec::new();
        surface
            .try_reserve_exact(n)
            .map_err(|_| RenderError::ExactOverviewDimensions)?;
        surface.resize(n, 0);
        Ok(Self {
            areas: (w, h),
            surface,
            filled: vec![false; w * h],
        })
    }

    /// Adds one saved area: each node is the mean of its 4 × 4 cells'
    /// heights, clamped at sea level.
    ///
    /// # Errors
    /// An area outside the world.
    pub fn add_area(&mut self, at: AreaCoord, cells: &AreaCells) -> Result<(), RenderError> {
        let (ax, ay) = (
            usize::try_from(at.x).map_err(|_| RenderError::ExactOverviewDimensions)?,
            usize::try_from(at.y).map_err(|_| RenderError::ExactOverviewDimensions)?,
        );
        if ax >= self.areas.0 || ay >= self.areas.1 {
            return Err(RenderError::AtlasContext {
                reason: "oblique relief area outside the world",
            });
        }
        let row = self.areas.0 * AREA_NODES;
        let mut sums = vec![0_i64; AREA_NODES * AREA_NODES];
        for (i, cell) in cells.iter().enumerate() {
            let (x, y) = (i % 512, i / 512);
            sums[(y / NODE_CELLS) * AREA_NODES + x / NODE_CELLS] +=
                i64::from(cell.height.raw().max(0));
        }
        let per = i64::try_from(NODE_CELLS * NODE_CELLS).unwrap_or(16);
        for (k, sum) in sums.into_iter().enumerate() {
            let (nx, ny) = (
                ax * AREA_NODES + k % AREA_NODES,
                ay * AREA_NODES + k / AREA_NODES,
            );
            self.surface[ny * row + nx] = i32::try_from(sum / per).unwrap_or(i32::MAX);
        }
        self.filled[ay * self.areas.0 + ax] = true;
        Ok(())
    }

    /// The finished fields at `tilt_q12`.
    ///
    /// # Errors
    /// A missing area.
    pub fn finish(self, tilt_q12: i64) -> Result<ObliqueRelief, RenderError> {
        if self.filled.iter().any(|f| !f) {
            return Err(RenderError::AtlasContext {
                reason: "oblique relief is missing an area",
            });
        }
        let nodes = (self.areas.0 * AREA_NODES, self.areas.1 * AREA_NODES);
        let cells = (
            u32::try_from(self.areas.0 * 512).map_err(|_| RenderError::ExactOverviewDimensions)?,
            u32::try_from(self.areas.1 * 512).map_err(|_| RenderError::ExactOverviewDimensions)?,
        );
        Ok(ObliqueRelief::from_nodes(
            cells,
            nodes,
            self.surface,
            tilt_q12,
        ))
    }
}

/// Three passes of a clamped box blur of radius `r` (about a Gaussian).
fn blur(src: &[i32], (w, h): (usize, usize), r: usize) -> Vec<i32> {
    let mut a = src.to_vec();
    let mut b = vec![0_i32; a.len()];
    let pass = |from: &[i32], to: &mut [i32], horizontal: bool| {
        let (outer, inner) = if horizontal { (h, w) } else { (w, h) };
        for o in 0..outer {
            for i in 0..inner {
                let lo = i.saturating_sub(r);
                let hi = (i + r).min(inner - 1);
                let mut sum = 0_i64;
                for k in lo..=hi {
                    let idx = if horizontal { o * w + k } else { k * w + o };
                    sum += i64::from(from[idx]);
                }
                let n = i64::try_from(hi - lo + 1).unwrap_or(1);
                let idx = if horizontal { o * w + i } else { i * w + o };
                to[idx] = i32::try_from(sum / n).unwrap_or(0);
            }
        }
    };
    for _ in 0..3 {
        pass(&a, &mut b, true);
        pass(&b, &mut a, false);
    }
    a
}

/// Uniform cubic B-spline weights and their derivatives for `t` in Q16.
fn bspline(t: i64) -> ([i64; 4], [i64; 4]) {
    let one = 1_i64 << 16;
    let t2 = (t * t) >> 16;
    let t3 = (t2 * t) >> 16;
    let u = one - t;
    let u2 = (u * u) >> 16;
    let u3 = (u2 * u) >> 16;
    (
        [
            u3 / 6,
            (3 * t3 - 6 * t2 + 4 * one) / 6,
            (-3 * t3 + 3 * t2 + 3 * t + one) / 6,
            t3 / 6,
        ],
        [
            -u2 / 2,
            (3 * t2 - 4 * t) / 2,
            (-3 * t2 + 2 * t + one) / 2,
            t2 / 2,
        ],
    )
}

/// Smoothstep from `e0` to `e1` (either order), Q12.
pub(super) fn smooth(e0: i64, e1: i64, x: i64) -> i64 {
    let t = ((x - e0) * ONE / (e1 - e0)).clamp(0, ONE);
    t * t / ONE * (3 * ONE - 2 * t) / ONE
}

/// Field values at one point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Sample {
    /// Surface, mm.
    pub surface: i64,
    /// Hollow-scale surface, mm.
    pub hollow: i64,
    /// Broad surface, mm.
    pub broad: i64,
    /// Hollow-scale gradient `(dz/dx, dz/dy)`, Q12 slopes.
    pub gradient: (i64, i64),
}

impl ObliqueRelief {
    /// Fields from 400 m node surfaces (`cells` is the world extent in
    /// saved cells; `nodes` the node grid).
    #[must_use]
    pub fn from_nodes(
        cells: (u32, u32),
        nodes: (usize, usize),
        surface: Vec<i32>,
        tilt_q12: i64,
    ) -> Self {
        let hollow = blur(&surface, nodes, 1);
        let broad = blur(&surface, nodes, 4);
        Self {
            cells,
            nodes,
            surface,
            hollow,
            broad,
            tilt_q12: tilt_q12.clamp(0, 2 * ONE),
        }
    }

    /// Northward shift per unit surface height, Q12.
    #[must_use]
    pub const fn tilt_q12(&self) -> i64 {
        self.tilt_q12
    }

    /// World extent in saved cells.
    #[must_use]
    pub const fn cells(&self) -> (u32, u32) {
        self.cells
    }

    /// Highest node surface, millimetres.
    #[must_use]
    pub fn max_surface_mm(&self) -> i64 {
        self.surface.iter().copied().max().map_or(0, i64::from)
    }

    /// Spline-smoothed surface at a position in saved cells (Q16), mm.
    #[must_use]
    pub fn surface_mm(&self, x_cells_q16: i64, y_cells_q16: i64) -> i64 {
        self.sample(x_cells_q16, y_cells_q16).surface
    }

    /// How far north the oblique view draws the point at a position in
    /// saved cells (Q16), millimetres on the ground; 0 at sea level.
    #[must_use]
    pub fn shift_mm(&self, x_cells_q16: i64, y_cells_q16: i64) -> i64 {
        self.surface_mm(x_cells_q16, y_cells_q16) * self.tilt_q12 / ONE
    }

    /// All field values at a position in saved cells (Q16).
    pub(super) fn sample(&self, x_cells_q16: i64, y_cells_q16: i64) -> Sample {
        // Node i sits at the centre of its 4 × 4 block (cell 4i + 2).
        let half = i64::try_from(NODE_CELLS / 2).unwrap_or(2) << 16;
        let per = i64::try_from(NODE_CELLS).unwrap_or(4);
        let (tx, ty) = ((x_cells_q16 - half) / per, (y_cells_q16 - half) / per);
        let (ix, iy) = (tx >> 16, ty >> 16);
        let (wx, dwx) = bspline(tx & 0xFFFF);
        let (wy, dwy) = bspline(ty & 0xFFFF);
        let (nw, nh) = (
            i64::try_from(self.nodes.0).unwrap_or(1),
            i64::try_from(self.nodes.1).unwrap_or(1),
        );
        let mut acc = [0_i128; 5];
        for (j, (wyj, dwyj)) in wy.iter().zip(dwy).enumerate() {
            let y = (iy - 1 + i64::try_from(j).unwrap_or(0)).clamp(0, nh - 1);
            for (i, (wxi, dwxi)) in wx.iter().zip(dwx).enumerate() {
                let x = (ix - 1 + i64::try_from(i).unwrap_or(0)).clamp(0, nw - 1);
                let k = usize::try_from(y * nw + x).unwrap_or(0);
                let w = i128::from(wxi * wyj);
                let (s, h, b) = (
                    i128::from(self.surface[k]),
                    i128::from(self.hollow[k]),
                    i128::from(self.broad[k]),
                );
                acc[0] += w * s;
                acc[1] += w * h;
                acc[2] += w * b;
                acc[3] += i128::from(dwxi * wyj) * h;
                acc[4] += i128::from(wxi * dwyj) * h;
            }
        }
        let q32 = 1_i128 << 32;
        let v = |a: i128| i64::try_from(a / q32).unwrap_or(0);
        // d(mm)/d(node) over the node spacing, as a Q12 slope.
        let slope =
            |a: i128| i64::try_from(a * i128::from(ONE) / q32 / i128::from(NODE_MM)).unwrap_or(0);
        Sample {
            surface: v(acc[0]).max(0),
            hollow: v(acc[1]).max(0),
            broad: v(acc[2]).max(0),
            gradient: (slope(acc[3]), slope(acc[4])),
        }
    }
}

/// Cool veil of aerial perspective.
const HAZE: [i64; 3] = [140, 160, 172];

/// The oblique colour terms for one land or sea pixel at field sample `s`.
pub(super) fn shade(c: [i64; 3], s: &Sample) -> [i64; 3] {
    // Land weight: sea and the shore strip are left as drawn.
    let land = smooth(0, 15_000, s.surface);
    if land == 0 {
        return c;
    }
    let hollow = s.hollow - s.surface; // > 0 in kilometre-scale hollows
    let valley = s.broad - s.surface; // > 0 below the surrounding ground
                                      // Valley occlusion up to 9 %, crest lift up to 3 %.
    let k = ONE - 369 * smooth(0, 220_000, hollow) / ONE + 123 * smooth(0, 220_000, -hollow) / ONE;
    let k = ONE + (k - ONE) * land / ONE;
    let mut c = c.map(|v| v * k / ONE);
    // Warm high ground (up to ±3 %), cool sky fill on shaded flanks.
    let high = smooth(700_000, 2_500_000, s.surface) * land / ONE;
    let warm = [
        ONE + 123 * high / ONE,
        ONE + 41 * high / ONE,
        ONE - 123 * high / ONE,
    ];
    // Light from the north-west: a flank faces away when z falls toward
    // the north-west, that is when dz/dx + dz/dy is negative.
    let away = smooth(0, 1_434, -(s.gradient.0 + s.gradient.1)) * land / ONE;
    let sky = [ONE - 164 * away / ONE, ONE, ONE + 246 * away / ONE];
    c = std::array::from_fn(|ch| c[ch] * warm[ch] / ONE * sky[ch] / ONE);
    // Aerial perspective: up to 6 % in deep valleys plus up to 4 % on low
    // ground, fading out on the coast strip. More reads as blur.
    let veil = (246 * smooth(0, 1_200_000, valley) + 164 * (ONE - smooth(0, 1_500_000, s.surface)))
        / ONE
        * land
        / ONE;
    std::array::from_fn(|ch| c[ch] + (HAZE[ch] - c[ch]) * veil / ONE)
}

#[cfg(test)]
mod tests;
