//! Renders the synthetic countryside windows to `out/fields/*.png`, with
//! their sidecars and the asset fallbacks taken.
//!
//! ```sh
//! cargo run --release -p arda-fields --example countryside [-- --ppsq 16]
//! ```

use arda_fields::degrade::{adapt, Fallback};
use arda_fields::overlay::paint_furrows;
use arda_fields::supplement::{placeholder_library, supplemented_library};
use arda_fields::synthetic::{self, Scenario};
use arda_fields::{generate, FieldsWindow};
use arda_tactical::{render, Library, RenderOptions};
use std::collections::BTreeMap;
use std::error::Error;
use std::path::Path;

const SEED: u64 = 39;

fn draw(
    lib: &Library,
    win: &FieldsWindow,
    ppsq: u32,
    out: &Path,
) -> Result<Vec<Fallback>, Box<dyn Error>> {
    let (mut layout, mut fallbacks) = adapt(&win.layout, lib);
    // The compositor lights every elevation step as a cliff, which striates
    // natural slopes in 5-ft contour lines. Previews keep only earthworks;
    // the layout itself keeps absolute elevations.
    for (sq, e) in layout.squares.iter_mut().zip(&win.earthworks_ft) {
        sq.elevation_ft = *e;
    }
    fallbacks.push(Fallback {
        kind: "render",
        wanted: "absolute elevation_ft".into(),
        used: Some("earthworks only (natural slope not shaded)".into()),
        count: layout.squares.len(),
    });
    let opts = RenderOptions {
        ppsq,
        grid: false,
        lighting: true,
    };
    let mut img = render(&layout, lib, SEED, &opts)?;
    paint_furrows(&mut img, &layout, &win.sidecar, lib, ppsq);
    img.write_png(out)?;
    println!("wrote {} ({}x{} px)", out.display(), img.width, img.height);
    Ok(fallbacks)
}

/// A close-up window of `size` squares centred on world point `c`.
fn closeup(s: &Scenario, c: [f64; 2], size: u32) -> Result<FieldsWindow, Box<dyn Error>> {
    let half = f64::from(size) / 2.0 * arda_fields::geom::SQUARE_M;
    Ok(generate(
        &s.inputs(),
        [c[0] - half, c[1] - half],
        size,
        size,
        SEED,
    )?)
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let ppsq = args
        .iter()
        .position(|a| a == "--ppsq")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(16);
    let dir = Path::new("out/fields");
    std::fs::create_dir_all(dir)?;
    let (lib, added) = supplemented_library()?;
    println!("supplement added {} placeholder assets", added.len());
    let bare = placeholder_library()?;
    let mut report: BTreeMap<String, Vec<Fallback>> = BTreeMap::new();
    for s in synthetic::all() {
        let win = generate(&s.inputs(), s.origin_m, s.w, s.h, SEED)?;
        std::fs::write(
            dir.join(format!("{}.json", s.name)),
            serde_json::to_string(&win.sidecar)?,
        )?;
        let fb = draw(&lib, &win, ppsq, &dir.join(format!("{}.png", s.name)))?;
        report.insert(s.name.to_string(), fb);
        if s.name == "hedge_country" {
            let fb = draw(&bare, &win, ppsq, &dir.join("hedge_country_bare.png"))?;
            report.insert("hedge_country_bare".into(), fb);
        }
    }
    let close: [(&str, [f64; 2], u32); 3] = [
        ("orchard_farmstead", [2050.0, 2050.0], 72),
        ("watermill", [2040.0, 2030.0], 64),
        ("quarry", [2090.0, 2020.0], 96),
    ];
    for (name, c, size) in close {
        let Some(s) = synthetic::by_name(name) else {
            continue;
        };
        let win = closeup(&s, c, size)?;
        let fb = draw(&lib, &win, 64, &dir.join(format!("{name}_close.png")))?;
        report.insert(format!("{name}_close"), fb);
    }
    std::fs::write(
        dir.join("fallbacks.json"),
        serde_json::to_string_pretty(&report)? + "\n",
    )?;
    for (name, fbs) in &report {
        for f in fbs {
            println!(
                "{name}: {} {} -> {} ({})",
                f.kind,
                f.wanted,
                f.used.as_deref().unwrap_or("dropped"),
                f.count
            );
        }
    }
    Ok(())
}
