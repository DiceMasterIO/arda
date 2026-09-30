//! The field partition: a jittered lattice of sites, each claiming the
//! squares nearest to it under a metric stretched along the contours.
//!
//! The metric at a square comes from a smooth tensor field of the terrain
//! gradient, so parcels lengthen along the contours on slopes and follow a
//! regional grain on the flat ("field boundaries follow the contours").
//! Sites claim squares only within [`R_MAX`], which bounds every field to a
//! disc of [`FIELD_REACH`] squares around its site; that bound is what lets a
//! window see every field touching it in full.

use crate::geom::{h2, h3, s11, u01, Grid, Sq, SQUARE_M};
use crate::input::{LandUse, LandUseMap, Terrain};

/// Site lattice spacing in squares (112.5 m).
pub const SITE_SPACING: i64 = 72;
/// Largest metric distance at which a site still claims a square.
pub const R_MAX: f64 = 108.0;
/// Largest Euclidean distance from a site to any square of its field.
pub const FIELD_REACH: i64 = 135;
/// Tensor node spacing in squares.
const NODE: i64 = 8;
/// `1 / along²` with an along-contour stretch of 1.25.
const ALONG_INV2: f64 = 0.64;
/// `1 / across² − 1 / along²` with an across-contour squeeze of 0.8.
const ACROSS_EXTRA: f64 = 0.9225;
/// Slope (rise over run) below which the regional grain takes over.
const FLAT: f64 = 0.02;

/// A field site.
#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    /// Lattice cell and index within it.
    pub key: (i64, i64, u8),
    /// Position in square units.
    pub p: [f64; 2],
    /// Land use of the 100 m cell holding the site.
    pub class: Option<LandUse>,
    /// Unit vector along the contour at the site.
    pub along: [f64; 2],
    /// Terrain slope at the site (rise over run).
    pub slope: f64,
}

/// A symmetric 2 × 2 tensor `[xx, xy, yy]`.
pub type Tensor = [f64; 3];

/// Terrain gradient (rise over run) at world point `m` over ±8 m.
#[must_use]
pub fn gradient(terrain: &dyn Terrain, m: [f64; 2]) -> [f64; 2] {
    let d = 8.0;
    let t = |dx: f64, dy: f64| terrain.sample(m[0] + dx, m[1] + dy).height_m;
    [
        (t(d, 0.0) - t(-d, 0.0)) / (2.0 * d),
        (t(0.0, d) - t(0.0, -d)) / (2.0 * d),
    ]
}

/// The regional grain: a seed-chosen unit direction used on flat ground.
#[must_use]
pub fn grain(seed: u64) -> [f64; 2] {
    let h = h2(seed, 0x6A41, 0, 0);
    let v = [s11(h), 0.35 + 0.65 * u01(h >> 21)];
    let n = (v[0] * v[0] + v[1] * v[1]).sqrt();
    [v[0] / n, v[1] / n]
}

/// A tensor field sampled on a coarse global node lattice.
#[derive(Debug, Clone)]
pub struct TensorField {
    x0: i64,
    y0: i64,
    w: i64,
    h: i64,
    nodes: Vec<Tensor>,
}

impl TensorField {
    /// Samples nodes covering the square rectangle `x0..x1 × y0..y1`.
    #[must_use]
    pub fn new(terrain: &dyn Terrain, seed: u64, rect: (i64, i64, i64, i64)) -> Self {
        let g = grain(seed);
        // The across-contour (gradient) direction of the grain.
        let across = [-g[1], g[0]];
        let (nx0, ny0) = (rect.0.div_euclid(NODE) - 1, rect.1.div_euclid(NODE) - 1);
        let (nx1, ny1) = (rect.2.div_euclid(NODE) + 2, rect.3.div_euclid(NODE) + 2);
        let mut nodes = Vec::new();
        for ny in ny0..ny1 {
            for nx in nx0..nx1 {
                #[allow(clippy::cast_precision_loss)] // map coordinates
                let m = [(nx * NODE) as f64 * SQUARE_M, (ny * NODE) as f64 * SQUARE_M];
                let gr = gradient(terrain, m);
                let e2 = FLAT * FLAT;
                let n2 = gr[0] * gr[0] + gr[1] * gr[1] + e2;
                nodes.push([
                    (gr[0] * gr[0] + e2 * across[0] * across[0]) / n2,
                    (gr[0] * gr[1] + e2 * across[0] * across[1]) / n2,
                    (gr[1] * gr[1] + e2 * across[1] * across[1]) / n2,
                ]);
            }
        }
        Self {
            x0: nx0,
            y0: ny0,
            w: nx1 - nx0,
            h: ny1 - ny0,
            nodes,
        }
    }

