//! Building-focused test layouts.

use super::Builder;
use crate::catalog::{AssetClass, WallRole};
use crate::layout::TacticalLayout;

/// A stone warehouse with a doorway, a timber-partitioned office and a yard.
pub fn stone_warehouse() -> TacticalLayout {
    let mut b = Builder::new("stone_warehouse", 18, 13, "dirt");
    b.ground_where("grass", |x, y| {
        x < 2 || y < 1 || x > 15 || (y > 11 && !(7..10).contains(&x))
    });
    b.ground(7, 10, 10, 13, "cobbles")
        .ground(15, 9, 18, 13, "gravel");
    b.room(2, 1, 15, 10, "stone", "stone_floor");
    b.ground(10, 1, 15, 6, "planks");
    // Office partition: a timber wall making tees against the stone shell
    // and an interior corner, with its own door.
    b.vrun(10, 1, 6, "timber").hrun(10, 15, 6, "timber");
    b.v(10, 3, WallRole::Door, "timber");
    b.h(8, 10, WallRole::Door, "stone")
        .h(4, 10, WallRole::Window, "stone")
        .h(12, 10, WallRole::Window, "stone");
    b.v(2, 4, WallRole::Window, "stone")
        .v(15, 8, WallRole::Door, "stone");
    // A ruined wall stub outside: end caps.
    b.hrun(15, 18, 12, "stone");
    // Goods.
    for x in [2.5, 3.5, 4.5, 5.5] {
        b.put("prop.crate", x, 1.5, 0).put("prop.sacks", x, 2.5, 90);
    }
    for y in [5.5, 6.5, 7.5, 8.5] {
        b.put("prop.barrel", 2.5, y, 0);
    }
    b.put("prop.crate", 6.5, 5.0, 0)
        .put("prop.crate", 7.5, 5.0, 90)
        .put("prop.crate", 7.0, 6.0, 0);
    b.put("prop.cart", 12.5, 8.0, 90)
        .put("prop.sacks", 9.5, 8.5, 0)
        .put("prop.barrel", 14.5, 6.5, 0);
    // The office.
    b.put("prop.table", 12.5, 3.0, 0)
        .put("prop.bench", 12.5, 4.0, 0)
        .put("prop.chest", 14.5, 1.5, 0);
    b.put("prop.bed", 14.5, 4.0, 0)
        .put("prop.brazier", 11.0, 1.6, 0);
    // Outside.
    b.put("prop.brazier", 6.5, 10.6, 0)
        .put("prop.brazier", 9.5, 10.6, 0)
        .put("prop.cart", 12.0, 12.0, 90);
    b.put("veg.tree_oak", 0.8, 11.6, 0)
        .put("veg.bush", 16.5, 1.5, 0)
        .put("veg.boulder", 16.6, 3.6, 0);
    b.put("prop.woodpile", 16.5, 6.5, 0)
        .put("prop.woodpile", 16.5, 7.5, 90);
    b.done()
}

/// A two-room timber house with a garden, a pond and a raised knoll.
pub fn timber_house() -> TacticalLayout {
    let mut b = Builder::new("timber_house", 16, 13, "grass");
    b.ground_where("dirt", |x, y| {
        (6..9).contains(&x) && y >= 9 || (3..14).contains(&x) && y == 1
    });
    b.ground(9, 9, 15, 12, "mud")
        .ground(11, 1, 16, 4, "gravel")
        .ground(0, 0, 3, 13, "sand");
    b.elevation(12, 0, 16, 4, 5);
    b.room(3, 2, 13, 8, "timber", "planks");
    b.vrun(8, 2, 8, "timber").hrun(8, 13, 5, "timber");
    b.v(8, 4, WallRole::Door, "timber")
        .h(10, 5, WallRole::Door, "timber")
        .h(7, 8, WallRole::Door, "timber");
    b.h(4, 2, WallRole::Window, "timber")
        .h(10, 2, WallRole::Window, "timber")
        .v(3, 5, WallRole::Window, "timber");
    b.v(13, 3, WallRole::Window, "timber")
        .h(11, 8, WallRole::Window, "timber");
    // Furniture.
    b.put("prop.table", 5.0, 4.5, 0)
        .put("prop.bench", 5.0, 5.5, 0)
        .put("prop.bench", 5.0, 3.5, 0);
    b.put("prop.barrel", 3.5, 7.5, 0)
        .put("prop.sacks", 4.5, 7.5, 0)
        .put("prop.chest", 7.5, 2.5, 0);
    b.put("prop.bed", 12.5, 3.0, 0)
        .put("prop.bed", 11.5, 3.0, 0)
        .put("prop.chest", 9.5, 2.5, 90);
    b.put("prop.table", 10.0, 7.5, 0)
        .put("prop.woodpile", 12.5, 7.5, 0);
    // Garden: fence with a gap, a well, a cart and trees.
    for x in 0..6 {
        if x != 3 {
            b.put("prop.fence", x as f32 + 9.5, 12.5, 0);
        }
    }
    b.put("prop.well", 5.5, 10.5, 0)
        .put("prop.cart", 1.5, 10.0, 0)
        .put("prop.sacks", 2.5, 11.5, 0);
    b.put("veg.tree_fruit", 1.5, 1.5, 0)
        .put("veg.tree_birch", 15.0, 9.0, 0)
        .put("veg.tree_elm", 14.5, 5.8, 90);
    // Pond with reeds.
    b.water(9, 9, 13, 11, 2).water(10, 9, 12, 10, 5);
    b.put("veg.reeds", 9.3, 11.2, 0)
        .put("veg.reeds", 12.8, 8.9, 90)
        .put("veg.reeds", 13.2, 10.4, 180);
    b.put("prop.rowboat", 11.0, 10.0, 90);
    // The knoll.
    b.put("veg.boulder", 13.5, 1.5, 0)
        .put("veg.stones", 15.4, 0.6, 0)
        .put("veg.bush_flowering", 14.5, 2.5, 0);
    for (x, y) in [(0.6, 6.4), (1.4, 4.6), (15.4, 11.6), (4.4, 12.4)] {
        b.query(AssetClass::Vegetation, "bush", x, y);
    }
    b.done()
}

/// Every junction type in both kits, for checking wall assembly by eye.
pub fn wall_junctions() -> TacticalLayout {
    let mut b = Builder::new("wall_junctions", 16, 9, "cobbles");
    b.ground(8, 0, 16, 9, "dirt");
    for (x0, kit) in [(0u32, "stone"), (8, "timber")] {
        // A cross, with a window and a door on its arms.
        b.hrun(x0 + 1, x0 + 5, 2, kit).vrun(x0 + 3, 0, 4, kit);
        b.h(x0 + 1, 2, WallRole::Window, kit)
            .v(x0 + 3, 3, WallRole::Door, kit);
        // A room with a tee partition, a door and a gate.
        b.room(x0 + 1, 5, x0 + 7, 8, kit, "stone_floor")
            .vrun(x0 + 4, 5, 8, kit);
        b.h(x0 + 2, 8, WallRole::Gate, kit)
            .v(x0 + 4, 6, WallRole::Door, kit);
        // A free-standing stub with two end caps.
        b.hrun(x0 + 6, x0 + 8, 1, kit);
    }
    b.done()
}
