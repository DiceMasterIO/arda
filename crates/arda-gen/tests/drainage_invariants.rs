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
use arda_gen::continent::build_continent;
use arda_gen::continent::bundles::bundle_for;
use std::sync::OnceLock;

const N: i32 = AREA_CELLS as i32;

fn cc(x: i32, y: i32) -> CellCoord {
    CellCoord::new(u16::try_from(x).unwrap(), u16::try_from(y).unwrap()).unwrap()
}

/// Fallible sibling of [`cc`]: `None` for a coordinate that falls off the
/// tile instead of panicking, for walking a lake cell's 8-neighbours where
/// some may be off-tile. Mirrors the private `coord` helper `arda-gen`
/// keeps in `fill`, `water`, and `area::mod` itself — duplicated here
/// because this suite, as an external integration test, cannot see it.
fn opt_cc(x: i32, y: i32) -> Option<CellCoord> {
    CellCoord::new(u16::try_from(x).ok()?, u16::try_from(y).ok()?)
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
        let c = build_continent(42, GenerateConfig::MICRO, 0);
        LAND_TILES
            .iter()
            .map(|&(x, y)| {
                let coord = AreaCoord::new(x, y);
                let b = bundle_for(42, &c, coord);
                let r = relief(42, &c.grid, &b);
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
                        arda_gen::continent::bundles::coarse_height(&c.grid, ax, ay)
                    })
                    .collect();
                erosion::erode(&mut heights, &uplift, &b);
                let filled = fill::fill(&heights, &b);
                let rain = arda_gen::area::area_rainfall(&b);
                let water = arda_gen::area::water(&filled, &b, &rain);
                let (cells, objects) = compose(&heights, &filled, &water, &rain, &b);
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
    let c = build_continent(42, GenerateConfig::MICRO, 0);
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

/// Checks every surviving lake in `objects` against the outlet contract
/// `arda_gen::area`'s private `recompute_outlet` promises: `Some` only
/// ever names a cell outside the lake itself, 8-adjacent to it, and not
/// submerged by any surviving lake — this one or another; `None` only
/// when no such free neighbour exists anywhere around the lake at all.
///
/// The free-neighbour condition is derived independently here, over
/// `objects.lakes` alone, rather than by calling `recompute_outlet`
/// itself (which this external test cannot see anyway, being private) —
/// so a regression that quietly turns a real outlet into `None` has no
/// shared code to hide behind.
///
/// Returns how many lakes were checked, so callers can guard against a
/// vacuous sweep.
fn assert_lake_outlets_are_sound(objects: &AreaObjects) -> u32 {
    // Every surviving lake's cells, unioned tile-wide: an outlet must sit
    // outside all of them, not just its own lake's.
    let submerged: std::collections::HashSet<CellCoord> = objects
        .lakes
        .iter()
        .flat_map(|l| l.cells.iter().copied())
        .collect();
    const OFFSETS: [(i32, i32); 8] = [
        (-1, -1),
        (-1, 0),
        (-1, 1),
        (0, -1),
        (0, 1),
        (1, -1),
        (1, 0),
        (1, 1),
    ];

    for l in &objects.lakes {
        if let Some(out) = l.outlet {
            assert!(
                !l.cells.contains(&out),
                "lake {} outlet {out:?} is one of its own cells",
                l.id
            );
            assert!(
                !submerged.contains(&out),
                "lake {} outlet {out:?} is submerged by a surviving lake",
                l.id
            );
            assert!(
                l.cells.iter().any(|c| {
                    (i32::from(c.x()) - i32::from(out.x())).abs() <= 1
                        && (i32::from(c.y()) - i32::from(out.y())).abs() <= 1
                }),
                "lake {} outlet {out:?} is not 8-adjacent to any of its cells",
                l.id
            );
        }

        // Independently derive whether a free (in-tile, not submerged by
        // any surviving lake) 8-neighbour exists anywhere around the
        // lake's own cells — the exact condition `recompute_outlet`'s own
        // doc names for when it must return `None`.
        let has_free_neighbour = l.cells.iter().any(|c| {
            let (x, y) = (i32::from(c.x()), i32::from(c.y()));
            OFFSETS
                .iter()
                .any(|(dx, dy)| opt_cc(x + dx, y + dy).is_some_and(|nb| !submerged.contains(&nb)))
        });
        assert_eq!(
            l.outlet.is_some(),
            has_free_neighbour,
            "lake {}: outlet is {:?} but a free neighbour {}exists — `None` \
             must mean genuinely no free neighbour, not a missed one",
            l.id,
            l.outlet,
            if has_free_neighbour { "" } else { "does not " }
        );
    }

    u32::try_from(objects.lakes.len()).unwrap_or(0)
}

#[test]
fn i_lake_outlets_are_geometrically_sound() {
    // `Lake.outlet` has needed two geometry-specific bugfixes (round-1 and
    // round-2 review), each caught only by a single hand-built synthetic
    // fixture. This sweeps every surviving lake across several REAL MICRO
    // tiles instead, checking the contract `assert_lake_outlets_are_sound`
    // documents.
    let mut lakes_checked = 0u32;

    for t in tiles() {
        lakes_checked += assert_lake_outlets_are_sound(&t.objects);
    }

    // A couple of extra (seed, tile) pairs on top of this file's own three
    // MICRO tiles — cheap to add because they are already known-good from
    // `arda-gen`'s own test suite rather than found by trial and error:
    // seed 123 tile (1, 0) and seed 99 tile (1, 1) each hold a near-rim
    // lake that survives the seam clamp (`area::mod`'s
    // `edge_touching_basins_take_the_continent_spill_level` and
    // `cross_tile.rs`'s
    // `seam_lakes_take_the_shared_surface_and_trim_below_it` both pin
    // "seed 099, 123 known to have one [a near-rim lake] per
    // task-5-report.md") — exactly the clamped/trimmed geometry both
    // historical outlet bugs lived in, so this is the coverage most
    // likely to catch a third one.
    let extra: [(u64, AreaCoord); 2] = [(123, AreaCoord::new(1, 0)), (99, AreaCoord::new(1, 1))];
    for (seed, coord) in extra {
        let c = build_continent(seed, GenerateConfig::MICRO, 0);
        let b = bundle_for(seed, &c, coord);
        let (_, objects) = arda_gen::area::generate_area(seed, &c, &b);
        lakes_checked += assert_lake_outlets_are_sound(&objects);
    }

    assert!(
        lakes_checked > 0,
        "no lakes were checked across any fixture -- a fixture drift that \
         silently removed every lake must fail loudly, not pass empty"
    );
}
