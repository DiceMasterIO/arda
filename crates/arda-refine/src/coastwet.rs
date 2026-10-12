//! Coast and wetland statistics of a world: how often each biome signal
//! fires, what ground the prior favours, how many pools there are and how
//! wide the coast strip is for each saved shore class (logic/09 §biome
//! signals). `arda-refine coastwet` prints it as JSON; the retuning of
//! [`crate::biome`] and [`crate::prior`] is checked against it.
//!
//! Sampling is a fixed stride over cell coordinates, so one world and one
//! stride always give the same bytes.

use crate::biome::{alpine, arid, coast_facts, coast_strip, marshy, Coast};
use crate::classes::{Class, COUNT};
use crate::context::Ctx;
use crate::error::RefineError;
use crate::hash::hash2;
use crate::pools;
use crate::prior::cell_weights;
use crate::source::{CellKey, Source};
use arda::TerrainKind;
use arda_core::ShoreClass;
use serde::Serialize;
use std::collections::BTreeMap;

/// Widest strip the walk inland counts, cells.
const WALK_MAX: usize = 8;
/// Squares between pool probes inside a block, per axis.
const POOL_STEP: usize = 8;
/// Wetness levels (`0..=1`) of the ladder in [`Group::wetness_ge_pct`].
const WET_LEVELS: [f64; 5] = [0.5, 0.6, 0.7, 0.8, 0.9];
/// Climate bands by mean annual temperature (°C): name and upper bound.
const BANDS: [(&str, f64); 3] = [("cold", 5.0), ("cool", 11.0), ("warm", f64::INFINITY)];

/// The `temperate_lowland` group: mean annual temperature range (°C) and
/// the height under which land counts as lowland (m).
const LOWLAND: ((f64, f64), f64) = ((5.0, 18.0), 300.0);

/// Counts for one group of sampled land cells.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Group {
    /// Sampled land cells.
    pub land_cells: u64,
    /// Share of land cells with `marshy > 0.5`, percent.
    pub marshy_pct: f64,
    /// Share with the coast strip above 0.5, percent.
    pub coastal_pct: f64,
    /// Share with `arid > 0.5`, percent.
    pub arid_pct: f64,
    /// Share with `alpine > 0.5`, percent.
    pub alpine_pct: f64,
    /// Share of land cells with wetness at or above each level, percent.
    pub wetness_ge_pct: BTreeMap<String, f64>,
    /// Share of cells whose strongest prior ground is each key, percent.
    pub ground_pct: BTreeMap<String, f64>,
    /// Mean share of the prior's total weight held by each key, percent.
    pub weight_pct: BTreeMap<String, f64>,
    /// Pool statistics over the blocks sampled in this group.
    pub pools: PoolStats,
}

/// Pools over the sampled blocks of a group.
#[derive(Debug, Default, Clone, Serialize)]
pub struct PoolStats {
    /// Blocks sampled.
    pub blocks: u64,
    /// Blocks that are wet cells ([`pools::wet_cell`]).
    pub wet_blocks: u64,
    /// Pools (sites holding water) per km² of wet cells.
    pub pools_per_km2_wet: f64,
    /// Share of probed squares in wet cells that lie in a pool, percent.
    pub pool_area_pct_wet: f64,
    /// Share of probed squares in cells that are not marsh (no marsh cover,
    /// `marshy <= 0.5`) that lie in a pool, percent.
    pub pool_area_pct_outside_marsh: f64,
}

/// Coast strip width for one shore class.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Strip {
    /// Sea-touching land cells sampled.
    pub cells: u64,
    /// Mean strip width, cells.
    pub mean_width: f64,
    /// Cells by strip width `0..=WALK_MAX` (the last bucket is "or more").
    pub width_hist: Vec<u64>,
    /// Share whose strongest prior ground is sand, percent.
    pub sand_ground_pct: f64,
}

