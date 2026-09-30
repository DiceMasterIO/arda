//! Numeric realism checks for one saved world (goal brief §7.3).
//! Usage: cargo run -p arda --release --example world_metrics -- WORLD
//!
//! Prints one JSON object: land/sea/lake cells, lake count and small-lake
//! count, watercourse density, the share of channel length in grid-straight
//! runs of 3 km or more, bed pits, land hypsometry and slope percentiles.
//! Worlds with a shore layer add a `"coast"` object (shore classes, where
//! beaches and cliffs sit, islands by cause; barrier and delta islands are
//! counted apart, since they are neither bays nor headlands) and every world a `"plains"`
//! object (slopes on locally flat ground and on river terraces). Lakes
//! take their origin from the stored water forms (`areas/*/water.bin`)
//! where the world has them; only lakes without one fall back to the
//! audited landforms.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::needless_range_loop
)]

use arda::{IslandCause, ShoreClass, ShoreLayer, TerrainKind, World};
use arda_core::water::LakeOrigin;
use arda_core::CellCoord;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::Path;

fn pct(sorted: &[i64], p: usize) -> i64 {
    if sorted.is_empty() {
        return 0;
    }
    sorted[(sorted.len() - 1) * p / 100]
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = std::env::args_os()
        .nth(1)
        .ok_or("usage: world_metrics WORLD")?;
    let world = World::load(Path::new(&root))?;
    let (mut land, mut sea, mut lake, mut water) = (0_u64, 0_u64, 0_u64, 0_u64);
    let mut pits = 0_u64;
    let mut heights = Vec::new();
    let mut slopes = Vec::new();
    let mut low_slopes = Vec::new();
    let mut rain_sum = 0_u64;
    let mut lake_cells: BTreeMap<u64, u64> = BTreeMap::new();
    let mut edges = BTreeMap::new();
    let mut lake_points: BTreeMap<u64, Vec<(i64, i64)>> = BTreeMap::new();
    let mut lake_origin: BTreeMap<u64, LakeOrigin> = BTreeMap::new();
    let (gw, gh) = (
        usize::try_from(world.manifest().areas_wide)? * 512,
        usize::try_from(world.manifest().areas_high)? * 512,
    );
    let mut grid = Grid {
        w: gw,
        h: gh,
        z: vec![0; gw * gh],
        kind: vec![0; gw * gh],
        slope: vec![0; gw * gh],
        order: vec![0; gw * gh],
        hand_dm: vec![0; gw * gh],
    };
    for (ax, ay) in world.area_coords() {
        let area = world.read_area(ax, ay)?;
        let cells = area.cells();
        let at = |x: i32, y: i32| CellCoord::new(x as u16, y as u16).map(|c| cells.get(c));
        for y in 0..512_i32 {
            for x in 0..512_i32 {
                let c = at(x, y).ok_or("cell coordinate")?;
                let g = (usize::try_from(ay)? * 512 + y as usize) * gw
                    + usize::try_from(ax)? * 512
                    + x as usize;
                grid.z[g] = c.height.raw();
                grid.kind[g] = c.terrain as u8;
                grid.slope[g] = c.slope_milli_deg;
                grid.order[g] = c.watercourse_order;
                grid.hand_dm[g] = c.height_above_river_dm;
                match c.terrain {
                    TerrainKind::Sea => sea += 1,
                    TerrainKind::Lake => lake += 1,
                    TerrainKind::Land => {
                        land += 1;
                        heights.push(i64::from(c.height.raw()));
                        slopes.push(i64::from(c.slope_milli_deg));
                        rain_sum += u64::from(c.rainfall.raw());
                        if c.height.raw() < 200_000 {
                            low_slopes.push(i64::from(c.slope_milli_deg));
                        }
                        water += u64::from(c.watercourse_order > 0);
                        if (1..511).contains(&x) && (1..511).contains(&y) {
                            let lower = (-1..=1).any(|dy| {
                                (-1..=1).any(|dx| {
                                    (dx, dy) != (0, 0)
                                        && at(x + dx, y + dy).is_some_and(|n| n.height < c.height)
                                })
                            });
                            pits += u64::from(!lower);
                        }
                    }
                }
            }
        }
        if let Some(forms) = area.water() {
            for (l, f) in area.lakes().iter().zip(&forms.lakes) {
                if f.origin != LakeOrigin::Unclassified {
                    lake_origin.insert(hash_id(&format!("{:?}", l.global_id)), f.origin);
                }
            }
        }
        for l in area.lakes() {
            let (ox, oy) = (i64::from(ax) * 512, i64::from(ay) * 512);
            for c in &l.cells {
                lake_points
                    .entry(hash_id(&format!("{:?}", l.global_id)))
                    .or_default()
                    .push((ox + i64::from(c.x()), oy + i64::from(c.y())));
            }
            *lake_cells
                .entry(hash_id(&format!("{:?}", l.global_id)))
                .or_default() += l.cells.len() as u64;
        }
        for e in area.channel_edges() {
            edges.insert((e.from.x, e.from.y), (e.to.x, e.to.y));
        }
    }
    // Straight runs: maximal chains of identical D8 steps; 3 km = 30 steps.
    let heads: BTreeSet<_> = edges.keys().copied().collect();
    let mut straight = 0_u64;
    let mut visited = BTreeSet::new();
    for &start in &heads {
        if visited.contains(&start) {
            continue;
        }
        let Some(&next) = edges.get(&start) else {
            continue;
        };
        let dir = (
            i64::from(next.0) - i64::from(start.0),
            i64::from(next.1) - i64::from(start.1),
        );
        // A chain starts where no donor arrives with the same step.
        let prev = (i64::from(start.0) - dir.0, i64::from(start.1) - dir.1);
        let prev_same = u32::try_from(prev.0)
            .ok()
            .zip(u32::try_from(prev.1).ok())
            .is_some_and(|p| edges.get(&p) == Some(&start));
        if prev_same {
            continue;
        }
        let mut n = 0_u64;
        let mut cur = start;
        while let Some(&t) = edges.get(&cur) {
            let d = (
                i64::from(t.0) - i64::from(cur.0),
                i64::from(t.1) - i64::from(cur.1),
            );
            if d != dir || !visited.insert(cur) {
                break;
            }
            n += 1;
            cur = t;
        }
        if n >= 30 {
            straight += n;
        }
    }
    heights.sort_unstable();
    slopes.sort_unstable();
    low_slopes.sort_unstable();
    let small = lake_cells.values().filter(|&&n| n <= 3).count();
    let shore = world.shore()?;
    let extra = format!(
        ",\"coast\":{},\"landforms\":{},\"plains\":{}",
        coast_json(&grid, shore.as_ref()),
        landforms_json(shore.as_ref(), &lake_points, &lake_origin),
        plains_json(&grid)
    );
    println!(
        "{{\"land_cells\":{land},\"sea_cells\":{sea},\"lake_cells\":{lake},\"lake_permille_of_land\":{},\"lakes\":{},\"lakes_le_3_cells\":{small},\"watercourse_permille_of_land\":{},\"channel_edges\":{},\"straight_run_permille\":{},\"bed_pits\":{pits},\"height_m_p10_50_90_99\":[{},{},{},{}],\"slope_deg_p50_90_99\":[{:.1},{:.1},{:.1}],\"lowland_lt200m_slope_deg_p50_90\":[{:.1},{:.1}],\"mean_land_rain_mm\":{}{extra}}}",
        lake * 1000 / land.max(1),
        lake_cells.len(),
        water * 1000 / land.max(1),
        edges.len(),
        straight * 1000 / (edges.len() as u64).max(1),
        pct(&heights, 10) / 1000,
        pct(&heights, 50) / 1000,
        pct(&heights, 90) / 1000,
        pct(&heights, 99) / 1000,
        pct(&slopes, 50) as f64 / 1000.0,
        pct(&slopes, 90) as f64 / 1000.0,
        pct(&slopes, 99) as f64 / 1000.0,
        pct(&low_slopes, 50) as f64 / 1000.0,
        pct(&low_slopes, 90) as f64 / 1000.0,
        rain_sum / land.max(1),
    );
    Ok(())
}

