//! The placeholder prop table: ids, footprints, painters, heights, cover
//! and the `function` tags that dressing queries such as `function:inn`
//! or `function:smithy` use to find furniture.

use super::props::{civic, farm, fire, furniture as fur, river, storage as st, work, yard};
use super::relief::Relief;
use super::{base, PPSQ};
use crate::catalog::{Asset, AssetClass, Cover, Layer, Light};
use crate::noise::hash_str;
use crate::raster::Rgba;

type Painter = fn(&mut Relief, u64, f32);

/// A prop description.
pub struct PropSpec {
    /// Asset id.
    pub id: &'static str,
    /// Footprint in squares.
    pub size: (u32, u32),
    paint: Painter,
    /// Height hint in feet.
    pub height_ft: u16,
    cover: Cover,
    /// Building functions the prop dresses.
    pub function: &'static [&'static str],
    free: &'static [&'static str],
}

const fn p(
    id: &'static str,
    size: (u32, u32),
    paint: Painter,
    height_ft: u16,
    cover: Cover,
    function: &'static [&'static str],
    free: &'static [&'static str],
) -> PropSpec {
    PropSpec {
        id,
        size,
        paint,
        height_ft,
        cover,
        function,
        free,
    }
}

const SEATS: &[&str] = &[
    "house",
    "farmhouse",
    "cottage",
    "manor",
    "inn",
    "tavern",
    "library",
    "apothecary",
    "guardhouse",
    "workshop",
];
const DRINK: &[&str] = &["inn", "tavern", "brewery"];
const STORES: &[&str] = &[
    "warehouse",
    "dock",
    "market",
    "stall",
    "workshop",
    "barn",
    "boathouse",
    "market_hall",
    "mine",
    "lumber_camp",
];
const FARMYARD: &[&str] = &["farm", "barn", "stable"];
const MARTIAL: &[&str] = &["barracks", "guardhouse", "keep", "smithy"];
const FAITH: &[&str] = &["temple", "shrine"];

use Cover::{Half, None as Open, ThreeQuarters as Most};

