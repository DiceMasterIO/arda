//! The review's §2 tactical measurements on a generated world (review
//! 2026-10-14 §2, 3.4-2, 3.4-7, 3.4-8, 3.4-9): rock against world forest on
//! steep cells, tactical snow near 0 °C, bank reshaping beside rivers and the
//! offset of the tactical channel from the formed thalweg. It asserts
//! nothing; it prints the numbers the changelog reports. Ignored by default
//! because it needs a world; run with
//! `ARDA_MEASURE_WORLD=<dir> cargo test -p arda-refine --release --test review_measures -- --ignored --nocapture`.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::unwrap_used,
    clippy::expect_used,
    missing_docs,
    clippy::cast_precision_loss
)]

use arda::{Cover, TerrainKind};
use arda_refine::context::{Ctx, SQUARE_M};
use arda_refine::scatter::Kind;
use arda_refine::{refine, CellKey, Source, WorldSource};

fn world() -> Option<arda::World> {
    let dir = std::env::var("ARDA_MEASURE_WORLD").ok()?;
    Some(arda::World::load(std::path::Path::new(&dir)).unwrap())
}

/// A keyed hash for picking a deterministic sample.
fn pick(x: i64, y: i64) -> u64 {
    let mut v = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    v ^= v >> 31;
    v.wrapping_mul(0xBF58_476D_1CE4_E5B9) >> 40
}

fn natural(c: &arda::Cell) -> bool {
    c.terrain == TerrainKind::Land && c.built_by == 0 && c.road == arda::RoadClass::None
}

