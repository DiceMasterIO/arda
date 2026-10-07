//! `arda dungeon …`: dungeon and cave battle maps (`crates/arda-dungeon`).

use anyhow::{bail, Context, Result};
use arda_dungeon::{generate, Kind, Params};
use arda_tactical::{render_with, Library, RenderOptions, Style};
use clap::{Args, ValueEnum};
use std::path::{Path, PathBuf};

/// What to generate.
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum KindArg {
    /// Built rooms and corridors.
    Dungeon,
    /// Natural caverns.
    Cave,
}

#[derive(Args)]
pub(crate) struct DungeonArgs {
    /// Seed.
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Dungeon (rooms and corridors) or cave (natural caverns).
    #[arg(long, value_enum, default_value_t = KindArg::Dungeon)]
    kind: KindArg,
    /// Size in squares, `WxH` (each side 16 to 160).
    #[arg(long, default_value = "48x36")]
    size: String,
    /// Output layout JSON (a `TacticalLayout`, renderable with
    /// `arda tactical render --layout FILE`).
    #[arg(long, default_value = "out/dungeon/layout.json")]
    out: PathBuf,
    /// Also write the rules sidecar (rock impassable, scree difficult).
    #[arg(long)]
    rules: Option<PathBuf>,
    /// Also write the description: rooms, doors, exits.
    #[arg(long)]
    meta: Option<PathBuf>,
    /// Also write the `arda-scene` scene JSON (needs the library).
    #[arg(long)]
    scene: Option<PathBuf>,
    /// Also render a PNG.
    #[arg(long)]
    render: Option<PathBuf>,
    /// Asset library or `top:…:bottom` stack for `--render` and `--scene`.
    #[arg(long, default_value = "assets/tactical/placeholder")]
    library: PathBuf,
    /// Output pixels per square for `--render`.
    #[arg(long, default_value_t = 128)]
    ppsq: u32,
    /// Draw the square grid on `--render`.
    #[arg(long)]
    grid: bool,
}

fn parse_size(text: &str) -> Result<(u32, u32)> {
    let Some((w, h)) = text.split_once(['x', 'X']) else {
        bail!("size `{text}` is not WxH");
    };
    Ok((
        w.trim()
            .parse()
            .with_context(|| format!("width in `{text}`"))?,
        h.trim()
            .parse()
            .with_context(|| format!("height in `{text}`"))?,
    ))
}

fn make_parent(path: &Path) -> Result<()> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    Ok(())
}

fn write(path: &Path, text: &str) -> Result<()> {
    make_parent(path)?;
    std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

pub(crate) fn run(a: &DungeonArgs) -> Result<()> {
    let (width, height) = parse_size(&a.size)?;
    let kind = match a.kind {
        KindArg::Dungeon => Kind::Dungeon,
        KindArg::Cave => Kind::Cave,
    };
    let d = generate(&Params {
        seed: a.seed,
        kind,
        width,
        height,
    })?;
    write(&a.out, &(d.layout.to_json()? + "\n"))?;
    println!(
        "{}: {}x{} squares, {} rooms, {} doors, {} props -> {}",
        d.layout.name,
        width,
        height,
        d.rooms.len(),
        d.doors.len(),
        d.layout.placements.len(),
        a.out.display()
    );
    if let Some(p) = &a.rules {
        write(p, &(serde_json::to_string_pretty(&d.rules)? + "\n"))?;
    }
    if let Some(p) = &a.meta {
        let meta = serde_json::json!({
            "params": d.params, "rooms": d.rooms, "doors": d.doors, "exits": d.exits,
        });
        write(p, &(serde_json::to_string_pretty(&meta)? + "\n"))?;
    }
    if a.scene.is_none() && a.render.is_none() {
        return Ok(());
    }
    let lib = Library::load_stack(&a.library)
        .with_context(|| format!("loading {}", a.library.display()))?;
    if let Some(p) = &a.scene {
        let scene = arda_scene::build_scene(&d.layout, &lib, a.seed, Some(&d.rules))?;
        write(p, &(scene.to_json_pretty()? + "\n"))?;
    }
    if let Some(p) = &a.render {
        let opts = RenderOptions {
            ppsq: a.ppsq,
            grid: a.grid,
            lighting: true,
        };
        let img = render_with(&d.layout, &lib, a.seed, &opts, &Style::default())?;
        make_parent(p)?;
        img.write_png(p)?;
        println!("rendered {} ({}x{} px)", p.display(), img.width, img.height);
    }
    Ok(())
}
