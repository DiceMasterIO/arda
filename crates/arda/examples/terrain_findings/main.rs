//! Terrain re-measurement tool (goal 19): parallel drainage (OI-03), angular
//! shorelines (OI-04) and rectangular basins (OI-05) on one saved world.
//!
//! Usage: `cargo run -p arda --release --example terrain_findings -- WORLD [--json out.json]`
//!
//! Prints deterministic JSON. Thresholds are constants in `drainage.rs`,
//! `coast.rs` and `basins.rs`, fixed against synthetic controls before any
//! real world was measured.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    dead_code
)]

mod basins;
mod coast;
mod drainage;
mod fft;
mod grid;
mod json;
#[cfg(test)]
mod synth;
mod trace;

use arda::World;
use arda_core::ShoreLayer;
use grid::{shares, Grid, LAND, SEA};
use json::J;
use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

/// Rotation of the control resample, degrees (deliberately not a multiple of 15).
const CONTROL_DEG: f64 = 17.0;

fn hist_json(h: &[u64; 24]) -> J {
    let s = shares(h);
    J::obj()
        .with("hist_24", J::counts(h))
        .with("samples", J::I(h.iter().sum::<u64>() as i64))
        .with("axis_share", J::F(s.axis))
        .with("diag_share", J::F(s.diag))
        .with("ratio_vs_uniform", J::F(s.ratio))
}

fn crop_size(g: &Grid) -> usize {
    (g.w.min(g.h) * 7 / 10).min(4096)
}

fn drainage_section(g: &Grid) -> (J, bool, J) {
    let full = drainage::descent_hist(g);
    let reach = drainage::reach_hist(g);
    let size = crop_size(g);
    let (cx, cy) = g.pick_crop(size, |i| g.kind[i] == LAND && g.z[i] > 800_000);
    let c0 = g.rotated_crop(cx, cy, size, 0.0);
    let c1 = g.rotated_crop(cx, cy, size, CONTROL_DEG);
    let (h0, h1) = (drainage::descent_hist(&c0), drainage::descent_hist(&c1));
    let rs = drainage::runs(g, 20);
    let sp = drainage::spectrum(g);
    let grid_bound = shares(&h0).ratio >= 1.5 && shares(&h1).ratio <= 1.25;
    let long = rs
        .iter()
        .filter(|r| r.cells >= drainage::LONG_RUN_CELLS)
        .count();
    let reproduced = sp.lattice() || long > 0 || grid_bound;
    let mut land_z: Vec<i32> =
        g.z.iter()
            .zip(&g.kind)
            .filter(|(_, &k)| k == LAND)
            .map(|(&z, _)| z)
            .collect();
    land_z.sort_unstable();
    let pz = |p: usize| {
        J::F(
            f64::from(
                land_z
                    .get(land_z.len().saturating_sub(1) * p / 100)
                    .copied()
                    .unwrap_or(0),
            ) / 1000.0,
        )
    };
    let worst = rs.iter().max_by(|a, b| {
        a.cells
            .cmp(&b.cells)
            .then(b.y.cmp(&a.y))
            .then(b.x.cmp(&a.x))
    });
    let worst_j = J::obj()
        .with(
            "spectral_window_centre_xy",
            sp.worst
                .map_or(J::A(vec![]), |(x, y, _)| J::A(vec![J::u(x), J::u(y)])),
        )
        .with(
            "longest_run_xy",
            worst.map_or(J::A(vec![]), |r| J::A(vec![J::u(r.x), J::u(r.y)])),
        );
    let j = J::obj()
        .with("descent_all_land", hist_json(&full))
        .with("descent_crop_unrotated", hist_json(&h0))
        .with("descent_crop_rotated_17deg", hist_json(&h1))
        .with("descent_grid_bound", J::B(grid_bound))
        .with("reach_chords_1km", hist_json(&reach))
        .with("straight_runs", drainage::runs_json(&rs))
        .with("valley_spectrum", sp.json())
        .with(
            "land_height_m_p50_p90_p99_max",
            J::A(vec![pz(50), pz(90), pz(99), pz(100)]),
        )
        .with("reproduced", J::B(reproduced));
    (j, reproduced, worst_j)
}

