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
/// cell scale) under the world's snow line at the cell's row.
fn world_snow(src: &WorldSource<'_>, k: CellKey, c: &arda::Cell) -> f64 {
    let frame = src.biome_frame();
    let lines = frame.lines(frame.latitude_mdeg(k.y));
    let t = f64::from(c.temperature.raw()) / 100.0;
    lines.snow(t, f64::from(c.slope_milli_deg) / 1000.0)
}

/// Horn and full-neighbourhood D8 slopes (degrees) of a cell from the
/// stored heights of its eight neighbours, across area edges.
fn slopes_across(src: &WorldSource<'_>, k: CellKey) -> (f64, f64) {
    let h = |dx: i64, dy: i64| f64::from(src.cell(k.offset(dx, dy)).unwrap().height.raw()) / 1000.0;
    let gx = (h(1, -1) + 2.0 * h(1, 0) + h(1, 1) - h(-1, -1) - 2.0 * h(-1, 0) - h(-1, 1)) / 800.0;
    let gy = (h(-1, 1) + 2.0 * h(0, 1) + h(1, 1) - h(-1, -1) - 2.0 * h(0, -1) - h(1, -1)) / 800.0;
    let horn = arda_refine::terrain::atan_deg((gx * gx + gy * gy).sqrt());
    let mut d8: f64 = 0.0;
    for (dx, dy) in [
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ] {
        let run = if dx != 0 && dy != 0 { 141.421 } else { 100.0 };
        d8 = d8.max((h(0, 0) - h(dx, dy)) / run);
    }
    (horn, arda_refine::terrain::atan_deg(d8.max(0.0)))
}

