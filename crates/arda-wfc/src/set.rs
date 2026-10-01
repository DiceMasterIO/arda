//! A fixed-size tile set: one bit per tile id, up to [`MAX_TILES`].

/// Largest tile vocabulary one problem may use.
pub const MAX_TILES: usize = 256;

const WORDS: usize = MAX_TILES / 64;

/// A set of tile ids `0..MAX_TILES`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TileSet([u64; WORDS]);

impl TileSet {
    /// The empty set.
    #[must_use]
    pub const fn empty() -> Self {
        Self([0; WORDS])
    }

    /// Tiles `0..n` (clamped to [`MAX_TILES`]).
    #[must_use]
    pub fn first_n(n: usize) -> Self {
        let mut s = Self::empty();
        for t in 0..n.min(MAX_TILES) {
            s.insert(t);
        }
        s
    }

    /// A set holding one tile.
    #[must_use]
    pub fn single(t: usize) -> Self {
        let mut s = Self::empty();
        s.insert(t);
        s
    }

    /// A set of the given tiles.
    #[must_use]
    pub fn of(tiles: &[usize]) -> Self {
        let mut s = Self::empty();
        for &t in tiles {
            s.insert(t);
        }
        s
    }

    /// Adds a tile (ids at or beyond [`MAX_TILES`] are ignored).
    pub fn insert(&mut self, t: usize) {
        if let Some(w) = self.0.get_mut(t / 64) {
            *w |= 1 << (t % 64);
        }
    }

    /// Removes a tile.
    pub fn remove(&mut self, t: usize) {
        if let Some(w) = self.0.get_mut(t / 64) {
            *w &= !(1 << (t % 64));
        }
    }

    /// Whether the tile is in the set.
    #[must_use]
    pub fn contains(&self, t: usize) -> bool {
        self.0.get(t / 64).is_some_and(|w| w & (1 << (t % 64)) != 0)
    }

    /// Number of tiles.
    #[must_use]
    pub fn count(&self) -> u32 {
        self.0.iter().map(|w| w.count_ones()).sum()
    }

    /// Whether the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|&w| w == 0)
    }

    /// The smallest tile id.
    #[must_use]
    pub fn first(&self) -> Option<usize> {
        self.0
            .iter()
            .enumerate()
            .find(|(_, &w)| w != 0)
            .map(|(k, w)| k * 64 + w.trailing_zeros() as usize)
    }

    /// Intersection.
    #[must_use]
    pub fn and(self, o: Self) -> Self {
        let mut r = self;
        for (a, b) in r.0.iter_mut().zip(o.0) {
            *a &= b;
        }
        r
    }

    /// Union.
    #[must_use]
    pub fn or(self, o: Self) -> Self {
        let mut r = self;
        for (a, b) in r.0.iter_mut().zip(o.0) {
            *a |= b;
        }
        r
    }

    /// Difference `self \ o`.
    #[must_use]
    pub fn minus(self, o: Self) -> Self {
        let mut r = self;
        for (a, b) in r.0.iter_mut().zip(o.0) {
            *a &= !b;
        }
        r
    }

    /// Whether the two sets share a tile.
    #[must_use]
    pub fn meets(&self, o: &Self) -> bool {
        self.0.iter().zip(o.0).any(|(a, b)| a & b != 0)
    }

    /// Tiles in ascending order.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.0.iter().enumerate().flat_map(|(k, &w)| {
            let mut bits = w;
            std::iter::from_fn(move || {
                if bits == 0 {
                    return None;
                }
                let t = bits.trailing_zeros() as usize;
                bits &= bits - 1;
                Some(k * 64 + t)
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_operations() {
        let a = TileSet::of(&[1, 70, 200]);
        let b = TileSet::of(&[70, 3]);
        assert_eq!(a.count(), 3);
        assert_eq!(a.and(b), TileSet::single(70));
        assert_eq!(a.or(b).count(), 4);
        assert_eq!(a.minus(b).iter().collect::<Vec<_>>(), vec![1, 200]);
        assert_eq!(a.first(), Some(1));
        assert!(a.meets(&b));
        assert!(TileSet::empty().first().is_none());
        assert_eq!(TileSet::first_n(300).count(), 256);
    }
}
