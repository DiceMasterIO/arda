//! Access in a furnished interior: the circulation kept open (each
//! room's spine and the paths from its entrances), the reachability check
//! every WFC interior must pass, and the legality check for review.

use super::furnish::{Frame, EXT, OPEN, PART};
use super::piece::{Vocab, What};
use arda_wfc::{Dir, Problem, TileSet};

/// Whether every walkable square is reachable from the exterior doors and
/// every furniture square touches a reachable one.
#[must_use]
pub fn reachable(v: &Vocab, f: &Frame<'_>, tiles: &[u16]) -> bool {
    let (w, n) = (f.w, tiles.len());
    let walk = |i: usize| v.walkable(usize::from(tiles[i]));
    let mut seen = vec![false; n];
    let mut stack = Vec::new();
    for y in 0..f.d {
        for x in 0..w {
            let Some(i) = f.idx(x, y) else { continue };
            if Dir::ALL.iter().any(|&d| f.kind(x, y, d) == EXT) && walk(i) {
                seen[i] = true;
                stack.push(i);
            }
        }
    }
    let door_side = |i: usize| -> Option<usize> {
        let t = v.tiles.get(usize::from(tiles[i]))?;
        (matches!(v.pieces[t.piece].what, What::Door)).then_some(usize::from(t.o))
    };
    while let Some(i) = stack.pop() {
        let (x, y) = (
            i64::try_from(i).unwrap_or(0) % w,
            i64::try_from(i).unwrap_or(0) / w,
        );
        for d in Dir::ALL {
            let (dx, dy) = d.step();
            let Some(j) = f.idx(x + dx, y + dy) else {
                continue;
            };
            let k = f.kind(x, y, d);
            let through = k == OPEN
                || (k == PART
                    && (door_side(i) == Some(d.index())
                        || door_side(j) == Some(d.opposite().index())));
            if through && walk(j) && !seen[j] {
                seen[j] = true;
                stack.push(j);
            }
        }
    }
    // Every furniture piece must be usable: some square of it touches a
    // reachable walkable square across open floor.
    let touches = |i: usize| {
        let (x, y) = (
            i64::try_from(i).unwrap_or(0) % w,
            i64::try_from(i).unwrap_or(0) / w,
        );
        Dir::ALL.iter().any(|&d| {
            let (dx, dy) = d.step();
            f.kind(x, y, d) == OPEN && f.idx(x + dx, y + dy).is_some_and(|j| seen[j])
        })
    };
    (0..n).all(|i| {
        if walk(i) {
            return seen[i];
        }
        let Some(t) = v.tiles.get(usize::from(tiles[i])) else {
            return false;
        };
        let (x, y) = (
            i64::try_from(i).unwrap_or(0) % w,
            i64::try_from(i).unwrap_or(0) / w,
        );
        let (x0, y0) = (x - t.px, y - t.py);
        (y0..y0 + t.rh).any(|yy| (x0..x0 + t.rw).any(|xx| f.idx(xx, yy).is_some_and(touches)))
    })
}

/// The circulation of each room: its **spine** (the middle line along its
/// long axis) and L-shaped paths from each entrance (inside exterior
/// doors, doorways and the squares across them) to the spine. These
/// squares stay open floor, so furniture gathers along the walls and
/// around tables and every square beside the spine stays reachable.
#[must_use]
pub fn circulation(f: &Frame<'_>, entrances: &[usize], aisles: &[bool]) -> Vec<bool> {
    let w = f.w;
    let n = usize::try_from(f.w * f.d).unwrap_or(0);
    let lay = f.layout;
    let mut keep = vec![false; n];
    let xy = |i: usize| {
        let i = i64::try_from(i).unwrap_or(0);
        (i % w, i / w)
    };
    for (r, room) in lay.rooms.iter().enumerate() {
        let ends: Vec<(i64, i64)> = entrances
            .iter()
            .filter(|&&i| lay.cell_room.get(i) == Some(&r))
            .map(|&i| xy(i))
            .collect();
        let [x0, y0, x1, y1] = room.rect;
        let mut mark = |x: i64, y: i64| {
            if let Some(i) = f.idx(x, y) {
                keep[i] = true;
            }
        };
        // The spine: along x in wide rooms, along y in deep ones; in a
        // room two squares across, the line holding the first entrance.
        // An aisle (a nave's) runs down the middle from front to back.
        let aisle = aisles.get(r).copied().unwrap_or(false);
        let wide = !aisle && x1 - x0 >= y1 - y0;
        let first = ends.first().copied().unwrap_or((x0, y0));
        let spine = |p: (i64, i64)| -> (i64, i64) {
            if aisle {
                // A central aisle, so pews stand in aligned rows either
                // side of it (`super::rows`); the entrance paths join it.
                ((x0 + x1 - 1) / 2, p.1)
            } else if wide {
                let row = if y1 - y0 == 2 {
                    first.1
                } else {
                    (y0 + y1 - 1) / 2
                };
                (p.0, row)
            } else {
                let col = if x1 - x0 == 2 {
                    first.0
                } else {
                    (x0 + x1 - 1) / 2
                };
                (col, p.1)
            }
        };
        let (a, b) = if wide {
            (spine((x0, 0)), spine((x1 - 1, 0)))
        } else {
            (spine((0, y0)), spine((0, y1 - 1)))
        };
        // Small rooms keep only their entrance paths.
        let spined = (x1 - x0).min(y1 - y0) == 2 || (x1 - x0) * (y1 - y0) >= 12;
        for t in [(a, b)].into_iter().filter(|_| spined) {
            let (mut x, mut y) = t.0;
            mark(x, y);
            while (x, y) != t.1 {
                x += (t.1 .0 - x).signum();
                y += (t.1 .1 - y).signum();
                mark(x, y);
            }
        }
        for &e in &ends {
            let t = if spined {
                spine(e)
            } else {
                ((x0 + x1 - 1) / 2, (y0 + y1 - 1) / 2)
            };
            let (mut x, mut y) = e;
            mark(x, y);
            while (x, y) != t {
                x += (t.0 - x).signum();
                y += (t.1 - y).signum();
                mark(x, y);
            }
        }
    }
    keep
}

/// Illegal places in a finished pass-B grid (for review and tests): the
/// tiles checked against the vocabulary's rules and the frame's edges.
#[must_use]
pub fn violations(v: &Vocab, f: &Frame<'_>, tiles: &[u16]) -> usize {
    let n = tiles.len();
    let edges = (0..n)
        .map(|i| {
            let i = i64::try_from(i).unwrap_or(0);
            let (x, y) = (i % f.w.max(1), i / f.w.max(1));
            Dir::ALL.map(|d| f.kind(x, y, d))
        })
        .collect();
    let weight = |_: usize, _: usize| 1;
    let p = Problem {
        rules: &v.rules,
        w: usize::try_from(f.w).unwrap_or(0),
        h: usize::try_from(f.d).unwrap_or(0),
        domains: vec![TileSet::first_n(v.tiles.len()); n],
        edges,
        weight: &weight,
        anchors: Vec::new(),
        limits: Vec::new(),
        seed: 0,
        key: [0; 3],
    };
    arda_wfc::violations(&p, tiles).len()
}
