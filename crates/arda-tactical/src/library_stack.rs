//! Layered libraries: an imported art library over the placeholder one, so
//! every asset the new library lacks falls back per id (goal 60: art can
//! be swapped, and swapped in piecemeal, without a rebuild).
//!
//! A stack is written `top:…:bottom`, like `PATH`; the leftmost library wins.
//! Fallback is resolved per *thing the compositor picks among*, so styles
//! never mix inside one pick:
//! - props and vegetation: per asset family (an id and its `.alt<N>`
//!   takes), so a layer's takes of `prop.barrel` all shadow the lower
//!   layers' `prop.barrel` and its takes;
//! - ground and water textures: per `ground` key (a key's variants all come
//!   from the highest library that paints that key);
//! - wall pieces: per kit and role (a kit missing `post` in the new library
//!   keeps the placeholder `post`).

use crate::catalog::{Asset, AssetClass, Catalog, WallRole};
use crate::error::TacticalError;
use crate::library::{family_base, read_catalog, read_images, Library};
use crate::raster::Rgba;
use crate::validate::{validate, Rule, Thresholds, MAX_FOOTPRINT_SQUARES};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Largest side, in pixels, a lower layer's image may be rescaled to: the
/// sprite bound of the footprint cap ([`MAX_FOOTPRINT_SQUARES`] squares at
/// 128 px). A top layer with a large `pixels_per_square` could otherwise
/// blow every placeholder asset up past what the cap keeps cached sprites to.
pub const MAX_RESCALED_SIDE_PX: u32 = MAX_FOOTPRINT_SQUARES * 128;

/// Separator between the layers of a library stack.
pub const STACK_SEPARATOR: char = ':';

/// The directories named by `spec`, top first. A path that exists as a
/// directory is taken whole, even if it contains the separator.
#[must_use]
pub fn stack_dirs(spec: &Path) -> Vec<PathBuf> {
    if spec.is_dir() {
        return vec![spec.to_path_buf()];
    }
    spec.to_string_lossy()
        .split(STACK_SEPARATOR)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// What decides whether a lower library's asset is shadowed by a higher one.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Slot {
    Id(String),
    Ground(String),
    Wall(String, WallRole),
}

fn slot(a: &Asset) -> Slot {
    match (&a.ground, &a.wall) {
        (Some(key), _) if a.class.is_texture() => Slot::Ground(key.clone()),
        (_, Some(w)) if a.class == AssetClass::Wall => Slot::Wall(w.kit.clone(), w.role),
        _ => Slot::Id(family_base(&a.id).to_string()),
    }
}

impl Library {
    /// Loads a library or a `top:…:bottom` stack of libraries (see the
    /// module docs for the fallback rules). Each layer is validated on its
    /// own, then the merged library is validated again.
    ///
    /// # Errors
    /// I/O, schema or validation errors of any layer or of the merge.
    pub fn load_stack(spec: &Path) -> Result<Self, TacticalError> {
        let dirs = stack_dirs(spec);
        match dirs.as_slice() {
            [] => Err(TacticalError::Options(format!(
                "empty library stack `{}`",
                spec.display()
            ))),
            [one] => Self::load(one),
            _ => {
                let layers = dirs
                    .iter()
                    .map(|d| Self::load_layer(d).map_err(|e| layer_error(d, e)))
                    .collect::<Result<Vec<_>, _>>()?;
                Self::layered(&layers)
            }
        }
    }

    /// Loads one layer of a stack: every rule applies except `wall_kit`,
    /// because an upper layer may hold part of a kit and take the other
    /// roles from below. The merged stack is checked in full.
    ///
    /// # Errors
    /// I/O, schema or validation errors.
    pub fn load_layer(dir: &Path) -> Result<Self, TacticalError> {
        let catalog = read_catalog(dir)?;
        let images = read_images(dir, &catalog);
        let issues: Vec<_> = validate(&catalog, &images, &Thresholds::default())
            .into_iter()
            .filter(|i| i.rule != Rule::WallKit)
            .collect();
        if !issues.is_empty() {
            return Err(TacticalError::Invalid(issues));
        }
        let images = images
            .into_iter()
            .filter_map(|(id, img)| img.ok().map(|i| (id, i)))
            .collect();
        Ok(Self::from_validated(catalog, images))
    }

    /// Merges loaded libraries, top first. Lower layers' images are
    /// rescaled to the top layer's `pixels_per_square` when it differs.
    ///
    /// # Errors
    /// [`TacticalError::Invalid`] if the merged catalogue fails validation.
    pub fn layered(layers: &[Self]) -> Result<Self, TacticalError> {
        let Some(top) = layers.first() else {
            return Err(TacticalError::Options("no library layers".into()));
        };
        let ppsq = top.ppsq();
        let mut taken = BTreeSet::new();
        let mut ids = BTreeSet::new();
        let mut assets = Vec::new();
        let mut images = BTreeMap::new();
        let mut vocab = top.catalog.vocabulary.clone();
        for layer in layers {
            let fresh: BTreeSet<Slot> = layer
                .catalog
                .assets
                .iter()
                .map(slot)
                .filter(|s| !taken.contains(s))
                .collect();
            for a in &layer.catalog.assets {
                if !fresh.contains(&slot(a)) || !ids.insert(a.id.clone()) {
                    continue;
                }
                if let Some(img) = layer.image(&a.id) {
                    images.insert(a.id.clone(), rescale(img, a, ppsq)?);
                }
                assets.push(a.clone());
            }
            taken.extend(fresh);
            merge_vocab(&mut vocab, &layer.catalog);
        }
        let names: Vec<&str> = layers.iter().map(|l| l.catalog.library.as_str()).collect();
        let versions: Vec<&str> = layers
            .iter()
            .map(|l| l.catalog.library_version.as_str())
            .collect();
        let catalog = Catalog {
            format_version: top.catalog.format_version,
            library: names.join(":"),
            library_version: versions.join(":"),
            pixels_per_square: ppsq,
            vocabulary: vocab,
            assets,
        };
        Self::from_parts(catalog, images)
    }
}

