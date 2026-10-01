//! Graceful degradation: rewrites a layout so every ground key, wall kit,
//! role and asset exists in a given library, walking a chain of nearest
//! substitutes and recording every fallback taken.
//!
//! The generator always emits the canonical vocabulary (`vocabulary.md`);
//! only the copy handed to the compositor is degraded, and the sidecar keeps
//! the canonical keys.

use arda_tactical::catalog::{Asset, WallRole};
use arda_tactical::compose::candidates;
use arda_tactical::layout::{AssetRef, TacticalLayout};
use arda_tactical::Library;
use serde::Serialize;
use std::collections::BTreeMap;

/// One substitution, counted over the layout.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Fallback {
    /// `ground`, `kit`, `role` or `asset`.
    pub kind: &'static str,
    /// What the layout asked for.
    pub wanted: String,
    /// What was used instead; `None` when the item was dropped.
    pub used: Option<String>,
    /// How many squares, segments or placements it affected.
    pub count: usize,
}

/// Nearest substitutes for ground keys, best first.
#[must_use]
pub fn ground_chain(key: &str) -> &'static [&'static str] {
    match key {
        "farmland" => &["dirt", "mud"],
        "stubble" => &["farmland", "sand", "dirt"],
        "fallow" => &["meadow", "pasture", "grass"],
        "pasture" | "moss" => &["grass"],
        "meadow" => &["pasture", "grass"],
        "scrub" => &["heath", "grass"],
        "heath" => &["scrub", "grass"],
        "forest_floor" => &["leaf_litter", "mud", "dirt"],
        "leaf_litter" => &["forest_floor", "dirt"],
        "scree" => &["gravel"],
        "rock" => &["scree", "stone_floor", "gravel"],
        "cliff" => &["rock", "stone_floor", "gravel"],
        "packed_earth" => &["dirt"],
        "flagstone" => &["cobbles", "stone_floor"],
        "rug" => &["planks"],
        "marsh" => &["mud"],
        "reed_bed" => &["marsh", "mud"],
        "salt_crust" => &["sand", "gravel"],
        "mudflat" => &["mud", "sand", "dirt"],
        "snow" | "ice" => &["sand", "stone_floor"],
        _ => &["grass", "dirt"],
    }
}

/// Nearest substitutes for wall kits, best first.
#[must_use]
pub fn kit_chain(kit: &str) -> &'static [&'static str] {
    match kit {
        "hedge" => &["wattle", "timber"],
        "drystone" | "city_wall" => &["stone"],
        "wattle" | "palisade" => &["timber"],
        _ => &["stone", "timber"],
    }
}

/// Nearest substitutes for asset ids, best first.
#[must_use]
pub fn asset_chain(id: &str) -> &'static [&'static str] {
    match id {
        "prop.hay_bale" => &["prop.sacks"],
        "prop.trough" => &["prop.bench"],
        "prop.millstone" | "prop.grindstone" => &["prop.millstone", "prop.well", "veg.boulder"],
        "prop.waterwheel" => &["prop.millstone", "prop.crane"],
        "prop.bucket" => &["prop.barrel"],
        "prop.wheelbarrow" | "prop.haycart" => &["prop.cart"],
        "prop.ladder" => &["prop.fence"],
        "prop.hearth" | "prop.lantern" | "prop.candle_stand" => &["prop.brazier"],
        "prop.cupboard" | "prop.shelf" | "prop.bookshelf" => &["prop.chest"],
        "prop.stool" | "prop.chair" | "prop.pew" => &["prop.bench"],
        "veg.tall_grass" => &["veg.flower_patch", "veg.reeds", "veg.bush"],
        "veg.flower_patch" | "veg.heather" => &["veg.bush_flowering", "veg.bush"],
        "veg.fern" => &["veg.bush"],
        "veg.fallen_log" => &["prop.woodpile"],
        "veg.stump" | "veg.mushroom_ring" | "veg.rock_small" | "veg.scree_patch" => &["veg.stones"],
        "veg.rock_large" => &["veg.boulder"],
        "veg.tree_pine" | "veg.tree_dead" => &["veg.tree_birch"],
        "veg.tree_spruce" => &["veg.tree_pine", "veg.tree_birch"],
        "veg.tree_willow" => &["veg.tree_elm"],
        _ => &[],
    }
}

fn has_ground(lib: &Library, k: &str) -> bool {
    !lib.textures(k).is_empty()
}

fn resolve_ground(lib: &Library, key: &str) -> Option<String> {
    if has_ground(lib, key) {
        return Some(key.to_string());
    }
    let mut seen = vec![key.to_string()];
    let mut queue: Vec<&str> = ground_chain(key).to_vec();
    while let Some(k) = queue.first().copied() {
        queue.remove(0);
        if seen.iter().any(|s| s == k) {
            continue;
        }
        if has_ground(lib, k) {
            return Some(k.to_string());
        }
        seen.push(k.to_string());
        queue.extend(ground_chain(k));
    }
    lib.catalog
        .assets
        .iter()
        .find(|a| a.class == arda_tactical::AssetClass::Ground)
        .and_then(|a| a.ground.clone())
}

