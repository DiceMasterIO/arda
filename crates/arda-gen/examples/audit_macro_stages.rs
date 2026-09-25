//! Diagnostic replay of the existing seed-42 MICRO continent stages.
use arda_core::GenerateConfig;
use arda_gen::{
    continent::{
        coast, erode, generate_continent_attempt,
        plates::{self, CrustType, SimExtent},
        tectonics,
    },
    noise::fbm,
};
use std::{
    error::Error,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

fn save(path: &Path, name: &str, values: &[i32]) -> Result<(), Box<dyn Error>> {
    let mut f = BufWriter::new(File::create(path.join(format!("{name}.i32le")))?);
    for v in values {
        f.write_all(&v.to_le_bytes())?;
    }
    f.flush()?;
    Ok(())
}

fn sample(
    a: &[i32],
    sim: SimExtent,
    km_x: i32,
    km_y: i32,
) -> Result<i32, std::num::TryFromIntError> {
    let gx = (km_x / 4).clamp(0, sim.width - 1);
    let gy = (km_y / 4).clamp(0, sim.height - 1);
    let gx1 = (gx + 1).min(sim.width - 1);
    let gy1 = (gy + 1).min(sim.height - 1);
    let fx = i64::from(km_x.rem_euclid(4)) * 65536 / 4;
    let fy = i64::from(km_y.rem_euclid(4)) * 65536 / 4;
    let at = |x: i32, y: i32| -> Result<i64, std::num::TryFromIntError> {
        Ok(i64::from(a[usize::try_from(y * sim.width + x)?]))
    };
    let top = at(gx, gy)? + (((at(gx1, gy)? - at(gx, gy)?) * fx) >> 16);
    let bottom = at(gx, gy1)? + (((at(gx1, gy1)? - at(gx, gy1)?) * fx) >> 16);
    i32::try_from(top + (((bottom - top) * fy) >> 16))
}

fn main() -> Result<(), Box<dyn Error>> {
    let out = std::env::args()
        .nth(1)
        .ok_or("expected new output directory")?;
    let out = Path::new(&out);
    fs::create_dir(out)?;
    let seed = 42u64;
    let attempt = 0u8;
    let (w, h) = (102i32, 204i32);
    let sim = SimExtent {
        width: 51,
        height: 102,
    };
    let plates = plates::seed_plates(seed, sim, attempt);
    let uplift = tectonics::run_tectonics(seed, &plates, sim, 20);
    let binary: Vec<i32> = (0..sim.height)
        .flat_map(|y| (0..sim.width).map(move |x| (x, y)))
        .map(|(x, y)| {
            let id = plates::plate_of_warped(seed, &plates, x, y);
            i32::from(
                plates
                    .iter()
                    .find(|p| p.id == id)
                    .is_some_and(|p| p.crust == CrustType::Continental),
            ) * 1000
        })
        .collect();
    let shelf = coast::shelf_gradient(&binary, sim.width, sim.height);
    let mask: Vec<i32> = (0..sim.height)
        .flat_map(|y| (0..sim.width).map(move |x| (x, y)))
        .map(|(x, y)| coast::continental_mask(x, y, sim.width, sim.height))
        .collect();
    let blend: Vec<i32> = shelf
        .iter()
        .zip(&mask)
        .map(|(s, m)| (s + 2 * m) / 3)
        .collect();
    let base: Vec<i32> = blend.iter().map(|c| coast::graded_base_mm(*c)).collect();
    let coarse: Vec<i32> = base
        .iter()
        .zip(&uplift)
        .map(|(b, u)| b.saturating_add(*u))
        .collect();
    for (name, a) in [
        ("uplift4", &uplift),
        ("binary4", &binary),
        ("shelf4", &shelf),
        ("mask4", &mask),
        ("blend4", &blend),
        ("base4", &base),
        ("coarse4", &coarse),
    ] {
        save(out, name, a)?;
    }
    let count = usize::try_from(w * h)?;
    let mut pre = Vec::with_capacity(count);
    let mut noisy = Vec::with_capacity(count);
    for y in 0..h {
        for x in 0..w {
            let h0 = sample(&coarse, sim, x + 51, y + 102)?;
            pre.push(h0);
            let shelf = 1024 - (h0.abs() / 400).clamp(0, 1024);
            let amp = 12000 + 38000 * i64::from(shelf) / 1024;
            let detail = i64::from(fbm(
                seed ^ 0x00DE_7A11 ^ (u64::from(attempt) << 48),
                x,
                y,
                24,
                5,
            ));
            let mut hn = h0.saturating_add(i32::try_from(detail * amp / 32768)?);
            if coast::rim_forced_ocean(x, y, w, h, 2) {
                hn = hn.min(-1);
            }
            noisy.push(hn);
        }
    }
    let mut final_h = noisy.clone();
    erode::erode_continent(&mut final_h, w, h);
    let original = generate_continent_attempt(seed, GenerateConfig::MICRO, attempt);
    let mismatch = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .zip(&final_h)
        .filter(|&((x, y), value)| original.get(x, y).raw() != *value)
        .count();
    if mismatch != 0 {
        return Err(format!("replay mismatch: {mismatch}").into());
    }
    for (name, a) in [
        ("pre_noise1", &pre),
        ("post_noise1", &noisy),
        ("final1", &final_h),
    ] {
        save(out, name, a)?;
    }
    println!("exact stage replay: {} cells; mismatches={mismatch}", w * h);
    Ok(())
}
