//! Resolving a canonical block onto a concrete library (goal 58: the real
//! library must work without code changes; goal 61: until then, the
//! placeholders stand in).
//!
//! Every ground key, wall kit and prop the library lacks is replaced by its
//! nearest available relative along a fixed chain, and every substitution
//! is recorded. Tag queries (`key:value` strings, convention I8) are
//! resolved here, by hashing the placement's world square, so a query picks
//! the same asset in every block that shows it.

use super::TownBlock;
use crate::num::{bits, floor_i};
use crate::rng::hash_i;
use arda_tactical::catalog::{Asset, AssetClass, WallRole};
use arda_tactical::layout::{AssetRef, TacticalLayout};
use arda_tactical::Library;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// What kind of key was replaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackKind {
    /// A ground key.
    Ground,
    /// A wall kit.
    Kit,
    /// A prop or vegetation id.
    Prop,
    /// A tag query.
    Query,
}

/// One recorded substitution.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Fallback {
    /// Kind.
    pub kind: FallbackKind,
    /// Key the layout asked for.
    pub wanted: String,
    /// Key used instead (`-` when omitted).
    pub used: String,
    /// How often.
    pub count: u32,
}

/// Nearest relatives of ground keys, most similar first.
#[must_use]
pub fn ground_chain(key: &str) -> &'static [&'static str] {
    match key {
        "meadow" | "pasture" | "heath" | "scrub" | "moss" | "forest_floor" => &["grass", "dirt"],
        "leaf_litter" | "farmland" | "packed_earth" => &["dirt", "mud", "grass"],
        "flagstone" => &["stone_floor", "cobbles", "gravel"],
        "rug" => &["planks", "stone_floor"],
        "scree" | "rock" | "cliff" => &["gravel", "stone_floor", "dirt"],
        "marsh" | "reed_bed" => &["mud", "grass"],
        "salt_crust" => &["sand", "gravel"],
        "mudflat" => &["mud", "sand", "dirt"],
        "snow" | "ice" => &["sand", "gravel"],
        _ => &["grass", "dirt"],
    }
}

/// Nearest relatives of wall kits.
#[must_use]
pub fn kit_chain(kit: &str) -> &'static [&'static str] {
    match kit {
        "wattle" | "palisade" | "hedge" => &["timber", "stone"],
        "log" => &["timber", "wattle", "stone"],
        "adobe" => &["stone", "drystone", "timber"],
        "drystone" | "city_wall" => &["stone", "timber"],
        "timber" => &["stone"],
        _ => &["stone", "timber"],
    }
}

/// Nearest relatives of props, by use and footprint; `None` means the
/// item is purely decorative and may be left out.
#[must_use]
pub fn prop_chain(id: &str) -> Option<&'static [&'static str]> {
    Some(match id {
        "prop.chair" => &["prop.stool", "prop.barrel"],
        "prop.stool" => &["prop.chair", "prop.barrel"],
        "prop.cupboard" => &["prop.shelf", "prop.chest"],
        "prop.shelf" => &["prop.bookshelf", "prop.cupboard", "prop.table"],
        "prop.bookshelf" => &["prop.shelf", "prop.table"],
        "prop.scroll_rack" => &["prop.bookshelf", "prop.shelf"],
        "prop.lectern" => &["prop.stool", "prop.chair"],
        "prop.hearth" | "prop.forge" => &["prop.oven", "prop.brazier"],
        "prop.oven" => &["prop.hearth", "prop.brazier"],
        "prop.bar_counter" | "prop.altar" | "prop.workbench" => &["prop.table"],
        "prop.loom" => &["prop.workbench", "prop.table"],
        "prop.cask_rack" => &["prop.barrel"],
        "prop.anvil" | "prop.grindstone" => &["prop.anvil", "prop.chest"],
        "prop.weapon_rack" => &["prop.shelf", "prop.table"],
        "prop.armour_stand" => &["prop.weapon_rack", "prop.chest"],
        "prop.pew" => &["prop.bench"],
        "prop.trough" => &["prop.bench"],
        "prop.candle_stand" | "prop.lantern" => &["prop.brazier"],
        "prop.statue" => &["veg.boulder"],
        "prop.hay_bale" => &["prop.sacks"],
        "prop.millstone" => &["prop.well"],
        "prop.bucket" => &["prop.barrel"],
        "prop.wheelbarrow" => &["prop.crate"],
        "prop.ladder" | "prop.signpost" => &["prop.fence"],
        "prop.stairs" => &["prop.ladder", "prop.bridge_deck"],
        "prop.throne" => &["prop.chair", "prop.chest"],
        "prop.grave" => &["veg.stones"],
        "prop.haycart" => &["prop.cart"],
        "prop.banner" | "prop.rug_small" | "prop.drain" => return None,
        "veg.tree_pine" | "veg.tree_spruce" => &["veg.tree_birch"],
        "veg.tree_willow" | "veg.tree_dead" => &["veg.tree_elm"],
        "veg.fallen_log" | "veg.stump" | "veg.rock_small" | "veg.scree_patch" => &["veg.stones"],
        "veg.rock_large" => &["veg.boulder"],
        "veg.fern" | "veg.heather" | "veg.flower_patch" | "veg.tall_grass"
        | "veg.mushroom_ring" => &["veg.bush", "veg.bush_flowering"],
        "veg.bush_flowering" => &["veg.bush"],
        "veg.cattail" | "veg.lily_pads" => &["veg.reeds"],
        _ => &["prop.crate"],
    })
}