fn percentile(v: &mut [f64], p: f64) -> f64 {
    if v.is_empty() {
        return f64::NAN;
    }
    v.sort_by(f64::total_cmp);
    v[((v.len() - 1) as f64 * p).round() as usize]
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

/// The world's snow fraction of a cell (the Atlas and contract rule at
/// cell scale: -4 → -7 °C, steep ground sheds a share).
fn world_snow(c: &arda::Cell) -> f64 {
    let s = |e0: f64, e1: f64, x: f64| {
        let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let t = f64::from(c.temperature.raw()) / 100.0;
    let slope = f64::from(c.slope_milli_deg) / 1000.0;
    s(-4.0, -7.0, t) * (1.0 - 0.75 * s(35.0, 52.4, slope))
}

#[test]
#[ignore = "needs a generated world"]
fn review_section_2_measurements() {
    let Some(w) = world() else {
        println!("ARDA_MEASURE_WORLD unset; nothing measured");
        return;
    };
    let src = WorldSource::new(&w).unwrap();
    let (cw, ch) = src.cells_wide_high();
    // Buckets: slope bands for 3.4-2, temperature bands for snow.
    let bands = [(24.0, 28.0), (28.0, 34.0), (34.0, 38.0), (38.0, 90.0)];
    let mut steep: Vec<Vec<CellKey>> = vec![Vec::new(); bands.len()];
    let temps = [(-0.5, 0.5), (-2.5, -1.5), (-4.5, -3.5), (-6.5, -5.5)];
    let mut cold: Vec<Vec<CellKey>> = vec![Vec::new(); temps.len()];
    let mut rivers = Vec::new();
    let (mut land, mut steep_land, mut steep_forest) = (0u64, 0u64, 0u64);
    for y in (0..ch).step_by(2) {
        for x in (0..cw).step_by(2) {
            let k = CellKey::new(x, y);
            let c = src.cell(k).unwrap();
            if c.terrain != TerrainKind::Land {
                continue;
            }
            land += 1;
            let slope = f64::from(c.slope_milli_deg) / 1000.0;
            if slope >= 24.0 {
                steep_land += 1;
                steep_forest += u64::from(c.cover == Cover::Forest);
            }
            if !natural(&c) || x < 3 || y < 3 || x >= cw - 3 || y >= ch - 3 {
                continue;
            }
            if let Some(b) = bands.iter().position(|&(a, z)| slope >= a && slope < z) {
                steep[b].push(k);
            }
            let t = f64::from(c.temperature.raw()) / 100.0;
            if let Some(b) = temps.iter().position(|&(a, z)| t >= a && t <= z) {
                cold[b].push(k);
            }
            if c.watercourse_order > 0 {
                rivers.push(k);
            }
        }
    }
    println!(
        "land cells (stride 2) {land}; >= 24 deg {:.1} %, of which world forest {:.1} %",
        100.0 * steep_land as f64 / land.max(1) as f64,
        100.0 * steep_forest as f64 / steep_land.max(1) as f64
    );
    let thin = |v: &mut Vec<CellKey>, n: usize| {
        v.sort_by_key(|k| (pick(k.x, k.y), k.y, k.x));
        v.truncate(n);
    };
    // 3.4-2: rock, scree and cliff squares against world forest.
    println!("-- steep natural cells: tactical rock/scree/cliff share, trees per block");
    for (b, cells) in steep.iter_mut().enumerate() {
        thin(cells, 150);
        let (mut forest, mut fd, mut rock_all, mut rock_forest, mut trees_forest) =
            (0, Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for &k in cells.iter() {
            let c = src.cell(k).unwrap();
            let blk = refine(&src, k).unwrap();
            let rock = blk
                .ground
                .iter()
                .filter(|g| matches!(**g, "rock" | "scree" | "cliff"))
                .count() as f64
                / 4096.0;
            let trees = blk
                .items
                .iter()
                .filter(|it| matches!(it.kind, Kind::TreeLarge | Kind::TreeSmall))
                .count() as f64;
            rock_all.push(rock);
            fd.push(f64::from(c.forest_density) / 255.0);
            if c.cover == Cover::Forest {
                forest += 1;
                rock_forest.push(rock);
                trees_forest.push(trees);
            }
        }
        let (a, z) = bands[b];
        let max_f = rock_forest.iter().copied().fold(0.0, f64::max);
        println!(
            "  {a:>2}-{z:<2} deg: {} cells, world forest {forest}, mean fd {:.2}; rock all {:.1} %; on forest cells mean {:.1} % max {:.1} %, trees {:.0}",
            cells.len(),
            mean(&fd),
            100.0 * mean(&rock_all),
            100.0 * mean(&rock_forest),
            100.0 * max_f,
            mean(&trees_forest)
        );
    }
    // Snow near 0 °C (3.4-9).
    println!("-- tactical snow against world snow by cell temperature");
    for (b, cells) in cold.iter_mut().enumerate() {
        thin(cells, 60);
        let (mut tact, mut world_s) = (Vec::new(), Vec::new());
        for &k in cells.iter() {
            let c = src.cell(k).unwrap();
            let blk = refine(&src, k).unwrap();
            let snow = blk
                .ground
                .iter()
                .filter(|g| matches!(**g, "snow" | "ice"))
                .count() as f64
                / 4096.0;
            tact.push(snow);
            world_s.push(world_snow(&c));
        }
        let (a, z) = temps[b];
        let max = tact.iter().copied().fold(0.0, f64::max);
        println!(
            "  {a:+.1}..{z:+.1} C: {} cells, tactical snow mean {:.1} % max {:.1} %, world snow mean {:.2}",
            cells.len(),
            100.0 * mean(&tact),
            100.0 * max,
            mean(&world_s)
        );
    }
    // Banks (3.4-8) and channel offsets (3.4-7).
    thin(&mut rivers, 200);
    let (mut diff, mut shaped, mut reach, mut above) = (Vec::new(), 0u64, 0.0f64, 0u64);
    let mut dry_near = 0u64;
    let mut offsets = Vec::new();
    for &k in &rivers {
        let ctx = Ctx::gather(&src, k).unwrap();
        let pieces = arda_refine::rivers::pieces(&ctx);
        let (x0, y0) = (k.x * 64, k.y * 64);
        let phys = arda_refine::terrain::physical(&ctx, &pieces, x0, y0, 64);
        for (i, p) in phys.data.iter().enumerate() {
            let (sx, sy) = (x0 + (i % 64) as i64, y0 + (i / 64) as i64);
            let (u, v) = (sx as f64 + 0.5, sy as f64 + 0.5);
            if p.water != arda_refine::terrain::Water::Dry || !p.river_d.is_finite() {
                continue;
            }
            if p.stand_v <= -0.6 && (p.elev_m - p.land_m).abs() > 0.25 {
                shaped += 1;
                reach = reach.max(p.river_d);
            }
            if p.river_d > 8.0 {
                continue;
            }
            dry_near += 1;
            diff.push(p.elev_m - ctx.base_height(u, v));
        }
        // Water standing above dry land within two squares of the bank.
        for (i, p) in phys.data.iter().enumerate() {
            if p.water != arda_refine::terrain::Water::River {
                continue;
            }
            let (ix, iy) = ((i % 64) as i64, (i / 64) as i64);
            let low = (-2..=2)
                .flat_map(|dy| (-2..=2).map(move |dx| (dx, dy)))
                .filter_map(|(dx, dy)| phys.get(x0 + ix + dx, y0 + iy + dy))
                .filter(|q| q.water == arda_refine::terrain::Water::Dry)
                .any(|q| q.elev_m + 0.05 < p.elev_m);
            above += u64::from(low);
        }
        // Thalweg offset: the formed thalweg against the drawn centreline.
        let (ax, ay) = (k.x.div_euclid(512), k.y.div_euclid(512));
        let area = w.area(ax as i32, ay as i32).unwrap();
        if let Some(t) = area.water().and_then(|wf| {
            wf.thalwegs.iter().find(|t| {
                i64::from(t.cell.x()) == k.x - ax * 512 && i64::from(t.cell.y()) == k.y - ay * 512
            })
        }) {
            let tx = x0 as f64 + 32.0 + f64::from(t.dx_dm) / 10.0 / SQUARE_M;
            let ty = y0 as f64 + 32.0 + f64::from(t.dy_dm) / 10.0 / SQUARE_M;
            let near = arda_refine::rivers::cell_pieces(&ctx, k)
                .iter()
                .flat_map(|p| p.pts.iter().copied().collect::<Vec<_>>())
                .map(|(x, y)| ((x - tx) * (x - tx) + (y - ty) * (y - ty)).sqrt())
                .fold(f64::INFINITY, f64::min);
            if near.is_finite() {
                offsets.push(near * SQUARE_M);
            }
        }
    }
    let worst = diff.iter().copied().fold(0.0, f64::min);
    let cut2 = diff.iter().filter(|d| **d < -2.0).count();
    println!(
        "-- banks: {} river cells, {dry_near} dry squares within 8 of a bank: tactical - fine mean {:+.2} m, worst {worst:+.1} m, {cut2} cut > 2 m; reshaped > 0.25 m: {shaped} squares, farthest {reach:.1} squares; river squares with drier land 5 cm lower within 2 squares: {above}",
        rivers.len(),
        mean(&diff)
    );
    let n = offsets.len();
    let med = percentile(&mut offsets, 0.5);
    let p90 = percentile(&mut offsets, 0.9);
    let max = offsets.iter().copied().fold(0.0, f64::max);
    println!("-- thalweg to tactical centreline: {n} cells, median {med:.1} m, p90 {p90:.1} m, max {max:.1} m");
}
