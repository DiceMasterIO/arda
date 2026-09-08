//! Explicit before/after measurements for the radial coast correction.
//!
//! Set `ARDA_COAST_MEASUREMENT_OUT` to a new evidence directory and run the
//! `measure_accepted_coast_geometry` ignored test in release mode. This
//! generates only the accepted continent surface, climate and coarse
//! drainage, never fine areas or a world export. Run the separate component
//! control with an existing output directory.

use std::{fs, path::Path, time::Instant};

use arda_core::GenerateConfig;
use arda_gen::continent::{
    climate::climate,
    coast::{continental_mask, graded_base_mm, shelf_gradient},
    generate_continent_attempt,
    hydrology::hydrology,
    plates::{plate_of_warped, seed_plates, CrustType, SimExtent},
    tectonics::run_tectonics,
    ContinentGrid,
};
use serde_json::{json, Value};

fn region_metrics(grid: &ContinentGrid, bounds: [i32; 4]) -> Value {
    let [x0, y0, x1, y1] = bounds;
    let mut land = 0_u64;
    let mut cardinal = 0_u64;
    let mut diagonal = 0_u64;
    let mut equal_adjacent_row_slopes = 0_u64;
    let mut row_slope_pairs = 0_u64;
    let mut directions = [0_u64; 8];
    let neighbours = [
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ];
    for y in y0.max(1)..y1.min(grid.height() - 1) {
        for x in x0.max(1)..x1.min(grid.width() - 1) {
            let height = grid.get(x, y).raw();
            if height <= 0 {
                continue;
            }
            land += 1;
            let mut best = None;
            let mut best_slope = 0_i64;
            for (direction, &(dx, dy)) in neighbours.iter().enumerate() {
                let drop = i64::from(height) - i64::from(grid.get(x + dx, y + dy).raw());
                let slope = drop * 1000 / if dx != 0 && dy != 0 { 1414 } else { 1000 };
                if slope > best_slope {
                    best_slope = slope;
                    best = Some(direction);
                }
            }
            if let Some(direction) = best {
                directions[direction] += 1;
                if direction % 2 == 0 {
                    cardinal += 1;
                } else {
                    diagonal += 1;
                }
            }
            let dx = i64::from(grid.get(x + 1, y).raw()) - i64::from(height);
            let adjacent_dx =
                i64::from(grid.get(x + 1, y + 1).raw()) - i64::from(grid.get(x, y + 1).raw());
            row_slope_pairs += 1;
            equal_adjacent_row_slopes += u64::from(dx == adjacent_dx);
        }
    }
    json!({
        "bounds_km_exclusive": bounds, "land_cells": land,
        "steepest_downhill_cardinal": cardinal, "steepest_downhill_diagonal": diagonal,
        "direction_counts_n_ne_e_se_s_sw_w_nw": directions,
        "equal_adjacent_row_slopes": equal_adjacent_row_slopes,
        "row_slope_pairs": row_slope_pairs,
    })
}

fn save_grid(path: &Path, grid: &ContinentGrid) -> std::io::Result<()> {
    let mut bytes = Vec::new();
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            bytes.extend(grid.get(x, y).raw().to_le_bytes());
        }
    }
    fs::write(path, bytes)
}

fn previous_square_mask(x: i32, y: i32, w: i32, h: i32) -> i32 {
    let nx = (i64::from(x) * 2 - i64::from(w)).abs() * 1024 / i64::from(w.max(1));
    let ny = (i64::from(y) * 2 - i64::from(h)).abs() * 1024 / i64::from(h.max(1));
    let r = nx.max(ny);
    if r <= 80 {
        return 1000;
    }
    if r >= 400 {
        return 0;
    }
    let t = 1024 - (r - 80) * 1024 / 320;
    let t2 = t * t / 1024;
    let t3 = t2 * t / 1024;
    i32::try_from((3 * t2 - 2 * t3) * 1000 / 1024).unwrap_or(0)
}

