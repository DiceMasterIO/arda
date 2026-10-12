//! OI-05 measures: shape and level of large lakes.

use crate::coast::measure;
use crate::grid::{Grid, LAKE, LAND};
use crate::json::J;
use crate::trace::{trace, Loop};
use std::collections::BTreeMap;
use std::f64::consts::PI;

/// Lakes at or above this many cells (500 km2) are reported.
pub const MIN_CELLS: usize = 50_000;
/// Oriented-rectangle fill at or above this flags a rectangular basin.
pub const RECT_FILL_FLAG: f64 = 0.88;
/// Share of loop edges lying on area-boundary lines that flags dams.
pub const AREA_LINE_FLAG: f64 = 0.05;

/// Heights, rim minimum, bounding box and coordinate sums of one lake.
type Gather = (Vec<i32>, f64, [usize; 4], f64, f64);

/// Measurements of one lake.
pub struct LakeReport {
    pub cells: usize,
    pub cx: usize,
    pub cy: usize,
    pub bbox_fill: f64,
    pub rect_fill: f64,
    pub iso: f64,
    pub axis_ratio_5km: f64,
    pub area_line_share: f64,
    pub level_m: f64,
    pub rim_min_m: f64,
}

impl LakeReport {
    /// Rectangular by the pre-registered rule.
    pub fn rectangular(&self) -> bool {
        self.rect_fill >= RECT_FILL_FLAG
    }

    /// Sides coincide with area boundaries by the pre-registered rule.
    pub fn on_area_lines(&self) -> bool {
        self.area_line_share >= AREA_LINE_FLAG
    }

    /// JSON object.
    pub fn json(&self, rank: usize) -> J {
        J::obj()
            .with("rank", J::u(rank))
            .with("area_km2", J::F(self.cells as f64 / 100.0))
            .with("centre_xy", J::A(vec![J::u(self.cx), J::u(self.cy)]))
            .with(
                "area_xy",
                J::A(vec![J::u(self.cx / 512), J::u(self.cy / 512)]),
            )
            .with("bbox_fill", J::F(self.bbox_fill))
            .with("oriented_rect_fill", J::F(self.rect_fill))
            .with("perimeter2_over_4pi_area", J::F(self.iso))
            .with("axis_diag_ratio_5km", J::F(self.axis_ratio_5km))
            .with("edges_on_area_lines_share", J::F(self.area_line_share))
            .with("level_m", J::F(self.level_m))
            .with("rim_min_m", J::F(self.rim_min_m))
            .with("rim_minus_level_m", J::F(self.rim_min_m - self.level_m))
            .with("rectangular", J::B(self.rectangular()))
            .with("sides_on_area_lines", J::B(self.on_area_lines()))
    }
}

fn label(g: &Grid) -> (Vec<u32>, Vec<usize>) {
    let mut lab = vec![0_u32; g.w * g.h];
    let mut sizes = vec![0_usize];
    let mut stack = Vec::new();
    for s in 0..g.w * g.h {
        if g.kind[s] != LAKE || lab[s] != 0 {
            continue;
        }
        let id = u32::try_from(sizes.len()).unwrap_or(u32::MAX);
        lab[s] = id;
        stack.push(s);
        let mut n = 0;
        while let Some(i) = stack.pop() {
            n += 1;
            let (x, y) = (i % g.w, i / g.w);
            let mut push = |j: usize| {
                if g.kind[j] == LAKE && lab[j] == 0 {
                    lab[j] = id;
                    stack.push(j);
                }
            };
            if x > 0 {
                push(i - 1);
            }
            if x + 1 < g.w {
                push(i + 1);
            }
            if y > 0 {
                push(i - g.w);
            }
            if y + 1 < g.h {
                push(i + g.w);
            }
        }
        sizes.push(n);
    }
    (lab, sizes)
}

/// Smoothed (1 km chord) perimeter of a loop, in cells.
fn smooth_perimeter(p: &[(i32, i32)]) -> f64 {
    let n = p.len();
    let m = 10.min(n / 4).max(1);
    (0..n)
        .step_by(m)
        .map(|i| {
            let (a, b) = (p[i], p[(i + m) % n]);
            f64::from(a.0 - b.0).hypot(f64::from(a.1 - b.1))
        })
        .sum()
}

