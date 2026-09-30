//! Global square coordinates, hashing and the region grid.
//!
//! Everything in this crate is computed on one global lattice of 5-ft
//! squares (`100 m / 64`, so a 100 m cell is exactly a 64 × 64 block), with
//! the origin at world `(0, 0)` m and y growing south. Every decision is a
//! pure function of global coordinates and the seed, which is what makes
//! windows agree exactly along their shared edges.

use arda_tactical::layout::EdgeAxis;
use arda_tactical::noise::{hash2, mix};

/// Side of one tactical square in metres (a 100 m cell is 64 squares).
pub const SQUARE_M: f64 = 100.0 / 64.0;
/// Squares per 100 m land-use cell side.
pub const CELL_SQUARES: i64 = 64;

/// A square on the global lattice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Sq {
    /// Column, east-positive.
    pub x: i64,
    /// Row, south-positive.
    pub y: i64,
}

impl Sq {
    /// A square from its column and row.
    #[must_use]
    pub const fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }

    /// The square containing a point given in square units.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // floor of in-range map positions
    pub fn containing(p: [f64; 2]) -> Self {
        Self::new(p[0].floor() as i64, p[1].floor() as i64)
    }

    /// The square centre in square units.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // map coordinates are far below 2^52
    pub fn centre(self) -> [f64; 2] {
        [self.x as f64 + 0.5, self.y as f64 + 0.5]
    }

    /// The world-metre centre of the square.
    #[must_use]
    pub fn centre_m(self) -> [f64; 2] {
        let c = self.centre();
        [c[0] * SQUARE_M, c[1] * SQUARE_M]
    }

    /// The 100 m land-use cell holding this square.
    #[must_use]
    pub fn cell(self) -> (i64, i64) {
        (
            self.x.div_euclid(CELL_SQUARES),
            self.y.div_euclid(CELL_SQUARES),
        )
    }

    /// The square offset by `(dx, dy)`.
    #[must_use]
    pub const fn offset(self, dx: i64, dy: i64) -> Self {
        Self::new(self.x + dx, self.y + dy)
    }
}

/// A square edge on the global lattice, keyed like
/// [`arda_tactical::layout::WallSegment`]: `Vertical (x, y)` is the west edge
/// of square `(x, y)`, `Horizontal (x, y)` its north edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Edge {
    /// Which edge of the square.
    pub axis: EdgeAxis,
    /// Column of the square the edge belongs to.
    pub x: i64,
    /// Row of the square the edge belongs to.
    pub y: i64,
}

impl Edge {
    /// The edge between two 4-neighbouring squares.
    #[must_use]
    pub fn between(a: Sq, b: Sq) -> Self {
        if a.y == b.y {
            Self {
                axis: EdgeAxis::Vertical,
                x: a.x.max(b.x),
                y: a.y,
            }
        } else {
            Self {
                axis: EdgeAxis::Horizontal,
                x: a.x,
                y: a.y.max(b.y),
            }
        }
    }

    /// The edge joining two lattice vertices one unit apart.
    #[must_use]
    pub fn from_vertices(a: (i64, i64), b: (i64, i64)) -> Self {
        if a.1 == b.1 {
            Self {
                axis: EdgeAxis::Horizontal,
                x: a.0.min(b.0),
                y: a.1,
            }
        } else {
            Self {
                axis: EdgeAxis::Vertical,
                x: a.0,
                y: a.1.min(b.1),
            }
        }
    }

    /// The two squares either side: west/east or north/south.
    #[must_use]
    pub fn sides(self) -> (Sq, Sq) {
        match self.axis {
            EdgeAxis::Vertical => (Sq::new(self.x - 1, self.y), Sq::new(self.x, self.y)),
            EdgeAxis::Horizontal => (Sq::new(self.x, self.y - 1), Sq::new(self.x, self.y)),
        }
    }

    /// The edge midpoint in square units.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // map coordinates are far below 2^52
    pub fn midpoint(self) -> [f64; 2] {
        match self.axis {
            EdgeAxis::Vertical => [self.x as f64, self.y as f64 + 0.5],
            EdgeAxis::Horizontal => [self.x as f64 + 0.5, self.y as f64],
        }
    }

    /// The next edge along the same line (east or south).
    #[must_use]
    pub fn next(self) -> Self {
        match self.axis {
            EdgeAxis::Vertical => Self {
                y: self.y + 1,
                ..self
            },
            EdgeAxis::Horizontal => Self {
                x: self.x + 1,
                ..self
            },
        }
    }

    /// The previous edge along the same line (west or north).
    #[must_use]
    pub fn prev(self) -> Self {
        match self.axis {
            EdgeAxis::Vertical => Self {
                y: self.y - 1,
                ..self
            },
            EdgeAxis::Horizontal => Self {
                x: self.x - 1,
                ..self
            },
        }
    }

    /// The two lattice vertices at its ends.
    #[must_use]
    pub fn vertices(self) -> [(i64, i64); 2] {
        match self.axis {
            EdgeAxis::Vertical => [(self.x, self.y), (self.x, self.y + 1)],
            EdgeAxis::Horizontal => [(self.x, self.y), (self.x + 1, self.y)],
        }
    }
}