    fn node(&self, x: i64, y: i64) -> Tensor {
        let cx = (x - self.x0).clamp(0, self.w - 1);
        let cy = (y - self.y0).clamp(0, self.h - 1);
        self.nodes
            .get(usize::try_from(cy * self.w + cx).unwrap_or(0))
            .copied()
            .unwrap_or([0.5, 0.0, 0.5])
    }

    /// The bilinearly interpolated tensor at a point in square units.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    pub fn at(&self, p: [f64; 2]) -> Tensor {
        let (fx, fy) = (p[0] / NODE as f64, p[1] / NODE as f64);
        let (ix, iy) = (fx.floor() as i64, fy.floor() as i64);
        let (tx, ty) = (fx - fx.floor(), fy - fy.floor());
        let (a, b) = (self.node(ix, iy), self.node(ix + 1, iy));
        let (c, d) = (self.node(ix, iy + 1), self.node(ix + 1, iy + 1));
        let mut out = [0.0; 3];
        for k in 0..3 {
            let top = a[k] + (b[k] - a[k]) * tx;
            let bot = c[k] + (d[k] - c[k]) * tx;
            out[k] = top + (bot - top) * ty;
        }
        out
    }
}

/// The squared metric distance of offset `d` under tensor `t`.
#[must_use]
pub fn metric2(t: Tensor, d: [f64; 2]) -> f64 {
    let e = d[0] * d[0] + d[1] * d[1];
    let q = t[0] * d[0] * d[0] + 2.0 * t[1] * d[0] * d[1] + t[2] * d[1] * d[1];
    ALONG_INV2 * e + ACROSS_EXTRA * q
}

/// The unit along-contour direction of a tensor (its minor eigenvector).
#[must_use]
pub fn along_of(t: Tensor) -> [f64; 2] {
    let (a, b, c) = (t[0], t[1], t[2]);
    let lam = 0.5 * (a + c) + (0.25 * (a - c) * (a - c) + b * b).sqrt();
    // Major eigenvector (the gradient direction).
    let v1 = [b, lam - a];
    let v2 = [lam - c, b];
    let n1 = v1[0] * v1[0] + v1[1] * v1[1];
    let n2 = v2[0] * v2[0] + v2[1] * v2[1];
    let (v, n) = if n1 >= n2 { (v1, n1) } else { (v2, n2) };
    if n < 1e-18 {
        return [1.0, 0.0];
    }
    let n = n.sqrt();
    [-v[1] / n, v[0] / n]
}

/// Sites of a lattice rectangle, bucketed per lattice cell.
#[derive(Debug, Clone)]
pub struct SiteIndex {
    lx0: i64,
    ly0: i64,
    lw: i64,
    lh: i64,
    cells: Vec<Vec<usize>>,
    /// Every site, in lattice order.
    pub sites: Vec<Site>,
}

