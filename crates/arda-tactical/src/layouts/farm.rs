//! A farm test layout for the farmed ground keys and field boundaries.

use super::Builder;
use crate::catalog::WallRole;
use crate::layout::TacticalLayout;

/// 30 × 20: a ploughed field bounded by a hedge and a drystone wall with a
/// gate, a pasture with a wattle pen, a timber barn on packed earth, farm
/// tracks and an orchard corner.
pub fn farm_field() -> TacticalLayout {
    let mut b = Builder::new("farm_field", 30, 20, "pasture");
    b.elevation_by(|x, y| if x >= 24 && y < 9 { 305 } else { 300 });
    b.ground(2, 2, 16, 12, "farmland");
    b.ground(16, 0, 30, 1, "meadow")
        .ground(0, 12, 17, 20, "meadow");
    // Tracks: north–south beside the field, east–west to the barn.
    b.ground(17, 0, 19, 20, "dirt")
        .ground(19, 12, 30, 13, "dirt");
    b.ground(16, 12, 17, 14, "packed_earth");
    // Hedges on the north and west, drystone on the south and east.
    b.hrun(2, 16, 2, "hedge").vrun(2, 2, 12, "hedge");
    b.hrun(2, 16, 12, "drystone").vrun(16, 2, 12, "drystone");
    b.h(9, 12, WallRole::Gate, "drystone");
    // A wattle sheep pen in the pasture.
    b.hrun(21, 27, 3, "wattle")
        .hrun(21, 27, 8, "wattle")
        .vrun(21, 3, 8, "wattle")
        .vrun(27, 3, 8, "wattle");
    b.v(21, 5, WallRole::Gate, "wattle");
    b.put("prop.trough", 24.0, 6.5, 0)
        .put("prop.hay_bale", 22.5, 3.5, 0)
        .put("prop.hay_bale", 26.5, 4.5, 90)
        .put("prop.bucket", 25.4, 7.4, 0);
    // A timber barn on packed earth.
    b.room(22, 14, 29, 19, "timber", "packed_earth");
    b.v(22, 16, WallRole::Gate, "timber")
        .h(25, 14, WallRole::Window, "timber");
    b.put("prop.hay_bale", 27.5, 14.5, 0)
        .put("prop.hay_bale", 28.5, 14.5, 90)
        .put("prop.hay_bale", 28.5, 15.5, 0)
        .put("prop.ladder", 27.5, 17.0, 0)
        .put("prop.sacks", 23.5, 18.5, 0)
        .put("prop.wheelbarrow", 25.5, 17.5, 0)
        .put("prop.cart", 24.0, 15.6, 90);
    // Along the tracks.
    b.put("prop.haycart", 18.0, 6.0, 0)
        .put("prop.signpost", 19.5, 11.5, 0)
        .put("prop.well", 20.5, 14.5, 0)
        .put("prop.fence", 20.5, 19.5, 0)
        .put("prop.fence", 21.5, 19.5, 0)
        .put("prop.woodpile", 21.4, 18.4, 0)
        .put("prop.milestone", 16.6, 16.5, 0)
        .put("prop.marker_post", 19.4, 3.5, 0);
    // An orchard and wild margins.
    for (x, y) in [
        (2.5, 14.8),
        (6.5, 14.4),
        (10.5, 15.0),
        (4.5, 18.4),
        (8.6, 18.6),
        (12.6, 18.2),
    ] {
        b.put("veg.tree_fruit", x, y, 0);
    }
    for (x, y) in [(14.5, 14.5), (0.6, 12.6), (15.4, 19.2)] {
        b.put("veg.flower_patch", x, y, 0);
    }
    b.put("veg.tree_oak", 28.0, 10.4, 0)
        .put("veg.tall_grass", 0.8, 1.2, 0)
        .put("veg.tall_grass", 1.2, 6.5, 0)
        .put("veg.bush", 16.6, 1.4, 0)
        .put("veg.stones", 12.5, 12.6, 0);
    b.done()
}
