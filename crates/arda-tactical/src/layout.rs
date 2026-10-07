//! The tactical layout: the serialisable input to the compositor.
//!
//! WFC stages (`crates/arda-gen/src/block/`) will fill this type later; for
//! now it is built by hand in [`crate::layouts`]. Coordinates are in 5-ft
//! squares with the origin at the top-left corner and y growing south.

use crate::catalog::{AssetClass, WallRole};
use crate::error::TacticalError;
use crate::library::Library;
use serde::{Deserialize, Serialize};

/// A W × H battle map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TacticalLayout {
    /// Layout name.
    pub name: String,
    /// Width in squares.
    pub width: u32,
    /// Height in squares.
    pub height: u32,
    /// Row-major squares, `width × height` of them.
    pub squares: Vec<Square>,
    /// Wall, door, window and gate segments on square edges.
    #[serde(default)]
    pub walls: Vec<WallSegment>,
    /// Props and vegetation.
    #[serde(default)]
    pub placements: Vec<Placement>,
    /// Free-standing light sources.
    #[serde(default)]
    pub lights: Vec<LightSource>,
    /// World position of square `(0, 0)`, in world squares. When present,
    /// the compositor hashes and samples noise at world square coordinates
    /// (origin + local), so adjacent blocks' art joins seamlessly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<[i64; 2]>,
}

/// One 5-ft square.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Square {
    /// Ground type key, matching a texture's `ground` field.
    pub ground: String,
    /// Elevation in feet.
    #[serde(default)]
    pub elevation_ft: i16,
    /// Water depth in feet; zero is dry land.
    #[serde(default)]
    pub water_depth_ft: u8,
}

/// Which edge of a square a segment lies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeAxis {
    /// The north edge of square `(x, y)`: vertex `(x, y)` to `(x + 1, y)`.
    Horizontal,
    /// The west edge of square `(x, y)`: vertex `(x, y)` to `(x, y + 1)`.
    Vertical,
}

/// A wall-kit edge feature. `x` may equal `width` for vertical segments and
/// `y` may equal `height` for horizontal ones (the far map border).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WallSegment {
    /// Square column.
    pub x: u32,
    /// Square row.
    pub y: u32,
    /// Which edge.
    pub axis: EdgeAxis,
    /// Edge role: run, door, window or gate.
    pub kind: WallRole,
    /// Wall kit name.
    pub kit: String,
    /// Free tags on the edge (`docs/goal-prompts/vocabulary.md`): on a
    /// door, `locked` (needs a key or a check to open) and `secret` (drawn
    /// as a plain run of its kit; the scene marks it a secret door).
    /// Omitted when empty, so older layouts round-trip unchanged.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Edge tag: a locked door.
pub const TAG_LOCKED: &str = "locked";
/// Edge tag: a secret door, drawn as a plain wall run.
pub const TAG_SECRET: &str = "secret";

impl WallSegment {
    /// Whether the segment carries free tag `tag`.
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// The role the compositor draws: a secret door looks like a plain run.
    #[must_use]
    pub fn drawn_role(&self) -> WallRole {
        if self.kind == WallRole::Door && self.has_tag(TAG_SECRET) {
            WallRole::Run
        } else {
            self.kind
        }
    }
}

/// An asset chosen by id or by a deterministic tag query.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetRef {
    /// An asset and its `.alt<N>` takes; the compositor draws one take
    /// per placement, picked by a hash of the seed and world position. An
    /// id that names a take (`prop.barrel.alt2`) pins exactly that asset.
    Id(String),
    /// Any asset of the class carrying every tag; the compositor picks a
    /// family by hashing the seed and placement index, then a take of it
    /// as for [`AssetRef::Id`].
    Query {
        /// Restrict to a class.
        #[serde(default)]
        class: Option<AssetClass>,
        /// Required tags (controlled or free).
        tags: Vec<String>,
    },
}

