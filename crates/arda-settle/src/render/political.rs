//! The political layer (logic/16 §political-map): each realm's colour as
//! an inner band, strongest at its land border and fading inward, then the
//! border itself as a dark dash-dot line over a pale halo. No area fill.
//!
//! Everything is prepared once in the pixel space of the whole image (the
//! smoothed border polylines and a bucket index over them) and painted
//! onto any window of it: the band at a pixel is a function of that
//! pixel's position alone (its distance to the nearest border line and the
//! realm of the cell under it), and the line masks are placed in
//! whole-image coordinates with the dash pattern measured from each
//! polyline's start. So strips, bands and slippy tiles of one image join
//! without a seam (`tests/band.rs`).

use super::mask::{chaikin, Mask};
use super::palette::Palette;
use super::View;
use crate::canvas::{Canvas, Rgb};
use crate::output::RealmsFile;
use std::collections::BTreeSet;

/// Width of the inner band of the legend swatch at scale 1, pixels.
pub const BAND_PX: f32 = 11.0;
/// Width of the inner band on the map as a fraction of the image's long
/// edge (0.65 %: 27 px on a 4096 px world map, clearly a coloured edge).
pub const BAND_FRAC: f32 = 0.0065;
/// Narrowest map band, pixels.
pub const BAND_MIN_PX: f32 = 10.0;
/// Widest map band, pixels (so deep zooms do not flood the land).
pub const BAND_MAX_PX: f32 = 90.0;
/// Width of the pale halo under the border line at scale 1, pixels.
pub const HALO_PX: f32 = 5.6;
/// Width of the dash-dot border line at scale 1, pixels.
pub const LINE_PX: f32 = 1.7;
/// Colour of the halo under the border line.
pub const HALO: Rgb = [255, 250, 232];
/// Opacity of the halo.
pub const HALO_ALPHA: f32 = 0.6;
/// Colour of the dash-dot border line.
pub const LINE: Rgb = [44, 22, 34];
/// Opacity of the band at the border itself.
pub const BAND_ALPHA: f32 = 1.0;
/// Dash, gap, dot, gap at scale 1, pixels.
pub const DASH_DOT: [f32; 4] = [8.0, 3.0, 1.2, 3.0];

/// One smoothed border polyline in whole-image pixels.
#[derive(Debug, Clone)]
struct Border {
    pts: Vec<(f32, f32)>,
    bbox: [f32; 4],
}

/// Realm bands and border lines, ready to paint onto any window.
#[derive(Debug, Clone)]
pub struct PoliticalLayer {
    palette: Palette,
    borders: Vec<Border>,
    /// Every border segment `(p, q)`.
    segs: Vec<((f32, f32), (f32, f32))>,
    /// Bucket side, pixels (at least the band width).
    cell: f32,
    /// Buckets across and down.
    cols: usize,
    rows: usize,
    /// Segment indices per bucket.
    buckets: Vec<Vec<u32>>,
    /// Band width, pixels.
    band: f32,
    /// Stroke scale.
    s: f32,
}

