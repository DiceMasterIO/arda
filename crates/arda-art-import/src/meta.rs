//! Catalogue metadata for imported assets: inherited from the base
//! (placeholder) library's record of the same id, else sensible defaults
//! per class and name, then overridden by the manifest.

use crate::image_io::GenMeta;
use crate::manifest::{AssetEntry, LibrarySection};
use crate::naming::{is_water, Target};
use arda_tactical::catalog::{
    Asset, AssetClass, Catalog, Cover, Footprint, Layer, PlacementRules, Tags, WallPiece,
};
use arda_tactical::compose::ROT_FREE;

/// Ground keys whose variants share an aligned layout by default.
const STRUCTURED_KEYS: &[&str] = &[
    "cobbles",
    "flagstone",
    "planks",
    "stone_floor",
    "rug",
    "farmland",
    "cliff",
];

/// The base libraries' records, top first.
#[derive(Debug, Clone, Default)]
pub struct Base {
    assets: Vec<Asset>,
}

impl Base {
    /// Collects the records of base catalogues, top first.
    #[must_use]
    pub fn new(catalogs: &[Catalog]) -> Self {
        Self {
            assets: catalogs.iter().flat_map(|c| c.assets.clone()).collect(),
        }
    }

    /// The base record standing for `target`: same id, same ground key or
    /// same kit and role.
    #[must_use]
    pub fn find(&self, target: &Target) -> Option<&Asset> {
        match target {
            Target::Texture { key } => self
                .assets
                .iter()
                .find(|a| a.class.is_texture() && a.ground.as_deref() == Some(key)),
            Target::Wall { kit, role } => self.assets.iter().find(|a| {
                a.wall
                    .as_ref()
                    .is_some_and(|w| &w.kit == kit && w.role == *role)
            }),
            _ => {
                let id = target.base_id();
                self.assets.iter().find(|a| a.id == id)
            }
        }
    }

    /// Whether any base library covers this slot.
    #[must_use]
    pub fn covers(&self, slot: &str) -> bool {
        if let Some(key) = slot.strip_prefix("ground:") {
            return self.assets.iter().any(|a| a.ground.as_deref() == Some(key));
        }
        if let Some(kit) = slot.strip_prefix("wall:") {
            return self
                .assets
                .iter()
                .any(|a| a.wall.as_ref().is_some_and(|w| w.kit == kit));
        }
        self.assets.iter().any(|a| a.id == slot)
    }
}

/// A record with no base to inherit from.
#[must_use]
pub fn class_default(target: &Target) -> Asset {
    let mut a = blank();
    match target {
        Target::Texture { key } => {
            let water = is_water(key);
            a.class = if water {
                AssetClass::Water
            } else {
                AssetClass::Ground
            };
            a.layer = if water { Layer::Water } else { Layer::Ground };
            a.footprint = Footprint { w: 2, h: 2 };
            a.rotations = vec![0, 90, 180, 270];
            a.mirror = true;
            a.tileable = true;
            a.ground = Some(key.clone());
            if STRUCTURED_KEYS.contains(&key.as_str()) {
                a.tags.free.push("structured".into());
                if matches!(key.as_str(), "cobbles" | "flagstone") {
                    a.footprint = Footprint { w: 4, h: 4 };
                }
            }
        }
        Target::Wall { kit, role } => {
            a.class = AssetClass::Wall;
            a.layer = Layer::Wall;
            a.rotations = vec![0, 90, 180, 270];
            a.blocks_sight = kit != "drystone";
            a.blocks_movement = true;
            a.cover = if kit == "drystone" {
                Cover::ThreeQuarters
            } else {
                Cover::Full
            };
            a.casts_shadow = true;
            a.height_ft = match kit.as_str() {
                "drystone" => 4,
                "hedge" | "wattle" => 6,
                "city_wall" => 20,
                _ => 10,
            };
            a.wall = Some(WallPiece {
                kit: kit.clone(),
                role: *role,
            });
        }
        Target::Prop { name } => prop_default(&mut a, name),
        Target::Vegetation { name } => veg_default(&mut a, name),
    }
    a
}

fn blank() -> Asset {
    Asset {
        id: String::new(),
        class: AssetClass::Prop,
        image: String::new(),
        footprint: Footprint { w: 1, h: 1 },
        anchor: None,
        rotations: vec![0],
        mirror: false,
        tags: Tags {
            biome: vec!["temperate".into()],
            culture: vec!["human".into()],
            ..Tags::default()
        },
        placement: PlacementRules::default(),
        blocks_sight: false,
        blocks_movement: false,
        difficult_terrain: false,
        cover: Cover::None,
        light: None,
        layer: Layer::Prop,
        z: 0,
        casts_shadow: false,
        height_ft: 0,
        tileable: false,
        ground: None,
        wall: None,
        provenance: String::new(),
        licence: String::new(),
    }
}

fn prop_default(a: &mut Asset, name: &str) {
    a.class = AssetClass::Prop;
    a.rotations = vec![0, 90, 180, 270];
    a.mirror = true;
    let flat = ["rug", "deck", "planks", "bridge", "grave", "drain"];
    if flat.iter().any(|f| name.contains(f)) {
        a.layer = Layer::Floor;
        return;
    }
    a.blocks_movement = true;
    a.cover = Cover::Half;
    a.casts_shadow = true;
    a.height_ft = 3;
    if name.contains("boat") {
        a.placement.on_water = true;
        a.blocks_movement = false;
        a.cover = Cover::None;
        a.footprint = Footprint { w: 1, h: 2 };
    }
}

