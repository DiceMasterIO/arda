//! Compare legacy array normalization with one fixed physical relief scale.

use arda_core::HeightMm;
use arda_gen::spectral::{calibration, filter_heights, filter_relief, white_noise};
use std::{
    env,
    error::Error,
    fs,
    io::{BufWriter, Write},
    path::Path,
    time::Instant,
};

fn create(path: &Path) -> Result<BufWriter<fs::File>, std::io::Error> {
    Ok(BufWriter::new(
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?,
    ))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 4 {
        return Err(
            "usage: physical_spectral <u32-seed> <width> <height> <new-output-directory>".into(),
        );
    }
    let seed: u32 = args[0].parse()?;
    let width: usize = args[1].parse()?;
    let height: usize = args[2].parse()?;
    let total = 512_u64 * 1024 * 1024;
    let scale = {
        let reference = white_noise(42, 512, 512, total)?;
        calibration(&reference, 512, 3_000_000, total - 512 * 512 * 8)?
    };
    let count = width.checked_mul(height).ok_or("dimension overflow")?;
    let input_bytes = u64::try_from(count)?
        .checked_mul(8)
        .ok_or("byte overflow")?;
    let legacy_budget = total
        .checked_sub(input_bytes)
        .ok_or("memory budget exceeded")?;
    let relief_budget = legacy_budget
        .checked_sub(input_bytes / 2)
        .ok_or("memory budget exceeded")?;
    let start = Instant::now();
    let white = white_noise(seed, width, height, total)?;
    let white_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let legacy = filter_heights(
        &white,
        width,
        height,
        HeightMm::new(1000),
        HeightMm::new(3_001_000),
        legacy_budget,
    )?;
    let legacy_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let relief = filter_relief(&white, width, height, &scale, relief_budget)?;
    let relief_seconds = start.elapsed().as_secs_f64();
    let out = Path::new(&args[3]);
    fs::create_dir(out)?;
    let mut file = create(&out.join("white.i64le"))?;
    for value in white {
        file.write_all(&value.to_le_bytes())?;
    }
    file.flush()?;
    for (name, values) in [("legacy.i32le", legacy), ("relief.i32le", relief)] {
        let mut file = create(&out.join(name))?;
        for value in values {
            file.write_all(&value.raw().to_le_bytes())?;
        }
        file.flush()?;
    }
    let report = serde_json::json!({
        "seed":seed,"width":width,"height":height,"spacing_um":39_062_500_u32,
        "calibration_seed":42,"reference_period_samples":scale.reference_period_samples(),
        "span_mm":scale.span_mm(),"calibration_raw_range":scale.calibration_raw_range().to_string(),
        "white_seconds":white_seconds,"legacy_seconds":legacy_seconds,"relief_seconds":relief_seconds,
        "total_admission_bytes":total,"scope":"signed physical relief; no world datum, coast, or hydrology"
    });
    let mut file = create(&out.join("receipt.json"))?;
    serde_json::to_writer_pretty(&mut file, &report)?;
    file.flush()?;
    println!("{report}");
    Ok(())
}
