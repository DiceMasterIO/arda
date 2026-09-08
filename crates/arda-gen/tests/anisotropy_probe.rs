//! Open-items #4 (part 2) measurement probe.
//!
//! The first investigation's probe was never committed (its whole candidate
//! was worked and reverted uncommitted — see `.superpowers/sdd/
//! anisotropy-report.md`), so this reimplements the same methodology fresh:
//! runs the real pipeline (`relief` → `erosion::erode` → `fill` → `water` →
//! `compose`) over every MICRO seed-42 **attempt 2** tile (the world the
//! orchestrator/CLI/golden fixture actually accept — see `cross_tile.rs`'s
//! module doc) and reports:
//!
//! - **Diagonal share**: fraction of interior cells whose D8 downstream
//!   neighbour is diagonal, pre- and post-erosion (mirrors
//!   `water.rs::diagonal_share_is_near_half`'s own loop exactly).
//! - **Convergence**: % of channel cells with ≥2 channel inflows.
//! - **Lake area / land area**: off the actual composed `AreaObjects.lakes`
//!   (post `LAKE_MIN_CELLS`/`LAKE_MIN_DEPTH_MM` threshold) — what would
//!   actually render.
//! - **Coastline roughness**: sea-touching edges per coastal land cell.
//!
//! A second, independent probe measures continent-tier hypsometry (median
//! land elevation, % land above 500 m, lake area) at default size for seeds
//! 42 and 7, mirroring `docs/capstone/logic/01-continent-generation.md`'s
//! own recorded method — provably unaffected by any area-tier-only change
//! (nothing here touches `continent::*`), kept only so the acceptance gate
//! has a number attached rather than an assertion by construction.
//!
//! Run: `cargo test -p arda-gen --release --test anisotropy_probe -- --ignored --nocapture`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use arda_core::{
    AreaCoord, CellCoord, GenerateConfig, LatitudeBand, SizeKm, TerrainKind, AREA_CELLS,
};
use arda_gen::area::water::WaterGrid;
use arda_gen::area::{area_rainfall, compose, erosion, fill::fill, relief::relief, water::water};
use arda_gen::continent::bundles::{abs_cell, bundle_for, coarse_height};
use arda_gen::continent::climate::climate;
use arda_gen::continent::hydrology::{hydrology, NO_BASIN};
use arda_gen::continent::{build_continent, generate_continent, Continent};

const N: i32 = AREA_CELLS as i32;

const NEIGHBOURS8: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

fn coord(x: i32, y: i32) -> Option<CellCoord> {
    CellCoord::new(u16::try_from(x).ok()?, u16::try_from(y).ok()?)
}

#[derive(Default)]
struct TileDiag {
    diag_pre: usize,
    total_pre: usize,
    diag_post: usize,
    total_post: usize,
    diag_pre_land: usize,
    total_pre_land: usize,
    diag_post_land: usize,
    total_post_land: usize,
    channel_cells: usize,
    convergent_cells: usize,
    land_cells: usize,
    lake_cells: usize,
    coastal_edges: usize,
    coastal_cells: usize,
}

impl TileDiag {
    fn add(&mut self, o: &TileDiag) {
        self.diag_pre += o.diag_pre;
        self.total_pre += o.total_pre;
        self.diag_post += o.diag_post;
        self.total_post += o.total_post;
        self.diag_pre_land += o.diag_pre_land;
        self.total_pre_land += o.total_pre_land;
        self.diag_post_land += o.diag_post_land;
        self.total_post_land += o.total_post_land;
        self.channel_cells += o.channel_cells;
        self.convergent_cells += o.convergent_cells;
        self.land_cells += o.land_cells;
        self.lake_cells += o.lake_cells;
        self.coastal_edges += o.coastal_edges;
        self.coastal_cells += o.coastal_cells;
    }

