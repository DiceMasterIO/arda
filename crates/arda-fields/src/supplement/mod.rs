//! A temporary supplement to the placeholder library for the land-use keys
//! it does not cover yet: farmed and natural grounds, hedge, drystone and
//! wattle kits, and a few farm props and animals.
//!
//! It only ever adds assets whose ground key, kit or id the base library
//! lacks, so a real library that has them takes over untouched. Everything
//! here is generated, deterministic, marked temporary and Apache-2.0.

pub mod paint;

use arda_tactical::catalog::{
    Asset, AssetClass, Catalog, Cover, Footprint, Layer, PlacementRules, Tags, WallPiece,
};
use arda_tactical::noise::hash_str;
use arda_tactical::placeholders::walls::{piece, Kit, ROLES};
use arda_tactical::placeholders::{self, ground, role_name};
use arda_tactical::raster::Rgba;
use arda_tactical::{Library, TacticalError};
use std::collections::BTreeMap;

/// Licence on every supplementary asset.
pub const LICENCE: &str = "Apache-2.0 (generated placeholder, arda-fields supplement)";

/// Grounds painted by recolouring a base placeholder texture:
/// `(key, base key, tint, share of the base colour kept)`.
const GROUNDS: [(&str, &str, [u8; 3], f32); 13] = [
    ("farmland", "dirt", [112, 84, 56], 0.25),
    ("stubble", "grass", [184, 164, 98], 0.1),
    ("fallow", "grass", [128, 134, 72], 0.25),
    ("pasture", "grass", [92, 136, 60], 0.4),
    ("meadow", "grass", [122, 152, 68], 0.3),
    ("scrub", "grass", [84, 104, 56], 0.3),
    ("heath", "grass", [116, 100, 76], 0.2),
    ("forest_floor", "dirt", [84, 76, 50], 0.2),
    ("leaf_litter", "dirt", [122, 90, 50], 0.2),
    ("scree", "gravel", [150, 146, 136], 0.4),
    ("rock", "stone_floor", [128, 124, 116], 0.4),
    ("cliff", "stone_floor", [92, 88, 84], 0.3),
    ("packed_earth", "dirt", [138, 112, 80], 0.4),
];

/// Kits drawn with the placeholder wall painter.
const KITS: [Kit; 2] = [
    Kit {
        name: "drystone",
        thickness: 0.38,
        body: [152, 148, 136],
        joint: [96, 92, 84],
        block: (0.08, 0.18),
        height_ft: 4,
    },
    Kit {
        name: "wattle",
        thickness: 0.12,
        body: [136, 104, 64],
        joint: [96, 70, 42],
        block: (0.0, 0.0),
        height_ft: 4,
    },
];

fn asset(id: &str, class: AssetClass, (w, h): (u32, u32), layer: Layer) -> Asset {
    Asset {
        id: id.to_string(),
        class,
        image: format!("supplement/{id}.png"),
        footprint: Footprint { w, h },
        anchor: None,
        rotations: vec![0, 90, 180, 270],
        mirror: true,
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
        layer,
        z: 0,
        casts_shadow: true,
        height_ft: 2,
        tileable: false,
        ground: None,
        wall: None,
        provenance: "arda-fields supplementary placeholder generator".into(),
        licence: LICENCE.into(),
    }
}

type Painter = fn(&mut Rgba, u64);

/// A supplementary prop: id, class, footprint, height, cover, free tags.
struct PropSpec {
    id: &'static str,
    class: AssetClass,
    size: (u32, u32),
    height_ft: u16,
    cover: Cover,
    free: &'static [&'static str],
    paint: Painter,
}

const fn spec(
    id: &'static str,
    class: AssetClass,
    size: (u32, u32),
    height_ft: u16,
    cover: Cover,
    free: &'static [&'static str],
    paint: Painter,
) -> PropSpec {
    PropSpec {
        id,
        class,
        size,
        height_ft,
        cover,
        free,
        paint,
    }
}

fn sheep(img: &mut Rgba, seed: u64) {
    paint::beast(img, seed, true);
}

