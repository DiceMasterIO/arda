//! Placeholder vegetation and rocks: trees on the canopy layer, low plants,
//! rocks and logs on the prop layer, lily pads floating on the floor layer.

pub mod plants;
pub mod rocks;
pub mod trees;

use super::relief::Relief;
use super::{base, PPSQ};
use crate::catalog::{Asset, AssetClass, Cover, Layer};
use crate::compose::FIXED_POSE;
use crate::noise::hash_str;
use crate::raster::Rgba;

/// How to paint one vegetation asset.
#[derive(Clone, Copy)]
enum Look {
    Broadleaf([u8; 3], bool, Option<[u8; 3]>),
    Conifer([u8; 3], bool),
    Willow,
    Dead,
    Bush(bool),
    Reeds,
    Cattail,
    Fern,
    Heather,
    Flowers,
    TallGrass,
    Lilies,
    Mushrooms,
    Boulders(u32),
    RockLarge,
    Scree,
    Log,
    Stump,
    Stunted,
    Juniper,
    Outcrop,
    Stalagmite,
}

/// A vegetation description: id, size in squares, look, height, cover,
/// whether it blocks movement, free tags and extra biomes.
struct VegSpec {
    id: &'static str,
    size: (u32, u32),
    look: Look,
    height_ft: u16,
    cover: Cover,
    blocks: bool,
    free: &'static [&'static str],
    biomes: &'static [&'static str],
}

#[allow(clippy::too_many_arguments)]
const fn v(
    id: &'static str,
    size: (u32, u32),
    look: Look,
    height_ft: u16,
    cover: Cover,
    blocks: bool,
    free: &'static [&'static str],
    biomes: &'static [&'static str],
) -> VegSpec {
    VegSpec {
        id,
        size,
        look,
        height_ft,
        cover,
        blocks,
        free,
        biomes,
    }
}

use Cover::{Half, None as Open, ThreeQuarters as Most};
use Look::{Boulders, Broadleaf, Bush, Conifer};

const COLD: &[&str] = &["boreal", "alpine"];
const WET: &[&str] = &["wetland"];
const HILL: &[&str] = &["alpine"];

