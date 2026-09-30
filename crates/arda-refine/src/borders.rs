//! Fixed block borders (goal 42, mockup "Continuity").
//!
//! The corners on a block edge are shared with the neighbouring block, so
//! they are decided before the WFC by a rule that reads only data both
//! blocks see identically: the corner masks and prior scores along the
//! edge and one corner either side of it. Block corners take their best
//! feasible class; each edge between two block corners is then the
//! cheapest legal class sequence (a Viterbi pass, always run in increasing
//! coordinate order). Two blocks sharing an edge therefore fix exactly the
//! same corners, and every pair of squares across the seam shares its two
//! edge corners.

use crate::classes::{Class, Mask, COUNT, PARTNERS};

/// Cost of a class change between consecutive border corners.
const CHANGE: f64 = 0.8;

/// One border corner: its mask, the masks of its neighbours on either side
/// of the edge, and its per-class cost (negative log weight).
#[derive(Debug, Clone, Copy)]
pub struct LineCorner {
    /// Allowed classes.
    pub mask: Mask,
    /// Mask of the corner one step to the left/north of the edge.
    pub side_a: Mask,
    /// Mask of the corner one step to the right/south of the edge.
    pub side_b: Mask,
    /// Cost per class.
    pub cost: [f64; COUNT],
}

/// Whether the square spanned by border corners `a, b` and inner corners
/// with masks `i1, i2` can still be completed.
fn square_ok(a: usize, b: usize, i1: Mask, i2: Mask) -> bool {
    if a != b && PARTNERS[a] & (1 << b) == 0 {
        return false;
    }
    crate::tiles::completable((1 << a) | (1 << b), &[i1, i2])
}

/// The best class for a block corner, given its mask, cost and the masks
/// of its eight neighbours (row-major 3 × 3 with the corner in the middle).
#[must_use]
pub fn corner_class(mask: Mask, cost: &[f64; COUNT], around: [Mask; 9]) -> Class {
    let squares = [[0, 1, 3], [1, 2, 5], [3, 6, 7], [5, 8, 7]];
    let mut best: Option<(f64, usize)> = None;
    for (c, cc) in cost.iter().enumerate() {
        if mask & (1 << c) == 0 {
            continue;
        }
        let ok = squares
            .iter()
            .all(|s| crate::tiles::completable(1 << c, &s.map(|k| around[k])));
        let penalty = if ok { 0.0 } else { 1e6 };
        let total = cc + penalty;
        if best.is_none_or(|b| total < b.0) {
            best = Some((total, c));
        }
    }
    best.and_then(|b| Class::from_index(b.1))
        .or_else(|| crate::classes::first(mask))
        .unwrap_or(Class::Grass)
}

/// Solves one edge with fixed end classes. Returns the classes of every
/// corner (ends included) and whether a legal sequence existed.
#[must_use]
pub fn solve_line(line: &[LineCorner], start: Class, end: Class) -> (Vec<Class>, bool) {
    let n = line.len();
    if n < 2 {
        return (vec![start; n], true);
    }
    let inf = f64::INFINITY;
    let mut cost = vec![[inf; COUNT]; n];
    let mut back = vec![[0u8; COUNT]; n];
    cost[0][start.index()] = 0.0;
    for i in 1..n {
        for b in 0..COUNT {
            let allowed = if i == n - 1 {
                b == end.index()
            } else {
                line[i].mask & (1 << b) != 0
            };
            if !allowed {
                continue;
            }
            let mut best = (inf, 0u8);
            for (a, &prev) in cost[i - 1].iter().enumerate() {
                if !prev.is_finite() {
                    continue;
                }
                let ok = square_ok(a, b, line[i - 1].side_a, line[i].side_a)
                    && square_ok(a, b, line[i - 1].side_b, line[i].side_b);
                if !ok {
                    continue;
                }
                let t = prev + if a == b { 0.0 } else { CHANGE };
                if t < best.0 {
                    best = (t, u8::try_from(a).unwrap_or(0));
                }
            }
            let emit = if i == n - 1 { 0.0 } else { line[i].cost[b] };
            cost[i][b] = best.0 + emit;
            back[i][b] = best.1;
        }
    }
    if !cost[n - 1][end.index()].is_finite() {
        // No legal sequence: best class per corner, flagged by the caller.
        let mut out: Vec<Class> = line
            .iter()
            .map(|c| {
                let mut best: Option<(f64, usize)> = None;
                for k in 0..COUNT {
                    if c.mask & (1 << k) != 0 && best.is_none_or(|b| c.cost[k] < b.0) {
                        best = Some((c.cost[k], k));
                    }
                }
                best.and_then(|b| Class::from_index(b.1)).unwrap_or(start)
            })
            .collect();
        out[0] = start;
        out[n - 1] = end;
        return (out, false);
    }
    let mut out = vec![end; n];
    let mut c = end.index();
    for i in (1..n).rev() {
        out[i] = Class::from_index(c).unwrap_or(end);
        c = usize::from(back[i][c]);
    }
    out[0] = start;
    (out, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classes::LAND_MASK;

    fn flat(mask: Mask, cheap: Class) -> LineCorner {
        let mut cost = [5.0; COUNT];
        cost[cheap.index()] = 0.0;
        LineCorner {
            mask,
            side_a: LAND_MASK,
            side_b: LAND_MASK,
            cost,
        }
    }

    #[test]
    fn a_line_follows_the_cheapest_legal_classes() {
        let mut line = vec![flat(LAND_MASK, Class::Grass); 10];
        for c in line.iter_mut().skip(5) {
            *c = flat(LAND_MASK, Class::ForestFloor);
        }
        let (out, ok) = solve_line(&line, Class::Grass, Class::ForestFloor);
        assert!(ok);
        assert_eq!(out[1], Class::Grass);
        assert_eq!(out[8], Class::ForestFloor);
        for w in out.windows(2) {
            assert!(crate::classes::may_meet(w[0], w[1]));
        }
    }

    #[test]
    fn an_impossible_line_is_reported() {
        let line = vec![flat(Class::Snow.bit(), Class::Snow); 4];
        let (_, ok) = solve_line(&line, Class::Water, Class::Water);
        assert!(!ok);
    }
}
