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

/// A class and every class that may meet it.
const fn with_partners(c: usize) -> Mask {
    PARTNERS[c] | (1 << c)
}

/// The inner corners on one side of a border step from corner class `prev`
/// to `next` that can still join a tile with an inner corner from `before`
/// (the previous step's feasible set), among `mask`. Two consecutive
/// squares share their inner corner, so a set emptied here is a frame the
/// WFC could never complete.
fn inner_step(prev: usize, next: usize, before: Mask, mask: Mask) -> Mask {
    let both = with_partners(prev) & with_partners(next);
    let earlier = before & both;
    if earlier == 0 {
        return 0;
    }
    let mut out = 0;
    let mut bits = mask & both;
    while bits != 0 {
        let a = bits.trailing_zeros() as usize;
        bits &= bits - 1;
        let fits = earlier & with_partners(a);
        // At most three classes per tile: with two border classes the inner
        // pair must repeat one of them or each other.
        let ok = if prev == next || (1 << a) & ((1 << prev) | (1 << next)) != 0 {
            fits != 0
        } else {
            fits & ((1 << prev) | (1 << next) | (1 << a)) != 0
        };
        if ok {
            out |= 1 << a;
        }
    }
    out
}

/// Whether the classes `used` (decided corners of one square) can still form
/// a tile with a free corner of class set `free`: at most
/// [`crate::tiles::MAX_CLASSES`] classes, every pair of them may meet, and
/// the free corner can join them.
fn square_fits(used: Mask, free: Mask) -> bool {
    if used.count_ones() > crate::tiles::MAX_CLASSES {
        return false;
    }
    let mut bits = used;
    while bits != 0 {
        let a = bits.trailing_zeros() as usize;
        bits &= bits - 1;
        if used & !(PARTNERS[a] | (1 << a)) != 0 {
            return false;
        }
    }
    crate::tiles::completable(used, &[free])
}

/// One corner of a block-corner star: its allowed classes and per-class
/// cost (negative log weight).
#[derive(Debug, Clone, Copy)]
pub struct StarCorner {
    /// Allowed classes.
    pub mask: Mask,
    /// Cost per class.
    pub cost: [f64; COUNT],
}

/// The classes of a block corner and of its four arms, the first corners
/// along the four block edges that meet there (north, east, south, west).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Star {
    /// The block corner.
    pub centre: Class,
    /// The arms: north, east, south, west.
    pub arms: [Class; 4],
}

/// Arm candidates kept per arm (the cheapest classes, plus the centre's).
const ARM_CANDIDATES: usize = 4;

