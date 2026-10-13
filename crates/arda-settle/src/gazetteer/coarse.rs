//! The coarse working raster of the feature finder: components, distances
//! and the masks they run on.
//!
//! One coarse cell is [`F`] × [`F`] world cells (400 m). Feature sizes are
//! kilometres, so the finder works here and a 50-million-cell world costs
//! three million.

use crate::grid::Grid;
use crate::num::{iu, ui};
use arda::{Cover, TerrainKind};

/// World cells along one coarse cell edge.
pub const F: usize = 4;
/// Hectares in one coarse cell.
pub const CELL_HA: u64 = (F * F) as u64;
/// Metres along one coarse cell edge.
pub const CELL_M: i64 = 400;
#[allow(clippy::cast_possible_wrap)]
const _: () = assert!(CELL_M == F as i64 * 100);
/// Canopy value from which a world cell counts as forest.
const FOREST_MIN: u8 = 150;

/// The masks one world yields.
pub struct Coarse {
    /// Columns.
    pub w: usize,
    /// Rows.
    pub h: usize,
    /// Mostly sea.
    pub sea: Vec<bool>,
    /// Mostly lake.
    pub lake: Vec<bool>,
    /// Mostly closed canopy on land.
    pub forest: Vec<bool>,
    /// Mostly marsh cover on land.
    pub marsh: Vec<bool>,
}

impl Coarse {
    /// Summarises `g`: a mask is set where at least half of the block is.
    #[must_use]
    pub fn of(g: &Grid) -> Self {
        let (w, h) = (g.width.div_ceil(F), g.height.div_ceil(F));
        let n = w * h;
        let mut c = Self {
            w,
            h,
            sea: vec![false; n],
            lake: vec![false; n],
            forest: vec![false; n],
            marsh: vec![false; n],
        };
        for cy in 0..h {
            for cx in 0..w {
                let mut t = [0_usize; 5];
                let mut cells = 0;
                for y in cy * F..((cy + 1) * F).min(g.height) {
                    for x in cx * F..((cx + 1) * F).min(g.width) {
                        let i = y * g.width + x;
                        cells += 1;
                        match g.terrain[i] {
                            TerrainKind::Sea => t[0] += 1,
                            TerrainKind::Lake => t[1] += 1,
                            _ => {
                                t[2] += usize::from(
                                    g.cover[i] == Cover::Forest || g.forest[i] >= FOREST_MIN,
                                );
                                t[3] += usize::from(g.cover[i] == Cover::Marsh);
                            }
                        }
                    }
                }
                let half = |v: usize| v * 2 >= cells;
                let k = cy * w + cx;
                c.sea[k] = half(t[0]);
                c.lake[k] = !c.sea[k] && half(t[1]);
                c.forest[k] = half(t[2]);
                c.marsh[k] = half(t[3]);
            }
        }
        c
    }

    /// Row-major index of a coarse cell.
    #[must_use]
    pub fn at(&self, x: i64, y: i64) -> Option<usize> {
        if x < 0 || y < 0 || x >= ui(self.w) || y >= ui(self.h) {
            None
        } else {
            Some(iu(y) * self.w + iu(x))
        }
    }

    /// Centre of a coarse cell in metres.
    #[must_use]
    pub fn centre_m(&self, i: usize) -> [i64; 2] {
        let (x, y) = (ui(i % self.w), ui(i / self.w));
        [x * CELL_M + CELL_M / 2, y * CELL_M + CELL_M / 2]
    }
}

/// One connected component.
#[derive(Debug, Clone)]
pub struct Comp {
    /// Cells in it.
    pub count: u64,
    /// Bounding box in coarse cells, inclusive: `x0, y0, x1, y1`.
    pub bbox: [usize; 4],
    /// First cell in scan order.
    pub first: usize,
}

/// Components of a mask, ids from 1 (0 is outside every component).
pub struct Comps {
    /// Component id per cell.
    pub label: Vec<u32>,
    /// Components by `id - 1`.
    pub list: Vec<Comp>,
}