fn resolve_kit(lib: &Library, kit: &str) -> Option<String> {
    let ok = |k: &str| !lib.wall_pieces(k, WallRole::Run).is_empty();
    if ok(kit) {
        return Some(kit.to_string());
    }
    kit_chain(kit)
        .iter()
        .chain(kit_chain("").iter())
        .find(|k| ok(k))
        .map(|k| (*k).to_string())
}

fn resolve_role(lib: &Library, kit: &str, role: WallRole) -> WallRole {
    let chain: &[WallRole] = match role {
        WallRole::Gate => &[WallRole::Gate, WallRole::Door, WallRole::Run],
        WallRole::Door => &[WallRole::Door, WallRole::Gate, WallRole::Run],
        WallRole::Window => &[WallRole::Window, WallRole::Run],
        _ => &[WallRole::Run],
    };
    chain
        .iter()
        .copied()
        .find(|r| !lib.wall_pieces(kit, *r).is_empty())
        .unwrap_or(WallRole::Run)
}

fn resolve_asset(lib: &Library, r: &AssetRef) -> Option<AssetRef> {
    if !candidates(lib, r).is_empty() {
        return Some(r.clone());
    }
    match r {
        AssetRef::Id(id) => {
            let mut queue: Vec<&str> = asset_chain(id).to_vec();
            let mut seen: Vec<&str> = vec![];
            while !queue.is_empty() {
                let k = queue.remove(0);
                if seen.contains(&k) {
                    continue;
                }
                if lib.asset(k).is_some() {
                    return Some(AssetRef::Id(k.to_string()));
                }
                seen.push(k);
                queue.extend(asset_chain(k));
            }
            None
        }
        AssetRef::Query { class, tags } => (1..tags.len()).rev().find_map(|n| {
            let q = AssetRef::Query {
                class: *class,
                tags: tags[..n].to_vec(),
            };
            (!candidates(lib, &q).is_empty()).then_some(q)
        }),
    }
}

fn describe(r: &AssetRef) -> String {
    match r {
        AssetRef::Id(id) => id.clone(),
        AssetRef::Query { tags, .. } => format!("query[{}]", tags.join(",")),
    }
}

/// A rotation and mirror every candidate accepts, nearest the wanted one.
fn fit_rotation(all: &[&Asset], rotation: u16, mirror: bool) -> Option<(u16, bool)> {
    let mirror = mirror && all.iter().all(|a| a.mirror);
    [rotation, (rotation + 180) % 360, 0, 90, 180, 270]
        .into_iter()
        .find(|r| all.iter().all(|a| a.rotations.contains(r)))
        .map(|r| (r, mirror))
}

/// Rewrites a layout to what `lib` can draw, returning the fallbacks taken.
#[must_use]
pub fn adapt(layout: &TacticalLayout, lib: &Library) -> (TacticalLayout, Vec<Fallback>) {
    let mut out = layout.clone();
    let mut log: BTreeMap<(&'static str, String, Option<String>), usize> = BTreeMap::new();
    let mut note = |kind, wanted: &str, used: Option<String>| {
        *log.entry((kind, wanted.to_string(), used)).or_insert(0) += 1;
    };
    let mut grounds: BTreeMap<String, Option<String>> = BTreeMap::new();
    for sq in &mut out.squares {
        let got = grounds
            .entry(sq.ground.clone())
            .or_insert_with(|| resolve_ground(lib, &sq.ground))
            .clone();
        if let Some(g) = got {
            if g != sq.ground {
                note("ground", &sq.ground, Some(g.clone()));
                sq.ground = g;
            }
        }
    }
    for w in &mut out.walls {
        if let Some(k) = resolve_kit(lib, &w.kit) {
            if k != w.kit {
                note("kit", &w.kit, Some(k.clone()));
                w.kit = k;
            }
        }
        let role = resolve_role(lib, &w.kit, w.kind);
        if role != w.kind {
            note(
                "role",
                &format!("{}:{:?}", w.kit, w.kind),
                Some(format!("{role:?}")),
            );
            w.kind = role;
        }
    }
    let mut kept = Vec::with_capacity(out.placements.len());
    for mut p in std::mem::take(&mut out.placements) {
        let wanted = describe(&p.asset);
        let Some(r) = resolve_asset(lib, &p.asset) else {
            note("asset", &wanted, None);
            continue;
        };
        let all = candidates(lib, &r);
        let (r, all) = match fit_rotation(&all, p.rotation, p.mirror) {
            Some(_) => (r, all),
            // Candidates disagree on rotations: pin the first one by id.
            None => match all.first() {
                Some(a) => (AssetRef::Id(a.id.clone()), vec![*a]),
                None => continue,
            },
        };
        let Some((rot, mirror)) = fit_rotation(&all, p.rotation, p.mirror) else {
            note("asset", &wanted, None);
            continue;
        };
        if r != p.asset {
            note("asset", &wanted, Some(describe(&r)));
        }
        p.asset = r;
        p.rotation = rot;
        p.mirror = mirror;
        kept.push(p);
    }
    out.placements = kept;
    let fallbacks = log
        .into_iter()
        .map(|((kind, wanted, used), count)| Fallback {
            kind,
            wanted,
            used,
            count,
        })
        .collect();
    (out, fallbacks)
}
