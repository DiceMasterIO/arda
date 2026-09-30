//! Run-length encoding for per-square layers.
//!
//! A [`Grid`] is a row-major `width × height` layer. In JSON it is written as
//! runs, `[[count, value], ...]`, which keeps big uniform maps small: a
//! 64 × 64 all-normal movement layer is `[[4096, "normal"]]`.

use serde::de::Error as _;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Upper bound on the squares a decoded grid may expand to (a 4 × 4 window
/// of 64 × 64 blocks is 65 536; this leaves generous headroom).
pub const MAX_SQUARES: usize = 1 << 22;

/// A row-major per-square layer, serialised run-length encoded.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Grid<T>(pub Vec<T>);

impl<T: Clone> Grid<T> {
    /// A grid of `len` copies of `value`.
    #[must_use]
    pub fn filled(len: usize, value: T) -> Self {
        Self(vec![value; len])
    }
}

impl<T> Grid<T> {
    /// Number of squares.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the grid is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The value at a row-major index.
    #[must_use]
    pub fn get(&self, i: usize) -> Option<&T> {
        self.0.get(i)
    }
}

impl<T: Serialize + PartialEq> Serialize for Grid<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut runs: Vec<(u32, &T)> = Vec::new();
        for v in &self.0 {
            match runs.last_mut() {
                Some((n, last)) if *last == v => *n += 1,
                _ => runs.push((1, v)),
            }
        }
        let mut seq = s.serialize_seq(Some(runs.len()))?;
        for r in &runs {
            seq.serialize_element(r)?;
        }
        seq.end()
    }
}

impl<'de, T: Deserialize<'de> + Clone> Deserialize<'de> for Grid<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let runs: Vec<(u32, T)> = Vec::deserialize(d)?;
        let mut out = Vec::new();
        for (n, v) in runs {
            let n = n as usize;
            if out.len() + n > MAX_SQUARES {
                return Err(D::Error::custom("grid expands past MAX_SQUARES"));
            }
            out.resize(out.len() + n, v);
        }
        Ok(Self(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_round_trips_as_runs() {
        let g = Grid(vec![1u8, 1, 1, 2, 2, 1]);
        let json = serde_json::to_string(&g).unwrap();
        assert_eq!(json, "[[3,1],[2,2],[1,1]]");
        let back: Grid<u8> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, g);
    }

    #[test]
    fn oversized_grids_are_refused() {
        let json = format!("[[{},0]]", MAX_SQUARES + 1);
        assert!(serde_json::from_str::<Grid<u8>>(&json).is_err());
    }
}