fn hash_id(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
    })
}

/// All area cells on one global 100 m grid.
struct Grid {
    w: usize,
    h: usize,
    z: Vec<i32>,
    kind: Vec<u8>,
    slope: Vec<u16>,
    order: Vec<u8>,
    hand_dm: Vec<u16>,
}

impl Grid {
    fn sea(&self, i: usize) -> bool {
        self.kind[i] == TerrainKind::Sea as u8
    }

    /// Box sum of `f` over radius `r` (clamped), via a summed-area table.
    fn box_mean(&self, f: impl Fn(usize) -> i64, r: usize) -> Vec<i64> {
        let (w, h) = (self.w, self.h);
        let mut sat = vec![0_i64; (w + 1) * (h + 1)];
        for y in 0..h {
            for x in 0..w {
                sat[(y + 1) * (w + 1) + x + 1] =
                    f(y * w + x) + sat[y * (w + 1) + x + 1] + sat[(y + 1) * (w + 1) + x]
                        - sat[y * (w + 1) + x];
            }
        }
        (0..w * h)
            .map(|i| {
                let (x, y) = (i % w, i / w);
                let (x0, y0, x1, y1) = (
                    x.saturating_sub(r),
                    y.saturating_sub(r),
                    (x + r + 1).min(w),
                    (y + r + 1).min(h),
                );
                let s = sat[y1 * (w + 1) + x1] - sat[y0 * (w + 1) + x1] - sat[y1 * (w + 1) + x0]
                    + sat[y0 * (w + 1) + x0];
                s / ((x1 - x0) * (y1 - y0)) as i64
            })
            .collect()
    }