/// The whole report.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// World seed.
    pub seed: u64,
    /// World extent, cells.
    pub cells: (i64, i64),
    /// Whether the world has a shore layer.
    pub has_shore_layer: bool,
    /// Sampling stride, cells.
    pub stride: i64,
    /// Land cell groups: `all`, the climate bands and `temperate_lowland`.
    pub groups: BTreeMap<String, Group>,
    /// Share of sea-touching land cells by shore class, percent.
    pub coast_class_pct: BTreeMap<String, f64>,
    /// Coast strip width by shore class (`unclassified` without a layer).
    pub strip: BTreeMap<String, Strip>,
}

#[derive(Default)]
struct Tally {
    land: u64,
    marshy: u64,
    coastal: u64,
    arid: u64,
    alpine: u64,
    ground: [u64; COUNT],
    weight: [f64; COUNT],
    wet: [u64; WET_LEVELS.len()],
    blocks: u64,
    wet_blocks: u64,
    sites: u64,
    wet_probes: u64,
    wet_pool: u64,
    dry_probes: u64,
    dry_pool: u64,
}

fn pct(n: u64, d: u64) -> f64 {
    if d == 0 {
        return 0.0;
    }
    #[allow(clippy::cast_precision_loss)] // counts are far below 2^52
    let v = n as f64 * 100.0 / d as f64;
    (v * 1000.0).round() / 1000.0
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// The coast facts of `k`, read straight from the source: the same answer
/// as the gathered [`Ctx::coast_at`].
///
/// # Errors
/// A layer failed to load.
pub fn coast_of(src: &dyn Source, k: CellKey) -> Result<Coast, RefineError> {
    let mut failed = None;
    let coast = coast_facts(
        |dx, dy| {
            let at = src.clamp(k.offset(dx, dy));
            match (src.cell(at), src.shore(at)) {
                (Ok(c), Ok(s)) => (c.terrain, s),
                (Err(e), _) | (_, Err(e)) => {
                    failed.get_or_insert(e);
                    (TerrainKind::Lake, ShoreClass::None)
                }
            }
        },
        src.has_shore(),
    );
    failed.map_or(Ok(coast), Err)
}

/// The strongest ground of a cell's prior and its weights as shares.
fn dominant(c: &arda::Cell, coast: Coast) -> (usize, [f64; COUNT]) {
    let w = cell_weights(c, coast);
    let mut best = 0;
    for (i, v) in w.iter().enumerate() {
        if *v > w[best] {
            best = i;
        }
    }
    let total: f64 = w.iter().sum::<f64>().max(1e-12);
    (best, w.map(|v| v / total))
}

fn class_name(s: ShoreClass) -> &'static str {
    match s {
        ShoreClass::None => "none",
        ShoreClass::SandBeach => "sand_beach",
        ShoreClass::ShingleBeach => "shingle_beach",
        ShoreClass::Cliff => "cliff",
        ShoreClass::RockyShore => "rocky_shore",
        ShoreClass::Marsh => "salt_marsh",
        ShoreClass::TidalFlat => "tidal_flat",
        ShoreClass::Estuary => "estuary",
    }
}

/// Which groups a land cell belongs to.
fn groups_of(c: &arda::Cell) -> Vec<&'static str> {
    let t = f64::from(c.temperature.raw()) / 100.0;
    let mut g = vec!["all"];
    if let Some((name, _)) = BANDS.iter().find(|(_, hi)| t < *hi) {
        g.push(name);
    }
    let h = f64::from(c.height.raw()) / 1000.0;
    if (LOWLAND.0 .0..LOWLAND.0 .1).contains(&t) && h < LOWLAND.1 {
        g.push("temperate_lowland");
    }
    g
}