/// Solves a block corner together with its four arms so that each of the
/// four squares around the corner can still be completed with its inner
/// diagonal corner (`diagonals`: north-east, south-east, south-west,
/// north-west masks). The four blocks meeting at the corner read the same
/// corners and arms, so they fix the same star; each edge is then solved
/// between the stars at its ends. Solving the corner and each edge's first
/// step independently could give a square three classes that may not meet,
/// a contradiction baked into the fixed frame that no WFC retry could undo
/// (review 3.4-5).
#[must_use]
pub fn corner_star(centre: StarCorner, arms: [StarCorner; 4], diagonals: [Mask; 4]) -> Star {
    let ranked = |c: &StarCorner| -> Vec<usize> {
        let mut v: Vec<usize> = (0..COUNT).filter(|&k| c.mask & (1 << k) != 0).collect();
        v.sort_by(|&a, &b| c.cost[a].total_cmp(&c.cost[b]).then(a.cmp(&b)));
        v
    };
    let centres = ranked(&centre);
    let arm_lists: Vec<Vec<usize>> = arms.iter().map(ranked).collect();
    // Squares as (arm, arm, diagonal): NE = N + E, SE = E + S, SW = S + W,
    // NW = W + N.
    let squares = [(0, 1, 0), (1, 2, 1), (2, 3, 2), (3, 0, 3)];
    let mut best: Option<(f64, Star)> = None;
    for &c in &centres {
        let lists: Vec<Vec<usize>> = arm_lists
            .iter()
            .map(|l| {
                let mut v: Vec<usize> = l.iter().copied().take(ARM_CANDIDATES).collect();
                if l.contains(&c) && !v.contains(&c) {
                    v.push(c);
                }
                v
            })
            .collect();
        if lists.iter().any(Vec::is_empty) {
            continue;
        }
        for &n in &lists[0] {
            for &e in &lists[1] {
                for &so in &lists[2] {
                    for &w in &lists[3] {
                        let pick = [n, e, so, w];
                        let ok = squares.iter().all(|&(a, b, d)| {
                            square_fits((1 << c) | (1 << pick[a]) | (1 << pick[b]), diagonals[d])
                        });
                        if !ok {
                            continue;
                        }
                        let mut total = centre.cost[c];
                        for (k, &a) in pick.iter().enumerate() {
                            total += arms[k].cost[a] + if a == c { 0.0 } else { CHANGE };
                        }
                        if best.is_none_or(|b| total < b.0) {
                            let arms = pick.map(|a| Class::from_index(a).unwrap_or(Class::Grass));
                            best = Some((
                                total,
                                Star {
                                    centre: Class::from_index(c).unwrap_or(Class::Grass),
                                    arms,
                                },
                            ));
                        }
                    }
                }
            }
        }
    }
    best.map_or_else(
        || {
            // No consistent star among the candidates: the cheapest class
            // everywhere, as the old independent rule did.
            let pick = |c: &StarCorner| {
                ranked(c)
                    .first()
                    .and_then(|&k| Class::from_index(k))
                    .unwrap_or(Class::Grass)
            };
            Star {
                centre: pick(&centre),
                arms: arms.map(|a| pick(&a)),
            }
        },
        |b| b.1,
    )
}

/// Solves one edge with fixed end classes. Returns the classes of every
/// corner (ends included) and whether a legal sequence existed.
///
/// A Viterbi pass over the border corners that carries, for each state, the
/// inner corners on both sides that its best path can still complete
/// ([`inner_step`]): consecutive squares share an inner corner, so checking
/// each square alone (as before) let a frame through that the WFC could
/// never fill, on every retry (review 3.4-5). The first and last corners'
/// side masks are their stars' arms, fixed by the caller.
///
/// When the corners' own masks admit no sequence at all (two fixed features
/// that may not meet, a cliff beside a reed-bed pool), the pass runs again
/// with every class allowed at a [`BENT_MASK`] cost per corner outside its
/// mask: the frame stays a legal tiling the WFC can fill, and only the
/// corner classes, not the fixed squares' ground, bend.
#[must_use]
pub fn solve_line(line: &[LineCorner], start: Class, end: Class) -> (Vec<Class>, bool) {
    let strict = viterbi(line, start, end, false);
    if strict.1 {
        return strict;
    }
    let bent = viterbi(line, start, end, true);
    if bent.1 {
        bent
    } else {
        strict
    }
}

/// Cost of giving a border corner a class outside its mask in the second,
/// bent pass of [`solve_line`].
pub const BENT_MASK: f64 = 40.0;

