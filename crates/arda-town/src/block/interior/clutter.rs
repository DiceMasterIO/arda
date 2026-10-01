//! Placement helpers for varied furnishing (goal 64): props against a
//! randomly chosen stretch of wall, in corners, beside another prop, and
//! clutter drawn from weighted pools. Every choice comes from the
//! building's own stream, so the result depends only on the building.

use super::{put, size};
use crate::block::frame::Canon;
use crate::plan::grid::Side;
use crate::rng::Rng;

/// Every wall of a room, in a random order.
pub fn walls(rng: &mut Rng) -> [Side; 4] {
    let mut w = [Side::North, Side::East, Side::South, Side::West];
    for i in (1..4).rev() {
        let j = rng.index(i + 1);
        w.swap(i, j);
    }
    w
}

/// Places `id` against `wall` of `room`, long side along the wall, at a
/// random position (then every other one in turn).
pub fn along(
    c: &mut Canon,
    rng: &mut Rng,
    room: [i64; 4],
    wall: Side,
    id: &'static str,
) -> Option<usize> {
    let (w, h) = size(id);
    let (long, short) = (w.max(h), w.min(h));
    let along_x = matches!(wall, Side::North | Side::South);
    let rot = match (wall, w >= h) {
        (Side::North, true) | (Side::West, false) => 0,
        (Side::South, true) | (Side::East, false) => 180,
        (Side::East, true) => 90,
        (Side::West, true) => 270,
        (Side::North, false) => 90,
        (Side::South, false) => 270,
    };
    let (s0, s1) = if along_x {
        (room[0], room[2])
    } else {
        (room[1], room[3])
    };
    let n = s1 - s0 - long + 1;
    if n < 1 {
        return None;
    }
    let start = i64::from(rng.range(0, i32::try_from(n - 1).unwrap_or(0)));
    for k in 0..n {
        let s = s0 + (start + k) % n;
        let r = match wall {
            Side::North => [s, room[1], s + long, room[1] + short],
            Side::South => [s, room[3] - short, s + long, room[3]],
            Side::West => [room[0], s, room[0] + short, s + long],
            Side::East => [room[2] - short, s, room[2], s + long],
        };
        if let Some(i) = c.put_id(id, r, rot) {
            return Some(i);
        }
    }
    None
}

/// Places `id` against any wall of `room`, walls tried in random order.
pub fn anywhere_along(
    c: &mut Canon,
    rng: &mut Rng,
    room: [i64; 4],
    id: &'static str,
) -> Option<usize> {
    for wall in walls(rng) {
        if let Some(i) = along(c, rng, room, wall, id) {
            return Some(i);
        }
    }
    None
}

/// Places `id` in a corner of `room`, corners tried in random order.
pub fn corner(c: &mut Canon, rng: &mut Rng, room: [i64; 4], id: &'static str) -> Option<usize> {
    let (w, h) = size(id);
    let (w, h, rot) = if rng.chance(0.5) {
        (w, h, 0)
    } else {
        (h, w, 90)
    };
    let mut corners = [
        (room[0], room[1]),
        (room[2] - w, room[1]),
        (room[0], room[3] - h),
        (room[2] - w, room[3] - h),
    ];
    let k = rng.index(4);
    corners.rotate_left(k);
    for (x, y) in corners {
        if let Some(i) = put(c, id, x, y, rot) {
            return Some(i);
        }
    }
    None
}

/// Places a one-square `id` on a free square beside prop `anchor`.
pub fn beside(c: &mut Canon, rng: &mut Rng, anchor: Option<usize>, id: &'static str) -> bool {
    let Some(p) = anchor.and_then(|i| c.props.get(i)).cloned() else {
        return false;
    };
    let mut spots: Vec<(i64, i64)> = Vec::new();
    for x in p.x0..p.x1 {
        spots.push((x, p.y0 - 1));
        spots.push((x, p.y1));
    }
    for y in p.y0..p.y1 {
        spots.push((p.x0 - 1, y));
        spots.push((p.x1, y));
    }
    let n = spots.len();
    if n == 0 {
        return false;
    }
    let start = rng.index(n);
    (0..n).any(|k| {
        let (x, y) = spots[(start + k) % n];
        put(c, id, x, y, 0).is_some()
    })
}

/// A weighted pool of clutter ids.
pub type Pool = &'static [(&'static str, u32)];

/// One id drawn from `pool`.
pub fn draw(rng: &mut Rng, pool: Pool) -> &'static str {
    let total: u32 = pool.iter().map(|p| p.1).sum();
    let mut t = u32::try_from(rng.next_u64() % u64::from(total.max(1))).unwrap_or(0);
    for &(id, w) in pool {
        if t < w {
            return id;
        }
        t -= w;
    }
    pool.first().map_or("prop.crate", |p| p.0)
}

/// Up to `n` clutter props from `pool` along the walls of `room`, sometimes
/// in its corners; clusters of the same kind are common (a row of barrels).
pub fn scatter(c: &mut Canon, rng: &mut Rng, room: [i64; 4], pool: Pool, n: usize) {
    let mut last: Option<&'static str> = None;
    for _ in 0..n {
        let id = match last {
            Some(id) if rng.chance(0.35) => id,
            _ => draw(rng, pool),
        };
        let placed = if rng.chance(0.3) {
            corner(c, rng, room, id)
        } else {
            anywhere_along(c, rng, room, id)
        };
        last = placed.map(|_| id);
    }
}

/// The room's area in squares.
#[must_use]
pub const fn area(r: [i64; 4]) -> i64 {
    (r[2] - r[0]) * (r[3] - r[1])
}

/// A table with seats at a random spot of `room`, away from its walls when
/// there is room; returns the table.
pub fn table(
    c: &mut Canon,
    rng: &mut Rng,
    room: [i64; 4],
    seat: &'static str,
    seats: usize,
) -> Option<usize> {
    let across = rng.chance(0.5);
    let (tw, th, rot) = if across { (2, 1, 0) } else { (1, 2, 90) };
    let (x0, y0) = (room[0] + 1, room[1] + 1);
    let (x1, y1) = (room[2] - 1 - tw, room[3] - 1 - th);
    let mut spots = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            spots.push((x, y));
        }
    }
    if spots.is_empty() {
        return None;
    }
    let start = rng.index(spots.len());
    let n = spots.len();
    let t = (0..n).find_map(|k| {
        let (x, y) = spots[(start + k) % n];
        c.put_id("prop.table", [x, y, x + tw, y + th], rot)
    })?;
    for _ in 0..seats {
        let _ = beside(c, rng, Some(t), seat);
    }
    Some(t)
}