/// Probes a block's pools; fills the tally of every group of the cell.
fn sample_block(
    src: &dyn Source,
    k: CellKey,
    c: &arda::Cell,
    tallies: &mut BTreeMap<&'static str, Tally>,
) -> Result<(), RefineError> {
    let ctx = Ctx::gather(src, k)?;
    let wet = pools::wet_cell(c);
    let marsh = c.cover == arda::Cover::Marsh || marshy(c) > 0.5;
    let sites = u64::try_from(pools::sites_in_cell(&ctx, k).len()).unwrap_or(0);
    let (mut probes, mut inside) = (0_u64, 0_u64);
    for j in 0..64 / POOL_STEP {
        for i in 0..64 / POOL_STEP {
            #[allow(clippy::cast_precision_loss)] // small indices
            let (u, v) = (
                (k.x * 64) as f64 + (i * POOL_STEP) as f64 + 0.5,
                (k.y * 64) as f64 + (j * POOL_STEP) as f64 + 0.5,
            );
            probes += 1;
            inside += u64::from(pools::pool(&ctx, u, v) > 0.0);
        }
    }
    for g in groups_of(c) {
        let t = tallies.entry(g).or_default();
        t.blocks += 1;
        if wet {
            t.wet_blocks += 1;
            t.sites += sites;
            t.wet_probes += probes;
            t.wet_pool += inside;
        }
        if !marsh {
            t.dry_probes += probes;
            t.dry_pool += inside;
        }
    }
    Ok(())
}

/// Walks inland from a sea-touching cell and counts the cells whose coast
/// strip stays above one half.
fn strip_width(src: &dyn Source, k: CellKey) -> Result<usize, RefineError> {
    // Inland is away from the first sea neighbour in row-major order.
    let mut away = (0, 0);
    'find: for dy in -1..=1_i64 {
        for dx in -1..=1_i64 {
            if (dx, dy) != (0, 0)
                && src.cell(src.clamp(k.offset(dx, dy)))?.terrain == TerrainKind::Sea
            {
                away = (-dx, -dy);
                break 'find;
            }
        }
    }
    let class = coast_of(src, k)?.shore;
    let mut width = 0;
    let mut at = k;
    while width < WALK_MAX {
        let c = src.cell(at)?;
        let coast = coast_of(src, at)?;
        // The walk ends where another shore's class takes over.
        if c.terrain != TerrainKind::Land || coast.shore != class || coast_strip(&c, coast) <= 0.5 {
            break;
        }
        width += 1;
        at = at.offset(away.0, away.1);
        if src.clamp(at) != at {
            break;
        }
    }
    Ok(width)
}

