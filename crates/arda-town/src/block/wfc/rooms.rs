//! Pass A: room partitions by WFC (logic/10 §town-interiors).
//!
//! A tile is a **position class**: where in its room a square lies
//! (north-west corner, north edge, …, south-east corner). Pairwise rules
//! on the classes force every room to be a rectangle of at least 2 × 2
//! squares that meets its neighbours only along partitions. The number of
//! rooms is capped by a limit on north-west corners, and the programme's
//! extra rooms are anchored as further corners. Zones (room kinds) are
//! then assigned to the rooms by a small deterministic search that keeps
//! the programme: required rooms, sizes, entry share, entrances in entry
//! zones and a doorway tree from the entrance.

use super::assign::{assign, max_rooms};
use super::programme::Programme;
use arda_wfc::{solve, Anchor, Dir, Failure, Limit, Problem, Rules, TileSet};

/// Position classes (3 × 3).
const P: usize = 9;
/// The north-west corner class.
const NW: usize = 0;

/// A room: zone index and canonical rectangle `[x0, y0, x1, y1)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Room {
    /// Zone index in the programme.
    pub zone: usize,
    /// Rectangle, canonical cells.
    pub rect: [i64; 4],
}

impl Room {
    /// Floor area, squares.
    #[must_use]
    pub const fn area(&self) -> i64 {
        (self.rect[2] - self.rect[0]) * (self.rect[3] - self.rect[1])
    }
}

/// A partition of the canonical frame into rooms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// Rooms in row-major order of their north-west squares.
    pub rooms: Vec<Room>,
    /// Room index of each canonical cell, row-major.
    pub cell_room: Vec<usize>,
    /// Doorway tree: `(child, parent)` room pairs, entrance rooms first.
    pub tree: Vec<(usize, usize)>,
    /// Attempts used.
    pub attempts: u8,
}

/// The position-class rules.
#[must_use]
pub fn rules() -> Option<Rules> {
    let mut r = Rules::new(P, 2).ok()?;
    for a in 0..P {
        let (ax, ay) = (a % 3, a / 3);
        // Shell edges only on the room's own outer sides.
        for (bad, d) in [
            (ay != 0, Dir::N),
            (ay != 2, Dir::S),
            (ax != 0, Dir::W),
            (ax != 2, Dir::E),
        ] {
            if bad {
                r.forbid_edge(a, d, 1);
            }
        }
        for b in 0..P {
            let (bx, by) = (b % 3, b / 3);
            let east = if ax == 2 {
                bx == 0
            } else {
                ay == by && bx >= 1
            };
            if east {
                r.allow(a, Dir::E, b, 0);
            }
            let south = if ay == 2 {
                by == 0
            } else {
                ax == bx && by >= 1
            };
            if south {
                r.allow(a, Dir::S, b, 0);
            }
        }
    }
    Some(r)
}

/// Reads room rectangles off a solved grid.
#[must_use]
pub fn decode(w: i64, d: i64, tiles: &[u16]) -> Option<(Vec<[i64; 4]>, Vec<usize>)> {
    let at = |x: i64, y: i64| -> Option<usize> {
        let i = usize::try_from(y * w + x).ok()?;
        tiles.get(i).map(|&t| usize::from(t))
    };
    let mut rects = Vec::new();
    let mut cell_room = vec![usize::MAX; tiles.len()];
    for y in 0..d {
        for x in 0..w {
            if at(x, y)? != NW {
                continue;
            }
            let mut x1 = x;
            while at(x1, y)? % 3 != 2 {
                x1 += 1;
            }
            let mut y1 = y;
            while at(x, y1)? / 3 != 2 {
                y1 += 1;
            }
            let room = rects.len();
            for yy in y..=y1 {
                for xx in x..=x1 {
                    let i = usize::try_from(yy * w + xx).ok()?;
                    if cell_room[i] != usize::MAX {
                        return None;
                    }
                    cell_room[i] = room;
                }
            }
            rects.push([x, y, x1 + 1, y1 + 1]);
        }
    }
    cell_room
        .iter()
        .all(|&r| r != usize::MAX)
        .then_some((rects, cell_room))
}

