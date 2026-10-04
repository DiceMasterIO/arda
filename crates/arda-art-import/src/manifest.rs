//! The optional `import.toml` manifest: library identity, provenance
//! defaults, the colour grade and per-file overrides.
//!
//! ```toml
//! [library]
//! name = "arda-ai"
//! version = "0.1.0"
//! licence = "CC0-1.0"
//! tool = "ComfyUI"
//! model = "FLUX.1-schnell"
//!
//! [grade]
//! reference = "assets/reference/tactical-target/4.jpg"  # or palette = ["#6b7d45", …]
//! strength = 0.5
//!
//! [[asset]]
//! file = "ComfyUI_00012_.png"      # or `id = "prop.anvil"` to match by name
//! id = "prop.anvil"
//! prompt = "top-down iron anvil, …"
//! seed = 812734
//! height_ft = 3
//! tags = { function = ["smithy"], free = ["craft:smith"] }
//! holes = "keep"                   # or "clear"; default by class
//! rot_free = true                  # may be turned and mirrored by hash
//! fixed_pose = true                # vegetation: no hashed scale or transpose
//! ```

use crate::error::{ImportError, ImportResult};
use arda_tactical::catalog::{Cover, Layer, Tags};
use serde::Deserialize;
use std::path::Path;

/// The whole manifest. Every section is optional.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Library identity and provenance defaults.
    #[serde(default)]
    pub library: LibrarySection,
    /// The optional global colour grade.
    #[serde(default)]
    pub grade: GradeSection,
    /// Per-file or per-id overrides.
    #[serde(default, rename = "asset")]
    pub assets: Vec<AssetEntry>,
}

/// `[library]`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibrarySection {
    /// Library name (default `imported`).
    pub name: Option<String>,
    /// Library version (default `0.1.0`).
    pub version: Option<String>,
    /// Pixels per square of the output (default: the base library's, else 128).
    pub pixels_per_square: Option<u32>,
    /// Default licence for every asset.
    pub licence: Option<String>,
    /// Default generator tool, e.g. `ComfyUI`.
    pub tool: Option<String>,
    /// Default model, e.g. `FLUX.1-schnell`.
    pub model: Option<String>,
    /// Who curated the art.
    pub author: Option<String>,
}

/// `[grade]`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GradeSection {
    /// A reference image (PNG or JPEG) whose colour statistics to match.
    pub reference: Option<String>,
    /// Target palette as `#rrggbb` strings (used when no reference).
    #[serde(default)]
    pub palette: Vec<String>,
    /// 0 = no grade, 1 = full statistics transfer (default 0.5).
    pub strength: Option<f32>,
}

/// `[[asset]]`: matches a raw file by `file` name, or every file whose
/// parsed id is `id` when `file` is absent.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetEntry {
    /// Raw file name (with extension) inside the raw directory.
    pub file: Option<String>,
    /// Asset spec, e.g. `prop.anvil` or `wall.stone.tee`.
    pub id: Option<String>,
    /// Generator prompt.
    pub prompt: Option<String>,
    /// Generator seed.
    pub seed: Option<u64>,
    /// Generator tool (overrides `[library].tool`).
    pub tool: Option<String>,
    /// Model (overrides `[library].model`).
    pub model: Option<String>,
    /// Licence (overrides `[library].licence`).
    pub licence: Option<String>,
    /// Footprint `[w, h]` in squares.
    pub footprint: Option<[u32; 2]>,
    /// Height in feet.
    pub height_ft: Option<u16>,
    /// Cover level.
    pub cover: Option<Cover>,
    /// Render layer.
    pub layer: Option<Layer>,
    /// Blocks line of sight.
    pub blocks_sight: Option<bool>,
    /// Blocks movement.
    pub blocks_movement: Option<bool>,
    /// Difficult terrain.
    pub difficult_terrain: Option<bool>,
    /// Tags; replace the inherited tags when given.
    pub tags: Option<Tags>,
    /// Marks a texture as `structured` (aligned variants).
    pub structured: Option<bool>,
    /// Adds (or removes) the free tag `rot_free`: the compositor may turn
    /// and mirror the cut-out by hash. Only for round or radial things
    /// (barrels, crates, trees, bushes, rocks) whose shading reads from
    /// any side.
    pub rot_free: Option<bool>,
    /// Adds (or removes) the free tag `fixed_pose`: the compositor draws
    /// the vegetation cut-out at its catalogued size and orientation,
    /// without the hashed scale and diagonal transpose.
    pub fixed_pose: Option<bool>,
    /// Shadow handling: `strip` (default) or `keep` (never strip, still flag).
    pub shadow: Option<ShadowMode>,
    /// Enclosed backdrop-coloured pockets: `clear` (any size) or `keep`
    /// (never). Default: small leaf gaps for vegetation, only exact-backdrop
    /// specks for props and walls.
    pub holes: Option<HoleMode>,
}

