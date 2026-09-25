//! Opt-in canonical fine-source diagnostic; does not publish a world.

use std::{env, error::Error, fs::OpenOptions, io::Write, path::Path};

use arda_core::{GenerateConfig, LatitudeBand, SizeKm};
use arda_gen::{
    continent::generate_continent_attempt,
    orchestrator::fine_source::{admit, generate, FineSourceLimits},
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 8 {
        return Err("usage: generate_fine_source <world-seed-u64> <attempt-u8> <width-km> <height-km> <max-ram-bytes> <max-file-bytes> <new-terrain-file> <new-receipt-json>".into());
    }
    let seed: u64 = args[0].parse()?;
    let attempt: u8 = args[1].parse()?;
    let config = GenerateConfig::new(
        SizeKm::new(args[2].parse()?, args[3].parse()?),
        LatitudeBand::new(35, 55),
        15,
    )?;
    let limits = FineSourceLimits {
        max_ram_bytes: args[4].parse()?,
        max_file_bytes: args[5].parse()?,
    };
    let plan = admit(config, limits)?;
    let output = Path::new(&args[6]);
    let receipt_path = Path::new(&args[7]);
    if output.exists() || receipt_path.exists() {
        return Err("output or receipt path already exists".into());
    }
    let macro_grid = generate_continent_attempt(seed, config, attempt);
    let receipt = generate(seed, attempt, config, &macro_grid, output, limits)?;
    let json = serde_json::json!({
        "recipe_version": receipt.recipe_version,
        "world_seed": receipt.world_seed,
        "attempt": receipt.attempt,
        "nominal_width_um": plan.nominal_width_um,
        "nominal_height_um": plan.nominal_height_um,
        "covered_width_um": plan.covered_width_um,
        "covered_height_um": plan.covered_height_um,
        "sampled_last_x_um": plan.sampled_last_x_um,
        "sampled_last_y_um": plan.sampled_last_y_um,
        "fine_width": plan.fine_width,
        "fine_height": plan.fine_height,
        "fft_width": plan.fft_width,
        "fft_height": plan.fft_height,
        "spacing_um": receipt.spacing_um,
        "admitted_peak_ram_bytes": plan.admitted_peak_ram_bytes.to_string(),
        "file_bytes": plan.file_bytes.to_string(),
        "calibration_seed": receipt.calibration_seed,
        "reference_period_samples": receipt.reference_period_samples,
        "reference_span_mm": receipt.reference_span_mm,
        "calibration_raw_range": receipt.calibration_raw_range,
        "scope": "opt-in complete-world canonical fine source, no normal-pipeline wiring or evolution"
    });
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(receipt_path)?;
    file.write_all(serde_json::to_string_pretty(&json)?.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?;
    println!("{}", serde_json::to_string(&json)?);
    Ok(())
}
