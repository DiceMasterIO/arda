//! Writes the synthetic "raw AI" set used by the tests (plus a tree and
//! sacks) and an `import.toml`, for trying the importer end to end:
//!
//! ```sh
//! cargo run -p arda-art-import --example synthetic_raw -- out/v4-import/raw
//! arda tactical import out/v4-import/raw --out out/v4-import/lib \
//!     --manifest out/v4-import/raw/import.toml --contact-sheet out/v4-import/sheet.png
//! arda tactical render --layout riverside \
//!     --library out/v4-import/lib:assets/tactical/placeholder --out out/v4-import/riverside.png
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

#[path = "../tests/fixtures/mod.rs"]
mod fixtures;

use arda_tactical::noise::hash2;
use arda_tactical::Rgba;
use std::path::PathBuf;

/// A broadleaf crown of lobes on white, with a soft shadow falling SE.
fn oak_on_white() -> Rgba {
    let s = 768u32;
    let mut img = Rgba::filled(s, s, [244, 244, 240, 255]);
    let lobes: Vec<(f32, f32, f32)> = (0..9)
        .map(|k| {
            let a = k as f32 * 0.698;
            // A cheap rotation without trig: points on a unit circle.
            let (c, sn) = (
                (1.0 - a * a / 2.0 + a.powi(4) / 24.0),
                (a - a.powi(3) / 6.0 + a.powi(5) / 120.0),
            );
            let r = 150.0 + (hash2(31, k, 0) % 40) as f32;
            (
                360.0 + r * c.clamp(-1.0, 1.0),
                360.0 + r * sn.clamp(-1.0, 1.0),
                120.0 + (hash2(32, k, 0) % 30) as f32,
            )
        })
        .collect();
    let inside = |x: f32, y: f32, dx: f32, dy: f32| {
        lobes
            .iter()
            .any(|(lx, ly, lr)| (x - lx - dx).powi(2) + (y - ly - dy).powi(2) < lr * lr)
            || (x - 360.0 - dx).powi(2) + (y - 360.0 - dy).powi(2) < 190.0f32.powi(2)
    };
    for y in 0..s {
        for x in 0..s {
            let (fx, fy) = (x as f32, y as f32);
            if inside(fx, fy, 0.0, 0.0) {
                let lit = 1.0 - 0.3 * ((fx - 360.0) + (fy - 360.0)) / 600.0;
                let n = (hash2(33, i64::from(x / 4), i64::from(y / 4)) % 40) as f32 - 20.0;
                let g = (92.0 * lit + n).clamp(0.0, 255.0);
                img.set(x, y, [(g * 0.55) as u8, g as u8, (g * 0.35) as u8, 255]);
            } else if inside(fx, fy, 40.0, 36.0) {
                img.set(x, y, [150, 150, 147, 255]);
            }
        }
    }
    img
}

/// Three grain sacks on a light grey backdrop.
fn sacks_on_grey() -> Rgba {
    let mut img = Rgba::filled(400, 400, [206, 206, 208, 255]);
    for (cx, cy) in [(150.0f32, 160.0f32), (250.0, 170.0), (200.0, 250.0)] {
        for y in 0..400u32 {
            for x in 0..400u32 {
                let (dx, dy) = ((x as f32 - cx) / 62.0, (y as f32 - cy) / 50.0);
                let d = dx * dx + dy * dy;
                if d < 1.0 {
                    let lit = 1.0 - 0.35 * (dx + dy) - 0.25 * d;
                    let n = (hash2(41, i64::from(x), i64::from(y)) % 16) as f32;
                    let px = [196.0 * lit + n, 170.0 * lit + n, 120.0 * lit + n]
                        .map(|v| v.clamp(0.0, 255.0) as u8);
                    img.set(x, y, [px[0], px[1], px[2], 255]);
                }
            }
        }
    }
    img
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("out/v4-import/raw"), PathBuf::from);
    fixtures::write_raw_set(&dir);
    fixtures::write_rgb_png(
        &oak_on_white(),
        &dir.join("vegetation/veg.tree_oak__flux_0003.png"),
    );
    fixtures::write_rgb_png(&sacks_on_grey(), &dir.join("props/prop.sacks.png"));
    std::fs::write(dir.join("import.toml"), fixtures::MANIFEST).unwrap();
    println!("wrote the synthetic raw set to {}", dir.display());
}
