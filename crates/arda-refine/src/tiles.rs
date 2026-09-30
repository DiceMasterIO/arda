//! The natural-terrain tile vocabulary.
//!
//! A tile is fixed by the classes on its four corners (a corner, or Wang,
//! tile set). The vocabulary has one pure tile per class; for every allowed
//! pair in [`PAIRS`], the 14 corner patterns that mix the two (straight
//! edges, outer and inner corners, and the two diagonals); and for every
//! triple of mutually allowed classes, the 36 junction patterns that use
//! all three, where three kinds of ground meet. Two tiles may sit side by
//! side exactly when the corners on their shared edge match, which is what
//! makes a transition such as grass to water a tile of its own rather than
//! a blend.

use crate::classes::{Class, COUNT, PAIRS, PARTNERS};

/// Corner order: north-west, north-east, south-east, south-west.
pub type Corners = [Class; 4];

/// Index of a tile in [`vocabulary`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TileId(pub u16);

/// Patterns per pair: the 16 two-class corner patterns minus the 2 pure ones.
pub const PATTERNS: usize = 14;
/// Patterns per triple: the 81 three-class patterns that use all three.
pub const JUNCTIONS: usize = 36;

const fn count_triples() -> usize {
    let mut n = 0;
    let mut a = 0;
    while a < COUNT {
        let mut b = a + 1;
        while b < COUNT {
            let mut c = b + 1;
            while c < COUNT {
                if PARTNERS[a] & (1 << b) != 0
                    && PARTNERS[a] & (1 << c) != 0
                    && PARTNERS[b] & (1 << c) != 0
                {
                    n += 1;
                }
                c += 1;
            }
            b += 1;
        }
        a += 1;
    }
    n
}

/// Number of mutually compatible class triples.
pub const TRIPLES: usize = count_triples();

/// Total number of tiles.
pub const TILE_COUNT: usize = COUNT + PAIRS.len() * PATTERNS + TRIPLES * JUNCTIONS;

// The goal asks for at least 60 natural tiles including transitions.
const _: () = assert!(TILE_COUNT >= 60);

/// Side of a tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// North edge.
    North,
    /// East edge.
    East,
    /// South edge.
    South,
    /// West edge.
    West,
}

const fn pair_table() -> [[u8; COUNT]; COUNT] {
    let mut t = [[u8::MAX; COUNT]; COUNT];
    let mut p = 0;
    while p < PAIRS.len() {
        let (a, b) = PAIRS[p];
        // `p` < 70 always fits a byte.
        #[allow(clippy::cast_possible_truncation)]
        let v = p as u8;
        t[a as usize][b as usize] = v;
        t[b as usize][a as usize] = v;
        p += 1;
    }
    t
}

const PAIR_INDEX: [[u8; COUNT]; COUNT] = pair_table();

/// Triples in lexicographic order, `(a, b, c)` with `a < b < c`.
const fn triple_list() -> [(u8, u8, u8); TRIPLES] {
    let mut out = [(0, 0, 0); TRIPLES];
    let mut n = 0;
    let mut a = 0;
    while a < COUNT {
        let mut b = a + 1;
        while b < COUNT {
            let mut c = b + 1;
            while c < COUNT {
                if PARTNERS[a] & (1 << b) != 0
                    && PARTNERS[a] & (1 << c) != 0
                    && PARTNERS[b] & (1 << c) != 0
                {
                    #[allow(clippy::cast_possible_truncation)]
                    let t = (a as u8, b as u8, c as u8);
                    out[n] = t;
                    n += 1;
                }
                c += 1;
            }
            b += 1;
        }
        a += 1;
    }
    out
}

const TRIPLE_LIST: [(u8, u8, u8); TRIPLES] = triple_list();

/// Base-3 corner patterns that use all three digits, in increasing order.
const fn junction_patterns() -> [u8; JUNCTIONS] {
    let mut out = [0; JUNCTIONS];
    let mut n = 0;
    let mut p = 0;
    while p < 81 {
        let (d0, d1, d2, d3) = (p % 3, p / 3 % 3, p / 9 % 3, p / 27);
        let mut seen = [false; 3];
        seen[d0] = true;
        seen[d1] = true;
        seen[d2] = true;
        seen[d3] = true;
        if seen[0] && seen[1] && seen[2] {
            #[allow(clippy::cast_possible_truncation)]
            let v = p as u8;
            out[n] = v;
            n += 1;
        }
        p += 1;
    }
    out
}

const JUNCTION_PATTERNS: [u8; JUNCTIONS] = junction_patterns();

fn triple_index(a: usize, b: usize, c: usize) -> Option<usize> {
    TRIPLE_LIST
        .binary_search(&(
            u8::try_from(a).ok()?,
            u8::try_from(b).ok()?,
            u8::try_from(c).ok()?,
        ))
        .ok()
}

/// The tile with these corners, or `None` when they mix more than three
/// classes or classes that may not meet.
#[must_use]
pub fn tile_of(c: Corners) -> Option<TileId> {
    let mut set: Vec<Class> = c.to_vec();
    set.sort();
    set.dedup();
    let id = match set.as_slice() {
        [a] => a.index(),
        [lo, hi] => {
            let p = PAIR_INDEX[lo.index()][hi.index()];
            if p == u8::MAX {
                return None;
            }
            let mut pattern = 0usize;
            for (i, &x) in c.iter().enumerate() {
                if x == *hi {
                    pattern |= 1 << i;
                }
            }
            COUNT + usize::from(p) * PATTERNS + pattern - 1
        }
        [a, b, d] => {
            let t = triple_index(a.index(), b.index(), d.index())?;
            let mut p = 0u8;
            let mut mul = 1u8;
            for &x in &c {
                let digit = if x == *a {
                    0
                } else if x == *b {
                    1
                } else {
                    2
                };
                p += digit * mul;
                mul *= 3;
            }
            let j = JUNCTION_PATTERNS.binary_search(&p).ok()?;
            COUNT + PAIRS.len() * PATTERNS + t * JUNCTIONS + j
        }
        _ => return None,
    };
    u16::try_from(id).ok().map(TileId)
}

