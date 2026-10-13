//! Where a feature's label goes: a pole of inaccessibility for the anchor
//! and, for long thin features, a path down the middle.
//!
//! Only integer arithmetic and square roots, so the result is the same on
//! every platform.

#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

use super::coarse::{edt, Coarse};
use crate::num::{iu, ui};

/// Elongation (length over breadth) from which a feature gets a path.
const ELONGATED: f64 = 2.2;
/// A path never has more points than this.
pub const MAX_PATH: usize = 24;

/// The label geometry of one component, in coarse cells.
#[derive(Debug, Clone)]
pub struct Shape {
    /// The deepest cell (pole of inaccessibility).
    pub anchor: usize,
    /// Cells down the middle of a long feature, reading left to right.
    pub path: Option<Vec<usize>>,
    /// Extent along the main axis, coarse cells.
    pub length: u64,
    /// Bounding box, inclusive: `x0, y0, x1, y1`.
    pub bbox: [usize; 4],
}

/// Shape of the cells of `label` equal to `id`; `bbox` is its bounding box.
#[must_use]
pub fn shape(co: &Coarse, label: &[u32], id: u32, bbox: [usize; 4], min_path: u64) -> Shape {
    let [x0, y0, x1, y1] = bbox;
    let (cw, ch) = (x1 - x0 + 3, y1 - y0 + 3);
    let mut inside = vec![false; cw * ch];
    let mut cells: Vec<(i64, i64)> = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            if label[y * co.w + x] == id {
                inside[(y - y0 + 1) * cw + (x - x0 + 1)] = true;
                cells.push((ui(x), ui(y)));
            }
        }
    }
    let d = edt(cw, ch, &inside);
    let mut best = (0_i64, 0_usize);
    for (k, &v) in d.iter().enumerate() {
        if v > best.0 {
            best = (v, k);
        }
    }
    let anchor = (y0 + best.1 / cw - 1) * co.w + (x0 + best.1 % cw - 1);
    let (axis, length, elong) = principal(&cells);
    let path = (elong >= ELONGATED && length >= min_path)
        .then(|| centre_line(co, label, id, &cells, axis, length))
        .flatten();
    Shape {
        anchor,
        path,
        length,
        bbox,
    }
}

/// Main axis direction (scaled integers), extent along it and elongation.
fn principal(cells: &[(i64, i64)]) -> ((i64, i64), u64, f64) {
    let n = i64::try_from(cells.len()).unwrap_or(1).max(1);
    let (sx, sy) = cells.iter().fold((0, 0), |a, c| (a.0 + c.0, a.1 + c.1));
    let (mx, my) = (sx as f64 / n as f64, sy as f64 / n as f64);
    let (mut a, mut b, mut c) = (0.0_f64, 0.0_f64, 0.0_f64);
    for &(x, y) in cells {
        let (dx, dy) = (x as f64 - mx, y as f64 - my);
        a += dx * dx;
        b += dx * dy;
        c += dy * dy;
    }
    let (a, b, c) = (a / n as f64, b / n as f64, c / n as f64);
    let half = (a + c) / 2.0;
    let root = (((a - c) / 2.0).powi(2) + b * b).sqrt();
    let (l1, l2) = (half + root, (half - root).max(1e-9));
    let dir = if b.abs() < 1e-12 {
        if a >= c {
            (1.0, 0.0)
        } else {
            (0.0, 1.0)
        }
    } else {
        let (vx, vy) = (b, l1 - a);
        let norm = (vx * vx + vy * vy).sqrt();
        (vx / norm, vy / norm)
    };
    let axis = (
        (dir.0 * 1024.0).round() as i64,
        (dir.1 * 1024.0).round() as i64,
    );
    let proj = |c: &(i64, i64)| c.0 * axis.0 + c.1 * axis.1;
    let lo = cells.iter().map(proj).min().unwrap_or(0);
    let hi = cells.iter().map(proj).max().unwrap_or(0);
    let length = u64::try_from((hi - lo) / 1024 + 1).unwrap_or(1);
    (axis, length, (l1 / l2).sqrt())
}

/// Bin the cells along the axis and take each bin's centre, snapped onto
/// the feature.
fn centre_line(
    co: &Coarse,
    label: &[u32],
    id: u32,
    cells: &[(i64, i64)],
    axis: (i64, i64),
    length: u64,
) -> Option<Vec<usize>> {
    let proj = |c: &(i64, i64)| c.0 * axis.0 + c.1 * axis.1;
    let lo = cells.iter().map(proj).min()?;
    let bins = usize::try_from((length / 3).clamp(2, MAX_PATH as u64)).unwrap_or(2);
    let span = (length.max(1) * 1024).max(1);
    let span = i64::try_from(span).unwrap_or(1);
    let mut acc: Vec<(i64, i64, i64)> = vec![(0, 0, 0); bins];
    let bin_of = |c: &(i64, i64)| {
        let t = (proj(c) - lo) * ui(bins) / span;
        iu(t.clamp(0, ui(bins) - 1))
    };
    for c in cells {
        let a = &mut acc[bin_of(c)];
        *a = (a.0 + c.0, a.1 + c.1, a.2 + 1);
    }
    let mut pts: Vec<usize> = Vec::new();
    for (k, &(sx, sy, n)) in acc.iter().enumerate() {
        if n == 0 {
            continue;
        }
        let (cx, cy) = ((sx + n / 2) / n, (sy + n / 2) / n);
        let inside = |x: i64, y: i64| co.at(x, y).filter(|&i| label[i] == id);
        let pick = inside(cx, cy).or_else(|| {
            cells
                .iter()
                .filter(|c| bin_of(c) == k)
                .min_by_key(|c| ((c.0 - cx).pow(2) + (c.1 - cy).pow(2), c.1, c.0))
                .and_then(|c| inside(c.0, c.1))
        })?;
        if pts.last() != Some(&pick) {
            pts.push(pick);
        }
    }
    if pts.len() < 2 {
        return None;
    }
    let (first, last) = (pts[0], pts[pts.len() - 1]);
    if (first % co.w, first / co.w) > (last % co.w, last / co.w) {
        pts.reverse();
    }
    Some(pts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gazetteer::coarse::components;

    fn co(w: usize, h: usize) -> Coarse {
        Coarse {
            w,
            h,
            sea: vec![false; w * h],
            lake: vec![false; w * h],
            forest: vec![false; w * h],
            marsh: vec![false; w * h],
        }
    }

    #[test]
    fn a_long_bar_gets_a_path_inside_it_and_a_blob_does_not() {
        let (w, h) = (60, 30);
        let c = co(w, h);
        let mut bar = vec![false; w * h];
        for y in 10..14 {
            for x in 5..55 {
                bar[y * w + x] = true;
            }
        }
        let comps = components(w, h, &bar, false);
        let s = shape(&c, &comps.label, 1, comps.list[0].bbox, 6);
        let path = s.path.unwrap();
        assert!(path.len() >= 6 && path.len() <= MAX_PATH);
        assert!(path.iter().all(|&i| bar[i]));
        assert!(path.windows(2).all(|p| p[0] % w <= p[1] % w));
        assert!(bar[s.anchor]);
        let mut blob = vec![false; w * h];
        for y in 5..20 {
            for x in 20..36 {
                blob[y * w + x] = true;
            }
        }
        let comps = components(w, h, &blob, false);
        let s = shape(&c, &comps.label, 1, comps.list[0].bbox, 6);
        assert!(s.path.is_none());
        assert_eq!((s.anchor % w, s.anchor / w), (27, 12));
    }
}
