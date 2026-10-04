//! A riverside village street, modelled on
//! `assets/reference/tactical-target/4.jpg`: a river on the west crossed by a
//! plank bridge, a cobbled street running east, a stone warehouse with a
//! doorway to the north, a walled yard and a timber house to the south, a
//! dock with a crane, and scattered props and trees.

use super::Builder;
use crate::catalog::{AssetClass, WallRole};
use crate::layout::TacticalLayout;

/// 30 × 17 squares (150 × 85 ft).
pub fn build() -> TacticalLayout {
    let mut b = Builder::new("riverside", 30, 17, "grass");
    // Worn ground: dirt shoulders and a trodden square before the warehouse.
    b.ground_where("dirt", |x, y| {
        (7..30).contains(&x) && (8..12).contains(&y) || (17..24).contains(&x) && (7..9).contains(&y)
    });
    b.ground_where("dirt", |x, y| (19..23).contains(&x) && y >= 11);
    b.ground(9, 1, 13, 4, "dirt").ground(25, 5, 29, 9, "dirt");
    // The cobbled street and its branch to the warehouse door.
    b.ground(6, 9, 30, 11, "cobbles")
        .ground(18, 7, 22, 9, "cobbles");
    b.ground(7, 11, 9, 13, "mud")
        .ground(26, 11, 29, 12, "mud")
        .ground(6, 12, 8, 17, "gravel");
    b.ground(6, 0, 8, 8, "gravel");

    // The river: shallow margins, a deep channel.
    b.water(0, 0, 6, 17, 2).water(1, 0, 4, 17, 7);
    // A plank bridge on the street's line, a dock and moored boats.
    for x in 0..7 {
        b.put("prop.bridge_deck", x as f32 + 0.5, 10.0, 0);
    }
    // Timber railings along both sides of the deck.
    b.hrun(0, 7, 9, "timber").hrun(0, 7, 11, "timber");
    for (x, y) in [(4.5, 1.5), (5.5, 1.5), (4.5, 2.5), (5.5, 2.5), (5.5, 3.5)] {
        b.put("prop.dock_planks", x, y, 90);
    }
    b.put("prop.crane", 7.1, 3.6, 270)
        .put("prop.rowboat", 2.6, 4.4, 0)
        .put_mirrored("prop.rowboat", 1.8, 13.6, 180);
    // Stone embankment walls along the east bank, ending at the bridge.
    b.vrun(6, 4, 8, "stone").vrun(6, 12, 17, "stone");
    for y in [0.6, 5.3, 7.4, 12.7, 14.2, 15.8] {
        b.put("veg.reeds", 5.6, y, 0);
    }
    b.put("veg.reeds", 0.4, 8.2, 90)
        .put("veg.reeds", 0.5, 15.1, 180);

    // Stone warehouse with a doorway to the street.
    b.room(15, 1, 24, 7, "stone", "stone_floor");
    b.h(19, 7, WallRole::Door, "stone")
        .h(16, 7, WallRole::Window, "stone")
        .h(22, 7, WallRole::Window, "stone");
    b.v(24, 3, WallRole::Window, "stone");
    for x in [15.5, 16.5, 17.5, 20.5, 21.5, 22.5] {
        b.put("prop.crate", x, 1.5, 0);
    }
    b.put("prop.sacks", 23.5, 1.5, 0)
        .put("prop.sacks", 23.5, 2.5, 90)
        .put("prop.barrel", 15.5, 5.5, 0);
    b.put("prop.barrel", 15.5, 6.5, 0)
        .put("prop.barrel", 16.5, 6.5, 0)
        .put("prop.table", 19.0, 3.5, 0);
    b.put("prop.bench", 19.0, 4.5, 0)
        .put("prop.chest", 23.5, 5.5, 90)
        .put("prop.sacks", 23.5, 6.5, 0);
    b.put("prop.brazier", 17.5, 7.6, 0)
        .put("prop.brazier", 21.5, 7.6, 0);
    b.put("prop.barrel", 14.5, 1.5, 0)
        .put("prop.crate", 14.5, 2.5, 90)
        .put("prop.sacks", 14.4, 3.6, 180);

    // Fenced plot and woodshed east of the warehouse.
    for x in [25.5, 26.5, 27.5] {
        b.put("prop.fence", x, 4.5, 0);
    }
    b.put("prop.woodpile", 28.5, 7.5, 0)
        .put("prop.cart", 26.5, 7.0, 90)
        .put("prop.crate", 28.5, 6.5, 0);

    // Walled yard south of the street, entered through a gate.
    b.ground(10, 12, 15, 16, "dirt");
    b.hrun(10, 15, 12, "stone")
        .hrun(10, 15, 16, "stone")
        .vrun(10, 12, 16, "stone")
        .vrun(15, 12, 16, "stone");
    b.h(12, 12, WallRole::Gate, "stone");
    b.put("prop.cart", 11.5, 14.0, 0)
        .put("prop.crate", 13.5, 12.5, 0)
        .put("prop.crate", 14.5, 12.5, 0);
    b.put("prop.sacks", 13.5, 13.5, 0)
        .put("prop.barrel", 14.5, 15.5, 0)
        .put("prop.crate", 13.5, 15.5, 90);
    b.put("prop.fence", 15.5, 15.5, 0)
        .put("prop.fence", 16.5, 15.5, 0)
        .put("prop.fence", 17.5, 15.5, 0);

    // Timber house with two rooms.
    b.room(24, 12, 29, 16, "timber", "planks");
    b.vrun(27, 12, 16, "timber");
    b.v(24, 14, WallRole::Door, "timber")
        .v(27, 14, WallRole::Door, "timber");
    b.h(25, 12, WallRole::Window, "timber")
        .h(28, 16, WallRole::Window, "timber");
    b.put("prop.table", 25.5, 14.5, 0)
        .put("prop.barrel", 24.5, 12.5, 0)
        .put("prop.bed", 28.5, 13.0, 0);
    b.put("prop.chest", 27.5, 15.5, 0)
        .put("prop.bench", 25.5, 15.5, 0);
    b.put("prop.tent", 22.0, 14.0, 90)
        .put("prop.woodpile", 23.5, 12.5, 0);
    b.light(24.5, 14.5, 10);

    // Market bits and a well by the street.
    b.put("prop.market_stall", 9.0, 6.5, 0)
        .put("prop.well", 12.5, 7.5, 0)
        .put("prop.sacks", 10.5, 7.5, 0);
    b.put("prop.tent", 10.0, 2.5, 0);

    // Vegetation.
    b.put("veg.tree_oak", 28.2, 1.6, 0)
        .put("veg.tree_elm", 18.8, 14.4, 90)
        .put("veg.tree_fruit", 16.2, 12.8, 0);
    b.put("veg.tree_birch", 8.4, 14.8, 0)
        .put("veg.tree_oak", 1.4, 16.4, 180);
    for (x, y) in [
        (8.6, 0.6),
        (13.6, 5.4),
        (24.6, 9.4),
        (14.4, 8.4),
        (29.4, 11.4),
        (9.4, 11.6),
        (16.6, 10.4),
    ] {
        b.query(AssetClass::Vegetation, "bush", x, y);
    }
    b.put("veg.boulder", 7.5, 12.6, 0)
        .put("veg.stones", 13.5, 0.6, 0)
        .put("veg.stones", 29.3, 16.3, 90);
    b.done()
}
