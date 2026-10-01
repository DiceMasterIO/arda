//! Render identity: what a tactical render is anchored to, its compositor
//! seed and its cache key (goal 67; vocabulary.md canonical conventions I10,
//! I17).
//!
//! A render is anchored either to its world origin (a world-derived block,
//! in world squares, 64 per cell) or, for the built-in demo layouts and
//! posted layouts without an origin, to the layout name.
//!
//! World-anchored renders share one compositor seed per world (logic/11
//! §seam-art 3): the layout carries its `origin`, so the compositor hashes
//! world square coordinates, and neighbouring blocks draw the same art on
//! their shared edge. Name-anchored renders keep a seed per name, so
//! different demo layouts differ.

use std::fmt;

/// Squares per world cell (convention I2: a cell is 64 × 64 squares).
pub const SQUARES_PER_CELL: i64 = 64;

/// What a render is anchored to.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Anchor {
    /// The layout's top-left square in world squares.
    Origin(i64, i64),
    /// A layout with no world position, by name.
    Name(String),
}

impl Anchor {
    /// The origin of the block at global cell `(gx, gy)`.
    #[must_use]
    pub fn of_cell(gx: u32, gy: u32) -> Self {
        Self::Origin(
            i64::from(gx) * SQUARES_PER_CELL,
            i64::from(gy) * SQUARES_PER_CELL,
        )
    }

    /// Parses `?origin=X,Y` (world squares).
    ///
    /// # Errors
    /// A message when the text is not two comma-separated integers.
    pub fn parse_origin(text: &str) -> Result<Self, String> {
        let (x, y) = text
            .split_once(',')
            .ok_or_else(|| format!("origin must be X,Y in world squares, not {text:?}"))?;
        let n = |v: &str| {
            v.trim()
                .parse::<i64>()
                .map_err(|_| format!("origin must be X,Y in world squares, not {text:?}"))
        };
        Ok(Self::Origin(n(x)?, n(y)?))
    }
}

impl fmt::Display for Anchor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Origin(x, y) => write!(f, "origin={x},{y}"),
            Self::Name(n) => write!(f, "name={n}"),
        }
    }
}

/// The compositor seed for `anchor` in the world of `world_seed`: one per
/// world for origin anchors (logic/11 §seam-art 3), one per name otherwise.
#[must_use]
pub fn render_seed(world_seed: u64, anchor: &Anchor) -> u64 {
    let mut h = blake3::Hasher::new();
    h.update(b"arda-server/tactical-render-seed/v1");
    h.update(&world_seed.to_le_bytes());
    match anchor {
        Anchor::Origin(..) => {
            h.update(b"W");
        }
        Anchor::Name(n) => {
            h.update(b"N");
            h.update(n.as_bytes());
        }
    }
    let mut first = [0_u8; 8];
    first.copy_from_slice(&h.finalize().as_bytes()[..8]);
    u64::from_le_bytes(first)
}

/// Cache identity of one render: world seed, anchor, layout hash, ppsq,
/// grid, crop, catalogue version and the opt-in world grade.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RenderKey {
    /// Seed of the served world.
    pub world_seed: u64,
    /// World origin or name the render is anchored to.
    pub anchor: Anchor,
    /// BLAKE3 of the layout's canonical JSON.
    pub layout: [u8; 32],
    /// Output pixels per square.
    pub ppsq: u32,
    /// Whether the grid is drawn.
    pub grid: bool,
    /// Catalogue `library_version`.
    pub library_version: String,
    /// `Some([x, y, w, h])` in squares: render the whole layout (the block
    /// plus its apron, logic/11 §seam-art 2), then keep only this part.
    pub crop: Option<[u32; 4]>,
    /// Pull ground and water toward the world map's colours (goal 49,
    /// opt-in `?world_grade=1`).
    pub world_grade: bool,
}

impl RenderKey {
    /// The compositor seed of this render.
    #[must_use]
    pub fn seed(&self) -> u64 {
        render_seed(self.world_seed, &self.anchor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_depend_on_world_and_anchor_and_are_stable() {
        let a = Anchor::Name("riverside".into());
        assert_eq!(render_seed(42, &a), render_seed(42, &a));
        assert_ne!(render_seed(42, &a), render_seed(43, &a));
        // Every world block shares the world's render seed; its layout's
        // origin makes the art differ from place to place.
        assert_eq!(
            render_seed(42, &Anchor::of_cell(1, 2)),
            render_seed(42, &Anchor::of_cell(2, 1))
        );
        assert_ne!(
            render_seed(42, &Anchor::of_cell(1, 2)),
            render_seed(43, &Anchor::of_cell(1, 2))
        );
        assert_eq!(Anchor::of_cell(1, 2), Anchor::Origin(64, 128));
    }

    #[test]
    fn origins_parse_as_two_integers() {
        assert_eq!(Anchor::parse_origin("-3, 7"), Ok(Anchor::Origin(-3, 7)));
        assert!(Anchor::parse_origin("3").is_err());
        assert!(Anchor::parse_origin("a,1").is_err());
    }
}
