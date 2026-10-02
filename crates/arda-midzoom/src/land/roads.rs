//! Roads and lanes on relief tiles (logic/17 §land-roads): the centrelines
//! `arda-ways` plans for the tactical map (quadratic B-spline pieces with
//! switchbacks, the same junction snapping), drawn at their class widths
//! (logic/08 §roads: highway 5, road 4, track 2, footpath 1 squares).
//!
//! The tactical surface is every square whose centre lies within half the
//! width of the centreline; relief pixels are box-filtered over the same
//! band, so at one pixel per square they cover the same squares. Coarser
//! pixels keep a minimum drawn width per class with proportionally less
//! opacity, so lanes fade before they would turn into a mesh of lines.
//! Crossings are not re-planned: where the tactical layer straightens an
//! approach to a bridge the two may differ by a few metres.

use super::fields::{line_cover, q12};
use crate::fixed::ONE;
use arda_people::terrain::Surroundings;
use arda_ways::plan::{self, Window};
use arda_ways::standing::Standing;
use arda_ways::{Road, RoadClass, SQUARE_M};

/// Road classes drawn, lowest first (later ones win overlaps).
pub const CLASSES: [RoadClass; 4] = [
    RoadClass::Footpath,
    RoadClass::Track,
    RoadClass::Road,
    RoadClass::Highway,
];

/// Surface colours, seen from above: packed earth for lanes, gravel and
/// cobbles paler on the larger classes.
const SURFACE: [[i64; 3]; 4] = [
    [150, 132, 98],
    [156, 136, 100],
    [170, 154, 120],
    [178, 164, 132],
];

/// Smallest drawn half-width, pixels, and the pixel size (m) above which
/// the class is not drawn at all.
const MIN_HALF_PX: [f64; 4] = [0.30, 0.38, 0.55, 0.70];
const MAX_PIXEL_M: [f64; 4] = [8.0, 24.0, 64.0, 128.0];
/// Opacity of each class's surface: lanes are trodden earth, nearly the
/// ground's own colour.
const INK: [f64; 4] = [0.6, 0.8, 1.0, 1.0];

fn class_index(c: RoadClass) -> Option<usize> {
    CLASSES.iter().position(|&k| k == c)
}

/// Ground heights for switchback grading, from the same cells the
/// tactical ways layer reads.
struct Ground<'a>(&'a Surroundings);

impl arda_ways::Terrain for Ground<'_> {
    fn height_m(&self, x_m: f64, y_m: f64) -> f64 {
        self.0.height_m(x_m, y_m)
    }
}

/// Per-pixel road coverage of a window.
#[derive(Debug, Clone)]
pub struct RoadRaster {
    w: usize,
    h: usize,
    /// Q12 coverage per pixel per class index.
    cover: Vec<[u16; 4]>,
}

/// The pixel grid a raster covers: pixel `(px, py)` is centred at
/// `((ox + px + 0.5)·pixel_m, (oy + py + 0.5)·pixel_m)` world metres.
#[derive(Debug, Clone, Copy)]
pub struct PixelGrid {
    /// First pixel column and row (global).
    pub origin: (i64, i64),
    /// Pixels across and down.
    pub size: (usize, usize),
    /// Pixel edge, metres.
    pub pixel_m: f64,
}

impl PixelGrid {
    /// World metres of a pixel centre (either axis).
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // pixel indices are far below 2^52
    pub fn centre_m(&self, p: i64) -> f64 {
        (p as f64 + 0.5) * self.pixel_m
    }
}

