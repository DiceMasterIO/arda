//! Export a saved world's overview with a 16,384-pixel long edge.
//!
//! Run `cargo run -p arda --release --example export_world_16k -- WORLD OUTPUT.png`.
//! Aspect ratio follows the manifest. The renderer's 134,217,728-pixel limit
//! still applies; broad aspect ratios that exceed it are refused. This renders
//! saved terrain only and refuses to replace an existing output.

use arda::World;
use arda_core::AreaCoord;
use arda_render::OverviewRaster;
use std::{error::Error, fs::OpenOptions, io::Write, path::PathBuf};

const LONG_EDGE: u64 = 16_384;

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let input = PathBuf::from(args.next().ok_or("missing saved world")?);
    let output = PathBuf::from(args.next().ok_or("missing output PNG")?);
    if args.next().is_some() || output.exists() {
        return Err("unexpected argument or output already exists".into());
    }
    let world = World::load(&input)?;
    let manifest = world.manifest();
    let aw = u64::try_from(manifest.areas_wide)?;
    let ah = u64::try_from(manifest.areas_high)?;
    let longest = aw.max(ah);
    if aw == 0 || ah == 0 {
        return Err("world must contain at least one area per axis".into());
    }
    let width = u32::try_from((LONG_EDGE * aw + longest / 2) / longest)?;
    let height = u32::try_from((LONG_EDGE * ah + longest / 2) / longest)?;
    let mut raster =
        OverviewRaster::new_exact(manifest.areas_wide, manifest.areas_high, width, height)?;
    for (x, y) in world.area_coords() {
        let area = world.read_area(x, y)?;
        raster.push(AreaCoord::new(x, y), area.cells())?;
    }
    let png = raster.finish()?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)?;
    file.write_all(&png)?;
    println!("{} ({width} × {height})", output.display());
    Ok(())
}
