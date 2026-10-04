//! The tactical asset catalogue format (goal 58).
//!
//! A library is a directory holding `catalog.json` plus the image files it
//! names. The JSON is the contract with the art library, so every field here
//! is documented in `crates/arda-tactical/README.md`; change both together.

use serde::{Deserialize, Serialize};

/// The catalogue schema version this crate reads and writes.
pub const FORMAT_VERSION: u32 = 1;

/// A whole asset library: metadata plus every asset record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    /// Schema version; must equal [`FORMAT_VERSION`].
    pub format_version: u32,
    /// Short library name, e.g. `placeholder`.
    pub library: String,
    /// Version of the art itself; part of the render's identity.
    pub library_version: String,
    /// Source art resolution: pixels per 5-ft square.
    pub pixels_per_square: u32,
    /// Allowed values for the controlled tag lists.
    #[serde(default)]
    pub vocabulary: TagVocabulary,
    /// Every asset in the library.
    pub assets: Vec<Asset>,
}

/// The controlled vocabularies that asset tags are checked against.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TagVocabulary {
    /// Biomes, e.g. `temperate`.
    #[serde(default)]
    pub biome: Vec<String>,
    /// Cultures, e.g. `human`.
    #[serde(default)]
    pub culture: Vec<String>,
    /// Wealth levels, e.g. `poor`.
    #[serde(default)]
    pub wealth: Vec<String>,
    /// Building functions, e.g. `warehouse`.
    #[serde(default)]
    pub function: Vec<String>,
}

/// One asset record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Asset {
    /// Unique, stable identifier, e.g. `prop.barrel`.
    pub id: String,
    /// Asset class.
    pub class: AssetClass,
    /// Image path relative to the library directory (PNG, RGBA).
    pub image: String,
    /// Size in 5-ft squares, before rotation.
    pub footprint: Footprint,
    /// Pivot in squares from the image's top-left corner; defaults to the centre.
    #[serde(default)]
    pub anchor: Option<Anchor>,
    /// Allowed clockwise rotations in degrees: any of 0, 90, 180, 270.
    #[serde(default = "default_rotations")]
    pub rotations: Vec<u16>,
    /// Whether a horizontal mirror may be applied.
    #[serde(default)]
    pub mirror: bool,
    /// Classification tags.
    #[serde(default)]
    pub tags: Tags,
    /// Where the asset may be placed.
    #[serde(default)]
    pub placement: PlacementRules,
    /// Blocks line of sight.
    #[serde(default)]
    pub blocks_sight: bool,
    /// Blocks movement through the squares it covers.
    #[serde(default)]
    pub blocks_movement: bool,
    /// Its squares are difficult terrain.
    #[serde(default)]
    pub difficult_terrain: bool,
    /// Cover granted to a creature behind it.
    #[serde(default)]
    pub cover: Cover,
    /// Light emitted, if any.
    #[serde(default)]
    pub light: Option<Light>,
    /// Render layer.
    pub layer: Layer,
    /// Order within the layer; higher draws later.
    #[serde(default)]
    pub z: i16,
    /// Whether the lighting pass casts a drop shadow for it.
    #[serde(default)]
    pub casts_shadow: bool,
    /// Height hint in feet; sets drop-shadow length.
    #[serde(default)]
    pub height_ft: u16,
    /// Texture tiles seamlessly (ground and water); checked by the seam rule.
    #[serde(default)]
    pub tileable: bool,
    /// Ground or water type key this texture paints, e.g. `grass`.
    #[serde(default)]
    pub ground: Option<String>,
    /// Wall-kit membership, for `wall` assets.
    #[serde(default)]
    pub wall: Option<WallPiece>,
    /// Where the art came from (tool, prompt, author).
    pub provenance: String,
    /// Licence under which the art may be used.
    pub licence: String,
}

fn default_rotations() -> Vec<u16> {
    vec![0]
}

impl Asset {
    /// The anchor, defaulting to the footprint centre.
    #[must_use]
    pub fn anchor_or_centre(&self) -> Anchor {
        self.anchor.unwrap_or(self.footprint_centre())
    }

