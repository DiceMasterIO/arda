use super::*;
use crate::continent::hydrology::NO_BASIN;
use crate::hydrology::routing::{
    route_and_own, CellIndex, Extent, Limits, MemoryPages, Receiver, RoutingStore,
};
use std::sync::atomic::{AtomicU64, Ordering};

struct Directory(std::path::PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "arda-fine-valleys-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn plane() -> (ContinentGrid, ContinentHydrology) {
    let heights: Vec<i32> = (0..6)
        .flat_map(|_| (0..6).map(|x| 300_000 - 1_000 * x))
        .collect();
    let grid = ContinentGrid::from_heights(6, 6, heights.clone()).unwrap();
    let mut downstream = vec![None; 36];
    let mut catchment_km2 = vec![0; 36];
    for x in 1..5 {
        let i = 3 * 6 + x;
        downstream[i] = Some(u32::try_from(i + 1).unwrap());
        catchment_km2[i] = 1_024 + 64 * u32::try_from(x).unwrap();
    }
    catchment_km2[3 * 6 + 5] = 2_048;
    let hydro = ContinentHydrology {
        filled: heights,
        basin_surface: vec![NO_BASIN; 36],
        downstream,
        downstream_dir: vec![arda_core::NO_DOWNSTREAM; 36],
        catchment_km2,
        discharge_l_s: vec![0; 36],
    };
    (grid, hydro)
}

fn source(path: &Path) {
    let output = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    let mut writer = TerrainFileWriter::new(
        output,
        TerrainPoint { x_um: 0, y_um: 0 },
        SPACING_UM,
        155,
        155,
    )
    .unwrap();
    for _ in 0..155 {
        let row: Vec<_> = (0..155)
            .map(|x| {
                HeightMm::new(
                    300_000
                        - i32::try_from(i64::from(x) * i64::from(SPACING_UM) / 1_000_000_000)
                            .unwrap()
                            * 1_000,
                )
            })
            .collect();
        writer.write_row(&row).unwrap();
    }
    writer.finish().unwrap();
}

fn sample(reader: &mut TerrainFileReader<File>, x_m: i64, y_m: i64) -> i32 {
    reader
        .sample(TerrainPoint {
            x_um: x_m * 1_000_000,
            y_um: y_m * 1_000_000,
        })
        .unwrap()
        .unwrap()
        .raw()
}

#[test]
fn saved_valley_is_real_bounded_terrain_and_replays_exactly() {
    let dir = Directory::new();
    let input = dir.0.join("source.terrain");
    let output = dir.0.join("valleys.terrain");
    let replay = dir.0.join("replay.terrain");
    source(&input);
    let (grid, hydro) = plane();
    let result = carve(&input, &output, &grid, &hydro, 42).unwrap();
    assert_eq!(result.edges, 4);
    assert!(result.lowered_samples > 100);
    assert_eq!(carve(&input, &replay, &grid, &hydro, 42).unwrap(), result);
    assert_eq!(
        std::fs::read(&output).unwrap(),
        std::fs::read(&replay).unwrap()
    );

    let mut original = TerrainFileReader::open(File::open(&input).unwrap(), 1 << 20).unwrap();
    let mut formed = TerrainFileReader::open(File::open(&output).unwrap(), 1 << 20).unwrap();
    // Both registered 1 km and off-node fine samples were physically
    // changed; the untouched bank and the sea-level sign remain stable.
    let center = node_m(42, 3, 3, displacement_q12(&grid, 3, 3));
    assert!(sample(&mut formed, center.0, center.1) < sample(&mut original, center.0, center.1));
    let next = node_m(42, 4, 3, displacement_q12(&grid, 4, 3));
    let midpoint = ((center.0 + next.0) / 2, (center.1 + next.1) / 2);
    assert!(
        sample(&mut formed, midpoint.0, midpoint.1) < sample(&mut original, midpoint.0, midpoint.1)
    );
    assert_eq!(
        sample(&mut formed, 500, 5_500),
        sample(&mut original, 500, 5_500)
    );
    assert_eq!(
        (formed.width(), formed.height(), formed.spacing_um()),
        (155, 155, SPACING_UM)
    );
}