impl PoliticalLayer {
    /// Prepares the layer for the whole image of `v` (its `fw × fh`) at
    /// stroke scale `v.s`.
    #[must_use]
    pub fn new(v: &View, realms: &RealmsFile, palette: Palette) -> Self {
        let whole = View {
            ox: 0.0,
            oy: 0.0,
            ..*v
        };
        let mut seen: BTreeSet<Vec<[i64; 2]>> = BTreeSet::new();
        let mut borders = Vec::new();
        for r in &realms.realms {
            for line in &r.borders {
                let mut key = line.clone();
                if key.first() > key.last() {
                    key.reverse();
                }
                if line.len() < 2 || !seen.insert(key.clone()) {
                    continue;
                }
                // Smooth the canonical direction, so a border shared by two
                // realms is the same line whichever lists it first.
                let pts: Vec<(f32, f32)> = key.iter().map(|&m| whole.px(m)).collect();
                let pts = chaikin(&pts, 3);
                let mut bbox = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
                for p in &pts {
                    bbox = [
                        bbox[0].min(p.0),
                        bbox[1].min(p.1),
                        bbox[2].max(p.0),
                        bbox[3].max(p.1),
                    ];
                }
                borders.push(Border { pts, bbox });
            }
        }
        let band = band_width(v);
        let cell = band.max(4.0);
        let cols = (v.fw / cell).ceil().max(1.0) as usize;
        let rows = (v.fh / cell).ceil().max(1.0) as usize;
        let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); cols * rows];
        let mut segs = Vec::new();
        for b in &borders {
            for w in b.pts.windows(2) {
                let k = u32::try_from(segs.len()).unwrap_or(u32::MAX);
                segs.push((w[0], w[1]));
                let bx = |x: f32| ((x / cell).floor().max(0.0) as usize).min(cols - 1);
                let by = |y: f32| ((y / cell).floor().max(0.0) as usize).min(rows - 1);
                for y in by(w[0].1.min(w[1].1))..=by(w[0].1.max(w[1].1)) {
                    for x in bx(w[0].0.min(w[1].0))..=bx(w[0].0.max(w[1].0)) {
                        buckets[y * cols + x].push(k);
                    }
                }
            }
        }
        Self {
            palette,
            borders,
            segs,
            cell,
            cols,
            rows,
            buckets,
            band,
            s: v.s,
        }
    }

    /// The realm colours this layer paints.
    #[must_use]
    pub const fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Distance from a whole-image point to the nearest border line, if it
    /// is within the band.
    fn distance(&self, x: f32, y: f32) -> Option<f32> {
        let bx = (x / self.cell).floor() as i64;
        let by = (y / self.cell).floor() as i64;
        let mut best = f32::INFINITY;
        for yy in by - 1..=by + 1 {
            for xx in bx - 1..=bx + 1 {
                if xx < 0 || yy < 0 || xx as usize >= self.cols || yy as usize >= self.rows {
                    continue;
                }
                for &k in &self.buckets[yy as usize * self.cols + xx as usize] {
                    let (p, q) = self.segs[k as usize];
                    let (dx, dy) = (q.0 - p.0, q.1 - p.1);
                    let len2 = (dx * dx + dy * dy).max(1e-6);
                    let t = (((x - p.0) * dx + (y - p.1) * dy) / len2).clamp(0.0, 1.0);
                    let (ex, ey) = (x - p.0 - t * dx, y - p.1 - t * dy);
                    best = best.min(ex * ex + ey * ey);
                }
            }
        }
        let d = best.sqrt();
        (d < self.band).then_some(d)
    }

    /// Paints every realm's inner band onto `c`, the window of the whole
    /// image that `v` places (`v.ox`, `v.oy`); `realm` is the realm id per
    /// grid cell (0 for sea and unclaimed land).
    pub fn paint_band(&self, c: &mut Canvas, v: &View, realm: &dyn Fn(usize) -> u16) {
        for py in 0..c.height {
            let y = py as f32 + v.oy + 0.5;
            for px in 0..c.width {
                let x = px as f32 + v.ox + 0.5;
                let Some(d) = self.distance(x, y) else {
                    continue;
                };
                let r = realm(v.cell_at(px, py));
                if r == 0 {
                    continue;
                }
                let a = band_alpha(d, self.band);
                c.blend(px as i64, py as i64, self.palette.colour(u64::from(r)), a);
            }
        }
    }

    /// A legend swatch of the border along the horizontal `seg`: realm
    /// `pair[0]`'s band above it, `pair[1]`'s below, both at most
    /// [`BAND_PX`]` · s / 2` deep (the swatch row is short), then the halo
    /// and the dash-dot line, all in the map's own constants.
    pub fn swatch(c: &mut Canvas, seg: [(f32, f32); 2], s: f32, pair: [Rgb; 2]) {
        let band = BAND_PX * s;
        let reach = band / 2.0;
        let (x0, x1, y) = (seg[0].0.min(seg[1].0), seg[0].0.max(seg[1].0), seg[0].1);
        for py in (y - reach).floor() as i64..=(y + reach).ceil() as i64 {
            let fy = py as f32 + 0.5;
            let d = (fy - y).abs();
            if d > reach {
                continue;
            }
            let col = if fy < y { pair[0] } else { pair[1] };
            for px in x0.floor() as i64..x1.ceil() as i64 {
                c.blend(px, py, col, band_alpha(d, band));
            }
        }
        let mut halo = c.mask();
        let mut dash = c.mask();
        halo.polyline(&seg, HALO_PX * s);
        dash_dot(&mut dash, &seg, LINE_PX * s, s);
        c.paint(&halo, HALO, HALO_ALPHA);
        c.paint(&dash, LINE, 1.0);
    }

    /// Paints the border lines (halo, then dash-dot) onto `c`, the window
    /// of the whole image starting at pixel `origin`.
    pub fn paint_lines(&self, c: &mut Canvas, origin: (i64, i64)) {
        let mut halo = Mask::new(origin.0, origin.1, c.width, c.height);
        let mut dash = Mask::new(origin.0, origin.1, c.width, c.height);
        let pad = HALO_PX * self.s + 2.0;
        let rect = [
            origin.0 as f32 - pad,
            origin.1 as f32 - pad,
            (origin.0 + c.width as i64) as f32 + pad,
            (origin.1 + c.height as i64) as f32 + pad,
        ];
        for b in &self.borders {
            if b.bbox[0] > rect[2]
                || b.bbox[2] < rect[0]
                || b.bbox[1] > rect[3]
                || b.bbox[3] < rect[1]
            {
                continue;
            }
            halo.polyline(&b.pts, HALO_PX * self.s);
            dash_dot(&mut dash, &b.pts, LINE_PX * self.s, self.s);
        }
        c.paint_at(&halo, HALO, HALO_ALPHA, origin);
        c.paint_at(&dash, LINE, 1.0, origin);
    }
}

