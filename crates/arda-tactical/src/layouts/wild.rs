//! Wilderness test layouts for the natural ground keys and vegetation: a
//! forest glade with a stream, a mountain scree slope under a cliff, and a
//! reedy marsh. Elevations are absolute feet in 5-ft steps (vocabulary I20).

use super::Builder;
use crate::layout::TacticalLayout;
use crate::noise::{hash2, unit};

/// A deterministic scatter value in `[0, 1)` for square `(x, y)`.
fn h(salt: u64, x: u32, y: u32) -> f32 {
    unit(hash2(salt, i64::from(x), i64::from(y)))
}

/// 28 × 20: a forest glade crossed by a stream with a deep pool.
pub fn forest_glade() -> TacticalLayout {
    const STREAM: [u32; 20] = [
        16, 16, 15, 15, 14, 13, 12, 12, 11, 11, 12, 13, 14, 15, 15, 14, 13, 12, 12, 11,
    ];
    let mut b = Builder::new("forest_glade", 28, 20, "forest_floor");
    let near = |x: u32, y: u32, r: u32| {
        let c = STREAM[y as usize];
        x + r >= c && x <= c + 1 + r
    };
    b.ground_where("leaf_litter", |x, y| {
        h(1, x / 3, y / 3) > 0.55 && !near(x, y, 2)
    });
    b.ground_where("moss", |x, y| near(x, y, 2) && h(2, x, y) > 0.35);
    b.ground_where("mud", |x, y| near(x, y, 1) && h(3, x, y) > 0.5);
    b.ground_where("gravel", |x, y| near(x, y, 1) && h(3, x, y) <= 0.5);
    // The glade: an open meadow ringed by grass.
    let glade = |x: u32, y: u32, r: f32| {
        let (dx, dy) = ((x as f32 - 5.5) / 4.6, (y as f32 - 9.5) / 4.0);
        dx * dx + dy * dy < r
    };
    b.ground_where("grass", |x, y| glade(x, y, 1.0));
    b.ground_where("meadow", |x, y| glade(x, y, 0.55));
    b.elevation_by(|x, y| {
        let base = if x >= 25 && y < 8 {
            410
        } else if x >= 20 {
            405
        } else {
            400
        };
        if near(x, y, 0) {
            395
        } else {
            base
        }
    });
    b.water_where(2, "gravel", |x, y| near(x, y, 0));
    b.water_where(6, "mud", |x, y| {
        (10..14).contains(&x) && (9..12).contains(&y)
    });
    b.put("veg.fallen_log", 16.0, 3.5, 0)
        .put("veg.lily_pads", 11.5, 10.5, 0)
        .put("veg.lily_pads", 12.6, 9.6, 90);
    for (x, y) in [
        (13.4, 6.2),
        (10.3, 13.0),
        (17.6, 1.2),
        (15.2, 15.5),
        (10.6, 18.4),
    ] {
        b.put("veg.cattail", x, y, 0);
    }
    for (id, x, y) in [
        ("veg.tree_oak", 2.0, 2.2),
        ("veg.tree_elm", 7.5, 1.6),
        ("veg.tree_birch", 11.2, 2.4),
        ("veg.tree_pine", 21.5, 2.0),
        ("veg.tree_spruce", 25.8, 4.6),
        ("veg.tree_oak", 24.0, 10.5),
        ("veg.tree_pine", 20.6, 15.8),
        ("veg.tree_elm", 25.4, 17.2),
        ("veg.tree_birch", 16.8, 12.4),
        ("veg.tree_oak", 2.4, 17.2),
        ("veg.tree_spruce", 7.8, 17.8),
        ("veg.tree_birch", 0.8, 9.6),
    ] {
        b.put(id, x, y, 0);
    }
    for (x, y) in [
        (4.5, 4.8),
        (9.2, 5.1),
        (19.4, 6.5),
        (22.6, 13.2),
        (18.5, 18.4),
        (1.5, 13.4),
        (26.4, 1.4),
    ] {
        b.put("veg.fern", x, y, 0);
    }
    b.put("veg.mushroom_ring", 5.5, 9.5, 0)
        .put("veg.flower_patch", 3.4, 8.0, 0)
        .put("veg.flower_patch", 7.4, 11.6, 90)
        .put("veg.tall_grass", 8.6, 8.2, 0)
        .put("veg.tall_grass", 2.8, 11.4, 0)
        .put("veg.stump", 19.5, 9.4, 0)
        .put("veg.rock_small", 14.6, 8.6, 0)
        .put("veg.boulder", 9.4, 14.2, 0)
        .put("veg.bush", 22.4, 7.8, 0)
        .put("veg.bush_flowering", 4.6, 14.6, 0);
    b.done()
}

