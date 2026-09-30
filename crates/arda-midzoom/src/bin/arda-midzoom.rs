//! `arda-midzoom`: render relief crops and tiles from a stored world, for
//! visual review and timing.

use arda_midzoom::{render_window, Pyramid, ReliefWorld};
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

#[derive(Parser)]
#[command(about = "Render a mid-zoom relief crop of a stored arda world")]
struct Args {
    /// World directory.
    #[arg(long)]
    world: PathBuf,
    /// Pyramid level (the overview's native level is log2(base / 256)).
    #[arg(long)]
    z: u32,
    /// Crop centre in world metres, `x,y`.
    #[arg(long)]
    center: String,
    /// Crop side in pixels.
    #[arg(long, default_value_t = 512)]
    size: u32,
    /// Overview pyramid base in pixels.
    #[arg(long, default_value_t = 4096)]
    base_px: u32,
    /// Output PNG.
    #[arg(long)]
    out: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let world = Arc::new(arda::World::load(&args.world)?);
    let m = world.manifest();
    let pyramid = Pyramid {
        max_zoom: (args.base_px / 256).trailing_zeros(),
        areas_wide: m.areas_wide,
        areas_high: m.areas_high,
    };
    let rw = ReliefWorld::new(Arc::clone(&world))?;
    let (cx, cy) = args.center.split_once(',').ok_or("center must be x,y")?;
    let (cx, cy): (f64, f64) = (cx.trim().parse()?, cy.trim().parse()?);
    let px_m = pyramid.pixel_um(args.z) as f64 / 1e6;
    let half = f64::from(args.size) / 2.0;
    #[allow(clippy::cast_possible_truncation)]
    let origin = (
        (cx / px_m - half).round() as i64,
        (cy / px_m - half).round() as i64,
    );
    let cold = Instant::now();
    let img = render_window(&rw, &pyramid, args.z, origin, (args.size, args.size))?;
    let cold = cold.elapsed();
    let warm = Instant::now();
    let img2 = render_window(&rw, &pyramid, args.z, origin, (args.size, args.size))?;
    let warm = warm.elapsed();
    if img != img2 {
        return Err("non-deterministic render".into());
    }
    let file = std::fs::File::create(&args.out)?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()?.write_image_data(&img.pixels)?;
    println!(
        "z={} {:.3} m/px origin=({},{}) {}px: cold {:.0} ms, warm {:.0} ms -> {}",
        args.z,
        px_m,
        origin.0,
        origin.1,
        args.size,
        cold.as_secs_f64() * 1e3,
        warm.as_secs_f64() * 1e3,
        args.out.display()
    );
    Ok(())
}