const N4: [(i64, i64); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
const N8: [(i64, i64); 8] = [
    (1, 0),
    (0, 1),
    (-1, 0),
    (0, -1),
    (1, 1),
    (-1, 1),
    (-1, -1),
    (1, -1),
];

/// Connected components of `mask` (4- or 8-connected), numbered in scan
/// order of their first cell.
#[must_use]
pub fn components(w: usize, h: usize, mask: &[bool], eight: bool) -> Comps {
    let mut label = vec![0_u32; w * h];
    let mut list: Vec<Comp> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let offsets: &[(i64, i64)] = if eight { &N8 } else { &N4 };
    for start in 0..w * h {
        if !mask[start] || label[start] != 0 {
            continue;
        }
        let id = u32::try_from(list.len() + 1).unwrap_or(u32::MAX);
        label[start] = id;
        stack.push(start);
        let mut comp = Comp {
            count: 0,
            bbox: [w, h, 0, 0],
            first: start,
        };
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            comp.count += 1;
            comp.bbox = [
                comp.bbox[0].min(x),
                comp.bbox[1].min(y),
                comp.bbox[2].max(x),
                comp.bbox[3].max(y),
            ];
            for &(dx, dy) in offsets {
                let (nx, ny) = (ui(x) + dx, ui(y) + dy);
                if nx < 0 || ny < 0 || nx >= ui(w) || ny >= ui(h) {
                    continue;
                }
                let j = iu(ny) * w + iu(nx);
                if mask[j] && label[j] == 0 {
                    label[j] = id;
                    stack.push(j);
                }
            }
        }
        list.push(comp);
    }
    Comps { label, list }
}

/// One-dimensional squared distance transform (Felzenszwalb and
/// Huttenlocher) of `f` in place.
fn edt_line(f: &mut [i64], v: &mut Vec<usize>, z: &mut Vec<i64>, out: &mut [i64]) {
    let n = f.len();
    v.clear();
    z.clear();
    v.push(0);
    z.push(i64::MIN);
    z.push(i64::MAX);
    let inter = |f: &[i64], q: usize, p: usize| {
        let (q_, p_) = (ui(q), ui(p));
        ((f[q] + q_ * q_) - (f[p] + p_ * p_)) / (2 * (q_ - p_))
    };
    for q in 1..n {
        let mut s = inter(f, q, v[v.len() - 1]);
        while s <= z[z.len() - 2] {
            v.pop();
            z.pop();
            s = inter(f, q, v[v.len() - 1]);
        }
        let top = z.len() - 1;
        z[top] = s;
        v.push(q);
        z.push(i64::MAX);
    }
    let mut k = 0;
    for (q, o) in out.iter_mut().enumerate() {
        while z[k + 1] < ui(q) {
            k += 1;
        }
        let d = ui(q) - ui(v[k]);
        *o = d * d + f[v[k]];
    }
}

/// Large enough to stand for "no boundary anywhere".
const FAR: i64 = 1 << 40;

/// Squared distance, in cells, from every `true` cell of `mask` to the
/// nearest `false` cell; `false` cells are 0. The map edge is not a
/// boundary: where no `false` cell exists the distance is [`FAR`].
#[must_use]
pub fn edt(w: usize, h: usize, mask: &[bool]) -> Vec<i64> {
    let mut d: Vec<i64> = mask.iter().map(|&m| if m { FAR } else { 0 }).collect();
    let (mut v, mut z) = (Vec::new(), Vec::new());
    let mut col = vec![0_i64; h];
    let mut out = vec![0_i64; h];
    for x in 0..w {
        for y in 0..h {
            col[y] = d[y * w + x];
        }
        edt_line(&mut col, &mut v, &mut z, &mut out);
        for y in 0..h {
            d[y * w + x] = out[y].min(FAR);
        }
    }
    let mut row = vec![0_i64; w];
    let mut out = vec![0_i64; w];
    for y in 0..h {
        row.copy_from_slice(&d[y * w..(y + 1) * w]);
        edt_line(&mut row, &mut v, &mut z, &mut out);
        for x in 0..w {
            d[y * w + x] = out[x].min(FAR);
        }
    }
    d
}

/// Whether a squared distance means "no boundary reached".
#[must_use]
pub const fn is_far(d: i64) -> bool {
    d >= FAR / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distances_are_exact_euclidean() {
        let (w, h) = (9, 7);
        let mut mask = vec![true; w * h];
        mask[3 * w + 4] = false;
        let d = edt(w, h, &mask);
        assert_eq!(d[3 * w + 4], 0);
        assert_eq!(d[3 * w + 6], 4);
        assert_eq!(d[5 * w + 5], 5);
        assert_eq!(d[0], 16 + 9);
    }

    #[test]
    fn components_number_in_scan_order() {
        let (w, h) = (6, 3);
        let mask: Vec<bool> = "##..##\n#...#.\n.#...."
            .chars()
            .filter(|c| *c != '\n')
            .map(|c| c == '#')
            .collect();
        let four = components(w, h, &mask, false);
        assert_eq!(four.list.len(), 3);
        assert_eq!(four.list[0].count, 3);
        let eight = components(w, h, &mask, true);
        assert_eq!(eight.list.len(), 2);
    }
}