/// 28 × 20: a scree fan below a 20-ft cliff, with a snowy rock plateau
/// above and heath below, crossed by a switchback path.
pub fn mountain_scree() -> TacticalLayout {
    const CLIFF: [u32; 28] = [
        6, 6, 6, 5, 5, 5, 6, 6, 6, 7, 7, 7, 6, 6, 6, 6, 5, 5, 5, 6, 6, 6, 6, 7, 7, 7, 6, 6,
    ];
    let mut b = Builder::new("mountain_scree", 28, 20, "heath");
    let top = |x: u32| CLIFF[x as usize];
    // The gully east of x = 22 breaks the cliff into a steep scree ramp.
    let gully = |x: u32| (22..25).contains(&x);
    let slope = |c: u32, y: u32| 1240 - 5 * i16::try_from(y.saturating_sub(c + 1) / 2).unwrap_or(0);
    b.elevation_by(|x, y| {
        let c = top(x);
        if gully(x) {
            let ramp = 1260 - 5 * i16::try_from(y.saturating_sub(2) * 3 / 5).unwrap_or(0);
            ramp.min(slope(6, y.max(7)))
        } else if y < c {
            1260
        } else if y == c {
            1250
        } else {
            slope(c, y)
        }
    });
    b.ground_where("rock", |x, y| y < top(x));
    b.ground_where("snow", |x, y| {
        y < top(x) && (x < 9 || y < 2) && h(4, x / 2, y / 2) > 0.3
    });
    b.ground_where("ice", |x, y| (3..6).contains(&x) && (1..3).contains(&y));
    b.ground_where("cliff", |x, y| y == top(x) && !gully(x));
    // The scree fan widens downhill from the cliff foot.
    let fan = |x: u32, y: u32| {
        let c = top(x);
        let spread = (y.saturating_sub(c)) / 2 + 2;
        y > c && (x + spread >= 14 && x <= 14 + spread) && y < 17
    };
    b.ground_where("rock", |x, y| y == top(x) + 1 && !gully(x));
    b.ground_where("scree", |x, y| fan(x, y) || gully(x) && y >= 3);
    b.ground_where("scrub", |x, y| y >= 16 && h(5, x, y) > 0.5);
    b.ground_where("grass", |x, y| y >= 18 && h(6, x, y) > 0.4);
    // A switchback path from the south-west up the gully.
    b.ground_where("gravel", |x, y| {
        (y == 17 && (2..9).contains(&x))
            || (x == 8 && (12..18).contains(&y))
            || (y == 12 && (8..22).contains(&x))
            || (x == 21 && (4..13).contains(&y))
            || (y == 4 && (21..26).contains(&x))
    });
    b.put("veg.rock_large", 11.0, 9.0, 0)
        .put("veg.rock_large", 19.6, 8.4, 90)
        .put("veg.scree_patch", 14.0, 10.5, 0)
        .put("veg.scree_patch", 16.5, 14.0, 90)
        .put("veg.scree_patch", 23.5, 7.0, 0);
    for (x, y) in [
        (12.6, 7.8),
        (17.2, 11.3),
        (15.4, 8.4),
        (6.5, 9.2),
        (25.5, 12.5),
        (3.4, 14.2),
        (13.2, 15.6),
    ] {
        b.put("veg.rock_small", x, y, 0);
    }
    for (x, y) in [(2.5, 10.4), (5.0, 12.5), (26.3, 16.4), (21.6, 18.4)] {
        b.put("veg.heather", x, y, 0);
    }
    b.put("veg.tree_pine", 2.2, 17.0, 0)
        .put("veg.tree_spruce", 4.6, 18.6, 0)
        .put("veg.tree_pine", 25.2, 18.2, 0)
        .put("veg.tree_dead", 17.8, 2.2, 0)
        .put("veg.tree_spruce", 26.4, 1.6, 0)
        .put("veg.boulder", 9.5, 2.6, 0)
        .put("veg.stones", 13.4, 3.5, 0)
        .put("veg.tall_grass", 11.5, 18.5, 0);
    b.done()
}

