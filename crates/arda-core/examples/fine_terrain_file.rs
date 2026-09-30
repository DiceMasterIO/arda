//! Stream a canonical fine surface to disk, then query it in a fresh process.
use arda_core::{
    formats::terrain::{TerrainFileReader, TerrainFileWriter},
    HeightMm, TerrainPoint,
};
use std::{
    env,
    error::Error,
    fs,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};

fn create(path: &str) -> Result<fs::File, std::io::Error> {
    fs::OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(path)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("store") if args.len()==8 => {
            let width: u32=args[2].parse()?;
            let height: u32=args[3].parse()?;
            let spacing: u32=args[4].parse()?;
            let origin=TerrainPoint{x_um:args[5].parse()?,y_um:args[6].parse()?};
            let expected=u64::from(width).checked_mul(u64::from(height)).and_then(|n|n.checked_mul(4)).ok_or("input dimensions overflow")?;
            if fs::metadata(&args[1])?.len()!=expected {return Err("raw field length mismatch".into());}
            // This diagnostic bounds both source row buffers before allocating.
            if !(2..=32768).contains(&width) {return Err("diagnostic row width out of range".into());}
            let row_size=usize::try_from(width)?.checked_mul(4).ok_or("row length overflow")?;
            let mut input=BufReader::new(fs::File::open(&args[1])?);
            let mut writer=TerrainFileWriter::new(BufWriter::new(create(&args[7])?),origin,spacing,width,height)?;
            let mut bytes=vec![0_u8;row_size];
            let mut row=vec![HeightMm::SEA_LEVEL;usize::try_from(width)?];
            for _ in 0..height {
                input.read_exact(&mut bytes)?;
                for (sample,chunk) in row.iter_mut().zip(bytes.as_chunks::<4>().0) {*sample=HeightMm::new(i32::from_le_bytes(*chunk));}
                writer.write_row(&row)?;
            }
            writer.finish()?.flush()?;
            println!("Stored {width}x{height} canonical heights without a full-field input allocation.");
        },
        Some("query") if args.len()==5 => {
            let count=fs::metadata(&args[2])?.len();
            if count%16!=0 {return Err("query file must contain i64 x,y pairs".into());}
            let budget: u64=args[4].parse()?;
            let mut reader=TerrainFileReader::open(fs::File::open(&args[1])?,budget)?;
            let mut queries=BufReader::new(fs::File::open(&args[2])?);
            let mut output=BufWriter::new(create(&args[3])?);
            for _ in 0..count/16 {
                let mut x=[0_u8;8];let mut y=[0_u8;8];
                queries.read_exact(&mut x)?;queries.read_exact(&mut y)?;
                let point=TerrainPoint{x_um:i64::from_le_bytes(x),y_um:i64::from_le_bytes(y)};
                let height=reader.sample(point)?.ok_or("query outside canonical coverage")?;
                output.write_all(&height.raw().to_le_bytes())?;
            }
            output.flush()?;
            println!("Reopened {} and sampled {} points with a {budget}-byte reader allowance.",Path::new(&args[1]).display(),count/16);
        },
        _ => return Err("usage: fine_terrain_file store <raw-i32le> <width> <height> <spacing-um> <origin-x-um> <origin-y-um> <new-file> | query <terrain-file> <xy-i64le> <new-i32le> <reader-budget-bytes>".into()),
    }
    Ok(())
}