#[rustfmt::skip]
const VEGETATION: [VegSpec; 43] = [
    v("veg.tree_oak", (3, 3), Broadleaf([78, 112, 44], false, None), 30, Half, false, &["tree", "forest", "broadleaf"], &[]),
    v("veg.tree_elm", (3, 3), Broadleaf([62, 98, 50], false, None), 35, Half, false, &["tree", "forest", "broadleaf"], &[]),
    v("veg.tree_birch", (2, 2), Broadleaf([124, 152, 62], true, None), 25, Half, false, &["tree", "forest", "broadleaf"], &[]),
    v("veg.tree_fruit", (2, 2), Broadleaf([86, 126, 50], false, Some([200, 60, 44])), 15, Half, false, &["tree", "orchard", "broadleaf"], &[]),
    v("veg.tree_pine", (2, 2), Conifer([56, 96, 60], false), 40, Half, false, &["tree", "forest", "conifer"], COLD),
    v("veg.tree_spruce", (2, 2), Conifer([44, 80, 64], true), 45, Half, false, &["tree", "forest", "conifer"], COLD),
    v("veg.tree_willow", (3, 3), Look::Willow, 30, Half, false, &["tree", "riverside", "broadleaf"], WET),
    v("veg.tree_dead", (2, 2), Look::Dead, 25, Open, false, &["tree", "dead"], COLD),
    v("veg.tree_alder", (2, 2), Broadleaf([56, 88, 46], false, None), 30, Half, false, &["tree", "riverside", "broadleaf"], WET),
    v("veg.tree_stunted", (1, 1), Look::Stunted, 8, Half, false, &["tree", "conifer", "stunted"], HILL),
    v("veg.juniper", (1, 1), Look::Juniper, 4, Half, false, &["bush", "conifer"], HILL),
    v("veg.bush", (1, 1), Bush(false), 3, Half, false, &["bush"], &[]),
    v("veg.bush_flowering", (1, 1), Bush(true), 3, Half, false, &["bush", "flower"], &[]),
    v("veg.reeds", (1, 1), Look::Reeds, 5, Half, false, &["reeds", "water_plant"], WET),
    v("veg.cattail", (1, 1), Look::Cattail, 5, Half, false, &["reeds", "water_plant"], WET),
    v("veg.fern", (1, 1), Look::Fern, 2, Open, false, &["forest", "undergrowth"], &[]),
    v("veg.heather", (1, 1), Look::Heather, 2, Open, false, &["heath", "undergrowth", "flower"], HILL),
    v("veg.flower_patch", (1, 1), Look::Flowers, 1, Open, false, &["flower", "meadow"], &[]),
    v("veg.tall_grass", (1, 1), Look::TallGrass, 3, Open, false, &["grass", "meadow", "undergrowth"], &[]),
    v("veg.lily_pads", (1, 1), Look::Lilies, 0, Open, false, &["water_plant"], WET),
    v("veg.mushroom_ring", (1, 1), Look::Mushrooms, 0, Open, false, &["forest", "fungus"], &[]),
    v("veg.fallen_log", (2, 1), Look::Log, 2, Half, false, &["forest", "log"], &[]),
    v("veg.stump", (1, 1), Look::Stump, 2, Half, false, &["forest", "stump"], &[]),
    v("veg.boulder", (1, 1), Boulders(1), 4, Most, true, &["rock"], HILL),
    v("veg.stones", (1, 1), Boulders(3), 2, Half, true, &["rock"], HILL),
    v("veg.rock_small", (1, 1), Boulders(1), 3, Half, true, &["rock"], HILL),
    v("veg.rock_large", (2, 2), Look::RockLarge, 8, Cover::Full, true, &["rock"], HILL),
    v("veg.scree_patch", (2, 2), Look::Scree, 1, Open, false, &["rock", "scree"], HILL),
    v("veg.rock_outcrop", (3, 3), Look::Outcrop, 10, Cover::Full, true, &["rock", "outcrop"], HILL),
    // Biome scatter (arda-refine `biome`): coast, alpine, steppe and marsh;
    // not tagged `rock` either, so dressing queries never pick them.
    v("veg.driftwood", (2, 1), Look::Log, 2, Half, false, &["deadwood", "coast"], &[]),
    v("veg.sea_rock", (1, 1), Boulders(2), 5, Most, true, &["sea_rock", "coast"], &[]),
    v("veg.tide_pool", (2, 2), Look::Scree, 0, Open, false, &["coast", "tide_pool"], &[]),
    v("veg.dune_grass", (1, 1), Look::TallGrass, 2, Open, false, &["coast", "dune"], &[]),
    v("veg.lichen_rock", (1, 1), Boulders(1), 3, Half, true, &["stone", "lichen"], HILL),
    v("veg.rock_snow", (1, 1), Boulders(1), 4, Most, true, &["stone", "snow"], HILL),
    v("veg.alpine_flowers", (1, 1), Look::Heather, 1, Open, false, &["alpine_plant", "flower"], HILL),
    v("veg.tussock", (1, 1), Look::TallGrass, 2, Open, false, &["tussock", "steppe"], &[]),
    v("veg.krummholz", (1, 1), Look::Stunted, 6, Half, false, &["conifer", "stunted", "krummholz"], HILL),
    v("veg.sagebrush", (1, 1), Look::Juniper, 3, Half, false, &["shrub", "steppe"], &[]),
    v("veg.dry_grass", (1, 1), Look::TallGrass, 2, Open, false, &["dry_grass", "steppe"], &[]),
    v("veg.sedge", (1, 1), Look::TallGrass, 2, Open, false, &["sedge", "water_plant"], WET),
    v("veg.marsh_flowers", (1, 1), Look::Flowers, 1, Open, false, &["flower", "marsh_plant"], WET),
    // Cave dressing (arda-dungeon); deliberately not tagged `rock`, so the
    // field and wild dressing queries never pick it.
    v("veg.stalagmite", (1, 1), Look::Stalagmite, 6, Most, true, &["cave", "stalagmite"], &[]),
];