/// What to do with backdrop-coloured pockets the border flood cannot reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoleMode {
    /// Leave every enclosed pocket opaque.
    Keep,
    /// Clear every flat pocket close to the backdrop colour, at any size.
    Clear,
}

/// What to do with a detected baked shadow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadowMode {
    /// Remove it when detected confidently; flag it otherwise.
    #[default]
    Strip,
    /// Leave the pixels alone; only flag.
    Keep,
}

impl Manifest {
    /// Reads and parses a manifest file.
    ///
    /// # Errors
    /// I/O or TOML errors.
    pub fn read(path: &Path) -> ImportResult<Self> {
        let text = std::fs::read_to_string(path).map_err(crate::error::io(path))?;
        let m: Self = toml::from_str(&text).map_err(|e| ImportError::Manifest {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
        m.check().map_err(|message| ImportError::Manifest {
            path: path.to_path_buf(),
            message,
        })?;
        Ok(m)
    }

    /// Refuses footprints the library validator would refuse anyway, before
    /// any image is fitted to `footprint × ppsq` (the catalogue footprint
    /// cap, [`arda_tactical::validate::MAX_FOOTPRINT_SQUARES`] a side).
    ///
    /// # Errors
    /// A message naming the first bad entry.
    pub fn check(&self) -> Result<(), String> {
        let max = arda_tactical::validate::MAX_FOOTPRINT_SQUARES;
        for e in &self.assets {
            if let Some([w, h]) = e.footprint {
                if w == 0 || h == 0 || w > max || h > max {
                    let name = e.id.as_deref().or(e.file.as_deref()).unwrap_or("?");
                    return Err(format!(
                        "asset {name}: footprint {w}x{h} must be 1 to {max} squares a side"
                    ));
                }
            }
        }
        Ok(())
    }

    /// The entry naming this file explicitly.
    #[must_use]
    pub fn for_file(&self, file: &str) -> Option<&AssetEntry> {
        self.assets.iter().find(|e| e.file.as_deref() == Some(file))
    }

    /// The entry matching an asset spec when no entry names the file.
    #[must_use]
    pub fn for_id(&self, spec: &str) -> Option<&AssetEntry> {
        self.assets
            .iter()
            .find(|e| e.file.is_none() && e.id.as_deref() == Some(spec))
    }
}

/// Parses `#rrggbb`.
#[must_use]
pub fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let h = s.strip_prefix('#').unwrap_or(s);
    if h.len() != 6 {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documented_example_parses() {
        let m: Manifest = toml::from_str(
            r##"
            [library]
            name = "arda-ai"
            licence = "CC0-1.0"
            tool = "ComfyUI"
            model = "FLUX.1-schnell"
            [grade]
            palette = ["#6b7d45", "#8a7357"]
            strength = 0.4
            [[asset]]
            file = "ComfyUI_00012_.png"
            id = "prop.anvil"
            seed = 812734
            cover = "half"
            tags = { function = ["smithy"], free = ["craft:smith"] }
            [[asset]]
            id = "ground.cobbles"
            structured = true
            [[asset]]
            id = "veg.tree_oak"
            holes = "clear"
            rot_free = true
            fixed_pose = true
            "##,
        )
        .unwrap();
        assert_eq!(m.library.name.as_deref(), Some("arda-ai"));
        assert_eq!(
            m.for_file("ComfyUI_00012_.png").unwrap().id.as_deref(),
            Some("prop.anvil")
        );
        assert_eq!(m.for_id("ground.cobbles").unwrap().structured, Some(true));
        assert_eq!(
            m.for_id("veg.tree_oak").unwrap().holes,
            Some(HoleMode::Clear)
        );
        assert_eq!(m.for_id("veg.tree_oak").unwrap().rot_free, Some(true));
        assert_eq!(m.for_id("veg.tree_oak").unwrap().fixed_pose, Some(true));
        assert!(toml::from_str::<Manifest>("[[asset]]\nholes = \"fill\"").is_err());
        assert_eq!(parse_hex("#6b7d45"), Some([0x6b, 0x7d, 0x45]));
        assert!(toml::from_str::<Manifest>("[library]\nnmae = 1").is_err());
    }

    #[test]
    fn footprints_beyond_the_catalogue_cap_are_refused() {
        let parse = |fp: &str| -> Manifest {
            toml::from_str(&format!(
                "[[asset]]\nid = \"prop.table\"\nfootprint = {fp}\n"
            ))
            .unwrap()
        };
        assert!(parse("[2, 1]").check().is_ok());
        assert!(parse("[16, 16]").check().is_ok());
        for bad in ["[17, 1]", "[1, 4000]", "[0, 2]"] {
            let err = parse(bad).check().unwrap_err();
            assert!(err.contains("prop.table"), "{err}");
        }
    }
}
