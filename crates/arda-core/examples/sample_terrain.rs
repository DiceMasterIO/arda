//! Diagnostic raw-height sampler for canonical terrain fields.
//!
//! Input heights are little-endian i32 millimetres; query records are two
//! little-endian i64 absolute micrometre coordinates. This is not a world codec.

use arda_core::{
    terrain::{TerrainField, TerrainPoint},
    HeightMm,
};
use std::{env, error::Error, fs, io::Write};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 8 {
        return Err("usage: sample_terrain <heights.i32le> <width> <height> <spacing_um> <origin_x_um> <origin_y_um> <queries.i64le> <output.i32le>".into());
    }
    let bytes = fs::read(&args[0])?;
    let (samples, remainder) = bytes.as_chunks::<4>();
    if !remainder.is_empty() {
        return Err("height input has a partial millimetre sample".into());
    }
    let heights = samples
        .iter()
        .map(|v| HeightMm::new(i32::from_le_bytes(*v)))
        .collect();
    let field = TerrainField::new(
        TerrainPoint {
            x_um: args[4].parse()?,
            y_um: args[5].parse()?,
        },
        args[3].parse()?,
        args[1].parse()?,
        args[2].parse()?,
        heights,
    )?;
    let queries = fs::read(&args[6])?;
    let (points, remainder) = queries.as_chunks::<16>();
    if !remainder.is_empty() {
        return Err("query input has a partial coordinate pair".into());
    }
    let mut result = Vec::with_capacity(queries.len() / 4);
    for pair in points {
        let x_um = i64::from_le_bytes(pair[..8].try_into()?);
        let y_um = i64::from_le_bytes(pair[8..].try_into()?);
        let h = field
            .sample(TerrainPoint { x_um, y_um })
            .ok_or("query outside canonical field")?;
        result.extend_from_slice(&h.raw().to_le_bytes());
    }
    // Refuse overwrite so repeated diagnostic calls cannot destroy evidence.
    let mut output = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args[7])?;
    output.write_all(&result)?;
    Ok(())
}