fn cow(img: &mut Rgba, seed: u64) {
    paint::beast(img, seed, false);
}

const PROPS: [PropSpec; 10] = {
    use AssetClass::{Prop, Vegetation};
    [
        spec(
            "prop.hay_bale",
            Prop,
            (1, 1),
            4,
            Cover::Half,
            &["hay"],
            paint::hay_bale,
        ),
        spec(
            "prop.trough",
            Prop,
            (2, 1),
            2,
            Cover::Half,
            &["water"],
            paint::trough,
        ),
        spec(
            "prop.millstone",
            Prop,
            (2, 2),
            2,
            Cover::Half,
            &["mill"],
            paint::millstone,
        ),
        spec(
            "prop.waterwheel",
            Prop,
            (3, 1),
            12,
            Cover::Full,
            &["mill"],
            paint::waterwheel,
        ),
        spec(
            "prop.haycart",
            Prop,
            (1, 2),
            6,
            Cover::Half,
            &["vehicle"],
            paint::haycart,
        ),
        spec(
            "prop.hearth",
            Prop,
            (1, 1),
            3,
            Cover::Half,
            &["light"],
            paint::hearth,
        ),
        spec(
            "prop.sheep",
            Prop,
            (1, 1),
            3,
            Cover::None,
            &["livestock:sheep"],
            sheep,
        ),
        spec(
            "prop.cow",
            Prop,
            (1, 2),
            5,
            Cover::Half,
            &["livestock:cattle"],
            cow,
        ),
        spec(
            "prop.hen",
            Prop,
            (1, 1),
            1,
            Cover::None,
            &["livestock:poultry"],
            paint::hen,
        ),
        spec(
            "veg.tall_grass",
            Vegetation,
            (1, 1),
            3,
            Cover::None,
            &["grass"],
            paint::tall_grass,
        ),
    ]
};

fn grounds(base: &Catalog, seed: u64, out: &mut Vec<(Asset, Rgba)>) {
    let ppsq = base.pixels_per_square;
    for (key, from, tint, keep) in GROUNDS {
        if base.assets.iter().any(|a| a.ground.as_deref() == Some(key)) {
            continue;
        }
        let variants = ground::TYPES
            .iter()
            .find(|(k, _, _)| *k == from)
            .map_or(1, |t| t.1);
        for v in 0..variants {
            let id = format!("ground.{key}.{v}");
            let side = ground::SQUARES;
            let mut a = asset(&id, AssetClass::Ground, (side, side), Layer::Ground);
            a.tileable = true;
            a.casts_shadow = false;
            a.height_ft = 0;
            a.ground = Some(key.to_string());
            a.difficult_terrain = crate::sidecar::difficult_ground(key);
            let base_id = format!("ground.{from}.{v}");
            let tex = ground::texture(from, hash_str(seed, &base_id), hash_str(seed, from), ppsq);
            out.push((a, paint::tinted(&tex, tint, keep)));
        }
    }
}

fn kits(base: &Catalog, seed: u64, out: &mut Vec<(Asset, Rgba)>) {
    let ppsq = base.pixels_per_square;
    let mut kits: Vec<(&str, u16)> = KITS.iter().map(|k| (k.name, k.height_ft)).collect();
    kits.push(("hedge", 6));
    for (name, height) in kits {
        let exists = base
            .assets
            .iter()
            .any(|a| a.wall.as_ref().is_some_and(|w| w.kit == name));
        if exists {
            continue;
        }
        for role in ROLES {
            let id = format!("wall.{name}.{}", role_name(role));
            let mut a = asset(&id, AssetClass::Wall, (1, 1), Layer::Wall);
            a.wall = Some(WallPiece {
                kit: name.to_string(),
                role,
            });
            a.mirror = false;
            a.height_ft = height;
            let (sight, movement, cover, _, _) = crate::sidecar::edge_rules(name, role);
            a.blocks_sight = sight;
            a.blocks_movement = movement;
            a.cover = cover.catalog();
            a.tags.function = vec!["farm".into()];
            let s = hash_str(seed, &id);
            let gate = role == arda_tactical::WallRole::Gate;
            let img = match KITS.iter().find(|k| k.name == name) {
                // Field gates are timber whatever the wall is built of.
                Some(_) if gate => piece(&placeholders::walls::KITS[1], role, s, ppsq),
                Some(kit) => piece(kit, role, s, ppsq),
                None => {
                    let mut img = Rgba::new(ppsq, ppsq);
                    paint::hedge(&mut img, s, role);
                    img
                }
            };
            out.push((a, img));
        }
    }
}