    /// The centre of the footprint, in squares.
    #[must_use]
    pub fn footprint_centre(&self) -> Anchor {
        Anchor {
            x: self.footprint.w as f32 / 2.0,
            y: self.footprint.h as f32 / 2.0,
        }
    }
}

/// The asset classes of goal 59.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetClass {
    /// Tileable ground texture.
    Ground,
    /// Wall-kit piece, drawn on square edges and vertices.
    Wall,
    /// Furniture, goods, vehicles and structures.
    Prop,
    /// Trees, bushes, reeds and rocks.
    Vegetation,
    /// Tileable water texture.
    Water,
    /// Any class this version does not know; rejected by the validator.
    #[serde(other)]
    Unknown,
}

impl AssetClass {
    /// Ground and water are opaque tileable textures; the rest are cut-outs.
    #[must_use]
    pub fn is_texture(self) -> bool {
        matches!(self, Self::Ground | Self::Water)
    }
}

/// Footprint in squares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Footprint {
    /// Width in squares.
    pub w: u32,
    /// Height in squares.
    pub h: u32,
}

/// A pivot point, in squares from the image's top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    /// Squares right of the image's left edge.
    pub x: f32,
    /// Squares below the image's top edge.
    pub y: f32,
}

/// Classification tags. The first four lists are controlled by the
/// catalogue's [`TagVocabulary`]; `free` is unchecked.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tags {
    /// Biomes the asset suits.
    #[serde(default)]
    pub biome: Vec<String>,
    /// Cultures the asset suits.
    #[serde(default)]
    pub culture: Vec<String>,
    /// Wealth levels the asset suits.
    #[serde(default)]
    pub wealth: Vec<String>,
    /// Building functions the asset suits.
    #[serde(default)]
    pub function: Vec<String>,
    /// Free-form tags, e.g. `container`.
    #[serde(default)]
    pub free: Vec<String>,
}

/// Placement rules for dressing (goal 64 consumes these).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementRules {
    /// Must stand on water (boats).
    #[serde(default)]
    pub on_water: bool,
    /// Must touch a wall edge.
    #[serde(default)]
    pub against_wall: bool,
    /// Prefers squares next to a road.
    #[serde(default)]
    pub near_road: bool,
    /// Empty squares required around the footprint.
    #[serde(default)]
    pub clearance_squares: u8,
    /// Ground types it may stand on; empty means any.
    #[serde(default)]
    pub ground: Vec<String>,
}

/// SRD cover levels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cover {
    /// No cover.
    #[default]
    None,
    /// Half cover (+2 AC).
    Half,
    /// Three-quarters cover (+5 AC).
    ThreeQuarters,
    /// Total cover. Serialised as `total` (the SRD term); `full` is
    /// accepted on input as an alias.
    #[serde(rename = "total", alias = "full")]
    Full,
}

/// A light source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Light {
    /// Bright-light radius in feet.
    pub radius_ft: u16,
    /// RGB colour.
    pub colour: [u8; 3],
}

/// Render layers, drawn in declaration order (goal 62).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer {
    /// Ground textures.
    Ground,
    /// Water textures.
    Water,
    /// Floors over ground or water: docks, bridges, rugs.
    Floor,
    /// Props and low vegetation.
    Prop,
    /// Wall kits.
    Wall,
    /// Tree canopies and roofs, drawn last.
    Canopy,
}

/// Membership of a wall kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WallPiece {
    /// Kit name, e.g. `stone`.
    pub kit: String,
    /// The piece's role in the kit.
    pub role: WallRole,
}

/// Wall-kit piece roles. Edge pieces span one square edge, drawn horizontally
/// (west to east) through the image centre. Joint pieces sit on a grid vertex
/// at the image centre; their canonical arms are listed per role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallRole {
    /// Plain wall along one edge.
    Run,
    /// Doorway along one edge.
    Door,
    /// Wall with a window along one edge.
    Window,
    /// Wide gate along one edge.
    Gate,
    /// Joint with arms east and south.
    Corner,
    /// Joint with arms east, south and west.
    Tee,
    /// Joint with arms in all four directions.
    Cross,
    /// Joint with one arm, east.
    End,
    /// Optional joint on a straight run, arms east and west.
    Post,
}

