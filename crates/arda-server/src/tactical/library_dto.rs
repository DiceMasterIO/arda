//! `GET /v1/tactical/library`: what the loaded catalogue offers, for the
//! viewer's layout editor (ground keys, wall kits and placeable assets).

use arda_tactical::catalog::EFFECT_DECAL_TAG;
use arda_tactical::Library;
use schemars::JsonSchema;
use serde::Serialize;
use std::collections::BTreeMap;
use ts_rs::TS;

/// The loaded asset library, summarised.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, JsonSchema)]
pub struct TacticalLibraryDto {
    /// Library name.
    pub library: String,
    /// `library_version`; part of every render cache key.
    pub library_version: String,
    /// Art resolution in pixels per square.
    pub pixels_per_square: u32,
    /// Ground keys a `Square.ground` may use, sorted.
    pub grounds: Vec<GroundKeyDto>,
    /// Wall kits a `WallSegment.kit` may use, sorted.
    pub wall_kits: Vec<WallKitDto>,
    /// Placeable assets (props and vegetation), sorted by id. Damage props
    /// (`prop.rubble_heap`, `prop.broken_deck`, `prop.charred_floor`) are
    /// props tagged `free:damage`.
    pub assets: Vec<LibraryAssetDto>,
    /// Effect art a consumer draws over tiles, one entry per effect set,
    /// sorted by id. Never placed by layouts or drawn by the compositor.
    /// Absent (not an empty list) when the library has none, so the field
    /// is an optional addition to the frozen `tactical_format` 1 body.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<EffectDto>,
}

/// One effect set: a transparent texture or decal with numbered variants.
/// The images are `GET /v1/tactical/library/asset/{id}.{n}.png` for
/// `n` in `0..variants`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, JsonSchema)]
pub struct EffectDto {
    /// The set id without its variant number, e.g. `effect.frost`.
    pub id: String,
    /// `texture` (tileable) or `decal` (single patch).
    pub kind: String,
    /// Footprint in squares, `[w, h]`; the same for every variant.
    pub footprint: [u32; 2],
    /// Tags as `key:value` strings, sorted; those of the first variant.
    pub tags: Vec<String>,
    /// Number of variants, numbered from 0.
    pub variants: u32,
}

/// One ground key and how many texture variants back it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, JsonSchema)]
pub struct GroundKeyDto {
    /// The key, e.g. `cobbles`.
    pub key: String,
    /// Texture variants.
    pub variants: u32,
    /// Whether the texture is a water texture.
    pub water: bool,
}

/// One wall kit and the roles it has pieces for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, JsonSchema)]
pub struct WallKitDto {
    /// Kit name, e.g. `stone`.
    pub kit: String,
    /// Piece roles (`run`, `door`, `window`, `gate`, joints), sorted.
    pub roles: Vec<String>,
}

/// One placeable asset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS, JsonSchema)]
pub struct LibraryAssetDto {
    /// Asset id for `AssetRef::Id`.
    pub id: String,
    /// `prop` or `vegetation`.
    pub class: String,
    /// Draw layer (`floor`, `prop`, `canopy`, …).
    pub layer: String,
    /// Footprint in squares, `[w, h]`.
    pub footprint: [u32; 2],
    /// Allowed rotations in degrees.
    pub rotations: Vec<u16>,
    /// Tags as `key:value` strings for tag queries (I8), sorted.
    pub tags: Vec<String>,
    /// SRD cover it grants.
    pub cover: String,
    /// Blocks sight.
    pub blocks_sight: bool,
    /// Blocks movement.
    pub blocks_movement: bool,
    /// Makes its squares difficult terrain.
    pub difficult_terrain: bool,
}

fn tag_list(t: &arda_tactical::catalog::Tags) -> Vec<String> {
    let mut tags: Vec<String> = [
        ("biome", &t.biome),
        ("culture", &t.culture),
        ("wealth", &t.wealth),
        ("function", &t.function),
        ("free", &t.free),
    ]
    .iter()
    .flat_map(|(k, vs)| vs.iter().map(move |v| format!("{k}:{v}")))
    .collect();
    tags.sort();
    tags
}

fn word<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|j| j.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Summarises `lib`.
#[must_use]
pub fn summarise(lib: &Library) -> TacticalLibraryDto {
    let c = &lib.catalog;
    let mut grounds: BTreeMap<String, (u32, bool)> = BTreeMap::new();
    let mut kits: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut assets = Vec::new();
    let mut effects: BTreeMap<String, EffectDto> = BTreeMap::new();
    for a in &c.assets {
        let class = word(&a.class);
        if a.class.is_effect() {
            let id = arda_tactical::library::effect_base(&a.id).to_string();
            let decal = a.tags.free.iter().any(|t| t == EFFECT_DECAL_TAG);
            effects
                .entry(id.clone())
                .and_modify(|e| e.variants += 1)
                .or_insert_with(|| EffectDto {
                    id,
                    kind: if decal { "decal" } else { "texture" }.into(),
                    footprint: [a.footprint.w, a.footprint.h],
                    tags: tag_list(&a.tags),
                    variants: 1,
                });
        }
        if let Some(key) = &a.ground {
            let e = grounds.entry(key.clone()).or_insert((0, false));
            e.0 += 1;
            e.1 |= class == "water";
        }
        if let Some(w) = &a.wall {
            kits.entry(w.kit.clone()).or_default().push(word(&w.role));
        }
        if class == "prop" || class == "vegetation" {
            let tags = tag_list(&a.tags);
            assets.push(LibraryAssetDto {
                id: a.id.clone(),
                class,
                layer: word(&a.layer),
                footprint: [a.footprint.w, a.footprint.h],
                rotations: a.rotations.clone(),
                tags,
                cover: word(&a.cover),
                blocks_sight: a.blocks_sight,
                blocks_movement: a.blocks_movement,
                difficult_terrain: a.difficult_terrain,
            });
        }
    }
    assets.sort_by(|a, b| a.id.cmp(&b.id));
    TacticalLibraryDto {
        library: c.library.clone(),
        library_version: c.library_version.clone(),
        pixels_per_square: c.pixels_per_square,
        grounds: grounds
            .into_iter()
            .map(|(key, (variants, water))| GroundKeyDto {
                key,
                variants,
                water,
            })
            .collect(),
        wall_kits: kits
            .into_iter()
            .map(|(kit, mut roles)| {
                roles.sort();
                roles.dedup();
                WallKitDto { kit, roles }
            })
            .collect(),
        assets,
        effects: effects.into_values().collect(),
    }
}
