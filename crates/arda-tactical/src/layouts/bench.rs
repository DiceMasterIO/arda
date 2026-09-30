//! A synthetic benchmark map (goal 62 performance target): a 64 × 64
//! block mixing every kind of work the compositor does: many ground keys
//! with noise-shaped borders, a river with shallows and a deep channel,
//! rolling hills with a cliff, walled buildings and a few hundred props
//! and trees.

use super::Builder;
use crate::catalog::WallRole;
use crate::layout::TacticalLayout;
use crate::noise::{fbm, hash2, unit};

const GROUNDS: [&str; 12] = [
    "grass",
    "meadow",
    "pasture",
    "dirt",
    "forest_floor",
    "moss",
    "heath",
    "scrub",
    "gravel",
    "leaf_litter",
    "farmland",
    "sand",
];

/// A `size × size` benchmark layout.
#[must_use]
// Map coordinates are small; the float conversions are exact.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn benchmark(size: u32) -> TacticalLayout {
    let mut b = Builder::new("benchmark", size, size, "grass");
    let n = GROUNDS.len() as f32;
    b.ground_where("grass", |_, _| true);
    for (i, g) in GROUNDS.iter().enumerate() {
        b.ground_where(g, |x, y| {
            let v = fbm(0xBE7C, x as f32 / 9.0, y as f32 / 9.0, 3, None);
            ((v * n * 1.6) as usize) % GROUNDS.len() == i
        });
    }
    // Rolling hills in 5-ft steps with a cliff band.
    b.elevation_by(|x, y| {
        let v = fbm(0x411, x as f32 / 14.0, y as f32 / 14.0, 3, None);
        let cliff = if y > size / 3 { 0 } else { 20 };
        (300 + cliff + 5 * ((v * 8.0) as i16)).min(360)
    });
    // A river meandering north to south.
    let river = |y: u32| {
        (size as f32 * 0.7 + 4.0 * (fbm(0x51, y as f32 / 8.0, 0.0, 2, None) - 0.5) * 4.0) as u32
    };
    b.water_where(3, "mud", |x, y| x + 2 >= river(y) && x <= river(y) + 2);
    b.water_where(7, "gravel", |x, y| x >= river(y) && x <= river(y) + 1);
    // Three walled buildings.
    for (x0, y0, kit, floor) in [
        (6, 40, "stone", "stone_floor"),
        (20, 44, "timber", "planks"),
        (8, 52, "wattle", "packed_earth"),
    ] {
        b.room(x0, y0, x0 + 8, y0 + 6, kit, floor);
        b.h(x0 + 3, y0 + 6, WallRole::Door, kit);
    }
    b.hrun(2, 30, 36, "drystone").hrun(30, 40, 36, "hedge");
    // Trees, bushes, rocks and props.
    let trees = [
        "veg.tree_oak",
        "veg.tree_pine",
        "veg.tree_elm",
        "veg.tree_birch",
        "veg.tree_spruce",
        "veg.tree_willow",
    ];
    let small = [
        "veg.bush",
        "veg.fern",
        "veg.rock_small",
        "veg.tall_grass",
        "veg.flower_patch",
        "veg.stump",
        "veg.heather",
    ];
    let props = [
        "prop.barrel",
        "prop.crate",
        "prop.sacks",
        "prop.table",
        "prop.chair",
        "prop.bench",
    ];
    for i in 0..320i64 {
        let h = hash2(0xB0B, i, 0);
        let (x, y) = (
            unit(h) * size as f32,
            unit(hash2(0xB0B, i, 1)) * size as f32,
        );
        let id = match i % 5 {
            0 => trees[(h % trees.len() as u64) as usize],
            1 | 2 => small[(h % small.len() as u64) as usize],
            _ if y > 40.0 && x < 30.0 => props[(h % props.len() as u64) as usize],
            _ => small[(h % small.len() as u64) as usize],
        };
        b.put(id, x, y, 0);
    }
    b.light(10.5, 43.5, 20).light(24.5, 47.5, 15);
    b.done()
}