/// A prop or vegetation placement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    /// What to place.
    pub asset: AssetRef,
    /// Where the asset's anchor lands, in squares (`3.5` is a square centre).
    pub x: f32,
    /// Where the asset's anchor lands, in squares.
    pub y: f32,
    /// Clockwise rotation in degrees; must be allowed by the asset.
    #[serde(default)]
    pub rotation: u16,
    /// Horizontal mirror, applied before rotation.
    #[serde(default)]
    pub mirror: bool,
}

/// A light source not attached to an asset.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LightSource {
    /// Position in squares.
    pub x: f32,
    /// Position in squares.
    pub y: f32,
    /// Bright-light radius in feet.
    pub radius_ft: u16,
    /// RGB colour.
    pub colour: [u8; 3],
}

impl TacticalLayout {
    /// A layout of one ground type everywhere.
    #[must_use]
    pub fn new(name: &str, width: u32, height: u32, ground: &str) -> Self {
        let square = Square {
            ground: ground.to_string(),
            elevation_ft: 0,
            water_depth_ft: 0,
        };
        Self {
            name: name.to_string(),
            width,
            height,
            squares: vec![square; width as usize * height as usize],
            walls: Vec::new(),
            placements: Vec::new(),
            lights: Vec::new(),
            origin: None,
        }
    }

    /// The world square of local square `(0, 0)`: `origin`, or `(0, 0)`.
    #[must_use]
    pub fn world_origin(&self) -> (i64, i64) {
        self.origin.map_or((0, 0), |[x, y]| (x, y))
    }

    /// The square at `(x, y)`, clamped to the map.
    #[must_use]
    pub fn square(&self, x: i64, y: i64) -> &Square {
        let cx = x.clamp(0, i64::from(self.width) - 1);
        let cy = y.clamp(0, i64::from(self.height) - 1);
        let i = usize::try_from(cy * i64::from(self.width) + cx).unwrap_or(0);
        &self.squares[i]
    }

