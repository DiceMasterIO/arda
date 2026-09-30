//! Label placement: a greedy candidate-position placer (Imhof's order of
//! preference) with halos, plus text along a curve for rivers.
//!
//! Labels are placed most important first. A point label tries eight
//! positions around its symbol (right, upper right, lower right, left,
//! upper left, lower left, above, below) and takes the first whose box
//! overlaps no placed label, no symbol and no page furniture; a label with
//! no free position is left out, unless it is forced (cities).

use super::font::{Fonts, Placed, Style};
use super::mask::Mask;
use crate::canvas::{Canvas, Rgb};

/// An axis-aligned box, pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    /// Left.
    pub x0: f32,
    /// Top.
    pub y0: f32,
    /// Right.
    pub x1: f32,
    /// Bottom.
    pub y1: f32,
}

impl Rect {
    /// A box from its corner and size.
    #[must_use]
    pub fn at(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            x0: x,
            y0: y,
            x1: x + w,
            y1: y + h,
        }
    }

    /// A square around a point.
    #[must_use]
    pub fn around(c: (f32, f32), r: f32) -> Self {
        Self::at(c.0 - r, c.1 - r, 2.0 * r, 2.0 * r)
    }

    fn hits(&self, o: &Self) -> bool {
        self.x0 < o.x1 && o.x0 < self.x1 && self.y0 < o.y1 && o.y0 < self.y1
    }
}

/// One line of a label.
#[derive(Debug, Clone)]
pub struct Line {
    /// Text.
    pub text: String,
    /// Look.
    pub style: Style,
    /// Colour.
    pub color: Rgb,
}

/// Something ready to composite: text masks over a halo.
#[derive(Debug, Clone)]
pub struct Ink {
    /// Halo coverage.
    pub halo: Mask,
    /// Text coverage and colour per line.
    pub text: Vec<(Mask, Rgb)>,
    /// Text opacity.
    pub opacity: f32,
}

/// Placed labels, symbols and furniture that later labels must avoid.
pub struct Placer<'f> {
    fonts: &'f Fonts,
    taken: Vec<Rect>,
    width: f32,
    height: f32,
    /// Halo radius, pixels.
    pub halo_r: f32,
}

/// Horizontal alignment of a label block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    Left,
    Centre,
    Right,
}

impl<'f> Placer<'f> {
    /// A placer over a `width × height` page.
    #[must_use]
    pub fn new(fonts: &'f Fonts, width: f32, height: f32, halo_r: f32) -> Self {
        Self {
            fonts,
            taken: Vec::new(),
            width,
            height,
            halo_r,
        }
    }

    /// Marks a box as occupied.
    pub fn reserve(&mut self, r: Rect) {
        self.taken.push(r);
    }

    /// Whether a box is on the page and clear.
    #[must_use]
    pub fn free(&self, r: &Rect) -> bool {
        r.x0 >= 0.0
            && r.y0 >= 0.0
            && r.x1 <= self.width
            && r.y1 <= self.height
            && !self.taken.iter().any(|t| t.hits(r))
    }

    /// Line heights (box height, baseline offset) and the block width.
    fn metrics(&self, lines: &[Line]) -> (Vec<(f32, f32)>, f32) {
        let mut out = Vec::new();
        let mut w: f32 = 0.0;
        for l in lines {
            let cap = self.fonts.cap_height(&l.style);
            let desc = self.fonts.descent(&l.style) * 0.55;
            out.push((cap + desc + l.style.size * 0.12, cap + l.style.size * 0.06));
            w = w.max(self.fonts.width(&l.style, &l.text));
        }
        (out, w)
    }

