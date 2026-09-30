//! Form broad, connected river valleys in the canonical fine field before water is solved.
//!
//! This is a source transformation, not a channel overlay. The resulting terrain
//! file is subsequently sampled for continent climate, prepared fine heights and
//! Atlas display. Coarse drainage only proposes where to form valleys; final
//! drainage is recomputed from the transformed physical heights.

use crate::{
    continent::{hydrology::ContinentHydrology, ContinentGrid},
    noise::value_noise,
};
use arda_core::{HeightMm, TerrainFileError, TerrainFileReader, TerrainFileWriter, TerrainPoint};
use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::Path,
};

const SPACING_UM: u32 = super::fine_source::FINE_SPACING_UM;
const HEADER_BYTES: u64 = 88;
const KM_M: i64 = 1_000;
const Q16: i64 = 65_536;
const MIN_CATCHMENT_KM2: u32 = 64;
const MAX_CUT_MM: i64 = 150_000;

/// A source-stage failure leaves only the private candidate and provisional output.
#[derive(Debug, thiserror::Error)]
pub enum ValleyError {
    /// The canonical terrain file was invalid or could not be finalized.
    #[error(transparent)]
    Terrain(#[from] TerrainFileError),
    /// A private source read or write failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Source and coarse drainage do not cover the same physical rectangle.
    #[error("fine valley source and coarse drainage geometry disagree")]
    Geometry,
    /// The sparse network or row allocation could not be represented.
    #[error("fine valley network allocation failed")]
    Allocation,
}

#[derive(Debug, Clone, Copy)]
struct Edge {
    ax_m: i64,
    ay_m: i64,
    dx_m: i64,
    dy_m: i64,
    bed_a_mm: i64,
    bed_b_mm: i64,
    depth_a_mm: i64,
    depth_b_mm: i64,
    radius_m: i64,
    bend_m: i64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct ValleyStats {
    pub edges: u64,
    pub lowered_samples: u64,
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

fn depth_mm(catchment: u32) -> i64 {
    let octaves = 31 - (catchment / MIN_CATCHMENT_KM2).max(1).leading_zeros();
    4_000 + 2_000 * i64::from(octaves.min(13))
}

fn radius_m(catchment: u32) -> i64 {
    let octaves = 31 - (catchment / MIN_CATCHMENT_KM2).max(1).leading_zeros();
    300 + 55 * i64::from(octaves.min(10))
}

fn normal_q16(dx_m: i64, dy_m: i64) -> (i64, i64) {
    let length_m = (dx_m * dx_m + dy_m * dy_m).isqrt();
    if length_m == 0 {
        return (0, 0);
    }
    (-dy_m * Q16 / length_m, dx_m * Q16 / length_m)
}

fn displacement_q12(grid: &ContinentGrid, x: i32, y: i32) -> i64 {
    let here = i64::from(grid.get(x, y).raw());
    let mut local_rise_mm = 0_i64;
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let (nx, ny) = (x + dx, y + dy);
        if nx >= 0 && ny >= 0 && nx < grid.width() && ny < grid.height() {
            local_rise_mm = local_rise_mm.max((i64::from(grid.get(nx, ny).raw()) - here).abs());
        }
    }
    // Wide lowland floodplains can wander by kilometres. Steep mountain
    // channels retain the nearby coarse trough instead of cutting across a
    // ridge merely to satisfy a planar noise offset.
    let relief_q12 = ((30_000 - local_rise_mm) * 4_096 / 25_000).clamp(0, 4_096);
    let border_km = x
        .min(y)
        .min(grid.width() - 1 - x)
        .min(grid.height() - 1 - y);
    let border_q12 = (i64::from(border_km) * 4_096 / 5).clamp(0, 4_096);
    relief_q12 * border_q12 / 4_096
}

fn node_m(seed: u64, x: i32, y: i32, displacement_q12: i64) -> (i64, i64) {
    // Every edge meeting at this coarse drainage node uses the same displaced
    // position. Smooth multi-kilometre offsets free long D8 cardinal runs from
    // the 1 km lattice without creating gaps at confluences.
    let east_short = i64::from(value_noise(seed ^ 0x07c5_c14b_d301_8b29, x, y, 14));
    let east_broad = i64::from(value_noise(seed ^ 0xf4d2_a753_9cb0_1e65, x, y, 37));
    let south_short = i64::from(value_noise(seed ^ 0x837f_449e_eb24_60d1, x, y, 14));
    let south_broad = i64::from(value_noise(seed ^ 0x2c8e_b1d4_0a56_399f, x, y, 37));
    (
        i64::from(x) * KM_M
            + (east_short * 2_700 + east_broad * 1_300) * displacement_q12 / (32_768 * 4_096),
        i64::from(y) * KM_M
            + (south_short * 2_700 + south_broad * 1_300) * displacement_q12 / (32_768 * 4_096),
    )
}

fn center_m(edge: Edge, t_q16: i64) -> (i64, i64) {
    let (nx, ny) = normal_q16(edge.dx_m, edge.dy_m);
    // A parabolic deviation meets both coarse endpoints exactly. Confluences
    // therefore share one real floor cell even when adjacent edges bend apart.
    let bend_q16 = 4 * t_q16 * (Q16 - t_q16) / Q16;
    let offset_x = nx * edge.bend_m * bend_q16 / (Q16 * Q16);
    let offset_y = ny * edge.bend_m * bend_q16 / (Q16 * Q16);
    (
        edge.ax_m + edge.dx_m * t_q16 / Q16 + offset_x,
        edge.ay_m + edge.dy_m * t_q16 / Q16 + offset_y,
    )
}

fn carve_sample(old_mm: i32, x_m: i64, y_m: i64, edge: Edge) -> i32 {
    if old_mm <= 0 {
        return old_mm;
    }
    let vx = x_m - edge.ax_m;
    let vy = y_m - edge.ay_m;
    let length_sq = edge.dx_m * edge.dx_m + edge.dy_m * edge.dy_m;
    let t_q16 = ((vx * edge.dx_m + vy * edge.dy_m) * Q16 / length_sq).clamp(0, Q16);
    let (cx, cy) = center_m(edge, t_q16);
    let dist_sq = (x_m - cx).pow(2) + (y_m - cy).pow(2);
    let radius_sq = edge.radius_m * edge.radius_m;
    if dist_sq >= radius_sq {
        return old_mm;
    }
    let bed = edge.bed_a_mm + (edge.bed_b_mm - edge.bed_a_mm) * t_q16 / Q16;
    let depth = edge.depth_a_mm + (edge.depth_b_mm - edge.depth_a_mm) * t_q16 / Q16;
    // A narrow flat bed is required at the 100 m routing lattice. If the
    // parabolic center moves 20–30 m between adjacent routing cells, a
    // point-only trough can turn uphill even while its 39 m center descends.
    let core_sq = 100_i64.pow(2).min(radius_sq / 4);
    let shoulder_sq = (dist_sq - core_sq).max(0);
    let span_sq = radius_sq - core_sq;
    let floor = bed + depth * shoulder_sq / span_sq;
    let wanted = (i64::from(old_mm) - floor).max(0);
    if wanted == 0 {
        return old_mm;
    }
    let remaining = radius_sq - dist_sq;
    let taper_q16 = (remaining * Q16 / span_sq).min(Q16);
    let cut =
        i128::from(wanted) * i128::from(taper_q16) * i128::from(taper_q16) / i128::from(Q16 * Q16);
    let cut = i64::try_from(cut.min(i128::from(MAX_CUT_MM))).unwrap_or(MAX_CUT_MM);
    i32::try_from((i64::from(old_mm) - cut).max(1)).unwrap_or(old_mm)
}

fn choose_bend<R: Read + Seek>(
    source: &mut TerrainFileReader<R>,
    seed: u64,
    ordinal: usize,
    edge: Edge,
) -> Result<i64, ValleyError> {
    let (nx, ny) = normal_q16(edge.dx_m, edge.dy_m);
    let max_bend = (edge.radius_m / 3).min(250);
    let mut chosen = (i64::MAX, 0);
    for (k, unit) in [-2_i64, -1, 0, 1, 2].into_iter().enumerate() {
        let offset = unit * max_bend / 2;
        let mx = edge.ax_m + edge.dx_m / 2 + nx * offset / Q16;
        let my = edge.ay_m + edge.dy_m / 2 + ny * offset / Q16;
        let Some(height) = source.sample(TerrainPoint {
            x_um: mx * 1_000_000,
            y_um: my * 1_000_000,
        })?
        else {
            continue;
        };
        // Existing low ground wins. A small seeded lateral-erosion term also
        // starts bends on nearly featureless plains where every sample ties.
        let jitter = i64::try_from(splitmix(seed ^ (ordinal as u64) ^ ((k as u64) << 40)) % 2_001)
            .map_err(|_| ValleyError::Allocation)?;
        let score = i64::from(height.raw()) + offset.abs() * 2 + jitter;
        if score < chosen.0 {
            chosen = (score, offset);
        }
    }
    Ok(chosen.1)
}

fn network<R: Read + Seek>(
    source: &mut TerrainFileReader<R>,
    grid: &ContinentGrid,
    hydro: &ContinentHydrology,
    seed: u64,
) -> Result<(Vec<Edge>, Vec<Vec<u32>>), ValleyError> {
    let width = usize::try_from(grid.width()).map_err(|_| ValleyError::Geometry)?;
    let height = usize::try_from(grid.height()).map_err(|_| ValleyError::Geometry)?;
    let count = width.checked_mul(height).ok_or(ValleyError::Geometry)?;
    if hydro.downstream.len() != count
        || hydro.catchment_km2.len() != count
        || hydro.filled.len() != count
    {
        return Err(ValleyError::Geometry);
    }
    let mut edges = Vec::new();
    let mut bins = Vec::new();
    bins.try_reserve_exact(count)
        .map_err(|_| ValleyError::Allocation)?;
    bins.resize_with(count, Vec::new);
    for i in 0..count {
        let area = hydro.catchment_km2[i];
        let Some(d) = hydro.downstream[i].and_then(|v| usize::try_from(v).ok()) else {
            continue;
        };
        if area < MIN_CATCHMENT_KM2 || d >= count {
            continue;
        }
        let (x, y) = (i % width, i / width);
        let (tx, ty) = (d % width, d / width);
        if x.abs_diff(tx) > 1 || y.abs_diff(ty) > 1 {
            return Err(ValleyError::Geometry);
        }
        let (x, y, tx, ty) = (
            i32::try_from(x).map_err(|_| ValleyError::Geometry)?,
            i32::try_from(y).map_err(|_| ValleyError::Geometry)?,
            i32::try_from(tx).map_err(|_| ValleyError::Geometry)?,
            i32::try_from(ty).map_err(|_| ValleyError::Geometry)?,
        );
        let from_h = grid.get(x, y).raw();
        let to_h = grid.get(tx, ty).raw();
        if from_h <= 0 || to_h <= 0 || hydro.filled[i] > from_h || hydro.filled[d] > to_h {
            continue;
        }
        let from = i64::from(from_h);
        let to = i64::from(to_h);
        let depth_a_mm = depth_mm(area);
        let depth_b_mm = depth_mm(hydro.catchment_km2[d].max(area));
        let (ax_m, ay_m) = node_m(seed, x, y, displacement_q12(grid, x, y));
        let (bx_m, by_m) = node_m(seed, tx, ty, displacement_q12(grid, tx, ty));
        let mut edge = Edge {
            ax_m,
            ay_m,
            dx_m: bx_m - ax_m,
            dy_m: by_m - ay_m,
            bed_a_mm: from - depth_a_mm,
            bed_b_mm: to - depth_b_mm,
            depth_a_mm,
            depth_b_mm,
            radius_m: radius_m(area),
            bend_m: 0,
        };
        edge.bend_m = choose_bend(source, seed, i, edge)?;
        let id = u32::try_from(edges.len()).map_err(|_| ValleyError::Allocation)?;
        let spread_m = edge.radius_m + edge.bend_m.abs();
        let x0 = usize::try_from(
            ((edge.ax_m.min(edge.ax_m + edge.dx_m) - spread_m).div_euclid(KM_M)).max(0),
        )
        .map_err(|_| ValleyError::Geometry)?;
        let y0 = usize::try_from(
            ((edge.ay_m.min(edge.ay_m + edge.dy_m) - spread_m).div_euclid(KM_M)).max(0),
        )
        .map_err(|_| ValleyError::Geometry)?;
        let x1 = ((edge.ax_m.max(edge.ax_m + edge.dx_m) + spread_m).div_euclid(KM_M))
            .min(i64::try_from(width).map_err(|_| ValleyError::Geometry)? - 1);
        let y1 = ((edge.ay_m.max(edge.ay_m + edge.dy_m) + spread_m).div_euclid(KM_M))
            .min(i64::try_from(height).map_err(|_| ValleyError::Geometry)? - 1);
        let x1 = usize::try_from(x1).map_err(|_| ValleyError::Geometry)?;
        let y1 = usize::try_from(y1).map_err(|_| ValleyError::Geometry)?;
        for by in y0..=y1 {
            for bx in x0..=x1 {
                bins[by * width + bx]
                    .try_reserve(1)
                    .map_err(|_| ValleyError::Allocation)?;
                bins[by * width + bx].push(id);
            }
        }
        edges.try_reserve(1).map_err(|_| ValleyError::Allocation)?;
        edges.push(edge);
    }
    Ok((edges, bins))
}

/// Write a checked replacement source into `destination` using only the
/// unmodified source and a proposed coarse drainage tree. The caller owns
/// staging, admission and atomic selection of the resulting file.
pub(super) fn carve(
    source_path: &Path,
    destination: &Path,
    grid: &ContinentGrid,
    hydro: &ContinentHydrology,
    seed: u64,
) -> Result<ValleyStats, ValleyError> {
    let mut source = TerrainFileReader::open(File::open(source_path)?, 1 << 20)?;
    if source.origin() != (TerrainPoint { x_um: 0, y_um: 0 }) || source.spacing_um() != SPACING_UM {
        return Err(ValleyError::Geometry);
    }
    let (edges, bins) = network(&mut source, grid, hydro, seed)?;
    let coarse_width = usize::try_from(grid.width()).map_err(|_| ValleyError::Geometry)?;
    let coarse_height = usize::try_from(grid.height()).map_err(|_| ValleyError::Geometry)?;
    let width = source.width();
    let height = source.height();
    let last_x_m = i64::from(width - 1) * i64::from(SPACING_UM) / 1_000_000;
    let last_y_m = i64::from(height - 1) * i64::from(SPACING_UM) / 1_000_000;
    if last_x_m / KM_M + 1 < grid.width() as i64 || last_y_m / KM_M + 1 < grid.height() as i64 {
        return Err(ValleyError::Geometry);
    }
    let mut input = BufReader::new(File::open(source_path)?);
    input.seek(SeekFrom::Start(HEADER_BYTES))?;
    let output = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(destination)?;
    let mut writer = TerrainFileWriter::new(
        BufWriter::new(output),
        source.origin(),
        source.spacing_um(),
        width,
        height,
    )?;
    let row_len = usize::try_from(width).map_err(|_| ValleyError::Allocation)?;
    let mut raw = Vec::new();
    raw.try_reserve_exact(row_len * 4)
        .map_err(|_| ValleyError::Allocation)?;
    raw.resize(row_len * 4, 0);
    let mut row = Vec::new();
    row.try_reserve_exact(row_len)
        .map_err(|_| ValleyError::Allocation)?;
    let mut stats = ValleyStats {
        edges: edges.len() as u64,
        lowered_samples: 0,
    };
    for y in 0..height {
        input.read_exact(&mut raw)?;
        row.clear();
        let y_m = i64::from(y) * i64::from(SPACING_UM) / 1_000_000;
        let by = usize::try_from(y_m / KM_M).map_err(|_| ValleyError::Geometry)?;
        for x in 0..width {
            let i = usize::try_from(x).map_err(|_| ValleyError::Allocation)?;
            let old = i32::from_le_bytes(
                raw[i * 4..i * 4 + 4]
                    .try_into()
                    .map_err(|_| ValleyError::Geometry)?,
            );
            let x_m = i64::from(x) * i64::from(SPACING_UM) / 1_000_000;
            let bx = usize::try_from(x_m / KM_M).map_err(|_| ValleyError::Geometry)?;
            let mut next = old;
            if old > 0 && bx < coarse_width && by < coarse_height {
                for &edge in &bins[by * coarse_width + bx] {
                    next = carve_sample(next, x_m, y_m, edges[edge as usize]);
                }
            }
            stats.lowered_samples += u64::from(next < old);
            row.push(HeightMm::new(next));
        }
        writer.write_row(&row)?;
    }
    let mut output = writer.finish()?;
    output.flush()?;
    Ok(stats)
}

#[cfg(test)]
mod tests {
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
        assert!(
            sample(&mut formed, center.0, center.1) < sample(&mut original, center.0, center.1)
        );
        let next = node_m(42, 4, 3, displacement_q12(&grid, 4, 3));
        let midpoint = ((center.0 + next.0) / 2, (center.1 + next.1) / 2);
        assert!(
            sample(&mut formed, midpoint.0, midpoint.1)
                < sample(&mut original, midpoint.0, midpoint.1)
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
        let south_range = nodes.iter().map(|(_, y)| y).max().unwrap()
            - nodes.iter().map(|(_, y)| y).min().unwrap();
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
}
