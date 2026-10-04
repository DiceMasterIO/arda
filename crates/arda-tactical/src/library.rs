//! Loading a library directory at run time, so art can be swapped without a
//! rebuild (goal 60).

use crate::catalog::{self, Asset, AssetClass, Catalog, WallRole};
use crate::compose::sample::TextureSet;
use crate::error::TacticalError;
use crate::raster::Rgba;
use crate::validate::{validate, Images, Issue, Thresholds};
use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

/// The catalogue file name inside a library directory.
pub const CATALOG_FILE: &str = "catalog.json";

/// A validated library: catalogue plus decoded images keyed by asset id.
#[derive(Debug, Clone)]
pub struct Library {
    /// The parsed catalogue.
    pub catalog: Catalog,
    images: BTreeMap<String, Rgba>,
    derived: Derived,
}

/// Scaled texture sets kept across renders (goal 50), keyed by output ppsq
/// and a fingerprint of the catalogue's textures, so a changed catalogue
/// never reuses stale art. A clone starts empty.
#[derive(Default)]
struct Derived {
    textures: Mutex<TextureSets>,
}

/// Texture sets by `(ppsq, catalogue fingerprint)`.
type TextureSets = BTreeMap<(u32, u64), Arc<TextureSet>>;

/// Texture sets kept; more are dropped wholesale.
const DERIVED_SETS: usize = 8;

impl Clone for Derived {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for Derived {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Derived").finish_non_exhaustive()
    }
}

/// Reads and parses `<dir>/catalog.json`.
///
/// # Errors
/// I/O or schema errors.
pub fn read_catalog(dir: &Path) -> Result<Catalog, TacticalError> {
    let path = dir.join(CATALOG_FILE);
    let text = std::fs::read_to_string(&path).map_err(|source| TacticalError::Io {
        path: path.clone(),
        source,
    })?;
    catalog::parse(&text).map_err(|source| TacticalError::Catalog { path, source })
}

/// Loads every image named by the catalogue, keeping failures per asset.
#[must_use]
pub fn read_images(dir: &Path, catalog: &Catalog) -> Images {
    catalog
        .assets
        .iter()
        .map(|a| (a.id.clone(), read_asset_image(dir, a)))
        .collect()
}

fn read_asset_image(dir: &Path, a: &Asset) -> Result<Rgba, String> {
    let rel = PathBuf::from(&a.image);
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(format!(
            "image path `{}` must be relative and stay inside the library",
            a.image
        ));
    }
    Rgba::read_png(&dir.join(rel)).map_err(|e| e.to_string())
}

/// Validates a library directory without keeping it.
///
/// # Errors
/// Only I/O and schema errors; rule failures come back as issues.
pub fn check_dir(dir: &Path, t: &Thresholds) -> Result<Vec<Issue>, TacticalError> {
    let catalog = read_catalog(dir)?;
    let images = read_images(dir, &catalog);
    Ok(validate(&catalog, &images, t))
}

impl Library {
    /// Loads and validates a library directory.
    ///
    /// # Errors
    /// I/O and schema errors, or [`TacticalError::Invalid`] listing issues.
    pub fn load(dir: &Path) -> Result<Self, TacticalError> {
        let catalog = read_catalog(dir)?;
        let images = read_images(dir, &catalog);
        let issues = validate(&catalog, &images, &Thresholds::default());
        if !issues.is_empty() {
            return Err(TacticalError::Invalid(issues));
        }
        let images = images
            .into_iter()
            .filter_map(|(id, img)| img.ok().map(|i| (id, i)))
            .collect();
        Ok(Self {
            catalog,
            images,
            derived: Derived::default(),
        })
    }

    /// Builds a library from in-memory parts after validating them.
    ///
    /// # Errors
    /// [`TacticalError::Invalid`] listing issues.
    pub fn from_parts(
        catalog: Catalog,
        images: BTreeMap<String, Rgba>,
    ) -> Result<Self, TacticalError> {
        let wrapped: Images = images
            .iter()
            .map(|(k, v)| (k.clone(), Ok(v.clone())))
            .collect();
        let issues = validate(&catalog, &wrapped, &Thresholds::default());
        if !issues.is_empty() {
            return Err(TacticalError::Invalid(issues));
        }
        Ok(Self {
            catalog,
            images,
            derived: Derived::default(),
        })
    }

    /// Wraps parts the caller has already validated.
    pub(crate) fn from_validated(catalog: Catalog, images: BTreeMap<String, Rgba>) -> Self {
        Self {
            catalog,
            images,
            derived: Derived::default(),
        }
    }

    /// Every ground and water texture scaled to `ppsq`, built once per
    /// ppsq and catalogue and then shared (the result equals
    /// [`TextureSet::new`]).
    #[must_use]
    pub fn texture_set(&self, ppsq: u32) -> Arc<TextureSet> {
        let key = (ppsq, self.texture_fingerprint());
        if let Some(hit) = self.derived_sets().and_then(|m| m.get(&key).cloned()) {
            return hit;
        }
        let set = Arc::new(TextureSet::new(self, ppsq));
        if let Some(mut m) = self.derived_sets() {
            if m.len() >= DERIVED_SETS {
                m.clear();
            }
            m.insert(key, Arc::clone(&set));
        }
        set
    }