/// Whether an asset matches a `key:value` tag (or a bare free tag).
fn has_tag(a: &Asset, tag: &str) -> bool {
    let t = &a.tags;
    match tag.split_once(':') {
        Some(("biome", v)) => t.biome.iter().any(|x| x == v),
        Some(("culture", v)) => t.culture.iter().any(|x| x == v),
        Some(("wealth", v)) => t.wealth.iter().any(|x| x == v),
        Some(("function", v)) => t.function.iter().any(|x| x == v),
        _ => t
            .free
            .iter()
            .any(|x| x == tag || tag.split_once(':').is_some_and(|(_, v)| x == v)),
    }
}

/// Assets of `class` carrying every tag.
fn query<'a>(lib: &'a Library, class: Option<AssetClass>, tags: &[String]) -> Vec<&'a Asset> {
    lib.catalog
        .assets
        .iter()
        .filter(|a| class.is_none_or(|c| a.class == c))
        .filter(|a| tags.iter().all(|t| has_tag(a, t)))
        .collect()
}

/// The resolved layout and what was substituted.
#[derive(Debug, Clone)]
pub struct Resolved {
    /// A layout every key of which exists in the library.
    pub layout: TacticalLayout,
    /// Substitutions, sorted.
    pub fallbacks: Vec<Fallback>,
}

struct Log(BTreeMap<(FallbackKind, String, String), u32>);

impl Log {
    fn note(&mut self, kind: FallbackKind, wanted: &str, used: &str) {
        *self
            .0
            .entry((kind, wanted.to_string(), used.to_string()))
            .or_default() += 1;
    }
}

fn pick_rotation(a: &Asset, rot: u16) -> u16 {
    if a.rotations.contains(&rot) {
        rot
    } else if a.rotations.contains(&((rot + 180) % 360)) {
        (rot + 180) % 360
    } else {
        a.rotations.first().copied().unwrap_or(0)
    }
}

/// Replaces every rug square with the nearest other floor along its row,
/// never looking past a wall (review 3.5-14: a rug against a wall took the
/// street's cobbles from beyond it).
fn unrug(out: &mut TacticalLayout, log: &mut Log) {
    let w = usize::try_from(out.width).unwrap_or(1).max(1);
    let walled: std::collections::BTreeSet<(u32, u32)> = out
        .walls
        .iter()
        .filter(|s| s.axis == arda_tactical::layout::EdgeAxis::Vertical)
        .map(|s| (s.x, s.y))
        .collect();
    let keys: Vec<String> = out.squares.iter().map(|s| s.ground.clone()).collect();
    let edge = |x: usize, y: usize| {
        walled.contains(&(
            u32::try_from(x).unwrap_or(u32::MAX),
            u32::try_from(y).unwrap_or(u32::MAX),
        ))
    };
    for (i, sq) in out.squares.iter_mut().enumerate() {
        if sq.ground != "rug" {
            continue;
        }
        let (x, y) = (i % w, i / w);
        let row = y * w;
        // The squares reachable along the row without crossing a wall,
        // nearest first, west before east.
        let (mut west, mut east) = (true, true);
        let mut under = None;
        for d in 1..w {
            // West to x - d crosses the west edge of x - d + 1.
            west &= x >= d && !edge(x + 1 - d, y);
            east &= x + d < w && !edge(x + d, y);
            let found = [west.then(|| row + x - d), east.then(|| row + x + d)]
                .into_iter()
                .flatten()
                .map(|j| &keys[j])
                .find(|k| *k != "rug");
            if let Some(k) = found {
                under = Some(k.clone());
                break;
            }
            if !west && !east {
                break;
            }
        }
        let under = under.unwrap_or_else(|| "planks".into());
        log.note(FallbackKind::Ground, "rug", &under);
        sq.ground = under;
    }
}

