//! A query-ready view of a scene: per-edge wall rules and square lookups.
//!
//! Build one [`SceneIndex`] per scene state and reuse it for many queries;
//! rebuild it after opening or closing a door.

use crate::types::{CoverLevel, Movement, Obscurement, Scene, Sq};
use crate::walls::unit_edges;
use arda_tactical::layout::EdgeAxis;

/// What a wall edge does to sight, movement and cover.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EdgeRule {
    /// Blocks line of sight.
    pub sight: bool,
    /// Blocks movement.
    pub movement: bool,
    /// Cover granted across it when it does not block sight.
    pub cover: CoverLevel,
}

/// Lookups over a scene for LOS, cover and path queries.
#[derive(Debug, Clone)]
pub struct SceneIndex<'a> {
    /// The scene.
    pub scene: &'a Scene,
    h_edges: Vec<EdgeRule>,
    v_edges: Vec<EdgeRule>,
}

impl<'a> SceneIndex<'a> {
    /// Indexes a scene with its recorded rule options.
    #[must_use]
    pub fn new(scene: &'a Scene) -> Self {
        let (w, h) = (scene.width as usize, scene.height as usize);
        let mut s = Self {
            scene,
            h_edges: vec![EdgeRule::default(); w * (h + 1)],
            v_edges: vec![EdgeRule::default(); (w + 1) * h],
        };
        for wall in &scene.walls {
            for (axis, x, y) in unit_edges(wall) {
                if let Some(e) = s.edge_slot(axis, x, y) {
                    e.sight |= wall.blocks_sight;
                    e.movement |= wall.blocks_movement;
                    e.cover = e.cover.max(wall.cover);
                }
            }
        }
        s
    }

    fn edge_pos(&self, axis: EdgeAxis, x: u32, y: u32) -> Option<usize> {
        let (w, h) = (self.scene.width, self.scene.height);
        match axis {
            EdgeAxis::Horizontal if x < w && y <= h => Some(y as usize * w as usize + x as usize),
            EdgeAxis::Vertical if x <= w && y < h => {
                Some(y as usize * (w as usize + 1) + x as usize)
            }
            _ => None,
        }
    }

    fn edge_slot(&mut self, axis: EdgeAxis, x: u32, y: u32) -> Option<&mut EdgeRule> {
        let i = self.edge_pos(axis, x, y)?;
        match axis {
            EdgeAxis::Horizontal => self.h_edges.get_mut(i),
            EdgeAxis::Vertical => self.v_edges.get_mut(i),
        }
    }

    /// The rule of an edge in layout terms (north edge of `(x, y)` for
    /// horizontal, west edge for vertical); default outside the map.
    #[must_use]
    pub fn edge(&self, axis: EdgeAxis, x: i64, y: i64) -> EdgeRule {
        let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
            return EdgeRule::default();
        };
        let Some(i) = self.edge_pos(axis, x, y) else {
            return EdgeRule::default();
        };
        let v = match axis {
            EdgeAxis::Horizontal => self.h_edges.get(i),
            EdgeAxis::Vertical => self.v_edges.get(i),
        };
        v.copied().unwrap_or_default()
    }

    /// Whether a square is inside the map.
    #[must_use]
    pub fn inside(&self, x: i64, y: i64) -> bool {
        x >= 0 && y >= 0 && x < i64::from(self.scene.width) && y < i64::from(self.scene.height)
    }

    fn at(&self, x: i64, y: i64) -> Option<usize> {
        self.inside(x, y)
            .then(|| usize::try_from(y * i64::from(self.scene.width) + x).ok())
            .flatten()
    }

    /// Movement of a square; impassable outside.
    #[must_use]
    pub fn movement(&self, x: i64, y: i64) -> Movement {
        self.at(x, y)
            .and_then(|i| self.scene.movement.get(i).copied())
            .unwrap_or(Movement::Impassable)
    }

    /// Whether a square blocks sight (heavily obscured).
    #[must_use]
    pub fn opaque(&self, x: i64, y: i64) -> bool {
        self.at(x, y)
            .and_then(|i| self.scene.obscured.get(i).copied())
            .is_some_and(|o| o == Obscurement::Heavy)
    }

    /// Cover of a square; none outside.
    #[must_use]
    pub fn cover(&self, x: i64, y: i64) -> CoverLevel {
        self.at(x, y)
            .and_then(|i| self.scene.cover.get(i).copied())
            .unwrap_or_default()
    }

    /// Climb mask of a square.
    #[must_use]
    pub fn climb(&self, x: i64, y: i64) -> u8 {
        self.at(x, y)
            .and_then(|i| self.scene.climb.get(i).copied())
            .unwrap_or(0)
    }

    /// Grid distance in feet between two squares: 5 ft per square, and a
    /// diagonal step counts as one square (5 ft), the SRD default.
    #[must_use]
    pub fn distance_ft(&self, a: Sq, b: Sq) -> u32 {
        5 * a.0.abs_diff(b.0).max(a.1.abs_diff(b.1))
    }
}

impl Scene {
    /// A query index over this scene.
    #[must_use]
    pub fn queries(&self) -> SceneIndex<'_> {
        SceneIndex::new(self)
    }
}