/// A salted 2-D hash; salts separate independent decisions.
#[must_use]
pub fn h2(seed: u64, salt: u64, x: i64, y: i64) -> u64 {
    hash2(mix(seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15)), x, y)
}

/// A salted 3-D hash.
#[must_use]
pub fn h3(seed: u64, salt: u64, x: i64, y: i64, z: i64) -> u64 {
    hash2(h2(seed, salt, x, y), z, 0)
}

/// A hash mapped to `[0, 1)` with 53 bits of precision.
///
/// The input is remixed first, so derived keys such as `h >> 7` or `h ^ 1`
/// give independent uniform values rather than a shrunken range.
#[must_use]
#[allow(clippy::cast_precision_loss)] // 53-bit integers are exact in f64
pub fn u01(h: u64) -> f64 {
    (mix(h) >> 11) as f64 / (1u64 << 53) as f64
}

/// One of four choices from a hash (remixed like [`u01`]).
#[must_use]
pub fn pick4(h: u64) -> usize {
    usize::try_from(mix(h) % 4).unwrap_or(0)
}

/// A hash mapped to `[-1, 1)`.
#[must_use]
pub fn s11(h: u64) -> f64 {
    u01(h) * 2.0 - 1.0
}

/// Squared distance from `p` to the segment `a–b`, and the parameter of the
/// closest point.
#[must_use]
pub fn seg_dist2(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> (f64, f64) {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (cx, cy) = (a[0] + t * dx - p[0], a[1] + t * dy - p[1]);
    (cx * cx + cy * cy, t)
}

/// A dense grid over a rectangle of global squares.
#[derive(Debug, Clone)]
pub struct Grid<T> {
    /// West-most column.
    pub x0: i64,
    /// North-most row.
    pub y0: i64,
    /// Width in squares.
    pub w: i64,
    /// Height in squares.
    pub h: i64,
    /// Row-major values.
    pub data: Vec<T>,
}

impl<T: Clone> Grid<T> {
    /// A grid filled with `v`.
    #[must_use]
    pub fn new(x0: i64, y0: i64, w: i64, h: i64, v: T) -> Self {
        let n = usize::try_from((w * h).max(0)).unwrap_or(0);
        Self {
            x0,
            y0,
            w,
            h,
            data: vec![v; n],
        }
    }

    /// Whether the square lies on the grid.
    #[must_use]
    pub fn contains(&self, s: Sq) -> bool {
        s.x >= self.x0 && s.y >= self.y0 && s.x < self.x0 + self.w && s.y < self.y0 + self.h
    }

    fn index(&self, s: Sq) -> Option<usize> {
        if !self.contains(s) {
            return None;
        }
        usize::try_from((s.y - self.y0) * self.w + (s.x - self.x0)).ok()
    }

    /// The value at a square, if on the grid.
    #[must_use]
    pub fn get(&self, s: Sq) -> Option<&T> {
        self.index(s).and_then(|i| self.data.get(i))
    }

    /// Mutable access to a square, if on the grid.
    pub fn get_mut(&mut self, s: Sq) -> Option<&mut T> {
        self.index(s).and_then(|i| self.data.get_mut(i))
    }

    /// Sets a square if it is on the grid.
    pub fn set(&mut self, s: Sq, v: T) {
        if let Some(slot) = self.get_mut(s) {
            *slot = v;
        }
    }

    /// Every square of the grid in row-major order.
    pub fn squares(&self) -> impl Iterator<Item = Sq> + '_ {
        (self.y0..self.y0 + self.h)
            .flat_map(move |y| (self.x0..self.x0 + self.w).map(move |x| Sq::new(x, y)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_between_neighbours_match_the_layout_convention() {
        let e = Edge::between(Sq::new(3, 4), Sq::new(2, 4));
        assert_eq!((e.axis, e.x, e.y), (EdgeAxis::Vertical, 3, 4));
        let e = Edge::between(Sq::new(3, 4), Sq::new(3, 5));
        assert_eq!((e.axis, e.x, e.y), (EdgeAxis::Horizontal, 3, 5));
        assert_eq!(e.sides(), (Sq::new(3, 4), Sq::new(3, 5)));
        assert_eq!(Edge::from_vertices((3, 5), (4, 5)), e);
    }

    #[test]
    fn shifted_hashes_stay_uniform() {
        // Regression: u01(h >> k) once collapsed to [0, 2^-k).
        let mut hi = 0;
        for i in 0..1000 {
            let h = h2(1, 2, i, 0);
            if u01(h >> 21) > 0.5 {
                hi += 1;
            }
        }
        assert!((400..600).contains(&hi), "{hi}");
    }

    #[test]
    fn cells_floor_towards_negative_infinity() {
        assert_eq!(Sq::new(-1, 64).cell(), (-1, 1));
        assert_eq!(Sq::containing([-0.25, 63.9]), Sq::new(-1, 63));
    }
}
