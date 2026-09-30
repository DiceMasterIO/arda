//! The rules sidecar: per-square SRD data from the terrain generator.
//!
//! `TacticalLayout::Square` is `deny_unknown_fields`, so per-square rules
//! travel beside the layout in their own JSON (see
//! `docs/goal-prompts/vocabulary.md`, "Placement queries"). The sidecar has
//! the layout's width and height and one row-major cell per square. Every
//! cell field is optional; a missing field keeps the value derived from the
//! layout and its assets. The merge is documented field by field on
//! [`RulesCell`] and in `crates/arda-scene/README.md`.
//!
//! Format 2 (logic/12 §scene-sidecar) is the one per-square rules schema of
//! the product (convention I9): arda-refine, arda-ways, arda-fields and
//! arda-town all write it. Layer-specific extras that the scene does not
//! interpret (a ways `feature`, `road_class` or `deck_elevation_ft`, a
//! field's `crop`) travel in each cell's optional `ext` map, and low edge
//! features that are not wall-kit walls (bridge parapets, hedges, drystone
//! walls) in the top-level `edges`. Several owners' sidecars combine with
//! [`RulesSidecar::overlay`] in reservation order (logic/09 §reservations).

use crate::error::SceneError;
use crate::types::CoverLevel;
use arda_tactical::layout::EdgeAxis;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Layer-specific extras on one square, by key (for example `feature`,
/// `road_class`, `deck_elevation_ft`). Keys are snake_case; values are
/// plain JSON. The scene passes them through without interpreting them.
pub type RulesExt = BTreeMap<String, serde_json::Value>;

/// The sidecar schema version this crate reads.
pub const SIDECAR_FORMAT_VERSION: u32 = 2;

/// Per-square rules produced beside a layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulesSidecar {
    /// Schema version; must equal [`SIDECAR_FORMAT_VERSION`].
    pub format_version: u32,
    /// Width in squares; must equal the layout's.
    pub width: u32,
    /// Height in squares; must equal the layout's.
    pub height: u32,
    /// Row-major cells, `width × height` of them.
    pub squares: Vec<RulesCell>,
    /// Rules for low edge features that are not wall-kit walls (logic/12
    /// §scene-sidecar): parapets, retaining walls, hedges and field walls.
    /// Each overrides the matching layout wall edge's blocking.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edges: Vec<EdgeRule>,
}

/// Why a rules edge exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeRole {
    /// Bridge parapet: blocks movement, not sight.
    Parapet,
    /// Low retaining wall at a bench's outer edge.
    Retaining,
    /// A building's wall, door or window.
    Building,
    /// A field boundary: hedge, drystone wall, wattle fence.
    Boundary,
    /// A gate in a field boundary.
    Gate,
}

/// Movement and sight rules for one unit edge, in layout edge coordinates
/// (as `WallSegment`: `x`, `y` and `axis`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeRule {
    /// Square column.
    pub x: u32,
    /// Square row.
    pub y: u32,
    /// Which edge of the square.
    pub axis: EdgeAxis,
    /// Why it is there.
    pub role: EdgeRole,
    /// Blocks movement across the edge.
    pub blocks_movement: bool,
    /// Blocks line of sight across the edge.
    pub blocks_sight: bool,
    /// Cover it grants to a creature behind it.
    #[serde(default)]
    pub cover: CoverLevel,
}

/// One square's rules. `None` (or an absent field) means "no opinion".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulesCell {
    /// Overrides difficult terrain: `true` sets it (undergrowth, scree, mud),
    /// `false` clears what props implied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficult: Option<bool>,
    /// Overrides the layout's water depth in feet (0 is dry, 1–4 wading,
    /// 5 or more swimming).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub water_depth_ft: Option<u8>,
    /// Cover from terrain (trees, boulders). Merged with prop cover by taking
    /// the stronger: the SRD applies only the most protective degree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<CoverLevel>,
    /// Overrides whether the square blocks sight (dense canopy clusters).
    /// `true` makes it heavily obscured; `false` clears asset-derived heavy
    /// obscurement (light canopy obscurement stays).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocks_sight: Option<bool>,
    /// Lightly obscured (moderate foliage, reeds), logic/09 §rules-sidecar.
    /// Never downgrades a heavily obscured square.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lightly_obscured: Option<bool>,
    /// Overrides whether the square blocks movement: `true` makes it
    /// impassable (cliff face, thicket), `false` clears prop blocking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocks_movement: Option<bool>,
    /// Overrides whether a walkable deck (bridge, jetty) spans the square:
    /// `true` means it is walked, not waded or swum, whatever the depth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deck: Option<bool>,
    /// Layer-specific extras the scene does not interpret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ext: Option<RulesExt>,
}

impl RulesCell {
    /// Merges `upper` (a higher-precedence owner) over `self` (logic/12
    /// §scene-sidecar): each field `upper` states wins, except that `cover`
    /// takes the stronger and `difficult` is true if either sets it; `ext`
    /// maps are unioned with `upper`'s keys winning.
    pub fn merge_from(&mut self, upper: &Self) {
        self.difficult = match (self.difficult, upper.difficult) {
            (Some(a), Some(b)) => Some(a || b),
            (a, b) => b.or(a),
        };
        self.cover = match (self.cover, upper.cover) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => b.or(a),
        };
        self.water_depth_ft = upper.water_depth_ft.or(self.water_depth_ft);
        self.blocks_sight = upper.blocks_sight.or(self.blocks_sight);
        self.lightly_obscured = upper.lightly_obscured.or(self.lightly_obscured);
        self.blocks_movement = upper.blocks_movement.or(self.blocks_movement);
        self.deck = upper.deck.or(self.deck);
        if let Some(up) = &upper.ext {
            let mine = self.ext.get_or_insert_with(RulesExt::new);
            for (k, v) in up {
                mine.insert(k.clone(), v.clone());
            }
        }
    }

    /// Sets one `ext` key.
    pub fn set_ext(&mut self, key: &str, value: serde_json::Value) {
        self.ext
            .get_or_insert_with(RulesExt::new)
            .insert(key.to_owned(), value);
    }
}