/// Loads only the catalogues of a stack, top first, for coverage reports.
///
/// # Errors
/// I/O or schema errors.
pub fn read_stack_catalogs(spec: &Path) -> Result<Vec<Catalog>, TacticalError> {
    stack_dirs(spec).iter().map(|d| read_catalog(d)).collect()
}

fn layer_error(dir: &Path, e: TacticalError) -> TacticalError {
    match e {
        TacticalError::Invalid(issues) => TacticalError::Layout {
            layout: format!("library stack layer {}", dir.display()),
            message: format!(
                "failed validation with {} issue(s):\n{}",
                issues.len(),
                issues
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
        },
        other => other,
    }
}

fn rescale(img: &Rgba, a: &Asset, ppsq: u32) -> Result<Rgba, TacticalError> {
    let side = |squares: u32| squares.checked_mul(ppsq).filter(|&px| px > 0);
    let (Some(w), Some(h)) = (side(a.footprint.w), side(a.footprint.h)) else {
        return Err(too_large(a, ppsq));
    };
    if (img.width, img.height) == (w, h) {
        return Ok(img.clone());
    }
    if w > MAX_RESCALED_SIDE_PX || h > MAX_RESCALED_SIDE_PX {
        return Err(too_large(a, ppsq));
    }
    Ok(img.resized(w, h))
}

fn too_large(a: &Asset, ppsq: u32) -> TacticalError {
    TacticalError::Options(format!(
        "library stack: {} ({}x{} squares) at the top layer's {ppsq} px/square exceeds \
         the {MAX_RESCALED_SIDE_PX} px rescale limit per side",
        a.id, a.footprint.w, a.footprint.h
    ))
}

fn merge_vocab(into: &mut crate::catalog::TagVocabulary, from: &Catalog) {
    let v = &from.vocabulary;
    for (dst, src) in [
        (&mut into.biome, &v.biome),
        (&mut into.culture, &v.culture),
        (&mut into.wealth, &v.wealth),
        (&mut into.function, &v.function),
    ] {
        for t in src {
            if !dst.contains(t) {
                dst.push(t.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placeholder_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder")
    }

    #[test]
    fn single_directory_is_not_split() {
        let dir = placeholder_dir();
        assert_eq!(stack_dirs(&dir), vec![dir.clone()]);
        let spec = PathBuf::from("a:b::c");
        assert_eq!(stack_dirs(&spec).len(), 3);
    }

    #[test]
    fn top_layer_wins_per_slot_and_the_rest_falls_back() {
        let base = Library::load(&placeholder_dir()).unwrap();
        // A tiny "imported" library: one prop and one grass texture.
        let mut top_cat = base.catalog.clone();
        top_cat.library = "imported".into();
        top_cat.library_version = "1".into();
        top_cat
            .assets
            .retain(|a| a.id == "prop.barrel" || a.id == "ground.grass.1");
        for a in &mut top_cat.assets {
            a.provenance = "test import".into();
        }
        let images = top_cat
            .assets
            .iter()
            .map(|a| (a.id.clone(), base.image(&a.id).unwrap().clone()))
            .collect();
        let top = Library::from_parts(top_cat, images).unwrap();
        let merged = Library::layered(&[top, base.clone()]).unwrap();
        assert_eq!(merged.catalog.library, "imported:placeholder");
        assert_eq!(
            merged.asset("prop.barrel").unwrap().provenance,
            "test import"
        );
        // Every grass variant comes from the top layer only.
        let grass: Vec<_> = merged
            .textures("grass")
            .iter()
            .map(|a| a.id.clone())
            .collect();
        assert_eq!(grass, vec!["ground.grass.1".to_string()]);
        // Everything else is the placeholder's.
        assert!(merged.asset("prop.crate").is_some());
        assert_eq!(merged.textures("dirt").len(), base.textures("dirt").len());
        assert_eq!(
            merged.catalog.assets.len(),
            base.catalog.assets.len() - base.textures("grass").len() + 1
        );
    }

    #[test]
    fn lower_layers_are_not_rescaled_past_the_sprite_cap() {
        let base = Library::load(&placeholder_dir()).unwrap();
        // A one-asset top layer at 1024 px/square: the placeholder's 4 × 4
        // assets would rescale to 4096 px a side.
        let mut top_cat = base.catalog.clone();
        top_cat.library = "huge".into();
        top_cat.pixels_per_square = 1024;
        top_cat.assets.retain(|a| a.id == "prop.barrel");
        let barrel = &top_cat.assets[0];
        let (w, h) = (barrel.footprint.w * 1024, barrel.footprint.h * 1024);
        let mut img = Rgba::new(w, h);
        for y in h / 8..h - h / 8 {
            for x in w / 8..w - w / 8 {
                img.set(x, y, [120, 80, 40, 255]);
            }
        }
        let top = Library::from_parts(top_cat, [("prop.barrel".to_string(), img)].into()).unwrap();
        assert!(base
            .catalog
            .assets
            .iter()
            .any(|a| a.footprint.w * 1024 > MAX_RESCALED_SIDE_PX));
        let err = Library::layered(&[top, base]).unwrap_err().to_string();
        assert!(err.contains("rescale limit"), "{err}");
    }
}
