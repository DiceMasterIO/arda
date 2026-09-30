//! Exercise the full world seed/attempt stream with fixed relief calibration.
use arda_gen::spectral::{calibration, filter_relief, white_noise, white_noise_world};
use std::{
    env,
    error::Error,
    fs,
    io::{BufWriter, Write},
    path::Path,
    time::Instant,
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 5 {
        return Err("usage: world_spectral_source <u64-seed> <u8-attempt> <width> <height> <new-output-dir>".into());
    }
    let seed: u64 = args[0].parse()?;
    let attempt: u8 = args[1].parse()?;
    let width: usize = args[2].parse()?;
    let height: usize = args[3].parse()?;
    let budget = 512_u64 * 1024 * 1024;
    let scale = {
        let reference = white_noise(42, 512, 512, budget)?;
        calibration(&reference, 512, 3_000_000, budget - 512 * 512 * 8)?
    };
    let bytes = u64::try_from(width.checked_mul(height).ok_or("dimension overflow")?)?
        .checked_mul(8)
        .ok_or("input byte overflow")?;
    let scratch = budget
        .checked_sub(bytes)
        .ok_or("diagnostic memory budget exceeded")?;
    let start = Instant::now();
    let white = white_noise_world(seed, attempt, width, height, budget)?;
    let relief = filter_relief(&white, width, height, &scale, scratch)?;
    let seconds = start.elapsed().as_secs_f64();
    let out = Path::new(&args[4]);
    fs::create_dir(out)?;
    let mut noise = BufWriter::new(
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(out.join("white.i64le"))?,
    );
    for v in white {
        noise.write_all(&v.to_le_bytes())?;
    }
    noise.flush()?;
    let mut field = BufWriter::new(
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(out.join("relief.i32le"))?,
    );
    for h in relief {
        field.write_all(&h.raw().to_le_bytes())?;
    }
    field.flush()?;
    let report = serde_json::json!({"world_seed":seed,"attempt":attempt,"width":width,"height":height,"spacing_um":39_062_500_u32,"reference_native_seed":42,"reference_period_samples":scale.reference_period_samples(),"span_mm":scale.span_mm(),"calibration_raw_range":scale.calibration_raw_range().to_string(),"generation_seconds":seconds,"total_admission_bytes":budget,"scope":"full world seed identity and signed relief; no composed/persisted full world"});
    let file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(out.join("receipt.json"))?;
    serde_json::to_writer_pretty(file, &report)?;
    println!("{report}");
    Ok(())
}