/// Every placeholder prop.
#[rustfmt::skip]
pub const PROPS: [PropSpec; 56] = [
    p("prop.barrel", (1, 1), st::barrel, 4, Half, &["warehouse", "dock", "inn", "tavern", "brewery", "market", "boathouse", "market_hall", "mine"], &["container"]),
    p("prop.crate", (1, 1), st::crate_box, 3, Half, STORES, &["container"]),
    p("prop.sacks", (1, 1), st::sacks, 2, Half, &["warehouse", "farm", "market", "mill", "bakery", "barn", "stall", "market_hall"], &["container"]),
    p("prop.chest", (1, 1), st::chest, 2, Half, &["house", "farmhouse", "cottage", "manor", "inn", "keep", "barracks"], &["container"]),
    p("prop.table", (2, 1), fur::table, 3, Half, &["house", "farmhouse", "cottage", "manor", "inn", "tavern", "warehouse", "library", "bakery", "apothecary", "barracks", "guardhouse", "school", "market_hall"], &["furniture"]),
    p("prop.bench", (2, 1), fur::bench, 2, Open, &["house", "inn", "tavern", "street", "barracks", "farmhouse", "guardhouse", "school", "market_hall"], &["furniture", "seating"]),
    p("prop.bed", (1, 2), fur::bed, 2, Open, &["house", "farmhouse", "cottage", "manor", "inn", "barracks"], &["furniture"]),
    p("prop.cart", (1, 2), yard::cart, 4, Half, &["street", "farm", "market", "barn", "stable", "mill", "mine", "lumber_camp"], &["vehicle"]),
    p("prop.rowboat", (1, 2), yard::rowboat, 2, Open, &["dock", "boathouse"], &["vehicle", "boat"]),
    p("prop.market_stall", (2, 2), yard::market_stall, 8, Most, &["market", "stall", "street", "market_hall"], &["structure"]),
    p("prop.tent", (2, 2), yard::tent, 7, Cover::Full, &["market", "farm", "barracks", "lumber_camp"], &["structure"]),
    p("prop.fence", (1, 1), yard::fence, 4, Half, &["farm", "street", "house", "stable", "farmhouse"], &["barrier"]),
    p("prop.dock_planks", (1, 1), yard::dock, 0, Open, &["dock", "boathouse"], &["floor"]),
    p("prop.bridge_deck", (1, 2), yard::bridge, 0, Open, &["street"], &["floor"]),
    p("prop.crane", (2, 2), yard::crane, 14, Half, &["dock", "warehouse"], &["structure"]),
    p("prop.well", (1, 1), yard::well, 3, Half, &["street", "farm", "farmhouse", "market"], &["structure", "water"]),
    p("prop.brazier", (1, 1), fire::brazier, 3, Open, &["street", "inn", "keep", "temple", "guardhouse"], &["light"]),
    p("prop.woodpile", (1, 1), st::woodpile, 3, Half, &["house", "farm", "farmhouse", "cottage", "smithy", "bakery", "lumber_camp"], &["container"]),
    p("prop.chair", (1, 1), fur::chair, 3, Open, SEATS, &["furniture", "seating"]),
    p("prop.stool", (1, 1), fur::stool, 2, Open, &["inn", "tavern", "workshop", "smithy", "cottage", "farmhouse", "tannery"], &["furniture", "seating"]),
    p("prop.cupboard", (1, 1), st::cupboard, 6, Most, &["house", "farmhouse", "cottage", "manor", "apothecary", "bakery"], &["furniture", "storage"]),
    p("prop.shelf", (1, 1), st::shelf, 5, Half, &["house", "apothecary", "workshop", "bakery", "library", "tannery", "stall"], &["furniture", "storage"]),
    p("prop.bookshelf", (2, 1), st::bookshelf, 7, Most, &["library", "manor", "temple", "apothecary", "school"], &["furniture", "storage"]),
    p("prop.hearth", (2, 1), fire::hearth, 4, Half, &["house", "farmhouse", "cottage", "manor", "inn", "tavern", "keep"], &["light", "hearth"]),
    p("prop.oven", (1, 1), fire::oven, 5, Half, &["bakery", "inn", "farmhouse"], &["light", "hearth"]),
    p("prop.bar_counter", (2, 1), fur::bar_counter, 4, Half, DRINK, &["furniture"]),
    p("prop.cask_rack", (2, 1), st::cask_rack, 4, Half, &["inn", "tavern", "brewery", "warehouse"], &["container", "storage"]),
    p("prop.anvil", (1, 1), work::anvil, 3, Half, &["smithy"], &["tool", "craft:smith"]),
    p("prop.forge", (2, 1), fire::forge, 4, Half, &["smithy"], &["light", "tool", "craft:smith"]),
    p("prop.workbench", (2, 1), work::workbench, 3, Half, &["workshop", "smithy", "tannery", "boathouse", "mill", "apothecary", "lumber_camp", "school"], &["furniture", "tool", "craft:carpenter"]),
    p("prop.loom", (2, 1), work::loom, 5, Half, &["workshop", "cottage", "farmhouse"], &["tool", "craft:weaver"]),
    p("prop.grindstone", (1, 1), work::grindstone, 3, Half, &["smithy", "workshop", "mill", "lumber_camp"], &["tool", "craft:cutler"]),
    p("prop.weapon_rack", (2, 1), work::weapon_rack, 5, Half, MARTIAL, &["storage", "weapons"]),
    p("prop.armour_stand", (1, 1), work::armour_stand, 6, Half, MARTIAL, &["weapons"]),
    p("prop.altar", (2, 1), civic::altar, 4, Half, FAITH, &["religious", "light"]),
    p("prop.pew", (2, 1), fur::pew, 3, Half, &["temple"], &["furniture", "seating", "religious"]),
    p("prop.candle_stand", (1, 1), fire::candle_stand, 4, Open, &["temple", "shrine", "manor", "library"], &["light", "religious"]),
    p("prop.statue", (1, 1), civic::statue, 8, Most, &["temple", "shrine", "manor", "street", "keep"], &["decoration", "religious"]),
    p("prop.hay_bale", (1, 1), farm::hay_bale, 3, Half, FARMYARD, &["container", "hay"]),
    p("prop.trough", (2, 1), farm::trough, 2, Half, FARMYARD, &["water"]),
    p("prop.millstone", (1, 1), farm::millstone, 1, Open, &["mill"], &["tool"]),
    p("prop.bucket", (1, 1), st::bucket, 1, Open, &["farm", "stable", "barn", "street", "tannery", "brewery", "smithy", "mine"], &["container", "water"]),
    p("prop.wheelbarrow", (1, 1), farm::wheelbarrow, 2, Half, &["farm", "barn", "workshop", "mill", "mine"], &["vehicle"]),
    p("prop.ladder", (1, 2), farm::ladder, 0, Open, &["barn", "warehouse", "workshop", "stable", "library", "mine"], &["tool"]),
    p("prop.throne", (1, 1), fur::throne, 5, Half, &["keep", "manor"], &["furniture", "seating", "decoration"]),
    p("prop.banner", (1, 1), fur::banner, 8, Open, &["keep", "manor", "barracks", "guardhouse", "temple"], &["decoration"]),
    p("prop.rug_small", (2, 1), fur::rug_small, 0, Open, &["manor", "house", "inn", "temple", "library", "cottage"], &["decoration", "floor"]),
    p("prop.lantern", (1, 1), fire::lantern, 1, Open, &["street", "dock", "inn", "tavern", "stable", "guardhouse", "mine"], &["light"]),
    p("prop.signpost", (1, 1), civic::signpost, 7, Open, &["street"], &["structure"]),
    p("prop.grave", (1, 1), civic::grave, 3, Half, FAITH, &["religious"]),
    p("prop.haycart", (1, 2), farm::haycart, 7, Half, FARMYARD, &["vehicle", "hay"]),
    p("prop.milestone", (1, 1), river::milestone, 3, Half, &["street", "waystation", "toll_house"], &["marker", "road"]),
    p("prop.marker_post", (1, 1), river::marker_post, 5, Open, &["street", "waystation", "toll_house", "farm"], &["marker", "road"]),
    p("prop.ferry_rope", (1, 1), river::ferry_rope, 3, Open, &["dock", "waystation", "toll_house"], &["ferry", "water"]),
    p("prop.ferry_boat", (2, 3), river::ferry_boat, 1, Open, &["dock", "waystation", "toll_house"], &["ferry", "vehicle", "boat", "floor"]),
    p("prop.bridge_deck_stone", (1, 1), river::bridge_deck_stone, 0, Open, &["street", "toll_house"], &["floor", "bridge"]),
];

