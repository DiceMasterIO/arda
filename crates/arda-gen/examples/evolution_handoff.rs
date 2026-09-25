//! One diagnostic call of the current shared-domain evolution on frozen terrain.
//! This is not the normal generator or a persisted-world handoff.

mod hydrology {
    pub use arda_gen::hydrology::HydrologyError;
}
mod fill {
    pub use arda_gen::area::fill::Filled;
    pub(crate) const NEIGHBOURS: [(i32, i32); 8] = [
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ];
}
#[allow(dead_code)] // This diagnostic invokes evolve without the other production entry points.
#[path = "../src/area/evolution.rs"]
mod evolution;
#[allow(dead_code)] // The included production module exports helpers unused by this example.
#[path = "../src/area/mfd.rs"]
mod mfd;

use arda_core::{HeightMm, TerrainField, TerrainPoint};
use std::{env, error::Error, fs, io::Write, path::Path, time::Instant};

const FINE_SIDE: usize = 2048;
const MACRO_SIDE: usize = 801;
const EVOLVE_SIDE: usize = 800;
const FINE_STEP_UM: u32 = 39_062_500;
const EVOLVE_STEP_UM: i64 = 100_000_000;

fn read_i32(path: &Path, count: usize) -> Result<Vec<i32>, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    if bytes.len() != count.checked_mul(4).ok_or("byte count overflow")? {
        return Err(format!("unexpected length for {}: {}", path.display(), bytes.len()).into());
    }
    let (words, remainder) = bytes.as_chunks::<4>();
    debug_assert!(remainder.is_empty());
    Ok(words.iter().copied().map(i32::from_le_bytes).collect())
}

fn write_i32(path: &Path, values: &[i32]) -> Result<(), Box<dyn Error>> {
    let mut out = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    for &value in values {
        out.write_all(&value.to_le_bytes())?;
    }
    out.flush()?;
    Ok(())
}

fn stats(before: &[i32], after: &[i32]) -> serde_json::Value {
    let mut absolute: Vec<i64> = before
        .iter()
        .zip(after)
        .map(|(&a, &b)| (i64::from(b) - i64::from(a)).abs())
        .collect();
    absolute.sort_unstable();
    let n = before.len();
    let mean = |v: &[i32]| v.iter().map(|&x| f64::from(x)).sum::<f64>() / n as f64;
    let rms_grade = |v: &[i32]| {
        let mut sum = 0_f64;
        let mut count = 0_usize;
        for y in 0..EVOLVE_SIDE {
            for x in 0..EVOLVE_SIDE {
                let i = y * EVOLVE_SIDE + x;
                if x + 1 < EVOLVE_SIDE {
                    let g = f64::from(v[i + 1]) - f64::from(v[i]);
                    sum += g * g;
                    count += 1;
                }
                if y + 1 < EVOLVE_SIDE {
                    let g = f64::from(v[i + EVOLVE_SIDE]) - f64::from(v[i]);
                    sum += g * g;
                    count += 1;
                }
            }
        }
        (sum / count as f64).sqrt() / 100_000_f64
    };
    let mut rim_changed = 0_usize;
    for y in 0..EVOLVE_SIDE {
        for x in 0..EVOLVE_SIDE {
            if (x == 0 || y == 0 || x + 1 == EVOLVE_SIDE || y + 1 == EVOLVE_SIDE)
                && before[y * EVOLVE_SIDE + x] != after[y * EVOLVE_SIDE + x]
            {
                rim_changed += 1;
            }
        }
    }
    serde_json::json!({
        "before_min_mm":before.iter().min(),"before_max_mm":before.iter().max(),"before_mean_mm":mean(before),
        "after_min_mm":after.iter().min(),"after_max_mm":after.iter().max(),"after_mean_mm":mean(after),
        "mean_change_mm":mean(after)-mean(before),
        "median_abs_change_mm":absolute[n/2],"p95_abs_change_mm":absolute[n*95/100],
        "max_abs_change_mm":absolute[n-1],"changed_cells":absolute.iter().filter(|&&x|x!=0).count(),
        "before_rms_grade":rms_grade(before),"after_rms_grade":rms_grade(after),
        "rim_changed_cells":rim_changed,
    })
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err(
            "usage: evolution_handoff <composed2048.i32le> <macro801.i32le> <new-output-dir>"
                .into(),
        );
    }
    let fine_raw = read_i32(Path::new(&args[0]), FINE_SIDE * FINE_SIDE)?;
    let macro_raw = read_i32(Path::new(&args[1]), MACRO_SIDE * MACRO_SIDE)?;
    let field = TerrainField::new(
        TerrainPoint { x_um: 0, y_um: 0 },
        FINE_STEP_UM,
        u32::try_from(FINE_SIDE)?,
        u32::try_from(FINE_SIDE)?,
        fine_raw.into_iter().map(HeightMm::new).collect(),
    )?;
    let mut before = Vec::with_capacity(EVOLVE_SIDE * EVOLVE_SIDE);
    let mut coarse = Vec::with_capacity(EVOLVE_SIDE * EVOLVE_SIDE);
    for y in 0..EVOLVE_SIDE {
        for x in 0..EVOLVE_SIDE {
            let point = TerrainPoint {
                x_um: i64::try_from(x)? * EVOLVE_STEP_UM,
                y_um: i64::try_from(y)? * EVOLVE_STEP_UM,
            };
            before.push(
                field
                    .sample(point)
                    .ok_or("100 m query outside fine source")?
                    .raw(),
            );
            coarse.push(macro_raw[y * MACRO_SIDE + x]);
        }
    }
    let mut after = before.clone();
    let start = Instant::now();
    evolution::evolve(&mut after, &coarse, EVOLVE_SIDE, EVOLVE_SIDE)?;
    let seconds = start.elapsed().as_secs_f64();
    let out = Path::new(&args[2]);
    fs::create_dir(out)?;
    write_i32(&out.join("before-100m.i32le"), &before)?;
    write_i32(&out.join("after-100m.i32le"), &after)?;
    let report = serde_json::json!({"fine_side":FINE_SIDE,"fine_spacing_um":FINE_STEP_UM,
        "macro_side":MACRO_SIDE,"evolved_side":EVOLVE_SIDE,"evolved_spacing_um":EVOLVE_STEP_UM,
        "iterations":evolution::SHARED_ITERATIONS,"evolution_seconds":seconds,
        "uplift_input":"top-left 800x800 of unchanged macro-100m.i32le (801x801)",
        "scope":"exact existing shared evolution, diagnostic source composition, not saved-world integration",
        "metrics":stats(&before,&after)});
    fs::write(
        out.join("receipt.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{report}");
    Ok(())
}