/// Fill of the smallest oriented rectangle (1 degree steps) around the points.
fn oriented_fill(p: &[(i32, i32)], area: f64) -> f64 {
    let mut best = f64::MAX;
    for a in 0..90 {
        let (s, c) = f64::from(a).to_radians().sin_cos();
        let (mut u0, mut u1, mut v0, mut v1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for &(x, y) in p {
            let (x, y) = (f64::from(x), f64::from(y));
            let (u, v) = (x * c + y * s, -x * s + y * c);
            u0 = u0.min(u);
            u1 = u1.max(u);
            v0 = v0.min(v);
            v1 = v1.max(v);
        }
        best = best.min((u1 - u0) * (v1 - v0));
    }
    area / best.max(1.0)
}

/// Reports every lake of at least `min_cells` cells, largest first.
pub fn lakes(g: &Grid, min_cells: usize) -> Vec<LakeReport> {
    let (lab, sizes) = label(g);
    let big: Vec<u32> = (1..sizes.len())
        .filter(|&i| sizes[i] >= min_cells)
        .map(|i| i as u32)
        .collect();
    if big.is_empty() {
        return Vec::new();
    }
    let mask: Vec<bool> = g.kind.iter().map(|&k| k == LAKE).collect();
    let loops = trace(g.w, g.h, &mask);
    let mut outer: BTreeMap<u32, &Loop> = BTreeMap::new();
    for l in &loops {
        let id = lab[l.cell.1 as usize * g.w + l.cell.0 as usize];
        if big.contains(&id) && outer.get(&id).is_none_or(|o| o.pts.len() < l.pts.len()) {
            outer.insert(id, l);
        }
    }
    let mut cells: BTreeMap<u32, Gather> = BTreeMap::new();
    for &id in &big {
        cells.insert(
            id,
            (
                Vec::new(),
                f64::MAX,
                [usize::MAX, usize::MAX, 0, 0],
                0.0,
                0.0,
            ),
        );
    }
    for (i, &id) in lab.iter().enumerate() {
        let Some(e) = cells.get_mut(&id) else {
            continue;
        };
        let (x, y) = (i % g.w, i / g.w);
        e.0.push(g.z[i]);
        e.2 = [e.2[0].min(x), e.2[1].min(y), e.2[2].max(x), e.2[3].max(y)];
        e.3 += x as f64;
        e.4 += y as f64;
        for (nx, ny) in [
            (x.wrapping_sub(1), y),
            (x + 1, y),
            (x, y.wrapping_sub(1)),
            (x, y + 1),
        ] {
            if nx < g.w && ny < g.h && g.kind[ny * g.w + nx] == LAND {
                e.1 = e.1.min(f64::from(g.z[ny * g.w + nx]));
            }
        }
    }
    let mut out = Vec::new();
    for (id, (mut zs, rim, bb, sx, sy)) in cells {
        let Some(l) = outer.get(&id) else { continue };
        let n = zs.len();
        zs.sort_unstable();
        let area = n as f64;
        let per = smooth_perimeter(&l.pts);
        let acc = measure(std::slice::from_ref(*l), 1);
        let on_lines = l
            .pts
            .iter()
            .enumerate()
            .filter(|(i, a)| {
                let b = l.pts[(i + 1) % l.pts.len()];
                (a.0 == b.0 && a.0 % 512 == 0) || (a.1 == b.1 && a.1 % 512 == 0)
            })
            .count();
        out.push(LakeReport {
            cells: n,
            cx: (sx / area) as usize,
            cy: (sy / area) as usize,
            bbox_fill: area / (((bb[2] - bb[0] + 1) * (bb[3] - bb[1] + 1)) as f64),
            rect_fill: oriented_fill(&l.pts, area),
            iso: per * per / (4.0 * PI * area),
            axis_ratio_5km: acc.ratio(1),
            area_line_share: on_lines as f64 / l.pts.len() as f64,
            level_m: f64::from(zs[n / 2]) / 1000.0,
            rim_min_m: if rim == f64::MAX {
                f64::NAN
            } else {
                rim / 1000.0
            },
        });
    }
    out.sort_by(|a, b| {
        b.cells
            .cmp(&a.cells)
            .then(a.cy.cmp(&b.cy))
            .then(a.cx.cmp(&b.cx))
    });
    out
}
