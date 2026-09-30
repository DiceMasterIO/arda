//! Outlines of cell sets as rectilinear lattice polygons.

use std::collections::{BTreeMap, BTreeSet};

/// The outer boundary of a set of cells, as lattice vertices in order
/// (clockwise on screen, y south), with collinear vertices removed. For a
/// set with several boundary loops the longest is returned.
#[must_use]
pub fn outline(cells: &[(i64, i64)]) -> Vec<(i64, i64)> {
    let set: BTreeSet<(i64, i64)> = cells.iter().copied().collect();
    let mut edges: BTreeMap<(i64, i64), Vec<(i64, i64)>> = BTreeMap::new();
    for &(x, y) in &set {
        if !set.contains(&(x, y - 1)) {
            edges.entry((x, y)).or_default().push((x + 1, y));
        }
        if !set.contains(&(x + 1, y)) {
            edges.entry((x + 1, y)).or_default().push((x + 1, y + 1));
        }
        if !set.contains(&(x, y + 1)) {
            edges.entry((x + 1, y + 1)).or_default().push((x, y + 1));
        }
        if !set.contains(&(x - 1, y)) {
            edges.entry((x, y + 1)).or_default().push((x, y));
        }
    }
    let mut best: Vec<(i64, i64)> = Vec::new();
    while let Some((&start, _)) = edges.iter().find(|(_, v)| !v.is_empty()) {
        let mut loop_pts = vec![start];
        let mut cur = start;
        while let Some(next) = edges.get_mut(&cur).and_then(Vec::pop) {
            if next == start {
                break;
            }
            loop_pts.push(next);
            cur = next;
        }
        if loop_pts.len() > best.len() {
            best = loop_pts;
        }
    }
    simplify(&best)
}

fn simplify(p: &[(i64, i64)]) -> Vec<(i64, i64)> {
    let n = p.len();
    if n < 3 {
        return p.to_vec();
    }
    (0..n)
        .filter(|&i| {
            let a = p[(i + n - 1) % n];
            let b = p[i];
            let c = p[(i + 1) % n];
            (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0) != 0
        })
        .map(|i| p[i])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_and_ell() {
        let rect: Vec<_> = (0..3).flat_map(|x| (0..2).map(move |y| (x, y))).collect();
        assert_eq!(outline(&rect).len(), 4);
        let ell = vec![(0, 0), (0, 1), (1, 1)];
        assert_eq!(outline(&ell).len(), 6);
    }
}