#[test]
#[ignore = "paired component measurement for MICRO 42 accepted attempt 2"]
fn measure_coast_source_control() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::var("ARDA_COAST_MEASUREMENT_OUT")?;
    let output = Path::new(&output);
    let sim = SimExtent {
        width: 51,
        height: 102,
    };
    let plates = seed_plates(42, sim, 2);
    let uplift = run_tectonics(42, &plates, sim, 20);
    let binary: Vec<i32> = (0..sim.height)
        .flat_map(|y| (0..sim.width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let id = plate_of_warped(42, &plates, x, y);
            1000 * i32::from(
                plates
                    .iter()
                    .any(|plate| plate.id == id && plate.crust == CrustType::Continental),
            )
        })
        .collect();
    let crust = shelf_gradient(&binary, sim.width, sim.height);
    let mut rows = Vec::new();
    for y in 45..=49 {
        for x in 28..=30 {
            let i = usize::try_from(y * sim.width + x)?;
            let old_mask = previous_square_mask(x, y, sim.width, sim.height);
            let new_mask = continental_mask(x, y, sim.width, sim.height);
            let old_base = graded_base_mm((crust[i] + 2 * old_mask) / 3);
            let new_base = graded_base_mm((crust[i] + 2 * new_mask) / 3);
            rows.push(json!({
                "simulation_x":x, "simulation_y":y,
                "visible_x_km":4*x-51, "visible_y_km":4*y-102,
                "blurred_crust":crust[i], "uplift_mm":uplift[i],
                "square_mask":old_mask, "radial_mask":new_mask,
                "square_base_mm":old_base, "radial_base_mm":new_base,
                "square_physical_mm":old_base.saturating_add(uplift[i]),
                "radial_physical_mm":new_base.saturating_add(uplift[i]),
            }));
        }
    }
    fs::write(
        output.join("coast-source-control.json"),
        serde_json::to_vec_pretty(&rows)?,
    )?;
    Ok(())
}

#[test]
#[ignore = "bounded radial-coast measurement; set ARDA_COAST_MEASUREMENT_OUT"]
fn measure_accepted_coast_geometry() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::var("ARDA_COAST_MEASUREMENT_OUT")?;
    let output = Path::new(&output);
    fs::create_dir(output)?;
    let mut results = Vec::new();
    for (size, seed, config) in [
        ("micro", 42, GenerateConfig::MICRO),
        ("micro", 99, GenerateConfig::MICRO),
        ("default", 42, GenerateConfig::default()),
        ("default", 7, GenerateConfig::default()),
        ("default", 436342, GenerateConfig::default()),
    ] {
        let started = Instant::now();
        let mut attempts = Vec::new();
        let mut accepted = None;
        for attempt in 0..5 {
            let attempt_started = Instant::now();
            let grid = generate_continent_attempt(seed, config, attempt);
            let land = grid.land_fraction_permille();
            attempts.push(json!({
                "attempt": attempt, "land_fraction_permille": land,
                "generation_seconds": attempt_started.elapsed().as_secs_f64(),
            }));
            if (250..=900).contains(&land) {
                accepted = Some((attempt, grid));
                break;
            }
        }
        let Some((attempt, grid)) = accepted else {
            results
                .push(json!({"size": size, "seed": seed, "accepted": false, "attempts": attempts}));
            continue;
        };
        let climate = climate(&grid, config.latitude_band());
        let water = hydrology(&grid, &climate);
        let mut sea_land_edges = 0_u64;
        let mut land = 0_u64;
        let mut river_directions = [0_u64; 8];
        for y in 0..grid.height() {
            for x in 0..grid.width() {
                if grid.get(x, y).raw() <= 0 {
                    continue;
                }
                land += 1;
                for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                    sea_land_edges += u64::from(grid.get(x + dx, y + dy).raw() <= 0);
                }
                let i = usize::try_from(y * grid.width() + x)?;
                if water.discharge_l_s[i] >= 4000 {
                    if let Some(count) =
                        river_directions.get_mut(usize::from(water.downstream_dir[i]))
                    {
                        *count += 1;
                    }
                }
            }
        }
        let heights_path = format!("{size}-{seed}-height-mm.i32le");
        save_grid(&output.join(&heights_path), &grid)?;
        let patch_rows: Vec<Vec<i32>> = (82..=90)
            .map(|y| (64..=72).map(|x| grid.get(x, y).raw()).collect())
            .collect();
        let result = json!({
            "size": size, "seed": seed, "accepted": true, "attempt": attempt,
            "attempts": attempts, "width": grid.width(), "height": grid.height(),
            "land_cells": land, "land_fraction_permille": grid.land_fraction_permille(),
            "sea_land_edges": sea_land_edges,
            "river_4000_l_s_direction_counts_n_ne_e_se_s_sw_w_nw": river_directions,
            "all_land_physical_slope": region_metrics(&grid, [0,0,grid.width(),grid.height()]),
            "pointed_patch": region_metrics(&grid, [64,82,73,91]),
            "pointed_patch_height_mm_rows_y82_to90_x64_to72": patch_rows,
            "height_binary": heights_path, "elapsed_seconds": started.elapsed().as_secs_f64(),
        });
        println!(
            "{size} seed {seed}: accepted attempt {attempt}, {} per mille land, {:.3}s",
            grid.land_fraction_permille(),
            started.elapsed().as_secs_f64()
        );
        results.push(result);
    }
    fs::write(
        output.join("measurements.json"),
        serde_json::to_vec_pretty(&results)?,
    )?;
    assert!(
        results.iter().all(|item| item["accepted"] == true),
        "at least one seed failed all five attempts; evidence retained"
    );
    Ok(())
}
