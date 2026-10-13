//! Mountain ranges: summits close together and joined along a ridge.
//!
//! Two summits are linked when they lie within `link_m` of each other and
//! the ground between them never drops more than [`SADDLE_MM`] below the
//! lower of the two. Linked summits form a range of at least `min_peaks`;
//! each peak keeps its own name and is named by `features::peaks`.

use super::shape::MAX_PATH;
use super::{Found, Thresholds};
use crate::features::Peak;
use crate::grid::{Grid, CELL_M};
use arda_names::PlaceKind;

/// Deepest saddle between two summits of one range, millimetres.
const SADDLE_MM: i32 = 1_300_000;
/// Cells between height samples along a link.
const STEP: i64 = 5;

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// Whether the ridge between two summits holds up.
fn linked(g: &Grid, a: &Peak, b: &Peak, link_cells: i64) -> bool {
    let ((ax, ay), (bx, by)) = (g.xy(a.cell), g.xy(b.cell));
    let (dx, dy) = (bx - ax, by - ay);
    if dx * dx + dy * dy > link_cells * link_cells {
        return false;
    }
    let steps = ((dx.abs().max(dy.abs())) / STEP).max(1);
    let floor = g.height_mm[a.cell].min(g.height_mm[b.cell]) - SADDLE_MM;
    (1..steps).all(|k| {
        g.at(ax + dx * k / steps, ay + dy * k / steps)
            .is_some_and(|i| g.height_mm[i] >= floor)
    })
}

/// Ranges of `peaks`, biggest first.
#[must_use]
pub fn ranges(g: &Grid, peaks: &[Peak], t: &Thresholds) -> Vec<Found> {
    let link_cells = t.range_link_m / CELL_M;
    let mut parent: Vec<usize> = (0..peaks.len()).collect();
    for i in 0..peaks.len() {
        for j in i + 1..peaks.len() {
            if linked(g, &peaks[i], &peaks[j], link_cells) {
                let (a, b) = (find(&mut parent, i), find(&mut parent, j));
                parent[a.max(b)] = a.min(b);
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); peaks.len()];
    for i in 0..peaks.len() {
        let r = find(&mut parent, i);
        groups[r].push(i);
    }
    let mut out: Vec<(usize, Found)> = groups
        .into_iter()
        .filter(|m| m.len() >= t.range_min_peaks)
        .map(|m| (m.len(), range(g, peaks, &m)))
        .collect();
    out.sort_by_key(|(n, f)| (std::cmp::Reverse(*n), f.anchor_m));
    out.truncate(t.max_ranges);
    out.into_iter().map(|(_, f)| f).collect()
}

/// One range from the indices of its summits.
fn range(g: &Grid, peaks: &[Peak], members: &[usize]) -> Found {
    let pos: Vec<(i64, i64)> = members.iter().map(|&k| g.xy(peaks[k].cell)).collect();
    // Order along the line between the two summits farthest apart.
    let d2 = |a: (i64, i64), b: (i64, i64)| (a.0 - b.0).pow(2) + (a.1 - b.1).pow(2);
    let mut far = (0, pos[0], pos[0]);
    for &a in &pos {
        for &b in &pos {
            if a < b && (d2(a, b), a, b) > far {
                far = (d2(a, b), a, b);
            }
        }
    }
    let (from, to) = (far.1, far.2);
    let axis = (to.0 - from.0, to.1 - from.1);
    let mut line: Vec<(i64, i64)> = pos.clone();
    line.sort_by_key(|p| ((p.0 - from.0) * axis.0 + (p.1 - from.1) * axis.1, p.1, p.0));
    line.dedup();
    if line.len() > MAX_PATH {
        let n = line.len();
        line = (0..MAX_PATH)
            .map(|k| line[k * (n - 1) / (MAX_PATH - 1)])
            .collect();
        line.dedup();
    }
    let path_m: Vec<[i64; 2]> = line
        .iter()
        .map(|&(x, y)| [x * CELL_M + CELL_M / 2, y * CELL_M + CELL_M / 2])
        .collect();
    let length_m = line
        .windows(2)
        .map(|w| u64::from(crate::num::dist_m(w[1].0 - w[0].0, w[1].1 - w[0].1)))
        .sum();
    let pad = 2_000;
    let xs = path_m.iter().map(|p| p[0]);
    let ys = path_m.iter().map(|p| p[1]);
    let extent_m = [
        xs.clone().min().unwrap_or(0) - pad,
        ys.clone().min().unwrap_or(0) - pad,
        xs.max().unwrap_or(0) + pad,
        ys.max().unwrap_or(0) + pad,
    ];
    let anchor_m = path_m[path_m.len() / 2];
    Found {
        kind: PlaceKind::Mountain,
        anchor_m,
        path_m: (path_m.len() >= 2).then_some(path_m),
        extent_m,
        area_ha: 0,
        length_m,
        detail: u32::try_from(members.len()).unwrap_or(u32::MAX),
    }
}
