//! Coverage masks: one alpha in 0–1 per pixel over a rectangle, with
//! anti-aliased strokes, discs and triangles drawn into them.
//!
//! Everything of one kind (all highway fills, one label's glyphs) is
//! drawn into a mask first and composited once, so overlapping strokes of
//! the same layer never darken where they join.

/// A coverage mask over `[x0, x0 + w) × [y0, y0 + h)`.
#[derive(Debug, Clone)]
pub struct Mask {
    /// Left edge, pixels.
    pub x0: i64,
    /// Top edge, pixels.
    pub y0: i64,
    /// Width, pixels.
    pub w: usize,
    /// Height, pixels.
    pub h: usize,
    /// Row-major coverage.
    pub a: Vec<f32>,
}

impl Mask {
    /// An empty mask.
    #[must_use]
    pub fn new(x0: i64, y0: i64, w: usize, h: usize) -> Self {
        Self {
            x0,
            y0,
            w,
            h,
            a: vec![0.0; w * h],
        }
    }

    fn index(&self, x: i64, y: i64) -> Option<usize> {
        let (lx, ly) = (x - self.x0, y - self.y0);
        if lx < 0 || ly < 0 {
            return None;
        }
        let (lx, ly) = (usize::try_from(lx).ok()?, usize::try_from(ly).ok()?);
        (lx < self.w && ly < self.h).then_some(ly * self.w + lx)
    }

    /// Raises a pixel's coverage to at least `c`.
    pub fn add(&mut self, x: i64, y: i64, c: f32) {
        if let Some(k) = self.index(x, y) {
            let v = &mut self.a[k];
            *v = v.max(c.min(1.0));
        }
    }

    /// Coverage at a pixel (0 outside).
    #[must_use]
    pub fn get(&self, x: i64, y: i64) -> f32 {
        self.index(x, y).map_or(0.0, |k| self.a[k])
    }

    /// Bilinear coverage at a point in pixel coordinates (pixel centres at
    /// `+0.5`).
    #[must_use]
    pub fn sample(&self, x: f32, y: f32) -> f32 {
        let (fx, fy) = (x - 0.5, y - 0.5);
        let (ix, iy) = (fx.floor() as i64, fy.floor() as i64);
        let (tx, ty) = (fx - fx.floor(), fy - fy.floor());
        let top = self.get(ix, iy) * (1.0 - tx) + self.get(ix + 1, iy) * tx;
        let bot = self.get(ix, iy + 1) * (1.0 - tx) + self.get(ix + 1, iy + 1) * tx;
        top * (1.0 - ty) + bot * ty
    }

    /// Clears every pixel.
    pub fn clear(&mut self) {
        self.a.fill(0.0);
    }

    /// This mask grown by `r` pixels with a soft edge (a halo).
    #[must_use]
    pub fn dilate(&self, r: f32) -> Self {
        let k = r.ceil() as i64 + 1;
        let ku = usize::try_from(k).unwrap_or(0);
        let mut out = Self::new(self.x0 - k, self.y0 - k, self.w + 2 * ku, self.h + 2 * ku);
        let offsets: Vec<(i64, i64, f32)> = (-k..=k)
            .flat_map(|dy| (-k..=k).map(move |dx| (dx, dy)))
            .filter_map(|(dx, dy)| {
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                let w = (r + 0.5 - d).clamp(0.0, 1.0);
                (w > 0.0).then_some((dx, dy, w))
            })
            .collect();
        for y in 0..self.h {
            for x in 0..self.w {
                let c = self.a[y * self.w + x];
                if c <= 0.0 {
                    continue;
                }
                let (px, py) = (self.x0 + x as i64, self.y0 + y as i64);
                for &(dx, dy, w) in &offsets {
                    out.add(px + dx, py + dy, c * w);
                }
            }
        }
        out
    }

    /// A round-capped stroke of width `w` from `p` to `q`.
    pub fn segment(&mut self, p: (f32, f32), q: (f32, f32), w: f32) {
        let r = w / 2.0;
        let x0 = (p.0.min(q.0) - r - 1.0).floor() as i64;
        let x1 = (p.0.max(q.0) + r + 1.0).ceil() as i64;
        let y0 = (p.1.min(q.1) - r - 1.0).floor() as i64;
        let y1 = (p.1.max(q.1) + r + 1.0).ceil() as i64;
        let (dx, dy) = (q.0 - p.0, q.1 - p.1);
        let len2 = (dx * dx + dy * dy).max(1e-6);
        for y in y0.max(self.y0)..=y1.min(self.y0 + self.h as i64 - 1) {
            for x in x0.max(self.x0)..=x1.min(self.x0 + self.w as i64 - 1) {
                let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
                let t = (((cx - p.0) * dx + (cy - p.1) * dy) / len2).clamp(0.0, 1.0);
                let (ex, ey) = (cx - p.0 - t * dx, cy - p.1 - t * dy);
                let d = (ex * ex + ey * ey).sqrt();
                let c = (r - d + 0.5).clamp(0.0, 1.0);
                if c > 0.0 {
                    self.add(x, y, c);
                }
            }
        }
    }

    /// A polyline of width `w`.
    pub fn polyline(&mut self, pts: &[(f32, f32)], w: f32) {
        for s in pts.windows(2) {
            self.segment(s[0], s[1], w);
        }
        if let [p] = pts {
            self.disc(*p, w / 2.0);
        }
    }