/// Lights by prop id: radius in feet and colour.
fn light(id: &str) -> Option<Light> {
    let (radius_ft, colour) = match id {
        "prop.brazier" => (20, [255, 176, 88]),
        "prop.hearth" => (20, [255, 168, 80]),
        "prop.forge" => (15, [255, 150, 70]),
        "prop.oven" => (10, [255, 160, 80]),
        "prop.lantern" => (15, [255, 204, 124]),
        "prop.candle_stand" | "prop.altar" => (10, [255, 212, 150]),
        _ => return None,
    };
    Some(Light { radius_ft, colour })
}

/// Builds the asset record and paints the image of one prop.
#[must_use]
pub fn prop_asset(spec: &PropSpec, seed: u64) -> (Asset, Rgba) {
    let (w, h) = spec.size;
    let mut a = base(spec.id, AssetClass::Prop, "props", w, h, Layer::Prop, seed);
    a.height_ft = spec.height_ft;
    a.casts_shadow = spec.height_ft > 0;
    a.cover = spec.cover;
    a.blocks_movement = spec.height_ft >= 3;
    a.difficult_terrain = (1..3).contains(&spec.height_ft);
    a.blocks_sight = matches!(spec.cover, Cover::Full | Cover::ThreeQuarters);
    a.tags.function = spec.function.iter().map(|s| (*s).to_string()).collect();
    a.tags.free = spec.free.iter().map(|s| (*s).to_string()).collect();
    a.tags.wealth = if matches!(
        spec.id,
        "prop.throne" | "prop.statue" | "prop.banner" | "prop.bookshelf"
    ) {
        vec!["modest".into(), "wealthy".into()]
    } else {
        vec!["poor".into(), "modest".into()]
    };
    a.light = light(spec.id);
    match spec.id {
        "prop.dock_planks" | "prop.bridge_deck" | "prop.bridge_deck_stone" => {
            a.layer = Layer::Floor;
            a.rotations = vec![0, 90];
            a.placement.on_water = spec.id == "prop.dock_planks";
        }
        "prop.ferry_boat" => {
            a.layer = Layer::Floor;
            a.placement.on_water = true;
            a.blocks_movement = false;
        }
        "prop.ferry_rope" => {
            a.rotations = vec![0, 90];
            a.placement.on_water = true;
            a.blocks_movement = false;
        }
        "prop.rug_small" => a.layer = Layer::Floor,
        "prop.rowboat" => a.placement.on_water = true,
        "prop.fence" => a.rotations = vec![0, 90],
        "prop.market_stall" | "prop.cart" | "prop.well" | "prop.signpost" | "prop.haycart" => {
            a.placement.near_road = true;
        }
        "prop.bed" | "prop.chest" | "prop.bench" | "prop.cupboard" | "prop.shelf"
        | "prop.bookshelf" | "prop.hearth" | "prop.cask_rack" | "prop.weapon_rack"
        | "prop.workbench" | "prop.banner" | "prop.oven" | "prop.forge" => {
            a.placement.against_wall = true;
        }
        _ => {}
    }
    if a.placement.near_road || spec.id == "prop.tent" {
        a.placement.clearance_squares = 1;
    }
    let mut r = Relief::new(w * PPSQ, h * PPSQ);
    (spec.paint)(&mut r, hash_str(seed, spec.id), PPSQ as f32);
    (a, r.finish(1.0))
}