    /// Places a point label beside a symbol of radius `r` at `anchor`.
    pub fn point(
        &mut self,
        lines: &[Line],
        anchor: (f32, f32),
        r: f32,
        force: bool,
    ) -> Option<Ink> {
        let (m, w) = self.metrics(lines);
        let h: f32 = m.iter().map(|l| l.0).sum();
        let first = m.first().map_or(0.0, |l| l.0);
        let g = r + 2.0 + lines.first().map_or(0.0, |l| l.style.size * 0.12);
        let d = g * 0.72;
        let (ax, ay) = anchor;
        let mid = ay - first / 2.0;
        let candidates = [
            (ax + g, mid, Align::Left),
            (ax + d, ay - d - h, Align::Left),
            (ax + d, ay + d, Align::Left),
            (ax - g - w, mid, Align::Right),
            (ax - d - w, ay - d - h, Align::Right),
            (ax - d - w, ay + d, Align::Right),
            (ax - w / 2.0, ay - g - h, Align::Centre),
            (ax - w / 2.0, ay + g, Align::Centre),
        ];
        let pad = self.halo_r * 0.6;
        let boxed = |x: f32, y: f32| Rect::at(x - pad, y - pad, w + 2.0 * pad, h + 2.0 * pad);
        let pick = candidates
            .iter()
            .find(|&&(x, y, _)| self.free(&boxed(x, y)))
            .copied()
            .or(force.then_some(candidates[0]))?;
        let (x, y, align) = pick;
        self.reserve(boxed(x, y));
        Some(self.ink(lines, &m, x, y, w, align))
    }

    /// Places a label centred on `c` if it is free (or `force`).
    pub fn centred(&mut self, lines: &[Line], c: (f32, f32), force: bool) -> Option<Ink> {
        let (m, w) = self.metrics(lines);
        let h: f32 = m.iter().map(|l| l.0).sum();
        let (x, y) = (c.0 - w / 2.0, c.1 - h / 2.0);
        let r = Rect::at(x, y, w, h);
        if !force && !self.free(&r) {
            return None;
        }
        self.reserve(r);
        Some(self.ink(lines, &m, x, y, w, Align::Centre))
    }

    /// Box a centred label would take.
    #[must_use]
    pub fn centred_box(&self, lines: &[Line], c: (f32, f32)) -> Rect {
        let (m, w) = self.metrics(lines);
        let h: f32 = m.iter().map(|l| l.0).sum();
        Rect::at(c.0 - w / 2.0, c.1 - h / 2.0, w, h)
    }

    fn ink(&self, lines: &[Line], m: &[(f32, f32)], x: f32, y: f32, w: f32, a: Align) -> Ink {
        let h: f32 = m.iter().map(|l| l.0).sum();
        let pad = 4;
        let x0 = x.floor() as i64 - pad;
        let y0 = y.floor() as i64 - pad;
        let (bw, bh) = (w.ceil() as usize + 10, h.ceil() as usize + 10);
        let mut text = Vec::new();
        let mut union = Mask::new(x0, y0, bw, bh);
        let mut top = y;
        for (l, &(lh, base)) in lines.iter().zip(m) {
            let lw = self.fonts.width(&l.style, &l.text);
            let lx = match a {
                Align::Left => x,
                Align::Centre => x + (w - lw) / 2.0,
                Align::Right => x + w - lw,
            };
            let mut mask = Mask::new(x0, y0, bw, bh);
            self.fonts
                .draw(&mut mask, &l.style, &l.text, lx, top + base);
            for (u, &v) in union.a.iter_mut().zip(&mask.a) {
                *u = u.max(v);
            }
            text.push((mask, l.color));
            top += lh;
        }
        Ink {
            halo: union.dilate(self.halo_r),
            text,
            opacity: 1.0,
        }
    }