fn coast_section(g: &Grid) -> (J, bool, J) {
    let sea: Vec<bool> = g.kind.iter().map(|&k| k == SEA).collect();
    let loops = trace::trace(g.w, g.h, &sea);
    let acc = coast::measure(&loops, 40);
    let size = crop_size(g);
    let (cx, cy) = g.pick_crop(size, |i| {
        g.kind[i] == SEA && (i + 1 < g.kind.len()) && g.kind[i + 1] != SEA
    });
    let crop_acc = |deg: f64| {
        let c = g.rotated_crop(cx, cy, size, deg);
        let m: Vec<bool> = c.kind.iter().map(|&k| k == SEA).collect();
        coast::measure(&trace::trace(c.w, c.h, &m), 40)
    };
    let (c0, c1) = (crop_acc(0.0), crop_acc(CONTROL_DEG));
    let angular = acc.angular(Some(&c1));
    let j = coast::Acc::json(&acc, Some(&c1))
        .with("crop_unrotated_ratio_5km", J::F(c0.ratio(1)))
        .with("crop_unrotated_edges", J::I(c0.edges as i64))
        .with("crop_rotated_edges", J::I(c1.edges as i64));
    let worst = J::obj().with(
        "longest_run_xy",
        J::A(vec![
            J::I(i64::from(acc.longest_at.0)),
            J::I(i64::from(acc.longest_at.1)),
        ]),
    );
    (j, angular, worst)
}

fn lake_shore_section(g: &Grid) -> (J, bool) {
    let m: Vec<bool> = g.kind.iter().map(|&k| k == grid::LAKE).collect();
    let loops = trace::trace(g.w, g.h, &m);
    let acc = coast::measure(&loops, 40);
    let angular = acc.angular(None);
    (acc.json(None).with("loops", J::u(loops.len())), angular)
}

fn shore_classes(g: &Grid, shore: Option<&ShoreLayer>) -> J {
    let Some(sl) = shore else {
        return J::S("absent".into());
    };
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for y in 0..g.h {
        for x in 0..g.w {
            let i = y * g.w + x;
            if g.kind[i] != LAND {
                continue;
            }
            let near_sea = (x > 0 && g.kind[i - 1] == SEA)
                || (x + 1 < g.w && g.kind[i + 1] == SEA)
                || (y > 0 && g.kind[i - g.w] == SEA)
                || (y + 1 < g.h && g.kind[i + g.w] == SEA);
            if near_sea {
                let c = sl.class_at(x as u32, y as u32);
                *counts.entry(format!("{c:?}")).or_insert(0) += 1;
            }
        }
    }
    J::O(
        counts
            .into_iter()
            .map(|(k, v)| (k, J::I(v as i64)))
            .collect(),
    )
}