/// Width in pixels of the map band for the whole image of `v`:
/// [`BAND_FRAC`] of its long edge between [`BAND_MIN_PX`] and
/// [`BAND_MAX_PX`]. It depends on the image (the zoom level's whole-world
/// size) and never on a tile's own size, so tiles of one level agree.
#[must_use]
pub fn band_width(v: &View) -> f32 {
    (BAND_FRAC * v.fw.max(v.fh)).clamp(BAND_MIN_PX, BAND_MAX_PX)
}

/// Opacity of the band `d` pixels inside a border, for a band `band`
/// pixels wide: [`BAND_ALPHA`] at the border fading to nothing as
/// `t^0.85`, `t = 1 − d / band` (kept to arithmetic as `t · (1 + 0.15(1 − t))`,
/// a fuller shoulder than a straight fade so the colour holds inward).
#[must_use]
pub fn band_alpha(d: f32, band: f32) -> f32 {
    let t = (1.0 - d / band).max(0.0);
    BAND_ALPHA * t * (1.0 + 0.15 * (1.0 - t))
}

/// A dash-dot line along `pts`: long dash, gap, dot, gap ([`DASH_DOT`] at
/// scale `s`), the pattern measured from the first point.
pub fn dash_dot(m: &mut Mask, pts: &[(f32, f32)], w: f32, s: f32) {
    let pat = DASH_DOT.map(|v| v * s);
    let total: f32 = pat.iter().sum();
    let mut at = 0.0_f32;
    for seg in pts.windows(2) {
        let (p, q) = (seg[0], seg[1]);
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        let len = (dx * dx + dy * dy).sqrt();
        let mut t = 0.0;
        while t < len {
            let mut u = at % total;
            let mut k = 0;
            while k < 3 && u >= pat[k] {
                u -= pat[k];
                k += 1;
            }
            let step = (pat[k] - u).min(len - t).max(0.01);
            if k % 2 == 0 {
                let f = |x: f32| (p.0 + dx * x / len, p.1 + dy * x / len);
                m.segment(f(t), f(t + step), w);
            }
            t += step;
            at += step;
        }
    }
}
