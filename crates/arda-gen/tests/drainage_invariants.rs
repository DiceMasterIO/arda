//! Global drainage invariants.
//!
//! Every sim test that existed before this suite asserted a *local*
//! property — round trips, per-cell bounds, determinism. A stage in which
//! 93% of the tile's catchment terminated in an interior pit satisfied all
//! of them. These assert properties no single cell can satisfy alone.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use arda_core::{
    AreaCells, AreaCoord, AreaObjects, CellCoord, GenerateConfig, Terminus, TerrainKind, AREA_CELLS,
};
use arda_gen::area::water::WaterGrid;
use arda_gen::area::{compose, erosion, fill, relief};
use arda_gen::continent::bundles::bundle_for;
use arda_gen::continent::generate_continent;
use std::sync::OnceLock;

const N: i32 = AREA_CELLS as i32;

fn cc(x: i32, y: i32) -> CellCoord {
    CellCoord::new(u16::try_from(x).unwrap(), u16::try_from(y).unwrap()).unwrap()
}

/// Tiles carrying enough land to be worth asserting against.
const LAND_TILES: [(i32, i32); 3] = [(0, 1), (0, 2), (1, 3)];

/// One tile, with the routing kept so invariants can walk the flow tree
/// rather than infer from stored fields.
struct Tile {
    coord: AreaCoord,
    cells: AreaCells,
    objects: AreaObjects,
    water: WaterGrid,
}

/// Built once and shared: erosion is the expensive part and every invariant
/// wants the same tiles.
fn tiles() -> &'static Vec<Tile> {
    static TILES: OnceLock<Vec<Tile>> = OnceLock::new();
    TILES.get_or_init(|| {
        let c = generate_continent(42, GenerateConfig::MICRO);
        LAND_TILES
            .iter()
            .map(|&(x, y)| {
                let coord = AreaCoord::new(x, y);
                let b = bundle_for(42, &c, coord);
                let r = relief(42, &c, &b);
                let mut heights: Vec<i32> = (0..(N * N)).map(|i| r.get(cc(i % N, i / N))).collect();
                // Mirror generate_area: uplift follows the smooth regional
                // surface, not the noise-refined relief.
                let uplift: Vec<i32> = (0..(N * N))
                    .map(|i| {
                        let (ax, ay) = arda_gen::continent::bundles::abs_cell(
                            coord,
                            u16::try_from(i % N).unwrap(),
                            u16::try_from(i / N).unwrap(),
                        );
                        arda_gen::continent::bundles::coarse_height(&c, ax, ay)
                    })
                    .collect();
                erosion::erode(&mut heights, &uplift, &b);
                let filled = fill::fill(&heights, &b);
                let water = arda_gen::area::water(&filled, &b);
                let (cells, objects) = compose(&heights, &filled, &water);
                Tile {
                    coord,
                    cells,
                    objects,
                    water,
                }
            })
            .collect()
    })
}

#[test]
fn a_every_land_cell_drains_to_water_or_off_tile() {
    // The invariant whose absence is the whole reason this suite exists.
    // Walk each land cell's flow path and require it to actually arrive
    // somewhere: the sea, a lake, or off the tile. Before this feature,
    // 93% of catchment terminated in an interior pit and every test passed.
    for t in tiles() {
        let mut stranded = 0u32;
        for y in 1..N - 1 {
            for x in 1..N - 1 {
                let at = cc(x, y);
                if t.cells.get(at).terrain != TerrainKind::Land {
                    continue;
                }
                let mut cursor = at;
                let mut hops = 0u32;
                let arrived = loop {
                    if t.water.is_outlet(cursor) {
                        break true;
                    }
                    match t.water.downstream_of(cursor) {
                        Some(next) => {
                            cursor = next;
                            hops += 1;
                            if t.cells.get(cursor).terrain != TerrainKind::Land {
                                break true; // sea or lake
                            }
                            if hops > (N * N) as u32 {
                                break false; // cycle guard
                            }
                        }
                        None => break false, // a sink
                    }
                };
                if !arrived {
                    stranded += 1;
                }
            }
        }
        assert_eq!(
            stranded, 0,
            "tile {:?}: {stranded} land cells drain nowhere",
            t.coord
        );
    }
}

#[test]
fn b_lakes_are_emitted_and_have_substance() {
    // Guards against the filter silently removing every basin: a stage that
    // emits no lakes would leave `Lake` producer-less again.
    let mut total_lakes = 0;
    for t in tiles() {
        let (cells, objects) = (&t.cells, &t.objects);
        total_lakes += objects.lakes.len();
        for l in &objects.lakes {
            assert!(
                l.cells.len() >= arda_gen::area::LAKE_MIN_CELLS,
                "lake {} is under the size threshold",
                l.id
            );
            assert!(
                l.depth_mm >= arda_gen::area::LAKE_MIN_DEPTH_MM,
                "lake {} is under the depth threshold",
                l.id
            );
            for cell in &l.cells {
                assert_eq!(
                    cells.get(*cell).terrain,
                    TerrainKind::Lake,
                    "lake {} covers a cell not marked as lake",
                    l.id
                );
            }
        }
    }
    assert!(total_lakes > 0, "no lakes were emitted across three tiles");
}