    fn report(&self, label: &str) {
        let pct = |n: usize, d: usize| 100.0 * n as f64 / d.max(1) as f64;
        println!("=== {label} ===");
        println!(
            "diagonal share pre-erosion:  {:.1}% ({}/{})",
            pct(self.diag_pre, self.total_pre),
            self.diag_pre,
            self.total_pre
        );
        println!(
            "diagonal share post-erosion: {:.1}% ({}/{})",
            pct(self.diag_post, self.total_post),
            self.diag_post,
            self.total_post
        );
        println!(
            "diagonal share pre-erosion, land-gated:  {:.1}% ({}/{})",
            pct(self.diag_pre_land, self.total_pre_land),
            self.diag_pre_land,
            self.total_pre_land
        );
        println!(
            "diagonal share post-erosion, land-gated: {:.1}% ({}/{})",
            pct(self.diag_post_land, self.total_post_land),
            self.diag_post_land,
            self.total_post_land
        );
        println!(
            "convergence (>=2 channel inflows): {:.2}% ({}/{})",
            pct(self.convergent_cells, self.channel_cells),
            self.convergent_cells,
            self.channel_cells
        );
        println!(
            "lake area: {:.3}% ({}/{})",
            pct(self.lake_cells, self.land_cells),
            self.lake_cells,
            self.land_cells
        );
        println!(
            "coastline roughness: {:.3} ({} edges / {} coastal cells)",
            self.coastal_edges as f64 / self.coastal_cells.max(1) as f64,
            self.coastal_edges,
            self.coastal_cells
        );
    }
}

/// Mirrors `water.rs::diagonal_share_is_near_half`'s own loop exactly — no
/// land gate, so a sea cell with a defined downstream still counts.
fn diagonal_share(w: &WaterGrid) -> (usize, usize) {
    let (mut diag, mut total) = (0usize, 0usize);
    for y in 1..N - 1 {
        for x in 1..N - 1 {
            let Some(at) = coord(x, y) else { continue };
            if let Some(d) = w.downstream_of(at) {
                total += 1;
                if d.x() != at.x() && d.y() != at.y() {
                    diag += 1;
                }
            }
        }
    }
    (diag, total)
}

/// Same as [`diagonal_share`], gated to cells whose *contemporaneous*
/// height (raw relief for the pre-erosion pass, eroded heights for the
/// post-erosion pass) is land. The first anisotropy investigation's 8-tile
/// aggregate table (38.1%/23.2%) reads land-only by its own total counts
/// (986,278 versus this file's unfiltered 2,080,800 at attempt 2); this
/// variant is kept alongside the unfiltered one so candidates can be
/// compared against either baseline without ambiguity.
fn diagonal_share_land(w: &WaterGrid, heights: &[i32]) -> (usize, usize) {
    let (mut diag, mut total) = (0usize, 0usize);
    for y in 1..N - 1 {
        for x in 1..N - 1 {
            let Some(at) = coord(x, y) else { continue };
            if heights[(y * N + x) as usize] <= 0 {
                continue;
            }
            if let Some(d) = w.downstream_of(at) {
                total += 1;
                if d.x() != at.x() && d.y() != at.y() {
                    diag += 1;
                }
            }
        }
    }
    (diag, total)
}

fn convergence(w: &WaterGrid) -> (usize, usize) {
    let (mut channel_cells, mut convergent) = (0usize, 0usize);
    for y in 0..N {
        for x in 0..N {
            let Some(at) = coord(x, y) else { continue };
            if !w.is_channel(at) {
                continue;
            }
            channel_cells += 1;
            let inflows = NEIGHBOURS8
                .iter()
                .filter_map(|(dx, dy)| coord(x + dx, y + dy))
                .filter(|&nb| w.is_channel(nb) && w.downstream_of(nb) == Some(at))
                .count();
            if inflows >= 2 {
                convergent += 1;
            }
        }
    }
    (channel_cells, convergent)
}