/// 28 × 20: a marsh of reed beds and pools with a deep channel crossed by
/// a boardwalk.
pub fn marsh() -> TacticalLayout {
    const CHANNEL: [u32; 28] = [
        9, 9, 10, 10, 11, 11, 11, 10, 10, 9, 9, 9, 10, 11, 11, 12, 12, 12, 11, 11, 10, 10, 10, 11,
        12, 12, 13, 13,
    ];
    let mut b = Builder::new("marsh", 28, 20, "marsh");
    let chan = |x: u32, y: u32| y == CHANNEL[x as usize] || y == CHANNEL[x as usize] + 1;
    b.ground_where("heath", |_, y| y < 2);
    b.ground_where("moss", |x, y| y == 2 || h(7, x, y) > 0.86);
    b.ground_where("reed_bed", |x, y| {
        let c = CHANNEL[x as usize];
        (y + 2 == c || y == c + 3) && h(8, x / 2, y) > 0.3
    });
    b.elevation_by(|_, y| if y < 2 { 105 } else { 100 });
    b.water_where(6, "mud", chan);
    // Shallow pools in the southern marsh.
    let pool = |x: u32, y: u32, cx: f32, cy: f32, r: f32| {
        let (dx, dy) = (x as f32 - cx, (y as f32 - cy) * 1.3);
        dx * dx + dy * dy < r * r
    };
    b.water_where(2, "mud", |x, y| {
        pool(x, y, 5.0, 16.0, 2.6) || pool(x, y, 20.0, 17.0, 3.2) || pool(x, y, 22.0, 5.0, 2.2)
    });
    b.water_where(1, "reed_bed", |x, y| pool(x, y, 13.0, 16.5, 1.6));
    for y in 7..15 {
        b.put("prop.dock_planks", 14.5, y as f32 + 0.5, 90);
    }
    // A rope ferry across the deep channel further east.
    for y in 8..14 {
        b.put("prop.ferry_rope", 22.5, y as f32 + 0.5, 90);
    }
    b.put("prop.ferry_boat", 22.5, 11.0, 0);
    for (i, (x, y)) in [
        (4.2, 8.1),
        (8.4, 13.2),
        (17.8, 8.6),
        (23.5, 10.4),
        (2.6, 12.6),
        (19.2, 14.6),
        (25.6, 7.4),
        (11.3, 7.2),
    ]
    .into_iter()
    .enumerate()
    {
        b.put(
            if i % 2 == 0 {
                "veg.reeds"
            } else {
                "veg.cattail"
            },
            x,
            y,
            0,
        );
    }
    for (x, y) in [
        (5.4, 16.2),
        (20.6, 17.4),
        (19.0, 16.4),
        (22.3, 4.8),
        (6.2, 11.0),
    ] {
        b.put("veg.lily_pads", x, y, 0);
    }
    b.put("veg.tree_willow", 3.0, 3.8, 0)
        .put("veg.tree_willow", 25.0, 15.0, 0)
        .put("veg.tree_dead", 9.0, 18.2, 0)
        .put("veg.tree_dead", 17.0, 3.6, 0)
        .put("veg.tree_birch", 26.2, 1.6, 0)
        .put("veg.fallen_log", 8.0, 5.2, 90)
        .put("veg.tall_grass", 12.0, 3.4, 0)
        .put("veg.tall_grass", 1.4, 18.4, 0)
        .put("veg.heather", 20.5, 0.8, 0)
        .put("veg.bush", 6.5, 1.0, 0)
        .put("veg.stump", 15.8, 18.6, 0);
    b.done()
}