/// Partitions a `w × d` canonical frame; `doors` are the canonical cells
/// inside exterior doors, the front door first (its room must be an entry
/// zone; back doors may open into any room).
///
/// # Errors
/// Every attempt failed (the caller falls back to the rule programme).
pub fn partition(
    prog: &Programme,
    w: i64,
    d: i64,
    doors: &[(i64, i64)],
    seed: u64,
    key: [i64; 3],
) -> Result<Layout, Failure> {
    let fail = Failure {
        attempts: 0,
        rejected: 0,
    };
    let (wu, du) = (
        usize::try_from(w).map_err(|_| fail)?,
        usize::try_from(d).map_err(|_| fail)?,
    );
    let r = rules().ok_or(fail)?;
    let n = wu * du;
    if max_rooms(prog) == 1 {
        // One room: the only tiling is the whole floor.
        let rect = [0, 0, w, d];
        let ent: Vec<usize> = if doors.is_empty() {
            Vec::new()
        } else {
            vec![0]
        };
        let (rooms, tree) = assign(prog, &[rect], &ent, d, seed, key).ok_or(fail)?;
        return Ok(Layout {
            rooms,
            cell_room: vec![0; n],
            tree,
            attempts: 1,
        });
    }
    let door_cells: Vec<usize> = doors
        .iter()
        .filter(|&&(x, y)| x >= 0 && y >= 0 && x < w && y < d)
        .filter_map(|&(x, y)| usize::try_from(y * w + x).ok())
        .collect();
    let edges: Vec<[u8; 4]> = (0..n)
        .map(|i| {
            let (x, y) = (i % wu, i / wu);
            [
                u8::from(y == 0),
                u8::from(x + 1 == wu),
                u8::from(y + 1 == du),
                u8::from(x == 0),
            ]
        })
        .collect();
    // Extra rooms the programme needs, anchored as further corners; the
    // corner's weight prefers splits across the depth, near the middle.
    let required = prog.zones.iter().filter(|z| z.required).count();
    let corner = TileSet::single(NW);
    let edge_corners: Vec<(usize, TileSet)> = (1..n)
        .filter(|&i| {
            let (x, y) = (i % wu, i / wu);
            (x == 0 && y >= 2 && y + 2 <= du) || (y == 0 && x >= 2 && x + 2 <= wu)
        })
        .map(|i| (i, corner))
        .collect();
    // The building's own north-west corner is always a room corner.
    let anchors = vec![Anchor {
        options: edge_corners,
        count: u32::try_from(required.saturating_sub(1)).unwrap_or(0),
    }];
    let limits = vec![Limit {
        tiles: corner,
        cells: Vec::new(),
        max: u32::try_from(max_rooms(prog)).unwrap_or(1),
    }];
    // Extra corners go behind the entry zone when it keeps to the front,
    // so the entrance room stays large.
    let front = prog.zones.first().is_some_and(|z| z.depth > 0);
    let weight = |i: usize, t: usize| -> u32 {
        let (px, py) = (t % 3, t / 3);
        if t == NW {
            let (x, y) = (i % wu, i / wu);
            if front && (y == 0 || 2 * y < du) {
                return 1;
            }
            let aim = if front { du * 2 / 3 } else { du / 2 };
            let off = y.abs_diff(aim) + usize::from(x > 0) * 2;
            return u32::try_from(10usize.saturating_sub(off).max(1)).unwrap_or(1);
        }
        (if px == 1 { 10 } else { 1 }) * (if py == 1 { 10 } else { 1 })
    };
    let problem = Problem {
        rules: &r,
        w: wu,
        h: du,
        domains: vec![TileSet::first_n(P); n],
        edges,
        weight: &weight,
        anchors,
        limits,
        seed,
        key,
    };
    let solved = |tiles: &[u16]| {
        let (rects, cr) = decode(w, d, tiles)?;
        let ent: Vec<usize> = door_cells.iter().map(|&i| cr[i]).collect();
        let (rooms, tree) = assign(prog, &rects, &ent, d, seed, key)?;
        Some((rooms, cr, tree))
    };
    let sol = solve(&problem, arda_wfc::MAX_ATTEMPTS, &|t| solved(t).is_some())?;
    let (rooms, cell_room, tree) = solved(&sol.tiles).ok_or(fail)?;
    Ok(Layout {
        rooms,
        cell_room,
        tree,
        attempts: sol.attempts,
    })
}

/// The position-class tiles of a layout, row-major.
#[must_use]
pub fn classes(layout: &Layout, w: i64, d: i64) -> Vec<u16> {
    (0..w * d)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let r = usize::try_from(i)
                .ok()
                .and_then(|i| layout.cell_room.get(i))
                .and_then(|&r| layout.rooms.get(r))
                .map_or([0, 0, 1, 1], |r| r.rect);
            let cx = if x == r[0] {
                0
            } else if x + 1 == r[2] {
                2
            } else {
                1
            };
            let cy = if y == r[1] {
                0
            } else if y + 1 == r[3] {
                2
            } else {
                1
            };
            u16::try_from(cy * 3 + cx).unwrap_or(0)
        })
        .collect()
}

/// Illegal places in a layout's position classes (for review and tests).
#[must_use]
pub fn violations(layout: &Layout, w: i64, d: i64) -> usize {
    let Some(r) = rules() else { return usize::MAX };
    let (wu, du) = (
        usize::try_from(w).unwrap_or(0),
        usize::try_from(d).unwrap_or(0),
    );
    let n = wu * du;
    let weight = |_: usize, _: usize| 1;
    let p = Problem {
        rules: &r,
        w: wu,
        h: du,
        domains: vec![TileSet::first_n(P); n],
        edges: (0..n)
            .map(|i| {
                let (x, y) = (i % wu, i / wu);
                [
                    u8::from(y == 0),
                    u8::from(x + 1 == wu),
                    u8::from(y + 1 == du),
                    u8::from(x == 0),
                ]
            })
            .collect(),
        weight: &weight,
        anchors: Vec::new(),
        limits: Vec::new(),
        seed: 0,
        key: [0; 3],
    };
    arda_wfc::violations(&p, &classes(layout, w, d)).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_rules_only_allow_rectangles() {
        let r = rules().unwrap();
        // NW → N east; NW never directly beside another NW.
        assert!(r.legal(0, Dir::E, 1, 0));
        assert!(!r.legal(0, Dir::E, 0, 0));
        // A room's east edge meets the next room's west edge.
        assert!(r.legal(2, Dir::E, 0, 0));
        assert!(!r.legal(1, Dir::E, 0, 0));
        // Shell only on outer sides.
        assert!(r.edge_ok(0, Dir::N, 1));
        assert!(!r.edge_ok(4, Dir::N, 1));
    }
}