fn paint(look: Look, r: &mut Relief, seed: u64) {
    match look {
        Broadleaf(leaf, airy, fruit) => trees::broadleaf(r, seed, leaf, airy, fruit),
        Conifer(leaf, spruce) => trees::conifer(r, seed, leaf, spruce),
        Look::Willow => trees::willow(r, seed, [132, 158, 70]),
        Look::Dead => trees::dead(r, seed),
        Bush(flowers) => plants::bush(
            r,
            seed,
            if flowers {
                [84, 118, 56]
            } else {
                [70, 110, 48]
            },
            flowers,
        ),
        Look::Reeds => plants::reeds(r, seed, 4),
        Look::Cattail => plants::cattail(r, seed),
        Look::Fern => plants::fern(r, seed),
        Look::Heather => plants::heather(r, seed),
        Look::Flowers => plants::flower_patch(r, seed),
        Look::TallGrass => plants::tall_grass(r, seed),
        Look::Lilies => plants::lily_pads(r, seed),
        Look::Mushrooms => plants::mushroom_ring(r, seed),
        Boulders(n) => rocks::boulders(r, seed, n),
        Look::RockLarge => rocks::rock_large(r, seed),
        Look::Scree => rocks::scree_patch(r, seed),
        Look::Log => rocks::fallen_log(r, seed),
        Look::Stump => rocks::stump(r, seed),
        Look::Stunted => trees::stunted(r, seed, [74, 96, 66]),
        Look::Juniper => plants::bush(r, seed, [66, 100, 72], false),
        Look::Outcrop => rocks::outcrop(r, seed),
        Look::Stalagmite => super::props::dungeon::stalagmite(r, seed),
    }
}

/// Every placeholder vegetation and rock asset with its image.
#[must_use]
pub fn assets(seed: u64) -> Vec<(Asset, Rgba)> {
    VEGETATION
        .iter()
        .map(|spec| {
            let (w, h) = spec.size;
            let tree = spec.id.starts_with("veg.tree_");
            let layer = match spec.look {
                Look::Lilies => Layer::Floor,
                _ if tree => Layer::Canopy,
                _ => Layer::Prop,
            };
            let mut a = base(
                spec.id,
                AssetClass::Vegetation,
                "vegetation",
                w,
                h,
                layer,
                seed,
            );
            a.height_ft = spec.height_ft;
            a.casts_shadow = spec.height_ft > 0;
            a.cover = spec.cover;
            a.blocks_movement = spec.blocks;
            a.difficult_terrain =
                !spec.blocks && !tree && spec.height_ft > 0 || spec.id == "veg.scree_patch";
            a.blocks_sight = matches!(spec.cover, Cover::Full | Cover::ThreeQuarters);
            a.tags.free = spec.free.iter().map(|s| (*s).to_string()).collect();
            // Placeholder art keeps its catalogued size and orientation, so
            // placeholder renders do not change with the hashed pose.
            a.tags.free.push(FIXED_POSE.to_string());
            a.tags
                .biome
                .extend(spec.biomes.iter().map(|s| (*s).to_string()));
            if tree {
                a.placement.clearance_squares = 1;
            }
            match spec.id {
                "veg.reeds" | "veg.cattail" => {
                    a.placement.ground = ["mud", "water_shallow", "grass", "marsh", "reed_bed"]
                        .map(String::from)
                        .to_vec();
                }
                "veg.lily_pads" => a.placement.on_water = true,
                "veg.fallen_log" | "veg.driftwood" => a.rotations = vec![0, 90, 180, 270],
                "veg.sea_rock" => a.placement.on_water = true,
                _ => {}
            }
            let mut r = Relief::new(w * PPSQ, h * PPSQ);
            paint(spec.look, &mut r, hash_str(seed, spec.id));
            (a, r.finish(1.0))
        })
        .collect()
}