fn measure_tile(seed: u64, ctx: &Continent, area: AreaCoord) -> TileDiag {
    let bundle = bundle_for(seed, ctx, area);
    let r = relief(seed, &ctx.grid, &bundle);
    let heights: Vec<i32> = (0..(N * N) as usize)
        .filter_map(|i| {
            let i = i32::try_from(i).ok()?;
            Some(r.get(coord(i % N, i / N)?))
        })
        .collect();
    let rain = area_rainfall(&bundle);

    // Pre-erosion: fill + water directly on raw relief.
    let filled_pre = fill(&heights, &bundle);
    let water_pre = water(&filled_pre, &bundle, &rain);
    let (diag_pre, total_pre) = diagonal_share(&water_pre);
    let (diag_pre_land, total_pre_land) = diagonal_share_land(&water_pre, &heights);

    // Post-erosion: the real pipeline, uplift from coarse_height exactly as
    // `generate_area` builds it.
    let uplift: Vec<i32> = (0..(N * N) as usize)
        .filter_map(|i| {
            let i = i32::try_from(i).ok()?;
            let (ax, ay) = abs_cell(area, u16::try_from(i % N).ok()?, u16::try_from(i / N).ok()?);
            Some(coarse_height(&ctx.grid, ax, ay))
        })
        .collect();
    let mut eroded = heights.clone();
    erosion::erode(&mut eroded, &uplift, &bundle);
    let filled_post = fill(&eroded, &bundle);
    let water_post = water(&filled_post, &bundle, &rain);
    let (diag_post, total_post) = diagonal_share(&water_post);
    let (diag_post_land, total_post_land) = diagonal_share_land(&water_post, &eroded);
    let (channel_cells, convergent_cells) = convergence(&water_post);

    let (cells, objects) = compose(&eroded, &filled_post, &water_post, &rain, &bundle).unwrap();
    let mut land_cells = 0usize;
    let mut coastal_edges = 0usize;
    let mut coastal_cells = 0usize;
    for y in 0..N {
        for x in 0..N {
            let Some(at) = coord(x, y) else { continue };
            if cells.get(at).terrain == TerrainKind::Sea {
                continue;
            }
            land_cells += 1;
            let sea_neighbours = [(0, -1), (0, 1), (-1, 0), (1, 0)]
                .iter()
                .filter_map(|(dx, dy)| coord(x + dx, y + dy))
                .filter(|&nb| cells.get(nb).terrain == TerrainKind::Sea)
                .count();
            if sea_neighbours > 0 {
                coastal_cells += 1;
                coastal_edges += sea_neighbours;
            }
        }
    }
    let lake_cells: usize = objects.lakes.iter().map(|l| l.cells.len()).sum();

    TileDiag {
        diag_pre,
        total_pre,
        diag_post,
        total_post,
        diag_pre_land,
        total_pre_land,
        diag_post_land,
        total_post_land,
        channel_cells,
        convergent_cells,
        land_cells,
        lake_cells,
        coastal_edges,
        coastal_cells,
    }
}

/// Run with `-- --nocapture --test-threads=1` and re-label between code
/// changes; kept permanently `#[ignore]`d, same precedent as
/// `continent_measures.rs` — measured, never gated.
#[test]
#[ignore = "open-items #4 measurement probe, run manually in release"]
fn anisotropy_measurements() {
    let seed = 42u64;
    let ctx = build_continent(seed, GenerateConfig::MICRO, 2);
    let mut agg = TileDiag::default();
    for area in GenerateConfig::MICRO.area_coords() {
        agg.add(&measure_tile(seed, &ctx, area));
    }
    agg.report("MICRO seed 42 attempt 2, 8 tiles, current tree");
}

/// Continent-tier hypsometry at default size, mirroring
/// `docs/capstone/logic/01-continent-generation.md`'s own recorded method
/// (seeds 42 and 7). Provably unaffected by any area-tier-only change.
#[test]
#[ignore = "open-items #4 hypsometry sanity, run manually in release"]
fn hypsometry_default_size() {
    let config = GenerateConfig::new(SizeKm::new(500, 1_000), LatitudeBand::new(35, 55), 15)
        .expect("valid default config");
    for seed in [42u64, 7] {
        let g = generate_continent(seed, config);
        let c = climate(&g, LatitudeBand::new(35, 55));
        let hy = hydrology(&g, &c);
        let (w, h) = (g.width(), g.height());
        let mut elevations: Vec<i32> = Vec::new();
        let (mut land, mut above_500, mut lake_any, mut lake_2m) = (0u64, 0u64, 0u64, 0u64);
        for y in 0..h {
            for x in 0..w {
                let e = g.get(x, y).raw();
                if e <= 0 {
                    continue;
                }
                land += 1;
                elevations.push(e);
                if e > 500_000 {
                    above_500 += 1;
                }
                let i = usize::try_from(y * w + x).unwrap_or(0);
                if hy.basin_surface[i] != NO_BASIN {
                    lake_any += 1;
                    // Area-tier's own LAKE_MIN_DEPTH_MM (2_000 mm) applied
                    // here at continent scale, for comparison against
                    // logic/01's recorded ~1% figure: `basin_surface` marks
                    // ANY nonzero depression, including sub-metre numerical
                    // noise pits a real "lake" definition would not count.
                    if hy.basin_surface[i] - g.get(x, y).raw() >= 2_000 {
                        lake_2m += 1;
                    }
                }
            }
        }
        elevations.sort_unstable();
        let median = elevations.get(elevations.len() / 2).copied().unwrap_or(0);
        println!(
            "seed {seed}: median land elevation {median} mm ({:.1} m), land>500m {:.1}%, \
             lake area (any depth) {:.2}%, lake area (>=2m deep) {:.2}%",
            f64::from(median) / 1000.0,
            100.0 * above_500 as f64 / land.max(1) as f64,
            100.0 * lake_any as f64 / land.max(1) as f64,
            100.0 * lake_2m as f64 / land.max(1) as f64,
        );
    }
}