    /// Mutable square access; `None` outside the map.
    pub fn square_mut(&mut self, x: u32, y: u32) -> Option<&mut Square> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.squares
            .get_mut(y as usize * self.width as usize + x as usize)
    }

    /// Parses a layout from JSON.
    ///
    /// # Errors
    /// Malformed JSON or schema violations.
    pub fn from_json(json: &str) -> Result<Self, TacticalError> {
        serde_json::from_str(json).map_err(TacticalError::LayoutJson)
    }

    /// Serialises as pretty JSON.
    ///
    /// # Errors
    /// Serialisation failure.
    pub fn to_json(&self) -> Result<String, TacticalError> {
        serde_json::to_string_pretty(self).map_err(TacticalError::LayoutJson)
    }

    fn fail(&self, message: String) -> TacticalError {
        TacticalError::Layout {
            layout: self.name.clone(),
            message,
        }
    }

    /// Checks the layout against itself and a library: sizes, ground keys,
    /// wall kits and roles, placements and rotations.
    ///
    /// # Errors
    /// The first inconsistency found.
    pub fn check(&self, lib: &Library) -> Result<(), TacticalError> {
        if self.width == 0 || self.height == 0 {
            return Err(self.fail("empty map".into()));
        }
        if self.squares.len() != self.width as usize * self.height as usize {
            return Err(self.fail(format!(
                "{} squares for a {}x{} map",
                self.squares.len(),
                self.width,
                self.height
            )));
        }
        for sq in &self.squares {
            if lib.textures(&sq.ground).is_empty() {
                return Err(self.fail(format!("no texture for ground `{}`", sq.ground)));
            }
        }
        if self.squares.iter().any(|s| s.water_depth_ft > 0) {
            for key in [crate::compose::WATER_SHALLOW, crate::compose::WATER_DEEP] {
                if lib.textures(key).is_empty() {
                    return Err(self.fail(format!("water needs a `{key}` texture")));
                }
            }
        }
        for w in &self.walls {
            let (mx, my) = match w.axis {
                EdgeAxis::Horizontal => (self.width - 1, self.height),
                EdgeAxis::Vertical => (self.width, self.height - 1),
            };
            if w.x > mx || w.y > my {
                return Err(self.fail(format!("wall segment ({}, {}) is off the map", w.x, w.y)));
            }
            if !w.kind.is_edge() {
                return Err(self.fail(format!(
                    "segment kind {:?} is a joint, not an edge piece",
                    w.kind
                )));
            }
            if lib.wall_pieces(&w.kit, w.kind).is_empty() {
                return Err(self.fail(format!("kit `{}` has no {:?} piece", w.kit, w.kind)));
            }
        }
        for (i, p) in self.placements.iter().enumerate() {
            let mut all = crate::compose::candidates(lib, &p.asset);
            if all.is_empty() {
                return Err(self.fail(format!("placement {i}: {:?} matches no asset", p.asset)));
            }
            // An id draws only the takes of its family that fit, so one
            // fitting take is enough; a query's matches must all fit.
            if let AssetRef::Id(_) = p.asset {
                let fitting: Vec<_> = all
                    .iter()
                    .copied()
                    .filter(|a| crate::compose::variants::fits(a, p.rotation, p.mirror))
                    .collect();
                if !fitting.is_empty() {
                    all = fitting;
                }
            }
            for asset in all {
                if !asset.rotations.contains(&p.rotation) {
                    return Err(self.fail(format!(
                        "placement {i}: {} does not allow rotation {}",
                        asset.id, p.rotation
                    )));
                }
                if p.mirror && !asset.mirror {
                    return Err(
                        self.fail(format!("placement {i}: {} may not be mirrored", asset.id))
                    );
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_round_trips_through_json() {
        let mut l = TacticalLayout::new("t", 3, 2, "grass");
        l.walls.push(WallSegment {
            x: 1,
            y: 0,
            axis: EdgeAxis::Vertical,
            kind: WallRole::Door,
            kit: "stone".into(),
            tags: Vec::new(),
        });
        l.placements.push(Placement {
            asset: AssetRef::Id("prop.barrel".into()),
            x: 0.5,
            y: 1.5,
            rotation: 90,
            mirror: true,
        });
        l.placements.push(Placement {
            asset: AssetRef::Query {
                class: Some(AssetClass::Vegetation),
                tags: vec!["tree".into()],
            },
            x: 2.0,
            y: 1.0,
            rotation: 0,
            mirror: false,
        });
        l.lights.push(LightSource {
            x: 1.0,
            y: 1.0,
            radius_ft: 20,
            colour: [255, 200, 120],
        });
        if let Some(sq) = l.square_mut(2, 1) {
            sq.water_depth_ft = 4;
            sq.elevation_ft = -2;
        }
        let back = TacticalLayout::from_json(&l.to_json().unwrap()).unwrap();
        assert_eq!(back, l);
    }

    #[test]
    fn square_access_clamps_to_the_map() {
        let l = TacticalLayout::new("t", 2, 2, "dirt");
        assert_eq!(l.square(-5, 9).ground, "dirt");
    }

    #[test]
    fn origin_is_optional_and_omitted_when_absent() {
        // Vocabulary I10; review round 1 open item 13: a layout carrying an
        // origin used to be refused by deny_unknown_fields.
        let mut l = TacticalLayout::new("t", 2, 2, "grass");
        let plain = l.to_json().unwrap();
        assert!(!plain.contains("origin"), "{plain}");
        assert_eq!(l.world_origin(), (0, 0));
        l.origin = Some([128, -64]);
        let json = l.to_json().unwrap();
        let back = TacticalLayout::from_json(&json).unwrap();
        assert_eq!(back.origin, Some([128, -64]));
        assert_eq!(back.world_origin(), (128, -64));
        let old = TacticalLayout::from_json(&plain).unwrap();
        assert_eq!(old.origin, None);
    }
}