/// Every supplementary asset the base catalogue lacks, with its image.
#[must_use]
pub fn missing_from(base: &Catalog, seed: u64) -> Vec<(Asset, Rgba)> {
    let mut out = Vec::new();
    grounds(base, seed, &mut out);
    kits(base, seed, &mut out);
    for p in &PROPS {
        if base.assets.iter().any(|a| a.id == p.id) {
            continue;
        }
        let (w, h) = p.size;
        let mut a = asset(p.id, p.class, (w, h), Layer::Prop);
        a.height_ft = p.height_ft;
        a.cover = p.cover;
        a.tags.free = p.free.iter().map(|t| (*t).to_string()).collect();
        a.blocks_movement = matches!(p.cover, Cover::Full);
        if p.id == "prop.hearth" {
            a.light = Some(arda_tactical::catalog::Light {
                radius_ft: 15,
                colour: [255, 170, 90],
            });
        }
        let ppsq = base.pixels_per_square;
        let mut img = Rgba::new(w * ppsq, h * ppsq);
        (p.paint)(&mut img, hash_str(seed, p.id));
        out.push((a, img));
    }
    out
}

/// The base catalogue plus every supplementary asset it lacks.
#[must_use]
pub fn extend(
    base: &Catalog,
    images: &BTreeMap<String, Rgba>,
    seed: u64,
) -> (Catalog, BTreeMap<String, Rgba>, Vec<String>) {
    let mut cat = base.clone();
    let mut imgs = images.clone();
    let mut added = Vec::new();
    for (a, img) in missing_from(base, seed) {
        added.push(a.id.clone());
        imgs.insert(a.id.clone(), img);
        cat.assets.push(a);
    }
    if !cat.vocabulary.function.iter().any(|x| x == "farm") {
        cat.vocabulary.function.push("farm".to_string());
    }
    cat.library_version = format!("{}+fields{}", cat.library_version, added.len());
    (cat, imgs, added)
}

/// The placeholder library alone.
///
/// # Errors
/// If the generated library fails validation.
pub fn placeholder_library() -> Result<Library, TacticalError> {
    let (cat, imgs) = placeholders::generate(placeholders::DEFAULT_SEED);
    Library::from_parts(cat, imgs)
}

/// The placeholder library with the supplement; also returns the added ids.
///
/// # Errors
/// If the combined library fails validation.
pub fn supplemented_library() -> Result<(Library, Vec<String>), TacticalError> {
    let (cat, imgs) = placeholders::generate(placeholders::DEFAULT_SEED);
    let (cat, imgs, added) = extend(&cat, &imgs, placeholders::DEFAULT_SEED);
    Ok((Library::from_parts(cat, imgs)?, added))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_supplemented_library_validates_and_only_fills_gaps() {
        let (lib, added) = supplemented_library().unwrap();
        // The full placeholder library already has hedges; the supplement
        // only adds what is still missing.
        assert!(!lib
            .wall_pieces("hedge", arda_tactical::WallRole::Run)
            .is_empty());
        assert!(!lib.textures("farmland").is_empty());
        assert!(!lib
            .wall_pieces("drystone", arda_tactical::WallRole::Gate)
            .is_empty());
        assert!(!added.iter().any(|a| a.starts_with("ground.grass")));
        let again = missing_from(&lib.catalog, 1);
        assert!(
            again.is_empty(),
            "{:?}",
            again.iter().map(|a| &a.0.id).collect::<Vec<_>>()
        );
    }
}
