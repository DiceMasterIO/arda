//! Generates the bounded spectral-source diagnostic, not a complete world.

use arda_core::{HeightMm, TerrainField, TerrainPoint};
use arda_gen::spectral::{filter_heights, white_noise};
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
            "usage: spectral_source <u32-seed> <width> <height> <new-output-directory>".into(),
        );
    }
    let seed: u32 = args[0].parse()?;
    let width: usize = args[1].parse()?;
    let height: usize = args[2].parse()?;
    let cells = width.checked_mul(height).ok_or("dimension overflow")?;
    let input_bytes = u64::try_from(cells.checked_mul(8).ok_or("input byte overflow")?)?;
    let total_limit = 512_u64 * 1024 * 1024;
    let filter_limit = total_limit
        .checked_sub(input_bytes)
        .ok_or("diagnostic memory budget exceeded")?;
    let start = Instant::now();
    let white = white_noise(seed, width, height, total_limit)?;
    let white_seconds = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let heights = filter_heights(
        &white,
        width,
        height,
        HeightMm::new(1000),
        HeightMm::new(3_001_000),
        filter_limit,
    )?;
    let filter_seconds = start.elapsed().as_secs_f64();
    let field = TerrainField::new(
        TerrainPoint { x_um: 0, y_um: 0 },
        39_062_500,
        u32::try_from(width)?,
        u32::try_from(height)?,
        heights,
    )?;
    let out = Path::new(&args[3]);
    fs::create_dir(out)?;
    let mut noise = create(&out.join("white.i64le"))?;
    for value in &white {
        noise.write_all(&value.to_le_bytes())?;
    }
    noise.flush()?;
    let mut terrain = create(&out.join("heights.i32le"))?;
    for value in field.heights() {
        terrain.write_all(&value.raw().to_le_bytes())?;
    }
    terrain.flush()?;
    let report = serde_json::json!({"seed":seed,"width":width,"height":height,"spacing_um":39_062_500_u32,"height_range_mm":[1000,3_001_000],"white_seconds":white_seconds,"filter_seconds":filter_seconds,"total_admission_bytes":total_limit,"scope":"procedural spectral source and canonical field; no coast or hydrology"});
    let mut receipt = create(&out.join("receipt.json"))?;
    serde_json::to_writer_pretty(&mut receipt, &report)?;
    receipt.flush()?;
    println!("{report}");
    Ok(())
}