/// The corners of a tile.
#[must_use]
pub fn corners_of(t: TileId) -> Option<Corners> {
    let i = usize::from(t.0);
    if i < COUNT {
        let c = Class::from_index(i)?;
        return Some([c; 4]);
    }
    let j = i - COUNT;
    if j < PAIRS.len() * PATTERNS {
        let (lo, hi) = *PAIRS.get(j / PATTERNS)?;
        let (lo, hi) = if lo < hi { (lo, hi) } else { (hi, lo) };
        let pattern = j % PATTERNS + 1;
        let mut c = [lo; 4];
        for (k, slot) in c.iter_mut().enumerate() {
            if pattern & (1 << k) != 0 {
                *slot = hi;
            }
        }
        return Some(c);
    }
    let k = j - PAIRS.len() * PATTERNS;
    let (a, b, d) = *TRIPLE_LIST.get(k / JUNCTIONS)?;
    let digits = [a, b, d];
    let mut p = *JUNCTION_PATTERNS.get(k % JUNCTIONS)?;
    let mut c = [Class::Water; 4];
    for slot in &mut c {
        *slot = Class::from_index(usize::from(digits[usize::from(p % 3)]))?;
        p /= 3;
    }
    Some(c)
}

/// Every tile, in id order.
#[must_use]
pub fn vocabulary() -> Vec<Corners> {
    (0..TILE_COUNT)
        .filter_map(|i| u16::try_from(i).ok().and_then(|i| corners_of(TileId(i))))
        .collect()
}

/// Most classes one tile may hold.
pub const MAX_CLASSES: u32 = 3;

fn common_partners(used: crate::classes::Mask) -> crate::classes::Mask {
    let mut m = crate::classes::ALL_MASK;
    let mut bits = used;
    while bits != 0 {
        let c = bits.trailing_zeros() as usize;
        bits &= bits - 1;
        m &= PARTNERS[c];
    }
    m
}

/// Whether corners with class sets `free` can be chosen so that, together
/// with the classes already in `used`, they form a tile: every pair may
/// meet and at most [`MAX_CLASSES`] classes appear.
#[must_use]
pub fn completable(used: crate::classes::Mask, free: &[crate::classes::Mask]) -> bool {
    let Some((&first, rest)) = free.split_first() else {
        return true;
    };
    let mut allowed = first & common_partners(used);
    if used.count_ones() >= MAX_CLASSES {
        allowed &= used;
    }
    // Reusing a class already present never adds a constraint.
    if allowed & used != 0 && completable(used, rest) {
        return true;
    }
    let mut bits = allowed & !used;
    while bits != 0 {
        let c = bits.trailing_zeros();
        bits &= bits - 1;
        if completable(used | (1 << c), rest) {
            return true;
        }
    }
    false
}

/// The two corners along one side, in a fixed order shared with the
/// opposite side of the neighbour (north/south: west then east;
/// east/west: north then south).
#[must_use]
pub const fn socket(c: Corners, side: Side) -> (Class, Class) {
    match side {
        Side::North => (c[0], c[1]),
        Side::East => (c[1], c[2]),
        Side::South => (c[3], c[2]),
        Side::West => (c[0], c[3]),
    }
}

/// Whether `b` may sit east of `a` (`east == true`) or south of `a`.
#[must_use]
pub fn legal_neighbours(a: Corners, b: Corners, east: bool) -> bool {
    if east {
        socket(a, Side::East) == socket(b, Side::West)
    } else {
        socket(a, Side::South) == socket(b, Side::North)
    }
}

/// The dominant land class of a tile: the most common non-water corner,
/// ties broken by [`Class::rank`]. `None` when every corner is water.
#[must_use]
pub fn dominant_land(c: Corners) -> Option<Class> {
    let mut best: Option<(usize, u8, Class)> = None;
    for &x in &c {
        if x == Class::Water {
            continue;
        }
        let n = c.iter().filter(|&&y| y == x).count();
        let key = (n, u8::MAX - x.rank(), x);
        if best.is_none_or(|b| (key.0, key.1) > (b.0, b.1)) {
            best = Some(key);
        }
    }
    best.map(|b| b.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vocabulary_is_large_and_round_trips() {
        let v = vocabulary();
        assert_eq!(v.len(), TILE_COUNT);
        for (i, c) in v.iter().enumerate() {
            assert_eq!(tile_of(*c), Some(TileId(u16::try_from(i).unwrap())));
        }
    }

    #[test]
    fn four_class_and_illegal_mixes_have_no_tile() {
        use Class::*;
        assert!(tile_of([Grass, Water, Sand, Mud]).is_none());
        assert!(tile_of([Grass, Water, Sand, Grass]).is_some());
        assert!(tile_of([Snow, Water, Sand, Snow]).is_none());
        assert!(tile_of([Snow, Water, Snow, Snow]).is_none());
        assert!(tile_of([Grass, Water, Water, Grass]).is_some());
    }

    #[test]
    fn dominant_land_prefers_majority_then_rank() {
        use Class::*;
        assert_eq!(dominant_land([Grass, Grass, Grass, Rock]), Some(Grass));
        assert_eq!(dominant_land([Grass, Rock, Rock, Grass]), Some(Rock));
        assert_eq!(dominant_land([Water, Water, Sand, Water]), Some(Sand));
        assert_eq!(dominant_land([Water; 4]), None);
    }
}