#[test]
fn c_non_land_cells_carry_no_flow() {
    for t in tiles() {
        let cells = &t.cells;
        for y in 0..N {
            for x in 0..N {
                let cell = cells.get(cc(x, y));
                if cell.terrain == TerrainKind::Land {
                    continue;
                }
                assert_eq!(cell.drainage_area_cells, 0, "flow stored at {x},{y}");
                assert_eq!(cell.discharge.raw(), 0);
                assert_eq!(cell.watercourse_order, 0);
            }
        }
    }
}

#[test]
fn d_segments_partition_the_channel_network() {
    for t in tiles() {
        let (cells, objects) = (&t.cells, &t.objects);
        let mut covered = std::collections::HashSet::new();
        for s in &objects.rivers {
            for cell in &s.course {
                assert!(covered.insert(*cell), "cell {cell:?} is in two segments");
            }
        }
        for y in 0..N {
            for x in 0..N {
                let at = cc(x, y);
                if cells.get(at).watercourse_order > 0 {
                    assert!(
                        covered.contains(&at),
                        "channel cell {x},{y} is in no segment"
                    );
                }
            }
        }
    }
}

#[test]
fn e_strahler_never_decreases_downstream() {
    for t in tiles() {
        let objects = &t.objects;
        let by_id: std::collections::HashMap<u16, u8> =
            objects.rivers.iter().map(|s| (s.id, s.order)).collect();
        for s in &objects.rivers {
            if let Some(f) = s.feeds {
                let downstream = by_id.get(&f).copied().unwrap_or(s.order);
                assert!(
                    downstream >= s.order,
                    "segment {} (order {}) feeds {} (order {})",
                    s.id,
                    s.order,
                    f,
                    downstream
                );
            }
        }
    }
}

#[test]
fn f_the_same_seed_yields_the_same_lakes_and_reaches() {
    let c = generate_continent(42, GenerateConfig::MICRO);
    for t in tiles() {
        let b = bundle_for(42, &c, t.coord);
        let again = arda_gen::area::generate_area(42, &c, &b);
        assert_eq!(again.1.lakes, t.objects.lakes, "lakes differ on rerun");
        assert_eq!(again.1.rivers, t.objects.rivers, "reaches differ on rerun");
    }
}

#[test]
fn g_segments_terminate_consistently_with_their_link() {
    // Artifact: "every segment knows which segment it feeds and how it ends".
    for t in tiles() {
        for s in &t.objects.rivers {
            match s.ends {
                Terminus::Junction => assert!(
                    s.feeds.is_some(),
                    "segment {} ends at a junction but feeds nothing",
                    s.id
                ),
                _ => assert!(
                    s.feeds.is_none(),
                    "segment {} ends at {:?} yet feeds another",
                    s.id,
                    s.ends
                ),
            }
            assert!(!s.course.is_empty(), "segment {} has no course", s.id);
        }
    }
}

#[test]
fn h_tile_seams_are_no_rougher_than_the_interior() {
    // Erosion holds the pinned rim fixed while the interior incises, which
    // risks a raised lip around every tile.
    //
    // The comparison must be like for like: land-to-land steps only. An
    // earlier version of this test compared the maximum seam step against
    // the interior *median* and failed on a legitimate 60 m drop from a
    // cliff into the sea, which is terrain, not an artifact.
    let cells = &tiles()[0].cells;
    let land_step = |ax: i32, ay: i32, bx: i32, by: i32| -> Option<i64> {
        let a = cells.get(cc(ax, ay));
        let b = cells.get(cc(bx, by));
        (a.terrain == TerrainKind::Land && b.terrain == TerrainKind::Land)
            .then(|| i64::from((a.height.raw() - b.height.raw()).abs()))
    };
    let p90 = |mut v: Vec<i64>| -> i64 {
        if v.is_empty() {
            return 0;
        }
        v.sort_unstable();
        v[v.len() * 9 / 10]
    };

    // The seam itself, and a control the same shape eight cells inward.
    let seam: Vec<i64> = (0..N)
        .filter_map(|y| land_step(N - 1, y, N - 2, y))
        .collect();
    let control: Vec<i64> = (0..N)
        .filter_map(|y| land_step(N - 9, y, N - 10, y))
        .collect();
    assert!(
        seam.len() > N as usize / 4,
        "not enough land on the seam to judge: {} rows",
        seam.len()
    );
    let (s90, c90) = (p90(seam), p90(control));
    assert!(
        s90 <= c90.max(1) * 3,
        "seam roughness p90 {s90} mm is far above the interior control {c90} mm"
    );
}