impl RulesSidecar {
    /// An all-`None` sidecar.
    #[must_use]
    pub fn empty(width: u32, height: u32) -> Self {
        Self {
            format_version: SIDECAR_FORMAT_VERSION,
            width,
            height,
            squares: vec![RulesCell::default(); width as usize * height as usize],
            edges: Vec::new(),
        }
    }

    /// Cell access; `None` outside.
    #[must_use]
    pub fn cell(&self, x: u32, y: u32) -> Option<&RulesCell> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.squares
            .get(y as usize * self.width as usize + x as usize)
    }

    /// Overlays a higher-precedence owner's sidecar of the same size onto
    /// this one, on the squares `owned` selects (row-major index), by
    /// [`RulesCell::merge_from`]. `upper`'s edges replace this sidecar's
    /// edges on the same unit edge. Owners are applied in rising precedence
    /// (logic/09 §reservations), so the last writer is the strongest.
    ///
    /// # Errors
    /// [`SceneError::Sidecar`] when the sizes differ.
    pub fn overlay(
        &mut self,
        upper: &Self,
        owned: impl Fn(usize) -> bool,
    ) -> Result<(), SceneError> {
        upper.check(self.width, self.height)?;
        for (i, (mine, up)) in self.squares.iter_mut().zip(&upper.squares).enumerate() {
            if owned(i) {
                mine.merge_from(up);
            }
        }
        for e in &upper.edges {
            self.edges
                .retain(|m| (m.x, m.y, m.axis) != (e.x, e.y, e.axis));
            self.edges.push(e.clone());
        }
        self.edges.sort_by_key(|e| (e.axis, e.y, e.x));
        Ok(())
    }

    /// Mutable cell access; `None` outside.
    pub fn cell_mut(&mut self, x: u32, y: u32) -> Option<&mut RulesCell> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.squares
            .get_mut(y as usize * self.width as usize + x as usize)
    }

    /// Parses a sidecar from JSON.
    ///
    /// # Errors
    /// Malformed JSON or schema violations.
    pub fn from_json(json: &str) -> Result<Self, SceneError> {
        Ok(serde_json::from_str(json)?)
    }

    /// Checks the version and that the sidecar fits a `width × height` map.
    ///
    /// # Errors
    /// [`SceneError::Sidecar`] naming the mismatch.
    pub fn check(&self, width: u32, height: u32) -> Result<(), SceneError> {
        if self.format_version != SIDECAR_FORMAT_VERSION {
            return Err(SceneError::Sidecar(format!(
                "format_version {} (expected {SIDECAR_FORMAT_VERSION})",
                self.format_version
            )));
        }
        if (self.width, self.height) != (width, height) {
            return Err(SceneError::Sidecar(format!(
                "{}x{} sidecar for a {width}x{height} layout",
                self.width, self.height
            )));
        }
        if self.squares.len() != width as usize * height as usize {
            return Err(SceneError::Sidecar(format!(
                "{} cells for a {width}x{height} layout",
                self.squares.len()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_cells_parse() {
        let json = r#"{"format_version":2,"width":2,"height":1,"squares":[
            {}, {"difficult":true,"cover":"full","blocks_sight":true}]}"#;
        let s = RulesSidecar::from_json(json).unwrap();
        assert_eq!(s.squares[0], RulesCell::default());
        assert_eq!(s.squares[1].cover, Some(CoverLevel::Total));
        assert!(s.check(2, 1).is_ok());
        assert!(s.check(1, 2).is_err());
        let v1 = json.replace("\"format_version\":2", "\"format_version\":1");
        assert!(RulesSidecar::from_json(&v1).unwrap().check(2, 1).is_err());
    }

    #[test]
    fn overlay_merges_by_precedence_and_keeps_ext() {
        let mut low = RulesSidecar::empty(2, 1);
        low.squares[0].water_depth_ft = Some(6);
        low.squares[0].cover = Some(CoverLevel::Half);
        low.squares[1].difficult = Some(true);
        let mut up = RulesSidecar::empty(2, 1);
        up.squares[0].deck = Some(true);
        up.squares[0].cover = Some(CoverLevel::None);
        up.squares[0].set_ext("feature", serde_json::json!("bridge"));
        up.squares[1].difficult = Some(false);
        up.edges.push(EdgeRule {
            x: 0,
            y: 0,
            axis: EdgeAxis::Horizontal,
            role: EdgeRole::Parapet,
            blocks_movement: true,
            blocks_sight: false,
            cover: CoverLevel::Half,
        });
        low.overlay(&up, |_| true).unwrap();
        let c = &low.squares[0];
        assert_eq!((c.water_depth_ft, c.deck), (Some(6), Some(true)));
        assert_eq!(c.cover, Some(CoverLevel::Half));
        assert_eq!(c.ext.as_ref().unwrap()["feature"], "bridge");
        assert_eq!(low.squares[1].difficult, Some(true));
        assert_eq!(low.edges.len(), 1);
        let json = serde_json::to_string(&low).unwrap();
        assert_eq!(RulesSidecar::from_json(&json).unwrap(), low);
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let json = r#"{"format_version":2,"width":1,"height":1,"squares":[{"lava":true}]}"#;
        assert!(RulesSidecar::from_json(json).is_err());
    }
}
