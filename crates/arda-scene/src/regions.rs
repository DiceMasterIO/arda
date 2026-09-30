//! Merging same-kind squares into region outlines.

use crate::types::{Movement, Region, RegionKind};
use std::collections::BTreeMap;

type V = (u32, u32);

/// Difficult, shallow-water and deep-water regions from the movement layer.
#[must_use]
pub fn build(movement: &[Movement], width: u32, height: u32) -> Vec<Region> {
    [
        (RegionKind::Difficult, Movement::Difficult),
        (RegionKind::ShallowWater, Movement::Wade),
        (RegionKind::DeepWater, Movement::Swim),
    ]
    .into_iter()
    .filter_map(|(kind, m)| {
        let mask: Vec<bool> = movement.iter().map(|v| *v == m).collect();
        let rings = trace(&mask, width, height);
        (!rings.is_empty()).then_some(Region { kind, rings })
    })
    .collect()
}

/// Outlines the `true` squares of a row-major mask as closed rings on grid
/// vertices. Squares touching only at a corner become separate rings.
#[must_use]
pub fn trace(mask: &[bool], width: u32, height: u32) -> Vec<Vec<[u32; 2]>> {
    let at = |x: i64, y: i64| {
        x >= 0
            && y >= 0
            && x < i64::from(width)
            && y < i64::from(height)
            && usize::try_from(y * i64::from(width) + x)
                .ok()
                .and_then(|i| mask.get(i).copied())
                .unwrap_or(false)
    };
    // Directed boundary edges with the region on the right (clockwise in y-down).
    let mut out_edges: BTreeMap<V, Vec<V>> = BTreeMap::new();
    for y in 0..height {
        for x in 0..width {
            let (xi, yi) = (i64::from(x), i64::from(y));
            if !at(xi, yi) {
                continue;
            }
            let mut add = |a: V, b: V| out_edges.entry(a).or_default().push(b);
            if !at(xi, yi - 1) {
                add((x, y), (x + 1, y));
            }
            if !at(xi + 1, yi) {
                add((x + 1, y), (x + 1, y + 1));
            }
            if !at(xi, yi + 1) {
                add((x + 1, y + 1), (x, y + 1));
            }
            if !at(xi - 1, yi) {
                add((x, y + 1), (x, y));
            }
        }
    }
    let mut rings = Vec::new();
    while let Some((&start, _)) = out_edges.first_key_value() {
        let mut ring = vec![start];
        let mut cur = start;
        let mut dir: Option<(i64, i64)> = None;
        while let Some(outs) = out_edges.get_mut(&cur) {
            let pick = choose(outs, cur, dir);
            let next = outs.remove(pick);
            // Drop spent vertices so finding the next ring's start stays
            // O(log n) instead of rescanning every consumed entry.
            if outs.is_empty() {
                out_edges.remove(&cur);
            }
            dir = Some(delta(cur, next));
            cur = next;
            if cur == start {
                break;
            }
            ring.push(cur);
        }
        rings.push(simplify(&ring));
    }
    rings
}

fn delta(a: V, b: V) -> (i64, i64) {
    (
        i64::from(b.0) - i64::from(a.0),
        i64::from(b.1) - i64::from(a.1),
    )
}

/// At a vertex shared by two diagonal squares, take the right turn so the
/// two squares stay separate rings.
fn choose(outs: &[V], cur: V, dir: Option<(i64, i64)>) -> usize {
    let Some((dx, dy)) = dir else { return 0 };
    let right = (-dy, dx);
    outs.iter()
        .position(|&o| delta(cur, o) == right)
        .or_else(|| outs.iter().position(|&o| delta(cur, o) == (dx, dy)))
        .unwrap_or(0)
}

/// Drops vertices where the ring runs straight on.
fn simplify(ring: &[V]) -> Vec<[u32; 2]> {
    let n = ring.len();
    (0..n)
        .filter(|&i| {
            let prev = ring[(i + n - 1) % n];
            let next = ring[(i + 1) % n];
            delta(prev, ring[i]) != delta(ring[i], next)
        })
        .map(|i| [ring[i].0, ring[i].1])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_is_one_rectangle() {
        let mask = [true, true, false, true, true, false];
        assert_eq!(
            trace(&mask, 3, 2),
            vec![vec![[0, 0], [2, 0], [2, 2], [0, 2]]]
        );
    }

    #[test]
    fn diagonal_neighbours_stay_apart() {
        let mask = [true, false, false, true];
        assert_eq!(trace(&mask, 2, 2).len(), 2);
    }

    #[test]
    fn a_ring_with_a_hole_has_two_loops() {
        let mut mask = [true; 9];
        mask[4] = false;
        let rings = trace(&mask, 3, 3);
        assert_eq!(rings.len(), 2);
        assert_eq!(rings[1], vec![[1, 1], [1, 2], [2, 2], [2, 1]]);
    }
}