    /// A dashed polyline: `on` pixels drawn, `off` skipped, continuing the
    /// pattern from `phase` (updated).
    pub fn dashed(&mut self, pts: &[(f32, f32)], w: f32, on: f32, off: f32, phase: &mut f32) {
        let period = on + off;
        for s in pts.windows(2) {
            let (p, q) = (s[0], s[1]);
            let len = ((q.0 - p.0).powi(2) + (q.1 - p.1).powi(2)).sqrt();
            let mut t = 0.0;
            while t < len {
                let at = *phase % period;
                let (draw, left) = if at < on {
                    (true, on - at)
                } else {
                    (false, period - at)
                };
                let step = left.min(len - t).max(0.01);
                if draw {
                    let a = t / len.max(1e-6);
                    let b = (t + step) / len.max(1e-6);
                    self.segment(
                        (p.0 + (q.0 - p.0) * a, p.1 + (q.1 - p.1) * a),
                        (p.0 + (q.0 - p.0) * b, p.1 + (q.1 - p.1) * b),
                        w,
                    );
                }
                t += step;
                *phase += step;
            }
        }
    }

    /// A filled disc of radius `r`.
    pub fn disc(&mut self, c: (f32, f32), r: f32) {
        self.ring_between(c, 0.0, r);
    }

    /// A ring from radius `r0` to `r1`.
    pub fn ring_between(&mut self, c: (f32, f32), r0: f32, r1: f32) {
        let k = r1.ceil() as i64 + 1;
        let (bx, by) = (c.0.floor() as i64, c.1.floor() as i64);
        for y in by - k..=by + k {
            for x in bx - k..=bx + k {
                let d = ((x as f32 + 0.5 - c.0).powi(2) + (y as f32 + 0.5 - c.1).powi(2)).sqrt();
                let outer = (r1 - d + 0.5).clamp(0.0, 1.0);
                let inner = if r0 > 0.0 {
                    (d - r0 + 0.5).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                let cov = outer.min(inner);
                if cov > 0.0 {
                    self.add(x, y, cov);
                }
            }
        }
    }

    /// A filled triangle, anti-aliased on its edges.
    pub fn triangle(&mut self, a: (f32, f32), b: (f32, f32), c: (f32, f32)) {
        let x0 = a.0.min(b.0).min(c.0).floor() as i64 - 1;
        let x1 = a.0.max(b.0).max(c.0).ceil() as i64 + 1;
        let y0 = a.1.min(b.1).min(c.1).floor() as i64 - 1;
        let y1 = a.1.max(b.1).max(c.1).ceil() as i64 + 1;
        let area = (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0);
        let sign = if area < 0.0 { -1.0 } else { 1.0 };
        let edge = |p: (f32, f32), q: (f32, f32), x: f32, y: f32| {
            let (dx, dy) = (q.0 - p.0, q.1 - p.1);
            let len = (dx * dx + dy * dy).sqrt().max(1e-6);
            sign * (dx * (y - p.1) - dy * (x - p.0)) / len
        };
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let d = edge(a, b, px, py)
                    .min(edge(b, c, px, py))
                    .min(edge(c, a, px, py));
                let cov = (d + 0.5).clamp(0.0, 1.0);
                if cov > 0.0 {
                    self.add(x, y, cov);
                }
            }
        }
    }
}

/// Chaikin corner cutting, `n` rounds, keeping the end points.
#[must_use]
pub fn chaikin(pts: &[(f32, f32)], n: usize) -> Vec<(f32, f32)> {
    let mut cur = pts.to_vec();
    for _ in 0..n {
        if cur.len() < 3 {
            return cur;
        }
        let mut next = Vec::with_capacity(cur.len() * 2);
        next.push(cur[0]);
        for s in cur.windows(2) {
            let (p, q) = (s[0], s[1]);
            next.push((0.75 * p.0 + 0.25 * q.0, 0.75 * p.1 + 0.25 * q.1));
            next.push((0.25 * p.0 + 0.75 * q.0, 0.25 * p.1 + 0.75 * q.1));
        }
        if let Some(&last) = cur.last() {
            next.push(last);
        }
        cur = next;
    }
    cur
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strokes_are_soft_edged_and_do_not_stack() {
        let mut m = Mask::new(0, 0, 40, 40);
        m.segment((5.0, 20.0), (35.0, 20.0), 3.0);
        m.segment((20.0, 5.0), (20.0, 35.0), 3.0);
        assert!((m.get(20, 20) - 1.0).abs() < 1e-6);
        assert!(m.a.iter().any(|&c| c > 0.05 && c < 0.95));
        let halo = m.dilate(2.0);
        assert!(halo.get(22, 10) > 0.4 && m.get(22, 10) < 0.01);
        let mut d = Mask::new(0, 0, 60, 10);
        let mut phase = 0.0;
        d.dashed(&[(0.0, 5.0), (60.0, 5.0)], 2.0, 4.0, 4.0, &mut phase);
        assert!(d.get(1, 5) > 0.5 && d.get(6, 5) < 0.5);
        assert_eq!(
            chaikin(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)], 1).len(),
            6
        );
    }
}
