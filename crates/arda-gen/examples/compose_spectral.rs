//! Execute the fixed 80 km source-composition diagnostic through shared code.
use arda_core::{HeightMm, TerrainField, TerrainPoint};
use arda_gen::spectral_composition::{apply_relief_gate, relief_gate_q16};
use std::{
    env,
    error::Error,
    fs,
    io::{BufWriter, Write},
    path::Path,
};

fn read(path: &Path, count: usize) -> Result<Vec<HeightMm>, Box<dyn Error>> {
    if fs::metadata(path)?.len() != u64::try_from(count * 4)? {
        return Err("unexpected diagnostic raster size".into());
    }
    let bytes = fs::read(path)?;
    if bytes.len() != count * 4 {
        return Err("diagnostic raster changed during read".into());
    }
    Ok(bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| HeightMm::new(i32::from_le_bytes(*b)))
        .collect())
}
fn writer(path: &Path) -> Result<BufWriter<fs::File>, std::io::Error> {
    Ok(BufWriter::new(
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?,
    ))
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: compose_spectral <901-square-macro-halo.i32le> <2048-square-relief.i32le> <new-output-dir>".into());
    }
    let macro_halo = read(Path::new(&args[0]), 901 * 901)?;
    let relief = read(Path::new(&args[1]), 2048 * 2048)?;
    let offsets = [
        (0, 50),
        (0, -50),
        (50, 0),
        (-50, 0),
        (35, 35),
        (35, -35),
        (-35, 35),
        (-35, -35),
    ];
    let mut gates = Vec::with_capacity(801 * 801);
    let mut macro_centers = Vec::with_capacity(801 * 801);
    for y in 50_i32..851 {
        for x in 50_i32..851 {
            let at = |dx, dy| {
                usize::try_from((y + dy) * 901 + x + dx)
                    .ok()
                    .and_then(|i| macro_halo.get(i))
                    .copied()
                    .ok_or("stencil outside macro halo")
            };
            let mut surrounding = [HeightMm::SEA_LEVEL; 8];
            for (height, (dx, dy)) in surrounding.iter_mut().zip(offsets) {
                *height = at(dx, dy)?;
            }
            let center = at(0, 0)?;
            let gate = relief_gate_q16(center, &surrounding);
            gates.push(HeightMm::new(i32::try_from(gate)?));
            macro_centers.push(center);
        }
    }
    let origin = TerrainPoint { x_um: 0, y_um: 0 };
    let gate = TerrainField::new(origin, 100_000_000, 801, 801, gates)?;
    let macro_field = TerrainField::new(origin, 100_000_000, 801, 801, macro_centers)?;
    let out = Path::new(&args[2]);
    fs::create_dir(out)?;
    let mut composed = writer(&out.join("composed.i32le"))?;
    let mut macro_out = writer(&out.join("macro-fine.i32le"))?;
    let mut gate_out = writer(&out.join("gate-fine.i32le"))?;
    for y in 0..2048 {
        for x in 0..2048 {
            let point = TerrainPoint {
                x_um: i64::try_from(x)? * 39_062_500,
                y_um: i64::try_from(y)? * 39_062_500,
            };
            let m = macro_field
                .sample(point)
                .ok_or("macro point outside field")?;
            let g = gate.sample(point).ok_or("gate point outside field")?;
            let h = apply_relief_gate(m, relief[y * 2048 + x], u32::try_from(g.raw())?)?;
            composed.write_all(&h.raw().to_le_bytes())?;
            macro_out.write_all(&m.raw().to_le_bytes())?;
            gate_out.write_all(&g.raw().to_le_bytes())?;
        }
    }
    composed.flush()?;
    macro_out.flush()?;
    gate_out.flush()?;
    println!("Composed 2048-square field with shared integer gate, canonical sampling and checked signed relief.");
    Ok(())
}