    /// Sets a label along a curve (a river), reading left to right and
    /// standing just above it, on the straightest free stretch near the
    /// middle. Returns `None` when no stretch is long, straight and free.
    pub fn along(&mut self, line: &Line, path: &[(f32, f32)], lift: f32) -> Option<Ink> {
        let (glyphs, len) = self.fonts.layout(&line.style, &line.text);
        let (cum, total) = cumulative(path);
        if total < len * 1.3 || glyphs.is_empty() {
            return None;
        }
        let steps = 24;
        let mut best: Option<(f32, f32, bool)> = None;
        for k in 0..=steps {
            let t = k as f32 / steps as f32;
            let start = (total - len) * (0.15 + 0.7 * t);
            let bend = bend(path, &cum, start, start + len);
            let off_mid = (t - 0.5).abs() * 0.4;
            let score = bend + off_mid;
            if bend > 0.9 || best.is_some_and(|b| b.0 <= score) {
                continue;
            }
            let a = at(path, &cum, start);
            let b = at(path, &cum, start + len);
            let flip = b.0 < a.0;
            if self.along_free(&glyphs, path, &cum, start, len, flip, lift, line.style.size) {
                best = Some((score, start, flip));
            }
        }
        let (_, start, flip) = best?;
        let size = line.style.size;
        let mut boxes = Vec::new();
        let (xs, ys): (Vec<f32>, Vec<f32>) = path.iter().copied().unzip();
        let x0 = xs.iter().copied().fold(f32::INFINITY, f32::min).floor() as i64 - 40;
        let y0 = ys.iter().copied().fold(f32::INFINITY, f32::min).floor() as i64 - 40;
        let x1 = xs.iter().copied().fold(f32::NEG_INFINITY, f32::max).ceil() as i64 + 40;
        let y1 = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max).ceil() as i64 + 40;
        let mut mask = Mask::new(
            x0,
            y0,
            usize::try_from(x1 - x0).unwrap_or(0),
            usize::try_from(y1 - y0).unwrap_or(0),
        );
        for g in &glyphs {
            let (c, ang) = glyph_frame(path, &cum, start, len, g, flip, lift);
            boxes.push(Rect::around(c, size * 0.45));
            self.fonts
                .glyph_rotated(&mut mask, line.style.face, g, c.0, c.1, ang, size * 0.3);
        }
        for b in boxes {
            self.reserve(b);
        }
        Some(Ink {
            halo: mask.dilate(self.halo_r * 0.8),
            text: vec![(mask, line.color)],
            opacity: 1.0,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn along_free(
        &self,
        glyphs: &[Placed],
        path: &[(f32, f32)],
        cum: &[f32],
        start: f32,
        len: f32,
        flip: bool,
        lift: f32,
        size: f32,
    ) -> bool {
        glyphs.iter().all(|g| {
            let (c, _) = glyph_frame(path, cum, start, len, g, flip, lift);
            self.free(&Rect::around(c, size * 0.45))
        })
    }
}

/// Composites inks onto the canvas: every halo first, then the text.
pub fn paint(c: &mut Canvas, inks: &[Ink], halo: Rgb, halo_opacity: f32) {
    for i in inks {
        c.paint(&i.halo, halo, halo_opacity * i.opacity);
    }
    for i in inks {
        for (m, col) in &i.text {
            c.paint(m, *col, i.opacity);
        }
    }
}

fn cumulative(path: &[(f32, f32)]) -> (Vec<f32>, f32) {
    let mut cum = vec![0.0];
    let mut t = 0.0;
    for s in path.windows(2) {
        t += ((s[1].0 - s[0].0).powi(2) + (s[1].1 - s[0].1).powi(2)).sqrt();
        cum.push(t);
    }
    (cum, t)
}

/// Point and tangent angle at arc length `s`.
fn at(path: &[(f32, f32)], cum: &[f32], s: f32) -> (f32, f32) {
    let k = cum
        .partition_point(|&c| c <= s)
        .clamp(1, path.len().max(2) - 1);
    let (a, b) = (path[k - 1], path[k]);
    let seg = (cum[k] - cum[k - 1]).max(1e-6);
    let t = ((s - cum[k - 1]) / seg).clamp(0.0, 1.0);
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// Total turning between arc lengths `s0` and `s1`, radians.
fn bend(path: &[(f32, f32)], cum: &[f32], s0: f32, s1: f32) -> f32 {
    let n = 8;
    let mut total = 0.0;
    let mut last: Option<f32> = None;
    for k in 0..=n {
        let s = s0 + (s1 - s0) * k as f32 / n as f32;
        let p = at(path, cum, (s - 3.0).max(0.0));
        let q = at(path, cum, s + 3.0);
        let ang = (q.1 - p.1).atan2(q.0 - p.0);
        if let Some(l) = last {
            let mut d = ang - l;
            while d > std::f32::consts::PI {
                d -= std::f32::consts::TAU;
            }
            while d < -std::f32::consts::PI {
                d += std::f32::consts::TAU;
            }
            total += d.abs();
        }
        last = Some(ang);
    }
    total
}

/// Centre and baseline angle of a glyph set along the path.
fn glyph_frame(
    path: &[(f32, f32)],
    cum: &[f32],
    start: f32,
    len: f32,
    g: &Placed,
    flip: bool,
    lift: f32,
) -> ((f32, f32), f32) {
    let mid = g.x + g.advance / 2.0;
    let s = if flip { start + len - mid } else { start + mid };
    let half = (g.advance / 2.0).max(2.0);
    let p = at(path, cum, (s - half).max(0.0));
    let q = at(path, cum, s + half);
    let (mut dx, mut dy) = (q.0 - p.0, q.1 - p.1);
    if flip {
        (dx, dy) = (-dx, -dy);
    }
    let ang = dy.atan2(dx);
    let c = at(path, cum, s);
    // Lift the glyph off the line, to its left-hand normal (above the
    // text's baseline).
    let n = (ang.sin(), -ang.cos());
    ((c.0 + n.0 * lift, c.1 + n.1 * lift), ang)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::font::Face;

    #[test]
    fn point_labels_never_overlap_and_curved_labels_follow_the_line() {
        let fonts = Fonts::new().unwrap();
        let mut pl = Placer::new(&fonts, 800.0, 600.0, 2.0);
        let style = Style::new(Face::Regular, 14.0);
        let mut placed = 0;
        for k in 0..120_u32 {
            let x = 40.0 + ((k * 97) % 720) as f32;
            let y = 40.0 + ((k * 61) % 520) as f32;
            pl.reserve(Rect::around((x, y), 3.0));
            let lines = [Line {
                text: format!("Place{k}"),
                style,
                color: [0, 0, 0],
            }];
            placed += usize::from(pl.point(&lines, (x, y), 3.0, false).is_some());
        }
        assert!(placed > 30, "only {placed} labels fit");
        for (i, a) in pl.taken.iter().enumerate() {
            for b in &pl.taken[i + 1..] {
                // Symbols may touch each other; labels never touch anything.
                let symbol = |r: &Rect| (r.x1 - r.x0 - 6.0).abs() < 1e-3;
                if !(symbol(a) && symbol(b)) {
                    assert!(!a.hits(b), "{a:?} overlaps {b:?}");
                }
            }
        }
        let mut pl = Placer::new(&fonts, 800.0, 600.0, 2.0);
        let path: Vec<(f32, f32)> = (0..=80).map(|k| (k as f32 * 10.0, 300.0)).collect();
        let line = Line {
            text: "Teyn".to_string(),
            style: Style::new(Face::Italic, 16.0),
            color: [0, 0, 255],
        };
        let ink = pl.along(&line, &path, 5.0).unwrap();
        let (m, _) = &ink.text[0];
        // The glyphs sit just above the line.
        let above: f32 = (280..300)
            .map(|y| (0..800).map(|x| m.get(x, y)).sum::<f32>())
            .sum();
        let below: f32 = (301..320)
            .map(|y| (0..800).map(|x| m.get(x, y)).sum::<f32>())
            .sum();
        assert!(above > 20.0 && below < above / 4.0, "{above} vs {below}");
    }
}