fn basin_section(g: &Grid, shore: Option<&ShoreLayer>) -> (J, bool, bool, J) {
    let ls = basins::lakes(g, basins::MIN_CELLS);
    let rect = ls.iter().any(basins::LakeReport::rectangular);
    let dams = ls.iter().any(basins::LakeReport::on_area_lines);
    let mut landforms = Vec::new();
    if let Some(sl) = shore {
        let s = i64::from(sl.spacing_um.max(1));
        for f in sl
            .landforms
            .iter()
            .filter(|f| f.kind == arda_core::LandformKind::Basin)
        {
            let (x, y) = (f.x_um / s, f.y_um / s);
            let wet = x >= 0
                && y >= 0
                && (x as usize) < g.w
                && (y as usize) < g.h
                && g.kind[y as usize * g.w + x as usize] == grid::LAKE;
            landforms.push(
                J::obj()
                    .with("area_km2", J::I(i64::from(f.area_km2)))
                    .with("relief_m", J::I(i64::from(f.relief_m)))
                    .with("cause", J::S(format!("{:?}", f.cause)))
                    .with("centroid_xy", J::A(vec![J::I(x), J::I(y)]))
                    .with("centroid_in_lake", J::B(wet)),
            );
        }
    }
    let worst = ls
        .iter()
        .max_by(|a, b| a.rect_fill.total_cmp(&b.rect_fill))
        .map_or(J::A(vec![]), |l| J::A(vec![J::u(l.cx), J::u(l.cy)]));
    let j = J::obj()
        .with("lakes_over_500km2", J::u(ls.len()))
        .with(
            "rectangular_lakes",
            J::u(ls.iter().filter(|l| l.rectangular()).count()),
        )
        .with(
            "lakes_with_sides_on_area_lines",
            J::u(ls.iter().filter(|l| l.on_area_lines()).count()),
        )
        .with(
            "lakes",
            J::A(
                ls.iter()
                    .take(40)
                    .enumerate()
                    .map(|(i, l)| l.json(i + 1))
                    .collect(),
            ),
        )
        .with("basin_landforms", J::A(landforms))
        .with(
            "tectonic_belt_edges",
            J::S("not measured: belt geometry is not saved in the world".into()),
        )
        .with("reproduced", J::B(rect || dams));
    (j, rect, dams, worst)
}

/// Runs every measure on a loaded grid.
pub fn analyse(g: &Grid, shore: Option<&ShoreLayer>) -> J {
    let (drain, d_flag, d_worst) = drainage_section(g);
    let (coast_j, c_flag, c_worst) = coast_section(g);
    let (lake_j, l_flag) = lake_shore_section(g);
    let (basin_j, rect, dams, b_worst) = basin_section(g, shore);
    J::obj()
        .with("grid_cells", J::A(vec![J::u(g.w), J::u(g.h)]))
        .with(
            "land_cells",
            J::u(g.kind.iter().filter(|&&k| k == LAND).count()),
        )
        .with(
            "lake_cells",
            J::u(g.kind.iter().filter(|&&k| k == grid::LAKE).count()),
        )
        .with(
            "channel_cells",
            J::u(g.order.iter().filter(|&&o| o > 0).count()),
        )
        .with("parallel_drainage", drain)
        .with("coast", coast_j)
        .with("lake_shores", lake_j)
        .with("shore_class_of_coast_land_cells", shore_classes(g, shore))
        .with("basins", basin_j)
        .with(
            "flags",
            J::obj()
                .with("oi03_parallel_drainage", J::B(d_flag))
                .with("oi04_angular_coast", J::B(c_flag))
                .with("oi04_angular_lake_shores", J::B(l_flag))
                .with("oi05_rectangular_lake", J::B(rect))
                .with("oi05_sides_on_area_lines", J::B(dams)),
        )
        .with(
            "worst",
            J::obj()
                .with("drainage", d_worst)
                .with("coast", c_worst)
                .with("basin_centre_xy", b_worst),
        )
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let root = args
        .next()
        .ok_or("usage: terrain_findings WORLD [--json out.json]")?;
    let mut json_out = None;
    while let Some(a) = args.next() {
        if a == "--json" {
            json_out = Some(args.next().ok_or("--json needs a path")?);
        } else {
            return Err(format!("unknown argument {}", a.to_string_lossy()).into());
        }
    }
    let world = World::load(Path::new(&root))?;
    let g = Grid::load(&world)?;
    let shore = world.shore()?;
    let m = world.manifest();
    let text = J::obj()
        .with("seed", J::I(i64::try_from(m.seed)?))
        .with(
            "areas",
            J::A(vec![
                J::I(i64::from(m.areas_wide)),
                J::I(i64::from(m.areas_high)),
            ]),
        )
        .with("format_version", J::I(i64::from(m.format_version)))
        .with("arda_version", J::S(m.arda_version.to_string()))
        .with("measures", analyse(&g, shore.as_ref()))
        .render();
    if let Some(p) = json_out {
        std::fs::write(p, &text)?;
    }
    print!("{text}");
    Ok(())
}
