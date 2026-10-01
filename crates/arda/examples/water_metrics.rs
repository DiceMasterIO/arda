//! Water-realism metrics for one saved world (goals 8-14, 21).
//! Usage: cargo run -p arda --release --example water_metrics -- WORLD
//!
//! Prints one JSON object:
//! - hydraulic geometry: least-squares exponents of stored bankfull width
//!   and depth against discharge over segments (targets 0.5 and 0.4), and
//!   the share of confluences where depth does not fall downstream;
//! - planform: segment length by pattern, and length-weighted mean
//!   sinuosity of rivers of at least 1 m³/s in slope bands (it should
//!   rise as slope falls);
//! - lakes by origin, terminal lakes, lakes of three cells or fewer, and
//!   lakes with an inflowing river and with an outlet;
//! - deltas, dolines and the closed annual water ledger.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use arda::World;
use arda_core::water::{ChannelPattern, LakeOrigin};
use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

fn fit(points: &[(f64, f64)]) -> f64 {
    let n = points.len() as f64;
    if n < 2.0 {
        return 0.0;
    }
    let (mx, my) = points
        .iter()
        .fold((0.0, 0.0), |(a, b), &(x, y)| (a + x / n, b + y / n));
    let (sxy, sxx) = points.iter().fold((0.0, 0.0), |(a, b), &(x, y)| {
        (a + (x - mx) * (y - my), b + (x - mx) * (x - mx))
    });
    sxy / sxx.max(1e-12)
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = std::env::args_os()
        .nth(1)
        .ok_or("usage: water_metrics WORLD")?;
    let world = World::load(Path::new(&root))?;
    let (mut wq, mut dq) = (Vec::new(), Vec::new());
    let (mut conf, mut conf_ok) = (0_u64, 0_u64);
    let mut pattern_cells: BTreeMap<&str, u64> = BTreeMap::new();
    // Slope bands (ppm upper bounds) → (Σ sinuosity × cells, Σ cells).
    let bands = [300_u32, 1_000, 3_000, 10_000, u32::MAX];
    let mut band_sum = [(0_u64, 0_u64); 5];
    let mut origins: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    let mut lake_cells: BTreeMap<u64, u64> = BTreeMap::new();
    let (mut deltas, mut dolines, mut terminal) = (0_u64, 0_u64, 0_u64);
    let mut have_forms = false;
    let (mut with_inlet, mut with_outlet) = (
        std::collections::BTreeSet::new(),
        std::collections::BTreeSet::new(),
    );
    for (ax, ay) in world.area_coords() {
        let area = world.read_area(ax, ay)?;
        let mut lake_at: BTreeMap<(u16, u16), u64> = BTreeMap::new();
        for l in area.lakes() {
            *lake_cells.entry(l.global_id.0).or_default() += l.cells.len() as u64;
            for c in &l.cells {
                lake_at.insert((c.x(), c.y()), l.global_id.0);
            }
            if l.outlet.is_some() {
                with_outlet.insert(l.global_id.0);
            }
        }
        // Inlets: rivers that end in a lake, next to one of its cells.
        for r in area.rivers() {
            if r.ends != arda_core::Terminus::Lake {
                continue;
            }
            let Some(last) = r.course.last() else {
                continue;
            };
            for dy in -1..=1_i32 {
                for dx in -1..=1_i32 {
                    let key = (
                        u16::try_from(i32::from(last.x()) + dx).unwrap_or(u16::MAX),
                        u16::try_from(i32::from(last.y()) + dy).unwrap_or(u16::MAX),
                    );
                    if let Some(&id) = lake_at.get(&key) {
                        with_inlet.insert(id);
                    }
                }
            }
        }
        // Planform measured on the saved courses, with or without forms.
        let cells = area.cells();
        for (r, win) in area
            .rivers()
            .iter()
            .zip(arda_core::water::course_windows(area.rivers()))
        {
            // Rivers of at least 1 m³/s: the reaches wide enough to meander
            // at the 100 m world scale.
            if r.discharge.raw() < 1_000 {
                continue;
            }
            let n = r.course.len() as u64;
            let drop = i64::from(cells.get(win.first).height.raw())
                - i64::from(cells.get(win.last).height.raw());
            let slope =
                u32::try_from(drop.max(0) * 10_000 / win.run_permille.max(1)).unwrap_or(u32::MAX);
            if let Some(b) = bands.iter().position(|&b| slope < b) {
                band_sum[b].0 += u64::from(win.sinuosity_permille) * n;
                band_sum[b].1 += n;
            }
        }
        let Some(w) = area.water() else { continue };
        have_forms = true;
        let rivers = area.rivers();
        for (r, f) in rivers.iter().zip(&w.segments) {
            let q = r.discharge.raw() as f64 / 1000.0;
            if q > 0.04 {
                wq.push((q.ln(), f64::from(f.bankfull_width_dm).ln()));
                dq.push((q.ln(), f64::from(f.bankfull_depth_cm.max(1)).ln()));
            }
            let name = match f.pattern {
                ChannelPattern::Straight => "straight",
                ChannelPattern::Meandering => "meandering",
                ChannelPattern::Braided => "braided",
            };
            let n = r.course.len() as u64;
            *pattern_cells.entry(name).or_default() += n;
            if let Some(down) = r
                .feeds
                .and_then(|id| rivers.iter().position(|d| d.id == id))
            {
                conf += 1;
                conf_ok += u64::from(w.segments[down].bankfull_depth_cm >= f.bankfull_depth_cm);
            }
        }
        for (l, f) in area.lakes().iter().zip(&w.lakes) {
            let o = match f.origin {
                LakeOrigin::Unclassified => "unclassified",
                LakeOrigin::Tectonic => "tectonic",
                LakeOrigin::Glacial => "glacial",
                LakeOrigin::Oxbow => "oxbow",
                LakeOrigin::Karst => "karst",
                LakeOrigin::AridTerminal => "arid_terminal",
            };
            origins
                .entry(o.to_owned())
                .or_default()
                .insert(format!("{}", l.global_id.0), 1);
            terminal += u64::from(f.terminal);
            if std::env::var_os("LAKES").is_some() {
                eprintln!(
                    "lake {} area {ax},{ay} cells {} at {:?} origin {:?} depth {} outlet {:?}",
                    l.global_id.0,
                    l.cells.len(),
                    l.cells.first().map(|c| (c.x(), c.y())),
                    f.origin,
                    l.depth_mm,
                    l.outlet.map(|c| (c.x(), c.y()))
                );
            }
        }
        deltas += w.deltas.len() as u64;
        if std::env::var_os("LAKES").is_some() {
            for (r, f) in rivers.iter().zip(&w.segments) {
                if f.pattern == ChannelPattern::Braided {
                    let c = r.course[0];
                    eprintln!(
                        "braided area {ax},{ay} cell {:?} belt {} dm",
                        (i32::from(c.x()) + ax * 512, i32::from(c.y()) + ay * 512),
                        f.belt_width_dm
                    );
                }
            }
            for d in &w.deltas {
                eprintln!(
                    "delta area {ax},{ay} apex {:?} radius {} m catchment {} km²",
                    (
                        i32::from(d.apex.x()) + ax * 512,
                        i32::from(d.apex.y()) + ay * 512
                    ),
                    d.radius_m,
                    d.catchment_km2
                );
            }
        }
        dolines += w.dolines.len() as u64;
    }
    let meta = std::fs::read(Path::new(&root).join("hydrology/metadata.bin"))?;
    let m: arda_core::hydrology::HydrologyMetadata =
        arda_core::formats::hydrology::decode_record(meta.get(32..).ok_or("metadata")?)?;
    let b = m.budget;
    let source = b.land_precipitation.0 + b.lake_precipitation.0;
    let sinks = b.land_loss.0
        + b.lake_evaporation.0
        + b.marginal_evaporation.0
        + b.sea_outflow.0
        + b.domain_outflow.0;
    let by_origin: BTreeMap<_, _> = origins.iter().map(|(k, v)| (k.clone(), v.len())).collect();
    let sinuosity: Vec<String> = bands
        .iter()
        .zip(&band_sum)
        .map(|(b, &(s, n))| {
            format!(
                "\"lt{}\":[{:.3},{}]",
                if *b == u32::MAX { 0 } else { *b },
                s as f64 / 1000.0 / n.max(1) as f64,
                n
            )
        })
        .collect();
    println!(
        "{{\"forms\":{have_forms},\"width_exponent\":{:.3},\"depth_exponent\":{:.3},\"confluences\":{conf},\"depth_not_falling_downstream\":{conf_ok},\"pattern_cells\":{pattern_cells:?},\"q_ge_1m3s_sinuosity_by_slope_ppm\":{{{}}},\"lakes\":{},\"lakes_le_3_cells\":{},\"lakes_with_inlet\":{},\"lakes_with_outlet\":{},\"lake_origins\":{by_origin:?},\"terminal_lake_fragments\":{terminal},\"deltas\":{deltas},\"dolines\":{dolines},\"water_in_l\":{source},\"water_out_l\":{sinks},\"ledger_residual_l\":{}}}",
        fit(&wq),
        fit(&dq),
        sinuosity.join(","),
        lake_cells.len(),
        lake_cells.values().filter(|&&n| n <= 3).count(),
        with_inlet.len(),
        with_outlet.len(),
        i128::try_from(source).unwrap_or(0) - i128::try_from(sinks).unwrap_or(0),
    );
    Ok(())
}