/// Measures a world with one cell in every `stride` columns and rows (one
/// block in every `2 × stride` for the pools, and one coast cell in every
/// `stride`).
///
/// # Errors
/// A layer failed to load.
pub fn measure(src: &dyn Source, stride: i64) -> Result<Report, RefineError> {
    let stride = stride.max(1);
    let (w, h) = src.cells_wide_high();
    let mut tallies: BTreeMap<&'static str, Tally> = BTreeMap::new();
    let mut coast_cells: BTreeMap<&'static str, (u64, u64, u64, Vec<u64>)> = BTreeMap::new();
    let seed = src.seed();
    for y in (0..h).step_by(usize::try_from(stride).unwrap_or(1)) {
        for x in (0..w).step_by(usize::try_from(stride).unwrap_or(1)) {
            let k = CellKey::new(x, y);
            let c = src.cell(k)?;
            if c.terrain != TerrainKind::Land {
                continue;
            }
            let coast = coast_of(src, k)?;
            let (dom, shares) = dominant(&c, coast);
            let wet = f64::from(c.wetness) / 255.0;
            for g in groups_of(&c) {
                let t = tallies.entry(g).or_default();
                t.land += 1;
                t.marshy += u64::from(marshy(&c) > 0.5);
                t.coastal += u64::from(coast_strip(&c, coast) > 0.5);
                t.arid += u64::from(arid(&c) > 0.5);
                t.alpine += u64::from(alpine(&c) > 0.5);
                t.ground[dom] += 1;
                for (acc, v) in t.weight.iter_mut().zip(shares) {
                    *acc += v;
                }
                for (n, level) in t.wet.iter_mut().zip(WET_LEVELS) {
                    *n += u64::from(wet >= level);
                }
            }
            if (x / stride + y / stride) % 2 == 0 {
                sample_block(src, k, &c, &mut tallies)?;
            }
        }
    }
    // Coast cells: every sea-touching land cell whose hash falls in 1/stride.
    let hits = |x: i64, y: i64| {
        hash2(seed, 0xC0A5, x, y).is_multiple_of(u64::try_from(stride).unwrap_or(1))
    };
    for y in 0..h {
        for x in 0..w {
            if !hits(x, y) {
                continue;
            }
            let k = CellKey::new(x, y);
            let c = src.cell(k)?;
            if c.terrain != TerrainKind::Land {
                continue;
            }
            let mut touches = false;
            for (dx, dy) in [
                (-1, -1),
                (0, -1),
                (1, -1),
                (-1, 0),
                (1, 0),
                (-1, 1),
                (0, 1),
                (1, 1),
            ] {
                touches |= src.cell(src.clamp(k.offset(dx, dy)))?.terrain == TerrainKind::Sea;
            }
            if !touches {
                continue;
            }
            let coast = coast_of(src, k)?;
            let name = if coast.layer {
                class_name(coast.shore)
            } else {
                "unclassified"
            };
            let width = strip_width(src, k)?;
            let e = coast_cells
                .entry(name)
                .or_insert_with(|| (0, 0, 0, vec![0; WALK_MAX + 1]));
            e.0 += 1;
            e.1 += u64::try_from(width).unwrap_or(0);
            e.2 += u64::from(dominant(&c, coast).0 == Class::Sand.index());
            e.3[width] += 1;
        }
    }
    let total: u64 = coast_cells.values().map(|e| e.0).sum();
    let groups = tallies
        .into_iter()
        .map(|(name, t)| {
            let ground_pct = (0..COUNT)
                .filter(|&i| t.ground[i] > 0)
                .filter_map(|i| Class::from_index(i).map(|c| (c, t.ground[i])))
                .map(|(c, n)| (c.ground_key().to_owned(), pct(n, t.land)))
                .collect();
            #[allow(clippy::cast_precision_loss)]
            let weight_pct = (0..COUNT)
                .filter_map(|i| Class::from_index(i).map(|c| (c, t.weight[i])))
                .filter(|(_, v)| *v > 0.0)
                .map(|(c, v)| {
                    (
                        c.ground_key().to_owned(),
                        round3(v * 100.0 / t.land.max(1) as f64),
                    )
                })
                .collect();
            let km2 = |n: u64| {
                #[allow(clippy::cast_precision_loss)]
                let v = n as f64 * 0.01;
                v
            };
            let per_km2 = if t.wet_blocks == 0 {
                0.0
            } else {
                round3(t.sites as f64 / km2(t.wet_blocks))
            };
            let g = Group {
                land_cells: t.land,
                marshy_pct: pct(t.marshy, t.land),
                coastal_pct: pct(t.coastal, t.land),
                arid_pct: pct(t.arid, t.land),
                alpine_pct: pct(t.alpine, t.land),
                wetness_ge_pct: WET_LEVELS
                    .iter()
                    .zip(t.wet)
                    .map(|(l, n)| (format!("{l:.1}"), pct(n, t.land)))
                    .collect(),
                ground_pct,
                weight_pct,
                pools: PoolStats {
                    blocks: t.blocks,
                    wet_blocks: t.wet_blocks,
                    pools_per_km2_wet: per_km2,
                    pool_area_pct_wet: pct(t.wet_pool, t.wet_probes),
                    pool_area_pct_outside_marsh: pct(t.dry_pool, t.dry_probes),
                },
            };
            (name.to_owned(), g)
        })
        .collect();
    let coast_class_pct = coast_cells
        .iter()
        .map(|(n, e)| ((*n).to_owned(), pct(e.0, total)))
        .collect();
    let strip = coast_cells
        .into_iter()
        .map(|(n, (cells, sum, sand, hist))| {
            #[allow(clippy::cast_precision_loss)]
            let mean = round3(sum as f64 / cells.max(1) as f64);
            (
                n.to_owned(),
                Strip {
                    cells,
                    mean_width: mean,
                    width_hist: hist,
                    sand_ground_pct: pct(sand, cells),
                },
            )
        })
        .collect();
    Ok(Report {
        seed,
        cells: (w, h),
        has_shore_layer: src.has_shore(),
        stride,
        groups,
        coast_class_pct,
        strip,
    })
}