/// World-level checks of the review (3.2-5 seams, 3.2-7 aspect, 3.2-8 cold
/// cover, 3.2-16 wetness lines).
#[test]
#[ignore = "needs a generated world"]
fn review_world_measurements() {
    let Some(w) = world() else {
        println!("ARDA_MEASURE_WORLD unset; nothing measured");
        return;
    };
    let src = WorldSource::new(&w).unwrap();
    let (cw, ch) = src.cells_wide_high();
    // Seams: land cells on an area edge, stored slope against the slope
    // their neighbours across the edge give.
    let (mut seam, mut off_horn, mut off_d8, mut interior, mut in_off) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    for y in 1..ch - 1 {
        for x in 1..cw - 1 {
            let edge = x % 512 == 0 || x % 512 == 511 || y % 512 == 0 || y % 512 == 511;
            let inner = (x % 512 == 256) && y % 4 == 0;
            if !edge && !inner {
                continue;
            }
            let k = CellKey::new(x, y);
            let c = src.cell(k).unwrap();
            if c.terrain != TerrainKind::Land {
                continue;
            }
            let stored = f64::from(c.slope_milli_deg) / 1000.0;
            let (horn, d8) = slopes_across(&src, k);
            let best = (stored - horn).abs().min((stored - d8).abs());
            if edge {
                seam += 1;
                off_horn += u64::from((stored - horn).abs() > 0.5);
                off_d8 += u64::from((stored - d8).abs() > 0.5);
            } else {
                interior += 1;
                in_off += u64::from(best > 0.5);
            }
        }
    }
    println!(
        "-- seams: {seam} edge land cells, stored slope off the cross-edge Horn slope by > 0.5 deg {:.1} %, off the cross-edge D8 slope {:.1} %; interior control off both {:.1} % of {interior}",
        100.0 * off_horn as f64 / seam.max(1) as f64,
        100.0 * off_d8 as f64 / seam.max(1) as f64,
        100.0 * in_off as f64 / interior.max(1) as f64
    );
    // Aspect, cold cover and wetness, stride 2.
    let mut bearings = std::collections::BTreeSet::new();
    let (mut flat, mut north) = (0u64, 0u64);
    let (mut cold, mut cold_grass, mut cold_forest, mut below4_forest, mut ice) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    let mut ice_min_m = f64::INFINITY;
    let (mut plain, mut lines) = (0u64, 0u64);
    for y in (2..ch - 2).step_by(2) {
        for x in (2..cw - 2).step_by(2) {
            let k = CellKey::new(x, y);
            let c = src.cell(k).unwrap();
            if c.terrain != TerrainKind::Land {
                continue;
            }
            bearings.insert(c.aspect_deg);
            flat += u64::from(c.aspect_deg == 0 && c.slope_milli_deg == 0);
            north += u64::from(c.aspect_deg == 360 || (c.aspect_deg == 0 && c.slope_milli_deg > 0));
            let t = c.temperature.raw();
            if t < -300 {
                cold += 1;
                cold_grass += u64::from(c.cover == Cover::Grass);
                cold_forest += u64::from(c.cover == Cover::Forest);
            }
            below4_forest += u64::from(t < -400 && c.cover == Cover::Forest);
            if c.cover == Cover::Ice {
                ice += 1;
                ice_min_m = ice_min_m.min(f64::from(c.height.raw()) / 1000.0);
            }
            if c.slope_milli_deg < 2_000 {
                plain += 1;
                let n = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .map(|(dx, dy)| f64::from(src.cell(k.offset(dx, dy)).unwrap().wetness));
                let mean = n.iter().sum::<f64>() / 4.0;
                lines += u64::from((f64::from(c.wetness) - mean).abs() > 30.0);
            }
        }
    }
    println!(
        "-- aspect: {} distinct stored values; flat {flat}, north-facing {north}",
        bearings.len()
    );
    println!(
        "-- cold cover: land < -3 C {cold}: grass {:.1} %, forest {:.1} %; forest < -4 C {below4_forest}; ice {ice} (lowest {ice_min_m:.0} m)",
        100.0 * cold_grass as f64 / cold.max(1) as f64,
        100.0 * cold_forest as f64 / cold.max(1) as f64
    );
    println!(
        "-- wetness on plains (< 2 deg): {plain} cells, {:.2} % differ from their 4-neighbour mean by > 30",
        100.0 * lines as f64 / plain.max(1) as f64
    );
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
            world_s.push(world_snow(&src, k, &c));
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
                .flat_map(|p| p.pts.clone())
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

/// Review 3.4-5: blocks whose WFC fell back to the relaxed fill, over a
/// sweep of every ninth land cell in both axes.
#[test]
#[ignore = "needs a generated world"]
fn review_relaxed_blocks() {
    let Some(w) = world() else {
        println!("ARDA_MEASURE_WORLD unset; nothing measured");
        return;
    };
    let src = WorldSource::new(&w).unwrap();
    let (cw, ch) = src.cells_wide_high();
    let mut cells = Vec::new();
    for y in (1..ch - 1).step_by(9) {
        for x in (1..cw - 1).step_by(9) {
            let k = CellKey::new(x, y);
            if src.cell(k).unwrap().terrain == TerrainKind::Land {
                cells.push(k);
            }
        }
    }
    let chunks: Vec<Vec<CellKey>> = cells
        .chunks(cells.len() / 4 + 1)
        .map(<[CellKey]>::to_vec)
        .collect();
    let (relaxed, illegal): (usize, usize) = std::thread::scope(|s| {
        let hs: Vec<_> = chunks
            .iter()
            .map(|part| {
                let src = &src;
                s.spawn(move || {
                    let (mut r, mut i) = (0, 0);
                    for &k in part {
                        let b = refine(src, k).unwrap();
                        if b.relaxed {
                            r += 1;
                            i += b.rules.iter().filter(|q| q.review).count();
                        }
                    }
                    (r, i)
                })
            })
            .collect();
        hs.into_iter()
            .map(|h| h.join().unwrap())
            .fold((0, 0), |a, b| (a.0 + b.0, a.1 + b.1))
    });
    println!(
        "-- relaxed blocks: {relaxed} of {} ({illegal} squares flagged for review)",
        cells.len()
    );
}