    fn near(&self, i: usize, pred: impl Fn(usize) -> bool) -> bool {
        let (x, y) = (i % self.w, i / self.w);
        (y.saturating_sub(1)..(y + 2).min(self.h))
            .any(|yy| (x.saturating_sub(1)..(x + 2).min(self.w)).any(|xx| pred(yy * self.w + xx)))
    }

    /// Highest height within two cells, metres.
    fn rise_m(&self, i: usize) -> i64 {
        let (x, y) = (i % self.w, i / self.w);
        let mut top = 0;
        for yy in y.saturating_sub(2)..(y + 3).min(self.h) {
            for xx in x.saturating_sub(2)..(x + 3).min(self.w) {
                top = top.max(self.z[yy * self.w + xx]);
            }
        }
        i64::from(top) / 1000
    }
}

/// Shore classes, where beaches and cliffs sit, and islands by cause.
fn coast_json(g: &Grid, shore: Option<&ShoreLayer>) -> String {
    let Some(shore) = shore else {
        return "null".into();
    };
    let class = |i: usize| {
        let (x, y) = ((i % g.w) as u32, (i / g.w) as u32);
        shore.class_at(x, y)
    };
    let share = g.box_mean(|i| if g.sea(i) { 1000 } else { 0 }, 10);
    let built = built_islands(g, shore);
    let mut counts = BTreeMap::new();
    let (mut bay_counts, mut head_counts) = (BTreeMap::new(), BTreeMap::new());
    let (mut bay, mut bay_beach, mut head, mut head_beach) = (0_u64, 0_u64, 0_u64, 0_u64);
    let (mut built_shore, mut built_beach) = (0_u64, 0_u64);
    let (mut cliff_rise, mut beach_rise) = (Vec::new(), Vec::new());
    let (mut mouths, mut mouth_soft) = (0_u64, 0_u64);
    for i in 0..g.w * g.h {
        if g.kind[i] != TerrainKind::Land as u8 || !g.near(i, |j| g.sea(j)) {
            continue;
        }
        let c = class(i);
        *counts.entry(format!("{c:?}")).or_insert(0_u64) += 1;
        if built[i] {
            // Barrier and delta islands are sand bodies waves and rivers
            // built: neither bays nor headlands of the coast.
            built_shore += 1;
            built_beach += u64::from(c.is_beach());
        } else if share[i] < 450 {
            bay += 1;
            bay_beach += u64::from(c.is_beach());
            *bay_counts.entry(format!("{c:?}")).or_insert(0_u64) += 1;
        } else if share[i] > 550 {
            head += 1;
            head_beach += u64::from(c.is_beach());
            *head_counts.entry(format!("{c:?}")).or_insert(0_u64) += 1;
        }
        match c {
            ShoreClass::Cliff => cliff_rise.push(g.rise_m(i)),
            ShoreClass::SandBeach | ShoreClass::ShingleBeach => beach_rise.push(g.rise_m(i)),
            _ => {}
        }
        if g.order[i] >= 2 {
            mouths += 1;
            let soft = |j: usize| {
                matches!(
                    class(j),
                    ShoreClass::SandBeach
                        | ShoreClass::ShingleBeach
                        | ShoreClass::Marsh
                        | ShoreClass::TidalFlat
                        | ShoreClass::Estuary
                )
            };
            mouth_soft += u64::from(g.near(i, soft));
        }
    }
    cliff_rise.sort_unstable();
    beach_rise.sort_unstable();
    let mut causes = BTreeMap::new();
    for island in &shore.islands {
        *causes.entry(format!("{:?}", island.cause)).or_insert(0_u64) += 1;
    }
    let map = |m: &BTreeMap<String, u64>| {
        m.iter()
            .map(|(k, v)| format!("\"{k}\":{v}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "{{\"shore_cells\":{{{}}},\"beach_permille_in_bays\":{},\"beach_permille_on_headlands\":{},\"beach_permille_on_built_islands\":{},\"bay_classes\":{{{}}},\"headland_classes\":{{{}}},\"cliff_rise_m_p50\":{},\"beach_rise_m_p50\":{},\"river_mouths\":{mouths},\"river_mouths_soft_shore_permille\":{},\"islands\":{{{}}}}}",
        map(&counts),
        bay_beach * 1000 / bay.max(1),
        head_beach * 1000 / head.max(1),
        built_beach * 1000 / built_shore.max(1),
        map(&bay_counts),
        map(&head_counts),
        pct(&cliff_rise, 50),
        pct(&beach_rise, 50),
        mouth_soft * 1000 / mouths.max(1),
        map(&causes),
    )
}

/// Cells of land components the shore census attributes to barrier or
/// delta building: each component smaller than 200,000 cells is matched to
/// the census island with the nearest centroid (within 3 cells).
fn built_islands(g: &Grid, shore: &ShoreLayer) -> Vec<bool> {
    let n = g.w * g.h;
    let land = |i: usize| g.kind[i] == TerrainKind::Land as u8;
    let mut out = vec![false; n];
    let mut seen = vec![false; n];
    let cell_um = i64::from(shore.spacing_um);
    for s0 in 0..n {
        if seen[s0] || !land(s0) {
            continue;
        }
        let (mut stack, mut comp) = (vec![s0], Vec::new());
        seen[s0] = true;
        while let Some(c) = stack.pop() {
            comp.push(c);
            let (x, y) = (c % g.w, c / g.w);
            for yy in y.saturating_sub(1)..(y + 2).min(g.h) {
                for xx in x.saturating_sub(1)..(x + 2).min(g.w) {
                    let k = yy * g.w + xx;
                    if !seen[k] && land(k) {
                        seen[k] = true;
                        stack.push(k);
                    }
                }
            }
        }
        if comp.len() >= 200_000 {
            continue;
        }
        let m = comp.len() as i64;
        let cx = comp.iter().map(|&c| (c % g.w) as i64).sum::<i64>() / m;
        let cy = comp.iter().map(|&c| (c / g.w) as i64).sum::<i64>() / m;
        let cause = shore
            .islands
            .iter()
            .map(|isl| {
                let (dx, dy) = (isl.x_um / cell_um - cx, isl.y_um / cell_um - cy);
                (dx * dx + dy * dy, isl.cause)
            })
            .min_by_key(|&(d, _)| d)
            .filter(|&(d, _)| d <= 9)
            .map(|(_, c)| c);
        if matches!(cause, Some(IslandCause::Barrier | IslandCause::Delta)) {
            for c in comp {
                out[c] = true;
            }
        }
    }
    out
}

/// Slopes on locally flat land (2.5 km height deviation under 30 m) and
/// the share of flat treads on valley sides 2-40 m above their river.
fn plains_json(g: &Grid) -> String {
    let land = |i: usize| g.kind[i] == TerrainKind::Land as u8;
    let m = |i: usize| i64::from(g.z[i].max(0)) / 1000;
    let mean = g.box_mean(m, 25);
    let mean_sq = g.box_mean(|i| m(i) * m(i), 25);
    let mut plain = Vec::new();
    let (mut side, mut tread) = (0_u64, 0_u64);
    for i in 0..g.w * g.h {
        if !land(i) {
            continue;
        }
        let var = mean_sq[i] - mean[i] * mean[i];
        if var < 900 {
            plain.push(i64::from(g.slope[i]));
            if (20..400).contains(&g.hand_dm[i]) {
                side += 1;
                tread += u64::from(g.slope[i] < 1_000);
            }
        }
    }
    plain.sort_unstable();
    format!(
        "{{\"plain_cells\":{},\"plain_slope_deg_p50_90\":[{:.2},{:.2}],\"valley_side_cells\":{side},\"valley_side_tread_permille\":{}}}",
        plain.len(),
        pct(&plain, 50) as f64 / 1000.0,
        pct(&plain, 90) as f64 / 1000.0,
        tread * 1000 / side.max(1),
    )
}

/// Audited basins, plateaus and troughs by kind and cause, lakes by
/// stored origin, and the lakes that neither a stored origin nor an
/// audited basin or glacial trough explains.
fn landforms_json(
    shore: Option<&ShoreLayer>,
    lakes: &BTreeMap<u64, Vec<(i64, i64)>>,
    origins: &BTreeMap<u64, LakeOrigin>,
) -> String {
    let Some(shore) = shore else {
        return "null".into();
    };
    let mut by = BTreeMap::new();
    for f in &shore.landforms {
        *by.entry(format!("{:?}/{:?}", f.kind, f.cause))
            .or_insert(0_u64) += 1;
    }
    // A lake is explained when an audited basin or trough lies within
    // max(20 km, basin radius) of any of its cells.
    let mut by_origin: BTreeMap<String, u64> = BTreeMap::new();
    for o in origins.values() {
        *by_origin.entry(format!("{o:?}")).or_default() += 1;
    }
    let unexplained = lakes
        .iter()
        .filter(|(id, _)| !origins.contains_key(id))
        .map(|(_, cells)| cells)
        .filter(|cells| {
            !shore.landforms.iter().any(|f| {
                let reach_cells = (20_000 + i64::from(f.area_km2).isqrt() * 1_000) / 100;
                let (fx, fy) = (f.x_um / 100_000_000, f.y_um / 100_000_000);
                cells.iter().any(|&(x, y)| {
                    (x - fx) * (x - fx) + (y - fy) * (y - fy) <= reach_cells * reach_cells
                })
            })
        })
        .count();
    let mut parts: Vec<String> = by.iter().map(|(k, v)| format!("\"{k}\":{v}")).collect();
    parts.push(format!("\"lakes\":{}", lakes.len()));
    let origin_parts: Vec<String> = by_origin
        .iter()
        .map(|(k, v)| format!("\"{k}\":{v}"))
        .collect();
    parts.push(format!("\"lake_origins\":{{{}}}", origin_parts.join(",")));
    parts.push(format!("\"lakes_unexplained\":{unexplained}"));
    format!("{{{}}}", parts.join(","))
}