    fn derived_sets(&self) -> Option<std::sync::MutexGuard<'_, TextureSets>> {
        self.derived.textures.lock().ok()
    }

    /// A hash of what [`TextureSet::new`] reads from the catalogue.
    fn texture_fingerprint(&self) -> u64 {
        let mut h = 0;
        for a in self.catalog.assets.iter().filter(|a| a.class.is_texture()) {
            h = crate::noise::hash_str(h, &a.id);
            h = crate::noise::hash_str(h, a.ground.as_deref().unwrap_or(""));
            h = crate::noise::hash2(h, i64::from(a.footprint.w), i64::from(a.footprint.h));
            for t in &a.tags.free {
                h = crate::noise::hash_str(h, t);
            }
        }
        h
    }

    /// Source pixels per square.
    #[must_use]
    pub fn ppsq(&self) -> u32 {
        self.catalog.pixels_per_square
    }

    /// An asset by id.
    #[must_use]
    pub fn asset(&self, id: &str) -> Option<&Asset> {
        self.catalog.assets.iter().find(|a| a.id == id)
    }

    /// The variant family of `id`: every asset whose [`family_base`] is
    /// `id`, in catalogue order (`prop.barrel`, `prop.barrel.alt1`, …). An
    /// id that names one take itself (`prop.barrel.alt2`) is its own
    /// family of one.
    #[must_use]
    pub fn family(&self, id: &str) -> Vec<&Asset> {
        if family_base(id) != id {
            return self.asset(id).into_iter().collect();
        }
        self.catalog
            .assets
            .iter()
            .filter(|a| family_base(&a.id) == id)
            .collect()
    }

    /// The image of an asset by id.
    #[must_use]
    pub fn image(&self, id: &str) -> Option<&Rgba> {
        self.images.get(id)
    }

    /// Texture variants painting ground type `key`, in catalogue order.
    #[must_use]
    pub fn textures(&self, key: &str) -> Vec<&Asset> {
        self.catalog
            .assets
            .iter()
            .filter(|a| a.class.is_texture() && a.ground.as_deref() == Some(key))
            .collect()
    }

    /// Wall pieces of `kit` with `role`, in catalogue order.
    #[must_use]
    pub fn wall_pieces(&self, kit: &str, role: WallRole) -> Vec<&Asset> {
        self.catalog
            .assets
            .iter()
            .filter(|a| {
                a.class == AssetClass::Wall
                    && a.wall
                        .as_ref()
                        .is_some_and(|w| w.kit == kit && w.role == role)
            })
            .collect()
    }

    /// Assets matching a class and carrying every listed free or controlled tag.
    ///
    /// A bare tag such as `inn` matches any tag list. A namespaced tag such
    /// as `function:inn` matches only that list; the namespaces are `biome`,
    /// `culture`, `wealth`, `function` and `free`.
    #[must_use]
    pub fn query(&self, class: Option<AssetClass>, tags: &[String]) -> Vec<&Asset> {
        self.catalog
            .assets
            .iter()
            .filter(|a| class.is_none_or(|c| a.class == c))
            .filter(|a| tags.iter().all(|q| has_tag(&a.tags, q)))
            .collect()
    }
}

/// The id of the asset family `id` belongs to: `id` without a trailing
/// `.alt<N>` take suffix (`prop.barrel.alt2` → `prop.barrel`), or `id`
/// itself. The art importer names extra takes of a cut-out this way.
#[must_use]
pub fn family_base(id: &str) -> &str {
    match id.rsplit_once(".alt") {
        Some((base, n))
            if !base.is_empty() && !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) =>
        {
            base
        }
        _ => id,
    }
}

/// Whether `tags` carries the bare or namespaced tag `q`.
fn has_tag(t: &crate::catalog::Tags, q: &str) -> bool {
    let list = |ns: &str| match ns {
        "biome" => Some(&t.biome),
        "culture" => Some(&t.culture),
        "wealth" => Some(&t.wealth),
        "function" => Some(&t.function),
        "free" => Some(&t.free),
        _ => None,
    };
    if let Some((ns, value)) = q.split_once(':') {
        if let Some(l) = list(ns) {
            return l.iter().any(|v| v == value);
        }
    }
    [&t.biome, &t.culture, &t.wealth, &t.function, &t.free]
        .iter()
        .any(|l| l.iter().any(|v| v == q))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn placeholder() -> Library {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tactical/placeholder");
        Library::load(&dir).unwrap()
    }

    fn ids(lib: &Library, tags: &[&str]) -> Vec<String> {
        let tags: Vec<String> = tags.iter().map(|t| (*t).to_string()).collect();
        lib.query(Some(AssetClass::Prop), &tags)
            .iter()
            .map(|a| a.id.clone())
            .collect()
    }

    #[test]
    fn key_value_queries_match_one_tag_list() {
        let lib = placeholder();
        let inn = ids(&lib, &["function:inn"]);
        for id in ["prop.barrel", "prop.table"] {
            assert!(inn.contains(&id.to_string()), "{id} not in {inn:?}");
        }
        assert!(!inn.contains(&"prop.crate".to_string()));
        // Namespaces are exact: `inn` is a function, never a biome.
        assert!(ids(&lib, &["biome:inn"]).is_empty());
        assert_eq!(ids(&lib, &["inn"]), inn, "bare tags still match");
        for id in ["prop.bar_counter", "prop.cask_rack", "prop.hearth"] {
            assert!(inn.contains(&id.to_string()), "{id} not in {inn:?}");
        }
        let smithy = ids(&lib, &["function:smithy"]);
        for id in ["prop.anvil", "prop.forge", "prop.grindstone"] {
            assert!(smithy.contains(&id.to_string()), "{id} not in {smithy:?}");
        }
        assert!(!inn.contains(&"prop.anvil".to_string()));
        assert!(ids(&lib, &["craft:weaver"]).contains(&"prop.loom".to_string()));
        let lit = ids(&lib, &["function:temple", "free:light"]);
        assert!(lit.contains(&"prop.candle_stand".to_string()));
        assert!(!lit.contains(&"prop.pew".to_string()));
    }
}