fn viterbi(line: &[LineCorner], start: Class, end: Class, bend: bool) -> (Vec<Class>, bool) {
    let n = line.len();
    if n < 2 {
        return (vec![start; n], true);
    }
    let inf = f64::INFINITY;
    // Per corner and class: cost, predecessor, feasible inner sets.
    let mut cost = vec![[inf; COUNT]; n];
    let mut back = vec![[0u8; COUNT]; n];
    let mut side_a = vec![[0 as Mask; COUNT]; n];
    let mut side_b = vec![[0 as Mask; COUNT]; n];
    let s0 = start.index();
    cost[0][s0] = 0.0;
    side_a[0][s0] = line[0].side_a;
    side_b[0][s0] = line[0].side_b;
    for i in 1..n {
        for b in 0..COUNT {
            let inside = line[i].mask & (1 << b) != 0;
            let allowed = if i == n - 1 {
                b == end.index()
            } else {
                inside || bend
            };
            if !allowed {
                continue;
            }
            let mut best: Option<(f64, usize, Mask, Mask)> = None;
            for (a, &prev) in cost[i - 1].iter().enumerate() {
                if !prev.is_finite() || (a != b && PARTNERS[a] & (1 << b) == 0) {
                    continue;
                }
                let sa = inner_step(a, b, side_a[i - 1][a], line[i].side_a);
                let sb = inner_step(a, b, side_b[i - 1][a], line[i].side_b);
                if sa == 0 || sb == 0 {
                    continue;
                }
                let t = prev + if a == b { 0.0 } else { CHANGE };
                if best.is_none_or(|x| t < x.0) {
                    best = Some((t, a, sa, sb));
                }
            }
            if let Some((t, a, sa, sb)) = best {
                let emit = if i == n - 1 {
                    0.0
                } else if inside {
                    line[i].cost[b]
                } else {
                    BENT_MASK + line[i].cost[b].min(BENT_MASK)
                };
                cost[i][b] = t + emit;
                back[i][b] = u8::try_from(a).unwrap_or(0);
                side_a[i][b] = sa;
                side_b[i][b] = sb;
            }
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

    fn star_corner(mask: Mask, cheap: Class) -> StarCorner {
        let mut cost = [5.0; COUNT];
        cost[cheap.index()] = 0.0;
        StarCorner { mask, cost }
    }

    #[test]
    fn a_corner_star_never_leaves_its_squares_unfinishable() {
        // Review 3.4-5: the east arm wants water, the south arm snow, which
        // may not meet; chosen one edge at a time they made the corner
        // square impossible.
        assert!(!crate::classes::may_meet(Class::Water, Class::Snow));
        let star = corner_star(
            star_corner(LAND_MASK | Class::Water.bit(), Class::Grass),
            [
                star_corner(LAND_MASK, Class::Grass),
                star_corner(LAND_MASK | Class::Water.bit(), Class::Water),
                star_corner(LAND_MASK, Class::Snow),
                star_corner(LAND_MASK, Class::Grass),
            ],
            [LAND_MASK; 4],
        );
        let c = star.centre;
        for (a, b) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
            let used = c.bit() | star.arms[a].bit() | star.arms[b].bit();
            assert!(square_fits(used, LAND_MASK), "{star:?}");
        }
    }

    #[test]
    fn a_line_keeps_its_shared_inner_corners_completable() {
        // Inner side B allows only snow or water at corner 2; a border of
        // grass then water there would leave square 1-2 needing snow and
        // square 2-3 needing water at that one inner corner (review 3.4-5).
        let mut line = vec![flat(LAND_MASK | Class::Water.bit(), Class::Grass); 5];
        line[2].side_b = Class::Snow.bit() | Class::Water.bit();
        line[3].cost[Class::Water.index()] = -10.0;
        let (out, ok) = solve_line(&line, Class::Grass, Class::Grass);
        assert!(ok);
        // Every pair of squares along side B shares a feasible inner corner.
        for i in 1..4 {
            let used = out[i - 1].bit() | out[i].bit();
            assert!(square_fits(used, line[i].side_b), "{out:?}");
        }
    }

    #[test]
    fn an_impossible_line_is_bent_into_a_legal_one() {
        // Snow-only corners between water ends: no sequence within the
        // masks (water and snow may not meet), so the strict pass reports
        // it and the bent pass gives the corners classes that tile.
        let line = vec![flat(Class::Snow.bit(), Class::Snow); 4];
        let (_, strict) = viterbi(&line, Class::Water, Class::Water, false);
        assert!(!strict);
        let (out, ok) = solve_line(&line, Class::Water, Class::Water);
        assert!(ok);
        for w in out.windows(2) {
            assert!(crate::classes::may_meet(w[0], w[1]), "{out:?}");
        }
    }
}