#[test]
fn dry_valley_has_lower_center_and_joins_at_coarse_nodes() {
    let edge = Edge {
        ax_m: 1_000,
        ay_m: 3_000,
        dx_m: 1_000,
        dy_m: 0,
        bed_a_mm: 280_000,
        bed_b_mm: 279_000,
        depth_a_mm: 20_000,
        depth_b_mm: 20_000,
        radius_m: 600,
        bend_m: 150,
    };
    let bank = carve_sample(300_000, 1_500, 3_800, edge);
    let (cx, cy) = center_m(edge, Q16 / 2);
    let center = carve_sample(300_000, cx, cy, edge);
    assert_eq!(bank, 300_000);
    assert!(center < carve_sample(300_000, cx, cy + 300, edge));
    assert_eq!(center_m(edge, 0), (1_000, 3_000));
    assert_eq!(center_m(edge, Q16), (2_000, 3_000));
}

#[test]
fn displaced_coarse_nodes_bend_cardinal_runs_and_join_edges() {
    let nodes: Vec<_> = (0..64).map(|x| node_m(42, x, 10, 4_096)).collect();
    let south_range =
        nodes.iter().map(|(_, y)| y).max().unwrap() - nodes.iter().map(|(_, y)| y).min().unwrap();
    assert!(
        south_range > 1_000,
        "cardinal run remained straight: {south_range} m"
    );
    for pair in nodes.windows(2) {
        assert!(pair[1].0 > pair[0].0, "centerline doubled back");
        assert!((pair[1].1 - pair[0].1).abs() < 1_000, "abrupt corner");
    }
    let (grid, hydro) = plane();
    let dir = Directory::new();
    let input = dir.0.join("source.terrain");
    source(&input);
    let mut reader = TerrainFileReader::open(File::open(&input).unwrap(), 1 << 20).unwrap();
    let (edges, _) = network(&mut reader, &grid, &hydro, 42).unwrap();
    for pair in edges.windows(2) {
        assert_eq!(center_m(pair[0], Q16), center_m(pair[1], 0));
    }
}

#[test]
fn lowland_wander_tapers_at_steep_slopes_and_world_edges() {
    let plain = ContinentGrid::from_heights(11, 11, vec![300_000; 121]).unwrap();
    assert_eq!(displacement_q12(&plain, 5, 5), 4_096);
    assert_eq!(displacement_q12(&plain, 0, 5), 0);
    let mut steep_heights = vec![300_000; 121];
    steep_heights[5 * 11 + 6] = 340_000;
    let steep = ContinentGrid::from_heights(11, 11, steep_heights).unwrap();
    assert_eq!(displacement_q12(&steep, 5, 5), 0);
}

#[test]
fn physical_receivers_follow_a_bent_valley_floor() {
    let edge = Edge {
        ax_m: 1_000,
        ay_m: 1_500,
        dx_m: 4_000,
        dy_m: 0,
        bed_a_mm: 289_000,
        bed_b_mm: 285_000,
        depth_a_mm: 10_000,
        depth_b_mm: 10_000,
        radius_m: 320,
        bend_m: 300,
    };
    let extent = Extent::new(61, 31).unwrap();
    assert_eq!(
        carve_sample(299_000, 1_000, 1_500, edge),
        289_000,
        "center={:?}, edge={edge:?}",
        center_m(edge, 0)
    );
    let heights: Vec<_> = (0..31)
        .flat_map(|y| {
            (0..61).map(move |x| {
                let old = 300_000 - x * 100;
                carve_sample(old, i64::from(x) * 100, i64::from(y) * 100, edge)
            })
        })
        .collect();
    let mut store =
        MemoryPages::new(extent, &heights, &vec![false; heights.len()], 1 << 20).unwrap();
    assert!(
        heights[15 * 61 + 11] < heights[15 * 61 + 10],
        "next={} start={}",
        heights[15 * 61 + 11],
        heights[15 * 61 + 10]
    );
    route_and_own(&mut store, Limits::for_extent(extent)).unwrap();
    let mut at = CellIndex::new(15 * 61 + 10, extent).unwrap();
    let mut min_y = u32::MAX;
    let mut max_y = 0;
    let mut farthest_x = 0;
    for _ in 0..100 {
        let (x, y) = extent.coordinates(at);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
        farthest_x = farthest_x.max(x);
        match store.read(at).unwrap().receiver(extent, at).unwrap() {
            Receiver::Cell(next) => at = next,
            Receiver::Stop(_) => break,
        }
    }
    assert!(
        farthest_x >= 45,
        "receiver left the formed valley at x={farthest_x}, y span={min_y}..{max_y}; row={:?}",
        &heights[15 * 61 + 10..15 * 61 + 17]
    );
    assert!(
        max_y - min_y >= 2,
        "receiver ignored its physical bend: x={farthest_x}, y span={min_y}..{max_y}"
    );
}
