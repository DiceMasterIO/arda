//! Graceful degradation: layouts use the canonical vocabulary keys
//! (`docs/goal-prompts/vocabulary.md`); a library that lacks one gets the
//! nearest thing it has, and every substitution is recorded.

use arda_tactical::catalog::WallRole;
use arda_tactical::layout::AssetRef;
use arda_tactical::{Library, TacticalLayout};
use serde::Serialize;
use std::collections::BTreeMap;

/// What kind of key was substituted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackKind {
    /// A ground key.
    Ground,
    /// A wall kit.
    Kit,
    /// A prop or vegetation id.
    Asset,
}

/// One recorded substitution.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Fallback {
    /// Kind of key.
    pub kind: FallbackKind,
    /// The key the layout asked for.
    pub wanted: String,
    /// The key drawn instead, or `(dropped)`.
    pub used: String,
    /// How many squares, segments or placements were affected.
    pub count: u32,
}

/// Nearest-first substitutes per ground key.
const GROUND: &[(&str, &[&str])] = &[
    ("flagstone", &["stone_floor", "cobbles", "gravel"]),
    ("packed_earth", &["dirt"]),
    ("scree", &["gravel", "dirt"]),
    ("rock", &["gravel", "stone_floor"]),
    ("cliff", &["gravel", "stone_floor"]),
    ("meadow", &["grass"]),
    ("pasture", &["grass"]),
    ("heath", &["grass"]),
    ("scrub", &["grass"]),
    ("moss", &["grass"]),
    ("forest_floor", &["dirt", "grass"]),
    ("leaf_litter", &["dirt", "grass"]),
    ("farmland", &["dirt"]),
    ("marsh", &["mud"]),
    ("reed_bed", &["mud"]),
    ("snow", &["sand"]),
    ("ice", &["sand"]),
    ("rug", &["planks"]),
];

/// Nearest-first substitutes per wall kit.
const KITS: &[(&str, &[&str])] = &[
    ("drystone", &["stone"]),
    ("city_wall", &["stone"]),
    ("palisade", &["timber"]),
    ("wattle", &["timber"]),
    ("hedge", &["timber"]),
];

/// Nearest-first substitutes per asset id.
const ASSETS: &[(&str, &[&str])] = &[
    (
        "prop.bridge_deck_stone",
        &["prop.dock_planks", "prop.bridge_deck"],
    ),
    ("prop.signpost", &["prop.fence"]),
    (
        "prop.milestone",
        &["veg.rock_small", "veg.stones", "veg.boulder"],
    ),
    ("prop.marker_post", &["prop.signpost", "prop.fence"]),
    ("prop.ferry_rope", &["prop.fence"]),
    ("prop.ferry_boat", &["prop.rowboat"]),
    ("prop.lantern", &["prop.brazier"]),
    ("prop.hearth", &["prop.brazier"]),
    ("prop.chair", &["prop.bench"]),
    ("prop.stool", &["prop.bench"]),
    ("prop.cupboard", &["prop.shelf", "prop.chest", "prop.crate"]),
    ("prop.shelf", &["prop.crate"]),
    ("prop.bar_counter", &["prop.table"]),
    ("veg.cattail", &["veg.reeds"]),
    (
        "veg.tree_pine",
        &["veg.tree_spruce", "veg.tree_birch", "veg.tree_elm"],
    ),
    (
        "veg.tree_spruce",
        &["veg.tree_pine", "veg.tree_birch", "veg.tree_elm"],
    ),
    ("veg.tree_willow", &["veg.tree_birch", "veg.tree_elm"]),
    ("veg.rock_large", &["veg.boulder"]),
    ("veg.rock_small", &["veg.stones", "veg.boulder"]),
    ("veg.scree_patch", &["veg.stones"]),
];

fn chain(
    table: &'static [(&'static str, &'static [&'static str])],
    key: &str,
) -> &'static [&'static str] {
    table
        .iter()
        .find(|(k, _)| *k == key)
        .map_or(&[], |(_, v)| *v)
}