impl WallRole {
    /// Edge pieces span an edge; the rest are vertex joints.
    #[must_use]
    pub fn is_edge(self) -> bool {
        matches!(self, Self::Run | Self::Door | Self::Window | Self::Gate)
    }

    /// Roles every kit must provide.
    pub const REQUIRED: [Self; 5] = [Self::Run, Self::Corner, Self::Tee, Self::Cross, Self::End];
}

/// Parses a catalogue from JSON text.
///
/// # Errors
/// Returns the serde error for malformed JSON or schema violations.
pub fn parse(json: &str) -> Result<Catalog, serde_json::Error> {
    serde_json::from_str(json)
}

/// Serialises a catalogue as pretty JSON.
///
/// # Errors
/// Returns the serde error if serialisation fails.
pub fn to_json(catalog: &Catalog) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_serialises_as_total_and_accepts_full() {
        assert_eq!(serde_json::to_string(&Cover::Full).unwrap(), "\"total\"");
        let a: Cover = serde_json::from_str("\"full\"").unwrap();
        let b: Cover = serde_json::from_str("\"total\"").unwrap();
        assert_eq!((a, b), (Cover::Full, Cover::Full));
    }

    const MINIMAL: &str = r#"{
        "format_version": 1, "library": "t", "library_version": "0.1",
        "pixels_per_square": 64,
        "assets": [{
            "id": "prop.barrel", "class": "prop", "image": "barrel.png",
            "footprint": {"w": 1, "h": 1}, "layer": "prop",
            "provenance": "hand", "licence": "CC0"
        }]
    }"#;

    #[test]
    fn minimal_catalog_parses_with_defaults() {
        let cat = parse(MINIMAL).unwrap();
        let a = &cat.assets[0];
        assert_eq!(a.class, AssetClass::Prop);
        assert_eq!(a.rotations, vec![0]);
        assert_eq!(a.cover, Cover::None);
        assert_eq!(a.anchor_or_centre(), Anchor { x: 0.5, y: 0.5 });
    }

    #[test]
    fn full_catalog_round_trips() {
        let mut cat = parse(MINIMAL).unwrap();
        let a = &mut cat.assets[0];
        a.anchor = Some(Anchor { x: 0.25, y: 0.75 });
        a.rotations = vec![0, 90, 180, 270];
        a.mirror = true;
        a.tags.function = vec!["warehouse".into()];
        a.tags.free = vec!["container".into()];
        a.placement.clearance_squares = 1;
        a.placement.ground = vec!["dirt".into()];
        a.cover = Cover::ThreeQuarters;
        a.light = Some(Light {
            radius_ft: 20,
            colour: [255, 180, 90],
        });
        a.wall = Some(WallPiece {
            kit: "stone".into(),
            role: WallRole::Tee,
        });
        a.height_ft = 4;
        let json = to_json(&cat).unwrap();
        assert_eq!(parse(&json).unwrap(), cat);
    }

    #[test]
    fn unknown_class_parses_for_the_validator_to_reject() {
        let json = MINIMAL.replace("\"class\": \"prop\"", "\"class\": \"spaceship\"");
        assert_eq!(parse(&json).unwrap().assets[0].class, AssetClass::Unknown);
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let json = MINIMAL.replace("\"layer\"", "\"colour\": 1, \"layer\"");
        assert!(parse(&json).is_err());
    }

    #[test]
    fn layers_order_ground_to_canopy() {
        assert!(Layer::Ground < Layer::Water);
        assert!(Layer::Water < Layer::Floor);
        assert!(Layer::Floor < Layer::Prop);
        assert!(Layer::Prop < Layer::Wall);
        assert!(Layer::Wall < Layer::Canopy);
    }
}
