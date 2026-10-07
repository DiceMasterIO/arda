//! The shared tactical vocabulary (`docs/goal-prompts/vocabulary.md`),
//! mirrored here so the importer can report unknown ids and coverage gaps.
//! Extend these lists only together with that file.

/// Ground and water keys (`Square.ground`).
pub const GROUND_KEYS: &[&str] = &[
    "grass",
    "dirt",
    "cobbles",
    "mud",
    "sand",
    "gravel",
    "stone_floor",
    "planks",
    "water_shallow",
    "water_deep",
    "meadow",
    "forest_floor",
    "leaf_litter",
    "heath",
    "scrub",
    "moss",
    "scree",
    "rock",
    "cliff",
    "snow",
    "ice",
    "marsh",
    "reed_bed",
    "farmland",
    "pasture",
    "stubble",
    "fallow",
    "packed_earth",
    "flagstone",
    "rug",
    "salt_crust",
    "mudflat",
    "trail",
    "cave_floor",
    "bedrock",
];

/// Wall kits (`WallSegment.kit`).
pub const WALL_KITS: &[&str] = &[
    "stone",
    "timber",
    "wattle",
    "palisade",
    "hedge",
    "drystone",
    "city_wall",
    "cave",
];

/// Prop names (`prop.<name>`).
pub const PROPS: &[&str] = &[
    "barrel",
    "crate",
    "sacks",
    "chest",
    "table",
    "bench",
    "bed",
    "cart",
    "rowboat",
    "market_stall",
    "tent",
    "fence",
    "dock_planks",
    "bridge_deck",
    "crane",
    "well",
    "brazier",
    "woodpile",
    "chair",
    "stool",
    "cupboard",
    "shelf",
    "bookshelf",
    "hearth",
    "oven",
    "bar_counter",
    "cask_rack",
    "anvil",
    "forge",
    "workbench",
    "loom",
    "grindstone",
    "weapon_rack",
    "armour_stand",
    "altar",
    "pew",
    "candle_stand",
    "statue",
    "hay_bale",
    "trough",
    "millstone",
    "bucket",
    "wheelbarrow",
    "ladder",
    "throne",
    "banner",
    "rug_small",
    "lantern",
    "signpost",
    "grave",
    "haycart",
    "waterwheel",
    "sheep",
    "cow",
    "hen",
    "stairs",
    "milestone",
    "marker_post",
    "ferry_rope",
    "ferry_boat",
    "bridge_deck_stone",
    "drain",
    "tomb",
    "coffin",
    "bone_pile",
    "skeleton",
    "cage",
    "gaol_cot",
    "rubble_pile",
    "chest_treasure",
    "stairs_down",
    "torch_sconce",
];

/// Vegetation names (`veg.<name>`).
pub const VEGETATION: &[&str] = &[
    "tree_oak",
    "tree_elm",
    "tree_birch",
    "tree_fruit",
    "bush",
    "bush_flowering",
    "reeds",
    "boulder",
    "stones",
    "tree_pine",
    "tree_spruce",
    "tree_willow",
    "tree_dead",
    "fallen_log",
    "stump",
    "fern",
    "mushroom_ring",
    "heather",
    "flower_patch",
    "tall_grass",
    "cattail",
    "lily_pads",
    "rock_small",
    "rock_large",
    "scree_patch",
    "tree_alder",
    "tree_stunted",
    "juniper",
    "rock_outcrop",
    "stalagmite",
];

/// Building functions (`function` tags).
pub const FUNCTIONS: &[&str] = &[
    "house",
    "farmhouse",
    "cottage",
    "manor",
    "inn",
    "tavern",
    "bakery",
    "brewery",
    "mill",
    "smithy",
    "workshop",
    "tannery",
    "apothecary",
    "temple",
    "shrine",
    "library",
    "warehouse",
    "market",
    "stall",
    "dock",
    "boathouse",
    "keep",
    "barracks",
    "guardhouse",
    "stable",
    "barn",
    "toll_house",
    "waystation",
    "street",
    "farm",
    "market_hall",
    "mine",
    "lumber_camp",
    "school",
];

/// Every vocabulary slot, as the coverage report names them:
/// `ground:<key>`, `wall:<kit>`, `prop.<name>` and `veg.<name>`.
#[must_use]
pub fn all_slots() -> Vec<String> {
    let mut out: Vec<String> = GROUND_KEYS.iter().map(|k| format!("ground:{k}")).collect();
    out.extend(WALL_KITS.iter().map(|k| format!("wall:{k}")));
    out.extend(PROPS.iter().map(|p| format!("prop.{p}")));
    out.extend(VEGETATION.iter().map(|v| format!("veg.{v}")));
    out
}

/// The closest known word within an edit distance of 3, for "did you mean".
#[must_use]
pub fn suggest<'a>(word: &str, known: &[&'a str]) -> Option<&'a str> {
    known
        .iter()
        .map(|k| (edit_distance(word, k), *k))
        .filter(|(d, _)| *d <= 3)
        .min()
        .map(|(_, k)| k)
}

/// Levenshtein distance over bytes (ids are ASCII).
#[must_use]
pub fn edit_distance(a: &str, b: &str) -> usize {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, &ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, &cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != cb);
            cur[j + 1] = sub.min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_near_misses_only() {
        assert_eq!(suggest("anvill", PROPS), Some("anvil"));
        assert_eq!(suggest("tree_ok", VEGETATION), Some("tree_oak"));
        assert_eq!(suggest("spaceship_hangar", PROPS), None);
    }

    #[test]
    fn slots_are_unique() {
        let slots = all_slots();
        let set: std::collections::BTreeSet<_> = slots.iter().collect();
        assert_eq!(set.len(), slots.len());
    }
}
