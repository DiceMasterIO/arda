//! Builds scenes for the arda-tactical test layouts and writes debug PNGs
//! and compact scene JSON to `out/scene/`.
//!
//! `cargo run -p arda-scene --example debug [-- --library top:…:bottom]`
//! (`--library` takes a library directory or a stack; default the
//! placeholder library).

use arda_scene::{build_scene, scene_debug_png, RulesSidecar};
use arda_tactical::{layouts, Library};
use std::error::Error;
use std::path::{Path, PathBuf};

/// The seed the images are rendered with.
const SEED: u64 = 7;
/// Debug pixels per square.
const PPSQ: u32 = 48;

/// The library named by `--library <dir or top:…:bottom stack>`, else the
/// placeholder library.
fn library_arg(root: &Path) -> PathBuf {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--library" {
            if let Some(spec) = args.next() {
                return PathBuf::from(spec);
            }
        } else if let Some(spec) = a.strip_prefix("--library=") {
            return PathBuf::from(spec);
        }
    }
    root.join("assets/tactical/placeholder")
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lib = Library::load_stack(&library_arg(&root))?;
    let out = root.join("out/scene");
    std::fs::create_dir_all(&out)?;
    for layout in layouts::all() {
        let scene = build_scene(&layout, &lib, SEED, None)?;
        write(&out, &layout.name, &scene)?;
        if layout.name == "riverside" {
            // A terrain sidecar: a band of undergrowth, a dense thicket that
            // blocks sight, and boulder cover, as arda-refine would emit.
            let mut rules = RulesSidecar::empty(layout.width, layout.height);
            for x in 0..layout.width.min(8) {
                if let Some(c) = rules.cell_mut(x, 0) {
                    c.difficult = Some(true);
                }
            }
            for (x, y) in [(1, 1), (2, 1), (1, 2)] {
                if let Some(c) = rules.cell_mut(x, y) {
                    c.blocks_sight = Some(true);
                    c.cover = Some(arda_scene::CoverLevel::ThreeQuarters);
                }
            }
            let scene = build_scene(&layout, &lib, SEED, Some(&rules))?;
            write(&out, "riverside_sidecar", &scene)?;
        }
    }
    Ok(())
}

fn write(out: &Path, name: &str, scene: &arda_scene::Scene) -> Result<(), Box<dyn Error>> {
    let json = scene.to_json()?;
    std::fs::write(out.join(format!("{name}.json")), &json)?;
    std::fs::write(
        out.join(format!("{name}.png")),
        scene_debug_png(scene, PPSQ)?,
    )?;
    println!(
        "{name}: {}x{}, {} walls, {} blockers, {} lights, {} regions, {} open squares, {} bytes json",
        scene.width,
        scene.height,
        scene.walls.len(),
        scene.vision_blockers.len(),
        scene.lights.len(),
        scene.regions.len(),
        scene.spawn_hints.open.len(),
        json.len()
    );
    Ok(())
}