impl SiteIndex {
    /// Generates the sites of every lattice cell overlapping `rect` ± 3 cells.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // lattice coordinates
    pub fn new(
        seed: u64,
        landuse: &dyn LandUseMap,
        terrain: &dyn Terrain,
        tf: &TensorField,
        rect: (i64, i64, i64, i64),
    ) -> Self {
        let lx0 = rect.0.div_euclid(SITE_SPACING) - 3;
        let ly0 = rect.1.div_euclid(SITE_SPACING) - 3;
        let lx1 = rect.2.div_euclid(SITE_SPACING) + 4;
        let ly1 = rect.3.div_euclid(SITE_SPACING) + 4;
        let s = SITE_SPACING as f64;
        let mut sites = Vec::new();
        let mut cells = Vec::new();
        for j in ly0..ly1 {
            for i in lx0..lx1 {
                let h = h2(seed, 0x517E, i, j);
                let count = if u01(h) < 0.22 { 2 } else { 1 };
                let mut here = Vec::new();
                for k in 0..count {
                    let hk = h3(seed, 0x517F, i, j, k);
                    let p = [
                        (i as f64 + 0.15 + 0.7 * u01(hk)) * s,
                        (j as f64 + 0.15 + 0.7 * u01(hk >> 26)) * s,
                    ];
                    let cell = Sq::containing(p).cell();
                    let m = [p[0] * SQUARE_M, p[1] * SQUARE_M];
                    let g = gradient(terrain, m);
                    here.push(sites.len());
                    sites.push(Site {
                        key: (i, j, u8::try_from(k).unwrap_or(0)),
                        p,
                        class: landuse.class_at(cell.0, cell.1),
                        along: along_of(tf.at(p)),
                        slope: (g[0] * g[0] + g[1] * g[1]).sqrt(),
                    });
                }
                cells.push(here);
            }
        }
        Self {
            lx0,
            ly0,
            lw: lx1 - lx0,
            lh: ly1 - ly0,
            cells,
            sites,
        }
    }

    /// The site nearest to square `s` under the metric, if within [`R_MAX`].
    #[must_use]
    pub fn nearest(&self, tf: &TensorField, s: Sq) -> Option<usize> {
        let p = s.centre();
        let t = tf.at(p);
        let (ci, cj) = (s.x.div_euclid(SITE_SPACING), s.y.div_euclid(SITE_SPACING));
        let mut best: Option<(f64, usize)> = None;
        for j in (cj - 2)..=(cj + 2) {
            for i in (ci - 2)..=(ci + 2) {
                let (li, lj) = (i - self.lx0, j - self.ly0);
                if li < 0 || lj < 0 || li >= self.lw || lj >= self.lh {
                    continue;
                }
                let Some(bucket) = usize::try_from(lj * self.lw + li)
                    .ok()
                    .and_then(|k| self.cells.get(k))
                else {
                    continue;
                };
                for &k in bucket {
                    let q = self.sites[k].p;
                    let d2 = metric2(t, [p[0] - q[0], p[1] - q[1]]);
                    if best.is_none_or(|(b, _)| d2 < b) {
                        best = Some((d2, k));
                    }
                }
            }
        }
        best.filter(|(d2, _)| *d2 <= R_MAX * R_MAX).map(|(_, k)| k)
    }
}

/// Labels the 4-connected components of equal values in `raw`, calling
/// `emit(value, squares)` for each in row-major order of first square.
pub fn components<T: Copy + PartialEq>(raw: &Grid<Option<T>>, mut emit: impl FnMut(T, &[Sq])) {
    let mut seen = Grid::new(raw.x0, raw.y0, raw.w, raw.h, false);
    let mut stack = Vec::new();
    let mut members = Vec::new();
    for s in raw.squares() {
        let Some(Some(v)) = raw.get(s).copied() else {
            continue;
        };
        if seen.get(s) == Some(&true) {
            continue;
        }
        members.clear();
        stack.push(s);
        seen.set(s, true);
        while let Some(c) = stack.pop() {
            members.push(c);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let n = c.offset(dx, dy);
                if seen.get(n) == Some(&false) && raw.get(n) == Some(&Some(v)) {
                    seen.set(n, true);
                    stack.push(n);
                }
            }
        }
        members.sort_unstable_by_key(|q| (q.y, q.x));
        emit(v, &members);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_metric_stretches_along_the_minor_direction() {
        // Gradient along x: contours run along y.
        let t = [1.0, 0.0, 0.0];
        assert!(metric2(t, [0.0, 10.0]) < metric2(t, [10.0, 0.0]));
        let a = along_of(t);
        assert!(a[0].abs() < 1e-9 && (a[1].abs() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn metric_distance_bounds_euclidean_reach() {
        for t in [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.5, 0.5, 0.5]] {
            for d in [[1.0, 0.0], [0.0, 1.0], [0.6, 0.8]] {
                let e = (metric2(t, d)).sqrt();
                assert!(1.0 / e <= f64::from(i32::try_from(FIELD_REACH).unwrap()) / R_MAX + 1e-9);
            }
        }
    }
}