/// Maps a canonical block onto `lib`.
#[must_use]
pub fn resolve(block: &TownBlock, lib: &Library, seed: u64) -> Resolved {
    let mut log = Log(BTreeMap::new());
    let mut out = block.layout.clone();
    // A missing rug disappears into the floor around it.
    if lib.textures("rug").is_empty() {
        unrug(&mut out, &mut log);
    }
    let mut ground_map: BTreeMap<String, String> = BTreeMap::new();
    for sq in &mut out.squares {
        if lib.textures(&sq.ground).is_empty() {
            let used = ground_map.entry(sq.ground.clone()).or_insert_with(|| {
                ground_chain(&sq.ground)
                    .iter()
                    .find(|k| !lib.textures(k).is_empty())
                    .map_or_else(|| "grass".to_string(), |k| (*k).to_string())
            });
            log.note(FallbackKind::Ground, &sq.ground, used);
            sq.ground.clone_from(used);
        }
    }
    for w in &mut out.walls {
        if lib.wall_pieces(&w.kit, w.kind).is_empty() {
            let role = w.kind;
            let used = kit_chain(&w.kit)
                .iter()
                .find(|k| !lib.wall_pieces(k, role).is_empty())
                .map_or("stone", |k| k);
            log.note(FallbackKind::Kit, &w.kit, used);
            w.kit = used.to_string();
            if lib.wall_pieces(&w.kit, role).is_empty() {
                w.kind = WallRole::Run;
            }
        }
    }
    let (ox, oy) = (block.origin[0], block.origin[1]);
    let mut keep = vec![true; out.placements.len()];
    let mut has_light = vec![false; out.placements.len()];
    for (i, p) in out.placements.iter_mut().enumerate() {
        let gx = ox + floor_i(f64::from(p.x));
        let gy = oy + floor_i(f64::from(p.y));
        let chosen: Option<&Asset> = match &p.asset {
            AssetRef::Id(id) => lib.asset(id).or_else(|| {
                let found = prop_chain(id).and_then(|c| c.iter().find_map(|k| lib.asset(k)));
                log.note(FallbackKind::Prop, id, found.map_or("-", |a| a.id.as_str()));
                found
            }),
            AssetRef::Query { class, tags } => {
                let mut t = tags.clone();
                let mut all = query(lib, *class, &t);
                while all.is_empty() && !t.is_empty() {
                    t.pop();
                    all = query(lib, *class, &t);
                }
                let n = all.len().max(1) as u64;
                let pick = all
                    .get(usize::try_from(hash_i(seed ^ bits(gx), gx, gy) % n).unwrap_or(0))
                    .copied();
                if t.len() < tags.len() {
                    log.note(
                        FallbackKind::Query,
                        &tags.join(","),
                        pick.map_or("-", |a| a.id.as_str()),
                    );
                }
                pick
            }
        };
        match chosen {
            Some(a) => {
                p.rotation = pick_rotation(a, p.rotation);
                p.mirror = p.mirror && a.mirror;
                has_light[i] = a.light.is_some();
                p.asset = AssetRef::Id(a.id.clone());
            }
            None => keep[i] = false,
        }
    }
    // Drop explicit lights whose prop now glows by itself, then renumber.
    let lights: Vec<_> = out
        .lights
        .iter()
        .zip(block.light_owner.iter().chain(std::iter::repeat(&None)))
        .filter(|(_, owner)| {
            owner.is_none_or(|o| keep.get(o).copied().unwrap_or(false) && !has_light[o])
        })
        .map(|(l, _)| *l)
        .collect();
    out.lights = lights;
    let placements = std::mem::take(&mut out.placements);
    out.placements = placements
        .into_iter()
        .zip(keep)
        .filter_map(|(p, k)| k.then_some(p))
        .collect();
    let fallbacks = log
        .0
        .into_iter()
        .map(|((kind, wanted, used), count)| Fallback {
            kind,
            wanted,
            used,
            count,
        })
        .collect();
    Resolved {
        layout: out,
        fallbacks,
    }
}

#[cfg(test)]
mod rug_tests {
    use super::*;
    use arda_tactical::layout::{EdgeAxis, WallSegment};

    /// Review 3.5-14: a rug against a wall takes the room's floor, not the
    /// street beyond the wall.
    #[test]
    fn a_rug_never_takes_the_floor_beyond_a_wall() {
        let mut l = TacticalLayout::new("t", 6, 1, "planks");
        l.squares[0].ground = "cobbles".into();
        for x in 1..4 {
            l.squares[x].ground = "rug".into();
        }
        l.walls.push(WallSegment {
            x: 1,
            y: 0,
            axis: EdgeAxis::Vertical,
            kind: arda_tactical::WallRole::Run,
            kit: "stone".into(),
            tags: Vec::new(),
        });
        let mut log = Log(BTreeMap::new());
        unrug(&mut l, &mut log);
        let g: Vec<&str> = l.squares.iter().map(|s| s.ground.as_str()).collect();
        assert_eq!(
            g,
            ["cobbles", "planks", "planks", "planks", "planks", "planks"]
        );
    }
}
