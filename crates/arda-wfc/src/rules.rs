//! Adjacency rules: which tile may stand next to which, per direction and
//! per **edge kind**. An edge kind says what lies on the shared edge (open
//! floor, a partition wall, a building's shell, a door, the problem's
//! border), so the same two tiles may meet across a wall but not across
//! open floor. Rules are symmetric by construction.

use crate::set::{TileSet, MAX_TILES};
use crate::WfcError;

/// A grid direction; y grows south.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Dir {
    /// Towards −y.
    N,
    /// Towards +x.
    E,
    /// Towards +y.
    S,
    /// Towards −x.
    W,
}

impl Dir {
    /// All four, clockwise from north.
    pub const ALL: [Self; 4] = [Self::N, Self::E, Self::S, Self::W];

    /// Index 0..4 (clockwise from north).
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::N => 0,
            Self::E => 1,
            Self::S => 2,
            Self::W => 3,
        }
    }

    /// The direction with index `i % 4`.
    #[must_use]
    pub const fn from_index(i: usize) -> Self {
        match i % 4 {
            0 => Self::N,
            1 => Self::E,
            2 => Self::S,
            _ => Self::W,
        }
    }

    /// The reverse direction.
    #[must_use]
    pub const fn opposite(self) -> Self {
        Self::from_index(self.index() + 2)
    }

    /// Rotated `q` quarter turns clockwise.
    #[must_use]
    pub const fn turned(self, q: usize) -> Self {
        Self::from_index(self.index() + q)
    }

    /// Unit step `(dx, dy)`.
    #[must_use]
    pub const fn step(self) -> (i64, i64) {
        match self {
            Self::N => (0, -1),
            Self::E => (1, 0),
            Self::S => (0, 1),
            Self::W => (-1, 0),
        }
    }
}

/// Symmetric adjacency rules over `tiles` tiles and `kinds` edge kinds.
#[derive(Debug, Clone)]
pub struct Rules {
    tiles: usize,
    kinds: usize,
    allow: Vec<TileSet>,
    edge_ok: Vec<TileSet>,
}

impl Rules {
    /// Rules with no adjacency allowed yet and every tile allowed on every
    /// edge kind.
    ///
    /// # Errors
    /// [`WfcError::Vocabulary`] for more than [`MAX_TILES`] tiles or no
    /// edge kinds.
    pub fn new(tiles: usize, kinds: usize) -> Result<Self, WfcError> {
        if tiles == 0 || tiles > MAX_TILES || kinds == 0 {
            return Err(WfcError::Vocabulary { tiles, kinds });
        }
        Ok(Self {
            tiles,
            kinds,
            allow: vec![TileSet::empty(); kinds * 4 * tiles],
            edge_ok: vec![TileSet::first_n(tiles); kinds * 4],
        })
    }

    /// Number of tiles.
    #[must_use]
    pub const fn tiles(&self) -> usize {
        self.tiles
    }

    /// Number of edge kinds.
    #[must_use]
    pub const fn kinds(&self) -> usize {
        self.kinds
    }

    fn ai(&self, k: usize, d: Dir, t: usize) -> Option<usize> {
        (k < self.kinds && t < self.tiles).then_some((k * 4 + d.index()) * self.tiles + t)
    }

    /// Lets `b` stand in direction `d` of `a` across an edge of kind `k`
    /// (and so `a` in the opposite direction of `b`).
    pub fn allow(&mut self, a: usize, d: Dir, b: usize, k: usize) {
        if let Some(i) = self.ai(k, d, a) {
            self.allow[i].insert(b);
        }
        if let Some(i) = self.ai(k, d.opposite(), b) {
            self.allow[i].insert(a);
        }
    }

    /// Forbids tile `t` from having an edge of kind `k` on its side `d`.
    pub fn forbid_edge(&mut self, t: usize, d: Dir, k: usize) {
        if k < self.kinds {
            self.edge_ok[k * 4 + d.index()].remove(t);
        }
    }

    /// Whether `b` may stand in direction `d` of `a` across kind `k`.
    #[must_use]
    pub fn legal(&self, a: usize, d: Dir, b: usize, k: usize) -> bool {
        self.edge_ok(a, d, k)
            && self.edge_ok(b, d.opposite(), k)
            && self.ai(k, d, a).is_some_and(|i| self.allow[i].contains(b))
    }

    /// Whether tile `t` may have an edge of kind `k` on side `d`.
    #[must_use]
    pub fn edge_ok(&self, t: usize, d: Dir, k: usize) -> bool {
        self.edge_set(d, k).contains(t)
    }

    /// Tiles that may have an edge of kind `k` on side `d`.
    #[must_use]
    pub fn edge_set(&self, d: Dir, k: usize) -> TileSet {
        self.edge_ok
            .get(k * 4 + d.index())
            .copied()
            .unwrap_or_default()
    }

    /// Tiles allowed in direction `d` of any tile of `from`, across `k`.
    #[must_use]
    pub fn support(&self, from: TileSet, d: Dir, k: usize) -> TileSet {
        let mut out = TileSet::empty();
        for t in from.iter() {
            if let Some(i) = self.ai(k, d, t) {
                out = out.or(self.allow[i]);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_are_symmetric_and_kind_specific() {
        let mut r = Rules::new(3, 2).unwrap();
        r.allow(0, Dir::E, 1, 0);
        assert!(r.legal(0, Dir::E, 1, 0));
        assert!(r.legal(1, Dir::W, 0, 0));
        assert!(!r.legal(0, Dir::E, 1, 1));
        assert!(!r.legal(1, Dir::E, 0, 0));
        r.forbid_edge(0, Dir::E, 0);
        assert!(!r.legal(0, Dir::E, 1, 0));
        assert!(Rules::new(300, 1).is_err());
        assert_eq!(Dir::N.turned(3), Dir::W);
        assert_eq!(Dir::E.opposite(), Dir::W);
    }
}
