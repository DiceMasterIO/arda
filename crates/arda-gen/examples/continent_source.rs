//! Export the existing macro-geography for source-composition diagnostics.
use arda_core::{GenerateConfig, LatitudeBand, SizeKm};
use arda_gen::continent::{bundles::coarse_height, generate_continent_attempt};
use std::{
    env,
    error::Error,
    fs,
    io::{BufWriter, Write},
    path::Path,
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 4 && args.len() != 7 {
        return Err("usage: continent_source <seed> <width-km> <height-km> <new-output-dir> [crop-x-100m crop-y-100m crop-side-100m]".into());
    }
    let seed = args[0].parse()?;
    let config = GenerateConfig::new(
        SizeKm::new(args[1].parse()?, args[2].parse()?),
        LatitudeBand::new(35, 55),
        15,
    )?;
    let (attempt, grid) = (0..5)
        .map(|a| (a, generate_continent_attempt(seed, config, a)))
        .find(|(_, g)| (250..=900).contains(&g.land_fraction_permille()))
        .ok_or("no accepted continent attempt")?;
    let out = Path::new(&args[3]);
    fs::create_dir(out)?;
    let mut file = BufWriter::new(
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(out.join("continent.i32le"))?,
    );
    for y in 0..grid.height() {
        for x in 0..grid.width() {
            file.write_all(&grid.get(x, y).raw().to_le_bytes())?;
        }
    }
    file.flush()?;
    let crop = if args.len() == 7 {
        let x0: i32 = args[4].parse()?;
        let y0: i32 = args[5].parse()?;
        let side: i32 = args[6].parse()?;
        if !(2..=2048).contains(&side)
            || x0 < 0
            || y0 < 0
            || x0.checked_add(side).is_none_or(|v| v > grid.width() * 10)
            || y0.checked_add(side).is_none_or(|v| v > grid.height() * 10)
        {
            return Err("crop outside configured continent or diagnostic size limit".into());
        }
        let mut file = BufWriter::new(
            fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(out.join("coarse-100m.i32le"))?,
        );
        for y in y0..y0 + side {
            for x in x0..x0 + side {
                file.write_all(&coarse_height(&grid, x, y).to_le_bytes())?;
            }
        }
        file.flush()?;
        Some(
            serde_json::json!({"x0_100m":x0,"y0_100m":y0,"side":side,"spacing_um":100_000_000_u32}),
        )
    } else {
        None
    };
    let report = serde_json::json!({"seed":seed,"config":config,"attempt":attempt,"land_fraction_permille":grid.land_fraction_permille(),"crop":crop,"scope":"unchanged existing macro heights; no new climate or hydrology"});
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(out.join("receipt.json"))?;
    serde_json::to_writer_pretty(&mut file, &report)?;
    println!("{report}");
    Ok(())
}
