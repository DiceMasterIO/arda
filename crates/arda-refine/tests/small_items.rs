//! The review's small tactical items (3.4-10): placements and their rules
//! agree on the square an item stands in, neighbouring blocks agree on the
//! trails their halos read, and 4–5 ft of water is waded, not swum.
#![allow(clippy::unwrap_used, missing_docs)]

use arda::{Cell, Cover, TerrainKind};
use arda_core::{HeightMm, RainfallMm, TempCentiC};
use arda_refine::context::Ctx;
use arda_refine::terrain::{Phys, Water};
use arda_refine::trails::trails;
use arda_refine::{refine, CellKey, GridSource};

fn land(h_m: i32) -> Cell {
    Cell {
        height: HeightMm::new(h_m * 1000),
        terrain: TerrainKind::Land,
        cover: Cover::Forest,
        forest_density: 180,
        slope_milli_deg: 3_000,
        temperature: TempCentiC::new(900),
        rainfall: RainfallMm::new(900),
        moisture: 120,
        wetness: 60,
        height_above_river_dm: 60,
        ..Cell::default()
    }
}

#[test]
fn every_placement_sits_on_a_sixty_fourth_of_a_square() {
    // The layout snaps placements to 1/64 square; the rules read the same
    // snapped position, so an item and its cover share a square.
    let src = GridSource::new(7, 9, 9, land(120));
    let b = refine(&src, CellKey::new(4, 4)).unwrap();
    assert!(!b.items.is_empty());
    for it in &b.items {
        assert_eq!((it.x * 64.0).fract(), 0.0, "{} at {}", it.asset, it.x);
        assert_eq!((it.y * 64.0).fract(), 0.0, "{} at {}", it.asset, it.y);
    }
}

/// A world with a saddle cell two cells east of block `(5, 5)`: whether it
/// is a pass depends on the cell three east, beyond a two-cell gather.
fn saddle(seed: u64) -> GridSource {
    let mut g = GridSource::new(seed, 12, 12, land(100));
    for y in 0..12 {
        for x in 0..12 {
            let mut c = land(100 + i32::try_from(x + y).unwrap() % 3);
            if x == 7 && y == 5 {
                c = land(150);
            }
            if x == 7 && (y == 4 || y == 6) {
                c = land(190);
            }
            if x == 8 && y == 5 {
                c = land(110);
            }
            if x == 6 && y == 5 {
                c = land(120);
            }
            g.set(CellKey::new(x, y), c);
        }
    }
    g
}

#[test]
fn neighbouring_blocks_read_the_same_trails_in_their_halos() {
    // Review 3.4-10: trails read the four-neighbours of cells two away, a
    // ring the block's gather clamped, so 4 % of halo trail squares differed
    // between neighbours.
    for seed in 0..40 {
        let src = saddle(seed);
        let (a, b) = (CellKey::new(5, 5), CellKey::new(6, 5));
        let (ta, tb) = (
            trails(&Ctx::gather(&src, a).unwrap()),
            trails(&Ctx::gather(&src, b).unwrap()),
        );
        for y in 5 * 64..6 * 64 {
            for x in 6 * 64..7 * 64 {
                assert_eq!(ta.at(x, y), tb.at(x, y), "seed {seed} at ({x}, {y})");
            }
        }
    }
}

#[test]
fn four_and_a_half_feet_of_water_is_waded() {
    // 1.37 m is 4.5 ft: it rounds to 4 ft (wading), as logic/09 states,
    // where the old ceiling made it 5 ft (swimming).
    let p = Phys {
        water: Water::River,
        depth_m: 1.37,
        ..Phys::default()
    };
    assert_eq!(arda_refine::block::depth_ft(&p), 4);
    let shallow = Phys {
        water: Water::Pool,
        depth_m: 0.05,
        ..Phys::default()
    };
    assert_eq!(arda_refine::block::depth_ft(&shallow), 1);
}