impl RoadRaster {
    /// Rasterises the ways near the grid. `skip(gx, gy)` drops road squares
    /// a town's streets replace (arda-blocks `Streets::replace`).
    #[must_use]
    pub fn build(
        grid: PixelGrid,
        roads: &[Road],
        heights: &Surroundings,
        seed: u64,
        skip: &dyn Fn(i64, i64) -> bool,
    ) -> Self {
        let (w, h) = grid.size;
        let mut cover = vec![[0_u16; 4]; w * h];
        let px = grid.pixel_m;
        #[allow(clippy::cast_precision_loss)]
        let (x0, y0) = (grid.origin.0 as f64 * px, grid.origin.1 as f64 * px);
        #[allow(clippy::cast_precision_loss)]
        let (x1, y1) = (x0 + w as f64 * px, y0 + h as f64 * px);
        let sq = |m: f64| arda_ways::plan::square_of(m);
        let (gx0, gy0) = (sq(x0) - 8, sq(y0) - 8);
        let (gx1, gy1) = (sq(x1) + 8, sq(y1) + 8);
        let (Ok(ww), Ok(wh)) = (u32::try_from(gx1 - gx0), u32::try_from(gy1 - gy0)) else {
            return Self { w, h, cover };
        };
        #[allow(clippy::cast_precision_loss)]
        let Ok(win) = Window::new([gx0 as f64 * SQUARE_M, gy0 as f64 * SQUARE_M], ww, wh) else {
            return Self { w, h, cover };
        };
        let planned = plan::build(
            win,
            roads,
            &[],
            &Ground(heights),
            seed,
            &Standing::default(),
        );
        for way in &planned.ways {
            let Some(ci) = class_index(way.class) else {
                continue;
            };
            if px > MAX_PIXEL_M[ci] {
                continue;
            }
            let half = f64::from(way.spec.width_sq) / 2.0 * SQUARE_M;
            let drawn = half.max(MIN_HALF_PX[ci] * px);
            // Thin classes drawn wider than they are get proportionally
            // fainter, so their ink matches their true area.
            let opacity = (half / drawn).powf(0.6);
            let reach = drawn + px;
            // Every station (0.5 m apart): far pieces' switchbacks shift arc
            // lengths between windows, so no subsampling by arc length.
            for run in &way.dense.runs {
                let pts: Vec<[f64; 2]> = run.iter().map(|s| s.p).collect();
                for seg in pts.windows(2) {
                    let (a, b) = (seg[0], seg[1]);
                    let lo = |u: f64, v: f64, o: f64| ((u.min(v) - reach - o) / px).floor();
                    let hi = |u: f64, v: f64, o: f64| ((u.max(v) + reach - o) / px).ceil();
                    #[allow(clippy::cast_possible_truncation)]
                    let (i0, i1) = (lo(a[0], b[0], x0) as i64, hi(a[0], b[0], x0) as i64);
                    #[allow(clippy::cast_possible_truncation)]
                    let (j0, j1) = (lo(a[1], b[1], y0) as i64, hi(a[1], b[1], y0) as i64);
                    #[allow(clippy::cast_possible_wrap)]
                    let (wi, hi_) = (w as i64, h as i64);
                    for j in j0.max(0)..j1.min(hi_) {
                        for i in i0.max(0)..i1.min(wi) {
                            let p = [
                                grid.centre_m(grid.origin.0 + i),
                                grid.centre_m(grid.origin.1 + j),
                            ];
                            let d = seg_dist(p, a, b);
                            if d > reach {
                                continue;
                            }
                            if skip(sq(p[0]), sq(p[1])) {
                                continue;
                            }
                            let c = q12(line_cover(d, 2.0 * drawn, px) * opacity);
                            let k = usize::try_from(j).unwrap_or(0) * w
                                + usize::try_from(i).unwrap_or(0);
                            if let Some(cell) = cover.get_mut(k) {
                                let v = u16::try_from(c).unwrap_or(u16::MAX);
                                cell[ci] = cell[ci].max(v);
                            }
                        }
                    }
                }
            }
        }
        Self { w, h, cover }
    }

    /// Paints the roads over pixel `(px, py)` of the grid, lowest class
    /// first.
    #[must_use]
    pub fn paint(&self, rgb: [u8; 3], px: usize, py: usize) -> [u8; 3] {
        let Some(c) = self.at(px, py) else {
            return rgb;
        };
        let mut out = rgb;
        for (k, &v) in c.iter().enumerate() {
            if v > 0 {
                // Roads keep a little of the ground's light.
                let mut paint = SURFACE[k];
                for (p, &g) in paint.iter_mut().zip(&rgb) {
                    *p = (*p * 7 + i64::from(g) * 3) / 10;
                }
                #[allow(clippy::cast_possible_truncation)] // within [0, ONE]
                let alpha = (f64::from(v) * INK[k]) as i64;
                out = super::tone::mix(out, paint, alpha);
            }
        }
        out
    }

    /// The highest class covering at least half the pixel.
    #[must_use]
    pub fn class_at(&self, px: usize, py: usize) -> Option<RoadClass> {
        let c = self.at(px, py)?;
        (0..4)
            .rev()
            .find(|&k| i64::from(c[k]) >= ONE / 2)
            .map(|k| CLASSES[k])
    }

    fn at(&self, px: usize, py: usize) -> Option<&[u16; 4]> {
        (px < self.w && py < self.h)
            .then(|| self.cover.get(py * self.w + px))
            .flatten()
    }
}

/// Distance from `p` to the segment `a`–`b`.
fn seg_dist(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let v = [b[0] - a[0], b[1] - a[1]];
    let l2 = v[0] * v[0] + v[1] * v[1];
    let t = if l2 > 0.0 {
        (((p[0] - a[0]) * v[0] + (p[1] - a[1]) * v[1]) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p[0] - a[0] - t * v[0]).hypot(p[1] - a[1] - t * v[1])
}