/// A substitution log, merged per `(kind, wanted, used)`.
#[derive(Debug, Clone, Default)]
pub struct Log(BTreeMap<(FallbackKind, String, String), u32>);

impl Log {
    fn note(&mut self, kind: FallbackKind, wanted: &str, used: &str) {
        *self
            .0
            .entry((kind, wanted.to_string(), used.to_string()))
            .or_insert(0) += 1;
    }

    /// The records, sorted.
    #[must_use]
    pub fn records(&self) -> Vec<Fallback> {
        self.0
            .iter()
            .map(|((kind, wanted, used), count)| Fallback {
                kind: *kind,
                wanted: wanted.clone(),
                used: used.clone(),
                count: *count,
            })
            .collect()
    }

    /// Adds another log's counts.
    pub fn merge(&mut self, other: &Self) {
        for (k, v) in &other.0 {
            *self.0.entry(k.clone()).or_insert(0) += v;
        }
    }
}

fn nearest_rotation(allowed: &[u16], want: u16) -> u16 {
    let diff = |a: u16| {
        let d = (i32::from(a) - i32::from(want)).rem_euclid(360);
        d.min(360 - d)
    };
    allowed
        .iter()
        .copied()
        .min_by_key(|&a| (diff(a), a))
        .unwrap_or(0)
}

/// Rewrites every key the library cannot draw to its nearest substitute,
/// fixing rotations and mirroring the substitute does not allow. Unknown
/// ground falls back to `dirt`; placements with no substitute are dropped.
pub fn degrade(layout: &mut TacticalLayout, lib: &Library) -> Log {
    let mut log = Log::default();
    for sq in &mut layout.squares {
        if lib.textures(&sq.ground).is_empty() {
            let wanted = sq.ground.clone();
            let used = chain(GROUND, &wanted)
                .iter()
                .find(|k| !lib.textures(k).is_empty())
                .copied()
                .unwrap_or("dirt");
            log.note(FallbackKind::Ground, &wanted, used);
            used.clone_into(&mut sq.ground);
        }
    }
    for w in &mut layout.walls {
        if lib.wall_pieces(&w.kit, w.kind).is_empty() {
            let wanted = w.kit.clone();
            let kind = w.kind;
            let used = chain(KITS, &wanted)
                .iter()
                .find(|k| !lib.wall_pieces(k, kind).is_empty())
                .copied()
                .unwrap_or("stone");
            log.note(FallbackKind::Kit, &wanted, used);
            used.clone_into(&mut w.kit);
        }
    }
    let mut kept = Vec::with_capacity(layout.placements.len());
    for mut p in std::mem::take(&mut layout.placements) {
        if let AssetRef::Id(id) = &p.asset {
            if lib.asset(id).is_none() {
                let wanted = id.clone();
                let used = resolve(lib, &wanted);
                log.note(FallbackKind::Asset, &wanted, used.unwrap_or("(dropped)"));
                match used {
                    Some(u) => p.asset = AssetRef::Id(u.to_string()),
                    None => continue,
                }
            }
            if let AssetRef::Id(id) = &p.asset {
                if let Some(a) = lib.asset(id) {
                    p.rotation = nearest_rotation(&a.rotations, p.rotation);
                    p.mirror &= a.mirror;
                }
            }
        }
        kept.push(p);
    }
    layout.placements = kept;
    log
}

/// The first substitute the library has, following chains transitively.
fn resolve(lib: &Library, id: &str) -> Option<&'static str> {
    let mut frontier: Vec<&'static str> = chain(ASSETS, id).to_vec();
    let mut seen: Vec<&str> = vec![id];
    while let Some(k) = frontier.first().copied() {
        frontier.remove(0);
        if seen.contains(&k) {
            continue;
        }
        if lib.asset(k).is_some() {
            return Some(k);
        }
        seen.push(k);
        frontier.extend_from_slice(chain(ASSETS, k));
    }
    None
}

/// Whether a wall role is available in some kit (for reports).
#[must_use]
pub fn has_kit(lib: &Library, kit: &str) -> bool {
    !lib.wall_pieces(kit, WallRole::Run).is_empty()
}