fn veg_default(a: &mut Asset, name: &str) {
    a.class = AssetClass::Vegetation;
    a.rotations = vec![0, 90, 180, 270];
    a.mirror = true;
    a.tags.free.push("plant".into());
    let has = |words: &[&str]| words.iter().any(|w| name.contains(w));
    if name.starts_with("tree_") {
        a.layer = Layer::Canopy;
        a.footprint = Footprint { w: 3, h: 3 };
        a.cover = Cover::Half;
        a.casts_shadow = true;
        a.height_ft = 30;
        a.placement.clearance_squares = 1;
        a.tags.free = vec!["tree".into()];
    } else if has(&["rock", "boulder", "stone"]) {
        a.blocks_movement = true;
        a.cover = Cover::Half;
        a.casts_shadow = true;
        a.height_ft = 3;
        a.tags.free = vec!["rock".into()];
    } else if has(&["lily"]) {
        a.placement.on_water = true;
    } else if has(&[
        "bush", "juniper", "heather", "reed", "cattail", "fern", "grass",
    ]) {
        a.difficult_terrain = true;
        a.cover = if has(&["bush", "juniper"]) {
            Cover::Half
        } else {
            Cover::None
        };
        a.casts_shadow = true;
        a.height_ft = if has(&["bush", "juniper", "reed"]) {
            4
        } else {
            2
        };
    }
}

/// Provenance and licence inputs for one file.
#[derive(Debug, Clone, Copy)]
pub struct Origin<'a> {
    /// The raw file name.
    pub file: &'a str,
    /// The manifest's `[library]` section.
    pub library: &'a LibrarySection,
    /// The matching manifest entry.
    pub entry: Option<&'a AssetEntry>,
    /// Metadata found in the PNG.
    pub found: &'a GenMeta,
}

/// The provenance string, and whether the generator is still unknown.
#[must_use]
pub fn provenance(o: Origin<'_>) -> (String, bool) {
    let e = o.entry;
    let pick = |f: fn(&AssetEntry) -> Option<String>| e.and_then(f);
    let tool = pick(|e| e.tool.clone()).or_else(|| o.library.tool.clone());
    let model = pick(|e| e.model.clone())
        .or_else(|| o.library.model.clone())
        .or_else(|| o.found.model.clone());
    let prompt = pick(|e| e.prompt.clone()).or_else(|| o.found.prompt.clone());
    let seed = e.and_then(|e| e.seed).or(o.found.seed);
    let mut parts = Vec::new();
    match (&tool, &model) {
        (Some(t), Some(m)) => parts.push(format!("{t} / {m}")),
        (Some(t), None) => parts.push(t.clone()),
        (None, Some(m)) => parts.push(m.clone()),
        (None, None) => parts.push("AI-generated (tool unknown)".into()),
    }
    if let Some(p) = prompt {
        parts.push(format!("prompt: \"{p}\""));
    }
    if let Some(s) = seed {
        parts.push(format!("seed {s}"));
    }
    if let Some(a) = &o.library.author {
        parts.push(format!("curated by {a}"));
    }
    parts.push(format!("raw file {}; arda tactical import", o.file));
    (parts.join("; "), tool.is_none() && model.is_none())
}

/// Applies manifest overrides to a record.
pub fn apply_entry(a: &mut Asset, e: &AssetEntry) {
    if let Some([w, h]) = e.footprint {
        a.footprint = Footprint { w, h };
    }
    if let Some(v) = e.height_ft {
        a.height_ft = v;
    }
    if let Some(v) = e.cover {
        a.cover = v;
    }
    if let Some(v) = e.layer {
        a.layer = v;
    }
    if let Some(v) = e.blocks_sight {
        a.blocks_sight = v;
    }
    if let Some(v) = e.blocks_movement {
        a.blocks_movement = v;
    }
    if let Some(v) = e.difficult_terrain {
        a.difficult_terrain = v;
    }
    if let Some(t) = &e.tags {
        a.tags = t.clone();
    }
    for (flag, tag) in [(e.structured, "structured"), (e.rot_free, ROT_FREE)] {
        if let Some(on) = flag {
            a.tags.free.retain(|t| t != tag);
            if on {
                a.tags.free.push(tag.into());
            }
        }
    }
}

/// Whether a record is a structured texture.
#[must_use]
pub fn is_structured(a: &Asset) -> bool {
    a.tags.free.iter().any(|t| t == "structured")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_by_name() {
        let tree = class_default(&Target::Vegetation {
            name: "tree_maple".into(),
        });
        assert_eq!(
            (tree.layer, tree.footprint.w, tree.height_ft),
            (Layer::Canopy, 3, 30)
        );
        let cob = class_default(&Target::Texture {
            key: "cobbles".into(),
        });
        assert!(is_structured(&cob) && cob.footprint.w == 4 && cob.tileable);
        let deep = class_default(&Target::Texture {
            key: "water_deep".into(),
        });
        assert_eq!(deep.class, AssetClass::Water);
        let rug = class_default(&Target::Prop {
            name: "rug_large".into(),
        });
        assert_eq!(rug.layer, Layer::Floor);
    }

    #[test]
    fn provenance_names_what_is_known() {
        let lib = LibrarySection {
            tool: Some("ComfyUI".into()),
            model: Some("FLUX.1-schnell".into()),
            ..LibrarySection::default()
        };
        let found = GenMeta {
            seed: Some(7),
            ..GenMeta::default()
        };
        let (p, unknown) = provenance(Origin {
            file: "prop.anvil.png",
            library: &lib,
            entry: None,
            found: &found,
        });
        assert!(!unknown);
        assert!(p.starts_with("ComfyUI / FLUX.1-schnell; seed 7"), "{p}");
    }
}
